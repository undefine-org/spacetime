//! BUG-367: a capture inside an optional group must REPORT as optional.
//!
//! `stdlib/macros/each.st` declares three clauses that are plainly optional:
//!
//! ```text
//! @each($source:binding as $item:binding, key: $key:expr? = undefined) {
//!   $invocations:template_invocation+
//!   (:entering { $_enterAnim:keyframes })?
//!   (:exiting  { $_exitAnim:keyframes  })?
//!   (:move     { $_moveAnim:keyframes  })?
//! }
//! ```
//!
//! The `( … )?` wrapper carries the optionality, but it was never propagated to
//! the captures INSIDE, which kept `CaptureModifier::Required` and no default.
//! So anything asking the registry "may this capture be absent?" got NO for a
//! clause the language treats as optional.
//!
//! # Why this is worth its own fix rather than a workaround
//!
//! The `%form` IS the language's self-description. Where it disagrees with the
//! parser, every consumer inherits the error — completion offering an optional
//! clause as mandatory, a validator rejecting valid source, and (the reason this
//! surfaced) any agent reasoning about a form's shape from the registry. For a
//! self-modifying system that is a lying map, not a cosmetic gap.
//!
//! The printer carried three local workarounds for exactly this; they are
//! removed by the fix, which is the honest test that the fix is real.

use spacetime::parser::meta_ast::{CaptureModifier, FormInlineElement};
use spacetime::syntax::STDLIB_REGISTRY;

/// Every capture reachable from a form's inline elements, paired with whether an
/// enclosing group made it optional.
fn captures_with_optionality(el: &FormInlineElement, in_optional: bool, out: &mut Vec<(String, bool, CaptureModifier)>) {
    match el {
        FormInlineElement::Capture(c, _) => {
            out.push((c.var_name.clone(), in_optional, c.modifier.clone()));
        }
        FormInlineElement::Group { elements, modifier } => {
            let optional = in_optional
                || !matches!(
                    modifier,
                    CaptureModifier::Required | CaptureModifier::OneOrMore
                );
            for e in elements {
                captures_with_optionality(e, optional, out);
            }
        }
        // A pseudo-selector clause carries its OWN modifier — `( :entering { … } )?`
        // is exactly this shape, not a `Group`, which is why the optionality has
        // to be threaded here too.
        FormInlineElement::PseudoSelector {
            body_params,
            modifier,
            ..
        } => {
            let optional = in_optional
                || !matches!(
                    modifier,
                    CaptureModifier::Required | CaptureModifier::OneOrMore
                );
            for p in body_params {
                for e in &p.elements {
                    captures_with_optionality(e, optional, out);
                }
            }
        }
        FormInlineElement::KeywordBlock { body_params, .. }
        | FormInlineElement::PseudoClass { body_params, .. } => {
            for p in body_params {
                for e in &p.elements {
                    captures_with_optionality(e, in_optional, out);
                }
            }
        }
        _ => {}
    }
}

fn each_body_captures() -> Vec<(String, bool, CaptureModifier)> {
    let form = STDLIB_REGISTRY
        .get_by_macro_name("each")
        .expect("`each` must be registered");

    let mut out = Vec::new();
    for p in &form.form.body_params {
        for e in &p.elements {
            captures_with_optionality(e, false, &mut out);
        }
    }
    out
}

#[test]
fn the_optional_clauses_exist_and_are_marked_optional_as_groups() {
    // Precondition: without this the test below could pass vacuously.
    let form = STDLIB_REGISTRY.get_by_macro_name("each").expect("registered");

    let optional_clauses = form
        .form
        .body_params
        .iter()
        .flat_map(|p| p.elements.iter())
        .filter(|e| match e {
            FormInlineElement::PseudoSelector { modifier, .. } => !matches!(
                modifier,
                CaptureModifier::Required | CaptureModifier::OneOrMore
            ),
            _ => false,
        })
        .count();

    assert_eq!(
        optional_clauses, 3,
        "`each` declares :entering / :exiting / :move as optional clauses"
    );
}

#[test]
fn a_capture_inside_an_optional_clause_reports_optional() {
    // THE BUG. `$_enterAnim` lives inside `( :entering { … } )?`, so the registry
    // must answer "may this be absent?" with YES.
    let captures = each_body_captures();

    for name in ["_enterAnim", "_exitAnim", "_moveAnim"] {
        let (_, enclosed_by_optional, modifier) = captures
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("`each` must declare {name}"))
            .clone();

        assert!(
            enclosed_by_optional,
            "{name} is inside an optional clause — the fixture is wrong if this fails"
        );

        assert!(
            matches!(
                modifier,
                CaptureModifier::Optional | CaptureModifier::ZeroOrMore
            ),
            "{name} sits inside `( … )?` but reports {modifier:?}: the registry \
             misanswers 'may this capture be absent?', and every consumer that \
             asks inherits the wrong answer (BUG-367)"
        );
    }
}

#[test]
fn an_each_without_animation_clauses_still_prints() {
    // The user-visible consequence, end to end: 34 corpus forms failed to print
    // because the printer trusted the registry's `Required` and refused an
    // absence the language allows.
    let src = "@each($items as $i) {\n  &row($i);\n}\n";
    let ast = spacetime::parse(src).expect("fixture must parse");

    let m = ast
        .matches
        .iter()
        .find(|m| m.matched_macro.as_deref() == Some("each"))
        .expect("an `each` match");

    assert!(
        !m.captures.contains_key("_enterAnim"),
        "no `:entering` clause was written, so the capture is absent"
    );

    spacetime::edn::print_form(m, &STDLIB_REGISTRY)
        .expect("an `each` with no animation clauses must print");
}

#[test]
fn a_required_capture_is_still_required() {
    // The fix must not make everything optional: a capture NOT inside an
    // optional group keeps its requiredness, or the registry starts lying the
    // other way.
    let form = STDLIB_REGISTRY.get_by_macro_name("each").expect("registered");

    // `$source` is a PARAM (`@each($source:binding as $item, …)`), not an inline
    // element — `each` has no inline elements at all.
    let mut inline = Vec::new();
    for p in &form.form.params {
        for e in &p.elements {
            captures_with_optionality(e, false, &mut inline);
        }
    }

    let (_, _, modifier) = inline
        .iter()
        .find(|(n, _, _)| n == "source")
        .expect("`each` declares $source")
        .clone();

    assert!(
        matches!(modifier, CaptureModifier::Required),
        "$source is not inside an optional group and must stay Required, \
         found {modifier:?}"
    );
}
