//! The escape list, as a relation — gradual typing's consistency (Siek & Taha).
//!
//! `validate_declaration_value` opened with six hand-written branches asking
//! "is this Spacetime rather than CSS?" by inspecting characters:
//!
//! ```ignore
//! if value.contains('$') || value.contains('`') || value.contains('&')
//!     || value.contains("->") || value.starts_with('%') || value.starts_with('@')
//! { return None; }
//! if is_token_reference(value) { return None; }
//! if matches!(folded, "inherit" | "initial" | ...) { return None; }
//! if folded.contains("var(") || folded.contains("env(") { return None; }
//! ```
//!
//! Each branch was added when something legal got refused. Each is correct.
//! Together they are a hand-maintained list of "things that are not CSS", and
//! ~55 sites across the compiler keep their own version of it.
//!
//! The replacement asks ONE question of the stdlib: does this value match the
//! `deferred` capture type? That production names the arms whose value is
//! unknowable at build time — a token reference, an `env()`, a binding — and it
//! lives in `stdlib/capture-types/value-node.st` as DATA, so a seventh
//! deferred shape is a paragraph there, not a `||` here.
//!
//! THE SEMANTICS, and why a false refusal becomes impossible: a deferred value
//! is `Unknown`, and Unknown is CONSISTENT with every type — admitted
//! everywhere, claiming nothing anywhere. Consistency is deliberately NOT
//! transitive, so `Unknown ~ length` and `Unknown ~ color` do not license
//! `length ~ color`. Unknown is therefore not a lattice top: top would CLAIM
//! everything.

use spacetime::validation::css::validate_declaration_value;

/// Every deferred shape is admitted on a typed property. These are the cases
/// the escape list existed to let through.
#[test]
fn a_deferred_value_is_admitted_on_any_typed_property() {
    for value in [
        "--ink",                      // bare token reference (preferred spelling)
        "var(--ink)",                 // the CSS spelling
        "var(--ink, #222)",           // with a fallback
        "var(--a, var(--b, red))",    // nested fallback
        "env(safe-area-inset-top)",   // user-agent substitution
        "$brand.ink",                 // a Spacetime binding
        "inherit",                    // CSS-wide keyword, legal on every property
        "INHERIT",                    // keywords are ASCII case-insensitive
    ] {
        assert!(
            validate_declaration_value("color", value).is_none(),
            "`color: {value}` must be admitted — its value is not knowable here, \
             and refusing it would assert something we cannot know"
        );
    }
}

/// A range is a keyframe pair, never a CSS value: `->` means TRANSITION
/// everywhere in the language.
#[test]
fn an_animation_range_is_not_a_css_value() {
    assert!(validate_declaration_value("opacity", "0 -> 1").is_none());
    assert!(validate_declaration_value("filter", "blur(18px) -> blur(0)").is_none());
}

/// The admission is the deferred SHAPE, not a blanket pass. If this ever goes
/// green-by-accident the whole diagnostic is worthless, so it is the test that
/// gives the others meaning.
#[test]
fn a_concrete_wrong_literal_is_still_refused() {
    assert!(
        validate_declaration_value("color", "notacolor").is_some(),
        "a concrete literal that fails its grammar must still be refused"
    );
}

/// One dash is not a token reference — `-ink` is an identifier, and admitting
/// it would widen the hole beyond the rule.
#[test]
fn one_dash_is_not_deferred() {
    assert!(validate_declaration_value("color", "-ink").is_some());
}

/// An unclosed `var(` is an error, not a deferral: the shape itself is broken,
/// which is knowable here.
#[test]
fn an_unclosed_substitution_is_not_admitted() {
    assert!(
        validate_declaration_value("color", "var(--ink").is_some(),
        "the parens must balance — an unclosed var() is a broken shape, not a deferred value"
    );
}

// ── WAVE 2: the lexical escape check is gone ──────────────────────────────
//
// `validate_declaration_value` used to open with a six-sigil `if`:
//
//     if value.contains('$') || value.contains('`') || value.contains('&')
//         || value.contains("->") || value.starts_with('%')
//         || value.starts_with('@') { return None; }
//
// Six characters, one meaning each, checked by substring. It was correct — and
// it was the LAST place in the validator that answered "is this Spacetime?" by
// inspecting bytes rather than by asking the grammar. Every arm it covered is
// now a production in `value-node.st`, so the question is asked once, of the
// stdlib, and a seventh Spacetime form is a paragraph there rather than another
// `||` here.
//
// These gates assert the BEHAVIOUR is preserved exactly. Each value below was
// admitted by the old `if` and must still be admitted — by grammar now. A
// regression here is a FALSE REFUSAL, which blocks a build the author cannot
// fix, and is strictly worse than a missed diagnostic.

fn admitted(property: &str, value: &str) -> bool {
    spacetime::validation::css::validate_declaration_value(property, value).is_none()
}

#[test]
fn a_binding_is_admitted_without_a_lexical_check() {
    assert!(admitted("color", "$brand.ink"));
    assert!(admitted("width", "$w"));
}

#[test]
fn a_hole_is_admitted_without_a_lexical_check() {
    assert!(admitted("color", "`$c.id`"));
    assert!(admitted("width", "`w`"));
}

#[test]
fn an_element_reference_is_admitted_without_a_lexical_check() {
    assert!(admitted("color", "&self"));
}

#[test]
fn a_transition_range_is_admitted_without_a_lexical_check() {
    assert!(admitted("opacity", "0 -> 1"));
    assert!(admitted("filter", "blur(18px) -> blur(0)"));
}

#[test]
fn a_directive_reference_is_admitted_without_a_lexical_check() {
    assert!(admitted("animation", "@fade-in"));
    assert!(admitted("color", "%something"));
}

/// And the thing the whole exercise is FOR: a real CSS error is still caught.
/// If removing the lexical check also removed the diagnostics, the function
/// would be a very fast `return None`.
#[test]
fn a_real_css_error_is_still_refused() {
    assert!(!admitted("color", "notacolor"));
}

// ── COMPOUND values: a Spacetime node INSIDE a CSS value ──────────────────
//
// `1px solid $brand.rule` is CSS shorthand whose colour slot is a binding. The
// old lexical check caught it by accident — `contains('$')` does not care where
// the sigil sits. A grammar that only matches a value STARTING with a binding
// routes it to `css`, and lightningcss then refuses it: seven E0958s across the
// demos on the first run of the grammar-only path.
//
// That is a FALSE REFUSAL — it blocks a build the author cannot fix — and it is
// strictly worse than the missed diagnostic it was meant to prevent. So the
// grammar has to say what the lexical check knew: a Spacetime node ANYWHERE in
// a value makes the whole value not-CSS.

#[test]
fn a_binding_inside_a_shorthand_is_admitted() {
    assert!(admitted("border", "1px solid $brand.rule"));
    assert!(admitted("border-top", "2px solid $brand.violet"));
}

#[test]
fn a_token_inside_a_shorthand_is_admitted() {
    assert!(admitted("border", "1px solid --ink"));
}

/// IGNORED, and the reason is a grammar limitation, not a decision.
///
/// A compound hole (`border: 1px solid `$c.tint``) cannot be written as a
/// production today. Measured while trying: `balanced('`')` causes the
/// production CONTAINING it to be dropped from the registry entirely (its
/// neighbours survive, so it is silent), and a `"`"` literal placed AFTER a
/// capture destabilises the pattern's own parse. A literal backtick alone works
/// — that is how `hole` itself is written — so the defect is narrow and lives
/// in the capture-type pattern parser. Filed as FUP-194.
///
/// Kept rather than deleted because the SHAPE is right and the corpus may grow
/// one: today all 19 corpus values with a non-leading backtick are diagnostic
/// prose inside quoted strings, so nothing is currently refused. If that
/// changes this test is already here, and it is a false refusal the moment it
/// is real.
#[test]
#[ignore = "FUP-194: balanced('`') drops its own production; a trailing backtick literal destabilises the parse"]
fn a_hole_inside_a_shorthand_is_admitted() {
    assert!(admitted("border", "1px solid `$c.tint`"));
}

/// The precision guard. Admitting compounds must not admit everything — a
/// shorthand with no Spacetime node in it is still judged.
#[test]
fn a_shorthand_with_no_spacetime_node_is_still_judged() {
    assert!(!admitted("border", "1px solid notacolor"));
}
