// Regression: `@value-change` on an inline-level, auto-width element (e.g. a
// single-line "flap" word-cycle with `display: inline-block` and no explicit
// width) must NOT force-wrap wider incoming text to a second line.
//
// Root cause (fixed in stdlib/primitives/animation/drivers.st,
// value-change-driver): the driver read `%&el.offsetWidth` as the wrap
// constraint for laying out the NEW text. For an inline-level auto-width
// element, offsetWidth is the shrink-to-fit width of the OLD text — not a
// real line-box constraint (by CSS auto-width semantics such an element never
// force-wraps on its own). Feeding that stale, too-narrow width into the
// pretext line-breaker made wider incoming text falsely wrap to 2 lines,
// animating the element to a doubled height while the absolutely-positioned
// exit layer (still showing the old text) visually overlapped the freshly
// wrapped new text underneath it — a persistent "ghost text" double-line
// artifact on every transition, not just a single dropped frame. Reproduced
// live via a served page (not this test harness): 100px steady-state →
// 191.9px false-wrap peak → 100px settle; fixed measurement stays flat at
// ~100px throughout the transition.
//
// Fix: only treat offsetWidth as a wrap constraint for block-level elements
// (block/flex/grid/list-item/table/flow-root), which stretch to their
// container regardless of content. Inline-level auto-width elements get an
// unconstrained (Infinity) width, matching their real CSS layout behavior:
// single line, no self-imposed wrap.
//
// The precise fix is asserted directly on the compiled driver source in
// `pipeline::integration_tests::tests::test_value_change_driver_guards_container_width_by_display_level`
// (src/pipeline/integration_tests.rs) — a pixel-level CDP assertion here was
// found to be unreliable due to separate, unrelated CDP-harness bugs (a
// `@mount`-scoped `.selector {}` CSS block's rules do not reliably reach the
// mounted element, and the WAAPI height-tween the driver kicks off does not
// reliably animate under the harness's timing model — filed separately).
// This fixture instead smoke-tests that `@value-change` on an inline-block
// auto-width element compiles, mounts, and drives a real text-change without
// throwing — the actual wrap-prevention behavior is proven above.
@import "stdlib/testing/test"

@test "@value-change on an inline-block auto-width element mounts and drives a text change without throwing" needs layout {
  @mount {
    <span class="vc-flap" style="display: inline-block; font-size: 32px; font-weight: 700;">short</span>

    .vc-flap {
      // Default keyframes (no explicit `{ :entering {...} :exiting {...} }`
      // block) intentionally — an explicit keyframe-arrow block nested inside
      // a `.selector {}` block within `@mount` currently triggers a separate,
      // unrelated compiler bug (BUG-173: the whole test body's $body:block
      // capture gets mis-scored as a CSS Properties capture and silently
      // truncated to empty, making every assertion vacuously pass). Filed
      // separately; this fixture avoids it to stay reliable.
      @value-change(duration: 100ms, stagger: 10ms)
    }
  }

  @eval (document.querySelector('.vc-flap').textContent = 'a much wider replacement phrase here')

  @wait 300ms

  @assert (document.querySelector('.vc-flap') !== null)
    "the flap element must still exist after driving a value-change text swap"
}
