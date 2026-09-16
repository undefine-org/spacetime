/// Unit Tests: dispatch-mount parity (PLAN-077 W5)
///
/// The merged `%primitive dispatch-mount(reactive: bool)` backs BOTH @match
/// (render-once) and @view (re-mount-on-change) off the SAME arm-resolution
/// code path. These tests prove each mode keeps its pre-merge contract:
///
///   1. @match mounts the matching arm ONCE — a later subject change does NOT
///      re-mount (the old pane persists, no teardown).
///   2. @view re-mounts on subject change — old pane torn down, new arm's
///      template mounted (view.st's pre-merge behavior).
///   3. Both resolve the SAME arm for the SAME subject value (shared code
///      path: armPattern/armRef normalization + first-match-wins + `_`).

@import "stdlib/testing/test"

@template &paneA($label) {
  <div class="pane-a">"`$label`"</div>
}

@template &paneB($label) {
  <div class="pane-b">"`$label`"</div>
}

@data inline $mode : "a";

// File-scope dispatch sites (one per fixture class).
.dpm-match {
  @match $mode { "a" => &paneA("MA"); "b" => &paneB("MB"); _ => &paneB("MW"); }
}

.dpm-view {
  @view $mode { "a" => &paneA("VA"); "b" => &paneB("VB"); _ => &paneB("VW"); }
}

// =============================================================================
// 1. @match mounts once — NO re-mount on subject change
// =============================================================================

@test "dispatch: @match mounts once and does not re-mount" {
  @fixture {
    <div class="dpm-match"></div>
  }

  @when .dpm-match mouseenter

  @eval (window.__dpm1_a = document.querySelector('.dpm-match .pane-a') !== null)
  @assert (window.__dpm1_a === true)
    "initial: @match must mount the `a` arm (paneA)"

  // Flip the subject — @match is render-ONCE: the pane must NOT change.
  @eval (window.SpacetimeLocal.mode = 'b')
  @eval (window.__dpm1_stillA = document.querySelector('.dpm-match .pane-a') !== null)
  @eval (window.__dpm1_noB = document.querySelector('.dpm-match .pane-b') === null)
  @assert (window.__dpm1_stillA === true)
    "after mode='b': @match must KEEP the original paneA (render-once)"
  @assert (window.__dpm1_noB === true)
    "after mode='b': @match must NOT mount paneB (no re-mount)"

  @eval (window.SpacetimeLocal.mode = 'a')
}

// =============================================================================
// 2. @view re-mounts on subject change
// =============================================================================

@test "dispatch: @view re-mounts the matching pane on change" {
  @fixture {
    <div class="dpm-view"></div>
  }

  @when .dpm-view mouseenter

  @eval (window.__dpm2_a = document.querySelector('.dpm-view .pane-a') !== null)
  @assert (window.__dpm2_a === true)
    "initial: @view must mount the `a` arm (paneA)"

  @eval (window.SpacetimeLocal.mode = 'b')
  @eval (window.__dpm2_oldGone = document.querySelector('.dpm-view .pane-a') === null)
  @eval (window.__dpm2_newHere = document.querySelector('.dpm-view .pane-b') !== null)
  @assert (window.__dpm2_oldGone === true)
    "after mode='b': @view must TEAR DOWN paneA (re-mount contract)"
  @assert (window.__dpm2_newHere === true)
    "after mode='b': @view must mount paneB (the new matching arm)"

  // Wildcard: a value with no literal arm hits `_`.
  @eval (window.SpacetimeLocal.mode = 'zzz')
  @eval (window.__dpm2_w = document.querySelector('.dpm-view .pane-b') !== null)
  @assert (window.__dpm2_w === true)
    "after mode='zzz': the `_` wildcard arm must mount (paneB VW)"

  @eval (window.SpacetimeLocal.mode = 'a')
}

// =============================================================================
// 3. Same subject → same arm on both code paths (shared resolution)
// =============================================================================

@test "dispatch: match and view resolve the same arm for the same value" {
  @fixture {
    <div class="dpm-match"></div>
    <div class="dpm-view"></div>
  }

  @when .dpm-match mouseenter

  // SAME initial value ('a') → both paths must mount the SAME arm (shared
  // armPattern/armRef resolution — one code path, two reactivity modes).
  @eval (window.__dpm3_match = document.querySelector('.dpm-match .pane-a') !== null)
  @eval (window.__dpm3_view = document.querySelector('.dpm-view .pane-a') !== null)
  @assert (window.__dpm3_match === true)
    "initial mode='a': @match must mount the `a` arm"
  @assert (window.__dpm3_view === true)
    "initial mode='a': @view must mount the `a` arm"
}
