//! Find All References support
//!
//! Provides "Find All References" LSP functionality using the workspace's
//! usage index.

use tower_lsp::lsp_types::{Location, Position, Url};

use super::document::DocumentState;
use super::position::PositionMapper;
use super::workspace::{ReferenceKind, WorkspaceManager};

/// Find all references to the symbol at the given position.
///
/// Currently supports single-document text-based references.
pub fn provide_references(
    doc: &DocumentState,
    position: Position,
    _workspace: &WorkspaceManager,
    _include_declaration: bool,
    doc_uri: &Url,
) -> Vec<Location> {
    let offset = doc.position_mapper.offset_from_position(position);
    let content = &doc.content;

    let symbol = match find_symbol_at_offset(content, offset) {
        Some(s) => s,
        None => return vec![],
    };

    find_all_occurrences(content, &symbol, &doc.position_mapper, doc_uri)
}

/// A symbol found at a cursor position.
#[derive(Debug)]
struct SymbolAtCursor {
    search_text: String,
}

/// Find the symbol at the given byte offset.
fn find_symbol_at_offset(content: &str, offset: usize) -> Option<SymbolAtCursor> {
    let bytes = content.as_bytes();
    if offset >= bytes.len() {
        return None;
    }

    let mut start = offset;
    while start > 0 {
        let prev = start - 1;
        let b = bytes[prev];
        if b == b'@' || b == b'$' || b == b'~' || b == b'&' || b == b'%' {
            start = prev;
            break;
        }
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
            start = prev;
        } else {
            break;
        }
    }

    let mut end = offset;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-' || bytes[end] == b'_')
    {
        end += 1;
    }

    let token = &content[start..end];
    if token.is_empty() {
        return None;
    }

    Some(SymbolAtCursor {
        search_text: token.to_string(),
    })
}

/// Find all occurrences of a symbol text in the content.
fn find_all_occurrences(
    content: &str,
    symbol: &SymbolAtCursor,
    mapper: &PositionMapper,
    doc_uri: &Url,
) -> Vec<Location> {
    let search = &symbol.search_text;
    let mut locations = Vec::new();
    let mut pos = 0;

    let uri = doc_uri.clone();

    while let Some(found) = content[pos..].find(search.as_str()) {
        let abs_start = pos + found;
        let abs_end = abs_start + search.len();

        let before_ok = abs_start == 0 || {
            let b = content.as_bytes()[abs_start - 1];
            !b.is_ascii_alphanumeric() && b != b'_' && b != b'-'
        };
        let after_ok = abs_end >= content.len() || {
            let b = content.as_bytes()[abs_end];
            !b.is_ascii_alphanumeric() && b != b'_' && b != b'-'
        };

        if before_ok && after_ok {
            let start_pos = mapper.position_from_offset(abs_start);
            let end_pos = mapper.position_from_offset(abs_end);
            locations.push(Location {
                uri: uri.clone(),
                range: tower_lsp::lsp_types::Range {
                    start: start_pos,
                    end: end_pos,
                },
            });
        }

        pos = abs_start + 1;
    }

    locations
}

/// Find all references given a file path and offset
pub async fn find_references_at(
    workspace: &WorkspaceManager,
    file_path: &std::path::Path,
    offset: usize,
    include_declaration: bool,
) -> Vec<Location> {
    // Find symbol at position
    let symbol = match workspace.symbol_at(file_path, offset) {
        Some(s) => s,
        None => return vec![],
    };

    // Get all references
    let refs = workspace.find_references(&symbol);

    // Convert to LSP Locations
    refs.iter()
        .filter(|r| include_declaration || r.kind != ReferenceKind::Definition)
        .filter_map(|r| {
            let uri = Url::from_file_path(&r.file).ok()?;

            // We need to read the file to convert span to range
            let content = std::fs::read_to_string(&r.file).ok()?;
            let mapper = PositionMapper::new(&content);
            let start = mapper.position_from_offset(r.span.start);
            let end = mapper.position_from_offset(r.span.end);

            Some(Location {
                uri,
                range: tower_lsp::lsp_types::Range { start, end },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_provide_references_empty() {
        // This is a basic smoke test - full testing would require
        // setting up a workspace with indexed files
    }
}
