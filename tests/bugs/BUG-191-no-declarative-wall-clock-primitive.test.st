/// ENHANCEMENT/GAP: no declarative live wall-clock (`now`/`clock`) primitive
/// exists — a ticking "current time" UI element (a clock, a "last updated"
/// stamp, a countdown, a timezone display) cannot be authored without
/// hand-written JS, which is forbidden by the project's zero-JS authoring
/// rule outside `%primitive`/`@test` bodies (see AGENTS.md decisionmaking).
///
/// What exists today (confirmed by reading source):
///   - `stdlib/primitives/tick.st`'s `tick` primitive yields `$t` / `$dt` /
///     `$frame` / `$elapsed` — all derived from `performance.now()`
///     (monotonic, frame-relative time since page load), NOT wall-clock time.
///     `grep -c "Date.now\|new Date" stdlib/primitives/tick.st` → 0 hits.
///   - `tick.st`'s `interval` primitive yields `$count` — a FIRE COUNT, not
///     the current time.
///   - `Date` IS used internally elsewhere (`stdlib/primitives/state.st`,
///     `fetch.st`, `data/source.st`) but never surfaced as a user-bindable
///     reactive time signal.
///   - There is no `@clock`/`@now`/`clock`/`now` primitive or directive
///     registered anywhere in `stdlib/primitives/` or `stdlib/macros/`.
///
/// NOTE: `@clock` is ALREADY a name in use — the TESTING DSL's virtual-time
/// control directive (`stdlib/testing/timing.st`: `@clock install` / `@clock
/// advance` / `@clock sample` / `@clock run-to-end`). A production wall-clock
/// primitive MUST NOT collide with that name; the enhancement issue's own
/// suggested spelling (`@clock(tz:.., every:..)`) would shadow the test DSL's
/// `@clock` if both were ever imported into the same file. This test uses
/// `@now` as the working name to avoid that collision — the actual fix
/// should pick a non-colliding name (`@now`, `live-clock`, etc.) as part of
/// its design, not necessarily `@clock`.
///
/// This test pins the GAP: an unknown `@now(...)` at-rule currently compiles
/// as an inert unknown-directive passthrough (verified — `cargo run --
/// check` accepts it silently, no error) rather than as a registered
/// primitive, so a live-updating time display is unreachable declaratively.
/// Expected RED (never produces live wall-clock text) until a `now`/`clock`
/// stdlib primitive is added (see BUG-191.org / the GitHub enhancement).

@import "stdlib/testing/test"

@test "an unknown @now(...) directive compiles without error (control — confirms silent passthrough, not a parse failure)" {
    @mount {
        <span class="now-compile-check">placeholder</span>
        .now-compile-check {
            @now(every: 1s) {
                text <- $now;
            }
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.now-compile-check'))
    @assert (el !== null)
        "sanity: the element itself must still exist (an unknown directive must not break the surrounding page)"
}

@test "GAP: no primitive yields a live, reactive wall-clock value (RED, BUG-191)" {
    @mount {
        <span class="now-display">--:--:--</span>
        .now-display {
            @now(every: 1s) {
                text <- $now;
            }
        }
    }
    @wait 30ms
    @let (el = document.querySelector('.now-display'))
    @let (t = el.textContent)
    @assert (t !== '--:--:--')
        "a live-clock primitive must overwrite the static placeholder with a real time value"
}

// NB: a planned third "control" test asserting tick's $t/performance.now()
// basis is page-relative, not epoch, was DROPPED after empirical probing this
// session: this headless V8 test runner's performance.now() returns an
// EPOCH-SCALE value, unlike a real browser's page-relative performance.now()
// -- a harness artifact, not evidence about the real primitive. The static
// SOURCE evidence in this file's header comment (zero Date.now()/new Date()
// hits in tick.st, confirmed via grep) is the reliable proof that tick's $t
// is not wall-clock time; kept as prose rather than a flaky runtime assertion.
