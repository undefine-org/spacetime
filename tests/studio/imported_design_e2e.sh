#!/usr/bin/env bash
# An imported design edit must reach every served route (PLAN-158).
# Uses a scratch site and real Chromium, through the existing Studio harness.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN=${SPACETIME_BIN:-${CARGO_TARGET_DIR:-$ROOT/target}/debug/spacetime}
PORT=${PORT:-4579}
export AGENT_BROWSER_SESSION=${AGENT_BROWSER_SESSION:-spacetime-imports-$$}
WORK=$(mktemp -d)
SERVER=
cleanup() {
  if [[ -n "$SERVER" ]]; then kill "$SERVER" 2>/dev/null || true; fi
  agent-browser close >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/design"
cat > "$WORK/index.st" <<'ST'
@import "./design/index.st";
<div class="probe"><--sample /></div>
ST
cp "$WORK/index.st" "$WORK/other.st"
cat > "$WORK/design/index.st" <<'ST'
@import "./piece.st";
ST
write_design() {
  cat > "$WORK/design/piece.st" <<ST
@form markup --sample { <span>$1</span> }
.probe { width: $2px; }
ST
}
write_design Before 120
cd "$ROOT"
"$BIN" serve "$WORK" -p "$PORT" > "$WORK/serve.log" 2>&1 &
SERVER=$!
for _ in $(seq 1 120); do
  curl -fsS "http://127.0.0.1:$PORT/" -o /dev/null 2>/dev/null && break
  sleep 0.5
done
js() { agent-browser eval "$1" 2>/dev/null | sed 's/^"//; s/"$//'; }
read_design() {
  js "(()=>{const e=document.querySelector('.probe'); return e ? e.textContent.trim()+' '+Math.round(e.getBoundingClientRect().width) : 'missing';})()"
}
agent-browser open "http://127.0.0.1:$PORT/other" >/dev/null
[[ "$(read_design)" = 'Before 120' ]] || { echo 'FAIL: second route did not mount'; exit 1; }
agent-browser open "http://127.0.0.1:$PORT/" >/dev/null
[[ "$(read_design)" = 'Before 120' ]] || { echo 'FAIL: entry did not mount'; exit 1; }
# Only the transitive import changes: entry bytes and mtime stay untouched.
write_design After 240
updated=false
for _ in $(seq 1 60); do
  if [[ "$(read_design)" = 'After 240' ]]; then updated=true; break; fi
  sleep 0.5
done
if [[ "$updated" != true ]]; then
  echo "FAIL: imported edit left the live browser at $(read_design)"
  js "JSON.stringify(Array.from(document.styleSheets).map(s=>({href:s.href,probe:Array.from(s.cssRules||[]).filter(r=>r.cssText.includes('.probe')).map(r=>r.cssText)}))))" || true
  curl -fsS "http://127.0.0.1:$PORT/__spacetime/styles.css?entry=index.st" -o "$WORK/last.css"
  rg -o '\.probe[^}]*}' "$WORK/last.css" || true
  exit 1
fi
agent-browser open "http://127.0.0.1:$PORT/other" >/dev/null
[[ "$(read_design)" = 'After 240' ]] || { echo "FAIL: second route retained stale imported design: $(read_design)"; exit 1; }
echo '2 passed, 0 failed: nested module text and layout refresh on both routes'
