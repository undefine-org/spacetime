//! MCP tool surface for Wave 1 of PLAN-037.
//!
//! Every tool returns an MCP `tools/call` result shaped as:
//!   { content: [ {type:"text", text} ], structuredContent: <machine-readable>, isError? }
//!
//! `content` is the human/LLM-readable summary; `structuredContent` is the typed
//! payload Spell surfaces on its `data` channel. We always populate both so a
//! programmatic consumer never has to re-parse prose.
//!
//! Wave 1 tools (prove the Environment→Tab model + the live compile loop):
//!   st_health    — handshake proof (version + capabilities)
//!   st_env_list  — discover environments under the workspace root
//!   st_env_open  — attach / spin up an environment (an import path)
//!   st_tab_list  — list tabs in an environment
//!   st_tab_open  — spin up a tab from a file OR inline code, and COMPILE it

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::resources;
use super::state::{McpState, TabSource};

/// Tool descriptors returned by `tools/list`.
pub fn list() -> Vec<Value> {
    vec![
        tool(
            "st_health",
            "Health check / handshake proof. Returns the Spacetime MCP server \
version and the active workspace root. Call this first to confirm the live \
environment is reachable.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "st_env_list",
            "Discover environments. An environment is an import path (a workspace \
directory that can hold Spacetime entry points). Scans projects/, demos/, and \
examples/ under the workspace root, plus any already-open environments.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "st_env_open",
            "Attach to (or spin up) an environment by path. Idempotent: opening the \
same directory twice returns the same env_id. The path is resolved relative to \
the workspace root.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path (the import path), relative to the workspace root or absolute." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_tab_list",
            "List tabs (entry points) currently open in an environment.",
            json!({
                "type": "object",
                "properties": {
                    "env": { "type": "string", "description": "Environment id from st_env_open / st_env_list." }
                },
                "required": ["env"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_tab_open",
            "Spin up a tab inside an environment and compile it live. A tab is an \
entry point: either an existing file (`entry`) or inline Spacetime code (`code`, \
the usual live-coding case). Compiles immediately and returns the output sizes \
plus any compiler errors as structuredContent.",
            json!({
                "type": "object",
                "properties": {
                    "env":   { "type": "string", "description": "Environment id." },
                    "entry": { "type": "string", "description": "Path to an existing .st file, relative to the environment root. Mutually exclusive with `code`." },
                    "code":  { "type": "string", "description": "Inline Spacetime (.st) source to compile. Mutually exclusive with `entry`." },
                    "title": { "type": "string", "description": "Optional human label for the tab." }
                },
                "required": ["env"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_comments_types",
            "Read the comment-type roster before adding or acting on comments. Use it to learn each type's metadata contract and agent follow-up hint for an environment.",
            json!({
                "type": "object",
                "properties": { "env": { "type": "string", "description": "Optional environment id from st_env_open; defaults to the MCP workspace root." } },
                "additionalProperties": false
            }),
        ),
        tool(
            "st_comments_list",
            "Read comments before choosing work. Each returned record includes its type's agent_hint inline, so use filters to select actionable comments without separately joining the roster.",
            json!({
                "type": "object",
                "properties": {
                    "env": { "type": "string", "description": "Optional environment id from st_env_open; defaults to the MCP workspace root." },
                    "file": { "type": "string", "description": "Optional environment-root-relative source file filter." },
                    "status": { "type": "string", "enum": ["open", "in-progress", "resolved", "wontfix"], "description": "Optional workflow status filter." },
                    "type": { "type": "string", "description": "Optional declared comment type id filter." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "st_comments_add",
            "Create an agent-authored comment for work that must survive into review and follow-up. Use a declared type and its roster metadata contract; this writes the canonical sidecar record.",
            json!({
                "type": "object",
                "properties": {
                    "env": { "type": "string", "description": "Environment id from st_env_open; its root owns .comments/." },
                    "type": { "type": "string", "description": "Declared comment type id from st_comments_types." },
                    "anchor": {
                        "description": "Where the comment is pinned. Inline anchors are NOT creatable here — an inline comment is written as `//@type: text` in the source, and its identity is derived from that line.",
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "kind": { "const": "file" },
                                    "file": { "type": "string", "description": "Project-relative path; absolute paths and `..` are refused." },
                                    "json_path": { "type": "string", "description": "Optional pointer within the file, e.g. /items/0." }
                                },
                                "required": ["kind", "file"],
                                "additionalProperties": false
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "kind": { "const": "page" },
                                    "route": { "type": "string", "description": "Route the remark is about, e.g. /pricing." }
                                },
                                "required": ["kind", "route"],
                                "additionalProperties": false
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "kind": { "const": "element" },
                                    "route": { "type": "string", "description": "Route containing the selected element, e.g. /pricing." },
                                    "selector": { "type": "string", "description": "Positional hint for the element (a data-st-node path or CSS selector). This is NOT the identity: structural ids shift when markup is inserted above the target." },
                                    "fingerprint": { "type": "string", "description": "Content fingerprint of the element, minted by the toolchain. This is the IDENTITY: it lets the comment follow its element when the page is edited, and orphan loudly when that element is gone. Omit only when you genuinely cannot compute it — an anchor without one is stored as unverified and may point at the wrong element after any edit." }
                                },
                                "required": ["kind", "route", "selector"],
                                "additionalProperties": false
                            }
                        ]
                    },
                    "text": { "type": "string", "description": "Human-readable work item text." },
                    "meta": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Optional metadata, validated against the selected type's fields." },
                    "agent_name": { "type": "string", "description": "Optional author name; defaults to 'agent'." },
                    "element_content": {
                        "type": "object",
                        "description": "For an element anchor: what you SEE at the element. The server mints the content fingerprint from this, so prefer it over computing a fingerprint yourself — it is the identity that lets the comment survive page edits.",
                        "properties": {
                            "tag": { "type": "string", "description": "Element tag name, e.g. p or h2." },
                            "class": { "type": "string", "description": "The element's literal class attribute, if any." },
                            "id_attr": { "type": "string", "description": "The element's literal id attribute, if any." },
                            "text": { "type": "string", "description": "The element's visible text." },
                            "selector": { "type": "string", "description": "Optional positional hint (data-st-node path)." }
                        },
                        "required": ["tag"],
                        "additionalProperties": false
                    }
                },
                "required": ["env", "type", "anchor", "text"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_comments_update",
            "Update the workflow state or append an agent reply after work progresses. Use this instead of editing .comments files so every writer keeps the canonical record shape.",
            json!({
                "type": "object",
                "properties": {
                    "env": { "type": "string", "description": "Environment id from st_env_open; its root owns .comments/." },
                    "id": { "type": "string", "description": "Existing comment id from st_comments_list." },
                    "status": { "type": "string", "enum": ["open", "in-progress", "resolved", "wontfix"], "description": "Optional replacement workflow status." },
                    "reply": { "type": "string", "description": "Optional reply text to append as the agent." },
                    "agent_name": { "type": "string", "description": "Optional author name for the reply; defaults to 'agent'." }
                },
                "required": ["env", "id"],
                "anyOf": [
                    { "required": ["status"] },
                    { "required": ["reply"] }
                ],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_workbench",
            "Mount the Spacetime-authored MCP workbench: one page showing discovered/open environments, tabs, registered functions, mounted instances, and recent events. The page is implemented by stdlib/__mcp__/env.st and uses the generic Function Environment.",
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Optional browser title for the workbench instance." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "st_fn_put",
            "Register or update a live Spacetime function in the MCP Function Environment. \
A function is source + a forward-compatible contract + compile artifacts. If `name` \
matches an existing function, this creates a new revision under the same fn_id. \
Returns the function id, revision, compile sizes, diagnostics, and contract.",
            json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Spacetime source for the function/interface." },
                    "name": { "type": "string", "description": "Optional stable function name. Reusing it updates the same fn_id revision." },
                    "env": { "type": "string", "description": "Optional environment id; imports in source resolve relative to its root." },
                    "contract": { "type": "object", "description": "Optional declared contract until compiler introspection owns this shape." }
                },
                "required": ["source"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_mount",
            "Mount a registered function (or inline source) as a live browser instance. \
Returns an instance id, URL, function metadata, and the contract. The mounted page \
gets /input.json beside it and can emit events to /__mcp/signal/{instance}.",
            json!({
                "type": "object",
                "properties": {
                    "fn_id": { "type": "string", "description": "Registered function id, e.g. fn-1. Mutually exclusive with source; can also pass `name`." },
                    "name": { "type": "string", "description": "Registered function name. Mutually exclusive with fn_id/source." },
                    "source": { "type": "string", "description": "Inline Spacetime source to register implicitly, then mount." },
                    "env": { "type": "string", "description": "Optional environment id for inline source import resolution." },
                    "title": { "type": "string", "description": "Optional human label for the mounted instance." },
                    "region": { "type": "string", "description": "Optional stable region id; injected as `data-st-region` on the mounted root. With `host`, names the host region to compose into (default `stage`). Standalone: defaults to `region-{instance_id}`." },
                    "host": { "type": "string", "description": "Optional host instance id (e.g. the workbench `inst-1`). When set, COMPOSE the function into that host's `region` IN PLACE (one DOM, one runtime) instead of spawning a standalone page. The host page's `@mcp-region` poll renders the guest bundle. Omit for a standalone preview page." },
                    "input": { "description": "JSON input exposed at the mounted page's relative input.json." },
                    "agent_control": { "type": "boolean", "description": "When true, this page is an agent-control interface: its otherwise-unknown mcp-actions route to the agent (delivered via st_await) instead of being rejected at the sink. Built-in actions (kit-*, st_env_open, ...) are unaffected. Defaults false." },
                    "contract": { "type": "object", "description": "Optional declared contract when mounting inline source." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "st_await",
            "Block until a mounted function emits its next event through the generic \
MCP signal sink, then return that event. Bounded under the MCP client timeout; \
returns {decided:false} when the user/page has not emitted yet.",
            json!({
                "type": "object",
                "properties": {
                    "instance_id": { "type": "string", "description": "Mounted instance id returned by st_mount." },
                    "timeout_secs": { "type": "integer", "description": "Max seconds to block (1-28, default 25)." }
                },
                "required": ["instance_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_inspect",
            "Inspect the live Function Environment. With no args, lists registered \
functions and mounted instances. With fn_id/name or instance_id, returns contract, \
compile diagnostics, input, event history, and await cursor.",
            json!({
                "type": "object",
                "properties": {
                    "fn_id": { "type": "string", "description": "Function id to inspect." },
                    "name": { "type": "string", "description": "Function name to inspect." },
                    "instance_id": { "type": "string", "description": "Mounted instance id to inspect." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "st_unmount",
            "Unmount a live instance, removing it from the environment. If the \
instance was composed as a region guest of a host page (e.g. on the workbench \
stage), that host region is cleared too and returns to its empty state. Errors \
if there is no such instance.",
            json!({
                "type": "object",
                "properties": {
                    "instance_id": { "type": "string", "description": "Instance id to unmount (from st_mount / st_inspect)." }
                },
                "required": ["instance_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_env_close",
            "Close an opened environment, dropping it and every tab bound to it. \
Refuses (with a clear message) while a frame from this environment is still \
mounted — unmount it first.",
            json!({
                "type": "object",
                "properties": {
                    "env": { "type": "string", "description": "Environment id to close (from st_env_open / st_env_list)." }
                },
                "required": ["env"],
                "additionalProperties": false
            }),
        ),
        tool(
            "st_fn_delete",
            "Delete a registered function from the environment. Refuses (with a \
clear message) while the function is still mounted — as a standalone instance or \
composed into a host region. Unmount first.",
            json!({
                "type": "object",
                "properties": {
                    "fn_id": { "type": "string", "description": "Function id to delete (mutually exclusive with name)." },
                    "name": { "type": "string", "description": "Function name to delete (mutually exclusive with fn_id)." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "st_elicit",
            "Ask the human a question DIRECTLY in their client (no browser) via MCP \
elicitation: a native prompt with a typed form. Returns {action:accept|decline|\
cancel, content}. Use this for quick structured input (a name, a choice, a \
toggle). If the client doesn't support elicitation, the result says so. The \
schema is a flat object of primitive \
fields (string/number/integer/boolean, optionally enum).",
            json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "The question/prompt shown to the human." },
                    "schema": {
                        "type": "object",
                        "description": "A restricted JSON Schema: { type:'object', properties:{ field:{type,title?,description?,enum?,default?} }, required?:[] }. Primitives only."
                    }
                },
                "required": ["message", "schema"],
                "additionalProperties": false
            }),
        ),
    ]
}

/// Dispatch a `tools/call`. Unknown tool names return an `isError` result
/// (per MCP, tool-level failures are results, not JSON-RPC errors).
pub fn call(name: &str, args: &Value, state: &mut McpState) -> Value {
    let result = match name {
        "st_health" => st_health(state),
        "st_env_list" => st_env_list(state),
        "st_env_open" => st_env_open(args, state),
        "st_tab_list" => st_tab_list(args, state),
        "st_tab_open" => st_tab_open(args, state),
        "st_workbench" => st_workbench(args, state),
        "st_comments_types" => st_comments_types(args, state),
        "st_comments_list" => st_comments_list(args, state),
        "st_comments_add" => st_comments_add(args, state),
        "st_comments_update" => st_comments_update(args, state),
        "st_fn_put" => st_fn_put(args, state),
        "st_mount" => st_mount(args, state),
        "st_await" => st_await(args, state),
        "st_inspect" => st_inspect(args, state),
        "st_unmount" => st_unmount(args, state),
        "st_env_close" => st_env_close(args, state),
        "st_fn_delete" => st_fn_delete(args, state),
        other => Err(format!("Unknown tool: {other}")),
    };
    match result {
        Ok((summary, data)) => ok_result(summary, data),
        Err(msg) => err_result(msg),
    }
}

// ---------------------------------------------------------------------------
// Tool implementations. Each returns Ok((human_summary, structured_value)).
// ---------------------------------------------------------------------------

fn st_health(state: &McpState) -> Result<(String, Value), String> {
    let data = json!({
        "ok": true,
        "server": "spacetime",
        "version": env!("CARGO_PKG_VERSION"),
        "workspace_root": state.workspace_root.display().to_string(),
        "environments_open": state.environments().len(),
    });
    Ok((
        format!(
            "Spacetime MCP v{} healthy. Workspace: {}",
            env!("CARGO_PKG_VERSION"),
            state.workspace_root.display()
        ),
        data,
    ))
}

fn st_env_list(state: &mut McpState) -> Result<(String, Value), String> {
    let discovered = state.discovered_environments_json();
    let open = state.open_environments_json();

    let data = json!({ "discovered": discovered, "open": open });
    let summary = format!(
        "{} environment(s) discovered, {} open.",
        data["discovered"].as_array().map(|a| a.len()).unwrap_or(0),
        open.len()
    );
    Ok((summary, data))
}

fn st_env_open(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let path = non_empty_arg(args, "path").ok_or("missing `path`")?;

    let root = resolve_under_root(&state.workspace_root, path);
    if !root.is_dir() {
        return Err(format!(
            "not a directory: {} (resolved from {path})",
            root.display()
        ));
    }
    let root = root.canonicalize().unwrap_or(root);
    let title = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    let env = state.open_environment(root, title);
    let data = json!({
        "id": env.id,
        "root": env.root.display().to_string(),
        "title": env.title,
    });
    Ok((format!("Environment {} → {}", env.id, env.title), data))
}

fn comment_root(args: &Value, state: &McpState, required: bool) -> Result<PathBuf, String> {
    match non_empty_arg(args, "env") {
        Some(env_id) => state
            .environment(env_id)
            .map(|env| env.root)
            .ok_or_else(|| format!("no such environment: {env_id}; call st_env_open first")),
        None if required => Err("missing `env`; call st_env_open first".to_string()),
        None => Ok(state.workspace_root.clone()),
    }
}

fn comment_roster_json(state: &mut McpState, root: &Path) -> Result<Vec<Value>, String> {
    let registry = state.comment_registry(root)?;
    Ok(registry
        .comment_types()
        .map(|comment_type| serde_json::to_value(comment_type).expect("comment type serializes"))
        .collect())
}

/// Scan each source once, retaining parser-provided markup exclusions so visible
/// page text can never become a private comment through this MCP reader.
fn scan_environment_comments(
    root: &Path,
) -> (
    Vec<crate::comments::ScannedComment>,
    Vec<crate::comments::CommentDiagnostic>,
    Vec<String>,
) {
    fn visit(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    visit(root, &path, files);
                }
            } else if crate::comments::is_scannable_source(&path)
                && path != root.join(crate::compiler::PROJECT_PRELUDE)
            {
                // `.st` AND `.st.md`: a literate document's fences ARE the
                // program, so a comment written there is as real as one in a
                // plain source file. Skipping it made comments visible to
                // `check` and invisible to agents — one store, two answers.
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort();
    let mut scanned = Vec::new();
    let mut diagnostics = Vec::new();
    let mut scanned_files = Vec::new();
    for path in files {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let file = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        // The SHARED scan — identical to the dev server's, so an agent and a
        // human never disagree about what is a comment.
        let (found, warnings) = crate::comments::scan_file(&file, &source);
        scanned_files.push(file);
        scanned.extend(found);
        diagnostics.extend(warnings);
    }
    (scanned, diagnostics, scanned_files)
}

fn comment_index(
    root: &Path,
    state: &mut McpState,
) -> Result<crate::comments::CommentIndex, String> {
    let roster: BTreeMap<_, _> = state
        .comment_registry(root)?
        .comment_types()
        .map(|comment_type| (comment_type.id.clone(), comment_type.clone()))
        .collect();
    let (scanned, mut diagnostics, scanned_files) = scan_environment_comments(root);
    for comment in &scanned {
        diagnostics.extend(crate::comments::validate_against_roster(comment, &roster));
    }
    let (sidecar, sidecar_diagnostics) = crate::comments::read_sidecar(root);
    diagnostics.extend(sidecar_diagnostics);
    let mut index = crate::comments::merge(scanned, sidecar, &scanned_files, &comment_now());
    index.diagnostics.splice(0..0, diagnostics);
    Ok(index)
}

fn st_comments_types(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let root = comment_root(args, state, false)?;
    let types = comment_roster_json(state, &root)?;
    Ok((
        format!("{} comment type(s) in {}.", types.len(), root.display()),
        json!({ "types": types }),
    ))
}

/// Builds the canonical MCP comment index without rebuilding the type roster.
///
/// Resources and tools share this projection so attaching comments to context
/// cannot silently fork from an explicit list request.
pub(crate) fn comment_index_json(root: &Path, state: &mut McpState) -> Result<Value, String> {
    let hints: BTreeMap<String, Option<String>> = state
        .comment_registry(root)?
        .comment_types()
        .map(|comment_type| (comment_type.id.clone(), comment_type.agent_hint.clone()))
        .collect();
    let index = comment_index(root, state)?;
    let records = index
        .records
        .into_iter()
        .map(|record| {
            // The hint travels BESIDE the record, never inside it. Keeping the
            // canonical record unchanged lets disk, routes, tools, and
            // resources remain interchangeable transports.
            let hint = hints.get(&record.type_id).cloned().flatten();
            json!({ "comment": record, "agent_hint": hint })
        })
        .collect::<Vec<_>>();
    // Orphans are rows too. Emitting them bare while records are wrapped would
    // mean one response carrying two shapes for the same thing — a consumer
    // reading `row.comment` gets `undefined` for exactly the records that need
    // a human decision, which is the worst set to lose.
    let orphans = index
        .orphans
        .into_iter()
        .map(|record| {
            let hint = hints.get(&record.type_id).cloned().flatten();
            json!({ "comment": record, "agent_hint": hint })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "records": records,
        "diagnostics": index.diagnostics,
        "orphans": orphans,
    }))
}

fn st_comments_list(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let root = comment_root(args, state, false)?;
    let file = non_empty_arg(args, "file");
    let status = match non_empty_arg(args, "status") {
        Some(value) => Some(
            crate::comments::Status::parse(value)
                .ok_or("invalid `status`; use open, in-progress, resolved, or wontfix")?,
        ),
        None => None,
    };
    let type_id = non_empty_arg(args, "type");
    let mut index = comment_index_json(&root, state)?;
    let matches = |row: &Value| {
        let comment = &row["comment"];
        file.is_none_or(|file| comment["anchor"]["file"] == file)
            && status.is_none_or(|status| comment["status"] == status.as_str())
            && type_id.is_none_or(|type_id| comment["type"] == type_id)
    };
    if let Some(records) = index["records"].as_array_mut() {
        records.retain(&matches);
    }
    // Orphans obey the SAME filters. Returning every orphan regardless of the
    // caller's filter would answer a narrow question with unrelated recovery
    // work — an agent asking about one file would be handed the whole
    // project's loose ends.
    if let Some(orphans) = index["orphans"].as_array_mut() {
        orphans.retain(&matches);
    }
    let shown = index["records"].as_array().map_or(0, Vec::len);
    Ok((
        format!("{} comment(s) in {}.", shown, root.display()),
        index,
    ))
}

fn st_comments_add(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let root = comment_root(args, state, true)?;
    let type_id = non_empty_arg(args, "type")
        .ok_or("missing `type`")?
        .to_string();
    let text = non_empty_arg(args, "text")
        .ok_or("missing `text`")?
        .to_string();
    let anchor: crate::comments::Anchor = serde_json::from_value(args.get("anchor").cloned().ok_or("missing `anchor`")?)
        .map_err(|error| format!("invalid `anchor`: {error}; use inline/file/page/element anchor shape from st_comments_add schema"))?;
    // An agent MAY report the element's content instead of a hash, exactly as
    // the pill's picker does. Minting server-side keeps ONE implementation of
    // the hash rule: a client that computed its own would silently orphan every
    // record it wrote the moment the two disagreed by a byte.
    let anchor = match args.get("element_content").cloned() {
        None | Some(Value::Null) => anchor,
        Some(content) => {
            let content: crate::comments::ElementCandidate = serde_json::from_value(content)
                .map_err(|error| {
                    format!("invalid `element_content`: {error}; expected tag/class/id_attr/text")
                })?;
            crate::comments::with_element_content(anchor, &content)
        }
    };
    // The SAME validator the dev-server route uses. An agent is no more
    // trusted than a browser here: an absolute or `..` path would let a
    // record claim a location outside the environment, and an inline anchor
    // minted with a fresh id could never match the source line it names (its
    // identity is a content hash), so the next scan would orphan it.
    if let Some(rejection) = crate::comments::anchor_rejection(&anchor) {
        return Err(rejection);
    }
    let meta: BTreeMap<String, String> = args
        .get("meta")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("invalid `meta`: {error}; values must be strings"))?
        .unwrap_or_default();
    let roster: BTreeMap<_, _> = state
        .comment_registry(&root)?
        .comment_types()
        .map(|comment_type| (comment_type.id.clone(), comment_type.clone()))
        .collect();
    let (file, line) = match &anchor {
        crate::comments::Anchor::Inline { file, line } => (file.clone(), *line),
        _ => (anchor.display(), 0),
    };
    let candidate = crate::comments::ScannedComment {
        id: String::new(),
        type_id: type_id.clone(),
        meta: meta.clone(),
        text: text.clone(),
        file,
        line,
        malformed_meta: Vec::new(),
    };
    let validation = crate::comments::validate_against_roster(&candidate, &roster);
    if !validation.is_empty() {
        return Err(validation
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>()
            .join("; "));
    }
    let now = comment_now();
    let record = crate::comments::CommentRecord {
        id: format!(
            "a{:x}-{:x}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            std::process::id()
        ),
        type_id,
        status: crate::comments::Status::Open,
        author: crate::comments::Author {
            kind: crate::comments::AuthorKind::Agent,
            name: non_empty_arg(args, "agent_name")
                .unwrap_or("agent")
                .to_string(),
        },
        claimed_by: None,
        anchor,
        text,
        meta,
        thread: Vec::new(),
        history: Vec::new(),
        created_at: now,
        updated_at: None,
        v: crate::comments::SCHEMA_VERSION,
        inline: false,
    };
    crate::comments::write_record(&root, &record)
        .map_err(|error| format!("could not write comment record: {error}"))?;
    Ok((
        format!("Created comment {}.", record.id),
        serde_json::to_value(record).expect("comment record serializes"),
    ))
}

fn st_comments_update(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let root = comment_root(args, state, true)?;
    let id = non_empty_arg(args, "id").ok_or("missing `id`")?;
    let known = comment_index(&root, state)?.records;
    // Name the ids that DO exist. An agent cannot infer the current set from a
    // bare "unknown id", so the only recovery is to guess that another list
    // call is needed — the same reason the type errors name the roster.
    let mut record = known
        .iter()
        .find(|record| record.id == id)
        .cloned()
        .ok_or_else(|| {
            let mut ids: Vec<&str> = known.iter().map(|r| r.id.as_str()).take(10).collect();
            if known.len() > ids.len() {
                ids.push("…");
            }
            if ids.is_empty() {
                format!("unknown comment id `{id}`; this project has no comments yet")
            } else {
                format!(
                    "unknown comment id `{id}`; existing ids: {}. Call st_comments_list \
                     for the current set.",
                    ids.join(", ")
                )
            }
        })?;
    let status = match non_empty_arg(args, "status") {
        Some(value) => Some(
            crate::comments::Status::parse(value)
                .ok_or("invalid `status`; use open, in-progress, resolved, or wontfix")?,
        ),
        None => None,
    };
    let reply = non_empty_arg(args, "reply");
    if status.is_none() && reply.is_none() {
        return Err("provide `status`, `reply`, or both".to_string());
    }
    let now = comment_now();
    if let Some(status) = status {
        // Same accountability rule as the dev-server route: a transition records
        // WHO moved it and from where, and the caller's name is preserved rather
        // than flattened to "agent" — losing the identity here would make the
        // history less trustworthy than the thread beside it.
        //
        // A no-op re-submit is not a transition.
        if record.status != status {
            record.history.push(crate::comments::StatusChange {
                author: crate::comments::Author {
                    kind: crate::comments::AuthorKind::Agent,
                    name: non_empty_arg(args, "agent_name")
                        .unwrap_or("agent")
                        .to_string(),
                },
                from: record.status,
                to: status,
                at: now.clone(),
            });
            record.status = status;
        }
    }
    if let Some(text) = reply {
        record.thread.push(crate::comments::Reply {
            author: crate::comments::Author {
                kind: crate::comments::AuthorKind::Agent,
                // The caller's name, exactly as `add` records it. Hard-coding
                // "agent" here let a named caller create a correctly
                // attributed record and then lose that identity in its own
                // thread — which is the distinction the hosted tier's actor
                // model is built on.
                name: non_empty_arg(args, "agent_name")
                    .unwrap_or("agent")
                    .to_string(),
            },
            text: text.to_string(),
            at: now.clone(),
        });
    }
    record.updated_at = Some(now);
    crate::comments::write_record(&root, &record)
        .map_err(|error| format!("could not write comment record: {error}"))?;
    Ok((
        format!("Updated comment {}.", record.id),
        serde_json::to_value(record).expect("comment record serializes"),
    ))
}

fn comment_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn st_tab_list(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let env_id = non_empty_arg(args, "env").ok_or("missing `env`")?;
    if state.environment(env_id).is_none() {
        return Err(format!("no such environment: {env_id}"));
    }
    let tabs = state.tabs_json(env_id);
    let summary = format!("{} tab(s) in {env_id}.", tabs.len());
    Ok((summary, json!({ "env": env_id, "tabs": tabs })))
}

/// PLAN-049 W2 (COLLAPSE): open a tab = REGISTER an env-origin function.
///
/// A tab is no longer a compile-and-die artifact. Opening one reads the entry
/// file (or inline `code`), registers it as an `Origin::Env(env_id)` FUNCTION in
/// the live registry (gallery-visible, stage-mountable like any function), and
/// records a Tab that carries that function's id. One object model: "tabs" are
/// the env-origin slice of the function registry, not a parallel registry. The
/// compile happens ONCE inside `put_function_from_args` (no separate
/// compile_file/compile_source pass — harmony: one need, one impl).
fn st_tab_open(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let env_id = non_empty_arg(args, "env")
        .ok_or("missing `env`")?
        .to_string();
    let env = state
        .environment(&env_id)
        .ok_or_else(|| format!("no such environment: {env_id}"))?;
    let env_root = env.root.clone();

    let entry = non_empty_arg(args, "entry");
    let code = non_empty_arg(args, "code");

    // Resolve the source text + a default title + the tab's source provenance.
    // `entry_path` carries the on-disk `.st` file (PLAN-066: passed through to
    // the compile step so it can detect a sibling `index.html` shell) — `None`
    // for inline agent-authored code, which has no on-disk sibling to check.
    let (source, title, tab_source, entry_path) = match (entry, code) {
        (Some(_), Some(_)) => return Err("provide `entry` OR `code`, not both".into()),
        (None, None) => return Err("provide `entry` (a file) or `code` (inline .st source)".into()),
        (Some(entry), None) => {
            let file = resolve_under_root(&env_root, entry);
            if !file.is_file() {
                return Err(format!("no such file: {}", file.display()));
            }
            let source = std::fs::read_to_string(&file)
                .map_err(|e| format!("could not read {}: {e}", file.display()))?;
            let title = args
                .get("title")
                .and_then(non_empty_str)
                .map(str::to_string)
                .unwrap_or_else(|| entry.to_string());
            (source, title, TabSource::File(file.clone()), Some(file))
        }
        (None, Some(code)) => {
            let title = args
                .get("title")
                .and_then(non_empty_str)
                .map(str::to_string)
                .unwrap_or_else(|| "inline".to_string());
            (
                code.to_string(),
                title,
                TabSource::Inline(code.to_string()),
                None,
            )
        }
    };

    // Register the source as an env-origin function (the COLLAPSE). The function
    // NAME is the tab title scoped by env so re-opening the same entry updates
    // the same function (a new revision) rather than spawning duplicates.
    let fn_name = format!("{env_id}/{title}");
    let contract = default_function_contract(Some(&fn_name));
    let origin = super::origin::Origin::Env(env_id.clone());
    let server = state.live()?;
    let outcome = server.put_source_with_entry(
        Some(fn_name),
        source,
        contract,
        env_root,
        origin,
        entry_path.as_deref(),
    );
    let record = outcome.record;
    let fn_id = record.id.clone();
    let compile_ok = record.compile_ok;

    let tab = state.open_tab(&env_id, title, tab_source, Some(fn_id.clone()));
    let tab_id = tab.id.clone();
    let tab_title = tab.title.clone();

    let data = json!({
        "id": tab_id,
        "env": env_id,
        "title": tab_title,
        "function_id": fn_id,
        "compile": {
            "ok": compile_ok,
            "diagnostics": record.diagnostics.iter().map(super::live::OptionDiagnostic::to_json).collect::<Vec<_>>(),
        }
    });
    let summary = if compile_ok {
        // PLAN-066: a clean compile can still carry WARNING diagnostics
        // (MCP-LOCALE / MCP-SHELL) — surface them in the summary too, not just
        // the `data.compile.diagnostics` field, so an agent driving st_tab_open
        // sees the composed preview may be incomplete WITHOUT a second
        // st_inspect round-trip.
        if record.diagnostics.is_empty() {
            format!("Tab {tab_id} ({tab_title}) registered as {fn_id} — composable onto the stage.")
        } else {
            let diags: Vec<Value> = record
                .diagnostics
                .iter()
                .map(super::live::OptionDiagnostic::to_json)
                .collect();
            format!(
                "Tab {tab_id} ({tab_title}) registered as {fn_id} — composable onto the stage, with {} warning(s):{}",
                diags.len(),
                format_record_diagnostics(&diags)
            )
        }
    } else {
        let diags: Vec<Value> = record
            .diagnostics
            .iter()
            .map(super::live::OptionDiagnostic::to_json)
            .collect();
        format!(
            "Tab {tab_id} ({tab_title}) registered as {fn_id} with {} diagnostic(s) — NOT composable until fixed:{}",
            diags.len(),
            format_record_diagnostics(&diags)
        )
    };
    Ok((summary, data))
}

fn st_workbench(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    // FUP-071: read the workbench shell from disk when present (dev overlay),
    // so editing env.st is live on the next mount with no rebuild/restart;
    // falls back to the embedded bytes in production.
    let source =
        super::stdlib_source::resolve_mcp_stdlib_source(&state.workspace_root, "env.st")
            .ok_or_else(|| "workbench source env.st not found (embedded or on disk)".to_string())?;
    let source = source.as_ref();
    let put_args = json!({
        "source": source,
        "name": "mcp-workbench",
        "contract": {
            "version": 1,
            "name": "mcp-workbench",
            "input": { "kind": "mcp-workbench-snapshot" },
            "events": [
                { "type": "mcp-action", "payload": { "kind": "json" } }
            ],
            "source": "stdlib/__mcp__/env.st"
        }
    });
    let function = put_function_from_args(&put_args, state)?.record;
    let input = resources::workbench_snapshot(state);
    let title = args
        .get("title")
        .and_then(non_empty_str)
        .map(str::to_string)
        .unwrap_or_else(|| "Spacetime MCP Workbench".to_string());
    let server = state.live()?;
    // The workbench emits only built-in server actions (nav/inspection); it is
    // not an agent-control surface.
    let instance = server.mount_function(&function.id, Some(title), input, None, false)?;
    let instance_id = instance.id.clone();
    let url = instance.url.clone();
    let events_resource = resources::events_uri(&instance_id);
    let events_read_url = format!("mcp://{}", events_resource);
    let data = json!({
        "instance": instance.to_json(),
        "function": function.to_json(),
        "instance_id": instance_id,
        "url": url,
        "events_resource": events_resource,
        "events_read_url": events_read_url,
        "snapshot_schema": 1,
    });
    Ok((
        format!("Mounted MCP workbench as {}: {}", instance.id, instance.url),
        data,
    ))
}

// ---------------------------------------------------------------------------
// Generic Function Environment (FEAT-122)
// ---------------------------------------------------------------------------

fn st_fn_put(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let outcome = put_function_from_args(args, state)?;
    let record = outcome.record;
    let compat = outcome.compatibility;
    let mut data = record.to_json();
    if let Some(obj) = data.as_object_mut() {
        obj.insert("compatibility".to_string(), json!(compat.label()));
    }
    let summary = if record.compile_ok {
        let mut s = format!(
            "Function {} revision {} compiled and registered.",
            record.id, record.revision
        );
        // FEAT-124: report how this revision relates to the prior one.
        match compat {
            super::live::Compatibility::Compatible => {
                s.push_str(
                    " Contract unchanged (compatible) -- mounted instances hot-swap in place.",
                );
            }
            super::live::Compatibility::Breaking => {
                s.push_str(" CONTRACT CHANGED (breaking) -- mounts may need remount/migration.");
            }
            super::live::Compatibility::Initial => {}
        }
        // Surface non-fatal diagnostics (warnings) even on success (FEAT-133).
        if let Some(diags) = data
            .get("compile")
            .and_then(|c| c.get("diagnostics"))
            .and_then(Value::as_array)
            && !diags.is_empty()
        {
            s.push_str(&format!(" {} diagnostic(s):", diags.len()));
            s.push_str(&format_record_diagnostics(diags));
        }
        s.push_str(&format_contract_params(&data["contract_params"]));
        s
    } else {
        let diags: Vec<Value> = record
            .diagnostics
            .iter()
            .map(super::live::OptionDiagnostic::to_json)
            .collect();
        format!(
            "Function {} revision {} registered with {} diagnostic(s) -- NOT mountable until fixed:{}",
            record.id,
            record.revision,
            record.diagnostics.len(),
            format_record_diagnostics(&diags)
        )
    };
    Ok((summary, data))
}

fn st_mount(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let function_key = if non_empty_arg(args, "source").is_some() {
        put_function_from_args(args, state)?.record.id
    } else if let Some(id) = non_empty_arg(args, "fn_id") {
        id.to_string()
    } else if let Some(name) = non_empty_arg(args, "name") {
        name.to_string()
    } else {
        return Err("provide `fn_id`, `name`, or inline `source`".into());
    };

    let title = args
        .get("title")
        .and_then(non_empty_str)
        .map(str::to_string);
    let input = args.get("input").cloned().unwrap_or_else(|| json!({}));
    let region = args
        .get("region")
        .and_then(non_empty_str)
        .map(str::to_string);
    // A region id flows into a CSS `@scope` selector and a `querySelector` on the
    // page; constrain it to a safe token so a crafted value cannot break out of
    // `[data-st-region="..."]` and inject unscoped CSS (mirrors the runtime guard
    // in registerBundle / isSafeRegionId).
    if let Some(r) = region.as_deref()
        && !r
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "invalid `region` id {r:?}: only [A-Za-z0-9_-] are allowed"
        ));
    }
    // PLAN-045: an agent-control page routes its OWN (otherwise-unknown)
    // mcp-actions to the agent instead of having them rejected. Built-in
    // registry actions are unaffected. Defaults false.
    let agent_control = args
        .get("agent_control")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // FEAT-126 / BUG-111: when `host` is given, COMPOSE the function into that
    // host instance's region instead of spawning a standalone page. `region`
    // names the target region (default "stage"). The host page's `@mcp-region`
    // poll picks up the guest bundle and renders it in place.
    if let Some(host_id) = non_empty_arg(args, "host") {
        let host_id = host_id.to_string();
        let region_name = region.clone().unwrap_or_else(|| "stage".to_string());
        let server = state.live()?;
        let host = server.mount_into_host(&host_id, &region_name, &function_key, &input)?;
        let function = server
            .get_function(&function_key)
            .ok_or_else(|| format!("mounted function vanished: {function_key}"))?;
        let host_url = host.url.clone();
        let data = json!({
            "host": host.to_json(),
            "function": function.to_json(),
            "host_id": host_id,
            "region": region_name,
            "composed": true,
            "url": host_url,
            "contract": function.contract,
        });
        return Ok((
            format!(
                "Composed {} r{} into {} region '{}' -> {}",
                function.id, function.revision, host_id, region_name, host.url
            ),
            data,
        ));
    }

    let server = state.live()?;
    let instance = server.mount_function(&function_key, title, input, region, agent_control)?;
    let function = server
        .get_function(&instance.function_id)
        .ok_or_else(|| format!("mounted function vanished: {}", instance.function_id))?;
    let events_resource = resources::events_uri(&instance.id);
    let events_read_url = format!("mcp://{}", events_resource);
    let data = json!({
        "instance": instance.to_json(),
        "function": function.to_json(),
        "instance_id": instance.id,
        "region_id": instance.region_id,
        "url": instance.url,
        "events_resource": events_resource,
        "events_read_url": events_read_url,
        "contract": function.contract,
    });
    Ok((
        format!(
            "Mounted {} r{} as {}: {}",
            function.id, function.revision, instance.id, instance.url
        ),
        data,
    ))
}

fn st_await(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let instance_id = non_empty_arg(args, "instance_id")
        .ok_or("missing `instance_id`")?
        .to_string();
    let secs = args
        .get("timeout_secs")
        .and_then(Value::as_u64)
        .unwrap_or(25)
        .clamp(1, 28);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let awaited = {
        let server = state.live()?;
        if server.get_instance(&instance_id).is_none() {
            return Err(format!("no such instance: {instance_id}"));
        }
        server.await_event(&instance_id, deadline)
    };

    match awaited {
        Some(event) => {
            // PLAN-045: only AGENT-audience events reach st_await, and the server
            // never resolves those — so `action` carries any sink-cached verdict
            // (present only for the rare agent-control case the sink touched) or
            // Null, and the agent reads `event.payload` directly. Server actions
            // are resolved at the sink and never delivered here, so there is no
            // second dispatcher: the sink (`dispatch_signal_action`) is the ONE
            // place a server action runs.
            let action = event.raw.get("_mcp_action").cloned().unwrap_or(Value::Null);
            Ok((
                format!(
                    "Instance {} emitted {}#{}.",
                    event.instance_id, event.event_type, event.seq
                ),
                json!({
                    "instance_id": instance_id,
                    "decided": true,
                    "event": event.to_json(),
                    "action": action,
                }),
            ))
        }
        None => Ok((
            format!(
                "No event emitted by {instance_id} after {secs}s. Call st_await again to keep waiting."
            ),
            json!({
                "instance_id": instance_id,
                "decided": false,
                "event": Value::Null,
                "action": Value::Null,
            }),
        )),
    }
}

fn st_inspect(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let server = state.live()?;
    if let Some(instance_id) = non_empty_arg(args, "instance_id") {
        let instance = server
            .get_instance(instance_id)
            .ok_or_else(|| format!("no such instance: {instance_id}"))?;
        let function = server.get_function(&instance.function_id);
        let inst_json = instance.to_json();
        let fn_json = function.map(|f| f.to_json());
        let mut s = format!(
            "Instance {} ({}) -- fn {} r{}, region {}, {} event(s).",
            instance.id,
            instance.title,
            instance.function_id,
            instance.function_revision,
            instance.region_id,
            instance.events.len()
        );
        if let Some(fj) = &fn_json {
            s.push_str(&format_contract_params(&fj["contract_params"]));
        }
        return Ok((s, json!({ "instance": inst_json, "function": fn_json })));
    }
    if let Some(key) = args
        .get("fn_id")
        .and_then(non_empty_str)
        .or_else(|| args.get("name").and_then(non_empty_str))
    {
        let function = server
            .get_function(key)
            .ok_or_else(|| format!("no such function: {key}"))?;
        let data = function.to_json();
        let compile_ok = data["compile"]["ok"].as_bool().unwrap_or(false);
        let mut s = format!(
            "Function {} revision {} ({}) -- {}.",
            function.id,
            function.revision,
            data["origin"]["label"].as_str().unwrap_or("?"),
            if compile_ok {
                "compiles"
            } else {
                "DOES NOT COMPILE"
            }
        );
        if let Some(hash) = data["contract_hash"].as_str()
            && !hash.is_empty()
        {
            s.push_str(&format!(" contract-hash {hash}."));
        }
        if let Some(diags) = data["compile"]["diagnostics"].as_array()
            && !diags.is_empty()
        {
            s.push_str(&format!(" {} diagnostic(s):", diags.len()));
            s.push_str(&format_record_diagnostics(diags));
        }
        s.push_str(&format_contract_params(&data["contract_params"]));
        return Ok((s, json!({ "function": data })));
    }

    let functions = server
        .functions()
        .into_iter()
        .map(|f| f.to_json())
        .collect::<Vec<_>>();
    let instances = server
        .instances()
        .into_iter()
        .map(|i| i.to_json())
        .collect::<Vec<_>>();
    let mut s = format!(
        "{} function(s), {} mounted instance(s).",
        functions.len(),
        instances.len()
    );
    for f in &functions {
        let ok = f["compile"]["ok"].as_bool().unwrap_or(false);
        let name = f["name"].as_str().unwrap_or("");
        let name_part = if name.is_empty() {
            String::new()
        } else {
            format!(" {name}")
        };
        s.push_str(&format!(
            "\n  fn {}{} r{} [{}]",
            f["id"].as_str().unwrap_or("?"),
            name_part,
            f["revision"].as_u64().unwrap_or(0),
            if ok { "ok" } else { "ERR" }
        ));
    }
    for i in &instances {
        s.push_str(&format!(
            "\n  inst {} -> fn {} (region {})",
            i["id"].as_str().unwrap_or("?"),
            i["function_id"].as_str().unwrap_or("?"),
            i["region_id"].as_str().unwrap_or("?")
        ));
    }
    Ok((s, json!({ "functions": functions, "instances": instances })))
}

/// PLAN-049 W1.S1: unmount a live instance (the tool twin of the `mcp_unmount`
/// browser action). Removes the instance + clears any host region it occupied.
fn st_unmount(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let instance_id = non_empty_arg(args, "instance_id").ok_or("missing `instance_id`")?;
    let server = state.live()?;
    let (id, title) = server.unmount_instance(instance_id)?;
    Ok((
        format!("Unmounted {id} ({title})."),
        json!({ "instance_id": id, "title": title }),
    ))
}

/// PLAN-049 W1.S3: close an environment (tool twin of `mcp_env_close`). Drops
/// the env + its tabs; refuses while a frame from it is mounted.
fn st_env_close(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    let env_id = non_empty_arg(args, "env")
        .ok_or("missing `env`")?
        .to_string();
    let server = state.live()?;
    let (title, tabs) = server.close_environment(&env_id)?;
    Ok((
        format!("Closed environment {env_id} ({title}); dropped {tabs} tab(s)."),
        json!({ "env": env_id, "title": title, "tabs_dropped": tabs }),
    ))
}

/// PLAN-049 W1.S3: delete a function (tool twin of `mcp_fn_delete`). Refuses
/// while the function is still mounted.
fn st_fn_delete(args: &Value, state: &mut McpState) -> Result<(String, Value), String> {
    // Destructive: require EXACTLY ONE identifier. Accepting both and silently
    // preferring fn_id could delete the wrong function on a desynced call
    // (reviewer W1 P2). The schema declares them mutually exclusive; enforce it.
    let fn_id = non_empty_arg(args, "fn_id");
    let name = non_empty_arg(args, "name");
    let key = match (fn_id, name) {
        (Some(_), Some(_)) => {
            return Err("provide `fn_id` OR `name`, not both".into());
        }
        (Some(id), None) => id.to_string(),
        (None, Some(n)) => n.to_string(),
        (None, None) => return Err("provide `fn_id` or `name`".into()),
    };
    let server = state.live()?;
    let (id, name) = server.delete_function(&key)?;
    Ok((
        format!(
            "Deleted function {id}{}.",
            name.as_deref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default()
        ),
        json!({ "function_id": id, "name": name }),
    ))
}

/// Derive a function's [`Origin`](super::origin::Origin) from `st_fn_put`/`st_mount`
/// args (FEAT-128). Precedence: `env` binding → Env; else a contract `source`
/// rooted under `stdlib/` → Stdlib (the MCP's own bundled functions); else Agent
/// (inline agent source). Pure on the args so it is unit-testable without a server.
fn origin_from_args(args: &Value) -> super::origin::Origin {
    if let Some(env_id) = non_empty_arg(args, "env") {
        return super::origin::Origin::Env(env_id.to_string());
    }
    let contract_source = args
        .get("contract")
        .and_then(|c| c.get("source"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if contract_source.starts_with("stdlib/") {
        super::origin::Origin::Stdlib
    } else {
        super::origin::Origin::Agent
    }
}

// PLAN-049 W2: the contract-param flattener moved to the canonical home
// `super::live::contract_params_from_bundle` (shared by put_source + tests).

fn put_function_from_args(
    args: &Value,
    state: &mut McpState,
) -> Result<super::live::PutOutcome, String> {
    let source = non_empty_arg(args, "source")
        .ok_or("missing `source`")?
        .to_string();
    let name = args.get("name").and_then(non_empty_str).map(str::to_string);
    let contract = args
        .get("contract")
        .cloned()
        .unwrap_or_else(|| default_function_contract(name.as_deref()));
    let resolve_root = resolve_root_from_args(args, state)?;
    // Provenance (FEAT-128): origin precedence is
    //   1. an `env` binding → Env(env_id);
    //   2. else a contract `source` rooted under `stdlib/` → Stdlib (the MCP's
    //      own bundled functions, e.g. the workbench `stdlib/__mcp__/env.st`,
    //      register through here with that declared source);
    //   3. else inline source sent by the agent → Agent.
    let origin = origin_from_args(args);

    // PLAN-049 W2: delegate to the shared `LiveServer::put_source`, the ONE
    // compile+register impl used by BOTH this tool path and the st_tab_open sink
    // path (no parallel compile/registry — AGENTS harmony). It compiles EXACTLY
    // ONCE (FUP-066): page `ok` + the bundle's `has_errors` derive from the same
    // result and cannot diverge; clean compiles surface contract param rows,
    // failing ones surface diagnostics + no params (FEAT-132).
    let server = state.live()?;
    Ok(server.put_source(name, source, contract, resolve_root, origin))
}

fn non_empty_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(non_empty_str)
}

fn non_empty_str(value: &Value) -> Option<&str> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty())
}

fn default_function_contract(name: Option<&str>) -> Value {
    json!({
        "version": 1,
        "name": name.unwrap_or("anonymous"),
        "input": { "kind": "json" },
        "events": [
            { "type": "message", "payload": { "kind": "json" } }
        ],
        "source": "declared-fallback",
    })
}

fn resolve_root_from_args(args: &Value, state: &McpState) -> Result<std::path::PathBuf, String> {
    if let Some(env_id) = non_empty_arg(args, "env") {
        return state
            .environment(env_id)
            .map(|e| e.root.clone())
            .ok_or_else(|| format!("no such environment: {env_id}"));
    }
    Ok(state.workspace_root.clone())
}

// PLAN-049 W2: the page/compile-summary projections (OptionOutcome,
// CompileSummary, compile_file/compile_source/summarize) were the old
// st_tab_open compile-and-die path. The COLLAPSE routes every source through
// `LiveServer::put_source` (the one compile+register impl), so these are gone
// (harmony: one need, one impl). Diagnostics now flow via OptionDiagnostic on
// the FunctionRecord, rendered by `format_record_diagnostics`.

// ---------------------------------------------------------------------------
// Result + path helpers
// ---------------------------------------------------------------------------

/// Resolve a user-supplied path against a root. Absolute paths pass through.
fn resolve_under_root(root: &std::path::Path, path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

/// Render OptionDiagnostic JSON (shape {severity,code,message}) as readable
/// lines for a tool summary (FEAT-133). The function-record diagnostics path
/// (st_fn_put / st_tab_open) consumed by the tool `content` text.
fn format_record_diagnostics(diagnostics: &[Value]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        let severity = d.get("severity").and_then(Value::as_str).unwrap_or("error");
        let code = d.get("code").and_then(Value::as_str).unwrap_or("");
        let message = d.get("message").and_then(Value::as_str).unwrap_or("");
        let glyph = if severity == "warning" { "!" } else { "x" };
        let code_part = if code.is_empty() {
            String::new()
        } else {
            format!("[{code}] ")
        };
        out.push_str(&format!("\n  {glyph} {code_part}{message}"));
    }
    out
}

/// Render a function's contract_params (flat list of {template,name,kind,
/// optional,type,default}) as readable lines for st_inspect (FEAT-133). The
/// inspector drawer shows these visually; the tool text mirrors them so an agent
/// driving the live loop sees the contract without parsing JSON.
fn format_contract_params(params: &Value) -> String {
    let Some(arr) = params.as_array() else {
        return String::new();
    };
    if arr.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n  contract:");
    for p in arr {
        let name = p.get("name").and_then(Value::as_str).unwrap_or("?");
        let template = p.get("template").and_then(Value::as_str).unwrap_or("");
        let kind = p.get("kind").and_then(Value::as_str).unwrap_or("");
        let optional = p.get("optional").and_then(Value::as_bool).unwrap_or(false);
        let ty = p.get("type").and_then(Value::as_str);
        let opt_mark = if optional { "?" } else { "" };
        let ty_part = ty.map(|t| format!(" {t}")).unwrap_or_default();
        let tpl_part = if template.is_empty() {
            String::new()
        } else {
            format!(" ({template})")
        };
        out.push_str(&format!(
            "\n    - {name}{opt_mark}{ty_part} [{kind}]{tpl_part}"
        ));
    }
    out
}
fn ok_result(summary: String, structured: Value) -> Value {
    json!({
        "content": [ { "type": "text", "text": summary } ],
        "structuredContent": structured,
    })
}

fn err_result(message: String) -> Value {
    json!({
        "content": [ { "type": "text", "text": format!("error: {message}") } ],
        "structuredContent": { "ok": false, "error": message },
        "isError": true,
    })
}

#[cfg(test)]
mod origin_from_args_tests {
    use super::super::origin::Origin;
    use super::*;

    #[test]
    fn env_binding_takes_precedence() {
        let args = json!({ "source": "x", "env": "env-2" });
        assert_eq!(origin_from_args(&args), Origin::Env("env-2".to_string()));
    }

    #[test]
    fn stdlib_contract_source_is_stdlib_origin() {
        // The workbench (stdlib/__mcp__/env.st) registers with this contract.source
        // and NO env — it must be Stdlib, not Agent (FEAT-128 reviewer finding).
        let args = json!({
            "source": "...",
            "contract": { "source": "stdlib/__mcp__/env.st" }
        });
        assert_eq!(origin_from_args(&args), Origin::Stdlib);
    }

    #[test]
    fn inline_agent_source_is_agent_origin() {
        let args = json!({ "source": "@page \"/\" {}" });
        assert_eq!(origin_from_args(&args), Origin::Agent);
        // A non-stdlib contract source is still agent.
        let args2 = json!({ "source": "x", "contract": { "source": "projects/foo/index.st" } });
        assert_eq!(origin_from_args(&args2), Origin::Agent);
    }
}

#[cfg(test)]
mod fn_delete_contract_tests {
    use super::*;
    use std::path::PathBuf;

    // PLAN-049 W1.S3 (reviewer W1 P2): st_fn_delete is destructive — supplying
    // BOTH fn_id and name must be rejected (not silently prefer one). The check
    // runs before `state.live()`, so a fresh state exercises it without a server.
    #[test]
    fn both_identifiers_rejected_before_touching_server() {
        let mut state = McpState::new(PathBuf::from("/tmp"));
        let err =
            st_fn_delete(&json!({ "fn_id": "fn-1", "name": "other" }), &mut state).unwrap_err();
        assert!(err.contains("not both"), "clear conflict error: {err}");
    }

    #[test]
    fn neither_identifier_rejected() {
        let mut state = McpState::new(PathBuf::from("/tmp"));
        let err = st_fn_delete(&json!({}), &mut state).unwrap_err();
        assert!(err.contains("provide"), "clear missing-arg error: {err}");
    }
}

#[cfg(test)]
mod contract_params_tests {
    #[test]
    fn flattens_template_param_specs() {
        // FEAT-132 v1: a function with two templates and several param kinds
        // flattens to one inspector row per param, tagged with its template.
        let src = "\
@template &card($title, &slot) {
  <article class=\"card\"><h2>`$title`</h2></article>
}
@template &btn($label) {
  <button>`$label`</button>
}
";
        let bundle = super::super::bundle::compile_to_bundle(src, std::path::Path::new("."))
            .expect("bundle");
        let rows = super::super::live::contract_params_from_bundle(&bundle);
        let arr = rows.as_array().expect("array");
        // card: $title (binding) + &slot (element); btn: $label (binding) = 3 rows.
        assert_eq!(arr.len(), 3, "three param rows: {arr:?}");
        let names: Vec<&str> = arr.iter().filter_map(|r| r["name"].as_str()).collect();
        assert!(names.contains(&"title"));
        assert!(names.contains(&"slot"));
        assert!(names.contains(&"label"));
        // Each row carries its owning template + a machine kind token.
        let title_row = arr.iter().find(|r| r["name"] == "title").unwrap();
        assert_eq!(title_row["template"], "card");
        assert_eq!(title_row["kind"], "binding");
        let slot_row = arr.iter().find(|r| r["name"] == "slot").unwrap();
        assert_eq!(slot_row["kind"], "element");
    }

    #[test]
    fn no_params_yields_empty_contract() {
        let src = "@template &x() { <div></div> }";
        let bundle = super::super::bundle::compile_to_bundle(src, std::path::Path::new("."))
            .expect("bundle");
        assert_eq!(
            super::super::live::contract_params_from_bundle(&bundle)
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}

#[cfg(test)]
mod contract_gate_tests {
    use super::*;

    #[test]
    fn broken_bundle_surfaces_no_contract_params() {
        // A valid-AST template with an undeclared body hole ($broken → E0900)
        // compiles to Ok(bundle) with has_errors=true. The inspector must NOT
        // show params for an unmountable function (W4 reviewer P2): the put-time
        // gate yields an empty contract in that case.
        let src = "\
@template &card($t) {
  <div class=\"c\">`$t`</div>
  $broken
}
.s { &card(\"x\"); }
";
        let bundle = super::super::bundle::compile_to_bundle(src, std::path::Path::new("."))
            .expect("valid AST compiles to a bundle");
        assert!(bundle.has_errors, "undeclared $broken must set has_errors");
        // The gate in put_function_from_args: params only when !has_errors.
        let gated = if !bundle.has_errors {
            super::super::live::contract_params_from_bundle(&bundle)
        } else {
            json!([])
        };
        assert_eq!(
            gated.as_array().unwrap().len(),
            0,
            "broken bundle => empty contract"
        );
    }
}

#[cfg(test)]
mod single_compile_tests {
    use super::*;

    /// FUP-066: a clean source's page outcome (`from_compiled`) and its contract
    /// params derive from ONE `compile_function` call — they agree on success.
    #[test]
    fn clean_source_compiles_once_into_page_and_contract() {
        let src = "\
@template &card($title) {
  <article class=\"card\"><h2>`$title`</h2></article>
}
";
        let compiled = super::super::bundle::compile_function(src, std::path::Path::new("."), None)
            .expect("clean source compiles");
        assert!(!compiled.bundle.has_errors, "clean source has no errors");
        // Contract params present (one binding param) + page output present — both
        // from the SAME compile, the invariant put_source relies on.
        let params = super::super::live::contract_params_from_bundle(&compiled.bundle);
        assert_eq!(params.as_array().unwrap().len(), 1, "$title param");
        // A clean template-only source yields a non-empty bundle (the mountable
        // unit); page html is empty for template-only (no @page), as expected.
        assert!(
            !compiled.bundle.templates.is_empty(),
            "clean compile yields a template bundle"
        );
    }

    /// FUP-066: a semantic error makes BOTH projections agree on failure from one
    /// compile — empty contract AND a not-ok page outcome carrying the error.
    /// This is the divergence the single compile makes structurally impossible.
    #[test]
    fn semantic_error_unifies_page_and_contract_failure() {
        let src = "\
@template &card($t) {
  <div class=\"c\">`$t`</div>
  $broken
}
.s { &card(\"x\"); }
";
        let compiled = super::super::bundle::compile_function(src, std::path::Path::new("."), None)
            .expect("valid AST compiles");
        assert!(compiled.bundle.has_errors, "$broken sets has_errors");
        // has_errors ⟺ empty contract ⟺ not-mountable: the single invariant
        // put_source enforces (page status + contract derive from one compile).
        let params = if !compiled.bundle.has_errors {
            super::super::live::contract_params_from_bundle(&compiled.bundle)
        } else {
            json!([])
        };
        assert_eq!(
            params.as_array().unwrap().len(),
            0,
            "broken => empty contract"
        );
        assert!(
            compiled
                .bundle
                .diagnostics
                .iter()
                .any(|d| d.severity == "error"),
            "broken compile surfaces an error diagnostic"
        );
    }
}

#[cfg(test)]
mod feat133_format_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn record_diagnostics_distinguish_warning_from_error() {
        let diags = vec![
            json!({ "severity": "error", "code": "E1", "message": "boom" }),
            json!({ "severity": "warning", "code": "W1", "message": "careful" }),
        ];
        let out = format_record_diagnostics(&diags);
        assert!(out.contains("E1"), "has error code: {out}");
        assert!(out.contains("W1"), "has warning code: {out}");
        assert!(
            out.contains("boom") && out.contains("careful"),
            "both messages: {out}"
        );
    }

    #[test]
    fn contract_params_render_name_kind_and_optional() {
        let params = json!([
            { "template": "card", "name": "title", "kind": "binding", "optional": false, "type": null },
            { "template": "card", "name": "subtitle", "kind": "binding", "optional": true, "type": "string" }
        ]);
        let out = format_contract_params(&params);
        assert!(out.contains("contract:"), "has header: {out}");
        assert!(
            out.contains("title") && out.contains("subtitle"),
            "both params: {out}"
        );
        assert!(out.contains("subtitle?"), "optional marked with ?: {out}");
        assert!(out.contains("string"), "type shown: {out}");
        assert!(out.contains("(card)"), "template tag shown: {out}");
    }

    #[test]
    fn contract_params_empty_is_empty_string() {
        assert_eq!(format_contract_params(&json!([])), "");
        assert_eq!(format_contract_params(&json!(null)), "");
    }
}

#[cfg(test)]
mod comment_tool_tests {
    use super::*;

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "spacetime_mcp_comments_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn comment_tools_describe_and_round_trip_agent_work() {
        let descriptors = list();
        for name in [
            "st_comments_types",
            "st_comments_list",
            "st_comments_add",
            "st_comments_update",
        ] {
            let descriptor = descriptors
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("comment descriptor");
            assert_eq!(descriptor["inputSchema"]["type"], "object");
            assert_eq!(descriptor["inputSchema"]["additionalProperties"], false);
            assert!(
                descriptor["description"]
                    .as_str()
                    .is_some_and(|description| !description.is_empty())
            );
        }
        let add = descriptors
            .iter()
            .find(|tool| tool["name"] == "st_comments_add")
            .unwrap();
        assert_eq!(
            add["inputSchema"]["required"],
            json!(["env", "type", "anchor", "text"])
        );
        for property in ["env", "type", "anchor", "text", "meta", "agent_name"] {
            assert!(
                add["inputSchema"]["properties"].get(property).is_some(),
                "add schema missing {property}"
            );
        }
        let anchor_kinds = add["inputSchema"]["properties"]["anchor"]["oneOf"]
            .as_array()
            .expect("closed anchor union");
        assert!(anchor_kinds.iter().any(|shape| {
            shape["properties"]["kind"]["const"] == "element"
                && shape["required"] == json!(["kind", "route", "selector"])
        }));
        // An agent must be able to SEND a fingerprint. The union is
        // `additionalProperties: false`, so a missing property is not merely
        // undocumented — it is REJECTED, which would silently force every
        // agent-authored element anchor into the legacy unverified shape that
        // FUP-171 W-b exists to eliminate.
        let element = anchor_kinds
            .iter()
            .find(|shape| shape["properties"]["kind"]["const"] == "element")
            .expect("the element anchor shape");
        assert_eq!(
            element["properties"]["fingerprint"]["type"], "string",
            "the element anchor must accept a content fingerprint, or agents can \
             only ever write unverified anchors"
        );
        let selector_doc = element["properties"]["selector"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            selector_doc.contains("NOT the identity"),
            "the selector's description must warn that it is a positional hint, \
             so an agent does not treat it as durable: {selector_doc:?}"
        );
        let update = descriptors
            .iter()
            .find(|tool| tool["name"] == "st_comments_update")
            .unwrap();
        assert_eq!(update["inputSchema"]["required"], json!(["env", "id"]));
        for property in ["env", "id", "status", "reply"] {
            assert!(
                update["inputSchema"]["properties"].get(property).is_some(),
                "update schema missing {property}"
            );
        }

        let root = temp_project();
        let mut state = McpState::new(root.clone());
        let opened = st_env_open(&json!({ "path": root }), &mut state).unwrap().1;
        let env = opened["id"].as_str().unwrap();
        let added = st_comments_add(
            &json!({
                "env": env,
                "type": "todo",
                "anchor": { "kind": "element", "route": "/", "selector": ".cta > button" },
                "text": "Keep the follow-up contract beside the work",
                "agent_name": "test-agent"
            }),
            &mut state,
        )
        .unwrap()
        .1;
        let id = added["id"].as_str().unwrap().to_string();
        let listed = st_comments_list(&json!({ "env": env }), &mut state)
            .unwrap()
            .1;
        // A row is `{ comment, agent_hint }`: the hint travels BESIDE the
        // record so `comment` stays byte-comparable with what the routes
        // return and what sits on disk. That parity is what lets the hosted
        // tier be a transport swap instead of a rewrite.
        let row = listed["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["comment"]["id"] == id)
            .expect("added record listed");
        assert!(
            row["agent_hint"].is_string(),
            "the type's follow-up contract must reach the agent with the work: {row}"
        );
        assert!(
            row["comment"]["agent_hint"].is_null(),
            "the canonical record must NOT carry a transport-only field: {row}"
        );
        st_comments_update(&json!({ "env": env, "id": id, "status": "in-progress", "reply": "Claimed for execution" }), &mut state).unwrap();
        let disk: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(".comments").join(format!("{id}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(disk["status"], "in-progress");
        assert_eq!(disk["thread"][0]["text"], "Claimed for execution");
        std::fs::remove_dir_all(root).ok();
    }

    /// A reply must carry WHO wrote it, not just what was said.
    ///
    /// `st_comments_update` reads `agent_name` at three separate write sites
    /// (the record author, the status transition, and the reply), and until
    /// this test NONE of them was asserted anywhere. The reply site is the one
    /// that matters most: the handler deliberately preserves the caller's name
    /// rather than flattening it to "agent", because a named caller that
    /// creates a correctly attributed record and then loses that identity in
    /// its own thread is worse than one that was never named -- the thread
    /// reads as a different actor speaking.
    ///
    /// Two agents on one record is the case the hosted tier is built around,
    /// so "which agent said this" must survive a round-trip to disk.
    #[test]
    fn a_reply_is_attributed_to_the_agent_that_wrote_it() {
        let root = temp_project();
        let mut state = McpState::new(root.clone());
        let opened = st_env_open(&json!({ "path": root }), &mut state).unwrap().1;
        let env = opened["id"].as_str().unwrap().to_string();
        let env = env.as_str();
        let added = st_comments_add(
            &json!({
                "env": env,
                "type": "todo",
                "anchor": { "kind": "page", "route": "/" },
                "text": "Needs a second opinion",
                "agent_name": "reviewer-a",
            }),
            &mut state,
        )
        .unwrap()
        .1;
        let id = added["id"].as_str().expect("added id").to_string();

        // A DIFFERENT agent answers. Attribution is only meaningful when the
        // replier can differ from the author.
        st_comments_update(
            &json!({ "env": env, "id": id, "reply": "Checked it, the markup is fine", "agent_name": "reviewer-b" }),
            &mut state,
        )
        .unwrap();
        // And an unnamed caller still lands, as "agent" -- absence must be a
        // documented default, never a silent adoption of the previous name.
        st_comments_update(
            &json!({ "env": env, "id": id, "reply": "Anonymous follow-up" }),
            &mut state,
        )
        .unwrap();

        let disk: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(".comments").join(format!("{id}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            disk["thread"][0]["author"]["name"], "reviewer-b",
            "the reply must name the agent that wrote it, not the record's author: {disk}"
        );
        assert_eq!(
            disk["thread"][0]["author"]["kind"], "agent",
            "an MCP reply is written by an agent: {disk}"
        );
        assert_eq!(
            disk["author"]["name"], "reviewer-a",
            "replying must not rewrite who opened the comment: {disk}"
        );
        assert_eq!(
            disk["thread"][1]["author"]["name"], "agent",
            "an unnamed caller falls back to `agent`, and must not inherit the previous replier: {disk}"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
