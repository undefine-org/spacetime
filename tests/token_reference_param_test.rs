//! A token reference is a value in every value position — INCLUDING a
//! paren-form parameter.
//!
//! `capture_type_accepts("easing", "--ease-out-expo")` has returned true since
//! PLAN-136 W2, and `tests/token_reference_test.rs` pins it. That gate tests
//! the PREDICATE. The form matcher does not call the predicate: it drives the
//! capture's EXTRACTOR over a token run. So the two halves disagreed, and the
//! disagreement was invisible because only one of them had a test.
//!
//! The visible symptom (measured, not reasoned):
//!
//! ```text
//! @fade-in { easing: --myease; }        -> builds        (body path)
//! @fade-in(easing: --myease)            -> E0946 REFUSED (paren path)
//! ```
//!
//! Same curve, declared in the same file, legal by the language's own rules —
//! accepted in one position and refused in the other. That is a FALSE REFUSAL,
//! the failure mode this arc holds to be worse than a missed diagnostic,
//! because it blocks a build that should succeed and the author has no way to
//! satisfy it.
//!
//! These gates were seen RED before the fix: `--myease`, `var(--x)` and the
//! stdlib's own `--ease-out-expo` all failed as paren params.

use spacetime::{CompileOptions, compile, parse};

fn errors_for(src: &str) -> Vec<String> {
    let ast = match parse(src) {
        Ok(a) => a,
        Err(e) => return vec![format!("PARSE: {e}")],
    };
    let compiled = compile(&ast, CompileOptions::default());
    compiled
        .pipeline_errors
        .iter()
        .map(|e| format!("{}: {}", e.code, e.message))
        .collect()
}

/// The reduced case: one locally-declared curve, used both ways.
#[test]
fn a_declared_curve_is_a_value_in_a_paren_param() {
    let src = r#"%scope page
@form easing --myease { cubic-bezier(0.16, 1, 0.3, 1) }
.box {
  @fade-in(duration: 300ms, easing: --myease);
}
"#;
    let errs = errors_for(src);
    assert!(
        !errs.iter().any(|e| e.contains("E0946")),
        "a curve declared in this very file must be usable as a paren param; got {errs:?}"
    );
}

/// The same value, the same directive, the other syntax. If this ever diverges
/// from the test above, the two paths have drifted again.
#[test]
fn body_and_paren_positions_agree_about_a_token_reference() {
    let decl = "%scope page\n@form easing --myease { cubic-bezier(0.16, 1, 0.3, 1) }\n";
    let body = format!("{decl}.box {{\n  @fade-in {{\n    duration: 300ms;\n    easing: --myease;\n  }}\n}}\n");
    let paren = format!("{decl}.box {{\n  @fade-in(duration: 300ms, easing: --myease);\n}}\n");

    let body_refused = errors_for(&body).iter().any(|e| e.contains("E0946"));
    let paren_refused = errors_for(&paren).iter().any(|e| e.contains("E0946"));

    assert_eq!(
        body_refused, paren_refused,
        "a value is a value in every position: body refused={body_refused}, paren refused={paren_refused}"
    );
    assert!(!body_refused, "neither position should refuse a declared curve");
}

/// `var(--x)` is the other token-reference spelling the predicate admits.
#[test]
fn a_var_reference_is_a_value_in_a_paren_param() {
    let src = r#"%scope page
.box {
  @fade-in(duration: 300ms, easing: var(--app-ease));
}
"#;
    let errs = errors_for(src);
    assert!(
        !errs.iter().any(|e| e.contains("E0946")),
        "var() names a value the cascade resolves later; refusing it asserts something we cannot know. got {errs:?}"
    );
}

/// A `--name` with no declaration anywhere is still admitted: a design token
/// may be defined in a project prelude, a `@tokens` block, or plain CSS this
/// compiler never sees. Refusing it would be the same overreach as refusing
/// `var()`. This pins the DELIBERATE limit of the fix — it admits the SHAPE of
/// a reference, and does not pretend to resolve it.
#[test]
fn an_undeclared_token_reference_is_admitted_not_resolved() {
    let src = r#"%scope page
.box {
  @fade-in(duration: 300ms, easing: --defined-somewhere-else);
}
"#;
    let errs = errors_for(src);
    assert!(
        !errs.iter().any(|e| e.contains("E0946")),
        "the shape is what a grammar can judge; existence is the cascade's business. got {errs:?}"
    );
}

/// The fix must NOT become "a scalar param accepts anything". A bare word is
/// not a token reference and `easing` has no such keyword, so it stays refused.
#[test]
fn the_admission_is_the_reference_shape_not_a_blanket_pass() {
    let src = r#"%scope page
.box {
  @fade-in(duration: 300ms, easing: notacurve);
}
"#;
    let errs = errors_for(src);
    assert!(
        errs.iter().any(|e| e.contains("E0946")),
        "`notacurve` is neither a keyword easing nor a token reference; it must still be refused. got {errs:?}"
    );
}
