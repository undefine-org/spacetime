//! E2E tests for dev edit protocol via WebSocket
//!
//! These tests verify the live editing workflow:
//! - EditJson messages trigger file updates and DataUpdate broadcasts
//! - Ack/Reject responses are sent to the originating client
//! - Unimplemented handlers (EditHtml, EditAst) return Reject

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

#[tokio::test]
async fn test_edit_json_ack_and_data_update() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::write(
        data_dir.join("products.json"),
        serde_json::to_string_pretty(&json!([
            {"name": "Product A", "price": 10},
            {"name": "Product B", "price": 20}
        ]))
        .unwrap(),
    )
    .unwrap();

    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;

    let (mut client, _) = connect_async(&url).await.unwrap();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "data/products.json".to_string(),
        path: "[1].price".to_string(),
        value: json!(29.99),
        op_id: "op_1".to_string(),
    });
    client
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    // Should receive Ack
    let mut got_ack = false;
    let mut got_data_update = false;

    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(WsMessage::Text(text))) = client.next().await {
            if let Ok(ServerMessage::Sync(msg)) = serde_json::from_str(&text) {
                match msg {
                    SyncServerMessage::Ack { op_id } => {
                        assert_eq!(op_id, "op_1");
                        got_ack = true;
                    }
                    SyncServerMessage::DataUpdate { source, data } => {
                        assert_eq!(source, "data/products.json");
                        assert_eq!(data[1]["price"], json!(29.99));
                        got_data_update = true;
                    }
                    _ => {}
                }
            }
            if got_ack && got_data_update {
                return true;
            }
        }
        false
    })
    .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_edit_json_reject_missing_file() {
    let dir = TempDir::new().unwrap();
    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;

    let (mut client, _) = connect_async(&url).await.unwrap();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "nonexistent.json".to_string(),
        path: "name".to_string(),
        value: json!("value"),
        op_id: "op_fail".to_string(),
    });
    client
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(WsMessage::Text(text))) = client.next().await {
            if let Ok(ServerMessage::Sync(SyncServerMessage::Reject { op_id, .. })) =
                serde_json::from_str(&text)
            {
                assert_eq!(op_id, "op_fail");
                return true;
            }
        }
        false
    })
    .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_edit_html_file_not_found_rejected() {
    let dir = TempDir::new().unwrap();
    let (url, _state) = start_test_server_with_site(dir.path().to_path_buf()).await;

    let (mut client, _) = connect_async(&url).await.unwrap();

    let edit = ClientMessage::Sync(SyncClientMessage::EditHtml {
        file: "index.html".to_string(),
        element_id: "title".to_string(),
        content: "New".to_string(),
        op_id: "op_html".to_string(),
    });
    client
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(WsMessage::Text(text))) = client.next().await {
            if let Ok(ServerMessage::Sync(SyncServerMessage::Reject { op_id, reason })) =
                serde_json::from_str(&text)
            {
                assert_eq!(op_id, "op_html");
                assert!(
                    reason.contains("not found"),
                    "Expected 'not found' in reason, got: {}",
                    reason
                );
                return true;
            }
        }
        false
    })
    .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
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

    let (mut client, _) = connect_async(&url).await.unwrap();

    let edit = ClientMessage::Sync(SyncClientMessage::EditJson {
        file: "test.json".to_string(),
        path: "name".to_string(),
        value: json!("value"),
        op_id: "op_nodir".to_string(),
    });
    client
        .send(WsMessage::Text(serde_json::to_string(&edit).unwrap()))
        .await
        .unwrap();

    let result = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(WsMessage::Text(text))) = client.next().await {
            if let Ok(ServerMessage::Sync(SyncServerMessage::Reject { op_id, reason })) =
                serde_json::from_str(&text)
            {
                assert_eq!(op_id, "op_nodir");
                assert!(reason.contains("No site directory"));
                return true;
            }
        }
        false
    })
    .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}
