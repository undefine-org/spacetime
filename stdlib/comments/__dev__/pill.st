/// Comments dev-dock pill (PLAN-123).
///
/// The comments registry is server-owned: this widget only renders the roster
/// and sends validated shapes to its same-origin rails. Inline anchors are
/// deliberately absent from capture because `//@` inside HTML markup is literal
/// page text, so v1 cannot safely infer an inline source location.
///
/// ELEMENT anchors ARE capturable, via `@comments-pick`: the picker reports the
/// element's content and the SERVER mints the fingerprint that identifies it.
/// See ./pick.st for why the hash is not computed here.

@import "./pick.st";
@import "./locate.st";

@data fetch $comTypesResponse : "/__spacetime/comments/types.json" { refresh: 3s }
@data fetch $comStatusResponse : "/__spacetime/comments/status.json" { refresh: 3s }

@data derive $comTypes array : (($comTypesResponse && $comTypesResponse.types) || $comTypesResponse || []);
@data derive $comRecords array : (($comStatusResponse && $comStatusResponse.records) || []);
@data derive $comOrphans array : (($comStatusResponse && $comStatusResponse.orphans) || []);
// A failed fetch must SAY so. Deriving `|| []` alone renders an empty panel
// that is indistinguishable from "no comments" — the reader would conclude
// their team's comments vanished rather than that a request failed.
@data derive $comLoadError string : (($comTypesResponse_error && "could not load the comment types") || ($comStatusResponse_error && "could not load comments") || "");
@data derive $comOpenCount number : $comRecords.filter(($comRecord) => $comRecord.status == "open" || $comRecord.status == "in-progress").length;
@data derive $comPrune array : (($comStatusResponse && $comStatusResponse.prune) || []);
@data derive $comPruneFiles string : Array.from(new Set($comPrune.map(($comSpan) => $comSpan.file))).join(", ");
@data derive $comPruneLabel string : ($comPrune.length + " resolved comment" + ($comPrune.length == 1 ? "" : "s") + ($comPrune.length ? (" from " + $comPruneFiles) : ""));

@host $comApi : http("");

@data signal $comUpdateStatus($comId string, $comStatus string) to $comApi {
  send POST "/__spacetime/comments/update" { id: $comId, status: $comStatus }
  receive to ComUpdateStatusResult {
    200 => ComUpdated($.body);
    _ => ComUpdateFailed($.body.error || $.statusText);
  }
  policy latest
}

@data signal $comReply($comId string, $comReplyText string) to $comApi {
  send POST "/__spacetime/comments/update" { id: $comId, reply: $comReplyText }
  receive to ComReplyResult {
    200 => ComReplied($.body);
    _ => ComReplyFailed($.body.error || $.statusText);
  }
  policy latest
}

@data signal $comAdd($comType string, $comAnchor object, $comText string, $comMeta object, $comContent object) to $comApi {
  send POST "/__spacetime/comments/add" { type: $comType, anchor: $comAnchor, text: $comText, meta: $comMeta, element_content: $comContent }
  receive to ComAddResult {
    201 => ComAdded($.body);
    _ => ComAddFailed($.body.error || $.statusText);
  }
  policy latest
}

@data signal $comSource($comId string) to $comApi {
  send GET "/__spacetime/comments/source.json?id=`$comId`"
  receive to ComSourceResult {
    200 => ComSourceFound($.body);
    _ => ComSourceFailed($.body.error || $.statusText);
  }
  policy latest
}

@data signal $comReanchor($comId string, $comAnchor object) to $comApi {
  send POST "/__spacetime/comments/re-anchor" { id: $comId, anchor: $comAnchor }
  receive to ComReanchorResult {
    200 => ComReanchored($.body);
    _ => ComReanchorFailed($.body.error || $.statusText);
  }
  policy latest
}

// Dismiss has its own endpoint because it destroys persisted team data; it is
// never folded into the ordinary update rail where it could look reversible.
// Prune is intentionally a separate destructive operation: the server will
// refuse stale snapshots and anything not still resolved + inline.
@data signal $comPrune($comSpans array) to $comApi {
  send POST "/__spacetime/comments/prune" { spans: $comSpans }
  receive to ComPruneResult {
    200 => ComPruned($.body);
    _ => ComPruneFailed($.body.error || $.statusText);
  }
  policy latest
}

@data signal $comDismiss($comId string) to $comApi {
  send POST "/__spacetime/comments/dismiss" { id: $comId }
  receive to ComDismissResult {
    204 => ComDismissed();
    _ => ComDismissFailed($.body.error || $.statusText);
  }
  policy latest
}

@data inline $comExpanded bool : false;
@data inline $comSelectedType string : "";
@data inline $comAnchorKind string : "page";
@data inline $comAnchorValue string : "";
@data inline $comNewText string : "";
@data inline $comReplyText string : "";
// A type may REQUIRE a field (stdlib `agent-task` requires `acceptance`), so
// the composer must be able to supply one — otherwise that type is
// uncreatable from the UI and the roster's own contract is unusable here.
@data inline $comMetaKey string : "";
@data inline $comMetaValue string : "";
@data inline $comOrphanAnchorKind string : "page";
@data inline $comOrphanAnchorValue string : "";
@data inline $comError string : "";

// ELEMENT PICKING (FUP-171 W-d).
// `$comPicking` arms the document listener; `$comPicked` receives what the
// picker SAW. The fingerprint is minted SERVER-SIDE from that content — hashing
// here would be a second implementation of one rule, and a byte of disagreement
// would orphan every anchor one surface wrote.
@data inline $comPicking bool : false;
@data inline $comPicked object : null;
@data derive $comPickedTag string : ($comPicked && $comPicked.tag) || "";
@data derive $comPickLabel string : ($comPicking ? "Click any element on the page…" : ($comPickedTag ? ("picked: " + $comPickedTag) : ""));

// LOCATING (FUP-171 W-e). `$comLocateFor` carries the stored content descriptor
// of the record whose "Show" was pressed; `$comLocateOutcome` reports what the
// page actually had. A comment about an element you cannot find is a riddle,
// and an outcome of `gone` is information — it says the element the note was
// about is no longer there, which is exactly what a reviewer needs to know.
@data inline $comLocateFor object : null;
@data inline $comLocateOutcome string : "";
@data derive $comLocateLabel string : (($comLocateOutcome == "found") && "highlighted on the page") || (($comLocateOutcome == "gone") && "that element is no longer on this page") || (($comLocateOutcome == "ambiguous") && "several identical elements match — cannot tell them apart") || (($comLocateOutcome == "unverified") && "this comment predates element identity, so it cannot be located") || "";

// JUMP TO SOURCE (FUP-170). "Show on page" finds where an element RENDERS;
// this finds where it was WRITTEN. They are different questions: source
// survives an element behind a false branch, an element emptied of text, and
// duplicate rows that render identically. The span is resolved by the server
// against the CURRENT file every time — storing it on the anchor would go
// stale on the first edit above it and then point confidently at the wrong
// markup, which is the positional failure fingerprints already fixed once.
@data inline $comSourceResponse object : null;
@data derive $comSourceOutcome string : ($comSourceResponse && $comSourceResponse.outcome) || "";
@data derive $comSourceWhere string : ($comSourceResponse && $comSourceResponse.file && ($comSourceResponse.file + ($comSourceResponse.template ? (" · &" + $comSourceResponse.template + "()") : ""))) || "";
// `moved` is reported, not hidden: it means the stored positional hint went
// stale and identity had to find the element instead. A reviewer seeing a
// surprising location deserves to know the anchor drifted.
@data derive $comSourceLabel string : (($comSourceOutcome == "found") && ($comSourceWhere + ($comSourceResponse.moved ? " (moved since the comment was made)" : ""))) || (($comSourceOutcome == "gone") && "no element in the source matches this comment") || (($comSourceOutcome == "ambiguous") && "several elements are written identically — they cannot be told apart") || (($comSourceOutcome == "unverified") && ($comSourceWhere + " (unconfirmed — this comment predates element identity)")) || (($comSourceOutcome == "synthesized") && "the HTML parser inserted this element, so it has no authored source") || (($comSourceOutcome == "no-source") && ($comSourceResponse.detail || "no source for this route")) || (($comSourceOutcome == "not-an-element") && "only element comments have an element to find") || "";

// An element anchor is only ready once something has actually been picked:
// a route alone would mint an anchor with no identity, i.e. the legacy shape.
@data derive $comCanAdd bool : ($comSelectedType != "" && $comNewText != "" && ($comAnchorKind == "element" ? ($comPickedTag != "") : ($comAnchorValue != "")));

@handle $comUpdateStatus {
  receive {
    ComUpdated(_) => { $comError <- ""; }
    ComUpdateFailed($comMessage) => { $comError <- $comMessage; }
  }
}

@handle $comReply {
  receive {
    ComReplied(_) => { $comError <- ""; $comReplyText <- ""; }
    ComReplyFailed($comMessage) => { $comError <- $comMessage; }
  }
}

@handle $comAdd {
  receive {
    ComAdded(_) => { $comError <- ""; $comNewText <- ""; $comAnchorValue <- ""; }
    ComAddFailed($comMessage) => { $comError <- $comMessage; }
  }
}

@handle $comSource {
  receive {
    ComSourceFound($comBody) => { $comError <- ""; $comSourceResponse <- $comBody; }
    ComSourceFailed($comMessage) => { $comError <- $comMessage; }
  }
}

@handle $comReanchor {
  receive {
    ComReanchored(_) => { $comError <- ""; $comOrphanAnchorValue <- ""; }
    ComReanchorFailed($comMessage) => { $comError <- $comMessage; }
  }
}

@handle $comPrune {
  receive {
    ComPruned(_) => { $comError <- ""; }
    ComPruneFailed($comMessage) => { $comError <- $comMessage; }
  }
}

@handle $comDismiss {
  receive {
    ComDismissed() => { $comError <- ""; }
    ComDismissFailed($comMessage) => { $comError <- $comMessage; }
  }
}

.st-comments-widget__pill {
  .is-open: $comExpanded;
  @on &.click { $comExpanded <- !$comExpanded; }
}

.st-comments-widget__panel { .is-open: $comExpanded; }
.st-comments-widget__count { text <- $comOpenCount; }
// A failed WRITE and a failed READ both land here: an empty panel caused by a
// dead endpoint must never read as "your team has no comments".
.st-comments-widget__error { .is-hidden: ($comError == "" && $comLoadError == ""); text <- ($comError || $comLoadError); }
.st-comments-widget__add { .is-disabled: !$comCanAdd; }

<section class="st-comments-widget">
  <button type="button" class="st-comments-widget__pill">
    <span class="st-comments-widget__mark"></span>
    <span class="st-comments-widget__label">Comments</span>
    <span class="st-comments-widget__count"></span>
    <span class="st-comments-widget__chevron"></span>
  </button>

  <aside class="st-comments-widget__panel" aria-label="Comments">
    <header class="st-comments-widget__header">
      <strong>Comments</strong>
      <span class="st-comments-widget__open-label">open / in progress</span>
    </header>
    <p class="st-comments-widget__error"></p>
    <div class="st-comments-widget__prune">
      <span>Remove `$comPruneLabel` from source. Settled threads stay in .comments as recoverable history.</span>
      <button type="button" class="st-comments-widget__prune-button">Prune resolved source comments</button>
    </div>
    <div class="st-comments-widget__types"></div>
    <p class="st-comments-widget__locate-state">`$comLocateLabel`</p>
    <p class="st-comments-widget__source-state">`$comSourceLabel`</p>
    <div class="st-comments-widget__records"></div>
    <div class="st-comments-widget__orphans"></div>
    <section class="st-comments-widget__composer">
      <strong>Add comment</strong>
      <div class="st-comments-widget__type-picker"></div>
      <div class="st-comments-widget__anchor-kinds">
        <button type="button" data-anchor-kind="page">Page</button>
        <button type="button" data-anchor-kind="file">File</button>
        <button type="button" data-anchor-kind="element">Element</button>
      </div>
      <p class="st-comments-widget__pick-state">`$comPickLabel`</p>
      <input class="st-comments-widget__anchor" placeholder="/route or source/file.st">
      <textarea class="st-comments-widget__text" placeholder="What should a reader know?"></textarea>
      <div class="st-comments-widget__meta">
        <input class="st-comments-widget__meta-key" placeholder="field (e.g. acceptance)">
        <input class="st-comments-widget__meta-value" placeholder="value">
      </div>
      <button type="button" class="st-comments-widget__add">Add comment</button>
    </section>
  </aside>
</section>

// Dynamic rows must be selector-scoped: widget markup only supplies stable hosts.
.st-comments-widget__types {
  @each($comTypes as $comType) {
    <span class="st-comments-widget__type" style="--com-type-color: `$comType.color`" title="`$comType.docs`">`$comType.label`</span>
  }
}

// Each record shows WHO moved its status, and from where. A status with no
// author cannot be questioned — "resolved" reads identically whether someone
// verified the fix or an agent closed work it never read.
//
// NB the history block below sits INSIDE markup, so it carries no `//` comment:
// inside an HTML block `//` is literal page text and would render to visitors.
.st-comments-widget__records {
  @each($comRecords as $comRecord) {
    <article class="st-comments-widget__record" data-id="`$comRecord.id`">
      <div class="st-comments-widget__record-head">
        <span class="st-comments-widget__record-type">`$comRecord.type_id`</span>
        <span class="st-comments-widget__record-status">`$comRecord.status`</span>
      </div>
      <span class="st-comments-widget__record-anchor">`$comRecord.anchor.route || $comRecord.anchor.file`</span>
      <span class="st-comments-widget__record-selector">`$comRecord.anchor.selector || ""`</span>
      <button type="button" class="st-comments-widget__locate" data-locate="`$comRecord.id`">Show on page</button>
      <button type="button" class="st-comments-widget__source" data-source="`$comRecord.id`">Where in source</button>
      <p class="st-comments-widget__record-text">`$comRecord.text`</p>
      <div class="st-comments-widget__record-history">
        @each($comRecord.history as $comChange) {
          <span class="st-comments-widget__change">`$comChange.from` → `$comChange.to` by `$comChange.author.name`</span>
        }
      </div>
      <div class="st-comments-widget__actions">
        <button type="button" data-status="in-progress">Claim</button>
        <button type="button" data-status="resolved">Resolve</button>
        <button type="button" data-status="wontfix">Won't fix</button>
        </div>
      <div class="st-comments-widget__reply">
        <textarea placeholder="Reply"></textarea>
        <button type="button" data-send-reply="`$comRecord.id`">Send reply</button>
      </div>
    </article>
  }
}

.st-comments-widget__orphans {
  @each($comOrphans as $comOrphan) {
    <article class="st-comments-widget__orphan" data-id="`$comOrphan.id`">
      <strong>`$comOrphan.type_id`</strong>
      <span class="st-comments-widget__record-anchor">was `$comOrphan.anchor.file`:`$comOrphan.anchor.line`</span>
      <p class="st-comments-widget__record-text">`$comOrphan.text`</p>
      <span class="st-comments-widget__orphan-thread">`$comOrphan.thread.length` replies</span>
      <span class="st-comments-widget__orphan-why">its header is gone from source — status and thread survive</span>
      <div class="st-comments-widget__orphan-anchor-kinds">
        <button type="button" data-orphan-anchor-kind="page">Page</button>
        <button type="button" data-orphan-anchor-kind="file">File</button>
      </div>
      <input class="st-comments-widget__orphan-anchor" placeholder="new /route or project/file">
      <div class="st-comments-widget__orphan-actions">
        <button type="button" data-reanchor="`$comOrphan.id`">Re-anchor</button>
        <button type="button" class="st-comments-widget__dismiss" data-dismiss="`$comOrphan.id`">Dismiss permanently</button>
      </div>
    </article>
  }
}

.st-comments-widget__type-picker {
  @each($comTypes as $comType) {
    <button type="button" class="st-comments-widget__type-option" data-type="`$comType.id`" style="--com-type-color: `$comType.color`">`$comType.label`</button>
  }
}

.st-comments-widget__type-picker { @on &.click [data-type] { $comSelectedType <- $.dataset.type; } }
// Choosing `element` ARMS pick mode; choosing anything else disarms it, so the
// document listener can never outlive the intent that created it.
.st-comments-widget__anchor-kinds { @on &.click [data-anchor-kind] { $comAnchorKind <- $.dataset.anchorKind; $comPicking <- ($.dataset.anchorKind == "element"); } }

// The picker itself: mounted on the widget so its scope owns $comPicking and
// $comPicked. It excludes .st-comments-widget internally, so arming pick mode
// never turns the pill's own controls into anchor targets.
.st-comments-widget {
  @comments-pick(on: $comPicking, to: $comPicked);
  @comments-locate(for: $comLocateFor, to: $comLocateOutcome);
}

// "Show" hands the record's STORED content descriptor to the locator, which
// re-derives the match by content. Passing the selector alone would highlight
// whatever now occupies that position — the silent mis-point this design refuses.
.st-comments-widget__records { @on &.click [data-locate] { $comLocateFor <- $comRecords.filter(($comRow) => $comRow.id == $.dataset.locate)[0].anchor.content; } }
.st-comments-widget__records { @on &.click [data-source] { $comSource($.dataset.source); } }
.st-comments-widget__anchor { value <- $comAnchorValue; @on &.input { $comAnchorValue <- $.value; } }
.st-comments-widget__text { value <- $comNewText; @on &.input { $comNewText <- $.value; } }
// The anchor is a TAGGED union server-side (`#[serde(tag = "kind")]`), so the
// discriminator must be sent explicitly — without `kind` the request fails
// deserialization before the handler ever runs, and the composer silently
// does nothing.
// An element anchor sends the PICKED CONTENT alongside; the server mints the
// fingerprint from it. Its route comes from the picked element's own page, not
// the anchor input, so a picked element is never filed under a route the user
// happened to have typed.
.st-comments-widget__add { @on &.click { $comAdd($comSelectedType, (($comAnchorKind == "element" && { kind: "element", route: ($comPicked.route || $comAnchorValue), selector: ($comPicked.selector || "") }) || ($comAnchorKind == "page" && { kind: "page", route: $comAnchorValue }) || { kind: "file", file: $comAnchorValue }), $comNewText, (($comMetaKey && { [$comMetaKey]: $comMetaValue }) || {}), (($comAnchorKind == "element" && $comPicked) || {})); } }
.st-comments-widget__meta-key { value <- $comMetaKey; @on &.input { $comMetaKey <- $.value; } }
.st-comments-widget__meta-value { value <- $comMetaValue; @on &.input { $comMetaValue <- $.value; } }
.st-comments-widget__prune-button { @on &.click { $comPrune($comPrune); } }
.st-comments-widget__records { @on &.click [data-status] { $comUpdateStatus($.closest(".st-comments-widget__record").dataset.id, $.dataset.status); } }
.st-comments-widget__records { @on &.input .st-comments-widget__reply textarea { $comReplyText <- $.value; } }
.st-comments-widget__records { @on &.click [data-send-reply] { $comReply($.dataset.sendReply, $comReplyText); } }
.st-comments-widget__orphans { @on &.click [data-orphan-anchor-kind] { $comOrphanAnchorKind <- $.dataset.orphanAnchorKind; } }
.st-comments-widget__orphan-anchor { value <- $comOrphanAnchorValue; @on &.input { $comOrphanAnchorValue <- $.value; } }
// This is the same tagged anchor shape as the composer; inline is absent
// because source-derived ids can only be minted by the scanner.
.st-comments-widget__orphans { @on &.click [data-reanchor] { $comReanchor($.dataset.reanchor, (($comOrphanAnchorKind == "page" && { kind: "page", route: $comOrphanAnchorValue }) || { kind: "file", file: $comOrphanAnchorValue })); } }
.st-comments-widget__orphans { @on &.click [data-dismiss] { $comDismiss($.dataset.dismiss); } }

.st-comments-widget { position: relative; display: flex; flex-direction: column; align-items: flex-end; color: #e5e7eb; font: 12px/1.4 ui-monospace, "SF Mono", monospace; pointer-events: none; }
.st-comments-widget__pill { pointer-events: auto; display: flex; align-items: center; gap: 7px; padding: 8px 13px; border: 1px solid rgba(255,255,255,.16); border-radius: 999px; background: rgba(20,20,25,.88); color: inherit; cursor: pointer; box-shadow: 0 8px 24px rgba(0,0,0,.32); }
.st-comments-widget__pill:hover { border-color: rgba(255,255,255,.32); }
.st-comments-widget__mark { width: 9px; height: 9px; border: 2px solid #7dd3fc; border-radius: 50%; box-shadow: inset 0 0 0 2px rgba(20,20,25,.88); }
.st-comments-widget__label { font-weight: 700; letter-spacing: .04em; text-transform: uppercase; }
.st-comments-widget__count { min-width: 18px; padding: 1px 5px; border-radius: 999px; background: #164e63; color: #bae6fd; text-align: center; }
.st-comments-widget__chevron { width: 6px; height: 6px; border-right: 1px solid #9ca3af; border-bottom: 1px solid #9ca3af; rotate: 45deg; translate: 0 -2px; }
.st-comments-widget__pill.is-open .st-comments-widget__chevron { rotate: 225deg; translate: 0 2px; }
.st-comments-widget__panel { pointer-events: none; position: absolute; right: 0; bottom: calc(100% + 8px); width: min(370px, calc(100vw - 24px)); max-height: min(68vh, 640px); overflow: auto; padding: 13px; border: 1px solid rgba(255,255,255,.13); border-radius: 14px; background: rgba(20,20,25,.96); box-shadow: 0 18px 48px rgba(0,0,0,.48); opacity: 0; transform: translateY(5px); transition: opacity 160ms ease, transform 160ms ease; }
.st-comments-widget__panel.is-open { pointer-events: auto; opacity: 1; transform: translateY(0); }
.st-comments-widget__header, .st-comments-widget__record-head, .st-comments-widget__actions, .st-comments-widget__anchor-kinds, .st-comments-widget__orphan-anchor-kinds, .st-comments-widget__orphan-actions { display: flex; align-items: center; gap: 7px; }
.st-comments-widget__header { justify-content: space-between; margin-bottom: 9px; }
.st-comments-widget__open-label, .st-comments-widget__record-anchor, .st-comments-widget__record-selector, .st-comments-widget__record-status { color: #9ca3af; font-size: 10px; }
.st-comments-widget__record-selector { overflow-wrap: anywhere; }
.st-comments-widget__types, .st-comments-widget__type-picker { display: flex; flex-wrap: wrap; gap: 5px; margin-bottom: 10px; }
.st-comments-widget__type, .st-comments-widget__type-option { border: 1px solid var(--com-type-color); border-radius: 999px; padding: 2px 6px; color: var(--com-type-color); background: transparent; font: inherit; font-size: 10px; }
.st-comments-widget__type-option { cursor: pointer; }
.st-comments-widget__records { display: grid; gap: 8px; }
.st-comments-widget__record, .st-comments-widget__orphan { padding: 9px; border: 1px solid rgba(255,255,255,.1); border-radius: 9px; background: rgba(255,255,255,.035); }
.st-comments-widget__record-type { color: #bae6fd; font-weight: 700; flex: 1; }
.st-comments-widget__record-text { margin: 5px 0 7px; white-space: pre-wrap; }
.st-comments-widget__actions button, .st-comments-widget__anchor-kinds button, .st-comments-widget__add, .st-comments-widget__reply button { border: 1px solid rgba(125,211,252,.34); border-radius: 6px; padding: 3px 6px; background: rgba(14,116,144,.18); color: #bae6fd; font: inherit; font-size: 10px; cursor: pointer; }
.st-comments-widget__reply { display: grid; margin-top: 7px; gap: 5px; }
.st-comments-widget__reply textarea { min-height: 42px; }
.st-comments-widget__composer { display: grid; gap: 7px; margin-top: 11px; padding-top: 11px; border-top: 1px solid rgba(255,255,255,.1); }
.st-comments-widget__anchor, .st-comments-widget__orphan-anchor, .st-comments-widget__text, .st-comments-widget__reply textarea { width: 100%; box-sizing: border-box; border: 1px solid rgba(255,255,255,.16); border-radius: 6px; padding: 6px; background: rgba(0,0,0,.2); color: #e5e7eb; font: inherit; font-size: 11px; resize: vertical; }
.st-comments-widget__text { min-height: 56px; }
.st-comments-widget__add.is-disabled { opacity: .5; }
.st-comments-widget__locate { border: 1px solid rgba(125,211,252,.34); border-radius: 6px; padding: 2px 6px; background: rgba(14,116,144,.18); color: #bae6fd; font: inherit; font-size: 10px; cursor: pointer; justify-self: start; }
.st-comments-widget__locate-state:empty, .st-comments-widget__pick-state:empty { display: none; }
.st-comments-widget__locate-state, .st-comments-widget__pick-state { margin: 4px 0 0; color: #9ca3af; font-size: 10px; }
// The located element is outlined on the PAGE, not in the widget — an outline
// plus offset reads on any background without touching the page's own layout.
.st-comments-located { outline: 2px solid #38bdf8 !important; outline-offset: 3px !important; }
.st-comments-widget__record-history:empty { display: none; }
.st-comments-widget__record-history { display: grid; gap: 2px; margin-top: 4px; }
.st-comments-widget__change { color: #9ca3af; font-size: 10px; }
.st-comments-widget__error { margin: 0 0 8px; color: #fca5a5; }.st-comments-widget__error.is-hidden { display: none; }
.st-comments-widget__orphans:empty { display: none; }.st-comments-widget__orphan { display: grid; gap: 5px; margin-top: 8px; color: #fbbf24; }.st-comments-widget__orphan span { display: block; color: #d1d5db; }.st-comments-widget__orphan-anchor-kinds button, .st-comments-widget__orphan-actions button { border: 1px solid rgba(251,191,36,.38); border-radius: 6px; padding: 3px 6px; background: rgba(146,64,14,.2); color: #fde68a; font: inherit; font-size: 10px; cursor: pointer; }.st-comments-widget__dismiss { border-color: rgba(248,113,113,.55) !important; background: rgba(127,29,29,.35) !important; color: #fecaca !important; }
@media (max-width: 430px) { .st-comments-widget__panel { right: -4px; width: min(370px, calc(100vw - 12px)); border-radius: 10px; } .st-comments-widget__actions { flex-wrap: wrap; } }
