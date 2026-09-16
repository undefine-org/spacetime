/// BUG-201 regression: a nested selector scope inside a keyframes body
/// (`.child { ... }` in @scroll/@mouse/@load/@on) must compile into
/// `anims.scopes` and actually ANIMATE the child — before the fix, the
/// %capture_type keyframes grammar had no scope production, so nested blocks
/// were silently dropped at capture (emit: `properties: []`, nothing
/// animated, no diagnostic). Runtime support for scopes existed all along
/// (apply-animations.st iterates anims.scopes) — only the capture/serialize
/// layer lost them.
///
/// Fix (PLAN-078): the keyframes grammar gained a recursive scope arm
/// (`$selector:selector "{" $scope:keyframes "}"`, tried FIRST so an element
/// selector is never eaten by the property arm's balanced run);
/// reify_keyframes tags scope entries (KeyframeDef.selector); the
/// CapturedValue::Keyframes serializer groups them into anims.scopes.
/// Verified via inspect: `.parent { @scroll d { opacity: 0 -> 1; .child {
/// rotate-x: ... } } }` now emits BOTH `properties: [opacity]` AND
/// `scopes: [{ selector: ".child", properties: [rotate-x] }]`.
///
/// Drive mechanism: ST.set(el, 'driver', progress) — the same direct-signal
/// drive the BUG-192 pin uses (headless has no scroll layout).

@version 2026-07-26;

@import "stdlib/testing/test"

.b201-parent {
    @on &.scroll(name: b201-driver, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        .b201-child {
            translate-x: 0 -> 100px;
        }

      }
}

.b201-mouse-parent {
    @on &.mouse(name: b201-tilt, axis: x, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .b201-mouse-child {
            rotate: -10deg -> 0 -> 10deg;
        }

      }
}

@test "nested-scope keyframes animate the CHILD element via @scroll (BUG-201)" {
    @fixture {
        <div class="b201-parent"><span class="b201-child">x</span></div>
    }
    @wait 100ms
    @then .b201-parent should have_timeline "b201-driver"
    @eval (ST.set(document.querySelector('.b201-parent'), 'b201-driver', 0.5))
    @wait 100ms
    @let (childT = document.querySelector('.b201-child').style.transform)
    @assert (childT && childT !== "" && childT !== "none")
        "the nested .b201-child scope must animate (translate-x at progress 0.5), not be dropped at capture"
    @let (parentVar = document.querySelector('.b201-parent').style.getPropertyValue('--st-b201-driver-opacity'))
    @assert (parentVar && parentVar !== "")
        "control: the root-level opacity keyframe must still apply alongside the nested scope"
}

@test "nested-scope keyframes animate the CHILD element via @mouse (BUG-201)" {
    @fixture {
        <div class="b201-mouse-parent" style="position:fixed;top:0;left:0;width:200px;height:200px;"><span class="b201-mouse-child">x</span></div>
    }
    @wait 100ms
    @eval (ST.set(document.querySelector('.b201-mouse-parent'), 'b201-tilt', 0.75))
    @wait 100ms
    @let (childT = document.querySelector('.b201-mouse-child').style.transform)
    @assert (childT && childT !== "" && childT !== "none")
        "the nested .b201-mouse-child scope must animate (rotate at progress 0.75), not be dropped at capture"
}
