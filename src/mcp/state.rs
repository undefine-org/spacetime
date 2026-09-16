//! In-memory registry for the MCP live-coding session.
//!
//! The workbench state is shared by the serial MCP stdio loop and the live HTTP
//! server. That matters because browser-originated `mcp-action` events must
//! mutate the same environments/tabs that tools like `st_env_open` and
//! `st_tab_open` inspect.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde_json::{Value, json};

use super::live::LiveServer;

/// Source of a tab's Spacetime code: a file on disk or inline text.
#[derive(Debug, Clone)]
pub enum TabSource {
    /// An entry point that exists on disk (relative to the environment root).
    File(PathBuf),
    /// Code sent directly by the agent (the usual live-coding case).
    #[allow(dead_code)]
    Inline(String),
}

/// A tab = one entry point inside an environment.
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub env_id: String,
    pub title: String,
    pub source: TabSource,
    /// PLAN-049 W2 (COLLAPSE): the env-origin FUNCTION this tab registered. A tab
    /// is no longer a compile-and-die artifact — opening it puts a function into
    /// the live registry (gallery-visible, stage-mountable). This is that
    /// function's id, so the tab card can compose it onto the stage / delete it.
    /// `None` only for a legacy tab opened before the collapse (defensive).
    pub function_id: Option<String>,
}

/// An environment = one import path (workspace directory).
#[derive(Debug, Clone)]
pub struct Environment {
    pub id: String,
    /// Absolute path to the environment root (the import path).
    pub root: PathBuf,
    pub title: String,
}

#[derive(Debug)]
struct WorkbenchRegistry {
    workspace_root: PathBuf,
    environments: BTreeMap<String, Environment>,
    tabs: BTreeMap<String, Tab>,
    next_env: u64,
    next_tab: u64,
}

/// Shared environment/tab registry used by MCP tools and the HTTP signal sink.
#[derive(Clone, Debug)]
pub struct WorkbenchSession {
    inner: Arc<Mutex<WorkbenchRegistry>>,
}

impl WorkbenchSession {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(WorkbenchRegistry {
                workspace_root,
                environments: BTreeMap::new(),
                tabs: BTreeMap::new(),
                next_env: 1,
                next_tab: 1,
            })),
        }
    }

    pub fn workspace_root(&self) -> PathBuf {
        self.inner.lock().unwrap().workspace_root.clone()
    }

    /// Attach to (or spin up) an environment at `root`. Idempotent by path:
    /// re-opening the same directory returns the existing handle.
    pub fn open_environment(&self, root: PathBuf, title: String) -> Environment {
        let mut guard = self.inner.lock().unwrap();
        if let Some(id) = guard
            .environments
            .iter()
            .find(|(_, e)| e.root == root)
            .map(|(id, _)| id.clone())
        {
            return guard.environments.get(&id).unwrap().clone();
        }
        let id = format!("env-{}", guard.next_env);
        guard.next_env += 1;
        let env = Environment {
            id: id.clone(),
            root,
            title,
        };
        guard.environments.insert(id, env.clone());
        env
    }

    pub fn environment(&self, id: &str) -> Option<Environment> {
        self.inner.lock().unwrap().environments.get(id).cloned()
    }

    pub fn environments(&self) -> Vec<Environment> {
        self.inner
            .lock()
            .unwrap()
            .environments
            .values()
            .cloned()
            .collect()
    }

    /// PLAN-049 W1.S3: close an opened environment, dropping it and every tab
    /// bound to it. Returns the dropped env's title + the number of tabs removed
    /// for the caller's summary; `None` if there was no such environment. Note:
    /// env-origin FUNCTIONS (the COLLAPSE model, W2) are a separate registry in
    /// LiveServer; the caller refuses the close while one is still mounted.
    pub fn close_environment(&self, env_id: &str) -> Option<(String, usize)> {
        let mut guard = self.inner.lock().unwrap();
        let env = guard.environments.remove(env_id)?;
        let tab_ids: Vec<String> = guard
            .tabs
            .iter()
            .filter(|(_, t)| t.env_id == env_id)
            .map(|(id, _)| id.clone())
            .collect();
        let dropped = tab_ids.len();
        for id in tab_ids {
            guard.tabs.remove(&id);
        }
        Some((env.title, dropped))
    }

    pub fn open_tab(
        &self,
        env_id: &str,
        title: String,
        source: TabSource,
        function_id: Option<String>,
    ) -> Tab {
        let mut guard = self.inner.lock().unwrap();
        // PLAN-049 W2 (reviewer W2 P2): tab-open is IDEMPOTENT per function. The
        // stable `{env_id}/{title}` fn name means re-opening the same entry
        // updates the SAME function (a new revision); a tab is the env-origin
        // VIEW of that one function, so reuse the existing tab row rather than
        // appending a duplicate. Without this, two tab records share one
        // function_id and closing one (which deletes the fn) orphans the other.
        if let Some(fid) = function_id.as_deref()
            && let Some(existing) = guard
                .tabs
                .values()
                .find(|t| t.function_id.as_deref() == Some(fid))
                .cloned()
        {
            // Refresh the title/source on the existing tab (the entry was
            // re-opened, possibly with edited inline source).
            if let Some(t) = guard.tabs.get_mut(&existing.id) {
                t.title = title;
                t.source = source;
                return t.clone();
            }
        }
        let id = format!("tab-{}", guard.next_tab);
        guard.next_tab += 1;
        let tab = Tab {
            id: id.clone(),
            env_id: env_id.to_string(),
            title,
            source,
            function_id,
        };
        guard.tabs.insert(id, tab.clone());
        tab
    }

    /// PLAN-049 W2 (reviewer W2 P2): drop every tab record referencing a given
    /// function id. Called when a function is DELETED (via any path — the tab
    /// Close button OR the generic function-delete) so a deleted function never
    /// leaves a dangling tab whose stage-open targets a missing function.
    /// Returns the number of tabs dropped.
    pub fn drop_tabs_for_function(&self, fn_id: &str) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let ids: Vec<String> = guard
            .tabs
            .iter()
            .filter(|(_, t)| t.function_id.as_deref() == Some(fn_id))
            .map(|(id, _)| id.clone())
            .collect();
        let n = ids.len();
        for id in ids {
            guard.tabs.remove(&id);
        }
        n
    }

    /// PLAN-049 W2 (COLLAPSE): the function id a tab registered, WITHOUT removing
    /// the tab. Outer `Option` = the tab exists; inner = its function id (a
    /// legacy/pre-collapse tab has `None`). Lets the caller gate a destructive
    /// close on the function's mounted-state before dropping the tab record.
    pub fn tab_function_id(&self, tab_id: &str) -> Option<Option<String>> {
        let guard = self.inner.lock().unwrap();
        guard.tabs.get(tab_id).map(|t| t.function_id.clone())
    }

    /// PLAN-049 W2 (COLLAPSE): remove a tab record by id. Returns the dropped
    /// tab's `function_id` (if any) so the caller can also delete the underlying
    /// env-origin function — closing a tab disposes BOTH halves of the collapsed
    /// object. `None` if there was no such tab.
    pub fn close_tab(&self, tab_id: &str) -> Option<Option<String>> {
        let mut guard = self.inner.lock().unwrap();
        guard.tabs.remove(tab_id).map(|t| t.function_id)
    }

    pub fn tabs_in(&self, env_id: &str) -> Vec<Tab> {
        self.inner
            .lock()
            .unwrap()
            .tabs
            .values()
            .filter(|tab| tab.env_id == env_id)
            .cloned()
            .collect()
    }

    pub fn discovered_environments_json(&self) -> Vec<Value> {
        let guard = self.inner.lock().unwrap();
        let mut discovered = Vec::new();
        for parent in ["projects", "demos", "examples"] {
            let base = guard.workspace_root.join(parent);
            let Ok(entries) = std::fs::read_dir(&base) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let rel = path
                    .strip_prefix(&guard.workspace_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                discovered.push(json!({
                    "path": rel,
                    "title": entry.file_name().to_string_lossy(),
                    "open": guard.environments.values().any(|env| env.root == path),
                }));
            }
        }
        discovered
    }

    pub fn open_environments_json(&self) -> Vec<Value> {
        self.environments()
            .into_iter()
            .map(|env| {
                json!({
                    "id": env.id,
                    "title": env.title,
                    "root": env.root.display().to_string(),
                    // PLAN-049 W2 (COLLAPSE): discoverable `.st` entries under the
                    // env root, so an open-env card can offer a one-click
                    // "+ tab" per entry (st_tab_open{env, entry}). This is the
                    // env's BROWSE surface — turning "open env" into a real
                    // entry-point picker rather than a dead card.
                    "entries": discover_st_entries(&env.root, &env.id),
                })
            })
            .collect()
    }

    pub fn tabs_json(&self, env_id: &str) -> Vec<Value> {
        self.tabs_in(env_id).into_iter().map(tab_json).collect()
    }

    pub fn all_tabs_json(&self) -> Vec<Value> {
        let guard = self.inner.lock().unwrap();
        guard.tabs.values().cloned().map(tab_json).collect()
    }
}

/// PLAN-049 W2 (COLLAPSE): discover `.st` entry points under an environment
/// root for the open-env card's entry picker. Lists files at the root and one
/// level deep (the usual `index.st` + a few siblings/subdirs), returning each as
/// `{ env, entry, title }` where `entry` is the env-root-relative path the
/// `st_tab_open` action consumes. Capped at 24 to keep the card bounded; sorted
/// for stable display. Hidden dirs + `dist/`/`node_modules/` are skipped.
fn discover_st_entries(root: &std::path::Path, env_id: &str) -> Vec<Value> {
    /// Cap the number of entries listed (keeps the card bounded AND bounds the
    /// per-poll scan — workbench_snapshot rebuilds this at 1Hz under the store
    /// lock, so traversal stops at the cap, never scanning a huge tree fully).
    const CAP: usize = 24;
    fn is_skippable(name: &str) -> bool {
        name.starts_with('.') || name == "dist" || name == "node_modules" || name == "target"
    }
    // reviewer W2 P2: a regular `.st` FILE that is not a symlink. `file_type`
    // reads the dir entry WITHOUT following a symlink, so a `link.st -> /etc/x`
    // or `linked-dir -> /tmp/other` cannot smuggle out-of-root paths into the
    // env-relative entry list.
    fn is_real_st_file(e: &std::fs::DirEntry) -> bool {
        e.file_type().is_ok_and(|ft| ft.is_file())
            && e.path().extension().is_some_and(|x| x == "st")
    }
    let mut rels: Vec<String> = Vec::new();
    let Ok(top) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    'outer: for entry in top.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_file() {
            if entry.path().extension().is_some_and(|x| x == "st") {
                rels.push(name);
                if rels.len() >= CAP {
                    break;
                }
            }
        } else if ft.is_dir() && !is_skippable(&name) {
            // Only descend into a REAL directory (ft.is_dir() is false for a
            // symlink, so a linked dir is never traversed — reviewer W2 P2).
            if let Ok(sub) = std::fs::read_dir(entry.path()) {
                for s in sub.flatten() {
                    if is_real_st_file(&s) {
                        rels.push(format!("{name}/{}", s.file_name().to_string_lossy()));
                        if rels.len() >= CAP {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    rels.sort();
    rels.into_iter()
        .map(|rel| json!({ "env": env_id, "entry": rel.clone(), "title": rel }))
        .collect()
}

fn tab_json(tab: Tab) -> Value {
    json!({
        "id": tab.id,
        "env": tab.env_id,
        "title": tab.title,
        // PLAN-049 W2 (COLLAPSE): the env-origin function this tab registered, so
        // the tab card can compose it onto the stage / delete it. `compile_ok` is
        // enriched server-side in workbench_snapshot (the WorkbenchSession has no
        // function registry; the LiveStore does).
        "function_id": tab.function_id,
        "source": match tab.source {
            TabSource::File(path) => json!({ "kind": "file", "path": path.display().to_string() }),
            TabSource::Inline(_) => json!({ "kind": "inline" }),
        },
    })
}

/// The session registry. The stdio loop is serial, but its env/tab registry is
/// shared with the live HTTP server so browser actions can be immediate.
#[derive(Debug, Clone)]
struct CachedCommentRoster {
    prelude_mtime: Option<SystemTime>,
    registry: crate::metasystem::MetaRegistry,
}

pub struct McpState {
    /// The directory the `spacetime mcp` process was launched in — the default
    /// scan root for discovering environments.
    pub workspace_root: PathBuf,
    workbench: WorkbenchSession,
    /// Lazily-booted live HTTP surface. Started on first live tool use.
    live: Option<LiveServer>,
    /// Whether the connected client advertised the `elicitation` capability
    /// (set at initialize). Gates st_elicit (server→user push).
    pub client_supports_elicitation: bool,
    /// Monotonic id source for server-initiated elicitation requests.
    pub next_elicit_id: u64,
    /// MCP resource subscriptions requested by the connected client. W1 stores
    /// intent; the async notification dispatcher lands in the next wave.
    resource_subscriptions: BTreeSet<String>,
    /// Registries are expensive stdlib loads. The project overlay is the only
    /// per-environment roster input, so its mtime is the cache invalidation key.
    comment_rosters: BTreeMap<PathBuf, CachedCommentRoster>,
}

impl McpState {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workbench: WorkbenchSession::new(workspace_root.clone()),
            workspace_root,
            live: None,
            client_supports_elicitation: false,
            next_elicit_id: 0,
            resource_subscriptions: BTreeSet::new(),
            comment_rosters: BTreeMap::new(),
        }
    }

    /// Get the live HTTP server, booting it on first use. Returns an error
    /// string if the server could not bind.
    pub fn live(&mut self) -> Result<&LiveServer, String> {
        if self.live.is_none() {
            self.live = Some(LiveServer::start(self.workbench.clone()).map_err(|e| e.to_string())?);
        }
        Ok(self.live.as_ref().unwrap())
    }

    pub fn live_if_started(&self) -> Option<&LiveServer> {
        self.live.as_ref()
    }

    pub fn open_environment(&mut self, root: PathBuf, title: String) -> Environment {
        self.workbench.open_environment(root, title)
    }

    pub fn environment(&self, id: &str) -> Option<Environment> {
        self.workbench.environment(id)
    }

    pub fn environments(&self) -> Vec<Environment> {
        self.workbench.environments()
    }

    pub fn open_tab(
        &mut self,
        env_id: &str,
        title: String,
        source: TabSource,
        function_id: Option<String>,
    ) -> Tab {
        self.workbench.open_tab(env_id, title, source, function_id)
    }

    pub fn discovered_environments_json(&self) -> Vec<Value> {
        self.workbench.discovered_environments_json()
    }

    pub fn open_environments_json(&self) -> Vec<Value> {
        self.workbench.open_environments_json()
    }

    pub fn tabs_json(&self, env_id: &str) -> Vec<Value> {
        self.workbench.tabs_json(env_id)
    }

    pub fn all_tabs_json(&self) -> Vec<Value> {
        self.workbench.all_tabs_json()
    }

    pub fn subscribe_resource(&mut self, uri: String) {
        self.resource_subscriptions.insert(uri);
    }

    pub fn unsubscribe_resource(&mut self, uri: &str) {
        self.resource_subscriptions.remove(uri);
    }

    #[cfg(test)]
    pub fn resource_subscriptions(&self) -> impl Iterator<Item = &String> {
        self.resource_subscriptions.iter()
    }
}

impl McpState {
    /// Return the roster for one environment without paying a stdlib load on
    /// every MCP call. `_prelude.st` is the only project-scoped roster source.
    pub fn comment_registry(
        &mut self,
        root: &std::path::Path,
    ) -> Result<&crate::metasystem::MetaRegistry, String> {
        let root = root.to_path_buf();
        let prelude_mtime = std::fs::metadata(root.join(crate::compiler::PROJECT_PRELUDE))
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        let stale = self
            .comment_rosters
            .get(&root)
            .is_none_or(|cached| cached.prelude_mtime != prelude_mtime);
        if stale {
            let (mut registry, errors) = crate::compiler::load_stdlib_registry();
            let overlay_errors = crate::compiler::load_project_overlay(&mut registry, Some(&root)).errors;
            let messages: Vec<String> = errors
                .into_iter()
                .chain(overlay_errors)
                .map(|error| error.message)
                .collect();
            if !messages.is_empty() {
                return Err(format!(
                    "could not load comment type roster for {}: {}",
                    root.display(),
                    messages.join("; ")
                ));
            }
            self.comment_rosters.insert(
                root.clone(),
                CachedCommentRoster {
                    prelude_mtime,
                    registry,
                },
            );
        }
        Ok(&self
            .comment_rosters
            .get(&root)
            .expect("roster inserted")
            .registry)
    }
}

// ============================================================================
// Bundle types (PLAN-041: Unified Live MCP Environment)
//
// A Bundle is the region-mountable unit: every loaded Spacetime composes INTO
// the workbench as a region-mounted bundle
// (see @research/region-model-unified-mcp.org). `bundle::compile_to_bundle`
// produces one from source; v1 is TEMPLATES ONLY.
// ============================================================================

/// A compiled Spacetime bundle: templates + page css + derived contract +
/// diagnostics. One bundle mounts into one region (`data-st-region`).
#[derive(Debug, Clone)]
pub struct Bundle {
    /// Every `@template &name(...) { ... }` (and bare `&name() {}`) in the
    /// source, serialized as a region-mountable factory + its body payload.
    pub templates: Vec<TemplateBundle>,
    /// The bundle's CSS (`CompiledSpacetime.css`). At region-mount time this is
    /// wrapped in `@scope ([data-st-region="{id}"]) { ... }`.
    pub css: String,
    /// Minimal contract derived from the templates' param specs.
    pub contract: Contract,
    /// Compile diagnostics (pipeline errors + v1 file-scope-html warnings).
    pub diagnostics: Vec<BundleDiagnostic>,
    /// True only when at least one ERROR diagnostic is present (warnings excluded).
    pub has_errors: bool,
}

/// One template within a bundle.
#[derive(Debug, Clone)]
pub struct TemplateBundle {
    /// Template name without `&` (e.g. "card").
    pub name: String,
    /// Resolved source path owning this template's body, used to address invocation spans.
    pub source_file: Option<String>,
    /// The register-template `body` payload, mirroring the structured object the
    /// serializer emits (see `TemplateBody`).
    pub body: TemplateBody,
    /// Contract-facing parameter specs (projection of the `param_list` capture).
    pub params: Vec<ParamSpec>,
    /// Scoped `@keyframes` for the template (register-template `animations` arg).
    /// `None` — the stdlib `@template` macro does not bind animations, so this is
    /// absent for macro-defined templates in v1.
    pub animations: Option<String>,
}

/// The register-template `body` payload. Field names MIRROR the structured
/// object `component_body_to_js_from_scope` (src/syntax/form_match.rs) emits —
/// `{ html, states, exports, refs, builder }` — plus `matches` (read by the
/// runtime primitive, not yet emitted by the serializer; empty in v1).
///
/// `serialized` holds the authoritative JS payload (the serializer's direct
/// output) that the region-mount path feeds to `register-template`'s `%body`.
/// The individual fields are inspectable projections for contract derivation
/// and the (design-target) contract inspector.
#[derive(Debug, Clone)]
pub struct TemplateBody {
    /// Clean body HTML (`scope.html`).
    pub html: String,
    /// Reactive DOM builder JS (`emit_builder_root_scoped` output — the SAME
    /// function the serializer calls internally, surfaced as an inspectable field).
    pub builder: String,
    /// Body local-state declarations (`$x bool: false;`).
    pub states: Vec<crate::syntax::ComponentStateDecl>,
    /// `@exports` metadata.
    pub exports: Vec<crate::syntax::ExportDecl>,
    /// Template-ref invocations within the body (`&counter()`).
    pub refs: Vec<crate::syntax::TemplateRef>,
    /// `@match` render-dispatch arms. Read by the runtime primitive
    /// (`rawBody.matches`) but NOT emitted by `component_payload_to_js`; v1-empty.
    pub matches: Vec<MatchArm>,
    /// Authoritative serialized body payload — direct output of
    /// `component_body_to_js_from_scope`. This is what registers the factory.
    pub serialized: String,
}

/// One `@match` render-dispatch arm (runtime shape `{ subject, arms }`).
/// The body serializer does not yet populate this; carried for shape completeness.
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// The match subject expression (e.g. `$variant`).
    pub subject: String,
}

/// Contract-facing parameter projection of `TemplateParamDef`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ParamSpec {
    pub name: String,
    pub kind: ParamKind,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Kind of template parameter — mirrors `TemplateParamKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ParamKind {
    /// Value param (`$name`).
    Binding,
    /// Element param (`&name`).
    Element,
}

/// Evaluate a param-default SOURCE token into its contract VALUE. A string
/// literal (`"Ship faster"` / `'x'`) yields its content with the delimiter
/// quotes stripped; a numeric/bool/other token is returned verbatim. The MCP
/// contract + region bundle are JSON, so a consumer that seeds
/// `data[name] = spec.default` needs the value, not the quoted token. (The
/// JS-codegen path in `pipeline/expand.rs` reads the raw AST `TemplateParamDef`,
/// where the quotes ARE the JS string delimiters, so it stays on the raw token.)
pub(crate) fn eval_param_default(d: &str) -> String {
    let t = d.trim();
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

impl From<&crate::syntax::TemplateParamDef> for ParamSpec {
    fn from(p: &crate::syntax::TemplateParamDef) -> Self {
        Self {
            name: p.name.clone(),
            kind: match p.kind {
                crate::syntax::TemplateParamKind::Binding => ParamKind::Binding,
                crate::syntax::TemplateParamKind::Element => ParamKind::Element,
            },
            optional: p.optional,
            type_ref: p.type_ref.clone(),
            default: p.default.as_deref().map(eval_param_default),
        }
    }
}

/// Minimal contract derived from template param specs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Contract {
    pub templates: Vec<TemplateContract>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateContract {
    pub name: String,
    pub params: Vec<ParamSpec>,
}

/// A compile diagnostic surfaced during bundle compilation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BundleDiagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
}

/// Fatal bundle-compilation failure: the source could not be reduced to an AST
/// (parse failure) or its imports could not be resolved. Semantic compile errors
/// are NOT this — they stay in `Bundle.diagnostics` with `has_errors = true`.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
}

impl CompileError {
    pub fn parse(message: String) -> Self {
        Self { message }
    }
    pub fn import(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CompileError {}

#[cfg(test)]
mod param_default_tests {
    use super::*;

    // Regression (PLAN-064 visual verify): a string-literal default MUST reach the
    // JSON contract as its VALUE, not the quoted source token. Before the fix the
    // region bundle serialized `"default":"\"Ship faster\""`, so a composed guest
    // seeding `data[title] = spec.default` rendered `"Ship faster"` (with quotes)
    // on the canvas. Substring assertions (`contains("Ship faster")`) missed it.
    #[test]
    fn eval_param_default_strips_string_delimiters() {
        assert_eq!(eval_param_default("\"Ship faster\""), "Ship faster");
        assert_eq!(eval_param_default("'x'"), "x");
        assert_eq!(eval_param_default("  \"padded\"  "), "padded");
    }

    #[test]
    fn eval_param_default_preserves_non_string_tokens() {
        assert_eq!(eval_param_default("0"), "0");
        assert_eq!(eval_param_default("42"), "42");
        assert_eq!(eval_param_default("true"), "true");
        // A lone quote or empty-ish token is returned as-is (no panic on len<2).
        assert_eq!(eval_param_default("\""), "\"");
        assert_eq!(eval_param_default(""), "");
    }

    #[test]
    fn param_spec_from_evaluates_default_to_value() {
        use crate::syntax::{TemplateParamDef, TemplateParamKind};
        let def = TemplateParamDef {
            name: "title".to_string(),
            kind: TemplateParamKind::Binding,
            optional: false,
            type_ref: None,
            default: Some("\"Ship faster\"".to_string()),
            collection: false,
        };
        let spec = ParamSpec::from(&def);
        assert_eq!(spec.default.as_deref(), Some("Ship faster"));
        // And it serializes clean (no escaped delimiter quotes in the JSON value).
        let js = serde_json::to_string(&spec).unwrap();
        assert!(js.contains("\"default\":\"Ship faster\""), "got: {js}");
        assert!(
            !js.contains("\\\"Ship"),
            "default must not carry delimiter quotes: {js}"
        );
    }
}
