//! Arm routing for `value_node` — assert the ARM, never a bool.
//!
//! `capture_type_accepts` on a production with a catch-all arm proves NOTHING:
//! `value_node`'s last arm is `balanced(';')`, so it returns true for
//! `"garbage ~~~ nonsense"`. Only the matched ARM NAME is a real signal, which
//! is why every assertion here names one.
//!
//! These arms exist so `validate_declaration_value` can stop asking the text
//! what it is. Each sigil that still appears in that function's lexical escape
//! `if` must first be routable HERE — otherwise removing it from the `if` is a
//! false refusal waiting to happen.

use spacetime::syntax::STDLIB_REGISTRY;
use spacetime::syntax::events::capture_type_matched_arm;

fn arm(text: &str) -> Option<String> {
    let defs = STDLIB_REGISTRY
        .capture_types()
        .map(|d| (d.name.clone(), d.clone()))
        .collect();
    capture_type_matched_arm("value_node", text, &defs)
}

fn assert_arm(text: &str, expected: &str) {
    assert_eq!(
        arm(text).as_deref(),
        Some(expected),
        "{text:?} must route to the `{expected}` arm"
    );
}

// ── the arms that already worked (regression guards) ──────────────────────

#[test]
fn a_token_reference_routes_to_token() {
    assert_arm("--ink", "token");
    assert_arm("var(--ink)", "token");
}

#[test]
fn a_binding_routes_to_binding() {
    assert_arm("$sig", "binding");
}

#[test]
fn a_wide_keyword_routes_to_wide() {
    assert_arm("inherit", "wide");
}

#[test]
fn plain_css_routes_to_css() {
    assert_arm("#ff0000", "css");
    assert_arm("1px solid red", "css");
}

// ── WAVE 1: the arms that did not exist ───────────────────────────────────

/// `&` is the IDENTITY sigil — it names an ELEMENT, never a CSS value.
/// Wrapping the builtin `selector` is NOT precise enough: `{ $sel:selector }`
/// accepts `#ff0000` as an id selector, which would steal a real colour.
#[test]
fn an_element_reference_routes_to_element() {
    assert_arm("&self", "element");
    assert_arm("&card", "element");
}

/// A directive or meta reference. `{ $e:expr }` is not precise enough either —
/// `expr` is a catch-all and accepts `notadirective`.
#[test]
fn a_directive_reference_routes_to_directive() {
    assert_arm("@fade-in", "directive");
    assert_arm("%scalar_type", "directive");
}

/// THE ONE THE LEXER FIX UNLOCKED. Before the backtick was a token, a hole
/// could not be written as a grammar at all — a grammar consumes tokens, and
/// there was no token. This assertion was IMPOSSIBLE last week.
#[test]
fn a_hole_routes_to_hole() {
    assert_arm("`$x`", "hole");
    assert_arm("`$c.id`", "hole");
}

/// A range whose endpoints are themselves value nodes.
#[test]
fn a_range_routes_to_range() {
    assert_arm("0 -> 1", "range");
}

// ── the discriminations that must NOT collapse ────────────────────────────

/// The precision test for `element`: a hex colour is not an element reference,
/// even though a CSS selector grammar would happily call `#ff0000` an id.
#[test]
fn a_hex_colour_is_not_an_element() {
    assert_arm("#ff0000", "css");
}

/// The precision test for `directive`: a bare word is not a directive.
#[test]
fn a_bare_word_is_not_a_directive() {
    assert_arm("notadirective", "css");
}

/// The precision test for `hole`: a backtick is only a hole when it CLOSES.
/// An unterminated one is text, not a deferred value — refusing to guess is
/// the whole doctrine of this file.
#[test]
fn an_unterminated_backtick_is_not_a_hole() {
    assert_ne!(
        arm("`$x").as_deref(),
        Some("hole"),
        "an unclosed backtick must not be admitted as a hole"
    );
}
