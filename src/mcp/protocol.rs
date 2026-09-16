//! MCP stdio transport: newline-delimited JSON-RPC 2.0 over stdin/stdout.
//!
//! Wave 1 of PLAN-037. This is the wire layer — it knows nothing about
//! Spacetime; it speaks the Model Context Protocol handshake and dispatches
//! `tools/list` / `tools/call` to the tool layer.
//!
//! INVARIANT: stdout carries protocol bytes ONLY. Every diagnostic goes to
//! stderr (`eprintln!`/`log`), because the client (Spell) parses stdout as a
//! strict newline-delimited JSON-RPC stream. A stray `println!` corrupts the
//! session.
//!
//! Protocol facts (verified against Spell @ packages/coding-agent/src/mcp):
//!   - transport     : one JSON object per line, `\n`-terminated
//!   - handshake     : `initialize` → reply, then client `notifications/initialized`
//!   - protocolVer   : client offers "2025-03-26"; we echo whatever it sends
//!   - discovery     : `tools/list` → { tools: [...] }
//!   - invoke        : `tools/call` { name, arguments } → { content, structuredContent? }

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use super::resources;
use super::state::McpState;
use super::tools;

/// Protocol version we advertise when the client omits one.
const DEFAULT_PROTOCOL_VERSION: &str = "2025-03-26";
const SERVER_NAME: &str = "spacetime";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the MCP stdio server loop until stdin closes (EOF).
///
/// Blocking single-threaded read loop: MCP stdio is strictly request/response
/// per line, so we never need concurrency here in Wave 1.
pub fn serve(workspace_root: std::path::PathBuf) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut state = McpState::new(workspace_root);

    eprintln!(
        "[spacetime-mcp] serving on stdio (root: {})",
        state.workspace_root.display()
    );

    // Pull-based line iterator: a tool (st_elicit) can send a server→client
    // request mid-dispatch and pull the reply off the same stream.
    let mut lines = stdin.lock().lines();
    while let Some(line) = lines.next() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[spacetime-mcp] parse error: {e} (line: {trimmed})");
                continue;
            }
        };

        // Notifications have no `id` and expect no response.
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");

        if id.is_none() {
            handle_notification(method);
            continue;
        }
        let id = id.unwrap();

        // Capture the client's advertised capabilities so a tool can know
        // whether elicitation (server→user push) is available.
        if method == "initialize" {
            let supports = msg
                .get("params")
                .and_then(|p| p.get("capabilities"))
                .and_then(|c| c.get("elicitation"))
                .is_some();
            state.client_supports_elicitation = supports;
            eprintln!("[spacetime-mcp] client elicitation capability: {supports}");
        }

        // st_elicit needs IO access (send a request, await the reply), so it is
        // handled here in the loop rather than in the pure dispatch fn.
        let response = if method == "tools/call" && tool_name(&msg) == Some("st_elicit") {
            let args = msg
                .get("params")
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let result = elicit_roundtrip(&args, &mut state, &mut out, &mut lines)?;
            json!({ "jsonrpc": "2.0", "id": id, "result": result })
        } else {
            match dispatch(method, &msg, &mut state) {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": err.code, "message": err.message },
                }),
            }
        };

        writeln!(out, "{}", serde_json::to_string(&response).unwrap())?;
        out.flush()?;
    }

    eprintln!("[spacetime-mcp] stdin closed, shutting down");
    Ok(())
}

fn tool_name(msg: &Value) -> Option<&str> {
    msg.get("params")
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
}

/// Perform a server→client `elicitation/create` round-trip: send the request,
/// then pull lines until the matching response arrives. Returns an MCP
/// tools/call result describing the user's action + content.
///
/// If the client never advertised the elicitation capability, returns a result
/// telling the agent the prompt could not be shown (no elicitation support).
fn elicit_roundtrip<W: Write, L: Iterator<Item = std::io::Result<String>>>(
    args: &Value,
    state: &mut McpState,
    out: &mut W,
    lines: &mut L,
) -> std::io::Result<Value> {
    if !state.client_supports_elicitation {
        return Ok(json!({
            "content": [{ "type": "text", "text":
                "This MCP client does not support elicitation (server→user prompts). \
        Ask the user directly in the conversation instead." }],
            "structuredContent": { "supported": false },
            "isError": true,
        }));
    }

    let message = args.get("message").and_then(Value::as_str).unwrap_or("");
    let schema = args
        .get("schema")
        .cloned()
        .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));

    // Send the server-initiated request with a distinctive id.
    state.next_elicit_id += 1;
    let req_id = format!("elicit-{}", state.next_elicit_id);
    let request = json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "method": "elicitation/create",
        "params": { "message": message, "requestedSchema": schema },
    });
    writeln!(out, "{}", serde_json::to_string(&request).unwrap())?;
    out.flush()?;
    eprintln!("[spacetime-mcp] sent elicitation/create id={req_id}");

    // Pull lines until we see the response to req_id. (The client is mid
    // tools/call to us, so it won't interleave other requests; any stray
    // notifications are simply skipped.)
    for line in lines.by_ref() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(reply): Result<Value, _> = serde_json::from_str(trimmed) else {
            continue;
        };
        let matches = reply.get("id").and_then(Value::as_str) == Some(req_id.as_str());
        if !matches {
            // Skip notifications; ignore anything else while we wait.
            continue;
        }
        if let Some(result) = reply.get("result") {
            let action = result
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("cancel");
            let content = result.get("content").cloned().unwrap_or(Value::Null);
            let summary = match action {
                "accept" => format!("User accepted: {content}"),
                "decline" => "User declined the prompt.".to_string(),
                _ => "User cancelled the prompt.".to_string(),
            };
            return Ok(json!({
                "content": [{ "type": "text", "text": summary }],
                "structuredContent": { "supported": true, "action": action, "content": content },
            }));
        }
        if let Some(err) = reply.get("error") {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Elicitation error: {err}") }],
                "structuredContent": { "supported": true, "action": "error", "error": err },
                "isError": true,
            }));
        }
    }
    // stdin closed mid-elicitation.
    Ok(json!({
        "content": [{ "type": "text", "text": "Elicitation aborted (stream closed)." }],
        "structuredContent": { "supported": true, "action": "cancel" },
        "isError": true,
    }))
}

/// A JSON-RPC error (code + human message).
#[derive(Debug)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {method}"),
        }
    }
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: msg.into(),
        }
    }
    /// Internal error (-32603). Reserved for W2 (live HTTP surface) failures.
    #[allow(dead_code)]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: msg.into(),
        }
    }
}

fn handle_notification(method: &str) {
    match method {
        "notifications/initialized" => {
            eprintln!("[spacetime-mcp] client initialized");
        }
        "notifications/cancelled" => {}
        other => {
            eprintln!("[spacetime-mcp] ignoring notification: {other}");
        }
    }
}

fn dispatch(method: &str, msg: &Value, state: &mut McpState) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(initialize_result(msg)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::list() })),
        "resources/templates/list" => Ok(resources::templates()),
        "resources/list" => Ok(resources::list(state)),
        "resources/read" => {
            let uri = msg
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::invalid_params("resources/read missing `uri`"))?;
            resources::read(uri, state).map_err(RpcError::invalid_params)
        }
        "resources/subscribe" => {
            let uri = msg
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::invalid_params("resources/subscribe missing `uri`"))?;
            resources::validate_uri(uri, state).map_err(RpcError::invalid_params)?;
            state.subscribe_resource(uri.to_string());
            Ok(json!({}))
        }
        "resources/unsubscribe" => {
            let uri = msg
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::invalid_params("resources/unsubscribe missing `uri`"))?;
            state.unsubscribe_resource(uri);
            Ok(json!({}))
        }
        "tools/call" => {
            let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::invalid_params("tools/call missing `name`"))?;
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(tools::call(name, &args, state))
        }
        other => Err(RpcError::method_not_found(other)),
    }
}

fn initialize_result(msg: &Value) -> Value {
    let protocol_version = msg
        .get("params")
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION)
        .to_string();

    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "subscribe": true, "listChanged": true }
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION,
        },
        "instructions": "Spacetime live-coding MCP. Model: an Environment is an \
    import path (workspace dir); a Tab is an entry point; a Function is live .st \
    source with a contract; an Instance is a mounted function with an event stream. \
    Discover/open envs and tabs, or use st_fn_put/st_mount/st_await/st_inspect for \
    function-template interfaces. Tool results carry machine-readable `structuredContent`."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })
    }

    fn tool_call(name: &str, arguments: Value) -> Value {
        request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn post_json(url: &str, path: &str, body: Value) -> Value {
        use std::io::{Read, Write};

        let host_port = url
            .strip_prefix("http://")
            .and_then(|rest| rest.split('/').next())
            .expect("instance URL has host:port");
        let body = body.to_string();
        let mut stream = std::net::TcpStream::connect(host_port).expect("connect live server");
        write!(
            stream,
            "POST {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write request");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read response");
        let (_, payload) = response.split_once("\r\n\r\n").expect("HTTP body");
        serde_json::from_str(payload).expect("JSON response")
    }

    fn read_json_content(result: &Value) -> Value {
        let text = result["contents"][0]["text"]
            .as_str()
            .expect("resource read returns text content");
        serde_json::from_str(text).expect("resource text is JSON")
    }

    #[test]
    fn initialize_advertises_resources() {
        let init = initialize_result(&request("initialize", json!({})));
        assert_eq!(init["capabilities"]["resources"]["subscribe"], true);
        assert_eq!(init["capabilities"]["resources"]["listChanged"], true);
    }

    #[test]
    fn mcp_resources_expose_env_tabs_and_instance_events() {
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);

        let listed = dispatch(
            "resources/list",
            &request("resources/list", json!({})),
            &mut state,
        )
        .expect("list resources");
        let resources = listed["resources"].as_array().expect("resources array");
        assert!(
            resources
                .iter()
                .any(|r| r["uri"] == "spacetime://mcp/environments")
        );

        let env = dispatch(
            "tools/call",
            &tool_call("st_env_open", json!({ "path": "demos/spacetime-docs" })),
            &mut state,
        )
        .expect("open env");
        let env_id = env["structuredContent"]["id"].as_str().expect("env id");

        let _tab = dispatch(
            "tools/call",
            &tool_call("st_tab_open", json!({ "env": env_id, "entry": "index.st" })),
            &mut state,
        )
        .expect("open tab");

        let tabs_uri = format!("spacetime://mcp/environments/{env_id}/tabs");
        let tabs = dispatch(
            "resources/read",
            &request("resources/read", json!({ "uri": tabs_uri })),
            &mut state,
        )
        .expect("read tabs");
        let tabs_body = read_json_content(&tabs);
        assert_eq!(tabs_body["env"]["id"], env_id);
        assert_eq!(tabs_body["tabs"].as_array().unwrap().len(), 1);

        let mounted = dispatch(
            "tools/call",
            &tool_call(
                "st_mount",
                json!({
                    "source": "<h1>MCP resource smoke</h1>",
                    "title": "resource-smoke"
                }),
            ),
            &mut state,
        )
        .expect("mount function");
        let instance_id = mounted["structuredContent"]["instance_id"]
            .as_str()
            .expect("instance id");
        let events_uri = mounted["structuredContent"]["events_resource"]
            .as_str()
            .expect("events resource");
        assert_eq!(
            events_uri,
            format!("spacetime://instances/{instance_id}/events")
        );

        let before = dispatch(
            "resources/read",
            &request("resources/read", json!({ "uri": events_uri })),
            &mut state,
        )
        .expect("read empty events");
        let before_body = read_json_content(&before);
        assert_eq!(before_body["latest_seq"], 0);
        assert_eq!(before_body["events"].as_array().unwrap().len(), 0);

        state
            .live()
            .expect("live server")
            .record_event(
                instance_id,
                json!({ "type": "clicked", "payload": { "value": "ok" } }),
            )
            .expect("record event");

        let after = dispatch(
            "resources/read",
            &request("resources/read", json!({ "uri": events_uri })),
            &mut state,
        )
        .expect("read events");
        let after_body = read_json_content(&after);
        assert_eq!(after_body["latest_seq"], 1);
        assert_eq!(after_body["events"][0]["type"], "clicked");
        assert_eq!(after_body["events"][0]["payload"]["value"], "ok");
    }

    #[test]
    fn comments_resource_matches_the_comment_list_contract() {
        let root = std::env::temp_dir().join(format!(
            "spacetime_mcp_comments_resource_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create project");
        let mut state = McpState::new(std::env::current_dir().expect("test cwd"));
        let env = dispatch(
            "tools/call",
            &tool_call("st_env_open", json!({ "path": root })),
            &mut state,
        )
        .expect("open env");
        let env_id = env["structuredContent"]["id"].as_str().expect("env id");

        let listed_resources = dispatch(
            "resources/list",
            &request("resources/list", json!({})),
            &mut state,
        )
        .expect("list resources");
        let descriptor = listed_resources["resources"]
            .as_array()
            .expect("resources array")
            .iter()
            .find(|resource| resource["uri"] == format!("comments://{env_id}"))
            .expect("comments resource descriptor");
        assert!(
            descriptor["description"]
                .as_str()
                .is_some_and(|description| description.contains("context")
                    && description.contains("st_comments_list")),
            "description explains attachment versus filtering: {descriptor}"
        );

        let _added = dispatch(
            "tools/call",
            &tool_call(
                "st_comments_add",
                json!({
                    "env": env_id,
                    "type": "todo",
                    "anchor": { "kind": "page", "route": "/" },
                    "text": "Keep resource and tool records identical"
                }),
            ),
            &mut state,
        )
        .expect("add comment");
        let listed = dispatch(
            "tools/call",
            &tool_call("st_comments_list", json!({ "env": env_id })),
            &mut state,
        )
        .expect("list comments");
        let resource = dispatch(
            "resources/read",
            &request(
                "resources/read",
                json!({ "uri": format!("comments://{env_id}") }),
            ),
            &mut state,
        )
        .expect("read comments resource");
        assert_eq!(
            read_json_content(&resource),
            listed["structuredContent"],
            "resources and tools must share the comment index projection"
        );
        dispatch(
            "resources/subscribe",
            &request(
                "resources/subscribe",
                json!({ "uri": format!("comments://{env_id}") }),
            ),
            &mut state,
        )
        .expect("subscribe through the existing resource rail");
        assert!(
            state
                .resource_subscriptions()
                .any(|uri| uri == &format!("comments://{env_id}"))
        );

        let empty_root = root.join("empty");
        std::fs::create_dir_all(&empty_root).expect("create empty project");
        let empty_env = dispatch(
            "tools/call",
            &tool_call("st_env_open", json!({ "path": empty_root })),
            &mut state,
        )
        .expect("open empty env");
        let empty_id = empty_env["structuredContent"]["id"]
            .as_str()
            .expect("empty env id");
        let empty = dispatch(
            "resources/read",
            &request(
                "resources/read",
                json!({ "uri": format!("comments://{empty_id}") }),
            ),
            &mut state,
        )
        .expect("read empty comments");
        let empty_index = read_json_content(&empty);
        assert_eq!(empty_index["records"], json!([]));
        assert_eq!(empty_index["diagnostics"], json!([]));
        assert_eq!(empty_index["orphans"], json!([]));

        let unknown = dispatch(
            "resources/read",
            &request("resources/read", json!({ "uri": "comments://missing" })),
            &mut state,
        )
        .expect_err("unknown env rejected");
        assert!(
            unknown.message.contains("st_env_open"),
            "actionable error: {unknown:?}"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn empty_string_tool_arguments_are_absent() {
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);

        let env = dispatch(
            "tools/call",
            &tool_call("st_env_open", json!({ "path": "demos/spacetime-docs" })),
            &mut state,
        )
        .expect("open env");
        let env_id = env["structuredContent"]["id"].as_str().expect("env id");

        let file_tab = dispatch(
            "tools/call",
            &tool_call(
                "st_tab_open",
                json!({
                    "env": env_id,
                    "entry": "index.st",
                    "code": "",
                    "title": ""
                }),
            ),
            &mut state,
        )
        .expect("wrapper-style entry tab ignores empty code/title");
        assert_eq!(file_tab["structuredContent"]["title"], "index.st");

        let inline_tab = dispatch(
            "tools/call",
            &tool_call(
                "st_tab_open",
                json!({
                    "env": env_id,
                    "entry": "",
                    "code": "<h1>inline</h1>",
                    "title": "inline smoke"
                }),
            ),
            &mut state,
        )
        .expect("wrapper-style inline tab ignores empty entry");
        assert_eq!(inline_tab["structuredContent"]["title"], "inline smoke");

        let put = dispatch(
            "tools/call",
            &tool_call("st_fn_put", json!({ "source": "<h1>mounted</h1>" })),
            &mut state,
        )
        .expect("register function");
        let fn_id = put["structuredContent"]["id"].as_str().expect("fn id");

        let mounted = dispatch(
            "tools/call",
            &tool_call(
                "st_mount",
                json!({
                    "fn_id": fn_id,
                    "name": "",
                    "source": "",
                    "title": ""
                }),
            ),
            &mut state,
        )
        .expect("mount ignores empty source/name/title and uses fn_id");
        assert_eq!(mounted["structuredContent"]["function"]["id"], fn_id);

        let inspected_all = dispatch(
            "tools/call",
            &tool_call(
                "st_inspect",
                json!({ "fn_id": "", "name": "", "instance_id": "" }),
            ),
            &mut state,
        )
        .expect("empty inspect identifiers list state");
        assert!(
            inspected_all["structuredContent"]["functions"]
                .as_array()
                .unwrap()
                .len()
                >= 1
        );

        let no_tab = dispatch(
            "tools/call",
            &tool_call(
                "st_tab_open",
                json!({ "env": env_id, "entry": "", "code": "" }),
            ),
            &mut state,
        );
        assert!(no_tab.is_ok(), "tool errors are represented as results");
        assert_eq!(no_tab.unwrap()["isError"], true);
    }

    #[test]
    fn st_workbench_mounts_env_browser() {
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);

        let env = dispatch(
            "tools/call",
            &tool_call("st_env_open", json!({ "path": "demos/spacetime-docs" })),
            &mut state,
        )
        .expect("open env");
        let env_id = env["structuredContent"]["id"].as_str().expect("env id");
        dispatch(
            "tools/call",
            &tool_call("st_tab_open", json!({ "env": env_id, "entry": "index.st" })),
            &mut state,
        )
        .expect("open tab");

        let workbench = dispatch(
            "tools/call",
            &tool_call("st_workbench", json!({ "title": "Workbench Test" })),
            &mut state,
        )
        .expect("mount workbench");
        assert_eq!(
            workbench["structuredContent"]["function"]["name"],
            "mcp-workbench"
        );
        assert!(
            workbench["structuredContent"]["url"]
                .as_str()
                .unwrap()
                .contains("/__mcp/instance/")
        );
        assert!(
            workbench["structuredContent"]["events_resource"]
                .as_str()
                .unwrap()
                .starts_with("spacetime://instances/")
        );
    }

    #[test]
    fn st_fn_put_reports_contract_compatibility() {
        // FEAT-124: re-putting with the same contract is "compatible"; a changed
        // contract is "breaking". The verdict rides in structuredContent + text.
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);
        let p1 = dispatch(
            "tools/call",
            &tool_call(
                "st_fn_put",
                json!({ "name": "f", "source": "@template &main($x) { <i>`$x`</i> }" }),
            ),
            &mut state,
        )
        .expect("put v1");
        assert_eq!(p1["structuredContent"]["compatibility"], json!("initial"));
        // Same contract (param $x), different body -> compatible.
        let p2 = dispatch(
            "tools/call",
            &tool_call(
                "st_fn_put",
                json!({ "name": "f", "source": "@template &main($x) { <b>`$x`</b> }" }),
            ),
            &mut state,
        )
        .expect("put v2");
        assert_eq!(
            p2["structuredContent"]["compatibility"],
            json!("compatible")
        );
        assert_eq!(p2["structuredContent"]["revision"], json!(2));
        // Changed contract (param renamed) -> breaking.
        let p3 = dispatch(
            "tools/call",
            &tool_call(
                "st_fn_put",
                json!({ "name": "f", "source": "@template &main($y) { <b>`$y`</b> }" }),
            ),
            &mut state,
        )
        .expect("put v3");
        assert_eq!(p3["structuredContent"]["compatibility"], json!("breaking"));
        let text = p3["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains("breaking"), "text warns breaking: {text}");
    }

    #[test]
    fn st_mount_host_composes_guest_into_stage_region() {
        // FEAT-126 / BUG-111: st_mount with `host` composes a function INTO the
        // host's region (recorded in region_mounts) instead of spawning a
        // standalone instance. The guest bundle is then served by
        // /bundles?region=stage for the host page's @mcp-region poll.
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);

        // Mount the workbench host.
        let workbench = dispatch(
            "tools/call",
            &tool_call("st_workbench", json!({ "title": "Host" })),
            &mut state,
        )
        .expect("mount workbench");
        let host_id = workbench["structuredContent"]["instance_id"]
            .as_str()
            .expect("host instance id")
            .to_string();

        // Register a clean guest function.
        let put = dispatch(
            "tools/call",
            &tool_call(
                "st_fn_put",
                json!({ "name": "guest", "source": "@template &main() { <h1>guest</h1> }" }),
            ),
            &mut state,
        )
        .expect("register guest");
        let fn_id = put["structuredContent"]["id"]
            .as_str()
            .expect("fn id")
            .to_string();

        // Compose it INTO the host's stage region.
        let composed = dispatch(
            "tools/call",
            &tool_call(
                "st_mount",
                json!({ "name": "guest", "host": host_id, "region": "stage" }),
            ),
            &mut state,
        )
        .expect("compose into host");
        assert_eq!(
            composed["structuredContent"]["composed"],
            json!(true),
            "result marks a composition, not a standalone mount"
        );
        assert_eq!(composed["structuredContent"]["region"], json!("stage"));
        // The host record now carries the region mount.
        let mounts = composed["structuredContent"]["host"]["region_mounts"]
            .as_array()
            .expect("region_mounts array");
        assert_eq!(mounts.len(), 1, "one region mount recorded");
        assert_eq!(mounts[0]["region"], json!("stage"));
        assert_eq!(mounts[0]["function_id"], json!(fn_id));
        // The summary text names the composition (FEAT-133 readability).
        let text = composed["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains("Composed"), "summary says Composed: {text}");
    }

    #[test]
    fn st_workbench_reads_env_st_from_disk_overlay() {
        // FUP-071: st_workbench must source env.st through the filesystem
        // overlay, not a frozen include_str!. Running from the repo root, the
        // on-disk stdlib/__mcp__/env.st exists, so the mounted workbench's
        // source must match the current disk bytes (live edit→see-it loop).
        let root = std::env::current_dir().expect("test cwd");
        let disk = std::fs::read_to_string(root.join("stdlib/__mcp__/env.st"))
            .expect("repo stdlib/__mcp__/env.st must exist for this test");
        let mut state = McpState::new(root);

        let workbench = dispatch(
            "tools/call",
            &tool_call("st_workbench", json!({ "title": "Overlay Test" })),
            &mut state,
        )
        .expect("mount workbench");
        let source_len = workbench["structuredContent"]["function"]["source_len"]
            .as_u64()
            .expect("workbench function source_len");
        // put_function_from_args trims the source (non_empty_str), so compare
        // against the trimmed on-disk length.
        assert_eq!(
            source_len as usize,
            disk.trim().len(),
            "workbench source length must match the on-disk env.st bytes (overlay path, not a stale/embedded copy)"
        );
    }

    #[test]
    fn server_actions_refresh_input_but_are_not_awaited_agent_actions_are() {
        // PLAN-045: a server action (st_env_open) recorded on the workbench
        // resolves at the sink — input.json refreshes — but is NOT delivered
        // through st_await (it is not agent-addressed). An agent action recorded
        // afterwards IS what st_await returns.
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);

        let workbench = dispatch(
            "tools/call",
            &tool_call("st_workbench", json!({ "title": "Routing Test" })),
            &mut state,
        )
        .expect("mount workbench");
        let instance_id = workbench["structuredContent"]["instance_id"]
            .as_str()
            .expect("instance id")
            .to_string();

        // Record a server action directly (record_event resolves nothing; the
        // sink HTTP path resolves + caches, but here we assert the AWAIT side):
        // the event is server-audience, so await must skip it. We separately
        // drive the sink HTTP path in `signal_endpoint_dispatches_*` for the
        // input.json refresh; here the focus is the await filter.
        state
            .live()
            .expect("live server")
            .record_event(
                &instance_id,
                json!({
                    "type": "mcp-action",
                    "payload": { "action": "st_env_open", "target": "demos/spacetime-docs" },
                    "correlation": "demos/spacetime-docs"
                }),
            )
            .expect("record env action");

        // A kit (agent) action recorded after the server action.
        state
            .live()
            .expect("live server")
            .record_event(
                &instance_id,
                json!({
                    "type": "mcp-action",
                    "payload": { "action": "kit-choice", "value": "blue-green" },
                    "correlation": "pick"
                }),
            )
            .expect("record kit action");

        // st_await must return the AGENT action, never the server nav action.
        let awaited = dispatch(
            "tools/call",
            &tool_call(
                "st_await",
                json!({ "instance_id": instance_id, "timeout_secs": 1 }),
            ),
            &mut state,
        )
        .expect("await agent action");
        assert_eq!(awaited["structuredContent"]["decided"], true);
        assert_eq!(
            awaited["structuredContent"]["event"]["payload"]["action"], "kit-choice",
            "st_await must deliver the agent action, not the server nav action"
        );
        assert_eq!(awaited["structuredContent"]["event"]["audience"], "agent");

        // A second await finds nothing else pending (the server action is never
        // delivered).
        let again = dispatch(
            "tools/call",
            &tool_call(
                "st_await",
                json!({ "instance_id": instance_id, "timeout_secs": 1 }),
            ),
            &mut state,
        )
        .expect("second await");
        assert_eq!(
            again["structuredContent"]["decided"], false,
            "only the one agent action was awaitable; the server action is not"
        );
    }

    #[test]
    fn signal_endpoint_dispatches_workbench_actions_immediately() {
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);

        let workbench = dispatch(
            "tools/call",
            &tool_call("st_workbench", json!({ "title": "Immediate Signal Test" })),
            &mut state,
        )
        .expect("mount workbench");
        let url = workbench["structuredContent"]["url"].as_str().unwrap();
        let instance_id = workbench["structuredContent"]["instance_id"]
            .as_str()
            .unwrap()
            .to_string();
        let signal_path = format!("/__mcp/signal/{instance_id}");

        let env_response = post_json(
            url,
            &signal_path,
            json!({
                "type": "mcp-action",
                "payload": { "action": "st_env_open", "target": "demos/spacetime-docs" },
                "correlation": "demos/spacetime-docs"
            }),
        );
        assert_eq!(env_response["action"]["ok"], true);
        assert_eq!(env_response["action"]["action"], "st_env_open");
        let env_id = env_response["action"]["result"]["id"].as_str().unwrap();

        let input_after_env = state
            .live()
            .unwrap()
            .get_instance(&instance_id)
            .expect("workbench instance")
            .input;
        assert_eq!(
            input_after_env["environments"]["open"]
                .as_array()
                .unwrap()
                .len(),
            1,
            "HTTP signal should refresh input.json without st_await"
        );

        let tab_response = post_json(
            url,
            &signal_path,
            json!({
                "type": "mcp-action",
                "payload": {
                    "action": "st_tab_open",
                    "env": env_id,
                    "code": "<h1>Immediate tab</h1>",
                    "title": "Immediate tab"
                },
                "correlation": "tab"
            }),
        );
        assert_eq!(tab_response["action"]["ok"], true);
        assert_eq!(tab_response["action"]["action"], "st_tab_open");

        let input_after_tab = state
            .live()
            .unwrap()
            .get_instance(&instance_id)
            .expect("workbench instance")
            .input;
        assert_eq!(input_after_tab["tabs"].as_array().unwrap().len(), 1);
        assert_eq!(input_after_tab["tabs"][0]["title"], "Immediate tab");

        // PLAN-045: server actions (st_env_open/st_tab_open) are resolved at the
        // sink and are NOT agent-addressed, so st_await must NOT return them — it
        // sees no agent event and reports `decided:false`. (Pre-PLAN-045 this
        // returned the dispatched server action; that conflated server work with
        // the agent's await stream.)
        let awaited_first = dispatch(
            "tools/call",
            &tool_call(
                "st_await",
                json!({ "instance_id": instance_id, "timeout_secs": 1 }),
            ),
            &mut state,
        )
        .expect("await call returns");
        assert_eq!(
            awaited_first["structuredContent"]["decided"], false,
            "server nav actions must not be delivered through st_await"
        );
    }

    #[test]
    fn resource_subscription_validates_uri() {
        let root = std::env::current_dir().expect("test cwd");
        let mut state = McpState::new(root);
        let ok = dispatch(
            "resources/subscribe",
            &request(
                "resources/subscribe",
                json!({ "uri": "spacetime://mcp/environments" }),
            ),
            &mut state,
        );
        assert!(ok.is_ok());
        assert!(
            state
                .resource_subscriptions()
                .any(|uri| uri == "spacetime://mcp/environments")
        );

        let bad = dispatch(
            "resources/subscribe",
            &request(
                "resources/subscribe",
                json!({ "uri": "spacetime://missing" }),
            ),
            &mut state,
        );
        assert!(bad.is_err());
        assert_eq!(bad.unwrap_err().code, -32602);
    }
}
