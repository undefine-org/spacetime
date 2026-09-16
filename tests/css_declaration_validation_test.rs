//! A malformed value in a PLAIN CSS DECLARATION is a spanned error (FUP-176).
//!
//! PLAN-122 made the scalar grammars authoritative for DIRECTIVE ARGUMENTS, so
//! `@object(color: #e8ee1)` is refused. A plain declaration did not go through a
//! capture type, so `.x { color: #e8ee1 }` compiled clean and shipped a
//! declaration the browser silently drops — the author sees an element render
//! wrong with no diagnostic anywhere in the toolchain.
//!
//! WHY THESE ASSERTIONS LOOK LOPSIDED: the negatives are the cheap half. The
//! expensive half is everything that must KEEP compiling, because this is the
//! only user-visible behaviour change in the whole arc — every other wave was a
//! deletion. A rule that rejects `#e8ee1` is worthless if it also rejects
//! `color: inherit`, and the naive form of this check (treat lightningcss's
//! `Unparsed` as an error) does exactly that: measured against the corpus it
//! flagged 45 VALID declarations — `color: inherit`, `box-shadow: none`,
//! `text-rendering: optimizeLegibility`, `transform-style: preserve-3d` — because
//! lightningcss reports CSS-wide keywords and its own coverage gaps as Unparsed
//! too.
//!
//! So the positives below are not padding. Each one is a real declaration from
//! the corpus that the naive rule broke.

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

fn assert_refused(decl: &str) {
    let src = format!(".probe {{ {decl}; }}");
    let errs = errors_for(&src);
    assert!(
        errs.iter().any(|e| e.contains("E0958")),
        "`{decl}` must be refused with E0958, got: {errs:?}"
    );
}

fn assert_accepted(decl: &str) {
    let src = format!(".probe {{ {decl}; }}");
    let errs = errors_for(&src);
    assert!(
        !errs.iter().any(|e| e.contains("E0958")),
        "`{decl}` must COMPILE, got: {errs:?}"
    );
}

#[test]
fn a_malformed_value_in_a_plain_declaration_is_refused() {
    // The original BUG-257 complaint, reduced to its last surface.
    assert_refused("color: #e8ee1");
    assert_refused("color: #zzz");
    assert_refused("color: #e8eef71");
    assert_refused("width: 8zz");
}

#[test]
fn the_alpha_hex_forms_still_pass() {
    // A naive "3 or 6 digits" rule regresses these, which is why the permitted
    // lengths are DATA (`{3|4|6|8}`) rather than a hardcoded pair.
    assert_accepted("color: #fff");
    assert_accepted("color: #fffa");
    assert_accepted("color: #e8eef7");
    assert_accepted("color: #11223344");
}

#[test]
fn css_wide_keywords_are_legal_on_every_property() {
    // 7 `color: inherit` + 13 `font: inherit` sites in the corpus die if this
    // is wrong, and lightningcss reports every one of them as Unparsed.
    for kw in ["inherit", "initial", "unset", "revert"] {
        assert_accepted(&format!("color: {kw}"));
    }
}

#[test]
fn substitution_functions_defer_to_the_browser() {
    // `var()` cannot be resolved at build time, so a property whose value is a
    // substitution is unknowable here — refusing it would be a lie.
    assert_accepted("color: var(--ink)");
    assert_accepted("width: calc(100% - var(--gutter))");
    assert_accepted("color: env(safe-area-inset-top)");
}

#[test]
fn a_property_with_no_scalar_type_is_never_validated() {
    // These are real corpus declarations that the naive rule rejected. A
    // property only gets checked when it demonstrably accepts a scalar; for
    // everything else silence is the correct answer, not a guess.
    assert_accepted("box-shadow: none");
    assert_accepted("text-rendering: optimizeLegibility");
    assert_accepted("transform-style: preserve-3d");
    assert_accepted("font-weight: 50 1000");
    assert_accepted("font: inherit");
    assert_accepted("display: grid");
    assert_accepted("grid-template-columns: repeat(auto-fill, minmax(200px, 1fr))");
}

#[test]
fn spacetime_surfaces_are_not_css_and_are_left_alone() {
    // A binding, a hole, or an animation range is not a CSS value; the checker
    // must not parse it as one.
    assert_accepted("color: $ink");
    assert_accepted("width: $w");
}

#[test]
fn the_diagnostic_points_at_the_value_not_the_rule() {
    // The span already exists on CssDeclaration and covers exactly the value
    // range — the whole reason this landed as a leaf change.
    let src = ".probe { color: #e8ee1; }";
    let ast = parse(src).expect("parses");
    let compiled = compile(&ast, CompileOptions::default());
    let d = compiled
        .pipeline_errors
        .iter()
        .find(|e| e.code == "E0958")
        .expect("E0958 expected");
    let span = d.span.expect("a spanned diagnostic");
    let sliced = &src[span.start as usize..span.end as usize];
    assert_eq!(
        sliced, "#e8ee1",
        "the span must cover the offending VALUE, got {sliced:?}"
    );
}

#[test]
fn a_function_value_is_not_ours_to_refuse() {
    // Where the parser's coverage runs out before CSS does. Every one of these
    // is valid CSS that lightningcss 1.0.0-alpha.67 cannot parse, so refusing
    // them would fail a build over a dependency's release schedule.
    //
    // This is not hypothetical: the first version of this check refused
    // `cross-fade(...)` and `paint(...)`, and refused a `radial-gradient` in a
    // real project with the message "is not a valid color" — which would have
    // sent the author looking at their colours.
    for v in [
        "color-mix(in oklch, #fff 20%, #000)",
        "light-dark(#fff, #000)",
        "oklch(from #ff0000 l c h)",
        "image-set('a.png' 1x)",
        "cross-fade(url(a.png) 50%)",
        "conic-gradient(from 0.25turn, #fff, #000)",
        "repeating-linear-gradient(45deg, #fff 0 10px, #000 10px 20px)",
        "paint(myPainter)",
        "-webkit-linear-gradient(top, #fff, #000)",
    ] {
        assert_accepted(&format!("background: {v}"));
    }
    for v in ["color-mix(in srgb, red, blue)", "oklab(59% 0.1 0.1)"] {
        assert_accepted(&format!("color: {v}"));
    }
}

#[test]
fn an_unbalanced_function_never_reaches_this_check() {
    // The balance guard in `validate_declaration_value` is a belt-and-braces
    // rule, not the primary defence: an unclosed function fails during PARSING,
    // which is both earlier and a better diagnostic. Asserted so a future
    // refactor that moves parenthesis handling knows this is already covered
    // upstream and does not "fix" it by loosening the parser.
    let errs = errors_for(".probe { background: radial-gradient(circle at 50% 50%, #fff; }");
    assert!(
        !errs.is_empty(),
        "an unbalanced function must be refused somewhere in the toolchain"
    );
}

// ── Defects found by the review swarm, each with the input that showed it ────
//
// Every one of these passed the suite before it was reported. They are kept as
// gates rather than fixed silently, because the class they belong to (a
// validator that refuses legal CSS) breaks a user's build.

#[test]
fn css_keywords_are_case_insensitive() {
    // `color: INHERIT` is legal CSS. Matching the keyword list exactly refused
    // it — a broken build over letter case.
    for kw in ["INHERIT", "Initial", "UnSet", "REVERT"] {
        assert_accepted(&format!("color: {kw}"));
    }
    assert_accepted("color: VAR(--ink)");
    assert_accepted("width: CALC(100% - VAR(--g))");
}

#[test]
fn a_paren_inside_a_string_is_content_not_structure() {
    // `url("a(b.png")` is BALANCED as CSS and unbalanced as characters, so
    // counting raw parens refused a legal declaration.
    assert_accepted("background: url(\"a(b.png\")");
    assert_accepted("content: \"(\"");
    assert_accepted("content: \")\"");
}

#[test]
fn important_is_a_flag_not_part_of_the_value() {
    assert_accepted("color: red !important");
    assert_accepted("width: 8px !important");
    // …and the flag must not launder a bad value past the check.
    assert_refused("color: #e8ee1 !important");
}

#[test]
fn an_empty_value_is_not_validated() {
    // Nothing to judge, and the parser has already had its say.
    assert_accepted("color:");
    assert_accepted("color:    ");
}
