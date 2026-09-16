//! Tests for LSP definition provider.
//!
//! Two layers:
//! 1. Classifier tests (`context_*`) — that the cursor position is recognised
//!    as a variable/template/directive site. These pass even when resolution
//!    returns None, so they are NOT enough on their own.
//! 2. RESOLUTION tests — that a resolved LOCATION (uri + span) comes back.
//!    These are the behavioral half (BUG-341): a green resolution test is the
//!    only thing that distinguishes a working go-to-definition from a dead one.

use spacetime::lsp::workspace::WorkspaceManager;
use spacetime::lsp::{
    DefinitionContext, DocumentState, extract_definition_context, provide_definition,
};
use tower_lsp::lsp_types::{GotoDefinitionResponse, Position, Url};

#[test]
fn context_directive() {
    let content = "@scroll";
    let ctx = extract_definition_context(content, 3);
    assert_eq!(
        ctx,
        DefinitionContext::Directive {
            name: "scroll".to_string()
        }
    );
}

#[test]
fn context_template() {
    // "&ora-nav(\"Ora\")"
    //  offset 0 is the '&'
    let content = "&ora-nav(\"Ora\")";
    let ctx = extract_definition_context(content, 0);
    assert_eq!(
        ctx,
        DefinitionContext::Template {
            name: "ora-nav".to_string()
        }
    );
}

#[test]
fn context_variable() {
    // "$navTheme <- \"dark\""
    //  offset 0 is the '$'
    let content = "$navTheme <- \"dark\"";
    let ctx = extract_definition_context(content, 0);
    assert_eq!(
        ctx,
        DefinitionContext::Variable {
            name: "navTheme".to_string()
        }
    );
}

#[test]
fn context_none() {
    let content = "plain text";
    let ctx = extract_definition_context(content, 5);
    assert_eq!(ctx, DefinitionContext::None);
}

fn document_from(content: &str) -> DocumentState {
    DocumentState::new(content.to_string(), 1)
}

// =============================================================================
// Resolution tests — assert a RESOLVED LOCATION, not a classification
// =============================================================================

/// Byte-offset → Position for the NTH occurrence (1-based) of `needle`, so a
/// test can point the cursor at a specific usage without manual column math.
fn nth_pos(doc: &DocumentState, needle: &str, n: usize) -> Position {
    let mut offset = 0usize;
    for _ in 0..n {
        let found = doc.content[offset..]
            .find(needle)
            .unwrap_or_else(|| panic!("needle `{needle}` occurrence #{n} not found"));
        offset += found + needle.len();
    }
    let offset = offset.saturating_sub(needle.len());
    doc.position_mapper.position_from_offset(offset)
}

/// Unwrap a scalar definition response or panic with a readable message.
fn scalar(def: Option<GotoDefinitionResponse>) -> tower_lsp::lsp_types::Location {
    match def {
        Some(GotoDefinitionResponse::Scalar(loc)) => loc,
        other => panic!("expected a scalar definition location, got {other:?}"),
    }
}

/// gh-30 / BUG-341 ACCEPTANCE: a local `$binding` reference resolves to the
/// `@data` declaration that binds it — a LOCATION comes back, not None.
#[tokio::test]
async fn local_binding_resolves_to_definition_location() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let manager = WorkspaceManager::new(vec![]);
    manager.add_root(root.clone()).await;

    let file = root.join("list.st");
    let content = "@data inline $items : [];\n.rows {\n    color: $items;\n}\n";
    std::fs::write(&file, content).unwrap();
    manager.index_content(&file, content, 1).await.unwrap();

    let doc = DocumentState::new(content.to_string(), 1);
    let doc_uri = Url::from_file_path(&file).unwrap();
    let registry = manager.form_registry().await;

    // Cursor on the `$items` USAGE inside `.rows` (2nd occurrence).
    let def = provide_definition(&doc, nth_pos(&doc, "$items", 2), &registry, &manager, &doc_uri);
    let loc = scalar(def);
    assert_eq!(loc.uri, doc_uri, "local binding resolves within the same file");
    assert_eq!(
        loc.range.start.line, 0,
        "lands on the `@data $items` declaration (line 0)"
    );
}

/// gh-30 / BUG-341 ACCEPTANCE: a reference to a binding defined in an
/// @import-ed module resolves CROSS-FILE to the module's declaration.
#[tokio::test]
async fn imported_binding_resolves_cross_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    // The binding is DEFINED in a module the main file imports. Both files must
    // exist BEFORE `add_root`, which scans the workspace to build the index —
    // writing them afterwards leaves nothing to index and the lookup can only
    // ever return None.
    let module = root.join("data.st");
    std::fs::write(&module, "@data inline $products : [];\n").unwrap();
    let main = root.join("main.st");
    let main_content = "@import \"./data.st\"\n.rows {\n    color: $products;\n}\n";
    std::fs::write(&main, main_content).unwrap();

    let manager = WorkspaceManager::new(vec![]);
    manager.add_root(root.clone()).await;

    let doc = DocumentState::new(main_content.to_string(), 1);
    let doc_uri = Url::from_file_path(&main).unwrap();
    let registry = manager.form_registry().await;
    let def = provide_definition(
        &doc,
        nth_pos(&doc, "$products", 1),
        &registry,
        &manager,
        &doc_uri,
    );
    let loc = scalar(def);
    assert_eq!(
        loc.uri.to_file_path().unwrap(),
        module,
        "imported binding resolves to the defining module, not the referencing file"
    );
}

/// gh-30 / BUG-341 ACCEPTANCE: a `&template(...)` invocation resolves to the
/// `@template &name` declaration.
#[tokio::test]
async fn template_resolves_to_definition_location() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let manager = WorkspaceManager::new(vec![]);
    manager.add_root(root.clone()).await;

    let file = root.join("page.st");
    let content =
        "@template &card($label) { <div class=\"c\">`$label`</div> }\n.hero {\n    &card(\"Hi\");\n}\n";
    std::fs::write(&file, content).unwrap();
    manager.index_content(&file, content, 1).await.unwrap();

    let doc = DocumentState::new(content.to_string(), 1);
    let doc_uri = Url::from_file_path(&file).unwrap();
    let registry = manager.form_registry().await;

    // Cursor on the invocation `&card("Hi")` (the template usage), not the decl.
    let def = provide_definition(
        &doc,
        nth_pos(&doc, "&card(\"Hi\")", 1),
        &registry,
        &manager,
        &doc_uri,
    );
    let loc = scalar(def);
    assert_eq!(loc.uri, doc_uri, "template resolves within the same file");
    assert_eq!(
        loc.range.start.line, 0,
        "lands on the `@template &card` declaration (line 0)"
    );
}

/// BUG-341 EDGE: a binding shadowed by an inner scope-local declaration
/// resolves to the NEAREST ENCLOSING definition, not the file-level one.
#[tokio::test]
async fn shadowed_binding_resolves_to_nearest_enclosing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let manager = WorkspaceManager::new(vec![]);
    manager.add_root(root.clone()).await;

    let file = root.join("shadow.st");
    // File-level `@data $count` (line 0) is shadowed by the scope-local
    // `$count number: 1;` (line 2) inside `.outer`. The reference on line 3
    // must resolve to the INNER declaration.
    let content = "@data $count : 0;\n.outer {\n    $count number: 1;\n    color: $count;\n}\n";
    std::fs::write(&file, content).unwrap();
    manager.index_content(&file, content, 1).await.unwrap();

    let doc = DocumentState::new(content.to_string(), 1);
    let doc_uri = Url::from_file_path(&file).unwrap();
    let registry = manager.form_registry().await;

    // Cursor on the `$count` USAGE on line 3 (3rd occurrence).
    let def = provide_definition(&doc, nth_pos(&doc, "$count", 3), &registry, &manager, &doc_uri);
    let loc = scalar(def);
    assert_eq!(
        loc.range.start.line, 2,
        "shadowed reference resolves to the nearest enclosing (inner) declaration, got line {}",
        loc.range.start.line
    );
}

/// BUG-341 NEGATIVE: a variable with no definition anywhere resolves to None —
/// we must NOT have made everything resolve to something.
#[tokio::test]
async fn variable_with_no_definition_returns_none() {
    let manager = WorkspaceManager::new(vec![]);
    let doc = document_from(".x { color: $ghost; }");
    let doc_uri = Url::parse("file:///test/ghost.st").unwrap();
    let registry = manager.form_registry().await;
    let def = provide_definition(&doc, nth_pos(&doc, "$ghost", 1), &registry, &manager, &doc_uri);
    assert!(def.is_none(), "undefined binding must not resolve to something");
}

/// BUG-341 NEGATIVE: a template invocation with no `@template` declaration
/// resolves to None.
#[tokio::test]
async fn template_with_no_definition_returns_none() {
    let manager = WorkspaceManager::new(vec![]);
    let doc = document_from(".hero { &ghost-panel(\"x\"); }");
    let doc_uri = Url::parse("file:///test/ghost.st").unwrap();
    let registry = manager.form_registry().await;
    let def = provide_definition(
        &doc,
        nth_pos(&doc, "&ghost-panel", 1),
        &registry,
        &manager,
        &doc_uri,
    );
    assert!(def.is_none(), "undefined template must not resolve to something");
}

/// gh-30 regression guard: a PROJECT macro in `_prelude.st` still resolves to
/// its real file (absolute path), not None.
#[tokio::test]
async fn project_macro_resolves_to_prelude() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let prelude = root.join("_prelude.st");
    std::fs::write(
        &prelude,
        r#"%macro project-card {
  %form {
    @project-card(title: $title:string) {
      $body:properties
    }
  }
}
"#,
    )
    .unwrap();

    let manager = WorkspaceManager::new(vec![]);
    manager.add_root(root.clone()).await;
    let registry = manager.form_registry().await;

    let doc = document_from("@project-card(title: \"Hi\")");
    let doc_uri = Url::from_file_path(&prelude).unwrap();
    let def = provide_definition(
        &doc,
        Position { line: 0, character: 5 },
        &registry,
        &manager,
        &doc_uri,
    );
    let loc = scalar(def);
    assert_eq!(
        loc.uri.to_file_path().unwrap(),
        prelude,
        "project macro must resolve to the prelude file"
    );
}

/// gh-30 / BUG-341: a STDLIB directive (`@scroll`) must resolve to a location,
/// not None. Stdlib source_file is a virtual embedded path (`stdlib/...`) that
/// `Url::from_file_path` refuses — the resolver must emit a usable URI for it.
#[tokio::test]
async fn stdlib_directive_resolves_to_source() {
    let manager = WorkspaceManager::new(vec![]);
    let doc = document_from("@scroll");
    let doc_uri = Url::parse("file:///test/page.st").unwrap();
    let registry = manager.form_registry().await;
    let def = provide_definition(
        &doc,
        Position { line: 0, character: 2 },
        &registry,
        &manager,
        &doc_uri,
    );
    let loc = scalar(def);
    let scheme = loc.uri.scheme();
    assert!(
        scheme == "file" || scheme == "spacetime",
        "stdlib directive must resolve to a file or virtual-doc URI, got {scheme}: {}",
        loc.uri
    );
}

/// gh-30 / BUG-341: an embedded STDLIB primitive (`intersection`) must resolve
/// to a location, not None.
#[tokio::test]
async fn stdlib_primitive_resolves_to_source() {
    let manager = WorkspaceManager::new(vec![]);
    let doc = document_from("%binds {\n    intersection(&self) -> { $visible }\n}");
    let doc_uri = Url::parse("file:///test/page.st").unwrap();
    let registry = manager.form_registry().await;
    let def = provide_definition(
        &doc,
        Position { line: 1, character: 4 },
        &registry,
        &manager,
        &doc_uri,
    );
    let loc = scalar(def);
    let scheme = loc.uri.scheme();
    assert!(
        scheme == "file" || scheme == "spacetime",
        "embedded stdlib primitive must resolve to a file or virtual-doc URI, got {scheme}: {}",
        loc.uri
    );
}
