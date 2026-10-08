#!/bin/sh
# Populate outputDirectory after Vercel may empty it or cd into it.
set -e

SRC=""
OUT="public"
if [ -f web/index.html ]; then
  SRC="web/index.html"
elif [ -f ../web/index.html ]; then
  SRC="../web/index.html"
  OUT="."
elif [ -f public/index.html ]; then
  SRC="public/index.html"
elif [ -f index.html ]; then
  exit 0
fi

if [ -z "$SRC" ]; then
  echo "no index.html source found" >&2
  ls -la >&2
  ls -la .. >&2 || true
  exit 1
fi

mkdir -p "$OUT"
cp "$SRC" "$OUT/index.html"
test -f "$OUT/index.html"
