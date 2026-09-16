//! A bare `--token` is a VALUE, in a plain CSS declaration too.
//!
//! Spacetime's own preference is `color: --ink`, not `color: var(--ink)` —
//! `var()` is a CSS construct the language does not need, because `--name`
//! already names a value unambiguously. PLAN-136 W2 established the rule as
//! "a token reference is a value in EVERY value position", and
//! `capture_type_accepts` has honoured it since.
//!
//! But the rule had two enforcement sites and only one guard:
//!
//! | path                              | code  | admits `--ink` |
//! |-----------------------------------|-------|----------------|
//! | `directive_property_diagnostics`  | E0967 | yes (`is_token_reference_value`) |
//! | `validate_declaration_value`      | E0958 | NO             |
//!
//! So a plain declaration got:
//!
//! ```text
//! error[E0958]: `--ink` is not a valid color for `color`
//! ```
//!
//! for a value the language prefers. That is a FALSE REFUSAL, and it is mine —
//! E0958 came from FUP-176. The escape list right above the check already
//! exempts `$binding`, `` `hole` ``, `&ref` and `0 -> 1` for exactly this
//! reason: they are not CSS to judge. A token reference belongs in that set.
//!
//! Seen RED before the fix.

use spacetime::validation::css::validate_declaration_value;

#[test]
fn a_bare_token_is_a_valid_value_for_any_property() {
    for (property, value) in [
        ("color", "--ink"),
        ("background-color", "--brand-surface"),
        ("font-size", "--text-lg"),
        ("margin", "--space-4"),
        ("transition-timing-function", "--ease-out-expo"),
    ] {
        assert!(
            validate_declaration_value(property, value).is_none(),
            "`{property}: {value}` must be accepted — `--name` names a value, and \
             refusing it forces the author into `var()`, the CSS spelling Spacetime \
             does not want"
        );
    }
}

/// `var()` must keep working — the fix ADMITS a spelling, it does not swap one
/// mandate for another. Plenty of existing pages use `var()`.
#[test]
fn the_var_spelling_still_works() {
    assert!(validate_declaration_value("color", "var(--ink)").is_none());
    assert!(validate_declaration_value("color", "var(--ink, #222)").is_none());
}

/// The admission is the REFERENCE SHAPE, not a blanket pass: a genuinely wrong
/// literal on a typed property must still be refused, or the diagnostic is worth
/// nothing. This test was GREEN throughout — it is what makes the others mean
/// something.
#[test]
fn a_wrong_literal_is_still_refused() {
    assert!(
        validate_declaration_value("color", "notacolor").is_some(),
        "`notacolor` is neither a colour nor a token reference"
    );
}

/// A single leading dash is NOT a token reference (`-ink` is a vendor-ish
/// identifier, not a custom property), so it must not slip through the new gate.
#[test]
fn one_dash_is_not_a_token_reference() {
    assert!(
        validate_declaration_value("color", "-ink").is_some(),
        "a custom property needs TWO dashes; one dash must not be admitted"
    );
}

/// Admitting the SHAPE is only half the job: `color: --ink` must be LOWERED to
/// `color: var(--ink)` on the way out, because no browser understands a bare
/// custom-property name in a value position. Accepting it and emitting it
/// verbatim would be worse than the original false refusal — a clean build that
/// produces dead CSS, which is the exact failure mode this arc keeps finding.
///
/// A custom property's own DECLARATION (`--ink: #222`) is untouched: there the
/// `--name` is the property, not a reference.
///
/// RED, DELIBERATELY COMMITTED. The validator half is done (the three tests
/// above), but the LOWERING is not: a page's plain CSS reaches the stylesheet as
/// `CssExpr::Raw` opaque text, bypassing `emit_decl`/`emit_decl_to_writer`
/// entirely — so patching those two seams (done, and correct for the paths that
/// DO use them) is not sufficient. Filed as FUP-186.
#[test]
#[ignore = "FUP-186: page CSS flows through CssExpr::Raw, bypassing emit_decl"]
fn a_bare_token_lowers_to_var_in_the_emitted_css() {
    use spacetime::{CompileOptions, compile, parse};

    let src = ":root {\n  --ink: #222;\n}\n\n<div class=\"c\">x</div>\n\n.c {\n  color: --ink;\n}\n";
    let ast = parse(src).expect("parse");
    let out = compile(&ast, CompileOptions::default());
    let css = out.css.clone();

    assert!(
        css.contains("color: var(--ink)"),
        "a bare token must lower to var() so a browser can resolve it; got:\n{css}"
    );
    assert!(
        css.contains("--ink: #222"),
        "the token's own DECLARATION must survive untouched; got:\n{css}"
    );
}
