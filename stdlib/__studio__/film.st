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

@data fetch $stuScore : "/__spacetime/studio/score.json?entry=__SPACETIME_STUDIO_ENTRY__";
@data inline $stuFilmSel string : "";
@data inline $stuFilmTime string : "00:00:00";
@data inline $stuFilmPlaying bool : false;
@data inline $stuFilmLoop string : "";
@data stored $stuFilmPanelName string : "Timeline";

@type StuFilmPanel { Timeline | Code }
@data derive $stuFilmPanel StuFilmPanel : @match {
  ($stuFilmPanelName == "Code") => Code;
  _                             => Timeline;
}

@data derive $stuFilmRows array : ST.stuFilmRows($stuScore);
@data derive $stuFilmKeys array : ST.stuFilmKeys($stuScore);
@data derive $stuFilmTicks array : ST.stuFilmTicks($stuScore);
@data derive $stuFilmFields array : ST.stuFilmFields($stuFilmSel, $stuScore);
@data derive $stuFilmGrid array : ST.stuFilmGrid($stuFilmSel, $stuScore);
@data derive $stuFilmParts array : ST.stuFilmParts($stuScore);
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
.stu-film__play {
  .is-playing: $stuFilmPlaying;
  @on &.click { $stuNoop <- ST.stuFilmToggle(); }
}
.stu-film__go { @on &.click { $stuNoop <- ST.stuFilmGo($); } }
.stu-film__tab { @on &.click { $stuFilmPanelName <- $.dataset.panel; } }

.stu-film__ticks {
  @each($stuFilmTicks as $k) {
    <span class="stu-tick" data-major="`$k.major`" style="left: `$k.left`%">`$k.label`</span>
  }
}
.stu-film__labels {
  @each($stuFilmRows as $r) {
    <button type="button" class="stu-lab" data-film="row" data-key="`$r.key`" data-kind="`$r.kind`" style="--stu-depth: `$r.depth`">`$r.label`</button>
  }
}
.stu-film__bars {
  @each($stuFilmRows as $r) {
    <div class="stu-bar-clip" data-film="clip" data-key="`$r.key`" data-kind="`$r.kind`" style="--stu-y: `$r.y`; left: `$r.left`%; width: `$r.width`%"><span class="stu-bar-clip__text">`$r.text`</span><canvas class="stu-wave" data-entry="`$r.entry`"></canvas><span class="stu-bar-clip__edge" data-film="edge" data-key="`$r.key`"></span></div>
  }
}
.stu-film__keys {
  @each($stuFilmKeys as $k) {
    <span class="stu-key" data-film="key" data-key="`$k.key`" data-held="`$k.held`" data-fixed="`$k.fixed`" title="`$k.value`" style="--stu-y: `$k.y`; left: `$k.left`%"></span>
  }
}
.stu-film__code {
  @each($stuFilmSrcs as $src) {
    <textarea class="stu-code" spellcheck="false" data-path="`$src.path`" data-hash="`$src.hash`">`$src.text`</textarea>
  }
}
.stu-code { @on &.change { $stuNoop <- ST.stuFilmSave($); } }

.stu-sec--film { .is-hidden: $stuFilmSel == ""; }
.stu-film-sel__label { text <- $stuFilmSelLabel; }
.stu-film-fields {
  @each($stuFilmFields as $f) {
    <label class="stu-field" data-kind="film"><span class="stu-field__key">`$f.label`</span><input class="stu-film-input" type="text" value="`$f.value`" data-mode="`$f.mode`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" spellcheck="false" /><span class="stu-field__hint">`$f.hint`</span></label>
  }
}
.stu-film-input { @on &.change { $stuNoop <- ST.stuFilmCommit($); } }
.stu-grid {
  @each($stuFilmGrid as $c) {
    <button type="button" class="stu-cell" data-kind="`$c.kind`" data-on="`$c.on`" data-text="`$c.text`" data-file="`$c.file`" data-start="`$c.start`" data-end="`$c.end`" title="`$c.text`" style="grid-row: `$c.row`; grid-column: `$c.col`">`$c.text`</button>
  }
}
.stu-cell { @on &.click { $stuNoop <- ST.stuFilmCell($); } }

.stu-tree--scene {
  @each($stuFilmParts as $p) {
    <button type="button" class="stu-node stu-part" data-key="`$p.key`" data-kind="`$p.kind`" style="--stu-depth: `$p.depth`"><span class="stu-node__mark"></span><span class="stu-node__label">`$p.label`</span><span class="stu-node__text">`$p.text`</span></button>
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
.stu-film__go, .stu-film__play, .stu-film__tab {
  height: 24px; min-width: 26px; padding: 0 8px; border: 0; border-radius: 5px;
  background: #26262c; color: var(--stu-film-ink); font: inherit; cursor: pointer;
}
.stu-film__go:hover, .stu-film__tab:hover { background: #303038; }
.stu-film__play { min-width: 44px; background: #0d99ff; color: #fff; font-weight: 600; }
.stu-film__play::after { content: "Play"; }
.stu-film__play.is-playing::after { content: "Pause"; }
.stu-film__time { font: 600 13px/1 ui-monospace, "SF Mono", Menlo, monospace; letter-spacing: 0.04em; padding: 0 8px; min-width: 9em; }
.stu-film__loop { color: #ffcf5a; font: 11px ui-monospace, Menlo, monospace; }
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

.stu-film__lanes { position: relative; min-height: 100%; }
.stu-film__ruler { position: sticky; top: 0; height: 22px; background: #1d1d22; border-bottom: 1px solid var(--stu-film-line); cursor: ew-resize; z-index: 2; }
.stu-film__ticks { position: absolute; inset: 0; pointer-events: none; }
.stu-tick { position: absolute; top: 0; bottom: 0; border-left: 1px solid #34343c; padding-left: 3px; color: var(--stu-film-soft); font-size: 10px; line-height: 22px; }
.stu-tick[data-major="false"] { top: 14px; }
.stu-film__bars, .stu-film__keys { position: absolute; left: 0; right: 0; top: 22px; bottom: 0; }
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

.stu-film__playhead {
  position: absolute; top: 0; bottom: 0; left: calc(var(--stu-t, 0) * 100%); width: 0;
  border-left: 1px solid #ff4d4d; pointer-events: none; z-index: 3;
}
.stu-film__playhead::before { content: ""; position: absolute; top: 0; left: -6px; border: 6px solid transparent; border-top-color: #ff4d4d; }

.stu-film__code { flex: 1; display: flex; }
.stu-code {
  flex: 1; margin: 0; border: 0; resize: none; padding: 10px 14px; background: #121215; color: #e8e8ec;
  font: 12px/1.55 ui-monospace, "SF Mono", Menlo, monospace; tab-size: 2; outline: none;
}

.stu-sec--film { display: block; }
.stu-sec--film.is-hidden { display: none; }
.stu-film-sel__label { margin: 0 0 8px; font-weight: 600; }
.stu-field__hint { display: block; color: var(--stu-soft); font-size: 10px; margin-top: 1px; }
.stu-film-input { width: 100%; box-sizing: border-box; border: 1px solid var(--stu-line); border-radius: 4px; padding: 4px 6px; font: 11px ui-monospace, Menlo, monospace; }
.stu-film-input:focus { outline: 1px solid var(--stu-blue); }
.stu-grid { display: grid; grid-template-columns: auto; grid-auto-columns: 14px; gap: 2px; margin-top: 10px; overflow-x: auto; }
.stu-cell[data-kind="name"] { width: auto; padding-right: 6px; background: transparent; color: var(--stu-soft); font: 10px ui-monospace, Menlo, monospace; text-align: left; cursor: default; white-space: nowrap; }
.stu-cell { width: 14px; height: 18px; padding: 0; border: 0; border-radius: 2px; background: #ececec; color: transparent; font-size: 0; cursor: pointer; }
.stu-cell[data-on="true"] { background: #8a5cf6; }
.stu-cell:nth-child(4n + 2) { box-shadow: inset 2px 0 0 #d0d0d0; }
