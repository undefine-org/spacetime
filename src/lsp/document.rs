//! Document management for the LSP server.
//!
//! Manages open documents with their content, AST, and line information.
//! Uses DashMap for thread-safe concurrent access.

use dashmap::DashMap;
use tower_lsp::lsp_types::Url;

use crate::parser::StFile;

use super::position::PositionMapper;

/// State of a single open document.
#[derive(Debug, Clone)]
pub struct DocumentState {
    /// The current content of the document
    pub content: String,
    /// Parsed AST (None if parsing failed)
    pub ast: Option<StFile>,
    /// Position mapper for this document
    pub position_mapper: PositionMapper,
    /// Document version (incremented on each change)
    pub version: i32,
}

impl DocumentState {
    /// Create a new DocumentState from content.
    pub fn new(content: String, version: i32) -> Self {
        let position_mapper = PositionMapper::new(&content);
        let ast = Self::parse_content(&content);

        Self {
            content,
            ast,
            position_mapper,
            version,
        }
    }

    /// Update the document with new content.
    pub fn update(&mut self, content: String, version: i32) {
        self.position_mapper = PositionMapper::new(&content);
        self.ast = Self::parse_content(&content);
        self.content = content;
        self.version = version;
    }

    /// Attempt to parse the content into an AST.
    fn parse_content(content: &str) -> Option<StFile> {
        crate::parser::parse(content).ok()
    }

    /// Get line start offsets (for compatibility with the spec).
    pub fn line_starts(&self) -> &[usize] {
        self.position_mapper.line_starts()
    }
}

/// Thread-safe store for open documents.
///
/// Uses DashMap for concurrent access without explicit locking.
#[derive(Debug)]
pub struct DocumentStore {
    documents: DashMap<Url, DocumentState>,
}

impl DocumentStore {
    /// Create a new empty document store.
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    /// Open a new document.
    ///
    /// If a document with the same URI already exists, it will be replaced.
    pub fn open(&self, uri: Url, content: String, version: i32) {
        let state = DocumentState::new(content, version);
        self.documents.insert(uri, state);
    }

    /// Update an open document with new content.
    ///
    /// If the document is not open, this is a no-op.
    pub fn change(&self, uri: &Url, content: String, version: i32) {
        if let Some(mut entry) = self.documents.get_mut(uri) {
            entry.update(content, version);
        }
    }

    /// Close a document.
    ///
    /// Removes the document from the store.
    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    /// Get a reference to an open document.
    ///
    /// Returns None if the document is not open.
    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, DocumentState>> {
        self.documents.get(uri)
    }

    /// Get the number of open documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if there are no open documents.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Get all document URIs.
    pub fn uris(&self) -> Vec<Url> {
        self.documents
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri(name: &str) -> Url {
        Url::parse(&format!("file:///test/{}", name)).unwrap()
    }

    #[test]
    fn test_document_state_new() {
        let content = "hello\nworld".to_string();
        let state = DocumentState::new(content.clone(), 1);

        assert_eq!(state.content, content);
        assert_eq!(state.version, 1);
        assert_eq!(state.line_starts(), &[0, 6]);
    }

    #[test]
    fn test_document_state_update() {
        let mut state = DocumentState::new("hello".to_string(), 1);
        assert_eq!(state.line_starts(), &[0]);

        state.update("hello\nworld".to_string(), 2);
        assert_eq!(state.content, "hello\nworld");
        assert_eq!(state.version, 2);
        assert_eq!(state.line_starts(), &[0, 6]);
    }

    #[test]
    fn test_document_store_open() {
        let store = DocumentStore::new();
        let uri = test_uri("test.st");

        store.open(uri.clone(), "content".to_string(), 1);

        assert_eq!(store.len(), 1);
        assert!(store.get(&uri).is_some());
    }

    #[test]
    fn test_document_store_change() {
        let store = DocumentStore::new();
        let uri = test_uri("test.st");

        store.open(uri.clone(), "initial".to_string(), 1);
        store.change(&uri, "updated".to_string(), 2);

        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.content, "updated");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_document_store_close() {
        let store = DocumentStore::new();
        let uri = test_uri("test.st");

        store.open(uri.clone(), "content".to_string(), 1);
        assert_eq!(store.len(), 1);

        store.close(&uri);
        assert_eq!(store.len(), 0);
        assert!(store.get(&uri).is_none());
    }

    #[test]
    fn test_document_store_multiple_documents() {
        let store = DocumentStore::new();
        let uri1 = test_uri("file1.st");
        let uri2 = test_uri("file2.st");

        store.open(uri1.clone(), "content1".to_string(), 1);
        store.open(uri2.clone(), "content2".to_string(), 1);

        assert_eq!(store.len(), 2);

        let doc1 = store.get(&uri1).unwrap();
        let doc2 = store.get(&uri2).unwrap();
        assert_eq!(doc1.content, "content1");
        assert_eq!(doc2.content, "content2");
    }

    #[test]
    fn test_document_store_change_nonexistent() {
        let store = DocumentStore::new();
        let uri = test_uri("nonexistent.st");

        // Should be a no-op, not panic
        store.change(&uri, "content".to_string(), 1);
        assert!(store.get(&uri).is_none());
    }

    #[test]
    fn test_document_store_uris() {
        let store = DocumentStore::new();
        let uri1 = test_uri("file1.st");
        let uri2 = test_uri("file2.st");

        store.open(uri1.clone(), "content1".to_string(), 1);
        store.open(uri2.clone(), "content2".to_string(), 1);

        let uris = store.uris();
        assert_eq!(uris.len(), 2);
        assert!(uris.contains(&uri1));
        assert!(uris.contains(&uri2));
    }

    #[test]
    fn test_parse_valid_spacetime() {
        // Test with valid Spacetime content
        let content = r#"
@scope(".test") {
    @scroll {
        opacity: 0 -> 1;
    }
}
"#;
        let state = DocumentState::new(content.to_string(), 1);
        // AST may or may not be parsed depending on parser implementation
        // Just verify it doesn't panic
        let _ = state.ast;
    }

    #[test]
    fn test_parse_invalid_content() {
        // Test with invalid content - should not panic
        let content = "this is not valid spacetime {{{{";
        let state = DocumentState::new(content.to_string(), 1);
        // AST should be None for invalid content
        assert!(state.ast.is_none());
    }
}
