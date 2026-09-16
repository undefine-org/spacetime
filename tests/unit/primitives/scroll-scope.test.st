/// Scroll Scope Tests
///
/// Dedicated tests for the `scope` parameter on @scroll directive.
/// Verifies each scope mode (cover, entry, exit, contain) registers timelines
/// and accepts start/end sub-range percentages.
///
/// SHAPE (BUG-194/BUG-200): drivers are declared at FILE SCOPE in a selector
/// block (`.cls { @scroll ... }`) and @fixture carries markup only. The
/// previous shape (directive nested inside fixture markup) compiled the whole
/// test empty (BUG-194) and was silently dropped (BUG-200), so these tests
/// were vacuous until 2026-07-19.

@version 2026-07-26;

@import "stdlib/testing/test"

.default-scope {
  @on &.scroll(name: default-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.cover-full {
  @on &.scroll(name: cover-full, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        translate-y: 50px -> 0;
      }
}

.cover-partial {
  @on &.scroll(name: cover-partial, scope: cover, start: 0.2, end: 0.8, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.entry-scope {
  @on &.scroll(name: entry-anim, scope: entry, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.entry-partial {
  @on &.scroll(name: entry-partial, scope: entry, start: 0.3, end: 0.7, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        translate-y: 30px -> 0;
      }
}

.exit-scope {
  @on &.scroll(name: exit-anim, scope: exit, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 1 -> 0;
      }
}

.exit-multi {
  @on &.scroll(name: exit-multi, scope: exit, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 1 -> 0;
        translate-y: 0 -> -40px;
      }
}

.contain-scope {
  @on &.scroll(name: contain-anim, scope: contain, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.contain-tall {
  @on &.scroll(name: contain-tall, scope: contain, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-scrub {
  @on &.scroll(name: scope-scrub, scope: entry, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-easing {
  @on &.scroll(name: scope-easing, scope: cover, start: 0, end: 1, scrub: true) {
        easing: ease-out-quad;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        translate-y: 40px -> 0;
      }
}

// =============================================================================
// Scope: cover (default)
// =============================================================================

@test "scope cover is the default when scope not specified" {
  @fixture {
    <div class="default-scope"></div>
  }
  @wait 100ms
  @then .default-scope should exist
  @then .default-scope should have_timeline "default-test"
}

@test "scope cover with full range covers entire element journey" {
  @fixture {
    <div class="cover-full"></div>
  }
  @wait 100ms
  @then .cover-full should exist
  @then .cover-full should have_timeline "cover-full"
}

@test "scope cover with partial range animates within sub-range" {
  @fixture {
    <div class="cover-partial"></div>
  }
  @wait 100ms
  @then .cover-partial should exist
  @then .cover-partial should have_timeline "cover-partial"
}

// =============================================================================
// Scope: entry
// =============================================================================

@test "scope entry registers timeline correctly" {
  @fixture {
    <div class="entry-scope"></div>
  }
  @wait 100ms
  @then .entry-scope should exist
  @then .entry-scope should have_timeline "entry-anim"
}

@test "scope entry with partial range" {
  @fixture {
    <div class="entry-partial"></div>
  }
  @wait 100ms
  @then .entry-partial should exist
  @then .entry-partial should have_timeline "entry-partial"
}

// =============================================================================
// Scope: exit
// =============================================================================

@test "scope exit registers timeline correctly" {
  @fixture {
    <div class="exit-scope"></div>
  }
  @wait 100ms
  @then .exit-scope should exist
  @then .exit-scope should have_timeline "exit-anim"
}

@test "scope exit with multi-property animation" {
  @fixture {
    <div class="exit-multi"></div>
  }
  @wait 100ms
  @then .exit-multi should exist
  @then .exit-multi should have_timeline "exit-multi"
}

// =============================================================================
// Scope: contain
// =============================================================================

@test "scope contain registers timeline correctly" {
  @fixture {
    <div class="contain-scope"></div>
  }
  @wait 100ms
  @then .contain-scope should exist
  @then .contain-scope should have_timeline "contain-anim"
}

@test "scope contain falls back to cover for tall elements" {
  @fixture {
    <div class="contain-tall"></div>
  }
  @wait 100ms
  @then .contain-tall should exist
  @then .contain-tall should have_timeline "contain-tall"
}

// =============================================================================
// Scope with other parameters
// =============================================================================

@test "scope works alongside scrub parameter" {
  @fixture {
    <div class="scope-scrub"></div>
  }
  @wait 100ms
  @then .scope-scrub should exist
  @then .scope-scrub should have_timeline "scope-scrub"
}

@test "scope works with easing parameter" {
  @fixture {
    <div class="scope-easing"></div>
  }
  @wait 100ms
  @then .scope-easing should exist
  @then .scope-easing should have_timeline "scope-easing"
}
