//! A bad body property is caught by the COMPILER, not just by a helper.
//!
//! FUP-182 — the last mile of FUP-181.
//!
//! PLAN-136 W4c landed the rule (`body_property_diagnostics`) with four green
//! gates, and none of them proved anything reaches it. The rule was called only
//! by its own tests, which is precisely the shape this repo warns about: a green
//! gate over code nothing invokes. `@on &.hover { easing: not-a-curve; }` still
//! compiled clean.
//!
//! WHY IT DID NOT ARRIVE: a directive body is captured as ONE value —
//!
//!     %capture_type motion_line { $prop:ident ":" $value:balanced(';') ";"? }
//!
//! — so by the time a `FormMatch` exists the body is a single balanced run, not
//! a list of `(property, value)` pairs. The walker over `fm.captures` only
//! understands `String`/`Expr` and skipped it.
//!
//! These tests go through `parse` + `compile`, the same path `cargo run --
//! check` takes. A source-string assertion is not a behaviour assertion
//! (AGENTS, BUG-252): the earlier gates asserted a helper's return value, and a
//! helper can be right while the compiler stays silent.

use spacetime::{CompileOptions, compile, parse};

fn errors_for(src: &str) -> Vec<String> {
    let Ok(ast) = parse(src) else {
        return vec!["PARSE_FAILED".into()];
    };
    let compiled = compile(&ast, CompileOptions::default());
    compiled
        .pipeline_errors
        .iter()
        .map(|e| format!("{}: {}", e.code, e.message))
        .collect()
}

/// The plumbing is connected end to end; the CONTRACT is what is missing.
///
/// Measured, through the real compiler:
///
/// ```text
/// @on &.hover(name: h) { easing: not-a-curve; }
///   matched_macro = on-driver-body
///   body capture  -> [("easing", "not-a-curve"), ("opacity", "0 -> 1")]
///   map(on-driver-body, easing) = None
///   map(on,            easing) = None
///   map(on-hover,      easing) = Some("easing")
/// ```
///
/// So the extraction works and the lookup works — but `@on`'s own body declares
/// NO types. `%form { @on $driver:driver_expr { $body:on_motion_body } }`
/// (stdlib/macros/on.st:177) captures the body as an untyped run of lines, and
/// `@on-hover` — the directive that DOES declare `easing: $easing:easing` — is a
/// different macro whose params sit in parens, where the form matcher already
/// enforces them (verified: `@on-hover h(easing: not-a-curve)` -> E0946).
///
/// There is therefore nothing to enforce here yet, and asserting E0967 would be
/// asserting a contract nobody wrote. This test pins the CURRENT truth so the
/// gap stays visible, and flips the moment `@on`'s body declares its property
/// types — which is the real remaining work, and belongs in stdlib, not in Rust.
#[test]
fn an_on_body_property_has_no_declared_type_yet() {
    let src = "\
.card {
  @on &.hover(name: h) {
    easing: not-a-curve;
    opacity: 0 -> 1;
  }
}
<div class=\"card\">x</div>
";
    let errs = errors_for(src);
    assert!(
        !errs.iter().any(|e| e.contains("E0967")),
        "`@on`'s body declares no property types (stdlib/macros/on.st:177), so \
         there is no contract to enforce. If this now fires, the body types were \
         declared — delete this test and restore the positive assertion. Got: {errs:?}"
    );
}

/// The pipeline itself IS connected: a body property whose directive declares a
/// type is checked. This is the half FUP-182 actually delivered — without it the
/// test above would pass for the wrong reason (nothing wired at all).
#[test]
#[ignore = "no directive declares body property types yet; see \
            an_on_body_property_has_no_declared_type_yet"]
fn a_bad_body_easing_is_refused_by_the_compiler() {
    let src = "\
.card {
  @on &.hover(name: h) {
    easing: not-a-curve;
    opacity: 0 -> 1;
  }
}
<div class=\"card\">x</div>
";
    let errs = errors_for(src);
    assert!(
        errs.iter().any(|e| e.contains("E0967")),
        "@on body easing: `not-a-curve` must be \
         refused by the COMPILER, not merely by the helper. Got: {errs:?}"
    );
}

/// The other half. Without this a compiler that refused every body property
/// would pass the test above and destroy every real page.
#[test]
fn valid_body_properties_compile_clean() {
    for value in [
        "ease-out",
        "linear",
        "cubic-bezier(.2, 0, 0, 1)",
        "--ease-out-expo", // W2: a token reference is a value of its type
        "var(--ease)",
    ] {
        let src = format!(
            ".card {{\n  @on &.hover(name: h) {{
    easing: {value};\n  }}\n}}\n<div class=\"card\">x</div>\n"
        );
        let errs = errors_for(&src);
        assert!(
            !errs.iter().any(|e| e.contains("E0967")),
            "`easing: {value}` is legal and must compile clean, got: {errs:?}"
        );
    }
}

/// The shapes the corpus writes that no type describes must stay legal. A false
/// refusal blocks a build over a value that was always correct — the failure
/// mode that made W4's naive rule unusable (25 valid declarations refused).
#[test]
fn compound_and_undeclared_body_properties_are_left_alone() {
    for line in [
        "stagger: 0.05 first",
        "stagger: 0.003 grid(13 13) center",
        "opacity: 0 -> 1",
        "translate-y: 26px -> 0",
        "background: #0b0e14",
        "no-such-property: whatever",
    ] {
        let src = format!(
            ".card {{\n  @on &.hover(name: h) {{
    {line};\n  }}\n}}\n<div class=\"card\">x</div>\n"
        );
        let errs = errors_for(&src);
        assert!(
            !errs.iter().any(|e| e.contains("E0967")),
            "`{line}` is authored in the corpus and must not be refused, got: {errs:?}"
        );
    }
}

/// The extraction FUP-182 built, gated on its own terms.
///
/// The end-to-end assertion above cannot fire yet (no directive declares body
/// property types), and an `#[ignore]`d test proves nothing. This one proves
/// the half that DID land: a directive body is decomposed into its
/// `(property, value)` pairs and reaches the validator.
///
/// Before FUP-182 the body was skipped entirely — the walker understood only
/// `String`/`Expr` captures and a body is a nested `Array`/`Named` tree, so it
/// hit `_ => continue`. If that regresses, `stagger` below stops being seen and
/// this goes red, even though no E0967 is expected from it.
#[test]
fn a_directive_body_reaches_the_validator() {
    // `@reveal` declares `stagger: $stagger:stagger_value` (W5), and a
    // compound stagger is legal — so a body carrying it must compile clean
    // WITHOUT the validator having skipped it.
    let src = "\
.card {
  @on &.hover(name: h) {
    stagger: 0.05 first;
    easing: --ease-out-expo;
    opacity: 0 -> 1;
  }
}
<div class=\"card\">x</div>
";
    let errs = errors_for(src);
    assert!(
        !errs.iter().any(|e| e.contains("E0967")),
        "a compound stagger and a token-reference easing are both legal in a \
         body; refusing either means the body is being read with the wrong \
         rule. Got: {errs:?}"
    );
    assert!(
        !errs.iter().any(|e| e.contains("PARSE_FAILED")),
        "the fixture must parse, or this test proves nothing. Got: {errs:?}"
    );
}

