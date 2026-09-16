/// var() Resolution in Scroll Animations
///
/// SHAPE (BUG-194/BUG-200): drivers at file scope in selector blocks;
/// @fixture carries markup only; @wait for the async selector-init scan
/// before timeline assertions. The fixture-nested form was silently dropped.

@version 2026-07-26;

@import "stdlib/testing/test"

.scroll-var {
  @on &.scroll(name: scroll-var-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 1 -> var(--fade-to);
      }
}

.scroll-color-var {
  @on &.scroll(name: color-var-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                color: #FFFFFF -> var(--text-target);
      }
}

@test "scroll animation with var() endpoint registers timeline" {
  @fixture {
    <div class="scroll-var" style="--fade-to: 0.5"></div>
  }
  @wait 100ms
  @then .scroll-var should exist
  @then .scroll-var should have_timeline "scroll-var-test"
}

@test "scroll animation with var() color endpoint registers" {
  @fixture {
    <div class="scroll-color-var" style="--text-target: #1A1A1A"></div>
  }
  @wait 100ms
  @then .scroll-color-var should exist
  @then .scroll-color-var should have_timeline "color-var-test"
}
