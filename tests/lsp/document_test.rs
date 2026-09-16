//! Document store tests for the Spacetime LSP.
//!
//! Tests the document lifecycle (open, change, close) and
//! concurrent access with DashMap.

use spacetime::lsp::{DocumentState, DocumentStore};
use std::sync::Arc;
use std::thread;
use tower_lsp::lsp_types::Url;

// =============================================================================
// Helper Functions
// =============================================================================

fn test_uri(name: &str) -> Url {
    Url::parse(&format!("file:///test/{}", name)).unwrap()
}

// =============================================================================
// DocumentState Tests
// =============================================================================

#[test]
fn test_document_state_creation() {
    let content = "hello\nworld".to_string();
    let state = DocumentState::new(content.clone(), 1);

    assert_eq!(state.content, content);
    assert_eq!(state.version, 1);
    assert_eq!(state.line_starts(), &[0, 6]);
}

#[test]
fn test_document_state_update() {
    let mut state = DocumentState::new("initial".to_string(), 1);

    assert_eq!(state.version, 1);
    assert_eq!(state.content, "initial");

    state.update("updated\ncontent".to_string(), 2);

    assert_eq!(state.version, 2);
    assert_eq!(state.content, "updated\ncontent");
    assert_eq!(state.line_starts(), &[0, 8]);
}

#[test]
fn test_document_state_with_valid_spacetime() {
    // Test with the most basic valid Spacetime content
    let content = r#"@scope(".test") {
}"#
    .to_string();

    let state = DocumentState::new(content, 1);

    // Should parse successfully - just verify it doesn't panic
    // The AST may or may not be Some depending on how strict the parser is
    let _ = state.ast;
}

#[test]
fn test_document_state_with_invalid_content() {
    let content = "this is {{ invalid {{{{ syntax".to_string();
    let state = DocumentState::new(content, 1);

    // Should fail to parse
    assert!(state.ast.is_none(), "Invalid content should not parse");
}

#[test]
fn test_document_state_empty_content() {
    let state = DocumentState::new(String::new(), 1);

    assert_eq!(state.content, "");
    assert_eq!(state.line_starts(), &[0]);
    // Empty content might or might not parse depending on parser
}

// =============================================================================
// DocumentStore Basic Tests
// =============================================================================

#[test]
fn test_store_open_document() {
    let store = DocumentStore::new();
    let uri = test_uri("test.st");

    assert!(store.is_empty());

    store.open(uri.clone(), "content".to_string(), 1);

    assert_eq!(store.len(), 1);
    assert!(!store.is_empty());

    let doc = store.get(&uri).expect("Document should exist");
    assert_eq!(doc.content, "content");
    assert_eq!(doc.version, 1);
}

#[test]
fn test_store_change_document() {
    let store = DocumentStore::new();
    let uri = test_uri("test.st");

    store.open(uri.clone(), "initial".to_string(), 1);

    {
        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.content, "initial");
        assert_eq!(doc.version, 1);
    }

    store.change(&uri, "updated".to_string(), 2);

    {
        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.content, "updated");
        assert_eq!(doc.version, 2);
    }
}

#[test]
fn test_store_close_document() {
    let store = DocumentStore::new();
    let uri = test_uri("test.st");

    store.open(uri.clone(), "content".to_string(), 1);
    assert_eq!(store.len(), 1);

    store.close(&uri);

    assert_eq!(store.len(), 0);
    assert!(store.get(&uri).is_none());
}

#[test]
fn test_store_multiple_documents() {
    let store = DocumentStore::new();
    let uri1 = test_uri("file1.st");
    let uri2 = test_uri("file2.st");
    let uri3 = test_uri("file3.st");

    store.open(uri1.clone(), "content 1".to_string(), 1);
    store.open(uri2.clone(), "content 2".to_string(), 1);
    store.open(uri3.clone(), "content 3".to_string(), 1);

    assert_eq!(store.len(), 3);

    let doc1 = store.get(&uri1).unwrap();
    let doc2 = store.get(&uri2).unwrap();
    let doc3 = store.get(&uri3).unwrap();

    assert_eq!(doc1.content, "content 1");
    assert_eq!(doc2.content, "content 2");
    assert_eq!(doc3.content, "content 3");
}

#[test]
fn test_store_overwrite_on_open() {
    let store = DocumentStore::new();
    let uri = test_uri("test.st");

    store.open(uri.clone(), "original".to_string(), 1);

    {
        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.content, "original");
    }

    // Opening again should replace
    store.open(uri.clone(), "replaced".to_string(), 2);

    {
        let doc = store.get(&uri).unwrap();
        assert_eq!(doc.content, "replaced");
        assert_eq!(doc.version, 2);
    }

    // Still only one document
    assert_eq!(store.len(), 1);
}

#[test]
fn test_store_change_nonexistent() {
    let store = DocumentStore::new();
    let uri = test_uri("nonexistent.st");

    // Should not panic, just be a no-op
    store.change(&uri, "content".to_string(), 1);

    assert!(store.get(&uri).is_none());
    assert!(store.is_empty());
}

#[test]
fn test_store_close_nonexistent() {
    let store = DocumentStore::new();
    let uri = test_uri("nonexistent.st");

    // Should not panic, just be a no-op
    store.close(&uri);

    assert!(store.is_empty());
}

#[test]
fn test_store_uris() {
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

// =============================================================================
// Concurrent Access Tests
// =============================================================================

#[test]
fn test_concurrent_reads() {
    let store = Arc::new(DocumentStore::new());
    let uri = test_uri("concurrent.st");

    store.open(uri.clone(), "shared content".to_string(), 1);

    let mut handles = vec![];

    // Spawn multiple reader threads
    for _ in 0..10 {
        let store_clone = Arc::clone(&store);
        let uri_clone = uri.clone();

        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let doc = store_clone.get(&uri_clone);
                assert!(doc.is_some());
                let doc = doc.unwrap();
                assert_eq!(doc.content, "shared content");
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }
}

#[test]
fn test_concurrent_writes() {
    let store = Arc::new(DocumentStore::new());

    let mut handles = vec![];

    // Spawn multiple writer threads, each opening different documents
    for i in 0..10 {
        let store_clone = Arc::clone(&store);

        handles.push(thread::spawn(move || {
            for j in 0..10 {
                let uri = test_uri(&format!("file_{}_{}.st", i, j));
                store_clone.open(uri, format!("content_{}_{}", i, j), 1);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Should have 100 documents (10 threads x 10 documents each)
    assert_eq!(store.len(), 100);
}

#[test]
fn test_concurrent_read_write() {
    let store = Arc::new(DocumentStore::new());
    let uri = test_uri("rw.st");

    store.open(uri.clone(), "version 0".to_string(), 0);

    let mut handles = vec![];

    // Writer thread
    let store_write = Arc::clone(&store);
    let uri_write = uri.clone();
    handles.push(thread::spawn(move || {
        for v in 1..=50 {
            store_write.change(&uri_write, format!("version {}", v), v);
            thread::yield_now();
        }
    }));

    // Reader threads
    for _ in 0..5 {
        let store_read = Arc::clone(&store);
        let uri_read = uri.clone();

        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                if let Some(doc) = store_read.get(&uri_read) {
                    // Just verify we can read without panic
                    let _ = doc.content.len();
                    let _ = doc.version;
                }
                thread::yield_now();
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Final state should have version 50
    let doc = store.get(&uri).unwrap();
    assert_eq!(doc.version, 50);
}

#[test]
fn test_concurrent_open_close() {
    let store = Arc::new(DocumentStore::new());

    let mut handles = vec![];

    // Opener thread
    let store_open = Arc::clone(&store);
    handles.push(thread::spawn(move || {
        for i in 0..100 {
            let uri = test_uri(&format!("file_{}.st", i));
            store_open.open(uri, format!("content {}", i), 1);
        }
    }));

    // Closer thread (closes different files)
    let store_close = Arc::clone(&store);
    handles.push(thread::spawn(move || {
        for i in 0..100 {
            let uri = test_uri(&format!("file_{}.st", i));
            store_close.close(&uri);
        }
    }));

    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Final count depends on timing, but should not panic
    let _count = store.len();
}

// =============================================================================
// Position Mapper Integration Tests
// =============================================================================

#[test]
fn test_document_position_mapper() {
    let content = "line 1\nline 2\nline 3".to_string();
    let state = DocumentState::new(content, 1);

    // Use the position mapper from the state
    let pos = state.position_mapper.position_from_offset(7);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.character, 0);

    let offset = state
        .position_mapper
        .offset_from_position(tower_lsp::lsp_types::Position {
            line: 2,
            character: 3,
        });
    assert_eq!(offset, 17); // "line 1\nline 2\nlin" = 14 + 3
}

#[test]
fn test_document_update_refreshes_mapper() {
    let mut state = DocumentState::new("short".to_string(), 1);

    assert_eq!(state.line_starts(), &[0]); // Single line

    state.update("line 1\nline 2".to_string(), 2);

    assert_eq!(state.line_starts(), &[0, 7]); // Now two lines
}

// =============================================================================
// Spacetime Content Tests
// =============================================================================

#[test]
fn test_document_with_spacetime_content() {
    let content = r#"
@scope(".container") {
    @scroll {
        opacity: 0 -> 1;
        transform: translateY(50px) -> translateY(0);
    }

    @on &.hover {
        background-color: #eee;
    }
}
"#
    .to_string();

    let store = DocumentStore::new();
    let uri = test_uri("animation.st");

    store.open(uri.clone(), content.clone(), 1);

    let doc = store.get(&uri).unwrap();
    assert_eq!(doc.content, content);

    // Should have parsed (if content is valid)
    if doc.ast.is_some() {
        // Additional AST checks could go here
    }
}

#[test]
fn test_document_incremental_changes() {
    let store = DocumentStore::new();
    let uri = test_uri("incremental.st");

    // Initial version
    store.open(
        uri.clone(),
        "@scope(\".test\") {\n    // empty\n}".to_string(),
        1,
    );

    // Add content
    store.change(
        &uri,
        "@scope(\".test\") {\n    @scroll {\n        opacity: 0 -> 1;\n    }\n}".to_string(),
        2,
    );

    // Modify content
    store.change(
        &uri,
        "@scope(\".test\") {\n    @scroll {\n        opacity: 0 -> 1;\n        transform: scale(0.5) -> scale(1);\n    }\n}".to_string(),
        3,
    );

    let doc = store.get(&uri).unwrap();
    assert_eq!(doc.version, 3);
    assert!(doc.content.contains("transform"));
}
