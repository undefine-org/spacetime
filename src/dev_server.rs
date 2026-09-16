//! WebSocket-enabled dev server for live test runner
//!
//! Provides real-time file change notifications and bidirectional
//! communication with browser clients for interactive debugging.
//! Also supports the dev edit protocol for live content editing
//! via the `src/sync` module.

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::sync::{self, ClientMessage as SyncClientMessage, ServerMessage as SyncServerMessage};
use crate::watcher::FileChange;

// =============================================================================
// Message Types
// =============================================================================

/// Messages sent from server to browser client
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServerMessage {
    /// Dev edit protocol messages (DataUpdate, Ack, Reject)
    Sync(SyncServerMessage),
    /// Dev server messages (reload, test results, etc.)
    DevServer(DevServerMessage),
}

/// Dev server specific messages
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DevServerMessage {
    Reload { files: Vec<String> },
    TestResults { results: String },
    Error { message: String },
    Ping,
}

/// Messages received from browser client
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClientMessage {
    /// Dev edit protocol messages (EditJson, EditHtml, EditAst)
    Sync(SyncClientMessage),
    /// Dev server messages (pause, resume, step, etc.)
    DevServer(DevServerClientMessage),
}

/// Dev server specific client messages
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DevServerClientMessage {
    Pause,
    Resume,
    Step { direction: String },
    Pong,
}

/// Is `ty` a discriminator the dev edit/control protocol owns? Used to decide
/// whether a frame that failed to parse as a [`ClientMessage`] is a genuine
/// protocol error (known type, malformed body) versus a page's own runtime
/// traffic (signal/presence frames) the edit socket simply isn't for. Mirrors
/// the `#[serde(tag = "type")]` variants of `SyncClientMessage` +
/// `DevServerClientMessage`.
fn is_edit_protocol_type(ty: &str) -> bool {
    matches!(
        ty,
        // SyncClientMessage (edit protocol)
        "EditJson"
            | "EditHtml"
            | "EditAst"
            | "EditJsonArray"
            | "EditToken"
            | "InspectElement"
            // DevServerClientMessage (control)
            | "Pause"
            | "Resume"
            | "Step"
            | "Pong"
    )
}

// =============================================================================
// Server State
// =============================================================================

#[derive(Clone)]
pub struct CompilationResult {
    pub css: String,
    pub js: String,
    pub error: Option<String>,
    pub timestamp: Instant,
}

pub struct ClientRegistry {
    clients: HashMap<String, mpsc::UnboundedSender<SyncServerMessage>>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn register(&mut self, client_id: String) -> mpsc::UnboundedReceiver<SyncServerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.clients.insert(client_id, tx);
        rx
    }

    pub fn unregister(&mut self, client_id: &str) {
        self.clients.remove(client_id);
    }

    pub fn send_to(&self, client_id: &str, msg: SyncServerMessage) -> bool {
        if let Some(tx) = self.clients.get(client_id) {
            tx.send(msg).is_ok()
        } else {
            false
        }
    }

    pub fn broadcast_all(&self, msg: SyncServerMessage) {
        for tx in self.clients.values() {
            let _ = tx.send(msg.clone());
        }
    }
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct TestServerState {
    pub file_changes_tx: broadcast::Sender<FileChange>,
    pub compilation_cache: Arc<RwLock<Option<CompilationResult>>>,
    /// Site directory for dev edit operations (path validation)
    pub site_dir: Option<PathBuf>,
    pub client_registry: Arc<RwLock<ClientRegistry>>,
    /// File watcher exemption mechanism: tracks files modified by EditAst to suppress reload
    /// Maps file path to the Instant it was written. Entries older than 500ms are auto-cleared.
    pub suppress_reload: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

impl TestServerState {
    pub fn new(
        file_changes_tx: broadcast::Sender<FileChange>,
        compilation_cache: Arc<RwLock<Option<CompilationResult>>>,
    ) -> Self {
        Self {
            file_changes_tx,
            compilation_cache,
            site_dir: None,
            client_registry: Arc::new(RwLock::new(ClientRegistry::new())),
            suppress_reload: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_site_dir(mut self, site_dir: PathBuf) -> Self {
        self.site_dir = Some(site_dir);
        self
    }
}

// =============================================================================
// Router Setup
// =============================================================================

pub fn create_test_runner_routes(state: Arc<TestServerState>) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(state)
}

// =============================================================================
// WebSocket Handler
// =============================================================================

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<TestServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<TestServerState>) {
    let (mut sender, mut receiver) = socket.split();

    let client_id = Uuid::new_v4().to_string();

    let mut sync_rx = {
        let mut registry = state.client_registry.write().unwrap();
        registry.register(client_id.clone())
    };

    let mut file_changes_rx = state.file_changes_tx.subscribe();

    let mut changed_files = Vec::new();

    let cleanup_state = state.clone();
    let cleanup_client_id = client_id.clone();
    let send_state = state.clone();

    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(change) = file_changes_rx.recv() => {
                    // Check if this file should be suppressed from reload
                    let should_suppress = {
                        let mut suppress = send_state.suppress_reload.lock().unwrap();
                        let now = Instant::now();
                        let mut to_remove = Vec::new();
                        let mut suppress_this = false;

                        for (path, written_at) in suppress.iter() {
                            let age = now.duration_since(*written_at);
                            if age.as_millis() < 500 {
                                if path == &change.path {
                                    suppress_this = true;
                                }
                            } else {
                                to_remove.push(path.clone());
                            }
                        }

                        for path in to_remove {
                            suppress.remove(&path);
                        }

                        suppress_this
                    };

                    if !should_suppress {
                        changed_files.push(change.path.clone());
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    if !changed_files.is_empty() {
                        let msg = ServerMessage::DevServer(DevServerMessage::Reload {
                            files: changed_files.iter()
                                .map(|p| p.display().to_string())
                                .collect(),
                        });

                        if let Ok(json) = serde_json::to_string(&msg)
                            && sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }

                        changed_files.clear();
                    }
                }

                Some(sync_msg) = sync_rx.recv() => {
                    let msg = ServerMessage::Sync(sync_msg);
                    if let Ok(json) = serde_json::to_string(&msg)
                        && sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                }

                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    let msg = ServerMessage::DevServer(DevServerMessage::Ping);
                    if let Ok(json) = serde_json::to_string(&msg)
                        && sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                }
            }
        }
    });

    let recv_state = state.clone();
    let recv_client_id = client_id.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        handle_client_message(client_msg, &recv_client_id, &recv_state).await;
                    }
                    Err(e) => {
                        // The dev `/ws` is the edit-protocol socket, but a served
                        // page may also aim its OWN runtime traffic here (a
                        // `@host ws("/ws")` signal frame `{type:"move",…}`, a
                        // `@presence` `{type:"PresenceJoin",…}`). Those carry a
                        // discriminator the edit protocol does not own — they are
                        // not malformed edits, just not-for-us, so swallow them
                        // quietly. Only a frame bearing a KNOWN edit/control
                        // `type` that still fails to parse is a real protocol
                        // error worth surfacing.
                        let known_type = serde_json::from_str::<serde_json::Value>(&text)
                            .ok()
                            .and_then(|v| {
                                v.get("type")
                                    .and_then(|t| t.as_str())
                                    .map(is_edit_protocol_type)
                            })
                            .unwrap_or(false);
                        if known_type {
                            eprintln!("[ws] Failed to parse client message: {e}");
                        }
                    }
                }
            } else if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    // Cleanup on disconnect
    let mut registry = cleanup_state.client_registry.write().unwrap();
    registry.unregister(&cleanup_client_id);
}

async fn handle_client_message(msg: ClientMessage, client_id: &str, state: &Arc<TestServerState>) {
    match msg {
        ClientMessage::DevServer(dev_msg) => match dev_msg {
            DevServerClientMessage::Pause => {
                eprintln!("Received pause command");
            }
            DevServerClientMessage::Resume => {
                eprintln!("Received resume command");
            }
            DevServerClientMessage::Step { direction } => {
                eprintln!("Received step command: {}", direction);
            }
            DevServerClientMessage::Pong => {}
        },
        ClientMessage::Sync(edit_msg) => {
            // Handle InspectElement directly — it's a query, not a mutation,
            // so it doesn't need HandleResult's suppress_reload or broadcast.
            if let SyncClientMessage::InspectElement {
                ref selector,
                ref file_hint,
            } = edit_msg
            {
                let site_dir = match &state.site_dir {
                    Some(dir) => dir.clone(),
                    None => {
                        let response = SyncServerMessage::ElementContext {
                            selector: selector.clone(),
                            source_files: vec![],
                            sections: vec![],
                            state_machine: None,
                            data_bindings: vec![],
                            not_found: true,
                        };
                        let registry = state.client_registry.read().unwrap();
                        registry.send_to(client_id, response);
                        return;
                    }
                };
                let response = sync::handlers::handle_inspect_element(
                    &site_dir,
                    selector,
                    file_hint.as_deref(),
                );
                let registry = state.client_registry.read().unwrap();
                registry.send_to(client_id, response);
                return;
            }

            // PLAN-064 B2: InspectStructure is likewise a QUERY (the dev-ws read
            // into the shared Structure IR). Route it directly, same as
            // InspectElement — no suppress_reload/broadcast.
            if let SyncClientMessage::InspectStructure {
                ref file,
                ref template,
            } = edit_msg
            {
                let site_dir = match &state.site_dir {
                    Some(dir) => dir.clone(),
                    None => {
                        let response = SyncServerMessage::Structure {
                            file: file.clone(),
                            template: template.clone().unwrap_or_else(|| "main".to_string()),
                            nodes: vec![],
                            error: Some("no site directory configured".to_string()),
                        };
                        let registry = state.client_registry.read().unwrap();
                        registry.send_to(client_id, response);
                        return;
                    }
                };
                let response =
                    sync::handlers::handle_inspect_structure(&site_dir, file, template.as_deref());
                let registry = state.client_registry.read().unwrap();
                registry.send_to(client_id, response);
                return;
            }

            // Log the incoming edit message
            let (msg_type, msg_file) = match &edit_msg {
                SyncClientMessage::EditJson { file, .. } => ("EditJson", file.clone()),
                SyncClientMessage::EditHtml { file, .. } => ("EditHtml", file.clone()),
                SyncClientMessage::EditAst { file, .. } => ("EditAst", file.clone()),
                SyncClientMessage::EditJsonArray { file, .. } => ("EditJsonArray", file.clone()),
                SyncClientMessage::EditToken { file, .. } => ("EditToken", file.clone()),
                // Query messages are handled above and return early.
                SyncClientMessage::InspectElement { .. }
                | SyncClientMessage::InspectStructure { .. } => unreachable!(),
            };
            eprintln!("[sync] Received {msg_type} for {msg_file}");

            let site_dir = match &state.site_dir {
                Some(dir) => dir.clone(),
                None => {
                    let op_id = match &edit_msg {
                        SyncClientMessage::EditJson { op_id, .. } => op_id.clone(),
                        SyncClientMessage::EditHtml { op_id, .. } => op_id.clone(),
                        SyncClientMessage::EditAst { op_id, .. } => op_id.clone(),
                        SyncClientMessage::EditJsonArray { op_id, .. } => op_id.clone(),
                        SyncClientMessage::EditToken { op_id, .. } => op_id.clone(),
                        // Query messages are handled above and return early.
                        SyncClientMessage::InspectElement { .. }
                        | SyncClientMessage::InspectStructure { .. } => unreachable!(),
                    };
                    eprintln!("[sync] Rejected {msg_type}: no site directory configured");
                    let reject = SyncServerMessage::Reject {
                        op_id,
                        reason: "No site directory configured".to_string(),
                    };
                    let registry = state.client_registry.read().unwrap();
                    registry.send_to(client_id, reject);
                    return;
                }
            };

            let result = sync::handle_message(&site_dir, edit_msg);

            // Log result
            match &result.response {
                SyncServerMessage::Ack { .. } => {
                    eprintln!("[sync] {msg_type} for {msg_file}: ok");
                }
                SyncServerMessage::Reject { reason, .. } => {
                    eprintln!("[sync] {msg_type} for {msg_file}: rejected — {reason}");
                }
                _ => {}
            }

            // Send Ack/Reject to the originating client
            {
                let registry = state.client_registry.read().unwrap();
                registry.send_to(client_id, result.response);
            }

            // If the edit succeeded and wrote a file, suppress reload for 500ms
            if let Some(file_path) = result.suppress_reload_path {
                let mut suppress = state.suppress_reload.lock().unwrap();
                suppress.insert(file_path, Instant::now());
            }

            // Broadcast DataUpdate to ALL connected clients
            if let Some(broadcast_msg) = result.broadcast {
                let registry = state.client_registry.read().unwrap();
                registry.broadcast_all(broadcast_msg);
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

    #[tokio::test]
    async fn test_websocket_broadcast() {
        let (tx, _rx) = broadcast::channel(100);
        let compilation_cache = Arc::new(RwLock::new(None));
        let state = Arc::new(TestServerState::new(tx.clone(), compilation_cache));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let app = create_test_runner_routes(state);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let url = format!("ws://{}/ws", addr);
        let (mut ws_stream, _) = connect_async(&url).await.unwrap();

        let file_change = FileChange {
            path: "test.st".into(),
            kind: crate::watcher::ChangeKind::Modified,
        };
        tx.send(file_change).unwrap();

        tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
            while let Some(Ok(msg)) = ws_stream.next().await {
                if let WsMessage::Text(text) = msg {
                    let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
                    if let ServerMessage::DevServer(DevServerMessage::Reload { files }) = server_msg
                    {
                        assert_eq!(files.len(), 1);
                        assert_eq!(files[0], "test.st");
                        return;
                    }
                }
            }
        })
        .await
        .expect("Should receive reload message");
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let (tx, _rx) = broadcast::channel(100);
        let compilation_cache = Arc::new(RwLock::new(None));
        let state = Arc::new(TestServerState::new(tx.clone(), compilation_cache));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let app = create_test_runner_routes(state);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let url = format!("ws://{}/ws", addr);
        let (mut client1, _) = connect_async(&url).await.unwrap();
        let (mut client2, _) = connect_async(&url).await.unwrap();

        let file_change = FileChange {
            path: "both.st".into(),
            kind: crate::watcher::ChangeKind::Modified,
        };
        tx.send(file_change).unwrap();

        let task1 = tokio::spawn(async move {
            while let Some(Ok(msg)) = client1.next().await {
                if let WsMessage::Text(text) = msg {
                    let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
                    if matches!(
                        server_msg,
                        ServerMessage::DevServer(DevServerMessage::Reload { .. })
                    ) {
                        return true;
                    }
                }
            }
            false
        });

        let task2 = tokio::spawn(async move {
            while let Some(Ok(msg)) = client2.next().await {
                if let WsMessage::Text(text) = msg {
                    let server_msg: ServerMessage = serde_json::from_str(&text).unwrap();
                    if matches!(
                        server_msg,
                        ServerMessage::DevServer(DevServerMessage::Reload { .. })
                    ) {
                        return true;
                    }
                }
            }
            false
        });

        let results = tokio::join!(
            tokio::time::timeout(tokio::time::Duration::from_secs(2), task1),
            tokio::time::timeout(tokio::time::Duration::from_secs(2), task2)
        );

        assert!(results.0.is_ok());
        assert!(results.1.is_ok());
        assert!(results.0.unwrap().unwrap());
        assert!(results.1.unwrap().unwrap());
    }

    #[test]
    fn test_message_serialization() {
        let reload_msg = ServerMessage::DevServer(DevServerMessage::Reload {
            files: vec!["test1.st".to_string(), "test2.html".to_string()],
        });
        let json = serde_json::to_string(&reload_msg).unwrap();
        assert!(json.contains("\"type\":\"Reload\""));
        assert!(json.contains("test1.st"));

        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            parsed,
            ServerMessage::DevServer(DevServerMessage::Reload { .. })
        ));

        let error_msg = ServerMessage::DevServer(DevServerMessage::Error {
            message: "Test error".to_string(),
        });
        let json = serde_json::to_string(&error_msg).unwrap();
        assert!(json.contains("\"type\":\"Error\""));

        let pause_json = r#"{"type":"Pause"}"#;
        let client_msg: ClientMessage = serde_json::from_str(pause_json).unwrap();
        assert!(matches!(
            client_msg,
            ClientMessage::DevServer(DevServerClientMessage::Pause)
        ));

        let step_json = r#"{"type":"Step","direction":"forward"}"#;
        let client_msg: ClientMessage = serde_json::from_str(step_json).unwrap();
        assert!(matches!(
            client_msg,
            ClientMessage::DevServer(DevServerClientMessage::Step { .. })
        ));
    }

    #[test]
    fn test_edit_message_serialization() {
        use serde_json::json;

        let edit = SyncClientMessage::EditJson {
            file: "data/products.json".to_string(),
            path: "[2].price".to_string(),
            value: json!(29.99),
            op_id: "op_1".to_string(),
        };
        let client_msg = ClientMessage::Sync(edit);
        let json_str = serde_json::to_string(&client_msg).unwrap();
        assert!(json_str.contains("\"type\":\"EditJson\""));
        assert!(json_str.contains("\"file\":\"data/products.json\""));

        let ack = SyncServerMessage::Ack {
            op_id: "op_1".to_string(),
        };
        let server_msg = ServerMessage::Sync(ack);
        let json_str = serde_json::to_string(&server_msg).unwrap();
        assert!(json_str.contains("\"type\":\"Ack\""));
        assert!(json_str.contains("\"op_id\":\"op_1\""));

        let data_update = SyncServerMessage::DataUpdate {
            source: "data/products.json".to_string(),
            data: json!([{"name": "Test"}]),
        };
        let server_msg = ServerMessage::Sync(data_update);
        let json_str = serde_json::to_string(&server_msg).unwrap();
        assert!(json_str.contains("\"type\":\"DataUpdate\""));
        assert!(json_str.contains("\"source\":\"data/products.json\""));
    }

    #[test]
    fn edit_protocol_type_recognises_own_frames_only() {
        // Every edit/control discriminator is owned by the dev `/ws`.
        for ty in [
            "EditJson",
            "EditHtml",
            "EditAst",
            "EditJsonArray",
            "EditToken",
            "InspectElement",
            "Pause",
            "Resume",
            "Step",
            "Pong",
        ] {
            assert!(is_edit_protocol_type(ty), "{ty} should be owned");
        }
        // A served page's OWN runtime traffic (signal/presence/collection frames)
        // is NOT the edit protocol's — those must be ignored quietly, not logged
        // as parse errors (the reported `/ws` spam on demos/liveview-kanban).
        for ty in [
            "move",
            "PresenceJoin",
            "PresenceUpdate",
            "PresenceLeave",
            "Subscribe",
            "Unsubscribe",
            "Insert",
            "Update",
            "Delete",
        ] {
            assert!(!is_edit_protocol_type(ty), "{ty} should NOT be owned");
        }
    }
}
