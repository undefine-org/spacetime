//! CSS-native typed `@data inline` seeds (FEAT-166 / PLAN-122 W2).
//!
//! The feature that started the arc. A seed whose type is declared should be
//! writable in the language of its field types:
//!
//! ```spacetime
//! @type Brand { ink: color; radius: length; reveal: duration; }
//! @data inline $brand Brand : { ink: #e8eef7; radius: 8px; reveal: 600ms };
//! ```
//!
//! THE ACCEPTANCE IS AN EQUIVALENCE, not a compile. The CSS-native seed and the
//! quoted seed must produce the SAME JavaScript — if they differ, they are two
//! features that happen to look alike, and the second one will drift.
//!
//! That framing matters because of how this wave failed the first time it was
//! spiked: the brace gate alone made the CSS-native form PARSE, and it then
//! emitted nothing at all while `check` reported zero errors. Any gate asserting
//! "it compiles" would have passed on a feature that did nothing. So the first
//! assertion here is the equivalence, and `compiles` is never enough.

use spacetime::{CompileOptions, compile, parse};

fn compile_js(src: &str) -> Result<String, String> {
    let ast = parse(src).map_err(|e| format!("parse failed: {e:?}"))?;
    let out = compile(&ast, CompileOptions::default());
    if !out.pipeline_errors.is_empty() {
        return Err(out
            .pipeline_errors
            .iter()
            .map(|e| format!("{}: {}", e.code, e.message))
            .collect::<Vec<_>>()
            .join("; "));
    }
    Ok(out.js)
}

/// Multi-line because that is the surface `@type` actually has — a single-line
/// `@type Brand { ink: color; }` does not parse, which cost a debugging detour
/// worth pinning here.
/// The trailing `;` on each field is REQUIRED, not stylistic: `properties`
/// captures a field's type with `balanced(';')`, so without the terminator the
/// capture runs past the newline and swallows the next field —
/// `ink: "color\n  radius: length"` as ONE field. The compiler accepts that
/// quietly, which is how a two-field type silently becomes a one-field type.
const TYPE_DECL: &str =
    "@type Brand {\n  ink: color;\n  radius: length;\n  reveal: duration;\n}";

fn quoted_seed() -> String {
    format!(
        "{TYPE_DECL}\n@data inline $brand Brand : {{ \"ink\": \"#e8eef7\", \
         \"radius\": \"8px\", \"reveal\": \"600ms\" }};\n"
    )
}

/// COMMA-separated, not semicolon.
///
/// A record literal is a VALUE, and `,` is what separates the elements of a
/// value everywhere in the language. `;` TERMINATES a declaration — it is the
/// CSS-declaration separator, and reusing it here would make the seed's inner
/// syntax disagree with its outer one (the seed itself ends in `;`).
///
/// Sigil harmony decides this, not preference: one separator, one meaning.
fn native_seed() -> String {
    format!(
        "{TYPE_DECL}\n@data inline $brand Brand : \
         {{ ink: #e8eef7, radius: 8px, reveal: 600ms }};\n"
    )
}

/// The wave's whole claim, in one assertion.
#[test]
fn a_css_native_seed_emits_the_same_javascript_as_the_quoted_seed() {
    let quoted = compile_js(&quoted_seed()).expect("the quoted seed compiles today");
    let native = compile_js(&native_seed()).expect("the CSS-native seed must compile");
    assert_eq!(
        native, quoted,
        "the two seed surfaces must be the SAME feature, not two features that resemble each other"
    );
}

/// The negative the failed spike proved is necessary.
///
/// A seed that parses and emits nothing is the exact failure this wave must not
/// reproduce, and it is invisible to any "does it compile" assertion.
#[test]
fn a_css_native_seed_actually_emits_its_values() {
    let js = compile_js(&native_seed()).expect("compiles");
    for expected in ["#e8eef7", "8px", "600ms"] {
        assert!(
            js.contains(expected),
            "the seed must carry `{expected}` into the emitted JS — \
             a seed that parses and emits nothing is the failure mode this gate exists for"
        );
    }
}

/// A field value inherits the REFUSALS of its declared type's grammar.
///
/// `#e8ee1` is refused because `hex_color` says 3/4/6/8 digits. That rule is not
/// restated here — if it were, there would be two answers to what a colour is,
/// which is the thing PLAN-122 deleted.
#[test]
fn a_malformed_field_value_is_refused() {
    let src = format!(
        "{TYPE_DECL}\n@data inline $brand Brand : \
         {{ ink: #e8ee1; radius: 8px; reveal: 600ms }};\n"
    );
    assert!(
        compile_js(&src).is_err(),
        "`#e8ee1` is not a colour and must not be accepted as one"
    );
}

/// A field whose value is the wrong TYPE is refused too — the seed is checked
/// against its declaration, not merely against CSS.
#[test]
fn a_wrong_typed_field_value_is_refused() {
    let src = format!(
        "{TYPE_DECL}\n@data inline $brand Brand : \
         {{ ink: 8px; radius: 8px; reveal: 600ms }};\n"
    );
    assert!(
        compile_js(&src).is_err(),
        "`8px` is a length, and `ink` is declared a colour"
    );
}

/// An unknown field is a typo far more often than an intention.
#[test]
fn an_unknown_field_is_refused() {
    let src = format!(
        "{TYPE_DECL}\n@data inline $brand Brand : \
         {{ accnet: #e8eef7; radius: 8px; reveal: 600ms }};\n"
    );
    assert!(
        compile_js(&src).is_err(),
        "`accnet` is not a field of Brand"
    );
}

/// The untyped surface is untouched: no type, no record grammar, no change.
#[test]
fn an_untyped_seed_still_takes_an_arbitrary_expression() {
    let src = "@data inline $count : 42;\n";
    compile_js(src).expect("an untyped seed keeps its expression surface");
}

// ── Defects found by the review swarm ───────────────────────────────────────

/// A dimension's unit must be ADJACENT to its number.
///
/// The seed checker lexed the value with trivia FILTERED OUT, so `40 px` became
/// `NUMBER IDENT` — exactly the shape of a valid length — and the grammar
/// reported a whole match. `AdjacentExtractor` can only refuse a spaced
/// dimension when the whitespace token is still there to see.
#[test]
fn a_spaced_dimension_is_not_a_dimension() {
    for bad in ["40 px", "300 ms"] {
        let src = format!(
            "{TYPE_DECL}\n@data inline $brand Brand : \
             {{ ink: #e8eef7, radius: {bad}, reveal: 600ms }};\n"
        );
        assert!(
            compile_js(&src).is_err(),
            "`{bad}` has a space between number and unit — CSS has no such value"
        );
    }
}

/// Trailing whitespace is not content: `#e8eef7 ` is the same colour.
#[test]
fn surrounding_whitespace_does_not_change_a_value() {
    let src = format!(
        "{TYPE_DECL}\n@data inline $brand Brand : \
         {{ ink:  #e8eef7 , radius: 8px , reveal: 600ms }};\n"
    );
    compile_js(&src).expect("whitespace around a value is not part of it");
}

/// A field the type REQUIRES and the seed omits: the seed claims to be a Brand.
#[test]
fn a_missing_required_field_is_refused() {
    let src = format!("{TYPE_DECL}\n@data inline $brand Brand : {{ ink: #e8eef7 }};\n");
    let err = compile_js(&src).expect_err("radius and reveal are required");
    assert!(
        err.contains("radius") || err.contains("reveal"),
        "the diagnostic should name the missing field, got: {err}"
    );
}

/// A record fragment with no `:` is malformed, not absent.
#[test]
fn a_field_without_a_separator_is_refused() {
    let src = format!(
        "{TYPE_DECL}\n@data inline $brand Brand : \
         {{ ink #e8eef7, radius: 8px, reveal: 600ms }};\n"
    );
    assert!(
        compile_js(&src).is_err(),
        "`ink #e8eef7` has no `:` — dropping it silently reports nothing at all"
    );
}
