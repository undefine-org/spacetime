/// Migrations pill (PLAN-076) — syntax-migration status + per-wave apply,
/// in the shared dev dock, sitting left of the host pill with identical
/// chrome.
///
/// SIGNAL NAMESPACE: co-hosted widgets share ONE global `SpacetimeLocal`
/// (window-level reactive scope) — every signal here carries the `$mig`
/// prefix so the host pill's `$status`/`$isExpanded`/etc. can't collide
/// (observed: an un-prefixed `$isExpanded` toggle opened BOTH panels).
/// Compiler-level widget namespacing is a tracked follow-up.

///
/// Data: `@data fetch` of THIS dev server's
/// `/__spacetime/migrations/status.json` (same-origin, no auth — unlike the
/// host widget's remote calls, no `@data signal` headers needed). Per-wave
/// Apply goes through `@data signal` POSTs to `/__spacetime/migrations/apply`
/// (same rail as the CLI: the server route rewrites the project's files with
/// the shim's engine, bumps @version, and the file watcher reloads this
/// page — the pill re-derives from the fresh status; no client-side state
/// surgery, same reload rail as deploy-init).
///
/// Resting rule: ZERO pending entries → the widget hides itself entirely
/// (`.is-hidden` on the root) — a current project shows no dead chrome.
///
/// Row kinds: a wave with automatic (rewrite-kind) entries gets an Apply
/// button; a wave of only manual (hint-kind) entries shows its hints with
/// no button — those need a human (auxiliary rules, multi-edit rewrites).

@import "../../macros/host"
@import "../../macros/data-kind"

// ---------------------------------------------------------------------------
// Status data
// ---------------------------------------------------------------------------

@data fetch $migStatus : "/__spacetime/migrations/status.json" { refresh: 2s }

// Total pending entries across waves (0 while the fetch is in flight).
@data derive $migPending number : (($migStatus && $migStatus.pending) || 0);
@data derive $migHasPending bool : ($migPending > 0);

// Panel-header version label: the project's @version, or "none".
@data derive $migVersionLabel string : (($migStatus && $migStatus.version) || "none");

// Wave list for the panel rows (empty while fetching / when current).
@data derive $migWaves array : (($migStatus && $migStatus.waves) || []);

// A same-origin @host binding for THIS dev server's own /__spacetime/*
// endpoints (empty base = relative fetch — the host widget's $migSelfApi idiom).
@host $migSelfApi : http("");

// The apply action: POST {date} to the apply route. `receive to` maps the
// 200 and everything-else arms; the error body is `{"error": "…"}`.
@data signal $migApplyWave($date string) to $migSelfApi {
  send POST "/__spacetime/migrations/apply" { date: $date }
  receive to ApplyResult {
    200 => Applied($.body);
    _   => Failed($.body.error || $.statusText);
  }
  policy latest
}

// Busy + error state for the applying row / error row.
@data derive $migApplying bool : $migApplyWave_pending;
@data derive $migApplyLabel string : (($migApplyWave_pending && "Applying…") || "Apply");
@data inline $migApplyError string : "";
@data derive $migHasApplyError bool : ($migApplyError != "");

@handle $migApplyWave {
  receive {
    Applied(_) => {
      // The route rewrote the files on disk; the watcher reloads this page
      // (deploy-init's rail). Nothing to do client-side but clear the error
      // (the 2s `refresh` poll covers any no-reload case; a `@data fetch`
      // binding has no callable refetch, so there is no `$migStatus()` here).
      $migApplyError <- "";
    }
    Failed(msg) => {
      $migApplyError <- msg;
    }
  }
}

// ---------------------------------------------------------------------------
// Pill + panel interaction (same idiom as the host pill)
// ---------------------------------------------------------------------------

@data inline $migIsExpanded bool : false;

.st-migrations-widget {
  // The resting rule: zero pending → no chrome at all.
  .is-hidden: !$migHasPending;
}

.st-migrations-widget__pill {
  .is-open: $migIsExpanded;
  .is-working: $migApplying;

  @on &.click {
    $migIsExpanded <- !$migIsExpanded;
  }
}

.st-migrations-widget__panel {
  .is-open: $migIsExpanded;
}

.st-migrations-widget__count {
  text <- $migPending;
}

.st-migrations-widget__version {
  text <- $migVersionLabel;
}

.st-migrations-widget__error {
  .is-hidden: !$migHasApplyError;

  text <- $migApplyError;
}

// ---------------------------------------------------------------------------
// Markup + per-row behavior
//
// @each lives in SELECTOR SCOPES (never directly in widget markup — it
// renders as literal text there). Rows are each-inline HTML (nested
// each-inline for the entries, the demos/spacetime-docs pattern); per-row
// state rides data-* attributes + CSS attribute selectors (the kanban/dnd
// idiom — templates/rows host no reactive bindings); the Apply click reads
// its wave from $.dataset.wave (the documented @on payload idiom).
// ---------------------------------------------------------------------------

<section class="st-migrations-widget">
  <button type="button" class="st-migrations-widget__pill">
    <span class="st-migrations-widget__mark"></span>
    <span class="st-migrations-widget__pill-label">Syntax</span>
    <span class="st-migrations-widget__count"></span>
    <span class="st-migrations-widget__chev"></span>
  </button>

  <div class="st-migrations-widget__panel">
    <header class="st-migrations-widget__head">
      <span class="st-migrations-widget__title">Migrations</span>
      <span class="st-migrations-widget__version"></span>
    </header>
    <p class="st-migrations-widget__error"></p>
    <div class="st-migrations-widget__rows"></div>
  </div>
</section>

.st-migrations-widget__rows {
  @each($migWaves as $wave) {
    <div class="st-migrations-widget__wave" data-appliable="`$wave.appliable`">
      <div class="st-migrations-widget__wave-head">
        <span class="st-migrations-widget__date">`$wave.date`</span>
        <span class="st-migrations-widget__total">`$wave.total`</span>
        <button type="button" class="st-migrations-widget__apply" data-wave="`$wave.date`">Apply</button>
      </div>
      @each($wave.entries as $entry) {
        <div class="st-migrations-widget__entry">
          <span class="st-migrations-widget__entry-id">`$entry.id`</span>
          <span class="st-migrations-widget__entry-count">`$entry.count`</span>
          <span class="st-migrations-widget__entry-kind" data-kind="`$entry.kind`">`$entry.kind`</span>
        </div>
        <p class="st-migrations-widget__entry-docs">`$entry.docs`</p>
        <p class="st-migrations-widget__entry-hint" data-kind="`$entry.kind`">`$entry.hint`</p>
        <details class="st-migrations-widget__expander">
          <summary>old → new</summary>
          @each($entry.rules as $rule) {
            <div class="st-migrations-widget__rule">
              <div class="st-migrations-widget__rule-id">`$rule.id`</div>
              <pre class="st-migrations-widget__rule-match">`$rule.match`</pre>
              <pre class="st-migrations-widget__rule-into">`$rule.into`</pre>
            </div>
          }
          @each($entry.hints as $h) {
            <p class="st-migrations-widget__hint-line">@`$h.directive` — `$h.text`</p>
          }
        </details>
      }
    </div>
  }
}

.st-migrations-widget__apply {
  text <- $migApplyLabel;

  @on &.click {
    $migApplyWave($.dataset.wave);
  }
}

// ---------------------------------------------------------------------------
// Chrome — identical tokens to the host pill (glass surface, 999px radius,
// same font stack, same spark mark + chevron vocabulary)
// ---------------------------------------------------------------------------

.st-migrations-widget {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  color-scheme: dark;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 13px;
  pointer-events: none;
}

.st-migrations-widget.is-hidden {
  display: none;
}

.st-migrations-widget__pill {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(17, 17, 22, 0.82);
  backdrop-filter: blur(16px) saturate(1.4);
  -webkit-backdrop-filter: blur(16px) saturate(1.4);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  color: #e5e7eb;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: border-color 200ms ease, box-shadow 200ms ease, transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.st-migrations-widget__pill:hover {
  border-color: rgba(255, 255, 255, 0.22);
  transform: translateY(-1px);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.08);
}

.st-migrations-widget__pill:active {
  transform: translateY(0) scale(0.98);
}

// The working state (a wave applying): the pill's count badge goes orange.
.st-migrations-widget__pill.is-working .st-migrations-widget__count {
  background: rgba(249, 115, 22, 0.28);
  border-color: rgba(249, 115, 22, 0.55);
  color: #fdba74;
}

.st-migrations-widget__mark {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  background: conic-gradient(from 45deg, #8b5cf6, #3b82f6, #14b8a6, #8b5cf6);
  mask: radial-gradient(circle at center, transparent 15%, black 40%);
  -webkit-mask: radial-gradient(circle at center, transparent 15%, black 40%);
  rotate: 45deg;
}

.st-migrations-widget__pill-label {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #9ca3af;
}

.st-migrations-widget__count {
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 999px;
  background: rgba(139, 92, 246, 0.28);
  border: 1px solid rgba(139, 92, 246, 0.5);
  color: #c4b5fd;
  font-size: 11px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.st-migrations-widget__chev {
  width: 6px;
  height: 6px;
  border-right: 1.5px solid #9ca3af;
  border-bottom: 1.5px solid #9ca3af;
  rotate: 45deg;
  translate: 0 -2px;
  transition: rotate 200ms ease;
}

.st-migrations-widget__pill.is-open .st-migrations-widget__chev {
  rotate: 225deg;
  translate: 0 2px;
}

.st-migrations-widget__panel {
  pointer-events: none;
  position: absolute;
  bottom: calc(100% + 8px);
  right: 0;
  width: 320px;
  max-height: 60vh;
  overflow-y: auto;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(17, 17, 22, 0.92);
  backdrop-filter: blur(20px) saturate(1.4);
  -webkit-backdrop-filter: blur(20px) saturate(1.4);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  padding: 14px;
  opacity: 0;
  transform: translateY(6px) scale(0.98);
  transform-origin: bottom right;
  transition: opacity 180ms ease, transform 220ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.st-migrations-widget__panel.is-open {
  pointer-events: auto;
  opacity: 1;
  transform: translateY(0) scale(1);
}

.st-migrations-widget__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 10px;
}

.st-migrations-widget__title {
  font-size: 12px;
  font-weight: 700;
  color: #e5e7eb;
}

.st-migrations-widget__version {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  color: #6b7280;
}

.st-migrations-widget__error {
  margin: 0 0 10px;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(239, 68, 68, 0.12);
  border: 1px solid rgba(239, 68, 68, 0.35);
  color: #fca5a5;
  font-size: 12px;
}

.st-migrations-widget__error.is-hidden {
  display: none;
}

.st-migrations-widget__rows {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.st-migrations-widget__wave {
  padding: 8px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.st-migrations-widget__wave-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.st-migrations-widget__date {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  color: #d1d5db;
  flex: 1;
}

.st-migrations-widget__total {
  font-size: 11px;
  font-weight: 700;
  color: #c4b5fd;
}

.st-migrations-widget__apply {
  padding: 3px 10px;
  border-radius: 999px;
  border: 1px solid rgba(139, 92, 246, 0.55);
  background: rgba(139, 92, 246, 0.22);
  color: #c4b5fd;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  transition: background 150ms ease, border-color 150ms ease;
}

.st-migrations-widget__apply:hover {
  background: rgba(139, 92, 246, 0.36);
  border-color: rgba(139, 92, 246, 0.8);
}

// Per-row state via data-* attributes (no reactive bindings in rows):
// an all-manual wave hides its Apply button; manual entries chip amber.
.st-migrations-widget__wave[data-appliable="false"] .st-migrations-widget__apply {
  display: none;
}

.st-migrations-widget__entry {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 3px 4px;
}

.st-migrations-widget__entry-id {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  color: #d1d5db;
  flex: 1;
}

.st-migrations-widget__entry-count {
  font-size: 11px;
  font-weight: 700;
  color: #9ca3af;
}

.st-migrations-widget__entry-kind {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #6b7280;
}

.st-migrations-widget__entry-kind[data-kind="manual"] {
  color: #fbbf24;
}

.st-migrations-widget__entry-kind[data-kind="mixed"] {
  color: #c4b5fd;
}

.st-migrations-widget__entry-docs {
  margin: 0 4px 4px;
  font-size: 11px;
  line-height: 1.45;
  color: #9ca3af;
}

// The manual %hint guidance: only rendered when the entry carries one
// (automatic entries have `hint: null` — the attribute selector hides it).
.st-migrations-widget__entry-hint {
  display: none;
  margin: 0 4px 6px;
  padding: 6px 8px;
  border-radius: 6px;
  background: rgba(251, 191, 36, 0.08);
  border: 1px solid rgba(251, 191, 36, 0.22);
  font-size: 11px;
  line-height: 1.45;
  color: #fcd34d;
}

// Entries carrying a %hint (manual or mixed) show it.
.st-migrations-widget__entry-hint[data-kind="manual"],
.st-migrations-widget__entry-hint[data-kind="mixed"] {
  display: block;
}


// Expander (per-entry old-to-new grammar + hints)
.st-migrations-widget__expander {
  margin: 2px 0 8px;
  font-size: 11px;
  color: #9ca3af;
}

.st-migrations-widget__expander summary {
  cursor: pointer;
  color: #6b7280;
  font-size: 10.5px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  user-select: none;
}

.st-migrations-widget__expander summary:hover {
  color: #d1d5db;
}

.st-migrations-widget__rule {
  margin: 6px 0;
  padding: 6px 8px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
}

.st-migrations-widget__rule-id {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: #818cf8;
  margin-bottom: 4px;
}

.st-migrations-widget__rule-match,
.st-migrations-widget__rule-into {
  margin: 2px 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10.5px;
  white-space: pre-wrap;
  word-break: break-word;
}

.st-migrations-widget__rule-match {
  color: #fca5a5;
}

.st-migrations-widget__rule-into {
  color: #86efac;
}

.st-migrations-widget__hint-line {
  margin: 4px 0;
  color: #d1d5db;
  line-height: 1.45;
}
