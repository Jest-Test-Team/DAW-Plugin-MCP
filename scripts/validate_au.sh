#!/usr/bin/env bash
# Install the wrapped AU where auval scans, wait for registration, then validate.
# auval -v TYPE SUBTYPE MANUFACTURER  ->  aufx Agnt DwMc
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-$ROOT/target/bundled/DAWAgent.component}"
NAME="DAWAgent"
TYPE="aufx"
SUBTYPE="Agnt"
MANU="DwMc"

if [[ ! -d "$SRC" ]]; then
  echo "missing AU bundle at $SRC" >&2
  exit 1
fi

chmod_macos_executables() {
  local root="$1"
  find "$root" -type d -name MacOS | while read -r dir; do
    find "$dir" -maxdepth 1 -type f -exec chmod +x {} +
  done
}

# Sign inner CLAP then the outer component. --deep can leave a bundle auval will not load.
sign_au() {
  local bundle="$1"
  local inner
  inner="$(find "$bundle/Contents/PlugIns" -maxdepth 2 -name "*.clap" -print | head -n 1 || true)"
  if [[ -n "$inner" ]]; then
    codesign --force --sign - --timestamp=none "$inner" || true
  fi
  codesign --force --sign - --timestamp=none "$bundle" || true
}

USER_COMP="$HOME/Library/Audio/Plug-Ins/Components"
mkdir -p "$USER_COMP"
rm -rf "$USER_COMP/$NAME.component"
cp -R "$SRC" "$USER_COMP/$NAME.component"
chmod_macos_executables "$USER_COMP/$NAME.component"
xattr -cr "$USER_COMP/$NAME.component" || true
sign_au "$USER_COMP/$NAME.component"

SYS_COMP="/Library/Audio/Plug-Ins/Components"
if sudo -n true 2>/dev/null; then
  sudo mkdir -p "$SYS_COMP"
  sudo rm -rf "$SYS_COMP/$NAME.component"
  sudo cp -R "$USER_COMP/$NAME.component" "$SYS_COMP/"
fi

rm -rf "$HOME/Library/Caches/AudioUnitCache" \
  "$HOME/Library/Caches/com.apple.audiounits.cache" || true
killall -9 AudioComponentRegistrar 2>/dev/null || true

listed=0
for _ in $(seq 1 25); do
  if auval -a 2>/dev/null | grep -q "$MANU"; then
    listed=1
    break
  fi
  sleep 1
done

if [[ "$listed" != 1 ]]; then
  echo "AU never appeared in auval -a ($TYPE $SUBTYPE $MANU)" >&2
  ls -la "$USER_COMP/$NAME.component/Contents/MacOS" >&2 || true
  file "$USER_COMP/$NAME.component/Contents/MacOS/"* >&2 || true
  plutil -p "$USER_COMP/$NAME.component/Contents/Info.plist" >&2 || true
  auval -a >&2 || true
  exit 1
fi

auval -v "$TYPE" "$SUBTYPE" "$MANU"
