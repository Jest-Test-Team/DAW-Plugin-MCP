#!/usr/bin/env bash
# Static landing for Vercel. This repo is Rust/Julia, not a Python app.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEBUG_LOG="${DEBUG_LOG_PATH:-$ROOT/.cursor/debug-a99a1e.log}"

# region agent log
dbg() {
  local hyp="$1" loc="$2" msg="$3" data="$4"
  local ts
  ts="$(python3 -c 'import time; print(int(time.time()*1000))' 2>/dev/null || date +%s000)"
  local line
  line="$(printf '{"sessionId":"a99a1e","runId":"%s","hypothesisId":"%s","location":"%s","message":"%s","data":%s,"timestamp":%s}\n' \
    "${DEBUG_RUN_ID:-vercel}" "$hyp" "$loc" "$msg" "${data:-{}}" "$ts")"
  echo "DEBUG_A99A1E $line"
  mkdir -p "$(dirname "$DEBUG_LOG")" 2>/dev/null || true
  echo "$line" >> "$DEBUG_LOG" 2>/dev/null || true
}
# endregion

cd "$ROOT"
# region agent log
dbg A "scripts/vercel-build.sh:start" "vercel static build" \
  "$(printf '{"has_index":%s,"has_pyproject":%s,"has_vercel_json":%s}' \
    "$( [[ -f public/index.html ]] && echo true || echo false )" \
    "$( [[ -f pyproject.toml ]] && echo true || echo false )" \
    "$( [[ -f vercel.json ]] && echo true || echo false )")"
# endregion

if [[ ! -f public/index.html ]]; then
  echo "missing public/index.html" >&2
  exit 1
fi

# region agent log
dbg A "scripts/vercel-build.sh:ok" "publishing public/" \
  "$(printf '{"bytes":%s}' "$(wc -c < public/index.html | tr -d ' ')")"
# endregion

echo "Vercel output: public/ (static landing, not Python)"
ls -la public
