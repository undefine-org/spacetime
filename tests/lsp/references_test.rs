//! Tests for LSP references provider.

use spacetime::lsp::workspace::WorkspaceManager;
use spacetime::lsp::{DocumentState, provide_references};
use tower_lsp::lsp_types::{Position, Url};

fn document_from(content: &str) -> DocumentState {
    DocumentState::new(content.to_string(), 1)
}

fn test_uri() -> Url {
    Url::parse("file:///test/document.st").unwrap()
}

#[test]
fn empty_content_returns_empty() {
    let doc = document_from("");
    let workspace = WorkspaceManager::new(vec![]);
    let uri = test_uri();
    let refs = provide_references(
        &doc,
        Position {
            line: 0,
            character: 0,
        },
        &workspace,
        true,
        &uri,
    );
    assert!(refs.is_empty());
}

#[test]
fn directive_references_found() {
    let content = r#".hero {
    @scroll {
        opacity: 0 -> 1;
    }
}
.footer {
    @scroll {
        opacity: 0 -> 1;
    }
}"#;
    let doc = document_from(content);
    let workspace = WorkspaceManager::new(vec![]);
    let uri = test_uri();
    // Position on first @scroll (line 1, character 5 = on 's' in scroll)
    let refs = provide_references(
        &doc,
        Position {
            line: 1,
            character: 5,
        },
        &workspace,
        true,
        &uri,
    );
    // Should find at least 2 references to @scroll
    assert!(
        refs.len() >= 2,
        "Expected >=2 references to @scroll, got {}",
        refs.len()
    );
}

#[test]
fn variable_references_across_scopes() {
    let content = r#"$navTheme <- "dark";
.hero {
    color: $navTheme;
}
.nav {
    $navTheme <- "light";
    bg: $navTheme;
}"#;
    let doc = document_from(content);
    let workspace = WorkspaceManager::new(vec![]);
    let uri = test_uri();
    // Position on $navTheme (line 0, character 1 = on 'n')
    let refs = provide_references(
        &doc,
        Position {
            line: 0,
            character: 1,
        },
        &workspace,
        true,
        &uri,
    );
    // Should find multiple references to $navTheme
    assert!(
        refs.len() >= 3,
        "Expected >=3 references to $navTheme, got {}",
        refs.len()
    );
}

#[test]
fn references_use_document_uri() {
    let content = "$x <- \"a\";\n.hero {\n    color: $x;\n}";
    let doc = document_from(content);
    let workspace = WorkspaceManager::new(vec![]);
    let uri = Url::parse("file:///my-project/styles.st").unwrap();
    // Position on $x (line 0, character 1)
    let refs = provide_references(
        &doc,
        Position {
            line: 0,
            character: 1,
        },
        &workspace,
        true,
        &uri,
    );
    assert!(!refs.is_empty(), "Expected references for $x");
    for r in &refs {
        assert_eq!(
            r.uri, uri,
            "Reference URI should match document URI, got {}",
            r.uri
        );
        assert_ne!(
            r.uri.as_str(),
            "file:///current-document",
            "Reference URI should not be hardcoded placeholder"
        );
    }
}

#[test]
fn directive_references_use_document_uri() {
    let content = ".hero {\n    @scroll {\n        opacity: 0 -> 1;\n    }\n}\n.footer {\n    @scroll {\n        opacity: 0 -> 1;\n    }\n}";
    let doc = document_from(content);
    let workspace = WorkspaceManager::new(vec![]);
    let uri = Url::parse("file:///my-project/page.st").unwrap();
    // Position on first @scroll (line 1, character 5)
    let refs = provide_references(
        &doc,
        Position {
            line: 1,
            character: 5,
        },
        &workspace,
        true,
        &uri,
    );
    assert!(
        refs.len() >= 2,
        "Expected >=2 references to @scroll, got {}",
        refs.len()
    );
    for r in &refs {
        assert_eq!(
            r.uri, uri,
            "All reference URIs should match document URI, got {}",
            r.uri
        );
    }
}
