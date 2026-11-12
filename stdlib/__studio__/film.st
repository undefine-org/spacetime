/// Film mode (PLAN-155): the studio, for a page with a `@score`.
///
/// Opens with the studio when the page is a film. Everything in it is drawn
/// from the film's own source (`/__spacetime/studio/score.json`, where every
/// word carries its span) and every gesture rewrites one of those words:
///
/// - the TRANSPORT: play / pause (Space, K, L), J back a second, arrows a
///   frame (Shift: a second), Home / End, I / O a loop, Esc clears it;
/// - the TIMELINE: a ruler to scrub, a bar per clip (drag it: its `at`; drag
///   its edge: its `for`), the clip's channels with a diamond per keyframe
///   (drag it: its `at p%`), and a lane per sound with its waveform;
/// - the INSPECTOR's Film section: the selected clip / keyframe / sound /
///   part as the words that make it, editable, and a pattern's step grid;
/// - the explorer's SCENE tab: the stage and its parts, every param editable
///   (a default param is written in when changed);
/// - CODE: the film's source, saved whole (Ctrl/Cmd+S or leaving it).
///
/// No JavaScript here: behaviour lives in ./primitives/film-kit.st.
@import "./primitives/film-kit.st";
// the field widgets the inspector renders (PLAN-156), shared with the admin
@import "../widgets/index.st";

@data fetch $stuScore : "/__spacetime/studio/score.json?entry=__SPACETIME_STUDIO_ENTRY__";
// the program as every tool reads it: a part's typed, documented params (PLAN-156)
@data fetch $stuReflect : "/__spacetime/reflect.json?entry=__SPACETIME_STUDIO_ENTRY__";
@data inline $stuFilmSel string : "";
@data inline $stuFilmTime string : "00:00:00";
@data inline $stuFilmPlaying bool : false;
@data inline $stuFilmLoop string : "";
@data stored $stuFilmPanelName string : "Timeline";
// the Select tool (pick a part in the 3D), the melodic cell the keyboard edits
// ("row:col"), and where the playhead is in the selected sound
@data inline $stuFilmPickOn bool : false;
@data inline $stuFilmNoteCell string : "";
@data inline $stuFilmSoundNow string : "";
// the view window (PLAN-156 P7): how far the timeline is zoomed in, and what it shows
@data inline $stuFilmZoom number : 1;
@data inline $stuFilmView string : "";
// the keyframes a box drag holds, space-separated stop keys (PLAN-156 P6)
@data inline $stuFilmMulti string : "";
// the camera the 3D is seen through, and the move tool (PLAN-156 P8)
@data inline $stuFilmCam string : "Film camera";
@data inline $stuFilmMoveOn bool : false;

@type StuFilmPanel { Timeline | Code }
@data derive $stuFilmPanel StuFilmPanel : @match {
  ($stuFilmPanelName == "Code") => Code;
  _                             => Timeline;
}

@data derive $stuFilmRows array : ST.stuFilmRows($stuScore, $stuFilmSel);
@data derive $stuFilmKeys array : ST.stuFilmKeys($stuScore, $stuFilmSel, $stuFilmMulti);
@data derive $stuFilmTicks array : ST.stuFilmTicks($stuScore, $stuFilmZoom);
@data derive $stuFilmFields array : ST.stuFilmFields($stuFilmSel, $stuScore);
@data derive $stuPartFields array : ST.stuPartFields($stuFilmSel, $stuScore, $stuReflect);
@data derive $stuFilmGrid array : ST.stuFilmGrid($stuFilmSel, $stuScore);
@data derive $stuFilmParts array : ST.stuFilmParts($stuScore, $stuFilmSel);
@data derive $stuFilmPiano array : ST.stuFilmPiano($stuFilmSel, $stuScore, $stuFilmNoteCell);
@data derive $stuFilmSrcs array : ST.stuFilmSources($stuScore);
@data derive $stuFilmSelLabel string : ST.stuFilmLabel($stuFilmSel, $stuScore);
@data derive $stuFilmTitle string : (($stuScore && $stuScore.scores && $stuScore.scores[0]) ? ("&" + ($stuScore.scores[0].name || "score") + "  " + ($stuScore.scores[0].total ? $stuScore.scores[0].total.text : "")) : "");

// ---- bindings ------------------------------------------------------------------------

.stu-film {
  @film-panel(on: $stuOpen, shape: $stuScore);
  @state(when: $stuFilmPanel is Timeline) { --stu-film-panel: 0; }
  @state(when: $stuFilmPanel is Code)     { --stu-film-panel: 1; }
}
.stu-film__title { text <- $stuFilmTitle; }
.stu-film__time { text <- $stuFilmTime; }
.stu-film__loop { text <- $stuFilmLoop; }
.stu-film__view { .is-hidden: $stuFilmZoom <= 1; text <- $stuFilmView; }
.stu-film__fit { @on &.click { $stuNoop <- ST.stuFilmFit(); } }
.stu-film__cam { text <- $stuFilmCam; @on &.click { $stuNoop <- ST.stuFilmCamToggle(); } }
.stu-film__move { .is-on: $stuFilmMoveOn; @on &.click { $stuNoop <- ST.stuFilmMoveToggle(); } }
.stu-film__play {
  .is-playing: $stuFilmPlaying;
  @on &.click { $stuNoop <- ST.stuFilmToggle(); }
}
.stu-film__go { @on &.click { $stuNoop <- ST.stuFilmGo($); } }
.stu-film__tab { @on &.click { $stuFilmPanelName <- $.dataset.panel; } }
.stu-film__pick {
  .is-on: $stuFilmPickOn;
  @on &.click { $stuNoop <- ST.stuFilmPickTool(); }
}
.stu-film-now { .is-hidden: $stuFilmSoundNow == ""; text <- $stuFilmSoundNow; }

// THE NOTE PICKER: one key of a keyboard, drawn from the selection (a chord's
// notes, or the melodic cell picked in the step grid). A key that is on is in
// the chord / is the cell's note; pressing it toggles that.
@template &piano-key($k) {
  <button type="button" class="stu-pkey" data-note="`$k.note`" data-black="`$k.black`" data-on="`$k.on`" data-target="`$k.target`" title="`$k.note`" style="--stu-i: `$k.i`"><span class="stu-pkey__label">`$k.label`</span></button>
}
.stu-piano {
  @each($stuFilmPiano as $k) {
    &piano-key($k);
  }
}
.stu-pkey { @on &.click { $stuNoop <- ST.stuFilmNote($); } }

.stu-film__ticks {
  @each($stuFilmTicks as $k) {
    <span class="stu-tick" data-major="`$k.major`" style="left: `$k.left`%">`$k.label`</span>
  }
}
.stu-film__labels {
  @each($stuFilmRows as $r) {
    <button type="button" class="stu-lab" data-film="row" data-key="`$r.key`" data-kind="`$r.kind`" data-selected="`$r.selected`" style="--stu-depth: `$r.depth`">`$r.label`</button>
  }
}
.stu-film__bars {
  @each($stuFilmRows as $r) {
    <div class="stu-bar-clip" data-film="clip" data-key="`$r.key`" data-kind="`$r.kind`" data-selected="`$r.selected`" style="--stu-y: `$r.y`; left: `$r.left`%; width: `$r.width`%"><span class="stu-bar-clip__text">`$r.text`</span><canvas class="stu-wave" data-entry="`$r.entry`"></canvas><span class="stu-bar-clip__edge" data-film="edge" data-key="`$r.key`"></span></div>
  }
}
.stu-film__keys {
  @each($stuFilmKeys as $k) {
    <span class="stu-key" data-film="key" data-key="`$k.key`" data-held="`$k.held`" data-fixed="`$k.fixed`" data-selected="`$k.selected`" title="`$k.value`" style="--stu-y: `$k.y`; left: `$k.left`%"></span>
  }
}
.stu-film__code {
  @each($stuFilmSrcs as $src) {
    <textarea class="stu-code" spellcheck="false" data-path="`$src.path`" data-hash="`$src.hash`">`$src.text`</textarea>
  }
}
.stu-code { @on &.change { $stuNoop <- ST.stuFilmSave($); } }

.stu-film-fields {
  @each($stuFilmFields as $f) {
    <label class="stu-field" data-kind="film"><span class="stu-field__key">`$f.label`</span><input class="stu-film-input" type="text" value="`$f.value`" data-mode="`$f.mode`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" spellcheck="false" /><span class="stu-field__hint">`$f.hint`</span></label>
  }
}
.stu-film-input { @on &.change { $stuNoop <- ST.stuFilmCommit($); } }
// a part's params: one widget per type, picked by the field (PLAN-156)
.stu-part-fields {
  @each($stuPartFields as $f) { &$f.tpl($f); }
}
.stu-part-fields .w-input { @on &.change { $stuNoop <- ST.stuWidgetCommit($); } }
.stu-part-fields .w-choice { @on &.click { $stuNoop <- ST.stuWidgetCommit($); } }
.stu-grid {
  @each($stuFilmGrid as $c) {
    <button type="button" class="stu-cell" data-kind="`$c.kind`" data-on="`$c.on`" data-text="`$c.text`" data-step="`$c.step`" data-rc="`$c.rc`" data-melodic="`$c.melodic`" data-file="`$c.file`" data-start="`$c.start`" data-end="`$c.end`" title="`$c.text`" style="grid-row: `$c.row`; grid-column: `$c.col`">`$c.text`</button>
  }
}
.stu-cell { @on &.click { $stuNoop <- ST.stuFilmCell($); } }

.stu-tree--scene {
  @each($stuFilmParts as $p) {
    <button type="button" class="stu-node stu-part" data-key="`$p.key`" data-kind="`$p.kind`" data-selected="`$p.selected`" style="--stu-depth: `$p.depth`"><span class="stu-node__mark"></span><span class="stu-node__label">`$p.label`</span><span class="stu-node__text">`$p.text`</span></button>
  }
}
.stu-part { @on &.click { $stuNoop <- ST.stuFilmPick($); } }

// ---- look: a timeline like an editor's ---------------------------------------------

html.stu-film-on { --stu-film-h: 248px; }
.st-studio { --stu-film-h: 248px; --stu-row: 22px; --stu-film-head: 128px; --stu-film-bg: #17171a; --stu-film-line: #2a2a30; --stu-film-ink: #e8e8ec; --stu-film-soft: #8b8b96; }
.stu-film { display: none; }
html.stu-film-on .stu-film { display: flex; }
html.stu-film-on body {
  position: fixed !important; top: 0; left: var(--stu-left); right: var(--stu-right); bottom: var(--stu-film-h);
  margin: 0 !important; transform: translateZ(0); overflow: hidden;
}
html.stu-film-on .st-dev-dock { bottom: calc(var(--stu-film-h) + 12px) !important; }

.stu-film {
  position: fixed;
  left: var(--stu-left);
  right: var(--stu-right);
  bottom: 0;
  height: var(--stu-film-h);
  flex-direction: column;
  background: var(--stu-film-bg);
  color: var(--stu-film-ink);
  font: 11px/1.4 Inter, -apple-system, sans-serif;
  border-top: 1px solid #000;
  z-index: 2147483000;
  user-select: none;
}
.stu-film__bar { display: flex; align-items: center; gap: 6px; height: 34px; padding: 0 10px; border-bottom: 1px solid var(--stu-film-line); flex: none; }
.stu-film__title { margin: 0 10px 0 0; font-weight: 600; color: var(--stu-film-soft); white-space: nowrap; }
.stu-film__go, .stu-film__play, .stu-film__tab, .stu-film__fit, .stu-film__cam, .stu-film__move {
  height: 24px; min-width: 26px; padding: 0 8px; border: 0; border-radius: 5px;
  background: #26262c; color: var(--stu-film-ink); font: inherit; cursor: pointer;
}
.stu-film__go:hover, .stu-film__tab:hover, .stu-film__fit:hover { background: #303038; }
.stu-film__play { min-width: 44px; background: #0d99ff; color: #fff; font-weight: 600; }
.stu-film__play::after { content: "Play"; }
.stu-film__play.is-playing::after { content: "Pause"; }
.stu-film__time { font: 600 13px/1 ui-monospace, "SF Mono", Menlo, monospace; letter-spacing: 0.04em; padding: 0 8px; min-width: 9em; }
.stu-film__loop { color: #ffcf5a; font: 11px ui-monospace, Menlo, monospace; }
.stu-film__view { color: var(--stu-film-soft); font: 11px ui-monospace, Menlo, monospace; }
.stu-film__move.is-on { background: #0d99ff; color: #fff; }
/* the 3D tools' hit layer: over the stage's canvas (above the film's own
   titles), under the studio's panels */
.stu-3d-hit { display: none; position: fixed; z-index: 2147482000; touch-action: none; }
html.stu-3d-tools .stu-3d-hit { display: block; }
html.stu-moving .stu-3d-hit { cursor: move; }
.stu-film__view.is-hidden { display: none; }
.stu-film__marquee { position: absolute; display: none; border: 1px solid #0d99ff; background: rgba(13, 153, 255, 0.12); pointer-events: none; z-index: 4; }
.stu-film__marquee.is-on { display: block; }
.stu-film__spacer { flex: 1; }
.stu-film[data-st-state="Timeline"] .stu-film__tab[data-panel="Timeline"],
.stu-film[data-st-state="Code"] .stu-film__tab[data-panel="Code"] { background: #3a3a44; }

.stu-film__body { display: none; flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; }
.stu-film[data-st-state="Timeline"] .stu-film__body--timeline { display: grid; grid-template-columns: var(--stu-film-head) 1fr; }
.stu-film[data-st-state="Code"] .stu-film__body--code { display: flex; }

.stu-film__labels { display: flex; flex-direction: column; padding-top: 22px; border-right: 1px solid var(--stu-film-line); }
.stu-lab {
  height: var(--stu-row); flex: none; text-align: left; border: 0; background: transparent; color: var(--stu-film-ink);
  font: inherit; padding: 0 8px 0 calc(8px + var(--stu-depth) * 12px); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; cursor: pointer;
}
.stu-lab[data-kind="channel"] { color: var(--stu-film-soft); }
.stu-lab:hover { background: #222228; }

/* the lanes stop short of the panel edge, so the film's last second (its last
   tick, a clip that ends there, the playhead parked at the end) stays in view */
.stu-film__lanes { position: relative; min-height: 100%; margin: 0 16px 0 6px; overflow-x: clip; }
/* the view window: the track is the film widened by --stu-zoom and shifted by
   --stu-from (fractions of the film), so zoom and pan move no data */
.stu-film__ruler, .stu-film__bars, .stu-film__keys {
  width: calc(var(--stu-zoom, 1) * 100%);
  transform: translateX(calc(var(--stu-from, 0) * -100%));
}
.stu-film__ruler { position: sticky; top: 0; height: 22px; background: #1d1d22; border-bottom: 1px solid var(--stu-film-line); cursor: ew-resize; z-index: 2; }
.stu-film__ticks { position: absolute; inset: 0; pointer-events: none; }
.stu-tick { position: absolute; top: 0; bottom: 0; border-left: 1px solid #34343c; padding-left: 3px; color: var(--stu-film-soft); font-size: 10px; line-height: 22px; }
.stu-tick[data-major="false"] { top: 14px; }
.stu-film__bars, .stu-film__keys { position: absolute; left: 0; top: 22px; bottom: 0; }
.stu-film__keys { pointer-events: none; }

.stu-bar-clip {
  position: absolute; top: calc(var(--stu-y) * var(--stu-row) + 3px); height: calc(var(--stu-row) - 6px);
  transform: translateX(var(--stu-dx, 0px)); border-radius: 4px; background: #3b5bdb; color: #fff;
  overflow: hidden; cursor: grab; box-sizing: border-box; padding: 0 6px;
}
.stu-bar-clip[data-kind="channel"] { background: #2b2b33; cursor: default; }
.stu-bar-clip[data-kind="sound"] { background: #2f7d5b; }
.stu-bar-clip[data-kind="pattern"] { background: #8a5cf6; }
.stu-bar-clip__text { position: relative; z-index: 1; font-size: 10px; line-height: calc(var(--stu-row) - 6px); white-space: nowrap; pointer-events: none; }
.stu-bar-clip[data-kind="channel"] .stu-bar-clip__text { color: var(--stu-film-soft); }
.stu-wave { position: absolute; inset: 0; width: 100%; height: 100%; color: rgba(255, 255, 255, 0.55); pointer-events: none; display: none; }
.stu-bar-clip[data-kind="sound"] .stu-wave, .stu-bar-clip[data-kind="pattern"] .stu-wave { display: block; }
.stu-bar-clip__edge { position: absolute; right: 0; top: 0; bottom: 0; width: 6px; cursor: ew-resize; background: rgba(255, 255, 255, 0.25); }
.stu-bar-clip[data-kind="channel"] .stu-bar-clip__edge { display: none; }

.stu-key {
  position: absolute; top: calc(var(--stu-y) * var(--stu-row) + var(--stu-row) / 2 - 5px); width: 10px; height: 10px; margin-left: -5px;
  background: #ffcf5a; transform: translateX(var(--stu-dx, 0px)) rotate(45deg); border-radius: 1px; pointer-events: auto; cursor: ew-resize;
}
.stu-key[data-held="true"] { background: #ff8a5a; border-radius: 0; }
.stu-key[data-fixed="true"] { background: #b8b8c4; }
.stu-key[data-selected="true"] { background: #fff; box-shadow: 0 0 0 2px #0d99ff; z-index: 1; }
.stu-bar-clip[data-selected="true"] { box-shadow: inset 0 0 0 1px #0d99ff; }
.stu-lab[data-selected="true"] { background: #1f2a3a; color: #fff; }

.stu-film__playhead {
  position: absolute; top: 0; bottom: 0; left: calc((var(--stu-t, 0) - var(--stu-from, 0)) * var(--stu-zoom, 1) * 100%); width: 0;
  border-left: 1px solid #ff4d4d; pointer-events: none; z-index: 3;
}
.stu-film__playhead::before { content: ""; position: absolute; top: 0; left: -6px; border: 6px solid transparent; border-top-color: #ff4d4d; }

.stu-film__code { flex: 1; display: flex; }
.stu-code {
  flex: 1; margin: 0; border: 0; resize: none; padding: 10px 14px; background: #121215; color: #e8e8ec;
  font: 12px/1.55 ui-monospace, "SF Mono", Menlo, monospace; tab-size: 2; outline: none;
}

.stu-field__hint { display: block; color: var(--stu-soft); font-size: 10px; margin-top: 1px; }
.stu-film-input { width: 100%; box-sizing: border-box; border: 1px solid var(--stu-line); border-radius: 4px; padding: 4px 6px; font: 11px ui-monospace, Menlo, monospace; }
.stu-film-input:focus { outline: 1px solid var(--stu-blue); }
.stu-grid { display: grid; grid-template-columns: auto; grid-auto-columns: 13px; gap: 1px; margin-top: 10px; overflow-x: auto; }
.stu-cell[data-kind="name"] { width: auto; padding-right: 6px; background: transparent; color: var(--stu-soft); font: 10px ui-monospace, Menlo, monospace; text-align: left; cursor: default; white-space: nowrap; }
.stu-cell { width: 13px; height: 18px; padding: 0; border: 0; border-radius: 2px; background: #ececec; color: transparent; font-size: 0; cursor: pointer; }
.stu-cell[data-on="true"] { background: #8a5cf6; }
.stu-cell:nth-child(4n + 2) { box-shadow: inset 2px 0 0 #d0d0d0; }
// a melodic row shows its notes; the step being played lights up as the
// playhead passes it, and a hit flashes
.stu-cell[data-melodic="true"][data-kind="cell"] { font: 600 6px/18px ui-monospace, Menlo, monospace; color: #fff; letter-spacing: -0.04em; overflow: hidden; }
.stu-cell.is-now { outline: 2px solid #ff4d4d; outline-offset: -1px; }
.stu-cell.is-now[data-on="true"] { background: #ff4d4d; }
.stu-cell.is-picked { outline: 2px solid #0d99ff; outline-offset: 1px; }

.stu-film-now { margin: 8px 0 0; font: 600 11px ui-monospace, Menlo, monospace; color: #ff4d4d; }
.stu-film-now.is-hidden { display: none; }

// the keyboard: white keys in a row, black keys over their gaps (a template
// renders a true flag as a present attribute, a false one not at all)
.stu-piano { position: relative; height: 74px; margin-top: 10px; }
.stu-pkey {
  position: absolute; top: 0; left: calc(var(--stu-i) * 16px); width: 15px; height: 72px; padding: 0;
  border: 1px solid #c8c8c8; border-radius: 0 0 3px 3px; background: #fff; cursor: pointer; z-index: 1;
}
.stu-pkey[data-black] { left: calc(var(--stu-i) * 16px + 10px); width: 11px; height: 44px; background: #222; border-color: #111; z-index: 2; }
.stu-pkey[data-on] { background: #8a5cf6; border-color: #6b3fe0; }
.stu-pkey:hover { filter: brightness(0.92); }
.stu-pkey__label { position: absolute; bottom: 2px; left: 0; right: 0; font: 7px ui-monospace, Menlo, monospace; color: #888; text-align: center; }

// the Select tool
.stu-film__pick.is-on { background: #0d99ff; color: #fff; }
html.stu-picking body { cursor: crosshair; }
.stu-part[data-selected="true"] { background: var(--stu-blue-soft); }
