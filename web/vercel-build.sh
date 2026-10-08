#!/bin/sh
# Populate outputDirectory after Vercel may empty it or cd into it.
set -e
# region agent log
dbg() {
  hyp="$1"
  msg="$2"
  data="$3"
  ts="$(python3 -c 'import time; print(int(time.time()*1000))' 2>/dev/null || date +%s000)"
  line="{\"sessionId\":\"a99a1e\",\"runId\":\"vercel-post-fix\",\"hypothesisId\":\"$hyp\",\"location\":\"web/vercel-build.sh\",\"message\":\"$msg\",\"data\":$data,\"timestamp\":$ts}"
  echo "DEBUG_A99A1E $line"
  logf="/Users/dennis/Documents/GitHub/DAW-Plugin-MCP/.cursor/debug-a99a1e.log"
  if [ -d "$(dirname "$logf")" ]; then
    echo "$line" >> "$logf" 2>/dev/null || true
  fi
}
# endregion

exists() {
  [ -f "$1" ] && echo true || echo false
}

# region agent log
dbg C "build cwd and sources" "{\"cwd\":\"$(pwd)\",\"public_index\":$(exists public/index.html),\"cwd_index\":$(exists index.html),\"web_index\":$(exists web/index.html),\"parent_web\":$(exists ../web/index.html)}"
# endregion

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
  # region agent log
  dbg C "already in output directory" "{\"out\":\".\"}"
  # endregion
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
# region agent log
dbg F "copied landing page into output dir" "{\"src\":\"$SRC\",\"out\":\"$OUT\",\"ok\":$(exists "$OUT/index.html")}"
# endregion
test -f "$OUT/index.html"
