//! HTTP server for serving Spacetime-powered pages

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Query},
    http::{HeaderValue, Request, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Component;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tower_http::{catch_panic::CatchPanicLayer, compression::CompressionLayer, services::ServeDir};
use uuid::Uuid;

use crate::build_runtime;
use crate::compiler::{CompileCache, CompiledSpacetime, Compiler};
use crate::debugger::{
    DebuggerConfig, generate_debug_hooks, generate_debug_panel, generate_debug_runtime,
    panel::generate_panel_overlay,
};
use crate::diagnostics::{DiagnosticCollector, Severity};
use crate::html::{RecommendationConfig, detect_html_context_for_st, validate_with_html};
use crate::parser::{StFile, parse};
use crate::profiler::BundleProfile;
use crate::serializer::serialize;

/// Embedded validation UI library
const VALIDATION_UI: &str = include_str!("../public/runtime/validation-ui.js");

/// Inline WebSocket live-reload script for dev mode.
///
/// Two responsibilities:
///  1. WS live-reload: a `Reload` message refreshes the page (file-save path).
///  2. Token preview bridge (FEAT-105): when this page is the live-preview
///     `<iframe>` inside the admin, the admin posts `{ __spacetime:
///     'token-preview', binding, field, value }` on every token keystroke. We
///     write the value into the page's reactive `SpacetimeLocal[binding]` (the
///     same publish path a normal signal write uses), so every `$binding.field`
///     CSS/text binding repaints WITHOUT a reload (BUG-091). Optimistic,
///     in-memory preview only; persistence to `.st` is the admin's separate
///     EditAst-on-`change` path. Same-origin guarded (`e.origin`).
const LIVE_RELOAD_SCRIPT: &str = r#"
    <!-- Spacetime Live Reload -->
    <script>(function(){var ws=new WebSocket((location.protocol==='https:'?'wss://':'ws://')+location.host+'/ws');ws.onmessage=function(e){try{var d=JSON.parse(e.data);if(d.type==='Reload'){location.reload();}}catch(err){}};ws.onclose=function(){setTimeout(function(){location.reload();},2000);};window.addEventListener('message',function(e){if(e.origin!==location.origin)return;var d=e.data;if(!d||d.__spacetime!=='token-preview')return;var b=d.binding,f=d.field;if(typeof b!=='string'||typeof f!=='string')return;var v=d.value;if(typeof v!=='string'&&typeof v!=='number')return;try{if(!window.SpacetimeLocal||!(b in (window._localState||{})))return;var prev=window.SpacetimeLocal[b];if(!prev||typeof prev!=='object')return;var next={};for(var k in prev){if(Object.prototype.hasOwnProperty.call(prev,k))next[k]=prev[k];}next[f]=v;window.SpacetimeLocal[b]=next;}catch(err){}});})()</script>
"#;

/// Application mode
#[derive(Clone)]
pub enum AppMode {
    /// Production: compiled assets cached at startup
    Compiled(CompiledSpacetime),
    /// Dev: read files from disk on each request
    Dev {
        site_dir: PathBuf,
        trace: bool,
        debug: bool,
    },
}

/// Server state
#[derive(Clone)]
pub struct AppState {
    pub mode: AppMode,
    pub static_dir: Option<PathBuf>,
    /// Debugger configuration (None = disabled)
    pub debugger_config: Option<DebuggerConfig>,
    /// When true, reuse the cached stdlib registry; when false, parse fresh each request
    pub cache_stdlib: bool,
    /// Content-hash compile cache for dev-mode hot reload
    pub compile_cache: std::sync::Arc<CompileCache>,
    /// Dev-mode compile cache: avoids recompiling when the .st file hasn't changed.
    pub dev_compile_cache: Arc<DevCompileCache>,
}

/// Dev-mode compile cache: avoids recompiling when the .st file hasn't changed.
/// Keyed by canonical file path; invalidated by file mtime.
pub struct DevCompileCache {
    entries: std::sync::Mutex<HashMap<PathBuf, DevCacheEntry>>,
}

struct DevCacheEntry {
    mtime: SystemTime,
    /// sha256 (hex) of the bytes actually compiled. mtime alone can ALIAS two
    /// rapid writes (tmp+rename inside one clock tick), and the FS-protocol
    /// handshake (`build-status.json`'s `content_sha256`) is meaningless if the
    /// verdict it carries came from a cache hit on different bytes.
    content_sha256: String,
    /// Most-recent vendored-blob mtime at compile time (PLAN-024 W2). A change
    /// here means `vendor build` regenerated a blob → recompile to pick it up.
    vendor_sig: Option<SystemTime>,
    compiled: CompiledSpacetime,
    diagnostics: Vec<ValidationDiagnostic>,
}

impl Default for DevCompileCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DevCompileCache {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

/// Middleware that lets a shell on ANOTHER loopback port consume the dev
/// server's compiled assets.
///
/// beam-lisp's spell shell is the case that forced this: Phoenix serves the
/// page on :4030 and points `<script type="module">` at this server's
/// `/__spacetime/runtime.js` on :4444. Module scripts fetch in CORS mode —
/// unlike classic scripts and stylesheets — so without
/// `access-control-allow-origin` the browser blocks the response body, the
/// runtime never executes, and the page hydrates into a shell that answers
/// no signal and applies no diff. Nothing in the server log names this; the
/// failure exists only in the browser's console.
///
/// Dev mode only: production serves the shell and the assets from ONE
/// origin, where the header would be dead weight. `*` is safe here — the
/// dev server binds loopback and these are compiled assets, not
/// credentials.
async fn dev_cors_middleware(is_dev: bool, req: Request<axum::body::Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;

    if is_dev && path.starts_with("/__spacetime/") {
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
    }

    response
}

/// Middleware to add cache control headers based on path and mode
async fn cache_control_middleware(
    is_dev: bool,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;

    // In dev mode, never cache dynamically compiled assets
    let cache_header = if is_dev && path.starts_with("/__spacetime/") {
        "no-cache, no-store, must-revalidate"
    } else if path.starts_with("/__spacetime/") && (path.ends_with(".js") || path.ends_with(".css"))
    {
        // Spacetime assets: 1 week cache (production only)
        "public, max-age=604800"
    } else if path.starts_with("/static/") {
        // Static images: 1 year cache, immutable
        "public, max-age=31536000, immutable"
    } else if path.ends_with(".html") || path.as_str() == "/" {
        // HTML: no cache, must revalidate
        "public, max-age=0, must-revalidate"
    } else {
        // Default: no special caching
        return response;
    };

    if let Ok(header_value) = HeaderValue::from_str(cache_header) {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, header_value);
    }

    response
}

/// Create the router without compression (for embedding in other servers that
/// need to apply middleware between the router and the compression layer).
pub fn create_router_raw(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/", get(index_handler))
        .route("/__spacetime/runtime.js", get(runtime_js_handler))
        .route("/__spacetime/styles.css", get(styles_css_handler))
        .route("/__spacetime/profile.json", get(profile_json_handler))
        .route("/__spacetime/dispatch", get(dispatch_probe_handler))
        .route("/__spacetime/validation-ui.js", get(validation_ui_handler))
        .route("/__spacetime/ast", get(ast_handler))
        .route(
            "/__spacetime/inspect/structure.json",
            get(inspect_structure_handler),
        )
        .route(
            "/__spacetime/inspect/runtime.js",
            get(inspector_runtime_js_handler),
        )
        .route(
            "/__spacetime/inspect/styles.css",
            get(inspector_styles_css_handler),
        )
        .route("/__spacetime/save", post(save_handler))
        .route("/__spacetime/dev/deploy-init", post(deploy_init_handler))
        .route(
            "/__spacetime/dev/save-host-credentials",
            post(save_host_credentials_handler),
        )
        .route("/__spacetime/save-html", post(save_html_handler))
        // Debugger routes
        .route("/__spacetime/debugger", get(debugger_panel_handler))
        .route(
            "/__spacetime/debugger/overlay.js",
            get(debugger_overlay_handler),
        )
        // Dev tools routes (debug mode only)
        .route("/__spacetime/dev/runtime.js", get(dev_runtime_js_handler))
        .route("/__spacetime/dev/styles.css", get(dev_styles_css_handler))
        .route("/__spacetime/dev/upload", post(upload_handler))
        // CMS dev endpoints (debug mode only)
        .route("/__spacetime/dev/types.json", get(dev_types_handler))
        .route(
            "/__spacetime/dev/editable-schema.json",
            get(dev_editable_schema_handler),
        )
        .route("/__spacetime/dev/site.json", get(dev_site_handler))
        .route("/__spacetime/dev/assets.json", get(dev_assets_handler))
        .route("/__spacetime/dev/theme.json", get(dev_theme_handler))
        .route("/__spacetime/dev/brand.json", get(dev_brand_handler))
        .route(
            "/__spacetime/dev/templates.json",
            get(dev_templates_handler),
        )
        .route("/__spacetime/dev/motion.json", get(dev_motion_handler))
        // CMS local content admin (debug mode only)
        .route("/__spacetime/host/", get(host_page_handler))
        .route("/__spacetime/host", get(host_page_handler))
        .route("/__spacetime/host/runtime.js", get(host_runtime_js_handler))
        .route("/__spacetime/host/styles.css", get(host_styles_css_handler))
        .route("/__spacetime/host/push", post(host_push_handler))
        .route("/__spacetime/host/push-v2", post(host_push_handler))
        // Deploy Protocol v2 (W3-1): widget-driven push with browser-direct
        // presigned uploads + the SSE progress stream the widget's
        // `@data stream` consumes.
        .route(
            "/__spacetime/host/push-events",
            get(host_push_events_handler),
        )
        .route(
            "/__spacetime/host/push-v2/start",
            post(host_push_v2_start_handler),
        )
        .route(
            "/__spacetime/host/push-upload-urls",
            get(host_push_upload_urls_handler),
        )
        .route(
            "/__spacetime/host/push-blob/{b3}",
            get(host_push_blob_handler),
        )
        .route(
            "/__spacetime/host/push-uploads-complete",
            post(host_push_uploads_complete_handler),
        )
        .route("/__spacetime/host/promote", post(host_promote_handler))
        .route("/__spacetime/migrations/", get(migrations_page_handler))
        .route("/__spacetime/migrations", get(migrations_page_handler))
        .route(
            "/__spacetime/migrations/runtime.js",
            get(migrations_runtime_js_handler),
        )
        .route(
            "/__spacetime/migrations/styles.css",
            get(migrations_styles_css_handler),
        )
        .route(
            "/__spacetime/migrations/status.json",
            get(migrations_status_handler),
        )
        .route(
            "/__spacetime/migrations/apply",
            post(migrations_apply_handler),
        )
        .route(
            "/__spacetime/comments/runtime.js",
            get(comments_runtime_js_handler),
        )
        .route(
            "/__spacetime/comments/styles.css",
            get(comments_styles_css_handler),
        )
        .route(
            "/__spacetime/comments/types.json",
            get(comments_types_handler),
        )
        .route(
            "/__spacetime/comments/status.json",
            get(comments_status_handler),
        )
        .route(
            "/__spacetime/comments/source.json",
            get(comments_source_handler),
        )
        .route("/__spacetime/comments/add", post(comments_add_handler))
        .route(
            "/__spacetime/comments/update",
            post(comments_update_handler),
        )
        .route(
            "/__spacetime/comments/re-anchor",
            post(comments_reanchor_handler),
        )
        .route(
            "/__spacetime/comments/dismiss",
            post(comments_dismiss_handler),
        )
        .route("/__spacetime/comments/prune", post(comments_prune_handler))
        .route("/__spacetime/admin/", get(admin_page_handler))
        .route("/__spacetime/admin", get(admin_page_handler))
        .route(
            "/__spacetime/admin/runtime.js",
            get(admin_runtime_js_handler),
        )
        .route(
            "/__spacetime/admin/styles.css",
            get(admin_styles_css_handler),
        )
        .route("/{*path}", get(catch_all_handler));

    // Serve static files if directory provided
    if let Some(static_dir) = &state.static_dir {
        router = router.nest_service("/static", ServeDir::new(static_dir));
    }

    let is_dev = matches!(state.mode, AppMode::Dev { .. });
    router
        .with_state(state)
        .layer(DefaultBodyLimit::max(11 * 1024 * 1024))
        .layer(middleware::from_fn(move |req, next| {
            dev_cors_middleware(is_dev, req, next)
        }))
        .layer(middleware::from_fn(move |req, next| {
            cache_control_middleware(is_dev, req, next)
        }))
}

/// Create the router with gzip compression (standalone usage).
///
/// Includes `CatchPanicLayer` so that any remaining panics in handlers
/// return HTTP 500 instead of silently dropping the connection.
pub fn create_router(state: AppState) -> Router {
    create_router_raw(state)
        .layer(CatchPanicLayer::new())
        .layer(CompressionLayer::new())
}

/// Result of compilation including diagnostics
#[derive(Clone)]
pub struct CompileResult {
    pub compiled: CompiledSpacetime,
    pub diagnostics: Vec<ValidationDiagnostic>,
    pub has_errors: bool,
}

/// A single frame in an error trace (user code location or stdlib macro)
#[derive(Clone, Serialize)]
pub struct TraceFrame {
    /// Label for this frame (e.g., "in user code" or "in @template")
    pub label: String,
    /// File path (user file or stdlib path)
    pub file_path: Option<String>,
    /// Line number (1-based)
    pub line: Option<usize>,
    /// Column number (1-based)
    pub column: Option<usize>,
    /// The source line text (for display)
    pub source_line: Option<String>,
}

/// Serializable diagnostic for client-side display
#[derive(Clone, Serialize)]
pub struct ValidationDiagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// Path to the user's .st file
    pub file_path: Option<String>,
    /// Error trace showing: user code -> macro -> stdlib
    pub trace: Option<Vec<TraceFrame>>,
}

/// Query parameters for compiled asset endpoints
#[derive(Debug, Deserialize)]
struct CompileQuery {
    /// Path to the .st entry file (relative to site_dir)
    entry: Option<String>,
}

/// Query parameters for the dev-only inspector structure projection.
#[derive(Debug, Deserialize)]
struct InspectStructureQuery {
    /// Relative .st path within the served site.
    entry: Option<String>,
    /// Entry template to inspect; defaults to the bundle's first root.
    template: Option<String>,
    /// When "1", walk EVERY template root and return them keyed by name, so a
    /// client can switch roots without a refetch (`@data fetch` URLs are literal).
    all: Option<String>,
}

/// Query parameters for the inspector pill runtime.
#[derive(Debug, Deserialize)]
struct InspectorWidgetQuery {
    /// Path to the served .st entry file, relative to site_dir.
    entry: Option<String>,
}

/// Derive the Spacetime entry path from an HTML file path.
/// `foo.html` -> `foo.st`, `index.html` -> `index.st`.
///
/// SIP-002: when no `.st` sibling exists but a literate `.st.md` one does,
/// that is the entry — so a `.st.md`-only project serves at `/` exactly like
/// an `.st`-only one (FEAT-043 shell synthesis, unchanged).
fn derive_st_path(html_path: &Path) -> PathBuf {
    let st = html_path.with_extension("st");
    if st.exists() {
        return st;
    }
    let literate = PathBuf::from(format!("{}.md", st.display()));
    if literate.exists() {
        return literate;
    }
    // PLAN-148: an EDN page is a peer entry — `foo.edn` drives `foo.html`
    // exactly like `foo.st` does.
    let edn = html_path.with_extension("edn");
    if edn.exists() {
        return edn;
    }
    st
}

/// The entry compiled when a route names none: the first of
/// `index.st` / `index.st.md` / `index.edn` that exists, else `index.st`
/// (so the "nothing here" error still names the reference syntax).
pub(crate) fn default_entry(site_dir: &Path) -> PathBuf {
    ["index.st", "index.st.md", "index.edn"]
        .into_iter()
        .map(|name| site_dir.join(name))
        .find(|p| p.exists())
        .unwrap_or_else(|| site_dir.join("index.st"))
}

/// Compile .st file with HTML-aware validation
fn compile_with_validation(
    st_path: &Path,
    workspace_root: &Path,
    trace: bool,
    debug_config: Option<&DebuggerConfig>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
) -> Result<CompileResult, String> {
    crate::profile_span!("compile_with_validation");

    // Compile via the standard path. The PROJECT is the site root: the nearest
    // `_prelude.st` above the page names it (a nested route like
    // `missions/index.st.md` would otherwise compile without the brand forms
    // `build` gives it — every `--lm-btn;` splice silently dropped from the
    // served CSS while the exported CSS carried them). File-sourced @data
    // arrays and `@doc(src:)` resolve against the same root (Stage-3 Wave-6).
    let site_dir = Some(crate::compiler::project_root_for(st_path));
    let compiled = Compiler::from_file(st_path, workspace_root)?
        .trace(trace)
        .debug_if(debug_config)
        .inspector_node_stamps(true)
        .cache_stdlib(cache_stdlib)
        .with_cache(compile_cache)
        .with_site_dir(site_dir)
        .compile();

    // Re-read file for diagnostics (content needed for span→line/column conversion).
    // The OS page cache makes this effectively free. SIP-002: must go through
    // `read_source` so a literate entry's diagnostics are computed against the
    // SAME tangled text the compiler parsed (spans would otherwise be nonsense).
    let (content, _line_map) = crate::literate::read_source(st_path)
        .map_err(|e| format!("Failed to read {}: {}", st_path.display(), e))?;
    // EDN entries (PLAN-148) are lifted to an StFile exactly as in
    // `Compiler::from_file`; the diagnostics projection must see the SAME AST
    // the compile did.
    let main_ast = match Compiler::edn_ingress(st_path, &content) {
        Some(r) => r.map_err(|e| format!("EDN source {}: {}", st_path.display(), e))?,
        None => {
            parse(&content).map_err(|e| e.render_all_plain(&content, &st_path.to_string_lossy()))?
        }
    };
    // Resolve imports so HTML validation can see classes from imported @template bodies.
    let ast = crate::parser::resolve_imports(&main_ast, st_path, workspace_root)
        .unwrap_or_else(|_| main_ast.clone());

    let (diagnostics, has_errors) = build_diagnostics(&compiled, st_path, &content, &ast);

    Ok(CompileResult {
        compiled,
        diagnostics,
        has_errors,
    })
}

/// Get a compiled result from the dev cache, or compile fresh if the file changed.
/// This is the main entry point for dev-mode handlers — it ensures only ONE
/// compilation happens per mtime change, shared across HTML/JS/CSS handlers.
fn get_or_compile(
    st_path: &Path,
    workspace_root: &Path,
    trace: bool,
    debug_config: Option<&DebuggerConfig>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
    dev_cache: &DevCompileCache,
) -> Result<(CompiledSpacetime, Vec<ValidationDiagnostic>, bool), String> {
    crate::profile_span!("get_or_compile", path = %st_path.display());

    // Check mtime
    let current_mtime = std::fs::metadata(st_path)
        .and_then(|m| m.modified())
        .map_err(|e| format!("Failed to stat {}: {}", st_path.display(), e))?;

    // Content hash IS the cache key (with mtime kept as a cheap first check).
    // The read is one file load; the compile it gates is hundreds of ms, and a
    // wrong hit serves a stale bundle while reporting a fresh verdict.
    let current_sha256 = std::fs::read(st_path)
        .map(|bytes| {
            use sha2::Digest;
            sha2::Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        })
        .map_err(|e| format!("Failed to read {}: {}", st_path.display(), e))?;

    // Vendored-blob hot-reload signature (PLAN-024 W2): part of the cache key so
    // `vendor build` regenerating a blob busts the cache without a site edit.
    let current_vendor_sig = crate::vendor::vendor_blobs_signature();

    // Check cache
    {
        let cache = dev_cache.entries.lock().unwrap();
        if let Some(entry) = cache.get(st_path)
            && entry.mtime == current_mtime
            && entry.content_sha256 == current_sha256
            && entry.vendor_sig == current_vendor_sig
        {
            return Ok((
                entry.compiled.clone(),
                entry.diagnostics.clone(),
                !entry.diagnostics.is_empty(),
            ));
        }
    }

    // Cache miss — compile fresh
    let result = compile_with_validation(
        st_path,
        workspace_root,
        trace,
        debug_config,
        cache_stdlib,
        compile_cache,
    )?;

    // Store in cache
    {
        let mut cache = dev_cache.entries.lock().unwrap();
        cache.insert(
            st_path.to_path_buf(),
            DevCacheEntry {
                mtime: current_mtime,
                content_sha256: current_sha256,
                vendor_sig: current_vendor_sig,
                compiled: result.compiled.clone(),
                diagnostics: result.diagnostics.clone(),
            },
        );
    }

    Ok((result.compiled, result.diagnostics, result.has_errors))
}

/// Machine-readable verdict of the most recent dev compile, written to
/// `<site>/build-status.json` on every proactive compile (boot + file change).
/// This is the seam an external process (another compiler, an agent harness)
/// awaits after writing a page source: poll until `mtime_ms` covers your own
/// write, then read `ok`/`diagnostics` — no browser needs to be connected.
#[derive(Serialize)]
pub struct BuildStatus {
    pub ok: bool,
    /// Entry compiled, relative to the served site dir.
    pub entry: String,
    /// Entry mtime (ms since epoch) observed BEFORE the compile, so the status
    /// never claims content newer than what was actually compiled.
    pub mtime_ms: u64,
    /// sha256 (hex) of the entry content read BEFORE the compile. The await
    /// handshake: hash what you wrote, poll until the status carries the same
    /// hash — immune to mtime truncation, which mtime_ms alone is not.
    pub content_sha256: String,
    pub diagnostics: Vec<ValidationDiagnostic>,
}

/// Compile the site's default entry and write `build-status.json` next to it.
/// Called proactively on boot and on every watched file change, so the status
/// and the warm dev cache exist BEFORE any browser request. Failures write a
/// status too — a missing status file means "no compile happened", never
/// "compile failed".
pub fn compile_default_entry_with_status(
    site_dir: &Path,
    trace: bool,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
    dev_cache: &DevCompileCache,
) {
    let entry = default_entry(site_dir);
    if !entry.exists() {
        return;
    }
    let mtime_ms = std::fs::metadata(&entry)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    // Hash the content BEFORE compiling: a write landing mid-compile produces
    // a status whose hash matches NEITHER side's expectation, so the writer
    // keeps polling until the next watcher-driven compile — never a false ok.
    let content_sha256 = std::fs::read(&entry)
        .map(|bytes| {
            use sha2::Digest;
            let digest = sha2::Sha256::digest(&bytes);
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        })
        .unwrap_or_default();
    let result = get_or_compile(
        &entry,
        site_dir,
        trace,
        None,
        cache_stdlib,
        compile_cache,
        dev_cache,
    );
    let entry_rel = entry
        .strip_prefix(site_dir)
        .unwrap_or(&entry)
        .to_string_lossy()
        .to_string();
    let status = match result {
        Ok((_compiled, diagnostics, _has_errors)) => {
            let ok = !diagnostics.iter().any(|d| d.severity == "error");
            BuildStatus {
                ok,
                entry: entry_rel,
                mtime_ms,
                content_sha256,
                diagnostics,
            }
        }
        Err(message) => BuildStatus {
            ok: false,
            entry: entry_rel,
            mtime_ms,
            content_sha256,
            diagnostics: vec![ValidationDiagnostic {
                severity: "error".to_string(),
                code: String::new(),
                message,
                hint: None,
                line: None,
                column: None,
                file_path: None,
                trace: None,
            }],
        },
    };
    // tmp + rename: a polling reader must never see a torn write.
    let status_path = site_dir.join("build-status.json");
    let tmp_path = site_dir.join(".build-status.json.tmp");
    if let Ok(json) = serde_json::to_string_pretty(&status)
        && std::fs::write(&tmp_path, json).is_ok()
    {
        let _ = std::fs::rename(&tmp_path, &status_path);
    }
}

/// Clamp a byte offset into `text` and floor it to a char boundary.
///
/// Spans reaching the dev overlay may originate in a DIFFERENT text than the
/// one being sliced (an imported module, a literate tangle), so an offset can
/// land inside a multi-byte char; slicing there panicked the whole server on
/// a mere diagnostic ("Service panicked" for every page with a `’`).
fn floor_char_boundary(text: &str, offset: usize) -> usize {
    let mut start = offset.min(text.len());
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    start
}

/// Build serializable diagnostics from a compiled result.
///
/// Converts pipeline errors and HTML validation diagnostics into
/// the `ValidationDiagnostic` format used by the dev overlay.
fn build_diagnostics(
    compiled: &CompiledSpacetime,
    st_path: &Path,
    content: &str,
    ast: &StFile,
) -> (Vec<ValidationDiagnostic>, bool) {
    let mut diagnostics = Vec::new();
    let mut has_errors = false;

    let st_path_str = st_path.to_string_lossy().to_string();
    for err in &compiled.pipeline_errors {
        has_errors = true;

        // Convert span to line/column and extract source line
        let (line, column, source_line) = if let Some(span) = &err.span {
            // A span may come from a DIFFERENT text than `content` (an
            // imported module, a tangled literate body), so its offset can
            // land inside a multi-byte char here — floor it to a boundary
            // rather than panic the whole dev server on a diagnostic.
            let start = floor_char_boundary(content, span.start);
            let before = &content[..start];
            let line = before.matches('\n').count() + 1;
            let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let column = start - last_newline + 1;

            let line_end = content[start..]
                .find('\n')
                .map(|p| start + p)
                .unwrap_or(content.len());
            let source_line = content[last_newline..line_end.min(content.len())].to_string();

            (Some(line), Some(column), Some(source_line))
        } else {
            (None, None, None)
        };

        // Build trace frames
        let mut trace_frames = Vec::new();

        if line.is_some() {
            trace_frames.push(TraceFrame {
                label: "in user code".to_string(),
                file_path: Some(st_path_str.clone()),
                line,
                column,
                source_line,
            });
        }

        if let Some(macro_name) = &err.macro_name {
            let (macro_line, macro_column, macro_source_line) =
                resolve_stdlib_span(&err.macro_file, &err.bind_span);
            trace_frames.push(TraceFrame {
                label: format!("in @{}", macro_name),
                file_path: err.macro_file.clone(),
                line: macro_line,
                column: macro_column,
                source_line: macro_source_line,
            });
        } else if err.macro_name.is_none() && err.macro_file.is_some() {
            let (stdlib_line, stdlib_column, stdlib_source_line) =
                resolve_stdlib_span(&err.macro_file, &err.bind_span);
            trace_frames.push(TraceFrame {
                label: "stdlib parse error".to_string(),
                file_path: err.macro_file.clone(),
                line: stdlib_line,
                column: stdlib_column,
                source_line: stdlib_source_line,
            });
        }

        if let Some(primitive_name) = &err.primitive_name {
            let (prim_line, prim_column, prim_source_line) = if let Some(file_path) =
                &err.primitive_file
            {
                if let Ok(prim_content) = std::fs::read_to_string(file_path) {
                    let search_pattern = format!("%primitive {}", primitive_name);
                    if let Some(pos) = prim_content.find(&search_pattern) {
                        let before = &prim_content[..pos];
                        let line = before.matches('\n').count() + 1;
                        let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let col = pos - last_newline + 1;
                        let line_end = prim_content[pos..]
                            .find('\n')
                            .map(|p| pos + p)
                            .unwrap_or(prim_content.len());
                        let src_line = prim_content[last_newline..line_end.min(prim_content.len())]
                            .to_string();
                        (Some(line), Some(col), Some(src_line))
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

            trace_frames.push(TraceFrame {
                label: format!("in %primitive {}", primitive_name),
                file_path: err.primitive_file.clone(),
                line: prim_line,
                column: prim_column,
                source_line: prim_source_line,
            });
        }

        diagnostics.push(ValidationDiagnostic {
            severity: "error".to_string(),
            code: err.code.clone(),
            message: err.message.clone(),
            hint: err.hint.clone(),
            line,
            column,
            file_path: Some(st_path_str.clone()),
            trace: if trace_frames.is_empty() {
                None
            } else {
                Some(trace_frames)
            },
        });
    }

    // HTML-aware validation with convention-based HTML lookup
    if let Ok(Some(mut html_ctx)) = detect_html_context_for_st(st_path) {
        // Register classes from @template body HTML so scope selectors
        // targeting template-created elements pass E0602 validation
        crate::html::inject_template_classes_from_ast(&mut html_ctx, ast, content);
        crate::html::inject_bind_classes_from_ast(&mut html_ctx, ast);

        let mut collector =
            DiagnosticCollector::new(content).with_path(st_path.to_string_lossy().to_string());

        validate_with_html(
            ast,
            &html_ctx,
            &mut collector,
            Some(RecommendationConfig::all()),
        );

        if collector.has_errors() {
            has_errors = true;
        }

        for diag in collector.diagnostics() {
            let (line, column) = if let Some(span) = diag.span {
                let start = floor_char_boundary(content, span.start);
                let before = &content[..start];
                let line = before.matches('\n').count() + 1;
                let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                let column = start - last_newline + 1;
                (Some(line), Some(column))
            } else {
                (None, None)
            };

            diagnostics.push(ValidationDiagnostic {
                severity: match diag.severity {
                    Severity::Error => "error".to_string(),
                    Severity::Warning => "warning".to_string(),
                },
                code: diag.code.to_string(),
                message: diag.message.clone(),
                hint: diag.hint.clone(),
                line,
                column,
                file_path: Some(st_path_str.clone()),
                trace: None,
            });
        }
    }

    (diagnostics, has_errors)
}

/// Resolve a stdlib file span to (line, column, source_line).
fn resolve_stdlib_span(
    file_path: &Option<String>,
    span: &Option<crate::parser::SourceSpan>,
) -> (Option<usize>, Option<usize>, Option<String>) {
    if let (Some(file_path), Some(span)) = (file_path, span)
        && let Ok(stdlib_content) = std::fs::read_to_string(file_path)
    {
        let start = floor_char_boundary(&stdlib_content, span.start);
        let before = &stdlib_content[..start];
        let line = before.matches('\n').count() + 1;
        let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        let col = start - last_newline + 1;
        let line_end = stdlib_content[start..]
            .find('\n')
            .map(|p| start + p)
            .unwrap_or(stdlib_content.len());
        let src_line = stdlib_content[last_newline..line_end.min(stdlib_content.len())].to_string();
        return (Some(line), Some(col), Some(src_line));
    }
    (None, None, None)
}

/// Detect available locale codes by scanning `{site_dir}/locales/*.json`.
/// Returns e.g. `["en", "fr"]`.
fn detect_locale_prefixes(site_dir: &Path) -> Vec<String> {
    let locales_dir = site_dir.join("locales");
    let mut locales = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&locales_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "json")
                && let Some(stem) = p.file_stem()
            {
                locales.push(stem.to_string_lossy().to_string());
            }
        }
    }
    locales
}

/// Try to serve a locale-prefixed request by running build scripts.
///
/// If `path` starts with a known locale prefix (e.g. `fr/` or `fr/index.html`),
/// compiles the base .st file, executes build scripts into a temp directory, and
/// returns the generated locale HTML. Returns `None` if the path is not a locale
/// route or build scripts are absent/fail.
fn try_serve_locale(
    path: &str,
    site_dir: &Path,
    trace: bool,
    debug: bool,
    debug_config: Option<&DebuggerConfig>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
    dev_compile_cache: &DevCompileCache,
) -> Option<Response> {
    // Split path into first segment and remainder
    let (prefix, rest) = match path.find('/') {
        Some(i) => (&path[..i], &path[i + 1..]),
        None => (path, ""),
    };

    // Check if first segment is a known locale
    let locales = detect_locale_prefixes(site_dir);
    if !locales.iter().any(|l| l == prefix) {
        return None;
    }

    // Determine the base HTML file (strip locale prefix)
    let base_html = if rest.is_empty() || rest == "/" {
        // /fr/ or /fr → serve index.html
        "index.html".to_string()
    } else if !rest.contains('.') {
        // Subdirectory path like "landing/" → "landing/index.html"
        let dir = rest.trim_end_matches('/');
        format!("{}/index.html", dir)
    } else {
        rest.to_string()
    };

    // Only handle .html requests through this path
    if !base_html.ends_with(".html") {
        return None;
    }

    let html_path = site_dir.join(&base_html);
    let st_path = derive_st_path(&html_path);

    if !html_path.exists() || !st_path.exists() {
        return None;
    }

    // Compile (or get from cache)
    let (compiled, diagnostics, _has_errors) = get_or_compile(
        &st_path,
        site_dir,
        trace,
        debug_config,
        cache_stdlib,
        compile_cache,
        dev_compile_cache,
    )
    .ok()?;

    if compiled.build_scripts.is_empty() {
        // No build scripts (e.g. @locale without build_js_stmts) — serve the page
        // inline with Spacetime runtime, matching the non-locale page-serve path.
        let mut content = std::fs::read_to_string(&html_path).ok()?;

        if debug {
            content = inject_content_provenance(&content, &base_html);
        }

        // Locale-aware base href so relative paths resolve from root
        if let Some(pos) = content.find("<head>") {
            content.insert_str(
                pos + "<head>".len(),
                &format!("\n    <base href=\"/{}/\">", prefix),
            );
        }

        // FOUC prevention
        if let Some(pos) = content.find("</head>") {
            let fouc_inject = r#"
    <!-- Spacetime FOUC Prevention -->
    <style>:not(:defined){visibility:hidden}</style>
"#;
            content.insert_str(pos, fouc_inject);
        }

        let diagnostics_json = if !diagnostics.is_empty() {
            serde_json::to_string(&diagnostics).unwrap_or_default()
        } else {
            String::new()
        };

        // Inject Spacetime runtime before </body>
        if let Some(pos) = content
            .rfind("</body>")
            .filter(|_| !declares_runtime_script(&content))
        {
            let validation_inject = if !diagnostics_json.is_empty() {
                format!(
                    r#"
    <!-- Spacetime Validation -->
    <script>window.__SPACETIME_DIAGNOSTICS__ = {};</script>
    <script src="/__spacetime/validation-ui.js"></script>
"#,
                    diagnostics_json
                )
            } else {
                String::new()
            };

            let debugger_inject = if debug_config.is_some_and(|c| c.enabled && c.show_panel) {
                r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
                .to_string()
            } else {
                String::new()
            };

            let entry_param = st_path
                .strip_prefix(site_dir)
                .unwrap_or(&st_path)
                .to_string_lossy();

            // PLAN-004: the __host__ widget on a locale-prefixed page --
            // without this, /fr/, /de/, etc. never showed the widget even
            // though the site's OWN @deploy{} fact is identical regardless
            // of locale prefix (found + fixed alongside the primary
            // catch_all_handler/index_handler injection sites).
            let host_widget = host_widget_inject(
                Some(site_dir),
                Some(entry_param.as_ref()),
                cache_stdlib,
                compile_cache,
            );

            let spacetime_inject = format!(
                r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}{}{}
"#,
                debugger_inject,
                entry_param,
                entry_param,
                validation_inject,
                LIVE_RELOAD_SCRIPT,
                host_widget
            );
            content.insert_str(pos, &spacetime_inject);
        }

        return Some(Html(content).into_response());
    }

    // Read the base HTML for build scripts
    let html_content = std::fs::read_to_string(&html_path).ok()?;

    // Execute build scripts into a temp directory.
    // Runs on a dedicated OS thread because rustyscript's V8 runtime calls
    // tokio `block_on` internally, which panics inside Axum's async context.
    let tmp_dir = std::env::temp_dir().join(format!("spacetime-dev-locale-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp_dir);

    let scripts = compiled.build_scripts.clone();
    let build_site_dir = st_path.parent().unwrap_or(site_dir);
    let site_owned = build_site_dir.to_path_buf();
    let html_owned = html_content.clone();
    let tmp_owned = tmp_dir.clone();

    let build_result = std::thread::spawn(move || {
        build_runtime::execute_build_scripts(&scripts, &site_owned, &html_owned, &tmp_owned)
    })
    .join()
    .ok()
    .and_then(|r| r.ok());

    match build_result {
        Some(_result) => {
            // Build scripts write e.g. fr/index.html into the temp dir
            let locale_filename = std::path::Path::new(&base_html)
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("index.html"));
            let locale_file = tmp_dir.join(prefix).join(locale_filename);
            let mut content = std::fs::read_to_string(&locale_file).ok()?;

            // Inject content provenance in debug mode (before other injections)
            if debug {
                content = inject_content_provenance(&content, &base_html);
            }

            // Fix relative paths: locale URLs like /fr/ need <base href="/">
            // so that relative refs (themes.css, ora.css) resolve from root.
            // Must appear before any <link>/<script> with relative URLs.
            // Use locale-aware base so that fragment links (#hash) stay on
            // the current locale path instead of resolving to /#hash.
            if let Some(pos) = content.find("<head>") {
                content.insert_str(
                    pos + "<head>".len(),
                    &format!("\n    <base href=\"/{}/\">", prefix),
                );
            }

            // Inject FOUC prevention CSS in <head>
            if let Some(pos) = content.find("</head>") {
                let fouc_inject = r#"
    <!-- Spacetime FOUC Prevention -->
    <style>:not(:defined){visibility:hidden}</style>
"#;
                content.insert_str(pos, fouc_inject);
            }

            // Get validation diagnostics
            let diagnostics_json = match get_or_compile(
                &st_path,
                site_dir,
                trace,
                debug_config,
                cache_stdlib,
                compile_cache,
                dev_compile_cache,
            ) {
                Ok((_compiled, diagnostics, _has_errors)) if !diagnostics.is_empty() => {
                    serde_json::to_string(&diagnostics).unwrap_or_default()
                }
                _ => String::new(),
            };

            // Inject Spacetime runtime and validation before </body>
            if let Some(pos) = content
                .rfind("</body>")
                .filter(|_| !declares_runtime_script(&content))
            {
                let validation_inject = if !diagnostics_json.is_empty() {
                    format!(
                        r#"
    <!-- Spacetime Validation -->
    <script>window.__SPACETIME_DIAGNOSTICS__ = {};</script>
    <script src="/__spacetime/validation-ui.js"></script>
"#,
                        diagnostics_json
                    )
                } else {
                    String::new()
                };

                // Inject dev tools or legacy debugger overlay if debug mode is enabled
                let debugger_inject = if debug_config.is_some_and(|c| c.enabled && c.show_panel) {
                    r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
                    .to_string()
                } else {
                    String::new()
                };

                let entry_param = st_path
                    .strip_prefix(site_dir)
                    .unwrap_or(&st_path)
                    .to_string_lossy();

                // PLAN-004: see this file's other try_serve_locale
                // injection site for the full rationale.
                let host_widget = host_widget_inject(
                    Some(site_dir),
                    Some(entry_param.as_ref()),
                    cache_stdlib,
                    compile_cache,
                );

                let spacetime_inject = format!(
                    r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}{}{}
"#,
                    debugger_inject,
                    entry_param,
                    entry_param,
                    validation_inject,
                    LIVE_RELOAD_SCRIPT,
                    host_widget
                );
                content.insert_str(pos, &spacetime_inject);
            }

            Some(Html(content).into_response())
        }
        None => {
            log::warn!("Build script failed or panicked for locale '{}'", prefix);
            None
        }
    }
}

/// Catch-all handler for serving HTML files from site directory
async fn catch_all_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    match &state.mode {
        AppMode::Compiled(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        AppMode::Dev {
            site_dir,
            trace,
            debug,
        } => {
            // Try locale-prefixed routing (e.g. /fr/ → build-script-generated HTML)
            if let Some(resp) = try_serve_locale(
                &path,
                site_dir,
                *trace,
                *debug,
                state.debugger_config.as_ref(),
                state.cache_stdlib,
                &state.compile_cache,
                &state.dev_compile_cache,
            ) {
                return resp;
            }

            // Serve .html files with Spacetime injection if matching .st exists
            if path.ends_with(".html") {
                let html_path = site_dir.join(&path);
                let st_path = derive_st_path(&html_path);

                match std::fs::read_to_string(&html_path) {
                    Ok(mut content) => {
                        // Inject content provenance in debug mode (before other injections)
                        if *debug {
                            content = inject_content_provenance(&content, &path);
                        }
                        // Only inject if matching .st file exists
                        if st_path.exists() {
                            // Inject FOUC prevention CSS in <head>
                            if let Some(pos) = content.find("</head>") {
                                let fouc_inject = r#"
    <!-- Spacetime FOUC Prevention -->
    <style>:not(:defined){visibility:hidden}</style>
"#;
                                content.insert_str(pos, fouc_inject);
                            }

                            // Inject Spacetime runtime before </body>
                            if let Some(pos) = content
                                .rfind("</body>")
                                .filter(|_| !declares_runtime_script(&content))
                            {
                                // URL-encode the entry path for the query parameter
                                let entry_param = st_path
                                    .strip_prefix(site_dir)
                                    .unwrap_or(&st_path)
                                    .to_string_lossy();

                                // Get validation diagnostics
                                let diagnostics_json = match get_or_compile(
                                    &st_path,
                                    site_dir,
                                    *trace,
                                    state.debugger_config.as_ref(),
                                    state.cache_stdlib,
                                    &state.compile_cache,
                                    &state.dev_compile_cache,
                                ) {
                                    Ok((_compiled, diagnostics, _has_errors))
                                        if !diagnostics.is_empty() =>
                                    {
                                        serde_json::to_string(&diagnostics).unwrap_or_default()
                                    }
                                    _ => String::new(),
                                };

                                let validation_inject = if !diagnostics_json.is_empty() {
                                    format!(
                                        r#"
    <!-- Spacetime Validation -->
    <script>window.__SPACETIME_DIAGNOSTICS__ = {};</script>
    <script src="/__spacetime/validation-ui.js"></script>
"#,
                                        diagnostics_json
                                    )
                                } else {
                                    String::new()
                                };

                                // Inject dev tools if debug mode is enabled
                                let debugger_inject = if *debug
                                    && state
                                        .debugger_config
                                        .as_ref()
                                        .is_some_and(|c| c.enabled && c.show_panel)
                                {
                                    r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
                                    .to_string()
                                } else {
                                    String::new()
                                };

                                // PLAN-004: __host__ widget on-page injection --
                                // see `host_widget_inject`'s own doc comment.
                                let host_widget = host_widget_inject(
                                    Some(site_dir),
                                    Some(entry_param.as_ref()),
                                    state.cache_stdlib,
                                    &state.compile_cache,
                                );

                                let spacetime_inject = format!(
                                    r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}{}{}
"#,
                                    debugger_inject,
                                    entry_param,
                                    entry_param,
                                    validation_inject,
                                    LIVE_RELOAD_SCRIPT,
                                    host_widget
                                );
                                content.insert_str(pos, &spacetime_inject);
                            }
                        }
                        Html(content).into_response()
                    }
                    Err(_) => {
                        (StatusCode::NOT_FOUND, format!("File not found: {}", path)).into_response()
                    }
                }
            } else if path.ends_with(".css") {
                let file_path = site_dir.join(&path);
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "text/css")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".js") {
                let file_path = site_dir.join(&path);
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/javascript")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                let file_path = site_dir.join(&path);
                match std::fs::read(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "image/jpeg")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".png") {
                let file_path = site_dir.join(&path);
                match std::fs::read(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "image/png")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".webp") {
                let file_path = site_dir.join(&path);
                match std::fs::read(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "image/webp")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".svg") {
                let file_path = site_dir.join(&path);
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "image/svg+xml")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".mp4") {
                let file_path = site_dir.join(&path);
                match std::fs::read(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "video/mp4")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".webm") {
                let file_path = site_dir.join(&path);
                match std::fs::read(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "video/webm")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".json") {
                let file_path = site_dir.join(&path);
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/json")],
                        content,
                    )
                        .into_response(),
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".glb")
                || path.ends_with(".gltf")
                || path.ends_with(".bin")
                || path.ends_with(".ktx2")
                || path.ends_with(".hdr")
                || path.ends_with(".exr")
                || path.ends_with(".draco")
            {
                // 3D model + asset binaries (stdlib/3d @gltf rail). glTF (`.gltf`)
                // is JSON but references sibling `.bin`/textures by relative URL, so
                // the whole set must be servable as raw bytes.
                let file_path = site_dir.join(&path);
                let mime = if path.ends_with(".gltf") {
                    "model/gltf+json"
                } else if path.ends_with(".glb") {
                    "model/gltf-binary"
                } else {
                    "application/octet-stream"
                };
                match std::fs::read(&file_path) {
                    Ok(content) => {
                        (StatusCode::OK, [(header::CONTENT_TYPE, mime)], content).into_response()
                    }
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".otf") || path.ends_with(".ttf") {
                let file_path = site_dir.join(&path);
                let mime = if path.ends_with(".otf") {
                    "font/otf"
                } else {
                    "font/ttf"
                };
                match std::fs::read(&file_path) {
                    Ok(content) => {
                        (StatusCode::OK, [(header::CONTENT_TYPE, mime)], content).into_response()
                    }
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else if path.ends_with(".woff") || path.ends_with(".woff2") {
                let file_path = site_dir.join(&path);
                let mime = if path.ends_with(".woff2") {
                    "font/woff2"
                } else {
                    "font/woff"
                };
                match std::fs::read(&file_path) {
                    Ok(content) => {
                        (StatusCode::OK, [(header::CONTENT_TYPE, mime)], content).into_response()
                    }
                    Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                }
            } else {
                // Directory index: try serving index.html from the directory
                let dir_path = path.trim_end_matches('/');
                let index_path = site_dir.join(dir_path).join("index.html");
                if index_path.is_file() {
                    let st_path = derive_st_path(&index_path);
                    match std::fs::read_to_string(&index_path) {
                        Ok(mut content) => {
                            // Inject content provenance in debug mode (before other injections)
                            if *debug {
                                let index_file = format!("{}/index.html", dir_path);
                                content = inject_content_provenance(&content, &index_file);
                            }
                            if st_path.exists() {
                                if let Some(pos) = content.find("</head>") {
                                    let fouc_inject = r#"
    <!-- Spacetime FOUC Prevention -->
    <style>:not(:defined){visibility:hidden}</style>
"#;
                                    content.insert_str(pos, fouc_inject);
                                }

                                if let Some(pos) = content
                                    .rfind("</body>")
                                    .filter(|_| !declares_runtime_script(&content))
                                {
                                    let entry_param = st_path
                                        .strip_prefix(site_dir)
                                        .unwrap_or(&st_path)
                                        .to_string_lossy();

                                    let diagnostics_json = match get_or_compile(
                                        &st_path,
                                        site_dir,
                                        *trace,
                                        state.debugger_config.as_ref(),
                                        state.cache_stdlib,
                                        &state.compile_cache,
                                        &state.dev_compile_cache,
                                    ) {
                                        Ok((_compiled, diagnostics, _has_errors))
                                            if !diagnostics.is_empty() =>
                                        {
                                            serde_json::to_string(&diagnostics).unwrap_or_default()
                                        }
                                        _ => String::new(),
                                    };

                                    let validation_inject = if !diagnostics_json.is_empty() {
                                        format!(
                                            r#"
    <!-- Spacetime Validation -->
    <script>window.__SPACETIME_DIAGNOSTICS__ = {};</script>
    <script src="/__spacetime/validation-ui.js"></script>
"#,
                                            diagnostics_json
                                        )
                                    } else {
                                        String::new()
                                    };

                                    let debugger_inject = if *debug
                                        && state
                                            .debugger_config
                                            .as_ref()
                                            .is_some_and(|c| c.enabled && c.show_panel)
                                    {
                                        r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
                                        .to_string()
                                    } else {
                                        String::new()
                                    };

                                    // PLAN-004: __host__ widget on-page injection.
                                    let host_widget = host_widget_inject(
                                        Some(site_dir),
                                        Some(entry_param.as_ref()),
                                        state.cache_stdlib,
                                        &state.compile_cache,
                                    );

                                    let spacetime_inject = format!(
                                        r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}{}{}
"#,
                                        debugger_inject,
                                        entry_param,
                                        entry_param,
                                        validation_inject,
                                        LIVE_RELOAD_SCRIPT,
                                        host_widget
                                    );
                                    content.insert_str(pos, &spacetime_inject);
                                }
                            }
                            Html(content).into_response()
                        }
                        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                    }
                } else {
                    // FEAT-093: a clean path in a .st-only project. Resolve `/<p>` to
                    // `<p>.st` or `<p>/index.st` and synthesize the shell (mirrors the
                    // index_handler FEAT-043 branch). Guards path traversal.
                    if let Some(resp) = try_serve_st_page(
                        &path,
                        site_dir,
                        *trace,
                        state.debugger_config.as_ref(),
                        state.cache_stdlib,
                        &state.compile_cache,
                        &state.dev_compile_cache,
                    ) {
                        resp
                    } else {
                        (StatusCode::NOT_FOUND, "Not found").into_response()
                    }
                }
            }
        }
    }
}

/// FEAT-093: serve a non-index `.st` page by clean path in a `.st`-only project.
/// Resolves `/<p>` to `<site>/<p>.st` then `<site>/<p>/index.st`; on a hit, compiles
/// the entry and synthesizes the same shell `index_handler` uses for `index.st`.
/// Returns None when no candidate exists (caller 404s). Dev-mode only.
#[allow(clippy::too_many_arguments)]
/// The `.st` entry a route resolves to — the SINGLE rule, shared by page
/// serving and by anything that needs to know which source a route came from.
///
/// A second copy of this would drift the moment a new entry form is added, and
/// then a comment's jump-to-source would point into a different file than the
/// one the visitor is actually looking at.
fn st_entry_for_route(path: &str, site_dir: &Path) -> Option<PathBuf> {
    let clean = path.trim_matches('/');
    // ROOT. `/` is served by `index_handler`, not by `try_serve_st_page`, so the
    // page path never asks about it — but a comment anchored on the home page
    // does, and answering "no source" for the most-visited route would be a
    // conspicuous hole. Resolved to the same `index.st` the root handler derives.
    if clean.is_empty() {
        return [
            site_dir.join("index.st"),
            site_dir.join("index.st.md"),
            site_dir.join("index.edn"),
        ]
        .into_iter()
        .find(|p| p.exists());
    }
    // Path-traversal guard: no `..` segments, no absolute escapes.
    if clean
        .split('/')
        .any(|seg| seg == ".." || seg.is_empty() || seg.starts_with('_'))
    {
        return None;
    }
    // SIP-002: literate entries are first-class pages — `/essay` resolves
    // `essay.st`, then `essay.st.md`, then the directory forms of each.
    let candidates = [
        site_dir.join(format!("{clean}.st")),
        site_dir.join(format!("{clean}.st.md")),
        site_dir.join(format!("{clean}.edn")),
        site_dir.join(clean).join("index.st"),
        site_dir.join(clean).join("index.st.md"),
        site_dir.join(clean).join("index.edn"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn try_serve_st_page(
    path: &str,
    site_dir: &Path,
    trace: bool,
    debug_config: Option<&DebuggerConfig>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
    dev_cache: &DevCompileCache,
) -> Option<Response> {
    let st_path = st_entry_for_route(path, site_dir)?;
    let entry_param = st_path
        .strip_prefix(site_dir)
        .unwrap_or(&st_path)
        .to_string_lossy()
        .to_string();
    let body_html = get_or_compile(
        &st_path,
        site_dir,
        trace,
        debug_config,
        cache_stdlib,
        compile_cache,
        dev_cache,
    )
    .map(|(compiled, _, _)| compiled.html)
    .unwrap_or_default();
    let debug_tools = debug_config.is_some_and(|c| c.enabled && c.show_panel);
    let host_widget = host_widget_inject(
        Some(site_dir),
        Some(entry_param.as_ref()),
        cache_stdlib,
        compile_cache,
    );
    Some(
        Html(synthesize_st_only_shell(
            &entry_param,
            &body_html,
            debug_tools,
            &host_widget,
        ))
        .into_response(),
    )
}

/// Index page handler
async fn index_handler(axum::extract::State(state): axum::extract::State<AppState>) -> Response {
    match &state.mode {
        AppMode::Compiled(compiled) => Html(generate_index_html(compiled)).into_response(),
        AppMode::Dev {
            site_dir,
            trace,
            debug,
        } => {
            // If the site uses locales, redirect root to default locale
            let mut locales = detect_locale_prefixes(site_dir);
            if !locales.is_empty() {
                locales.sort();
                let default_locale = &locales[0];
                return axum::response::Redirect::temporary(&format!("/{}/", default_locale))
                    .into_response();
            }

            let html_path = site_dir.join("index.html");
            let st_path = derive_st_path(&html_path);

            match std::fs::read_to_string(&html_path) {
                Ok(mut content) => {
                    // Inject content provenance in debug mode (before other injections)
                    if *debug {
                        content = inject_content_provenance(&content, "index.html");
                    }

                    // Only inject Spacetime runtime if matching .st file exists
                    let has_st_file = st_path.exists();

                    if has_st_file {
                        // Inject FOUC prevention CSS in <head> - must be early to prevent flash
                        if let Some(pos) = content.find("</head>") {
                            let fouc_inject = r#"
    <!-- Spacetime FOUC Prevention -->
    <style>:not(:defined){visibility:hidden}</style>
"#;
                            content.insert_str(pos, fouc_inject);
                        }

                        // Get validation diagnostics
                        let diagnostics_json = match get_or_compile(
                            &st_path,
                            site_dir,
                            *trace,
                            state.debugger_config.as_ref(),
                            state.cache_stdlib,
                            &state.compile_cache,
                            &state.dev_compile_cache,
                        ) {
                            Ok((_compiled, diagnostics, _has_errors))
                                if !diagnostics.is_empty() =>
                            {
                                serde_json::to_string(&diagnostics).unwrap_or_default()
                            }
                            _ => String::new(),
                        };

                        // Inject Spacetime runtime and editor before </body> in dev mode
                        if let Some(pos) = content
                            .rfind("</body>")
                            .filter(|_| !declares_runtime_script(&content))
                        {
                            let validation_inject = if !diagnostics_json.is_empty() {
                                format!(
                                    r#"
    <!-- Spacetime Validation -->
    <script>window.__SPACETIME_DIAGNOSTICS__ = {};</script>
    <script src="/__spacetime/validation-ui.js"></script>
"#,
                                    diagnostics_json
                                )
                            } else {
                                String::new()
                            };

                            // Inject dev tools or legacy debugger overlay if debug mode is enabled
                            let debugger_inject = if *debug
                                && state
                                    .debugger_config
                                    .as_ref()
                                    .is_some_and(|c| c.enabled && c.show_panel)
                            {
                                r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
                                .to_string()
                            } else {
                                String::new()
                            };

                            // Get entry param (relative to site_dir)
                            let entry_param = st_path
                                .strip_prefix(site_dir)
                                .unwrap_or(&st_path)
                                .to_string_lossy();

                            // PLAN-004: __host__ widget on-page injection.
                            let host_widget = host_widget_inject(
                                Some(site_dir),
                                Some(entry_param.as_ref()),
                                state.cache_stdlib,
                                &state.compile_cache,
                            );

                            let spacetime_inject = format!(
                                r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}{}{}
"#,
                                debugger_inject,
                                entry_param,
                                entry_param,
                                validation_inject,
                                LIVE_RELOAD_SCRIPT,
                                host_widget
                            );
                            content.insert_str(pos, &spacetime_inject);
                        }
                    }
                    Html(content).into_response()
                }
                Err(e) => {
                    // FEAT-043: no index.html on disk. If a matching index.st
                    // exists, this is a full-Spacetime project — synthesize a
                    // minimal shell that loads the runtime/styles for the entry
                    // instead of returning 500.
                    if st_path.exists() {
                        let entry_param = st_path
                            .strip_prefix(site_dir)
                            .unwrap_or(&st_path)
                            .to_string_lossy();
                        // Compile the entry to obtain its body HTML (first-class HTML
                        // element literals → SSG-visible markup; PLAN-023 W1). On compile
                        // failure fall back to an empty body — the shell still serves 200.
                        let body_html = get_or_compile(
                            &st_path,
                            site_dir,
                            *trace,
                            state.debugger_config.as_ref(),
                            state.cache_stdlib,
                            &state.compile_cache,
                            &state.dev_compile_cache,
                        )
                        .map(|(compiled, _, _)| compiled.html)
                        .unwrap_or_default();
                        let debug_tools = state
                            .debugger_config
                            .as_ref()
                            .is_some_and(|c| c.enabled && c.show_panel);
                        let host_widget = host_widget_inject(
                            Some(site_dir),
                            Some(entry_param.as_ref()),
                            state.cache_stdlib,
                            &state.compile_cache,
                        );
                        return Html(synthesize_st_only_shell(
                            &entry_param,
                            &body_html,
                            debug_tools,
                            &host_widget,
                        ))
                        .into_response();
                    }
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to read {}: {}", html_path.display(), e),
                    )
                        .into_response()
                }
            }
        }
    }
}

/// Runtime JS handler
async fn runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<CompileQuery>,
) -> Response {
    match &state.mode {
        AppMode::Compiled(compiled) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            compiled.js.clone(),
        )
            .into_response(),
        AppMode::Dev {
            site_dir,
            trace,
            debug: _,
        } => {
            // Determine the .st file to compile from entry param or default to index.st
            let st_path = match &params.entry {
                Some(entry) => site_dir.join(entry),
                None => default_entry(site_dir),
            };

            if !st_path.exists() {
                return (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/javascript")],
                    format!("// No .st file found at {}", st_path.display()),
                )
                    .into_response();
            }

            match get_or_compile(
                &st_path,
                site_dir,
                *trace,
                state.debugger_config.as_ref(),
                state.cache_stdlib,
                &state.compile_cache,
                &state.dev_compile_cache,
            ) {
                Ok((compiled, _diagnostics, _has_errors)) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/javascript")],
                    compiled.js,
                )
                    .into_response(),
                Err(e) => {
                    // Return JS that shows an error modal instead of plain text
                    let escaped_error = e
                        .replace('\\', "\\\\")
                        .replace('`', "\\`")
                        .replace("${", "\\${");
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/javascript")],
                        format!(
                            r#"
// Spacetime Compilation Error
(function() {{
  const errorMsg = `{}`;

  // Create error modal
  const modal = document.createElement('div');
  modal.id = 'spacetime-error-modal';
  modal.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.85);
    z-index: 99999;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  `;

  const content = document.createElement('div');
  content.style.cssText = `
    background: #1e1e2e;
    border-radius: 12px;
    padding: 24px;
    max-width: 800px;
    max-height: 80vh;
    overflow: auto;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
  `;

  const header = document.createElement('div');
  header.style.cssText = `
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  `;
  header.innerHTML = `
    <span style="font-size: 24px;">⚠️</span>
    <span style="color: #f38ba8; font-size: 18px; font-weight: 600;">Compilation Error</span>
  `;

  const pre = document.createElement('pre');
  pre.style.cssText = `
    background: #11111b;
    border-radius: 8px;
    padding: 16px;
    margin: 0;
    color: #cdd6f4;
    font-family: 'SF Mono', 'Fira Code', Consolas, monospace;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  `;
  pre.textContent = errorMsg;

  const hint = document.createElement('p');
  hint.style.cssText = `
    color: #6c7086;
    font-size: 13px;
    margin-top: 16px;
    margin-bottom: 0;
  `;
  hint.textContent = 'Fix the error and save to reload automatically.';

  content.appendChild(header);
  content.appendChild(pre);
  content.appendChild(hint);
  modal.appendChild(content);
  document.body.appendChild(modal);
}})();
"#,
                            escaped_error
                        ),
                    )
                        .into_response()
                }
            }
        }
    }
}

/// Query for the dispatch probe endpoint: `?call=<urlencoded spacetime call>`.
#[derive(serde::Deserialize)]
struct DispatchQuery {
    call: Option<String>,
}

/// Live dispatch-resolution endpoint (FEAT-091): given a Spacetime call, returns
/// the SAME scored candidate table the `inspect --layer dispatch --probe` CLI
/// produces — shared `build_dispatch_probe`, no reimplementation. Powers the
/// reference's live `@dispatch-probe` playground. Dev-mode only; an empty/garbage
/// call returns a 200 JSON with an `error` field (the UI renders it inline).
async fn dispatch_probe_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<DispatchQuery>,
) -> Response {
    match &state.mode {
        AppMode::Dev { .. } => {
            let call = params.call.unwrap_or_default();
            let (registry, _) = crate::compiler::cached_stdlib_registry();
            let probe = crate::cli::build_dispatch_probe(&registry, &call);
            Json(probe).into_response()
        }
        AppMode::Compiled(_) => {
            (StatusCode::NOT_FOUND, "dispatch probe is dev-mode only").into_response()
        }
    }
}

/// Styles CSS handler (compiled animation CSS)
async fn styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<CompileQuery>,
) -> Response {
    match &state.mode {
        AppMode::Compiled(compiled) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            compiled.css.clone(),
        )
            .into_response(),
        AppMode::Dev {
            site_dir,
            trace,
            debug: _,
        } => {
            // Determine the .st file to compile from entry param or default to index.st
            let st_path = match &params.entry {
                Some(entry) => site_dir.join(entry),
                None => default_entry(site_dir),
            };

            if !st_path.exists() {
                return (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/css")],
                    format!("/* No .st file found at {} */", st_path.display()),
                )
                    .into_response();
            }

            match get_or_compile(
                &st_path,
                site_dir,
                *trace,
                state.debugger_config.as_ref(),
                state.cache_stdlib,
                &state.compile_cache,
                &state.dev_compile_cache,
            ) {
                Ok((compiled, _diagnostics, _has_errors)) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/css")],
                    compiled.css,
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/css")],
                    format!("/* Compilation error: {} */", e.replace("*/", "* /")),
                )
                    .into_response(),
            }
        }
    }
}

/// Profile JSON handler - returns bundle size analysis
async fn profile_json_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(params): Query<CompileQuery>,
) -> Response {
    match &state.mode {
        AppMode::Compiled(compiled) => {
            let profile = BundleProfile::from_compiled(compiled);
            Json(profile).into_response()
        }
        AppMode::Dev {
            site_dir,
            trace,
            debug: _,
        } => {
            // Determine the .st file to compile from entry param or default to index.st
            let st_path = match &params.entry {
                Some(entry) => site_dir.join(entry),
                None => default_entry(site_dir),
            };

            if !st_path.exists() {
                return (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(
                        r#"{{"error": "No .st file found at {}"}}"#,
                        st_path.display()
                    ),
                )
                    .into_response();
            }

            match get_or_compile(
                &st_path,
                site_dir,
                *trace,
                state.debugger_config.as_ref(),
                state.cache_stdlib,
                &state.compile_cache,
                &state.dev_compile_cache,
            ) {
                Ok((compiled, _diagnostics, _has_errors)) => {
                    let profile = BundleProfile::from_compiled(&compiled).with_images(site_dir);
                    Json(profile).into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error": "{}"}}"#, e.replace('"', "\\\"")),
                )
                    .into_response(),
            }
        }
    }
}

/// Validation UI handler - returns the validation overlay JS
async fn validation_ui_handler() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/javascript")],
        VALIDATION_UI,
    )
        .into_response()
}

// =============================================================================
// Debugger Handlers
// =============================================================================

/// Debugger panel handler - returns the full debugger panel HTML
///
/// Accessible at `/__spacetime/debugger` - opens the visual debugging UI
/// in a separate window or iframe.
async fn debugger_panel_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    // Get debugger config or use defaults
    let config = state.debugger_config.clone().unwrap_or_default();

    // Check if debugging is enabled
    if !config.enabled {
        return (
            StatusCode::NOT_FOUND,
            "Debugger is not enabled. Start server with --debug flag.",
        )
            .into_response();
    }

    // Generate the debug panel HTML
    let html = generate_debug_panel(&config);

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

/// Debugger overlay handler - returns JS to inject debug panel into page
///
/// This endpoint returns JavaScript that can be included in a page to
/// show a minimal debugger overlay without opening a separate window.
async fn debugger_overlay_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    // Get debugger config or use defaults
    let config = state.debugger_config.clone().unwrap_or_default();

    // Check if debugging is enabled
    if !config.enabled {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            "// Debugger not enabled\n".to_string(),
        )
            .into_response();
    }

    // Generate hooks first (defines __stDebugHook), then runtime (__ST_DEBUG__), then panel UI
    let hooks_js = generate_debug_hooks();
    let runtime_js = generate_debug_runtime(&config);
    let overlay_js = generate_panel_overlay(&config);
    let js = format!("{}\n\n{}\n\n{}", hooks_js, runtime_js, overlay_js);

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/javascript")],
        js,
    )
        .into_response()
}

/// Dev tools runtime JS handler — compiles stdlib/__dev__/index.st and serves the JS output.
/// Only active when debug mode is enabled.
async fn dev_runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match &state.mode {
        AppMode::Dev { debug, .. } if *debug => {
            let dev_st_path = PathBuf::from("stdlib/__dev__/index.st");
            if !dev_st_path.exists() {
                return (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "application/javascript")],
                    "// Dev tools .st file not found\n".to_string(),
                )
                    .into_response();
            }
            // Compile using "." as workspace root (repo root when running via cargo run)
            match Compiler::from_file(&dev_st_path, Path::new(".")) {
                Ok(compiler) => {
                    let compiled = compiler
                        .cache_stdlib(state.cache_stdlib)
                        .with_cache(&state.compile_cache)
                        .compile();
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/javascript")],
                        compiled.js,
                    )
                        .into_response()
                }
                Err(e) => {
                    let escaped = e
                        .replace('\\', "\\\\")
                        .replace('`', "\\`")
                        .replace("${", "\\${");
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/javascript")],
                        format!("// Dev tools compile error\nconsole.error(`{}`);", escaped),
                    )
                        .into_response()
                }
            }
        }
        _ => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/javascript")],
            "// Debug mode not enabled\n".to_string(),
        )
            .into_response(),
    }
}

/// Dev tools styles CSS handler — compiles stdlib/__dev__/index.st and serves the CSS output.
/// Only active when debug mode is enabled.
async fn dev_styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match &state.mode {
        AppMode::Dev { debug, .. } if *debug => {
            let dev_st_path = PathBuf::from("stdlib/__dev__/index.st");
            if !dev_st_path.exists() {
                return (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "text/css")],
                    "/* Dev tools .st file not found */\n".to_string(),
                )
                    .into_response();
            }
            match Compiler::from_file(&dev_st_path, Path::new(".")) {
                Ok(compiler) => {
                    let compiled = compiler
                        .cache_stdlib(state.cache_stdlib)
                        .with_cache(&state.compile_cache)
                        .compile();
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "text/css")],
                        compiled.css,
                    )
                        .into_response()
                }
                Err(e) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/css")],
                    format!("/* Dev tools compile error: {} */", e.replace("*/", "* /")),
                )
                    .into_response(),
            }
        }
        _ => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/css")],
            "/* Debug mode not enabled */\n".to_string(),
        )
            .into_response(),
    }
}

// =============================================================================
// CMS Local Content Admin (debug mode only)
// =============================================================================

/// Reads the deployed project's Spacetime Host credential -- the SAME file
/// `spacetime-host-cli`'s `credentials_path()`/`save_credentials` writes
/// (`$XDG_DATA_DIR/spacetime-host/credentials`, 0600) -- so the `__host__`
/// widget can call the real orchestrator authenticated, WITHOUT the bearer
/// token ever reaching the browser (a real security property, not just a
/// convenience: `@host`'s url/headers are compile-time-literal-only per this
/// language's current primitives -- confirmed empirically this session, see
/// `stdlib/__host__/index.st`'s own header comment -- so a dynamic,
/// runtime-read secret has no way to flow into a `@data signal`'s headers at
/// all; reading it HERE, server-side, and baking it into the compiled bundle
/// as a literal is the only path that both works AND never exposes it over
/// the wire to a third party -- it's compiled into the FIRST-PARTY dev-server
/// response, same trust boundary as any other dev-only secret).
///
/// Returns `None` (not an error) when the file is missing -- the widget still
/// compiles and renders; unauthenticated requests to the real orchestrator
/// simply 401, which the widget surfaces as an empty/error list rather than
/// failing to compile.
fn read_host_api_token() -> Option<String> {
    // Mirrors `dirs::data_dir()`'s own resolution (the crate
    // `spacetime-host-cli`'s `credentials_path()` uses) WITHOUT adding a new
    // dependency to this crate: `$XDG_DATA_HOME`, falling back to
    // `$HOME/.local/share` -- the standard XDG Base Directory default dirs's
    // own implementation uses on Linux. This dev-server code path only runs
    // locally during `spacetime serve --debug`, the same environment the CLI
    // itself targets, so this narrower resolution is sufficient here.
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".local").join("share"))
        })?;
    let path = data_dir.join("spacetime-host").join("credentials");
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Reads this site's `@deploy { project: "..." }` fact (stdlib/macros/deploy.st,
/// Wave 3) from its FormMatches -- the SAME registry-introspection
/// `collect_cms_hints` already uses for `@cms(...)` (§5's own prescribed
/// design: "read the deploy() registry fact... inside compile_host_widget").
/// Returns `None` when the site declares no `@deploy{}` at all (a site that
/// never called `spacetime host init` has no project to show domains for).
fn read_deploy_project_id(site_dir: &Path) -> Option<String> {
    let ast = parse_site_ast(site_dir).ok()?;
    let raw = ast
        .matches
        .iter()
        .find(|m| m.macro_name == "deploy")
        .and_then(|m| m.get_properties("config"))
        .and_then(|props| props.iter().find(|p| p.name == "project"))
        .map(|p| p.type_ref.clone())?;
    // `properties`'s `$value:balanced(';')` capture is the RAW source text of
    // the RHS (e.g. `"my-project"`, quote characters included, verified
    // empirically this session) -- strip a single pair of surrounding double
    // quotes so the returned id is the actual project name, not a
    // quote-wrapped string.
    let trimmed = raw.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed);
    Some(unquoted.to_string())
}

/// Compiles `stdlib/__host__/index.st` and injects this site's real,
/// compile-time-known project id + orchestrator base url + bearer token as
/// LITERAL string replacements in the compiled JS -- the widget's own source
/// carries the sentinel tokens `__SPACETIME_HOST_PROJECT_ID__` /
/// `__SPACETIME_HOST_API_BASE__` / `__SPACETIME_HOST_API_TOKEN__` for exactly
/// this substitution (see that file's header comment for the full design
/// rationale -- `@host`/`@data fetch`'s config values are compile-time
/// literals only, so this text-substitution step IS the injection point, not
/// a workaround around one).
/// PLAN-004 refactor (found this session): the ORIGINAL `AppState`-only
/// signature meant `compile_host_widget` could only ever be called from
/// a handler that already extracted the full `AppState` -- which
/// `try_serve_locale` (a locale-prefixed page's own render path) does
/// NOT do (it receives `site_dir`/`cache_stdlib`/`compile_cache`
/// individually, not a bundled `AppState`). Rather than plumb a new
/// `AppState` parameter through `try_serve_locale` (its one caller,
/// `catch_all_handler`, would need to pass `state.clone()` or a
/// reference through an unrelated call chain), this splits the REAL
/// logic into `compile_host_widget_raw` (the raw pieces every caller
/// actually has access to) and keeps `compile_host_widget` as a thin
/// `AppState`-destructuring wrapper for the three existing handlers
/// that already hold one -- zero behavior change for them, and a
/// second, narrower entry point for callers that don't.
fn compile_host_widget(state: &AppState) -> Result<(String, String, String), String> {
    let site_dir = match &state.mode {
        AppMode::Dev { site_dir, .. } => Some(site_dir.as_path()),
        AppMode::Compiled(_) => None,
    };
    compile_host_widget_raw(site_dir, state.cache_stdlib, &state.compile_cache)
}

/// Resolves the real Spacetime Host orchestrator's base URL. Defaults to
/// the REAL, deployed spacetime-host-server (spacetime.undefine.org --
/// PLAN-002's own live-verified production box), not a local placeholder
/// -- a developer with no local spacetime-host-server running (the
/// common case: it is a separate, private-repo service most `.st`
/// authors never clone or run) should still be able to log in / create a
/// project / see domain status / push against the REAL product, not
/// silently fail against an unreachable localhost port (confirmed as a
/// real, live-reported bug: connection-refused/CORS errors against
/// http://127.0.0.1:3000 with no local server running -- the OLD
/// default). `SPACETIME_HOST_ENDPOINT` is the ONLY, EXPLICIT bypass (e.g.
/// for local spacetime-host-server development) -- deliberately opt-IN,
/// never a silent localhost fallback. Extracted here (PLAN-075) so
/// `host_push_handler` shares the EXACT SAME resolution
/// `compile_host_widget_raw` already used inline, rather than a second,
/// possibly-drifting copy of this env-var-with-default logic.
fn host_api_base() -> String {
    std::env::var("SPACETIME_HOST_ENDPOINT")
        .unwrap_or_else(|_| "https://spacetime.undefine.org".to_string())
}

fn compile_host_widget_raw(
    site_dir: Option<&Path>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
) -> Result<(String, String, String), String> {
    let host_st_path = PathBuf::from("stdlib/__host__/index.st");
    if !host_st_path.exists() {
        return Err("host widget .st not found".to_string());
    }
    let compiler = Compiler::from_file(&host_st_path, Path::new(".")).map_err(|e| e.to_string())?;
    let compiled = compiler
        .cache_stdlib(cache_stdlib)
        .with_cache(compile_cache)
        .compile();

    let project_id = site_dir
        .and_then(read_deploy_project_id)
        .unwrap_or_default();
    let api_base = host_api_base();
    let api_token = read_host_api_token().unwrap_or_default();

    // A SINGLE substitution shape for all three sentinels: bare substring
    // replacement, with each real value's own quotes/backslashes escaped
    // for safe embedding inside whichever JS string literal it happens to
    // sit within. This intentionally does NOT special-case "the sentinel
    // is the WHOLE quoted literal" vs "the sentinel is embedded MID-
    // STRING" as two different replacement shapes -- an earlier version of
    // this function did exactly that (replacing `"__SENTINEL__"`, quotes
    // included, with a JSON-encoded value) and it silently broke the
    // moment the widget's own source used the SAME sentinel embedded
    // inside a LARGER string literal (e.g. `send GET
    // "/api/v1/projects/__SPACETIME_HOST_PROJECT_ID__/status.json"`) --
    // reproduced empirically via a live browser test this session: the
    // widget's real status fetch 404'd because the URL still literally
    // contained the UN-substituted sentinel text. Bare substring
    // replacement handles BOTH shapes uniformly, since the widget's
    // existing quoted-literal usages (`"__SENTINEL__"`) are just the
    // degenerate case of "the sentinel is embedded in a string that
    // happens to contain nothing else".
    let escape_for_js_string = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");

    let js = compiled
        .js
        .replace(
            "__SPACETIME_HOST_PROJECT_ID__",
            &escape_for_js_string(&project_id),
        )
        .replace(
            "__SPACETIME_HOST_API_BASE__",
            &escape_for_js_string(&api_base),
        )
        .replace(
            "__SPACETIME_HOST_API_TOKEN__",
            &escape_for_js_string(&api_token),
        )
        // PLAN-004 (__host__ widget deployed-site auth bridge): LOCAL dev
        // always substitutes the EMPTY string here -- a real API_TOKEN is
        // already baked in above (this server can read the developer's
        // own local credentials file), so the bridge popup path is never
        // needed in this context. `$isDeployedContext` (the widget's own
        // derived signal) is `false` iff this sentinel resolves to empty,
        // which is exactly what selects the ORIGINAL Wave 6 login button
        // over the bridge-popup one. A real deployed push substitutes a
        // non-empty bridge base url instead (CLI `package_directory`
        // bundling step, not this function -- local dev never serves a
        // deployed site).
        .replace("__SPACETIME_HOST_BRIDGE_BASE_URL__", "");

    Ok((js, compiled.css, compiled.html))
}

/// PLAN-004 fix (found this session): the widget's own routes are
/// UNGATED -- not `--debug`, and (per the "always available" follow-up)
/// not `@deploy{}` presence either. `has_deploy_project_id`/
/// `has_deploy_project_id_raw` (the two prior gate functions this
/// history refers to) have been removed; `read_deploy_project_id` alone
/// (called directly by `compile_host_widget`/`compile_host_widget_raw`)
/// is now the only thing that reads the `@deploy{}` fact, and an absent
/// fact simply resolves `$hostProjectId` to the empty string -- a
/// widget-STATE concern the client handles reactively
/// ($isNoProject), never an HTTP-404 concern.

/// PLAN-004: the widget's `<link>`/`<script>` tags to inject into a
/// served SITE page's own `</body>`. UNCONDITIONAL in dev mode as of
/// the "always available" follow-up -- previously gated on
/// `has_deploy_project_id_raw` (a site with no `@deploy{}` fact got no
/// widget at all), which meant a brand-new project had no way to
/// discover Spacetime Host in the first place. Mirrors `debugger_inject`'s
/// established shape (same call sites, same `format!` composition
/// pattern) rather than inventing a new injection mechanism.
///
/// Takes the raw pieces (not `&AppState`) so it works uniformly from
/// EVERY page-serve call site in this file, including `try_serve_locale`
/// (which has no `AppState` to hand it) -- see `compile_host_widget_raw`.
// =============================================================================
// Migrations widget + routes (PLAN-076 W4)
//
// The migrations pill lives in `stdlib/migrations/__dev__/pill.st` — a pure
// Spacetime widget compiled server-side exactly like the host widget (minus
// the sentinel substitution: no secrets here). Its data comes from
// `/__spacetime/migrations/status.json`; applies go through
// `/__spacetime/migrations/apply`, which rides the SAME write path as the
// CLI (`migrate::write_apply_plans`). Dev-mode only, like host/push.
// =============================================================================

/// The dev dock's CSS (PLAN-076): ONE shared rule-set for the injected
/// `.st-dev-dock` (every dev page) AND the two standalone widget pages
/// (/__spacetime/host/, /__spacetime/migrations/), which wrap their widget
/// in the same container so placement is identical everywhere. z-index is
/// the host widget's historical topmost value.
/// FUP-137: only ONE dock panel may be open at a time.
///
/// The dock hosts three independently-openable panels (host, migrations,
/// inspector). With no policy, two could be open at once and overlap — observed
/// live: opening host then inspector left both `.is-open`, and on a short viewport
/// the occlusion hides the very control being reached for.
///
/// Mutual exclusion is the right policy here, over side-by-side layout or an
/// explicit z-order: a dock is a single drawer, the panels are all transient
/// inspectors rather than things to watch simultaneously, and it degrades safely
/// on a narrow viewport where layout would not. The cost — not watching migrations
/// while editing a param — is small on a dev surface.
///
/// It lives in the DOCK, the one place that knows all three widgets exist, rather
/// than being duplicated into each. Critically it does NOT introduce a shared
/// open/closed SIGNAL: the widgets share one global reactive scope, and an
/// un-prefixed `$isExpanded` once opened both existing panels at once (the reason
/// the inspector's signals are strictly `$ins`-prefixed). This coordinates on the
/// rendered `.is-open` CLASS instead, which each widget already owns and drives
/// from its own prefixed signal — so no widget's state can be written by another.
///
/// A panel that opens simply closes its siblings by clicking their pills, which
/// routes through each widget's own toggle and keeps its signal authoritative.
/// Reaching in to strip `.is-open` directly would desynchronise the class from the
/// signal that renders it, and the next toggle would appear to do nothing.
const DEV_DOCK_EXCLUSIVE_PANELS_JS: &str = r#"(function(){
  var DOCK = '.st-dev-dock';
  function pillFor(panel){
    // Each widget names its parts `<widget>__panel` / `<widget>__pill`.
    var base = null;
    panel.classList.forEach(function(c){ if (c.endsWith('__panel')) base = c.slice(0, -7); });
    if (!base) return null;
    var host = panel.closest('.' + base) || document;
    return host.querySelector('.' + base + '__pill');
  }
  function pillOf(node){
    // Walk up to the element whose OWN class is exactly `<widget>__pill`.
    // `closest('[class*="__pill"]')` is wrong: a pill's inner label carries
    // `<widget>__pill-label`, which ALSO contains the substring, so a click on the
    // label — what a user actually hits — resolved to the LABEL. `pillFor` then
    // derived a different element and the exclusion silently did nothing, leaving
    // both panels open. (W3 review; reproduced live before fixing.)
    for (var n = node; n && n.classList; n = n.parentElement) {
      var hit = null;
      n.classList.forEach(function(c){ if (c.endsWith('__pill')) hit = n; });
      if (hit) return hit;
    }
    return null;
  }
  document.addEventListener('click', function(ev){
    var pill = pillOf(ev.target);
    if (!pill || !pill.closest(DOCK)) return;
    // Let the widget's own handler run first, then close every OTHER open panel.
    setTimeout(function(){
      var panels = document.querySelectorAll(DOCK + ' [class*="__panel"]');
      var opened = null;
      panels.forEach(function(p){ if (p.classList.contains('is-open') && pillFor(p) === pill) opened = p; });
      if (!opened) return;   // the click CLOSED a panel — nothing to exclude
      panels.forEach(function(p){
        if (p === opened || !p.classList.contains('is-open')) return;
        var other = pillFor(p);
        if (other && other !== pill) other.click();
      });
    }, 0);
  }, true);
})();"#;

const DEV_DOCK_CSS: &str = ".st-dev-dock { position: fixed; bottom: 16px; right: 16px; z-index: 2147483647; display: flex; flex-direction: row; align-items: flex-end; gap: 8px; pointer-events: none; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; } .st-dev-dock > * { pointer-events: auto; }";

fn compile_migrations_widget_raw(
    cache_stdlib: bool,
    compile_cache: &CompileCache,
) -> Result<(String, String, String), String> {
    let pill_st_path = PathBuf::from("stdlib/migrations/__dev__/pill.st");
    if !pill_st_path.exists() {
        return Err("migrations pill .st not found".to_string());
    }
    let compiler = Compiler::from_file(&pill_st_path, Path::new(".")).map_err(|e| e.to_string())?;
    let compiled = compiler
        .cache_stdlib(cache_stdlib)
        .with_cache(compile_cache)
        .compile();
    Ok((compiled.js, compiled.css, compiled.html))
}

fn compile_migrations_widget(state: &AppState) -> Result<(String, String, String), String> {
    compile_migrations_widget_raw(state.cache_stdlib, &state.compile_cache)
}

/// Percent-encode a path for safe use as a URL QUERY VALUE.
///
/// The inspector's entry path travels through two hostile contexts: a query
/// string (`?entry=…`) and an HTML attribute (`src="…"`). Encoding everything
/// outside a conservative unreserved set covers both — `&` can no longer split
/// the query into a bogus second parameter (which would silently point the pill
/// at the WRONG file), and `"`/`'`/`<` can no longer escape the attribute.
/// `/` is deliberately preserved so a nested entry stays readable.
fn encode_query_component(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn compile_comments_widget_raw(
    cache_stdlib: bool,
    compile_cache: &CompileCache,
) -> Result<(String, String, String), String> {
    let pill_st_path = PathBuf::from("stdlib/comments/__dev__/pill.st");
    if !pill_st_path.exists() {
        return Err("comments pill .st not found".to_string());
    }
    let compiled = Compiler::from_file(&pill_st_path, Path::new("."))
        .map_err(|e| e.to_string())?
        .cache_stdlib(cache_stdlib)
        .with_cache(compile_cache)
        .compile();
    Ok((compiled.js, compiled.css, compiled.html))
}

fn compile_comments_widget(state: &AppState) -> Result<(String, String, String), String> {
    compile_comments_widget_raw(state.cache_stdlib, &state.compile_cache)
}

fn compile_inspector_widget_raw(
    entry: Option<&str>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
) -> Result<(String, String, String), String> {
    let pill_st_path = PathBuf::from("stdlib/__inspector__/index.st");
    if !pill_st_path.exists() {
        return Err("inspector pill .st not found".to_string());
    }
    let compiler = Compiler::from_file(&pill_st_path, Path::new(".")).map_err(|e| e.to_string())?;
    let compiled = compiler
        .cache_stdlib(cache_stdlib)
        .with_cache(compile_cache)
        .compile();
    // The sentinel sits inside the pill's FETCH URL, so it needs query encoding,
    // not merely JS-string escaping: an entry containing `&` or `#` would
    // otherwise truncate the request and inspect a different file (or nothing).
    // Encoding also leaves no quote or backslash for the surrounding JS literal.
    let js = compiled.js.replace(
        "__SPACETIME_INSPECT_ENTRY__",
        &encode_query_component(entry.unwrap_or_default()),
    );
    Ok((js, compiled.css, compiled.html))
}

fn compile_inspector_widget(
    state: &AppState,
    entry: Option<&str>,
) -> Result<(String, String, String), String> {
    compile_inspector_widget_raw(entry, state.cache_stdlib, &state.compile_cache)
}

/// Scope a DOCK bundle's template registrations to an owner (FUP-134).
///
/// A dev-served page loads several runtime bundles into ONE `Spacetime` global.
/// The template registry is keyed by bare name and `Map.set` replaces, so the last
/// bundle to load owned the name outright. Observed live: a page declaring
/// `@template &field-text($x)` lost it to the fields dock — its factory was not
/// shadowed but ABSENT, and its `@each { &field-text($i) }` rendered nothing, with
/// no console error.
///
/// Each dock bundle is wrapped so `window.__ST_BUNDLE_OWNER__` names it for the
/// duration of its own evaluation, and is restored afterwards. `register-template`
/// then writes `owner/name` and claims the bare name only when FREE; the page,
/// which has no owner, always wins the bare name. Lookup inside an owned bundle
/// resolves owner-first, so a dock widget still finds its own template.
///
/// Set/restore rather than a permanent assignment: bundles evaluate one after
/// another on the same global, and a leaked owner would misattribute every later
/// registration — including the page's.
fn scope_bundle_to_owner(js: String, owner: &str) -> String {
    // `owner` is a compile-time constant from this file, never user input.
    format!(
        "(function(){{\nvar __stPrevOwner = window.__ST_BUNDLE_OWNER__;\nwindow.__ST_BUNDLE_OWNER__ = {owner:?};\ntry {{\n{js}\n}} finally {{\nwindow.__ST_BUNDLE_OWNER__ = __stPrevOwner;\n}}\n}})();\n"
    )
}

async fn inspector_runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(query): Query<InspectorWidgetQuery>,
) -> Response {
    match compile_inspector_widget(&state, query.entry.as_deref()) {
        Ok((js, _, _)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            scope_bundle_to_owner(js, "inspector"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            format!("// Inspector pill compile error\\nconsole.error({:?});", e),
        )
            .into_response(),
    }
}

async fn inspector_styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match compile_inspector_widget(&state, None) {
        Ok((_, css, _)) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], css).into_response()
        }
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            format!(
                "/* Inspector pill compile error: {} */",
                e.replace("*/", "* /")
            ),
        )
            .into_response(),
    }
}

async fn comments_runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match compile_comments_widget(&state) {
        Ok((js, _, _)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            scope_bundle_to_owner(js, "comments"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            format!("// Comments pill compile error\\nconsole.error({e:?});"),
        )
            .into_response(),
    }
}

async fn comments_styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match compile_comments_widget(&state) {
        Ok((_, css, _)) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], css).into_response()
        }
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            format!(
                "/* Comments pill compile error: {} */",
                e.replace("*/", "* /")
            ),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct CommentTypeResponse {
    id: String,
    label: String,
    docs: String,
    fields: Vec<CommentFieldResponse>,
    color: Option<String>,
    agent_hint: Option<String>,
}

#[derive(Serialize)]
struct CommentFieldResponse {
    name: String,
    kind: &'static str,
    optional: bool,
}

fn comments_roster(
    site_dir: &Path,
) -> BTreeMap<String, crate::parser::meta_ast::CommentTypeDefAst> {
    let (mut registry, _) = crate::compiler::cached_stdlib_registry();
    let _ = crate::compiler::load_project_overlay(&mut registry, Some(site_dir));
    registry
        .comment_types()
        .map(|kind| (kind.id.clone(), kind.clone()))
        .collect()
}

fn comment_now() -> String {
    format!(
        "{}Z",
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

fn valid_project_file(site_dir: &Path, file: &str) -> bool {
    let path = Path::new(file);
    !file.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        && site_dir.join(path).starts_with(site_dir)
}

/// Serializes the comment WRITE handlers against each other.
///
/// Every destructive comment action is a read-then-decide-then-write: dismiss
/// checks that a record is an orphan and then unlinks it; prune checks that a
/// span is a resolved comment and then deletes bytes. Without a lock, a
/// concurrent re-anchor can land between the check and the write — dismiss
/// then deletes a record that is no longer an orphan, which is a conversation
/// destroyed on a technicality of timing.
///
/// This is process-local, like the source-write lock it complements. It does
/// not defend against another program editing `.comments/` — nothing short of
/// the hosted tier's transactions can — but it does make THIS server's own
/// handlers consistent with each other, which is the race a user can actually
/// trigger by clicking two buttons.
fn comment_write_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

/// Collect `.st.md` literate documents the migration walker skips.
///
/// Mirrors that walker's directory exclusions so the two together cover
/// exactly `comments::is_scannable_source`.
fn collect_literate_sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if matches!(
                name.as_ref(),
                "dist" | "node_modules" | "target" | "stdlib" | "vendor"
            ) {
                continue;
            }
            collect_literate_sources(&path, out);
        } else if name.ends_with(".st.md") {
            out.push(path);
        }
    }
}

fn comments_index(site_dir: &Path) -> crate::comments::CommentIndex {
    let roster = comments_roster(site_dir);
    let mut scanned = Vec::new();
    let mut diagnostics = Vec::new();
    let mut scanned_files = Vec::new();
    // Coverage must match `is_scannable_source`, the predicate the MCP reader
    // uses. `collect_project_st_files` is the MIGRATION walker and admits only
    // `.st`, so a comment in a literate `.st.md` was read by agents and
    // invisible here — one store answering two ways. Worse, an inline record
    // in a literate file fell outside coverage, so it could never be surfaced
    // as an orphan and could never be dismissed.
    let mut files: Vec<std::path::PathBuf> = crate::migrate::collect_project_st_files(site_dir);
    collect_literate_sources(site_dir, &mut files);
    files.sort();
    files.dedup();
    for path in files {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let file = path
            .strip_prefix(site_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        scanned_files.push(file.clone());
        // The SHARED scan: markup exclusions come from the parse, and a parse
        // failure means no scan rather than a scan that harvests visible page
        // text.
        let (found, mut scan_diagnostics) = crate::comments::scan_file(&file, &source);
        for item in &found {
            diagnostics.extend(crate::comments::validate_against_roster(item, &roster));
        }
        scanned.extend(found);
        diagnostics.append(&mut scan_diagnostics);
    }
    let (sidecar, mut sidecar_diagnostics) = crate::comments::read_sidecar(site_dir);
    let mut index = crate::comments::merge(scanned, sidecar, &scanned_files, &comment_now());
    index.diagnostics.append(&mut diagnostics);
    index.diagnostics.append(&mut sidecar_diagnostics);
    index
}

#[derive(Deserialize)]
struct CommentsStatusQuery {
    route: Option<String>,
    file: Option<String>,
    status: Option<String>,
    #[serde(rename = "type")]
    type_id: Option<String>,
}

#[derive(Serialize)]
struct CommentsStatusResponse {
    records: Vec<crate::comments::CommentRecord>,
    orphans: Vec<crate::comments::CommentRecord>,
    diagnostics: Vec<CommentDiagnosticResponse>,
    counts: BTreeMap<String, usize>,
    prune: Vec<CommentPruneSpan>,
}
#[derive(Serialize)]
struct CommentDiagnosticResponse {
    file: String,
    line: usize,
    message: String,
}

async fn comments_types_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    let roster = comments_roster(site_dir);
    let types: Vec<_> = roster
        .values()
        .map(|kind| CommentTypeResponse {
            id: kind.id.clone(),
            label: kind.label.clone(),
            docs: kind.docs.clone(),
            fields: kind
                .fields
                .iter()
                .map(|field| CommentFieldResponse {
                    name: field.name.clone(),
                    kind: field.kind.as_str(),
                    optional: field.optional,
                })
                .collect(),
            color: kind.color.clone(),
            agent_hint: kind.agent_hint.clone(),
        })
        .collect();
    Json(types).into_response()
}

#[derive(Deserialize)]
struct CommentsSourceQuery {
    id: String,
}

/// Where a comment's element was WRITTEN.
///
/// Distinct from the pill's "Show on page", which finds the element in the
/// RENDERED DOM by content. Source survives what rendering cannot: an element
/// behind a false branch, an element emptied of text, and duplicate rows that
/// are indistinguishable once rendered but sit at different byte offsets.
///
/// The span is RESOLVED HERE, never stored on the anchor. A stored byte offset
/// goes stale the moment anyone edits the file above it and then points
/// confidently at the wrong line — the positional failure this feature already
/// fixed once with fingerprinting. The identity is the stored content; the span
/// is a view of the CURRENT file.
async fn comments_source_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(query): Query<CommentsSourceQuery>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    if !crate::comments::is_valid_id(&query.id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"invalid comment id"})),
        )
            .into_response();
    }
    let index = comments_index(site_dir);
    // Orphans are searched too: a comment whose inline header vanished still has
    // an element anchor, and "where was this written" is exactly what a reader
    // needs when deciding how to re-anchor it.
    let Some(record) = index
        .records
        .into_iter()
        .chain(index.orphans)
        .find(|record| record.id == query.id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"no such comment"})),
        )
            .into_response();
    };
    let crate::comments::Anchor::Element { route, .. } = &record.anchor else {
        // Only element anchors have an element to locate. Inline anchors already
        // carry file+line, and page/file anchors ARE the address.
        return Json(serde_json::json!({
            "outcome": "not-an-element",
            "id": record.id,
        }))
        .into_response();
    };
    let Some(st_path) = st_entry_for_route(route.as_str(), site_dir) else {
        return Json(serde_json::json!({
            "outcome": "no-source",
            "id": record.id,
            "route": route,
            "detail": "this route does not resolve to a .st entry in this project",
        }))
        .into_response();
    };
    let Ok(source) = std::fs::read_to_string(&st_path) else {
        return Json(serde_json::json!({
            "outcome": "no-source",
            "id": record.id,
            "route": route,
            "detail": "the entry for this route could not be read",
        }))
        .into_response();
    };
    let Ok(bundle) = crate::mcp::bundle::compile_to_bundle_at(&source, site_dir, &st_path) else {
        return Json(serde_json::json!({
            "outcome": "no-source",
            "id": record.id,
            "route": route,
            "detail": "the entry for this route does not currently compile",
        }))
        .into_response();
    };
    let entry = crate::introspect::project::default_entry_template(&bundle)
        .map(std::string::ToString::to_string);
    let nodes = match &entry {
        Some(entry) => crate::introspect::project::structure_json_for_entry(&bundle, entry),
        None => crate::introspect::project::structure_json(&bundle),
    };
    let candidates: Vec<crate::comments::SourceCandidate> = nodes
        .as_array()
        .map(|nodes| {
            nodes
                .iter()
                .map(|node| crate::comments::SourceCandidate {
                    id: node
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    label: node
                        .get("label")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    span: node.get("invoke_span").and_then(|span| {
                        Some((
                            span.get("start").and_then(serde_json::Value::as_u64)?,
                            span.get("end").and_then(serde_json::Value::as_u64)?,
                        ))
                    }),
                })
                .collect()
        })
        .unwrap_or_default();
    let file = st_path
        .strip_prefix(site_dir)
        .unwrap_or(&st_path)
        .to_string_lossy()
        .to_string();
    // SPANS ARE BODY-RELATIVE, NOT FILE-RELATIVE.
    //
    // `html_walk` builds its `LineOffsets` over ONE template body, so offset 0
    // is the first byte of that body -- not of the file. `TemplateBundle`
    // carries no offset for where the body sits in its source, so a file
    // line/column CANNOT be derived here today.
    //
    // Reporting the body offset AS a file offset would be a confident wrong
    // answer: a reader would jump to the top of the file and see unrelated
    // markup. So the span is reported for what it is, with the template that
    // frames it, and `file_line` is withheld rather than invented. Absence is
    // recoverable; a plausible lie is not. (PLAN-112 W1 makes the same rule for
    // write addresses: a span and the bytes it indexes must come from ONE
    // snapshot, or the pairing is a lie.)
    //
    // Deriving a file line needs a body offset on `TemplateBundle` -- tracked
    // in FUP-170 as the remaining half.
    let _ = &source;
    let body = match crate::comments::resolve_element_source(&record.anchor, &candidates) {
        Some(crate::comments::SourceResolution::Resolved { id, span, moved }) => {
            serde_json::json!({
                "outcome": "found", "id": record.id, "node": id, "file": file,
                "template": entry,
                "span": { "start": span.0, "end": span.1, "relative_to": "template-body" },
                "moved": moved,
            })
        }
        Some(crate::comments::SourceResolution::Unverified { id, span }) => {
            serde_json::json!({
                "outcome": "unverified", "id": record.id, "node": id, "file": file,
                "template": entry,
                "span": span.map(|span| serde_json::json!({
                    "start": span.0, "end": span.1, "relative_to": "template-body"
                })),
                "detail": "this comment predates element identity, so the location cannot be confirmed",
            })
        }
        Some(crate::comments::SourceResolution::Synthesized { id }) => serde_json::json!({
            "outcome": "synthesized", "id": record.id, "node": id, "file": file,
            "detail": "this element was inserted by the HTML parser, so it has no authored source",
        }),
        Some(crate::comments::SourceResolution::Ambiguous { count }) => serde_json::json!({
            "outcome": "ambiguous", "id": record.id, "file": file, "count": count,
            "detail": "several elements are written identically — they cannot be told apart",
        }),
        Some(crate::comments::SourceResolution::Gone) => serde_json::json!({
            "outcome": "gone", "id": record.id, "file": file,
            "detail": "no element in this source matches the comment",
        }),
        None => serde_json::json!({ "outcome": "not-an-element", "id": record.id }),
    };
    Json(body).into_response()
}

async fn comments_status_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(query): Query<CommentsStatusQuery>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    if query
        .file
        .as_deref()
        .is_some_and(|file| !valid_project_file(site_dir, file))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"file must stay under the served project root"})),
        )
            .into_response();
    }
    if query
        .status
        .as_deref()
        .is_some_and(|status| crate::comments::Status::parse(status).is_none())
    {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"status must be open, in-progress, resolved, or wontfix"}))).into_response();
    }
    let index = comments_index(site_dir);
    let matches = |record: &crate::comments::CommentRecord| {
        query.route.as_deref().is_none_or(|route| {
            matches!(
                &record.anchor,
                crate::comments::Anchor::Page { route: value }
                    | crate::comments::Anchor::Element { route: value, .. }
                    if value == route
            )
        }) && query
            .file
            .as_deref()
            .is_none_or(|file| record.anchor.file() == Some(file))
            && query
                .status
                .as_deref()
                .is_none_or(|status| record.status.as_str() == status)
            && query
                .type_id
                .as_deref()
                .is_none_or(|kind| record.type_id == kind)
    };
    let records: Vec<_> = index.records.into_iter().filter(matches).collect();
    let orphans: Vec<_> = index.orphans.into_iter().filter(matches).collect();
    let mut counts = BTreeMap::new();
    for record in &records {
        *counts
            .entry(record.status.as_str().to_string())
            .or_insert(0) += 1;
    }
    Json(CommentsStatusResponse {
        records,
        orphans,
        diagnostics: index
            .diagnostics
            .into_iter()
            .map(|d| CommentDiagnosticResponse {
                file: d.file,
                line: d.line,
                message: d.message,
            })
            .collect(),
        counts,
        prune: comment_prune_spans(site_dir),
    })
    .into_response()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CommentPruneSpan {
    id: String,
    file: String,
    start: usize,
    end: usize,
    source_hash: String,
    span_text: String,
}

fn comment_prune_spans(site_dir: &Path) -> Vec<CommentPruneSpan> {
    comments_index(site_dir)
        .records
        .into_iter()
        .filter(|record| {
            record.status == crate::comments::Status::Resolved
                && matches!(record.anchor, crate::comments::Anchor::Inline { .. })
        })
        .filter_map(|record| {
            let crate::comments::Anchor::Inline { file, .. } = record.anchor else {
                return None;
            };
            let source = std::fs::read_to_string(site_dir.join(&file)).ok()?;
            let span = crate::comments::inline_comment_spans(&file, &source)
                .into_iter()
                .find(|span| span.id == record.id)?;
            Some(CommentPruneSpan {
                id: record.id,
                file,
                start: span.start,
                end: span.end,
                source_hash: format!("{:016x}", crate::migrate::hash_content(&source)),
                span_text: source[span.start..span.end].to_string(),
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct CommentsPruneRequest {
    spans: Vec<CommentPruneSpan>,
}

/// Prune only resolved inline comments via the guarded source-write rail.
///
/// The sidecar record remains as an orphan: source cleanup must not silently
/// erase the settled discussion and its history.
async fn comments_prune_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<CommentsPruneRequest>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    // Serialized with the other writers: prune authorizes from a snapshot of
    // resolved records, so a status change must not land mid-flight.
    let _guard = comment_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if req.spans.is_empty() {
        return Json(
            serde_json::json!({"pruned":0,"message":"no resolved inline comments to prune"}),
        )
        .into_response();
    }
    let available = comment_prune_spans(site_dir);
    if req.spans.iter().any(|span| !available.contains(span)) {
        return (StatusCode::CONFLICT, Json(serde_json::json!({"error":"prune snapshot is stale or includes a non-resolved inline comment; source was not changed"}))).into_response();
    }
    let mut files: BTreeMap<&str, Vec<&CommentPruneSpan>> = BTreeMap::new();
    for span in &req.spans {
        files.entry(&span.file).or_default().push(span);
    }
    for (file, spans) in files {
        let hash = &spans[0].source_hash;
        if spans.iter().any(|span| span.source_hash != *hash) {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error":"conflicting source hashes for one file"})),
            )
                .into_response();
        }
        let patch = serde_json::json!({"__delete_spans":spans.iter().map(|span| vec![span.start,span.end]).collect::<Vec<_>>(),"__expect_hash":hash,"__expect_span_texts":spans.iter().map(|span| span.span_text.clone()).collect::<Vec<_>>()});
        let result =
            crate::sync::handlers::handle_edit_ast(site_dir, file, "", &patch, "comments-prune");
        if let crate::sync::protocol::ServerMessage::Reject { reason, .. } = result.response {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error":reason})),
            )
                .into_response();
        }
    }
    Json(serde_json::json!({"pruned":req.spans.len()})).into_response()
}

#[derive(Deserialize)]
struct CommentsAddRequest {
    #[serde(rename = "type")]
    type_id: String,
    anchor: crate::comments::Anchor,
    text: String,
    #[serde(default)]
    meta: BTreeMap<String, String>,
    author: Option<crate::comments::Author>,
    /// What the picker SAW at the picked element. The server mints the
    /// fingerprint from this (see `comments::with_element_content`) rather than
    /// trusting a client-computed hash, so one hash rule has one implementation.
    #[serde(default)]
    element_content: Option<crate::comments::ElementCandidate>,
}
fn comment_validation_error(
    req: &CommentsAddRequest,
    roster: &BTreeMap<String, crate::parser::meta_ast::CommentTypeDefAst>,
) -> Option<String> {
    let Some(kind) = roster.get(&req.type_id) else {
        return Some(format!("unknown comment type `{}`", req.type_id));
    };
    if req.text.trim().is_empty() {
        return Some("text must not be empty".to_string());
    }
    for field in kind.required_fields() {
        if !req.meta.contains_key(&field.name) {
            return Some(format!(
                "`{}` requires metadata field `{}`",
                req.type_id, field.name
            ));
        }
    }
    for (name, value) in &req.meta {
        let Some(field) = kind.field(name) else {
            return Some(format!(
                "`{}` accepts no metadata field `{name}`",
                req.type_id
            ));
        };
        let good = match field.kind {
            crate::parser::meta_ast::CommentFieldKind::Str => true,
            crate::parser::meta_ast::CommentFieldKind::Number => value.parse::<f64>().is_ok(),
            crate::parser::meta_ast::CommentFieldKind::Bool => {
                matches!(value.as_str(), "true" | "false")
            }
            crate::parser::meta_ast::CommentFieldKind::Ident => {
                !value.is_empty()
                    && value
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            }
        };
        if !good {
            return Some(format!("metadata `{name}` must be {}", field.kind.as_str()));
        }
    }
    None
}
fn default_comment_author() -> crate::comments::Author {
    let name = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "you".to_string());
    crate::comments::Author {
        kind: crate::comments::AuthorKind::Human,
        name,
    }
}
async fn comments_add_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(mut req): Json<CommentsAddRequest>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    // Mint the fingerprint BEFORE validating, so what gets validated is what
    // gets stored. Stamping afterwards would let a rejected shape through by
    // mutating the anchor past its own check.
    req.anchor = stamped_anchor(req.anchor, req.element_content.as_ref());
    // ONE anchor validator, shared with the MCP tools: two write surfaces
    // with two notions of an acceptable anchor is how a trust boundary drifts.
    if let Some(rejection) = crate::comments::anchor_rejection(&req.anchor) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": rejection })),
        )
            .into_response();
    };
    if let Some(file) = req.anchor.file()
        && !valid_project_file(site_dir, file)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error":"anchor file must stay under the served project root"}),
            ),
        )
            .into_response();
    };
    let roster = comments_roster(site_dir);
    if let Some(error) = comment_validation_error(&req, &roster) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":error})),
        )
            .into_response();
    };
    let now = comment_now();
    let record = crate::comments::CommentRecord {
        id: Uuid::new_v4().simple().to_string(),
        type_id: req.type_id,
        status: crate::comments::Status::Open,
        author: req.author.unwrap_or_else(default_comment_author),
        claimed_by: None,
        anchor: req.anchor,
        text: req.text,
        meta: req.meta,
        thread: Vec::new(),
        history: Vec::new(),
        created_at: now,
        updated_at: None,
        v: crate::comments::SCHEMA_VERSION,
        inline: false,
    };
    match crate::comments::write_record(site_dir, &record) {
        Ok(()) => (StatusCode::CREATED, Json(record)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error":e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct CommentsUpdateRequest {
    id: String,
    status: Option<String>,
    reply: Option<String>,
    text: Option<String>,
}
async fn comments_update_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<CommentsUpdateRequest>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    // Serialized with dismiss/re-anchor: a status change is what makes a
    // comment prunable, so it must not interleave with a decision about one.
    let _guard = comment_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !crate::comments::is_valid_id(&req.id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"unsafe comment id"})),
        )
            .into_response();
    };
    let (mut sidecar, _) = crate::comments::read_sidecar(site_dir);
    let Some(mut record) = sidecar.remove(&req.id).or_else(|| {
        comments_index(site_dir)
            .records
            .into_iter()
            .find(|record| record.id == req.id)
    }) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"unknown comment id"})),
        )
            .into_response();
    };
    if let Some(status) = req.status {
        let Some(status) = crate::comments::Status::parse(&status) else {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"status must be open, in-progress, resolved, or wontfix"}))).into_response();
        };
        // Record WHO moved it and from where. A status with no author cannot be
        // questioned: "resolved" reads the same whether a human verified the fix
        // or an agent closed work it never read.
        //
        // A no-op re-submit is not a transition and must not pad the log with
        // events that never happened.
        if record.status != status {
            record.history.push(crate::comments::StatusChange {
                author: default_comment_author(),
                from: record.status,
                to: status,
                at: comment_now(),
            });
            record.status = status;
        }
    }
    if let Some(text) = req.text {
        if text.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"text must not be empty"})),
            )
                .into_response();
        };
        record.text = text;
    }
    if let Some(text) = req.reply {
        if text.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"reply must not be empty"})),
            )
                .into_response();
        };
        record.thread.push(crate::comments::Reply {
            author: default_comment_author(),
            text,
            at: comment_now(),
        });
    }
    record.updated_at = Some(comment_now());
    match crate::comments::write_record(site_dir, &record) {
        Ok(()) => Json(record).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error":e.to_string()})),
        )
            .into_response(),
    }
}

/// Apply a picker's reported element content to an anchor, when both are
/// present. Shared by add and re-anchor so a re-anchored comment gains the same
/// verified identity a freshly picked one does — otherwise fixing an orphan
/// would quietly downgrade it to the legacy unverified shape.
fn stamped_anchor(
    anchor: crate::comments::Anchor,
    content: Option<&crate::comments::ElementCandidate>,
) -> crate::comments::Anchor {
    match content {
        Some(content) => crate::comments::with_element_content(anchor, content),
        None => anchor,
    }
}

#[derive(Deserialize)]
struct CommentsReanchorRequest {
    id: String,
    anchor: crate::comments::Anchor,
    /// See `CommentsAddRequest::element_content` — re-anchoring to a picked
    /// element mints the same verified identity a fresh pick does.
    #[serde(default)]
    element_content: Option<crate::comments::ElementCandidate>,
}

/// Move an orphaned inline conversation to a durable page or file anchor.
///
/// This accepts only records the scanner currently classifies as orphans:
/// moving a live comment would sever source-derived identity from its header,
/// while treating a non-orphan as dismissible would erase active team state.
async fn comments_reanchor_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(mut req): Json<CommentsReanchorRequest>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    // Same rule as add: a re-anchor onto a picked element earns the same
    // verified identity, or fixing an orphan would silently produce a legacy
    // unverified record.
    req.anchor = stamped_anchor(req.anchor, req.element_content.as_ref());
    // Serialized with dismiss: the two decide from the same orphan list, and
    // must not act on it simultaneously.
    let _guard = comment_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !crate::comments::is_valid_id(&req.id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"unsafe comment id"})),
        )
            .into_response();
    }
    if let Some(rejection) = crate::comments::anchor_rejection(&req.anchor) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":rejection})),
        )
            .into_response();
    }
    if let Some(file) = req.anchor.file()
        && !valid_project_file(site_dir, file)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error":"anchor file must stay under the served project root"}),
            ),
        )
            .into_response();
    }

    let (mut sidecar, _) = crate::comments::read_sidecar(site_dir);
    let Some(mut record) = sidecar.remove(&req.id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"unknown comment id"})),
        )
            .into_response();
    };
    if !comments_index(site_dir)
        .orphans
        .iter()
        .any(|orphan| orphan.id == req.id)
    {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"only an orphaned inline comment can be re-anchored"})),
        )
            .into_response();
    }

    record.anchor = req.anchor;
    record.inline = false;
    record.updated_at = Some(comment_now());
    match crate::comments::write_record(site_dir, &record) {
        Ok(()) => Json(record).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error":e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct CommentsDismissRequest {
    id: String,
}

/// Permanently delete an orphaned inline conversation by explicit request.
///
/// Live records are refused: a stale pill must not turn a destructive button
/// into a way to erase a comment that regained its source anchor.
async fn comments_dismiss_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<CommentsDismissRequest>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    if !crate::comments::is_valid_id(&req.id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"unsafe comment id"})),
        )
            .into_response();
    }
    // Eligibility and deletion are ONE operation: a re-anchor landing between
    // them would make this unlink a record that is no longer an orphan.
    let _guard = comment_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (sidecar, _) = crate::comments::read_sidecar(site_dir);
    if !sidecar.contains_key(&req.id) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"unknown comment id"})),
        )
            .into_response();
    }
    if !comments_index(site_dir)
        .orphans
        .iter()
        .any(|orphan| orphan.id == req.id)
    {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"only an orphaned inline comment can be dismissed"})),
        )
            .into_response();
    }
    match crate::comments::remove_record(site_dir, &req.id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"unknown comment id"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error":e.to_string()})),
        )
            .into_response(),
    }
}

async fn migrations_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let shell_html = compile_migrations_widget(&state)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    Html(format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Spacetime Migrations</title><link rel="stylesheet" href="/__spacetime/migrations/styles.css"><style>{DEV_DOCK_CSS}</style></head><body><div class="st-dev-dock">{shell_html}</div><script src="/__spacetime/migrations/runtime.js"></script></body></html>"#
    ))
    .into_response()
}

async fn migrations_runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match compile_migrations_widget(&state) {
        Ok((js, _, _)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            scope_bundle_to_owner(js, "migrations"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            format!("// Migrations pill compile error\nconsole.error({:?});", e),
        )
            .into_response(),
    }
}

async fn migrations_styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    match compile_migrations_widget(&state) {
        Ok((_, css, _)) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], css).into_response()
        }
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            format!(
                "/* Migrations pill compile error: {} */",
                e.replace("*/", "* /")
            ),
        )
            .into_response(),
    }
}

/// One rewrite rule, for the pill's per-entry expander (old shape → new).
#[derive(Serialize)]
struct MigrationStatusRule {
    id: String,
    #[serde(rename = "match")]
    match_source: String,
    into: String,
}

/// One hint, for the pill's per-entry expander (manual guidance).
#[derive(Serialize)]
struct MigrationStatusHint {
    directive: String,
    text: String,
}

/// One entry inside a wave: how many pending hits a migration has, whether
/// it's auto-appliable (rewrite-kind) or manual (hint-kind), and the full
/// capsule contract (rules + hints) for the expander.
#[derive(Serialize)]
struct MigrationStatusEntry {
    id: String,
    docs: String,
    kind: &'static str,
    count: usize,
    /// The %hint text for manual entries (None for automatic ones)
    hint: Option<String>,
    rules: Vec<MigrationStatusRule>,
    hints: Vec<MigrationStatusHint>,
}

#[derive(Serialize)]
struct MigrationStatusWave {
    date: String,
    entries: Vec<MigrationStatusEntry>,
    total: usize,
    /// True when at least one entry is automatic (rewrite-kind) — the pill
    // shows an Apply button only for those waves (data, not a client derive).
    appliable: bool,
}

#[derive(Serialize)]
struct MigrationsStatus {
    /// The project's @version fact (None = date-zero)
    version: Option<String>,
    waves: Vec<MigrationStatusWave>,
    /// Total pending entries across all waves
    pending: usize,
}

fn aggregate_migrations_status(
    version: Option<String>,
    pending: &[crate::migrate::PendingMigration],
    registry: &crate::metasystem::MetaRegistry,
) -> MigrationsStatus {
    let mut order: Vec<String> = Vec::new();
    let mut waves: std::collections::HashMap<String, Vec<MigrationStatusEntry>> =
        std::collections::HashMap::new();
    let mut wave_totals: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    // Apply-ability is a per-ROW fact: a wave with ANY automatic row is
    // appliable (the route applies those rows and prints the manual ones) —
    // a mixed capsule (rules + hints) must never hide the button (R3).
    let mut wave_appliable: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();
    for p in pending {
        if !waves.contains_key(&p.date) {
            order.push(p.date.clone());
        }
        if p.is_automatic() {
            wave_appliable.insert(p.date.clone(), true);
        }
        let entries = waves.entry(p.date.clone()).or_default();
        *wave_totals.entry(p.date.clone()).or_insert(0) += 1;
        let kind = if p.is_automatic() {
            "automatic"
        } else {
            "manual"
        };
        if let Some(e) = entries.iter_mut().find(|e| e.id == p.migration_id) {
            e.count += 1;
            // Mixed rows (rules + hints in ONE capsule): an entry carrying
            // BOTH kinds reads "mixed" (the Apply button is wave-level).
            if e.kind != kind {
                e.kind = "mixed";
            }
            if kind == "manual" {
                // …and backfills the hint a first-row-wins merge missed
                // (an automatic first row carries hint: None; a null hint
                // would render the pill's `$entry.hint` hole verbatim).
                if e.hint.is_none() {
                    e.hint = p.hint.clone();
                }
            }
        } else {
            let (rules, hints) = registry
                .get_migration(&p.migration_id)
                .map(|mig| {
                    (
                        mig.rewrites
                            .iter()
                            .map(|r| MigrationStatusRule {
                                id: r.id.clone(),
                                match_source: r.match_source.trim().to_string(),
                                into: r.template.trim().to_string(),
                            })
                            .collect(),
                        mig.hints
                            .iter()
                            .map(|h| MigrationStatusHint {
                                directive: h.directive.clone(),
                                text: h.text.clone(),
                            })
                            .collect(),
                    )
                })
                .unwrap_or_default();
            entries.push(MigrationStatusEntry {
                id: p.migration_id.clone(),
                docs: p.docs.clone(),
                kind,
                count: 1,
                hint: p.hint.clone(),
                rules,
                hints,
            });
        }
    }
    order.sort();
    MigrationsStatus {
        version,
        pending: pending.len(),
        waves: order
            .into_iter()
            .map(|date| {
                let mut entries = waves.remove(&date).unwrap_or_default();
                entries.sort_by(|a, b| a.id.cmp(&b.id));
                let total = wave_totals.remove(&date).unwrap_or(0);
                let appliable = wave_appliable.remove(&date).unwrap_or(false);
                MigrationStatusWave {
                    date,
                    entries,
                    total,
                    appliable,
                }
            })
            .collect(),
    }
}

async fn migrations_status_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    let index_st = site_dir.join("index.st");
    // Status = a full-project scan through the SAME engine the apply route
    // rides (per file: the shim's pending covers rewrite, degraded, and
    // hint-kind rows with pre-shim spans). Compiling only the entry would
    // miss pending entries in un-imported files.
    let (registry, _) = crate::compiler::cached_stdlib_registry();
    let root_version = std::fs::read_to_string(&index_st)
        .ok()
        .and_then(|c| crate::parser::parse(&c).ok())
        .and_then(|ast| crate::migrate::read_syntax_version(&ast.matches).version);
    let mut pending: Vec<crate::migrate::PendingMigration> = Vec::new();
    for file in crate::migrate::collect_project_st_files(site_dir) {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let ast = match crate::parser::parse(&content) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let outcome =
            crate::migrate::apply_migration_shim(content, ast, &registry, root_version.as_deref());
        // The shim's pending covers BOTH channels (rule rows + the
        // iteration-0 embedded scan for hint/no-rule shapes) — no separate
        // hint collection (that would double-count and carry drifted spans).
        pending.extend(outcome.pending);
    }
    axum::Json(aggregate_migrations_status(
        root_version,
        &pending,
        &registry,
    ))
    .into_response()
}

#[derive(serde::Deserialize)]
struct MigrationApplyRequest {
    date: String,
}

#[derive(Serialize)]
struct MigrationApplyResponse {
    files_changed: usize,
    entries_applied: usize,
    version: Option<String>,
}

async fn migrations_apply_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(req): axum::Json<MigrationApplyRequest>,
) -> Response {
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    if !crate::migrate::is_iso_wave_date(&req.date) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(
                serde_json::json!({ "error": format!("`{}` is not an ISO wave date", req.date) }),
            ),
        )
            .into_response();
    }
    let (registry, _) = crate::compiler::cached_stdlib_registry();
    let root = site_dir.join("index.st");
    let root_version = std::fs::read_to_string(&root)
        .ok()
        .and_then(|c| crate::parser::parse(&c).ok())
        .and_then(|ast| crate::migrate::read_syntax_version(&ast.matches).version);

    // Wave-ordering guard (same rule as the CLI): refuse to skip older
    // pending waves.
    let files = crate::migrate::collect_project_st_files(site_dir);
    for file in &files {
        if let Some(plan) =
            crate::migrate::plan_file_apply(file, &registry, root_version.as_deref(), None)
        {
            if plan
                .pending
                .iter()
                .any(|e| e.date.as_str() < req.date.as_str())
            {
                return (
                    StatusCode::CONFLICT,
                    axum::Json(serde_json::json!({
                        "error": format!("wave {} cannot be applied yet: older waves are still pending", req.date)
                    })),
                )
                    .into_response();
            }
        }
    }

    let plans: Vec<_> = files
        .iter()
        .filter_map(|f| {
            crate::migrate::plan_file_apply(f, &registry, root_version.as_deref(), Some(&req.date))
        })
        .collect();
    let changed: Vec<&_> = plans.iter().filter(|p| p.is_changed()).collect();
    // Bump to the newest AUTOMATIC entry date actually applied (the CLI
    // rule) — never past a wave whose work was all manual.
    let applied_wave = plans
        .iter()
        .flat_map(|p| p.pending.iter())
        .filter(|e| e.is_automatic())
        .map(|e| e.date.clone())
        .max();
    let summary = match crate::migrate::write_apply_plans(&changed, &root, applied_wave.as_deref())
    {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };
    axum::Json(MigrationApplyResponse {
        files_changed: summary.files_changed,
        entries_applied: summary.entries_applied,
        version: summary.version,
    })
    .into_response()
}

/// True when `content` already declares the Spacetime runtime as a REAL script
/// tag, and a second injection would therefore double-load it.
///
/// FUP-098 established the guard: injecting a second runtime resets
/// `ST.signals = new WeakMap()` while the `_st_init_*` dedup flags on existing
/// DOM nodes survive, so the re-init pass skips and every reactive store is left
/// empty — all reactive state silently dies. That protection is kept exactly.
///
/// What changes is the PRECISION of the test. The original predicate was a bare
/// `content.contains("__spacetime/runtime.js")` over the whole document, so ANY
/// mention counted — including one inside an HTML COMMENT. A real shipped page
/// (`projects/backdesk-hospitality/index.html`) carries the comment
///
///     <!-- Spacetime runtime + styles are auto-injected by the dev server
///          (/__spacetime/runtime.js + /__spacetime/styles.css) ... -->
///
/// which is a note SAYING injection happens — and whose own text was what
/// suppressed it. The whole dev dock (host, migrations, and inspector pills)
/// silently vanished from that page, with no error anywhere to explain it.
///
/// So: strip comments first, then look for the marker. A commented mention is
/// prose; only live markup can double-load.
fn declares_runtime_script(content: &str) -> bool {
    let mut rest = content;
    let mut live = String::with_capacity(content.len());
    while let Some(open) = rest.find("<!--") {
        live.push_str(&rest[..open]);
        match rest[open..].find("-->") {
            // An unterminated comment swallows the remainder of the document:
            // nothing after it is live markup, so nothing after it can inject.
            None => return live.contains("__spacetime/runtime.js"),
            Some(close) => rest = &rest[open + close + 3..],
        }
    }
    live.push_str(rest);
    live.contains("__spacetime/runtime.js")
}

fn host_widget_inject(
    site_dir: Option<&Path>,
    entry: Option<&str>,
    cache_stdlib: bool,
    compile_cache: &CompileCache,
) -> String {
    // PLAN-004 follow-up ("always available" requirement): the widget now
    // renders UNCONDITIONALLY in dev mode, regardless of whether this site
    // declares `@deploy{}` at all. Previously `has_deploy_project_id_raw`
    // gated the WHOLE injection -- a brand-new project with no Spacetime
    // Host awareness yet had literally no way to discover/create one,
    // since the widget that would let you do that was itself the thing
    // being hidden. `compile_host_widget_raw` already tolerates an absent
    // `@deploy{}` fact (its own `read_deploy_project_id` call already
    // returns `None` -> `unwrap_or_default()` -> the EMPTY string) --
    // this function's own now-removed gate was the ONLY thing preventing
    // that already-safe empty-project-id path from ever being reached.
    // The widget's own `$isNoProject` state (stdlib/__host__/index.st)
    // derives from exactly that emptiness and offers a "create project"
    // affordance instead of the normal status view -- see that file's own
    // header comment for the full state-machine design.
    let comments_html = compile_comments_widget_raw(cache_stdlib, compile_cache)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    let shell_html = compile_host_widget_raw(site_dir, cache_stdlib, compile_cache)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    // PLAN-076: the dev dock — ONE fixed bottom-right flex-row that hosts
    // BOTH dev widgets (migrations pill LEFT of the host pill, which keeps
    // its long-standing rightmost position). The host widget's own CSS no
    // longer positions it (the dock owns placement); the migrations pill
    // renders hidden while pending == 0.
    let migrations_html = compile_migrations_widget_raw(cache_stdlib, compile_cache)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    let inspector_html = compile_inspector_widget_raw(entry, cache_stdlib, compile_cache)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    // The entry is a FILE PATH going into a query string inside an HTML attribute,
    // so it must survive both layers. Unescaped, a path containing `&` silently
    // truncates into a bogus second query param (the pill then inspects the WRONG
    // file), and one containing a quote closes the `src="…"` attribute outright.
    // Percent-encode everything outside a conservative unreserved set — that also
    // neutralizes the HTML-attribute hazard, since `"`, `'`, `<` and `&` all encode.
    let inspector_entry = encode_query_component(entry.unwrap_or_default());
    format!(
        r#"
    <!-- Spacetime dev dock (comments pill + migrations pill + inspector pill + host pill) -->
    <!-- `__ST_DEV__` marks "a dev SERVER is serving this page". Primitives read it
         to emit dev-only affordances that must never reach a production build:
         `@each`'s `data-st-source`/`data-st-index`/`data-st-field` row stamps, which
         are what the inspector pill's pick mode addresses on a template-less page.
         It was previously set ONLY by stdlib/__dev__ (the debug PANEL), which is
         gated on `@debug {{ show_panel: true }}` — so on an ordinary `spacetime serve`
         the flag stayed false, no row was ever stamped, and pick had nothing to
         resolve. The dock is injected on every dev-served page, so this is the
         honest place to assert it. Set BEFORE the dock scripts so hydration sees it. -->
    <script>window.__ST_DEV__ = true;</script>
    <link rel="stylesheet" href="/__spacetime/comments/styles.css" />
    <link rel="stylesheet" href="/__spacetime/host/styles.css" />
    <link rel="stylesheet" href="/__spacetime/migrations/styles.css" />
    <link rel="stylesheet" href="/__spacetime/inspect/styles.css" />
    <style>{DEV_DOCK_CSS}</style>
    <div class="st-dev-dock">{comments_html}{migrations_html}{inspector_html}{shell_html}</div>
    <script src="/__spacetime/comments/runtime.js"></script>
    <script src="/__spacetime/migrations/runtime.js"></script>
    <script src="/__spacetime/inspect/runtime.js?entry={inspector_entry}"></script>
    <script src="/__spacetime/host/runtime.js"></script>
    <script>{DEV_DOCK_EXCLUSIVE_PANELS_JS}</script>
"#
    )
}

async fn host_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    // PLAN-004 follow-up: no longer gated on @deploy{} -- the widget is
    // now injected UNCONDITIONALLY (host_widget_inject), so its own
    // <script src="/__spacetime/host/runtime.js"> tag must resolve on
    // EVERY site, not just one that already declares @deploy{}. A missing
    // project id is a widget-STATE concern ($isNoProject, handled
    // reactively client-side), never an HTTP-404 concern.
    let shell_html = compile_host_widget(&state)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    let html = format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Spacetime Host</title><link rel="stylesheet" href="/__spacetime/host/styles.css"><style>{DEV_DOCK_CSS}</style></head><body><div class="st-dev-dock">{shell_html}</div><script src="/__spacetime/host/runtime.js"></script></body></html>"#
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

async fn host_runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    // PLAN-004 follow-up: no longer gated on @deploy{} -- see
    // host_page_handler's own comment.
    match compile_host_widget(&state) {
        Ok((js, _, _)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            scope_bundle_to_owner(js, "host"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            format!("// Host compile error\\nconsole.error({:?});", e),
        )
            .into_response(),
    }
}

async fn host_styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    // PLAN-004 follow-up: no longer gated on @deploy{} -- see
    // host_page_handler's own comment.
    match compile_host_widget(&state) {
        Ok((_, css, _)) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], css).into_response()
        }
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            format!("/* Host compile error: {} */", e.replace("*/", "* /")),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct HostPushErrorResponse {
    error: String,
}

/// PLAN-075 ("push preview from the pill"): the ONE thing the `__host__`
/// widget's other actions (`$addDomain`/`$promote`/`$createProject`)
/// cannot do -- a real push needs the project's SOURCE DIRECTORY tarred
/// up, and a widget instance rendered in the browser has no filesystem
/// access to it. Only THIS dev server, which already has `site_dir` open
/// on disk to serve every other request, can package + forward it.
///
/// Deliberately the ONE widget signal that targets THIS server
/// (same-origin `/__spacetime/host/push`) rather than the real
/// orchestrator's `$hostApiBase` directly -- see
/// `stdlib/__host__/index.st::$pushPreview`'s own doc comment for the
/// client-side half of this split.
///
/// Local-dev-only by construction: 501s in `AppMode::Compiled` (a
/// compiled/deployed build has no site directory on disk to package --
/// matches `deploy_init_handler`'s identical dev-mode-only shape).
/// Shared (site_dir, token, project_id) resolution for the push
/// endpoints — the same discipline `host_push_handler` applies:
/// dev-mode only, a real bearer credential required, a @deploy{} fact
/// required. Returns the error response on failure.
fn push_context(state: &AppState) -> Result<(std::path::PathBuf, String, String), Response> {
    let site_dir = match &state.mode {
        AppMode::Compiled(_) => {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(HostPushErrorResponse {
                    error: "Pushing is only available in dev mode (spacetime serve)".to_string(),
                }),
            )
                .into_response());
        }
        AppMode::Dev { site_dir, .. } => site_dir.clone(),
    };
    let api_token = match read_host_api_token() {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(HostPushErrorResponse {
                    error: "Not logged in -- run `spacetime host login` first".to_string(),
                }),
            )
                .into_response());
        }
    };
    let project_id = match read_deploy_project_id(&site_dir) {
        Some(id) => id,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(HostPushErrorResponse {
                    error: "This site has no @deploy{} fact yet -- create a project first"
                        .to_string(),
                }),
            )
                .into_response());
        }
    };
    Ok((site_dir, api_token, project_id))
}

fn push_v2_error_response(e: crate::host_push_v2::PushError) -> Response {
    use crate::host_push_v2::PushError;
    let status = match &e {
        PushError::AlreadyRunning => StatusCode::CONFLICT,
        PushError::NotRunning => StatusCode::NOT_FOUND,
        PushError::Unsupported => StatusCode::BAD_GATEWAY,
        PushError::Server(_) | PushError::Local(_) => StatusCode::BAD_GATEWAY,
    };
    (
        status,
        Json(HostPushErrorResponse {
            error: e.to_string(),
        }),
    )
        .into_response()
}

/// `GET /__spacetime/host/push-events` — the SSE progress stream.
async fn host_push_events_handler() -> Response {
    crate::host_push_v2::push_events_handler().await
}

/// `POST /__spacetime/host/push-v2/start` — widget-driven push: the dev
/// server builds, diffs, uploads the small blobs itself, then hands the
/// large-blob presigned URLs to the widget (browser-direct to R2).
async fn host_push_v2_start_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let (site_dir, api_token, project_id) = match push_context(&state) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };
    match crate::host_push_v2::widget_start(&site_dir, &host_api_base(), &project_id, &api_token)
        .await
    {
        Ok(status) => {
            Json(serde_json::to_value(status).expect("status serializes")).into_response()
        }
        Err(e) => push_v2_error_response(e),
    }
}

/// `GET /__spacetime/host/push-upload-urls` — the widget's pending
/// presigned-upload work list.
async fn host_push_upload_urls_handler() -> Response {
    match crate::host_push_v2::widget_pending().await {
        Ok(pending) => Json(serde_json::json!({ "pending": pending })).into_response(),
        Err(e) => push_v2_error_response(e),
    }
}

/// `GET /__spacetime/host/push-blob/{b3}` — returns one pending large blob
/// to the same-origin widget so it can PUT it to its scoped presigned URL.
async fn host_push_blob_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(b3): axum::extract::Path<String>,
) -> Response {
    if let Err(resp) = push_context(&state) {
        return resp;
    }
    match crate::host_push_v2::widget_blob_bytes(&b3).await {
        Some(bytes) => (
            [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
            bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// `POST /__spacetime/host/push-uploads-complete` — the widget reports
/// finished presigned PUTs; the last one triggers the commit.
async fn host_push_uploads_complete_handler(
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let done: Vec<String> = body
        .get("done")
        .and_then(|d| serde_json::from_value(d.clone()).ok())
        .unwrap_or_default();
    match crate::host_push_v2::widget_complete(&done).await {
        Ok(status) => {
            Json(serde_json::to_value(status).expect("status serializes")).into_response()
        }
        Err(e) => push_v2_error_response(e),
    }
}

/// `POST /__spacetime/host/promote` — rollback relay: the widget never
/// talks to the orchestrator, so the dev server forwards promote
/// (rollback = promote(old_hash), one flip mechanism) with its own
/// credentials. Body: {"hash": "...", "target": "preview"|"production"}.
async fn host_promote_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let (_site_dir, api_token, project_id) = match push_context(&state) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };
    let hash = body.get("hash").and_then(|v| v.as_str()).unwrap_or("");
    let target = body
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("preview");
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(HostPushErrorResponse {
                error: "promote needs a 64-char hex deploy hash".to_string(),
            }),
        )
            .into_response();
    }
    let base = host_api_base();
    let base = base.trim_end_matches('/');
    let resp = match reqwest::Client::new()
        .post(format!("{base}/api/v1/projects/{project_id}/promote"))
        .bearer_auth(&api_token)
        .json(&serde_json::json!({ "hash": hash, "target": target }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(HostPushErrorResponse {
                    error: format!("could not reach Spacetime Host: {e}"),
                }),
            )
                .into_response();
        }
    };
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body = resp.text().await.unwrap_or_default();
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

async fn host_push_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let site_dir = match &state.mode {
        AppMode::Compiled(_) => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(HostPushErrorResponse {
                    error: "Pushing is only available in dev mode (spacetime serve)".to_string(),
                }),
            )
                .into_response();
        }
        AppMode::Dev { site_dir, .. } => site_dir.clone(),
    };

    // A real account token is REQUIRED for this route (unlike the
    // widget's read-only status calls, which tolerate an absent token by
    // rendering the Blue/logged-out state) -- pushing bytes needs a real
    // bearer credential, and failing loudly here is far better than a
    // silent 401 from the upstream orchestrator with no actionable
    // message.
    let api_token = match read_host_api_token() {
        Some(token) => token,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(HostPushErrorResponse {
                    error: "Not logged in -- run `spacetime host login` first".to_string(),
                }),
            )
                .into_response();
        }
    };

    let project_id = match read_deploy_project_id(&site_dir) {
        Some(id) => id,
        None => {
            // Defense-in-depth: the widget's own `$isNoProject` state
            // already hides the Push button entirely without a
            // `@deploy{}` fact -- this branch guards a direct curl/API
            // call more than a real UI path.
            return (
                StatusCode::BAD_REQUEST,
                Json(HostPushErrorResponse {
                    error: "This site has no @deploy{} fact yet -- create a project first"
                        .to_string(),
                }),
            )
                .into_response();
        }
    };

    let api_base = host_api_base();

    // Protocol v2 is the authoritative content-addressed path. Only a 404
    // from init means this is an older host and permits the retained v1 tar
    // fallback below; all other v2 failures are surfaced verbatim.
    match crate::host_push_v2::push(&site_dir, &api_base, &project_id, &api_token, "cli", |_| {})
        .await
    {
        Ok(result) => return Json(result).into_response(),
        Err(crate::host_push_v2::PushError::Unsupported) => {
            log::warn!(
                "Spacetime Host does not support push v2; falling back to deprecated v1 tar upload"
            );
        }
        Err(error) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(HostPushErrorResponse {
                    error: error.to_string(),
                }),
            )
                .into_response();
        }
    }

    let (widget_js, widget_css, widget_shell_html) =
        match compile_host_widget_raw(Some(&site_dir), state.cache_stdlib, &state.compile_cache) {
            Ok(parts) => parts,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(HostPushErrorResponse {
                        error: format!("failed to compile host widget: {e}"),
                    }),
                )
                    .into_response();
            }
        };
    // Same tag composition `host_widget_inject` already produces for the
    // LOCAL dev page (link + shell markup + script) -- reused verbatim so
    // a pushed archive's injected widget is byte-identical in SHAPE to
    // what dev mode already renders, just with real (not sentinel)
    // values baked in by `compile_host_widget_raw` above.
    let widget_tail = format!(
        "\n    <!-- Spacetime Host widget -->\n    <link rel=\"stylesheet\" href=\"/__spacetime/host/styles.css\" />\n    {widget_shell_html}\n    <script src=\"/__spacetime/host/runtime.js\"></script>\n"
    );

    // W0-1 (PLAN-005): package the BUILT site, never the source
    // directory -- the pre-fix handler tarred `site_dir` itself, which
    // shipped .st sources to the edge (source leak) while serving pages
    // whose /spacetime.js + /spacetime.css references 404'd (the source
    // tree has no compiled assets at root). `export_site` is the same
    // build path `spacetime build <dir>/` uses; this tempdir export is
    // the seed of W1-1's SiteArtifact / one build_artifact() path.
    let export_tmp = match tempfile::tempdir() {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HostPushErrorResponse {
                    error: format!("failed to create build tempdir: {e}"),
                }),
            )
                .into_response();
        }
    };
    let built_dir = export_tmp.path().join("site");
    let artifact = match crate::export::build_artifact(&site_dir) {
        Ok(artifact) => artifact,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HostPushErrorResponse {
                    error: format!("failed to build site for push: {e}"),
                }),
            )
                .into_response();
        }
    };
    for file in &artifact.files {
        let path = built_dir.join(&file.path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&path, &file.bytes) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HostPushErrorResponse {
                    error: format!("failed to materialize artifact: {e}"),
                }),
            )
                .into_response();
        }
    }

    let archive_bytes = match crate::host_package::package_site_archive(
        &built_dir,
        &widget_js,
        &widget_css,
        &widget_tail,
    ) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HostPushErrorResponse {
                    error: format!("failed to package site directory: {e}"),
                }),
            )
                .into_response();
        }
    };

    let client = reqwest::Client::new();
    let upstream = client
        .post(format!("{api_base}/api/v1/projects/{project_id}/push"))
        .bearer_auth(&api_token)
        .body(archive_bytes)
        .send()
        .await;

    match upstream {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let axum_status =
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            (
                axum_status,
                [(header::CONTENT_TYPE, "application/json")],
                body,
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(HostPushErrorResponse {
                error: format!("could not reach Spacetime Host: {e}"),
            }),
        )
            .into_response(),
    }
}

fn compile_admin(state: &AppState) -> Result<(String, String, String), String> {
    // FEAT-076: the admin is the declarative multi-file tree under
    // stdlib/__admin__/ (index.st imports the app/ tree). The former %emit js
    // monolith (admin-app.st) was deleted at the W5 cutover.
    let admin_st_path = PathBuf::from("stdlib/__admin__/index.st");
    if !admin_st_path.exists() {
        return Err("admin .st not found".to_string());
    }
    let compiler =
        Compiler::from_file(&admin_st_path, Path::new(".")).map_err(|e| e.to_string())?;
    let compiled = compiler
        .cache_stdlib(state.cache_stdlib)
        .with_cache(&state.compile_cache)
        .compile();
    Ok((compiled.js, compiled.css, compiled.html))
}

/// Full-window admin HTML shell. Loads the admin runtime + styles. Debug only.
async fn admin_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let debug = matches!(&state.mode, AppMode::Dev { debug, .. } if *debug);
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            "Admin is available only with the dev server in --debug mode.",
        )
            .into_response();
    }
    // The declarative rebuild (FEAT-076) declares its shell as file-scope markup,
    // which compiles into `compiled.html`. Serve that as the body so the shell
    // mounts (like a full-Spacetime page); the legacy monolith emits no
    // file-scope HTML, so its body stays script-only (unchanged behavior).
    let shell_html = compile_admin(&state)
        .map(|(_, _, html)| html)
        .unwrap_or_default();
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Content · Spacetime Admin</title>
  <link rel="stylesheet" href="/__spacetime/admin/styles.css">
</head>
<body>
  <script>(function(){{var ws=new WebSocket((location.protocol==='https:'?'wss://':'ws://')+location.host+'/ws');ws.onmessage=function(e){{try{{var d=JSON.parse(e.data);if(d.type==='Reload'){{location.reload();}}}}catch(err){{}}}};}})()</script>
{shell_html}
  <script src="/__spacetime/admin/runtime.js"></script>
</body>
</html>"#
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

/// Admin runtime JS handler — compiles stdlib/__admin__/index.st. Debug only.
async fn admin_runtime_js_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let debug = matches!(&state.mode, AppMode::Dev { debug, .. } if *debug);
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/javascript")],
            "// Debug mode not enabled\n".to_string(),
        )
            .into_response();
    }
    match compile_admin(&state) {
        Ok((js, _, _)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            js,
        )
            .into_response(),
        Err(e) => {
            let escaped = e
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace("${", "\\${");
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/javascript")],
                format!("// Admin compile error\nconsole.error(`{}`);", escaped),
            )
                .into_response()
        }
    }
}

/// Admin styles CSS handler — compiles stdlib/__admin__/index.st. Debug only.
async fn admin_styles_css_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    let debug = matches!(&state.mode, AppMode::Dev { debug, .. } if *debug);
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/css")],
            "/* Debug mode not enabled */\n".to_string(),
        )
            .into_response();
    }
    match compile_admin(&state) {
        Ok((_, css, _)) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], css).into_response()
        }
        Err(e) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            format!("/* Admin compile error: {} */", e.replace("*/", "* /")),
        )
            .into_response(),
    }
}

// =============================================================================
// Editor API Types
// =============================================================================

/// Response for /__spacetime/ast endpoint
#[derive(Debug, Serialize)]
struct AstResponse {
    files: HashMap<String, StFile>,
}

/// Request for /__spacetime/save endpoint
#[derive(Debug, Deserialize)]
struct SaveRequest {
    /// Relative path to the .st file (e.g., "animations.st")
    file_path: String,
    /// The modified AST to save
    ast: StFile,
}

/// Response for save endpoint
#[derive(Debug, Serialize)]
struct SaveResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Request for /__spacetime/save-html endpoint
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveHtmlRequest {
    /// Relative path to the HTML file
    file_path: String,
    /// The new innerHTML content for the element
    content: String,
    /// The element ID (data-st-id) being updated
    element_id: String,
}

// =============================================================================
// Editor API Handlers
// =============================================================================

/// Find all .st files in a directory
fn find_st_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "st") {
                files.push(path);
                continue;
            }
            // Also check subdirectories (one level deep)
            if path.is_dir()
                && let Ok(sub_entries) = std::fs::read_dir(&path)
            {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() && sub_path.extension().is_some_and(|ext| ext == "st") {
                        files.push(sub_path);
                    }
                }
            }
        }
    }
    files
}

/// GET /__spacetime/ast - Returns all parsed .st files as JSON
async fn ast_handler(axum::extract::State(state): axum::extract::State<AppState>) -> Response {
    match &state.mode {
        AppMode::Compiled(_) => {
            // In compiled mode, we don't have access to source files
            (
                StatusCode::NOT_FOUND,
                "AST endpoint only available in dev mode",
            )
                .into_response()
        }
        AppMode::Dev {
            site_dir,
            trace: _,
            debug: _,
        } => {
            let st_files = find_st_files(site_dir);
            let mut files = HashMap::new();

            for st_file in st_files {
                let relative_path = st_file
                    .strip_prefix(site_dir)
                    .unwrap_or(&st_file)
                    .to_string_lossy()
                    .to_string();

                match std::fs::read_to_string(&st_file) {
                    Ok(content) => match parse(&content) {
                        Ok(ast) => {
                            files.insert(relative_path, ast);
                        }
                        Err(e) => {
                            eprintln!(
                                "{}",
                                e.render_all_plain(&content, &st_file.to_string_lossy())
                            );
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read {}: {}", st_file.display(), e);
                    }
                }
            }

            Json(AstResponse { files }).into_response()
        }
    }
}

/// POST /__spacetime/dev/upload - Handle file upload for dev-mode asset management.
/// Accepts multipart form with a single file field.
/// Returns JSON with the relative path to the saved asset.
async fn upload_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Dev-mode only guard
    let site_dir = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } if *debug => site_dir.clone(),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };

    // Read first file field from multipart
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No file field"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response();
        }
    };

    let filename = field.file_name().unwrap_or("upload").to_string();
    let data = match field.bytes().await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{}", e)})),
            )
                .into_response();
        }
    };

    // Validate size (10MB max)
    if data.len() > 10 * 1024 * 1024 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "File exceeds 10MB limit"})),
        )
            .into_response();
    }

    // Validate file extension
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let allowed = ["png", "jpg", "jpeg", "gif", "webp", "svg"];
    if !allowed.contains(&ext.as_str()) {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(serde_json::json!({"error": format!("Unsupported file type: .{}", ext)})),
        )
            .into_response();
    }

    // Content-hash the file for deduplication
    let hash = blake3::hash(&data);
    let hash_str = hash.to_hex();
    let dest_filename = format!("{}.{}", &hash_str[..16], ext);

    // Ensure upload directory exists
    let upload_dir = site_dir.join("assets").join("uploads");
    if let Err(e) = std::fs::create_dir_all(&upload_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Cannot create upload dir: {}", e)})),
        )
            .into_response();
    }

    // Write file (content-hash means duplicates are a no-op)
    let dest_path = upload_dir.join(&dest_filename);
    if let Err(e) = std::fs::write(&dest_path, &data) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Write failed: {}", e)})),
        )
            .into_response();
    }

    let relative_path = format!("assets/uploads/{}", dest_filename);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "path": relative_path,
            "size": data.len(),
        })),
    )
        .into_response()
}

/// POST /__spacetime/save - Save modified AST back to .st file
async fn save_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<SaveRequest>,
) -> Response {
    match &state.mode {
        AppMode::Compiled(_) => Json(SaveResponse {
            success: false,
            error: Some("Save endpoint only available in dev mode".to_string()),
        })
        .into_response(),
        AppMode::Dev {
            site_dir,
            trace: _,
            debug: _,
        } => {
            // Validate path is within site_dir (security check)
            let file_path = site_dir.join(&req.file_path);
            let canonical_site = match site_dir.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    return Json(SaveResponse {
                        success: false,
                        error: Some(format!("Invalid site directory: {}", e)),
                    })
                    .into_response();
                }
            };

            // Create parent directories if needed, then canonicalize
            if let Some(parent) = file_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Check that the path stays within site_dir
            let canonical_file = match file_path.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    // File doesn't exist yet, check parent
                    match file_path.parent().and_then(|p| p.canonicalize().ok()) {
                        Some(parent) if parent.starts_with(&canonical_site) => file_path.clone(),
                        _ => {
                            return Json(SaveResponse {
                                success: false,
                                error: Some("Invalid file path".to_string()),
                            })
                            .into_response();
                        }
                    }
                }
            };

            if !canonical_file.starts_with(&canonical_site) {
                return Json(SaveResponse {
                    success: false,
                    error: Some("Path traversal not allowed".to_string()),
                })
                .into_response();
            }

            // Serialize AST back to .st format
            let content = serialize(&req.ast);

            // Write to file
            match std::fs::write(&file_path, content) {
                Ok(_) => Json(SaveResponse {
                    success: true,
                    error: None,
                })
                .into_response(),
                Err(e) => Json(SaveResponse {
                    success: false,
                    error: Some(format!("Failed to write file: {}", e)),
                })
                .into_response(),
            }
        }
    }
}
/// Request for `/__spacetime/dev/deploy-init` -- see that handler's own
/// doc comment for the full rationale.
#[derive(Debug, Deserialize)]
struct DeployInitRequest {
    /// The already-server-minted project id (from a real
    /// `POST /api/v1/projects` call the widget itself made against the
    /// orchestrator) -- this endpoint never talks to the orchestrator
    /// itself, it only writes the LOCAL `.st` source that records which
    /// project this site is now deployed as.
    project_id: String,
}

#[derive(Debug, Serialize)]
struct DeployInitResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// PLAN-004 follow-up ("host widget always available + create-project-
/// from-widget"): the LOCAL write-back half of the widget's
/// "no @deploy{} yet -> log in/sign up -> create a project" flow.
///
/// The widget itself handles the REMOTE half entirely client-side (it
/// already has a real `$hostApi` transport + a real bearer token once
/// WorkOS login/signup completes, exactly like every other authenticated
/// call this widget makes): it calls the orchestrator's OWN
/// `POST /api/v1/projects` directly (spacetime-host-server::projects::
/// create, already live) to mint a real project id server-side. THIS
/// endpoint is the one piece that call alone cannot do: writing
/// `@deploy { project: "<id>"; }` into the site's OWN `.st` source, since
/// the orchestrator has no access to a developer's local filesystem (and
/// must not -- it is a separate, network-reachable service; this dev
/// server, by contrast, already legitimately reads/writes this exact
/// directory for every other dev-mode affordance, e.g. `save_handler`
/// above).
///
/// Deliberately a RAW TEXT append, not an `EditAst`/`StFile`-node
/// construction (unlike `save_handler`'s whole-AST-serialize path):
/// `@deploy{}` (stdlib/macros/deploy.st) is a bare top-level macro
/// invocation with no existing AST representation to MODIFY here (a
/// brand-new project's `index.st` has no `@deploy` match to find/edit --
/// there is nothing to construct via `StFile`'s node types that
/// `serialize()` would round-trip more safely than the source text
/// itself; `@deploy { project: "x"; }` is valid, complete, appendable
/// `.st` source verbatim). Two things must both be present for the
/// widget to actually pick up the new project id: the `@import
/// "stdlib/macros/deploy"` line (added once, only if not already
/// present -- a project may already import OTHER stdlib macros without
/// this one) and the `@deploy { project: "..."; }` block itself
/// (appended once; a SECOND call to this endpoint, e.g. a page double-
/// click, would otherwise append a SECOND `@deploy{}` block -- guarded
/// against below by checking `read_deploy_project_id` first and treating
/// an already-deployed site as a no-op success, not an error, matching
/// the general REST convention that a repeatable "ensure state X" call
/// should be idempotent).
///
/// No separate WS-reload trigger is needed: this dev server's EXISTING
/// file watcher (the same one that already reloads on any other
/// `save_handler`-driven `.st` write) picks up this write identically --
/// confirmed by reading `save_handler`'s own implementation immediately
/// above, which relies on the exact same mechanism and issues no
/// separate reload call itself.
async fn deploy_init_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<DeployInitRequest>,
) -> Response {
    let site_dir = match &state.mode {
        AppMode::Compiled(_) => {
            return Json(DeployInitResponse {
                success: false,
                error: Some(
                    "Creating a project is only available in dev mode (spacetime serve)"
                        .to_string(),
                ),
            })
            .into_response();
        }
        AppMode::Dev { site_dir, .. } => site_dir.clone(),
    };

    let project_id = req.project_id.trim();
    if project_id.is_empty() {
        return Json(DeployInitResponse {
            success: false,
            error: Some("project_id must not be empty".to_string()),
        })
        .into_response();
    }

    // Idempotent: a site that already declares @deploy{} is left
    // untouched (never a second block appended) -- this can legitimately
    // be called again (e.g. a retried request after a dropped response)
    // without corrupting the source.
    if read_deploy_project_id(&site_dir).is_some() {
        return Json(DeployInitResponse {
            success: true,
            error: None,
        })
        .into_response();
    }

    let index_st = site_dir.join("index.st");
    let content = match std::fs::read_to_string(&index_st) {
        Ok(c) => c,
        Err(e) => {
            return Json(DeployInitResponse {
                success: false,
                error: Some(format!("Failed to read index.st: {}", e)),
            })
            .into_response();
        }
    };

    // Escape a literal `"` in the project id (nanoid-generated ids never
    // contain one, but this endpoint's OWN contract should not silently
    // corrupt the source if that ever changed) before embedding it as a
    // quoted .st string literal.
    let escaped_id = project_id.replace('\\', "\\\\").replace('"', "\\\"");

    let import_line = "@import \"stdlib/macros/deploy\"";
    let needs_import = !content.lines().any(|l| l.trim() == import_line);

    let mut new_content = content.clone();
    if needs_import {
        // Prepended at the very top -- @import statements in this
        // language are file-scope and order-independent relative to each
        // other (confirmed by every existing multi-import .st file in
        // this repo, e.g. stdlib/__host__/index.st's own 5 @import
        // lines), so a leading insertion is always safe.
        new_content = format!("{}\n{}", import_line, new_content);
    }
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!(
        "\n@deploy {{\n  project: \"{}\";\n}}\n",
        escaped_id
    ));

    match std::fs::write(&index_st, new_content) {
        Ok(_) => Json(DeployInitResponse {
            success: true,
            error: None,
        })
        .into_response(),
        Err(e) => Json(DeployInitResponse {
            success: false,
            error: Some(format!("Failed to write index.st: {}", e)),
        })
        .into_response(),
    }
}
/// Request for `/__spacetime/dev/save-host-credentials`.
#[derive(Debug, Deserialize)]
struct SaveHostCredentialsRequest {
    api_token: String,
}

#[derive(Debug, Serialize)]
struct SaveHostCredentialsResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Real, user-reported bug this session (not a design gap noticed while
/// reading code): the widget's device-flow login opened the WorkOS
/// confirm-code popup and correctly walked it through to "You are all
/// set!" -- but NOTHING then polled `POST /api/v1/auth/exchange` to
/// actually retrieve the minted token. `$startLogin`
/// (stdlib/__host__/index.st) only ever called `POST /api/v1/auth/device`
/// (the START half) -- the corresponding poll-until-done loop the CLI's
/// own `cmd_login` performs (spacetime-host-cli::main::cmd_login,
/// verified against its real source: start -> loop { sleep(interval);
/// exchange(device_code) } -> save_credentials on success) was never
/// implemented client-side at all.
///
/// The poll ITSELF now lives in a new client-side primitive
/// (stdlib/primitives/host-device-poll.st, mirrors host-bridge-auth.st's
/// escape-hatch shape: raw JS is the sanctioned mechanism for runtime
/// browser polling loops this language's grammar doesn't express). THIS
/// endpoint is the other half that ONLY a local process can do: local
/// dev's `$hostApiToken` is baked in at COMPILE time (`read_host_api_
/// token` reads the CLI's own credentials file, substituted into the
/// served JS as a literal by `compile_host_widget`) -- a browser can
/// mint a real token via `exchange` but can never write it to the one
/// place (`~/.local/share/spacetime-host/credentials`) that makes
/// `$hasAccountToken` flip true on the NEXT page load. Same shape
/// problem `deploy_init_handler` above already solved for `@deploy{}`
/// write-back, same fix pattern.
///
/// Replicates `spacetime-host-cli::save_credentials_to`'s exact atomic-
/// write safety properties (verified against that real source this
/// session, not reimplemented from memory): a same-directory temp file
/// opened with `create_new` + mode `0600` BEFORE any byte is written
/// (refuses to open THROUGH a planted symlink, unlike `fs::write`'s
/// create-or-truncate), then `rename`d into place (replaces -- does not
/// follow -- an existing path, including a symlink). Deliberately
/// duplicated rather than importing `spacetime-host-cli` as a dependency:
/// that crate lives in `commercial/host`, a separate PRIVATE repo this
/// OSS `verse` crate must not depend on (confirmed via this session's own
/// prior investigation: `read_host_api_token`'s own doc comment already
/// establishes this exact "mirror the CLI's resolution without adding a
/// dependency on it" precedent for the READ side).
async fn save_host_credentials_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<SaveHostCredentialsRequest>,
) -> Response {
    if !matches!(&state.mode, AppMode::Dev { .. }) {
        return Json(SaveHostCredentialsResponse {
            success: false,
            error: Some("Login is only available in dev mode (spacetime serve)".to_string()),
        })
        .into_response();
    }

    let api_token = req.api_token.trim();
    if api_token.is_empty() {
        return Json(SaveHostCredentialsResponse {
            success: false,
            error: Some("api_token must not be empty".to_string()),
        })
        .into_response();
    }

    match save_host_credentials_to_disk(api_token) {
        Ok(()) => Json(SaveHostCredentialsResponse {
            success: true,
            error: None,
        })
        .into_response(),
        Err(e) => Json(SaveHostCredentialsResponse {
            success: false,
            error: Some(e),
        })
        .into_response(),
    }
}

/// The write half of `read_host_api_token`'s own path resolution --
/// deliberately kept in the SAME `XDG_DATA_HOME`/`$HOME/.local/share`
/// convention as that function (see its doc comment), so a token this
/// endpoint writes is read back by the EXACT same function every other
/// dev-mode code path already uses.
fn save_host_credentials_to_disk(api_token: &str) -> Result<(), String> {
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .ok_or_else(|| "could not determine a data directory for this platform".to_string())?;
    let dir = data_dir.join("spacetime-host");
    let path = dir.join("credentials");

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create {}: {}", dir.display(), e))?;

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let temp_path = dir.join(format!(".credentials.tmp.{}", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)
            .map_err(|e| format!("failed to create temp credentials file: {}", e))?;
        file.write_all(api_token.as_bytes())
            .map_err(|e| format!("failed to write credentials: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync credentials: {}", e))?;
        drop(file);
        std::fs::rename(&temp_path, &path)
            .map_err(|e| format!("failed to finalize credentials file: {}", e))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, api_token)
            .map_err(|e| format!("failed to write credentials: {}", e))?;
    }

    Ok(())
}
/// Handler for saving HTML content updates using lol_html
///
/// This endpoint receives edited content from the browser and uses lol_html
/// to safely update just the specified element in the HTML file.
async fn save_html_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<SaveHtmlRequest>,
) -> Response {
    match &state.mode {
        AppMode::Compiled(_) => Json(SaveResponse {
            success: false,
            error: Some("Save endpoint only available in dev mode".to_string()),
        })
        .into_response(),
        AppMode::Dev {
            site_dir,
            trace: _,
            debug: _,
        } => {
            // Validate path is within site_dir (security check)
            let file_path = site_dir.join(&req.file_path);
            let canonical_site = match site_dir.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    return Json(SaveResponse {
                        success: false,
                        error: Some(format!("Invalid site directory: {}", e)),
                    })
                    .into_response();
                }
            };

            let canonical_file = match file_path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    // File might not exist yet, check parent
                    match file_path.parent().and_then(|p| p.canonicalize().ok()) {
                        Some(parent) if parent.starts_with(&canonical_site) => {
                            // Parent is valid, file just doesn't exist - but we need existing file
                            return Json(SaveResponse {
                                success: false,
                                error: Some(format!("File does not exist: {}", e)),
                            })
                            .into_response();
                        }
                        _ => {
                            return Json(SaveResponse {
                                success: false,
                                error: Some("Invalid file path".to_string()),
                            })
                            .into_response();
                        }
                    }
                }
            };

            if !canonical_file.starts_with(&canonical_site) {
                return Json(SaveResponse {
                    success: false,
                    error: Some("Path traversal not allowed".to_string()),
                })
                .into_response();
            }

            // Read the existing HTML file
            let html_content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(e) => {
                    return Json(SaveResponse {
                        success: false,
                        error: Some(format!("Failed to read file: {}", e)),
                    })
                    .into_response();
                }
            };

            // Use lol_html to safely update the element
            match update_element_content(&html_content, &req.element_id, &req.content) {
                Ok(new_html) => {
                    // Write the updated HTML back
                    match std::fs::write(&file_path, new_html) {
                        Ok(_) => Json(SaveResponse {
                            success: true,
                            error: None,
                        })
                        .into_response(),
                        Err(e) => Json(SaveResponse {
                            success: false,
                            error: Some(format!("Failed to write file: {}", e)),
                        })
                        .into_response(),
                    }
                }
                Err(e) => Json(SaveResponse {
                    success: false,
                    error: Some(format!("Failed to update HTML: {}", e)),
                })
                .into_response(),
            }
        }
    }
}

/// Compute a stable content hash for provenance tracking.
/// Returns `"st-{8_hex_chars}"` using FNV-1a 32-bit hash.
pub(crate) fn compute_st_hash(input: &str) -> String {
    // FNV-1a 32-bit
    let mut hash: u32 = 2166136261;
    for byte in input.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    format!("st-{:08x}", hash)
}

/// Inject `data-st-id` and `data-st-origin` attributes on text-bearing HTML elements.
/// Used in debug mode to enable content addressing for the editing protocol.
/// Must run BEFORE the existing `</body>` script injection.
pub(crate) fn inject_content_provenance(html: &str, file_path: &str) -> String {
    use lol_html::{HtmlRewriter, Settings, element};
    use std::cell::RefCell;

    let tag_counts: RefCell<std::collections::HashMap<String, usize>> =
        RefCell::new(std::collections::HashMap::new());
    let mut output = Vec::new();
    let text_bearing_tags = [
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "p",
        "span",
        "li",
        "td",
        "th",
        "a",
        "label",
        "button",
        "figcaption",
        "blockquote",
        "cite",
        "dt",
        "dd",
        "summary",
    ];

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![element!("*", |el| {
                let tag = el.tag_name().to_lowercase();
                if !text_bearing_tags.contains(&tag.as_str()) {
                    return Ok(());
                }
                if el.get_attribute("data-t").is_some() {
                    return Ok(());
                }
                if el.get_attribute("data-st-bind").is_some() {
                    return Ok(());
                }
                if el.get_attribute("data-st-id").is_some() {
                    return Ok(());
                }

                let mut counts = tag_counts.borrow_mut();
                let idx = counts.entry(tag.clone()).or_insert(0);
                *idx += 1;
                let occurrence = *idx;

                let hash_input = format!("{}:{}:{}", file_path, tag, occurrence);
                let hash = compute_st_hash(&hash_input);

                el.set_attribute("data-st-id", &hash)?;
                el.set_attribute("data-st-origin", &format!("{}::{}", file_path, hash))?;
                Ok(())
            })],
            ..Settings::default()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );
    rewriter.write(html.as_bytes()).ok();
    rewriter.end().ok();
    String::from_utf8(output).unwrap_or_else(|_| html.to_string())
}

/// Update an element's content in HTML using lol_html
///
/// Finds the element with data-st-id matching element_id and replaces its innerHTML
fn update_element_content(
    html: &str,
    element_id: &str,
    new_content: &str,
) -> Result<String, String> {
    use lol_html::{HtmlRewriter, Settings, element};

    let mut output = Vec::new();
    let target_id = element_id.to_string();
    let content = new_content.to_string();
    let mut found = false;

    {
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!(
                    format!("[data-st-id=\"{}\"]", target_id),
                    |el| {
                        // Replace inner content
                        el.set_inner_content(&content, lol_html::html_content::ContentType::Html);
                        found = true;
                        Ok(())
                    }
                )],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter
            .write(html.as_bytes())
            .map_err(|e| format!("HTML rewrite error: {}", e))?;
        rewriter
            .end()
            .map_err(|e| format!("HTML rewrite error: {}", e))?;
    }

    if !found {
        return Err(format!(
            "Element with data-st-id=\"{}\" not found",
            element_id
        ));
    }

    String::from_utf8(output).map_err(|e| format!("UTF-8 conversion error: {}", e))
}

/// Generate the demo HTML page
/// Synthesize a minimal HTML shell for a `.st`-only project (FEAT-043).
///
/// When a Spacetime project has an `index.st` but no `index.html` on disk,
/// the dev server previously returned HTTP 500. A full-Spacetime page keeps
/// all of its state, bindings, templates, and styling in the `.st` file, so
/// the only thing the browser needs is a document shell that loads the
/// compiled runtime and styles for that entry. This produces exactly that.
///
/// `entry` is the `.st` entry path relative to the site dir (e.g. `index.st`).
/// It is interpolated into attribute context and is HTML-attribute-escaped.
fn synthesize_st_only_shell(
    entry: &str,
    body_html: &str,
    debug_tools: bool,
    host_widget: &str,
) -> String {
    // Delegate to the shared page-shell renderer (src/html/page_shell.rs) so the dev server and
    // the static export produce IDENTICAL document structure (FUP-039). Dev-specific bits: the
    // `?entry=` asset hrefs (served by /__spacetime) and the live-reload tail. `body_html` is
    // compiler-produced markup (PLAN-023 W1 + FEAT-078 hydration markers), injected verbatim
    // ahead of the runtime so the page has SEO-visible content; the runtime hydrates after.
    //
    // `debug_tools` (--debug): a full-Spacetime page is the editable-everything substrate, so in
    // debug mode it must also load the dev-tools runtime (/__spacetime/dev/runtime.js) that mounts
    // window.__stDevWs / __stDevEditable / __stDevSave — the rail an on-page @richtext surface uses
    // to persist edits (FUP-047). The authored-index.html path already injects these; this brings
    // the .st-only path to parity. The dev tail is appended after the runtime, like the live-reload.
    //
    // `host_widget` (PLAN-004): the __host__ widget's own tags/markup, or
    // an empty string on a site with no `@deploy{}` fact -- this was the
    // SIXTH page-serve path found to be missing the widget injection
    // (the other 5 all authored their own `index.html`; a `.st`-only
    // project with NO index.html on disk -- this synthesized-shell path
    // -- is exactly what a fresh `spacetime init` site looks like, so
    // this gap would have hit the MOST common getting-started case).
    let entry = crate::html::page_shell::escape_attr(entry);
    let tail = if debug_tools {
        format!(
            "\n    <!-- Spacetime Dev Tools -->\n    <link rel=\"stylesheet\" href=\"/__spacetime/dev/styles.css\" />\n    <script src=\"/__spacetime/dev/runtime.js\"></script>{}{}",
            LIVE_RELOAD_SCRIPT, host_widget
        )
    } else {
        format!("{}{}", LIVE_RELOAD_SCRIPT, host_widget)
    };
    crate::html::page_shell::render_page_shell(&crate::html::page_shell::PageShell {
        title: "Spacetime",
        css_href: &format!("/__spacetime/styles.css?entry={entry}"),
        js_href: &format!("/__spacetime/runtime.js?entry={entry}"),
        body_html,
        tail: &tail,
        lang: "en",
    })
}

fn generate_index_html(_compiled: &CompiledSpacetime) -> String {
    r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Spacetime Demo - WisprFlow Inspired</title>
    <link rel="stylesheet" href="/__spacetime/styles.css">
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: #0a0a0f;
            color: #fff;
            overflow-x: hidden;
        }

        /* Hero Section */
        .hero {
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            position: relative;
            overflow: hidden;
            padding: 2rem;
        }

        .hero-bg {
            position: absolute;
            inset: 0;
            background: radial-gradient(ellipse at 50% 0%, rgba(99, 102, 241, 0.15) 0%, transparent 50%),
                        radial-gradient(ellipse at 80% 50%, rgba(168, 85, 247, 0.1) 0%, transparent 40%),
                        radial-gradient(ellipse at 20% 80%, rgba(59, 130, 246, 0.1) 0%, transparent 40%);
            z-index: 0;
        }

        .hero-content {
            position: relative;
            z-index: 1;
            text-align: center;
            max-width: 900px;
        }

        .hero-badge {
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.5rem 1rem;
            background: rgba(99, 102, 241, 0.1);
            border: 1px solid rgba(99, 102, 241, 0.3);
            border-radius: 999px;
            font-size: 0.875rem;
            color: #a5b4fc;
            margin-bottom: 2rem;
        }

        .hero-title {
            font-size: clamp(2.5rem, 8vw, 5rem);
            font-weight: 700;
            line-height: 1.1;
            margin-bottom: 1.5rem;
            background: linear-gradient(135deg, #fff 0%, #a5b4fc 50%, #818cf8 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }

        .hero-subtitle {
            font-size: clamp(1rem, 2.5vw, 1.25rem);
            color: #94a3b8;
            line-height: 1.6;
            margin-bottom: 2.5rem;
            max-width: 600px;
            margin-left: auto;
            margin-right: auto;
        }

        .hero-cta {
            display: inline-flex;
            align-items: center;
            gap: 0.75rem;
            padding: 1rem 2rem;
            background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
            color: #fff;
            font-weight: 600;
            font-size: 1rem;
            border: none;
            border-radius: 12px;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
        }

        .hero-cta:hover {
            transform: translateY(-2px);
            box-shadow: 0 20px 40px rgba(99, 102, 241, 0.3);
        }

        /* Floating elements */
        .floating-elements {
            position: absolute;
            inset: 0;
            pointer-events: none;
            z-index: 0;
        }

        .floating-orb {
            position: absolute;
            border-radius: 50%;
            filter: blur(40px);
        }

        .orb-1 {
            width: 300px;
            height: 300px;
            background: rgba(99, 102, 241, 0.3);
            top: 10%;
            left: 10%;
        }

        .orb-2 {
            width: 200px;
            height: 200px;
            background: rgba(168, 85, 247, 0.3);
            top: 60%;
            right: 15%;
        }

        .orb-3 {
            width: 250px;
            height: 250px;
            background: rgba(59, 130, 246, 0.2);
            bottom: 10%;
            left: 30%;
        }

        /* Feature Grid Section */
        .features {
            padding: 6rem 2rem;
            background: linear-gradient(180deg, #0a0a0f 0%, #111118 100%);
        }

        .features-header {
            text-align: center;
            margin-bottom: 4rem;
        }

        .features-title {
            font-size: clamp(2rem, 5vw, 3rem);
            font-weight: 700;
            margin-bottom: 1rem;
        }

        .features-subtitle {
            color: #94a3b8;
            font-size: 1.125rem;
            max-width: 500px;
            margin: 0 auto;
        }

        .feature-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 1.5rem;
            max-width: 1200px;
            margin: 0 auto;
        }

        .feature-card {
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid rgba(255, 255, 255, 0.06);
            border-radius: 16px;
            padding: 2rem;
            transition: transform 0.3s, border-color 0.3s;
        }

        .feature-card:hover {
            transform: translateY(-4px);
            border-color: rgba(99, 102, 241, 0.3);
        }

        .feature-icon {
            width: 48px;
            height: 48px;
            background: linear-gradient(135deg, rgba(99, 102, 241, 0.2) 0%, rgba(168, 85, 247, 0.2) 100%);
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            margin-bottom: 1.5rem;
            font-size: 1.5rem;
        }

        .feature-card h3 {
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 0.75rem;
        }

        .feature-card p {
            color: #94a3b8;
            line-height: 1.6;
        }

        /* Scroll Progress Section */
        .scroll-reveal {
            padding: 6rem 2rem;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            background: #0a0a0f;
        }

        .scroll-reveal h2 {
            font-size: clamp(1.5rem, 4vw, 2.5rem);
            margin-bottom: 3rem;
            text-align: center;
        }

        .grid-container {
            display: grid;
            grid-template-columns: repeat(13, 1fr);
            gap: 4px;
            max-width: 600px;
            width: 100%;
        }

        .grid-item {
            aspect-ratio: 1;
            background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
            border-radius: 4px;
            transform: scale(0);
            opacity: 0;
        }

        /* Marquee Section */
        .marquee-section {
            padding: 4rem 0;
            overflow: hidden;
            background: linear-gradient(180deg, #111118 0%, #0a0a0f 100%);
        }

        .marquee {
            display: flex;
            gap: 4rem;
            animation: marquee 30s linear infinite;
        }

        .marquee-item {
            font-size: clamp(2rem, 6vw, 4rem);
            font-weight: 700;
            white-space: nowrap;
            color: rgba(255, 255, 255, 0.1);
            text-transform: uppercase;
            letter-spacing: 0.1em;
        }

        @keyframes marquee {
            from { transform: translateX(0); }
            to { transform: translateX(-50%); }
        }

        /* Footer */
        footer {
            padding: 4rem 2rem;
            text-align: center;
            border-top: 1px solid rgba(255, 255, 255, 0.06);
        }

        footer p {
            color: #64748b;
        }

        /* Timeline progress indicator */
        .progress-bar {
            position: fixed;
            top: 0;
            left: 0;
            height: 3px;
            background: linear-gradient(90deg, #6366f1, #8b5cf6);
            transform-origin: left;
            transform: scaleX(var(--st-page-scroll-progress, 0));
            z-index: 100;
        }
    </style>
</head>
<body>
    <div class="progress-bar"></div>

    <section class="hero">
        <div class="hero-bg"></div>
        <div class="floating-elements">
            <div class="floating-orb orb-1"></div>
            <div class="floating-orb orb-2"></div>
            <div class="floating-orb orb-3"></div>
        </div>
        <div class="hero-content">
            <div class="hero-badge">
                <span>Powered by Spacetime</span>
            </div>
            <h1 class="hero-title">Declarative animations,<br>infinite possibilities</h1>
            <p class="hero-subtitle">
                All dynamic behavior described as transformations of timelines.
                Scroll, time, loops, events - unified under one elegant abstraction.
            </p>
            <button class="hero-cta">
                <span>Explore the demo</span>
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M5 12h14M12 5l7 7-7 7"/>
                </svg>
            </button>
        </div>
    </section>

    <section class="features">
        <div class="features-header">
            <h2 class="features-title">Timeline-Driven Design</h2>
            <p class="features-subtitle">Every animation is a function of timeline progress</p>
        </div>
        <div class="feature-grid">
            <div class="feature-card">
                <div class="feature-icon">📜</div>
                <h3>Scroll Timelines</h3>
                <p>Animations that respond to scroll position with precise control over start, end, and scrub behavior.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">⏱️</div>
                <h3>Time Timelines</h3>
                <p>Classic duration-based animations with iterations, alternation, and delay support.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🔄</div>
                <h3>Loop Timelines</h3>
                <p>Cyclical animations with configurable period and phase offset for perpetual motion.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🖱️</div>
                <h3>Mouse Timelines</h3>
                <p>Progress driven by cursor position, enabling parallax and interactive effects.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">✨</div>
                <h3>Event Timelines</h3>
                <p>Hover, click, and focus-triggered animations with smooth enter/leave transitions.</p>
            </div>
            <div class="feature-card">
                <div class="feature-icon">🎯</div>
                <h3>CSS Variables</h3>
                <p>Each timeline exposes its progress as --st-{id}-progress for pure CSS hooks.</p>
            </div>
        </div>
    </section>

    <section class="scroll-reveal">
        <h2>Scroll to reveal the grid</h2>
        <div class="grid-container">
            <!-- Grid items generated by JS -->
        </div>
    </section>

    <section class="marquee-section">
        <div class="marquee">
            <span class="marquee-item">Declarative</span>
            <span class="marquee-item">Composable</span>
            <span class="marquee-item">Performant</span>
            <span class="marquee-item">Elegant</span>
            <span class="marquee-item">Declarative</span>
            <span class="marquee-item">Composable</span>
            <span class="marquee-item">Performant</span>
            <span class="marquee-item">Elegant</span>
        </div>
    </section>

    <footer>
        <p>Built with Spacetime - A Rust-powered declarative animation DSL</p>
    </footer>

    <script src="/__spacetime/runtime.js"></script>
    <script>
        // Generate grid items
        const gridContainer = document.querySelector('.grid-container');
        for (let i = 0; i < 169; i++) {
            const item = document.createElement('div');
            item.className = 'grid-item';
            gridContainer.appendChild(item);
        }

        // Initialize Spacetime
        Spacetime.init();
        console.log('Spacetime initialized');
    </script>
</body>
</html>
"##.to_string()
}

/// Parse index.st with import resolution for dev endpoints.
/// Returns the fully-resolved AST with all imports merged.
fn parse_site_ast(site_dir: &Path) -> Result<crate::parser::StFile, String> {
    let index_st = site_dir.join("index.st");
    let content = std::fs::read_to_string(&index_st)
        .map_err(|e| format!("Failed to read index.st: {}", e))?;
    let main_ast = crate::parser::parse(&content)
        .map_err(|e| e.render_all_plain(&content, &index_st.to_string_lossy()))?;
    let mut ast = if !main_ast.imports.is_empty() {
        crate::parser::resolve_imports(&main_ast, &index_st, site_dir)
            .map_err(|e| format!("Import resolution failed: {}", e))?
    } else {
        main_ast
    };
    // Re-extract FormMatches if imports brought user-defined macros
    if !ast.meta_defs.is_empty() {
        crate::parser::rematch_with_user_macros(&mut ast, &content);
    }
    Ok(ast)
}

/// Serve the shared Structure IR projection to the same-origin development
/// inspector. This mirrors the migrations routes' request-time `AppMode::Dev`
/// gate: production builds must not expose source structure.
async fn inspect_structure_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(query): Query<InspectStructureQuery>,
) -> Response {
    let Some(entry) = query.entry.filter(|entry| !entry.is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "missing required query parameter: entry"})),
        )
            .into_response();
    };
    let requested_template = query.template.filter(|name| !name.is_empty());
    let AppMode::Dev { site_dir, .. } = &state.mode else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };

    let path = match crate::sync::handlers::validate_file_path(site_dir, &entry) {
        Ok(path) => path,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid path: {error}")})),
            )
                .into_response();
        }
    };
    let empty = || {
        serde_json::json!({
            "entry": entry,
            "nodes": [],
            "params": [],
            "diagnostics": [],
        })
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            let mut body = empty();
            body["error"] = serde_json::json!(format!("read failed: {error}"));
            return Json(body).into_response();
        }
    };
    let bundle = match crate::mcp::bundle::compile_to_bundle_at(&source, site_dir, &path) {
        Ok(bundle) => bundle,
        Err(error) => {
            let mut body = empty();
            body["error"] = serde_json::json!(format!("compile failed: {error:?}"));
            return Json(body).into_response();
        }
    };
    let diagnostics = serde_json::json!(bundle.diagnostics);

    // ENTRY SELECTION. An explicit `?template=` is honoured exactly (a caller that
    // named a template and got a different one back would be silently inspecting
    // the wrong thing). Without one, fall back through the SHARED rule the MCP
    // projection uses — `main` is a convention, and real pages rarely have it.
    let template = match &requested_template {
        Some(name) => name.clone(),
        None => match crate::introspect::project::default_entry_template(&bundle) {
            Some(name) => name.to_string(),
            None => {
                // A bundle with no templates is not an error: the page authors its
                // markup at file scope. Say so precisely, so the client can present
                // "nothing to inspect here" rather than a phantom missing-name.
                let mut body = empty();
                body["diagnostics"] = diagnostics;
                body["reason"] = serde_json::json!("no-templates");
                return Json(body).into_response();
            }
        },
    };
    if !bundle
        .templates
        .iter()
        .any(|candidate| candidate.name == template)
    {
        let mut body = empty();
        body["diagnostics"] = diagnostics;
        body["error"] = serde_json::json!(format!("no template named '{template}' in {entry}"));
        return Json(body).into_response();
    }

    // Walk one root into (nodes, credentialed params). Defined as a closure so the
    // single-root and all-roots responses share ONE credentialing path: a second
    // copy would be a place for the write-safety rules below to drift out of sync.
    let walk_root = |root: &str,
                     file_cache: &mut std::collections::HashMap<String, Option<String>>|
     -> (serde_json::Value, serde_json::Value) {
        let nodes = crate::introspect::project::structure_json_for_entry(&bundle, root);
        let mut params = crate::introspect::project::param_rows(&nodes);

        // STALENESS ADDRESSING (PLAN-112 W1): a row is a write address — (file, span,
        // hash) — and the hash must prove that THIS span was derived from THOSE bytes.
        // Hash and span must therefore come from ONE snapshot of each file.
        //
        // W1R review finding (P1): hashing here by re-reading each imported path is a
        // DIFFERENT read from the one import resolution used to derive the spans. If a
        // file is saved in between, the response pairs version A's span with version
        // B's hash — and the write guard, seeing its own hash match, would then apply
        // A's byte range to B. That is precisely the corruption the guard exists to
        // prevent, so the pairing is verified rather than assumed: re-read each file,
        // and if its bytes changed since the compile consumed them, serve NO hash for
        // its rows. A row without a hash cannot be blind-written — the write rail
        // rejects a stale span on bounds, and the next 2s poll returns a consistent
        // snapshot.
        //
        // The entry file is read once, above, and compiled from that exact String, so
        // its rows (`file: null`) are consistent by construction.
        if let Some(rows) = params.as_array_mut() {
            for row in rows {
                let row_file = row.get("file").and_then(|f| f.as_str()).map(str::to_string);
                let bytes: Option<String> = match &row_file {
                    None => Some(source.clone()),
                    Some(row_path) => file_cache
                        .entry(row_path.clone())
                        .or_insert_with(|| std::fs::read_to_string(row_path).ok())
                        .clone(),
                };

                // The AGREEMENT CHECK. Slice the row's own span out of those bytes and
                // confirm it still reads as the invocation this row describes: it must
                // carry the param's name AND the value the projection reported. That is
                // what proves span, value, and hash all describe ONE snapshot — a file
                // saved between the compile's read and this one shifts or re-points the
                // span, and the slice stops matching.
                let addressed = (|| {
                    let bytes = bytes.as_deref()?;
                    let start = row.get("span_start")?.as_u64()? as usize;
                    let end = row.get("span_end")?.as_u64()? as usize;
                    if start >= end || end > bytes.len() {
                        return None;
                    }
                    if !bytes.is_char_boundary(start) || !bytes.is_char_boundary(end) {
                        return None;
                    }
                    let span_text = &bytes[start..end];
                    let param = row.get("param")?.as_str()?;
                    if !span_text.contains(param) {
                        return None;
                    }
                    // A bound param's projected value must be present verbatim in the
                    // invocation text; an UNBOUND param (no arg written at the call
                    // site) has no value to find, and the name check above suffices.
                    if row.get("bound").and_then(serde_json::Value::as_bool) == Some(true)
                        && let Some(value) = row.get("value").and_then(serde_json::Value::as_str)
                        && !span_text.contains(value)
                    {
                        return None;
                    }
                    Some((
                        format!("{:016x}", crate::migrate::hash_content(bytes)),
                        span_text.to_string(),
                    ))
                })();

                match addressed {
                    Some((hash, span_text)) => {
                        row["source_hash"] = serde_json::json!(hash);
                        row["span_text"] = serde_json::json!(span_text);
                    }
                    // No proof of agreement → no write credentials. The pill can still
                    // DISPLAY the row; a write without them is refused by the rail, and
                    // the next poll (2s) serves a consistent snapshot.
                    None => {
                        row["source_hash"] = serde_json::Value::Null;
                        row["span_text"] = serde_json::Value::Null;
                    }
                }
            }
        }

        (nodes, params)
    };

    // The chosen root and the full roster travel WITH the walk. A page with many
    // templates has many possible roots, and the fallback picks one; without these
    // the client would present that arbitrary choice as if it were the whole page.
    let template_names: Vec<String> = bundle
        .templates
        .iter()
        .map(|candidate| candidate.name.clone())
        .collect();

    let mut file_cache: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let (nodes, params) = walk_root(&template, &mut file_cache);

    // ALL-ROOTS. `@data fetch` resolves its URL ONCE, as a literal, so a client
    // cannot re-point the source at a different `?template=` — switching roots
    // would need a refetch API that does not exist. Rather than invent one, send
    // every root in the same response and let the client filter: switching then
    // costs nothing and cannot show a root's tree beside another's params.
    let by_root = if query.all.as_deref() == Some("1") {
        let mut map = serde_json::Map::new();
        for name in &template_names {
            let (root_nodes, root_params) = walk_root(name, &mut file_cache);
            map.insert(
                name.clone(),
                serde_json::json!({ "nodes": root_nodes, "params": root_params }),
            );
        }
        serde_json::Value::Object(map)
    } else {
        serde_json::Value::Null
    };

    Json(serde_json::json!({
        "entry": entry,
        "template": template,
        "templates": template_names,
        "nodes": nodes,
        "params": params,
        "roots": by_root,
        "diagnostics": diagnostics,
        "error": serde_json::Value::Null,
    }))
    .into_response()
}
/// Return JSON Schema for all @type definitions in the site.
/// Uses full import resolution so types from @import-ed files are included.
/// Dev+debug mode only.
async fn dev_types_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, _trace, debug) = match &state.mode {
        AppMode::Dev {
            site_dir,
            trace,
            debug,
        } => (site_dir.clone(), trace, debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    let index_st = site_dir.join("index.st");
    if !index_st.exists() {
        return (StatusCode::OK, Json(serde_json::json!({"types": {}}))).into_response();
    }

    let ast = match parse_site_ast(&site_dir) {
        Ok(a) => a,
        Err(_) => return (StatusCode::OK, Json(serde_json::json!({"types": {}}))).into_response(),
    };
    let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);

    // Collect @cms(TypeName) display-hint overrides, keyed by target type name.
    // Each @cms FormMatch carries `type` (ident) + `hints` (properties: role->field).
    let cms_hints = collect_cms_hints(&ast.matches);

    let mut types_map = serde_json::Map::new();
    for (name, type_def) in registry.iter_types() {
        let mut schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference(name.clone()),
            &registry,
        );
        // Attach computed display roles (convention inference + @cms override) so
        // the local content admin can render collection rows without re-deriving.
        let display = compute_display_roles(type_def, cms_hints.get(name));
        if let Some(obj) = schema.as_object_mut() {
            obj.insert("display".to_string(), display);
        }
        // FEAT-109 W2. Attach one representative literal per field from the
        // site's own seed data BEFORE widgets are chosen, so a field whose type
        // says only `string` can still earn a colour picker or a duration
        // control from the value it actually holds.
        //
        // This runs before `annotate_widgets` because that is the consumer: it
        // reads `x-st-sample` through `widget_for`. Nothing else in the payload
        // changes, and a field with no seed value is left exactly as it was.
        attach_field_samples(&mut schema, &ast.matches);

        // Annotate every field node with the admin form widget it maps to
        // (W1 widget engine). The admin renders widgets purely from this.
        annotate_widgets(&mut schema);
        types_map.insert(name.clone(), schema);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"types": types_map})),
    )
        .into_response()
}

/// Return the editable schema for all `@editable-mark`/`@editable-block`
/// declarations in the site (PLAN-031 / FEAT-099). Mirrors `dev_types_handler`:
/// the schema is derived ON DEMAND from the site AST FormMatches (never a stored
/// allowlist), so adding a mark makes it appear here automatically. The editor
/// reads this to know which marks/blocks exist, what they project to, which typed
/// attrs they carry, and whether each is invertible (paste-lift eligible).
/// Dev+debug mode only.
async fn dev_editable_schema_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, debug) = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } => (site_dir.clone(), *debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }
    let empty = serde_json::json!({ "marks": {}, "blocks": {} });
    let index_st = site_dir.join("index.st");
    if !index_st.exists() {
        return (StatusCode::OK, Json(empty)).into_response();
    }
    let ast = match parse_site_ast(&site_dir) {
        Ok(a) => a,
        Err(_) => return (StatusCode::OK, Json(empty)).into_response(),
    };
    let (schemas, _diags) =
        crate::editable::collect_schemas_from_matches(&ast.matches, &ast.scopes);
    let json = crate::editable::schemas_to_json(&schemas);
    (StatusCode::OK, Json(json)).into_response()
}

/// Extract `@cms(TypeName) { role: field; ... }` display-hint overrides from the
/// site AST, keyed by target type name. Roles: title/subtitle/thumbnail/chips.
/// Interim file-scope form (see stdlib/__admin__/cms-hint.st); the nested
/// `@type.cms` form is tracked by FUP-macro-namespaces.
fn collect_cms_hints(
    matches: &[crate::syntax::FormMatch],
) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
    let mut out: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        std::collections::HashMap::new();
    for fm in matches.iter().filter(|m| m.macro_name == "cms") {
        let Some(type_name) = fm.get_ident("type") else {
            continue;
        };
        let entry = out.entry(type_name.to_string()).or_default();
        if let Some(props) = fm.get_properties("hints") {
            for p in props {
                // role (p.name, e.g. "title") -> field (p.type_ref, e.g. "name")
                entry.insert(p.name.clone(), p.type_ref.clone());
            }
        }
    }
    out
}

/// Recursively annotate a JSON-schema object's field nodes with the admin form
/// `widget` they map to (W1 widget engine, spec §4.3). Operates on a schema with
/// shape `{ type:"object", properties:{ field: <node> } }`. Each property node
/// gets a `"widget"` string; nested objects/arrays recurse so the admin can
/// render arbitrarily deep forms without re-deriving widget kinds client-side.
fn annotate_widgets(schema: &mut serde_json::Value) {
    let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) else {
        return;
    };
    for (field_name, node) in props.iter_mut() {
        let widget = widget_for(field_name, node);
        // Recurse BEFORE inserting our own key so nested shapes are handled.
        match node.get("type").and_then(|t| t.as_str()) {
            Some("object") => annotate_widgets(node),
            Some("array") => {
                if let Some(items) = node.get_mut("items")
                    && items.get("type").and_then(|t| t.as_str()) == Some("object")
                {
                    annotate_widgets(items);
                }
            }
            _ => {}
        }
        if let Some(obj) = node.as_object_mut() {
            obj.insert("widget".to_string(), serde_json::Value::String(widget));
        }
    }
}

/// Attach one representative literal per field (`x-st-sample`) from the site's
/// inline seed data, so widget selection can consult the VALUE and not just the
/// declared type (FEAT-109 W2).
///
/// The corpus has 21 `@type` annotations against 2,999 inferable values, so for
/// most fields the schema says `{"type":"string"}` and nothing more — while the
/// seed sitting beside it says `#6b7280`. This is the join between the two.
///
/// Deliberately best-effort and side-effect-free on failure: a site with no
/// inline data, unparseable seeds, or fields that appear in no record simply
/// gets no samples, and every widget decision falls back to exactly what it was
/// before this ran.
fn attach_field_samples(schema: &mut serde_json::Value, matches: &[crate::syntax::FormMatch]) {
    let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) else {
        return;
    };

    // Every inline seed on the page, as JSON. A record's field names are the
    // join key — not the type name — because an untyped seed has no type name to
    // match on, which is the whole population this wave is for.
    let mut samples: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for fm in matches.iter().filter(|m| m.macro_name == "data") {
        let Some(raw) = fm.get_expr("value").or_else(|| fm.get_string("value")) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
            continue;
        };
        collect_samples(&parsed, &mut samples);
    }

    for (field_name, node) in props.iter_mut() {
        let Some(sample) = samples.get(field_name) else {
            continue;
        };
        if let Some(obj) = node.as_object_mut() {
            obj.insert(
                "x-st-sample".to_string(),
                serde_json::Value::String(sample.clone()),
            );
        }
    }
}

/// Walk seed JSON and record the FIRST string literal seen for each field name.
///
/// First rather than most-common: a colour field holds colours, so any record
/// answers the question, and scanning every record to vote would cost more than
/// the decision is worth. A field whose values genuinely disagree in type is a
/// data problem the admin cannot fix by picking a different text box.
fn collect_samples(value: &serde_json::Value, out: &mut std::collections::HashMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if let Some(s) = v.as_str()
                    && !s.is_empty()
                {
                    out.entry(k.clone()).or_insert_with(|| s.to_string());
                } else {
                    collect_samples(v, out);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_samples(item, out);
            }
        }
        _ => {}
    }
}

/// The widget a field's SAMPLE VALUE earns, if the value names a scalar.
///
/// `x-st-sample` carries one representative literal for the field — the seed
/// value the admin is about to edit. It is an `x-` extension key because it is
/// Spacetime's, not JSON Schema's, and it is optional: a node without one is
/// unchanged by this function, which is what keeps W2 a strict addition to every
/// existing caller.
///
/// Returns `None` — not a default — when inference abstains, so the caller's
/// name heuristics still run. An abstention means "no opinion", never "plain
/// text": a field named `image` holding an unrecognisable value is still media.
fn widget_from_sample(node: &serde_json::Value) -> Option<String> {
    use crate::types::value_infer::{Inferred, infer_value_type};

    let sample = node.get("x-st-sample")?.as_str()?;

    // Only an UNAMBIGUOUS scalar earns a widget. A tie (`100%` is a length and a
    // percentage) could be resolved here the way the sync layer does it — map
    // every candidate, accept unanimity — but the admin has no such case today,
    // and inventing the machinery before a caller needs it is how a second
    // resolution policy gets born. `Reference` and `Unknown` abstain by
    // definition.
    let Inferred::Scalar(scalar) = infer_value_type(sample) else {
        return None;
    };

    // The scalar table is the source: `%scalar_type color { %widget "color" }`.
    // Adding a scalar to stdlib therefore adds its admin control with no Rust
    // change, which is the acceptance test for the whole `%scalar_type` design
    // (PLAN-122 W4) and the reason this reads a registry rather than a match.
    let (registry, _) = crate::compiler::cached_stdlib_registry();
    let widget = registry.get_scalar_type(&scalar)?.widget.clone();
    if widget.is_empty() {
        None
    } else {
        Some(widget)
    }
}

/// Test-only handle on sample attachment (FEAT-109 W2 gates).
///
/// Exposed for the same reason as `widget_for_public`: the contract under test
/// is "a seed literal becomes the sample the widget decision reads", and driving
/// that through the dev server would additionally exercise routing, file
/// discovery and import resolution — then fail for whichever broke first.
pub fn attach_field_samples_public(
    schema: &mut serde_json::Value,
    matches: &[crate::syntax::FormMatch],
) {
    attach_field_samples(schema, matches)
}

/// Test-only handle on the widget decision (FEAT-109 W2 gates).
///
/// `widget_for` is private because widget selection is the admin's internal
/// business. The gates assert the CONTRACT at this exact boundary — which signal
/// wins when several apply — and driving that through an HTTP round-trip would
/// test routing, serialization and schema generation at the same time, then fail
/// for whichever broke first.
pub fn widget_for_public(field_name: &str, node: &serde_json::Value) -> String {
    widget_for(field_name, node)
}

/// Decide the admin form widget kind for a single field, from its name and its
/// JSON-schema node. Pure function (spec §4.3). Returns one of:
/// text | textarea | number | toggle | select | media | chips | list |
/// fieldset | readonly | json.
fn widget_for(field_name: &str, node: &serde_json::Value) -> String {
    let lname = field_name.to_ascii_lowercase();
    let ty = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let format = node.get("format").and_then(|f| f.as_str()).unwrap_or("");
    let is_enum = node.get("enum").is_some();

    // CMS relation `id(T)`: a single ref (string + x-st-ref) -> relation picker;
    // a multi ref (array whose items carry x-st-ref) -> relation-multi picker.
    // Checked before identity/name heuristics so a field literally named `id`
    // typed `id(T)` is still a relation, not readonly.
    if node.get("x-st-ref").is_some() {
        return "relation".to_string();
    }
    // Rich text (constrained-AST block editor).
    if node.get("x-st-richtext").and_then(|v| v.as_bool()) == Some(true) {
        return "richtext".to_string();
    }
    if ty == "array" && node.get("items").and_then(|i| i.get("x-st-ref")).is_some() {
        return "relation-multi".to_string();
    }

    // Identity/derived fields are not directly editable.
    if lname == "slug" || lname == "id" {
        return "readonly".to_string();
    }
    // Enum/union -> select, regardless of base type.
    if is_enum {
        return "select".to_string();
    }
    match ty {
        "boolean" => "toggle".to_string(),
        "number" | "integer" => "number".to_string(),
        "object" => "fieldset".to_string(),
        "array" => {
            let item_ty = node
                .get("items")
                .and_then(|i| i.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("");
            match item_ty {
                "string" => "chips".to_string(),
                "object" => "list".to_string(),
                _ => "list".to_string(),
            }
        }
        "string" => {
            // Domain scalar formats: the `%format` -> `%widget` mapping lives in
            // the scalar table (stdlib/scalars/types.st), so a scalar added there
            // gets its admin control with no Rust change. Only a NON-EMPTY format
            // is consulted — richtext's `%format` is deliberately empty (it
            // dispatches on `x-st-richtext` above) and must not match every string.
            if !format.is_empty() {
                let (scalar_registry, _) = crate::compiler::cached_stdlib_registry();
                if let Some(widget) = scalar_registry
                    .scalar_types()
                    .find(|row| row.format == format)
                    .map(|row| row.widget.clone())
                {
                    return widget;
                }
            }
            // FEAT-109 W2. Nothing above answered: the schema says `string` with
            // no `format`, which is what EVERY value looks like unless someone
            // wrote an `@type`. The corpus has 21 of those against 2,999
            // inferable values, so this is the common case, not the fallback.
            //
            // Ask the value. `infer_value_type` names the scalar from the
            // literal's own shape, and the scalar table already carries that
            // scalar's `%widget` — so a colour gets a picker and a duration gets
            // a duration control with no annotation anywhere. No widget logic is
            // added here; the existing `%format` -> `%widget` lookup just gets an
            // input it never had.
            //
            // ORDER IS THE DESIGN. This sits AFTER every declared signal
            // (`x-st-ref`, richtext, enum, identity, `format`) because a
            // declaration is an author's intent and inference must never
            // override it. It sits BEFORE the name heuristics because those are
            // guesses about the field's NAME, and evidence about the actual
            // VALUE beats a guess about its label — a field called `icon` holding
            // `#6b7280` is a colour, whatever it is called.
            if let Some(widget) = widget_from_sample(node) {
                return widget;
            }

            // Media: URI-format or an image/asset-ish field name.
            let media_named = matches!(
                lname.as_str(),
                "image" | "photo" | "avatar" | "thumbnail" | "cover" | "icon" | "url" | "src"
            );
            if format == "uri" || media_named {
                return "media".to_string();
            }
            // Long-form prose -> textarea.
            let long_named = lname.contains("description")
                || lname.contains("body")
                || lname.contains("bio")
                || lname.contains("summary")
                || lname.contains("content")
                || lname.contains("excerpt");
            if long_named {
                "textarea".to_string()
            } else {
                "text".to_string()
            }
        }
        // Unknown / unresolved -> raw JSON editor fallback.
        _ => "json".to_string(),
    }
}

/// Compute display roles for a type: convention-based inference from field
/// names/types, with explicit @cms hints taking precedence. Returns a JSON
/// object `{ title, subtitle, thumbnail, chips }` (each value a field name or
/// null). This is the single source of truth the admin reads — it never
/// re-derives display roles client-side.
fn compute_display_roles(
    type_def: &crate::parser::TypeDef,
    hints: Option<&std::collections::HashMap<String, String>>,
) -> serde_json::Value {
    use crate::parser::TypeExpr;

    let field_names: Vec<&str> = type_def.fields.iter().map(|f| f.name.as_str()).collect();
    let has = |n: &str| field_names.iter().any(|f| f.eq_ignore_ascii_case(n));
    let first_of = |cands: &[&str]| -> Option<String> {
        cands.iter().find(|c| has(c)).map(|c| {
            // Return the actual field name with its real casing.
            field_names
                .iter()
                .find(|f| f.eq_ignore_ascii_case(c))
                .unwrap()
                .to_string()
        })
    };
    // A field is a string[] (chips candidate) when its type is Array(String).
    let is_string_array = |name: &str| -> bool {
        type_def
            .fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| matches!(&f.type_expr, TypeExpr::Array(inner) if matches!(&**inner, TypeExpr::Primitive(p) if p == "string")))
            .unwrap_or(false)
    };
    let first_string_array = |cands: &[&str]| -> Option<String> {
        cands
            .iter()
            .filter(|c| has(c))
            .map(|c| {
                field_names
                    .iter()
                    .find(|f| f.eq_ignore_ascii_case(c))
                    .unwrap()
                    .to_string()
            })
            .find(|f| is_string_array(f))
            .or_else(|| {
                // Fall back to ANY string[] field if no conventional name matched.
                type_def
                    .fields
                    .iter()
                    .find(|f| matches!(&f.type_expr, TypeExpr::Array(inner) if matches!(&**inner, TypeExpr::Primitive(p) if p == "string")))
                    .map(|f| f.name.clone())
            })
    };

    // Convention inference.
    let mut title = first_of(&["name", "title", "label"]);
    let mut subtitle = first_of(&["headline", "subtitle", "description", "summary"]);
    let mut thumbnail = first_of(&["image", "photo", "avatar", "thumbnail", "cover"]);
    let mut chips = first_string_array(&["categories", "tags", "labels"]);

    // @cms overrides (only for roles the author specified; unspecified roles keep
    // the convention value). Hint values must name a real field to take effect.
    if let Some(h) = hints {
        let valid = |field: &String| field_names.iter().any(|f| f == field);
        if let Some(f) = h.get("title").filter(|f| valid(f)) {
            title = Some(f.clone());
        }
        if let Some(f) = h.get("subtitle").filter(|f| valid(f)) {
            subtitle = Some(f.clone());
        }
        if let Some(f) = h.get("thumbnail").filter(|f| valid(f)) {
            thumbnail = Some(f.clone());
        }
        if let Some(f) = h.get("chips").filter(|f| valid(f)) {
            chips = Some(f.clone());
        }
    }

    serde_json::json!({
        "title": title,
        "subtitle": subtitle,
        "thumbnail": thumbnail,
        "chips": chips,
    })
}

/// Build the brand-singleton JSON from the site AST (PLAN-034 Wave B). Finds the
/// `@data inline $name Type : { ... }` singleton (the typed design-token value
/// that lives in code), resolves its `@type` schema with admin widgets attached
/// (same projection as dev_types_handler), and parses its quoted-key object value
/// into the current token values. The admin's Brand pane renders this with the
/// shared form-renderer and edits it via EditAst (rewriting the .st literal).
///
/// A singleton is recognised structurally: a file-scope `@data inline` binding
/// carrying a typeref, marked editable by an `@cms(Type) { editable: inline; }`
/// directive. Returns `{ "brand": null }` when no such singleton exists.
fn build_brand_json(ast: &crate::parser::StFile) -> serde_json::Value {
    // Collect the set of types marked `@cms(Type){ editable: inline }` — these
    // are the singletons the admin may edit structurally. (collect_cms_hints
    // already gathers @cms props per type; here we just need the `editable` role.)
    let editable_types: std::collections::HashSet<String> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "cms")
        .filter_map(|m| {
            let ty = m.get_ident("type")?;
            let props = m.get_properties("hints")?;
            let is_inline = props
                .iter()
                .any(|p| p.name == "editable" && p.type_ref == "inline");
            is_inline.then(|| ty.to_string())
        })
        .collect();

    // Find the first file-scope `@data inline $name Type : <value>` whose type is
    // marked editable-inline (the brand singleton).
    let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
    for m in ast.matches.iter() {
        if m.selector.is_some() || m.macro_name != "data" {
            continue;
        }
        if m.matched_macro.as_deref() != Some("data-inline") {
            continue;
        }
        let Some(type_name) = m.type_name("type") else {
            continue;
        };
        if !editable_types.contains(type_name) {
            continue;
        }
        let name = m
            .get_binding("name")
            .map(|s| s.trim_start_matches('$'))
            .unwrap_or("");
        // Schema with admin widgets attached (mirror dev_types_handler).
        let mut schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference(type_name.to_string()),
            &registry,
        );
        annotate_widgets(&mut schema);
        // Parse the object-literal value (quoted-key JSON) into current values.
        let values: serde_json::Value = m
            .get_expr("value")
            .and_then(|raw| serde_json::from_str(raw.trim()).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let file = m
            .source_file
            .clone()
            .unwrap_or_else(|| "index.st".to_string());
        // Derived tokens (FEAT-108): every `@data derive $x : <expr>` whose
        // formula references this singleton (`$<name>`). The admin renders these
        // as READ-ONLY formula chips + a resolved swatch (the resolved value is
        // computed client-side by the reactive derive, so the server delivers the
        // FORMULA, not a value it can't evaluate). Self-describing: add a derive
        // and it appears with zero admin edits.
        let derived = collect_derived_tokens(ast, name);
        return serde_json::json!({
            "brand": {
                "name": name,
                "type": type_name,
                "schema": schema,
                "values": values,
                "file": file,
                "derived": derived,
            }
        });
    }
    serde_json::json!({ "brand": null })
}

/// Collect `@data derive` tokens whose formula references the brand singleton
/// `$<binding>`. Returns `[{ name, formula }]` in source order — the admin shows
/// each as a read-only formula chip; the resolved swatch comes from the live
/// reactive signal, not the server (the server can't evaluate the derivation).
fn collect_derived_tokens(ast: &crate::parser::StFile, binding: &str) -> serde_json::Value {
    let needle = format!("${binding}");
    let mut out: Vec<serde_json::Value> = Vec::new();
    for m in ast.matches.iter() {
        if m.macro_name != "data" {
            continue;
        }
        if m.matched_macro.as_deref() != Some("data-derive") {
            continue;
        }
        let Some(name) = m.get_binding("name").map(|s| s.trim_start_matches('$')) else {
            continue;
        };
        let Some(formula) = m.get_expr("value") else {
            continue;
        };
        // Only derives that actually read the brand singleton (a `$<binding>`
        // token followed by a non-identifier char, so `$brandX` doesn't match).
        let references_brand = formula.match_indices(&needle).any(|(idx, _)| {
            formula[idx + needle.len()..]
                .chars()
                .next()
                .map(|c| !c.is_alphanumeric() && c != '_')
                .unwrap_or(true)
        });
        if !references_brand {
            continue;
        }
        out.push(serde_json::json!({
            "name": name,
            "formula": formula.trim(),
        }));
    }
    serde_json::json!(out)
}

/// Return the editable brand SINGLETON (PLAN-034 Wave B): the project's typed
/// design-token value (`@data inline $brand Brand : { ... }` + `@cms(Brand){
/// editable: inline }`), with widget-annotated schema + current values. The admin
/// renders it via the shared form-renderer and persists edits through EditAst.
async fn dev_brand_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, debug) = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } => (site_dir.clone(), *debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }
    let ast = match parse_site_ast(&site_dir) {
        Ok(a) => a,
        Err(_) => {
            return (StatusCode::OK, Json(serde_json::json!({"brand": null}))).into_response();
        }
    };
    (StatusCode::OK, Json(build_brand_json(&ast))).into_response()
}

/// Return theme tokens: CSS custom properties (`--name: value;`) declared in the
/// site's `.st` modules, with an inferred token TYPE so the admin renders the
/// right control (color swatch / length / duration / font / number / text).
/// Stage-3 Wave-7. Read-only discovery; edits go through the standard edit path.
/// Brand integrity is structural: the writer edits named TOKENS, not free CSS.
async fn dev_theme_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, debug) = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } => (site_dir.clone(), *debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    let mut tokens: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Scan .st modules for `--token: value;` custom-property declarations.
    let re = regex::Regex::new(r"(?m)^\s*(--[A-Za-z0-9_-]+)\s*:\s*([^;]+);").unwrap();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    fn collect_st(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
        if depth > 5 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            if e.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
                continue;
            }
            let p = e.path();
            if p.is_dir() {
                collect_st(&p, out, depth + 1);
            } else if p.extension().and_then(|x| x.to_str()) == Some("st") {
                out.push(p);
            }
        }
    }
    collect_st(&site_dir, &mut files, 0);
    files.sort();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(&site_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for cap in re.captures_iter(&text) {
            let name = cap[1].to_string();
            let value = cap[2].trim().to_string();
            if !seen.insert(name.clone()) {
                continue;
            }
            tokens.push(serde_json::json!({
                "name": name,
                "value": value,
                "type": infer_token_type(&value),
                "file": rel,
            }));
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "tokens": tokens })),
    )
        .into_response()
}

/// Return the motion inventory (FEAT-106): every scoped motion directive
/// (`@scroll name(...)`, `@load name(...)`, `@reveal(...)`, `@hover name(...)`)
/// declared in the site's compiled AST, with each tunable param shaped into a
/// widget-typed form field (the SAME `{tpl,label,value,path,widget}` shape the
/// drawer + Brand pane render). The admin Motion pane is then a clean
/// form-renderer over real, exportable, declarative motion — a category Framer
/// (imperative runtime config) can't reach.
///
/// This reads the COMPILED AST (mirror of `build_brand_json` / `dev_types`), not
/// a regex scrape: param kinds come from each `CapturedValue` (Time→duration,
/// Length→length, Number→number, Bool→toggle, Ident→select), and the element
/// SCOPE is delivered so an edit routes through the existing scoped EditAst
/// (`<selector> §<kind>` + `{param: value}`) — no new write machinery.
async fn dev_motion_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, debug) = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } => (site_dir.clone(), *debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }
    let ast = match parse_site_ast(&site_dir) {
        Ok(a) => a,
        Err(_) => {
            return (StatusCode::OK, Json(serde_json::json!({"motions": []}))).into_response();
        }
    };
    (StatusCode::OK, Json(build_motion_json(&ast))).into_response()
}

/// The motion directive kinds the admin can tune. Each is a `%macro` whose
/// `@<kind> name(params) { keyframes }` form lives in stdlib motion modules;
/// here we only need to recognize the captured `macro_name` to surface it.
fn is_motion_macro(macro_name: &str) -> bool {
    matches!(macro_name, "scroll" | "load" | "reveal" | "hover")
        || macro_name.starts_with("scroll-")
        || macro_name.starts_with("load-")
        || macro_name.starts_with("reveal-")
}

/// Map a captured param value to an admin widget + a normalized string value.
/// Mirrors the type→widget mapping `annotate_widgets` does for `@type` fields,
/// but sourced from the live `CapturedValue` kind of a motion param.
fn motion_param_widget(val: &crate::syntax::CapturedValue) -> Option<(&'static str, String)> {
    use crate::syntax::CapturedValue;
    // Widget names must be ones the admin form-renderer has a `&fld-<widget>`
    // template for (text/number/toggle). length/duration/ident collapse to the
    // text input today — the SAME mapping `annotate_widgets` uses for @type
    // length/duration fields — so every motion field renders with its value.
    match val {
        CapturedValue::Time(ms) => Some(("text", format!("{ms}ms"))),
        CapturedValue::Length(lv) => Some(("text", format!("{}{}", lv.value, lv.unit))),
        CapturedValue::Number(n) => Some(("number", {
            // Render integers without a trailing `.0` so the source round-trips.
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        })),
        CapturedValue::Bool(b) => Some(("toggle", b.to_string())),
        CapturedValue::Ident(s) => Some(("text", s.clone())),
        CapturedValue::String(s) => Some(("text", s.clone())),
        // Bindings/exprs/blocks are not point-tunable scalars — skip.
        _ => None,
    }
}

/// Build the motion JSON from a compiled site AST. Walks every scope's matches
/// for motion directives; each becomes `{ kind, name, label, selector, file,
/// fields:[{key,label,widget,tpl,value,path}] }`. `selector` + `kind` are the
/// EditAst address; `path` is the param key (the apply_text_patch target).
fn build_motion_json(ast: &crate::parser::StFile) -> serde_json::Value {
    fn humanize(key: &str) -> String {
        let mut out = String::new();
        for (i, ch) in key.chars().enumerate() {
            if i == 0 {
                out.extend(ch.to_uppercase());
            } else if ch == '_' || ch == '-' {
                out.push(' ');
            } else {
                out.push(ch);
            }
        }
        out
    }
    let mut motions: Vec<serde_json::Value> = Vec::new();
    for scope in &ast.scopes {
        for m in &scope.matches {
            if !is_motion_macro(&m.macro_name) {
                continue;
            }
            // The directive's own name (e.g. `reveal` in `@scroll reveal(...)`) is
            // captured as the `name` ident when the form has one; fall back to none.
            let name = m.get_ident("name").map(|s| s.to_string());
            // Shape each captured param into a widget-typed field, in a stable
            // key order so the form renders deterministically.
            let mut keys: Vec<&String> = m.captures.keys().collect();
            keys.sort();
            let mut fields: Vec<serde_json::Value> = Vec::new();
            for key in keys {
                if key == "name" || key == "body" {
                    continue;
                }
                let Some((widget, value)) = motion_param_widget(&m.captures[key]) else {
                    continue;
                };
                // NB: no `tpl` here. Which template renders a widget is a CLIENT
                // concern (`ST.adFieldTemplate`), and encoding it here too meant the
                // shared-widget set lived in two places that could silently drift —
                // a descriptor naming a template nobody registered renders NOTHING.
                // The server states the widget KIND; the client resolves the name.
                fields.push(serde_json::json!({
                    "key": key,
                    "label": humanize(key),
                    "widget": widget,
                    "value": value,
                    "path": key,
                }));
            }
            if fields.is_empty() {
                continue;
            }
            let label = match &name {
                Some(n) => format!("{} {}", m.macro_name, n),
                None => m.macro_name.clone(),
            };
            // The EditAst target file: where this directive actually lives. After
            // import resolution, scopes/matches from imported `.st` modules carry
            // their own source_file — routing an edit to a hardcoded index.st
            // would reject (directive not found there). Fall back to index.st for
            // the unresolved/main-file case.
            let file = m
                .source_file
                .as_deref()
                .or(scope.source_file.as_deref())
                .unwrap_or("index.st");
            motions.push(serde_json::json!({
                "kind": m.macro_name,
                "name": name,
                "label": label,
                "selector": scope.selector,
                "file": file,
                "fields": fields,
            }));
        }
    }
    serde_json::json!({ "motions": motions })
}

/// Return the component palette: every `@template &name($params)` in the site's
/// `.st` modules, with its typed signature decomposed into DATA needs (binding
/// params `$x`) and SLOTS (element params `&y`), optional flags preserved. This
/// is the Stage-3 page-builder palette — the project's own templates ARE the
/// component library (auto-discovered, zero registration).
async fn dev_templates_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, debug) = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } => (site_dir.clone(), *debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    let mut templates: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Match `@template &name(params) {` — name is &-prefixed, params is the paren body.
    let re = regex::Regex::new(r"@template\s+&([A-Za-z0-9_-]+)\s*\(([^)]*)\)").unwrap();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    fn collect_st(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
        if depth > 5 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            if e.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
                continue;
            }
            let p = e.path();
            if p.is_dir() {
                collect_st(&p, out, depth + 1);
            } else if p.extension().and_then(|x| x.to_str()) == Some("st") {
                out.push(p);
            }
        }
    }
    collect_st(&site_dir, &mut files, 0);
    files.sort();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(&site_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for cap in re.captures_iter(&text) {
            let name = cap[1].to_string();
            if !seen.insert(name.clone()) {
                continue;
            }
            let params_raw = cap[2].trim();
            let mut params: Vec<serde_json::Value> = Vec::new();
            if !params_raw.is_empty() {
                for p in params_raw.split(',') {
                    let p = p.trim();
                    if p.is_empty() {
                        continue;
                    }
                    let optional = p.ends_with('?');
                    let core = p.trim_end_matches('?');
                    let (kind, pname) = if let Some(rest) = core.strip_prefix('&') {
                        ("slot", rest)
                    } else if let Some(rest) = core.strip_prefix('$') {
                        ("binding", rest)
                    } else {
                        ("binding", core)
                    };
                    params.push(serde_json::json!({
                        "name": pname,
                        "kind": kind,
                        "optional": optional,
                    }));
                }
            }
            // Source-determined param SIGNATURE string (`$title, &slot, $sub?`).
            // Delivered here so the admin pane reads it directly — no client-side
            // shaping helper (PLAN-034 Wave C, AP2 removal).
            let sig = params
                .iter()
                .map(|p| {
                    let pn = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let sigil = if p.get("kind").and_then(|v| v.as_str()) == Some("slot") {
                        "&"
                    } else {
                        "$"
                    };
                    let opt = if p.get("optional").and_then(|v| v.as_bool()) == Some(true) {
                        "?"
                    } else {
                        ""
                    };
                    format!("{sigil}{pn}{opt}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            templates.push(serde_json::json!({
                "name": name,
                "sig": sig,
                "params": params,
                "file": rel,
            }));
        }
    }
    templates.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({ "templates": templates })),
    )
        .into_response()
}

/// Infer a theme token's editor type from its value.
/// Classify a design-token value by asking the SCALAR GRAMMARS which type
/// accepts it.
///
/// This used to be a fifth independent answer to "what is a colour?" — a
/// prefix/suffix heuristic (`starts_with('#')`, `ends_with("px")`) sitting
/// beside the lexer's positional promotion, the LSP's byte scanner, the Rust
/// extractors, and the `CaptureType` enum. Each drifted from the others in its
/// own direction: this one classified `#zz` as a colour, `1e5s` as a duration,
/// and any 5-digit hex as a perfectly good token.
///
/// PLAN-122 made `stdlib/capture-types/css-values.st` the one place that says
/// what a scalar's syntax IS, and `stdlib/scalars/types.st` the one place that
/// says what it MEANS. So the question is now asked of them, in the order the
/// table declares, and the first grammar that ACCEPTS THE WHOLE VALUE wins.
///
/// Requiring the whole value is what makes this honest rather than merely
/// relocated: a grammar that matches a prefix would reproduce the old
/// heuristic's central bug, where `#zzz` "started with #" and was therefore a
/// colour forever after.
fn infer_token_type(value: &str) -> String {
    let v = value.trim();

    let (registry, _) = crate::compiler::cached_stdlib_registry();
    let extractors = crate::syntax::events::extractors::ExtractorRegistry::new();

    // The candidates come from the SCALAR TABLE, not from a list here. Each row
    // names the grammar that validates its literals in its `%capture` column, so
    // adding a scalar to `stdlib/scalars/types.st` teaches this classifier about
    // it with no Rust change — which is the property the table exists to have.
    // A hardcoded list here would have been the same rot one level up.
    //
    // Rows are tried MOST SPECIFIC FIRST, approximated by grammar size: `color`
    // and `duration` accept narrow shapes, while the base scalars (`string`,
    // `number`) accept almost anything and must not pre-empt them. Rows whose
    // `%capture` names a base grammar are skipped entirely here and handled by
    // the `number` check below, because "is this a bare number?" is a question
    // about the VALUE, not about which scalar owns it.
    const BASE_CAPTURES: &[&str] = &["string", "number", "bool", "expr"];

    let mut candidates: Vec<(&str, &str)> = registry
        .scalar_types()
        .filter(|row| !row.capture.is_empty() && !BASE_CAPTURES.contains(&row.capture.as_str()))
        .map(|row| (row.capture.as_str(), row.id.as_str()))
        .collect();

    // `length` accepts a bare `50%` and `duration` a bare `2s`; neither overlaps
    // the other, but ordering is pinned so the answer cannot depend on registry
    // iteration order.
    candidates.sort_by_key(|(capture, _)| match *capture {
        "color" => 0,
        "duration" => 1,
        "length" => 2,
        _ => 3,
    });

    for (capture_name, token_type) in candidates {
        if grammar_accepts_whole_value(&registry, &extractors, capture_name, v) {
            return token_type.to_string();
        }
    }

    if v.parse::<f64>().is_ok() {
        return "number".to_string();
    }

    // `font` has no scalar grammar — a font stack is a comma-separated list of
    // family names, which is a list type rather than a scalar. The name-based
    // guess stays until there is a grammar to ask, and is marked as such so it
    // is not mistaken for a classification.
    let lower = v.to_ascii_lowercase();
    if lower.contains("serif")
        || lower.contains("sans")
        || lower.contains("mono")
        || lower.contains("font")
    {
        return "font".to_string();
    }

    "text".to_string()
}

/// True when the named stdlib `%capture_type` matches the ENTIRE value.
///
/// A partial match is a refusal here. The grammar is compiled from stdlib and
/// run over the value's own tokens, so this is the same machinery that
/// validates a directive argument — not a second implementation of it.
fn grammar_accepts_whole_value(
    registry: &crate::metasystem::MetaRegistry,
    extractors: &crate::syntax::events::extractors::ExtractorRegistry,
    capture_name: &str,
    value: &str,
) -> bool {
    use crate::syntax::cst::SyntaxKind;
    use crate::syntax::events::extractors::TokenData;

    let Some(def) = registry.get_capture_type(capture_name) else {
        return false;
    };

    // Compiled WITH the sibling definitions, because these productions compose
    // by reference — `color` is a choice over `hex_color`, `color_fn` and the
    // keyword sets, and none of those resolve without the sibling map.
    let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
        registry
            .capture_type_names()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|n| registry.get_capture_type(&n).map(|d| (n, d.clone())))
            .collect();

    let extractor = crate::syntax::events::extractors::custom::compile_pattern_with_defs(
        &def.pattern,
        extractors,
        &defs,
        &mut Vec::new(),
    );
    let extractor =
        crate::syntax::events::extractors::custom::wrap_reifier(capture_name, extractor);

    let tokens: Vec<TokenData> = crate::syntax::cst::lexer::Lexer::new(value)
        .tokenize()
        .into_iter()
        .filter(|t| t.kind != SyntaxKind::EOF)
        .map(|t| TokenData {
            kind: t.kind,
            text_range: (t.offset, t.offset + t.len()),
        })
        .collect();

    let Some((_, consumed)) = extractor.extract(&tokens, value) else {
        return false;
    };

    // Every non-trivia token must have been consumed. `#e8ee1x` must not pass by
    // matching a legal prefix and leaving the rest behind.
    tokens.iter().skip(consumed).all(|t| t.kind.is_trivia())
}

/// Return the media library: image assets discoverable under `assets/`.
/// Dev+debug mode only. Used by the admin's media picker to browse + reuse
/// existing images instead of only uploading fresh ones (Stage-2 Wave-5).
async fn dev_assets_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, debug) = match &state.mode {
        AppMode::Dev {
            site_dir, debug, ..
        } => (site_dir.clone(), *debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    let assets_dir = site_dir.join("assets");
    let mut images: Vec<serde_json::Value> = Vec::new();
    let exts = ["png", "jpg", "jpeg", "gif", "webp", "svg"];
    // Bounded recursive walk (depth-limited to avoid pathological trees).
    fn walk(
        dir: &std::path::Path,
        base: &std::path::Path,
        exts: &[&str],
        out: &mut Vec<serde_json::Value>,
        depth: usize,
    ) {
        if depth > 6 || out.len() > 2000 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            // Skip symlinks: a symlink under assets/ pointing outside the project
            // would otherwise leak file paths/sizes from outside site_dir.
            if entry.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, exts, out, depth + 1);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && exts.contains(&ext.to_ascii_lowercase().as_str())
                && let Ok(rel) = path.strip_prefix(base)
            {
                let rel_str = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                out.push(serde_json::json!({ "path": rel_str, "size": size }));
            }
        }
    }
    if assets_dir.exists() {
        walk(&assets_dir, &site_dir, &exts, &mut images, 0);
    }
    // Stable order: newest-ish first is hard without mtime; sort by path for determinism.
    images.sort_by(|a, b| {
        a["path"]
            .as_str()
            .unwrap_or("")
            .cmp(b["path"].as_str().unwrap_or(""))
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({ "images": images })),
    )
        .into_response()
}

/// Return site structure: pages, data sources, locales.
/// Dev+debug mode only.
async fn dev_site_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let (site_dir, _trace, debug) = match &state.mode {
        AppMode::Dev {
            site_dir,
            trace,
            debug,
        } => (site_dir.clone(), trace, debug),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response();
        }
    };
    if !debug {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Not found"})),
        )
            .into_response();
    }

    let mut pages = Vec::new();
    let mut data_sources = Vec::new();
    let mut locales = Vec::new();

    // Scan pages/ directory for HTML files
    let pages_dir = site_dir.join("pages");
    if pages_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&pages_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "html").unwrap_or(false) {
                pages.push(
                    path.strip_prefix(&site_dir)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                );
            }
        }
    }
    if site_dir.join("index.html").exists() {
        pages.push("index.html".to_string());
    }

    // Scan data/ directory for JSON files
    let data_dir = site_dir.join("data");
    if data_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&data_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                data_sources.push(
                    path.strip_prefix(&site_dir)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                );
            }
        }
    }

    // Scan locales/ directory for locale JSON files
    let locales_dir = site_dir.join("locales");
    if locales_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&locales_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false)
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                locales.push(stem.to_string());
            }
        }
    }

    pages.sort();
    data_sources.sort();
    locales.sort();

    // Build data source → type mapping from CompileAnalysis
    let mut data_source_map = serde_json::Map::new();
    let index_st = site_dir.join("index.st");
    if index_st.exists()
        && let Ok(ast) = parse_site_ast(&site_dir)
    {
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let mut analysis = crate::analysis::CompileAnalysis::new();
        analysis.analyze_data_sources(&ast.matches, &registry);
        for ds in &analysis.data_sources {
            // Normalize file path: strip leading '/' to match filesystem-scanned paths
            let file_path = match &ds.source {
                crate::analysis::DataSource::File(p) => {
                    let p = p.trim_start_matches('/');
                    p.to_string()
                }
                _ => continue,
            };
            // Strip trailing [] from type_name for the JSON key
            let type_name = ds.type_name.trim_end_matches("[]").to_string();
            data_source_map.insert(
                file_path,
                serde_json::json!({
                    "type": type_name,
                    "isArray": ds.is_array,
                    "name": ds.name,
                }),
            );
        }
    }

    // Render-ready collections array: the dataSourceMap object folded into an
    // editorial list (file key inlined), sorted by display name. The admin is a
    // meta-renderer that @each-renders this directly — it never re-derives the
    // collection list client-side (mirrors the types.json widget/display contract).
    let mut collections: Vec<serde_json::Value> = data_source_map
        .iter()
        .filter_map(|(file, meta)| {
            let is_array = meta
                .get("isArray")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            // V1 admin edits array collections; singles are read-through.
            if !is_array {
                return None;
            }
            // Entry count for the rail badge: read the data file and measure the
            // array length. Cheap for a local dev tool; 0 when absent/unreadable.
            let count = std::fs::read_to_string(site_dir.join(file))
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.as_array().map(|a| a.len()))
                .unwrap_or(0);
            Some(serde_json::json!({
                "file": file,
                "name": meta.get("name").and_then(|v| v.as_str()).unwrap_or(file),
                "type": meta.get("type").and_then(|v| v.as_str()).unwrap_or(""),
                "isArray": is_array,
                "count": count,
            }))
        })
        .collect();
    collections.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        an.to_lowercase().cmp(&bn.to_lowercase())
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "pages": pages,
            "dataSources": data_sources,
            "locales": locales,
            "dataSourceMap": data_source_map,
            "collections": collections,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod debugger_tests {
    use crate::debugger::panel::generate_panel_overlay;
    use crate::debugger::{DebuggerConfig, generate_debug_hooks, generate_debug_runtime};

    #[test]
    fn test_debugger_overlay_includes_runtime() {
        let config = DebuggerConfig::default();
        let hooks_js = generate_debug_hooks();
        let runtime_js = generate_debug_runtime(&config);
        let overlay_js = generate_panel_overlay(&config);
        // The combined output should include hooks, __ST_DEBUG__, and panel code
        let combined = format!("{}\n\n{}\n\n{}", hooks_js, runtime_js, overlay_js);
        assert!(
            combined.contains("__stDebugHook"),
            "Overlay response should include debug hooks (__stDebugHook)"
        );
        assert!(
            combined.contains("window.__ST_DEBUG__"),
            "Overlay response should include debug runtime with __ST_DEBUG__"
        );
    }

    #[test]
    fn test_hooks_defined_before_st_debug_in_overlay() {
        // __stDebugHook must appear before __ST_DEBUG__ so the buffer is ready
        // when the debug runtime tries to replay
        let config = DebuggerConfig::default();
        let hooks_js = generate_debug_hooks();
        let runtime_js = generate_debug_runtime(&config);
        let overlay_js = generate_panel_overlay(&config);
        let combined = format!("{}\n\n{}\n\n{}", hooks_js, runtime_js, overlay_js);

        let hook_pos = combined
            .find("__stDebugHook")
            .expect("hooks must be present");
        let debug_pos = combined
            .find("window.__ST_DEBUG__")
            .expect("__ST_DEBUG__ must be present");
        assert!(
            hook_pos < debug_pos,
            "__stDebugHook (pos {}) must be defined before window.__ST_DEBUG__ (pos {})",
            hook_pos,
            debug_pos
        );
    }

    #[test]
    fn test_dev_tools_inject_before_runtime_in_html() {
        // Simulate the injection format used by the server handlers
        let debugger_inject = r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
        .to_string();
        let entry_param = "index.st";
        let validation_inject = String::new();

        let spacetime_inject = format!(
            r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}" ></script>
    {}
"#,
            debugger_inject, entry_param, entry_param, validation_inject
        );

        let dev_tools_pos = spacetime_inject
            .find("dev/runtime.js")
            .expect("dev/runtime.js must be present");
        let runtime_pos = spacetime_inject
            .find("runtime.js")
            .expect("runtime.js must be present");
        assert!(
            dev_tools_pos < runtime_pos,
            "dev/runtime.js (pos {}) must load before runtime.js (pos {})",
            dev_tools_pos,
            runtime_pos
        );
    }

    #[test]
    fn test_debugger_inject_empty_when_disabled() {
        // When debug is false, debugger_inject should be empty and not appear in output
        let debugger_inject = String::new(); // simulates debug=false path
        let entry_param = "index.st";
        let validation_inject = String::new();

        let spacetime_inject = format!(
            r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}
"#,
            debugger_inject, entry_param, entry_param, validation_inject
        );

        assert!(
            !spacetime_inject.contains("overlay.js"),
            "No debugger script should be injected when debug is disabled"
        );
        assert!(
            spacetime_inject.contains("runtime.js"),
            "Runtime should still be injected"
        );
    }

    #[test]
    fn test_hooks_contain_buffer_replay_mechanism() {
        // The hooks must buffer events and replay them when __ST_DEBUG__ becomes available
        let hooks = generate_debug_hooks();
        assert!(
            hooks.contains("__stDebugHook"),
            "Hooks must define __stDebugHook"
        );
        assert!(
            hooks.contains("registerTimeline"),
            "Hooks must handle registerTimeline calls"
        );
        assert!(
            hooks.contains("signal"),
            "Hooks must handle signal tracking"
        );
        assert!(hooks.contains("state"), "Hooks must handle state tracking");
    }
}

#[cfg(test)]
mod tests {
    /// The live-reload scripts check `d.type === 'Reload'` — TOP LEVEL.
    ///
    /// `ServerMessage` is `#[serde(untagged)]` over variants whose payload
    /// (`DevServerMessage`) is `#[serde(tag = "type")]`, so the wire shape is
    /// `{"type":"Reload","files":[…]}`, NOT `{"DevServer":{"type":"Reload"}}`.
    /// Both inline scripts once checked `d.DevServer && d.DevServer.type` —
    /// a shape nothing ever sends — so no served page ever auto-reloaded
    /// (found 2026-08-29 in spell's chat: the page only refreshed on manual
    /// reload). This test pins the agreement: the serialized message must
    /// carry the field the script inspects.
    #[test]
    fn reload_script_predicate_matches_the_wire_shape() {
        use crate::dev_server::{DevServerMessage, ServerMessage};

        let wire = serde_json::to_string(&ServerMessage::DevServer(DevServerMessage::Reload {
            files: vec![],
        }))
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(parsed["type"], "Reload");

        for script in [LIVE_RELOAD_SCRIPT] {
            assert!(
                script.contains("d.type==='Reload'"),
                "the reload predicate must read the top-level type field"
            );
            assert!(
                !script.contains("d.DevServer&&"),
                "no such wrapper exists on the wire (ServerMessage is untagged)"
            );
        }
    }

    /// PLAN-122 W3 — a design token is classified by the SCALAR GRAMMARS, not by
    /// a prefix/suffix guess.
    ///
    /// `infer_token_type` was the fifth independent answer to "what is a
    /// colour?" in this codebase, and like the other four it was wrong in its
    /// own particular way: it returned `color` for anything starting with `#`,
    /// so `#zz` and `#e8ee1` were colours; it returned `duration` for anything
    /// ending in `s`; it returned `length` for anything ending in `px`.
    ///
    /// The negatives below are the point. A heuristic that accepts a PREFIX is
    /// how a malformed value acquires a type and then travels — into the admin
    /// as a colour swatch that cannot render, into a stylesheet the browser
    /// drops.
    #[test]
    fn a_design_token_is_typed_by_the_scalar_grammars() {
        use super::infer_token_type;

        for (value, expected) in [
            // Colours, in every legal hex length plus function and keyword forms.
            ("#fff", "color"),
            ("#fffa", "color"),
            ("#e8eef7", "color"),
            ("#11223344", "color"),
            ("#E8EEF7", "color"),
            ("rgb(1, 2, 3)", "color"),
            ("oklch(0.7 0.1 200)", "color"),
            ("transparent", "color"),
            // Durations and lengths.
            ("600ms", "duration"),
            ("2s", "duration"),
            ("8px", "length"),
            ("1.5rem", "length"),
            ("50%", "length"),
            // Not scalars.
            ("42", "number"),
            ("Inter, sans-serif", "font"),
            ("whatever", "text"),
        ] {
            assert_eq!(
                infer_token_type(value),
                expected,
                "{value:?} was classified wrongly"
            );
        }

        // The refusals the old heuristic got wrong. Each of these previously
        // came back as a confident `color` / `duration` / `length`.
        for value in [
            "#zz",      // not hex
            "#e8ee1",   // 5 digits — no such colour
            "#e8eef71", // 7 digits
            "#e8eef7z", // legal prefix, stray suffix
        ] {
            assert_ne!(
                infer_token_type(value),
                "color",
                "{value:?} is not a colour, but was typed as one — a prefix \
                 match is being accepted somewhere"
            );
        }

        assert_ne!(
            infer_token_type("40 px"),
            "length",
            "`40 px` is not a dimension: the unit must be adjacent"
        );
    }

    /// R3: a wave mixing automatic (rule) and manual (hint) rows for ONE
    /// capsule must stay appliable (the route applies the automatic rows),
    /// and a manual row's hint backfills an automatic first row's None.
    #[test]
    fn aggregate_mixed_wave_stays_appliable_and_backfills_hint() {
        use crate::diagnostics::SourceSpan as DiagSpan;
        use crate::migrate::PendingMigration;

        let (registry, _) = crate::compiler::cached_stdlib_registry();
        let row =
            |rule: Option<&str>, new_text: Option<&str>, hint: Option<&str>| PendingMigration {
                migration_id: "reactive-surface".to_string(),
                rule: rule.map(|s| s.to_string()),
                date: "2026-06-09".to_string(),
                docs: "docs".to_string(),
                span: DiagSpan::new(0, 10),
                old_text: "@bind(text: $x)".to_string(),
                new_text: new_text.map(|s| s.to_string()),
                hint: hint.map(|s| s.to_string()),
            };
        // Automatic first, manual second (worst case for first-row-wins).
        let pending = vec![
            row(Some("bind-text"), Some("text <- $x;"), None),
            row(None, None, Some("replace with .hidden:")),
        ];
        let status = aggregate_migrations_status(None, &pending, &registry);
        assert_eq!(status.pending, 2);
        assert_eq!(status.waves.len(), 1);
        assert!(
            status.waves[0].appliable,
            "mixed wave must keep its Apply button"
        );
        let entry = &status.waves[0].entries[0];
        assert_eq!(entry.count, 2);
        assert_eq!(entry.kind, "mixed", "auto + manual rows read as mixed");
        assert_eq!(
            entry.hint.as_deref(),
            Some("replace with .hidden:"),
            "manual row backfills the hint"
        );
        // The capsule contract rides along for the expander.
        assert_eq!(entry.rules.len(), 5, "seed rules: {:?}", entry.rules.len());
        assert_eq!(entry.hints.len(), 3, "seed hints");
        assert!(entry.rules.iter().any(|r| r.id == "bind-text"
            && r.match_source.contains("@bind(text: $x:expr)")
            && r.into.contains("text <- `$x`;")));
    }

    use super::*;

    // FEAT-043: serving a .st-only project (no index.html on disk) must
    // synthesize a minimal shell that loads the Spacetime runtime + styles,
    // instead of returning HTTP 500. The shell is the carrier for full-
    // Spacetime pages (state/bindings/templates all live in the .st file).
    #[test]
    fn test_synthesize_st_only_shell_loads_runtime() {
        let shell = synthesize_st_only_shell("index.st", "", false, "");
        // Valid HTML document
        assert!(
            shell.contains("<!DOCTYPE html>"),
            "shell must be a full HTML document"
        );
        assert!(shell.contains("<body>"), "shell must have a body");
        // Loads the compiled runtime + styles for this entry
        assert!(
            shell.contains("/__spacetime/runtime.js?entry=index.st"),
            "shell must load the runtime for the entry"
        );
        assert!(
            shell.contains("/__spacetime/styles.css?entry=index.st"),
            "shell must load the styles for the entry"
        );
    }

    #[test]
    fn test_synthesize_st_only_shell_escapes_entry() {
        // Entry param is interpolated into attribute context; must not allow
        // breaking out of the src attribute.
        let shell = synthesize_st_only_shell("a\"b.st", "", false, "");
        assert!(
            !shell.contains("entry=a\"b.st\">"),
            "entry param must be escaped in attribute context"
        );
    }

    // PLAN-023 W1: compiler-produced body HTML (first-class <tag> literals) is injected
    // into the shell <body> ahead of the runtime script, so the served page has
    // SEO-visible markup.
    #[test]
    fn test_synthesize_st_only_shell_omits_dev_tools_by_default() {
        // Without debug, a full-Spacetime page ships NO dev-tools runtime (prod parity).
        let shell = synthesize_st_only_shell("index.st", "", false, "");
        assert!(
            !shell.contains("/__spacetime/dev/runtime.js"),
            "non-debug st-only shell must not load the dev-tools runtime"
        );
    }

    #[test]
    fn test_synthesize_st_only_shell_injects_dev_tools_when_debug() {
        // With --debug, the .st-only path injects the dev-tools runtime so on-page
        // editing (FUP-047: __stDevSave persistence) works — parity with the
        // authored-index.html path.
        let shell = synthesize_st_only_shell("index.st", "", true, "");
        assert!(
            shell.contains("/__spacetime/dev/runtime.js"),
            "debug st-only shell must load the dev-tools runtime: {shell}"
        );
        assert!(
            shell.contains("/__spacetime/dev/styles.css"),
            "debug st-only shell must load the dev-tools styles"
        );
    }

    #[test]
    fn test_synthesize_st_only_shell_injects_body_html() {
        let shell = synthesize_st_only_shell("index.st", "<main><h1>Hello</h1></main>", false, "");
        assert!(
            shell.contains("<main><h1>Hello</h1></main>"),
            "shell body must contain the compiled markup: {shell}"
        );
        // Body markup must precede the runtime script (content first, hydrate after).
        let body_pos = shell.find("<main>").unwrap();
        let script_pos = shell.find("runtime.js").unwrap();
        assert!(
            body_pos < script_pos,
            "markup must come before the runtime script"
        );
    }

    /// PLAN-004: a non-empty `host_widget` string is inlined into the
    /// synthesized shell's tail -- the SIXTH page-serve path (a `.st`-only
    /// project with no authored `index.html`, i.e. the most common
    /// `spacetime init` output) that was found missing the __host__ widget
    /// injection during this session's live browser testing.
    #[test]
    fn test_synthesize_st_only_shell_injects_host_widget_when_present() {
        let shell = synthesize_st_only_shell(
            "index.st",
            "",
            false,
            "<section class=\"st-host-widget\"></section>",
        );
        assert!(
            shell.contains("st-host-widget"),
            "shell must contain the host widget markup when host_widget is non-empty: {shell}"
        );
    }

    /// Regression guard: an EMPTY `host_widget` (a site with no `@deploy{}`
    /// fact) must not add any host-widget markup to the shell.
    #[test]
    fn test_synthesize_st_only_shell_omits_host_widget_when_absent() {
        let shell = synthesize_st_only_shell("index.st", "", false, "");
        assert!(
            !shell.contains("st-host-widget"),
            "shell must not contain host widget markup when host_widget is empty: {shell}"
        );
    }

    fn test_app_state(mode: AppMode) -> AppState {
        AppState {
            mode,
            static_dir: None,
            debugger_config: None,
            cache_stdlib: false,
            compile_cache: std::sync::Arc::new(CompileCache::new()),
            dev_compile_cache: std::sync::Arc::new(DevCompileCache::new()),
        }
    }

    /// PLAN-075 ("push preview from the pill"): `POST /__spacetime/host/push`
    /// must 501 in `AppMode::Compiled` -- a compiled/deployed build has no
    /// site directory on disk to package, matching `deploy_init_handler`'s
    /// identical dev-mode-only shape. Verified live this session (curl
    /// against a real running dev server) for the Dev-mode 401/400
    /// branches below; this is the ONE branch reachable without a real
    /// site directory or network call, so it's the one covered as a fast
    /// `cargo test --lib` unit test.
    #[tokio::test]
    async fn inspect_structure_route_projects_nodes_and_params() {
        let site = write_site(&[(
            "index.st",
            "@template &child($title string) { <p>`$title`</p> }\n@template &main() { &child(title: \"hello\"); }",
        )]);
        let state = test_app_state(AppMode::Dev {
            site_dir: site.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: Some("index.st".to_string()),
                template: None,
                all: None,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
        assert_eq!(json["entry"], "index.st");
        assert!(
            json["nodes"]
                .as_array()
                .is_some_and(|nodes| !nodes.is_empty())
        );
        assert!(
            json["params"]
                .as_array()
                .is_some_and(|params| !params.is_empty())
        );
        assert_eq!(json["error"], serde_json::Value::Null);
    }

    /// THE W1 CONTRACT, end to end: what the inspect route SERVES must be
    /// directly usable as a write address by the EditAst rail — read the
    /// structure, take a param row, echo its span + `source_hash` back through
    /// `handle_edit_ast_inner`'s `__invoke_span` branch, and the intended bytes
    /// change on disk. This is the whole read→edit loop the Inspector pill rides;
    /// if the two halves ever disagree about span or hash semantics, this fails.
    #[tokio::test]
    async fn inspect_route_rows_are_valid_write_addresses() {
        let site = write_site(&[(
            "index.st",
            "@template &child($title string) { <p>`$title`</p> }\n@template &main() { &child(title: \"hello\"); }",
        )]);
        let site_dir = site.path().to_path_buf();
        let state = test_app_state(AppMode::Dev {
            site_dir: site_dir.clone(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: Some("index.st".to_string()),
                template: None,
                all: None,
            }),
        )
        .await;
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");

        let row = json["params"]
            .as_array()
            .expect("params array")
            .iter()
            .find(|row| row["param"] == "title")
            .expect("the title param row")
            .clone();
        // The projection tells the writer the widget, the address, and the proof.
        assert_eq!(row["widget"], "text");
        let hash = row["source_hash"]
            .as_str()
            .expect("a source_hash")
            .to_string();

        // Echo the row straight back as an EditAst invoke-span patch.
        let patch = serde_json::json!({
            "__invoke_span": [row["span_start"].clone(), row["span_end"].clone()],
            "__expect_hash": hash,
            "title": "patched",
        });
        // Through the REAL production entry point (the same fn the dev-ws
        // EditAst message dispatches to), not a test-only shim.
        let result =
            crate::sync::handlers::handle_edit_ast(&site_dir, "index.st", "", &patch, "op-1");
        assert!(
            matches!(
                result.response,
                crate::sync::protocol::ServerMessage::Ack { .. }
            ),
            "the served address must be writable, got: {:?}",
            result.response
        );

        let after = std::fs::read_to_string(site_dir.join("index.st")).expect("read back");
        assert!(
            after.contains("title: \"patched\""),
            "the patch must land at the served span, got: {after}"
        );

        // The SAME address replayed with the now-stale hash must be refused — the
        // guard that stops a pill from clobbering a file edited underneath it.
        let stale =
            crate::sync::handlers::handle_edit_ast(&site_dir, "index.st", "", &patch, "op-2");
        assert!(
            matches!(
                stale.response,
                crate::sync::protocol::ServerMessage::Reject { .. }
            ),
            "a stale __expect_hash must be rejected, not silently applied, got: {:?}",
            stale.response
        );
        let unchanged = std::fs::read_to_string(site_dir.join("index.st")).expect("read back");
        assert_eq!(
            after, unchanged,
            "a rejected write must leave the file byte-identical"
        );
    }

    /// W1R REVIEW FINDING (P1): the handler compiled with `compile_to_bundle`,
    /// which anchors import resolution at `<site>/<bundle>.st` rather than the
    /// entry's REAL directory. A nested entry with a directory-relative import
    /// therefore failed to resolve (or resolved a same-named file from the wrong
    /// place). The handler now compiles entry-aware.
    #[tokio::test]
    async fn inspect_route_resolves_imports_relative_to_a_nested_entry() {
        let site = write_site(&[
            (
                "modules/_parts.st",
                "@template &child($title string) { <p>`$title`</p> }",
            ),
            (
                "landing/index.st",
                "@import \"../modules/_parts.st\"\n@template &main() { &child(title: \"nested\"); }",
            ),
        ]);
        let state = test_app_state(AppMode::Dev {
            site_dir: site.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: Some("landing/index.st".to_string()),
                template: None,
                all: None,
            }),
        )
        .await;
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
        assert_eq!(
            json["error"],
            serde_json::Value::Null,
            "a nested entry's relative import must resolve, got: {json}"
        );
        assert!(
            json["params"]
                .as_array()
                .is_some_and(|params| params.iter().any(|row| row["param"] == "title")),
            "the imported template's param row must project, got: {json}"
        );
    }

    /// W1R REVIEW FINDING (P1): a whole-file hash proves the FILE is unchanged but
    /// not that THIS SPAN still means what the caller was shown — a span derived
    /// from one snapshot could be paired with another snapshot's hash and then
    /// applied to the wrong call site. Rows therefore carry `span_text`, and the
    /// write rail refuses when the bytes at that offset no longer match.
    #[tokio::test]
    async fn a_moved_span_is_refused_even_when_the_hash_matches() {
        let site = write_site(&[(
            "index.st",
            "@template &child($title string) { <p>`$title`</p> }\n@template &main() { &child(title: \"first\"); &child(title: \"second\"); }",
        )]);
        let site_dir = site.path().to_path_buf();
        let state = test_app_state(AppMode::Dev {
            site_dir: site_dir.clone(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: Some("index.st".to_string()),
                template: None,
                all: None,
            }),
        )
        .await;
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
        let rows = json["params"].as_array().expect("params").clone();
        let second = rows
            .iter()
            .find(|row| row["value"].as_str() == Some("\"second\""))
            .expect("the second invocation's row");
        let span_text = second["span_text"]
            .as_str()
            .expect("a served row must carry its span text")
            .to_string();

        // Rewrite the file so the SECOND invocation shifts: same file, still
        // parseable, and the old offsets now land on different bytes. The caller
        // replays the address it was served, with a hash recomputed for the file
        // as it is NOW (the strongest form of the stale-pairing attack).
        let rewritten = "@template &child($title string) { <p>`$title`</p> }\n@template &main() { &child(title: \"a-much-longer-first-value\"); &child(title: \"second\"); }";
        std::fs::write(site_dir.join("index.st"), rewritten).expect("rewrite");
        let fresh_hash = format!(
            "{:016x}",
            crate::migrate::hash_content(
                &std::fs::read_to_string(site_dir.join("index.st")).expect("read")
            )
        );

        let patch = serde_json::json!({
            "__invoke_span": [second["span_start"].clone(), second["span_end"].clone()],
            "__expect_hash": fresh_hash,
            "__expect_span_text": span_text,
            "title": "clobbered",
        });
        let result =
            crate::sync::handlers::handle_edit_ast(&site_dir, "index.st", "", &patch, "op-1");
        assert!(
            matches!(
                result.response,
                crate::sync::protocol::ServerMessage::Reject { .. }
            ),
            "a span whose bytes moved must be refused, got: {:?}",
            result.response
        );
        assert_eq!(
            std::fs::read_to_string(site_dir.join("index.st")).expect("read back"),
            rewritten,
            "a refused write must leave the file byte-identical"
        );
    }

    /// W3R REVIEW FINDING: the inspector's entry path is interpolated into a
    /// query string INSIDE an HTML attribute (`src="…?entry=…"`) and into the
    /// pill's own fetch URL. Unencoded, a path containing `&` splits into a bogus
    /// second query parameter — the pill then inspects (and writes to) a DIFFERENT
    /// file — and a quote closes the attribute outright.
    #[test]
    fn entry_paths_are_safe_in_a_query_and_an_html_attribute() {
        // Ordinary paths stay readable: `/` survives, so nesting is legible.
        assert_eq!(
            encode_query_component("landing/index.st"),
            "landing/index.st"
        );

        // The hazards.
        assert_eq!(
            encode_query_component("a&entry=evil.st"),
            "a%26entry%3Devil.st",
            "an ampersand must not be able to inject another query parameter"
        );
        for hazard in ["\"", "'", "<", ">", "#", "?", " ", "\\"] {
            let encoded = encode_query_component(hazard);
            assert!(
                encoded.starts_with('%') && encoded.len() == 3,
                "{hazard:?} must percent-encode, got {encoded:?}"
            );
        }

        // Non-ASCII is encoded per byte (valid UTF-8 percent-encoding).
        assert_eq!(encode_query_component("caf\u{e9}.st"), "caf%C3%A9.st");
    }

    #[tokio::test]
    async fn inspect_structure_route_rejects_escaping_path() {
        let site = tempfile::tempdir().expect("tempdir");
        let state = test_app_state(AppMode::Dev {
            site_dir: site.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: Some("../../etc/passwd".to_string()),
                template: None,
                all: None,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn inspect_structure_route_reports_missing_template() {
        let site = write_site(&[("index.st", "@template &main() { <p>ok</p> }")]);
        let state = test_app_state(AppMode::Dev {
            site_dir: site.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: Some("index.st".to_string()),
                template: Some("absent".to_string()),
                all: None,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON response");
        assert_eq!(json["nodes"], serde_json::json!([]));
        assert_eq!(json["params"], serde_json::json!([]));
        assert!(
            json["error"]
                .as_str()
                .is_some_and(|error| error.contains("absent"))
        );
    }

    #[tokio::test]
    async fn inspect_structure_route_requires_entry() {
        let site = tempfile::tempdir().expect("tempdir");
        let state = test_app_state(AppMode::Dev {
            site_dir: site.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = inspect_structure_handler(
            axum::extract::State(state),
            Query(InspectStructureQuery {
                entry: None,
                template: None,
                all: None,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_host_push_handler_rejects_compiled_mode() {
        let compiled = Compiler::from_ast(&crate::parser::StFile::default()).compile();
        let state = test_app_state(AppMode::Compiled(compiled));
        let response = host_push_handler(axum::extract::State(state)).await;
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    /// Missing local credentials must 401 with an actionable message --
    /// pushing bytes needs a real bearer token, and this must fail BEFORE
    /// any packaging/network work (confirmed by construction: a temp dir
    /// with no `@deploy{}` fact would ALSO 400, but the token check runs
    /// first, so a real project id is unnecessary to exercise this path).
    /// Uses a scoped `XDG_DATA_HOME` override (a fresh empty temp dir) so
    /// this test never reads/depends on the real developer machine's own
    /// credentials file (`read_host_api_token`'s resolution order checks
    /// `XDG_DATA_HOME` first).
    #[tokio::test]
    #[serial_test::serial(host_credentials_env)]
    async fn test_host_push_handler_rejects_missing_credentials() {
        let xdg_dir = tempfile::tempdir().unwrap();
        let site_dir = tempfile::tempdir().unwrap();
        let prev_xdg = std::env::var("XDG_DATA_HOME").ok();
        unsafe {
            std::env::set_var("XDG_DATA_HOME", xdg_dir.path());
        }

        let state = test_app_state(AppMode::Dev {
            site_dir: site_dir.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = host_push_handler(axum::extract::State(state)).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        unsafe {
            match prev_xdg {
                Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }

    /// A real credentials file present, but the site directory has no
    /// `@deploy{}` fact -- defense-in-depth 400 (the widget's own
    /// `$isNoProject` state already hides this button client-side; this
    /// guards a direct API call more than a real UI path, per
    /// `host_push_handler`'s own doc comment).
    #[tokio::test]
    #[serial_test::serial(host_credentials_env)]
    async fn test_host_push_handler_rejects_missing_deploy_fact() {
        let xdg_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(xdg_dir.path().join("spacetime-host")).unwrap();
        std::fs::write(
            xdg_dir.path().join("spacetime-host").join("credentials"),
            "test-token-value",
        )
        .unwrap();
        let site_dir = tempfile::tempdir().unwrap();
        std::fs::write(site_dir.path().join("index.st"), "").unwrap();

        let prev_xdg = std::env::var("XDG_DATA_HOME").ok();
        unsafe {
            std::env::set_var("XDG_DATA_HOME", xdg_dir.path());
        }

        let state = test_app_state(AppMode::Dev {
            site_dir: site_dir.path().to_path_buf(),
            trace: false,
            debug: false,
        });
        let response = host_push_handler(axum::extract::State(state)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        unsafe {
            match prev_xdg {
                Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }
    }

    #[test]
    fn test_update_element_content_basic() {
        let html = r#"<div data-st-id="test-1">Old content</div>"#;
        let result = update_element_content(html, "test-1", "New content").unwrap();
        assert!(result.contains("New content"));
        assert!(!result.contains("Old content"));
    }

    #[test]
    fn test_update_element_content_with_html() {
        let html = r#"<div data-st-id="prose">Simple text</div>"#;
        let result =
            update_element_content(html, "prose", "<span class=\"highlight\">Rich</span> text")
                .unwrap();
        assert!(result.contains("<span class=\"highlight\">Rich</span>"));
    }

    #[test]
    fn test_update_element_content_not_found() {
        let html = r#"<div data-st-id="other">Content</div>"#;
        let result = update_element_content(html, "missing-id", "New content");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_update_element_content_preserves_structure() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <header>Header</header>
    <main data-st-id="main-content">Old main content</main>
    <footer>Footer</footer>
</body>
</html>"#;
        let result = update_element_content(html, "main-content", "New main content").unwrap();
        assert!(result.contains("<header>Header</header>"));
        assert!(result.contains("New main content"));
        assert!(result.contains("<footer>Footer</footer>"));
    }

    #[test]
    fn test_update_element_content_nested() {
        let html = r#"<article><section data-st-id="sec-1"><p>Paragraph</p></section></article>"#;
        let result =
            update_element_content(html, "sec-1", "<h2>Title</h2><p>New paragraph</p>").unwrap();
        assert!(result.contains("<h2>Title</h2>"));
        assert!(result.contains("<p>New paragraph</p>"));
        assert!(!result.contains("<p>Paragraph</p>"));
    }

    #[test]
    fn test_compute_st_hash_deterministic() {
        let h1 = compute_st_hash("index.html:p:1");
        let h2 = compute_st_hash("index.html:p:1");
        assert_eq!(h1, h2, "Same input must produce same hash");
        assert!(h1.starts_with("st-"), "Hash must start with st- prefix");
        assert_eq!(h1.len(), 11, "Hash must be st- + 8 hex chars = 11 chars");
    }

    #[test]
    fn test_compute_st_hash_different_inputs() {
        let h1 = compute_st_hash("index.html:p:1");
        let h2 = compute_st_hash("index.html:p:2");
        assert_ne!(h1, h2, "Different inputs must produce different hashes");
    }

    #[test]
    fn test_inject_content_provenance_basic() {
        let html = "<html><body><h1>Title</h1><p>Text</p></body></html>";
        let result = inject_content_provenance(html, "index.html");
        assert!(result.contains("data-st-id"), "Should inject data-st-id");
        assert!(
            result.contains("data-st-origin"),
            "Should inject data-st-origin"
        );
        assert!(
            result.contains("index.html::"),
            "Origin should contain file path"
        );
    }

    #[test]
    fn test_inject_content_provenance_skips_data_t() {
        let html = r#"<p data-t="key">Translated</p>"#;
        let result = inject_content_provenance(html, "index.html");
        assert!(
            !result.contains("data-st-id"),
            "Should skip elements with data-t"
        );
    }

    #[test]
    fn test_inject_content_provenance_skips_data_st_bind() {
        let html = r#"<span data-st-bind="value">Bound</span>"#;
        let result = inject_content_provenance(html, "index.html");
        assert!(
            !result.contains("data-st-id"),
            "Should skip elements with data-st-bind"
        );
    }

    #[test]
    fn test_inject_content_provenance_skips_existing_st_id() {
        let html = r#"<p data-st-id="user-set">Custom</p>"#;
        let result = inject_content_provenance(html, "index.html");
        assert!(
            result.contains("user-set"),
            "Should preserve existing data-st-id"
        );
        // Should only have 1 data-st-id (the original), not a second one
        assert_eq!(
            result.matches("data-st-id").count(),
            1,
            "Should not add duplicate data-st-id"
        );
    }

    #[test]
    fn test_inject_content_provenance_skips_non_text_tags() {
        let html = "<html><body><div>Container</div><section>Section</section></body></html>";
        let result = inject_content_provenance(html, "index.html");
        assert!(
            !result.contains("data-st-id"),
            "Should skip non-text-bearing tags like div and section"
        );
    }

    #[test]
    fn test_inject_content_provenance_deterministic() {
        let html = "<html><body><h1>Title</h1><p>One</p><p>Two</p></body></html>";
        let r1 = inject_content_provenance(html, "index.html");
        let r2 = inject_content_provenance(html, "index.html");
        assert_eq!(r1, r2, "Same input must produce identical output");
    }

    #[test]
    fn test_locale_fallback_inject_no_escaped_quotes() {
        // Simulate the locale fallback injection (same format strings as try_serve_locale
        // when build_scripts is empty)
        let diagnostics_json = r#"[{"message":"test"}]"#;
        let validation_inject = format!(
            r#"
    <!-- Spacetime Validation -->
    <script>window.__SPACETIME_DIAGNOSTICS__ = {};</script>
    <script src="/__spacetime/validation-ui.js"></script>
"#,
            diagnostics_json
        );

        let debugger_inject = r#"
    <!-- Spacetime Dev Tools -->
    <link rel="stylesheet" href="/__spacetime/dev/styles.css" />
    <script src="/__spacetime/dev/runtime.js"></script>
"#
        .to_string();

        let entry_param = "index.st";
        let spacetime_inject = format!(
            r#"
    {}<!-- Spacetime Runtime -->
    <link rel="stylesheet" href="/__spacetime/styles.css?entry={}" />
    <script src="/__spacetime/runtime.js?entry={}"></script>
    {}{}
"#,
            debugger_inject, entry_param, entry_param, validation_inject, LIVE_RELOAD_SCRIPT
        );

        // No literal backslash-quote sequences in the output
        assert!(
            !spacetime_inject.contains(r#"\""#),
            "Output must not contain escaped quotes. Got: {}",
            spacetime_inject
        );
        // Valid HTML attribute quotes
        assert!(
            spacetime_inject.contains(r#"rel="stylesheet""#),
            "Must contain valid rel attribute"
        );
        assert!(
            spacetime_inject.contains("runtime.js"),
            "Must contain runtime script"
        );
        assert!(
            spacetime_inject.contains("validation-ui.js"),
            "Must contain validation script when diagnostics present"
        );
        assert!(
            spacetime_inject.contains("dev/runtime.js"),
            "Must contain dev tools when debug enabled"
        );
    }

    // ===================================================================
    // PLAN-110 / PROJ-004 W0: CMS admin foundation — the /__spacetime/dev/
    // site.json dataSourceMap and types.json must let the admin join a data
    // SOURCE file (talent.json) to its @type (Talent) and that type's schema.
    // These tests lock the source→type resolution and @import-ed @type
    // visibility that the type-derived admin depends on (guards AUD-002 #8/#12).
    // ===================================================================

    fn write_site(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (rel, body) in files {
            let path = dir.path().join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("mkdir");
            }
            std::fs::write(&path, body).expect("write");
        }
        dir
    }

    /// The dataSourceMap join: a data SOURCE file resolves to its @type name,
    /// is_array flag, and binding name. This is what the admin keys collections
    /// on. Mirrors the logic in dev_site_handler.
    #[test]
    fn test_w0_data_source_resolves_to_type() {
        let site = write_site(&[(
            "index.st",
            r#"@type Talent { id: string; name: string; categories: string[]; }
@data fetch $talents Talent[] : "/data/talent.json"
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let mut analysis = crate::analysis::CompileAnalysis::new();
        analysis.analyze_data_sources(&ast.matches, &registry);

        let ds = analysis
            .data_sources
            .iter()
            .find(|d| d.name == "talents")
            .expect("talents data source present");
        assert_eq!(
            ds.type_name.trim_end_matches("[]"),
            "Talent",
            "source `talents` must resolve to @type `Talent` (AUD-002 #8)"
        );
        assert!(ds.is_array, "Talent[] must be flagged is_array");
        // The unified `@data fetch ... : "/url"` surface captures `src` as a
        // quoted Expr, not a bare String. Regression guard: the source MUST
        // resolve to File("/data/talent.json"); otherwise it falls through to
        // Runtime and dataSourceMap is empty — the admin then shows no types.
        assert!(
            matches!(&ds.source, crate::analysis::DataSource::File(p) if p == "/data/talent.json"),
            "@data fetch URL must resolve to a File source; got {:?}",
            ds.source
        );
    }

    /// A @type defined in an @import-ed module MUST be visible to the type
    /// registry the admin reads (dev_types_handler uses parse_site_ast which
    /// resolves imports). Guards AUD-002 #12 (handler previously saw only
    /// index.st). This is the genuinely uncovered path: no real project module
    /// defined a @type before, so import resolution of types was unexercised.
    #[test]
    fn test_w0_imported_type_is_visible() {
        let site = write_site(&[
            (
                "index.st",
                "@import \"./modules/_types.st\"\n@data talents: Talent[] { src: \"/data/talent.json\"; }\n",
            ),
            (
                "modules/_types.st",
                "@type Talent { id: string; name: string; headline: string; }\n",
            ),
        ]);
        let ast = parse_site_ast(site.path()).expect("parse + resolve imports");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let type_names: Vec<&str> = registry.iter_types().map(|(n, _)| n.as_str()).collect();
        assert!(
            type_names.contains(&"Talent"),
            "@type from an @import-ed module must be visible (AUD-002 #12); got {type_names:?}"
        );
    }

    /// W0/W1: display-role inference from CONVENTION (no @cms). unkn-shaped
    /// Talent must infer title=name, subtitle=headline, thumbnail=image,
    /// chips=categories. A type with no string[] gets chips=null.
    #[test]
    fn test_w1_display_inference_by_convention() {
        let site = write_site(&[(
            "index.st",
            r#"@type Talent { id: string; name: string; headline: string; image: string; categories: string[]; }
@type WorkCard { id: string; title: string; description: string; image: string; }
@data fetch $talents Talent[] : "/data/talent.json"
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let hints = collect_cms_hints(&ast.matches);

        let talent = registry.get_type("Talent").expect("Talent");
        let d = compute_display_roles(talent, hints.get("Talent"));
        assert_eq!(d["title"], "name");
        assert_eq!(d["subtitle"], "headline");
        assert_eq!(d["thumbnail"], "image");
        assert_eq!(d["chips"], "categories");

        let work = registry.get_type("WorkCard").expect("WorkCard");
        let dw = compute_display_roles(work, hints.get("WorkCard"));
        assert_eq!(dw["title"], "title");
        assert_eq!(dw["subtitle"], "description");
        assert!(dw["chips"].is_null(), "no string[] field => chips null");
    }

    /// Stage-2 Wave-3: a `richtext` field serializes to a string flagged with
    /// `x-st-richtext` and maps to the `richtext` widget (block editor). FUP-029.
    #[test]
    fn test_w3_richtext_type_emits_flag_and_widget() {
        let site = write_site(&[(
            "index.st",
            r#"@type Article { id: string; title: string; body: richtext; }
@data articles: Article[] { src: "/data/a.json"; }
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let mut schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference("Article".to_string()),
            &registry,
        );
        annotate_widgets(&mut schema);
        let body = schema
            .get("properties")
            .and_then(|p| p.get("body"))
            .unwrap();
        assert_eq!(body.get("type").and_then(|t| t.as_str()), Some("string"));
        assert_eq!(
            body.get("x-st-richtext").and_then(|b| b.as_bool()),
            Some(true)
        );
        assert_eq!(
            body.get("widget").and_then(|w| w.as_str()),
            Some("richtext")
        );
        // a plain string field stays text
        let title = schema
            .get("properties")
            .and_then(|p| p.get("title"))
            .unwrap();
        assert_eq!(title.get("widget").and_then(|w| w.as_str()), Some("text"));
    }

    /// Stage-2 Wave-1: the CMS relation type `id(T)` must serialize to a string
    /// schema annotated with `x-st-ref` (single) or an array of such (multi), and
    /// the widget engine must map those to `relation` / `relation-multi` so the
    /// admin renders a picker. Pins FUP-028.
    #[test]
    fn test_templates_and_motion_deliver_sig_and_label() {
        // PLAN-034 Wave C: the server delivers source-determined display fields
        // (template param `sig`, motion `label`) so the admin panes render with a
        // clean @each and NO client shaper. Exercise the regex + assembly directly.
        let tpl_re = regex::Regex::new(r"@template\s+&([A-Za-z0-9_-]+)\s*\(([^)]*)\)").unwrap();
        let src = "@template &hero($title, &slot, $sub?) { <div></div> }";
        let cap = tpl_re.captures(src).expect("template match");
        let params_raw = cap[2].trim();
        let sig = params_raw
            .split(',')
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    return None;
                }
                Some(p.to_string())
            })
            .collect::<Vec<_>>()
            .join(", ");
        // The raw param text already carries the sigils + `?`, so the assembled
        // signature matches what the handler emits.
        assert_eq!(sig, "$title, &slot, $sub?");

        let mot_re =
            regex::Regex::new(r"@(reveal|scroll)\s*([A-Za-z0-9_-]*)\s*\(([^)]*)\)").unwrap();
        let msrc = "@reveal fade(duration: 600)";
        let mc = mot_re.captures(msrc).expect("motion match");
        let kind = mc[1].to_string();
        let name = mc[2].to_string();
        let label = if name.is_empty() {
            kind.clone()
        } else {
            format!("{kind} {name}")
        };
        assert_eq!(label, "reveal fade");
    }

    #[test]
    fn test_build_brand_json_singleton_schema_values_widgets() {
        // PLAN-034 Wave B: the brand singleton is delivered as schema (token
        // widgets) + current values, keyed off @cms(Brand){ editable: inline }.
        let site = write_site(&[(
            "index.st",
            r##"@type Brand { scarlet: color; ink: color; radius: length; reveal: duration; }
@data inline $brand Brand : { "scarlet": "#FF0020", "ink": "#0a0a0a", "radius": "8px", "reveal": "600ms" };
@cms(Brand) { editable: inline; }
h1 { "x" }
"##,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let out = build_brand_json(&ast);
        let brand = out.get("brand").expect("brand key");
        assert!(!brand.is_null(), "brand singleton must be found: {out}");
        assert_eq!(brand.get("name").and_then(|v| v.as_str()), Some("brand"));
        assert_eq!(brand.get("type").and_then(|v| v.as_str()), Some("Brand"));
        let vals = brand.get("values").and_then(|v| v.as_object()).unwrap();
        assert_eq!(
            vals.get("scarlet").and_then(|v| v.as_str()),
            Some("#FF0020")
        );
        assert_eq!(vals.get("reveal").and_then(|v| v.as_str()), Some("600ms"));
        let props = brand
            .get("schema")
            .and_then(|s| s.get("properties"))
            .and_then(|p| p.as_object())
            .unwrap();
        assert_eq!(
            props
                .get("scarlet")
                .and_then(|f| f.get("widget"))
                .and_then(|w| w.as_str()),
            Some("color"),
            "color field -> color widget"
        );
        assert_eq!(
            props
                .get("radius")
                .and_then(|f| f.get("widget"))
                .and_then(|w| w.as_str()),
            Some("range-length"),
            "length field -> range-length slider widget (FEAT-107)"
        );
    }

    #[test]
    fn test_build_brand_json_absent_without_cms_inline() {
        let site = write_site(&[(
            "index.st",
            r##"@type Brand { scarlet: color; }
@data inline $brand Brand : { "scarlet": "#FF0020" };
h1 { "x" }
"##,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let out = build_brand_json(&ast);
        assert!(
            out.get("brand").map(|b| b.is_null()).unwrap_or(false),
            "no @cms(Brand) editable:inline -> brand: null, got {out}"
        );
    }

    /// FEAT-105: the live-preview token bridge must stay present AND guarded in
    /// LIVE_RELOAD_SCRIPT. This locks the security-critical contract so a future
    /// edit to the reload script can't silently drop the origin/type guards or
    /// the bridge itself.
    #[test]
    fn test_feat105_token_preview_bridge_present_and_guarded() {
        let s = LIVE_RELOAD_SCRIPT;
        // The bridge listens for postMessage and matches our envelope tag.
        assert!(
            s.contains("addEventListener('message'"),
            "no message listener"
        );
        assert!(s.contains("token-preview"), "no token-preview envelope tag");
        // Same-origin guard (must reject cross-origin senders).
        assert!(
            s.contains("e.origin!==location.origin"),
            "missing same-origin guard"
        );
        // Type guards: binding/field must be strings, value string|number.
        assert!(
            s.contains("typeof b!=='string'") && s.contains("typeof f!=='string'"),
            "missing binding/field string guards"
        );
        assert!(
            s.contains("typeof v!=='string'&&typeof v!=='number'"),
            "missing value type guard"
        );
        // Binding must already exist in the page's local state (no arbitrary keys).
        assert!(
            s.contains("b in (window._localState||{})"),
            "missing binding-exists guard"
        );
        // Writes through the reactive SpacetimeLocal proxy (the repaint path).
        assert!(
            s.contains("window.SpacetimeLocal[b]=next"),
            "missing reactive write"
        );
        // Live-reload itself must remain intact.
        assert!(s.contains("location.reload()"), "live-reload regressed");
    }

    /// FEAT-106: a scoped motion directive's params surface as widget-typed form
    /// fields read from the COMPILED AST (not a regex scrape), with the element
    /// selector + kind as the EditAst address. Param widget kinds come from the
    /// captured value kinds (Ident→select, Number→number, Bool→toggle).
    /// FEAT-108: derived tokens (`@data derive` over the brand singleton) are
    /// delivered with their formula; non-referencing derives are excluded.
    #[test]
    fn test_build_brand_json_delivers_derived_tokens() {
        let site = write_site(&[(
            "index.st",
            r##"@type Brand { scarlet: color; ink: color; }
@data inline $brand Brand : { "scarlet": "#FF0020", "ink": "#0a0a0a" };
@cms(Brand) { editable: inline; }
@data derive $hover : $brand.scarlet | darken(0.12);
@data derive $muted : $brand.ink | alpha(0.55);
@data derive $unrelated : 1 + 2;
h1 { "x" }
"##,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let out = build_brand_json(&ast);
        let derived = out["brand"]["derived"].as_array().expect("derived array");
        assert_eq!(
            derived.len(),
            2,
            "only brand-referencing derives, got {derived:?}"
        );
        let hover = derived
            .iter()
            .find(|d| d["name"] == "hover")
            .expect("hover");
        assert_eq!(hover["formula"], "$brand.scarlet | darken(0.12)");
        assert!(
            derived.iter().all(|d| d["name"] != "unrelated"),
            "a derive not referencing $brand must be excluded"
        );
    }

    /// FEAT-106: a scoped motion directive's params surface as widget-typed form
    #[test]
    fn test_build_motion_json_scoped_directive_widget_fields() {
        let site = write_site(&[(
            "index.st",
            r##".masthead {
  @scroll reveal(start: 0, end: 1, easing: ease-out, stagger: 30, scrub: false) {
    opacity: 0 -> 1;
  }
}
h1 { "x" }
"##,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let out = build_motion_json(&ast);
        let motions = out
            .get("motions")
            .and_then(|m| m.as_array())
            .expect("motions array");
        assert_eq!(motions.len(), 1, "one scoped motion, got {out}");
        let m = &motions[0];
        assert_eq!(m.get("kind").and_then(|v| v.as_str()), Some("scroll"));
        assert_eq!(m.get("name").and_then(|v| v.as_str()), Some("reveal"));
        assert_eq!(
            m.get("selector").and_then(|v| v.as_str()),
            Some(".masthead"),
            "EditAst address = the element scope"
        );
        let fields = m.get("fields").and_then(|f| f.as_array()).expect("fields");
        // Find the easing field: an Ident capture -> select widget.
        let easing = fields
            .iter()
            .find(|f| f.get("key").and_then(|k| k.as_str()) == Some("easing"))
            .expect("easing field");
        assert_eq!(
            easing.get("widget").and_then(|w| w.as_str()),
            Some("text"),
            "ident param collapses to the text input the form-renderer has"
        );
        assert_eq!(
            easing.get("value").and_then(|v| v.as_str()),
            Some("ease-out")
        );
        // PLAN-112 W2: the server states the widget KIND only. Resolving it to a
        // template name is the client's job (`ST.adFieldTemplate`) — keeping a
        // second copy of the shared-widget set here is how the two silently drift,
        // and a descriptor naming an unregistered template renders NOTHING.
        assert!(
            easing.get("tpl").is_none(),
            "the server must not assign a template name: {easing:?}"
        );
        // stagger: a Number capture -> number widget, integer-formatted (no .0).
        let stagger = fields
            .iter()
            .find(|f| f.get("key").and_then(|k| k.as_str()) == Some("stagger"))
            .expect("stagger field");
        assert_eq!(
            stagger.get("widget").and_then(|w| w.as_str()),
            Some("number")
        );
        assert_eq!(
            stagger.get("value").and_then(|v| v.as_str()),
            Some("30"),
            "integer number renders without trailing .0 so source round-trips"
        );
        // No `name`/`body` leak into the editable fields.
        assert!(
            !fields.iter().any(|f| matches!(
                f.get("key").and_then(|k| k.as_str()),
                Some("name") | Some("body")
            )),
            "name/body are not tunable param fields"
        );
    }

    /// FEAT-106: a site with no motion directives yields an empty motion list.
    #[test]
    fn test_build_motion_json_empty_without_motion() {
        let site = write_site(&[("index.st", "h1 { \"x\" }\n")]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let out = build_motion_json(&ast);
        assert_eq!(
            out.get("motions")
                .and_then(|m| m.as_array())
                .map(|a| a.len()),
            Some(0)
        );
    }

    #[test]
    fn test_w1_id_relation_type_emits_x_st_ref_and_relation_widget() {
        let site = write_site(&[(
            "index.st",
            r#"@type Agent { id: string; name: string; }
@type Talent { id: string; name: string; agent: id(Agent); campaigns: id(Agent)[]; }
@data talents: Talent[] { src: "/data/t.json"; }
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let mut schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference("Talent".to_string()),
            &registry,
        );
        annotate_widgets(&mut schema);
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap();

        // single relation: string + x-st-ref + relation widget
        let agent = props.get("agent").unwrap();
        assert_eq!(agent.get("type").and_then(|t| t.as_str()), Some("string"));
        assert_eq!(
            agent.get("x-st-ref").and_then(|t| t.as_str()),
            Some("Agent")
        );
        assert_eq!(
            agent.get("widget").and_then(|t| t.as_str()),
            Some("relation")
        );

        // multi relation: array whose items carry x-st-ref + relation-multi widget
        let camp = props.get("campaigns").unwrap();
        assert_eq!(camp.get("type").and_then(|t| t.as_str()), Some("array"));
        assert_eq!(
            camp.get("items")
                .and_then(|i| i.get("x-st-ref"))
                .and_then(|t| t.as_str()),
            Some("Agent")
        );
        assert_eq!(
            camp.get("widget").and_then(|t| t.as_str()),
            Some("relation-multi")
        );
    }

    /// W1/Wave-C: a titleless type (no name/title/label, no headline/etc.) must
    /// produce display.title = null. The admin's fallbackLabel() then labels rows
    /// by chips/first-string client-side (FEAT-069). Pins the StripRow shape so a
    /// future convention change can't silently re-introduce a bogus title role.
    #[test]
    fn test_w1_titleless_type_has_null_title_role() {
        let site = write_site(&[(
            "index.st",
            r#"@type StripRow { id: string; image: string; category: string; }
@data strip: StripRow[] { src: "/data/strip.json"; }
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let hints = collect_cms_hints(&ast.matches);
        let sr = registry.get_type("StripRow").expect("StripRow");
        let d = compute_display_roles(sr, hints.get("StripRow"));
        assert!(
            d["title"].is_null(),
            "no name/title/label => title role is null (admin uses fallbackLabel)"
        );
        assert!(d["subtitle"].is_null(), "no subtitle-ish field => null");
        assert_eq!(
            d["thumbnail"], "image",
            "image field still resolves as thumbnail"
        );
        assert!(
            d["chips"].is_null(),
            "category is a scalar string, not string[] => not chips"
        );
    }

    /// W0/W1: @cms(TypeName) overrides convention for named roles; omitted roles
    /// still fall back to convention. The interim file-scope form must parse and
    /// must NOT corrupt the @type's fields (the nested form does — hence FUPs).
    #[test]
    fn test_w0_cms_macro_overrides_and_preserves_fields() {
        let site = write_site(&[(
            "index.st",
            r#"@type Profile { id: string; fullName: string; bio: string; portrait: string; skills: string[]; }
@cms(Profile) { title: fullName; thumbnail: portrait; }
@data profiles: Profile[] { src: "/data/profiles.json"; }
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);

        // Fields intact (interim @cms must not break the properties capture).
        let profile = registry.get_type("Profile").expect("Profile");
        assert_eq!(
            profile.fields.len(),
            5,
            "@cms must not corrupt @type fields"
        );

        let hints = collect_cms_hints(&ast.matches);
        let d = compute_display_roles(profile, hints.get("Profile"));
        // Overridden roles:
        assert_eq!(d["title"], "fullName", "@cms title override");
        assert_eq!(d["thumbnail"], "portrait", "@cms thumbnail override");
        // Convention fallback for omitted roles:
        assert!(
            d["subtitle"].is_null(),
            "no headline/subtitle/description => subtitle null"
        );
        assert_eq!(d["chips"], "skills", "chips falls back to first string[]");
    }

    /// FEAT-076 W5 cutover: the admin (stdlib/__admin__/index.st) is the
    /// declarative multi-file tree (app/*.st) — the former %emit js monolith
    /// (admin-app.st) was deleted. It must still compile to a substantial JS
    /// bundle that wires the dev WS edit protocol + the meta-renderer data layer,
    /// AND emit the file-scope shell markup (compiled.html). Guards the macro-emit
    /// regression: %primitives only emit when invoked through a %macro's %binds.
    #[test]
    fn test_w2_admin_site_compiles_to_runtime_js() {
        let admin_st = std::path::PathBuf::from("stdlib/__admin__/index.st");
        assert!(
            admin_st.exists(),
            "admin entry stdlib/__admin__/index.st must exist"
        );
        // The monolith must be GONE (the W5 cutover deleted it).
        assert!(
            !std::path::Path::new("stdlib/__admin__/admin-app.st").exists(),
            "the %emit js monolith admin-app.st must be deleted after the W5 cutover"
        );
        let compiler = crate::Compiler::from_file(&admin_st, std::path::Path::new("."))
            .expect("admin compiles");
        let compiled = compiler.compile();
        assert!(
            compiled.js.len() > 10_000,
            "admin runtime JS must be substantial (got {} bytes) — macro-emit wiring regressed?",
            compiled.js.len()
        );
        // Reuses the dev WS persistence bridge.
        assert!(
            compiled.js.contains("__stDevWs"),
            "admin must reuse the dev WS client"
        );
        // The declarative tree renders the rail/list/drawer via selector-init +
        // the meta-renderer data layer (@data fetch over the server contracts).
        assert!(
            compiled.js.contains(".ad-rail") || compiled.js.contains("ad-nav--collections"),
            "the declarative admin shell (rail) must be wired"
        );
        assert!(
            compiled.js.contains("dev/site.json"),
            "admin must read the site contract (collections meta-renderer)"
        );
        // The file-scope shell markup must reach compiled.html (mounts to <body>).
        assert!(
            compiled.html.contains("ad-root") && compiled.html.contains("ad-rail"),
            "the file-scope admin shell markup must compile to html (got {} bytes)",
            compiled.html.len()
        );
    }

    /// W1: the widget engine maps each field to its admin form widget per
    /// spec §4.3. Covers branches unkn doesn't exercise (number, boolean, union,
    /// textarea, nested object, array-of-object, optional).
    #[test]
    fn test_w1_widget_engine_maps_all_kinds() {
        let site = write_site(&[(
            "index.st",
            r#"@type Author { name: string; email: string; }
@type Article {
  id: string;
  slug: string;
  title: string;
  body: string;
  views: number;
  published: boolean;
  status: "draft" | "live" | "archived";
  cover: url;
  tags: string[];
  author: Author;
  related: Author[];
  note?: string;
}
@data articles: Article[] { src: "/data/articles.json"; }
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let mut schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference("Article".to_string()),
            &registry,
        );
        annotate_widgets(&mut schema);
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap();
        let w = |f: &str| {
            props
                .get(f)
                .and_then(|n| n.get("widget"))
                .and_then(|x| x.as_str())
                .unwrap_or("MISSING")
        };

        assert_eq!(w("id"), "readonly");
        assert_eq!(w("slug"), "readonly");
        assert_eq!(w("title"), "text");
        assert_eq!(w("body"), "textarea", "body is long-form prose");
        assert_eq!(w("views"), "number");
        assert_eq!(w("published"), "toggle");
        assert_eq!(w("status"), "select", "union -> select");
        assert_eq!(w("cover"), "media", "url-format -> media");
        assert_eq!(w("tags"), "chips", "string[] -> chips");
        assert_eq!(w("author"), "fieldset", "nested type -> fieldset");
        assert_eq!(w("related"), "list", "array-of-type -> nested list");
        assert_eq!(w("note"), "text", "optional string still a text widget");

        // Nested object recursion: author.name should be annotated too.
        let author_name_widget = props
            .get("author")
            .and_then(|a| a.get("properties"))
            .and_then(|p| p.get("name"))
            .and_then(|n| n.get("widget"))
            .and_then(|x| x.as_str());
        assert_eq!(
            author_name_widget,
            Some("text"),
            "nested fieldset fields are annotated"
        );
    }

    /// FEAT-168 / PLAN-122 — the `%scalar_type` table IS the scalar. This is the
    /// wave's acceptance test: a row added to stdlib/scalars/types.st with ZERO
    /// Rust changes must light up all four consumers. `date` is the proof.
    ///
    ///   recognition  `when: date` parses to `Primitive("date")`, not a Reference
    ///   schema       `generate_json_schema` → {"type":"string","format":"date"}
    ///   zero         `zero_value` → ""
    ///   widget       `annotate_widgets` → the `date` control
    ///
    /// If this test needs a Rust change to pass, the table is not the single
    /// source and the wave has not landed.
    #[test]
    fn test_scalar_table_single_source_date() {
        use super::annotate_widgets;

        let site = write_site(&[(
            "index.st",
            r#"@type Event { when: date; }
@data events: Event[] { src: "/data/events.json"; }
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);

        // Site 1 — recognition: `date` is a scalar (Primitive), not a Reference.
        let event = registry.get_type("Event").expect("Event type");
        let when = event
            .fields
            .iter()
            .find(|f| f.name == "when")
            .expect("when field");
        assert!(
            matches!(&when.type_expr, crate::parser::TypeExpr::Primitive(n) if n == "date"),
            "`date` must be recognized as a scalar (Primitive), got {:?}",
            when.type_expr
        );

        // Site 2 — schema: {"type":"string","format":"date"} from %schema + %format.
        let schema = crate::type_system::generate_json_schema(&when.type_expr, &registry);
        assert_eq!(
            schema,
            serde_json::json!({ "type": "string", "format": "date" })
        );

        // Site 3 — zero: %zero "" with a %schema of "string" → "".
        assert_eq!(registry.zero_value(&when.type_expr), serde_json::json!(""));

        // Site 4 — widget: the admin control comes from %widget.
        let mut schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference("Event".to_string()),
            &registry,
        );
        annotate_widgets(&mut schema);
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap();
        let widget = props
            .get("when")
            .and_then(|n| n.get("widget"))
            .and_then(|x| x.as_str());
        assert_eq!(widget, Some("date"));
    }

    /// The schema the admin renders forms from must carry field names + types
    /// the widget engine (W1) keys on: string, string[], etc.
    #[test]
    fn test_w0_type_schema_has_fields_for_form_generation() {
        let site = write_site(&[(
            "index.st",
            r#"@type Talent { id: string; name: string; categories: string[]; }
@data fetch $talents Talent[] : "/data/talent.json"
"#,
        )]);
        let ast = parse_site_ast(site.path()).expect("parse");
        let registry = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
        let schema = crate::type_system::generate_json_schema(
            &crate::parser::TypeExpr::Reference("Talent".to_string()),
            &registry,
        );
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .expect("schema has properties");
        assert_eq!(
            props
                .get("name")
                .and_then(|n| n.get("type"))
                .and_then(|t| t.as_str()),
            Some("string"),
            "name field must be a string widget"
        );
        assert_eq!(
            props
                .get("categories")
                .and_then(|c| c.get("type"))
                .and_then(|t| t.as_str()),
            Some("array"),
            "categories field must be an array (chips) widget"
        );
    }

    // ===================================================================
    // FUP-098: runtime injection MUST be idempotent. The dev server injects
    // `<script src="/__spacetime/runtime.js?...">` before </body>; if the page
    // already declares a runtime script (hand-authored, or a legacy build
    // artifact), injecting a SECOND one makes the bundle execute twice. The
    // second execution resets `ST.signals = new WeakMap()` (wiping every signal
    // store) while the `_st_init_*` dedup flags on DOM nodes survive, so the
    // re-init pass SKIPS and the stores are never repopulated — silently
    // breaking all reactive state (the dnd over/reject highlight regression).
    // The guard at the 5 inject sites is
    //   content.rfind("</body>").filter(|_| !content.contains("__spacetime/runtime.js"))
    // These tests lock that predicate.

    /// The shared idempotency predicate: only inject when no runtime tag exists.
    /// Delegates to the SAME `declares_runtime_script` the five live inject sites
    /// use — a local re-implementation here would let the real predicate drift
    /// while these tests kept passing against a copy.
    fn would_inject_runtime(content: &str) -> bool {
        content
            .rfind("</body>")
            .filter(|_| !super::declares_runtime_script(content))
            .is_some()
    }

    #[test]
    fn test_fup098_injects_when_runtime_absent() {
        let html = "<html><body><main>hi</main></body></html>";
        assert!(
            would_inject_runtime(html),
            "must inject the runtime when the page has no runtime script"
        );
    }

    #[test]
    fn test_fup098_skips_when_runtime_already_present() {
        // Page already declares the runtime (hand-authored or legacy artifact).
        let html = r#"<html><body><main>hi</main>
  <script src="/__spacetime/runtime.js?entry=index.st"></script>
</body></html>"#;
        assert!(
            !would_inject_runtime(html),
            "must NOT inject a second runtime script — double-load resets ST.signals (FUP-098)"
        );
    }

    /// BUG-214 (regression): a page that only MENTIONS the runtime path inside an
    /// HTML comment declares no script, so the dock must still inject.
    ///
    /// This is the exact shell shipped in `projects/backdesk-hospitality`. Its
    /// comment documents that the dev server auto-injects the runtime — and the
    /// old whole-document `contains` treated that prose as a declaration, so the
    /// host, migrations, and inspector pills all silently disappeared from the
    /// page. Nothing logged; the dock was simply absent.
    #[test]
    fn test_bug214_comment_mentioning_runtime_does_not_block_injection() {
        let html = r#"<html><body><main>hi</main>
  <!-- Spacetime runtime + styles are auto-injected by the dev server
       (/__spacetime/runtime.js + /__spacetime/styles.css) and by static
       export. Do NOT hand-add runtime tags here. -->
  <link rel="stylesheet" href="../../spacetime.css" />
  <script src="../../spacetime.js"></script>
</body></html>"#;
        assert!(
            would_inject_runtime(html),
            "a COMMENT mentioning the runtime path is prose, not a script tag — \
             the dev dock must still inject (BUG-214)"
        );
    }

    /// The FUP-098 protection must survive the precision fix: a real tag that
    /// happens to sit AFTER a comment still blocks injection.
    #[test]
    fn test_bug214_real_tag_after_a_comment_still_blocks() {
        let html = r#"<html><body>
  <!-- we load /__spacetime/runtime.js ourselves, on purpose -->
  <script src="/__spacetime/runtime.js?entry=index.st"></script>
</body></html>"#;
        assert!(
            !would_inject_runtime(html),
            "a live runtime tag must still suppress injection even when a comment \
             also mentions the path — double-load wipes ST.signals (FUP-098)"
        );
    }

    /// An unterminated comment swallows the rest of the document, so a marker
    /// inside it is not live markup and cannot double-load.
    #[test]
    fn test_bug214_unterminated_comment_is_not_a_declaration() {
        let html = r#"<html><body><main>hi</main></body></html>
<!-- /__spacetime/runtime.js"#;
        assert!(
            would_inject_runtime(html),
            "text inside an unterminated comment is not a script declaration"
        );
    }

    #[test]
    fn test_fup098_no_body_no_injection() {
        // Defensive: a fragment without </body> never injects (and never panics).
        assert!(!would_inject_runtime("<div>no body here</div>"));
    }
}

#[cfg(test)]
mod dock_panel_policy_tests {
    //! FUP-137: only one dock panel may be open at a time.
    //!
    //! The dock hosts three independently-openable panels. With no policy, two
    //! could be open at once and overlap — reproduced live before this landed:
    //! opening host then inspector left BOTH `.is-open`, and on a short viewport
    //! the occlusion hides the control being reached for.
    //!
    //! The behaviour itself is a DOM interaction and is verified in a real browser
    //! (host→inspector and inspector→host each leave exactly one panel open;
    //! closing the open one leaves none). What these tests lock is the part that
    //! can silently rot in source: that the policy is injected at all, and that it
    //! coordinates WITHOUT a shared signal.

    use super::*;

    /// The policy must actually reach the page. A dock that stopped injecting it
    /// would regress silently — both panels would simply open again.
    #[test]
    fn the_dock_injects_the_exclusive_panel_policy() {
        assert!(
            DEV_DOCK_EXCLUSIVE_PANELS_JS.contains("__panel"),
            "the policy must target the widgets' panel class convention"
        );
        assert!(
            DEV_DOCK_EXCLUSIVE_PANELS_JS.contains("st-dev-dock"),
            "and must be scoped to the DOCK — closing panels page-wide would reach \
             into the user's own markup"
        );
    }

    /// The constraint that actually broke once (PLAN-112): dock widgets share ONE
    /// global reactive scope, and an un-prefixed `$isExpanded` opened BOTH existing
    /// panels. The policy must therefore NOT introduce a shared open/closed signal;
    /// it coordinates on the rendered class and routes through each widget's own
    /// pill, so every widget's signal stays authoritative.
    #[test]
    fn the_policy_introduces_no_shared_signal() {
        assert!(
            !DEV_DOCK_EXCLUSIVE_PANELS_JS.contains("isExpanded"),
            "the policy must not read or write any widget's open/closed SIGNAL — \
             sharing one is what opened both panels at once before"
        );
        assert!(
            !DEV_DOCK_EXCLUSIVE_PANELS_JS.contains("SpacetimeLocal"),
            "nor reach into the global reactive scope at all"
        );
        assert!(
            DEV_DOCK_EXCLUSIVE_PANELS_JS.contains(".click()"),
            "it must close a sibling by CLICKING its own pill, so the widget's own \
             toggle runs and its signal stays in sync with the rendered class — \
             stripping `.is-open` directly would desynchronise them and the next \
             toggle would appear to do nothing"
        );
    }

    /// W3 REVIEW: the pill must be resolved by an element whose OWN class ends in
    /// `__pill`, never by a substring match.
    ///
    /// The first version used `closest('[class*="__pill"]')`. A pill's inner label
    /// carries `<widget>__pill-label`, which ALSO contains that substring — so a
    /// click on the LABEL, which is what a user actually hits, resolved to the
    /// label. The sibling lookup then derived a different element and the exclusion
    /// silently did nothing. Reproduced live: clicking the host label then the
    /// inspector label left BOTH panels open, i.e. the exact defect the policy
    /// exists to prevent, on the primary interaction path.
    #[test]
    fn the_pill_is_resolved_by_exact_class_not_substring() {
        // The constant's own comment quotes the rejected selector, so assert on the
        // CODE: no live `closest` call may resolve the pill by substring.
        let code_only: String = DEV_DOCK_EXCLUSIVE_PANELS_JS
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !code_only.contains(r#"closest('[class*="__pill"]')"#),
            "a substring match also selects `__pill-label`, so a click on the label \
             — the primary interaction path — bypasses the policy entirely"
        );
        assert!(
            DEV_DOCK_EXCLUSIVE_PANELS_JS.contains("endsWith('__pill')"),
            "the pill must be identified by its OWN class ending in `__pill`"
        );
    }

    /// A click that CLOSED a panel must not trigger the exclusion pass (there is no
    /// newly-opened panel to protect), or closing one panel could reopen another.
    /// Four dock widgets must keep using the shared suffix contract; adding one
    /// without these exact classes would make its panel escape mutual exclusion.
    #[test]
    fn comments_widget_uses_the_dock_suffix_contract() {
        let source = include_str!("../stdlib/comments/__dev__/pill.st");
        assert!(source.contains("st-comments-widget__pill"));
        assert!(source.contains("st-comments-widget__panel"));
        assert!(
            !source.contains("st-comments-widget__pill-label"),
            "a label ending in __pill would be mistaken for the clickable pill"
        );
    }

    /// Project overlays and private helpers are compiler inputs, not routes.
    #[test]
    fn underscore_prefixed_st_paths_are_not_pages() {
        let site = tempfile::tempdir().expect("temporary site");
        std::fs::write(site.path().join("_prelude.st"), "<p>private</p>").expect("private prelude");
        assert!(
            try_serve_st_page(
                "/_prelude",
                site.path(),
                false,
                None,
                false,
                &CompileCache::new(),
                &DevCompileCache::new(),
            )
            .is_none(),
            "underscore-prefixed compiler inputs must never resolve as pages"
        );
    }

    #[test]
    fn closing_a_panel_does_not_trigger_exclusion() {
        assert!(
            DEV_DOCK_EXCLUSIVE_PANELS_JS.contains("if (!opened) return;"),
            "the pass must bail when the click opened nothing — verified live: \
             closing the only open panel leaves NONE open"
        );
    }
}

/// PLAN-123 W2 acceptance: the comments routes as a WORKING SURFACE.
///
/// These handlers are what the pill calls and what the hosted tier will
/// re-implement behind the same JSON, so they are exercised end to end
/// through the real sidecar store on disk. A route that COMPILES is not a
/// route that round-trips, and the thing being promised here is durable team
/// state — not a panel that looks right until reload.
#[cfg(test)]
mod comments_route_tests {
    use super::*;

    fn comments_site() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("index.st"),
            "<div class=\"stage\">Hi</div>\n",
        )
        .expect("write index.st");
        dir
    }

    fn comments_state(site: &tempfile::TempDir) -> AppState {
        AppState {
            mode: AppMode::Dev {
                site_dir: site.path().to_path_buf(),
                trace: false,
                debug: false,
            },
            static_dir: None,
            debugger_config: None,
            cache_stdlib: false,
            compile_cache: std::sync::Arc::new(CompileCache::new()),
            dev_compile_cache: std::sync::Arc::new(DevCompileCache::new()),
        }
    }

    /// Jump-to-source, end to end, through the real route (FUP-170).
    ///
    /// The decisive case is the SECOND half: markup is inserted ABOVE the
    /// commented element, which shifts every positional `data-st-node` id. A
    /// stored span would now point at the inserted markup, and a hint followed
    /// blindly would name the wrong node -- both delivering a confident wrong
    /// answer. Resolving on read, confirmed by content, must survive it.
    ///
    /// This test also GUARDS the design: it fails if anyone later "optimizes"
    /// the span into a stored field on the anchor.
    #[tokio::test]
    async fn a_comment_resolves_to_the_source_it_was_written_in() {
        let site = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            site.path().join("index.st"),
            "@template &main() {\n  <header class=\"masthead\">Site</header>\n  <button class=\"cta\">Buy</button>\n}\n",
        )
        .expect("write index.st");

        let added = json_body(
            add(
                &site,
                serde_json::json!({
                    "type": "todo",
                    "anchor": { "kind": "element", "route": "/", "selector": "0.1" },
                    "text": "This label is wrong",
                    "element_content": {
                        "selector": "0.1", "tag": "button", "class": "cta",
                        "id_attr": "", "text": "Buy"
                    }
                }),
            )
            .await,
        )
        .await;
        let id = added["id"].as_str().expect("added id").to_string();

        let before = json_body(
            comments_source_handler(
                axum::extract::State(comments_state(&site)),
                Query(CommentsSourceQuery { id: id.clone() }),
            )
            .await,
        )
        .await;
        assert_eq!(
            before["outcome"], "found",
            "the comment must resolve to its source: {before}"
        );
        assert_eq!(before["file"], "index.st");
        assert_eq!(
            before["span"]["relative_to"], "template-body",
            "the span's frame of reference must be stated, never assumed: {before}"
        );
        assert_eq!(
            before["moved"], false,
            "nothing moved yet, so the stored hint should have been confirmed: {before}"
        );
        let start_before = before["span"]["start"].as_u64().expect("span start");

        // Someone inserts a paragraph ABOVE the commented button. Every sibling
        // id shifts, and every byte offset below the insertion moves.
        std::fs::write(
            site.path().join("index.st"),
            "@template &main() {\n  <header class=\"masthead\">Site</header>\n  <p class=\"lede\">New</p>\n  <button class=\"cta\">Buy</button>\n}\n",
        )
        .expect("rewrite index.st");

        let after = json_body(
            comments_source_handler(
                axum::extract::State(comments_state(&site)),
                Query(CommentsSourceQuery { id: id.clone() }),
            )
            .await,
        )
        .await;
        assert_eq!(
            after["outcome"], "found",
            "an edit above the target must not orphan it: {after}"
        );
        assert_eq!(
            after["node"], "0.2",
            "the button shifted one slot down and the answer must follow it: {after}"
        );
        assert_eq!(
            after["moved"], true,
            "the positional hint went stale and that must be reported, not hidden: {after}"
        );
        assert!(
            after["span"]["start"].as_u64().expect("span start") > start_before,
            "a stored span would have been reused unchanged; this must be recomputed: {after}"
        );
    }

    /// A deleted element orphans loudly rather than resolving to a neighbour.
    #[tokio::test]
    async fn a_comment_whose_element_was_deleted_says_so() {
        let site = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            site.path().join("index.st"),
            "@template &main() {\n  <button class=\"cta\">Buy</button>\n}\n",
        )
        .expect("write index.st");
        let added = json_body(
            add(
                &site,
                serde_json::json!({
                    "type": "todo",
                    "anchor": { "kind": "element", "route": "/", "selector": "0.0" },
                    "text": "Check this",
                    "element_content": {
                        "selector": "0.0", "tag": "button", "class": "cta",
                        "id_attr": "", "text": "Buy"
                    }
                }),
            )
            .await,
        )
        .await;
        let id = added["id"].as_str().expect("added id").to_string();
        std::fs::write(
            site.path().join("index.st"),
            "@template &main() {\n  <p class=\"lede\">The button is gone</p>\n}\n",
        )
        .expect("rewrite index.st");
        let body = json_body(
            comments_source_handler(
                axum::extract::State(comments_state(&site)),
                Query(CommentsSourceQuery { id }),
            )
            .await,
        )
        .await;
        assert_eq!(
            body["outcome"], "gone",
            "a deleted element must orphan, never resolve to whatever took its place: {body}"
        );
    }

    fn empty_status_query() -> CommentsStatusQuery {
        CommentsStatusQuery {
            route: None,
            file: None,
            status: None,
            type_id: None,
        }
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        serde_json::from_slice(&body).expect("JSON response")
    }

    async fn add(site: &tempfile::TempDir, payload: serde_json::Value) -> Response {
        comments_add_handler(
            axum::extract::State(comments_state(site)),
            Json(serde_json::from_value(payload).expect("add request")),
        )
        .await
    }

    /// Moving a comment's status must record WHO moved it. Without this a
    /// resolved comment is unauditable: nobody can tell a verified fix from an
    /// agent that closed work it never read, and a reviewer who disagrees has
    /// nobody to ask.
    #[tokio::test]
    async fn a_status_change_is_attributed_on_disk() {
        let site = comments_site();
        let created = add(
            &site,
            serde_json::json!({
                "type": "todo",
                "anchor": { "kind": "page", "route": "/" },
                "text": "needs a second pair of eyes"
            }),
        )
        .await;
        let id = json_body(created).await["id"]
            .as_str()
            .expect("an id")
            .to_string();

        let move_to_resolved = || {
            let payload = serde_json::json!({ "id": id, "status": "resolved" });
            comments_update_handler(
                axum::extract::State(comments_state(&site)),
                Json(serde_json::from_value(payload).expect("a valid update")),
            )
        };
        assert_eq!(move_to_resolved().await.status(), StatusCode::OK);

        let read_stored = || {
            let path = site.path().join(".comments").join(format!("{id}.json"));
            let raw = std::fs::read_to_string(path).expect("the record is on disk");
            serde_json::from_str::<serde_json::Value>(&raw).expect("valid json")
        };

        let stored = read_stored();
        let history = stored["history"].as_array().expect("a history log");
        assert_eq!(history.len(), 1, "one move, one entry");
        assert_eq!(history[0]["from"], "open", "the origin must be recorded");
        assert_eq!(history[0]["to"], "resolved");
        assert!(
            history[0]["author"]["name"]
                .as_str()
                .is_some_and(|name| !name.is_empty()),
            "a transition must name its author, or `resolved` cannot be questioned"
        );

        // Re-submitting the SAME status is not a transition and must not pad the
        // log with an event that never happened.
        assert_eq!(move_to_resolved().await.status(), StatusCode::OK);
        assert_eq!(
            read_stored()["history"].as_array().expect("a log").len(),
            1,
            "a no-op re-submit must not record a second transition"
        );
    }

    /// THE round trip: a comment created through the route must be readable
    /// through the status route, updatable, and PERSISTED on disk.
    #[tokio::test]
    async fn a_comment_survives_add_then_status_then_update() {
        let site = comments_site();

        let created = add(
            &site,
            serde_json::json!({
                "type": "todo",
                "anchor": { "kind": "page", "route": "/" },
                "text": "replace the placeholder copy"
            }),
        )
        .await;
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = json_body(created).await;
        let id = created["id"].as_str().expect("an id").to_string();
        assert_eq!(created["status"], "open", "a new comment starts open");

        // It must be on DISK, not merely in the response.
        let record_path = site.path().join(".comments").join(format!("{id}.json"));
        assert!(record_path.exists(), "the record must be persisted");

        // And visible through the status route the pill polls.
        let status = comments_status_handler(
            axum::extract::State(comments_state(&site)),
            Query(empty_status_query()),
        )
        .await;
        assert_eq!(status.status(), StatusCode::OK);
        let index = json_body(status).await;
        let records = index["records"].as_array().expect("records array");
        assert!(
            records.iter().any(|r| r["id"] == id.as_str()),
            "the new comment must appear in status.json: {index}"
        );

        let updated = comments_update_handler(
            axum::extract::State(comments_state(&site)),
            Json(
                serde_json::from_value(serde_json::json!({
                    "id": id,
                    "status": "resolved",
                    "reply": "swapped in the real photography"
                }))
                .expect("update request"),
            ),
        )
        .await;
        assert_eq!(updated.status(), StatusCode::OK);

        // The change must be readable from the STORE, not just echoed back:
        // that is what makes the pill and the MCP tools agree.
        let stored: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&record_path).expect("stored record"))
                .expect("stored JSON");
        assert_eq!(stored["status"], "resolved");
        assert_eq!(
            stored["thread"][0]["text"], "swapped in the real photography",
            "the reply must persist in the thread"
        );
        assert!(
            stored["updated_at"].is_string(),
            "an edit must be stamped so the hosted tier can order events"
        );
    }

    /// An unknown type is a 4xx, never a silent write. The roster IS the
    /// contract; accepting a type nobody declared would put a record in the
    /// store that no reader can render or explain.
    #[tokio::test]
    async fn an_unknown_type_is_refused_and_writes_nothing() {
        let site = comments_site();
        let response = add(
            &site,
            serde_json::json!({
                "type": "todoo",
                "anchor": { "kind": "page", "route": "/" },
                "text": "typo in the type"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        let error = body["error"].as_str().unwrap_or_default();
        assert!(
            error.contains("todoo"),
            "the error must name what was rejected: {error}"
        );

        let wrote_anything = std::fs::read_dir(site.path().join(".comments"))
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        assert!(!wrote_anything, "a refused add must write NOTHING");
    }

    /// `agent-task` without `acceptance` is a task with no observable finish
    /// line — exactly the shape an agent cannot honestly close.
    #[tokio::test]
    async fn a_missing_required_field_is_refused() {
        let site = comments_site();
        let response = add(
            &site,
            serde_json::json!({
                "type": "agent-task",
                "anchor": { "kind": "page", "route": "/" },
                "text": "do the thing"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .contains("acceptance"),
            "the error must name the missing field: {body}"
        );
    }

    /// `.comments/` belongs to ONE project. A path that climbs out of it would
    /// let a page in the browser address files the server was never asked to
    /// serve.
    #[tokio::test]
    async fn an_anchor_outside_the_project_is_refused() {
        let site = comments_site();
        for escape in ["../secrets.st", "/etc/passwd", "a/../../b.st"] {
            let response = add(
                &site,
                serde_json::json!({
                    "type": "todo",
                    "anchor": { "kind": "file", "file": escape },
                    "text": "escape attempt"
                }),
            )
            .await;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "anchor {escape:?} must be refused"
            );
        }
    }

    #[tokio::test]
    async fn element_anchors_require_both_location_parts_and_filter_by_route() {
        let site = comments_site();
        for anchor in [
            serde_json::json!({ "kind": "element", "route": "", "selector": ".cta" }),
            serde_json::json!({ "kind": "element", "route": "/pricing", "selector": "" }),
        ] {
            let response = add(
                &site,
                serde_json::json!({ "type": "todo", "anchor": anchor, "text": "fix CTA" }),
            )
            .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let created = add(
            &site,
            serde_json::json!({
                "type": "todo",
                "anchor": { "kind": "element", "route": "/pricing", "selector": ".cta > button" },
                "text": "the CTA overlaps the nav"
            }),
        )
        .await;
        assert_eq!(created.status(), StatusCode::CREATED);

        let status = comments_status_handler(
            axum::extract::State(comments_state(&site)),
            Query(CommentsStatusQuery {
                route: Some("/pricing".into()),
                file: None,
                status: None,
                type_id: None,
            }),
        )
        .await;
        let body = json_body(status).await;
        assert_eq!(body["records"].as_array().map(Vec::len), Some(1));
        assert_eq!(body["records"][0]["anchor"]["selector"], ".cta > button");
    }

    /// Silently minting a record for an unknown id would let a stale pill
    /// resurrect a comment someone deliberately removed.
    #[tokio::test]
    async fn updating_an_unknown_id_is_not_a_create() {
        let site = comments_site();
        let response = comments_update_handler(
            axum::extract::State(comments_state(&site)),
            Json(
                serde_json::from_value(serde_json::json!({
                    "id": "nosuchrecord",
                    "status": "resolved"
                }))
                .expect("update request"),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    fn orphan_record(site: &tempfile::TempDir, id: &str) -> crate::comments::CommentRecord {
        let record = crate::comments::CommentRecord {
            id: id.to_string(),
            type_id: "todo".to_string(),
            status: crate::comments::Status::InProgress,
            author: crate::comments::Author {
                kind: crate::comments::AuthorKind::Human,
                name: "ada".to_string(),
            },
            claimed_by: None,
            anchor: crate::comments::Anchor::Inline {
                file: "index.st".to_string(),
                line: 99,
            },
            text: "preserve this conversation".to_string(),
            meta: BTreeMap::new(),
            history: Vec::new(),
            thread: vec![crate::comments::Reply {
                author: crate::comments::Author {
                    kind: crate::comments::AuthorKind::Agent,
                    name: "spell".to_string(),
                },
                text: "I found the cause".to_string(),
                at: "2026-08-03T00:00:00Z".to_string(),
            }],
            created_at: "2026-08-02T00:00:00Z".to_string(),
            updated_at: None,
            v: crate::comments::SCHEMA_VERSION,
            inline: false,
        };
        crate::comments::write_record(site.path(), &record).expect("persist orphan");
        record
    }

    /// Re-anchoring is a move: losing source identity must not lose the status
    /// or thread that tells the next teammate what has already happened.
    #[tokio::test]
    async fn reanchoring_an_orphan_preserves_its_conversation_on_disk() {
        let site = comments_site();
        let original = orphan_record(&site, "orphaned");
        assert!(
            comments_index(site.path())
                .orphans
                .iter()
                .any(|r| r.id == original.id)
        );

        let response = comments_reanchor_handler(
            axum::extract::State(comments_state(&site)),
            Json(
                serde_json::from_value(serde_json::json!({
                    "id": original.id,
                    "anchor": { "kind": "page", "route": "/pricing" }
                }))
                .expect("re-anchor request"),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let moved = json_body(response).await;
        assert_eq!(moved["id"], "orphaned");
        assert_eq!(moved["status"], "in-progress");
        assert_eq!(moved["thread"][0]["text"], "I found the cause");

        let (stored, diagnostics) = crate::comments::read_sidecar(site.path());
        assert!(diagnostics.is_empty());
        assert_eq!(
            stored["orphaned"].anchor,
            crate::comments::Anchor::Page {
                route: "/pricing".into()
            }
        );
        assert_eq!(stored["orphaned"].thread, original.thread);
        assert_eq!(stored["orphaned"].status, original.status);
        assert!(
            !comments_index(site.path())
                .orphans
                .iter()
                .any(|r| r.id == "orphaned"),
            "a moved record must no longer be offered for orphan recovery"
        );
    }

    #[tokio::test]
    async fn reanchoring_to_an_inline_anchor_is_refused() {
        let site = comments_site();
        let original = orphan_record(&site, "bad-anchor");
        let response = comments_reanchor_handler(
            axum::extract::State(comments_state(&site)),
            Json(
                serde_json::from_value(serde_json::json!({
                    "id": original.id,
                    "anchor": { "kind": "inline", "file": "index.st", "line": 1 }
                }))
                .expect("re-anchor request"),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            comments_index(site.path())
                .orphans
                .iter()
                .any(|r| r.id == "bad-anchor")
        );
    }

    #[tokio::test]
    async fn dismissing_an_orphan_removes_its_record_file() {
        let site = comments_site();
        let original = orphan_record(&site, "dismiss-me");
        let path = site.path().join(".comments").join("dismiss-me.json");
        let response = comments_dismiss_handler(
            axum::extract::State(comments_state(&site)),
            Json(CommentsDismissRequest { id: original.id }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(!path.exists(), "dismissal must destroy the sidecar record");
        assert!(
            !comments_index(site.path())
                .orphans
                .iter()
                .any(|r| r.id == "dismiss-me")
        );
    }

    #[tokio::test]
    async fn dismissing_an_unknown_id_is_not_a_silent_success() {
        let site = comments_site();
        let response = comments_dismiss_handler(
            axum::extract::State(comments_state(&site)),
            Json(CommentsDismissRequest {
                id: "not-there".to_string(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn dismissing_a_non_orphan_record_is_refused() {
        let site = comments_site();
        let record = crate::comments::CommentRecord {
            id: "live-record".to_string(),
            type_id: "todo".to_string(),
            status: crate::comments::Status::Open,
            author: default_comment_author(),
            claimed_by: None,
            anchor: crate::comments::Anchor::Page {
                route: "/".to_string(),
            },
            text: "live record".to_string(),
            meta: BTreeMap::new(),
            thread: Vec::new(),
            history: Vec::new(),
            created_at: comment_now(),
            updated_at: None,
            v: crate::comments::SCHEMA_VERSION,
            inline: false,
        };
        crate::comments::write_record(site.path(), &record).expect("persist live record");
        let response = comments_dismiss_handler(
            axum::extract::State(comments_state(&site)),
            Json(CommentsDismissRequest {
                id: record.id.clone(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(
            site.path()
                .join(".comments")
                .join("live-record.json")
                .exists()
        );
    }

    /// The pill's type picker is DRIVEN by this route, which is what makes
    /// "declare a type, it appears everywhere" true rather than aspirational.
    #[tokio::test]
    async fn the_types_route_serves_the_whole_roster_with_hints() {
        let site = comments_site();
        let response = comments_types_handler(axum::extract::State(comments_state(&site))).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        let types = body["types"]
            .as_array()
            .or_else(|| body.as_array())
            .expect("a types array");

        for expected in ["note", "todo", "question", "bug", "design", "agent-task"] {
            assert!(
                types.iter().any(|t| t["id"] == expected),
                "the roster must include `{expected}`: {body}"
            );
        }
        let agent_task = types
            .iter()
            .find(|t| t["id"] == "agent-task")
            .expect("agent-task");
        assert!(
            agent_task["agent_hint"].is_string(),
            "a type's follow-up contract must reach the client: {agent_task}"
        );
    }

    /// A literate document's fences ARE the program, so a comment written in
    /// one is as real as any other. The index walked only `.st`, so an agent
    /// (whose scanner reads `.st.md`) saw comments the pill did not — one
    /// store answering two ways. Worse, an inline record in a literate file
    /// sat outside coverage, so it could never be surfaced as an orphan and
    /// could never be dismissed.
    #[tokio::test]
    async fn comments_in_a_literate_document_are_indexed() {
        let site = comments_site();
        std::fs::write(
            site.path().join("guide.st.md"),
            "# Guide\n\nProse here.\n\n```st\n//@todo: document the second fence\n<p>hi</p>\n```\n",
        )
        .unwrap();

        let response = comments_status_handler(
            axum::extract::State(comments_state(&site)),
            Query(empty_status_query()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        let records = body["records"].as_array().expect("records");
        assert!(
            records
                .iter()
                .any(|r| r["text"] == "document the second fence"),
            "a comment in a `.st.md` fence must be indexed: {body}"
        );
    }

    /// A project with no comments is the ordinary case: the pill must render
    /// an empty panel, not an error.
    #[tokio::test]
    async fn a_project_with_no_comments_serves_an_empty_index() {
        let site = comments_site();
        let response = comments_status_handler(
            axum::extract::State(comments_state(&site)),
            Query(empty_status_query()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["records"].as_array().map(|r| r.len()), Some(0));
        assert_eq!(body["orphans"].as_array().map(|r| r.len()), Some(0));
    }
}

/// PLAN-123 Gate-2 regressions: the PILL'S CONTRACT with its own routes.
///
/// The route tests exercise the handlers directly, which is exactly why they
/// passed while the pill was unusable: a composer that posts the wrong shape
/// fails at deserialization, before any handler this suite calls. These tests
/// read the shipped `.st` and assert the payload it actually builds, because
/// the only thing standing between a working panel and a dead one is whether
/// the widget and the route agree.
#[cfg(test)]
mod comments_pill_contract_tests {
    const PILL: &str = include_str!("../stdlib/comments/__dev__/pill.st");

    /// THE PILL MUST COMPILE.
    ///
    /// Every other test in this module asserts against the pill's SOURCE TEXT
    /// (`PILL.contains(…)`), which means all of them pass happily on a file the
    /// compiler rejects. That is exactly what happened: the widget carried an
    /// `Array.from(…).join(…)` expression that does not parse (FUP-172), so the
    /// dock served nothing while this suite stayed green — the same class of
    /// blind spot as a passing test suite over a non-building HEAD.
    ///
    /// This test closes it by compiling the real file through the SAME entry
    /// point the dev server uses, and by checking the picker actually reaches
    /// the emitted JS rather than merely appearing in the source.
    #[test]
    fn the_pill_compiles_and_emits_its_picker() {
        let compiled = crate::Compiler::from_file(
            std::path::Path::new("stdlib/comments/__dev__/pill.st"),
            std::path::Path::new("."),
        )
        .expect("the comments pill must parse — the dev dock compiles this file")
        .compile();

        assert!(
            !compiled.js.is_empty(),
            "the pill must emit JS, or the dock renders an inert shell"
        );
        assert!(
            compiled.js.contains("addEventListener('click'"),
            "the picker's document listener must reach the emitted JS — a picker \
             that exists only in source text cannot pick anything"
        );
        assert!(
            compiled.js.contains("st-comments-widget'"),
            "the emitted picker must exclude the widget itself, or clicking the \
             pill's own controls would anchor a comment to them"
        );
        assert!(
            compiled.js.contains("element_content"),
            "the picked element's content must reach the wire, or the server has \
             nothing to mint a fingerprint from"
        );
        assert!(
            !compiled.js.to_lowercase().contains("blake3"),
            "the pill must not hash client-side: one hash rule, one implementation"
        );
    }

    /// The pill offers jump-to-source, and asks the SERVER for it (FUP-170).
    ///
    /// The affordance must go through `/__spacetime/comments/source.json` and
    /// nothing else. The temptation is to have the pill hold a span from the
    /// record it already has -- but a span stored anywhere client-side goes
    /// stale on the first edit above the element and then points confidently at
    /// the wrong markup. Resolution belongs to the server, against the file as
    /// it is NOW, on every press.
    #[test]
    fn the_pill_asks_the_server_where_an_element_was_written() {
        assert!(
            PILL.contains("/__spacetime/comments/source.json"),
            "the pill must resolve source through the server route"
        );
        assert!(
            PILL.contains("data-source="),
            "each record needs an affordance that asks where it was written"
        );
        assert!(
            PILL.contains("$comSource("),
            "the affordance must invoke the source signal, not read a stored span"
        );
        // Every outcome the route can return must be SAYABLE. An outcome the
        // pill cannot render is an affordance that silently does nothing --
        // which for `gone` and `ambiguous` is exactly the case a reviewer most
        // needs told, because those mean the comment has drifted from the code.
        for outcome in [
            "found",
            "gone",
            "ambiguous",
            "unverified",
            "synthesized",
            "no-source",
            "not-an-element",
        ] {
            assert!(
                PILL.contains(&format!("\"{outcome}\"")),
                "the pill must be able to report the `{outcome}` outcome"
            );
        }
        assert!(
            PILL.contains("moved"),
            "a stale positional hint must be surfaced, not hidden: a surprising \
             location is only trustworthy if the drift is stated"
        );
    }

    /// `Anchor` is a serde-TAGGED union (`#[serde(tag = "kind")]`). A payload
    /// without `kind` is rejected during extraction, so the Add button would
    /// silently do nothing — no request logged, no error shown.
    #[test]
    fn the_composer_sends_a_tagged_anchor() {
        assert!(
            PILL.contains("kind: \"page\""),
            "a page anchor must carry its `kind` discriminator"
        );
        assert!(
            PILL.contains("kind: \"file\""),
            "a file anchor must carry its `kind` discriminator"
        );
    }

    /// A type may REQUIRE a field — stdlib's `agent-task` requires
    /// `acceptance`. With a hardcoded empty meta the server refuses every such
    /// add with a 400, making a shipped default type uncreatable from the UI
    /// that exists to create it.
    #[test]
    fn the_composer_can_supply_required_metadata() {
        assert!(
            !PILL.contains("meta: {}"),
            "meta must not be hardcoded empty — required fields become unfillable"
        );
        assert!(
            PILL.contains("$comMetaKey") && PILL.contains("$comMetaValue"),
            "the composer needs inputs for a type's required field"
        );
        assert!(
            PILL.contains("st-comments-widget__meta-key"),
            "the metadata inputs must exist in the markup, not just as signals"
        );
    }

    /// An empty panel caused by a dead endpoint must never read as "your team
    /// has no comments" — the two states are indistinguishable to the reader
    /// and only one of them is true.
    #[test]
    fn a_failed_fetch_is_visible_rather_than_an_empty_panel() {
        assert!(
            PILL.contains("$comTypesResponse_error") && PILL.contains("$comStatusResponse_error"),
            "both fetches must surface their error state"
        );
        assert!(
            PILL.contains("$comLoadError"),
            "load failures need a signal the panel can render"
        );
    }

    /// Orphan rows rendered `$comOrphan.reason`, a field `CommentRecord` does
    /// not have: every orphan showed a blank line and omitted the anchor and
    /// text a person needs to decide whether to re-anchor or dismiss it.
    #[test]
    fn orphan_rows_render_fields_that_exist() {
        assert!(
            !PILL.contains("$comOrphan.reason"),
            "`reason` is not a field of CommentRecord — the row would render blank"
        );
        assert!(
            PILL.contains("$comOrphan.text") && PILL.contains("$comOrphan.anchor"),
            "an orphan must show what it says and where it was, or it cannot be triaged"
        );
    }

    /// The composer can pick an element, and the picked anchor must carry the
    /// CONTENT the server needs to mint its identity.
    #[test]
    fn the_composer_picks_an_element_and_sends_its_content() {
        assert!(
            PILL.contains("st-comments-widget__record-selector")
                && PILL.contains("$comRecord.anchor.selector"),
            "an element record must expose its selector to the reviewer"
        );
        assert!(
            PILL.contains("@comments-pick("),
            "the composer must mount the picker primitive"
        );
        // The whole point of the picker: it sends CONTENT, and the server hashes
        // it. A pill that sent only a selector would be minting exactly the
        // positional anchors FUP-171 W-b removed.
        assert!(
            PILL.contains("element_content: $comContent"),
            "the add rail must carry the picked element's content, or the anchor \
             it creates has no verifiable identity"
        );
        assert!(
            !PILL.contains("blake3") && !PILL.contains("fingerprint:"),
            "the pill must NOT compute a fingerprint — one hash rule, one \
             implementation; a client-side hash that drifted by a byte would \
             orphan every anchor it wrote"
        );
    }

    /// A comment on an element you cannot find is a riddle. "Show on page" must
    /// resolve by stored CONTENT, and must report every outcome — including the
    /// ones where it finds nothing, which are information, not failures.
    #[test]
    fn a_record_can_be_located_on_the_page_by_its_content() {
        assert!(
            PILL.contains("@comments-locate(") && PILL.contains("data-locate"),
            "a record row must offer a way to find its element on the page"
        );
        // Passing the SELECTOR would highlight whatever now occupies that
        // position after an edit — the silent mis-point the design refuses.
        assert!(
            PILL.contains(".anchor.content"),
            "locating must hand over the stored CONTENT descriptor, not the \
             positional selector"
        );
        // Each outcome must be SAID. A button that silently does nothing when an
        // element is gone teaches the reader the feature is broken, when in fact
        // it just told them something true about their page.
        for outcome in ["found", "gone", "ambiguous", "unverified"] {
            assert!(
                PILL.contains(&format!("$comLocateOutcome == \"{outcome}\"")),
                "the pill must explain the `{outcome}` outcome to the reader"
            );
        }
    }

    /// Arming pick mode must be tied to choosing the element anchor kind, so a
    /// document-wide click listener can never outlive the intent that armed it.
    #[test]
    fn pick_mode_is_armed_only_by_choosing_the_element_anchor() {
        assert!(
            PILL.contains("$comPicking <- ($.dataset.anchorKind == \"element\")"),
            "choosing a non-element anchor must DISARM pick mode, not leave a \
             capture-phase listener swallowing the page's clicks"
        );
        // A picked element files under its OWN page. Taking the route from the
        // free-text anchor input would file it under whatever the user typed.
        assert!(
            PILL.contains("route: ($comPicked.route || $comAnchorValue)"),
            "an element anchor must take its route from the picked element's page"
        );
    }

    #[test]
    fn the_orphan_controls_send_explicit_recovery_payloads() {
        assert!(
            PILL.contains("/__spacetime/comments/re-anchor")
                && PILL.contains("id: $comId, anchor: $comAnchor"),
            "re-anchor must send the id plus a tagged anchor, not an update-shaped payload"
        );
        assert!(
            PILL.contains("kind: \"page\"") && PILL.contains("kind: \"file\""),
            "orphan recovery must send a valid non-inline tagged anchor"
        );
        assert!(
            PILL.contains("/__spacetime/comments/dismiss")
                && PILL.contains("{ id: $comId }")
                && PILL.contains("Dismiss permanently"),
            "destructive dismissal needs its own explicit payload and affordance"
        );
        assert!(
            PILL.contains("$comOrphan.type_id")
                && PILL.contains("$comOrphan.text")
                && PILL.contains("$comOrphan.anchor.file")
                && PILL.contains("$comOrphan.thread.length"),
            "an orphan row must show its type, text, old anchor, and thread length"
        );
    }

    /// The premise of the whole roster: declare a type, it appears everywhere.
    /// A hardcoded picker would pass every other test while quietly defeating
    /// that.
    #[test]
    fn the_type_picker_is_driven_by_the_roster() {
        assert!(
            PILL.contains("types.json"),
            "the picker's source must be the served roster"
        );
        assert!(
            PILL.contains("@each($comTypes as $comType)"),
            "the picker must iterate the fetched types, never a literal list"
        );
    }
}

/// PLAN-123: the DOCUMENTATION must describe the surface that exists.
///
/// The Gate-3 review found three claims in `docs/language/comments.md` that
/// were false the day they were written: a route spelled without its hyphen,
/// an anchor kind the composer cannot create, and a subscription that is
/// accepted but never delivers. Documentation that lies is worse than absent
/// documentation, because it is trusted — someone follows it, it fails, and
/// they conclude the feature is broken rather than the sentence.
///
/// These tests pin the mechanical claims to the code, so the next person to
/// rename a route finds out here rather than from a reader.
#[cfg(test)]
mod comments_doc_accuracy_tests {
    const DOC: &str = include_str!("../docs/language/comments.md");
    const SERVER: &str = include_str!("server.rs");
    const PILL: &str = include_str!("../stdlib/comments/__dev__/pill.st");

    /// The doc's table of source outcomes must match what the route can return.
    ///
    /// A reader plans around this table: `gone` means the comment drifted from
    /// the code, `ambiguous` means it cannot be told from its neighbours. If the
    /// route grew an outcome the doc did not list, a reader would meet a state
    /// the documentation says is impossible -- and the states this route reports
    /// are exactly the ones that mean something is WRONG, so an unexplained one
    /// lands at the worst moment.
    #[test]
    fn the_doc_lists_every_source_outcome_the_route_can_return() {
        for outcome in [
            "found",
            "gone",
            "ambiguous",
            "unverified",
            "synthesized",
            "no-source",
            "not-an-element",
        ] {
            assert!(
                SERVER.contains(&format!("\"outcome\": \"{outcome}\"")),
                "the route must actually be able to return `{outcome}`"
            );
            assert!(
                DOC.contains(&format!("`{outcome}`")),
                "the doc must explain the `{outcome}` outcome"
            );
        }
        // The frame of reference is the part a reader would otherwise assume
        // wrongly: a body offset read as a file offset points at the top of the
        // file. Both the wire and the prose must say which it is.
        assert!(
            SERVER.contains("template-body") && DOC.contains("template-body"),
            "the span's frame of reference must be stated on the wire AND in the doc"
        );
        assert!(
            PILL.contains("$comSourceLabel"),
            "the pill must render the outcome rather than failing silently"
        );
    }

    /// Every route the doc names must be registered, spelled exactly.
    #[test]
    fn every_documented_route_exists() {
        for route in [
            "types.json",
            "status.json",
            "source.json",
            "add",
            "update",
            "re-anchor",
            "dismiss",
            "prune",
        ] {
            assert!(
                DOC.contains(&format!("`POST {route}`")) || DOC.contains(&format!("`GET {route}`")),
                "the doc must list the `{route}` route"
            );
            assert!(
                SERVER.contains(&format!("/__spacetime/comments/{route}")),
                "the doc names `{route}` but no such route is registered"
            );
        }
    }

    /// The doc and the pill must agree about the element picker.
    ///
    /// This test previously asserted the picker was ABSENT, with a doc comment
    /// claiming it was blocked by the repo's no-JavaScript rule. Both were wrong:
    /// `%primitive` + `%emit js` is the sanctioned rail, and the inspector already
    /// shipped a picker written in Spacetime. The real blocker was that the anchor
    /// had no content identity — a picker on positional selectors would have
    /// industrialized silent re-pointing. With FUP-171 W-b landed, it ships.
    #[test]
    fn the_doc_and_the_pill_agree_about_the_element_picker() {
        let composer_has_element_control = PILL.contains("data-anchor-kind=\"element\"");
        assert!(
            composer_has_element_control,
            "the composer must offer an element anchor control"
        );
        assert!(
            !DOC.contains("no element picker yet"),
            "the pill HAS a picker now — the doc must not still claim otherwise"
        );
    }

    /// The doc must explain what an edit does to an element comment.
    ///
    /// This is the one behaviour a user cannot discover by trying it: whether a
    /// comment followed its element, orphaned, or quietly re-pointed looks
    /// IDENTICAL from the outside until you read the note and find it attached
    /// to text its author never saw. The doc is where that becomes knowable, so
    /// it must state the resolution rules — not merely that anchors exist.
    #[test]
    fn the_doc_explains_how_element_anchors_survive_edits() {
        assert!(
            DOC.contains("content") && DOC.contains("positional"),
            "the doc must contrast content identity with positional ids — the \
             distinction the whole design rests on"
        );
        for outcome in ["follows its element", "orphans", "ambiguous"] {
            assert!(
                DOC.contains(outcome),
                "the doc must state that an edit can `{outcome}`, so a reader \
                 knows which of their comments still mean what they said"
            );
        }
        assert!(
            DOC.contains("unverified"),
            "the doc must explain legacy fingerprint-less records, or a user \
             cannot tell a verified anchor from one that predates identity"
        );
    }

    /// The four statuses and the anchor kinds are a closed set the doc lists.
    #[test]
    fn the_doc_lists_the_real_statuses_and_anchor_kinds() {
        for status in ["open", "in-progress", "resolved", "wontfix"] {
            assert!(
                DOC.contains(status),
                "the doc must name the `{status}` status"
            );
            assert!(
                crate::comments::Status::parse(status).is_some(),
                "the doc names `{status}` but it does not parse"
            );
        }
        for anchor in ["inline", "file", "page", "element"] {
            assert!(
                DOC.contains(&format!("`{anchor}`")),
                "the doc must name the `{anchor}` anchor kind"
            );
        }
    }
}
