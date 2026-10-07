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

WRAP="$ROOT/target/clap-wrapper-src"
BUILD="$ROOT/target/clap-wrapper-build"
if [[ ! -d "$WRAP/.git" ]]; then
  git clone --depth 1 https://github.com/free-audio/clap-wrapper.git "$WRAP"
fi

# Subdirectory usage (not clap-wrapper as top-level) so we only build AUv2
# as an effect (`aufx`) and embed DAWAgent.clap inside the .component.
cmake -S "$ROOT/scripts/wrap_au" -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=10.13 \
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

# Ad-hoc sign so hosts will load CI artifacts. Developer ID is optional and not used here.
codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.component" || true
if [[ -d "$OUT_DIR/$NAME.vst3" ]]; then
  codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.vst3" || true
fi
if [[ -d "$OUT_DIR/$NAME.clap" || -f "$OUT_DIR/$NAME.clap" ]]; then
  codesign --force --sign - --timestamp=none "$OUT_DIR/$NAME.clap" || true
fi

echo "AU wrapper at $OUT_DIR/$NAME.component"
