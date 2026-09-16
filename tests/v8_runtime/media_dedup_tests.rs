//! Media Query CSS-First Tests
//!
//! Tests that @media directives produce pure CSS @media rules instead of
//! JS matchMedia listeners. Uses lightningcss for structural CSS inspection.

use lightningcss::printer::PrinterOptions;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;

/// Compile .st source through the full pipeline and return compiled output
fn compile_st(source: &str) -> spacetime::CompiledSpacetime {
    let ast = spacetime::parse(source).expect("test source should parse");
    spacetime::Compiler::from_ast(&ast)
        .without_runtime()
        .compile()
}

/// Parsed @media rule: (query_string, vec of (selector, declarations))
struct MediaRule {
    query: String,
    rules: Vec<StyleRule>,
}

struct StyleRule {
    selector: String,
    declarations: String,
}

/// Parse CSS and extract all @media rules with their nested style rules
fn find_media_rules(css: &str) -> Vec<MediaRule> {
    let sheet = StyleSheet::parse(css, ParserOptions::default())
        .unwrap_or_else(|e| panic!("compiled CSS should be valid:\n{}\nError: {}", css, e));

    sheet
        .rules
        .0
        .iter()
        .filter_map(|rule| match rule {
            CssRule::Media(media) => {
                let query = media
                    .query
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let rules: Vec<StyleRule> = media
                    .rules
                    .0
                    .iter()
                    .filter_map(|r| match r {
                        CssRule::Style(style) => {
                            let selector = style
                                .selectors
                                .to_css_string(PrinterOptions::default())
                                .unwrap_or_default();
                            let declarations = style
                                .declarations
                                .to_css_string(PrinterOptions::default())
                                .unwrap_or_default();
                            Some(StyleRule {
                                selector,
                                declarations,
                            })
                        }
                        _ => None,
                    })
                    .collect();
                Some(MediaRule { query, rules })
            }
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Case 1: Single @media inside selector scope
// ---------------------------------------------------------------------------
#[test]
fn test_media_emits_css_not_js() {
    let compiled = compile_st(
        r#"
.responsive {
    @media("max-width: 1023px") {
        font-size: 14px;
    }
}
"#,
    );

    // JS should be empty — CSS handles media queries natively
    assert!(
        compiled.js.is_empty(),
        "Should not generate JS for @media with styles.\nJS:\n{}",
        compiled.js
    );

    // CSS should be valid and parseable
    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(
        media_rules.len(),
        1,
        "Expected 1 @media rule.\nCSS:\n{}",
        compiled.css
    );

    let rule = &media_rules[0];
    assert!(
        rule.query.contains("1023px"),
        "Query should contain 1023px, got: {}",
        rule.query
    );

    assert_eq!(
        rule.rules.len(),
        1,
        "Should have 1 style rule inside @media"
    );
    assert!(
        rule.rules[0].selector.contains(".responsive"),
        "Selector should be .responsive, got: {}",
        rule.rules[0].selector
    );
    assert!(
        rule.rules[0].declarations.contains("font-size"),
        "Should have font-size declaration, got: {}",
        rule.rules[0].declarations
    );
}

// ---------------------------------------------------------------------------
// Case 2: Multiple @media inside same selector — no redeclaration
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_media_blocks_no_redeclaration() {
    let compiled = compile_st(
        r#"
.responsive {
    @media("max-width: 1023px") {
        font-size: 14px;
    }
    @media("max-width: 743px") {
        font-size: 12px;
    }
}
"#,
    );

    // JS should be empty
    assert!(
        compiled.js.is_empty(),
        "Should not generate JS for @media.\nJS:\n{}",
        compiled.js
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(
        media_rules.len(),
        2,
        "Expected 2 @media rules.\nCSS:\n{}",
        compiled.css
    );

    // First @media: max-width: 1023px
    assert!(
        media_rules[0].query.contains("1023px"),
        "First query should contain 1023px, got: {}",
        media_rules[0].query
    );
    assert_eq!(media_rules[0].rules.len(), 1);
    assert!(media_rules[0].rules[0].selector.contains(".responsive"));
    assert!(
        media_rules[0].rules[0].declarations.contains("14px"),
        "First rule should have 14px"
    );

    // Second @media: max-width: 743px
    assert!(
        media_rules[1].query.contains("743px"),
        "Second query should contain 743px, got: {}",
        media_rules[1].query
    );
    assert!(
        media_rules[1].rules[0].declarations.contains("12px"),
        "Second rule should have 12px"
    );
}

// ---------------------------------------------------------------------------
// Case 2b: @dark and @light — deferred (needs colorScheme %emit css)
// ---------------------------------------------------------------------------
#[test]
fn test_dark_and_light_no_redeclaration() {
    let compiled = compile_st(
        r#"
.card {
    @dark {
        background: #111;
        color: #fff;
    }
    @light {
        background: #fff;
        color: #111;
    }
}
"#,
    );

    eprintln!("JS: {}", compiled.js);
    eprintln!("CSS: {}", compiled.css);
    eprintln!("ERRORS: {:?}", compiled.pipeline_errors);

    let media_rules = find_media_rules(&compiled.css);
    assert!(
        media_rules
            .iter()
            .any(|r| r.query.contains("prefers-color-scheme")),
        "Should contain color scheme media queries.\nCSS:\n{}",
        compiled.css
    );
}

// ---------------------------------------------------------------------------
// Case 3: Same query on different selectors
// ---------------------------------------------------------------------------
#[test]
fn test_duplicate_media_queries_different_selectors() {
    let compiled = compile_st(
        r#"
.a {
    @media("max-width: 768px") {
        display: none;
    }
}
.b {
    @media("max-width: 768px") {
        display: block;
    }
}
"#,
    );

    assert!(
        compiled.js.is_empty(),
        "Should not generate JS.\nJS:\n{}",
        compiled.js
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(
        media_rules.len(),
        2,
        "Expected 2 @media rules.\nCSS:\n{}",
        compiled.css
    );

    // Collect selectors from both rules
    let selectors: Vec<&str> = media_rules
        .iter()
        .flat_map(|r| r.rules.iter().map(|s| s.selector.as_str()))
        .collect();
    assert!(
        selectors.iter().any(|s| s.contains(".a")),
        "Should have .a selector, got: {:?}",
        selectors
    );
    assert!(
        selectors.iter().any(|s| s.contains(".b")),
        "Should have .b selector, got: {:?}",
        selectors
    );
}

// ---------------------------------------------------------------------------
// Case 4: Multiple properties in body
// ---------------------------------------------------------------------------
#[test]
fn test_media_multiple_properties() {
    let compiled = compile_st(
        r#"
.sidebar {
    @media("min-width: 768px") {
        width: 300px;
        position: fixed;
    }
}
"#,
    );

    assert!(
        compiled.js.is_empty(),
        "Should not generate JS.\nJS:\n{}",
        compiled.js
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(media_rules.len(), 1);

    let decls = &media_rules[0].rules[0].declarations;
    assert!(
        decls.contains("width") && decls.contains("300px"),
        "Should have width: 300px, got: {}",
        decls
    );
    assert!(
        decls.contains("position") && decls.contains("fixed"),
        "Should have position: fixed, got: {}",
        decls
    );
}

// ---------------------------------------------------------------------------
// Case 5: Compound/nested selectors
// ---------------------------------------------------------------------------
#[test]
fn test_media_compound_selector() {
    let compiled = compile_st(
        r#"
.parent .child {
    @media("min-width: 768px") {
        color: red;
    }
}
"#,
    );

    assert!(
        compiled.js.is_empty(),
        "Should not generate JS.\nJS:\n{}",
        compiled.js
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(media_rules.len(), 1);

    let selector = &media_rules[0].rules[0].selector;
    assert!(
        selector.contains(".parent") && selector.contains(".child"),
        "Should have compound selector .parent .child, got: {}",
        selector
    );
}

// ---------------------------------------------------------------------------
// Case 7 (from original): Query string should not have double quotes
// ---------------------------------------------------------------------------
#[test]
fn test_media_query_no_double_quotes() {
    let compiled = compile_st(
        r#"
.responsive {
    @media("max-width: 1023px") {
        font-size: 14px;
    }
}
"#,
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(media_rules.len(), 1);

    // lightningcss parses the query — if it contains nested quotes it would fail
    // or show up in the serialized form. Note: lightningcss normalizes
    // `max-width: 1023px` to `(width <= 1023px)` (modern range syntax).
    let query = &media_rules[0].query;
    assert!(
        !query.contains('"'),
        "Query should not contain double quotes, got: {}",
        query
    );
    assert!(
        query.contains("1023px"),
        "Query should contain 1023px, got: {}",
        query
    );
}

// ---------------------------------------------------------------------------
// Case 8 (from original): Multiple @media produce valid CSS structure
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_media_css_structure() {
    let compiled = compile_st(
        r#"
.foo {
    @media("max-width: 1023px") {
        display: none;
    }
}
.bar {
    @media("max-width: 743px") {
        font-size: 12px;
    }
}
"#,
    );

    assert!(
        compiled.js.is_empty(),
        "Should not generate JS.\nJS:\n{}",
        compiled.js
    );

    // CSS should be valid (lightningcss parse succeeds via find_media_rules)
    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(
        media_rules.len(),
        2,
        "Expected 2 @media rules.\nCSS:\n{}",
        compiled.css
    );

    // Both queries should be present
    let queries: Vec<&str> = media_rules.iter().map(|r| r.query.as_str()).collect();
    assert!(
        queries.iter().any(|q| q.contains("1023px")),
        "Should have 1023px query, got: {:?}",
        queries
    );
    assert!(
        queries.iter().any(|q| q.contains("743px")),
        "Should have 743px query, got: {:?}",
        queries
    );
}

// ---------------------------------------------------------------------------
// Top-level @media tests
// ---------------------------------------------------------------------------

#[test]
fn top_level_media_wrapping_selectors_emits_valid_css() {
    let compiled = compile_st(
        r#"
@media("max-width: 768px") {
    .card {
        font-size: 14px;
    }
    .sidebar {
        display: none;
    }
}
"#,
    );

    // Should produce valid CSS with no pipeline errors
    assert!(
        compiled.pipeline_errors.is_empty(),
        "top-level @media should not produce pipeline errors: {:?}",
        compiled.pipeline_errors
    );

    // Should produce a @media rule in CSS output
    let media_rules = find_media_rules(&compiled.css);
    assert!(
        !media_rules.is_empty(),
        "top-level @media should emit CSS @media rule.\nCSS:\n{}",
        compiled.css
    );
    assert!(media_rules[0].query.contains("768px"));
}

#[test]
fn test_toplevel_media_uses_body() {
    let compiled = compile_st(
        r#"
@media("max-width: 1023px") {
    .foo {
        display: none;
    }
}
"#,
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(
        media_rules.len(),
        1,
        "Expected 1 @media rule.\nCSS:\n{}",
        compiled.css
    );
}

#[test]
fn test_multiple_toplevel_media_no_redeclaration() {
    let compiled = compile_st(
        r#"
@media("max-width: 1290px") {
    .tablet { display: none; }
}
@media("max-width: 1023px") {
    .mobile { display: none; }
}
@media("max-width: 743px") {
    .small { display: none; }
}
"#,
    );

    let media_rules = find_media_rules(&compiled.css);
    assert_eq!(
        media_rules.len(),
        3,
        "Expected 3 @media rules.\nCSS:\n{}",
        compiled.css
    );
}
