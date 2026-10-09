#!/usr/bin/env bash
# Wrap a bundled CLAP as AUv2 via clap-wrapper. macOS only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLAP="${1:-$ROOT/target/bundled/DAWAgent.clap}"
OUT_DIR="${2:-$ROOT/target/bundled}"
NAME="DAWAgent"
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"

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

# Do not store clap-wrapper under target/: Swatinem/rust-cache restores target/
# and can leave an incomplete tree with .git but no CMakeLists.txt.
WRAP="${CLAP_WRAPPER_SRC:-$ROOT/.cache/clap-wrapper-src}"
BUILD="${CLAP_WRAPPER_BUILD:-$ROOT/.cache/clap-wrapper-build}"
if [[ ! -f "$WRAP/CMakeLists.txt" ]]; then
  rm -rf "$WRAP"
  git clone --depth 1 https://github.com/free-audio/clap-wrapper.git "$WRAP"
fi
if [[ ! -f "$WRAP/CMakeLists.txt" ]]; then
  echo "clap-wrapper clone missing CMakeLists.txt at $WRAP" >&2
  ls -la "$WRAP" >&2 || true
  exit 1
fi

# Subdirectory usage (not clap-wrapper as top-level) so we only build AUv2
# as an effect (`aufx`) and embed DAWAgent.clap inside the .component.
cmake -S "$ROOT/scripts/wrap_au" -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=10.13 \
  -DCMAKE_CXX_STANDARD=17 \
  -DCMAKE_CXX_STANDARD_REQUIRED=ON \
  -DCMAKE_OBJCXX_STANDARD=17 \
  -DCMAKE_OBJCXX_STANDARD_REQUIRED=ON \
  -DCLAP_WRAPPER_CXX_STANDARD=17 \
  -DCLAP_WRAPPER_SRC="$WRAP" \
  -DDAW_CLAP_PATH="$CLAP"

cmake --build "$BUILD" --config Release --parallel

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

# Embed sidecar before chmod/codesign so daw-mcp is inside the signed bundles.
DAEMON="${ROOT}/target/release/daw-mcp"
if [[ -x "$DAEMON" ]]; then
  bash "$ROOT/scripts/embed_daemon.sh" "$DAEMON" "$OUT_DIR"
fi

# clap-wrapper copies can land as -rw-r--r--; Logic then lists the AU in
# Plug-in Manager but fails to instantiate it in the insert menu.
chmod_macos_executables() {
  local root="$1"
  if [[ ! -e "$root" ]]; then
    return 0
  fi
  find "$root" -type d -name MacOS | while read -r dir; do
    find "$dir" -maxdepth 1 -type f -exec chmod +x {} +
  done
}
chmod_macos_executables "$OUT_DIR/$NAME.component"
chmod_macos_executables "$OUT_DIR/$NAME.clap"

# Sign inner CLAP then outer bundle. --deep can produce a component auval will not load.
INNER_CLAP="$(find "$OUT_DIR/$NAME.component/Contents/PlugIns" -maxdepth 2 -name "*.clap" -print | head -n 1 || true)"
if [[ -n "$INNER_CLAP" ]]; then
  codesign --force --sign - --timestamp=none "$INNER_CLAP" || true
fi
codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.component" || true
if [[ -d "$OUT_DIR/$NAME.vst3" ]]; then
  codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.vst3" || true
fi
if [[ -d "$OUT_DIR/$NAME.clap" || -f "$OUT_DIR/$NAME.clap" ]]; then
  codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.clap" || true
fi

echo "AU wrapper at $OUT_DIR/$NAME.component"
