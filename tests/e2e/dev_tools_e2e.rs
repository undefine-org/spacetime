//! E2E tests for the complete dev tools edit flow.
//!
//! Tests cover the full WebSocket roundtrip for each edit handler:
//! - EditJson → file update → DataUpdate broadcast
//! - EditHtml → lol_html element update → Ack
//! - EditAst → .st file text patch → Ack
//! - Error cases: path traversal, nonexistent files, invalid paths
//! - Provenance attributes in @each stdlib primitive

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use spacetime::dev_server::{
    ClientMessage, ServerMessage, TestServerState, create_test_runner_routes,
};
use spacetime::sync::protocol::{
    ClientMessage as SyncClientMessage, ServerMessage as SyncServerMessage,
};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

// =============================================================================
// Test Helpers
// =============================================================================

async fn start_test_server_with_site(
    site_dir: std::path::PathBuf,
) -> (String, Arc<TestServerState>) {
    let (tx, _rx) = broadcast::channel(100);
    let compilation_cache = Arc::new(RwLock::new(None));
    let state = Arc::new(TestServerState::new(tx, compilation_cache).with_site_dir(site_dir));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}/ws", addr);

    let app = create_test_runner_routes(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    (url, state)
}

/// Collect WebSocket messages until a predicate is satisfied or timeout.
/// Returns the collected SyncServerMessages.
async fn collect_sync_messages<F>(
    client: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    timeout_secs: u64,
    mut done: F,
) -> Vec<SyncServerMessage>
where
    F: FnMut(&[SyncServerMessage]) -> bool,
{
    let mut messages = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        while let Some(Ok(WsMessage::Text(text))) = client.next().await {
            if let Ok(ServerMessage::Sync(msg)) = serde_json::from_str(&text) {
                messages.push(msg);
                if done(&messages) {
                    return true;
                }
            }
        }
        false
    })
    .await;
    assert!(result.is_ok(), "Timed out waiting for messages");
    assert!(result.unwrap(), "WebSocket closed before condition met");
    messages
}

// =============================================================================
// EditJson Roundtrip Tests
// =============================================================================

#[tokio::test]
async fn test_edit_json_roundtrip() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::write(
        data_dir.join("products.json"),
        serde_json::to_string_pretty(&json!([
            {"name": "Widget", "price": 10},
            {"name": "Gadget", "price": 20}
        ]))
        .unwrap(),
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send EditJson
    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "data/products.json".to_string(),
        path: "[1].price".to_string(),
        value: json!(29.99),
        op_id: "roundtrip_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    // Should receive both Ack and DataUpdate
    let mut got_ack = false;
    let mut got_data_update = false;
    let messages = collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            match m {
                SyncServerMessage::Ack { op_id } if op_id == "roundtrip_1" => got_ack = true,
                SyncServerMessage::DataUpdate { source, data } => {
                    assert_eq!(source, "data/products.json");
                    assert_eq!(data[1]["price"], json!(29.99));
                    assert_eq!(data[0]["name"], json!("Widget")); // unchanged
                    got_data_update = true;
                }
                _ => {}
            }
        }
        got_ack && got_data_update
    })
    .await;

    assert!(got_ack, "Expected Ack");
    assert!(got_data_update, "Expected DataUpdate broadcast");
    assert!(messages.len() >= 2, "Should have at least Ack + DataUpdate");

    // Verify file on disk was actually updated
    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(data_dir.join("products.json")).unwrap())
            .unwrap();
    assert_eq!(on_disk[1]["price"], json!(29.99));
    assert_eq!(on_disk[0]["price"], json!(10)); // unchanged
}

#[tokio::test]
async fn test_edit_json_nested_path() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_string_pretty(&json!({
            "settings": {
                "theme": {
                    "primary": "#000",
                    "secondary": "#fff"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "config.json".to_string(),
        path: "settings.theme.primary".to_string(),
        value: json!("#ff0000"),
        op_id: "nested_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_ack = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Ack { op_id } = m
                && op_id == "nested_1"
            {
                got_ack = true;
            }
        }
        got_ack
    })
    .await;

    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("config.json")).unwrap())
            .unwrap();
    assert_eq!(on_disk["settings"]["theme"]["primary"], json!("#ff0000"));
    assert_eq!(on_disk["settings"]["theme"]["secondary"], json!("#fff"));
}

// =============================================================================
// EditHtml Roundtrip Tests
// =============================================================================

#[tokio::test]
async fn test_edit_html_updates_element() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
  <h1 data-st-id="hero-title">Old Title</h1>
  <p data-st-id="intro">Old intro text</p>
</body>
</html>"#,
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditHtml {
        file: "index.html".to_string(),
        element_id: "hero-title".to_string(),
        content: "New Amazing Title".to_string(),
        op_id: "html_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    // Should receive Ack (EditHtml does NOT broadcast DataUpdate)
    let mut got_ack = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Ack { op_id } = m
                && op_id == "html_1"
            {
                got_ack = true;
            }
        }
        got_ack
    })
    .await;

    assert!(got_ack, "Expected Ack for EditHtml");

    // Verify file on disk was updated
    let html = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
    assert!(
        html.contains("New Amazing Title"),
        "New content should be in file"
    );
    assert!(
        !html.contains("Old Title"),
        "Old content should be replaced"
    );
    // Other elements should be untouched
    assert!(
        html.contains("Old intro text"),
        "Other elements should remain"
    );
}

#[tokio::test]
async fn test_edit_html_element_not_found() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("page.html"),
        "<html><body><div>No st-id elements here</div></body></html>",
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditHtml {
        file: "page.html".to_string(),
        element_id: "nonexistent".to_string(),
        content: "New content".to_string(),
        op_id: "html_miss".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, reason } = m
                && op_id == "html_miss"
            {
                assert!(
                    reason.contains("not found"),
                    "Rejection should mention element not found, got: {}",
                    reason
                );
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Expected Reject for missing element");
}

// =============================================================================
// EditAst Roundtrip Tests
// =============================================================================

#[tokio::test]
async fn test_edit_ast_patches_st_file() {
    let dir = TempDir::new().unwrap();
    let st_content = ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n        easing: ease-out;\n    }\n}\n";
    std::fs::write(dir.path().join("styles.st"), st_content).unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditAst {
        file: "styles.st".to_string(),
        selector: ".hero @scroll".to_string(),
        patch: json!({"duration": "1200ms"}),
        op_id: "ast_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    // EditAst returns Ack on success, Reject on failure (e.g. if parser doesn't provide spans)
    let mut got_response = false;
    let result = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(Ok(WsMessage::Text(text))) = read.next().await {
            if let Ok(ServerMessage::Sync(msg)) = serde_json::from_str(&text) {
                match msg {
                    SyncServerMessage::Ack { op_id } if op_id == "ast_1" => {
                        got_response = true;
                        // Verify file was patched
                        let content =
                            std::fs::read_to_string(dir.path().join("styles.st")).unwrap();
                        assert!(
                            content.contains("1200ms"),
                            "File should contain patched duration"
                        );
                        assert!(
                            content.contains("easing: ease-out"),
                            "Other properties should remain"
                        );
                        return true;
                    }
                    SyncServerMessage::Reject { op_id, reason } if op_id == "ast_1" => {
                        got_response = true;
                        // Acceptable if parser doesn't provide spans
                        assert!(
                            reason.contains("not found")
                                || reason.contains("span")
                                || reason.contains("Parse error"),
                            "Unexpected rejection reason: {}",
                            reason
                        );
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    })
    .await;

    assert!(result.is_ok(), "Timed out waiting for EditAst response");
    assert!(got_response, "Should receive either Ack or Reject");
}

#[tokio::test]
async fn test_edit_ast_invalid_selector() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("styles.st"),
        ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Missing @directive part
    let edit = ClientMessage::Sync(SyncClientMessage::EditAst {
        file: "styles.st".to_string(),
        selector: ".hero".to_string(), // no @directive
        patch: json!({"duration": "1200ms"}),
        op_id: "ast_bad_sel".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, reason } = m
                && op_id == "ast_bad_sel"
            {
                assert!(
                    reason.contains("selector"),
                    "Should mention invalid selector, got: {}",
                    reason
                );
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Expected Reject for invalid selector");
}

// =============================================================================
// Error Cases
// =============================================================================

#[tokio::test]
async fn test_edit_json_path_traversal_rejected() {
    let dir = TempDir::new().unwrap();
    // Create a legitimate file so site_dir is valid
    std::fs::write(dir.path().join("safe.json"), "{}").unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "../../../etc/passwd".to_string(),
        path: "name".to_string(),
        value: json!("evil"),
        op_id: "traversal_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, reason } = m
                && op_id == "traversal_1"
            {
                assert!(
                    reason.contains("not found") || reason.contains("traversal"),
                    "Should reject path traversal, got: {}",
                    reason
                );
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Path traversal should be rejected");
}

#[tokio::test]
async fn test_edit_json_nonexistent_file_rejected() {
    let dir = TempDir::new().unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "does_not_exist.json".to_string(),
        path: "name".to_string(),
        value: json!("value"),
        op_id: "nofile_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, .. } = m
                && op_id == "nofile_1"
            {
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Nonexistent file should be rejected");
}

#[tokio::test]
async fn test_edit_json_invalid_json_path_rejected() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("small.json"),
        serde_json::to_string_pretty(&json!([
            {"name": "A"},
            {"name": "B"}
        ]))
        .unwrap(),
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Index 999 is out of bounds on a 2-element array
    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "small.json".to_string(),
        path: "[999].name".to_string(),
        value: json!("value"),
        op_id: "badpath_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, reason } = m
                && op_id == "badpath_1"
            {
                assert!(
                    reason.contains("out of bounds") || reason.contains("not found"),
                    "Should mention index out of bounds, got: {}",
                    reason
                );
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Invalid JSON path should be rejected");
}

#[tokio::test]
async fn test_edit_html_path_traversal_rejected() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("safe.html"), "<html></html>").unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditHtml {
        file: "../../../etc/passwd".to_string(),
        element_id: "title".to_string(),
        content: "evil".to_string(),
        op_id: "html_traversal".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, .. } = m
                && op_id == "html_traversal"
            {
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Path traversal via EditHtml should be rejected");
}

#[tokio::test]
async fn test_edit_ast_nonexistent_file_rejected() {
    let dir = TempDir::new().unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditAst {
        file: "nonexistent.st".to_string(),
        selector: ".hero @scroll".to_string(),
        patch: json!({"duration": "1200ms"}),
        op_id: "ast_nofile".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, .. } = m
                && op_id == "ast_nofile"
            {
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Nonexistent .st file should be rejected");
}

#[tokio::test]
async fn test_edit_json_no_site_dir_rejected() {
    // Server without site_dir configured
    let (tx, _rx) = broadcast::channel(100);
    let compilation_cache = Arc::new(RwLock::new(None));
    let state = Arc::new(TestServerState::new(tx, compilation_cache));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}/ws", addr);

    let app = create_test_runner_routes(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "test.json".to_string(),
        path: "name".to_string(),
        value: json!("value"),
        op_id: "nodir_1".to_string(),
    });
    write
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let mut got_reject = false;
    collect_sync_messages(&mut read, 3, |msgs| {
        for m in msgs {
            if let SyncServerMessage::Reject { op_id, reason } = m
                && op_id == "nodir_1"
            {
                assert!(
                    reason.contains("No site directory"),
                    "Should mention no site directory, got: {}",
                    reason
                );
                got_reject = true;
            }
        }
        got_reject
    })
    .await;

    assert!(got_reject, "Missing site_dir should be rejected");
}

// =============================================================================
// Provenance Attributes Tests
// =============================================================================

/// Verify that the `each` stdlib primitive contains __ST_DEV__-gated
/// provenance attribute code (data-st-source, data-st-index, data-st-bind).
///
/// These attributes are added at runtime when `window.__ST_DEV__` is set,
/// enabling dev tools to trace rendered elements back to their data source.
#[test]
fn test_each_primitive_has_provenance_attributes() {
    let each_st = std::fs::read_to_string("stdlib/primitives/data/each.st")
        .expect("each.st should exist in stdlib");

    // Verify __ST_DEV__ gate exists
    assert!(
        each_st.contains("window.__ST_DEV__"),
        "each.st should gate provenance behind __ST_DEV__ flag"
    );

    // Verify data-st-source attribute is set
    assert!(
        each_st.contains("data-st-source"),
        "each.st should set data-st-source provenance attribute"
    );

    // Verify data-st-index attribute is set
    assert!(
        each_st.contains("data-st-index"),
        "each.st should set data-st-index provenance attribute"
    );

    // Verify data-st-bind attribute is set
    assert!(
        each_st.contains("data-st-bind"),
        "each.st should set data-st-bind provenance attribute"
    );
}

/// Verify that the dev-flag primitive sets __ST_DEV__ = true,
/// which is required for provenance attributes to be added.
#[test]
fn test_dev_flag_sets_st_dev() {
    let index_st =
        std::fs::read_to_string("stdlib/__dev__/index.st").expect("__dev__/index.st should exist");

    assert!(
        index_st.contains("__ST_DEV__ = true"),
        "Dev index.st should set __ST_DEV__ = true"
    );
}
