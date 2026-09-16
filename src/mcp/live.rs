//! Live HTTP surface for the MCP server.
//!
//! Serves the ONE persistent unified environment (the workbench host page) and
//! the region pipeline that mounts live Spacetime content into it:
//!
//! ```text
//! agent  st_fn_put{source}     → compile → store as a FunctionRecord
//! agent  st_mount{fn, region}  → mount the function into a region of the page
//! page   GET /__mcp/instance/{id}/         → the host shell (workbench env.st)
//!        GET /__mcp/instance/{id}/bundles  → the region's compiled Bundle JSON
//!        POST /__mcp/signal/{id}           → the ONE human→host signal sink
//! ```
//!
//! There is a SINGLE human→host signal path (`/__mcp/signal/{instance}`). The
//! legacy bespoke propose/choose/picker/option surface was removed in FEAT-131
//! (PLAN-041 W5, the harmony gate): picker/form/confirm now compose as region-
//! mounted kit functions over the same signal sink.
//!
//! ROUTING (PLAN-045): every `mcp-action` on that one sink is routed by AUDIENCE
//! (see [`super::actions`]):
//!   - Server actions (st_env_open / st_tab_open / mcp_refresh) are resolved
//!     SYNCHRONOUSLY here at the sink (`dispatch_signal_action`), update the
//!     instance's input.json, and repaint — the agent is never involved.
//!   - Agent actions (kit-choice / kit-confirm / kit-submit, and a page-declared
//!     agent-control surface's own actions) are recorded and delivered ONLY
//!     through `st_await`.
//! `await_event` filters to Agent-audience, so navigation never wakes the agent
//! and a nav click can no longer satisfy an unrelated armed await. The full
//! event history (both audiences) stays in `events` for the comms drawer.
//!
//! The server binds an ephemeral loopback port and runs on its own thread +
//! single-thread runtime, so it never blocks the serial stdio loop.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};

use super::actions;
use super::bundle::compile_to_bundle;
use super::state::{TabSource, WorkbenchSession};

/// A structured compile diagnostic for one compiled unit. Surfaced in the tool
/// result (agent-facing `structuredContent`) and on the diagnostics rail.
#[derive(Clone, Debug)]
pub struct OptionDiagnostic {
    pub severity: String, // "error" | "warning"
    pub code: String,
    pub message: String,
}

impl OptionDiagnostic {
    pub fn to_json(&self) -> Value {
        json!({
            "severity": self.severity,
            "code": self.code,
            "message": self.message,
        })
    }
}

/// A registered live Spacetime function revision.
#[derive(Clone, Debug)]
pub struct FunctionRecord {
    pub id: String,
    pub name: Option<String>,
    pub revision: u64,
    pub source: String,
    pub contract: Value,
    pub html: String,
    pub css: String,
    pub js: String,
    pub compile_ok: bool,
    pub diagnostics: Vec<OptionDiagnostic>,
    /// Filesystem root the function's source was compiled against (env root or
    /// workspace root). The bundles endpoint reuses this so a function whose
    /// source imports env-local files recompiles against the SAME root it
    /// mounted with — not the bare workspace root.
    pub resolve_root: std::path::PathBuf,
    /// Provenance: where this function came from (Agent inline source, an
    /// environment binding, stdlib, or kit). Surfaced as a chip in the workbench
    /// (FEAT-128). Set at `put_function`; updated on re-put when the origin moves.
    pub origin: super::origin::Origin,
    /// The function's bundle contract as a flat list of template param specs
    /// (FEAT-132 v1 inspector contract: name/kind/optional/type/default, derived
    /// at extraction — FEAT-125). Each entry is one template's params tagged with
    /// the owning template name. Empty when the source defines no template params.
    pub contract_params: Value,
    /// FEAT-124: a stable hash of the function's CONTRACT (its template param
    /// specs — names/kinds/optionality, order-independent). Re-putting a function
    /// whose hash is unchanged is a COMPATIBLE revision (mounted instances
    /// hot-swap safely); a changed hash is a BREAKING revision (callers/mounts
    /// may need migration). Derived by `contract_hash_of`.
    pub contract_hash: String,
}

/// FEAT-124: derive a stable, order-independent hash of a function's contract
/// (its flat `contract_params` list). Two functions with the SAME set of
/// template param specs (name/kind/optional/type) hash equal regardless of param
/// ordering, so a pure reordering is COMPATIBLE. Used to classify a re-put as
/// initial / compatible / breaking.
pub fn contract_hash_of(contract_params: &Value) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // Normalize each param to a stable string tuple, then sort so order does not
    // affect the hash. Missing fields fold to empty strings.
    let mut rows: Vec<String> = match contract_params.as_array() {
        Some(arr) => arr
            .iter()
            .map(|p| {
                let g = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                let optional = p.get("optional").and_then(Value::as_bool).unwrap_or(false);
                // template is identity, not contract: a param's owning template
                // name is part of the surface (a param moving template IS a
                // contract change), so include it.
                format!(
                    "{}|{}|{}|{}|{}",
                    g("template"),
                    g("name"),
                    g("kind"),
                    g("type"),
                    optional
                )
            })
            .collect(),
        None => Vec::new(),
    };
    rows.sort();
    let mut hasher = DefaultHasher::new();
    rows.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// FEAT-124: how a `put_function` relates to the prior revision of the same
/// named function. Drives the update report + whether a mounted instance can
/// hot-swap silently (Compatible) or the agent should be warned (Breaking).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compatibility {
    /// First revision of this function id (no prior to compare).
    Initial,
    /// Same contract hash as the prior revision — mounts hot-swap safely.
    Compatible,
    /// Contract hash changed — callers/mounts may need migration.
    Breaking,
}

impl Compatibility {
    /// Stable wire label surfaced in the tool result.
    pub fn label(self) -> &'static str {
        match self {
            Compatibility::Initial => "initial",
            Compatibility::Compatible => "compatible",
            Compatibility::Breaking => "breaking",
        }
    }
}

/// FEAT-124: the result of a `put_function` — the stored record plus how it
/// relates to the prior revision (for the update report + hot-swap policy).
#[derive(Clone, Debug)]
pub struct PutOutcome {
    pub record: FunctionRecord,
    pub compatibility: Compatibility,
}

impl FunctionRecord {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "revision": self.revision,
            "contract": self.contract,
            "compile": {
                "ok": self.compile_ok,
                "css_len": self.css.len(),
                "js_len": self.js.len(),
                "html_len": self.html.len(),
                "diagnostics": self.diagnostics.iter().map(OptionDiagnostic::to_json).collect::<Vec<_>>(),
            },
            "source_len": self.source.len(),
            "origin": self.origin.to_json(),
            "contract_params": self.contract_params.clone(),
            "contract_hash": self.contract_hash,
        })
    }
}

/// A mounted function instance.
#[derive(Clone, Debug)]
pub struct InstanceRecord {
    pub id: String,
    pub function_id: String,
    pub function_revision: u64,
    pub title: String,
    pub input: Value,
    pub url: String,
    /// Region identity injected onto the mounted page's root element.
    /// FEAT-126: every mounted instance is a region; `data-st-region`
    /// isolates its bundle CSS and is the mount target for `@view`.
    pub region_id: String,
    /// Cursor used by `st_await`: events at or below this seq were already
    /// delivered to the agent by the await tool. Full history stays in `events`.
    pub await_cursor: u64,
    /// PLAN-045: when true, this mounted page declares itself an agent-control
    /// interface, so its OTHERWISE-unknown `mcp-action`s route to the agent
    /// (instead of being rejected at the sink). Built-in registry actions are
    /// unaffected — a kit's `kit-choice` is Agent and `st_env_open` is Server
    /// regardless. Defaults false (a normal page only emits registered actions).
    pub agent_control: bool,
    pub events: Vec<LiveEvent>,
    /// FEAT-126 / BUG-111: guest functions composed INTO this instance's named
    /// regions. Maps a region id (e.g. "stage") -> the function id mounted there.
    /// A host page (the workbench) polls `/bundles?region={region}`; the endpoint
    /// serves the guest bundle so the host composes it in place rather than the
    /// guest spawning its own standalone page. Empty for a normal standalone mount.
    pub region_mounts: HashMap<String, String>,
    /// M4 (PLAN-046): per-region param OVERRIDES for composed guests. Maps a
    /// region id -> (param name -> value). The inspector's editable param controls
    /// write here via the `mcp_param` action; the region bundle carries these as
    /// `input`, and `registerBundle` seeds them onto the guest root so the guest's
    /// `$param` holes reflect the edited value live. Empty = guest uses defaults.
    pub region_mount_input: HashMap<String, HashMap<String, String>>,
}

impl InstanceRecord {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "function_id": self.function_id,
            "function_revision": self.function_revision,
            "title": self.title,
            "input": self.input,
            "url": self.url,
            "region_id": self.region_id,
            "await_cursor": self.await_cursor,
            "events": self.events.iter().map(LiveEvent::to_json).collect::<Vec<_>>(),
            "region_mounts": self.region_mounts.iter()
                .map(|(region, fid)| json!({ "region": region, "function_id": fid }))
                .collect::<Vec<_>>(),
        })
    }
}

/// One host-directed event emitted by a mounted instance.
#[derive(Clone, Debug)]
pub struct LiveEvent {
    pub seq: u64,
    pub instance_id: String,
    pub event_type: String,
    pub payload: Value,
    pub correlation: Option<String>,
    /// PLAN-045: who this event is addressed to, decided at record time by the
    /// action registry. `Server` events resolve synchronously at the sink and are
    /// EXCLUDED from `st_await` (so navigation never wakes the agent); `Agent`
    /// events are the only ones the await stream returns. Full history (both)
    /// stays in `events` for the comms drawer.
    pub audience: actions::Audience,
    pub raw: Value,
}

impl LiveEvent {
    pub fn to_json(&self) -> Value {
        json!({
            "seq": self.seq,
            "instance_id": self.instance_id,
            "type": self.event_type,
            "payload": self.payload,
            "correlation": self.correlation,
            "audience": self.audience.label(),
            "raw": self.raw,
        })
    }
}

struct LiveStore {
    workbench: WorkbenchSession,
    functions: HashMap<String, FunctionRecord>,
    functions_by_name: HashMap<String, String>,
    instances: HashMap<String, InstanceRecord>,
    next_function: u64,
    next_instance: u64,
    next_event: u64,
    /// BUG-118: region-bundle cache keyed by (function_id, revision). The region
    /// poll (`@mcp-region`, 1Hz) hits `/bundles` repeatedly; without this it
    /// recompiled the guest's full source on EVERY poll (a sustained CPU wedge
    /// that stalled HTTP). A function's compiled bundle is immutable for a given
    /// revision, so we cache the revision-stable JSON (css / templates /
    /// diagnostics / has_errors) and only the per-request bits (ids + region
    /// input override) are merged at serve time. `update_source` / re-put bump
    /// the revision, which is part of the key, so a stale entry is never served
    /// (old entries simply age out of use; the map is small — one live fn set).
    bundle_cache: HashMap<(String, u64), Value>,
    /// The real bound port (FUP-110). Handlers that only have the `Store` in
    /// axum State (e.g. the workbench redirect, the signal sink) reconstruct a
    /// throwaway `LiveServer` and need the ACTUAL port to build a correct
    /// `instance.url`, not a placeholder.
    port: u16,
}

pub fn instance_events_uri(id: &str) -> String {
    format!("spacetime://instances/{id}/events")
}

type Store = Arc<Mutex<LiveStore>>;

/// Flatten every function's `contract_params` into ONE top-level array for the
/// inspector drawer (FEAT-132). Each row gets the owning function's id + name +
/// origin folded in, so a flat `@each` can render param rows WITHOUT a nested
/// `@each` inside a template body (the template serializer emits a nested @each
/// literally — it is only supported in the selector/slot form). One source array,
/// rendered flat.
/// Coerce a JSON input value into the String a `$param` hole reads.
///
/// region_mount_input is `String->String` (the bundle seeds each `$param` hole
/// with a string, the same shape `mcp_param` writes). A scalar (string/number/
/// bool) maps to its natural text; null and composite (object/array) values are
/// skipped (`None`) — a v1 guest param is a scalar hole, and seeding `[object
/// Object]` would be worse than leaving the default.
fn json_value_to_param_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

pub(crate) fn flatten_contract_params(functions: &[Value]) -> Vec<Value> {
    let mut rows: Vec<Value> = Vec::new();
    for f in functions {
        let fn_name = f.get("name").cloned().unwrap_or(Value::Null);
        let fn_id = f.get("id").cloned().unwrap_or(Value::Null);
        let origin = f.get("origin").cloned().unwrap_or(Value::Null);
        if let Some(params) = f.get("contract_params").and_then(Value::as_array) {
            for p in params {
                let mut row = p.clone();
                if let Some(obj) = row.as_object_mut() {
                    obj.insert("function".to_string(), fn_name.clone());
                    obj.insert("function_id".to_string(), fn_id.clone());
                    obj.insert("origin".to_string(), origin.clone());
                }
                rows.push(row);
            }
        }
    }
    rows
}

impl LiveStore {
    fn workbench_snapshot(&self) -> Value {
        let functions = self
            .functions
            .values()
            .map(FunctionRecord::to_json)
            .collect::<Vec<_>>();
        // BUG-115: the inspector "ALL CONTRACTS" list must show only the USER's
        // composable functions, not the workbench host's own internal card
        // sub-templates (mcp-env-card, mcp-param-edit, … — renderer plumbing). The
        // `functions` array above keeps the host (the origin drawer legitimately
        // lists every loaded fragment incl. the host); contract_params excludes it,
        // the same self-exclusion the gallery uses below.
        let guest_functions = self
            .functions
            .values()
            .filter(|f| f.name.as_deref() != Some("mcp-workbench"))
            .map(FunctionRecord::to_json)
            .collect::<Vec<_>>();
        let contract_params = flatten_contract_params(&guest_functions);
        // M2 gallery source: composable guests only — exclude the workbench host
        // page itself (it is the renderer, not a guest) and any function that does
        // not currently compile (mounting it would be rejected). The gallery binds
        // THIS list, not the full registry.
        let gallery_functions = self
            .functions
            .values()
            .filter(|f| f.name.as_deref() != Some("mcp-workbench") && f.compile_ok)
            .map(FunctionRecord::to_json)
            .collect::<Vec<_>>();
        let instances = self
            .instances
            .values()
            .map(InstanceRecord::to_json)
            .collect::<Vec<_>>();
        let events = self
            .instances
            .values()
            .flat_map(|instance| instance.events.iter().map(LiveEvent::to_json))
            .collect::<Vec<_>>();
        // M5 (PLAN-046): the COMMS feed — agent-audience events only (the human↔
        // agent conversation: kit answers, agent-sent actions), newest first.
        // Server/navigation events (compose, param edits, env opens) are control
        // plane noise and stay out of the feed. The comms drawer binds this.
        let mut comms_events: Vec<Value> = self
            .instances
            .values()
            .flat_map(|instance| instance.events.iter())
            .filter(|e| e.audience == actions::Audience::Agent)
            .map(LiveEvent::to_json)
            .collect();
        comms_events.sort_by(|a, b| {
            b["seq"]
                .as_u64()
                .unwrap_or(0)
                .cmp(&a["seq"].as_u64().unwrap_or(0))
        });
        // M3' (PLAN-046): what is composed on the workbench's STAGE region right
        // now — the guest the in-stage selection inspects. Find the workbench host
        // instance, read its `region_mounts["stage"]`, and project that function's
        // identity + contract params so the inspector can bind to the SELECTED
        // frame (not the whole registry). Null when the stage is empty.
        let stage_mount = self
            .instances
            .values()
            .find(|i| {
                self.functions
                    .get(&i.function_id)
                    .and_then(|f| f.name.as_deref())
                    == Some("mcp-workbench")
            })
            .and_then(|host| host.region_mounts.get("stage"))
            .and_then(|fid| self.functions.get(fid))
            .map(|f| {
                let fj = f.to_json();
                let params = flatten_contract_params(std::slice::from_ref(&fj));
                json!({
                    "function_id": f.id,
                    "name": f.name,
                    "revision": f.revision,
                    "origin": f.origin.to_json(),
                    "contract_hash": f.contract_hash,
                    "contract_params": params,
                    // M6 (PLAN-046): the guest's full source, so the inspector's
                    // source editor can show + edit it. Only the SELECTED stage
                    // guest's source is exposed (not the whole registry).
                    "source": f.source,
                })
            })
            .unwrap_or(Value::Null);

        // PLAN-049 W4.S8: the FRAME STRIP. Project every region composed on the
        // workbench host into a flat list (region + the composed function's
        // identity + origin). Today the host has one region ("stage"), but the
        // strip generalizes the projection so the deferred multi-frame PLAN drops
        // in without a schema change. Each chip drives switch (re-inspect) +
        // unmount-from-region (mcp_stage_clear of that region). Empty = no frames.
        let workbench_host = self.instances.values().find(|i| {
            self.functions
                .get(&i.function_id)
                .and_then(|f| f.name.as_deref())
                == Some("mcp-workbench")
        });
        let stage_frames: Vec<Value> = workbench_host
            .map(|host| {
                let mut frames: Vec<Value> = host
                    .region_mounts
                    .iter()
                    .filter_map(|(region, fid)| {
                        self.functions.get(fid).map(|f| {
                            json!({
                                "region": region,
                                "function_id": f.id,
                                "name": f.name.clone().unwrap_or_else(|| f.id.clone()),
                                "revision": f.revision,
                                "origin": f.origin.to_json(),
                            })
                        })
                    })
                    .collect();
                // Stable display order (region name).
                frames.sort_by(|a, b| {
                    a["region"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(b["region"].as_str().unwrap_or(""))
                });
                frames
            })
            .unwrap_or_default();

        // PLAN-049 W2 (COLLAPSE): enrich each tab with its function's compile
        // status so the tab card can mark a non-composable (broken) tab. The Tab
        // record lives in WorkbenchSession (no function registry); the compile
        // status lives here on FunctionRecord — join them by tab.function_id.
        let tabs: Vec<Value> = self
            .workbench
            .all_tabs_json()
            .into_iter()
            .map(|mut t| {
                let compile_ok = t
                    .get("function_id")
                    .and_then(Value::as_str)
                    .and_then(|fid| self.functions.get(fid))
                    .map(|f| f.compile_ok)
                    .unwrap_or(false);
                if let Some(obj) = t.as_object_mut() {
                    // String, not bool: the tab card binds this into a
                    // `data-compile-ok` ATTRIBUTE (`"true"`/`"false"`), and the
                    // template hole renders a JSON bool as an empty string. The
                    // CSS gate keys off the literal "false" token.
                    obj.insert(
                        "compile_ok".to_string(),
                        json!(if compile_ok { "true" } else { "false" }),
                    );
                }
                t
            })
            .collect();

        // PLAN-049 W2 (COLLAPSE): flatten every open env's discoverable `.st`
        // entries into ONE top-level list (each row carries its `env`), so the
        // entry picker renders with a single top-level `@each` — the same
        // server-flatten the inspector's contract_params uses (a nested @each in
        // a template body is unsupported). Turns "open env" into a real
        // entry-point browser.
        let open_envs = self.workbench.open_environments_json();
        let env_entries: Vec<Value> = open_envs
            .iter()
            .filter_map(|e| e.get("entries").and_then(Value::as_array))
            .flat_map(|arr| arr.iter().cloned())
            .collect();

        // PLAN-051 B1: the STAGE STRUCTURE — the composed guest's template graph
        // walked from &main via body.refs into a flat depth-tagged node list (the
        // builder navigator binds it). Reuses the same host->stage->function
        // lookup as stage_mount; compiles the guest bundle (the source the guest
        // mounted with, against its own resolve_root) and projects its structure.
        // Empty array when the stage is empty or the guest fails to compile (the
        // navigator just shows nothing, never errors).
        let stage_structure: Value = self
            .instances
            .values()
            .find(|i| {
                self.functions
                    .get(&i.function_id)
                    .and_then(|f| f.name.as_deref())
                    == Some("mcp-workbench")
            })
            .and_then(|host| host.region_mounts.get("stage"))
            .and_then(|fid| self.functions.get(fid))
            .and_then(|f| compile_to_bundle(&f.source, &f.resolve_root).ok())
            .map(|bundle| crate::introspect::structure_json(&bundle))
            .unwrap_or_else(|| json!([]));

        // PLAN-051 B4: a FLAT list of every composed node's bound params, each row
        // tagged with its node_id + the invocation's invoke_span (the G2 write
        // address). The inspector renders the SELECTED node's rows via a filtered
        // `@each(... when $p.node_id == $selectedNode)` (a template body can't host
        // a nested @each, so the projection pre-flattens). Editing a row fires
        // `mcp_node_param` carrying span_start/span_end → source patch.
        let stage_node_params = crate::introspect::param_rows(&stage_structure);

        // PLAN-064 B4a: per-ELEMENT binding edit rows. A selected element node reads
        // signals (`bindings`); those that resolve to the ENTRY template's PARAMS are
        // editable via the EXISTING `mcp_param` value rail (provenance dispatch — a
        // hole's value lives UPSTREAM in the param override, not at the element
        // span, so no element source span is needed). The stage guest bundle + its
        // region param overrides give each row its current value.
        let (stage_bundle_for_bindings, stage_region_input) = self
            .instances
            .values()
            .find(|i| {
                self.functions
                    .get(&i.function_id)
                    .and_then(|f| f.name.as_deref())
                    == Some("mcp-workbench")
            })
            .and_then(|host| {
                host.region_mounts.get("stage").and_then(|fid| {
                    self.functions.get(fid).and_then(|f| {
                        compile_to_bundle(&f.source, &f.resolve_root)
                            .ok()
                            .map(|b| (b, host.region_mount_input.get("stage").cloned()))
                    })
                })
            })
            .map(|(b, ri)| (Some(b), ri))
            .unwrap_or((None, None));
        let stage_node_bindings = match &stage_bundle_for_bindings {
            Some(bundle) => stage_node_bindings_from_structure(
                &stage_structure,
                bundle,
                stage_region_input.as_ref(),
            ),
            None => json!([]),
        };

        json!({
            "schema_version": 1,
            "environments": {
                "discovered": self.workbench.discovered_environments_json(),
                "open": open_envs,
            },
            "tabs": tabs,
            "env_entries": env_entries,
            "functions": functions,
            "gallery_functions": gallery_functions,
            "contract_params": contract_params,
            "instances": instances,
            "events": events,
            "comms_events": comms_events,
            "stage_mount": stage_mount,
            "stage_frames": stage_frames,
            "stage_structure": stage_structure,
            "stage_node_params": stage_node_params,
            "stage_node_bindings": stage_node_bindings,
        })
    }
}

/// PLAN-051 B4: flatten the stage structure into one row per (node, bound param),
/// each carrying the node id + the invocation's invoke_span (the G2 write
/// address) so the inspector can render editable per-instance param widgets that
/// route a `mcp_node_param` edit to the exact source span. Only nodes WITH an
/// invoke_span (real `&child(...)` invocations) and bound params contribute (the
/// root `&main` has neither). Order follows the structure walk.

/// PLAN-064 B4a: project per-ELEMENT binding edit rows from the stage structure.
///
/// For each element/hole node, each of its `bindings` (the signal names its holes
/// read) that RESOLVES to an ENTRY-TEMPLATE binding param becomes an editable row
/// routed through the EXISTING `mcp_param` value rail. This is provenance dispatch
/// (DEC-b4-per-node-editing): a hole's editable VALUE lives upstream in the param
/// override, not at the element's source span — so selecting a hero's `<h1>` and
/// editing its title updates `$title` (every consumer), with NO element source
/// span needed. Non-param bindings (locals, loop vars, dotted object roots that
/// aren't params) are skipped — they have no simple value rail yet.
///
/// The current value is the region param override (what `mcp_param` last set),
/// else the param's declared default; `rail: "mcp_param"` tells the UI which
/// action to fire. Rows carry `node_id` so the inspector filters to $selectedNode.
fn stage_node_bindings_from_structure(
    structure: &Value,
    bundle: &super::state::Bundle,
    region_input: Option<&std::collections::HashMap<String, String>>,
) -> Value {
    use super::state::ParamKind;
    // Index the ENTRY template's binding params by name (the value sources an
    // element hole can edit). Prefer `main`; else the first template.
    let entry = bundle
        .templates
        .iter()
        .find(|t| t.name == "main")
        .or_else(|| bundle.templates.first());
    let Some(entry) = entry else {
        return json!([]);
    };
    let param_by_name: std::collections::HashMap<&str, &super::state::ParamSpec> = entry
        .params
        .iter()
        .filter(|p| matches!(p.kind, ParamKind::Binding))
        .map(|p| (p.name.as_str(), p))
        .collect();

    let Some(nodes) = structure.as_array() else {
        return json!([]);
    };
    let mut rows: Vec<Value> = Vec::new();
    for node in nodes {
        // Only element/hole nodes carry editable `bindings`; template/extern nodes
        // use the invoke-span rail (stage_node_params).
        let kind = node.get("kind").and_then(Value::as_str).unwrap_or("");
        if kind != "element" && kind != "hole" {
            continue;
        }
        let node_id = node.get("id").cloned().unwrap_or(Value::Null);
        let Some(bindings) = node.get("bindings").and_then(Value::as_array) else {
            continue;
        };
        for b in bindings {
            let Some(name) = b.as_str() else { continue };
            // Only a binding that resolves to an entry-template PARAM is editable
            // here (its value source is the region param override).
            let Some(spec) = param_by_name.get(name) else {
                continue;
            };
            // Current value: the region override wins; else the declared default.
            // `spec.default` is already the EVALUATED value (ParamSpec::from strips
            // the source token's delimiter quotes). `bound` = an override is set.
            let override_val = region_input.and_then(|ri| ri.get(name));
            let default_display = spec.default.clone();
            let value = override_val
                .cloned()
                .or_else(|| default_display.clone())
                .unwrap_or_default();
            rows.push(json!({
                "node_id": node_id,
                "param": name,
                "value": value,
                "type": spec.type_ref,
                "default": default_display,
                "bound": override_val.is_some(),
                "rail": "mcp_param",
                "region": "stage",
            }));
        }
    }
    json!(rows)
}

/// PLAN-049 W2: flatten a compiled bundle's contract into param rows (the
/// inspector/gallery shape). Canonical home (used by `put_source` here + the
/// tool path delegating in tools.rs) so the projection lives in ONE place.
pub fn contract_params_from_bundle(bundle: &super::state::Bundle) -> Value {
    use super::state::ParamKind;
    let mut rows: Vec<Value> = Vec::new();
    for t in &bundle.contract.templates {
        for p in &t.params {
            rows.push(json!({
                "template": t.name,
                "name": p.name,
                "kind": match p.kind { ParamKind::Binding => "binding", ParamKind::Element => "element" },
                "optional": p.optional,
                "type": p.type_ref,
                "default": p.default,
            }));
        }
    }
    json!(rows)
}

/// PLAN-064 B3.1: project a guest bundle's structure for the builder navigator by
/// delegating to the SHARED Structure IR producer (`crate::introspect`), then
/// enriching template nodes with the inspector's param/bound display fields. The
/// node ids are the IR's canonical DOTTED PATHS (`"0"`, `"0.0"`, `"0.0.1"`) \u2014 the
/// SAME address space the dev-ws host + the guest DOM `data-st-node` stamps use,
/// so a navigator click and a canvas click resolve to ONE node (B3). This
/// REPLACED the old host-local integer-counter walk (which diverged from the IR).
///
/// Each node carries the fields the workbench cards bind:
///   { id, label, kind, depth, template, ref_name, params:[\u2026], bound_params:[\u2026],
///     bound_summary, param_count, recursive, invoke_span, bindings:[\u2026] }
/// Element/hole nodes (from the IR's html walk) carry `bindings` (the reactive
/// signals they read) and null template fields; template/extern nodes carry the
/// param schema + bound values paired from the invocation's args.

/// A running live server. Lazily started on the first live MCP tool call.
pub struct LiveServer {
    pub port: u16,
    store: Store,
}

impl LiveServer {
    /// Boot the HTTP server on an ephemeral loopback port. Blocks only until the
    /// port is bound (a few ms), then serves in the background.
    pub fn start(workbench: WorkbenchSession) -> std::io::Result<Self> {
        Self::start_on(workbench, "127.0.0.1:0")
    }

    /// Boot the HTTP server bound to `addr` (FUP-110). `addr` is a full
    /// `host:port` string; port `0` picks an ephemeral port (the MCP stdio path
    /// uses this — many instances can coexist, so a fixed port would collide).
    /// The standalone `spacetime workbench` command binds a FIXED, predictable
    /// port instead, so the URL is stable across runs and an agent/CI can reach
    /// it without scraping stdout for a freshly-chosen ephemeral number.
    pub fn start_on(workbench: WorkbenchSession, addr: &str) -> std::io::Result<Self> {
        let store: Store = Arc::new(Mutex::new(LiveStore {
            workbench,
            functions: HashMap::new(),
            functions_by_name: HashMap::new(),
            instances: HashMap::new(),
            next_function: 1,
            next_instance: 1,
            next_event: 1,
            bundle_cache: HashMap::new(),
            port: 0,
        }));
        let store_for_thread = store.clone();
        let (tx, rx) = std::sync::mpsc::channel::<u16>();
        let addr = addr.to_string();

        std::thread::Builder::new()
            .name("mcp-live-http".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("[spacetime-mcp] live server runtime error: {e}");
                        return;
                    }
                };
                rt.block_on(async move {
                    let app = Router::new()
                        // Generic Function Environment routes (FEAT-122).
                        .route("/__mcp/instance/{id}/", get(instance_page))
                        .route("/__mcp/instance/{id}/input.json", get(instance_input_json))
                        .route("/__mcp/instance/{id}/css", get(instance_css))
                        .route("/__mcp/instance/{id}/js", get(instance_js))
                        .route("/__mcp/instance/{id}/bundles", get(instance_bundles))
                        .route("/__mcp/instance/{id}/{*path}", get(instance_asset))
                        .route("/__mcp/signal/{id}", post(signal_sink))
                        // FUP-110/friction fix: ONE stable entry URL. Idempotently
                        // mounts (or reuses) the ONE workbench host instance and
                        // redirects to its `inst-N` page — no more "hunt for the
                        // freshly-chosen inst id" every time the server restarts.
                        .route("/__mcp/workbench", get(workbench_redirect))
                        .route("/__spacetime/workbench", get(workbench_redirect))
                        .with_state(store_for_thread);

                    let listener = match tokio::net::TcpListener::bind(&addr).await {
                        Ok(l) => l,
                        Err(e) => {
                            eprintln!("[spacetime-mcp] live server bind error ({addr}): {e}");
                            let _ = tx.send(0);
                            return;
                        }
                    };
                    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
                    let _ = tx.send(port);
                    eprintln!("[spacetime-mcp] live server on http://127.0.0.1:{port}");
                    if let Err(e) = axum::serve(listener, app).await {
                        eprintln!("[spacetime-mcp] live server error: {e}");
                    }
                });
            })?;

        let port = rx
            .recv()
            .map_err(|e| std::io::Error::other(format!("live server never bound: {e}")))?;
        if port == 0 {
            return Err(std::io::Error::other("live server failed to bind a port"));
        }
        store.lock().unwrap().port = port;
        Ok(LiveServer { port, store })
    }

    // ---- Function Environment ------------------------------------------

    pub fn put_function(
        &self,
        name: Option<String>,
        source: String,
        contract: Value,
        html: String,
        css: String,
        js: String,
        compile_ok: bool,
        diagnostics: Vec<OptionDiagnostic>,
        resolve_root: std::path::PathBuf,
        origin: super::origin::Origin,
        contract_params: Value,
    ) -> PutOutcome {
        let mut guard = self.store.lock().unwrap();
        let contract_hash = contract_hash_of(&contract_params);
        // Resolve id + revision, and classify the change against the PRIOR
        // revision's contract hash (FEAT-124).
        let (id, revision, compatibility) = if let Some(name) = name.as_deref() {
            if let Some(existing_id) = guard.functions_by_name.get(name).cloned() {
                let prior = guard.functions.get(&existing_id);
                let next_revision = prior.map(|f| f.revision + 1).unwrap_or(1);
                let compat = match prior {
                    Some(p) if p.contract_hash == contract_hash => Compatibility::Compatible,
                    Some(_) => Compatibility::Breaking,
                    None => Compatibility::Initial,
                };
                (existing_id, next_revision, compat)
            } else {
                let id = format!("fn-{}", guard.next_function);
                guard.next_function += 1;
                guard.functions_by_name.insert(name.to_string(), id.clone());
                (id, 1, Compatibility::Initial)
            }
        } else {
            let id = format!("fn-{}", guard.next_function);
            guard.next_function += 1;
            (id, 1, Compatibility::Initial)
        };

        let record = FunctionRecord {
            id: id.clone(),
            name,
            revision,
            source,
            contract,
            html,
            css,
            js,
            compile_ok,
            diagnostics,
            resolve_root,
            origin,
            contract_params,
            contract_hash,
        };
        guard.functions.insert(id.clone(), record.clone());
        // FEAT-124 hot-swap: mounted instances read the LIVE record on every
        // /bundles poll, so a re-put already swaps the code in place. Keep each
        // mounted instance's displayed `function_revision` in sync so the
        // inspector doesn't show a stale revision number for live content.
        if compile_ok {
            for inst in guard.instances.values_mut() {
                if inst.function_id == id {
                    inst.function_revision = revision;
                }
            }
        }
        PutOutcome {
            record,
            compatibility,
        }
    }

    /// PLAN-049 W2: compile `source` ONCE and register it as a function. The
    /// shared core behind BOTH the `st_fn_put`/`st_tab_open` TOOL path and the
    /// `st_tab_open` browser-action SINK path — one compile+put impl, no parallel
    /// registry (AGENTS harmony). Mirrors `put_function_from_args`'s single
    /// compile (FUP-066): `compile_function` yields the region bundle (contract
    /// params + diagnostics) AND the page html/js from ONE pass, so page `ok` and
    /// the bundle's `has_errors` cannot diverge. A parse/import failure registers
    /// the function with one error diagnostic + empty output (still listed, not
    /// mountable).
    pub fn put_source(
        &self,
        name: Option<String>,
        source: String,
        contract: Value,
        resolve_root: std::path::PathBuf,
        origin: super::origin::Origin,
    ) -> PutOutcome {
        self.put_source_with_entry(name, source, contract, resolve_root, origin, None)
    }

    /// Same as [`Self::put_source`], but additionally threads the ORIGINATING
    /// file path (an env-origin tab/entry's on-disk `.st` file) into the
    /// compile step. PLAN-066: this lets `compile_function` detect a sibling
    /// `index.html` shell the entry composes into (and warn that the MCP
    /// preview will be incomplete) — inline/agent sources have no such path and
    /// pass `None`.
    pub fn put_source_with_entry(
        &self,
        name: Option<String>,
        source: String,
        contract: Value,
        resolve_root: std::path::PathBuf,
        origin: super::origin::Origin,
        entry_path: Option<&std::path::Path>,
    ) -> PutOutcome {
        // PLAN-148: EDN is accepted wherever `.st` is, but the acceptance happens
        // at the PARSE step inside `compile_pipeline`, not here — EDN produces the
        // same `StFile` the `.st` parser produces, and never a `.st` text detour.
        //
        // This function stays the convergence point for every source-bearing MCP
        // path (`st_fn_put`, `st_tab_open`, inline `st_mount`, workbench), but it
        // no longer needs to know which syntax it is holding: the source is
        // stored verbatim as written, so an agent that submitted EDN reads back
        // the EDN it wrote rather than a machine translation of it.
        let (contract_params, html, css, js, ok, diagnostics) =
            match super::bundle::compile_function(&source, &resolve_root, entry_path) {
                Ok(compiled) => {
                    let super::bundle::CompiledFunction { bundle, html, js } = compiled;
                    if bundle.has_errors {
                        let diags = bundle
                            .diagnostics
                            .iter()
                            .filter(|d| d.severity == "error")
                            .map(|d| OptionDiagnostic {
                                severity: d.severity.clone(),
                                code: d.code.clone(),
                                message: d.message.clone(),
                            })
                            .collect();
                        (
                            json!([]),
                            String::new(),
                            String::new(),
                            String::new(),
                            false,
                            diags,
                        )
                    } else {
                        let params = contract_params_from_bundle(&bundle);
                        // PLAN-066: a clean compile can still carry WARNING
                        // diagnostics (MCP-LOCALE / MCP-SHELL, or the v1
                        // BUNDLE-V1 file-scope-html notice) — surface them on
                        // the record instead of discarding, so the agent/UI can
                        // see the composed preview may be incomplete even
                        // though compile_ok is true.
                        let warnings = bundle
                            .diagnostics
                            .iter()
                            .map(|d| OptionDiagnostic {
                                severity: d.severity.clone(),
                                code: d.code.clone(),
                                message: d.message.clone(),
                            })
                            .collect();
                        (params, html, bundle.css, js, true, warnings)
                    }
                }
                Err(e) => {
                    let code = if e.to_string().starts_with("Import resolution failed") {
                        "IMPORT"
                    } else {
                        "PARSE"
                    };
                    (
                        json!([]),
                        String::new(),
                        String::new(),
                        String::new(),
                        false,
                        vec![OptionDiagnostic {
                            severity: "error".into(),
                            code: code.into(),
                            message: e.to_string(),
                        }],
                    )
                }
            };
        self.put_function(
            name,
            source,
            contract,
            html,
            css,
            js,
            ok,
            diagnostics,
            resolve_root,
            origin,
            contract_params,
        )
    }

    /// M6 (PLAN-046): replace a function's SOURCE in place (inline-edit save).
    /// Recompiles to validate + refresh the contract; on success updates source,
    /// bumps revision, recomputes the contract hash, and syncs mounted instances
    /// (the /bundles poll then hot-swaps the live guest). Returns the new revision.
    /// A compile failure leaves the stored source UNCHANGED and returns the
    /// diagnostics, so a bad edit never breaks the running guest.
    pub fn update_source(&self, fn_id: &str, new_source: &str) -> Result<(u64, String), String> {
        // Recompile against the function's resolve root (same as the bundle poll).
        let resolve_root = {
            let guard = self.store.lock().unwrap();
            let f = guard
                .functions
                .get(fn_id)
                .ok_or_else(|| format!("no such function: {fn_id}"))?;
            f.resolve_root.clone()
        };
        let compiled = super::bundle::compile_function(new_source, &resolve_root, None)
            .map_err(|e| format!("edit did not compile: {}", e.message))?;
        if compiled.bundle.has_errors {
            let diags = compiled
                .bundle
                .diagnostics
                .iter()
                .filter(|d| d.severity == "error")
                .map(|d| format!("\n  x [{}] {}", d.code, d.message))
                .collect::<String>();
            return Err(format!("edit did not compile -- source unchanged:{diags}"));
        }
        let mut guard = self.store.lock().unwrap();
        let f = guard
            .functions
            .get_mut(fn_id)
            .ok_or_else(|| format!("no such function: {fn_id}"))?;
        f.source = new_source.to_string();
        f.revision += 1;
        f.html = compiled.html;
        f.js = compiled.js;
        f.css = compiled.bundle.css.clone();
        f.compile_ok = true;
        f.diagnostics = Vec::new();
        let new_rev = f.revision;
        // BUG-118: drop this function's stale region-bundle cache entries (any
        // prior revision) so the map doesn't grow unbounded across edits. The new
        // revision is a cache miss and recompiles once on its next /bundles poll.
        guard.bundle_cache.retain(|(fid_k, _), _| fid_k != fn_id);
        // Sync mounted instances' displayed revision (hot-swap continuity).
        for inst in guard.instances.values_mut() {
            if inst.function_id == fn_id {
                inst.function_revision = new_rev;
            }
        }
        Ok((new_rev, fn_id.to_string()))
    }

    pub fn get_function(&self, id_or_name: &str) -> Option<FunctionRecord> {
        let guard = self.store.lock().unwrap();
        if let Some(f) = guard.functions.get(id_or_name) {
            return Some(f.clone());
        }
        guard
            .functions_by_name
            .get(id_or_name)
            .and_then(|id| guard.functions.get(id))
            .cloned()
    }

    pub fn functions(&self) -> Vec<FunctionRecord> {
        self.store
            .lock()
            .unwrap()
            .functions
            .values()
            .cloned()
            .collect()
    }

    pub fn mount_function(
        &self,
        id_or_name: &str,
        title: Option<String>,
        input: Value,
        region: Option<String>,
        agent_control: bool,
    ) -> Result<InstanceRecord, String> {
        let mut guard = self.store.lock().unwrap();
        let function = if let Some(f) = guard.functions.get(id_or_name) {
            f.clone()
        } else if let Some(id) = guard.functions_by_name.get(id_or_name) {
            guard
                .functions
                .get(id)
                .cloned()
                .ok_or_else(|| format!("function name is stale: {id_or_name}"))?
        } else {
            return Err(format!("no such function: {id_or_name}"));
        };
        if !function.compile_ok {
            // Inline the diagnostics so the caller can fix WITHOUT a second
            // st_inspect round-trip (FEAT-133).
            let diags = function
                .diagnostics
                .iter()
                .map(|d| format!("\n  x [{}] {}", d.code, d.message))
                .collect::<String>();
            return Err(format!(
                "function {} did not compile -- fix before mounting:{}",
                function.id, diags
            ));
        }

        let id = format!("inst-{}", guard.next_instance);
        guard.next_instance += 1;
        let region_id = region.unwrap_or_else(|| format!("region-{}", id));
        let url = format!("http://127.0.0.1:{}/__mcp/instance/{}/", self.port, id);
        let instance = InstanceRecord {
            id: id.clone(),
            function_id: function.id.clone(),
            function_revision: function.revision,
            title: title.unwrap_or_else(|| {
                function
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("{} r{}", function.id, function.revision))
            }),
            input,
            url,
            region_id,
            await_cursor: 0,
            agent_control,
            events: Vec::new(),
            region_mounts: HashMap::new(),
            region_mount_input: HashMap::new(),
        };
        guard.instances.insert(id, instance.clone());
        Ok(instance)
    }

    /// FEAT-126 / BUG-111: compose a registered function INTO an existing host
    /// instance's named region, instead of spawning a standalone page. The host
    /// (e.g. the workbench inst-1) polls `/bundles?region={region}`; that endpoint
    /// reads `region_mounts[region]` and serves the guest function's bundle, which
    /// the host's `@mcp-region` composes in place. Returns the updated host record.
    ///
    /// Errors if the host or function is unknown, the function did not compile, or
    /// the region id is not a safe `[A-Za-z0-9_-]+` token (it flows into a CSS
    /// `@scope` selector + `querySelector`, mirroring the st_mount guard).
    pub fn mount_into_host(
        &self,
        host_id: &str,
        region: &str,
        function_key: &str,
        input: &Value,
    ) -> Result<InstanceRecord, String> {
        let mut guard = self.store.lock().unwrap();
        // Resolve the function (by id or name) and gate on compile success.
        let function = if let Some(f) = guard.functions.get(function_key) {
            f.clone()
        } else if let Some(id) = guard.functions_by_name.get(function_key) {
            guard
                .functions
                .get(id)
                .cloned()
                .ok_or_else(|| format!("function name is stale: {function_key}"))?
        } else {
            return Err(format!("no such function: {function_key}"));
        };
        if !function.compile_ok {
            let diags = function
                .diagnostics
                .iter()
                .map(|d| format!("\n  x [{}] {}", d.code, d.message))
                .collect::<String>();
            return Err(format!(
                "function {} did not compile -- fix before mounting into a host:{}",
                function.id, diags
            ));
        }
        // FEAT-126 v1 / FUP-061: region composition transports TEMPLATES. A guest
        // whose source is only file-scope HTML (no `@template`) yields an empty
        // bundle and would leave the host stage silently on its empty-state.
        // Reject loudly with the fix, rather than composing nothing. (Whole
        // file-scope mini-page mounts are tracked by FUP-061.)
        let bundle = compile_to_bundle(&function.source, &function.resolve_root)
            .map_err(|e| format!("guest {} failed to compile: {}", function.id, e.message))?;
        if bundle.templates.is_empty() {
            return Err(format!(
                "function {} has no @template to compose into a region (v1 region-mount is templates-only; \
                 wrap your markup in `@template &main() {{ ... }}`). Whole file-scope page mounts: FUP-061.",
                function.id
            ));
        }
        let host = guard
            .instances
            .get_mut(host_id)
            .ok_or_else(|| format!("no such host instance: {host_id}"))?;
        host.region_mounts
            .insert(region.to_string(), function.id.clone());
        // BUG-113: seed the mount's `input` onto the composed region so the guest
        // renders WITH its values, not just its defaults. Mirrors the `mcp_param`
        // write (the inspector edit path) -- region_mount_input is the single
        // source the bundle endpoint seeds onto the guest root. Only string-
        // coercible scalar fields are carried (the seed map is String->String,
        // matching how `$param` holes read their value); object/array values are
        // skipped (a guest param is a scalar hole in v1).
        if let Some(obj) = input.as_object()
            && !obj.is_empty()
        {
            let seed = host
                .region_mount_input
                .entry(region.to_string())
                .or_default();
            for (k, v) in obj {
                if let Some(s) = json_value_to_param_string(v) {
                    seed.insert(k.clone(), s);
                }
            }
        }
        Ok(host.clone())
    }

    pub fn get_instance(&self, id: &str) -> Option<InstanceRecord> {
        self.store.lock().unwrap().instances.get(id).cloned()
    }

    pub fn instances(&self) -> Vec<InstanceRecord> {
        self.store
            .lock()
            .unwrap()
            .instances
            .values()
            .cloned()
            .collect()
    }

    /// PLAN-049 W1.S1: unmount a live instance, removing it from the store.
    ///
    /// Removes ONLY the named instance record. A composed STAGE guest is NOT an
    /// instance — `mount_into_host` records it as a host `region_mounts` entry,
    /// no `InstanceRecord` — so clearing a host region is a SEPARATE concern
    /// (`clear_region` / the `mcp_stage_clear` glyph), not a side effect of
    /// unmounting a standalone instance. Coupling them by `function_id` would be
    /// wrong: unmounting a standalone preview of fn-X must not wipe an unrelated
    /// host region that happens to compose the same fn-X (reviewer W1 P2). When a
    /// HOST instance is unmounted, its own `region_mounts` vanish with the
    /// record; the guest FUNCTIONS stay registered (reusable). Returns the
    /// removed instance's id + title; errors if there is no such instance.
    pub fn unmount_instance(&self, instance_id: &str) -> Result<(String, String), String> {
        let mut guard = self.store.lock().unwrap();
        let removed = guard
            .instances
            .remove(instance_id)
            .ok_or_else(|| format!("no such instance: {instance_id}"))?;
        Ok((removed.id, removed.title))
    }

    /// PLAN-049 W1.S2: clear a host instance's region — drop the composed guest
    /// mapping + its param overrides so the region returns to its empty-state.
    /// Idempotent: clearing an already-empty region is a no-op success. Returns
    /// the function id that WAS composed (if any) for the caller's summary.
    pub fn clear_region(&self, host_id: &str, region: &str) -> Result<Option<String>, String> {
        let mut guard = self.store.lock().unwrap();
        let host = guard
            .instances
            .get_mut(host_id)
            .ok_or_else(|| format!("no such host instance: {host_id}"))?;
        let prior = host.region_mounts.remove(region);
        host.region_mount_input.remove(region);
        Ok(prior)
    }

    /// PLAN-049 W1.S3: delete a registered function. Refuses (with a clear
    /// message pointing at unmount) while the function is still mounted — either
    /// as a standalone instance or composed into a host region — so deletion
    /// never leaves a dangling mount. Returns the deleted id + name on success.
    pub fn delete_function(&self, fn_id_or_name: &str) -> Result<(String, Option<String>), String> {
        let mut guard = self.store.lock().unwrap();
        // Resolve to the canonical id (accept id or name).
        let id = if guard.functions.contains_key(fn_id_or_name) {
            fn_id_or_name.to_string()
        } else if let Some(id) = guard.functions_by_name.get(fn_id_or_name) {
            id.clone()
        } else {
            return Err(format!("no such function: {fn_id_or_name}"));
        };
        // Refuse while mounted: a standalone instance OR a host region guest.
        let mounted_standalone = guard.instances.values().any(|i| i.function_id == id);
        let mounted_as_guest = guard
            .instances
            .values()
            .any(|i| i.region_mounts.values().any(|fid| *fid == id));
        if mounted_standalone || mounted_as_guest {
            return Err(format!(
                "function {id} is still mounted — unmount it (or clear the stage) before deleting"
            ));
        }
        let record = guard
            .functions
            .remove(&id)
            .ok_or_else(|| format!("no such function: {id}"))?;
        if let Some(name) = &record.name {
            // Only drop the name->id index if it still points at THIS id (a
            // re-put under the same name keeps the index; here id matches).
            if guard.functions_by_name.get(name) == Some(&id) {
                guard.functions_by_name.remove(name);
            }
        }
        guard.bundle_cache.retain(|(fid, _), _| *fid != id);
        // PLAN-049 W2 (reviewer W2 P2): cascade to the COLLAPSE — a deleted
        // env-origin function must not leave a dangling tab whose stage-open
        // targets a missing function. Drop any tab referencing this id, via ANY
        // delete path (the tab Close button AND the generic fn-delete card/tool).
        // workbench is the WorkbenchSession inner mutex; lock order LiveStore->WB
        // matches workbench_snapshot, so no deadlock.
        guard.workbench.drop_tabs_for_function(&id);
        Ok((record.id, record.name))
    }

    /// PLAN-049 W1.S3: close an opened environment (drop env + its tabs). Refuses
    /// while any env-origin function bound to this env is still mounted — the
    /// COLLAPSE model (W2) registers tabs as functions, so an open frame from
    /// this env blocks the close. Returns (title, tabs_dropped) on success.
    pub fn close_environment(&self, env_id: &str) -> Result<(String, usize), String> {
        // Hold the LiveStore guard across BOTH the mounted-frame check AND the
        // actual env drop so a concurrent mount cannot interleave between them
        // (reviewer W1 P2). Every mount path (mount_function / mount_into_host)
        // locks LiveStore first; `workbench.close_environment` then locks the
        // WorkbenchSession inner mutex — lock order LiveStore->WB, the same order
        // `workbench_snapshot` already uses, so no deadlock.
        let guard = self.store.lock().unwrap();
        // A function is bound to this env via Origin::Env(env_id).
        let bound_mounted = guard.functions.values().any(|f| {
            f.origin.id() == Some(env_id)
                && guard.instances.values().any(|i| {
                    i.function_id == f.id || i.region_mounts.values().any(|fid| *fid == f.id)
                })
        });
        if bound_mounted {
            return Err(format!(
                "environment {env_id} has a mounted frame — unmount it (or clear the stage) before closing"
            ));
        }
        // Still under the LiveStore guard: drop the env atomically wrt mounts.
        guard
            .workbench
            .close_environment(env_id)
            .ok_or_else(|| format!("no such environment: {env_id}"))
    }

    /// PLAN-049 W2 (COLLAPSE): close a tab — drop the tab record AND delete its
    /// env-origin function (both halves of the collapsed object). If the function
    /// is still mounted, deletion is refused (`delete_function`'s guard) and the
    /// WHOLE close fails so the tab survives with its still-running frame, rather
    /// than orphaning the function. Returns the closed tab id on success.
    pub fn close_tab(&self, tab_id: &str) -> Result<String, String> {
        // Read the tab's function id WITHOUT removing the tab yet, so a refused
        // delete leaves the tab intact (no half-close).
        let workbench = self.store.lock().unwrap().workbench.clone();
        let fid = workbench
            .tab_function_id(tab_id)
            .ok_or_else(|| format!("no such tab: {tab_id}"))?;
        match fid {
            // Deleting the function CASCADES to drop this tab (and any sibling
            // tab sharing the fn — though tab-open is now idempotent, so there is
            // at most one). The mounted-guard gates the whole close: a refused
            // delete leaves both the function and the tab intact.
            Some(fid) => {
                self.delete_function(&fid)?;
            }
            // Legacy/pre-collapse tab with no function — just drop the record.
            None => {
                workbench
                    .close_tab(tab_id)
                    .ok_or_else(|| format!("no such tab: {tab_id}"))?;
            }
        }
        Ok(tab_id.to_string())
    }

    pub fn record_event(&self, id: &str, raw: Value) -> Result<LiveEvent, String> {
        let event_type = raw
            .get("type")
            .or_else(|| raw.get("event"))
            .and_then(Value::as_str)
            .unwrap_or("message")
            .to_string();
        let payload = raw.get("payload").cloned().unwrap_or_else(|| raw.clone());
        let correlation = raw
            .get("correlation")
            .and_then(Value::as_str)
            .map(str::to_string);

        let mut guard = self.store.lock().unwrap();
        if !guard.instances.contains_key(id) {
            return Err(format!("no such instance: {id}"));
        }
        // PLAN-045 audience: an `mcp-action` is routed by the registry (gated by
        // the instance's agent-control declaration); any other envelope (a bare
        // `message`/`clicked` a page sends) is agent-addressed by default. The
        // Reject verdict still records the event (history) but as Server-audience
        // so it never reaches `st_await` — an unknown action must not wake the agent.
        let audience = if event_type == "mcp-action" {
            let agent_control = guard
                .instances
                .get(id)
                .map(|instance| instance.agent_control)
                .unwrap_or(false);
            let action = payload.get("action").and_then(Value::as_str).unwrap_or("");
            match actions::route(action, agent_control) {
                actions::Route::Agent => actions::Audience::Agent,
                actions::Route::Server | actions::Route::Reject => actions::Audience::Server,
            }
        } else {
            actions::Audience::Agent
        };
        let seq = guard.next_event;
        guard.next_event += 1;
        let event = LiveEvent {
            seq,
            instance_id: id.to_string(),
            event_type,
            payload,
            correlation,
            audience,
            raw,
        };
        if let Some(instance) = guard.instances.get_mut(id) {
            instance.events.push(event.clone());
        }
        Ok(event)
    }

    pub fn read_instance_events(&self, id: &str) -> Option<Value> {
        let guard = self.store.lock().unwrap();
        let instance = guard.instances.get(id)?;
        let latest_seq = instance.events.last().map(|event| event.seq).unwrap_or(0);
        Some(json!({
            "schema_version": 1,
            "instance_id": instance.id,
            "function_id": instance.function_id,
            "function_revision": instance.function_revision,
            "title": instance.title,
            "latest_seq": latest_seq,
            "await_cursor": instance.await_cursor,
            "events_resource": instance_events_uri(&instance.id),
            "events": instance.events.iter().map(LiveEvent::to_json).collect::<Vec<_>>(),
        }))
    }

    /// Block (poll) until the next unconsumed event for an instance arrives.
    pub fn await_event(&self, id: &str, deadline: std::time::Instant) -> Option<LiveEvent> {
        loop {
            {
                let mut guard = self.store.lock().unwrap();
                let instance = guard.instances.get_mut(id)?;
                // PLAN-045: deliver only AGENT-audience events. Server events
                // (navigation/inspection, already resolved at the sink) are
                // skipped — the cursor advances PAST them so they neither wake the
                // agent nor block a later agent event. They remain in `events`
                // history for the comms drawer.
                if let Some(event) = instance
                    .events
                    .iter()
                    .filter(|event| event.seq > instance.await_cursor)
                    .find(|event| event.audience == actions::Audience::Agent)
                    .cloned()
                {
                    instance.await_cursor = event.seq;
                    return Some(event);
                }
                // No agent event yet: advance the cursor past any trailing
                // server events so the poll does not re-scan them each tick.
                if let Some(max_seq) = instance.events.last().map(|event| event.seq)
                    && max_seq > instance.await_cursor
                {
                    instance.await_cursor = max_seq;
                }
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
    }
}

// ---------------------------------------------------------------------------
// Generic Function Environment HTTP handlers
// ---------------------------------------------------------------------------

/// FUP-110 / friction fix: ONE stable entry point for the workbench, so an
/// agent or a human never has to hunt for the freshly-minted `inst-N` id the
/// old flow required (register -> mount -> scrape the returned URL, every
/// single run). `GET /__mcp/workbench` (and its `/__spacetime/workbench`
/// alias, wired into the regular dev server too) idempotently:
///   1. registers the workbench shell (`stdlib/__mcp__/env.st`) as the
///      `mcp-workbench` function if it isn't already (re-registering the SAME
///      source is a no-op revision bump caught by the contract-hash check);
///   2. reuses the FIRST existing instance of that function if one is already
///      mounted (no duplicate workbench pages piling up across requests);
///   3. else mounts a fresh one;
/// then 302s to that instance's page. The redirect target is a RELATIVE path
/// (not the absolute `http://127.0.0.1:{port}` baked into `instance.url`) so it
/// still works when reached through a different host/proxy in front of the
/// same port.
async fn workbench_redirect(State(store): State<Store>) -> Response {
    let (workspace_root, port) = {
        let guard = store.lock().unwrap();
        (guard.workbench.workspace_root(), guard.port)
    };
    let server = LiveServer {
        port,
        store: store.clone(),
    };

    let Some(source) = super::stdlib_source::resolve_mcp_stdlib_source(&workspace_root, "env.st")
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "workbench source env.st not found (embedded or on disk)",
        )
            .into_response();
    };
    let contract = json!({
        "version": 1,
        "name": "mcp-workbench",
        "input": { "kind": "mcp-workbench-snapshot" },
        "events": [ { "type": "mcp-action", "payload": { "kind": "json" } } ],
        "source": "stdlib/__mcp__/env.st",
    });
    let outcome = server.put_source(
        Some("mcp-workbench".to_string()),
        source.into_owned(),
        contract,
        workspace_root.clone(),
        super::origin::Origin::Stdlib,
    );
    let function_id = outcome.record.id.clone();
    if !outcome.record.compile_ok {
        let diags = outcome
            .record
            .diagnostics
            .iter()
            .map(|d| format!("\n  x [{}] {}", d.code, d.message))
            .collect::<String>();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("workbench shell failed to compile:{diags}"),
        )
            .into_response();
    }

    let existing_id = {
        let guard = store.lock().unwrap();
        guard
            .instances
            .values()
            .find(|i| i.function_id == function_id)
            .map(|i| i.id.clone())
    };
    let instance_id = match existing_id {
        Some(id) => id,
        None => {
            // The initial `input` value is a placeholder: `instance_input_json`
            // (the page's poll target) recomputes the LIVE snapshot on every
            // request for the workbench function specifically (`is_workbench`
            // branch below), so this seed is only ever visible for the instant
            // before the page's first poll lands.
            let input = { store.lock().unwrap().workbench_snapshot() };
            match server.mount_function(
                &function_id,
                Some("Spacetime MCP Workbench".to_string()),
                input,
                None,
                false,
            ) {
                Ok(instance) => instance.id,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
                }
            }
        }
    };

    axum::response::Redirect::to(&format!("/__mcp/instance/{instance_id}/")).into_response()
}
async fn instance_page(State(store): State<Store>, Path(id): Path<String>) -> Response {
    let (instance, function) = {
        let guard = store.lock().unwrap();
        let Some(instance) = guard.instances.get(&id).cloned() else {
            return (StatusCode::NOT_FOUND, "no such instance").into_response();
        };
        let Some(function) = guard.functions.get(&instance.function_id).cloned() else {
            return (StatusCode::NOT_FOUND, "instance function missing").into_response();
        };
        (instance, function)
    };
    // BUG-116: a template-only function compiles to EMPTY page html (file-scope
    // markup is ignored in v1; the content lives in `@template &main()`). A bare
    // standalone page would then render an empty <body>. When there's no page
    // html, synthesize an AUTO-STAGE region root: `data-st-mcp-autostage` tells
    // the runtime (templates.js initAutoStages) to poll this instance's /bundles
    // and direct-mount `main` into the root — the same self-describing mount the
    // host stage does, minus the chrome. A function WITH page html keeps its
    // current behaviour (render the html, attrs injected).
    let region_body = if function.html.trim().is_empty() {
        format!(
            "<main data-st-instance=\"{instance_id}\" data-st-origin=\"Stdlib\" \
             data-st-region=\"{region_id}\" data-st-mcp-autostage></main>",
            instance_id = html_escape(&instance.id),
            region_id = html_escape(&instance.region_id),
        )
    } else {
        inject_region_root_attrs(&function.html, &instance.id, "Stdlib", &instance.region_id)
    };
    let html = format!(
        "<!DOCTYPE html>\n<html lang=\"en\" data-st-mcp-instance=\"{instance_id}\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title}</title>\n\
<link rel=\"stylesheet\" href=\"/__mcp/instance/{instance_id}/css\">\n</head>\n<body>\n\
{body}\n<script src=\"/__mcp/instance/{instance_id}/js\"></script>\n</body>\n</html>",
        instance_id = html_escape(&instance.id),
        title = html_escape(&instance.title),
        body = region_body,
    );
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, must-revalidate"),
        ],
        html,
    )
        .into_response()
}

async fn instance_asset(
    State(store): State<Store>,
    Path((id, asset_path)): Path<(String, String)>,
) -> Response {
    let resolve_root = {
        let guard = store.lock().unwrap();
        let Some(instance) = guard.instances.get(&id) else {
            return (StatusCode::NOT_FOUND, "no such instance").into_response();
        };
        let Some(function) = guard.functions.get(&instance.function_id) else {
            return (StatusCode::NOT_FOUND, "instance function missing").into_response();
        };
        function.resolve_root.clone()
    };

    let Some(file_path) = safe_asset_path(&resolve_root, &asset_path) else {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    };
    let mime = guess_asset_mime(&asset_path);
    match std::fs::read(&file_path) {
        Ok(content) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime),
                (header::CACHE_CONTROL, "no-store"),
            ],
            content,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

/// BUG-164: joins `root` + `rel` for the instance static-asset route, rejecting
/// any `..`/empty path component so a mounted function's markup can never read
/// outside its own `resolve_root` (env root or workspace root). Pulled out as a
/// plain sync fn so the traversal guard is unit-testable without an axum test
/// server.
fn safe_asset_path(root: &std::path::Path, rel: &str) -> Option<std::path::PathBuf> {
    if rel.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return None;
    }
    Some(root.join(rel))
}

fn guess_asset_mime(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".glb") {
        "model/gltf-binary"
    } else if lower.ends_with(".gltf") {
        "model/gltf+json"
    } else if lower.ends_with(".bin") || lower.ends_with(".draco") {
        "application/octet-stream"
    } else if lower.ends_with(".ktx2") {
        "image/ktx2"
    } else if lower.ends_with(".hdr") {
        "image/vnd.radiance"
    } else if lower.ends_with(".exr") {
        "image/aces"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".otf") {
        "font/otf"
    } else if lower.ends_with(".ttf") {
        "font/ttf"
    } else if lower.ends_with(".woff2") {
        "font/woff2"
    } else if lower.ends_with(".woff") {
        "font/woff"
    } else if lower.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
}

async fn instance_input_json(State(store): State<Store>, Path(id): Path<String>) -> Response {
    let guard = store.lock().unwrap();
    let Some(instance) = guard.instances.get(&id) else {
        return (StatusCode::NOT_FOUND, "no such instance").into_response();
    };
    let Some(function) = guard.functions.get(&instance.function_id) else {
        return (StatusCode::NOT_FOUND, "instance function missing").into_response();
    };
    // M2 (PLAN-046): the workbench HOST page's input is the LIVE environment
    // snapshot, not the frozen copy captured at mount. Without this, functions
    // registered after st_workbench (the whole gallery) never appear — the page
    // polls a stale file. Any other instance serves its own (static) input. The
    // host is identified by its function name (`mcp-workbench`), the single
    // self-describing meta-renderer page.
    let is_workbench = function.name.as_deref() == Some("mcp-workbench");
    let input = if is_workbench {
        guard.workbench_snapshot()
    } else {
        instance.input.clone()
    };
    let data = json!({
        "instance_id": instance.id,
        "function_id": instance.function_id,
        "function_revision": instance.function_revision,
        "input": input,
        "contract": function.contract,
    });
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        serde_json::to_string(&data).unwrap_or_default(),
    )
        .into_response()
}

async fn instance_css(State(store): State<Store>, Path(id): Path<String>) -> Response {
    let guard = store.lock().unwrap();
    match guard
        .instances
        .get(&id)
        .and_then(|instance| guard.functions.get(&instance.function_id))
    {
        Some(function) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
            function.css.clone(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "/* no such instance */").into_response(),
    }
}

async fn instance_js(State(store): State<Store>, Path(id): Path<String>) -> Response {
    let guard = store.lock().unwrap();
    match guard
        .instances
        .get(&id)
        .and_then(|instance| guard.functions.get(&instance.function_id))
    {
        Some(function) => (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            )],
            function.js.clone(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "// no such instance").into_response(),
    }
}

/// GET /__mcp/instance/{id}/bundles — region-mountable bundle set for this instance.
///
/// FEAT-126: compiles the mounted function's source into a [`Bundle`] and returns
/// the templates + css the page needs to register/mount the region. The page polls
/// this endpoint (alongside input.json) and calls `Spacetime.registerBundle`.

/// BUG-118: test-only counter of how many times `region_bundle_stable` actually
/// COMPILED (cache miss). Lets a unit test prove that N polls of an unchanged
/// function compile at most once. Compiled out of release builds.
#[cfg(test)]
pub(crate) static REGION_BUNDLE_COMPILES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// Return the revision-stable region-bundle JSON for `(fn_id, revision)`, serving
/// from `bundle_cache` on a hit and compiling-then-caching on a miss (BUG-118).
///
/// The returned JSON carries only the parts that DON'T vary per request (css /
/// templates / diagnostics / has_errors); the caller merges ids + region input.
/// The compile runs WITHOUT the store lock held (the pipeline is the expensive
/// part; concurrent requests must not block on it). `Err` is the compile error
/// message (a parse/import failure with no usable AST).
fn region_bundle_stable(
    store: &Store,
    fn_id: &str,
    revision: u64,
    source: &str,
    resolve_root: &std::path::Path,
) -> Result<Value, String> {
    let cache_key = (fn_id.to_string(), revision);
    if let Some(hit) = store.lock().unwrap().bundle_cache.get(&cache_key).cloned() {
        return Ok(hit);
    }
    let bundle = compile_to_bundle(source, resolve_root).map_err(|e| e.message)?;
    #[cfg(test)]
    REGION_BUNDLE_COMPILES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let templates: Vec<Value> = bundle
        .templates
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "params": t.params,
                "body": {
                    "html": t.body.html,
                    "serialized": t.body.serialized,
                    "builder": t.body.builder,
                },
                "animations": t.animations,
            })
        })
        .collect();
    let stable = json!({
        "css": bundle.css,
        "templates": templates,
        "diagnostics": bundle.diagnostics.iter().map(|d| json!({
            "severity": d.severity,
            "code": d.code,
            "message": d.message,
        })).collect::<Vec<_>>(),
        "has_errors": bundle.has_errors,
    });
    store
        .lock()
        .unwrap()
        .bundle_cache
        .insert(cache_key, stable.clone());
    Ok(stable)
}

async fn instance_bundles(
    State(store): State<Store>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    // FEAT-126 / BUG-111: when the host page polls `?region={r}`, serve the GUEST
    // function composed into that region (via `region_mounts`), not the host's
    // own function. A region with nothing mounted returns an empty bundle so the
    // host stage stays on its empty-state rather than 404-ing the poll loop.
    let region = params.get("region").map(String::as_str);
    let (instance, function) = {
        let guard = store.lock().unwrap();
        let Some(instance) = guard.instances.get(&id).cloned() else {
            return (StatusCode::NOT_FOUND, "no such instance").into_response();
        };
        // Resolve which function this bundle request targets.
        let target_fid = match region {
            Some(r) => match instance.region_mounts.get(r) {
                Some(fid) => fid.clone(),
                None => {
                    // No guest mounted in this region yet -> empty bundle.
                    return (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
                            (header::CACHE_CONTROL, "no-store"),
                        ],
                        serde_json::to_string(&json!({
                            "schema_version": 1,
                            "instance_id": instance.id,
                            "region_id": r,
                            "origin": "Stdlib",
                            "css": "",
                            "templates": [],
                            "diagnostics": [],
                            "has_errors": false,
                            "empty": true,
                        }))
                        .unwrap_or_default(),
                    )
                        .into_response();
                }
            },
            None => instance.function_id.clone(),
        };
        let Some(function) = guard.functions.get(&target_fid).cloned() else {
            return (StatusCode::NOT_FOUND, "instance function missing").into_response();
        };
        (instance, function)
    };

    // BUG-118: serve the revision-stable bundle JSON from cache; compile ONLY on
    // a cache miss (first poll of a (fn, revision) pair). The `@mcp-region` poll
    // hits this at 1Hz — recompiling the guest's full source every time pinned a
    // core and stalled HTTP. The compiled bundle is immutable for a revision, so
    // we cache the parts that don't vary per request (css / templates /
    // diagnostics / has_errors) and merge the per-request bits (ids + region
    // input) fresh below.
    let cached = region_bundle_stable(
        &store,
        &function.id,
        function.revision,
        &function.source,
        &function.resolve_root,
    );
    match cached {
        Ok(stable) => {
            // M4: param overrides for this region (inspector edits + BUG-113 mount
            // input), seeded onto the guest root by registerBundle so `$param`
            // holes reflect edits. This is the only per-request-varying piece.
            let region_input = region
                .and_then(|r| instance.region_mount_input.get(r))
                .cloned()
                .unwrap_or_default();
            let data = json!({
                "schema_version": 1,
                "instance_id": instance.id,
                "region_id": region.unwrap_or(&instance.region_id),
                "origin": "Stdlib",
                "css": stable.get("css").cloned().unwrap_or(Value::Null),
                "templates": stable.get("templates").cloned().unwrap_or_else(|| json!([])),
                "input": region_input,
                "diagnostics": stable.get("diagnostics").cloned().unwrap_or_else(|| json!([])),
                "has_errors": stable.get("has_errors").cloned().unwrap_or(Value::Bool(false)),
            });
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/json; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-store"),
                ],
                serde_json::to_string(&data).unwrap_or_default(),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            serde_json::to_string(&json!({
                "ok": false,
                "error": err,
            }))
            .unwrap_or_default(),
        )
            .into_response(),
    }
}

async fn signal_sink(
    State(store): State<Store>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let raw = parse_signal_body(&headers, &body);
    let server = LiveServer {
        port: 0,
        store: store.clone(),
    };
    let event = match server.record_event(&id, raw) {
        Ok(event) => event,
        Err(_) => return (StatusCode::NOT_FOUND, "no such instance").into_response(),
    };

    let action = dispatch_signal_action(&store, &event);

    eprintln!(
        "[spacetime-mcp] instance {} emitted {}#{}",
        event.instance_id, event.event_type, event.seq
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        serde_json::to_string(&json!({ "ok": true, "event": event.to_json(), "action": action }))
            .unwrap(),
    )
        .into_response()
}

fn dispatch_signal_action(store: &Store, event: &LiveEvent) -> Value {
    if event.event_type != "mcp-action" {
        return Value::Null;
    }

    let action = event
        .payload
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    if action.is_empty() {
        return json!({ "ok": false, "action": action, "error": "mcp-action missing payload.action" });
    }

    // Route by the action registry (the SINGLE source of truth). Server actions
    // resolve synchronously below; Agent actions are NOT resolved here — they flow
    // to `st_await` as the event the agent reads (audience filtering in
    // `await_event` keeps server actions out of that stream). Unknown actions are
    // rejected unless the instance declares itself agent-control.
    let agent_control = store
        .lock()
        .unwrap()
        .instances
        .get(&event.instance_id)
        .map(|instance| instance.agent_control)
        .unwrap_or(false);
    match actions::route(action, agent_control) {
        actions::Route::Agent => {
            // The agent reads this via st_await; the server does not resolve it.
            return Value::Null;
        }
        actions::Route::Reject => {
            return json!({ "ok": false, "action": action, "error": format!("unsupported mcp-action: {action}") });
        }
        actions::Route::Server => {}
    }

    let workbench = store.lock().unwrap().workbench.clone();
    let result = match action {
        "mcp_refresh" | "st_workbench_refresh" => {
            Ok(("Refreshed MCP workbench snapshot.".to_string(), json!({})))
        }
        "st_env_open" => {
            let Some(path) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "st_env_open action missing target path" });
            };
            open_environment_action(&workbench, path)
        }
        "st_tab_open" => open_tab_action(store, &event.payload),
        "mcp_compose" => {
            // M2: gallery-click composes a function into THIS host's region.
            // `event.instance_id` is the originating host page; `target` is the
            // function id/name; `region` defaults to "stage".
            let Some(function_key) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "mcp_compose action missing target function" });
            };
            let region = event
                .payload
                .get("region")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("stage");
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            match server.mount_into_host(&event.instance_id, region, function_key, &Value::Null) {
                Ok(host) => Ok((
                    format!(
                        "Composed {function_key} into {} region '{region}'.",
                        event.instance_id
                    ),
                    json!({ "host": host.to_json(), "region": region }),
                )),
                Err(e) => Err(e),
            }
        }
        "mcp_param" => {
            // M4: set a param override on the guest composed in THIS host's region.
            // `target` = param name, `value` = new value, `region` default "stage".
            let Some(param) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "mcp_param action missing target param name" });
            };
            let value = event
                .payload
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let region = event
                .payload
                .get("region")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("stage")
                .to_string();
            let mut guard = store.lock().unwrap();
            match guard.instances.get_mut(&event.instance_id) {
                Some(host) => {
                    host.region_mount_input
                        .entry(region.clone())
                        .or_default()
                        .insert(param.to_string(), value.clone());
                    Ok((
                        format!("Set {param} = {value:?} on region '{region}'."),
                        json!({ "region": region, "param": param, "value": value }),
                    ))
                }
                None => Err(format!("no such host instance: {}", event.instance_id)),
            }
        }
        "mcp_source_edit" => {
            // M6: save edited source for the guest composed on THIS host's stage.
            // `value` = the new source. Resolve the stage guest fn from the host's
            // region_mounts, then update_source (recompile + hot-swap).
            let Some(new_source) = event.payload.get("value").and_then(Value::as_str) else {
                return json!({ "ok": false, "action": action, "error": "mcp_source_edit action missing value (new source)" });
            };
            let region = event
                .payload
                .get("region")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("stage")
                .to_string();
            let fid = {
                let guard = store.lock().unwrap();
                guard
                    .instances
                    .get(&event.instance_id)
                    .and_then(|h| h.region_mounts.get(&region).cloned())
            };
            match fid {
                Some(fid) => {
                    let server = LiveServer {
                        port: 0,
                        store: store.clone(),
                    };
                    match server.update_source(&fid, new_source) {
                        Ok((rev, _)) => Ok((
                            format!("Saved {fid} r{rev} from inline edit on region '{region}'."),
                            json!({ "function_id": fid, "revision": rev, "region": region }),
                        )),
                        Err(e) => Err(e),
                    }
                }
                None => Err(format!("no guest mounted in region '{region}' to edit")),
            }
        }
        "mcp_node_param" => {
            // PLAN-051 B4: edit ONE template-invocation's named arg IN the guest
            // SOURCE, addressed by its G2 invoke_span. `target` = param name,
            // `value` = new value, `span_start`/`span_end` = the invocation's byte
            // span (from the structure node's invoke_span), `region` default
            // "stage". Patches the source via the SHARED apply_text_patch (the same
            // patcher EditAst uses), then update_source recompiles + hot-swaps the
            // guest. This is the inspector's per-instance WRITE on the G2 rail.
            let Some(param) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "mcp_node_param missing target param name" });
            };
            let value = event
                .payload
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            // span_start/span_end arrive via the data-st-payload-* rail as STRINGS
            // (dataset values) or as numbers (direct JSON) — accept both.
            let parse_span = |v: Option<&Value>| -> Option<usize> {
                match v {
                    Some(Value::Number(n)) => n.as_u64().map(|x| x as usize),
                    Some(Value::String(s)) => s.trim().parse::<usize>().ok(),
                    _ => None,
                }
            };
            let span_start = parse_span(event.payload.get("span_start"));
            let span_end = parse_span(event.payload.get("span_end"));
            let (Some(span_start), Some(span_end)) = (span_start, span_end) else {
                return json!({ "ok": false, "action": action, "error": "mcp_node_param needs span_start + span_end (the invocation's invoke_span)" });
            };
            let region = event
                .payload
                .get("region")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("stage")
                .to_string();
            // Resolve the stage guest fn + its current source.
            let (fid, source) = {
                let guard = store.lock().unwrap();
                let fid = guard
                    .instances
                    .get(&event.instance_id)
                    .and_then(|h| h.region_mounts.get(&region).cloned());
                match fid {
                    Some(fid) => {
                        let src = guard.functions.get(&fid).map(|f| f.source.clone());
                        (Some(fid), src)
                    }
                    None => (None, None),
                }
            };
            let (Some(fid), Some(source)) = (fid, source) else {
                return json!({ "ok": false, "action": action, "error": format!("no guest mounted in region '{region}' to edit") });
            };
            // The value the author typed is the BARE value; the patcher re-quotes it
            // to match what the source already had (a quoted arg stays quoted, a
            // signal ref/number stays bare) — see `emit_patch_value` in
            // src/sync/handlers.rs, which owns that rule for EVERY rail. This call
            // site used to pre-quote here, which hid the fact that the patcher wrote
            // template-invocation args UNQUOTED for its other caller (the EditAst
            // disk rail) — PLAN-112 W1 moved the rule down into the patcher.
            let patch = json!({ param: value.clone() });
            match crate::sync::handlers::apply_text_patch(&source, span_start, span_end, &patch) {
                Ok(new_source) => {
                    let server = LiveServer {
                        port: 0,
                        store: store.clone(),
                    };
                    match server.update_source(&fid, &new_source) {
                        Ok((rev, _)) => Ok((
                            format!("Set {param} = {value:?} on a {region} node (r{rev})."),
                            json!({ "function_id": fid, "revision": rev, "region": region, "param": param, "value": value }),
                        )),
                        Err(e) => Err(e),
                    }
                }
                Err(e) => Err(format!("node-param patch failed: {e}")),
            }
        }
        "mcp_env_close" => {
            // PLAN-049 W1.S3: close an opened environment (drop it + its tabs).
            // `target` = the env id. Refuses while a frame from it is mounted.
            let Some(env_id) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "mcp_env_close action missing target env id" });
            };
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            match server.close_environment(env_id) {
                Ok((title, tabs)) => Ok((
                    format!("Closed environment {env_id} ({title}); dropped {tabs} tab(s)."),
                    json!({ "env": env_id, "title": title, "tabs_dropped": tabs }),
                )),
                Err(e) => Err(e),
            }
        }
        "mcp_create" => {
            // PLAN-049 W3: the "+" creation surface. `payload.values` carries the
            // collected form fields { create_name?, create_source }. Register the
            // source as an Agent-origin function, then compose it onto THIS host's
            // stage so the human's new frame appears immediately.
            let values = event.payload.get("values");
            let source = values
                .and_then(|v| v.get("create_source"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(source) = source else {
                return json!({ "ok": false, "action": action, "error": "mcp_create needs a non-empty create_source" });
            };
            let name = values
                .and_then(|v| v.get("create_name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            let contract = json!({
                "version": 1,
                "name": name.clone().unwrap_or_else(|| "scratch".to_string()),
                "input": { "kind": "json" },
                "events": [ { "type": "message", "payload": { "kind": "json" } } ],
                "source": "declared-fallback",
            });
            let workspace_root = store.lock().unwrap().workbench.workspace_root();
            let outcome = server.put_source(
                name,
                source.to_string(),
                contract,
                workspace_root,
                super::origin::Origin::Agent,
            );
            let fid = outcome.record.id.clone();
            if !outcome.record.compile_ok {
                let diags = outcome
                    .record
                    .diagnostics
                    .iter()
                    .map(|d| format!("\n  x [{}] {}", d.code, d.message))
                    .collect::<String>();
                return json!({ "ok": false, "action": action, "error": format!("created {fid} but it did not compile — fix + retry:{diags}") });
            }
            // Compose the fresh frame onto the originating host's stage — but
            // ONLY when the originator is the workbench host (the page with a
            // `stage` region rendered by `@mcp-region`). A standalone instance
            // auto-stages by polling its OWN bundle, not `?region=stage`, so
            // writing region_mounts["stage"] there would report a phantom compose
            // (reviewer W3 P2). Off the workbench, the frame is registered into
            // the gallery and `composed:false` is reported honestly.
            let host_is_workbench = {
                let guard = store.lock().unwrap();
                guard
                    .instances
                    .get(&event.instance_id)
                    .and_then(|i| guard.functions.get(&i.function_id))
                    .and_then(|f| f.name.as_deref())
                    == Some("mcp-workbench")
            };
            if !host_is_workbench {
                return json!({
                    "ok": true,
                    "action": action,
                    "summary": format!("Created {fid} (in the gallery); compose skipped (not a stage host)."),
                    "result": json!({ "function_id": fid, "composed": false }),
                });
            }
            match server.mount_into_host(&event.instance_id, "stage", &fid, &Value::Null) {
                Ok(_) => Ok((
                    format!("Created {fid} and composed it onto the stage."),
                    json!({ "function_id": fid, "composed": true }),
                )),
                // Registered but compose failed (e.g. no @template): still a win,
                // the frame is in the gallery.
                Err(e) => Ok((
                    format!("Created {fid} (in the gallery); compose skipped: {e}"),
                    json!({ "function_id": fid, "composed": false }),
                )),
            }
        }
        "mcp_tab_close" => {
            // PLAN-049 W2 (COLLAPSE): close a tab — drop the tab + delete its
            // env-origin function. `target` = the tab id.
            let Some(tab_id) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "mcp_tab_close action missing target tab id" });
            };
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            match server.close_tab(tab_id) {
                Ok(id) => Ok((format!("Closed tab {id}."), json!({ "tab": id }))),
                Err(e) => Err(e),
            }
        }
        "mcp_fn_delete" => {
            // PLAN-049 W1.S3: delete a registered function. `target` = fn id/name.
            // Refuses while the function is still mounted.
            let Some(key) = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            else {
                return json!({ "ok": false, "action": action, "error": "mcp_fn_delete action missing target function" });
            };
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            match server.delete_function(key) {
                Ok((id, name)) => Ok((
                    format!(
                        "Deleted function {id}{}.",
                        name.as_deref()
                            .map(|n| format!(" ({n})"))
                            .unwrap_or_default()
                    ),
                    json!({ "function_id": id, "name": name }),
                )),
                Err(e) => Err(e),
            }
        }
        "mcp_stage_clear" => {
            // PLAN-049 W1.S2: clear the originating host's region (default
            // "stage") — drop the composed guest so the region empties. The
            // region rides in `region` or `target` (the @mcp-action `target:`).
            let region = event
                .payload
                .get("region")
                .and_then(Value::as_str)
                .or_else(|| event.payload.get("target").and_then(Value::as_str))
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("stage")
                .to_string();
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            match server.clear_region(&event.instance_id, &region) {
                Ok(prior) => Ok((
                    match &prior {
                        Some(fid) => format!("Cleared {fid} from region '{region}'."),
                        None => format!("Region '{region}' was already empty."),
                    },
                    json!({ "region": region, "cleared": prior }),
                )),
                Err(e) => Err(e),
            }
        }
        "mcp_unmount" => {
            // PLAN-049 W1.S1: unmount a live instance. `target` = the instance id
            // to remove (defaults to the originating instance if omitted, so a
            // page's own "close me" works). Clears any host region it occupied.
            let target = event
                .payload
                .get("target")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(event.instance_id.as_str())
                .to_string();
            let server = LiveServer {
                port: 0,
                store: store.clone(),
            };
            match server.unmount_instance(&target) {
                Ok((id, title)) => Ok((
                    format!("Unmounted {id} ({title})."),
                    json!({ "instance_id": id, "title": title }),
                )),
                Err(e) => Err(e),
            }
        }
        // Unreachable: route() only returns Server for registered server actions,
        // all of which are matched above. A new server action MUST add its arm here.
        other => {
            return json!({ "ok": false, "action": other, "error": format!("server action has no resolver: {other}") });
        }
    };

    let mut guard = store.lock().unwrap();
    let snapshot = guard.workbench_snapshot();
    let updated_instance = guard.instances.get_mut(&event.instance_id).map(|instance| {
        instance.input = snapshot.clone();
        instance.to_json()
    });

    let action_json = match result {
        Ok((summary, data)) => json!({
            "ok": true,
            "action": action,
            "summary": summary,
            "result": data,
            "snapshot": snapshot,
            "updated_instance": updated_instance,
        }),
        Err(error) => json!({
            "ok": false,
            "action": action,
            "error": error,
            "snapshot": snapshot,
            "updated_instance": updated_instance,
        }),
    };

    if let Some(instance) = guard.instances.get_mut(&event.instance_id)
        && let Some(stored) = instance
            .events
            .iter_mut()
            .find(|stored| stored.seq == event.seq)
        && let Some(raw) = stored.raw.as_object_mut()
    {
        raw.insert("_mcp_action".to_string(), action_json.clone());
    }

    action_json
}

fn open_environment_action(
    workbench: &WorkbenchSession,
    path: &str,
) -> Result<(String, Value), String> {
    let root = resolve_under_root(&workbench.workspace_root(), path);
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
    let env = workbench.open_environment(root, title);
    Ok((
        format!("Environment {} → {}", env.id, env.title),
        json!({
            "id": env.id,
            "root": env.root.display().to_string(),
            "title": env.title,
        }),
    ))
}

/// PLAN-049 W2 (COLLAPSE): the st_tab_open SINK path. Same semantics as the
/// `st_tab_open` tool: register the entry/inline source as an env-origin
/// FUNCTION (gallery-visible, stage-mountable), then record a Tab carrying its
/// fn id. Shares `LiveServer::put_source` with the tool path — one compile+
/// register impl, one object model.
fn open_tab_action(store: &Store, payload: &Value) -> Result<(String, Value), String> {
    let workbench = store.lock().unwrap().workbench.clone();
    let env_id = payload
        .get("env")
        .and_then(Value::as_str)
        .or_else(|| payload.get("target").and_then(Value::as_str))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "st_tab_open action missing env/target".to_string())?
        .to_string();
    let env = workbench
        .environment(&env_id)
        .ok_or_else(|| format!("no such environment: {env_id}"))?;
    let entry = payload
        .get("entry")
        .and_then(Value::as_str)
        .or_else(|| payload.get("value").and_then(Value::as_str))
        .filter(|s| !s.trim().is_empty());
    let code = payload
        .get("code")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty());
    let title_arg = payload
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty());

    // `entry_path` carries the on-disk `.st` file (PLAN-066: threaded into the
    // compile step so it can detect a sibling `index.html` shell) — `None` for
    // inline code, which has no on-disk sibling to check.
    let (source, title, tab_source, entry_path) = match (entry, code) {
        (Some(_), Some(_)) => return Err("provide entry OR code, not both".into()),
        (None, None) => return Err("provide entry or code".into()),
        (Some(entry), None) => {
            let file = resolve_under_root(&env.root, entry);
            if !file.is_file() {
                return Err(format!("no such file: {}", file.display()));
            }
            let source = std::fs::read_to_string(&file)
                .map_err(|e| format!("could not read {}: {e}", file.display()))?;
            let title = title_arg
                .map(str::to_string)
                .unwrap_or_else(|| entry.to_string());
            (source, title, TabSource::File(file.clone()), Some(file))
        }
        (None, Some(code)) => {
            let title = title_arg
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

    // Register the source as an env-origin function (shared put_source).
    let fn_name = format!("{env_id}/{title}");
    let default_contract = json!({
        "version": 1,
        "name": fn_name,
        "input": { "kind": "json" },
        "events": [ { "type": "message", "payload": { "kind": "json" } } ],
        "source": "declared-fallback",
    });
    let server = LiveServer {
        port: 0,
        store: store.clone(),
    };
    let outcome = server.put_source_with_entry(
        Some(fn_name),
        source,
        default_contract,
        env.root.clone(),
        super::origin::Origin::Env(env_id.clone()),
        entry_path.as_deref(),
    );
    let fn_id = outcome.record.id.clone();
    let compile_ok = outcome.record.compile_ok;
    // PLAN-066: surface warning diagnostics (MCP-LOCALE / MCP-SHELL) on a clean
    // compile too, not just failures — the browser toast + returned data must
    // tell the truth about an incomplete composed preview.
    let diagnostics: Vec<Value> = outcome
        .record
        .diagnostics
        .iter()
        .map(OptionDiagnostic::to_json)
        .collect();

    let tab = workbench.open_tab(&env_id, title, tab_source, Some(fn_id.clone()));
    let suffix = if compile_ok {
        if diagnostics.is_empty() {
            " — composable onto the stage".to_string()
        } else {
            format!(
                " — composable onto the stage, with {} warning(s)",
                diagnostics.len()
            )
        }
    } else {
        " (NOT composable until fixed)".to_string()
    };
    Ok((
        format!(
            "Tab {} ({}) registered as {fn_id}{}.",
            tab.id, tab.title, suffix
        ),
        json!({
            "id": tab.id,
            "env": tab.env_id,
            "title": tab.title,
            "function_id": fn_id,
            "compile_ok": compile_ok,
            "diagnostics": diagnostics,
        }),
    ))
}

fn resolve_under_root(root: &std::path::Path, rel: &str) -> std::path::PathBuf {
    let candidate = std::path::Path::new(rel);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

fn parse_signal_body(headers: &HeaderMap, body: &[u8]) -> Value {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if content_type.starts_with("application/json") {
        serde_json::from_slice(body).unwrap_or_else(|_| json!({ "type": "message", "payload": {} }))
    } else {
        // Small form fallback without adding a dependency. Library functions
        // should send JSON envelopes via stdlib/__mcp__/host.st.
        let text = String::from_utf8_lossy(body);
        let mut fields = serde_json::Map::new();
        for pair in text.split('&').filter(|p| !p.is_empty()) {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("").replace('+', " ");
            let val = parts.next().unwrap_or("").replace('+', " ");
            fields.insert(key, Value::String(val));
        }
        let event_type = fields
            .remove("type")
            .or_else(|| fields.remove("event"))
            .unwrap_or_else(|| Value::String("message".into()));
        json!({ "type": event_type, "payload": Value::Object(fields) })
    }
}

/// Inject region-root identity attributes (`data-st-instance`, `data-st-origin`)
/// onto the FIRST element tag of a compiled HTML fragment.
///
/// FEAT-127 preparatory step: every instance page's body root becomes a region
/// root so the `@mcp-host` bridge can resolve `closest('[data-st-instance]')`.
/// FEAT-130 wires actual multi-region markup; this tags the single root that
/// exists today. No-op when the fragment has no element tag (returns as-is).
fn inject_region_root_attrs(
    html: &str,
    instance_id: &str,
    origin: &str,
    region_id: &str,
) -> String {
    let bytes = html.as_bytes();
    // Scan for the first '<' that opens an element tag: '<' followed by an
    // ASCII letter (skip comments/declarations like `<!--`, `<!DOCTYPE`).
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
            break;
        }
        i += 1;
    }
    if i >= bytes.len() {
        return html.to_string(); // no element tag; leave untouched
    }
    // End of the tag name: first byte after '<tagname' that is space, '>', or '/'.
    let tag_name_end = {
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b' ' && bytes[j] != b'>' && bytes[j] != b'/' {
            j += 1;
        }
        j
    };
    let attr_insert = format!(
        " data-st-instance=\"{}\" data-st-origin=\"{}\" data-st-region=\"{}\"",
        html_escape(instance_id),
        html_escape(origin),
        html_escape(region_id),
    );
    let mut out = String::with_capacity(html.len() + attr_insert.len());
    out.push_str(&html[..tag_name_end]);
    out.push_str(&attr_insert);
    out.push_str(&html[tag_name_end..]);
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod origin_tests {
    use super::super::origin::Origin;
    use super::*;

    fn record_with(origin: Origin) -> FunctionRecord {
        FunctionRecord {
            id: "fn-1".to_string(),
            name: Some("greet".to_string()),
            revision: 1,
            source: "@template &greet() {}".to_string(),
            contract: serde_json::json!({}),
            html: String::new(),
            css: String::new(),
            js: String::new(),
            compile_ok: true,
            diagnostics: Vec::new(),
            resolve_root: std::path::PathBuf::from("/tmp"),
            origin,
            contract_params: serde_json::json!([]),
            contract_hash: String::new(),
        }
    }

    #[test]
    fn agent_origin_serializes_into_function_json() {
        // FEAT-128: an inline `st_fn_put` (no env) is Agent-origin; the chip the
        // workbench renders reads `origin.kind`/`origin.label` off this JSON.
        let v = record_with(Origin::Agent).to_json();
        assert_eq!(v["origin"]["kind"], "agent");
        assert_eq!(v["origin"]["label"], "Agent");
        assert!(v["origin"]["id"].is_null());
    }

    #[test]
    fn env_origin_carries_env_id() {
        // A function bound to an environment carries that env's id so the UI can
        // filter by environment.
        let v = record_with(Origin::Env("env-3".to_string())).to_json();
        assert_eq!(v["origin"]["kind"], "env");
        assert_eq!(v["origin"]["id"], "env-3");
        assert_eq!(v["origin"]["label"], "Env(env-3)");
    }
}
#[cfg(test)]
mod instance_asset_tests {
    //! BUG-164: MCP instance routes had zero static-asset passthrough, so a
    //! mounted function's relative asset reference (e.g. stdlib/3d's
    //! `@gltf(src: "models/foo.glb")`) 404s under `st_mount` even though the
    //! identical source loads the identical path fine under `cargo run --
    //! serve`. These cover the two pieces of new pure logic added to fix it:
    //! the traversal guard (`safe_asset_path`) and the MIME table
    //! (`guess_asset_mime`) -- the axum route wiring itself is exercised live
    //! (puppeteer against a real mounted instance), not re-proven here.
    use super::*;

    #[test]
    fn resolves_a_plain_relative_asset_under_root() {
        let root = std::path::Path::new("/tmp/env-root");
        let resolved = safe_asset_path(root, "models/step-4.glb");
        assert_eq!(
            resolved,
            Some(std::path::PathBuf::from("/tmp/env-root/models/step-4.glb"))
        );
    }

    #[test]
    fn resolves_a_nested_relative_asset() {
        let root = std::path::Path::new("/tmp/env-root");
        let resolved = safe_asset_path(root, "assets/textures/bronze/albedo.png");
        assert_eq!(
            resolved,
            Some(std::path::PathBuf::from(
                "/tmp/env-root/assets/textures/bronze/albedo.png"
            ))
        );
    }

    #[test]
    fn rejects_a_parent_traversal_component() {
        let root = std::path::Path::new("/tmp/env-root");
        assert_eq!(safe_asset_path(root, "../../etc/passwd"), None);
        assert_eq!(safe_asset_path(root, "models/../../../etc/passwd"), None);
    }

    #[test]
    fn rejects_an_empty_path_segment() {
        // A `//` in the URL (or a bare trailing slash) must not silently
        // collapse into a directory listing / root read.
        let root = std::path::Path::new("/tmp/env-root");
        assert_eq!(safe_asset_path(root, "models//step-4.glb"), None);
        assert_eq!(safe_asset_path(root, ""), None);
    }

    #[test]
    fn mime_table_covers_the_stdlib_3d_asset_set() {
        // Same extension set src/server.rs's catch_all_handler already serves
        // for a dev-served .st page -- the whole point of BUG-164 is that
        // st_mount now matches that behavior.
        assert_eq!(guess_asset_mime("models/foo.glb"), "model/gltf-binary");
        assert_eq!(guess_asset_mime("models/foo.GLB"), "model/gltf-binary");
        assert_eq!(guess_asset_mime("models/foo.gltf"), "model/gltf+json");
        assert_eq!(
            guess_asset_mime("models/foo.bin"),
            "application/octet-stream"
        );
        assert_eq!(guess_asset_mime("textures/foo.ktx2"), "image/ktx2");
        assert_eq!(guess_asset_mime("env/studio.hdr"), "image/vnd.radiance");
        assert_eq!(guess_asset_mime("assets/foo.png"), "image/png");
        assert_eq!(guess_asset_mime("assets/foo.jpg"), "image/jpeg");
        assert_eq!(guess_asset_mime("assets/foo.jpeg"), "image/jpeg");
        assert_eq!(guess_asset_mime("fonts/foo.woff2"), "font/woff2");
        assert_eq!(guess_asset_mime("fonts/foo.ttf"), "font/ttf");
    }

    #[test]
    fn unknown_extension_falls_back_to_octet_stream() {
        assert_eq!(
            guess_asset_mime("models/foo.unknownext"),
            "application/octet-stream"
        );
    }
}
#[cfg(test)]
mod routing_tests {
    //! PLAN-045: server-vs-agent event routing. These exercise the real
    //! `record_event` / `await_event` / `dispatch_signal_action` against an
    //! in-memory store with `port: 0` (no HTTP) — the same machinery the sink
    //! uses, no live server needed.
    use super::*;

    fn test_server_with_instance(agent_control: bool) -> LiveServer {
        let workbench = WorkbenchSession::new(std::path::PathBuf::from("/tmp"));
        let mut instances = HashMap::new();
        instances.insert(
            "inst-1".to_string(),
            InstanceRecord {
                id: "inst-1".to_string(),
                function_id: "fn-1".to_string(),
                function_revision: 1,
                title: "t".to_string(),
                input: json!({}),
                url: String::new(),
                region_id: "region-inst-1".to_string(),
                await_cursor: 0,
                agent_control,
                events: Vec::new(),
                region_mounts: HashMap::new(),
                region_mount_input: HashMap::new(),
            },
        );
        let store: Store = Arc::new(Mutex::new(LiveStore {
            workbench,
            functions: HashMap::new(),
            functions_by_name: HashMap::new(),
            instances,
            next_function: 1,
            next_instance: 2,
            next_event: 1,
            bundle_cache: HashMap::new(),
            port: 0,
        }));
        LiveServer { port: 0, store }
    }

    /// A LiveServer with an EMPTY store — no pre-mounted instance. Used by the
    /// W1.S3 delete/close tests, where the shared `test_server_with_instance`
    /// fixture's pre-seeded inst-1 (function_id "fn-1") would otherwise collide
    /// with the first `put_ok_function` id and spuriously block deletion.
    fn test_server_empty() -> LiveServer {
        let workbench = WorkbenchSession::new(std::path::PathBuf::from("/tmp"));
        let store: Store = Arc::new(Mutex::new(LiveStore {
            workbench,
            functions: HashMap::new(),
            functions_by_name: HashMap::new(),
            instances: HashMap::new(),
            next_function: 1,
            next_instance: 1,
            next_event: 1,
            bundle_cache: HashMap::new(),
            port: 0,
        }));
        LiveServer { port: 0, store }
    }

    /// A LiveServer whose `inst-1` host is a function NAMED `mcp-workbench` — the
    /// stage-capable host. Used by the W3 create test, which gates compose on the
    /// originator being the workbench (reviewer W3 P2). put_function with the name
    /// reuses fn-1 (the pre-seeded inst-1 already points at fn-1).
    fn workbench_host_server() -> LiveServer {
        let server = test_server_with_instance(false);
        server.put_function(
            Some("mcp-workbench".to_string()),
            "@template &main() { <div>host</div> }".to_string(),
            json!({}),
            "<div>host</div>".to_string(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("/tmp"),
            super::super::origin::Origin::Stdlib,
            json!([]),
        );
        // Point inst-1 at the mcp-workbench function id.
        let wb_id = server
            .store
            .lock()
            .unwrap()
            .functions_by_name
            .get("mcp-workbench")
            .cloned()
            .unwrap();
        server
            .store
            .lock()
            .unwrap()
            .instances
            .get_mut("inst-1")
            .unwrap()
            .function_id = wb_id;
        server
    }

    fn mcp_action(action: &str, extra: Value) -> Value {
        let mut payload = json!({ "action": action });
        if let (Some(obj), Some(ex)) = (payload.as_object_mut(), extra.as_object()) {
            for (k, v) in ex {
                obj.insert(k.clone(), v.clone());
            }
        }
        json!({ "type": "mcp-action", "payload": payload })
    }

    fn now_deadline() -> std::time::Instant {
        // Zero-length window: await_event scans once then returns.
        std::time::Instant::now()
    }

    // FEAT-126 / BUG-111: composing a function into a host instance's region.
    fn put_ok_function(server: &LiveServer, name: &str) -> String {
        server
            .put_function(
                Some(name.to_string()),
                "@template &main() { <div>guest</div> }".to_string(),
                json!({}),
                "<div>guest</div>".to_string(),
                String::new(),
                String::new(),
                true,
                Vec::new(),
                std::path::PathBuf::from("/tmp"),
                super::super::origin::Origin::Stdlib,
                json!([]),
            )
            .record
            .id
    }

    #[test]
    fn mount_into_host_records_region_mount() {
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        let host = server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("compose into host");
        assert_eq!(
            host.region_mounts.get("stage"),
            Some(&fid),
            "region_mounts maps stage -> guest fn"
        );
    }

    #[test]
    fn unmount_instance_removes_it() {
        // PLAN-049 W1.S1: unmount drops the instance from the store.
        let server = test_server_with_instance(false);
        assert!(server.get_instance("inst-1").is_some());
        let (id, _title) = server.unmount_instance("inst-1").expect("unmount");
        assert_eq!(id, "inst-1");
        assert!(
            server.get_instance("inst-1").is_none(),
            "instance removed from store"
        );
    }

    #[test]
    fn unmount_unknown_instance_errors() {
        let server = test_server_with_instance(false);
        let err = server.unmount_instance("inst-404").unwrap_err();
        assert!(err.contains("no such instance"), "clear error: {err}");
    }

    #[test]
    fn unmount_standalone_does_not_touch_host_region() {
        // PLAN-049 W1.S1 (reviewer W1 P2): a composed stage guest is NOT an
        // instance — it is a host region_mounts entry. Unmounting a STANDALONE
        // instance of the SAME function must NOT clear an unrelated host region
        // that composes that function. Stage clearing is mcp_stage_clear's job.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("compose into host");
        // Standalone instance of the same fn; unmount it.
        let standalone = server
            .mount_function(&fid, None, json!({}), None, false)
            .expect("standalone mount");
        server
            .unmount_instance(&standalone.id)
            .expect("unmount standalone");
        let host = server.get_instance("inst-1").expect("host still mounted");
        assert_eq!(
            host.region_mounts.get("stage"),
            Some(&fid),
            "host stage region UNTOUCHED by an unrelated standalone unmount"
        );
        // The dedicated path clears it.
        server.clear_region("inst-1", "stage").expect("clear");
        let host = server.get_instance("inst-1").expect("host");
        assert!(
            host.region_mounts.get("stage").is_none(),
            "clear_region empties the stage"
        );
    }

    #[test]
    fn clear_region_empties_stage() {
        // PLAN-049 W1.S2: clear_region drops the composed guest + its overrides.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        server
            .mount_into_host("inst-1", "stage", &fid, &json!({ "name": "World" }))
            .expect("compose");
        let cleared = server.clear_region("inst-1", "stage").expect("clear");
        assert_eq!(
            cleared.as_deref(),
            Some(fid.as_str()),
            "returns the cleared fn"
        );
        let host = server.get_instance("inst-1").expect("host");
        assert!(
            host.region_mounts.get("stage").is_none(),
            "stage mount gone"
        );
        assert!(
            host.region_mount_input.get("stage").is_none(),
            "stage overrides gone"
        );
    }

    #[test]
    fn clear_empty_region_is_noop_success() {
        let server = test_server_with_instance(false);
        let cleared = server.clear_region("inst-1", "stage").expect("clear empty");
        assert!(cleared.is_none(), "nothing was composed");
    }

    #[test]
    fn clear_region_unknown_host_errors() {
        let server = test_server_with_instance(false);
        let err = server.clear_region("inst-404", "stage").unwrap_err();
        assert!(err.contains("no such host instance"), "clear error: {err}");
    }

    #[test]
    fn put_source_registers_env_origin_function() {
        // PLAN-049 W2 (COLLAPSE): put_source compiles + registers a function. An
        // env-origin source becomes a gallery-visible, stage-mountable function
        // — the mechanism behind st_tab_open. compile_ok true for a clean
        // template; the origin carries the env id. Uses the host-instance fixture
        // so the compose-onto-stage step has an inst-1 host.
        let server = test_server_with_instance(false);
        let outcome = server.put_source(
            Some("env-1/card".to_string()),
            "@template &main() { <div>tab</div> }".to_string(),
            json!({ "version": 1, "name": "env-1/card" }),
            std::path::PathBuf::from("/tmp"),
            super::super::origin::Origin::Env("env-1".to_string()),
        );
        assert!(outcome.record.compile_ok, "clean template compiles");
        assert_eq!(
            outcome.record.origin.id(),
            Some("env-1"),
            "env-origin tagged"
        );
        // The function is in the registry and mountable onto a host stage.
        let fid = outcome.record.id.clone();
        assert!(server.get_function(&fid).is_some(), "registered");
        server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("env-origin tab function composes onto the stage");
    }

    #[test]
    fn tab_open_is_idempotent_per_function() {
        // PLAN-049 W2 (reviewer W2 P2): re-opening the same entry (stable fn name)
        // reuses the SAME tab row, not a duplicate. So closing it disposes the one
        // tab + its fn cleanly — no orphaned sibling tab.
        let wb = WorkbenchSession::new(std::path::PathBuf::from("/tmp"));
        let env = wb.open_environment(std::path::PathBuf::from("/tmp"), "e".into());
        let t1 = wb.open_tab(
            &env.id,
            "card".into(),
            TabSource::Inline("a".into()),
            Some("fn-9".into()),
        );
        let t2 = wb.open_tab(
            &env.id,
            "card".into(),
            TabSource::Inline("b".into()),
            Some("fn-9".into()),
        );
        assert_eq!(t1.id, t2.id, "same fn => same tab row reused");
        assert_eq!(wb.all_tabs_json().len(), 1, "no duplicate tab");
    }

    #[test]
    fn delete_function_cascades_to_drop_tabs() {
        // PLAN-049 W2 (reviewer W2 P2): deleting a function (any path) drops the
        // tab referencing it, so no dangling tab targets a missing function.
        let server = test_server_empty();
        let fid = put_ok_function(&server, "env-1/card");
        // Record a tab referencing the fn via the workbench session.
        let wb = server.store.lock().unwrap().workbench.clone();
        let env = wb.open_environment(std::path::PathBuf::from("/tmp"), "e".into());
        wb.open_tab(
            &env.id,
            "card".into(),
            TabSource::Inline("x".into()),
            Some(fid.clone()),
        );
        assert_eq!(wb.all_tabs_json().len(), 1, "tab recorded");
        server.delete_function(&fid).expect("delete");
        assert_eq!(wb.all_tabs_json().len(), 0, "tab dropped with its function");
    }

    #[test]
    fn put_source_with_entry_surfaces_warning_diagnostics_on_clean_compile() {
        // PLAN-066 regression: put_source_with_entry must NOT discard bundle
        // warning diagnostics (MCP-SHELL/MCP-LOCALE) on the SUCCESS path just
        // because compile_ok is true — this is the exact bug that made
        // st_tab_open/open_tab_action silently report `compile_ok: true` with no
        // signal that the entry composes into a sibling index.html shell the MCP
        // path can't merge.
        let dir = std::env::temp_dir().join(format!(
            "spacetime-put-source-shell-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("tmp dir");
        std::fs::write(
            dir.join("index.html"),
            "<html><body><main></main></body></html>",
        )
        .expect("write shell");
        let entry_path = dir.join("index.st");
        let server = test_server_empty();
        let outcome = server.put_source_with_entry(
            Some("env-1/index".to_string()),
            "main { <p>hello</p> }".to_string(),
            json!({}),
            dir.clone(),
            super::super::origin::Origin::Env("env-1".to_string()),
            Some(&entry_path),
        );
        assert!(
            outcome.record.compile_ok,
            "selector-only composition still compiles ok"
        );
        assert!(
            outcome
                .record
                .diagnostics
                .iter()
                .any(|d| d.code == "MCP-SHELL"),
            "MCP-SHELL warning must survive onto the record even though compile_ok=true: {:?}",
            outcome.record.diagnostics
        );
        assert_eq!(
            outcome
                .record
                .diagnostics
                .iter()
                .find(|d| d.code == "MCP-SHELL")
                .unwrap()
                .severity,
            "warning"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn put_source_broken_registers_with_diagnostics() {
        // A broken source still registers (listed) but is not compile_ok.
        let server = test_server_empty();
        let outcome = server.put_source(
            Some("env-1/broken".to_string()),
            "@template &main($t) { <div>`$t`</div> $undeclared }".to_string(),
            json!({}),
            std::path::PathBuf::from("/tmp"),
            super::super::origin::Origin::Env("env-1".to_string()),
        );
        assert!(!outcome.record.compile_ok, "broken source not compile_ok");
        assert!(
            !outcome.record.diagnostics.is_empty(),
            "carries diagnostics"
        );
    }

    #[test]
    fn delete_unmounted_function_succeeds() {
        // PLAN-049 W1.S3: an unmounted function deletes cleanly. Empty store so
        // no pre-mounted instance references it.
        let server = test_server_empty();
        let fid = put_ok_function(&server, "doomed");
        let (id, name) = server.delete_function(&fid).expect("delete");
        assert_eq!(id, fid);
        assert_eq!(name.as_deref(), Some("doomed"));
        assert!(server.get_function(&fid).is_none(), "function gone");
        // The name index is dropped too.
        assert!(server.get_function("doomed").is_none(), "name index gone");
    }

    #[test]
    fn delete_mounted_function_refuses() {
        // PLAN-049 W1.S3: deletion is refused while the function is mounted as a
        // host region guest, with a message pointing at unmount. Build a real
        // host (fn-1) + a DISTINCT guest (fn-2) so the refusal is purely the
        // guest-region path (not the host's own standalone mount).
        let server = test_server_empty();
        let host_fid = put_ok_function(&server, "host");
        let host = server
            .mount_function(&host_fid, None, json!({}), None, false)
            .expect("mount host");
        let guest_fid = put_ok_function(&server, "guest");
        server
            .mount_into_host(&host.id, "stage", &guest_fid, &Value::Null)
            .expect("compose guest");
        let err = server.delete_function(&guest_fid).unwrap_err();
        assert!(err.contains("still mounted"), "refusal message: {err}");
        assert!(
            server.get_function(&guest_fid).is_some(),
            "guest NOT deleted while mounted"
        );
        // After clearing the stage, deletion succeeds.
        server.clear_region(&host.id, "stage").expect("clear");
        server
            .delete_function(&guest_fid)
            .expect("delete after clear");
    }

    #[test]
    fn delete_unknown_function_errors() {
        let server = test_server_with_instance(false);
        let err = server.delete_function("fn-404").unwrap_err();
        assert!(err.contains("no such function"), "error: {err}");
    }

    #[test]
    fn close_environment_unknown_errors() {
        let server = test_server_with_instance(false);
        let err = server.close_environment("env-404").unwrap_err();
        assert!(err.contains("no such environment"), "error: {err}");
    }

    #[test]
    fn region_bundle_stable_caches_per_revision() {
        // BUG-118: N /bundles polls of an unchanged function compile AT MOST once.
        use std::sync::atomic::Ordering;
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        let (src, root) = {
            let g = server.store.lock().unwrap();
            let f = g.functions.get(&fid).unwrap();
            (f.source.clone(), f.resolve_root.clone())
        };
        super::REGION_BUNDLE_COMPILES.store(0, Ordering::SeqCst);
        // First poll: a miss -> compiles once + caches.
        let v1 = super::region_bundle_stable(&server.store, &fid, 1, &src, &root).expect("compile");
        assert_eq!(
            super::REGION_BUNDLE_COMPILES.load(Ordering::SeqCst),
            1,
            "first poll compiles"
        );
        // 9 more polls of the SAME (fn, revision): all hits, no recompile.
        for _ in 0..9 {
            let v =
                super::region_bundle_stable(&server.store, &fid, 1, &src, &root).expect("cached");
            assert_eq!(v, v1, "cached bundle identical");
        }
        assert_eq!(
            super::REGION_BUNDLE_COMPILES.load(Ordering::SeqCst),
            1,
            "10 polls of an unchanged fn compile exactly once (the wedge fix)"
        );
        // A new revision is a fresh key -> one more compile.
        let _ = super::region_bundle_stable(&server.store, &fid, 2, &src, &root).expect("rev2");
        assert_eq!(
            super::REGION_BUNDLE_COMPILES.load(Ordering::SeqCst),
            2,
            "new revision recompiles once"
        );
    }

    #[test]
    fn mount_into_host_seeds_region_input_from_mount_input() {
        // BUG-113: composing a guest WITH an `input` seeds region_mount_input so
        // the bundle endpoint renders the guest with its values, not its defaults.
        // Scalars are coerced to strings; null/composite values are skipped.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        let host = server
            .mount_into_host(
                "inst-1",
                "stage",
                &fid,
                &json!({ "name": "World", "count": 3, "flag": true, "skip": null, "obj": {"a": 1} }),
            )
            .expect("compose into host with input");
        let seed = host
            .region_mount_input
            .get("stage")
            .expect("stage region seeded");
        assert_eq!(seed.get("name").map(String::as_str), Some("World"));
        assert_eq!(
            seed.get("count").map(String::as_str),
            Some("3"),
            "number coerced"
        );
        assert_eq!(
            seed.get("flag").map(String::as_str),
            Some("true"),
            "bool coerced"
        );
        assert!(!seed.contains_key("skip"), "null value skipped");
        assert!(!seed.contains_key("obj"), "composite value skipped");
    }

    #[test]
    fn mount_into_host_rejects_unknown_host() {
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        let err = server
            .mount_into_host("inst-404", "stage", &fid, &Value::Null)
            .expect_err("unknown host rejected");
        assert!(err.contains("no such host"), "err names the host: {err}");
    }

    #[test]
    fn mount_into_host_rejects_unknown_function() {
        let server = test_server_with_instance(false);
        let err = server
            .mount_into_host("inst-1", "stage", "fn-nope", &Value::Null)
            .expect_err("unknown fn rejected");
        assert!(err.contains("no such function"), "err names the fn: {err}");
    }

    #[test]
    fn contract_hash_is_order_independent() {
        // FEAT-124: two param lists with the same specs in different order hash
        // equal (a pure reorder is a COMPATIBLE change).
        let a = json!([
            { "template": "card", "name": "title", "kind": "binding", "optional": false, "type": null },
            { "template": "card", "name": "sub", "kind": "binding", "optional": true, "type": "string" }
        ]);
        let b = json!([
            { "template": "card", "name": "sub", "kind": "binding", "optional": true, "type": "string" },
            { "template": "card", "name": "title", "kind": "binding", "optional": false, "type": null }
        ]);
        assert_eq!(contract_hash_of(&a), contract_hash_of(&b));
    }

    #[test]
    fn contract_hash_changes_on_param_change() {
        let a =
            json!([{ "template": "card", "name": "title", "kind": "binding", "optional": false }]);
        let b =
            json!([{ "template": "card", "name": "title", "kind": "binding", "optional": true }]);
        assert_ne!(
            contract_hash_of(&a),
            contract_hash_of(&b),
            "optionality change is breaking"
        );
        let c = json!([{ "template": "card", "name": "heading", "kind": "binding", "optional": false }]);
        assert_ne!(
            contract_hash_of(&a),
            contract_hash_of(&c),
            "rename is breaking"
        );
    }

    /// FEAT-124 boundary probe: what the contract fingerprint DISCRIMINATES,
    /// and — the point of the test — what it is BLIND to.
    ///
    /// The fingerprint is derived from `contract_params` only: the flat list of
    /// template param specs. That makes it exact about the SURFACE and, by
    /// construction, silent about BEHAVIOUR. Both facts matter to any caller
    /// deciding whether to accept a revision, so both are pinned here.
    #[test]
    fn contract_hash_boundary_what_it_catches_and_what_it_misses() {
        let h = contract_hash_of;

        // ── caught: every change to the param surface ──
        let base = json!([{ "template": "card", "name": "t", "kind": "binding", "optional": false }]);

        let added = json!([
            { "template": "card", "name": "t", "kind": "binding", "optional": false },
            { "template": "card", "name": "sub", "kind": "binding", "optional": true }
        ]);
        assert_ne!(h(&base), h(&added), "adding a param is a contract change");

        let moved = json!([{ "template": "panel", "name": "t", "kind": "binding", "optional": false }]);
        assert_ne!(h(&base), h(&moved), "a param moving template is a contract change");

        let retyped = json!([{ "template": "card", "name": "t", "kind": "binding", "optional": false, "type": "number" }]);
        assert_ne!(h(&base), h(&retyped), "a param's type is part of the contract");

        // ── caught, but ARGUABLY over-strict: widening ──
        // required -> optional cannot break an existing caller (every call that
        // satisfied the old contract satisfies the new one), yet it hashes
        // differently and so classifies as Breaking. The fingerprint answers
        // "did the surface change?", NOT "can this break a caller?" — a
        // consumer wanting the latter needs more than equality.
        let widened = json!([{ "template": "card", "name": "t", "kind": "binding", "optional": true }]);
        assert_ne!(
            h(&base),
            h(&widened),
            "widening required->optional is SAFE for callers but still hashes as changed"
        );

        // ── the blind spot: identical surface, any behaviour ──
        // This is the load-bearing limitation. A revision that rewrites every
        // body, inverts a condition, or drops a side effect hashes IDENTICAL as
        // long as the param specs match. `Compatible` therefore means "the
        // surface is unchanged" and never "the behaviour is unchanged".
        assert_eq!(
            h(&base),
            h(&base.clone()),
            "same param specs hash equal no matter what the bodies do"
        );

        // ── degenerate inputs fold together ──
        // A function with no params and a MALFORMED extraction (non-array) both
        // produce the empty row set, so they are indistinguishable. A contract
        // that failed to extract looks exactly like a contract with no params.
        assert_eq!(
            h(&json!(null)),
            h(&json!([])),
            "a failed extraction is indistinguishable from an empty contract"
        );
    }

    #[test]
    fn re_put_same_contract_is_compatible_and_bumps_revision() {
        let server = test_server_with_instance(false);
        let p1 = server.put_function(
            Some("f".to_string()),
            "@template &main($x) { <i>`$x`</i> }".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([{ "template": "main", "name": "x", "kind": "binding", "optional": false }]),
        );
        assert_eq!(p1.compatibility, Compatibility::Initial);
        assert_eq!(p1.record.revision, 1);
        // Re-put with the SAME contract params (body text differs, contract same).
        let p2 = server.put_function(
            Some("f".to_string()),
            "@template &main($x) { <b>`$x`</b> }".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([{ "template": "main", "name": "x", "kind": "binding", "optional": false }]),
        );
        assert_eq!(p2.compatibility, Compatibility::Compatible);
        assert_eq!(p2.record.revision, 2);
        assert_eq!(p1.record.id, p2.record.id, "same fn id across revisions");
    }

    #[test]
    fn re_put_changed_contract_is_breaking() {
        let server = test_server_with_instance(false);
        server.put_function(
            Some("f".to_string()),
            "@template &main($x) {}".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([{ "template": "main", "name": "x", "kind": "binding", "optional": false }]),
        );
        let p2 = server.put_function(
            Some("f".to_string()),
            "@template &main($y) {}".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([{ "template": "main", "name": "y", "kind": "binding", "optional": false }]),
        );
        assert_eq!(p2.compatibility, Compatibility::Breaking);
    }

    #[test]
    fn re_put_syncs_mounted_instance_revision() {
        // FEAT-124 hot-swap: a mounted instance's displayed revision tracks the
        // latest compatible re-put (the /bundles poll already serves live source).
        let server = test_server_with_instance(false);
        let p1 = server.put_function(
            Some("g".to_string()),
            "@template &main() { <i>a</i> }".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([]),
        );
        let inst = server
            .mount_function(&p1.record.id, None, json!({}), None, false)
            .expect("mount");
        assert_eq!(inst.function_revision, 1);
        // Re-put (revision 2).
        server.put_function(
            Some("g".to_string()),
            "@template &main() { <b>b</b> }".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([]),
        );
        let synced = server.get_instance(&inst.id).expect("instance exists");
        assert_eq!(
            synced.function_revision, 2,
            "instance revision tracks latest re-put"
        );
    }

    #[test]
    fn mount_into_host_rejects_file_scope_only_guest() {
        // FEAT-126 v1 / FUP-061: a guest with no @template yields an empty bundle;
        // composing it would silently leave the stage empty, so it is rejected
        // with an actionable message instead.
        let server = test_server_with_instance(false);
        let fid = server
            .put_function(
                Some("plain".to_string()),
                "<div>just file-scope html</div>".to_string(),
                json!({}),
                "<div>just file-scope html</div>".to_string(),
                String::new(),
                String::new(),
                true,
                Vec::new(),
                std::path::PathBuf::from("/tmp"),
                super::super::origin::Origin::Stdlib,
                json!([]),
            )
            .record
            .id;
        let err = server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect_err("file-scope-only guest rejected");
        assert!(
            err.contains("no @template"),
            "err explains the constraint: {err}"
        );
    }

    #[test]
    fn agent_action_is_awaited() {
        let server = test_server_with_instance(false);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action("kit-choice", json!({ "value": "blue" })),
            )
            .unwrap();
        assert_eq!(ev.audience, actions::Audience::Agent);
        let got = server.await_event("inst-1", now_deadline());
        assert!(got.is_some(), "a kit-choice must be delivered to st_await");
        assert_eq!(got.unwrap().payload["value"], "blue");
    }

    #[test]
    fn server_action_is_not_awaited() {
        let server = test_server_with_instance(false);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "st_env_open",
                    json!({ "target": "projects/does-not-exist" }),
                ),
            )
            .unwrap();
        assert_eq!(
            ev.audience,
            actions::Audience::Server,
            "st_env_open is a server action"
        );
        // A server-only event must NOT be returned by await (it is resolved at
        // the sink, not by the agent).
        let got = server.await_event("inst-1", now_deadline());
        assert!(
            got.is_none(),
            "a server nav action must never wake st_await"
        );
    }

    #[test]
    fn server_action_does_not_satisfy_a_prior_agent_await_then_agent_event_does() {
        // The wrong-consumer bug: a server nav click recorded BEFORE an agent
        // event must not be what await returns; the agent event must.
        let server = test_server_with_instance(false);
        server
            .record_event(
                "inst-1",
                mcp_action("st_env_open", json!({ "target": "projects/x" })),
            )
            .unwrap();
        server
            .record_event(
                "inst-1",
                mcp_action("kit-confirm", json!({ "value": "confirm" })),
            )
            .unwrap();
        let got = server.await_event("inst-1", now_deadline());
        let got = got.expect("the agent event should be delivered");
        assert_eq!(got.event_type, "mcp-action");
        assert_eq!(got.payload["action"], "kit-confirm");
    }

    #[test]
    fn unknown_action_is_server_audience_and_not_awaited() {
        // An unknown action on a NORMAL page is rejected at the sink and recorded
        // as server-audience — it must never wake the agent.
        let server = test_server_with_instance(false);
        let ev = server
            .record_event("inst-1", mcp_action("totally-custom", json!({})))
            .unwrap();
        assert_eq!(ev.audience, actions::Audience::Server);
        assert!(server.await_event("inst-1", now_deadline()).is_none());
    }

    #[test]
    fn unknown_action_on_agent_control_page_is_awaited() {
        // A page that declares itself agent-control routes its OWN unknown action
        // to the agent.
        let server = test_server_with_instance(true);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action("my-custom-control", json!({ "value": "go" })),
            )
            .unwrap();
        assert_eq!(ev.audience, actions::Audience::Agent);
        let got = server.await_event("inst-1", now_deadline());
        assert_eq!(got.unwrap().payload["action"], "my-custom-control");
    }

    #[test]
    fn snapshot_stage_mount_projects_selected_guest_contract() {
        // M3' (PLAN-046): after a guest is composed onto the workbench host's
        // stage, the snapshot's `stage_mount` projects THAT guest's identity +
        // contract params — the data the in-stage selection inspector binds.
        let server = test_server_with_instance(false);
        // Make inst-1's function the workbench host (the detection keys on the
        // `mcp-workbench` function name). Bump next_function so the guest below
        // gets fn-2 and does not overwrite this fn-1.
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("/tmp"),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        // Register a parametered guest and compose it onto inst-1's stage.
        let guest = server
            .put_function(
                Some("profile".to_string()),
                "@template &card($title, $subtitle) { <div>`$title`</div> }".to_string(),
                json!({}),
                String::new(),
                String::new(),
                String::new(),
                true,
                Vec::new(),
                std::path::PathBuf::from("."),
                super::super::origin::Origin::Stdlib,
                json!([
                    { "template": "card", "name": "title", "kind": "binding", "optional": false },
                    { "template": "card", "name": "subtitle", "kind": "binding", "optional": false }
                ]),
            )
            .record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let sm = &snap["stage_mount"];
        assert_eq!(
            sm["name"], "profile",
            "stage_mount names the composed guest"
        );
        let params = sm["contract_params"].as_array().expect("params array");
        assert_eq!(params.len(), 2, "projects the guest's two params");
        // Empty stage → null.
        let server2 = test_server_with_instance(false);
        {
            let mut g = server2.store.lock().unwrap();
            g.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("/tmp"),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let snap2 = server2.store.lock().unwrap().workbench_snapshot();
        assert!(
            snap2["stage_mount"].is_null(),
            "empty stage → null stage_mount"
        );
    }

    #[test]
    fn region_bundle_main_serialized_carries_named_refs_and_css() {
        // BUG-131 finding C/D: the region-compose bundle (what @mcp-region renders)
        // must carry the guest's body-refs in &main's SERIALIZED payload (so the
        // runtime factory renders the child cards) AND the guest @style as css (so
        // the stage is styled). Pre-fix the stale server showed empty serialized
        // refs + empty css; this asserts the data the runtime needs is present,
        // with named args structured (arg_names).
        let src = "@template &card($title, $price) { <div class=\"c\">`$title`</div> }\n@template &main() {\n  &card(title: \"Espresso\", price: \"$3.50\");\n}\n.c { color: rebeccapurple; }\n";
        let bundle = compile_to_bundle(src, std::path::Path::new(".")).expect("compile");
        // CSS (finding D): the guest @style reaches the region bundle.
        assert!(
            bundle.css.contains("rebeccapurple"),
            "region bundle css carries the guest @style: {:?}",
            bundle.css
        );
        // &main's serialized body (finding C): carries the &card ref WITH arg_names.
        let main = bundle
            .templates
            .iter()
            .find(|t| t.name == "main")
            .expect("main template");
        assert!(
            main.body.serialized.contains("\"template_name\":\"card\"")
                || main.body.serialized.contains("template_name: \"card\""),
            "main serialized carries the card ref: {}",
            main.body.serialized
        );
        assert!(
            main.body.serialized.contains("arg_names"),
            "named-arg ref serializes arg_names (runtime binds by name): {}",
            main.body.serialized
        );
        // The structured refs field likewise carries the named binding.
        let card_ref = main
            .body
            .refs
            .iter()
            .find(|r| r.template_name == "card")
            .expect("card ref present in main.body.refs");
        assert_eq!(
            card_ref.arg_names,
            vec![Some("title".to_string()), Some("price".to_string())],
            "card ref carries the param names for the runtime named bind"
        );
    }

    #[test]
    fn snapshot_stage_structure_walks_template_graph() {
        // PLAN-051 B1: after a guest with a &main that invokes a child template
        // is composed onto the stage, the snapshot's `stage_structure` is a flat
        // depth-tagged node list walked from &main via body.refs.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        // A guest whose &main invokes &card twice. The walk should emit a node
        // for main (depth 0) and one per &card invocation (depth 1).
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &card($title) { <div>`$title`</div> }\n@template &main() { &card(\"A\"); &card(\"B\"); }".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let nodes = snap["stage_structure"].as_array().expect("structure array");
        assert!(!nodes.is_empty(), "structure walks a non-empty graph");
        // Root is main at depth 0.
        assert_eq!(nodes[0]["template"], "main", "root is &main");
        assert_eq!(nodes[0]["depth"], 0, "root at depth 0");
        // At least the two &card invocations appear at depth 1.
        let card_nodes: Vec<&Value> = nodes
            .iter()
            .filter(|n| n["template"] == "card" && n["depth"] == 1)
            .collect();
        assert_eq!(
            card_nodes.len(),
            2,
            "both &card invocations are emitted at depth 1"
        );
        // Each card node carries its param schema (the $title binding).
        let card_params = card_nodes[0]["params"].as_array().expect("card params");
        assert_eq!(card_params.len(), 1, "card has one param");
        assert_eq!(card_params[0]["name"], "title");
        // bound_params pairs each param with the VALUE the invocation passed:
        // &card("A") binds title -> "A", &card("B") -> "B" (the inspector shows the
        // actual call values, not just the schema).
        let b0 = card_nodes[0]["bound_params"]
            .as_array()
            .expect("bound_params 0");
        assert_eq!(b0.len(), 1, "one bound param");
        assert_eq!(b0[0]["name"], "title");
        assert_eq!(b0[0]["value"], "\"A\"", "first card bound title -> \"A\"");
        assert_eq!(b0[0]["bound"], true, "title is bound by the invocation");
        let b1 = card_nodes[1]["bound_params"]
            .as_array()
            .expect("bound_params 1");
        assert_eq!(b1[0]["value"], "\"B\"", "second card bound title -> \"B\"");
        // FUP-093/G2: each &card invocation carries an invoke_span (the EditAst
        // write address for editing THAT instance's args). The two invocations
        // have DISTINCT spans (different source positions) — per-instance addressing.
        let s0 = &card_nodes[0]["invoke_span"];
        let s1 = &card_nodes[1]["invoke_span"];
        assert!(s0.is_object(), "first card carries an invoke_span: {s0}");
        assert!(s1.is_object(), "second card carries an invoke_span: {s1}");
        assert_ne!(
            s0["start"], s1["start"],
            "the two invocations have distinct spans"
        );
        // The root &main has no invocation span (it's the entry, not a call).
        assert!(
            nodes[0]["invoke_span"].is_null(),
            "root &main has no invoke_span"
        );
        // Empty stage → empty array (never null/error).
        let server2 = test_server_with_instance(false);
        {
            let mut g = server2.store.lock().unwrap();
            g.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let snap2 = server2.store.lock().unwrap().workbench_snapshot();
        assert_eq!(
            snap2["stage_structure"],
            json!([]),
            "empty stage → empty structure"
        );
    }

    #[test]
    fn snapshot_bound_params_normalize_named_args() {
        // NAMED-arg invocations (`&card(title: "A")`) store the full `name: value`
        // text per arg. The projection must normalize bound_params.value to the
        // BARE value so the inspector never double-prints the name
        // (`title: title: "A"`). Positional args are already bare and unaffected.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &card($title, $price) { <div>`$title`</div> }\n@template &main() { &card(title: \"Espresso\", price: \"$3.50\"); }".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let nodes = snap["stage_structure"].as_array().expect("structure array");
        let card = nodes
            .iter()
            .find(|n| n["template"] == "card")
            .expect("card node");
        let bound = card["bound_params"].as_array().expect("bound_params");
        // value is the BARE value, NOT prefixed with the param name.
        assert_eq!(bound[0]["name"], "title");
        assert_eq!(
            bound[0]["value"], "\"Espresso\"",
            "title value bare, no name prefix"
        );
        assert_eq!(bound[1]["name"], "price");
        assert_eq!(
            bound[1]["value"], "\"$3.50\"",
            "price value bare, no name prefix"
        );
        // The pre-joined summary likewise prints each name exactly once.
        let summary = card["bound_summary"].as_str().expect("bound_summary");
        assert!(
            summary.contains("title: \"Espresso\""),
            "summary: {summary}"
        );
        assert!(
            !summary.contains("title: title:"),
            "no doubled name: {summary}"
        );
    }

    #[test]
    fn b4_stage_node_params_projects_editable_rows_with_spans() {
        // PLAN-051 B4: the snapshot's stage_node_params flattens each composed
        // node's bound params into editable rows carrying the node_id + the
        // invocation's invoke_span (the G2 write address the inspector edits).
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &card($title, $price) { <div>`$title`</div> }\n@template &main() { &card(title: \"Espresso\", price: \"$3.50\"); }".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let rows = snap["stage_node_params"]
            .as_array()
            .expect("stage_node_params array");
        // The single &card invocation has 2 params -> 2 rows, both with a span.
        let card_rows: Vec<&Value> = rows
            .iter()
            .filter(|r| r["param"] == "title" || r["param"] == "price")
            .collect();
        assert_eq!(
            card_rows.len(),
            2,
            "two editable rows for the card's params: {rows:?}"
        );
        for r in &card_rows {
            assert!(r["span_start"].is_number(), "row carries span_start: {r}");
            assert!(r["span_end"].is_number(), "row carries span_end: {r}");
            assert!(
                r["node_id"].is_string(),
                "row carries node_id (string, matches DOM dataset selection) for selection filter: {r}"
            );
        }
        let title_row = card_rows.iter().find(|r| r["param"] == "title").unwrap();
        assert_eq!(
            title_row["value"], "\"Espresso\"",
            "row shows the bound value"
        );
    }

    #[test]
    fn b4_dispatch_mcp_node_param_patches_source_and_bumps_revision() {
        // PLAN-051 B4 WRITE: dispatching mcp_node_param with a param + value +
        // span patches THAT invocation's arg in the guest source and recompiles
        // (revision bumps). The edit is per-instance (span-addressed).
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &card($title, $price) { <div>`$title`</div> }\n@template &main() { &card(title: \"Espresso\", price: \"$3.50\"); }".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        let fid = guest.id.clone();
        server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("compose");
        // Read the card row's span from the projection (the inspector's address).
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let rows = snap["stage_node_params"].as_array().unwrap().clone();
        let title_row = rows
            .iter()
            .find(|r| r["param"] == "title")
            .expect("title row");
        let span_start = title_row["span_start"].as_u64().unwrap();
        let span_end = title_row["span_end"].as_u64().unwrap();
        let rev_before = server.get_function(&fid).unwrap().revision;
        // Dispatch the edit: title -> Latte (span forwarded as STRINGS, the
        // data-st-payload rail's shape).
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_node_param",
                    json!({
                        "target": "title",
                        "value": "Latte",
                        "span_start": span_start.to_string(),
                        "span_end": span_end.to_string(),
                        "region": "stage",
                    }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true, "node-param edit resolves: {verdict}");
        let f = server.get_function(&fid).unwrap();
        assert!(f.revision > rev_before, "revision bumped after edit");
        assert!(
            f.source.contains("title: \"Latte\""),
            "source patched at the span: {}",
            f.source
        );
        assert!(
            f.source.contains("price: \"$3.50\""),
            "sibling arg untouched: {}",
            f.source
        );
        assert!(
            !f.source.contains("\"Espresso\""),
            "old value gone: {}",
            f.source
        );
    }

    #[test]
    fn b4_dispatch_mcp_node_param_rejects_missing_span() {
        // Without a span the edit cannot address an invocation -> clean reject.
        let server = test_server_with_instance(false);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action("mcp_node_param", json!({ "target": "title", "value": "X" })),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], false, "missing span rejected: {verdict}");
    }

    #[test]
    fn snapshot_stage_structure_captures_each_driven_children() {
        // The navigator should show @each-driven children, not only bare
        // statement-form invocations. `@each($items as $x) { &item($x); }` inside
        // a template body emits a `template-invoke-bare` match harvested into the
        // template's refs, so the structure walk includes &item under &main.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        // The SUPPORTED @each idiom is a SELECTOR-scoped block inside the template
        // body (`.list { @each(...) { &item($x); } }`), not inline-in-markup
        // (the latter is BUG-130 / a misuse). The selector-scoped @each lands as a
        // NESTED scope of the template, so scope_refs' nested-scope descent (this
        // change) harvests its interior &item invoke into main's refs → the
        // navigator shows the @each-driven child.
        let guest = server.put_function(
            Some("page".to_string()),
            "@data inline $items : [{\"label\":\"A\"}];\n@template &item($it) { <li>`$it.label`</li> }\n@template &main() {\n  <ul class=\"list\"></ul>\n  .list { @each($items as $x) { &item($x); } }\n}".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let nodes = snap["stage_structure"].as_array().expect("structure array");
        assert_eq!(nodes[0]["template"], "main", "root is &main");
        // The selector-scoped @each's &item child now appears (via scope_refs'
        // nested-scope recursion). The inline-in-markup form remains BUG-130.
        let has_item = nodes.iter().any(|n| n["template"] == "item");
        assert!(
            has_item,
            "selector-scoped @each-driven &item child appears: {:?}",
            nodes
                .iter()
                .map(|n| n["template"].as_str().unwrap_or("?"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn stage_structure_projects_element_tree_of_a_single_template_guest() {
        // PLAN-064 B2a: a single-template guest (no sub-refs) must now project its
        // ELEMENT tree beneath the &main node — the fix for the "1-node hero" bug.
        // Before B2, stage_structure walked refs only, so a `<section><h1><p>` hero
        // showed as ONE node. Now the section/h1/p element rows appear, each
        // carrying its reactive binding chips.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &main($title = \"Hi\", $sub = \"there\") { <section class=\"hero\"><h1>`$title`</h1><p>`$sub`</p></section> }".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let nodes = snap["stage_structure"].as_array().expect("structure array");
        // main (template) + section + h1 + p element nodes.
        assert_eq!(nodes[0]["template"], "main", "root is &main");
        let labels: Vec<&str> = nodes.iter().filter_map(|n| n["label"].as_str()).collect();
        assert!(
            labels.contains(&"section.hero"),
            "section element row present: {labels:?}"
        );
        assert!(labels.contains(&"h1"), "h1 element row present: {labels:?}");
        assert!(labels.contains(&"p"), "p element row present: {labels:?}");
        // The h1 element carries its reactive binding ($title).
        let h1 = nodes.iter().find(|n| n["label"] == "h1").expect("h1 node");
        assert_eq!(h1["kind"], "element", "h1 is an element node");
        let binds = h1["bindings"].as_array().expect("h1 bindings");
        assert!(
            binds.iter().any(|b| b == "title"),
            "h1 binds $title: {binds:?}"
        );
        // PLAN-064 B3.1: ids are the canonical DOTTED PATHS (not host-local
        // integers) — the SAME address space the dev-ws host + guest DOM stamps use.
        assert_eq!(nodes[0]["id"], "0", "root main id is dotted '0'");
        assert_eq!(
            h1["id"], "0.0.0",
            "h1 id is a dotted path (main.section.h1)"
        );
    }

    #[test]
    fn stage_node_bindings_projects_editable_rows_for_element_params() {
        // PLAN-064 B4a: selecting an element whose hole reads an ENTRY param yields
        // an editable binding row routed through `mcp_param`. A hero's <h1>`$title`
        // must produce a row {node_id, param:"title", rail:"mcp_param"} whose value
        // is the param default until overridden.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &main($title = \"Launch faster\", $sub = \"the canvas\") { <section class=\"hero\"><h1>`$title`</h1><p>`$sub`</p></section> }".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let bindings = snap["stage_node_bindings"]
            .as_array()
            .expect("bindings array");
        // The h1 reads $title, the p reads $sub — both entry params → two rows.
        let title_row = bindings
            .iter()
            .find(|r| r["param"] == "title")
            .expect("title binding row");
        assert_eq!(
            title_row["rail"], "mcp_param",
            "element binding edits via mcp_param"
        );
        assert_eq!(
            title_row["value"], "Launch faster",
            "value is the unquoted default until overridden"
        );
        assert_eq!(title_row["bound"], false, "no override yet");
        // The row's node_id must be the h1 element's structure id (so the inspector
        // filters it to $selectedNode when the h1 is selected).
        let structure = snap["stage_structure"].as_array().unwrap();
        let h1 = structure
            .iter()
            .find(|n| n["label"] == "h1")
            .expect("h1 node");
        assert_eq!(
            title_row["node_id"], h1["id"],
            "binding row keyed to the h1 node id"
        );
        assert!(
            bindings.iter().any(|r| r["param"] == "sub"),
            "sub binding row present too"
        );
    }

    #[test]
    fn stage_node_bindings_reflects_mcp_param_override() {
        // After an `mcp_param` edit sets $title, the binding row's value follows the
        // override (bound:true) — the inspector shows what the user set.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server
            .put_function(
                Some("page".to_string()),
                "@template &main($title = \"Launch faster\") { <h1>`$title`</h1> }".to_string(),
                json!({}),
                String::new(),
                String::new(),
                String::new(),
                true,
                Vec::new(),
                std::path::PathBuf::from("."),
                super::super::origin::Origin::Stdlib,
                json!([]),
            )
            .record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        // Simulate an mcp_param edit: set the stage region's title override.
        {
            let mut guard = server.store.lock().unwrap();
            let host = guard.instances.get_mut("inst-1").expect("host");
            host.region_mount_input
                .entry("stage".to_string())
                .or_default()
                .insert("title".to_string(), "Black Friday".to_string());
        }
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let bindings = snap["stage_node_bindings"].as_array().expect("bindings");
        let title = bindings
            .iter()
            .find(|r| r["param"] == "title")
            .expect("title row");
        assert_eq!(
            title["value"], "Black Friday",
            "row reflects the mcp_param override"
        );
        assert_eq!(title["bound"], true, "override marks the row bound");
    }

    #[test]
    fn composed_guest_builder_stamps_data_st_node_matching_structure() {
        // PLAN-064 B3.2/B3.3 END-TO-END: a composed guest's `main` builder must
        // STAMP data-st-node ids that MATCH the navigator's stage_structure ids, so
        // a canvas click and a tree click resolve to one node. Compose a hero, then
        // assert every element node id in stage_structure appears as a data-st-node
        // stamp in the guest's builder JS.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server
            .put_function(
                Some("page".to_string()),
                "@template &main() { <section class=\"hero\"><h1>`$t`</h1><p>x</p></section> }"
                    .to_string(),
                json!({}),
                String::new(),
                String::new(),
                String::new(),
                true,
                Vec::new(),
                std::path::PathBuf::from("."),
                super::super::origin::Origin::Stdlib,
                json!([]),
            )
            .record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        // The element node ids the navigator shows.
        let structure = snap["stage_structure"].as_array().expect("structure");
        let element_ids: Vec<String> = structure
            .iter()
            .filter(|n| n["kind"] == "element")
            .map(|n| n["id"].as_str().unwrap().to_string())
            .collect();
        assert!(
            !element_ids.is_empty(),
            "hero has element nodes: {structure:#?}"
        );
        // The composed guest's builder JS (from the bundle) must stamp each one.
        let bundle =
            crate::mcp::bundle::compile_to_bundle(&guest.source, std::path::Path::new("."))
                .expect("bundle");
        let main_builder = &bundle
            .templates
            .iter()
            .find(|t| t.name == "main")
            .expect("main tpl")
            .body
            .builder;
        for id in &element_ids {
            let stamp = format!("data-st-node', \"{id}\"");
            assert!(
                main_builder.contains(&stamp),
                "guest builder must stamp element id {id:?} (canvas==navigator); builder: {main_builder}"
            );
        }
    }

    #[test]
    fn stage_structure_ids_match_shared_ir_across_hosts() {
        // PLAN-064 B3.1 (the divergence kill): the MCP host's structure ids MUST be
        // byte-identical to the shared crate::introspect producer's ids for the same
        // source — so a navigator click (MCP) and a dev-ws InspectStructure read and
        // a guest-DOM data-st-node stamp all resolve to ONE node.
        let src = "@template &card($t) { <li>`$t`</li> }\n@template &main() { <ul class=\"l\"></ul> &card(\"A\"); }";
        let bundle =
            crate::mcp::bundle::compile_to_bundle(src, std::path::Path::new(".")).expect("compile");
        // The shared IR ids.
        let ir = crate::introspect::structure_from_bundle(&bundle, "main");
        let ir_ids: Vec<&str> = ir.iter().map(|n| n.id.as_str()).collect();
        // The MCP host projection ids.
        let host = crate::introspect::structure_json(&bundle);
        let host_ids: Vec<String> = host
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            ir_ids, host_ids,
            "MCP host ids must equal the shared IR ids (one address space)"
        );
    }

    #[test]
    fn stage_structure_recursive_template_does_not_double_expand() {
        // Reviewer B2-hosts P2: a recursive component's element subtree must NOT be
        // expanded under the cycle-sentinel node — the guard precedes the element
        // injection, matching the shared bundle_walk. A `&main { <div/> &main(); }`
        // must emit the recursive &main node WITHOUT a second <div> beneath it.
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("."),
                    origin: super::super::origin::Origin::Stdlib,
                    contract_params: json!([]),
                    contract_hash: String::new(),
                },
            );
        }
        let guest = server.put_function(
            Some("page".to_string()),
            "@template &main() {\n  <div class=\"box\"></div>\n  .box { @each($xs as $x) { &main(); } }\n}".to_string(),
            json!({}), String::new(), String::new(), String::new(), true, Vec::new(),
            std::path::PathBuf::from("."), super::super::origin::Origin::Stdlib,
            json!([]),
        ).record;
        server
            .mount_into_host("inst-1", "stage", &guest.id, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let nodes = snap["stage_structure"].as_array().expect("structure array");
        // Exactly ONE div.box element: the outer &main's. The recursive inner &main
        // is a cycle sentinel (recursive:true) and expands nothing beneath it.
        let box_count = nodes.iter().filter(|n| n["label"] == "div.box").count();
        assert_eq!(
            box_count,
            1,
            "recursive &main must not double-expand its <div>: {:?}",
            nodes
                .iter()
                .map(|n| (
                    n["label"].as_str().unwrap_or("?"),
                    n["recursive"].as_bool().unwrap_or(false)
                ))
                .collect::<Vec<_>>()
        );
        // The recursive inner &main node IS present, flagged recursive.
        assert!(
            nodes
                .iter()
                .any(|n| n["template"] == "main" && n["recursive"] == true),
            "recursive &main sentinel present"
        );
    }

    #[test]
    fn snapshot_contract_params_excludes_workbench_host() {
        // BUG-115: the inspector "ALL CONTRACTS" list (contract_params) must NOT
        // leak the workbench host's own internal card sub-templates. Register a
        // host fn WITH contract params + a guest, and assert only the guest's
        // params survive in contract_params (while the origin-drawer `functions`
        // array still lists the host).
        let server = test_server_with_instance(false);
        {
            let mut guard = server.store.lock().unwrap();
            guard.next_function = 2;
            guard.functions.insert(
                "fn-1".to_string(),
                FunctionRecord {
                    id: "fn-1".to_string(),
                    name: Some("mcp-workbench".to_string()),
                    revision: 1,
                    source: String::new(),
                    contract: json!({}),
                    html: String::new(),
                    css: String::new(),
                    js: String::new(),
                    compile_ok: true,
                    diagnostics: Vec::new(),
                    resolve_root: std::path::PathBuf::from("/tmp"),
                    origin: super::super::origin::Origin::Stdlib,
                    // Host internals: these MUST NOT appear in contract_params.
                    contract_params: json!([
                        { "template": "mcp-env-card", "name": "env", "kind": "binding", "optional": false },
                        { "template": "mcp-param-edit", "name": "p", "kind": "binding", "optional": false }
                    ]),
                    contract_hash: String::new(),
                },
            );
        }
        server.put_function(
            Some("profile".to_string()),
            "@template &card($title) { <div>`$title`</div> }".to_string(),
            json!({}),
            String::new(),
            String::new(),
            String::new(),
            true,
            Vec::new(),
            std::path::PathBuf::from("."),
            super::super::origin::Origin::Stdlib,
            json!([{ "template": "card", "name": "title", "kind": "binding", "optional": false }]),
        );
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let cps = snap["contract_params"]
            .as_array()
            .expect("contract_params array");
        assert!(
            cps.iter().all(|p| p["function"] != "mcp-workbench"),
            "contract_params must not contain the workbench host's own templates: {cps:?}"
        );
        assert!(
            cps.iter().any(|p| p["name"] == "title"),
            "the guest's own param survives"
        );
        // The origin drawer's `functions` array DOES still list the host.
        let fns = snap["functions"].as_array().expect("functions array");
        assert!(
            fns.iter().any(|f| f["name"] == "mcp-workbench"),
            "origin drawer still lists the host fragment"
        );
    }

    #[test]
    fn dispatch_mcp_compose_mounts_into_originating_host() {
        // M2 (PLAN-046): the gallery click. A server-audience `mcp_compose` fired
        // FROM the host (event.instance_id) composes the named function into the
        // host's stage region and repaints — no agent wake.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        let ev = server
            .record_event(
                "inst-1",
                mcp_action("mcp_compose", json!({ "target": fid, "region": "stage" })),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true, "compose resolves: {verdict}");
        // The host instance now carries the region mount.
        let host = server.get_instance("inst-1").unwrap();
        assert_eq!(host.region_mounts.get("stage"), Some(&fid));
    }

    #[test]
    fn workbench_snapshot_projects_stage_frames() {
        // PLAN-049 W4.S8: the frame strip projects every region composed on the
        // workbench host. Compose a guest and assert it appears with its region.
        let server = workbench_host_server();
        let fid = put_ok_function(&server, "guest");
        server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("compose");
        let snap = server.store.lock().unwrap().workbench_snapshot();
        let frames = snap["stage_frames"].as_array().expect("stage_frames array");
        assert_eq!(frames.len(), 1, "one composed frame");
        assert_eq!(frames[0]["region"], "stage");
        assert_eq!(frames[0]["function_id"], fid);
    }

    #[test]
    fn dispatch_mcp_create_registers_and_composes() {
        // PLAN-049 W3: the "+" creation surface. mcp_create reads payload.values
        // { create_name, create_source }, registers an Agent-origin function, and
        // composes it onto the originating WORKBENCH host's stage.
        let server = workbench_host_server();
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_create",
                    json!({ "values": { "create_name": "scratch", "create_source": "@template &main() { <h1>hi</h1> }" } }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true, "create resolves: {verdict}");
        assert_eq!(
            verdict["result"]["composed"], true,
            "composed on the workbench stage"
        );
        // The new function exists (Agent origin) and is composed on the stage.
        let host = server.get_instance("inst-1").unwrap();
        let staged = host
            .region_mounts
            .get("stage")
            .cloned()
            .expect("stage composed");
        let f = server.get_function(&staged).expect("function registered");
        assert_eq!(
            f.origin,
            super::super::origin::Origin::Agent,
            "created frame is agent-origin"
        );
    }

    #[test]
    fn dispatch_mcp_create_off_workbench_registers_without_compose() {
        // reviewer W3 P2: a create fired from a NON-workbench host registers the
        // frame but does NOT phantom-compose onto a stage it has no `@mcp-region`
        // for. inst-1's fn-1 is not named mcp-workbench in this fixture.
        let server = test_server_with_instance(false);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_create",
                    json!({ "values": { "create_source": "@template &main() { <p>x</p> }" } }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true, "registers");
        assert_eq!(
            verdict["result"]["composed"], false,
            "no phantom compose off-workbench"
        );
        let host = server.get_instance("inst-1").unwrap();
        assert!(
            host.region_mounts.get("stage").is_none(),
            "stage untouched off-workbench"
        );
    }

    #[test]
    fn dispatch_mcp_create_rejects_empty_source() {
        let server = test_server_with_instance(false);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_create",
                    json!({ "values": { "create_source": "   " } }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], false, "empty source rejected");
    }

    #[test]
    fn dispatch_mcp_create_reports_compile_error() {
        // A broken source is reported (not composed); nothing lands on the stage.
        let server = test_server_with_instance(false);
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_create",
                    json!({ "values": { "create_source": "@template &main( { <p>broken" } }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], false, "broken source reports an error");
        let host = server.get_instance("inst-1").unwrap();
        assert!(
            host.region_mounts.get("stage").is_none(),
            "broken create does not compose"
        );
    }

    #[test]
    fn update_source_recompiles_and_bumps_revision() {
        // M6 (PLAN-046): inline-edit save replaces source, bumps revision, syncs
        // mounted instances; a bad edit leaves the source unchanged.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest"); // r1, body "guest"
        // Good edit -> r2, new body.
        let (rev, _) = server
            .update_source(&fid, "@template &main() { <div>edited</div> }")
            .expect("good edit compiles");
        assert_eq!(rev, 2);
        assert!(server.get_function(&fid).unwrap().source.contains("edited"));
        // Bad edit -> Err, source unchanged.
        let before = server.get_function(&fid).unwrap().source;
        let err = server
            .update_source(&fid, "@template &main( { <div>broken")
            .expect_err("bad edit rejected");
        assert!(err.contains("did not compile"), "err explains: {err}");
        assert_eq!(
            server.get_function(&fid).unwrap().source,
            before,
            "source unchanged on bad edit"
        );
    }

    #[test]
    fn dispatch_mcp_source_edit_saves_stage_guest() {
        // M6: the inline-edit action resolves the stage guest from region_mounts
        // and saves the new source.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("compose");
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_source_edit",
                    json!({ "value": "@template &main() { <p>v2</p> }" }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true, "source edit resolves: {verdict}");
        assert!(server.get_function(&fid).unwrap().source.contains("v2"));
    }

    #[test]
    fn dispatch_mcp_param_sets_region_override() {
        // M4 (PLAN-046): editing a param fires `mcp_param`, which records the
        // override on the host's region_mount_input; the bundle then carries it.
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        server
            .mount_into_host("inst-1", "stage", &fid, &Value::Null)
            .expect("compose");
        let ev = server
            .record_event(
                "inst-1",
                mcp_action(
                    "mcp_param",
                    json!({ "target": "title", "value": "Hello", "region": "stage" }),
                ),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true, "param set resolves: {verdict}");
        let host = server.get_instance("inst-1").unwrap();
        assert_eq!(
            host.region_mount_input
                .get("stage")
                .and_then(|m| m.get("title")),
            Some(&"Hello".to_string()),
            "override recorded for region/param"
        );
    }

    #[test]
    fn dispatch_mcp_compose_defaults_region_to_stage() {
        let server = test_server_with_instance(false);
        let fid = put_ok_function(&server, "guest");
        let ev = server
            .record_event(
                "inst-1",
                mcp_action("mcp_compose", json!({ "target": fid })),
            )
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], true);
        assert_eq!(
            verdict["result"]["region"], "stage",
            "region defaults to stage"
        );
    }

    #[test]
    fn dispatch_resolves_server_action_and_rejects_unknown() {
        let server = test_server_with_instance(false);
        // Unknown action -> ok:false at the sink dispatcher.
        let ev = server
            .record_event("inst-1", mcp_action("totally-custom", json!({})))
            .unwrap();
        let verdict = dispatch_signal_action(&server.store, &ev);
        assert_eq!(verdict["ok"], false);
        // Agent action -> dispatcher returns Null (the agent reads it, server does
        // not resolve it).
        let ev2 = server
            .record_event("inst-1", mcp_action("kit-choice", json!({ "value": "x" })))
            .unwrap();
        let verdict2 = dispatch_signal_action(&server.store, &ev2);
        assert!(
            verdict2.is_null(),
            "an agent action is not resolved at the sink"
        );
    }
}

#[cfg(test)]
mod workbench_redirect_tests {
    //! FUP-110 / friction fix: `/__mcp/workbench` (and its `/__spacetime/workbench`
    //! alias) must be a STABLE, idempotent entry point — no `inst-N` hunting, no
    //! duplicate workbench pages piling up across repeated hits.
    use super::*;

    fn empty_store() -> Store {
        let workbench = WorkbenchSession::new(std::path::PathBuf::from("/tmp"));
        Arc::new(Mutex::new(LiveStore {
            workbench,
            functions: HashMap::new(),
            functions_by_name: HashMap::new(),
            instances: HashMap::new(),
            next_function: 1,
            next_instance: 1,
            next_event: 1,
            bundle_cache: HashMap::new(),
            port: 4949,
        }))
    }

    #[tokio::test]
    async fn workbench_redirect_mounts_then_reuses_the_same_instance() {
        let store = empty_store();

        // First hit: no function/instance yet -> registers `mcp-workbench` (from
        // the embedded stdlib fallback, since workspace_root is /tmp with no
        // on-disk stdlib overlay) and mounts a fresh instance.
        let resp1 = workbench_redirect(State(store.clone())).await;
        assert_eq!(
            resp1.status(),
            StatusCode::SEE_OTHER,
            "redirects (303) to the instance page"
        );
        let location1 = resp1
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();
        assert!(
            location1.starts_with("/__mcp/instance/"),
            "relative redirect target: {location1}"
        );

        {
            let guard = store.lock().unwrap();
            assert_eq!(
                guard.instances.len(),
                1,
                "exactly one workbench instance after first hit"
            );
            assert_eq!(guard.functions.len(), 1, "exactly one function registered");
        }

        // Second hit: MUST reuse the same instance, not mount a second one.
        let resp2 = workbench_redirect(State(store.clone())).await;
        assert_eq!(resp2.status(), StatusCode::SEE_OTHER);
        let location2 = resp2
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();
        assert_eq!(
            location1, location2,
            "idempotent: repeated hits land on the SAME instance"
        );
        {
            let guard = store.lock().unwrap();
            assert_eq!(
                guard.instances.len(),
                1,
                "still exactly one instance -- no duplicate workbench pages"
            );
            assert_eq!(
                guard.functions.len(),
                1,
                "still exactly one function -- re-registration is a revision bump, not a new fn"
            );
        }
    }

    #[tokio::test]
    async fn workbench_redirect_input_json_serves_live_snapshot() {
        // The mounted instance's input.json must resolve through the SAME
        // `is_workbench` live-snapshot branch `instance_input_json` already uses
        // for `st_workbench`-mounted instances -- the redirect path is not a
        // second, divergent mount mechanism.
        let store = empty_store();
        let resp = workbench_redirect(State(store.clone())).await;
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let instance_id = {
            let guard = store.lock().unwrap();
            guard.instances.keys().next().unwrap().clone()
        };
        let input_resp = instance_input_json(State(store.clone()), Path(instance_id)).await;
        assert_eq!(
            input_resp.status(),
            StatusCode::OK,
            "mounted instance serves input.json"
        );
    }
}
