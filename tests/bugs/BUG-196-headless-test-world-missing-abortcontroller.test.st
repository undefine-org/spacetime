/// BUG-196: the headless V8 test world (src/headless_shims.js + the LinkeDOM
/// sandbox) does not define `AbortController`. Every interactive animation
/// driver constructs one at the top of its init --
/// stdlib/primitives/animation/drivers.st mouse-driver L426
/// (`const controller = new AbortController();`), and the hover/focus/click
/// trigger drivers further down (L571-581) attach their listeners with
/// `{ signal: controller.signal }`. In a headless test the init therefore
/// THROWS at the AbortController line -- AFTER the runtime's selector-init
/// has already set the element's `_st_init_<hash>` flag -- so the driver
/// silently never attaches its listener and never yields its initial
/// progress. No error surfaces in the test output (the selector-init scanner
/// swallows the exception); the animation simply never happens, and because
/// an unset inline style in LinkeDOM reads back as `undefined` (not ""),
/// naive `value !== ""` assertions PASS VACUOUSLY on top of the breakage.
///
/// Confirmed this session with direct probes:
///   - `el.addEventListener('mousemove', fn, {signal: new AbortController()
///     .signal})` throws "AbortController is not defined" in the test world.
///   - With the flag set but the driver dead, no ST.set call ever fires and
///     no --st-<driver>-* CSS variable is ever written.
///   - Polyfilling AbortController BEFORE @fixture insertion (so it is in
///     place when the selector-init scan runs) fully unbreaks the chain:
///     mousemove then drives progress 0 -> 1 and the transform lands
///     (`--st-pac-tilt:1; transform:translateX(100px)` observed).
///
/// This gap stayed invisible because the existing @mouse test suite
/// (tests/unit/animations/mouse-tilt.test.st) is ALSO vacuous for an
/// independent reason -- its keyframe directives sit inside the @test bodies
/// and get compiled away entirely (BUG-194). Two silent failures stacked.
///
/// Related headless-shim gaps found in the same audit (fix together in
/// headless_shims.js): `window.matchMedia` is unshimmed (the reveal engine
/// throws on its first line without an author shim -- see BUG-190's test),
/// and `performance.now()` is epoch-scale rather than time-origin-relative
/// (harness artifact noted in BUG-191's authoring).
///
/// Expected RED until headless_shims.js ships an AbortController (and
/// AbortSignal) polyfill. BUG-195's tests carry the workaround inline and
/// can drop their first-step polyfill once this lands.

@import "stdlib/testing/test"

@test "the headless test world provides AbortController (RED, BUG-196)" {
    @let (hasIt = typeof AbortController)
    @assert (hasIt === "function")
        "headless_shims.js must define AbortController -- every interactive driver (mouse/hover/focus/click) constructs one in init and silently dies without it (BUG-196)"
}

@test "CONTROL: with a manual polyfill, addEventListener accepts the {signal} option (must stay green)" {
    @eval (eval("if (typeof AbortController === 'undefined') { window.AbortController = function() { this.signal = { aborted: false }; }; AbortController.prototype.abort = function() { this.signal.aborted = true; }; }"))
    @fixture {
        <div class="bug196-target" style="width: 100px; height: 100px;">x</div>
    }
    @wait 50ms
    @eval (eval("var c = new AbortController(); var el = document.querySelector('.bug196-target'); el.addEventListener('mousemove', function() { window.__bug196Fired = true; }, {signal: c.signal}); el.dispatchEvent(new MouseEvent('mousemove', {bubbles: true}))"))
    @wait 50ms
    @let (fired = window.__bug196Fired)
    @assert (fired === true)
        "control: once AbortController exists, addEventListener with {signal} must register and fire -- this is the exact call shape every interactive driver uses"
}
