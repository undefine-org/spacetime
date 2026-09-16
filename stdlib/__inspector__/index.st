/// Dev dock inspector: source-backed template tree + typed invocation editors.
/// Every signal is `$ins*`: dock widgets share one global reactive scope.
@import "stdlib/fields";
@import "../__dev__/primitives/dev-ws.st";
@import "./primitives/inspect-write.st";
@import "./primitives/inspect-data-write.st";
@import "./primitives/inspect-pick.st";

// ROOT SELECTION. `main` is a convention held by MCP guests and by this
// projection's original fixtures; real pages overwhelmingly do NOT have one, so a
// pill that could only walk `main` reported "no template named 'main'" and hid
// itself on 23 of 23 checked projects. The server now falls back to the bundle's
// first template and reports the WHOLE roster in one response, which is why this
// URL stays static: `@data fetch` takes its source as a literal (the emitted
// primitive assigns `srcValue` verbatim and never evaluates it), so a signal in
// the URL would be sent as the text `$insRootQuery`. Re-pointing a live source
// would need a refetch API that does not exist — a new mechanism, not a fix. The
// server therefore sends every root's nodes at once and the switcher filters
// them here, which also keeps switching instant instead of a 2s poll away.
@data fetch $insStructure : "/__spacetime/inspect/structure.json?entry=__SPACETIME_INSPECT_ENTRY__&all=1" { refresh: 2s }
@data inline $insRoot string : "";
@data derive $insNodes array : ST.insNodesForRoot($insStructure, $insRoot);
@data derive $insParams array : ST.insParamsForRoot($insStructure, $insRoot);
@data derive $insError string : (($insStructure && ($insStructure.error || (($insStructure.diagnostics || []).join(" · ")))) || "");
@data derive $insCount number : $insNodes.length;
// The roster of walkable roots, and the one the server actually chose (which may
// differ from `$insRoot` when this pill let the server pick).
@data derive $insTemplates array : (($insStructure && $insStructure.templates) || []);
@data derive $insActiveRoot string : ($insRoot != "" ? $insRoot : (($insStructure && $insStructure.template) || ""));
@data derive $insHasRoots bool : ($insTemplates.length > 1);
// A page whose entry declares NO templates has nothing to walk. That is a real,
// common shape (markup at file scope), and it is NOT an error — so the pill stays
// visible and SAYS so, rather than vanishing and leaving the absence unexplained.
@data derive $insNoTemplates bool : (($insStructure && $insStructure.reason) == "no-templates");
// A template-less page has NO tree — but it is exactly the page whose @data rows
// are pickable, so the pill must stay visible and offer the gesture. Hiding it
// (the old rule) is what made pick look broken on 12 of 23 projects: the surface
// was editable, the entry point simply was not there.
@data derive $insVisible bool : ($insError == "");
@data inline $insIsExpanded bool : false;
@data inline $insPicking bool : false;
@data inline $insSelectedNode string : "";
@data inline $insActiveParams array : [];
@data inline $insWriteRequest object : null;
// The @data half of the write rail: a JSON (file, path, value) address rather
// than a source span. Separate signal so the two rails can never claim one edit.
@data inline $insDataWrite object : null;
@data inline $insNoop string : "";
// Recomputed on the POLL as well as on selection: instances mount and unmount as
// the page lives (an `@each` gains a row, a `@portal` opens), so a count derived
// from the selection alone goes stale and misreports how many places an edit
// touches. `$insParams` ticks with every 2s poll, which is the cheapest honest
// clock available here.
@data derive $insSelectedInstanceCount number : ST.insInstanceCount($insSelectedNode, $insParams);
@data derive $insSelectedInstanceLabel string : ($insSelectedInstanceCount > 1 ? $insSelectedInstanceCount + " instances" : "");
@data derive $insHasSelectedInstanceLabel bool : ($insSelectedInstanceLabel != "");


.st-inspector-widget { .is-hidden: !$insVisible; }
.st-inspector-widget { .is-picking: $insPicking; @inspector-pick(on: $insPicking); }
.st-inspector-widget__pill { .is-open: $insIsExpanded; @on &.click { $insIsExpanded <- !$insIsExpanded; } }
.st-inspector-widget__panel { .is-open: $insIsExpanded; }
.st-inspector-widget__count { text <- $insCount; }
.st-inspector-widget__instance-count { .is-hidden: !$insHasSelectedInstanceLabel; text <- $insSelectedInstanceLabel; }
// `text <-` binds a SIGNAL, not an expression: a `||` inside the binding is
// parsed as a filter pipe (`| $insError)`), which splits the expression and
// emits unbalanced parens — a SyntaxError that kills the whole pill bundle, so
// no signal registers and the panel silently stays empty. Combine upstream in a
// derive and bind the result, which is what the other two pills do.
// Bind the write rails' exports by NAME, at file scope.
//
// `%yield expr -> $export` publishes onto the element the primitive is bound to,
// under the ALIAS declared in `%binds` (`$status as $insWriteStatus`), and a
// file-scope reader crosses that plane via `ST._bridgeExport` — which a `@data
// derive` registers for each of its deps. So naming the aliases in a derive is
// the whole link; no per-element reading required.
//
// This did not work before, for a reason that had nothing to do with scope: the
// `%binds` output parser split its list on `,` only, and the bind-line
// accumulator had already joined the one-per-line block into a single run. Both
// rails' lists therefore parsed as ONE output whose alias was the rest of the
// text, no remap was recorded, and `%yield` wrote the RAW export name — so
// `$insWriteStatus` never existed to be read. Fixed in parse_bind_outputs.
//
// Both rails export the same pair; whichever fired last wins, and only one can
// fire per edit since a row is either span- or data-addressed, never both.
// Instantiate the shared dev WebSocket the write rails send over (see
// inspect-data-write.st for why the pill binds this itself).
@inspector-dev-bridge

@data derive $insStatusText string : ($insWriteStatus || $insDataWriteStatus || "");
@data derive $insErrorText string : ($insWriteError || $insDataWriteError || "");

@data derive $insMessage string : ($insErrorText || $insError);
@data derive $insHasMessage bool : ($insMessage != "");

.st-inspector-widget__error { .is-hidden: !$insHasMessage; text <- $insMessage; }
.st-inspector-widget__status { .is-hidden: $insStatusText == ""; text <- $insStatusText; }

// The pill's markup lives at FILE SCOPE, because that is what the dev dock
// injects: `compile_inspector_widget_raw` takes `compiled.html` and drops it
// into the dock div (src/server.rs). Moving this into a `@template` leaves that
// html EMPTY, so the pill silently disappears from every real page while its
// tests keep passing — the markup has to be here.
//
// `&inspector-widget` below re-declares the SAME markup as a mountable template
// purely so tests can instantiate the pill in isolation; keep the two in step.
<section class="st-inspector-widget">
  <button type="button" class="st-inspector-widget__pill" aria-label="Inspect template structure">
    <span class="st-inspector-widget__mark"></span><span class="st-inspector-widget__pill-label">Inspect</span><span class="st-inspector-widget__count"></span><span class="st-inspector-widget__chev"></span>
  </button>
  <div class="st-inspector-widget__panel">
    <header class="st-inspector-widget__head"><span class="st-inspector-widget__title">Template tree</span><span class="st-inspector-widget__instance-count"></span><span class="st-inspector-widget__status"></span></header>
    <p class="st-inspector-widget__error"></p>
    <button type="button" class="st-inspector-widget__pick" aria-pressed="false">Pick element</button>
    <label class="st-inspector-widget__roots"><span class="st-inspector-widget__roots-label">Root</span><select class="st-inspector-widget__root-select"></select></label>
    <p class="st-inspector-widget__empty"></p>
    <div class="st-inspector-widget__tree"></div>
    <section class="st-inspector-widget__editors"><p class="st-inspector-widget__edit-label">Parameters</p><div class="st-inspector-widget__param-list"></div></section>
  </div>
</section>

@template &inspector-widget() {
  <section class="st-inspector-widget">
    <button type="button" class="st-inspector-widget__pill" aria-label="Inspect template structure">
      <span class="st-inspector-widget__mark"></span><span class="st-inspector-widget__pill-label">Inspect</span><span class="st-inspector-widget__count"></span><span class="st-inspector-widget__chev"></span>
    </button>
    <div class="st-inspector-widget__panel">
      <header class="st-inspector-widget__head"><span class="st-inspector-widget__title">Template tree</span><span class="st-inspector-widget__instance-count"></span><span class="st-inspector-widget__status"></span></header>
      <p class="st-inspector-widget__error"></p>
      <button type="button" class="st-inspector-widget__pick" aria-pressed="false">Pick element</button>
      <label class="st-inspector-widget__roots"><span class="st-inspector-widget__roots-label">Root</span><select class="st-inspector-widget__root-select"></select></label>
      <p class="st-inspector-widget__empty"></p>
      <div class="st-inspector-widget__tree"></div>
      <section class="st-inspector-widget__editors"><p class="st-inspector-widget__edit-label">Parameters</p><div class="st-inspector-widget__param-list"></div></section>
    </div>
  </section>
}

&inspector-widget();
.st-inspector-widget__tree {
  @each($insNodes as $n) {
    <button type="button" class="st-inspector-widget__node" data-node="`$n.id`" data-kind="`$n.kind`" style="--ins-depth: `$n.depth`"><span class="st-inspector-widget__node-mark"></span><span class="st-inspector-widget__node-label">`$n.label`</span><span class="st-inspector-widget__node-template">`$n.template`</span></button>
  }
}
.st-inspector-widget__node {
  @on &.click {
    $insNoop <- ST.insSelectNode($.closest('.st-inspector-widget'), $.closest('[data-node]').dataset.node, $insParams);
  }
}
.st-inspector-widget__pick { .is-picking: $insPicking; [aria-pressed]: $insPicking; @on &.click { $insPicking <- !$insPicking; } }

// The root switcher. Only shown when there is a real choice to make; a
// single-template page needs no chrome for it. Changing the root re-points the
// fetch URL, so the tree, the params, and the write addresses all move together.
.st-inspector-widget__roots { .is-hidden: !$insHasRoots; }
.st-inspector-widget__root-select {
  @each($insTemplates as $t) {
    <option value="`$t`">`$t`</option>
  }
  // A `selected` ATTRIBUTE cannot express the choice here: a falsy hole renders
  // as the literal string "false", which is still a present attribute, so EVERY
  // option came back selected and the browser kept the last one — the dropdown
  // disagreed with the tree it was supposed to label. The select's `value` is the
  // honest surface for "which option is current", so bind that instead.
  value <- $insActiveRoot;
  @on &.change { $insRoot <- $.value; $insSelectedNode <- ""; $insActiveParams <- []; }
}
// The no-templates state. Says what IS true (the page authors markup at file
// scope) instead of leaving an empty panel to imply the inspector is broken.
.st-inspector-widget__empty {
  .is-hidden: !$insNoTemplates;
  text <- "No templates on this page — use Pick element to select a row rendered from @data and edit its fields.";
}
.st-inspector-widget__node { .is-selected: node.dataset.node == $insSelectedNode; }
// Every accepted write changes the file, so the credentials the rendered editors
// hold go stale at once and the NEXT edit is silently refused. Re-project on each
// poll and patch the addresses IN PLACE — never re-render, which is what BUG-151
// forbids (it would destroy a focused input mid-edit).
.st-inspector-widget__param-list {
  @on $insParams.change {
    $insNoop <- ST.insRefreshCredentials($, $insParams, $insSelectedNode);
  }
}
// ONE @each, dispatching each row straight to its typed widget. (A wrapper
// template cannot host the `&ref` — a template body is scanned as HTML, so the
// ref renders as literal text — and giving each row its own selector scope makes
// every row re-render the whole list. The write address rides on the descriptor
// instead, which the shared widgets already emit as `data-addr-*`.)
.st-inspector-widget__param-list { @each($insActiveParams as $p) { &$p.tpl($p); } }
// The @data write rail sits on its OWN selector: two %exports-bearing primitives
// on one selector left the second one's signals unregistered.
.st-inspector-widget__edit-label { @inspector-data-write(on: $insDataWrite); }
// TWO events, not one. A simple control (text/number/toggle) commits on
// `change` — blur/enter, so a half-typed value is never written. But the
// COMPOSITE widgets never fire `change` at all: a range slider and the HSV
// sliders write through their twin/hex input with a bubbling `input`
// (stdlib/fields/primitives/field-value.st), so listening only for `change`
// silently dropped every slider and colour-picker edit.
//
// `ST.insWriteRequest` builds the request from whichever control fired,
// returning null when the row is unwritable or the value is not committable —
// so a stray event can never manufacture a write.
.st-inspector-widget__editors {
  @inspector-write(on: $insWriteRequest, entry: "__SPACETIME_INSPECT_ENTRY__");
  @on &.change [data-widget] {
    $insWriteRequest <- ST.insWriteRequest($);
    $insDataWrite <- ST.insDataWrite($);
  }
  @on &.input [data-range-twin] {
    $insWriteRequest <- ST.insWriteRequest($);
    $insDataWrite <- ST.insDataWrite($);
  }
  @on &.input .st-fld-color-text {
    $insWriteRequest <- ST.insWriteRequest($);
    $insDataWrite <- ST.insDataWrite($);
  }
}

.st-inspector-widget { position: relative; display: flex; flex-direction: column; align-items: flex-end; gap: 8px; color-scheme: dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; font-size: 13px; pointer-events: none; }
.st-inspector-widget.is-hidden { display: none; }
.st-inspector-widget__roots { display: flex; align-items: center; gap: 6px; margin: 0 0 10px; }
.st-inspector-widget__roots.is-hidden { display: none; }
.st-inspector-widget__roots-label { font: 10px ui-monospace,"SF Mono",monospace; letter-spacing: .08em; text-transform: uppercase; color: #9ca3af; }
.st-inspector-widget__root-select { flex: 1; padding: 5px 6px; border: 1px solid rgba(148,163,184,.35); border-radius: 5px; background: rgba(15,23,42,.85); color: #e5e7eb; font: 11px ui-monospace,"SF Mono",monospace; }
.st-inspector-widget__empty { margin: 0 0 8px; color: #cbd5e1; font-size: 11px; line-height: 1.5; }
.st-inspector-widget__empty.is-hidden { display: none; }
.st-inspector-widget__pill { pointer-events: auto; display: flex; align-items: center; gap: 8px; padding: 8px 14px; border-radius: 999px; border: 1px solid rgba(255,255,255,.12); background: rgba(17,17,22,.82); backdrop-filter: blur(16px) saturate(1.4); -webkit-backdrop-filter: blur(16px) saturate(1.4); box-shadow: 0 8px 24px rgba(0,0,0,.35), inset 0 1px 0 rgba(255,255,255,.06); color: #e5e7eb; font: inherit; font-size: 12px; font-weight: 600; letter-spacing: .02em; cursor: pointer; transition: border-color 200ms ease, box-shadow 200ms ease, transform 200ms cubic-bezier(.34,1.56,.64,1); }
.st-inspector-widget__pill:hover { border-color: rgba(255,255,255,.22); transform: translateY(-1px); box-shadow: 0 12px 32px rgba(0,0,0,.45), inset 0 1px 0 rgba(255,255,255,.08); }
.st-inspector-widget__pill:active { transform: translateY(0) scale(.98); }
.st-inspector-widget__mark { width: 14px; height: 14px; border-radius: 3px; background: conic-gradient(from 45deg,#8b5cf6,#3b82f6,#14b8a6,#8b5cf6); mask: radial-gradient(circle at center,transparent 15%,black 40%); -webkit-mask: radial-gradient(circle at center,transparent 15%,black 40%); rotate: 45deg; }
.st-inspector-widget__pill-label { font-family: ui-monospace,"SF Mono","Cascadia Code",monospace; font-size: 11px; font-weight: 500; letter-spacing: .08em; text-transform: uppercase; color: #9ca3af; }
.st-inspector-widget__count { min-width: 18px; height: 18px; padding: 0 5px; border-radius: 999px; background: rgba(139,92,246,.28); border: 1px solid rgba(139,92,246,.5); color: #c4b5fd; font-size: 11px; font-weight: 700; display: inline-flex; align-items: center; justify-content: center; }
.st-inspector-widget__chev { width: 6px; height: 6px; border-right: 1.5px solid #9ca3af; border-bottom: 1.5px solid #9ca3af; rotate: 45deg; translate: 0 -2px; transition: rotate 200ms ease; }
.st-inspector-widget__pill.is-open .st-inspector-widget__chev { rotate: 225deg; translate: 0 2px; }
.st-inspector-widget__panel { pointer-events: none; position: absolute; bottom: calc(100% + 8px); right: 0; width: 400px; max-height: 60vh; overflow-y: auto; border-radius: 14px; border: 1px solid rgba(255,255,255,.1); background: rgba(17,17,22,.92); backdrop-filter: blur(20px) saturate(1.4); -webkit-backdrop-filter: blur(20px) saturate(1.4); box-shadow: 0 16px 48px rgba(0,0,0,.5), inset 0 1px 0 rgba(255,255,255,.06); padding: 14px; opacity: 0; transform: translateY(6px) scale(.98); transform-origin: bottom right; transition: opacity 180ms ease, transform 220ms cubic-bezier(.34,1.56,.64,1); }
.st-inspector-widget__panel.is-open { pointer-events: auto; opacity: 1; transform: translateY(0) scale(1); }
.st-inspector-widget__head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 10px; }.st-inspector-widget__title { font-size: 12px; font-weight: 700; color: #e5e7eb; }.st-inspector-widget__status { font: 11px ui-monospace,"SF Mono",monospace; color: #86efac; }.st-inspector-widget__status.is-hidden,.st-inspector-widget__error.is-hidden { display: none; }
.st-inspector-widget__instance-count { margin-left: auto; margin-right: 8px; color: #c4b5fd; font: 10px ui-monospace,"SF Mono",monospace; }.st-inspector-widget__instance-count.is-hidden { display: none; }.st-inspector-widget__pick { width: 100%; margin: 0 0 10px; padding: 6px 8px; border: 1px solid rgba(167,139,250,.45); border-radius: 6px; background: rgba(139,92,246,.12); color: #ddd6fe; font: 600 11px ui-monospace,"SF Mono",monospace; cursor: pointer; }.st-inspector-widget__pick.is-picking { background: rgba(139,92,246,.32); border-color: #a78bfa; color: #fff; }
.st-inspector-widget__error { margin: 0 0 10px; padding: 8px 10px; border-radius: 8px; background: rgba(239,68,68,.12); border: 1px solid rgba(239,68,68,.35); color: #fca5a5; font-size: 12px; }
.st-inspector-widget__tree { display: flex; flex-direction: column; gap: 2px; padding-bottom: 10px; border-bottom: 1px solid rgba(255,255,255,.08); }.st-inspector-widget__node { display: flex; align-items: center; gap: 7px; width: 100%; min-width: 0; padding: 5px 6px 5px calc(6px + var(--ins-depth) * 14px); border: 0; border-radius: 6px; background: transparent; color: #d1d5db; font: inherit; font-size: 11px; text-align: left; cursor: pointer; }.st-inspector-widget__node:hover { background: rgba(255,255,255,.07); }.st-inspector-widget__node.is-selected { background: rgba(139,92,246,.28); color: #f5f3ff; }.st-inspector-widget__node-mark { width: 6px; height: 6px; border-radius: 2px; background: #64748b; }.st-inspector-widget__node[data-kind="template"] .st-inspector-widget__node-mark { background: #a78bfa; }.st-inspector-widget__node[data-kind="element"] .st-inspector-widget__node-mark { background: #2dd4bf; }.st-inspector-widget__node-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }.st-inspector-widget__node-template { color: #6b7280; font: 10px ui-monospace,"SF Mono",monospace; }
/* FUP-136: outline exactly ONE node — the INNERMOST stamped ancestor of the
   pointer, which is also the node a click resolves to (`closest('[data-st-node]')`).
   `:hover` matches every ancestor under the pointer, so a template invoked inside
   another invocation drew BOTH outlines and the highlight read as two boxes with
   no indication which one a click would take.
   `:not(:has([data-st-node]:hover))` removes any stamped element that CONTAINS a
   hovered stamped element, leaving the innermost one. Same rule for the two @data
   granularities below, so a row does not outline alongside its own field. */
body:has(.st-inspector-widget.is-picking) [data-st-node]:hover:not(:has([data-st-node]:hover)) { outline: 2px solid #a78bfa; outline-offset: 2px; }
/* A @data row is pickable too, and a stamped FIELD is the finer target — a
   distinct hue so the two granularities are visibly different while picking. */
body:has(.st-inspector-widget.is-picking) [data-st-source][data-st-index]:hover:not(:has([data-st-source][data-st-index]:hover)):not(:has([data-st-field]:hover)) { outline: 2px solid #a78bfa; outline-offset: 2px; }
body:has(.st-inspector-widget.is-picking) [data-st-field]:hover { outline: 2px solid #34d399; outline-offset: 1px; }
.st-inspector-widget__editors { padding-top: 10px; }.st-inspector-widget__edit-label { margin: 0 0 6px; color: #9ca3af; font: 10px ui-monospace,"SF Mono",monospace; letter-spacing: .08em; text-transform: uppercase; }.st-inspector-widget__param-list { display: flex; flex-direction: column; gap: 7px; }.st-fld-field:has([data-addr=""]) { opacity: .55; }.st-fld-field:has([data-addr=""]) input { pointer-events: none; }.st-inspector-widget__readonly { display: block; padding: 7px 9px; border-radius: 6px; background: rgba(255,255,255,.04); color: #9ca3af; font: 11px ui-monospace,"SF Mono",monospace; white-space: pre-wrap; }
