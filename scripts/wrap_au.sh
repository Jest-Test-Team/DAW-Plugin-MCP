#!/usr/bin/env bash
# Wrap a bundled CLAP as AUv2 via clap-wrapper. macOS only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLAP="${1:-$ROOT/target/bundled/DAWAgent.clap}"
OUT_DIR="${2:-$ROOT/target/bundled}"
NAME="DAWAgent"
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
DEBUG_LOG="${DEBUG_LOG_PATH:-$ROOT/.cursor/debug-a99a1e.log}"
# region agent log
dbg() {
  local hyp="$1" loc="$2" msg="$3" data="$4"
  local ts
  ts="$(python3 -c 'import time; print(int(time.time()*1000))' 2>/dev/null || date +%s000)"
  local line
  line="$(printf '{"sessionId":"a99a1e","runId":"%s","hypothesisId":"%s","location":"%s","message":"%s","data":%s,"timestamp":%s}\n' \
    "${DEBUG_RUN_ID:-post-fix}" "$hyp" "$loc" "$msg" "${data:-{}}" "$ts")"
  echo "DEBUG_A99A1E $line"
  mkdir -p "$(dirname "$DEBUG_LOG")" 2>/dev/null || true
  echo "$line" >> "$DEBUG_LOG" 2>/dev/null || true
}
# endregion

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "AU wrapping requires macOS" >&2
  exit 1
fi
if ! command -v cmake >/dev/null; then
  echo "cmake is required to wrap AU (GitHub macos-14 provides it)" >&2
  exit 1
fi
if [[ ! -e "$CLAP" ]]; then
  echo "missing CLAP at $CLAP" >&2
  exit 1
fi

WRAP="$ROOT/target/clap-wrapper-src"
BUILD="$ROOT/target/clap-wrapper-build"
if [[ ! -d "$WRAP/.git" ]]; then
  git clone --depth 1 https://github.com/free-audio/clap-wrapper.git "$WRAP"
fi

# Subdirectory usage (not clap-wrapper as top-level) so we only build AUv2
# as an effect (`aufx`) and embed DAWAgent.clap inside the .component.
# region agent log
dbg A "scripts/wrap_au.sh:pre-cmake" "cmake configure inputs" \
  "$(printf '{"cmake":"%s","cxx_std_flag":"17","generator_hint":"%s","clap":"%s"}' \
    "$(command -v cmake || echo missing)" "${CMAKE_GENERATOR:-default}" "$CLAP")"
# endregion

cmake -S "$ROOT/scripts/wrap_au" -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=10.13 \
  -DCMAKE_CXX_STANDARD=17 \
  -DCMAKE_CXX_STANDARD_REQUIRED=ON \
  -DCMAKE_OBJCXX_STANDARD=17 \
  -DCMAKE_OBJCXX_STANDARD_REQUIRED=ON \
  -DCLAP_WRAPPER_CXX_STANDARD=17 \
  -DCMAKE_EXPORT_COMPILE_COMMANDS=ON \
  -DCLAP_WRAPPER_SRC="$WRAP" \
  -DDAW_CLAP_PATH="$CLAP"

# region agent log
cxx_val="$(grep -E '^CMAKE_CXX_STANDARD:' "$BUILD/CMakeCache.txt" 2>/dev/null | head -n1 | cut -d= -f2- || true)"
obj_val="$(grep -E '^CMAKE_OBJCXX_STANDARD:' "$BUILD/CMakeCache.txt" 2>/dev/null | head -n1 | cut -d= -f2- || true)"
gen_val="$(grep -E '^CMAKE_GENERATOR:' "$BUILD/CMakeCache.txt" 2>/dev/null | head -n1 | cut -d= -f2- || true)"
mm_has_std17=false
if [[ -f "$BUILD/compile_commands.json" ]] && grep -Eq 'std=c\+\+17|std=gnu\+\+17' "$BUILD/compile_commands.json"; then
  mm_has_std17=true
fi
dbg A "scripts/wrap_au.sh:post-cmake" "configured C++/ObjC++ standard" \
  "$(printf '{"cxx":"%s","objcxx":"%s","mm_has_std17":%s}' "$cxx_val" "$obj_val" "$mm_has_std17")"
dbg B "scripts/wrap_au.sh:post-cmake" "OBJCXX compile_commands present" \
  "$(printf '{"compile_commands":%s}' "$( [[ -f "$BUILD/compile_commands.json" ]] && echo true || echo false )")"
dbg C "scripts/wrap_au.sh:post-cmake" "cmake generator" \
  "$(printf '{"generator":"%s"}' "$gen_val")"
# endregion

set +e
cmake --build "$BUILD" --config Release --parallel
build_rc=$?
set -e
# region agent log
dbg A "scripts/wrap_au.sh:post-build" "cmake build exit" \
  "$(printf '{"exit":%s,"component_found":%s}' "$build_rc" "$(find "$BUILD" -name '*.component' -print -quit | grep -q . && echo true || echo false)")"
# endregion
if [[ "$build_rc" -ne 0 ]]; then
  exit "$build_rc"
fi

mkdir -p "$OUT_DIR"
if [[ "$CLAP" != "$OUT_DIR/$NAME.clap" ]]; then
  rm -rf "$OUT_DIR/$NAME.clap"
  cp -R "$CLAP" "$OUT_DIR/$NAME.clap"
fi

FOUND="$(find "$BUILD" -name "*.component" -print | head -n 1 || true)"
if [[ -z "$FOUND" ]]; then
  echo "clap-wrapper did not produce a .component" >&2
  find "$BUILD" -maxdepth 3 -print >&2 || true
  exit 1
fi
rm -rf "$OUT_DIR/$NAME.component"
cp -R "$FOUND" "$OUT_DIR/$NAME.component"

# Ad-hoc sign so hosts will load CI artifacts. Developer ID is optional and not used here.
codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.component" || true
if [[ -d "$OUT_DIR/$NAME.vst3" ]]; then
  codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.vst3" || true
fi
if [[ -d "$OUT_DIR/$NAME.clap" || -f "$OUT_DIR/$NAME.clap" ]]; then
  codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.clap" || true
fi

echo "AU wrapper at $OUT_DIR/$NAME.component"
