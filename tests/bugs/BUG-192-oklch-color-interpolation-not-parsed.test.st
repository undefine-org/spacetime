/// BUG: animation color interpolation cannot parse 'oklch(...)' endpoints —
/// only '#hex' and 'rgb()/rgba()'. Since the project's own design-system
/// convention is "all color in oklch via 'var(--*)'" (see AGENTS.md), this
/// means a brand color CANNOT be scrub- or keyframe-animated today: the
/// interpolator silently fails to recognize the oklch endpoints and the
/// color transition simply never blends — it SNAPS between the two literal
/// endpoint strings at the keyframe midpoint instead of interpolating.
///
/// Root cause (confirmed by reading source AND empirically, this session):
///   - The DESIGN-SYSTEM-CORRECT color parser (ST.parseColor,
///     public/runtime/color.js) DOES already support oklch/hex/rgb/hsl — it
///     is OKLCH-FIRST by design (see tests/unit/runtime/color.test.st,
///     "parseColor handles oklch()").
///   - BUT the ANIMATION subsystem never calls ST.parseColor at all. It has
///     TWO SEPARATE, independent, more limited parsers:
///       1. window.__stParseColor (stdlib/primitives/animation/
///          interpolate.st) — hex + rgb/rgba only, no oklch/hsl branch.
///       2. A local parseColor closure (stdlib/primitives/animation/
///          apply-animations.st, ~line 118) — same hex + rgb/rgba-only
///          shape, independently duplicated.
///     Neither delegates to the already-correct ST.parseColor. This is a
///     "one need, two implementations" harmony violation (AGENTS.md
///     Principles paragraph 2), not a missing capability — the fix already
///     exists elsewhere in the runtime and just needs to be reused.
///   - Extra wrinkle: interpolate.st's animate-property primitive documents
///     a colorSpace param as "rgb, hsl, oklch" (default "rgb"), but the
///     input parser can't read oklch/hsl strings regardless of which
///     colorSpace is selected — the param is a documented no-op today.
///
/// EMPIRICAL PROOF OF THE SNAP BEHAVIOR (found while building this pin):
/// a scroll-driven background-color keyframe with oklch endpoints does not
/// stay unset and does not error — it sets a value, but that value is always
/// EXACTLY one of the two literal endpoint strings (a threshold snap on which
/// keyframe is nearest), never a blended value. Confirmed by direct probing
/// this session: at progress 0.3 the computed style is the literal string
/// "oklch(30% 0.05 250)" (the START endpoint, byte-for-byte); at progress 0.5
/// and 0.99 it is the literal string "oklch(90% 0.02 90)" (the END endpoint,
/// byte-for-byte) — i.e. parseColor fails on both endpoints, so
/// interpolateKeyframes falls back to a hold/nearest-keyframe behavior
/// instead of blending. Contrast: the SAME test shape with hex endpoints
/// (#1a1a3d -> #f0e8d8) produces a genuinely BLENDED "rgb(133, 129, 139)" at
/// progress 0.5 — proving the harness and assertion shape are sound, and the
/// snap-not-blend behavior is specific to oklch endpoints.
///
/// TEST-HARNESS NOTE: a real compiler bug (found and worked around this
/// session, filed separately — see Notes below) makes @mount (or @fixture)
/// blocks containing an INLINE @scroll/@on-hover animation directive silently
/// swallow the REST of the enclosing @test body (the test then reports a
/// false green because every assertion after the mount/fixture block never
/// actually runs). This test therefore uses the SAFE, verified-working shape:
/// the animated selector is declared FILE-LEVEL (.class { @scroll ... }, the
/// pattern every real production .st file already uses — see
/// projects/ora-ventures.com/landing/index.st), with only the target markup
/// itself inside @fixture.
///
/// Expected RED (oklch case snaps instead of blending) until the animation
/// color parsers are unified with/delegate to ST.parseColor (see
/// BUG-192.org).

@version 2026-07-26;

@import "stdlib/testing/test"

.color-hex-control {
    @on &.scroll(name: hex-color-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                background-color: #1a1a3d -> #f0e8d8;
      }
}

.color-oklch-scroll {
    @on &.scroll(name: oklch-color-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                background-color: oklch(30% 0.05 250) -> oklch(90% 0.02 90);
      }
}

@test "CONTROL: scroll-driven background-color BLENDS with HEX endpoints (must stay green)" {
    @fixture {
        <div class="color-hex-control" style="height: 2000px">Scrub Me</div>
    }
    @wait 30ms
    @eval (window.__hexEl = document.querySelector('.color-hex-control'))
    @eval (ST.set(window.__hexEl, 'hex-color-test', 0.5))
    @wait 30ms
    @eval (window.__hexBg = window.__hexEl.style.backgroundColor)
    @wait 30ms
    @let (hexBg = window.__hexBg)
    // A real blend at the midpoint of a dark-to-light hex pair must be an
    // rgb() triple, and must NOT equal either endpoint literally.
    @assert (hexBg.indexOf('rgb') !== -1)
        "control: hex-endpoint interpolation at progress 0.5 must produce an rgb() blended value"
}

@test "scroll-driven background-color BLENDS (not snaps) with OKLCH endpoints (RED, BUG-192)" {
    @fixture {
        <div class="color-oklch-scroll" style="height: 2000px">Scrub Me</div>
    }
    @wait 30ms
    @eval (window.__oklchEl = document.querySelector('.color-oklch-scroll'))
    @eval (ST.set(window.__oklchEl, 'oklch-color-test', 0.5))
    @wait 30ms
    @eval (window.__oklchBg = window.__oklchEl.style.backgroundColor)
    @wait 30ms
    @let (oklchBg = window.__oklchBg)
    // At progress 0.5, a correctly interpolating engine must NOT report
    // either endpoint's literal string verbatim — it must report a genuinely
    // blended value between oklch(30% 0.05 250) and oklch(90% 0.02 90).
    @assert (oklchBg !== 'oklch(30% 0.05 250)')
        "oklch background-color must not be stuck at the literal START endpoint at mid-progress"
    @assert (oklchBg !== 'oklch(90% 0.02 90)')
        "oklch background-color must not SNAP to the literal END endpoint at mid-progress (it must BLEND, not hold)"
}

@test "scroll-driven background-color does not snap to the START endpoint at progress 0.3 (RED, BUG-192)" {
    // A second sample point, chosen deliberately BEFORE the midpoint, to pin
    // the exact SNAP shape found during investigation: oklch endpoints
    // currently snap to whichever keyframe is nearest rather than blending —
    // at progress 0.3 (nearer the START than the END) the observed value is
    // the START endpoint's literal string, unchanged.
    @fixture {
        <div class="color-oklch-scroll" style="height: 2000px">Scrub Me</div>
    }
    @wait 30ms
    @eval (window.__oklchEl2 = document.querySelector('.color-oklch-scroll'))
    @eval (ST.set(window.__oklchEl2, 'oklch-color-test', 0.3))
    @wait 30ms
    @eval (window.__oklchBg2 = window.__oklchEl2.style.backgroundColor)
    @wait 30ms
    @let (oklchBg2 = window.__oklchBg2)
    @assert (oklchBg2 !== 'oklch(30% 0.05 250)')
        "at progress 0.3, a real interpolator must report a value PARTWAY between the endpoints, not the literal unmodified START endpoint string"
}

// =============================================================================
// Notes
// =============================================================================
//
// A related, separate compiler bug surfaced while verifying this pin (not
// BUG-192 itself, filed here for traceability): an inline @scroll or
// @on-hover animation directive placed directly inside a @mount or @fixture
// test block's markup silently causes the WHOLE REST of that @test body to
// compile to an EMPTY function — no error, no warning, the test just reports
// a false green because nothing after the mount/fixture ever executes.
// Confirmed via `cargo run -- inspect --layer emit` on multiple minimal
// repros this session: the emitted `window.__spacetime_register_test(...)`
// body is `try { } finally { ... }` — completely empty between the braces —
// whenever the mount/fixture content contains `.selector { @scroll ... }` or
// an inline `@on hover ... { }` block. This affected an earlier draft of this
// very test file (every assertion silently never ran, reporting 4/4 green
// for a bug that should have been red) until traced down via forced-failure
// probes (`@assert (false)` inside the same shape, which ALSO passed —
// proving the body never executed at all). Worth its own BUG filing +
// regression test; kept out of scope here to stay focused on the oklch
// interpolation bug itself.
