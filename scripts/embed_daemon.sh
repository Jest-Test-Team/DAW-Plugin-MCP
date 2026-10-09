#!/usr/bin/env bash
# Copy daw-mcp next to the plugin binary so the editor can spawn --sidecar.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${1:-}"
OUT="${2:-$ROOT/target/bundled}"

if [[ -z "$BIN" ]]; then
  if [[ -x "$ROOT/target/release/daw-mcp" ]]; then
    BIN="$ROOT/target/release/daw-mcp"
  elif [[ -x "$ROOT/target/release/daw-mcp.exe" ]]; then
    BIN="$ROOT/target/release/daw-mcp.exe"
  elif [[ -x "$ROOT/target/debug/daw-mcp" ]]; then
    BIN="$ROOT/target/debug/daw-mcp"
  else
    echo "missing daw-mcp; run: cargo build -p daw-mcp --release" >&2
    exit 1
  fi
fi
if [[ ! -f "$BIN" ]]; then
  echo "missing binary $BIN" >&2
  exit 1
fi

NAME="$(basename "$BIN")"

embed_into_bundle() {
  local bundle="$1"
  if [[ ! -e "$bundle" ]]; then
    return 0
  fi
  local dest=""
  if [[ -d "$bundle/Contents/MacOS" ]]; then
    dest="$bundle/Contents/MacOS"
  else
    dest="$(find "$bundle" -type d \( -name MacOS -o -name x86_64-win -o -name i386-win \) 2>/dev/null | head -n 1 || true)"
  fi
  if [[ -z "$dest" ]]; then
    echo "no MacOS/x86_64-win dir in $bundle" >&2
    return 0
  fi
  mkdir -p "$dest"
  cp "$BIN" "$dest/$NAME"
  chmod +x "$dest/$NAME" || true
  if command -v codesign >/dev/null && [[ "$(uname -s)" == "Darwin" ]]; then
    codesign --force --sign - --timestamp=none "$dest/$NAME" || true
  fi
  echo "embedded $NAME -> $dest/$NAME"
}

embed_into_bundle "$OUT/DAWAgent.clap"
embed_into_bundle "$OUT/DAWAgent.vst3"

if [[ -d "$OUT/DAWAgent.component" ]]; then
  INNER="$(find "$OUT/DAWAgent.component/Contents/PlugIns" -maxdepth 2 -name "*.clap" -print 2>/dev/null | head -n 1 || true)"
  if [[ -n "$INNER" ]]; then
    embed_into_bundle "$INNER"
  fi
fi
