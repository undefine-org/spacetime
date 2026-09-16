//! Go-to-definition provider for Spacetime LSP.
//!
//! Provides navigation to macro and primitive definitions from usage sites.

use std::fs;

use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

use super::document::DocumentState;
use super::form_registry::FormRegistry;
use super::position::PositionMapper;
use super::workspace::WorkspaceManager;
use super::workspace::{ReferenceLocation, SymbolId, SymbolKind};
use std::path::Path;

use crate::parser::SourceSpan;

// =============================================================================
// Definition Context
// =============================================================================

/// Context for what definition to look up.
#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionContext {
    /// Looking for a directive definition
    Directive { name: String },
    /// Looking for a primitive definition (inside %binds)
    Primitive { name: String },
    /// Looking for a variable reference ($name)
    Variable { name: String },
    /// Looking for a template reference (&name)
    Template { name: String },
    /// No definition context found
    None,
}

// =============================================================================
// Main Definition Function
// =============================================================================

/// Provide go-to-definition for the given document position.
pub fn provide_definition(
    doc: &DocumentState,
    position: Position,
    form_registry: &FormRegistry,
    workspace: &WorkspaceManager,
    doc_uri: &Url,
) -> Option<GotoDefinitionResponse> {
    let offset = doc.position_mapper.offset_from_position(position);
    let content = &doc.content;

    let context = extract_definition_context(content, offset);

    match context {
        DefinitionContext::Directive { name } => {
            let sig = form_registry.get_directive(&name)?;

            // Resolve the source file to a URI + range. Stdlib source_file is a
            // virtual embedded path (e.g. `stdlib/macros/presets.st`) that
            // `Url::from_file_path` refuses — this emits a real file URI when
            // the file is on disk, else a stable virtual document URI (gh-30).
            let (def_uri, range) = resolve_source_location(&sig.source_file, &sig.definition_span)?;

            Some(GotoDefinitionResponse::Scalar(Location {
                uri: def_uri,
                range,
            }))
        }
        DefinitionContext::Primitive { name } => {
            let prim = form_registry.get_primitive(&name)?;

            // Same embedded-source resolution as directives (gh-30).
            let (def_uri, range) = resolve_source_location(&prim.source_file, &prim.definition_span)?;

            Some(GotoDefinitionResponse::Scalar(Location {
                uri: def_uri,
                range,
            }))
        }
        DefinitionContext::Variable { name } => {
            resolve_variable_definition(doc, &name, offset, workspace, doc_uri)
        }
        DefinitionContext::Template { name } => {
            resolve_template_definition(doc, &name, workspace, doc_uri)
        }
        DefinitionContext::None => None,
    }
}


// =============================================================================
// Binding + Template resolution (gh-30 / BUG-341)
// =============================================================================

/// Resolve a `$binding` reference to its definition.
///
/// Prefers the NEAREST ENCLOSING in-document declaration (which handles scope
/// shadowing), then falls back to the workspace's global binding index (which
/// covers bindings imported from other files).
fn resolve_variable_definition(
    doc: &DocumentState,
    name: &str,
    offset: usize,
    workspace: &WorkspaceManager,
    doc_uri: &Url,
) -> Option<GotoDefinitionResponse> {
    if let Some(span) = nearest_enclosing_binding_def(doc, name, offset) {
        return Some(in_doc_location(doc, doc_uri, span));
    }
    let symbol = SymbolId::new(SymbolKind::Binding, name.to_string());
    let loc = workspace.get_definition(&symbol)?;
    location_from_reference(&loc)
}

/// Resolve a `&template` reference to its declaration.
///
/// Prefers an in-document `@template` declaration, then the workspace template
/// index (imported templates).
fn resolve_template_definition(
    doc: &DocumentState,
    name: &str,
    workspace: &WorkspaceManager,
    doc_uri: &Url,
) -> Option<GotoDefinitionResponse> {
    if let Some(span) = in_doc_template_def(doc, name) {
        return Some(in_doc_location(doc, doc_uri, span));
    }
    let symbol = SymbolId::new(SymbolKind::Template, name.to_string());
    let loc = workspace.get_definition(&symbol)?;
    location_from_reference(&loc)
}

/// The nearest PRECEDING binding declaration for `name` at or before `offset`
/// in the document. Walks the flat FormMatch list for binding-producing
/// directives (`@data`/`@computed`/`@fn` + scope-local `local-state`/`element-ref`)
/// and picks the declaration that starts closest to (but not after) the cursor —
/// i.e. the innermost enclosing scope's declaration when a binding is shadowed.
fn nearest_enclosing_binding_def(
    doc: &DocumentState,
    name: &str,
    offset: usize,
) -> Option<SourceSpan> {
    let ast = doc.ast.as_ref()?;
    let mut best: Option<SourceSpan> = None;
    for m in &ast.matches {
        let is_binding_def = matches!(
            m.macro_name.as_str(),
            "data" | "computed" | "fn" | "local-state" | "element-ref"
        );
        if !is_binding_def {
            continue;
        }
        let Some(captured) = m.get_binding("name").or_else(|| m.get_ident("name")) else {
            continue;
        };
        let captured = captured.strip_prefix('$').unwrap_or(captured);
        if captured != name {
            continue;
        }
        let Some(span) = m.get_capture_span("name") else {
            continue;
        };
        if span.start <= offset && best.map_or(true, |b: SourceSpan| span.start > b.start) {
            best = Some(span);
        }
    }
    best
}

/// The in-document `@template &name` declaration's name-capture span.
fn in_doc_template_def(doc: &DocumentState, name: &str) -> Option<SourceSpan> {
    let ast = doc.ast.as_ref()?;
    ast.matches.iter().find_map(|m| {
        if m.macro_name != "template" {
            return None;
        }
        if m.get_ident("name")? != name {
            return None;
        }
        m.get_capture_span("name")
    })
}

/// Build an in-document location (the doc's own URI + its position mapper).
fn in_doc_location(doc: &DocumentState, doc_uri: &Url, span: SourceSpan) -> GotoDefinitionResponse {
    let diag_span = crate::diagnostics::SourceSpan::from(span);
    GotoDefinitionResponse::Scalar(Location {
        uri: doc_uri.clone(),
        range: doc.position_mapper.span_to_range(&diag_span),
    })
}

/// Convert a workspace index definition location (canonical file + span) into
/// an LSP Location by reading the file to compute line/column positions.
fn location_from_reference(loc: &ReferenceLocation) -> Option<GotoDefinitionResponse> {
    let uri = Url::from_file_path(&loc.file).ok()?;
    let content = std::fs::read_to_string(&loc.file).ok()?;
    let mapper = PositionMapper::new(&content);
    let range = mapper.span_to_range(&crate::diagnostics::SourceSpan::from(loc.span));
    Some(GotoDefinitionResponse::Scalar(Location { uri, range }))
}

/// Resolve a definition's source file to a (URI, range) pair.
///
/// `source_file` may be:
/// - an ABSOLUTE path (project macros) → a real `file://` URI;
/// - a repo-root-relative path (stdlib on disk) → an absolute `file://` URI;
/// - a VIRTUAL embedded path (`stdlib/...`) with no on-disk presence → a stable
///   virtual document URI (`spacetime://stdlib/...`) over the embedded source.
///
/// Returns None only when the source is genuinely unavailable.
fn resolve_source_location(source_file: &Path, span: &SourceSpan) -> Option<(Url, Range)> {
    // Absolute path on disk — the project-macro case.
    if source_file.is_absolute() {
        let uri = Url::from_file_path(source_file).ok()?;
        let range = compute_definition_range(source_file, span)?;
        return Some((uri, range));
    }

    // On-disk relative path (the stdlib under the source tree): prefer a real
    // file URI so the editor can open it directly.
    if let Some(range) = compute_definition_range(source_file, span) {
        if let Ok(abs) = std::env::current_dir().map(|c| c.join(source_file)) {
            if let Ok(uri) = Url::from_file_path(&abs) {
                return Some((uri, range));
            }
        }
        // A relative path we could read but not turn into a file URI.
        if let Some(uri) = virtual_stdlib_uri(source_file) {
            return Some((uri, range));
        }
    }

    // Truly embedded-only source: no on-disk file. Compute the range over the
    // embedded content and emit the stable virtual document URI (gh-30).
    let uri = virtual_stdlib_uri(source_file)?;
    let content = embedded_stdlib_source(source_file)?;
    let mapper = PositionMapper::new(&content);
    let range = mapper.span_to_range(&crate::diagnostics::SourceSpan::from(*span));
    Some((uri, range))
}

/// The stable virtual document URI for an embedded stdlib source path.
fn virtual_stdlib_uri(source_file: &Path) -> Option<Url> {
    Url::parse(&format!("spacetime://stdlib/{}", source_file.display())).ok()
}

/// Fetch embedded stdlib source by its virtual path (`stdlib/<category>/<file>`).
fn embedded_stdlib_source(source_file: &Path) -> Option<String> {
    let lossy = source_file.to_string_lossy();
    let key = lossy.strip_prefix("stdlib/")?;
    for (category, files) in crate::stdlib_embedded::all_embedded_files() {
        for f in files {
            if format!("{}/{}", category, f.path) == key {
                return Some(f.content.to_string());
            }
        }
    }
    None
}

// =============================================================================
// Context Extraction
// =============================================================================

/// Extract definition context at a given offset.
pub fn extract_definition_context(content: &str, offset: usize) -> DefinitionContext {
    // Check if we're on a directive name (after @)
    if let Some(name) = find_directive_name_at_offset(content, offset) {
        return DefinitionContext::Directive { name };
    }

    // Check if we're on a primitive name (inside %binds)
    if let Some(name) = find_primitive_name_at_offset(content, offset) {
        return DefinitionContext::Primitive { name };
    }

    // Check for prefixed sigils: $variable, &template
    if let Some(name) = find_prefixed_name_at_offset(content, offset, '$') {
        return DefinitionContext::Variable { name };
    }
    if let Some(name) = find_prefixed_name_at_offset(content, offset, '&') {
        return DefinitionContext::Template { name };
    }

    DefinitionContext::None
}

/// Find a prefixed name at offset. The prefix is a single char like `$`, or `&`.
fn find_prefixed_name_at_offset(content: &str, offset: usize, prefix: char) -> Option<String> {
    let before = &content[..offset.min(content.len())];
    let bytes = content.as_bytes();
    let prefix_byte = prefix as u8;

    // Find the prefix position
    let prefix_pos = if offset < bytes.len() && bytes[offset] == prefix_byte {
        Some(offset)
    } else {
        let mut pos = None;
        for (i, ch) in before.char_indices().rev() {
            if ch == prefix {
                pos = Some(i);
                break;
            }
            if !ch.is_alphanumeric() && ch != '-' && ch != '_' {
                break;
            }
        }
        pos
    };

    let prefix_pos = prefix_pos?;
    let name_start = prefix_pos + prefix.len_utf8();
    let after_prefix = &content[name_start..];

    let name: String = after_prefix
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    if name.is_empty() {
        return None;
    }

    let span_end = name_start + name.len();
    if offset > span_end {
        return None;
    }

    Some(name)
}

/// Find primitive name at offset (inside %binds blocks).
fn find_primitive_name_at_offset(content: &str, offset: usize) -> Option<String> {
    let before = &content[..offset.min(content.len())];

    // Look for %binds context
    let binds_pos = before.rfind("%binds")?;

    // Check if we're still inside the binds block (look for matching braces)
    let after_binds = &before[binds_pos..];
    let open_braces = after_binds.matches('{').count();
    let close_braces = after_binds.matches('}').count();
    if open_braces <= close_braces {
        // We've exited the %binds block
        return None;
    }

    // Now we're inside %binds - look for primitive name
    // Primitive calls look like: primitive_name(&el, args) -> { $outputs }

    // Find the start of the current statement (after last newline or opening brace)
    let stmt_start = before.rfind(['\n', '{']).map(|p| p + 1).unwrap_or(0);

    let stmt_before = &before[stmt_start..];

    // Look for an identifier at the start of the statement (the primitive name)
    let trimmed = stmt_before.trim_start();
    let whitespace_len = stmt_before.len() - trimmed.len();
    let identifier_start = stmt_start + whitespace_len;

    // Check if we're still in the identifier part (before any special chars).
    // NB an EMPTY `trimmed` is the cursor sitting on the identifier's FIRST
    // character (nothing before it on the statement but whitespace). That is a
    // legitimate go-to-definition position — the name is entirely AFTER the
    // offset — so fall through and let `id_after` supply it. Returning None
    // here made clicking the start of a primitive name silently do nothing.
    if trimmed.is_empty() {
        let after = &content[offset..];
        let id_after: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if id_after.is_empty() {
            return None;
        }
        return Some(id_after);
    }

    // Check if the trimmed part only contains valid identifier chars
    // (that means we're on the identifier)
    if trimmed
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '_' && c != '-')
    {
        // There's a special char between the start and our position
        // Check if it's before our current position in the trimmed part
        let first_special = trimmed.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
        if let Some(special_pos) = first_special {
            let cursor_in_trimmed = offset.saturating_sub(identifier_start);
            if cursor_in_trimmed > special_pos {
                // We're past the identifier
                return None;
            }
        }
    }

    // Get the identifier part from before
    let id_before: String = trimmed
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();

    // Get the rest of the identifier from after the offset
    let after = &content[offset..];
    let id_after: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();

    let full_id = format!("{}{}", id_before, id_after);

    if full_id.is_empty() {
        None
    } else {
        Some(full_id)
    }
}

/// Find directive name at offset.
fn find_directive_name_at_offset(content: &str, offset: usize) -> Option<String> {
    let before = &content[..offset.min(content.len())];

    // Look backwards for @
    let at_pos = before.rfind('@')?;
    let between = &before[at_pos + 1..];

    // Check for special chars that would break the directive name
    // Allow space for multi-word directives like "@data stream"
    if between.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != ' ') {
        return None;
    }

    // Find the end of the directive name
    let after = &content[offset..];
    let end_offset = after
        .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .unwrap_or(after.len());

    let name = format!("{}{}", between, &after[..end_offset]);

    if name.is_empty() { None } else { Some(name) }
}

/// Compute the LSP range for a definition span in a file.
fn compute_definition_range(
    source_file: &std::path::Path,
    span: &crate::parser::SourceSpan,
) -> Option<Range> {
    // Try to read the file to compute accurate positions
    let content = fs::read_to_string(source_file).ok()?;
    let mapper = PositionMapper::new(&content);

    // Convert parser::SourceSpan to diagnostics::SourceSpan
    let diag_span = crate::diagnostics::SourceSpan::from(*span);
    Some(mapper.span_to_range(&diag_span))
}

// =============================================================================
// Additional Definition Types (for future expansion)
// =============================================================================

/// Provide definition for a pattern reference.
pub fn provide_pattern_definition(
    doc: &DocumentState,
    pattern_name: &str,
    doc_uri: &Url,
) -> Option<GotoDefinitionResponse> {
    let ast = doc.ast.as_ref()?;

    for pattern in &ast.patterns {
        if pattern.name == pattern_name {
            let diag_span = crate::diagnostics::SourceSpan::from(pattern.span);
            let range = doc.position_mapper.span_to_range(&diag_span);
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: doc_uri.clone(),
                range,
            }));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_directive_name_simple() {
        let content = "@scroll";
        let name = find_directive_name_at_offset(content, 3);
        assert_eq!(name, Some("scroll".to_string()));
    }

    #[test]
    fn test_find_directive_name_at_start() {
        let content = "@scroll";
        let name = find_directive_name_at_offset(content, 1);
        assert_eq!(name, Some("scroll".to_string()));
    }

    #[test]
    fn test_find_directive_name_at_end() {
        let content = "@scroll";
        let name = find_directive_name_at_offset(content, 7);
        assert_eq!(name, Some("scroll".to_string()));
    }

    #[test]
    fn test_find_directive_name_with_hyphen() {
        let content = "@fade-in";
        let name = find_directive_name_at_offset(content, 5);
        assert_eq!(name, Some("fade-in".to_string()));
    }

    #[test]
    fn test_find_directive_name_with_space() {
        let content = "@data stream";
        let name = find_directive_name_at_offset(content, 7);
        assert_eq!(name, Some("data stream".to_string()));
    }

    #[test]
    fn test_find_directive_name_in_context() {
        let content = ".hero { @scroll { opacity: 0 -> 1; } }";
        let name = find_directive_name_at_offset(content, 12);
        assert_eq!(name, Some("scroll".to_string()));
    }

    #[test]
    fn test_find_directive_name_none() {
        let content = "some text without directive";
        let name = find_directive_name_at_offset(content, 10);
        assert!(name.is_none());
    }

    #[test]
    fn test_extract_definition_context_directive() {
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
    fn test_extract_definition_context_none() {
        let content = "some text";
        let ctx = extract_definition_context(content, 3);
        assert_eq!(ctx, DefinitionContext::None);
    }

    #[test]
    fn test_find_primitive_in_binds() {
        let content = r#"%binds {
    intersection(&self) -> { $visible }
}"#;
        // Offset pointing to 'inter' in 'intersection'
        // "%binds {\n    inter" = 14 chars
        let name = find_primitive_name_at_offset(content, 14);
        assert_eq!(name, Some("intersection".to_string()));
    }

    #[test]
    fn test_find_primitive_not_in_binds() {
        let content = "intersection(&self)";
        let name = find_primitive_name_at_offset(content, 5);
        assert!(name.is_none());
    }

    #[test]
    fn test_extract_definition_context_primitive() {
        let content = r#"%binds {
    scroll(&el) -> { $progress }
}"#;
        // %binds {\n    scroll
        // 01234567 8 9012345  = 14 to be on 'c' in 'scroll'
        // First verify find_primitive_name_at_offset works
        let name = find_primitive_name_at_offset(content, 14);
        assert_eq!(
            name,
            Some("scroll".to_string()),
            "find_primitive_name_at_offset failed"
        );

        // Now test extract_definition_context
        let ctx = extract_definition_context(content, 14);
        assert_eq!(
            ctx,
            DefinitionContext::Primitive {
                name: "scroll".to_string()
            }
        );
    }
}
