//! Every capture type a form declares must actually CHECK the value.
//!
//! PLAN-136 W1. A form parameter carries a type — `stagger: $stagger:number`,
//! `open: $open:bool` — and until this wave 84% of those declarations (344 of
//! 409) were decoration. Measured before the fix:
//!
//!     number  <- 60   Y   60ms Y   #e8eef7 Y   abc Y
//!     bool    <- true Y   60ms Y   #e8eef7 Y   abc Y
//!     ident   <- x    Y   60ms Y   #e8eef7 Y   abc Y
//!
//! Every capture type backed by a Rust `CaptureType` enum arm accepted
//! EVERYTHING, because the arm resolves to a permissive extractor rather than a
//! grammar. Only the types PLAN-122 moved to stdlib (`color`, `length`, `time`,
//! `duration`, `easing`) discriminated at all.
//!
//! That is why `easing: not-a-curve` shipped: 1,010 corpus `easing` declarations
//! are validated by `:ident`, and `:ident` cannot fail. The annotation was
//! never the thing doing the checking.
//!
//! These tests are the contract that the annotation MEANS something. They are
//! written to fail against the pre-wave compiler.

use spacetime::syntax::events::capture_type_accepts;

/// `number` is a number. It was accepting `#e8eef7`.
#[test]
fn number_refuses_things_that_are_not_numbers() {
    // `+2` and `1e3` are deliberately absent. The lexer emits neither a
    // signed-positive NUMBER nor an exponent form, and no `.st` anywhere in the
    // corpus (stdlib, demos, examples, projects) writes either — checked, not
    // assumed. Asserting them here would invent a requirement this wave did not
    // set out to meet, and the honest place to widen the number syntax is the
    // lexer, with its own gate.
    for ok in ["60", "6.5", "-3", "0", ".5"] {
        assert!(
            capture_type_accepts("number", ok),
            "`number` must accept `{ok}`"
        );
    }
    for bad in ["60ms", "8px", "#e8eef7", "abc", "ease-out", "60zz", "50%"] {
        assert!(
            !capture_type_accepts("number", bad),
            "`number` must REFUSE `{bad}` — a dimension is not a number, and \
             accepting it is why `stagger: $stagger:number` checked nothing"
        );
    }
}

/// `bool` is two words. It was accepting `abc`.
#[test]
fn bool_refuses_things_that_are_not_booleans() {
    for ok in ["true", "false"] {
        assert!(capture_type_accepts("bool", ok), "`bool` must accept `{ok}`");
    }
    for bad in ["60ms", "abc", "#e8eef7", "1", "0", "yes", "no", "True"] {
        assert!(
            !capture_type_accepts("bool", bad),
            "`bool` must REFUSE `{bad}`"
        );
    }
}

/// `ident` is an identifier — not a dimension, not a hex colour, not a string.
#[test]
fn ident_refuses_things_that_are_not_identifiers() {
    for ok in ["ease-out", "linear", "visible", "auto", "_x"] {
        assert!(
            capture_type_accepts("ident", ok),
            "`ident` must accept `{ok}`"
        );
    }
    for bad in ["60ms", "#e8eef7", "\"quoted\"", "8px", "50%"] {
        assert!(
            !capture_type_accepts("ident", bad),
            "`ident` must REFUSE `{bad}`"
        );
    }
}

/// The scalars PLAN-122 already moved must not regress while their neighbours
/// move. This is the control: if these break, the move broke the shared road
/// rather than the three types it was aimed at.
#[test]
fn migrated_scalars_still_discriminate() {
    assert!(capture_type_accepts("color", "#e8eef7"));
    assert!(!capture_type_accepts("color", "#e8ee1"));
    assert!(capture_type_accepts("time", "60ms"));
    assert!(!capture_type_accepts("time", "abc"));
    assert!(capture_type_accepts("length", "8px"));
    assert!(!capture_type_accepts("length", "8zz"));
    assert!(capture_type_accepts("easing", "ease-out"));
    assert!(!capture_type_accepts("easing", "not-a-curve"));
}

/// An unregistered type is NOT ours to refuse — silence is the honest answer,
/// and it is what keeps a project's own `%capture_type` from being rejected by
/// a compiler that has not loaded it yet.
#[test]
fn an_unknown_capture_type_accepts_anything() {
    assert!(capture_type_accepts("no_such_type_exists", "literally anything"));
}
