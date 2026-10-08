#!/bin/sh
# Copy the landing page into Vercel's output directory.
# The project Root Directory may be a subdirectory (tests/robot) while this
# script and web/index.html live at the repository root. Build commands run
# with cwd set to that Root Directory, and outputDirectory is relative to cwd.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SRC="$ROOT/web/index.html"

if [ ! -f "$SRC" ]; then
  echo "missing landing page: $SRC" >&2
  exit 1
fi

if [ "$(basename "$PWD")" = "public" ]; then
  cp "$SRC" "$PWD/index.html"
  test -f "$PWD/index.html"
else
  mkdir -p "$PWD/public"
  cp "$SRC" "$PWD/public/index.html"
  test -f "$PWD/public/index.html"
fi
