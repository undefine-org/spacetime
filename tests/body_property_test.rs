//! A property inside a directive BODY is checked like one in its parens.
//!
//! PLAN-136 W4c. This is the surface FUP-181 was actually about, and W4b's
//! wiring proved it is the only one still open:
//!
//! ```text
//! @reveal(once: banana)              -> E0946   the matcher already refuses this
//!
//! @on &.hover(name: h) {
//!   easing: not-a-curve;             -> PASSES  and so does `60ms`
//! }
//! ```
//!
//! A parenthesized param is a `FormParam` with a declared capture type, so the
//! form matcher enforces it — that half was never broken. A BODY property is
//! not. It is captured by
//!
//!     %capture_type motion_line { $prop:ident ":" $value:balanced(';') ";"? }
//!
//! where the value is `balanced(';')` — a run of tokens up to the semicolon,
//! deliberately shapeless because one grammar must serve `easing:`,
//! `translate-x:`, `opacity: 0 -> 1` and every CSS property at once. Nothing
//! downstream then asks what `easing` should have been.
//!
//! That is why 1,010 corpus `easing` declarations are validated by nothing, and
//! why the fix cannot be "give motion_line a stricter type": the line grammar is
//! right to be permissive. The type question belongs one level up, where the
//! DIRECTIVE is known and W3's derived map can answer it.

use spacetime::validation::css::body_property_diagnostics;

/// The headline: a body property whose value fails its declared type is caught.
#[test]
fn a_body_property_is_checked_against_its_declared_type() {
    // @fade-in-up declares `easing: $easing:easing` — the real grammar
    // (stdlib/macros/fade-in.st:54). Its sibling @fade-in declares `:ident`,
    // which accepts any identifier, so `not-a-curve` legitimately passes there
    // until W5 tightens it. Asserting against @fade-in would be asserting a fix
    // this wave has not made.
    let diags = body_property_diagnostics("fade-in-up", &[("easing", "not-a-curve")]);
    assert_eq!(
        diags.len(),
        1,
        "`easing: not-a-curve` in a @fade-in-up body must be refused"
    );
    assert!(
        diags[0].message.contains("not-a-curve"),
        "the diagnostic must quote the offending value, got: {}",
        diags[0].message
    );
}

/// And the values that are right must pass, or a validator that refuses
/// everything would score as working.
#[test]
fn a_valid_body_property_passes() {
    for (directive, prop, value) in [
        ("fade-in-up", "easing", "ease-out"),
        ("fade-in-up", "easing", "linear"),
        ("fade-in-up", "easing", "cubic-bezier(.2, 0, 0, 1)"),
        // W2/W4c: a token reference is a value in ANY value position — including
        // one whose declaration is still the weak `:ident`.
        ("fade-in-up", "easing", "--ease-out-expo"),
        ("fade-in-up", "easing", "var(--ease)"),
        ("fade-in", "easing", "--ease-out-expo"),
        ("fade-in", "easing", "var(--ease)"),
    ] {
        let diags = body_property_diagnostics(directive, &[(prop, value)]);
        assert!(
            diags.is_empty(),
            "@{directive} body `{prop}: {value}` must pass, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// The corpus writes shapes no scalar describes. They must stay legal — a false
/// refusal blocks a build over a value that was always correct.
#[test]
fn shapes_no_type_describes_are_left_alone() {
    for (directive, prop, value) in [
        // Compound: delay + origin + grid. No %capture_type covers it.
        ("on", "stagger", "0.05 first"),
        ("on", "stagger", "0.003 grid(13 13) center"),
        // A transition, not a value.
        ("on", "opacity", "0 -> 1"),
        ("on", "translate-y", "26px -> 0"),
        // Plain CSS inside a motion body.
        ("on", "background", "#0b0e14"),
    ] {
        let diags = body_property_diagnostics(directive, &[(prop, value)]);
        assert!(
            diags.is_empty(),
            "@{directive} body `{prop}: {value}` is authored in the corpus and \
             must not be refused, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// A property no form declares is not ours to judge, in a body exactly as in
/// parens.
#[test]
fn an_undeclared_body_property_is_silent() {
    let diags = body_property_diagnostics("fade-in", &[("no-such-prop", "whatever")]);
    assert!(diags.is_empty());

    let diags = body_property_diagnostics("no-such-directive", &[("easing", "not-a-curve")]);
    assert!(diags.is_empty());
}
