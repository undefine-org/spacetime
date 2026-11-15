#!/usr/bin/env bash
# The reusable property editor writes a real imported definition, preserves its
# spelling, reloads the preview and uses the shared undo/stale-write contract.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN=${SPACETIME_BIN:-$ROOT/result/bin/spacetime}
PORT=${PORT:-4563}
WORK=$(mktemp -d)
SESSION="authoring-source-$$"
cp -r "$ROOT/demos/authoring-catalog/." "$WORK/"
"$BIN" serve "$WORK" -p "$PORT" >"$WORK/serve.log" 2>&1 &
SERVER=$!
cleanup() { kill "$SERVER" 2>/dev/null || true; agent-browser --session "$SESSION" close >/dev/null 2>&1 || true; rm -rf "$WORK"; }
trap cleanup EXIT
browser() { agent-browser --session "$SESSION" "$@"; }
js() { browser eval "$1" 2>/dev/null | sed 's/^"//;s/"$//'; }
wait_for() {
  for _ in $(seq 1 120); do
    if [ "$(js "Boolean($1)")" = true ]; then return 0; fi
    sleep 0.25
  done
  echo "FAIL: $2" >&2
  cat "$WORK/serve.log" >&2
  return 1
}
for _ in $(seq 1 120); do
  if curl --max-time 1 -sf -o /dev/null "http://127.0.0.1:$PORT/"; then break; fi
  sleep 0.25
done
browser set viewport 1440 1100 >/dev/null
browser open "http://127.0.0.1:$PORT/" >/dev/null
wait_for "document.querySelector('.catalog-properties .w-swatch')" "color control never appeared"
browser fill '.catalog-properties .w-text' '#d8ec9b' >/dev/null
browser press Tab >/dev/null
wait_for "getComputedStyle(document.querySelector('.sample-message')).backgroundColor === 'rgb(216, 236, 155)'" "the actual message did not take its new definition color"
rg -Fq '$fill color = "#d8ec9b"' "$WORK/fixtures.st" || { echo 'FAIL: quoted default lost its spelling' >&2; exit 1; }
echo 'ok: typed color writes its imported default, retains quotes and changes the preview'

wait_for "document.querySelector('[data-name=\"--light-surface\"]')" "catalog inventory did not settle after the edit"
browser click '[data-name="--light-surface"]' >/dev/null
wait_for "document.querySelector('.catalog-properties .w-number')" "number control never appeared"
browser fill '.catalog-properties .w-number' '2.4' >/dev/null
browser press Tab >/dev/null
wait_for "getComputedStyle(document.querySelector('.light-orb')).filter === 'brightness(2.4)'" "the shared surface did not change strength"
rg -Fq '$intensity number = 2.4' "$WORK/fixtures.st"
echo 'ok: the same property editor changes a numeric surface parameter'

wait_for "document.querySelector('[data-name=\"--arrival\"]')" "catalog inventory did not settle after the surface edit"
browser click '[data-name="--arrival"]' >/dev/null
wait_for "document.querySelectorAll('.catalog-properties .w-text').length === 2" "motion controls never appeared"
browser fill '.catalog-properties .au-property:first-child .w-text' '160px' >/dev/null
browser press Tab >/dev/null
wait_for "SpacetimeLocal.program.definitions.find(d => d.name === '--arrival').params[0].default === '160px'" "the motion definition did not refresh"
rg -Fq '$distance length = 160px' "$WORK/fixtures.st"
echo 'ok: a dimensioned motion default is edited through the same component'

curl -sf -X POST "http://127.0.0.1:$PORT/__spacetime/rewrite/undo" >/dev/null
wait_for "SpacetimeLocal.program.definitions.find(d => d.name === '--arrival').params[0].default === '80px'" "undo did not restore the motion default"
rg -Fq '$distance length = 80px' "$WORK/fixtures.st"
echo 'ok: shared source undo restores the exact prior default'

STALE=$(js "fetch('/__spacetime/rewrite', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({entry:'index.st',edits:[{file:'fixtures.st',start:0,end:1,expect:'stale',text:'x'}]})}).then(r=>r.json()).then(r=>r.error)")
case "$STALE" in *STALE*) ;; *) echo "FAIL: stale edit was not refused: $STALE" >&2; exit 1;; esac
rg -Fq '$distance length = 80px' "$WORK/fixtures.st"
echo 'ok: stale source edits are refused without changing the definition'
