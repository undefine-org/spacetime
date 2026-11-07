/// Studio (PLAN-154): edit the page you are looking at, and the source changes.
///
/// A chip in the dev dock opens three surfaces around the page:
///
/// - the EXPLORER (left), drawn from the shape of the code: the markup layers
///   of every file the page is made of, its lists (`@data` arrays, row by row,
///   draggable), its components (`@template`s with their docs), and its pages;
/// - the INSPECTOR (right): what is selected, the list item it was rendered
///   from with every field editable, its design as the browser computes it, and
///   the source rules that style it, with their declarations editable;
/// - the CANVAS (the page itself): click to select, double-click to edit text in
///   place, drag a list item's handle to reorder.
///
/// Every edit is a source edit (src/studio): the server rewrites the `.st`, the
/// page reloads, and the studio puts you back where you were. Signals are
/// `$stu*` because dock widgets share one reactive scope. This file holds no
/// JavaScript: behaviour lives in ./primitives/studio-kit.st.
@import "./primitives/studio-kit.st";

// ---- state ---------------------------------------------------------------------

@data inline $stuEntry string : "__SPACETIME_STUDIO_ENTRY__";
@data fetch $stuOutline : "/__spacetime/studio/outline.json?entry=__SPACETIME_STUDIO_ENTRY__";

// Open/closed and the explorer tab follow you from page to page.
@data stored $stuOpen bool : false;
@data stored $stuTabName string : "Layers";
@data inline $stuQuery string : "";
// The tool: Select edits the page; Interact lets it behave like the site, to
// reach a state (a tab, a filter, a hover) before selecting in it.
@data stored $stuToolName string : "Select";

// Written by the kit: the selection, the explorer key it maps to, and the
// result of the last edit.
@data inline $stuSel object : null;
@data inline $stuSelKey string : "";
@data inline $stuStatus string : "";
@data inline $stuError string : "";
@data inline $stuNoop string : "";

@type StuTab { Layers | Lists | Components | Pages }
@type StuSelection { Nothing | Element | Row }
@type StuTool { Select | Interact }

@data derive $stuTool StuTool : @match {
  ($stuToolName == "Interact") => Interact;
  _                            => Select;
}

@data derive $stuTab StuTab : @match {
  ($stuTabName == "Lists")      => Lists;
  ($stuTabName == "Components") => Components;
  ($stuTabName == "Pages")      => Pages;
  _                             => Layers;
}

@data derive $stuSelection StuSelection : @match {
  ($stuSel && $stuSel.row) => Row;
  ($stuSel)                => Element;
  _                        => Nothing;
}

// ---- views, derived from the outline -------------------------------------------

@data derive $stuLayers array : ST.stuLayers($stuOutline, $stuQuery);
@data derive $stuLists array : ST.stuLists($stuOutline, $stuQuery);
@data derive $stuComponents array : ST.stuComponents($stuOutline, $stuQuery);
@data derive $stuPages array : ST.stuPages($stuOutline);
@data derive $stuFields array : ST.stuFields($stuSel);
@data derive $stuRules array : ST.stuRules($stuSel, $stuOutline);
@data derive $stuBox array : ST.stuBox($stuSel);

@data derive $stuTitle string : (($stuOutline && $stuOutline.title) || "This page");
@data derive $stuFiles string : (($stuOutline && $stuOutline.files) ? $stuOutline.files.join("  ") : "");
@data derive $stuSelLabel string : ($stuSel ? $stuSel.label : "Nothing selected");
@data derive $stuSelCrumbs string : ($stuSel ? $stuSel.crumbs : "");
@data derive $stuSelText string : ($stuSel ? $stuSel.text : "");
@data derive $stuRowTitle string : (($stuSel && $stuSel.row) ? ("$" + $stuSel.row.binding + "  item " + ($stuSel.row.index + 1) + " of " + $stuSel.row.count) : "");
@data derive $stuRowFile string : (($stuSel && $stuSel.row) ? $stuSel.row.file : "");
@data derive $stuMessage string : ($stuError || $stuStatus);
@data derive $stuHasError bool : ($stuError != "");
@data derive $stuOutlineError string : (($stuOutline && $stuOutline.error) || "");

// ---- the chip and the surfaces -----------------------------------------------------

.st-studio { @studio-canvas(on: $stuOpen); }
.st-studio__pill {
  .is-open: $stuOpen;
  [aria-pressed]: $stuOpen;
  @on &.click { $stuOpen <- !$stuOpen; }
}
.stu-bar__close { @on &.click { $stuOpen <- false; } }
.stu-bar {
  @state(when: $stuTool is Select)   { --stu-tool: 0; }
  @state(when: $stuTool is Interact) { --stu-tool: 1; }
}
.stu-tool { @on &.click { $stuToolName <- $.dataset.tool; } }

.stu-explorer {
  @state(when: $stuTab is Layers)     { --stu-tab: 0; }
  @state(when: $stuTab is Lists)      { --stu-tab: 1; }
  @state(when: $stuTab is Components) { --stu-tab: 2; }
  @state(when: $stuTab is Pages)      { --stu-tab: 3; }
}
.stu-tab { @on &.click { $stuTabName <- $.dataset.tab; } }
.stu-find {
  value <- $stuQuery;
  @on &.input { $stuQuery <- $.value; }
}
.stu-explorer__title { text <- $stuTitle; }
.stu-explorer__files { text <- $stuFiles; }
.stu-explorer__error { .is-hidden: $stuOutlineError == ""; text <- $stuOutlineError; }

.stu-tree--layers {
  @each($stuLayers as $n) {
    <button type="button" class="stu-node" data-key="`$n.key`" data-kind="`$n.kind`" data-file="`$n.file`" data-at="`$n.index`" style="--stu-depth: `$n.depth`"><span class="stu-node__mark"></span><span class="stu-node__label">`$n.label`</span><span class="stu-node__text">`$n.text`</span></button>
  }
}
.stu-tree--lists {
  @each($stuLists as $n) {
    <button type="button" class="stu-node" draggable="`$n.drag`" data-key="`$n.key`" data-kind="`$n.kind`" data-binding="`$n.binding`" data-at="`$n.index`" style="--stu-depth: `$n.depth`"><span class="stu-node__mark"></span><span class="stu-node__label">`$n.label`</span><span class="stu-node__text">`$n.text`</span></button>
  }
}
.stu-tree--components {
  @each($stuComponents as $c) {
    <article class="stu-comp" data-group="`$c.group`"><header class="stu-comp__head"><code class="stu-comp__name">`$c.name`</code><code class="stu-comp__params">`$c.params`</code></header><p class="stu-comp__doc">`$c.doc`</p><footer class="stu-comp__foot"><span>`$c.group`</span><span>`$c.file`</span></footer></article>
  }
}
.stu-tree--pages {
  @each($stuPages as $p) {
    <a class="stu-page" href="`$p.route`" data-current="`$p.current`"><span class="stu-page__route">`$p.route`</span><span class="stu-page__file">`$p.file`</span></a>
  }
}

.stu-node { @on &.click { $stuNoop <- ST.stuReveal($); } }

.stu-inspector {
  @state(when: $stuSelection is Nothing) { --stu-sel: 0; }
  @state(when: $stuSelection is Element) { --stu-sel: 1; }
  @state(when: $stuSelection is Row)     { --stu-sel: 2; }
}
.stu-inspector__label { text <- $stuSelLabel; }
.stu-inspector__crumbs { text <- $stuSelCrumbs; }
.stu-text { text <- $stuSelText; }
.stu-row__title { text <- $stuRowTitle; }
.stu-row__file { text <- $stuRowFile; }
.stu-act { @on &.click { $stuNoop <- ST.stuRowAction($); } }

.stu-fields {
  @each($stuFields as $f) {
    <label class="stu-field" data-kind="`$f.kind`"><span class="stu-field__key">`$f.key`</span><textarea class="stu-field__input" rows="`$f.rows`" data-binding="`$f.binding`" data-at="`$f.index`" data-field="`$f.field`" spellcheck="false">`$f.value`</textarea></label>
  }
}
.stu-field__input { @on &.change { $stuNoop <- ST.stuCommitField($); } }

.stu-box {
  @each($stuBox as $b) {
    <div class="stu-prop" data-group="`$b.group`"><span class="stu-prop__group">`$b.group`</span><span class="stu-prop__k">`$b.label`</span><span class="stu-prop__v">`$b.value`</span><span class="stu-prop__sw" data-swatch="`$b.swatch`" style="background: `$b.swatch`"></span></div>
  }
}

.stu-rules {
  @each($stuRules as $r) {
    <div class="stu-rule" data-kind="`$r.kind`"><span class="stu-rule__a">`$r.a`</span><span class="stu-rule__b">`$r.b`</span><input class="stu-rule__input" type="text" value="`$r.value`" data-file="`$r.file`" data-start="`$r.start`" data-end="`$r.end`" data-expect="`$r.source`" spellcheck="false" /></div>
  }
}
.stu-rule__input { @on &.change { $stuNoop <- ST.stuCommitDecl($); } }

.stu-bar__msg { .is-error: $stuHasError; text <- $stuMessage; }

// The markup sits at FILE SCOPE: the dev dock injects this file's compiled html.
<section class="st-studio">
  <button type="button" class="st-studio__pill" aria-label="Edit this page" aria-pressed="false"><span class="st-studio__mark"></span><span class="st-studio__pill-label">Edit</span></button>

  <aside class="stu-explorer stu-ui" aria-label="Explorer">
    <header class="stu-explorer__head">
      <p class="stu-explorer__title">This page</p>
      <p class="stu-explorer__files"></p>
      <p class="stu-explorer__error"></p>
    </header>
    <nav class="stu-tabs" role="tablist" aria-label="Explorer views">
      <button type="button" class="stu-tab" data-tab="Layers">Layers</button>
      <button type="button" class="stu-tab" data-tab="Lists">Lists</button>
      <button type="button" class="stu-tab" data-tab="Components">Components</button>
      <button type="button" class="stu-tab" data-tab="Pages">Pages</button>
    </nav>
    <input class="stu-find" type="search" placeholder="Filter" aria-label="Filter the explorer" />
    <div class="stu-tree stu-tree--layers"></div>
    <div class="stu-tree stu-tree--lists"></div>
    <div class="stu-tree stu-tree--components"></div>
    <div class="stu-tree stu-tree--pages"></div>
  </aside>

  <aside class="stu-inspector stu-ui" aria-label="Inspector">
    <header class="stu-inspector__head">
      <p class="stu-inspector__label">Nothing selected</p>
      <p class="stu-inspector__crumbs"></p>
    </header>

    <section class="stu-sec stu-sec--empty">
      <p class="stu-empty">Click anything on the page to select it. Double-click text to edit it where it stands. Lists are in the explorer: drag their items to reorder.</p>
    </section>

    <section class="stu-sec stu-sec--row">
      <h3 class="stu-sec__h">List item</h3>
      <p class="stu-row__title"></p>
      <p class="stu-row__file"></p>
      <div class="stu-row__acts">
        <button type="button" class="stu-act" data-act="up" title="Move up (Alt+Up)">Up</button>
        <button type="button" class="stu-act" data-act="down" title="Move down (Alt+Down)">Down</button>
        <button type="button" class="stu-act" data-act="duplicate">Duplicate</button>
        <button type="button" class="stu-act stu-act--danger" data-act="delete">Delete</button>
      </div>
      <div class="stu-fields"></div>
    </section>

    <section class="stu-sec stu-sec--text">
      <h3 class="stu-sec__h">Content</h3>
      <p class="stu-text"></p>
    </section>

    <section class="stu-sec stu-sec--design">
      <h3 class="stu-sec__h">Design</h3>
      <div class="stu-box"></div>
    </section>

    <section class="stu-sec stu-sec--rules">
      <h3 class="stu-sec__h">Source rules</h3>
      <div class="stu-rules"></div>
    </section>
  </aside>

  <div class="stu-bar stu-ui" role="status">
    <span class="stu-tools" role="group" aria-label="Tool">
      <button type="button" class="stu-tool" data-tool="Select" title="Select and edit (V)">Select</button>
      <button type="button" class="stu-tool" data-tool="Interact" title="Use the page like a visitor (I); Alt-click does it once">Interact</button>
    </span>
    <button type="button" class="stu-bar__close">Done</button>
    <span class="stu-bar__msg"></span>
    <span class="stu-bar__hint">double-click edit / Esc parent / Alt+arrows move / Alt-click use</span>
  </div>
</section>

// ---- look: quiet, dense, Figma-like ------------------------------------------------

.st-studio {
  --stu-left: 264px;
  --stu-right: 300px;
  --stu-bg: #ffffff;
  --stu-line: #e6e6e6;
  --stu-ink: #1e1e1e;
  --stu-soft: #8a8a8a;
  --stu-blue: #0d99ff;
  --stu-blue-soft: #e5f4ff;
  --stu-red: #e5484d;
  --stu-violet: #9747ff;
  font: 11px/1.45 Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: var(--stu-ink);
}

.st-studio__pill {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 32px;
  padding: 0 14px;
  border: 0;
  border-radius: 999px;
  background: #1e1e1e;
  color: #fff;
  font: 600 12px/1 Inter, -apple-system, sans-serif;
  letter-spacing: 0.02em;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.18);
}
.st-studio__pill.is-open { background: var(--stu-blue); }
.st-studio__mark { width: 9px; height: 9px; border: 2px solid currentColor; border-radius: 2px; transform: rotate(45deg); }

.stu-explorer, .stu-inspector, .stu-bar { display: none; }
html.stu-on .stu-explorer, html.stu-on .stu-inspector { display: flex; }
html.stu-on .stu-bar { display: flex; }
// The dock steps out of the inspector's way while the studio is open.
html.stu-on .st-dev-dock { right: calc(var(--stu-right) + 16px) !important; }
html.stu-on { --stu-left: 264px; --stu-right: 300px; }
html.stu-on body { margin-left: var(--stu-left) !important; margin-right: var(--stu-right) !important; }

.stu-explorer, .stu-inspector {
  position: fixed;
  top: 0;
  bottom: 0;
  flex-direction: column;
  background: var(--stu-bg);
  box-shadow: 0 0 0 1px var(--stu-line);
  overflow: hidden;
  text-align: left;
}
.stu-explorer { left: 0; width: var(--stu-left); }
.stu-inspector { right: 0; width: var(--stu-right); overflow-y: auto; }

.stu-explorer__head, .stu-inspector__head { padding: 12px 14px 10px; border-bottom: 1px solid var(--stu-line); }
.stu-explorer__title, .stu-inspector__label { margin: 0; font-weight: 600; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.stu-explorer__files, .stu-inspector__crumbs { margin: 3px 0 0; color: var(--stu-soft); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.stu-explorer__error { margin: 6px 0 0; color: var(--stu-red); }
.stu-explorer__error.is-hidden { display: none; }

.stu-tabs { display: flex; gap: 2px; padding: 6px 8px; border-bottom: 1px solid var(--stu-line); }
.stu-tab {
  flex: 1;
  border: 0;
  background: transparent;
  color: var(--stu-soft);
  font: inherit;
  font-weight: 600;
  padding: 6px 4px;
  border-radius: 5px;
  cursor: pointer;
}
.stu-tab:hover { color: var(--stu-ink); background: #f5f5f5; }
.stu-explorer[data-st-state="Layers"] .stu-tab[data-tab="Layers"],
.stu-explorer[data-st-state="Lists"] .stu-tab[data-tab="Lists"],
.stu-explorer[data-st-state="Components"] .stu-tab[data-tab="Components"],
.stu-explorer[data-st-state="Pages"] .stu-tab[data-tab="Pages"] { color: var(--stu-ink); background: #f0f0f0; }

.stu-find {
  margin: 8px;
  padding: 6px 8px;
  border: 1px solid transparent;
  border-radius: 5px;
  background: #f5f5f5;
  font: inherit;
  color: var(--stu-ink);
  outline: none;
}
.stu-find:focus { border-color: var(--stu-blue); background: #fff; }

.stu-tree { display: none; flex: 1; overflow-y: auto; padding: 0 0 80px; }
.stu-explorer[data-st-state="Layers"] .stu-tree--layers,
.stu-explorer[data-st-state="Lists"] .stu-tree--lists,
.stu-explorer[data-st-state="Components"] .stu-tree--components,
.stu-explorer[data-st-state="Pages"] .stu-tree--pages { display: block; }

.stu-node {
  display: flex;
  align-items: baseline;
  gap: 6px;
  width: 100%;
  min-height: 28px;
  padding: 5px 10px 5px calc(10px + var(--stu-depth, 0) * 12px);
  border: 0;
  border-left: 2px solid transparent;
  background: transparent;
  color: var(--stu-ink);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.stu-node:hover { background: #f5f5f5; }
.stu-node.is-selected { background: var(--stu-blue-soft); border-left-color: var(--stu-blue); }
.stu-node__mark { flex: none; width: 8px; height: 8px; border: 1.5px solid var(--stu-soft); border-radius: 2px; transform: translateY(1px); }
.stu-node[data-kind="file"] { font-weight: 600; background: #fafafa; border-top: 1px solid var(--stu-line); }
.stu-node[data-kind="file"] .stu-node__mark { border-radius: 0; border-color: var(--stu-ink); }
.stu-node[data-kind="bound"] .stu-node__mark { border-color: var(--stu-violet); border-radius: 50%; }
.stu-node[data-kind="list"] { font-weight: 600; color: var(--stu-violet); }
.stu-node[data-kind="list"] .stu-node__mark { border-color: var(--stu-violet); }
.stu-node[data-kind="row"] { cursor: grab; }
.stu-node[data-kind="row"] .stu-node__mark { border-style: dotted; }
.stu-node__label { flex: none; max-width: 60%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stu-node__text { flex: 1; min-width: 0; color: var(--stu-soft); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.stu-comp { margin: 8px; padding: 10px; border: 1px solid var(--stu-line); border-radius: 6px; }
.stu-comp__head { display: flex; flex-wrap: wrap; gap: 4px; }
.stu-comp__name { font-weight: 600; color: var(--stu-violet); }
.stu-comp__params { color: var(--stu-soft); }
.stu-comp__doc { margin: 6px 0; }
.stu-comp__foot { display: flex; justify-content: space-between; color: var(--stu-soft); }

.stu-page { display: flex; justify-content: space-between; gap: 8px; padding: 7px 12px; color: var(--stu-ink); text-decoration: none; }
.stu-page:hover { background: #f5f5f5; }
.stu-page[data-current="true"] { background: var(--stu-blue-soft); font-weight: 600; }
.stu-page__file { color: var(--stu-soft); }

.stu-sec { display: none; padding: 12px 14px; border-bottom: 1px solid var(--stu-line); }
.stu-inspector[data-st-state="Nothing"] .stu-sec--empty,
.stu-inspector[data-st-state="Row"] .stu-sec--row,
.stu-inspector[data-st-state="Element"] .stu-sec--text,
.stu-inspector[data-st-state="Element"] .stu-sec--design,
.stu-inspector[data-st-state="Element"] .stu-sec--rules,
.stu-inspector[data-st-state="Row"] .stu-sec--design,
.stu-inspector[data-st-state="Row"] .stu-sec--rules { display: block; }
.stu-sec__h { margin: 0 0 8px; font-size: 11px; font-weight: 600; }
.stu-empty { margin: 0; color: var(--stu-soft); line-height: 1.6; }
.stu-text { margin: 0; color: var(--stu-ink); max-height: 7em; overflow: hidden; }

.stu-row__title { margin: 0; font-weight: 600; color: var(--stu-violet); }
.stu-row__file { margin: 2px 0 10px; color: var(--stu-soft); }
.stu-row__acts { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 12px; }
.stu-act {
  padding: 5px 9px;
  border: 1px solid var(--stu-line);
  border-radius: 5px;
  background: #fff;
  color: var(--stu-ink);
  font: inherit;
  cursor: pointer;
}
.stu-act:hover { border-color: var(--stu-ink); }
.stu-act--danger:hover { border-color: var(--stu-red); color: var(--stu-red); }

.stu-field { display: grid; grid-template-columns: 64px 1fr; gap: 6px; align-items: start; margin-bottom: 6px; }
.stu-field__key { padding-top: 6px; color: var(--stu-soft); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stu-field__input {
  width: 100%;
  box-sizing: border-box;
  padding: 5px 7px;
  border: 1px solid transparent;
  border-radius: 4px;
  background: #f5f5f5;
  color: var(--stu-ink);
  font: inherit;
  resize: vertical;
  outline: none;
}
.stu-field__input:hover { border-color: var(--stu-line); }
.stu-field__input:focus { border-color: var(--stu-blue); background: #fff; }

.stu-box { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 8px; }
.stu-prop__sw[data-swatch=""] { visibility: hidden; }
.stu-prop[data-group="Frame"] { grid-template-columns: 14px 1fr; }
.stu-prop[data-group="Frame"] .stu-prop__sw { display: none; }
.stu-prop { display: grid; grid-template-columns: 70px 1fr 12px; gap: 4px; align-items: center; min-width: 0; padding: 3px 6px; border-radius: 4px; background: #f5f5f5; }
.stu-prop[data-group="Layout"], .stu-prop[data-group="Type"], .stu-prop[data-group="Fill"], .stu-prop[data-group="Stroke"], .stu-prop[data-group="Effects"] { grid-column: 1 / -1; }
.stu-prop__group { display: none; }
.stu-prop__k { color: var(--stu-soft); }
.stu-prop__v { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stu-prop__sw { width: 12px; height: 12px; border-radius: 3px; box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12); }

.stu-rule { display: grid; grid-template-columns: 1fr 1.3fr; gap: 4px 8px; align-items: center; padding: 2px 0; }
.stu-rule__b { color: var(--stu-soft); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stu-rule[data-kind="rule"] { grid-template-columns: 1fr auto; margin-top: 8px; padding-top: 6px; border-top: 1px dashed var(--stu-line); }
.stu-rule[data-kind="rule"] .stu-rule__a { font-weight: 600; font-family: ui-monospace, Menlo, monospace; }
.stu-rule[data-kind="form"] .stu-rule__a { color: var(--stu-violet); font-family: ui-monospace, Menlo, monospace; }
.stu-rule[data-kind="decl"] .stu-rule__b, .stu-rule[data-kind="rule"] .stu-rule__input, .stu-rule[data-kind="form"] .stu-rule__input { display: none; }
.stu-rule__a { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--stu-soft); }
.stu-rule__input {
  min-width: 0;
  padding: 4px 6px;
  border: 1px solid transparent;
  border-radius: 4px;
  background: #f5f5f5;
  color: var(--stu-ink);
  font: 11px/1.3 ui-monospace, Menlo, monospace;
  outline: none;
}
.stu-rule__input:focus { border-color: var(--stu-blue); background: #fff; }

.stu-bar {
  position: fixed;
  bottom: 16px;
  left: calc(var(--stu-left) + 12px);
  right: calc(var(--stu-right) + 380px);
  justify-content: flex-start;
  align-items: center;
  gap: 10px;
  pointer-events: none;
}
.stu-bar > * { pointer-events: auto; }
.stu-tools { display: inline-flex; padding: 2px; border-radius: 7px; background: rgba(30, 30, 30, 0.92); }
.stu-tool {
  padding: 4px 10px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: #bdbdbd;
  font: 600 11px/1.3 Inter, -apple-system, sans-serif;
  cursor: pointer;
}
.stu-bar[data-st-state="Select"] .stu-tool[data-tool="Select"],
.stu-bar[data-st-state="Interact"] .stu-tool[data-tool="Interact"] { background: var(--stu-blue); color: #fff; }
.stu-bar__msg, .stu-bar__hint {
  min-width: 0;
  flex: 0 1 auto;
  padding: 6px 10px;
  border-radius: 6px;
  background: rgba(30, 30, 30, 0.92);
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.stu-bar__msg:empty { display: none; }
.stu-bar__msg.is-error { background: var(--stu-red); }
.stu-bar__hint { color: #bdbdbd; }
.stu-bar__close {
  padding: 6px 12px;
  border: 0;
  border-radius: 6px;
  background: var(--stu-blue);
  color: #fff;
  font: 600 11px/1.3 Inter, -apple-system, sans-serif;
  cursor: pointer;
}

// The canvas overlays, created by the kit at the end of <body>.
.stu-overlay { position: fixed; left: 0; top: 0; display: none; pointer-events: none; z-index: 2147483646; box-sizing: border-box; }
.stu-hover { outline: 1px solid var(--stu-blue, #0d99ff); }
.stu-select { outline: 2px solid #0d99ff; }
.stu-select.is-editing { outline: 2px solid #9747ff; }
.stu-select__label {
  position: absolute;
  left: -2px;
  bottom: 100%;
  margin-bottom: 4px;
  padding: 2px 6px;
  border-radius: 3px;
  background: #0a66b8;
  color: #fff;
  font: 600 10px/1.4 Inter, -apple-system, sans-serif;
  white-space: nowrap;
}
.stu-select.is-editing .stu-select__label { background: #9747ff; }
.stu-handle {
  position: absolute;
  top: 50%;
  left: -26px;
  display: none;
  width: 20px;
  height: 28px;
  margin-top: -14px;
  border: 0;
  border-radius: 4px;
  background: #0a66b8;
  color: #fff;
  font: 700 12px/28px ui-monospace, monospace;
  letter-spacing: -1px;
  cursor: grab;
  pointer-events: auto;
}
.stu-select.has-row .stu-handle { display: block; }
.stu-drop { height: 4px; border-radius: 2px; background: #0d99ff; box-shadow: 0 0 0 2px #fff; }
.stu-is-editing { outline: none !important; cursor: text; }
html.stu-on a, html.stu-on button { cursor: default; }
