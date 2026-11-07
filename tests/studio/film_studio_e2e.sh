#!/usr/bin/env bash
# Film studio, end to end (PLAN-155): a real browser edits a real film.
#
# Serves a COPY of demos/film-robot-day/, opens the studio in Chromium through
# agent-browser, and makes each kind of gesture with real input events. Every
# check reads the copy's SOURCE afterwards: a gesture must rewrite exactly its
# word, and the film must come back where it was (playhead, pause, selection).
#
#   tests/studio/film_studio_e2e.sh            # uses target/debug/spacetime
#   SPACETIME_BIN=... PORT=4519 tests/studio/film_studio_e2e.sh
#
# Needs: agent-browser (Chromium), a cdp build of spacetime. Exit 0 = all pass.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN=${SPACETIME_BIN:-${CARGO_TARGET_DIR:-$ROOT/target}/debug/spacetime}
PORT=${PORT:-4519}
WORK=$(mktemp -d)
FILM="$WORK/film"
cp -r "$ROOT/demos/film-robot-day" "$FILM"
SRC="$FILM/index.st"
cd "$ROOT"
"$BIN" serve "$FILM" -p "$PORT" >"$WORK/serve.log" 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null; agent-browser close >/dev/null 2>&1; rm -rf "$WORK"' EXIT

pass=0; fail=0
ok() { echo "  ok    $1"; pass=$((pass + 1)); }
bad() { echo "  FAIL  $1"; fail=$((fail + 1)); }
js() { agent-browser eval "$1" 2>/dev/null | sed 's/^"//; s/"$//'; }
ready() {  # the film's transport and the studio's score are up on a settled page
  for _ in $(seq 1 120); do
    [ "$(js "(typeof ST!=='undefined' && ST.stuFilm && ST.stuFilm.transport() && window.SpacetimeLocal && SpacetimeLocal.stuScore && performance.now()>1500) ? 'y' : 'n'")" = y ] && return 0
    sleep 0.5
  done
  return 1
}
click_at() { agent-browser mouse move "$1" "$2" >/dev/null; agent-browser mouse down >/dev/null; agent-browser mouse up >/dev/null; }
drag_by() { agent-browser mouse move "$1" "$2" >/dev/null; agent-browser mouse down >/dev/null; agent-browser mouse move $(($1 + $3 / 2)) "$2" >/dev/null; agent-browser mouse move $(($1 + $3)) "$2" >/dev/null; agent-browser mouse up >/dev/null; }
centre() { js "var r=($1).getBoundingClientRect(); Math.round(r.x+r.width/2)+' '+Math.round(r.y+r.height/2)"; }

for _ in $(seq 1 120); do curl -s -o /dev/null "http://127.0.0.1:$PORT/" && break; sleep 0.5; done
agent-browser set viewport 1600 900 >/dev/null 2>&1
agent-browser open "http://127.0.0.1:$PORT/" >/dev/null 2>&1
ready || { echo "the page never came up"; exit 1; }
# open the studio the way a person does: its chip in the dev dock
js "document.documentElement.classList.contains('stu-on') || document.querySelector('.st-studio__pill').click(); 1" >/dev/null
sleep 1
[ "$(js "document.documentElement.classList.contains('stu-film-on')")" = true ] || { echo "the film studio did not open"; exit 1; }

echo "film studio:"
[ "$(js "document.querySelectorAll('.stu-film__bars .stu-bar-clip').length")" -gt 10 ] && ok "the timeline is drawn from the score" || bad "the timeline is drawn from the score"

# 1. the transport holds a frame
js "ST.stuFilm.transport().pause(); ST.stuFilm.seek(12000); 1" >/dev/null
sleep 0.5
[ "$(js "Math.round(ST.stuFilm.transport().position)")" = 12000 ] && ok "seek + pause holds 12 s" || bad "seek + pause holds 12 s"

# 2. drag a clip: its `at` word
js "SpacetimeLocal.stuFilmSel='clip:1'; var i=SpacetimeLocal.stuFilmRows.findIndex(r=>r.label=='&card'); document.querySelectorAll('.stu-film__bars .stu-bar-clip')[i].scrollIntoView({block:'center'}); 1" >/dev/null
read -r X Y <<<"$(centre "document.querySelectorAll('.stu-film__bars .stu-bar-clip')[SpacetimeLocal.stuFilmRows.findIndex(r=>r.label=='&card')]")"
drag_by "$X" "$Y" 23
sleep 2; ready
grep -q '&card at 1s for 5.2s;' "$SRC" && ok "dragging &card rewrites its at (0.4s -> 1s)" || bad "dragging &card rewrites its at: $(grep -n '&card at' "$SRC")"
[ "$(js "Math.round(ST.stuFilm.transport().position)+' '+ST.stuFilm.transport().playing+' '+SpacetimeLocal.stuFilmSel")" = "12000 false clip:1" ] && ok "after the reload: 12 s, paused, &card selected" || bad "restore: $(js "Math.round(ST.stuFilm.transport().position)+' '+ST.stuFilm.transport().playing+' '+SpacetimeLocal.stuFilmSel")"

# 3. type in the inspector: its `for` word
js "var i=document.querySelectorAll('.stu-film-input')[1]; i.focus(); i.select(); 1" >/dev/null
agent-browser keyboard type "4.5s" >/dev/null; agent-browser press Tab >/dev/null
sleep 2; ready
grep -q '&card at 1s for 4.5s;' "$SRC" && ok "typing in the inspector rewrites for (5.2s -> 4.5s)" || bad "inspector: $(grep -n '&card at' "$SRC")"

# 4. drag a keyframe: its stop
js "var k=SpacetimeLocal.stuFilmKeys.findIndex(k=>k.key=='stop:1:0:1'); document.querySelectorAll('.stu-key')[k].scrollIntoView({block:'center'}); 1" >/dev/null
read -r X Y <<<"$(centre "document.querySelectorAll('.stu-key')[SpacetimeLocal.stuFilmKeys.findIndex(k=>k.key=='stop:1:0:1')]")"
drag_by "$X" "$Y" 20
sleep 2; ready
grep -Eq 'opacity: 0 -> 1 at (2[0-9]|3[0-9])(\.[0-9])?% -> 1 at 78% -> 0;' "$SRC" && ok "dragging a keyframe rewrites its stop (18% -> later)" || bad "keyframe: $(grep -n 'opacity: 0 -> 1 at' "$SRC" | head -1)"

# 5. a drum cell
js "SpacetimeLocal.stuFilmSel='sound:8'; 1" >/dev/null; sleep 1
read -r X Y <<<"$(centre "document.querySelectorAll('.stu-cell[data-kind=cell]')[1]")"
click_at "$X" "$Y"
sleep 2; ready
grep -q -- '--kick:  x x \. \. \. \. \. \. x \. x \. \. \. \. \. ;' "$SRC" && ok "clicking a drum cell writes x" || bad "cell: $(grep -n -- '--kick:' "$SRC")"
[ "$(js "SpacetimeLocal.stuScore.programs[0].forms['--beat'].props['--kick'][0]")" = "x x . . . . . . x . x . . . . ." ] && ok "the synth program hears it" || bad "the synth program hears it"

# 6. a stage part's unwritten param (inserted after the last written one)
js "var i=SpacetimeLocal.stuFilmParts.findIndex(p=>p.label=='&robot'); SpacetimeLocal.stuFilmSel='part:'+i; 1" >/dev/null; sleep 1
js "var x=[...document.querySelectorAll('.stu-film-input')].find(i=>i.dataset.label=='spinY'); x.focus(); x.select(); 1" >/dev/null
agent-browser keyboard type "0.5" >/dev/null; agent-browser press Tab >/dev/null
sleep 2; ready
grep -q 'posZ: -7, spinY: 0.5)' "$SRC" && ok "a default param is written in where it belongs" || bad "param: $(grep -n '@gltf &robot' "$SRC")"

# 7. the code pane saves the whole file
js "document.querySelector('.stu-film__tab[data-panel=Code]').click(); var a=document.querySelector('.stu-code'); a.focus(); a.setSelectionRange(0,0); 1" >/dev/null
agent-browser keyboard type "// from the code pane" >/dev/null; agent-browser press Enter >/dev/null
js "document.querySelector('.stu-code').blur(); 1" >/dev/null
sleep 2; ready
[ "$(head -1 "$SRC")" = "// from the code pane" ] && ok "the code pane saves" || bad "code pane: $(head -1 "$SRC")"
js "document.querySelector('.stu-film__tab[data-panel=Timeline]').click(); 1" >/dev/null

# 8. keys
js "document.activeElement && document.activeElement.blur(); ST.stuFilm.transport().pause(); 1" >/dev/null
agent-browser press Space >/dev/null; sleep 0.3
[ "$(js "ST.stuFilm.transport().playing")" = true ] && ok "Space plays" || bad "Space plays"
agent-browser press Space >/dev/null

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
