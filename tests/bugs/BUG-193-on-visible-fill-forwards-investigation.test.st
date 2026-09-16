/// INVESTIGATION (NOT REPRODUCED): the reported bug — "a one-shot @on visible
/// (or any load-driver-backed) keyframe timeline does NOT hold its end state;
/// once the animation completes, the element reverts to its CSS resting
/// state" — could NOT be reproduced this session, across BOTH the headless
/// V8/LinkeDOM backend AND a real Chromium browser (--cdp).
///
/// Root-cause claim under test (from the original report): application in
/// stdlib/primitives/animation/apply-animations.st is gated on
///   const isActive = progress > 0 && progress < 1;
/// and a completed one-shot driver (progress === 1, therefore NOT active per
/// that condition) is claimed to stop contributing to el._stTransforms,
/// dropping the end-state transform and snapping the element back.
///
/// What this session actually found, reading the SAME code more closely:
/// the isActive gate is an OWNERSHIP-ARBITRATION check, not an
/// apply-or-don't-apply gate --
///   const owner = el._stAnimOwner[normProp];
///   const isActive = progress > 0 && progress < 1;
///   if (owner && owner !== driverSignal && !isActive) return;   // (*)
///   if (isActive || !owner) el._stAnimOwner[normProp] = driverSignal;
///   const value = interpolateKeyframes(prop.keyframes, progress, ...);
///   applyStyle(el, prop.property, value, driverSignal, scopeId);
/// Line (*) only SKIPS a driver that does NOT already own the property AND
/// is inactive (i.e. it prevents a SECOND, inactive driver from clobbering an
/// active first driver's property) -- it does not prevent the driver's OWN
/// completed frame (progress===1) from applying. `applyStyle` (and thus the
/// final progress===1 keyframe value) still runs on every watched update,
/// including the completion frame, for the driver that owns (or is first to
/// touch) the property. There is no unsubscribe-before-final-apply race in
/// the reference chain: load-driver's animate() calls %yield progress -> $name
/// with progress===1 on the LAST frame BEFORE removing itself from the RAF
/// loop, so apply-animations' ST.watch callback fires with progress===1 at
/// least once.
///
/// Empirical confirmation (this session, both backends):
///   - @on visible + opacity/translate-y (0->1 style): holds opacity:1,
///     translateY(0) after completion, sampled at multiple wait intervals up
///     to 2s post-load, on BOTH --headless and --cdp (real Chromium).
///   - @load (load-driver's OTHER macro consumer, same primitive, immediate
///     un-gated firing): same hold behavior confirmed on --cdp.
///   - A FADE-OUT curtain (opacity 1 -> 0, the literal reported use case: "a
///     load-time curtain, a fixed full-screen overlay"): holds opacity:0
///     after completion on --cdp.
///   - Two drivers contending for the same property (@load then a @scroll at
///     progress 0): the completed @load's held value survives a subsequent
///     @scroll update at progress 0 without being clobbered, confirming the
///     ownership-arbitration reading above.
///
/// This test PINS the CORRECTLY-OBSERVED hold behavior as a regression guard
/// (green today; would catch a REAL future regression of this class) and
/// documents the non-reproduction. It intentionally does NOT restate the
/// original issue's exact repro shape as a RED test, because doing so would
/// fabricate a false failure this session's own evidence contradicts --
/// see AGENTS.md discipline: "get evidence first...else label it likely/guess".
///
/// Recommended next step for BUG-193/the GitHub issue: request a live URL
/// (or a minimal projects/sakazuki-derived repro file) from the reporter and
/// re-run under `--cdp`/`@drive` against THAT exact markup+CSS, since the
/// class of bug described (interaction with a SPECIFIC page's own resting
/// CSS, cascade, or a second competing driver/transition not modeled here)
/// may be page-specific rather than a general apply-animations defect.
///
/// BACKEND NOTE: this pin's assertions read real-time-elapsed style state
/// (not a `have_timeline`-style registry check), so it needs the `--cdp` real
/// Chromium backend to observe genuine completion -- the headless V8/LinkeDOM
/// runner's rAF/setTimeout loop does not reliably advance a real-duration
/// animation to completion within bounded `@wait` calls (confirmed this
/// session: identical assertions pass consistently under --cdp, fail
/// nondeterministically under --headless purely on timing, not logic). Run:
///   cargo run --features cdp -- test tests/bugs/BUG-193-on-visible-fill-forwards-investigation.test.st --cdp

@import "stdlib/testing/test"

.regression-guard-693 {
    @on &.visible(200ms) as $regression-693-driver {
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
    }
}

@test "REGRESSION GUARD: @on visible opacity+transform keyframe holds its end state after completion" needs timing {
    @fixture {
        <div class="regression-guard-693" style="position:fixed;top:0;left:0;width:100%;height:100%;"></div>
    }
    @wait 200ms
    @wait 200ms
    @eval (window.__guardEl = document.querySelector('.regression-guard-693'))
    @eval (window.__guardOpacity = window.__guardEl.style.opacity)
    @eval (window.__guardTransform = window.__guardEl.style.transform)
    @wait 30ms
    @let (guardOpacity = window.__guardOpacity)
    @let (guardTransform = window.__guardTransform)
    @assert (guardOpacity === '1')
        "a completed @on visible opacity keyframe must hold its final value (1), not revert to the CSS resting state"
    @assert (guardTransform === 'translateY(0px)' || guardTransform === 'translateY(0)')
        "a completed @on visible transform keyframe must hold its final value, not revert"
}

.regression-guard-fadeout {
    @on &.visible(200ms) as $regression-fadeout-driver {
        opacity: 1 -> 0;
    }
}

@test "REGRESSION GUARD: a FADE-OUT curtain (opacity from 1 to 0) holds at 0 after completion" needs timing {
    // The literal reported use case: "a load-time curtain, a fixed
    // full-screen overlay" -- a curtain typically fades OUT to reveal the
    // page beneath, not in.
    @fixture {
        <div class="regression-guard-fadeout" style="position:fixed;top:0;left:0;width:100%;height:100%;background:black;"></div>
    }
    @wait 200ms
    @wait 200ms
    @eval (window.__fadeEl = document.querySelector('.regression-guard-fadeout'))
    @eval (window.__fadeOpacity = window.__fadeEl.style.opacity)
    @wait 30ms
    @let (fadeOpacity = window.__fadeOpacity)
    @assert (fadeOpacity === '0')
        "a completed fade-out curtain must hold opacity:0 (fully revealed), not revert to opacity:1"
}
