//! `&` starts a selector — the false refusal W1 exposed.
//!
//! W1 made builtin capture types actually CHECK instead of waving every value
//! through. That was right, and it immediately surfaced a gap that had been
//! invisible for as long as the check was a no-op: `SelectorExtractor` did not
//! list `AMPERSAND` in `is_selector_start`, so it refused `&`.
//!
//! While nothing enforced the type, that omission cost nothing. The moment the
//! type was enforced it became eight false refusals in `tests/fixtures/`:
//!
//!     error[E0946]: Expected `$target:Selector`, got token "AMPERSAND"
//!       --> tests/fixtures/native-template/on-visible-animation-only.test.st:79
//!        |
//!     79 |   @then &mixInst {
//!
//! `&` is not an edge case. It is the CSS nesting selector AND Spacetime's
//! element-ref sigil — `&.hover`, `&mixInst`, `& > .child` are all ordinary
//! authoring. A rule that refuses them refuses the language.
//!
//! This is the arc's stated priority made concrete: A FALSE REFUSAL THAT BLOCKS
//! A BUILD IS WORSE THAN A MISSED DIAGNOSTIC. The check stays; the definition of
//! "selector" gets corrected to include the sigil it always should have.

use spacetime::syntax::events::capture_type_accepts;

/// The shapes that were refused. Each appears in the fixture corpus.
#[test]
fn an_ampersand_starts_a_selector() {
    for v in ["&", "&mixInst", "&.hover", "&:hover", "& > .child", "&.a.b"] {
        assert!(
            capture_type_accepts("selector", v),
            "`{v}` is ordinary Spacetime/CSS authoring \u{2014} refusing it blocks a \
             build that was correct. See tests/fixtures/native-template/."
        );
    }
}

/// The refusal must stay real, or this is not a check at all.
///
/// W1's whole point was that a builtin type must be able to say no. Widening
/// `selector` to admit `&` must not widen it to admit everything — otherwise the
/// fix trades a false refusal for a missed diagnostic, which is the other half
/// of the same mistake.
#[test]
fn widening_for_ampersand_does_not_admit_everything() {
    // NB: `#e8eef7abc` is deliberately NOT in this list, though I first wrote it
    // here. It is indistinguishable from the id selector `#e8eef7abc`, which is
    // legal — the lexer emits one COLOR token for both spellings, and an id may
    // be any identifier. Asserting a refusal there would have been asserting a
    // diagnostic the language cannot honestly give.
    for v in ["600ms", "\"quoted\"", "42"] {
        assert!(
            !capture_type_accepts("selector", v),
            "`{v}` is not a selector; if it now passes, the extractor was widened \
             too far and the type stopped discriminating"
        );
    }
}

/// The selector shapes that already worked keep working.
#[test]
fn ordinary_selectors_are_unaffected() {
    for v in [".mq-box", "#id", "div", "*", "[data-x]", ".a > .b", ":is(.a, .b)"] {
        assert!(
            capture_type_accepts("selector", v),
            "`{v}` parsed as a selector before the ampersand fix and must still"
        );
    }
}

/// `&name(…)` is a template INVOCATION, and admitting `&` must not swallow it.
///
/// This is the cost of the fix above, found by the fixture corpus rather than by
/// reasoning: once `&` could start a selector, `&card("Hello")` became one, and
/// a greedy `$target:selector` ate the call site in
/// `tests/fixtures/native-template/template-factory.test.st:10`.
///
/// The paren cannot disambiguate downstream — the extractor deliberately treats
/// a depth-0 `(` as part of a functional pseudo-class (`:not(.bar)`, BUG-065) —
/// so the discrimination has to happen at the head of the token run.
#[test]
fn a_template_invocation_is_not_a_selector() {
    for v in ["&card(\"Hello\")", "&row(1, 2)", "&card ( \"x\" )"] {
        assert!(
            !capture_type_accepts("selector", v),
            "`{v}` is a template invocation; treating it as a selector lets \
             `@then`/`@on` swallow the call site"
        );
    }

    // ...while the element refs that motivated admitting `&` still work,
    // including one immediately followed by a descendant selector.
    for v in ["&card", "&card .inner", "&noActions .card__actions"] {
        assert!(
            capture_type_accepts("selector", v),
            "`{v}` is an element-ref selector and must still be accepted"
        );
    }

    // A functional pseudo-class still keeps its parens (BUG-065).
    assert!(capture_type_accepts("selector", "&:not(.bar)"));
}

