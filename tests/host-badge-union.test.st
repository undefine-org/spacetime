/// Unit Tests: __host__ badge machine as a @match-derived union (PLAN-077 W7)
///
/// W7 migrated stdlib/__host__/index.st's 9-derive boolean chain
/// ($isBlue/$isOrange/$isTurquoise/$isGreen + 4× $showXBadge + the
/// stateLabel ||/-chain) into ONE closed sum + ONE @match derive, with the
/// dot/state indicators gated by @state(when: $badge is V).
///
/// __host__/index.st can't be imported standalone (it carries @host http
/// facts with compile-time __SPACETIME_HOST_* placeholders), so this test
/// REPLICATES the machine verbatim — same @type, same guard expressions,
/// same arm order, same label chain, same gate names — and pins the exact
/// outcomes across every documented state (V1_IMPLEMENTATION_PLAN.org §5):
///
///   precedence:  NoProject > Orange (in-flight) > Blue (401)
///                > Turquoise (preview-ahead) > Green (live) > None
///   label-only:  never-pushed, host-unreachable, checking…
///
/// If __host__/index.st's machine changes, change this replica with it
/// (the migration commit keeps them in lockstep by construction).

@import "stdlib/testing/test"

@type BadgeState {
  Blue | Orange | Turquoise | Green | NoProject | None
}

// ---- inputs (mirrors __host__'s @data inline signals) -------------------
@data inline $hostProjectId : "proj-1";
@data inline $statusData : null;
@data inline $isUnauthorized : false;
@data inline $status_pending : false;
@data inline $promote_pending : false;
@data inline $addDomain_pending : false;
@data inline $pushPreview_pending : false;
@data inline $statusFailed : false;

// ---- intermediate derives (verbatim from __host__) ----------------------
@data derive $isNoProject bool : ($hostProjectId == "");
@data derive $isNeverPushed bool : ($statusData && !$statusData.preview_hash);

// ---- the union machine (verbatim guard text + arm order) ----------------
@data derive $badge BadgeState : @match {
  ($isNoProject)  => NoProject;
  ($status_pending || $promote_pending || $addDomain_pending || $pushPreview_pending) => Orange;
  ($isUnauthorized) => Blue;
  ($statusData && $statusData.preview_hash && $statusData.preview_hash != $statusData.production_hash) => Turquoise;
  ($statusData && $statusData.preview_hash && $statusData.preview_hash == $statusData.production_hash) => Green;
  _ => None;
}

// ---- the label chain (verbatim) ------------------------------------------
@data derive $stateLabel string : (($badge && $badge.type == "NoProject" && "Not connected") || ($badge && $badge.type == "Orange" && "Working…") || ($badge && $badge.type == "Blue" && "Logged out") || ($badge && $badge.type == "Turquoise" && "Preview ahead of production") || ($badge && $badge.type == "Green" && "Live & current") || ($isNeverPushed && "Connected — nothing pushed yet") || ($statusFailed && "Host unreachable") || "Checking…");

// ---- the gated indicators (verbatim gate set) -----------------------------
.hb-dot {
  @state(when: $badge is NoProject) { background: #a78bfa; }
  @state(when: $badge is Blue) { background: #60a5fa; }
  @state(when: $badge is Orange) { background: #fb923c; }
  @state(when: $badge is Turquoise) { background: #2dd4bf; }
  @state(when: $badge is Green) { background: #4ade80; }
}

.hb-label {
  text <- $stateLabel;
}

// =============================================================================
// 1. The documented states resolve to the same variant + label the old
//    boolean chain produced. Driven as ONE state walk so precedence
//    transitions are exercised too (not just static endpoints).
// =============================================================================

@test "host-badge: every documented state resolves identically to the old chain" {
  @fixture {
    <div class="hb-dot"></div>
    <div class="hb-label"></div>
  }

  @when .hb-dot mouseenter

  // -- initial window: nothing checked yet ----------------------------------
  @eval (window.__hb = function() { return document.querySelector('.hb-dot').getAttribute('data-st-state') + '|' + document.querySelector('.hb-label').textContent; })
  @eval (window.__hb0 = window.__hb())
  @assert (window.__hb0 === 'None|Checking…')
    "initial (status unchecked): badge is None (matches no gate -> unstyled dot), label is Checking…"

  // -- 401: a REAL Unauthorized ---------------------------------------------
  @eval (window.SpacetimeLocal.isUnauthorized = true)
  @eval (window.__hb1 = window.__hb())
  @assert (window.__hb1 === 'Blue|Logged out')
    "401: Blue + 'Logged out'"

  // -- in-flight WINS over the stale 401 (Orange precedence) -----------------
  @eval (window.SpacetimeLocal.pushPreview_pending = true)
  @eval (window.__hb2 = window.__hb())
  @assert (window.__hb2 === 'Orange|Working…')
    "in-flight while 401: Orange wins visually ('in progress' beats stale state)"

  // -- call settles; back to the settled 401 state ---------------------------
  @eval (window.SpacetimeLocal.pushPreview_pending = false)
  @eval (window.__hb3 = window.__hb())
  @assert (window.__hb3 === 'Blue|Logged out')
    "call settled: falls back to Blue"

  // -- a real session with a preview ahead of production ---------------------
  @eval (window.SpacetimeLocal.isUnauthorized = false)
  @eval (window.SpacetimeLocal.statusData = {preview_hash: 'p1', production_hash: 'q1'})
  @eval (window.__hb4 = window.__hb())
  @assert (window.__hb4 === 'Turquoise|Preview ahead of production')
    "preview-ahead: Turquoise + 'Preview ahead of production'"

  // -- promoted: preview == production ---------------------------------------
  @eval (window.SpacetimeLocal.statusData = {preview_hash: 'p1', production_hash: 'p1'})
  @eval (window.__hb5 = window.__hb())
  @assert (window.__hb5 === 'Green|Live & current')
    "live: Green + 'Live & current'"

  // -- never pushed: real 200, no preview_hash — None badge, label-only state -
  @eval (window.SpacetimeLocal.statusData = {preview_hash: null, production_hash: null})
  @eval (window.__hb6 = window.__hb())
  @assert (window.__hb6 === 'None|Connected — nothing pushed yet')
    "never-pushed: badge is None (unstyled dot), label-only fallback"

  // -- host unreachable: genuine failure matches NO named state ---------------
  @eval (window.SpacetimeLocal.statusData = null)
  @eval (window.SpacetimeLocal.statusFailed = true)
  @eval (window.__hb7 = window.__hb())
  @assert (window.__hb7 === 'None|Host unreachable')
    "host-unreachable: badge is None (unstyled dot), 'Host unreachable'"

  // -- no-project wins over EVERYTHING (even an in-flight call) ---------------
  @eval (window.SpacetimeLocal.statusFailed = false)
  @eval (window.SpacetimeLocal.pushPreview_pending = true)
  @eval (window.SpacetimeLocal.hostProjectId = "")
  @eval (window.__hb8 = window.__hb())
  @assert (window.__hb8 === 'NoProject|Not connected')
    "no-project + in-flight: NoProject takes precedence over Orange"

  // -- restore (other tests share the global channel) -------------------------
  @eval (window.SpacetimeLocal.hostProjectId = "proj-1")
  @eval (window.SpacetimeLocal.pushPreview_pending = false)
}

// =============================================================================
// 2. None is a real, reflectable variant: the carrier reflects 'None'
//    (state-reflect writes the union's current variant name; None simply
//    matches NO gate, so the dot takes no state styling — the old chain's
//    "no badge class" outcome). A Turquoise→None transition must move the
//    carrier off Turquoise (no stale gated state).
// =============================================================================

@test "host-badge: None reflects but matches no gate" {
  @fixture {
    <div class="hb-dot"></div>
  }

  @when .hb-dot mouseenter

  @eval (window.SpacetimeLocal.statusData = {preview_hash: 'p2', production_hash: 'q2'})
  @eval (window.__hn0 = document.querySelector('.hb-dot').getAttribute('data-st-state'))
  @assert (window.__hn0 === 'Turquoise')
    "preview-ahead: carrier is Turquoise"

  @eval (window.SpacetimeLocal.statusData = null)
  @eval (window.__hn1 = document.querySelector('.hb-dot').getAttribute('data-st-state') || '')
  @assert (window.__hn1 === 'None')
    "badge → None: carrier moves to 'None' — matches no gate, no stale Turquoise"
}
