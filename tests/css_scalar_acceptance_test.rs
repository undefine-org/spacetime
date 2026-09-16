//! PLAN-122 W1.2/W3 — what a CSS scalar ACCEPTS, and what it REFUSES.
//!
//! The value grammars in `stdlib/capture-types/css-values.st` replaced a set of
//! hand-written Rust extractors. The point of the replacement is not tidiness:
//! the old extractors validated almost nothing. `ColorExtractor` depth-scanned
//! to the next `;` or `,` and returned whatever it found, so `#e8ee1` — a
//! five-digit hex, which is not a colour in any CSS version — passed `check`
//! with a tick and landed verbatim in the emitted stylesheet, where the browser
//! drops the declaration and the author sees an element render wrong with no
//! diagnostic anywhere in the toolchain.
//!
//! So this file is mostly NEGATIVES. A grammar that accepts every valid value
//! is half a grammar; the half that matters is the refusals, and every defect
//! this wave shipped and then fixed was invisible to a positive-only test:
//!
//!   - `#e8eef7z` was ACCEPTED (a legal 6-digit prefix, with a stray suffix
//!     riding along inside the same token).
//!   - `40 px` was ACCEPTED as a length, because the generic sequence extractor
//!     skips whitespace between elements.
//!   - `50%` was REFUSED as a length, taking `score_measure` and every clip
//!     driver with it.
//!   - `RGB(1, 2, 3)` was REFUSED, because literal terminals compared
//!     case-sensitively and CSS keywords are not.
//!
//! Each of those is one line in the table below.

use spacetime::{CompileOptions, compile, parse};

/// Compile a page and report whether the directive matched its grammar.
///
/// Grammar refusal surfaces as E0946 ("directive does not match its declared
/// grammar"), which is the diagnostic a capture failure produces.
fn accepts(source: &str) -> bool {
    let Ok(ast) = parse(source) else {
        return false;
    };
    let compiled = compile(&ast, CompileOptions::default());
    !compiled
        .pipeline_errors
        .iter()
        .any(|e| e.code == "E0946")
}

/// A colour in the position authors actually write one: a directive argument.
///
/// This position is load-bearing for the test, not incidental. The wave's first
/// colour gate ran the terminals against a CSS DECLARATION slice and passed
/// while `@object(color: #c65b3c)` was refused in every real page — the slice
/// there ends at `)` rather than `;`, and the scan window had been derived from
/// the wrong end. A grammar is only as good as the position it is exercised in.
fn colour_arg(value: &str) -> String {
    format!(".x {{\n  @object(shape: \"box\", color: {value})\n}}\n\n<div class=\"x\">y</div>\n")
}

fn length_arg(value: &str) -> String {
    format!(".x {{\n  @fade-in(distance: {value})\n}}\n\n<div class=\"x\">y</div>\n")
}

fn duration_arg(value: &str) -> String {
    format!(".x {{\n  @fade-in(duration: {value})\n}}\n\n<div class=\"x\">y</div>\n")
}

#[test]
fn a_colour_accepts_every_legal_hex_length_and_no_others() {
    // The four legal hex lengths. The 4- and 8-digit forms carry alpha; they are
    // the reason the permitted counts are DATA (`{3|4|6|8}`) rather than a
    // "3 or 6" rule, which would silently drop them.
    for value in ["#fff", "#fffa", "#c65b3c", "#11223344"] {
        assert!(
            accepts(&colour_arg(value)),
            "{value} is a legal CSS colour and was refused"
        );
    }

    // CSS is case-insensitive, so the grammar says so once (on the terminal)
    // rather than restating every range and keyword in upper case.
    for value in ["#E8EEF7", "#AaBbCc"] {
        assert!(
            accepts(&colour_arg(value)),
            "{value} differs from an accepted colour only in case"
        );
    }

    // The refusals. Each is a value the OLD extractor accepted and emitted.
    for (value, why) in [
        ("#e8ee1", "5 hex digits — no such CSS colour"),
        ("#e8eef71", "7 hex digits — nor this"),
        ("#zz", "not hex at all"),
        (
            "#e8eef7z",
            "a legal 6-digit prefix with a stray suffix in the same token: the \
             count check passes on the prefix, so only a token-boundary check \
             catches it",
        ),
    ] {
        assert!(
            !accepts(&colour_arg(value)),
            "{value} was ACCEPTED ({why}). It will reach the emitted stylesheet \
             verbatim and the browser will drop the declaration silently."
        );
    }
}

#[test]
fn a_colour_accepts_functions_and_keywords_whatever_their_case() {
    for value in [
        "rgb(1, 2, 3)",
        "RGB(1, 2, 3)",
        "oklch(0.7 0.1 200)",
        "transparent",
        "TRANSPARENT",
        "currentColor",
        "inherit",
    ] {
        assert!(
            accepts(&colour_arg(value)),
            "{value} is a legal CSS colour value and was refused"
        );
    }
}

#[test]
fn a_dimension_requires_its_unit_to_be_adjacent() {
    // `40px` is one value. `40 px` is a number and an identifier, and CSS has no
    // such value — but a grammar sequence normally skips whitespace between its
    // elements (`param_list`'s `$a, &b` depends on that), so the unit terminal
    // has to opt into the stricter rule explicitly.
    assert!(accepts(&length_arg("40px")), "`40px` is a length");
    assert!(
        !accepts(&length_arg("40 px")),
        "`40 px` was accepted as a length. The captured value becomes the \
         literal string \"40 px\", which is not a CSS dimension, and it is \
         emitted with the space intact."
    );

    assert!(accepts(&duration_arg("500ms")), "`500ms` is a duration");
    assert!(
        !accepts(&duration_arg("500 ms")),
        "`500 ms` was accepted as a duration"
    );
}

#[test]
fn a_percentage_is_a_length() {
    // `50%` is legal wherever a length is, and `score_measure`
    // (stdlib/macros/score.st) is `( $len:length ) | ( $time:duration )` — so
    // dropping `%` from the length grammar silently took `&title for 50%` with
    // it and broke every clip driver. The unit set is not free to be tidier
    // than CSS.
    assert!(accepts(&length_arg("50%")), "`50%` is a length");
    assert!(accepts(&length_arg("100%")), "`100%` is a length");
}

#[test]
fn a_scalar_accepts_a_binding_because_a_value_position_is_reactive() {
    // `@fade-in(duration: $speed)` is ordinary usage: the value is known at
    // runtime. The binding arm is listed LAST in each union so a literal always
    // wins — and, critically, so that a MALFORMED literal is not rescued by it.
    assert!(
        accepts(&duration_arg("$speed")),
        "a binding is a legal value in a scalar position"
    );
    assert!(
        !accepts(&colour_arg("#e8ee1")),
        "a malformed hex colour fell through to the binding arm and was \
         accepted — that would defeat every refusal in this file"
    );
}

#[test]
fn a_duration_refuses_a_bare_identifier() {
    // The failure mode that made this whole class of gate necessary. When
    // `%capture_type time` did not exist, the name resolved to an unregistered
    // Custom, which `form_compiler` silently falls back to the greedy `Expr`
    // extractor for — so ~69 `$x:time` captures across stdlib accepted
    // arbitrary expressions while every positive test stayed green.
    //
    // A positive-only suite cannot see that. This assertion can.
    assert!(
        !accepts(&duration_arg("banana")),
        "`duration: banana` was accepted. The scalar grammar is not being \
         consulted — most likely the capture-type name is unregistered and is \
         falling back to `Expr`, which matches nearly anything."
    );
}

#[test]
fn a_scalar_is_never_serialized_as_a_record() {
    // PLAN-122's representation decision: a scalar's captured value is its
    // SOURCE TEXT (`"800ms"`), with the type as compile-time metadata.
    //
    // This is the BEHAVIOUR half of the contract (AGENTS BUG-252). Asserting
    // that a capture is a String proves a representation, not that anything
    // works: rebuilding the value from its grammar parts rather than reading its
    // source span satisfied a String assertion happily while producing `"s6"`,
    // because the parts land in a HashMap and come back in whatever order it
    // chose. What actually matters is what reaches generated code.
    //
    // The failure this guards is a scalar arriving as a RECORD
    // (`{"n":"800","u":"ms"}`) where every consumer expects a string — which is
    // how `%color.rgba` came to parse an object as a colour, and how an
    // animation duration came to compute NaN.
    let source = ".hero {\n  @fade-in(duration: 800ms, distance: 40px)\n}\n\n<div class=\"hero\">x</div>\n";
    let ast = parse(source).expect("source should parse");
    let compiled = compile(&ast, CompileOptions::default());

    assert!(
        compiled.pipeline_errors.is_empty(),
        "a page using ordinary scalar values failed to compile: {:?}",
        compiled.pipeline_errors
    );
    assert!(
        !compiled.js.is_empty(),
        "the page emitted no JS at all, so this gate would pass vacuously"
    );

    // The record shapes are searched for specifically rather than scanning for
    // "NaN": the emitted page bundles the whole Spacetime runtime, which
    // legitimately contains nine NaN references of its own. A gate that trips on
    // library code is not a gate on this wave.
    for corruption in [
        "{\"n\":",
        "{n:",
        "\"u\":\"ms\"",
        "\"u\":\"px\"",
        "\"wide\":",
        "\"keyword\":",
        "\"hex\":",
    ] {
        assert!(
            !compiled.js.contains(corruption),
            "generated JS contains {corruption:?} — a scalar's grammar parts \
             survived into codegen instead of being flattened to its source text"
        );
    }
}

/// An EMPTY value satisfies no grammar — including one we do not model.
///
/// `capture_type_accepts` has two escapes that must not compose: an empty value
/// is refused, and an UNKNOWN production is "not ours to refuse". Splitting the
/// matcher out for BUG-279 briefly moved the empty check inside the shared
/// matcher, so `None` became ambiguous — it meant both "no match" and "nothing
/// to match" — and an empty value started passing whenever the named production
/// was missing.
///
/// That is not hypothetical: `src/stdlib_embedded.rs` documents a shipped
/// release where `stdlib/scalars/types.st` travelled WITHOUT the grammars it
/// names, so every lookup took the unknown-type path. In that state this
/// regression turns the seed checker's refusal of a blank field into silent
/// acceptance — the exact class of silent-accept the file above exists to stop.
#[test]
fn an_empty_value_is_refused_even_for_an_unmodelled_type() {
    use spacetime::syntax::events::capture_type_accepts;

    assert!(
        !capture_type_accepts("color", ""),
        "an empty value satisfies no grammar"
    );
    assert!(
        !capture_type_accepts("no_such_production_xyz", ""),
        "an unknown production is 'not ours to refuse' — but an EMPTY value is \
         still empty, and accepting it here is how a blank field slips past the \
         seed checker when the stdlib grammars are missing"
    );
    assert!(
        !capture_type_accepts("no_such_production_xyz", "   "),
        "whitespace-only is empty too"
    );
    // The escape itself must survive: a real value against an unmodelled
    // production is still accepted, or every unknown type becomes a hard error.
    assert!(
        capture_type_accepts("no_such_production_xyz", "whatever"),
        "a non-empty value against a production we cannot model stays accepted"
    );
}

