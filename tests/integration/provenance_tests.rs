//! Integration tests for compiler-side provenance injection.
//!
//! Tests 7-9 from ITEM-107-017: verify `inject_provenance_into_matches`
//! adds data-st-id / data-st-origin attributes to template body HTML.

use spacetime::parser::SourceSpan;
use spacetime::parser::ast::{ScopeBlock, ScopeKind};
use spacetime::syntax::{CapturedValue, FormMatch, inject_provenance_into_matches};
use std::collections::HashMap;
use std::path::Path;

// FEAT-119: template body html lives on the World-A `@template:<name>` scope, not the
// capture. This helper builds the template match + its scope (carrying `html`), and
// `inject_one` runs provenance injection and returns the resulting SCOPE html.
fn make_template_match(
    html: &str,
    selector: &str,
    source_file: &str,
    offset: usize,
) -> (FormMatch, ScopeBlock) {
    let mut captures = HashMap::new();
    captures.insert("name".to_string(), CapturedValue::Ident("t".to_string()));
    captures.insert("body".to_string(), CapturedValue::ComponentBody);
    let fm = FormMatch {
        macro_name: "template".to_string(),
        matched_macro: None,
        captures,
        capture_spans: HashMap::new(),
        selector: Some(selector.to_string()),
        span: SourceSpan::new(offset, offset + 100),
        source_file: Some(source_file.to_string()),
        doc: None,
        namespace_qualifier: Vec::new(),
    };
    let scope = ScopeBlock {
        kind: ScopeKind::Construct("template".to_string()),
        selector: "@template:t".to_string(),
        html: html.to_string(),
        ..Default::default()
    };
    (fm, scope)
}

fn inject_one(fm: FormMatch, scope: ScopeBlock, default_sf: &str, ws: &Path) -> String {
    let matches = vec![fm];
    let mut scopes = vec![scope];
    inject_provenance_into_matches(&matches, &mut scopes, default_sf, ws);
    scopes.into_iter().next().unwrap().html
}

#[test]
#[cfg(debug_assertions)]
fn test_template_provenance_injected_in_debug() {
    let (fm, scope) = make_template_match(
        "<h3>Card Title</h3><p>Description</p>",
        ".card",
        "card.st",
        100,
    );
    let ws = std::path::Path::new(".");
    let html = inject_one(fm, scope, "card.st", ws);
    {
        let body_html = &html;
        assert!(
            body_html.contains("data-st-id"),
            "debug build should inject data-st-id"
        );
        assert!(
            body_html.contains("data-st-origin"),
            "debug build should inject data-st-origin"
        );
        assert!(
            body_html.contains("card.st::.card §template"),
            "origin should reference template definition, got: {}",
            body_html
        );
    }
}

#[test]
#[cfg(not(debug_assertions))]
fn test_template_provenance_no_op_in_release() {
    let original_html = "<h3>Card Title</h3><p>Description</p>";
    let (fm, scope) = make_template_match(original_html, ".card", "card.st", 100);
    let ws = std::path::Path::new(".");
    let html = inject_one(fm, scope, "card.st", ws);
    assert_eq!(html, original_html, "release build should not modify HTML");
}

#[test]
#[cfg(debug_assertions)]
fn test_template_instances_share_same_origin() {
    // Two separate FormMatch objects with identical parameters simulate
    // two instances of the same template definition.
    let src = "<h3>Card Title</h3>";
    let (fm1, sc1) = make_template_match(src, ".card", "card.st", 100);
    let (fm2, sc2) = make_template_match(src, ".card", "card.st", 100);

    let ws = std::path::Path::new(".");
    let html1 = inject_one(fm1, sc1, "card.st", ws);
    let html2 = inject_one(fm2, sc2, "card.st", ws);

    assert_eq!(
        html1, html2,
        "same template definition should produce identical provenance HTML"
    );

    // Extract and compare data-st-origin values explicitly
    fn extract_origin(html: &str) -> Option<String> {
        let marker = "data-st-origin=\"";
        let start = html.find(marker)? + marker.len();
        let end = html[start..].find('"')? + start;
        Some(html[start..end].to_string())
    }

    let origin1 = extract_origin(&html1).expect("matches1 should have data-st-origin");
    let origin2 = extract_origin(&html2).expect("matches2 should have data-st-origin");
    assert_eq!(
        origin1, origin2,
        "all instances of same template should share same data-st-origin"
    );
}

/// BUG-024: Subfolder .st files in a temp dir emit basename-only origin (the bug).
/// After the fix, origins should contain workspace-relative paths.
#[test]
#[cfg(debug_assertions)]
fn test_subfolder_template_uses_basename_only_current_bug() {
    use tempfile::TempDir;

    let dir = TempDir::new().expect("tempdir");
    let modules = dir.path().join("modules");
    std::fs::create_dir_all(&modules).expect("create modules/");

    let card_path = modules.join("card.st");
    std::fs::write(
        &card_path,
        "<template .card>\n  <h2>Card Title</h2>\n</template>\n",
    )
    .expect("write card.st");

    // Absolute source_file as set by resolve_imports for a subfolder file
    let source_file = card_path.to_string_lossy().to_string();
    assert!(
        source_file.contains("modules/card.st"),
        "temp path should contain modules/card.st, got: {}",
        source_file
    );

    let (fm, scope) = make_template_match("<h2>Card Title</h2>", ".card", &source_file, 100);
    let ws = dir.path();
    let html = inject_one(fm, scope, "index.st", ws);
    // After fix: origin should be workspace-relative (modules/card.st)
    assert!(
        html.contains("modules/card.st::.card §template h2"),
        "BUG-024: origin should be modules/card.st::, got: {}",
        html
    );
}
