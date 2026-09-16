//! The same declaration gets the same answer, whatever block it sits in.
//!
//! PLAN-136 W6 / FUP-179. Measured before the fix:
//!
//! ```text
//! .a { color: #e8ee1; }                file scope -> E0958  ✓
//! .a { color: red; .b { color: #e8ee1; } }  nested -> E0958  ✓
//! @style { .a { color: #e8ee1; } }     in a block -> SILENT  ✗
//! ```
//!
//! Same bytes, same property, same malformed colour — refused or ignored
//! depending on the enclosing block. That is the two-answers-for-one-question
//! shape this whole arc exists to delete, and the validator shipped with it.
//!
//! WHY: `@style`, `@media`, `@supports` and `@keyframes` bodies are captured as
//! `RawCssBlock { source: String, span }` and never lowered to
//! `CssDeclaration`, so `declaration_value_diagnostics` — which walks
//! `scope.css_declarations` and `nested_scopes` — cannot see them. Nothing is
//! broken; the values simply never arrive.
//!
//! 535 declarations across 111 files sit inside such blocks. Small in share
//! (2.5%), but `@media` is exactly where responsive overrides live: a malformed
//! value there is a rule that silently does nothing at ONE breakpoint, which is
//! the hardest class of visual bug to catch by eye.

use spacetime::validation::css::raw_css_block_diagnostics;

/// The headline: a malformed value inside a raw block is refused.
#[test]
fn a_malformed_value_inside_a_raw_block_is_refused() {
    let diags = raw_css_block_diagnostics(".a { color: #e8ee1; }", 0);
    assert_eq!(
        diags.len(),
        1,
        "`color: #e8ee1` must be refused inside a raw block exactly as it is \
         at file scope"
    );
    assert!(
        diags[0].message.contains("#e8ee1"),
        "the diagnostic must quote the offending value, got: {}",
        diags[0].message
    );
}

/// A media query is the case that matters most — a dead rule at one breakpoint.
#[test]
fn a_malformed_value_inside_a_media_query_is_refused() {
    let src = "@media (min-width: 40rem) { .a { color: #zzz; } }";
    let diags = raw_css_block_diagnostics(src, 0);
    assert_eq!(diags.len(), 1, "a bad value in a @media body must be caught");
}

/// Valid CSS in a raw block stays valid. Without this a validator that refuses
/// everything would score as working.
#[test]
fn valid_declarations_in_a_raw_block_pass() {
    let src = "
        .a { color: #e8eef7; background: rebeccapurple; }
        .b { width: 8px; margin: 0 auto; transition-duration: 300ms; }
        .c { color: inherit; box-shadow: none; font: inherit; }
        .d { color: var(--ink); width: calc(100% - 8px); }
    ";
    let diags = raw_css_block_diagnostics(src, 0);
    assert!(
        diags.is_empty(),
        "valid CSS must pass, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Spans are offset by the block's own start, so the diagnostic points at the
/// value in the FILE, not at an offset inside a substring nobody can see.
#[test]
fn spans_are_offset_into_the_file() {
    let block = ".a { color: #e8ee1; }";
    let at_zero = raw_css_block_diagnostics(block, 0);
    let at_offset = raw_css_block_diagnostics(block, 1000);
    assert_eq!(at_zero.len(), 1);
    assert_eq!(at_offset.len(), 1);

    let (Some(a), Some(b)) = (at_zero[0].span, at_offset[0].span) else {
        panic!("both diagnostics must carry a span");
    };
    assert_eq!(
        b.start,
        a.start + 1000,
        "a block starting at byte 1000 must report its value 1000 bytes further \
         into the file, or the caret lands on unrelated source"
    );
}

/// Keyframe bodies are raw blocks too, and their percentages are selectors
/// rather than declarations — a naive parse would read `0%` as a property.
#[test]
fn keyframe_bodies_do_not_confuse_selectors_for_declarations() {
    let src = "@keyframes fade { 0% { opacity: 0; } 100% { opacity: 1; } }";
    let diags = raw_css_block_diagnostics(src, 0);
    assert!(
        diags.is_empty(),
        "keyframe stops are selectors, not declarations, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// A comment is CONTENT, not structure.
///
/// Found by the review swarm while the suite was green. The scanner skipped
/// comments in its outer loop only, so a `;` inside `/* fallback; */` ended the
/// property name and the leftover text was validated as a value — a FALSE
/// REFUSAL on legal CSS, which blocks a build. Worse than a missed diagnostic.
#[test]
fn comments_inside_a_declaration_are_not_structure() {
    for src in [
        "@media (min-width: 40rem) { .a { color: /* fallback; */ red; } }",
        ".a { color: red; /* a stray } and ; live here */ background: blue; }",
        ".a { /* leading */ color: #e8eef7; }",
        ".a { color: red /* trailing */ ; }",
    ] {
        let diags = raw_css_block_diagnostics(src, 0);
        assert!(
            diags.is_empty(),
            "legal CSS with comments must not be refused — `{src}` produced {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// A `;` or `{` inside a STRING is content too.
#[test]
fn strings_inside_a_declaration_are_not_structure() {
    for src in [
        r#".a { content: "a; b"; }"#,
        r#".a { content: "a { b"; color: #e8eef7; }"#,
        r#".a { background: url("i;mg.png"); }"#,
    ] {
        let diags = raw_css_block_diagnostics(src, 0);
        assert!(
            diags.is_empty(),
            "a delimiter inside a string is content — `{src}` produced {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// Malformed input must terminate. An earlier scanner bug HUNG the suite rather
/// than failing it, which is the failure mode that teaches you to distrust a
/// green board.
#[test]
fn malformed_input_terminates() {
    for src in [
        ".a { color: #e8ee1",       // unterminated declaration
        ".a { /* unterminated",     // unterminated comment
        r#".a { content: "unterminated"#, // unterminated string
        ".a { { { }",               // unbalanced braces
        "}}}",                      // closers only
        ":::;;;",                   // punctuation soup
    ] {
        let _ = raw_css_block_diagnostics(src, 0);
    }
}

