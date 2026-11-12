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
mark() { js "window.__live = 1; window.__t0 = performance.now(); 1" >/dev/null; }
# a HOT edit: the page took it in place — the marker survives, the studio
# read the new source (its panels show the new word); prints how long it took
live() {
  for _ in $(seq 1 60); do
    [ "$(js "(window.__live===1 && (SpacetimeLocal.stuStatus||'').indexOf('live')>=0 && ($1)) ? 'y' : 'n'")" = y ] && {
      echo "        (live in $(js "Math.round(performance.now()-window.__t0)") ms, no reload)"; return 0; }
    sleep 0.1
  done
  return 1
}

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
mark; drag_by "$X" "$Y" 23
live "SpacetimeLocal.stuScore.scores[0].entries.find(e=>e.subject=='card').at.text=='1s'" && ok "dragging &card is live (no reload)" || bad "dragging &card is live"
grep -q '&card at 1s for 5.2s;' "$SRC" && ok "dragging &card rewrites its at (0.4s -> 1s)" || bad "dragging &card rewrites its at: $(grep -n '&card at' "$SRC")"
[ "$(js "Math.round(ST.stuFilm.transport().position)+' '+ST.stuFilm.transport().playing+' '+SpacetimeLocal.stuFilmSel")" = "12000 false clip:1" ] && ok "after the reload: 12 s, paused, &card selected" || bad "restore: $(js "Math.round(ST.stuFilm.transport().position)+' '+ST.stuFilm.transport().playing+' '+SpacetimeLocal.stuFilmSel")"

# 3. type in the inspector: its `for` word
js "var i=document.querySelectorAll('.stu-film-input')[1]; i.focus(); i.select(); 1" >/dev/null
mark; agent-browser keyboard type "4.5s" >/dev/null; agent-browser press Tab >/dev/null
live "SpacetimeLocal.stuScore.scores[0].entries.find(e=>e.subject=='card').for.text=='4.5s'" && ok "the inspector edit is live" || bad "the inspector edit is live"
grep -q '&card at 1s for 4.5s;' "$SRC" && ok "typing in the inspector rewrites for (5.2s -> 4.5s)" || bad "inspector: $(grep -n '&card at' "$SRC")"

# 4. drag a keyframe: its stop
js "var k=SpacetimeLocal.stuFilmKeys.findIndex(k=>k.key=='stop:1:0:1'); document.querySelectorAll('.stu-key')[k].scrollIntoView({block:'center'}); 1" >/dev/null
read -r X Y <<<"$(centre "document.querySelectorAll('.stu-key')[SpacetimeLocal.stuFilmKeys.findIndex(k=>k.key=='stop:1:0:1')]")"
mark; drag_by "$X" "$Y" 20
live "SpacetimeLocal.stuScore.clips.card.channels[0].stops[1].at > 0.2" && ok "the keyframe drag is live" || bad "the keyframe drag is live"
grep -Eq 'opacity: 0 -> 1 at (2[0-9]|3[0-9])(\.[0-9])?% -> 1 at 78% -> 0;' "$SRC" && ok "dragging a keyframe rewrites its stop (18% -> later)" || bad "keyframe: $(grep -n 'opacity: 0 -> 1 at' "$SRC" | head -1)"

# 5. a drum cell
js "SpacetimeLocal.stuFilmSel='sound:8'; 1" >/dev/null; sleep 1
read -r X Y <<<"$(centre "document.querySelectorAll('.stu-cell[data-kind=cell]')[1]")"
mark; click_at "$X" "$Y"
live "SpacetimeLocal.stuScore.programs[0].forms['--beat'].props['--kick'][0].indexOf('x x')==0" && ok "the drum cell is live (the synth re-rendered)" || bad "the drum cell is live"
grep -q -- '--kick:  x x \. \. \. \. \. \. x \. x \. \. \. \. \. ;' "$SRC" && ok "clicking a drum cell writes x" || bad "cell: $(grep -n -- '--kick:' "$SRC")"
[ "$(js "SpacetimeLocal.stuScore.programs[0].forms['--beat'].props['--kick'][0]")" = "x x . . . . . . x . x . . . . ." ] && ok "the synth program hears it" || bad "the synth program hears it"

# 6. a stage part's unwritten param (inserted after the last written one)
js "var i=SpacetimeLocal.stuFilmParts.findIndex(p=>p.label=='&robot'); SpacetimeLocal.stuFilmSel='part:'+i; 1" >/dev/null; sleep 1
js "var x=document.querySelector('.stu-part-fields .w-field[data-label=rotateY] input.w-number'); x.focus(); x.select(); 1" >/dev/null
mark; agent-browser keyboard type "30" >/dev/null; agent-browser press Tab >/dev/null
live "Math.abs(document.querySelector('canvas.world')._stStageObjects['&robot']._stPart.props.rotation[1] - Math.PI/6) < 0.001 && SpacetimeLocal.stuScore.sources[0].text.indexOf('rotateY: 30') >= 0" && ok "a scene param is live (the robot turned)" || bad "a scene param is live"
grep -q 'posZ: -7, rotateY: 30)' "$SRC" && ok "a default param is written in where it belongs" || bad "param: $(grep -n '@gltf &robot' "$SRC")"

# 7. the code pane saves the whole file
js "document.querySelector('.stu-film__tab[data-panel=Code]').click(); var a=document.querySelector('.stu-code'); a.focus(); a.setSelectionRange(0,0); 1" >/dev/null
agent-browser keyboard type "// from the code pane" >/dev/null; agent-browser press Enter >/dev/null
js "document.querySelector('.stu-code').blur(); 1" >/dev/null
sleep 2; ready
[ "$(head -1 "$SRC")" = "// from the code pane" ] && ok "the code pane saves" || bad "code pane: $(head -1 "$SRC")"
js "document.querySelector('.stu-film__tab[data-panel=Timeline]').click(); 1" >/dev/null

# 8. the Select tool: point at the robot in the 3D, click -> the Scene opens it
js "document.querySelector('.stu-film__tab[data-panel=Timeline]').click(); SpacetimeLocal.stuFilmSel=''; ST.stuFilm.transport().pause(); ST.stuFilm.seek(8000); 1" >/dev/null
sleep 0.5
agent-browser press p >/dev/null
read -r X Y <<<"$(js "var st=ST.stuFilm.stage(); var pt=ST.stuFilm.stagePart(SpacetimeLocal.stuScore, SpacetimeLocal.stuScore.parts.findIndex(p=>p.name=='robot')); var b=new st.THREE.Box3().setFromObject(pt.obj); var c=new st.THREE.Vector3(); b.getCenter(c); c.project(st.camera); var r=st.renderer.domElement.getBoundingClientRect(); Math.round(r.left+(c.x+1)/2*r.width)+' '+Math.round(r.top+(1-c.y)/2*r.height)")"
agent-browser mouse move "$X" "$Y" >/dev/null; sleep 0.4
[ "$(js "(ST.stuFilm.outlines.hover && ST.stuFilm.outlines.hover.pt.name) || ''")" = robot ] && ok "the Select tool outlines what it points at" || bad "the Select tool outlines what it points at"
click_at "$X" "$Y"; sleep 0.6
[ "$(js "SpacetimeLocal.stuTabName+' '+SpacetimeLocal.stuFilmSelLabel")" = "Scene &robot (gltf)" ] && ok "clicking a part opens it in the Scene" || bad "pick: $(js "SpacetimeLocal.stuTabName+' '+SpacetimeLocal.stuFilmSelLabel")"
agent-browser press p >/dev/null

# 9. hovering a part in the Scene list outlines it in the 3D
read -r X Y <<<"$(centre "[...document.querySelectorAll('.stu-part')].find(b=>b.textContent.indexOf('&globe2')>=0)")"
agent-browser mouse move "$X" "$Y" >/dev/null; sleep 0.4
[ "$(js "(ST.stuFilm.outlines.hover && ST.stuFilm.outlines.hover.pt.name) || ''")" = globe2 ] && ok "hovering the Scene list outlines the part" || bad "list hover"

# 10. a pattern's step grid follows the playhead
js "var r=SpacetimeLocal.stuFilmRows.find(r=>r.kind=='pattern'); SpacetimeLocal.stuFilmSel=r.key; ST.stuFilm.seek(9000); 1" >/dev/null; sleep 0.6
[ "$(js "document.querySelectorAll('.stu-cell.is-now').length")" -ge 3 ] && [ -n "$(js "SpacetimeLocal.stuFilmSoundNow.indexOf('step 5/16')>=0 ? 'y' : ''")" ] && ok "the step grid lights the step being played (step 5/16 at 9 s)" || bad "step playhead: $(js "SpacetimeLocal.stuFilmSoundNow")"

# 11. the note picker: a chord is edited from the keyboard
js "var r=SpacetimeLocal.stuFilmRows.find(r=>r.label.indexOf('--pad')>=0); SpacetimeLocal.stuFilmSel=r.key; 1" >/dev/null; sleep 0.6
read -r X Y <<<"$(js "var r=document.querySelector('.stu-pkey[data-note=\"D4\"]').getBoundingClientRect(); Math.round(r.x+r.width/2)+' '+Math.round(r.y+r.height*0.8)")"
mark; click_at "$X" "$Y"
live "SpacetimeLocal.stuScore.scores[0].entries.some(e=>e.notes && e.notes.text.indexOf('D4')>=0)" && ok "a key adds its note to the chord, live" || bad "a key adds its note to the chord"
grep -q '@sound(--pad, notes: "C3 Eb3 G3 Bb3 D4")' "$SRC" && ok "the chord's notes word is rewritten" || bad "chord: $(grep -n '@sound(--pad' "$SRC")"

# 12. keys
js "document.activeElement && document.activeElement.blur(); ST.stuFilm.transport().pause(); 1" >/dev/null
agent-browser press Space >/dev/null; sleep 0.3
[ "$(js "ST.stuFilm.transport().playing")" = true ] && ok "Space plays" || bad "Space plays"
agent-browser press Space >/dev/null

# 13. the inspector follows a film selection (PLAN-156 P0)
js "ST.stuFilm.select('stop:0:0:2'); 1" >/dev/null; sleep 0.6
[ "$(js "document.querySelector('.stu-inspector').dataset.stState+' | '+document.querySelector('.stu-inspector__label').textContent+' | '+getComputedStyle(document.querySelector('.stu-sec--empty')).display")" = "Film | Keyframe 3 of posZ | none" ] \
  && ok "a keyframe selection heads the inspector, no empty-state text" || bad "inspector: $(js "document.querySelector('.stu-inspector').dataset.stState+' | '+document.querySelector('.stu-inspector__label').textContent")"
[ "$(js "document.querySelectorAll('.stu-key[data-selected=true]').length+' '+document.querySelector('.stu-key[data-selected=true]').dataset.key+' '+document.querySelector('.stu-lab[data-selected=true]').dataset.key")" = "1 stop:0:0:2 chan:0:0" ] \
  && ok "the selected keyframe and its row are marked" || bad "marks: $(js "document.querySelectorAll('.stu-key[data-selected=true]').length")"
# a page selection replaces the film's
js "ST.stu.select(document.querySelector('.card h1')); 1" >/dev/null; sleep 0.4
[ "$(js "SpacetimeLocal.stuFilmSel+'|'+document.querySelector('.stu-inspector').dataset.stState")" = "|Element" ] && ok "a page selection clears the film's" || bad "exclusive: $(js "SpacetimeLocal.stuFilmSel+'|'+document.querySelector('.stu-inspector').dataset.stState")"
js "ST.stuFilm.select('clip:0'); 1" >/dev/null; sleep 0.3
[ "$(js "String(SpacetimeLocal.stuSel)+'|'+SpacetimeLocal.stuFilmSel")" = "null|clip:0" ] && ok "a film selection clears the page's" || bad "exclusive back: $(js "String(SpacetimeLocal.stuSel)+'|'+SpacetimeLocal.stuFilmSel")"
# Escape leaves the selection
js "document.activeElement && document.activeElement.blur(); 1" >/dev/null
agent-browser press Escape >/dev/null; sleep 0.3
[ "$(js "SpacetimeLocal.stuFilmSel")" = "" ] && ok "Escape clears the film selection" || bad "Escape: $(js "SpacetimeLocal.stuFilmSel")"
# I marks the loop in the film; it doesn't switch the page studio to Interact
agent-browser press i >/dev/null; sleep 0.2
[ "$(js "SpacetimeLocal.stuToolName+' '+(ST.stuFilm.loopIn!=null)")" = "Select true" ] && ok "I sets the loop's in point, the tool stays Select" || bad "I: $(js "SpacetimeLocal.stuToolName+' '+ST.stuFilm.loopIn")"
agent-browser press Escape >/dev/null
# closing and reopening on a film parked at its end shows its first frame
js "ST.stuFilm.transport().pause(); ST.stuFilm.seek(ST.stuFilm.transport().duration); document.querySelector('.stu-bar__close').click(); 1" >/dev/null; sleep 0.5
js "document.querySelector('.st-studio__pill').click(); 1" >/dev/null; sleep 1
[ "$(js "Math.round(ST.stuFilm.transport().position)")" = 0 ] && ok "reopening at the film's end rewinds to 0" || bad "rewind: $(js "Math.round(ST.stuFilm.transport().position)")"
# the film's last second is in view: the lanes end before the panel's edge
[ "$(js "var l=document.querySelector('.stu-film__lanes').getBoundingClientRect(), b=document.querySelector('.stu-film__body--timeline'); (l.right <= b.getBoundingClientRect().left + b.clientWidth - 8) ? 'y' : 'n'")" = y ] && ok "the timeline's end is inside the panel" || bad "lanes run to the edge"

# 14. undo: the last edit comes back out of the source (PLAN-156 P1, the rewrite journal)
# an edit through the one writer, addressed from the file as it is now
AT=$(python3 -c "import sys; print(len(open(sys.argv[1],encoding='utf-8').read().split('scale: 0.42')[0].encode())+7)" "$SRC")
curl -s -X POST "http://127.0.0.1:$PORT/__spacetime/rewrite" -H 'Content-Type: application/json' \
  -d "{\"entry\":\"index.st\",\"edits\":[{\"file\":\"index.st\",\"start\":$AT,\"end\":$((AT + 4)),\"expect\":\"0.42\",\"text\":\"0.5\"}]}" >/dev/null
for _ in $(seq 1 60); do grep -q 'scale: 0.5,' "$SRC" && break; sleep 0.2; done
grep -q 'scale: 0.5,' "$SRC" && ok "an edit writes (scale 0.42 -> 0.5)" || bad "edit before undo: $(grep -o 'scale: [0-9.]*' "$SRC" | head -1)"
# the page has taken the edit (in place or by a reload) and the studio has read it
for _ in $(seq 1 100); do
  [ "$(js "(window.SpacetimeLocal && SpacetimeLocal.stuScore && SpacetimeLocal.stuScore.sources[0].text.indexOf('scale: 0.5,') >= 0 && !ST.stu.inflight) ? 'y' : 'n'")" = y ] && break; sleep 0.2
done
ready >/dev/null; sleep 0.5
js "document.activeElement && document.activeElement.blur(); 1" >/dev/null
agent-browser press Control+z >/dev/null
for _ in $(seq 1 60); do grep -q 'scale: 0.42,' "$SRC" && break; sleep 0.2; done
grep -q 'scale: 0.42,' "$SRC" && ok "Ctrl+Z puts the source back (scale 0.42)" || bad "undo: $(grep -o 'scale: [0-9.]*' "$SRC" | head -1)"
# reflect: the program as every tool reads it
[ "$(curl -s "http://127.0.0.1:$PORT/__spacetime/reflect.json?entry=index.st" | python3 -c "import json,sys; d=json.load(sys.stdin); m=[m for m in d['matches'] if m['macro']=='gltf'][0]; p={p['name']:p for p in m['params']}; print(p['scale']['type'], p['scale']['written']['text'], p['spinY']['written'], p['spinY']['default'])")" = "number 0.42 None 0" ] \
  && ok "reflect.json gives the robot's declared params, written and defaulted" || bad "reflect.json"

# 15. typed fields (PLAN-156): each param's widget comes from the type its form declares
part() {
  js "var i=SpacetimeLocal.stuFilmParts.findIndex(p=>p.label=='$1'); ST.stuFilm.select('part:'+i); 1" >/dev/null
  # its fields are on the page (reflect read, widgets rendered)
  for _ in $(seq 1 50); do [ "$(js "document.querySelectorAll('.stu-part-fields .w-field').length > 3")" = true ] && break; sleep 0.2; done
  sleep 0.5
}
part '&robot'
[ "$(js "var f=document.querySelector('.stu-part-fields .w-field[data-label=clip]'); f.dataset.kind+' '+f.querySelectorAll('.w-choice').length+' '+f.querySelector('.w-choice[data-on=true]').dataset.word")" = "choice 14 Idle" ] \
  && ok "the robot's clip is a choice of robot.glb's 14 clips" || bad "clip field: $(js "var f=document.querySelector('.stu-part-fields .w-field[data-label=clip]'); f && f.outerHTML.slice(0,200)")"
# the score drives the robot's clip, so a pick keys a stop at the playhead (2 s = 8.3%)
js "ST.stuFilm.transport().pause(); ST.stuFilm.seek(2000); 1" >/dev/null; sleep 0.4
# (a click that lands while the fields re-render is lost: try again)
for _ in 1 2 3; do
  js "[...document.querySelectorAll('.stu-part-fields .w-field[data-label=clip] .w-choice')].find(b=>b.dataset.word=='Wave').click(); 1" >/dev/null
  for _ in $(seq 1 20); do grep -q '"Idle" -> "Wave" at 8.3%' "$SRC" && break; sleep 0.2; done
  grep -q '"Idle" -> "Wave" at 8.3%' "$SRC" && break
done
grep -q '"Idle" -> "Wave" at 8.3%' "$SRC" && ok "picking a clip of a driven model keys it at the playhead, quoted as written" || bad "clip pick: $(grep -o '&robot *{ clip:[^;]*' "$SRC" | cut -c1-80)"
true || bad "clip pick: $(grep -o '@gltf &robot[^)]*' "$SRC") | status: $(js "SpacetimeLocal.stuStatus+' / '+SpacetimeLocal.stuError")"
sleep 1; ready >/dev/null
part '&sun'
[ "$(js "var f=document.querySelector('.stu-part-fields .w-field[data-label=type]'); f.dataset.kind+' '+[...f.querySelectorAll('.w-choice')].map(b=>b.dataset.word).join(',')")" = "choice ambient,directional,point,spot" ] \
  && ok "a light's type is its keyword set" || bad "type field"
[ "$(js "var f=document.querySelector('.stu-part-fields .w-field[data-label=penumbra]'); f.dataset.kind+' '+f.querySelector('.w-slider').min+' '+f.querySelector('.w-slider').max+' '+f.hasAttribute('data-written')")" = "range 0 1 false" ] \
  && ok "penumbra is a 0-1 slider, at its default" || bad "penumbra field"
[ "$(js "document.querySelector('.stu-part-fields .w-field[data-label=type] .w-field__label').title")" = "how it shines" ] && ok "a param's /// doc is its label's tooltip" || bad "doc tooltip"
# scrub: drag the sun's X label to the right (intensity is driven: it would key a stop)
LXY=$(centre "document.querySelector('.stu-part-fields .w-part[data-label=posX] .w-field__label')")
drag_by ${LXY% *} ${LXY#* } 40
for _ in $(seq 1 60); do grep -q '@light &sun ([^)]*posX: -6,' "$SRC" || break; sleep 0.2; done
NOW=$(grep -o '@light &sun ([^)]*posX: [-0-9.]*' "$SRC" | sed 's/.*posX: //')
[ "$NOW" != "-6" ] && [ -n "$NOW" ] && ok "scrubbing a label writes the number (posX -6 -> $NOW)" || bad "scrubbing posX didn't write ($NOW)"

# 16. a shorthand's longhands share a row; a driven param keys the stop at the playhead (PLAN-156 P6)
part '&lamp2'
[ "$(js "var g=document.querySelector('.stu-part-fields .w-group[data-kind=vec3]'); g.querySelector('.w-field__label').textContent+' '+[...g.querySelectorAll('.w-part')].map(p=>p.dataset.label).join(',')")" = "pos posX,posY,posZ" ] \
  && ok "posX/Y/Z share one pos row" || bad "vec3 row"
[ "$(js "document.querySelector('.stu-part-fields .w-field[data-label=intensity]').dataset.drive != ''")" = true ] && ok "a param the score animates is marked driven" || bad "driven mark"
# at 14.2 s, set intensity to 3: a stop at that moment, not the rest value
js "ST.stuFilm.transport().pause(); ST.stuFilm.seek(14200); 1" >/dev/null; sleep 0.5
js "var x=document.querySelector('.stu-part-fields .w-field[data-label=intensity] input.w-number'); x.value='3'; x.dispatchEvent(new Event('change',{bubbles:true})); 1" >/dev/null
for _ in $(seq 1 60); do grep -q '&lamp2 *{[^}]*-> 3 at 5[0-9.]*%' "$SRC" && break; sleep 0.2; done
grep -q '&lamp2 *{[^}]*-> 3 at 5[0-9.]*%' "$SRC" && ok "editing a driven param keys a stop at the playhead ($(grep -o -- '-> 3 at 5[0-9.]*%' "$SRC" | head -1))" || bad "driven edit: $(grep -o '&lamp2 *{[^}]*' "$SRC" | head -1)"
grep -q '@light &lamp2 (type: "point", intensity: 0,' "$SRC" && ok "the rest value is untouched" || bad "rest value changed"

# 17. the timeline zooms about the cursor, fits, and box-selects keyframes (PLAN-156 P6/P7)
js "document.querySelector('.stu-film__tab[data-panel=Timeline]').click(); ST.stuFilm.select(''); ST.stuFilm.fit(); 1" >/dev/null; sleep 0.3
# the moment under the cursor stays put while zooming in with Ctrl+wheel
read ZX ZY <<<"$(js "var r=document.querySelector('.stu-film__lanes').getBoundingClientRect(); Math.round(r.left+r.width*0.6)+' '+Math.round(r.top+60)")"
B4=$(js "var r=document.querySelector('.stu-film__lanes').getBoundingClientRect(); Math.round(ST.stuFilm.view.from + ($ZX-r.left)/r.width*(ST.stuFilm.view.span||24000))")
js "var l=document.querySelector('.stu-film__lanes'); for (var i=0;i<6;i++) l.dispatchEvent(new WheelEvent('wheel',{deltaY:-100,ctrlKey:true,clientX:$ZX,clientY:$ZY,bubbles:true,cancelable:true})); 1" >/dev/null; sleep 0.3
AF=$(js "var r=document.querySelector('.stu-film__lanes').getBoundingClientRect(); Math.round(ST.stuFilm.view.from + ($ZX-r.left)/r.width*ST.stuFilm.view.span)")
[ "$(js "SpacetimeLocal.stuFilmZoom > 3")" = true ] && [ $(( AF > B4 ? AF - B4 : B4 - AF )) -lt 60 ] \
  && ok "Ctrl+wheel zooms in about the cursor (x$(js "SpacetimeLocal.stuFilmZoom"), ${B4} ms stays under it)" || bad "zoom: $(js "SpacetimeLocal.stuFilmZoom") $B4 -> $AF"
js "document.querySelector('.stu-film__fit').click(); 1" >/dev/null; sleep 0.2
[ "$(js "SpacetimeLocal.stuFilmZoom")" = 1 ] && ok "Fit shows the whole film" || bad "fit: $(js "SpacetimeLocal.stuFilmZoom")"
# zoom to 14.8-16.3 s, then box the camera posX keys at 64% / 64.6% / 65.4% (15.4-15.7 s)
js "ST.stuFilm.setView(14800, 1500); 1" >/dev/null; sleep 0.4
read X1 Y1 X2 Y2 <<<"$(js "var ks=SpacetimeLocal.stuFilmKeys.filter(k=>k.key.indexOf('stop:0:2:')===0).map(k=>document.querySelector('.stu-key[data-key=\"'+k.key+'\"]').getBoundingClientRect()), L=document.querySelector('.stu-film__lanes').getBoundingClientRect(); ks=ks.filter(b=>b.left>L.left+4&&b.right<L.right-4); var xs=ks.map(b=>b.left), y=ks[0].top; [Math.min(...xs)-8, y-6, Math.max(...xs)+18, y+16].map(Math.round).join(' ')")"
agent-browser mouse move $X1 $Y1 >/dev/null; agent-browser mouse down >/dev/null; agent-browser mouse move $(( (X1+X2)/2 )) $(( (Y1+Y2)/2 )) >/dev/null; agent-browser mouse move $X2 $Y2 >/dev/null; agent-browser mouse up >/dev/null; sleep 0.3
N=$(js "SpacetimeLocal.stuFilmMulti.split(' ').filter(Boolean).length")
[ "$N" -ge 3 ] && ok "a box drag selects the keyframes inside it ($N)" || bad "marquee: $(js "SpacetimeLocal.stuFilmMulti")"
BEFORE=$(grep -c 'at 64%' "$SRC")
KXY=$(centre "document.querySelector('.stu-key[data-key=\"'+SpacetimeLocal.stuFilmMulti.split(' ')[0]+'\"]')")
drag_by ${KXY% *} ${KXY#* } 30
for _ in $(seq 1 60); do [ "$(grep -o '&camera[^}]*posX:[^;]*' "$SRC" | grep -c 'at 64%')" = 0 ] && break; sleep 0.2; done
[ "$(grep -o 'posX: 0 -> 0.12 at [0-9.]*% -> -0.1 at [0-9.]*% -> 0 at [0-9.]*%' "$SRC" | grep -c 'at 64%')" = 0 ] \
  && ok "dragging one of them moves all three, in one write ($(grep -o 'posX: 0 -> 0.12 at [0-9.]*% -> -0.1 at [0-9.]*% -> 0 at [0-9.]*%' "$SRC" | head -1))" || bad "multi move: $(grep -o '&camera[^}]*' "$SRC" | head -1 | cut -c1-200)"

# 18. the editor camera and the move tool (PLAN-156 P8)
js "ST.stuFilm.fit(); document.activeElement && document.activeElement.blur(); 1" >/dev/null
agent-browser press c >/dev/null; sleep 0.4
[ "$(js "var st=ST.stuFilm.stage(); SpacetimeLocal.stuFilmCam+' '+(st.looking()!==st.camera)")" = "Editor camera true" ] && ok "C sees the 3D through an editor camera" || bad "editor camera: $(js "SpacetimeLocal.stuFilmCam")"
FILMCAM=$(js "ST.stuFilm.stage().camera.position.toArray().map(v=>v.toFixed(3)).join(',')")
ED0=$(js "ST.stuFilm.ed.cam.position.toArray().map(v=>v.toFixed(3)).join(',')")
CXY=$(centre "document.querySelector('.stu-3d-hit')")
drag_by ${CXY% *} ${CXY#* } 120; sleep 0.3
ED1=$(js "ST.stuFilm.ed.cam.position.toArray().map(v=>v.toFixed(3)).join(',')")
[ "$ED0" != "$ED1" ] && [ "$(js "ST.stuFilm.stage().camera.position.toArray().map(v=>v.toFixed(3)).join(',')")" = "$FILMCAM" ] \
  && ok "dragging orbits the editor camera; the film's camera stays where the score puts it" || bad "orbit: $ED0 -> $ED1"
# move a globe (its position isn't animated): select it, M, drag it on screen
part '&globe1'
agent-browser press f >/dev/null; sleep 0.4
agent-browser press m >/dev/null; sleep 0.2
read GX GY <<<"$(js "var st=ST.stuFilm.stage(), i=SpacetimeLocal.stuFilmParts.findIndex(p=>p.label=='&globe1'), pt=ST.stuFilm.stagePart(SpacetimeLocal.stuScore,i), r=st.renderer.domElement.getBoundingClientRect(), v=pt.obj.getWorldPosition(new st.THREE.Vector3()).project(st.looking()); Math.round(r.left+(v.x+1)/2*r.width)+' '+Math.round(r.top+(1-v.y)/2*r.height)")"
drag_by $GX $GY 60
for _ in $(seq 1 60); do grep -q '@object &globe1 ([^)]*posX: -2.6,' "$SRC" || break; sleep 0.2; done
NEWX=$(grep -o '@object &globe1 ([^)]*' "$SRC" | grep -o 'posX: [-0-9.]*' | sed 's/posX: //')
[ -n "$NEWX" ] && [ "$NEWX" != "-2.6" ] && ok "dragging a part with Move writes its position (globe1 posX -2.6 -> $NEWX)" || bad "move: $(grep -o '@object &globe1 ([^)]*' "$SRC")"
agent-browser press m >/dev/null; agent-browser press c >/dev/null

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
