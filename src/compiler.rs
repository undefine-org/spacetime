//! Compiler: Transforms Spacetime definitions into executable JS/CSS
//!
//! The compilation pipeline is:
//! ```text
//! Parse -> FormMatch -> Resolve -> Sort -> Expand(ExpandedPrimitive) -> Emit
//! ```
//!
//! Use `Compiler::from_ast()` or `Compiler::from_file()` to build and compile.
//!
//! ## Output Validation
//!
//! When the `headless` feature is enabled, the compiler can validate generated
//! JS and CSS for syntax errors. Use `validate_output()` to check the compiled
//! output and get diagnostics for any syntax errors.

use std::path::Path;
use std::sync::{LazyLock, Mutex};

use crate::debugger::DebuggerConfig;
use crate::metasystem::{MetaRegistry, MetaRegistryErrorKind};
use crate::parser::{CssDeclaration, NestedScope, ScopeBlock, SourceSpan, StFile};
use crate::pipeline;

/// Controls where the stdlib MetaRegistry comes from during compilation.
#[derive(Debug, Clone, Default)]
pub enum RegistrySource {
    /// Use the process-wide cached registry (default — fast for dev server / watch mode).
    #[default]
    Cached,
    /// Parse a fresh registry from the stdlib on every compilation.
    Fresh,
    /// Use a caller-provided registry (useful for tests or custom pipelines).
    Provided(MetaRegistry),
}

/// Single-entry compile cache. Content hash invalidates automatically.
pub struct CompileCache {
    entry: std::sync::Mutex<Option<CacheEntry>>,
}

struct CacheEntry {
    content_hash: u64,
    stdlib_mtime_hash: u64,
    result: CompiledSpacetime,
}

impl Default for CompileCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CompileCache {
    pub fn new() -> Self {
        Self {
            entry: std::sync::Mutex::new(None),
        }
    }
}

/// Compute a fast content hash of an StFile for cache invalidation.
fn hash_stfile(ast: &StFile) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    // Hash the debug representation as a stable proxy for content equality.
    // StFile doesn't impl Hash, but its Debug output captures all fields.
    format!("{:?}", ast).hash(&mut hasher);
    hasher.finish()
}

/// Compute a hash of stdlib file modification times.
/// Used to invalidate the CompileCache when stdlib files change on disk.
/// Results are cached for 1 second to avoid repeated filesystem scans.
///
/// PLAN-123: `site_dir`'s `_prelude.st` participates too. The project overlay
/// feeds the SAME registry the stdlib does, so editing a `%comment_type`
/// there must invalidate exactly what editing a stdlib entry invalidates —
/// otherwise a dev-server author declares a type and the roster keeps serving
/// the previous one until restart.
fn hash_stdlib_mtimes_with_overlay(site_dir: Option<&Path>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let base = hash_stdlib_mtimes();
    let Some(dir) = site_dir else { return base };

    let mut hasher = DefaultHasher::new();
    base.hash(&mut hasher);
    let prelude = dir.join(PROJECT_PRELUDE);
    // Presence itself is part of the key: ADDING or REMOVING a prelude must
    // invalidate as surely as editing one.
    prelude.exists().hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(&prelude)
        && let Ok(mtime) = meta.modified()
    {
        mtime.hash(&mut hasher);
    }
    hasher.finish()
}

/// Compute a hash of stdlib file modification times.
/// Used to invalidate the CompileCache when stdlib files change on disk.
/// Results are cached for 1 second to avoid repeated filesystem scans.
fn hash_stdlib_mtimes() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{Duration, Instant};

    static CACHED: Mutex<Option<(Instant, u64)>> = Mutex::new(None);

    // Return cached value if less than 1 second old
    if let Ok(guard) = CACHED.lock()
        && let Some((when, hash)) = *guard
        && when.elapsed() < Duration::from_secs(1)
    {
        return hash;
    }

    let mut hasher = DefaultHasher::new();
    for dir in crate::metasystem::STDLIB_DIRS {
        if let Ok(entries) = std::fs::read_dir(dir) {
            // Collect and sort for deterministic ordering
            let mut paths: Vec<_> = entries.flatten().collect();
            paths.sort_by_key(|e| e.path());
            for entry in paths {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        mtime.hash(&mut hasher);
                    }
                    entry.path().hash(&mut hasher);
                }
            }
        }
    }
    let hash = hasher.finish();

    // Cache the result
    if let Ok(mut guard) = CACHED.lock() {
        *guard = Some((Instant::now(), hash));
    }

    hash
}

/// FUP-149 (PLAN-119 W3): give every bodyless `@data subscribe` a `seed` capture
/// so its `%binds` `local-state-impl(initial: $seed)` has a value to thread.
///
/// - TYPED subscribe (`$x FormEnvelope from …`): seed = the type's ZERO VALUE,
///   built structurally (`TypeRegistry::zero_value`) and serialized to a JS
///   literal. This makes (possibly nested) reads TOTAL from mount instead of
///   guarded against a null seed — and, because the value is built as an AST and
///   serialized (never re-parsed), a nested envelope works here even though a
///   hand-written nested `{ initial: … }` literal is still blocked by the
///   balanced-capture limit (BUG-228 / PLAN-120).
/// - UNTYPED subscribe: seed = `null` (byte-identical to the pre-FUP-149
///   hardcoded `initial: null`).
///
/// A subscribe carrying an explicit `{ initial: … }` body is the DISTINCT
/// `data-subscribe-seeded` macro and already has a `seed` capture; it is skipped
/// (the author's seed wins). The first server assign diff overwrites the seed
/// regardless, so it is never authoritative server data.
fn inject_subscribe_zero_value_seeds(matches: &mut [crate::syntax::FormMatch]) {
    use crate::syntax::CapturedValue;
    // Build the type registry ONCE from the page's @type defs (immutable borrow),
    // then resolve each seed before the mutable pass (avoids aliasing the slice).
    let registry = crate::type_system::TypeRegistry::from_form_matches(matches);
    for fm in matches.iter_mut() {
        // Only the bodyless subscribe macro; the seeded variant already has $seed.
        if fm.matched_macro.as_deref() != Some("data-subscribe") || fm.has("seed") {
            continue;
        }
        // Resolve the seed value: type zero-value if typed+resolvable, else null.
        let seed_js = match fm.type_name("type") {
            Some(t) if !t.is_empty() && t != "any" => {
                let type_expr = crate::type_system::parse_type_ref_to_expr(t);
                let zero = registry.zero_value(&type_expr);
                if zero.is_null() {
                    "null".to_string()
                } else {
                    zero.to_string()
                }
            }
            _ => "null".to_string(),
        };
        fm.captures
            .insert("seed".to_string(), CapturedValue::Expr(seed_js));
    }
}

/// Options for compilation.
///
/// Prefer using the [`Compiler`] builder for new code.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Enable trace logging in generated code
    pub trace: bool,
    /// Optional debugger configuration
    pub debug_config: Option<DebuggerConfig>,
    /// Where to obtain the stdlib MetaRegistry (default: `Cached`)
    pub registry_source: RegistrySource,
    /// Behave as if the project's @version were this wave date (`check
    /// --at-version`): overrides the @version fact read from the source for
    /// the pipeline's retired-syntax tri-branch (E0911 inert-wave check).
    pub version_override: Option<String>,
    /// Whether to wrap emitted JS with the Spacetime runtime (default: `true`)
    pub include_runtime: bool,
    /// Site root for compile-time hydration (Stage-3 Wave-6). When set, file-
    /// sourced `@data fetch … : "/data/x.json"` arrays are READ from disk at
    /// build time and their `@each` rows are unrolled into static HTML (prerender
    /// == runtime-hydrate). `None` keeps compilation pure/file-free (the default;
    /// dynamic sources stay runtime-only).
    pub site_dir: Option<std::path::PathBuf>,
    /// Emit reactive scope-binding JS (the `local:<sig>:updated` listeners for
    /// `.sel { .cls: $sig }` / `text <- $sig`) BEFORE the pipeline directive JS
    /// rather than after (BUG-119 §2). Set ONLY for a test-body sub-program
    /// (`compile_block_body_to_js`): there the body's `@when click` runs inline and
    /// would otherwise fire BEFORE the binding listener is registered, so the
    /// reactive class/text never reacts to the click. A normal page loads all init
    /// synchronously before any interaction, so ordering is immaterial there and the
    /// default stays `false` (bindings last, preserving base-style cascade order).
    pub bindings_first: bool,
    /// Development-only template invocation root stamps for the inspector.
    pub inspector_node_stamps: bool,
    /// PLAN-076: products of the compile-time migration shim (`Compiler::from_file`),
    /// threaded into the final output.
    pub migration_outcome: MigrationOutcome,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            trace: false,
            debug_config: None,
            registry_source: RegistrySource::default(),
            version_override: None,
            include_runtime: true,
            site_dir: None,
            bindings_first: false,
            inspector_node_stamps: false,
            migration_outcome: MigrationOutcome::default(),
        }
    }
}

/// PLAN-076: what the compile-time migration shim produced for one compile —
/// diagnostics (W0715 per applied match; E0911/E0912 for authoring errors) and
/// the pending (not-yet-persisted) entries the pill/CLI/check read.
#[derive(Debug, Clone, Default)]
pub struct MigrationOutcome {
    /// Shim diagnostics (error-severity ones are promoted to pipeline errors).
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
    /// Pending migration entries (what a persisted apply will change).
    pub pending: Vec<crate::migrate::PendingMigration>,
    /// True when the from_file shim actually ran (its pending rows carry
    /// PRE-shim on-disk spans). When false (source-less from_ast compiles),
    /// the pipeline falls back to `collect_hint_pending` on the AST.
    pub shim_ran: bool,
}

impl CompileOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_trace(mut self, trace: bool) -> Self {
        self.trace = trace;
        self
    }

    pub fn with_version_override(mut self, version: Option<String>) -> Self {
        self.version_override = version;
        self
    }

    pub fn with_debug(mut self, config: DebuggerConfig) -> Self {
        self.debug_config = Some(config);
        self
    }

    /// Use a freshly-parsed stdlib registry instead of the cached one.
    pub fn with_fresh_registry(mut self) -> Self {
        self.registry_source = RegistrySource::Fresh;
        self
    }

    /// Use a caller-provided MetaRegistry instead of loading the stdlib.
    pub fn with_registry(mut self, registry: MetaRegistry) -> Self {
        self.registry_source = RegistrySource::Provided(registry);
        self
    }
}

/// Compiled output ready to serve
#[derive(Debug, Clone)]
pub struct CompiledSpacetime {
    /// CSS to inject (custom properties, keyframes)
    pub css: String,
    /// JavaScript runtime
    pub js: String,
    /// Body HTML produced from top-level HTML element literals (PLAN-023 W1).
    /// Empty unless the .st source contains first-class `<tag>` markup at file scope.
    pub html: String,
    /// Build-time scripts from %emit build-js blocks
    pub build_scripts: Vec<String>,
    /// Source map for CSS output (V3 JSON format)
    pub css_source_map: Option<String>,
    /// Source map for JavaScript output (V3 JSON format)
    pub js_source_map: Option<String>,
    /// Pipeline errors (for routing to validation UI)
    pub pipeline_errors: Vec<PipelineErrorInfo>,
    /// PLAN-076: pending (not-yet-persisted) syntax migrations for this
    /// project — the single source the pill status route, `check`, and the
    /// CLI read.
    pub pending_migrations: Vec<crate::migrate::PendingMigration>,
    /// PLAN-076: warning-severity shim diagnostics (W0715: old syntax
    /// auto-migrated in memory — persist via the pill or `spacetime migrate`).
    pub migration_warnings: Vec<PipelineErrorInfo>,
}

/// Information about a pipeline compilation error
#[derive(Debug, Clone)]
pub struct PipelineErrorInfo {
    /// Error code (e.g., "E0801" for missing parameter)
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
    /// Byte offset span in user source code (for line/column conversion)
    pub span: Option<SourceSpan>,
    /// Name of the macro being processed when error occurred (e.g., "template")
    pub macro_name: Option<String>,
    /// File path to the stdlib macro definition (e.g., "stdlib/macros/template.st")
    pub macro_file: Option<String>,
    /// CSS selector context if available (e.g., ".card-section")
    pub selector: Option<String>,
    /// Location of the failing bind clause in the macro definition
    pub bind_span: Option<SourceSpan>,
    /// Name of the primitive in the failing bind clause (e.g., "register-template")
    pub bind_primitive: Option<String>,
    /// Parameters the user actually provided in their macro call
    pub provided_params: Option<Vec<String>>,
    /// Parameters expected by the failing bind clause
    pub expected_params: Option<Vec<String>>,
    /// Name of the primitive being expanded when error occurred (for expand errors)
    pub primitive_name: Option<String>,
    /// File path to the primitive definition (for expand errors)
    pub primitive_file: Option<String>,
}

// ============================================================================
// Compiler builder
// ============================================================================

/// Builder for compiling Spacetime source.
///
/// # Examples
///
/// ```rust,ignore
/// // From a parsed AST:
/// let compiled = Compiler::from_ast(&ast).compile();
///
/// // From a file on disk:
/// let compiled = Compiler::from_file(path, workspace_root)?.compile();
///
/// // With options:
/// let compiled = Compiler::from_ast(&ast)
///     .trace(true)
///     .debug(config)
///     .compile();
/// ```
pub struct Compiler<'a> {
    input: CompilerInput<'a>,
    trace: bool,
    debug_config: Option<DebuggerConfig>,
    include_runtime: bool,
    registry_source: RegistrySource,
    version_override: Option<String>,
    cache: Option<&'a CompileCache>,
    site_dir: Option<std::path::PathBuf>,
    bindings_first: bool,
    /// Emit development-only structure ids on template invocation roots.
    inspector_node_stamps: bool,
    /// PLAN-076: shim products collected while building this compiler
    /// (`from_file`; empty for `from_ast` — the shim needs source text).
    migration_outcome: MigrationOutcome,
    /// PLAN-076: hash of the ORIGINAL on-disk content (pre-shim). The
    /// CompileCache MUST key on this, not the post-shim AST: an old-syntax
    /// file and its already-migrated twin shim to the SAME AST, and keying
    /// on that AST would serve the clean file the twin's stale pending
    /// entries and W0715 warnings.
    source_hash: Option<u64>,
}

enum CompilerInput<'a> {
    Ast(&'a StFile),
    Owned(StFile),
}

impl<'a> Compiler<'a> {
    /// Create a compiler from a pre-parsed AST.
    pub fn from_ast(ast: &'a StFile) -> Self {
        Self {
            input: CompilerInput::Ast(ast),
            trace: false,
            debug_config: None,
            include_runtime: true,
            registry_source: RegistrySource::Cached,
            version_override: None,
            cache: None,
            site_dir: None,
            bindings_first: false,
            inspector_node_stamps: false,
            migration_outcome: MigrationOutcome::default(),
            source_hash: None,
        }
    }

    /// Emit reactive scope-binding JS before the directive JS (BUG-119 §2).
    /// Set for a test-body sub-program so a body-inline `@when click` does not
    /// fire before the `local:<sig>:updated` binding listener is registered.
    pub fn bindings_first(mut self, yes: bool) -> Self {
        self.bindings_first = yes;
        self
    }

    /// Emit structure ids only for the dev server's inspector selection rail.
    pub fn inspector_node_stamps(mut self, enabled: bool) -> Self {
        self.inspector_node_stamps = enabled;
        self
    }

    /// Set the site root for compile-time hydration (Stage-3 Wave-6): file-
    /// sourced `@data` arrays are read from disk and their `@each` rows unrolled
    /// into static HTML at build time.
    pub fn with_site_dir(mut self, dir: Option<std::path::PathBuf>) -> Self {
        self.site_dir = dir;
        self
    }

    /// EDN ingress for file compilation (PLAN-148 in serve/check/build).
    ///
    /// EDN is a peer concrete syntax over the same `%form` registry: an `.edn`
    /// file is lifted straight to an `StFile` — NEVER through the printer,
    /// whose coverage must not gate expressibility (the same rule MCP's
    /// `compile_pipeline` follows, `src/mcp/bundle.rs`). Returns `None` for
    /// any other extension, so the reference path gains no branch.
    ///
    /// Gating is by EXTENSION, not `edn::detect`: detection is a
    /// content-heuristic for inline sources that have no filename (MCP), and
    /// its `[`/`{` arm consults a whole-source `:st/` substring — a `.st`
    /// file like `[data-role] { content: ":st/x"; }` would misfire. A file
    /// has a name; the name decides.
    pub fn edn_ingress(path: &Path, content: &str) -> Option<Result<StFile, String>> {
        if path.extension().and_then(|e| e.to_str()) == Some("edn") {
            Some(crate::edn::to_st_file(
                content,
                &crate::syntax::STDLIB_REGISTRY,
            ))
        } else {
            None
        }
    }

    /// Create a compiler from a file on disk (reads, parses, resolves imports).
    pub fn from_file(path: &Path, workspace_root: &Path) -> Result<Compiler<'static>, String> {
        crate::profile_span!("compiler_from_file", path = %path.display());
        // SIP-002: a literate document (`*.st.md`) is TANGLED to ordinary `.st`
        // source before anything else sees it — prose becomes `@doc(content:)`
        // sections, `st` fences splice through verbatim, document order kept.
        // Wiring the single central read means serve/check/build/export all
        // gain literate support without a parallel pipeline.
        let (content, line_map) = crate::literate::read_source(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        // EDN is a peer concrete syntax (PLAN-148): an `.edn` page enters the
        // pipeline as an `StFile` here, at the same point `.st` text would have
        // produced one. Gated by EXTENSION (see edn_ingress), so ordinary `.st`
        // source can never wander into the EDN reader.
        let is_edn = path.extension().and_then(|e| e.to_str()) == Some("edn");
        let main_ast = if is_edn {
            Self::edn_ingress(path, &content)
                .expect("extension said .edn")
                .map_err(|e| format!("EDN source {}: {}", path.display(), e))?
        } else {
            crate::profile_span!("parse");
            crate::parser::parse(&content).map_err(|e| {
                // SIP-002 W4: for a literate entry the parser saw TANGLED text,
                // so its offsets address lines the author never wrote. Remap
                // them into `.st.md` coordinates and render against the
                // ORIGINAL document — the reader gets their own file back.
                match &line_map {
                    Some(map) => {
                        let original = std::fs::read_to_string(path).unwrap_or_default();
                        crate::literate::remap_parse_errors(&e, Some(map), &content, &original)
                            .render_all_plain(&original, &path.to_string_lossy())
                    }
                    None => e.render_all_plain(&content, &path.to_string_lossy()),
                }
            })?
        };
        let source_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            hasher.finish()
        };

        // PLAN-076: syntax-migration compat shim. Read the project's
        // `@version` fact, then rewrite-kind matches are rewritten IN MEMORY
        // (W0715 per match), wave by wave, BEFORE imports/rematch/provenance
        // see the AST. Hint-kind (and unsound multi-shape) matches stay in
        // the AST and the pipeline's retired-syntax check errors them
        // (E0910). The shim needs the stdlib registry for `%migration`
        // entries; the cached registry is the same source compile_pipeline
        // defaults to (RegistrySource::Provided registries with custom
        // migrations are an unsupported edge for the shim — documented).
        let mut migration_outcome = MigrationOutcome::default();
        let (content, main_ast) = if is_edn {
            // No shim for EDN: it projects the CURRENT `%form` registry, so
            // retired syntax cannot be expressed — and the shim rewrites `.st`
            // TEXT, which EDN source is not.
            (content, main_ast)
        } else {
            let version_fact = crate::migrate::read_syntax_version(&main_ast.matches);
            // (The fact's own integrity — duplicate decls, non-ISO date — is
            // enforced pipeline-side in check_retired_syntax so from_ast
            // compiles (`spacetime check`) agree with from_file.)
            let (migration_registry, _stdlib_errors) = cached_stdlib_registry();
            // HARD CUTOVER: a file that declares no `@version` compiles at the
            // CURRENT wave, not before every wave — so retired syntax in new code
            // is refused instead of silently rewritten. See
            // `migrate::effective_syntax_version`.
            let version = crate::migrate::effective_syntax_version(
                version_fact.version.clone(),
                &migration_registry,
            );
            let shimmed = crate::migrate::apply_migration_shim(
                content,
                main_ast,
                &migration_registry,
                version.as_deref(),
            );
            migration_outcome.diagnostics.extend(shimmed.diagnostics);
            migration_outcome.pending.extend(shimmed.pending);
            migration_outcome.shim_ran = true;
            (shimmed.source, shimmed.ast)
        };
        let mut ast = if !main_ast.imports.is_empty() {
            crate::profile_span!("resolve_imports");
            crate::parser::resolve_imports(&main_ast, path, workspace_root)
                .map_err(|e| format!("Import resolution failed: {}", e))?
        } else {
            // PLAN-117 W3: a page with NO imports never enters import
            // resolution, so the qualified-reference pass must run here too.
            // Without this an ambiguous `1/$denominator` (E0939) or a stray
            // qualifier in a single-file page would go unreported — the exact
            // silent-drop class this wave exists to close.
            let mut solo = main_ast;
            let diags = crate::pipeline::qualified_refs::resolve_qualified_refs(&mut solo);
            solo.diagnostics.extend(diags);
            solo
        };

        // Re-extract FormMatches if imports brought user-defined macros.
        // The initial parse only matched against STDLIB_REGISTRY; user macros
        // need a second pass with an augmented registry.
        if !ast.meta_defs.is_empty() {
            crate::profile_span!("rematch_user_macros");
            // Pass the main path: after import resolution the main file's scopes
            // are tagged with its path (not None), so rematch must target them.
            let main_sf = path.to_string_lossy().to_string();
            crate::parser::rematch_with_user_macros_in(&mut ast, &content, Some(&main_sf));
        }

        // Inject provenance attributes into template body HTML (debug builds only)
        // FEAT-119: provenance is injected into the World-A scope html (the emit
        // source), keyed by template name, using each match's hash metadata.
        let prov_matches = ast.matches.clone();
        crate::syntax::inject_provenance_into_matches(
            &prov_matches,
            &mut ast.scopes,
            &path.to_string_lossy(),
            workspace_root,
        );

        // PLAN-135 W4: tag the main file's OWN matches with its path, the
        // same stamp import resolution puts on imported matches. A page with
        // no imports never passes through that tagging, and the `@data forms`
        // catalog's `source` column would read "" for the page's own
        // declarations — the one file the author knows best.
        let main_path = path.to_string_lossy().to_string();
        let mut stamp = |m: &mut crate::syntax::FormMatch| {
            if m.source_file.is_none() {
                m.source_file = Some(main_path.clone());
            }
        };
        for m in ast.matches.iter_mut() {
            stamp(m);
        }
        for scope in ast.scopes.iter_mut() {
            for m in scope.matches.iter_mut() {
                stamp(m);
            }
        }

        Ok(Compiler {
            input: CompilerInput::Owned(ast),
            trace: false,
            debug_config: None,
            include_runtime: true,
            registry_source: RegistrySource::default(),
            version_override: None,
            cache: None,
            site_dir: None,
            bindings_first: false,
            inspector_node_stamps: false,
            migration_outcome,
            source_hash: Some(source_hash),
        })
    }

    /// Enable trace logging in generated code.
    pub fn trace(mut self, enabled: bool) -> Self {
        self.trace = enabled;
        self
    }

    /// `check --at-version`: behave as if the project's @version were this
    /// wave date in the pipeline's retired-syntax tri-branch.
    pub fn with_version_override(mut self, version: Option<String>) -> Self {
        self.version_override = version;
        self
    }

    /// Enable debugger with the given configuration.
    pub fn debug(mut self, config: DebuggerConfig) -> Self {
        self.debug_config = Some(config);
        self
    }

    /// Disable runtime inclusion in output.
    pub fn without_runtime(mut self) -> Self {
        self.include_runtime = false;
        self
    }

    /// Use a freshly-parsed stdlib registry instead of the cached one.
    pub fn fresh_registry(mut self) -> Self {
        self.registry_source = RegistrySource::Fresh;
        self
    }

    /// Use a caller-provided MetaRegistry instead of loading the stdlib.
    pub fn registry(mut self, registry: MetaRegistry) -> Self {
        self.registry_source = RegistrySource::Provided(registry);
        self
    }

    /// Use a fresh stdlib registry instead of the cached one (when `false`).
    pub fn cache_stdlib(mut self, cache: bool) -> Self {
        if !cache {
            self.registry_source = RegistrySource::Fresh;
        }
        self
    }

    /// Enable debugger if the config is present and enabled.
    pub fn debug_if(self, config: Option<&DebuggerConfig>) -> Self {
        match config {
            Some(cfg) if cfg.enabled => self.debug(cfg.clone()),
            _ => self,
        }
    }

    /// Attach a compile cache for content-hash dedup.
    pub fn with_cache(mut self, cache: &'a CompileCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// The fully import-resolved AST — the same merged tree `compile()` emits.
    /// `from_file` resolves imports into this tree (`resolve_imports` flattens
    /// imported matches, scopes, markup, …), so a caller reading totals or
    /// structure from it sees the COMPLETE page, not just the entry file. The
    /// render backend uses this to compute score durations against exactly what
    /// the compiler emits, instead of re-parsing the entry alone.
    pub fn merged_ast(&self) -> &StFile {
        match &self.input {
            CompilerInput::Ast(ast) => ast,
            CompilerInput::Owned(ast) => ast,
        }
    }

    /// Compile and return the output.
    pub fn compile(self) -> CompiledSpacetime {
        // If cache is set and input is Owned, check content hash + stdlib mtimes
        if let Some(cache) = self.cache
            && let CompilerInput::Owned(ref ast) = self.input
        {
            // PLAN-076: key on the PRE-SHIM source hash when present (an
            // old-syntax file and its migrated twin share a post-shim AST).
            let hash = self.source_hash.unwrap_or_else(|| hash_stfile(ast));
            // PLAN-123: the project overlay feeds the same registry, so its
            // mtime belongs in the same key — otherwise editing a
            // `%comment_type` in `_prelude.st` serves a stale roster from
            // cache until restart.
            let stdlib_hash = hash_stdlib_mtimes_with_overlay(self.site_dir.as_deref());
            {
                let guard = cache.entry.lock().unwrap();
                if let Some(entry) = guard.as_ref()
                    && entry.content_hash == hash
                    && entry.stdlib_mtime_hash == stdlib_hash
                {
                    return entry.result.clone();
                }
            }

            let result = self.compile_inner();
            *cache.entry.lock().unwrap() = Some(CacheEntry {
                content_hash: hash,
                stdlib_mtime_hash: stdlib_hash,
                result: result.clone(),
            });
            return result;
        }
        self.compile_inner()
    }

    /// Internal: dispatch to the right compilation path.
    fn compile_inner(self) -> CompiledSpacetime {
        let options = CompileOptions {
            trace: self.trace,
            debug_config: self.debug_config,
            registry_source: self.registry_source,
            version_override: self.version_override.clone(),
            include_runtime: self.include_runtime,
            site_dir: self.site_dir,
            bindings_first: self.bindings_first,
            inspector_node_stamps: self.inspector_node_stamps,
            migration_outcome: self.migration_outcome,
        };
        match self.input {
            CompilerInput::Ast(ast) => compile_pipeline(ast, options),
            CompilerInput::Owned(ref ast) => compile_pipeline(ast, options),
        }
    }
}

/// Compile a parsed AST into CSS + JS output.
///
/// Uses the 5-layer pipeline:
/// 1. Adapter: StFile → Vec<FormMatch>
/// 2. Resolve: FormMatch → ResolvedPrimitive
/// 3. Sort: Order by (phase, order)
/// 4. Expand: ResolvedPrimitive → ExpandedPrimitive
/// 5. Emit: ExpandedPrimitive → PipelineOutput
pub fn compile(ast: &StFile, options: CompileOptions) -> CompiledSpacetime {
    compile_pipeline(ast, options)
}

/// Load the stdlib registry for macro compilation, collecting any errors.
///
/// First tries to load from the filesystem (for CLI usage where stdlib/ exists).
/// Falls back to embedded stdlib when filesystem loading fails (for library usage).
pub fn load_stdlib_registry() -> (MetaRegistry, Vec<pipeline::StdlibLoadError>) {
    let mut registry = MetaRegistry::new();
    let mut all_errors = Vec::new();

    // Load each stdlib directory, collecting all errors
    let dirs = crate::metasystem::STDLIB_DIRS;

    // Check if any stdlib directory exists
    let any_dir_exists = dirs.iter().any(|dir| Path::new(dir).exists());

    if any_dir_exists {
        // Load from filesystem
        for dir in dirs {
            let result = registry.load_stdlib_collecting_errors(Path::new(dir));
            all_errors.extend(result.errors);
        }
    } else {
        // Fallback to embedded stdlib
        log::debug!("Stdlib directories not found, using embedded stdlib");
        load_embedded_stdlib(&mut registry, &mut all_errors);
    }

    // Build the provider index from %binds outputs for error messages
    registry.rebuild_provider_index();

    // PLAN-076: post-load migration validation — ORDER-INDEPENDENT checks
    // (forward-only chains, intra-wave overlap) that need EVERY wave
    // registered before they can run.
    all_errors.extend(migration_chain_load_errors(&registry));
    // PLAN-123: same rationale, same place — roster checks that need every
    // entry registered before they can be judged.
    all_errors.extend(comment_type_load_errors(&registry));
    // FEAT-168 / PLAN-122 W4: same rationale, same place — a malformed scalar
    // row (missing required key, unknown sub-clause) must fail here, where the
    // whole table is in view.
    all_errors.extend(scalar_type_load_errors(&registry));
    // I4 / gh-18: every %form capture has a consumer. Registry-level lint over
    // all macros, in view of the whole registry (runs after every file loads).
    all_errors.extend(capture_consumption_load_errors(&registry));

    (registry, all_errors)
}

/// PLAN-123: post-load validation of the comment-type roster.
///
/// Runs where `migration_chain_load_errors` runs and for the same reason:
/// these checks are ORDER-DEPENDENT across files (a project's `_prelude.st`
/// loads after the stdlib), so they can only be judged once every entry is
/// registered. Parsing itself stays total.
///
/// What is enforced, and why each one is a REAL failure rather than fussiness:
/// - `%label` / `%docs` present: the roster is a picker in the pill and a
///   contract for agents. An unexplained type is unusable in both.
/// - field kinds known: a typo'd kind (`%field due date`) would otherwise
///   drop the field from the shape, and the author would later see
///   "accepts no field `due`" pointing at their comment instead of at the
///   declaration that is actually wrong.
/// - required-before-optional: the picker and the `//@type(…)` examples read
///   left to right; an optional field ahead of a required one makes the
///   generated example misleading.
pub(crate) fn comment_type_load_errors(registry: &MetaRegistry) -> Vec<pipeline::StdlibLoadError> {
    let mut errors = Vec::new();
    for t in registry.comment_types() {
        let file = t
            .source_file
            .clone()
            .unwrap_or_else(|| "stdlib/comments/entries".to_string());
        let mut push = |message: String| {
            errors.push(pipeline::StdlibLoadError {
                file_path: std::path::PathBuf::from(&file),
                span: Some(t.span),
                message,
                kind: pipeline::StdlibLoadErrorKind::ParseError,
            });
        };

        if t.label.trim().is_empty() {
            push(format!(
                "comment type `{}` has no %label — every type must name itself for the \
                 pill's picker and `check` output",
                t.id
            ));
        }
        if t.docs.trim().is_empty() {
            push(format!(
                "comment type `{}` has no %docs — every type must say what it MEANS, or \
                 nobody (human or agent) can choose it correctly",
                t.id
            ));
        }
        for (field, written) in &t.unknown_field_kinds {
            push(format!(
                "comment type `{}`: field `{field}` declares unknown kind `{written}` — \
                 valid kinds are string, number, bool, ident (add `?` for optional)",
                t.id
            ));
        }
        if let Some(first_optional) = t.fields.iter().position(|f| f.optional)
            && let Some(late_required) = t.fields[first_optional..].iter().find(|f| !f.optional)
        {
            push(format!(
                "comment type `{}`: required field `{}` is declared after an optional one — \
                 declare required fields first so the generated `//@{}(…)` example reads \
                 in order",
                t.id, late_required.name, t.id
            ));
        }
    }
    errors
}

/// I4 / gh-18: registry-level capture-consumption lint.
///
/// Every `%form` capture a macro binds must be CONSUMED somewhere — passed to a
/// primitive in `%binds`, referenced in an `%emit` block, or read in a
/// `%when`/`%derives`/`%includes`/`%states` clause. A capture that is bound and
/// never read is a half-implemented macro: the author's intent (a keyframes
/// body, a selector, a threshold) silently vanishes at compile time with zero
/// diagnostics. The information is already in the registry — the `%form` records
/// every capture and the body records every reference — so this is a lint over
/// DATA, not new machinery.
///
/// A capture is declared intentionally-unused by prefixing its name with `_`
/// (the established convention for `$_:skip_block`). Runs where every other
/// post-load check runs, in view of the whole registry.
/// Whether a byte is part of an identifier run for the capture-consumption
/// lint's word-boundary check. `$`/`%`/`&` are sigil boundaries (a different
/// token), alphanumeric/`_`/`-` continue the run.
fn is_cap_boundary(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

pub(crate) fn capture_consumption_load_errors(
    registry: &MetaRegistry,
) -> Vec<pipeline::StdlibLoadError> {
    use crate::parser::meta_ast::*;
    use std::collections::HashSet;

    // ---- capture enumeration ----
    fn param_captures(p: &FormParam, out: &mut HashSet<String>) {
        if let Some(cap) = p.capture() {
            out.insert(cap.var_name.clone());
            if let Some(a) = &cap.alias_capture {
                out.insert(a.var_name.clone());
            }
        }
        for el in &p.elements {
            inline_captures(el, out);
        }
    }
    fn inline_captures(el: &FormInlineElement, out: &mut HashSet<String>) {
        match el {
            FormInlineElement::Capture(cap, _) => {
                out.insert(cap.var_name.clone());
                if let Some(a) = &cap.alias_capture {
                    out.insert(a.var_name.clone());
                }
            }
            FormInlineElement::Comparison { capture, .. } => {
                out.insert(capture.var_name.clone());
            }
            FormInlineElement::Group { elements, .. } => {
                for e in elements {
                    inline_captures(e, out);
                }
            }
            FormInlineElement::KeywordBlock { body_params, .. }
            | FormInlineElement::PseudoSelector { body_params, .. }
            | FormInlineElement::PseudoClass { body_params, .. } => {
                for p in body_params {
                    param_captures(p, out);
                }
            }
            FormInlineElement::Literal(_) => {}
        }
    }
    fn pattern_captures(p: &CapturePatternAst, out: &mut HashSet<String>) {
        match p {
            CapturePatternAst::Capture { var_name, .. } => {
                out.insert(var_name.clone());
            }
            CapturePatternAst::Literal(_) | CapturePatternAst::CharClass { .. } => {}
            CapturePatternAst::Group { pattern, .. } => pattern_captures(pattern, out),
            CapturePatternAst::Sequence(items) => {
                for i in items {
                    pattern_captures(i, out);
                }
            }
            CapturePatternAst::Choice(items) => {
                for i in items {
                    pattern_captures(i, out);
                }
            }
        }
    }
    // A capture whose type is a Union (a fixed keyword set) is a GRAMMAR
    // disambiguator — `$kind:("motion")` selects which macro variant a directive
    // call matches, it never carries author value to forward. Exempt it.
    fn is_union(cap: &FormCapture) -> bool {
        matches!(&cap.capture_type, CaptureType::Union(_))
    }
    // A `typeref` capture (`$x:typeref`) is a COMPILE-TIME type annotation used
    // by the type system (e.g. `local-state`'s `$type:typeref`) — it is never a
    // runtime value to forward, so it is not a consumption violation.
    fn is_typeref(cap: &FormCapture) -> bool {
        matches!(&cap.capture_type, CaptureType::Typeref)
    }
    fn param_exempt(p: &FormParam) -> bool {
        p.capture().is_some_and(|c| is_union(c) || is_typeref(c))
    }
    // `body_capture` arrives as the RAW body string incl. its outer `{ }`
    // (refine_body_capture keeps it verbatim), e.g. `{ $body:keyframes }`.
    // Extract every `$ident` capture name from it.
    fn body_capture_names(raw: &str, out: &mut HashSet<String>) {
        let mut rest = raw;
        while let Some(pos) = rest.find('$') {
            let after = &rest[pos + 1..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let nlen = name.chars().count();
            if !name.is_empty() && name.chars().next().unwrap().is_ascii_alphabetic() {
                out.insert(name);
            }
            rest = &after[nlen.min(after.len())..];
        }
    }
    fn form_captures(form: &FormClause) -> (HashSet<String>, HashSet<String>) {
        // returns (all captures, exempt captures)
        let mut names = HashSet::new();
        let mut exempt = HashSet::new();
        for el in &form.inline_elements {
            collect_inline(el, &mut names, &mut exempt);
        }
        for el in &form.post_arg_inline {
            collect_inline(el, &mut names, &mut exempt);
        }
        for p in &form.params {
            collect_param(p, &mut names, &mut exempt);
        }
        for p in &form.body_params {
            collect_param(p, &mut names, &mut exempt);
        }
        for g in &form.body_groups {
            pattern_captures(g, &mut names);
        }
        if let Some(bc) = &form.body_capture {
            body_capture_names(bc, &mut names);
        }
        (names, exempt)
    }
    fn collect_inline(
        el: &FormInlineElement,
        names: &mut HashSet<String>,
        exempt: &mut HashSet<String>,
    ) {
        match el {
            FormInlineElement::Capture(cap, _) => {
                names.insert(cap.var_name.clone());
                if is_union(cap) || is_typeref(cap) {
                    exempt.insert(cap.var_name.clone());
                }
                if let Some(a) = &cap.alias_capture {
                    names.insert(a.var_name.clone());
                }
            }
            FormInlineElement::Comparison { capture, .. } => {
                names.insert(capture.var_name.clone());
            }
            FormInlineElement::Group { elements, .. } => {
                for e in elements {
                    collect_inline(e, names, exempt);
                }
            }
            FormInlineElement::KeywordBlock { body_params, .. }
            | FormInlineElement::PseudoSelector { body_params, .. }
            | FormInlineElement::PseudoClass { body_params, .. } => {
                for p in body_params {
                    collect_param(p, names, exempt);
                }
            }
            FormInlineElement::Literal(_) => {}
        }
    }
    fn collect_param(p: &FormParam, names: &mut HashSet<String>, exempt: &mut HashSet<String>) {
        if param_exempt(p) {
            if let Some(cap) = p.capture() {
                names.insert(cap.var_name.clone());
                exempt.insert(cap.var_name.clone());
            }
        } else if let Some(cap) = p.capture() {
            names.insert(cap.var_name.clone());
            if let Some(a) = &cap.alias_capture {
                names.insert(a.var_name.clone());
            }
        }
        for el in &p.elements {
            collect_inline(el, names, exempt);
        }
    }

    // ---- consumption surface (everything except the %form) ----
    fn bind_value_text(v: &BindValue, out: &mut String) {
        match v {
            BindValue::Variable(n) => {
                out.push_str("$");
                out.push_str(n);
                out.push(' ');
            }
            BindValue::FunctionCall { name, args } => {
                out.push_str(name);
                out.push(' ');
                for a in args {
                    bind_value_text(a, out);
                }
            }
            BindValue::Array(items) => {
                for i in items {
                    out.push_str("$");
                    out.push_str(i);
                    out.push(' ');
                }
            }
            BindValue::String(x) | BindValue::Ident(x) => {
                out.push_str(x);
                out.push(' ');
            }
            BindValue::Number(_) | BindValue::ElementRef(_) => {}
        }
    }
    fn push_binds(binds: &[BindDecl], out: &mut String) {
        for b in binds {
            out.push_str(&b.primitive);
            out.push(' ');
            for arg in &b.args {
                match arg {
                    BindArg::Element { name, .. } => {
                        out.push_str("&");
                        out.push_str(name);
                        out.push(' ');
                    }
                    BindArg::Named { name, value } => {
                        out.push_str(name);
                        out.push(' ');
                        bind_value_text(value, out);
                    }
                    BindArg::Positional(v) => bind_value_text(v, out),
                }
            }
            for o in &b.outputs {
                out.push_str("$");
                out.push_str(&o.name);
                out.push(' ');
                if let Some(a) = &o.alias {
                    out.push_str(a);
                    out.push(' ');
                }
            }
        }
    }
    fn push_state_props(props: &[MetaStateProperty], out: &mut String) {
        for p in props {
            out.push_str(&p.value);
            out.push(' ');
        }
    }
    fn push_if_cond(c: &MetaIfCondition, out: &mut String) {
        match c {
            MetaIfCondition::Truthy(v) => {
                out.push_str("$");
                out.push_str(v);
            }
            MetaIfCondition::Falsy(v) => {
                out.push_str("$");
                out.push_str(v);
            }
            MetaIfCondition::Equals(a, b)
            | MetaIfCondition::NotEquals(a, b)
            | MetaIfCondition::LessThan(a, b)
            | MetaIfCondition::GreaterThan(a, b)
            | MetaIfCondition::LessThanOrEqual(a, b)
            | MetaIfCondition::GreaterThanOrEqual(a, b) => {
                // LHS is the signal name (stored WITHOUT `$` in the AST) —
                // emit it sigiled + separated so the capture-consumption
                // scan's word-boundary check finds it standalone.
                out.push_str("$");
                out.push_str(a);
                out.push(' ');
                out.push_str(b);
            }
            MetaIfCondition::Or(a, b) | MetaIfCondition::And(a, b) => {
                push_if_cond(a, out);
                push_if_cond(b, out);
            }
        }
    }
    fn body_text(m: &MacroDefAst, out: &mut String) {
        push_binds(&m.binds, out);
        for d in &m.derives {
            out.push_str(&d.name);
            out.push_str(&d.expr);
            out.push(' ');
        }
        for r in &m.requires {
            out.push_str("$");
            out.push_str(r);
            out.push(' ');
        }
        if let Some(reg) = &m.registers {
            out.push_str(&reg.name);
            out.push(' ');
            for a in &reg.args {
                match a {
                    RegistersArg::Positional(v) => {
                        out.push_str("$");
                        out.push_str(v);
                        out.push(' ');
                    }
                    RegistersArg::Named { name, var } => {
                        out.push_str(name);
                        out.push_str("$");
                        out.push_str(var);
                        out.push(' ');
                    }
                }
            }
            for item in &reg.items {
                match item {
                    RegisterItem::Field(f) => match &f.value {
                        RegisterValue::Var(v) => {
                            out.push_str("$");
                            out.push_str(v);
                        }
                        RegisterValue::Array(arr) => {
                            for a in arr {
                                if let RegisterArrayItem::Var(v) = a {
                                    out.push_str("$");
                                    out.push_str(v);
                                }
                            }
                        }
                        RegisterValue::String(x) | RegisterValue::Ident(x) => out.push_str(x),
                        RegisterValue::Bool(_) => {}
                        RegisterValue::Properties(props) => {
                            for (_, v) in props {
                                out.push_str(v);
                            }
                        }
                    },
                    RegisterItem::VarRef(v) => {
                        out.push_str("$");
                        out.push_str(v);
                    }
                }
                out.push(' ');
            }
        }
        if let Some(imp) = &m.imports {
            out.push_str("$");
            out.push_str(&imp.module);
            out.push(' ');
            if let Some(a) = &imp.alias {
                out.push_str("$");
                out.push_str(a);
                out.push(' ');
            }
            for o in &imp.only {
                out.push_str("$");
                out.push_str(o);
                out.push(' ');
            }
            for h in &imp.hiding {
                out.push_str("$");
                out.push_str(h);
                out.push(' ');
            }
        }
        if let Some(res) = &m.resolves {
            for map in &res.mappings {
                out.push_str(&map.symbol);
                out.push_str(&map.registry);
                out.push(' ');
            }
        }
        if let Some(st) = &m.states {
            for def in &st.states {
                match def {
                    MetaStateDef::Named {
                        condition,
                        properties,
                        ..
                    } => {
                        if let Some(c) = condition {
                            out.push_str(c);
                            out.push(' ');
                        }
                        push_state_props(properties, out);
                    }
                    MetaStateDef::Variable {
                        name, properties, ..
                    } => {
                        out.push_str("$");
                        out.push_str(name);
                        out.push(' ');
                        push_state_props(properties, out);
                    }
                }
            }
        }
        push_body_items(&m.body, out);
    }
    fn push_body_items(items: &[MacroBodyItem], out: &mut String) {
        for item in items {
            match item {
                MacroBodyItem::When(w) => {
                    out.push_str(&w.condition);
                    out.push(' ');
                    push_body_items(&w.body, out);
                }
                MacroBodyItem::On(o) => {
                    match &o.trigger {
                        MetaOnTrigger::VarTransition { var, value } => {
                            out.push_str(var);
                            out.push_str(value);
                        }
                        MetaOnTrigger::VarEvent { var, .. } => {
                            out.push_str("$");
                            out.push_str(var);
                        }
                        MetaOnTrigger::Event(e) => out.push_str(e),
                    }
                    out.push(' ');
                    for b in &o.body {
                        match b {
                            MetaOnBodyItem::Assignment { var, expr } => {
                                out.push_str("$");
                                out.push_str(var);
                                out.push_str(expr);
                                out.push(' ');
                            }
                            MetaOnBodyItem::Action(a) => {
                                out.push_str(a);
                                out.push(' ');
                            }
                            MetaOnBodyItem::MacroItem(mi) => {
                                push_body_items(&[mi.as_ref().clone()], out)
                            }
                            MetaOnBodyItem::Emit(e) => {
                                out.push_str(&e.content);
                                out.push(' ');
                            }
                        }
                    }
                }
                MacroBodyItem::For(fc) => {
                    out.push_str("$");
                    out.push_str(&fc.variable);
                    out.push_str(&fc.source);
                    out.push(' ');
                    push_body_items(&fc.body, out);
                }
                MacroBodyItem::If(ifc) => {
                    push_if_cond(&ifc.condition, out);
                    push_body_items(&ifc.then_body, out);
                    for e in &ifc.elif_clauses {
                        push_if_cond(&e.condition, out);
                        push_body_items(&e.body, out);
                    }
                    if let Some(eb) = &ifc.else_body {
                        push_body_items(eb, out);
                    }
                }
                MacroBodyItem::Animates(a) => {
                    for p in &a.properties {
                        out.push_str(&p.variable);
                        out.push(' ');
                    }
                }
                MacroBodyItem::Applies(a) => {
                    for v in &a.variables {
                        out.push_str("$");
                        out.push_str(v);
                        out.push(' ');
                    }
                    push_state_props(&a.properties, out);
                }
                MacroBodyItem::Includes(inc) => {
                    for pat in &inc.patterns {
                        out.push_str(&pat.name);
                        out.push(' ');
                        for arg in &pat.args {
                            out.push_str(&arg.name);
                            out.push_str(&arg.value);
                            out.push(' ');
                        }
                    }
                }
                MacroBodyItem::Mutate(mut_) => {
                    out.push_str(&mut_.timing.state);
                    for op in &mut_.operations {
                        out.push_str(&op.name);
                        for a in &op.args {
                            out.push_str(&a.name);
                            out.push_str(&a.value);
                            out.push(' ');
                        }
                    }
                }
                MacroBodyItem::Trigger(t) => {
                    out.push_str(t);
                    out.push(' ');
                }
                MacroBodyItem::Emit(e) => {
                    out.push_str(&e.content);
                    out.push(' ');
                }
                MacroBodyItem::Binds(binds) => push_binds(binds, out),
            }
        }
    }

    let mut errors = Vec::new();
    for (name, m) in registry.iter_macros() {
        let Some(form) = &m.form else { continue };
        let (captures, exempt) = form_captures(form);
        if captures.is_empty() {
            continue;
        }
        // Declared markers exempt the whole macro: `%diagnostic` (the captures
        // detect an invalid shape and are consumed by the diagnostic it emits)
        // and `%pipeline-consumed` (the captures are read by the compile
        // pipeline or decomposed into a custom capture type's record whose
        // sub-keys are forwarded in %binds — neither is a name-reference the
        // scan can see). These are ASSERTED by the author, never inferred, so a
        // genuinely half-implemented macro cannot hide behind them.
        if m.diagnostic_only || m.pipeline_consumed {
            continue;
        }
        let mut text = String::new();
        body_text(m, &mut text);
        // A rewrite-rule macro's `%into` template consumes its `%match`
        // captures by interpolation (`$name` re-emitted into the new shape).
        if let Some(tpl) = &m.rewrite_template {
            text.push_str(tpl);
            text.push(' ');
        }
        // A capture is consumed if its bare name appears as a standalone
        // identifier ANYWHERE in the macro's non-form body — `$name` in a bind
        // arg, `name` in a %if/%when condition or %includes arg, `%$name` /
        // `%name` / `%&name` in an %emit block. Word-boundary aware so a
        // capture `duration` is not falsely "consumed" by a hyphenated
        // neighbour `my-duration` or a longer word `durability`.
        let has_ref = |cap: &str| {
            if cap.is_empty() {
                return false;
            }
            let wb = cap.as_bytes();
            let mut i = 0;
            let blen = text.len();
            let t = text.as_bytes();
            while i + wb.len() <= blen {
                if &t[i..i + wb.len()] == wb {
                    let before_ok = i == 0 || !is_cap_boundary(t[i - 1]);
                    let after = i + wb.len();
                    let after_ok = after >= blen || !is_cap_boundary(t[after]);
                    if before_ok && after_ok {
                        return true;
                    }
                }
                i += 1;
            }
            false
        };
        let mut unconsumed: Vec<String> = captures
            .iter()
            .filter(|c| {
                !c.starts_with('_')
                    && !exempt.contains(*c)
                    && !m.drops.iter().any(|d| d == *c)
                    && !has_ref(c)
            })
            .cloned()
            .collect();
        if unconsumed.is_empty() {
            continue;
        }
        unconsumed.sort();
        let file = m
            .source_file
            .clone()
            .unwrap_or_else(|| "stdlib/macros".to_string());
        let span = if form.span == SourceSpan::default() {
            m.span.clone()
        } else {
            form.span.clone()
        };
        let list = unconsumed
            .iter()
            .map(|c| format!("`${}`", c))
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(pipeline::StdlibLoadError {
            file_path: std::path::PathBuf::from(&file),
            span: Some(span),
            message: format!(
                "macro `{}` binds {} in %form but never consumes {} in %binds/%emit/%when/                 %derives/%registers/%includes — a bound-but-unread capture is a                  half-implemented macro whose author intent silently vanishes. Pass it to                  a primitive, or mark it intentionally-unused by prefixing the name with `_` (E0964)",
                name, list, list
            ),
            kind: pipeline::StdlibLoadErrorKind::ParseError,
        });
    }
    errors
}

/// FEAT-168 / PLAN-122 W4: post-load validation for the scalar type table.
/// Runs wherever comment-type validation runs (full load + the compile-time
/// project overlay), so a malformed row fails the same ways a malformed
/// comment type does.
///
/// A row is data, not code — so its required keys and sub-clause names have
/// to be checked here, in view of the whole table, rather than by the
/// compiler. `%schema`, `%zero`, and `%widget` are REQUIRED to be DECLARED
/// (the parser records which were missing into `missing_required_keys`);
/// `%format`/`%docs`/`%capture` may be absent by design. `%zero` may be the
/// empty string — that IS the zero — so presence is what is enforced, never
/// content.
pub(crate) fn scalar_type_load_errors(registry: &MetaRegistry) -> Vec<pipeline::StdlibLoadError> {
    let mut errors = Vec::new();
    for t in registry.scalar_types() {
        let file = t
            .source_file
            .clone()
            .unwrap_or_else(|| "stdlib/scalars".to_string());
        let mut push = |message: String| {
            errors.push(pipeline::StdlibLoadError {
                file_path: std::path::PathBuf::from(&file),
                span: Some(t.span),
                message,
                kind: pipeline::StdlibLoadErrorKind::ParseError,
            });
        };

        for key in &t.missing_required_keys {
            push(format!(
                "scalar type `{}` has no %{key} — a scalar must declare every \
                 required key (%schema, %zero, %widget)",
                t.id
            ));
        }
        for sub in &t.unknown_subclauses {
            push(format!(
                "scalar type `{}`: unknown sub-clause `%{sub}` — valid sub-clauses are \
                 %capture, %schema, %format, %zero, %widget, %docs",
                t.id
            ));
        }
    }
    errors
}

/// PLAN-076: run the order-independent post-load migration checks and render
/// them as stdlib load errors. Called by EVERY registry-load path (full
/// load, incremental cache slow path + refresh, embedded fallback) so chain
/// validation never diverges between Fresh and Cached registries.
pub(crate) fn migration_chain_load_errors(
    registry: &MetaRegistry,
) -> Vec<pipeline::StdlibLoadError> {
    match crate::metasystem::validate_migration_chains(registry) {
        Ok(()) => Vec::new(),
        Err(chain_errors) => chain_errors
            .into_iter()
            .map(|e| pipeline::StdlibLoadError {
                file_path: std::path::PathBuf::from("stdlib/migrations/entries"),
                span: Some(e.span),
                message: e.to_string(),
                kind: pipeline::StdlibLoadErrorKind::ParseError,
            })
            .collect(),
    }
}

/// Load stdlib from embedded files (compiled into the binary)
pub(crate) fn load_embedded_stdlib(
    registry: &mut MetaRegistry,
    errors: &mut Vec<pipeline::StdlibLoadError>,
) {
    use crate::parser;
    use crate::stdlib_embedded::{self, EmbeddedFile};
    use std::path::PathBuf;

    for (category, files) in stdlib_embedded::all_embedded_files() {
        for EmbeddedFile { path, content } in files.iter() {
            let virtual_path = format!("stdlib/{}/{}", category, path);

            // Parse the embedded content
            match parser::parse(content) {
                Ok(ast) => {
                    // Load meta definitions (primitives, macros, capture types) from the AST.
                    // Stamp the virtual path as provenance so embedded defs match
                    // filesystem-loaded ones (FEAT-118 namespace derivation +
                    // incremental unregister_file rely on source_file).
                    if let Err(e) =
                        registry.load_from_defs_with_source(ast.meta_defs, &virtual_path)
                    {
                        errors.push(pipeline::StdlibLoadError {
                            file_path: PathBuf::from(&virtual_path),
                            span: Some(e.span),
                            message: format!("Registry error: {:?}", e.kind),
                            kind: pipeline::StdlibLoadErrorKind::ParseError,
                        });
                    }
                }
                Err(e) => {
                    errors.push(pipeline::StdlibLoadError {
                        file_path: PathBuf::from(&virtual_path),
                        span: None,
                        message: format!("Parse error: {}", e),
                        kind: pipeline::StdlibLoadErrorKind::ParseError,
                    });
                }
            }
        }
    }
}

/// Incremental stdlib cache, initialized once on first access.
///
/// On each call to `cached_stdlib_registry()`, checks for stdlib file changes
/// via mtime and re-parses only modified files. This gives both fast reuse
/// AND stdlib hot-reload during development.
static INCREMENTAL_CACHE: LazyLock<
    Mutex<crate::metasystem::incremental_cache::IncrementalStdlibCache>,
> = LazyLock::new(|| {
    Mutex::new(crate::metasystem::incremental_cache::IncrementalStdlibCache::from_full_load())
});

/// Return a clone of the cached stdlib registry, refreshing any changed files.
///
/// On each call, quickly checks which stdlib files changed (via mtime),
/// re-parses only those, and returns the updated registry. For fresh
/// registries (tests, CLI build/check), call [`load_stdlib_registry()`] directly.
pub fn cached_stdlib_registry() -> (MetaRegistry, Vec<pipeline::StdlibLoadError>) {
    let mut cache = INCREMENTAL_CACHE.lock().unwrap();
    cache.refresh_if_needed();
    cache.get()
}

/// Resolve a `RegistrySource` into a concrete `(MetaRegistry, errors)` pair.
fn resolve_registry(source: RegistrySource) -> (MetaRegistry, Vec<pipeline::StdlibLoadError>) {
    crate::profile_span!("resolve_registry");
    match source {
        RegistrySource::Cached => cached_stdlib_registry(),
        RegistrySource::Fresh => load_stdlib_registry(),
        RegistrySource::Provided(registry) => (registry, Vec::new()),
    }
}

/// THE PROJECT OVERLAY (PLAN-123): the file name a project uses to declare
/// its own metasystem data.
///
/// Auto-loaded into the MetaRegistry before any page in that project compiles,
/// and NEVER routed as a page. `%comment_type` is its first citizen; future
/// project-scoped kinds ride the same mechanism rather than each inventing a
/// convention. The leading underscore is the existing "not a page" signal
/// (`projects/animations/_helpers.st`) — but note that ONLY `_prelude.st`
/// auto-loads: `_helpers.st` and friends are `@import`ed explicitly, and
/// auto-loading them too would double-register every definition they hold.
pub const PROJECT_PRELUDE: &str = "_prelude.st";

/// What [`load_project_overlay`] lifted out of the project's `_prelude.st`.
pub struct ProjectOverlay {
    /// Load/parse/register errors (empty for a project without a prelude).
    pub errors: Vec<pipeline::StdlibLoadError>,
    /// The prelude's `@form` DECLARATIONS (PLAN-135 W1). Forms are matches,
    /// not meta_defs, so the meta_defs-only registration above never carried
    /// them: a brand form in `_prelude.st` compiled away to nothing. These
    /// are filtered to form-declaring macros (stdlib's six `@form <kind>`
    /// macros, or the overlay's own `%macro` registering category `form`),
    /// stamped with the prelude path, and merged into every page's AST at the
    /// compile seam — the "globally imported brand attributes" surface.
    pub form_matches: Vec<crate::syntax::FormMatch>,
}

/// The project a page belongs to: the nearest ancestor directory (the page's
/// own directory included) that holds a `_prelude.st`, else the page's own
/// directory.
///
/// `check missions/x.st.md` and `render film.st.md` name a FILE, and the file's
/// parent is not necessarily the project — a page in a sub-folder would
/// otherwise compile without its brand forms and fail with E0947 "unknown
/// form" while `serve`/`build` of the same project pass. One walk, every
/// single-file entry point agrees with the directory entry point.
pub fn project_root_for(file: &Path) -> std::path::PathBuf {
    let start = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let mut current = Some(start.as_path());
    while let Some(dir) = current {
        if dir.join(PROJECT_PRELUDE).is_file() {
            return dir.to_path_buf();
        }
        current = dir.parent().filter(|p| !p.as_os_str().is_empty());
    }
    start
}

/// Load a project's `_prelude.st` overlay into `registry`.
///
/// A project without a prelude is the normal case and produces nothing.
/// Declarations land in the SAME registry the stdlib occupies, which is what
/// makes a project-declared comment type a first-class citizen of the roster
/// rather than a second-tier lookup every consumer would have to remember to
/// check.
pub fn load_project_overlay(
    registry: &mut MetaRegistry,
    site_dir: Option<&Path>,
) -> ProjectOverlay {
    let none = || ProjectOverlay {
        errors: Vec::new(),
        form_matches: Vec::new(),
    };
    let Some(dir) = site_dir else {
        return none();
    };
    let path = dir.join(PROJECT_PRELUDE);
    if !path.exists() {
        return none();
    }

    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return ProjectOverlay {
                errors: vec![pipeline::StdlibLoadError {
                    file_path: path.clone(),
                    span: None,
                    message: format!("could not read project overlay: {e}"),
                    kind: pipeline::StdlibLoadErrorKind::ParseError,
                }],
                form_matches: Vec::new(),
            };
        }
    };

    let parsed = match crate::parser::parse(&source) {
        Ok(ast) => ast,
        Err(e) => {
            return ProjectOverlay {
                errors: vec![pipeline::StdlibLoadError {
                    file_path: path.clone(),
                    span: None,
                    message: format!(
                        "project overlay failed to parse: {}",
                        e.render_all_plain(&source, &path.to_string_lossy())
                    ),
                    kind: pipeline::StdlibLoadErrorKind::ParseError,
                }],
                form_matches: Vec::new(),
            };
        }
    };

    let source_file = path.to_string_lossy().to_string();
    // The overlay's own `%macro`s, kept for the augmented syntax-registry
    // match extraction below (the registration loop moves `meta_defs`).
    let macro_defs: Vec<crate::parser::meta_ast::MacroDefAst> = parsed
        .meta_defs
        .iter()
        .filter_map(|d| match d {
            crate::parser::meta_ast::MetaDef::Macro(m) => Some(m.clone()),
            _ => None,
        })
        .collect();
    let mut errors = Vec::new();
    for mut def in parsed.meta_defs {
        crate::parser::set_meta_def_source_file(&mut def, &source_file);
        if let Err(e) = registry.register(def) {
            // A duplicate id would make a `//@type` ambiguous for every reader.
            errors.push(pipeline::StdlibLoadError {
                file_path: path.clone(),
                span: Some(e.span),
                message: e.to_string(),
                kind: pipeline::StdlibLoadErrorKind::ParseError,
            });
        }
    }

    // PLAN-135 W1: the overlay's `@form` declarations. Extracted with an
    // augmented SYNTAX registry (stdlib + the overlay's own `%macro`s), so a
    // prelude-declared macro carrying `%registers form(...)` is honored
    // alongside stdlib's six. Non-form matches keep their historical
    // treatment (ignored — the prelude is a declarations surface).
    let mut syntax_augmented = (*crate::syntax::STDLIB_REGISTRY).clone();
    for def in &macro_defs {
        syntax_augmented.register(def);
    }
    let (overlay_matches, _) = crate::syntax::events::parse_matches(&source, &syntax_augmented);
    let form_matches = overlay_matches
        .into_iter()
        .filter(|m| {
            syntax_augmented
                .get_by_macro_name(m.matched_macro.as_deref().unwrap_or(&m.macro_name))
                .and_then(|f| f.macro_def.registers.as_ref())
                .is_some_and(|r| r.name == "form")
        })
        .map(|mut m| {
            m.source_file = Some(source_file.clone());
            // The events-level re-parse above bypasses parser::parse (and
            // with it fill_match_docs) — attach the declaration's /// block
            // here so the `@data forms` catalog carries the prelude's copy
            // (PLAN-135 W4/W5: the brand catalog renders doc as card text).
            m.doc = crate::syntax::doc_comment_before(&source, m.span.start as usize);
            m
        })
        .collect();

    ProjectOverlay {
        errors,
        form_matches,
    }
}

/// Internal: run the 5-layer pipeline on a parsed AST.
fn compile_pipeline(ast: &StFile, options: CompileOptions) -> CompiledSpacetime {
    let (mut registry, mut stdlib_errors) = resolve_registry(options.registry_source);

    // THE PROJECT OVERLAY: project-scoped declarations join the registry
    // before anything compiles against it. Deliberately AFTER the stdlib load
    // so a project's duplicate id collides with the stdlib entry (an error)
    // rather than silently shadowing it. The overlay's `@form` declarations
    // (form_matches) merge into the page AST at the clone seam below
    // (PLAN-135 W1) — the global brand-attribute surface.
    let overlay = load_project_overlay(&mut registry, options.site_dir.as_deref());
    stdlib_errors.extend(overlay.errors);
    // The overlay can introduce a malformed type of its own, so the roster is
    // re-judged with the project's entries present.
    if options.site_dir.is_some() {
        stdlib_errors.extend(comment_type_load_errors(&registry));
        stdlib_errors.extend(scalar_type_load_errors(&registry));
    }

    // If there are stdlib load errors, fail compilation with all errors
    if !stdlib_errors.is_empty() {
        let pipeline_errors: Vec<PipelineErrorInfo> = stdlib_errors
            .into_iter()
            .map(|e| pipeline_error_to_info(&pipeline::CompileError::from(e)))
            .collect();

        return CompiledSpacetime {
            js: String::new(),
            css: String::new(),
            html: String::new(),
            build_scripts: Vec::new(),
            css_source_map: None,
            js_source_map: None,
            pipeline_errors,
            pending_migrations: Vec::new(),
            migration_warnings: Vec::new(),
        };
    }

    // Register user's meta definitions (primitives, macros, capture types)
    // These come from the AST and include both direct definitions and imported ones.
    // Skip duplicates silently — they occur when @import re-imports stdlib definitions
    // that are already loaded.
    crate::profile_span!("register_user_meta_defs");
    for def in ast.meta_defs.clone() {
        match registry.register(def) {
            Ok(()) => {}
            Err(e) => match &e.kind {
                MetaRegistryErrorKind::DuplicatePrimitive(_)
                | MetaRegistryErrorKind::DuplicateMacro(_) => {
                    // Already loaded from stdlib — skip silently
                }
                MetaRegistryErrorKind::DuplicateCommentType(_) => {
                    // Same benign case: a page that explicitly `@import`s the
                    // project's `_prelude.st` re-presents types the overlay
                    // already registered. Failing here would make a VALID
                    // import break the build. A genuine cross-file collision
                    // is caught at LOAD time (comment_type_load_errors /
                    // load_project_overlay), before any page compiles.
                }
                MetaRegistryErrorKind::DuplicateScalarType(_) => {
                    // Same benign case as DuplicateCommentType: a page that
                    // imports `_prelude.st` re-presents scalars the overlay
                    // already registered. A genuine collision is caught at
                    // LOAD time by scalar_type_load_errors.
                }
                _ => {
                    return CompiledSpacetime {
                        js: String::new(),
                        css: String::new(),
                        html: String::new(),
                        build_scripts: Vec::new(),
                        css_source_map: None,
                        js_source_map: None,
                        pipeline_errors: vec![PipelineErrorInfo {
                            code: "USER_META_DEF_ERROR".to_string(),
                            message: format!("Failed to load user meta definitions: {:?}", e.kind),
                            hint: Some(
                                "Check that your %primitive and %macro definitions are valid"
                                    .to_string(),
                            ),
                            span: Some(e.span),
                            macro_name: None,
                            macro_file: None,
                            selector: None,
                            bind_span: None,
                            bind_primitive: None,
                            provided_params: None,
                            expected_params: None,
                            primitive_name: None,
                            primitive_file: None,
                        }],
                        pending_migrations: Vec::new(),
                        migration_warnings: Vec::new(),
                    };
                }
            },
        }
    }

    // FEAT-084: rewrite `@data registry $x [from KEY VAL]` matches into
    // `@data inline $x : <registry-json>` so they flow through the identical
    // static-inline path (local-state + @each SSG-unroll). Needs the populated
    // registry, so it runs after meta-def registration. Clone the AST only when
    // there is something to inject (the common case has no @data registry).
    // The `@doc(src: "x.md")` macro inlines the markdown file's bytes at build
    // time (mirrors registry injection): the rendered HTML is produced from the
    // very markdown the docs are written in, so the page cannot drift from them.
    let ast_owned;
    let needs_seed = has_subscribe_needing_seed(ast);
    // PLAN-135: the form SPLICE seam (BUG-298) — a page carrying statement-
    // position `--name;` refs, or a project whose `_prelude.st` declares
    // forms (W1). The overlay's form matches join the page AST, validation
    // re-runs authoritatively (clearing the parse-time file-local E0947 a
    // cross-file form would otherwise carry), then style splices EXPAND into
    // scope CSS.
    let needs_forms = !overlay.form_matches.is_empty() || crate::parser::ast_has_form_refs(ast);
    let ast: &StFile = if has_registry_data(ast)
        || has_dispatch_data(ast)
        || has_doc_src(ast)
        || has_forms_data(ast)
        || needs_seed
        || needs_forms
    {
        let mut cloned = ast.clone();
        if needs_forms {
            cloned.matches.extend(overlay.form_matches.iter().cloned());
            crate::parser::validate_form_refs(&mut cloned);
            crate::parser::expand_style_form_splices(&mut cloned);
        }
        inject_registry_data(&mut cloned, &registry);
        inject_dispatch_data(&mut cloned, &registry);
        inject_forms_data(&mut cloned, &overlay.form_matches);
        inject_doc_content(&mut cloned, options.site_dir.as_deref());
        // FUP-149 (PLAN-119 W3): fill each bodyless `@data subscribe` with a
        // `seed` capture so its `%binds` `local-state-impl(initial: $seed)`
        // has a value to thread. Typed → the type's ZERO VALUE (structural,
        // serialized — nested envelopes work without the balanced-capture
        // parser, BUG-228/PLAN-120); untyped → `null` (byte-identical to the
        // pre-FUP-149 hardcoded `initial: null`). An explicit `{ initial: … }`
        // is the DISTINCT `data-subscribe-seeded` macro (already has a `seed`
        // capture) and is left untouched. This is the shared pipeline seam
        // every compile path funnels through (build, check, headless test).
        inject_subscribe_zero_value_seeds(&mut cloned.matches);
        ast_owned = cloned;
        &ast_owned
    } else {
        ast
    };

    // PLAN-117 W4: eliminate page-global cells the page cannot observe.
    //
    // Runs HERE, before `form_matches` is handed to the pipeline, so a dead
    // cell's declaration never reaches emit: no initialiser, no `SpacetimeLocal`
    // slot, no update plumbing. `analyze_state` needs the fully-assembled page,
    // which is exactly what `ast` is by this point.
    //
    // Sound rather than hopeful because Spacetime has no custom JavaScript: "no
    // reader" is settled by scanning the page, not guessed about a foreign
    // script. The pass is skipped entirely when the page declares no dead cell,
    // so the common case pays one analysis and zero clones.
    let ast_folded;
    let ast: &StFile = {
        let provenance = crate::pipeline::state_analysis::analyze_state(ast);
        let has_dead = provenance
            .iter()
            .any(|p| p.verdict == crate::pipeline::scope::StateVerdict::Dead);
        // NB const-FOLDING (`fold_const_state`) is deliberately NOT wired here,
        // only dead-cell elimination.
        //
        // Substituting a write-free literal at its read sites is correct in
        // isolation and its analysis is sound — but the emitted binding is
        // registered from the reference TEXT, so replacing `$host` with its
        // literal removes the very token the binding pipeline keys on and the
        // element renders empty. Measured, not feared: wiring it dropped the
        // headless matrix from 22 passing to 16, taking every DIVISION test with
        // it (`$total / $count` renders "" once `$total` becomes a literal).
        //
        // The verdict still ships — `inspect --layer state` reports `const`, and
        // `fold_const_state` is unit-tested — so a future binding-aware fold has
        // its input ready. Landing the substitution requires binding
        // registration to key on the resolved CELL rather than the source token,
        // which is its own change. An optimisation that renders the wrong page
        // is not an optimisation.
        if has_dead {
            let mut cloned = ast.clone();
            crate::pipeline::state_analysis::eliminate_dead_state(&mut cloned, &provenance);
            ast_folded = cloned;
            &ast_folded
        } else {
            ast
        }
    };

    // Use pre-populated matches from AST (populated during parsing in convert.rs)
    let form_matches = &ast.matches;

    // Hint-kind (and no-rule-covering) matches are "manual" pending rows.
    // The from_file shim already collected them at iteration 0 with PRE-shim
    // on-disk spans (compiler.rs sets shim_ran); only a source-less from_ast
    // compile needs this AST-side fallback (its spans index the caller's own
    // AST — no drift possible).
    let hint_pending = if options.migration_outcome.shim_ran {
        Vec::new()
    } else {
        let hint_version = crate::migrate::read_syntax_version(&ast.matches).version;
        crate::migrate::collect_hint_pending(&ast.matches, &registry, hint_version.as_deref())
    };

    // Create pipeline context
    let active_registry = registry.clone();
    let context = pipeline::CompileContext::new(registry)
        .without_runtime()
        .with_version_override(options.version_override.clone())
        .with_scopes(ast.scopes.clone())
        .with_invocation_node_ids(if options.inspector_node_stamps {
            // The stamp walk MUST start from the same root the inspect route
            // walks, or a click on the page resolves to an id the tree does not
            // contain (and vice versa). `main` is only a convention — see
            // `default_entry_template`; hardcoding it here left every
            // `main`-less page unstamped, so pick mode silently selected nothing.
            let stamp_bundles = crate::mcp::bundle::collect_template_bundles(ast, None);
            let stamp_root = if stamp_bundles.iter().any(|t| t.name == "main") {
                "main"
            } else {
                stamp_bundles
                    .first()
                    .map(|t| t.name.as_str())
                    .unwrap_or("main")
            };
            crate::introspect::bundle_walk::structure_from_templates(&stamp_bundles, stamp_root)
                .into_iter()
                .filter_map(|node| {
                    let span = node.source_span?;
                    (node.kind == crate::introspect::NodeKind::Template)
                        .then_some(((span.start, span.end), node.id))
                })
                .collect()
        } else {
            Vec::new()
        })
        .with_imports(&ast.imports)
        .with_body_diagnostics({
            // FUP-176: a malformed value in a plain CSS declaration is refused
            // HERE, on the shared compile path, rather than in the `check` CLI.
            // Wiring it into the CLI first made `check` and `compile` disagree —
            // the exact two-answers-for-one-question shape this arc exists to
            // delete, and the gate caught it.
            let mut diags = ast.diagnostics.clone();
            diags.extend(crate::validation::css::declaration_value_diagnostics(
                &ast.scopes,
            ));
            // FEAT-166: and a typed seed's fields against the type they were
            // declared as, through those same grammars.
            diags.extend(crate::validation::css::typed_seed_value_diagnostics(
                form_matches,
            ));
            // PLAN-136 W4b: and a directive property against the type its own
            // `%form` declares for it — the map derived from the forms, not a
            // table written beside them.
            diags
                .extend(crate::validation::css::directive_property_match_diagnostics(form_matches));
            // PLAN-136 W6 / FUP-179: `@style`/`@media`/`@supports`/`@keyframes`
            // bodies are raw text and never became `CssDeclaration`s, so the
            // same value was refused at file scope and silent inside a block.
            for block in &ast.raw_css_blocks {
                diags.extend(crate::validation::css::raw_css_block_diagnostics(
                    &block.source,
                    block.span.start as usize,
                ));
            }
            diags
        });

    // Run through the new pipeline
    crate::profile_span!("pipeline_compile");
    let (mut js, mut css, build_scripts, userland_html, mut pipeline_errors, pipeline_warnings) =
        match pipeline::compile(form_matches, &context) {
            Ok(output) => {
                // Convert any diagnostics from code generation to pipeline errors
                // Convert only ERROR-severity diagnostics from code generation to pipeline errors.
                // Warnings (e.g., W0707 @bind deprecation) are informational and must not
                // be treated as compilation failures; W0715 migration warnings (the
                // hint-kind compile-through path) ride the dedicated channel below.
                let errors: Vec<PipelineErrorInfo> = output
                    .diagnostics
                    .iter()
                    .filter(|diag| diag.severity == crate::diagnostics::Severity::Error)
                    .map(|diag| PipelineErrorInfo {
                        code: diag.code.to_string(),
                        message: diag.message.clone(),
                        hint: diag.hint.clone(),
                        span: diag.span.as_ref().map(|s| SourceSpan {
                            start: s.start,
                            end: s.end,
                        }),
                        macro_name: None,
                        macro_file: None,
                        selector: None,
                        bind_span: None,
                        bind_primitive: None,
                        provided_params: None,
                        expected_params: None,
                        primitive_name: None,
                        primitive_file: None,
                    })
                    .collect();
                let warnings: Vec<PipelineErrorInfo> = output
                    .diagnostics
                    .iter()
                    .filter(|diag| {
                        diag.severity == crate::diagnostics::Severity::Warning
                            && matches!(
                                diag.code,
                                crate::diagnostics::DiagnosticCode::W0715
                                    | crate::diagnostics::DiagnosticCode::W0718
                            )
                    })
                    .map(|diag| PipelineErrorInfo {
                        code: diag.code.to_string(),
                        message: diag.message.clone(),
                        hint: diag.hint.clone(),
                        span: diag.span.as_ref().map(|s| SourceSpan {
                            start: s.start,
                            end: s.end,
                        }),
                        macro_name: None,
                        macro_file: None,
                        selector: None,
                        bind_span: None,
                        bind_primitive: None,
                        provided_params: None,
                        expected_params: None,
                        primitive_name: None,
                        primitive_file: None,
                    })
                    .collect();
                (
                    output.js,
                    output.css,
                    output.build_scripts,
                    output.html,
                    errors,
                    warnings,
                )
            }
            Err(e) => {
                let error_info = pipeline_error_to_info(&e);
                (
                    String::new(),
                    String::new(),
                    Vec::new(),
                    Vec::new(),
                    vec![error_info],
                    Vec::new(),
                )
            }
        };

    // BUG-121: totality-of-lowering guard. An `@`-prefixed ATTRIBUTE on an HTML element
    // (e.g. `<button @mcp-action="approve">`) is a MISPLACED directive or a typo — `@` is
    // THE directive sigil (AGENTS harmony), and directives are selector-scoped, never element
    // attributes. Left alone such a name lowers to a dead `setAttribute("@mcp-action", …)`
    // that nothing listens to (the silent no-op). REFUSE it here with E0930 so the build
    // fails loud instead of shipping an inert attribute (fidelity ladder: refuse, don't
    // fake-green). Scans both template-body scopes and file-scope HTML — the two places
    // authored markup lives. Verified safe: zero `.st` sources use an inline `@`-attr.
    for scope in &ast.scopes {
        if scope.html.is_empty() {
            continue;
        }
        let exprs = crate::emit::html_reactive::component_html_to_exprs(&scope.html);
        for (tag, attr) in crate::emit::html_reactive::find_directive_attrs(&exprs) {
            pipeline_errors.push(directive_attr_error(&tag, &attr, scope.span.start));
        }
    }
    for block in &ast.html_blocks {
        if block.skeleton.is_empty() {
            continue;
        }
        let exprs = crate::html::treesink::parse_html_skeleton(&block.skeleton, &block.holes);
        for (tag, attr) in crate::emit::html_reactive::find_directive_attrs(&exprs) {
            pipeline_errors.push(directive_attr_error(&tag, &attr, block.span.start));
        }
    }

    // GH-12 / PLAN-137 W6 (I6): `%scope element(<tag>)` constraint enforcement.
    // A macro declared `element(canvas)` (the `@stage` macro) is refused when its
    // bound selector's implied element cannot be that tag — a `div.hero { @stage }`
    // previously compiled clean and failed only at runtime (three.js
    // `canvas.getContext` on a div). ERROR-severity diagnostics FAIL the build here;
    // the W0963 warning (element unknowable — `.hero`) is surfaced on the `check`
    // path, which reads this same function and filters the warning half.
    pipeline_errors.extend(
        crate::validation::element_scope::scope_element_diagnostics(ast, &active_registry)
            .into_iter()
            .filter(|d| d.severity == crate::diagnostics::Severity::Error)
            .map(|d| diag_to_pipeline_error_info(&d)),
    );

    // Emit static CSS declarations from scope blocks FIRST (base styles),
    // so pipeline-generated CSS (@media queries, etc.) comes AFTER and can override.
    let mut scope_css = String::new();
    let mut scope_binding_js = String::new();
    emit_scope_css(
        &active_registry,
        &ast.scopes,
        &mut scope_css,
        &mut scope_binding_js,
    );
    if !scope_css.is_empty() {
        css = format!("{}{}", scope_css, css);
    }

    // Append authored top-level CSS at-rule blocks (@media/@supports/@keyframes) verbatim
    // AFTER all base + pipeline CSS so their conditional rules win the cascade (BUG-087).
    if !ast.raw_css_blocks.is_empty() {
        let raw_blocks = ast
            .raw_css_blocks
            .iter()
            .map(|b| b.source.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        css = if css.is_empty() {
            raw_blocks
        } else {
            format!("{}\n{}", css, raw_blocks)
        };
    }
    if !scope_binding_js.is_empty() {
        js = if js.is_empty() {
            scope_binding_js
        } else if options.bindings_first {
            // BUG-119 §2: in a test-body sub-program the reactive `local:<sig>:updated`
            // listeners must be registered BEFORE the body's inline `@when click`
            // directives run, or the click fires before anything is listening and the
            // reactive class/text never reacts. Page builds load all init synchronously
            // before interaction, so they keep the default (bindings last).
            format!("{}\n{}", scope_binding_js, js)
        } else {
            format!("{}\n{}", js, scope_binding_js)
        };
    }

    // Editable schema injection (PLAN-031 / editable-everything): a published page
    // that mounts an editable surface (@editable / @richtext → ST.Editable) needs the
    // mark/block SCHEMA at runtime so marks project to their real tags (bold→<strong>,
    // not <bold>). The admin fetches editable-schema.json over the dev bridge, but a
    // standalone page has no such fetch — so we BAKE the schema (already a projection
    // of the registered @editable-mark/@editable-block constructs) into the bundle.
    // Self-describing per AGENTS: the schema falls out of registered constructs; add a
    // mark and it appears here automatically. Gated on actual surface use (js mentions
    // ST.Editable), so non-editable pages ship zero of it.
    if js.contains("ST.Editable") {
        let (schemas, _diags) =
            crate::editable::collect_schemas_from_matches(&ast.matches, &ast.scopes);
        if !schemas.is_empty() {
            let schema_json = crate::editable::schemas_to_json(&schemas);
            let schema_inject = format!(
                ";(function(){{ if (typeof ST !== 'undefined') {{ var __s = {}; ST.editable = ST.editable || {{}}; ST.editable.marks = Object.assign(ST.editable.marks || {{}}, __s.marks || {{}}); ST.editable.blocks = Object.assign(ST.editable.blocks || {{}}, __s.blocks || {{}}); if (ST.Editable && ST.Editable.setSchema) {{ ST.Editable.setSchema({{ marks: ST.editable.marks, blocks: ST.editable.blocks }}); }} }} }})();",
                schema_json
            );
            js = format!("{}\n{}", js, schema_inject);
        }
    }

    // Custom easing form REGISTRATIONS (BUG-297): every `@form easing`
    // declaration in the merged AST (page + imports + the project overlay)
    // emits its declared curve into the runtime easing map, so ST.getEasing
    // resolves the author's curve — never the silent linear fallback. Page/
    // import declarations register AFTER overlay ones: local overrides global,
    // the CSS-like precedence. PREPENDED to the site JS (runtime wraps later
    // still): animation setup reads ST.easings eagerly, so registration must
    // run before any site code.
    if !js.is_empty()
        && let Some(registrations) = easing_registrations_js(&ast)
    {
        js = format!("{}\n{}", registrations, js);
    }

    // Wrap with runtime last so the order is: runtime → file_js → site_code.
    // Vendor injection is DEFERRED past the html-blocks JS merge below: a
    // reactive-markdown node (`[:st/md]`) emits its `snarkdown(...)` reference
    // into `hole_hydration_js`, which is generated AFTER this point — scanning
    // for vendor references here would miss it and ship a page that calls an
    // uninjected engine. See the injection just after the hydration merge.
    if options.include_runtime && !js.is_empty() {
        js = pipeline::emit::format_js_with_runtime(&js);
    }

    // Lower top-level HTML element literals (PLAN-023 W1): run the html5ever TreeSink on
    // each captured skeleton to build ir::HtmlExpr, then emit to a body-HTML string.
    // First, SSG-unroll static `@each` rows into the captured containers (PLAN-023 W4):
    // a compile-time-constant `@data inline` source renders real rows into the served
    // HTML (SEO / no-JS), and the runtime `each-inline` hydration reproduces the same
    // DOM (unroll == hydrate). Dynamic sources are untouched (runtime-only).
    let mut html_blocks = ast.html_blocks.clone();
    crate::html::ssg_unroll::unroll_static_each(
        ast,
        &mut html_blocks,
        options.site_dir.as_deref(),
        Some(&active_registry),
    );
    // FEAT-154: SSG-unroll provably-static `@template` invocations (literal args,
    // no ELEMENT/body-slot params) right after the `@each` unroll above — same
    // `unroll == hydrate` invariant, same HtmlBlockAst mutation-in-place contract.
    // Runs SECOND so a static `@each` row that itself invokes a static template
    // (composition) has already unrolled its container by the time this pass scans
    // for empty-container injection targets.
    crate::html::ssg_unroll::unroll_static_invocations(ast, &mut html_blocks);
    // Static markdown (`@doc(content:)` — every literate prose segment and
    // every inlined `@doc(src:)`) renders into its container at build time
    // with the vendored snarkdown engine, so the served HTML carries the prose
    // (SEO / no-JS / first paint). The runtime re-renders the same string on
    // mount: unroll == hydrate.
    crate::html::ssg_unroll::unroll_static_docs(ast, &mut html_blocks, Some(&active_registry));
    // PLAN-144 W1: expand COMPONENT holes (`` `&card("T")` ``) in file-scope
    // markup. Runs before `emit_html_blocks`, which would otherwise treat the
    // hole as a signal expression, evaluate it to nothing, and emit an empty
    // `<span data-st-hole="N">` — a green build rendering nothing (BUG-344).
    // Reuses the SAME static renderer as the selector-scoped `&name(…);`
    // invocation above, so the hole form and the selector form are one
    // expansion path with two spellings.
    crate::html::ssg_unroll::expand_component_holes(ast, &mut html_blocks);
    // File-scope reactive holes (FEAT-078): render each `$signal` hole's compile-time initial
    // value statically (SSG / no-JS) + emit the Global-scope hydration JS that re-renders it on
    // `local:<dep>:updated`. Initials come from the file's local-state declarations.
    let global_initials: std::collections::HashMap<String, String> =
        crate::pipeline::scope::build_scope_tree(&ast.matches, &[])
            .global_states
            .into_iter()
            .map(|s| (s.var_name, s.initial))
            .collect();
    let (mut html, hole_hydration_js) = emit_html_blocks(&html_blocks, &global_initials);
    // FEAT-142 WAVE A: inject a static marker element for each entity scope
    // (`&name { @c }`). The entity's interior component directives are selector-
    // scoped to `.st-entity-<name>`; selector-init emit binds directives only
    // against elements present in the STATIC HTML at compile time, so the marker
    // must be a real authored element in the page body. The compiler owns the
    // structural entity lowering, so it owns the structural marker too (general —
    // no vocabulary). Entity-ness is carried as DATA by the synthesized
    // `entity-scope-impl` match (not a ScopeKind arm), so scan the matches for the
    // entity names. Prepended so the marker exists before any content referencing it.
    {
        let mut entity_markers = String::new();
        for m in &ast.matches {
            if m.macro_name != "entity-scope-impl" {
                continue;
            }
            if let Some(name) = m.get_ident("name") {
                entity_markers.push_str(&format!(
                    "<st-entity class=\"st-entity-{}\" data-st-entity=\"{}\" style=\"display:none\"></st-entity>\n",
                    name, name
                ));
            }
        }
        if !entity_markers.is_empty() {
            html = format!("{entity_markers}{html}");
        }
    }
    // FEAT-082: splice userland `%emit html` fragments into the body. They are
    // appended in expansion order (v1: position is end-of-body; selector-anchored
    // placement is a follow-up). A page with no html-emit macros is unaffected.
    if !userland_html.is_empty() {
        let frag = userland_html.join("\n");
        html = if html.is_empty() {
            frag
        } else {
            format!("{html}\n{frag}")
        };
    }
    if !hole_hydration_js.is_empty() {
        // Append after the runtime+site JS so SpacetimeLocal + the local:* event channel exist.
        js = if js.is_empty() {
            // No site JS: the hole-hydration arc IS the bundle — wrap with the runtime so
            // SpacetimeLocal + the local:* channel are present.
            if options.include_runtime {
                crate::pipeline::emit::format_js_with_runtime(&hole_hydration_js)
            } else {
                hole_hydration_js
            }
        } else {
            // Site JS already carries the runtime; append the hole hydration after it.
            format!("{}\n{}", js, hole_hydration_js)
        };
    }

    // Demand-driven vendored-blob injection (lean rail, PLAN-024 W2): prepend a
    // %vendor's IIFE only when its global is actually referenced in the FINAL
    // site JS, so pages that never invoke a vendored capability ship zero of
    // its bytes. DEFERRED to here (rather than before the runtime wrap) because
    // `hole_hydration_js` — which carries a reactive-markdown node's
    // `snarkdown(...)` reference — is merged just above; scanning earlier would
    // miss it. The IIFE is self-contained and defines its global before any
    // site or hydration code runs, so prepending to the fully-assembled bundle
    // keeps `vendor → runtime → site → hydration` ordering intact.
    if !js.is_empty() {
        js = crate::vendor::inject_vendor_preludes(&js, &context.meta_registry);
    }

    // PLAN-076: merge the shim's products. Error-severity shim diagnostics
    // (E0911 duplicate @version, E0912 cycle) join pipeline_errors and FAIL
    // the compile; W0715 warnings ride the dedicated channel. Hint-kind
    // migrations never pass through the shim — collect them here as
    // "manual" pending rows so the pill/`check` show the full picture.
    let (shim_errors, shim_warnings): (Vec<_>, Vec<_>) = options
        .migration_outcome
        .diagnostics
        .iter()
        .partition(|d| d.severity == crate::diagnostics::Severity::Error);
    pipeline_errors.extend(shim_errors.iter().map(|d| diag_to_pipeline_error_info(d)));

    // PLAN-117 W2: page-global state provenance. Runs HERE because `ast` is
    // fully import-resolved by this point — which is the whole reason the
    // duplicate-declaration defect was invisible: a clash between two @import-ed
    // files only exists once the page is assembled. E0938 refuses it; every
    // other registry has always refused its own duplicates.
    let state_provenance = crate::pipeline::state_analysis::analyze_state(ast);
    pipeline_errors.extend(
        crate::pipeline::state_analysis::check_duplicate_state(&state_provenance)
            .iter()
            .map(diag_to_pipeline_error_info),
    );
    let mut migration_warnings: Vec<PipelineErrorInfo> = shim_warnings
        .iter()
        .map(|d| diag_to_pipeline_error_info(d))
        .collect();
    // Hint-kind compile-through warnings surface from the pipeline's
    // retired-syntax check (they never pass through the shim).
    migration_warnings.extend(pipeline_warnings);
    let mut pending_migrations = options.migration_outcome.pending;
    pending_migrations.extend(hint_pending);

    // PLAN-123 convention: a `%comment_type` in a PAGE file is a mistake with
    // a silent failure mode — the page compiles, but the type never reaches
    // the roster (page meta_defs feed the page's own compile, not the project
    // registry), so the author's `//@mytype` comments would report "unknown
    // type" with no obvious cause. Say so at the declaration.
    //
    // ONLY for declarations the page itself owns. `ast.meta_defs` includes
    // everything imports merged in — and merge_ast STAMPS `source_file` on
    // each imported def while a page's own declarations keep `None`. Warning
    // blindly would tell an author to "move" a declaration that lives in the
    // file they imported, including `_prelude.st` itself where it is already
    // in the right place.
    for def in &ast.meta_defs {
        if let crate::parser::meta_ast::MetaDef::CommentType(ct) = def {
            if ct.source_file.is_some() {
                continue;
            }
            migration_warnings.push(diag_to_pipeline_error_info(
                &crate::diagnostics::Diagnostic::warning(
                    crate::diagnostics::DiagnosticCode::W0717,
                    format!(
                        "`%comment_type {}` declared in a page file — it will NOT join \
                         the comments roster",
                        ct.id
                    ),
                )
                .with_hint(format!(
                    "project-scoped declarations live in `{PROJECT_PRELUDE}` at the project \
                     root, which is auto-loaded before any page compiles. Move the \
                     declaration there and `//@{}` comments will resolve everywhere \
                     (pill, check, MCP).",
                    ct.id
                ))
                .with_span(crate::diagnostics::SourceSpan {
                    start: ct.span.start,
                    end: ct.span.end,
                }),
            ));
        }
    }

    // THE BUNDLE MUST PARSE (BUG-262).
    //
    // Every emitter is individually careful and the ASSEMBLED result was never
    // checked. `text <- ($a || $b)` emitted `try { __v = (($a ` — a
    // SyntaxError, shipped green, which killed the whole comments widget on
    // every page that served it. A SyntaxError anywhere in a bundle means the
    // WHOLE bundle never evaluates, so the blast radius of a one-character
    // emit bug is the entire site.
    //
    // `node --check` found it in one second; the compiler did not look. It
    // does now, with the swc parser already vendored for per-block validation
    // (`validation::js`) — the same parser, applied one level up.
    //
    // This is the pipeline-level half of FEAT-170 (a primitive emit-block
    // parse error must fail the build, not emit silence). Emitting invalid JS
    // is ALWAYS a compiler bug, never an author error, so the diagnostic says
    // so and asks for a report rather than blaming the source.
    if let Err(errs) = crate::validation::validate_js(&js) {
        // Debug aid: `SPACETIME_JS_DUMP=<path>` writes the offending bundle so
        // the reported line:col can be read against the real text.
        if let Ok(dump) = std::env::var("SPACETIME_JS_DUMP") {
            let _ = std::fs::write(&dump, &js);
        }
        let detail = errs
            .iter()
            .take(3)
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        pipeline_errors.push(PipelineErrorInfo {
            code: "E0952".to_string(),
            message: format!("the compiler emitted JavaScript that does not parse: {detail}"),
            hint: Some(
                "this is a COMPILER bug, not an error in your source — a bundle that \
                 does not parse never evaluates, so the whole page is dead. Please \
                 report it with the .st that produced it."
                    .to_string(),
            ),
            span: None,
            macro_name: None,
            macro_file: None,
            selector: None,
            bind_span: None,
            bind_primitive: None,
            provided_params: None,
            expected_params: None,
            primitive_name: None,
            primitive_file: None,
        });
    }

    CompiledSpacetime {
        js,
        css,
        html,
        build_scripts,
        css_source_map: None,
        js_source_map: None,
        pipeline_errors,
        pending_migrations,
        migration_warnings,
    }
}

/// Build body HTML from captured top-level HTML element literals (PLAN-023 W1).
/// Build body HTML from captured top-level HTML element literals (PLAN-023 W1) and emit the
/// file-scope reactive HYDRATION JS for any `$signal` holes (FEAT-078). Returns (html, js):
/// the html carries each hole's static initial value wrapped in a hydration marker (SSG /
/// SEO / no-JS correct), and the js re-renders each marker on `local:<dep>:updated` (Global
/// scope). `initials` maps a global-signal name to its compile-time initial (from local-state
/// decls), used to render holes statically.
/// True when any `@data registry` match exists (file- or selector-scoped). Used
/// to skip the AST clone in the common no-registry case (FEAT-084).
fn has_registry_data(ast: &StFile) -> bool {
    fn is_reg(m: &crate::syntax::FormMatch) -> bool {
        matches!(
            m.matched_macro.as_deref(),
            Some("data-registry-all") | Some("data-registry-filtered")
        )
    }
    ast.matches.iter().any(is_reg) || ast.scopes.iter().any(|s| s.matches.iter().any(is_reg))
}

/// Build the runtime REGISTRATION block for every `@form easing` declaration
/// in the merged AST (BUG-297): each declared curve becomes a
/// `ST.registerEasing("name", ST.cubicBezier(…)|ST.springCurve(…))` call, so
/// the runtime easing map holds the AUTHOR'S curve and `getEasing` never
/// falls back to linear for a declared name.
///
/// Curve translation: `cubic-bezier(...)` → `ST.cubicBezier`, `spring(...)`
/// → `ST.springCurve`. Any other body shape registers nothing — the easing
/// body's `expr` capture is deliberately lax today (form.st's tight/lenient
/// table); per-kind body validation is that table's FUP, not this pass.
///
/// Precedence: overlay (`_prelude.st`) declarations emit FIRST, page/import
/// declarations LAST — local overrides global, the CSS-like rule. `None`
/// when the page declares no easing forms (the common case: zero bytes).
fn easing_registrations_js(ast: &StFile) -> Option<String> {
    let mut overlay: Vec<(String, String)> = Vec::new();
    let mut local: Vec<(String, String)> = Vec::new();
    for m in crate::parser::form_declaration_matches(ast) {
        let kind = m
            .captures
            .get("kind")
            .and_then(|v| match v {
                crate::syntax::CapturedValue::Ident(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or_default();
        if kind != "easing" {
            continue;
        }
        let name = m.captures.get("name").and_then(|v| match v {
            crate::syntax::CapturedValue::Ident(s) | crate::syntax::CapturedValue::String(s) => {
                Some(s.trim_start_matches("--").to_string())
            }
            _ => None,
        });
        let Some(name) = name else { continue };
        let body = m.captures.get("body").and_then(|v| match v {
            crate::syntax::CapturedValue::Expr(s) => Some(s.trim().to_string()),
            _ => None,
        });
        let Some(body) = body else { continue };
        // The `expr` capture arrives with the body's braces attached
        // (`"{ cubic-bezier(0.4, 0, 0.2, 1)"`) — strip them to the curve
        // expression itself.
        let body = body
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim()
            .to_string();
        let factory = if let Some(args) = body.strip_prefix("cubic-bezier(") {
            format!("ST.cubicBezier({args}")
        } else if let Some(args) = body.strip_prefix("spring(") {
            format!("ST.springCurve({args}")
        } else {
            continue;
        };
        let entry = (name, factory);
        if m.source_file
            .as_deref()
            .is_some_and(|f| f.ends_with(PROJECT_PRELUDE))
        {
            overlay.push(entry);
        } else {
            local.push(entry);
        }
    }
    if overlay.is_empty() && local.is_empty() {
        return None;
    }
    // Dedupe by name with LATER winning (overlay first, locals last — the
    // precedence above), preserving first-emission order per winner.
    let mut order: Vec<String> = Vec::new();
    let mut winner: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (name, factory) in overlay.into_iter().chain(local) {
        if !winner.contains_key(&name) {
            order.push(name.clone());
        }
        winner.insert(name, factory);
    }
    let mut out = String::from(
        ";(function(){ if (typeof ST === 'undefined' || !ST.registerEasing) { return; }\n",
    );
    for name in order {
        out.push_str(&format!(
            "ST.registerEasing(\"{}\", {});\n",
            name, winner[&name]
        ));
    }
    out.push_str("})();");
    Some(out)
}

/// True when the page has a bodyless `@data subscribe` (the `data-subscribe`
/// macro) that still needs a `seed` capture injected (FUP-149 / PLAN-119 W3).
/// The seeded variant (`data-subscribe-seeded`) already carries its own `$seed`.
fn has_subscribe_needing_seed(ast: &StFile) -> bool {
    fn is_bodyless_subscribe(m: &crate::syntax::FormMatch) -> bool {
        m.matched_macro.as_deref() == Some("data-subscribe") && !m.has("seed")
    }
    ast.matches.iter().any(is_bodyless_subscribe)
        || ast
            .scopes
            .iter()
            .any(|s| s.matches.iter().any(is_bodyless_subscribe))
}

/// Rewrite every `@data registry $x [from KEY VAL]` match into an equivalent
/// `@data inline $x : <json>` match, where `<json>` is the introspection summary
/// array synthesised from `registry` (FEAT-084). After this the rows are an
/// ordinary static-inline source: `@each` SSG-unrolls them and the runtime
/// hydrates the same DOM.
fn inject_registry_data(ast: &mut StFile, registry: &MetaRegistry) {
    use crate::syntax::{CapturedValue, FormMatch};

    fn rewrite(m: &mut FormMatch, registry: &MetaRegistry) {
        let filter = match (m.captures.get("filterKey"), m.captures.get("filterVal")) {
            (Some(k), Some(v)) => {
                // filterKey is an Ident (`kind`); filterVal is a String ("test").
                let key = k
                    .as_type_name()
                    .or_else(|| k.as_string_literal())
                    .unwrap_or_default()
                    .to_string();
                let val = v.as_string_literal().unwrap_or_default().to_string();
                Some((key, val))
            }
            _ => None,
        };
        let json = build_registry_json(registry, filter.as_ref());
        m.macro_name = "data".to_string();
        m.matched_macro = Some("data-inline".to_string());
        m.captures.remove("filterKey");
        m.captures.remove("filterVal");
        m.captures
            .insert("value".to_string(), CapturedValue::Expr(json));
    }

    let is_reg = |m: &FormMatch| {
        matches!(
            m.matched_macro.as_deref(),
            Some("data-registry-all") | Some("data-registry-filtered")
        )
    };
    for m in ast.matches.iter_mut().filter(|m| is_reg(m)) {
        rewrite(m, registry);
    }
    for scope in ast.scopes.iter_mut() {
        for m in scope.matches.iter_mut().filter(|m| is_reg(m)) {
            rewrite(m, registry);
        }
    }
}

/// True when any `@doc` match carries a non-empty `src` path. Used to gate the
/// AST clone (the common no-doc case is untouched).
fn has_doc_src(ast: &StFile) -> bool {
    // True when a `@doc` needs the injection pass: either it names a FILE
    // (`src:`) whose bytes must be inlined, or it carries inline `content:`
    // holding a backslash escape that must be decoded (`\n` in prose).
    fn needs_injection(m: &crate::syntax::FormMatch) -> bool {
        if m.matched_macro.as_deref() != Some("doc") {
            return false;
        }
        let has_src = m
            .captures
            .get("src")
            .and_then(|v| v.as_string_literal())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let escaped_content = m
            .captures
            .get("content")
            .and_then(|v| v.as_string_literal())
            .map(|s| s.contains('\\'))
            .unwrap_or(false);
        has_src || escaped_content
    }
    ast.matches.iter().any(needs_injection)
        || ast
            .scopes
            .iter()
            .any(|s| s.matches.iter().any(needs_injection))
}

/// Inline `@doc(src: "path/to/x.md")` file contents at build time: read the
/// markdown file's bytes and move them into the `content` capture, clearing
/// `src`. The `@doc` macro then binds `render-markdown(content)` exactly as the
/// `@doc(content: …)` inline form does — one render path, one source of truth.
///
/// The path resolves relative to `site_dir` first (the served directory), then
/// the current working directory (so a demo under `demos/foo/` can point at the
/// repo-root `docs/` it documents). A missing file becomes a visible inline
/// error string rather than a silent blank — docs that point at a moved file
/// must fail loudly.
fn inject_doc_content(ast: &mut StFile, site_dir: Option<&std::path::Path>) {
    use crate::syntax::{CapturedValue, FormMatch};

    fn resolve_md(src: &str, site_dir: Option<&std::path::Path>) -> Result<String, String> {
        let candidates = site_dir
            .map(|d| d.join(src))
            .into_iter()
            .chain(std::iter::once(std::path::PathBuf::from(src)));
        for path in candidates {
            if let Ok(text) = std::fs::read_to_string(&path) {
                return Ok(text);
            }
        }
        Err(format!(
            "# Document not found\n\n`@doc` could not read `{src}`."
        ))
    }

    /// Decode the escape sequences an author writes in a `content:` string.
    ///
    /// `.st` string literals are captured RAW (the lexer does not process
    /// escapes), so `@doc(content: "# Title\n\nProse")` would otherwise reach
    /// snarkdown with a literal backslash-n and render as one line of noise —
    /// including the example in `stdlib/md/macros/doc.st`'s own docstring.
    /// `@doc(src:)` never hit this because it injects file bytes below.
    /// Decoding here makes both spellings equivalent, which is also what lets
    /// SIP-002 prose segments (multi-line markdown) render correctly.
    fn decode_escapes(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                // Unknown escape: keep it verbatim — markdown has backslash
                // escapes of its own (e.g. `\*literal asterisks\*`).
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        }
        out
    }

    fn rewrite(m: &mut FormMatch, site_dir: Option<&std::path::Path>) {
        let Some(src) = m
            .captures
            .get("src")
            .and_then(|v| v.as_string_literal())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
        else {
            // No `src:` — this is the inline `content:` spelling. Decode its
            // escapes in place so it matches the file-sourced path.
            if let Some(raw) = m
                .captures
                .get("content")
                .and_then(|v| v.as_string_literal())
                .filter(|s| s.contains('\\'))
                .map(|s| s.to_string())
            {
                m.captures.insert(
                    "content".to_string(),
                    CapturedValue::String(decode_escapes(&raw)),
                );
            }
            return;
        };
        let text = match resolve_md(&src, site_dir) {
            Ok(t) => t,
            Err(e) => e,
        };
        m.captures
            .insert("content".to_string(), CapturedValue::String(text));
        m.captures.remove("src");
    }

    let is_doc = |m: &FormMatch| m.matched_macro.as_deref() == Some("doc");
    for m in ast.matches.iter_mut().filter(|m| is_doc(m)) {
        rewrite(m, site_dir);
    }
    for scope in ast.scopes.iter_mut() {
        for m in scope.matches.iter_mut().filter(|m| is_doc(m)) {
            rewrite(m, site_dir);
        }
    }
}

/// Build the introspection summary as a JS array literal string:
/// `[{"name":"@balance","kind":"text","doc":"...","exports":[...]}, ...]`.
/// Only entries carrying a doc-comment are surfaced (the documentable subset).
/// A `(key, val)` filter keeps only rows whose field equals `val`.
fn build_registry_json(registry: &MetaRegistry, filter: Option<&(String, String)>) -> String {
    use serde_json::{Value, json};

    let kind_of = |source_file: &Option<String>, fallback: &str| -> String {
        if let Some(sf) = source_file {
            for seg in [
                "text",
                "testing",
                "webgl",
                "data",
                "animation",
                "motion",
                "scene",
                "events",
            ] {
                if sf.contains(&format!("/{seg}/")) || sf.contains(&format!("/{seg}.")) {
                    return if seg == "testing" {
                        "test".into()
                    } else {
                        seg.into()
                    };
                }
            }
        }
        fallback.to_string()
    };

    let mut rows: Vec<Value> = Vec::new();

    let mut macro_names: Vec<&str> = registry.macro_names().collect();
    macro_names.sort();
    for name in macro_names {
        if let Some(mac) = registry.get_macro(name) {
            let Some(doc) = mac.doc.as_deref() else {
                continue;
            };
            // form.directive_name already carries the leading `@`.
            let directive = mac
                .form
                .as_ref()
                .map(|f| f.directive_name.clone())
                .unwrap_or_else(|| format!("@{name}"));
            rows.push(json!({
                "name": directive,
                "kind": kind_of(&mac.source_file, "macro"),
                "doc": doc.lines().next().unwrap_or("").trim(),
                "exports": Vec::<String>::new(),
            }));
        }
    }

    let mut prim_names: Vec<&str> = registry.primitive_names().collect();
    prim_names.sort();
    for name in prim_names {
        if let Some(prim) = registry.get_primitive(name) {
            let Some(doc) = prim.doc.as_deref() else {
                continue;
            };
            let exports: Vec<String> = prim
                .body
                .exports
                .iter()
                .map(|e| format!("${}", e.name))
                .collect();
            rows.push(json!({
                "name": name,
                "kind": kind_of(&prim.source_file, "primitive"),
                "doc": doc.lines().next().unwrap_or("").trim(),
                "exports": exports,
            }));
        }
    }

    if let Some((key, val)) = filter {
        rows.retain(|r| r.get(key.as_str()).and_then(Value::as_str) == Some(val.as_str()));
    }

    Value::Array(rows).to_string()
}

/// True when any `@data dispatch` match exists (file- or selector-scoped). Gates
/// the AST clone for the no-dispatch case (FEAT-089).
/// True when the AST carries a `@data forms` match (PLAN-135 W4) or its
/// general form `@data declarations … from <entity>` (PLAN-144 W2). This gates
/// whether the injection runs at all — miss a variant here and the macro
/// survives to primitive resolution as `unknown primitive: data-…`.
fn has_forms_data(ast: &StFile) -> bool {
    fn is_forms(m: &crate::syntax::FormMatch) -> bool {
        matches!(
            m.matched_macro.as_deref(),
            Some("data-forms-all") | Some("data-forms-filtered") | Some("data-declarations")
        )
    }
    ast.matches.iter().any(is_forms) || ast.scopes.iter().any(|s| s.matches.iter().any(is_forms))
}

/// Rewrite every `@data forms $x [from KEY VAL]` match into a `@data inline
/// $x : <json>` whose value is the `@form` DECLARATION catalog (PLAN-135 W4).
/// Enumerates the merged AST's own form-declaring matches (page + imports +
/// the `_prelude.st` overlay, which the needs_forms block has already merged
/// into this clone) plus the stdlib preset library — the same rewrite-to-
/// inline path `@data registry`/`@data dispatch` take.
fn inject_forms_data(ast: &mut StFile, overlay_forms: &[crate::syntax::FormMatch]) {
    use crate::syntax::{CapturedValue, FormMatch};

    let rows = build_forms_json(ast, overlay_forms);

    // A `@data declarations … from <entity>` match needs the rows of ITS
    // entity, so they are built per-match and memoised; `@data forms` keeps
    // using the `form` rows built once above (it is sugar for `entity form`).
    let mut entity_rows: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();

    fn rewrite(m: &mut FormMatch, rows: &[serde_json::Value]) {
        let filter = match (m.captures.get("filterKey"), m.captures.get("filterVal")) {
            (Some(k), Some(v)) => {
                let key = k
                    .as_type_name()
                    .or_else(|| k.as_string_literal())
                    .unwrap_or_default()
                    .to_string();
                let val = v.as_string_literal().unwrap_or_default().to_string();
                Some((key, val))
            }
            _ => None,
        };
        // Mirrors build_dispatch_json's filter: keep rows whose field equals
        // the value; an unrecognised key keeps every row (a slice, not a gate).
        let filtered: Vec<serde_json::Value> = match &filter {
            Some((key, val)) => rows
                .iter()
                .filter(|r| {
                    r.get(key)
                        .and_then(|f| f.as_str())
                        .is_some_and(|f| f == val)
                })
                .cloned()
                .collect(),
            None => rows.to_vec(),
        };
        let json = serde_json::Value::Array(filtered).to_string();
        m.macro_name = "data".to_string();
        m.matched_macro = Some("data-inline".to_string());
        m.captures.remove("filterKey");
        m.captures.remove("filterVal");
        m.captures
            .insert("value".to_string(), CapturedValue::Expr(json));
    }

    let is_forms = |m: &FormMatch| {
        matches!(
            m.matched_macro.as_deref(),
            Some("data-forms-all") | Some("data-forms-filtered") | Some("data-declarations")
        )
    };
    // Collect the entities named by `@data declarations` matches BEFORE mutating
    // any of them (the rewrite consumes the `entity` capture), so the row sets
    // can be built against the pristine AST.
    let mut wanted: Vec<String> = Vec::new();
    {
        let mut note = |m: &FormMatch| {
            if m.matched_macro.as_deref() == Some("data-declarations")
                && let Some(e) = m.captures.get("entity")
                && let Some(name) = e.as_type_name().or_else(|| e.as_string_literal())
                && !wanted.iter().any(|w| w == name)
            {
                wanted.push(name.to_string());
            }
        };
        for m in ast.matches.iter().filter(|m| is_forms(m)) {
            note(m);
        }
        for scope in ast.scopes.iter() {
            for m in scope.matches.iter().filter(|m| is_forms(m)) {
                note(m);
            }
        }
    }
    for entity in wanted {
        let built = build_declarations_json(ast, overlay_forms, &entity);
        entity_rows.insert(entity, built);
    }

    let pick = |m: &FormMatch,
                entity_rows: &std::collections::HashMap<String, Vec<serde_json::Value>>|
     -> Vec<serde_json::Value> {
        if m.matched_macro.as_deref() == Some("data-declarations") {
            let entity = m
                .captures
                .get("entity")
                .and_then(|e| e.as_type_name().or_else(|| e.as_string_literal()))
                .unwrap_or_default()
                .to_string();
            return entity_rows.get(&entity).cloned().unwrap_or_default();
        }
        rows.clone()
    };

    for m in ast.matches.iter_mut().filter(|m| is_forms(m)) {
        let r = pick(m, &entity_rows);
        m.captures.remove("entity");
        rewrite(m, &r);
    }
    for scope in ast.scopes.iter_mut() {
        for m in scope.matches.iter_mut().filter(|m| is_forms(m)) {
            let r = pick(m, &entity_rows);
            m.captures.remove("entity");
            rewrite(m, &r);
        }
    }
}

/// Build the form-declaration catalog rows (PLAN-135 W4):
/// `{ name, kind, params, body, doc, source }` for every `@form <kind>
/// --name { … }` declaration in the merged AST (page + imports; the overlay
/// matches were merged by the needs_forms block before this ran, but the
/// overlay argument covers a page with NO form refs of its own, where that
/// merge is skipped) plus the stdlib preset library.
/// Parse the `/// key: value` lines of a declaration's doc comment into a map.
///
/// PLAN-144 W4=A. The showcase tree already writes 6 consistent keys per
/// pattern (scenario / style / school / description / dependencies /
/// screenshot / useWhen), and `///` is ALREADY consumed here as `doc` — so
/// structuring it adds no second place to say the same thing. A `meta { }`
/// block would have.
///
/// Only leading `key: value` lines count; prose continues to live in `doc`
/// untouched. A key is `[A-Za-z_][A-Za-z0-9_-]*` so a prose line containing a
/// colon ("Note: this is fine") does not silently become a field.
fn parse_doc_fields(doc: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for line in doc.lines() {
        let line = line.trim().trim_start_matches('/').trim();
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim();
        if key.is_empty()
            || !key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        let val = v.trim();
        if val.is_empty() {
            continue;
        }
        out.insert(key.to_string(), serde_json::Value::String(val.to_string()));
    }
    out
}

fn build_forms_json(
    ast: &StFile,
    overlay_forms: &[crate::syntax::FormMatch],
) -> Vec<serde_json::Value> {
    build_declarations_json(ast, overlay_forms, "form")
}

fn build_declarations_json(
    ast: &StFile,
    overlay_forms: &[crate::syntax::FormMatch],
    entity: &str,
) -> Vec<serde_json::Value> {
    use crate::syntax::{CapturedValue, FormMatch};
    use serde_json::{Value, json};

    fn cap_string(m: &FormMatch, key: &str) -> Option<String> {
        match m.captures.get(key) {
            Some(CapturedValue::Ident(s)) | Some(CapturedValue::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Render a captured form BODY as the author wrote it, per kind. The
    /// catalog shows the declared chunk; param substitution is a consumer
    /// concern, never baked into the row.
    fn body_text(m: &FormMatch) -> String {
        match m.captures.get("body") {
            Some(CapturedValue::Properties(props)) => props
                .iter()
                .map(|p| format!("{}: {};", p.name, p.type_ref))
                .collect::<Vec<_>>()
                .join(" "),
            Some(CapturedValue::StyleProperties(pairs)) => pairs
                .iter()
                .map(|(k, v)| format!("{k}: {v};"))
                .collect::<Vec<_>>()
                .join(" "),
            Some(CapturedValue::Keyframes(kfs)) => kfs
                .iter()
                .map(|kf| {
                    let line = format!("{}: {};", kf.property, kf.values.join(" -> "));
                    match &kf.selector {
                        Some(sel) => format!("{sel} {{ {line} }}"),
                        None => line,
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => {
                // An easing `expr` capture can arrive carrying one or both
                // body delimiters (the capture includes the opening `{`) —
                // strip each independently, the same normalization the
                // BUG-297 registration applies.
                let t = s.trim();
                let t = t.strip_prefix('{').unwrap_or(t).trim();
                let t = t.strip_suffix('}').unwrap_or(t).trim();
                t.to_string()
            }
            Some(other) => serde_json::to_string(other).unwrap_or_default(),
            None => String::new(),
        }
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
    }

    /// Render the declaration's parameter list as the author wrote it:
    /// `$pad = 8px`, `&content?`, `&items[]`.
    fn params_text(m: &FormMatch) -> String {
        match m.captures.get("params") {
            Some(CapturedValue::ParamList(params)) => params
                .iter()
                .map(|p| {
                    let sigil = match p.kind {
                        crate::syntax::TemplateParamKind::Binding => "$",
                        crate::syntax::TemplateParamKind::Element => "&",
                    };
                    let mut out = format!("{sigil}{}", p.name);
                    if let Some(t) = &p.type_ref {
                        out.push_str(&format!(" {t}"));
                    }
                    if let Some(d) = &p.default {
                        out.push_str(&format!(" = {d}"));
                    }
                    if p.optional {
                        out.push('?');
                    }
                    if p.collection {
                        out.push_str("[]");
                    }
                    out
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        }
    }

    let mut rows: Vec<Value> = Vec::new();

    // Which LAYER a declaration came from — stdlib preset, project overlay
    // (`_prelude.st`), or the page itself. A catalog that lists 42 forms is
    // unreadable without it: "which of these are MINE" is the first question a
    // reader has, and `source` (a file path) answers it only if you already
    // know the tree.
    // The overlay's declarations are merged into `ast.matches` before this
    // runs, so PUSH ORDER cannot tell a `_prelude.st` form from a page form —
    // the page pass would relabel every overlay row. Origin is read from the
    // declaration's own source instead, which is the fact rather than a proxy
    // for it.
    let origin_of = |m: &FormMatch, fallback: &str| -> &'static str {
        match m.source_file.as_deref() {
            Some(path) if path.ends_with("_prelude.st") => "project",
            Some(path) if path.contains("/stdlib/") || path.starts_with("stdlib/") => "stdlib",
            Some(_) => "page",
            None => match fallback {
                "stdlib" => "stdlib",
                "project" => "project",
                _ => "page",
            },
        }
    };
    let mut push_matches = |matches: &[FormMatch], origin: &str, rows: &mut Vec<Value>| {
        for m in matches {
            let macro_name = m.matched_macro.as_deref().unwrap_or(&m.macro_name);
            if !crate::parser::declares_entity(ast, macro_name, entity) {
                continue;
            }
            let Some(name) = cap_string(m, "name") else {
                continue;
            };
            let kind = cap_string(m, "kind").unwrap_or_default();
            // A name is unique per kind (CSS-like override: later sources
            // replace earlier ones — page > overlay > stdlib).
            let key = (kind.clone(), name.clone());
            let row_key = format!("{}|{}", key.0, key.1);
            if let Some(pos) = rows
                .iter()
                .position(|r| r.get("__key").and_then(|k| k.as_str()) == Some(row_key.as_str()))
            {
                rows.remove(pos);
            }
            let row = json!({
                "name": name,
                "kind": kind,
                "params": params_text(m),
                "body": body_text(m),
                "doc": m.doc.clone().unwrap_or_default(),
                "fields": serde_json::Value::Object(
                    parse_doc_fields(m.doc.as_deref().unwrap_or_default()),
                ),
                "source": m.source_file.clone().unwrap_or_default(),
                "origin": origin_of(m, origin),
                "__key": format!("{}|{}", key.0, key.1),
            });
            rows.push(row);
        }
    };

    // stdlib first, then overlay, then the page's own — later overrides
    // earlier under the same (kind, name), the CSS-like precedence the
    // easing registration (BUG-297) already uses.
    push_matches(
        &crate::syntax::STDLIB_FORM_DECLARATION_MATCHES,
        "stdlib",
        &mut rows,
    );
    push_matches(overlay_forms, "project", &mut rows);
    push_matches(&ast.matches, "page", &mut rows);
    for scope in &ast.scopes {
        push_matches(&scope.matches, "page", &mut rows);
    }

    // The __key is an internal dedup marker — strip before serving.
    for row in rows.iter_mut() {
        if let Some(obj) = row.as_object_mut() {
            obj.remove("__key");
        }
    }
    rows
}

fn has_dispatch_data(ast: &StFile) -> bool {
    fn is_disp(m: &crate::syntax::FormMatch) -> bool {
        matches!(
            m.matched_macro.as_deref(),
            Some("data-dispatch-all") | Some("data-dispatch-filtered")
        )
    }
    ast.matches.iter().any(is_disp) || ast.scopes.iter().any(|s| s.matches.iter().any(is_disp))
}

/// Rewrite every `@data dispatch $x [from KEY VAL]` match into a `@data inline
/// $x : <json>` whose value is the macro-dispatch catalog synthesised from
/// `registry` (FEAT-089). Like `inject_registry_data`, this flows the rows
/// through the identical static-inline path (`@each` SSG-unroll + hydration).
fn inject_dispatch_data(ast: &mut StFile, registry: &MetaRegistry) {
    use crate::syntax::{CapturedValue, FormMatch};

    fn rewrite(m: &mut FormMatch, registry: &MetaRegistry) {
        let filter = match (m.captures.get("filterKey"), m.captures.get("filterVal")) {
            (Some(k), Some(v)) => {
                let key = k
                    .as_type_name()
                    .or_else(|| k.as_string_literal())
                    .unwrap_or_default()
                    .to_string();
                let val = v.as_string_literal().unwrap_or_default().to_string();
                Some((key, val))
            }
            _ => None,
        };
        let json = build_dispatch_json(registry, filter.as_ref());
        m.macro_name = "data".to_string();
        m.matched_macro = Some("data-inline".to_string());
        m.captures.remove("filterKey");
        m.captures.remove("filterVal");
        m.captures
            .insert("value".to_string(), CapturedValue::Expr(json));
    }

    let is_disp = |m: &FormMatch| {
        matches!(
            m.matched_macro.as_deref(),
            Some("data-dispatch-all") | Some("data-dispatch-filtered")
        )
    };
    for m in ast.matches.iter_mut().filter(|m| is_disp(m)) {
        rewrite(m, registry);
    }
    for scope in ast.scopes.iter_mut() {
        for m in scope.matches.iter_mut().filter(|m| is_disp(m)) {
            rewrite(m, registry);
        }
    }
}

/// Build the macro-dispatch catalog as a JS array literal string (FEAT-089):
/// one row per `%form`-carrying macro
/// `{ directive, name, signature, tokens[], doc, kind, scopes[], binds[], overloads[] }`.
/// `tokens` is the structured signature (text+role) the reference UI chip-styles.
/// An optional `(key, val)` filter keeps only rows whose field equals `val`.
fn build_dispatch_json(registry: &MetaRegistry, filter: Option<&(String, String)>) -> String {
    use crate::metasystem::signature::{form_signature_tokens, format_form_signature};
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

    let kind_of = |source_file: &Option<String>, fallback: &str| -> String {
        if let Some(sf) = source_file {
            for seg in [
                "text",
                "testing",
                "webgl",
                "data",
                "animation",
                "motion",
                "scene",
                "events",
            ] {
                if sf.contains(&format!("/{seg}/")) || sf.contains(&format!("/{seg}.")) {
                    return if seg == "testing" {
                        "test".into()
                    } else {
                        seg.into()
                    };
                }
            }
        }
        fallback.to_string()
    };

    // Provenance (FEAT-128): a macro's ORIGIN is a coarse where-from axis that
    // sits ALONGSIDE the finer `kind` (text/animation/…). Derived from the same
    // `source_file` signal: a path under `stdlib/` (or an embedded def with no
    // path) is shipped with the toolchain; anything else is an authored project
    // file. Shape matches `mcp::origin::Origin::to_json` ({kind,id,label}); the
    // macro rail has no env binding so `id` is always null. Kept inline (not a
    // cross-module import) so the compiler stays independent of the MCP layer.
    let origin_of = |source_file: &Option<String>| -> Value {
        let is_stdlib = match source_file {
            // Match a LEADING `stdlib/` segment only — both the embedded loader
            // (`stdlib/{category}/{file}`) and the filesystem loader stamp that
            // prefix. A `/stdlib/` SUBSTRING would misclassify an authored file
            // under a dir named `stdlib/` (e.g. `projects/foo/stdlib/card.st`)
            // as shipped (FEAT-128 reviewer finding).
            Some(sf) => sf.starts_with("stdlib/"),
            None => true,
        };
        if is_stdlib {
            json!({ "kind": "stdlib", "id": Value::Null, "label": "Stdlib" })
        } else {
            json!({ "kind": "env", "id": Value::Null, "label": "Env" })
        }
    };

    // First pass: group macro names by directive so each row can list its overloads.
    let mut names: Vec<&str> = registry.macro_names().collect();
    names.sort();
    let mut overloads_by_dir: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in &names {
        if let Some(mac) = registry.get_macro(name)
            && let Some(form) = &mac.form
        {
            let dir = if form.directive_name.starts_with('@') {
                form.directive_name.clone()
            } else {
                format!("@{}", form.directive_name)
            };
            overloads_by_dir
                .entry(dir)
                .or_default()
                .push(mac.name.clone());
        }
    }

    let mut rows: Vec<Value> = Vec::new();
    for name in &names {
        let Some(mac) = registry.get_macro(name) else {
            continue;
        };
        let Some(form) = &mac.form else { continue };
        let directive = if form.directive_name.starts_with('@') {
            form.directive_name.clone()
        } else {
            format!("@{}", form.directive_name)
        };
        let tokens: Vec<Value> = form_signature_tokens(form)
            .into_iter()
            .map(|t| json!({ "text": t.text, "role": t.role }))
            .collect();
        rows.push(json!({
            "directive": directive,
            "name": mac.name,
            "signature": format_form_signature(form),
            "tokens": tokens,
            "doc": mac.doc.as_deref().map(|d| d.lines().next().unwrap_or("").trim()).unwrap_or(""),
            "kind": kind_of(&mac.source_file, "macro"),
            "origin": origin_of(&mac.source_file),
            "scopes": mac.scopes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "binds": mac.binds.iter().map(|b| b.primitive.clone()).collect::<Vec<_>>(),
            "overloads": overloads_by_dir.get(&directive).cloned().unwrap_or_default(),
        }));
    }

    if let Some((key, val)) = filter {
        rows.retain(|r| r.get(key.as_str()).and_then(Value::as_str) == Some(val.as_str()));
    }

    Value::Array(rows).to_string()
}

fn emit_html_blocks(
    blocks: &[crate::parser::ast::HtmlBlockAst],
    initials: &std::collections::HashMap<String, String>,
) -> (String, String) {
    if blocks.is_empty() {
        return (String::new(), String::new());
    }
    use crate::parser::ast::HtmlInjection;
    let opts = crate::emit::EmitOptions::default();
    let mut html_parts = Vec::with_capacity(blocks.len());
    let mut js = String::new();
    // Thread one monotonic hole-id counter across ALL blocks: bindings address holes by a
    // document-wide querySelectorAll, so ids must be unique page-wide, not per-block (BUG-067).
    let mut next_id = 0usize;
    for (block_index, block) in blocks.iter().enumerate() {
        match block.injection {
            HtmlInjection::Raw => html_parts.push(block.skeleton.clone()),
            HtmlInjection::Reactive => {
                // Keep a stable insertion marker in the SSG output, then replace it with
                // the builder's node/fragment at startup. Global scope is correct for
                // file-level EDN markup; template bodies use root-scoped lowering instead.
                let marker = format!("data-st-html-insert=\"{block_index}\"");
                html_parts.push(format!("<span {marker} style=\"display:contents\"></span>"));
                let exprs = vec![crate::ir::HtmlExpr::Html(block.skeleton.clone())];
                let builder = crate::emit::html_reactive::emit_builder(
                    &exprs,
                    crate::syntax::SignalScope::Global,
                );
                js.push_str(&format!(
                    "\n{{const __mount=document.querySelector('[{marker}]');if(__mount){{const __built=({builder})(__mount);if(__built)__mount.replaceWith(__built);else __mount.remove();}}}}\n"
                ));
            }
            HtmlInjection::Markdown => {
                // The reactive-markdown dual of `Reactive`: the skeleton is a
                // Spacetime expression, and the builder renders its value as
                // Markdown (via snarkdown) into a node whose innerHTML tracks the
                // expression's signals. Same mount-marker dance; the only change
                // is the IR node lowered.
                let marker = format!("data-st-md-insert=\"{block_index}\"");
                html_parts.push(format!("<span {marker} style=\"display:contents\"></span>"));
                let exprs = vec![crate::ir::HtmlExpr::Markdown(block.skeleton.clone())];
                let builder = crate::emit::html_reactive::emit_builder(
                    &exprs,
                    crate::syntax::SignalScope::Global,
                );
                js.push_str(&format!(
                    "\n{{const __mount=document.querySelector('[{marker}]');if(__mount){{const __built=({builder})(__mount);if(__built)__mount.replaceWith(__built);else __mount.remove();}}}}\n"
                ));
            }
            HtmlInjection::Parsed => {
                let exprs =
                    crate::html::treesink::parse_html_skeleton(&block.skeleton, &block.holes);
                let (hydrated, used) =
                    crate::emit::html_hydrate::emit_hydrated_from(&exprs, initials, &opts, next_id);
                next_id = used;
                html_parts.push(hydrated.html);
                js.push_str(&hydrated.js);
            }
        }
    }
    (html_parts.join("\n"), js)
}

/// PLAN-150 W6: rewrite `random()` / `random(N)` occurrences in a CSS value to
/// `var(--st-rnd-N)` (stream N, default 0), collecting the stream indices seen so
/// the caller can emit a hydration stamp. `random()` → stream 0; `random(2)` →
/// stream 2 (an independent seed). Whitespace-tolerant; leaves everything else
/// untouched. The bare token `random` without parens is NOT rewritten (it is not
/// a call).
fn rewrite_random_calls(value: &str, streams: &mut Vec<u32>) -> String {
    if !value.contains("random(") {
        return value.to_string();
    }
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        // Match `random(` at a word boundary (previous char not alphanumeric/-).
        let boundary = i == 0
            || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'-' && bytes[i - 1] != b'_';
        if boundary && value[i..].starts_with("random(") {
            let open = i + "random(".len();
            // Find the matching close paren (args are simple: a number or empty).
            if let Some(close_rel) = value[open..].find(')') {
                let arg = value[open..open + close_rel].trim();
                let stream: u32 = if arg.is_empty() {
                    0
                } else {
                    arg.parse::<f64>().ok().map(|n| n as u32).unwrap_or(0)
                };
                if !streams.contains(&stream) {
                    streams.push(stream);
                }
                out.push_str(&format!("var(--st-rnd-{stream})"));
                i = open + close_rel + 1;
                continue;
            }
        }
        // Copy this UTF-8 char verbatim.
        let ch_len = value[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        out.push_str(&value[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// PLAN-150 W6: emit a hydration stamp that gives every element matching
/// `selector` a deterministic value for each `--st-rnd-N` stream it uses. The
/// value is a seeded pseudo-random in 0..1, seeded from the element's document
/// index AND the stream N, so: stable across builds (a rebuild stamps the same
/// values → bit-identical `spacetime render`), varying per element, and
/// independent per stream. Pure data — no per-frame work, stamped once on mount.
fn emit_random_stamp_js(selector: &str, streams: &[u32], js: &mut String) {
    let sel_js = selector.replace('\\', "\\\\").replace('"', "\\\"");
    let streams_js = streams
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",");
    // Deferred + dynamic-node-aware via ST.registerSelectorInit (the same rail
    // reactive selector bindings use), so the stamp fires AFTER the DOM exists
    // and re-fires for nodes added later (`@each` output). The per-node index
    // is the node's position among its matching siblings under the same parent
    // — stable across builds, so the seed is deterministic.
    js.push_str(&format!(
        "(function() {{\n\
         var __streams = [{streams_js}];\n\
         var __rnd = function(index, stream) {{\n\
           var h = (index * 2654435761 + stream * 40503 + 0x9E3779B9) >>> 0;\n\
           h ^= h >>> 15; h = Math.imul(h, 0x2C1B3C6D) >>> 0;\n\
           h ^= h >>> 12; h = Math.imul(h, 0x297A2D39) >>> 0;\n\
           h ^= h >>> 15; return (h >>> 0) / 4294967296;\n\
         }};\n\
         var __stamp = function(node) {{\n\
           var idx = 0;\n\
           if (node.parentNode) {{ idx = Array.prototype.indexOf.call(node.parentNode.children, node); }}\n\
           for (var s = 0; s < __streams.length; s++) {{\n\
             node.style.setProperty('--st-rnd-' + __streams[s], String(__rnd(idx, __streams[s])));\n\
           }}\n\
         }};\n\
         // Register for dynamically-added nodes (`@each` output) via the
         // selector-init rail, AND sweep the nodes that already exist. The
         // sweep is deferred to a microtask + DOMContentLoaded so it runs after
         // the page's own DOM (and a test fixture) is in place.
         var __sweep = function() {{\n\
           if (typeof document === 'undefined') return;\n\
           var __els = document.querySelectorAll(\"{sel_js}\");\n\
           for (var k = 0; k < __els.length; k++) __stamp(__els[k]);\n\
         }};\n\
         if (typeof ST !== 'undefined' && ST.registerSelectorInit) {{\n\
           ST.registerSelectorInit(\"{sel_js}\", __stamp);\n\
         }}\n\
         if (typeof document !== 'undefined') {{\n\
           if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', __sweep);\n\
           else __sweep();\n\
           if (typeof queueMicrotask === 'function') queueMicrotask(__sweep); else setTimeout(__sweep, 0);\n\
         }}\n\
         }})();\n"
    ));
}

/// Emit CSS from scope blocks' css_declarations into the output string.
/// Handles selector composition for nested scopes.
fn emit_scope_css(
    registry: &crate::metasystem::MetaRegistry,
    scopes: &[ScopeBlock],
    css: &mut String,
    js: &mut String,
) {
    for scope in scopes {
        // A `Construct("template")` scope's selector is a SYNTHETIC association key
        // (`@template:<name>`), not a CSS/DOM selector. Its inner `.sel{}` regions are
        // addressed STANDALONE within each mounted instance (the factory creates the
        // instance subtree), so they must NOT be prefixed by the synthetic key. Emit the
        // template scope's own (empty) declarations, then each nested region as a
        // top-level selector. Real `.sel{}` scopes keep CSS nesting (parent .child).
        if matches!(scope.kind, crate::parser::ast::ScopeKind::Construct(_)) {
            for nested in &scope.nested_scopes {
                emit_scope_css_inner(
                    registry,
                    &nested.composed_selector,
                    &nested.css_declarations,
                    &nested.nested_scopes,
                    css,
                    js,
                );
                // BUG-130/BUG-141: a collection-ref block `&cards[] .sel { @each }`
                // exposes `$cards` on the @template instance root = the live array of
                // the container's children (the @each output). Emit a MutationObserver
                // on the container selector that mirrors children into ST.set(host, name).
                if let Some(ref_name) = &nested.collection_ref {
                    emit_collection_ref_js(&nested.composed_selector, ref_name, js);
                }
            }
        } else {
            emit_scope_css_inner(
                registry,
                &scope.selector,
                &scope.css_declarations,
                &scope.nested_scopes,
                css,
                js,
            );
        }
    }
}

/// BUG-130/BUG-141: emit the collection-ref state binding for a `&name[] .sel { @each }`
/// block. A MutationObserver on the container selector mirrors its child elements
/// (the @each output) into `ST.set(host, name, [children])` on the enclosing
/// @template instance root, so `$name` aggregates the rendered rows. The host is the
/// container's enclosing instance root: walk up while the parent is still an element
/// inside the same instance (the container is created by the same factory subtree),
/// stopping at the outermost element whose parent is the external mount point. In the
/// common (and tested) shape the container is a direct child of the instance root, so
/// `parentElement` is the host; the walk generalises to deeper nesting.
fn emit_collection_ref_js(container_selector: &str, ref_name: &str, js: &mut String) {
    let sel_js = container_selector
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let name_js = ref_name.replace('\\', "\\\\").replace('"', "\\\"");
    js.push_str(&format!(
        "(function() {{\n  \
  const __sync = function(container) {{\n    \
    if (!container) return;\n    \
    // Host = the @template instance root that owns this collection ref. The\n    \
    // container is rendered inside it; climb to the highest ancestor still within\n    \
    // the instance (its parent is the external mount host), defaulting to the\n    \
    // direct parent for the common direct-child shape.\n    \
    let host = container.parentElement || container;\n    \
    const setIt = function() {{\n      \
      if (typeof ST !== 'undefined' && ST.set) {{\n        \
        ST.set(host, \"{name}\", Array.prototype.slice.call(container.children));\n      \
      }}\n    }};\n    \
    setIt();\n    \
    if (typeof MutationObserver !== 'undefined') {{\n      \
      const obs = new MutationObserver(setIt);\n      \
      obs.observe(container, {{ childList: true }});\n    }}\n  }};\n  \
  if (typeof ST !== 'undefined' && ST.registerSelectorInit) {{\n    \
    ST.registerSelectorInit(\"{sel}\", __sync);\n  }} else if (typeof document !== 'undefined') {{\n    \
    document.querySelectorAll(\"{sel}\").forEach(__sync);\n  }}\n}})();\n",
        name = name_js,
        sel = sel_js,
    ));
}

/// A declaration whose value references a `$signal` is a file-scope reactive
/// binding, not CSS.
fn is_reactive_declaration(value: &str) -> bool {
    !crate::syntax::collect_signal_deps(value).is_empty()
}

/// Emit a file-scope reactive binding: re-evaluate `value` (an expression over
/// global `$signals`) whenever any referenced signal changes, applying the
/// result to every element matching `selector`. Subscribes via the
/// `local:<sig>:updated` DOM event — the same channel state mutations and
/// @computed sources dispatch on. `text`/`content` -> textContent; any other CSS
/// property name -> that attribute.
/// Read a registered value facet's `expr_lowering` template for expression
/// positions (reactive CSS, guards): `%subject` substitutes the signal name.
/// Returns None when the segment is not a registered value facet — the
/// dotted read then stays a plain member access (the §5 collision rule:
/// unknown type ⇒ property).
fn facet_expr_lowering(
    registry: &crate::metasystem::MetaRegistry,
    subject: &str,
    segment: &str,
) -> Option<String> {
    let entries = registry.entries_of("driver");
    let (_, clause) = entries
        .iter()
        .find(|(_, c)| registry.entry_key(c) == Some(segment))?;
    match registry.entry_field(clause, "expr_lowering") {
        Some(crate::parser::meta_ast::RegisterValue::String(t)) => {
            Some(t.replace("%subject", subject))
        }
        _ => None,
    }
}

/// Lower a reactive CSS declaration value to a JS expression (BUG-250).
///
/// Two readings, never a guess:
/// - EXPRESSION: the value is ONE expression (`$brand.panel`, `$a + 1`,
///   `$items.length`, `$open ? 'x' : 'none'`) — the established transpile path.
/// - TEMPLATE: literal CSS outside the holes (`1px solid $brand.rule`,
///   `calc(100% - $x)`, `translate($x, $y)`) — string-concat, each hole spliced
///   as its FULL dotted read. The expression path emitted the literal parts
///   RAW (`1px solid ST.resolve(...)`) — invalid JS that killed the whole
///   page runtime at load.
///
/// Discriminator: strip quoted strings and `$ident(.seg)*` reads; an ASCII
/// letter in the remainder means literal CSS (template); digits, operators
/// and parens alone stay expressions (`$a + 1`).
fn lower_css_binding_value(
    registry: &crate::metasystem::MetaRegistry,
    value: &str,
    scope: crate::syntax::SignalScope,
) -> String {
    use crate::syntax::{signal_read, transpile_signal_expr_with};
    let bytes = value.as_bytes();
    let mut remainder_has_letter = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                i += 1;
                if ch == '\\' && i < bytes.len() {
                    i += 1;
                    continue;
                }
                if ch == quote {
                    break;
                }
            }
            continue;
        }
        if c == '$' {
            // Consume `$ident(.seg)*` as one read.
            let mut j = i + 1;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            // A REGISTERED value facet (`$reveal.done`, `$price.prev`) counts
            // as literal CSS for discrimination (review MA): the generic
            // expression transpile would emit a bare member access — force
            // the template path, where facet lowerings apply.
            if j + 1 < bytes.len()
                && bytes[j] == b'.'
                && (bytes[j + 1] as char).is_ascii_alphabetic()
            {
                let seg_start = j + 1;
                let mut k = seg_start;
                while k < bytes.len()
                    && ((bytes[k] as char).is_ascii_alphanumeric() || bytes[k] == b'_')
                {
                    k += 1;
                }
                if facet_expr_lowering(registry, &value[i + 1..j], &value[seg_start..k]).is_some() {
                    remainder_has_letter = true;
                }
            }
            while j + 1 < bytes.len()
                && bytes[j] == b'.'
                && (bytes[j + 1] as char).is_ascii_alphabetic()
            {
                j += 1;
                while j < bytes.len()
                    && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
                {
                    j += 1;
                }
            }
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() {
            remainder_has_letter = true;
        }
        i += 1;
    }

    if !remainder_has_letter {
        return transpile_signal_expr_with(value, scope);
    }

    // TEMPLATE: literal runs quoted, holes spliced as full dotted reads.
    let mut out = String::with_capacity(value.len() + 24);
    let mut literal = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        // `&$name` — an element reference (BUG-333): reads the element registered
        // under ref `name` via `ST.ref`, spliced as a code piece (never wrapped in
        // a string). Mirrors the `$` branch's path handling; without this the `&`
        // dangled and `$name` became a scoped signal read — invalid JS.
        if c == '&' && bytes.get(i + 1) == Some(&b'$') {
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            if j > name_start {
                let name = &value[name_start..j];
                let mut path = String::new();
                while j + 1 < bytes.len()
                    && bytes[j] == b'.'
                    && (bytes[j + 1] as char).is_ascii_alphabetic()
                {
                    let seg_start = j + 1;
                    let mut k = seg_start;
                    while k < bytes.len()
                        && ((bytes[k] as char).is_ascii_alphanumeric() || bytes[k] == b'_')
                    {
                        k += 1;
                    }
                    path.push('.');
                    path.push_str(&value[seg_start..k]);
                    j = k;
                }
                if !literal.is_empty() {
                    if !out.is_empty() {
                        out.push_str(" + ");
                    }
                    out.push('"');
                    for ch in literal.chars() {
                        match ch {
                            '\\' => out.push_str("\\\\"),
                            '"' => out.push_str("\\\""),
                            _ => out.push(ch),
                        }
                    }
                    out.push('"');
                    literal.clear();
                }
                if !out.is_empty() {
                    out.push_str(" + ");
                }
                out.push_str(&format!("ST.ref({:?})", name));
                out.push_str(&path);
                i = j;
                continue;
            }
        }
        if c == '$' {
            let mut j = i + 1;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            let name = &value[i + 1..j];
            // A REGISTERED value facet (PLAN-126: `$price.prev`, `$reveal.done`)
            // lowers through the entry's `expr_lowering` template — never a
            // plain member access (data-driven: a new facet is a registry row).
            if j + 1 < bytes.len() && bytes[j] == b'.' {
                let seg_start = j + 1;
                let mut k = seg_start;
                while k < bytes.len()
                    && ((bytes[k] as char).is_ascii_alphanumeric() || bytes[k] == b'_')
                {
                    k += 1;
                }
                if let Some(lowered) = facet_expr_lowering(registry, name, &value[seg_start..k]) {
                    if !literal.is_empty() {
                        if !out.is_empty() {
                            out.push_str(" + ");
                        }
                        out.push('"');
                        for ch in literal.chars() {
                            match ch {
                                '\\' => out.push_str("\\\\"),
                                '"' => out.push_str("\\\""),
                                _ => out.push(ch),
                            }
                        }
                        out.push('"');
                        literal.clear();
                    }
                    if !out.is_empty() {
                        out.push_str(" + ");
                    }
                    out.push_str(&lowered);
                    i = k;
                    continue;
                }
            }
            // Dotted path segments stay raw member access (same semantics as
            // the expression transpile).
            let mut path = String::new();
            while j + 1 < bytes.len()
                && bytes[j] == b'.'
                && (bytes[j + 1] as char).is_ascii_alphabetic()
            {
                let seg_start = j + 1;
                let mut k = seg_start;
                while k < bytes.len()
                    && ((bytes[k] as char).is_ascii_alphanumeric() || bytes[k] == b'_')
                {
                    k += 1;
                }
                path.push('.');
                path.push_str(&value[seg_start..k]);
                j = k;
            }
            if !literal.is_empty() {
                if !out.is_empty() {
                    out.push_str(" + ");
                }
                out.push('"');
                for ch in literal.chars() {
                    match ch {
                        '\\' => out.push_str("\\\\"),
                        '"' => out.push_str("\\\""),
                        _ => out.push(ch),
                    }
                }
                out.push('"');
                literal.clear();
            }
            if !out.is_empty() {
                out.push_str(" + ");
            }
            out.push_str(&signal_read(scope, name));
            out.push_str(&path);
            i = j;
            continue;
        }
        literal.push(c);
        i += 1;
    }
    if !literal.is_empty() {
        if !out.is_empty() {
            out.push_str(" + ");
        }
        out.push('"');
        for ch in literal.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                _ => out.push(ch),
            }
        }
        out.push('"');
    }
    out
}

/// The longhand a reactive SHORTHAND declaration should really be applied to,
/// or `None` to leave the property exactly as the author wrote it (BUG-279).
///
/// A reactive declaration cannot go into the stylesheet (its value is not known
/// until a signal renders), so it is applied inline. For a longhand that is
/// harmless. For a shorthand it is destructive: `style.setProperty("background",
/// …)` resets `background-size`, `background-clip`, `background-position` and
/// the rest to their initial values, at inline specificity — overriding the
/// static longhands the author wrote in the same rule.
///
/// WHERE THE KNOWLEDGE LIVES: not here. Which longhand a value belongs to is a
/// question about the value's KIND, and the language already declares value
/// kinds as grammars (`stdlib/capture-types/css-values.st`). This function is
/// the seam, not the table: it looks up a `{property}_route` production, matches
/// the value against it, and reads the matched arm's capture NAME as the
/// longhand. `background_route` names its arms `$background_image:image_core`
/// and `$background_color:color_core`; the only transformation applied here is
/// `_` → `-`, because a capture name is an identifier and a CSS property is not.
///
/// Adding `font`, `border` or `grid-area` is therefore a paragraph in that file
/// and no change here — which is the point. A Rust match arm per shorthand would
/// be a second place that knows what a gradient is.
///
/// FALLS THROUGH (returns `None`) when:
///   - the property has no `{property}_route` production (every longhand, and
///     every shorthand nobody has described yet),
///   - no arm matches: `background: inherit` (a cascade instruction, not a
///     value — inheriting ONE longhand is not what the author asked for),
///     `background: $sig` (kind unknowable until it renders), a multi-layer or
///     compound value (`#fff url(a.png) no-repeat`) that genuinely IS a
///     shorthand.
///
/// In every fall-through case the emitted property is unchanged from before the
/// router existed, so nothing that worked can break: the router only ever
/// NARROWS a shorthand it can positively identify.
fn route_reactive_shorthand(
    registry: &crate::metasystem::MetaRegistry,
    property: &str,
    value: &str,
) -> Option<String> {
    let route_type = format!("{}_route", property.replace('-', "_"));
    registry.get_capture_type(&route_type)?;
    // The referenced productions (`image_core`, `color_core`, and everything
    // they compose) must be visible to the matcher, so hand it the whole set
    // from THIS registry — the compiler's, resolved against the source tree the
    // page is being compiled with (BUG-326 makes the global one cwd-relative).
    let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
        registry
            .capture_type_names()
            .filter_map(|n| {
                registry
                    .get_capture_type(n)
                    .map(|d| (n.to_string(), d.clone()))
            })
            .collect();
    let arm = crate::syntax::events::capture_type_matched_arm(&route_type, value, &defs)?;
    Some(arm.replace('_', "-"))
}

/// The JS for one `<-` attribute injection.
///
/// BUG-378: for a FORM CONTROL the content attribute and the IDL property are
/// two different things, and the property is what the browser DISPLAYS. They
/// start mirrored and the mirror breaks permanently once the user interacts:
/// after someone types, `input.value` is the typed text and
/// `setAttribute("value", …)` only moves the default a form reset restores.
///
/// Writing the attribute alone therefore produced a binding that worked on a
/// fresh page and silently stopped working exactly when it mattered — a chat
/// composer bound `value <- $draft` cleared its signal after a send, the
/// attribute went empty, and the input still displayed the sent message. The
/// next keystroke wrote that text back into the signal and the following
/// message arrived concatenated onto the last.
///
/// Both are written: the attribute keeps the serialized DOM honest (it is what
/// an attribute selector sees), the property makes the change visible. The
/// property write is guarded by `in node` so a `value` attribute on a
/// non-control is untouched, and by an inequality check because assigning
/// `value` moves the caret to the end — rewriting an identical value would
/// fight a user typing mid-word.
///
/// Shared by both emitters (here and `emit/html_hydrate.rs`) on purpose: two
/// copies of one rule is how the hydrate path keeps a defect the compile path
/// has already fixed.
pub(crate) fn attribute_injection_js(attr: &str) -> String {
    // Whether this attribute needs the property write is decided HERE, at emit
    // time, from the attribute name — so the generated JS carries no runtime
    // set to look up and no bundle-global to order correctly.
    let form_prop = matches!(attr, "value" | "checked" | "selected");

    if !form_prop {
        return format!(
            "      if (__v === undefined || __v === null || __v === false) node.removeAttribute(\"{attr}\"); else node.setAttribute(\"{attr}\", __v === true ? '' : __v);\n"
        );
    }

    // `"{attr}" in node` keeps a `value` attribute on a NON-control untouched.
    let empty = if attr == "value" { "''" } else { "false" };
    format!(
        "      if (__v === undefined || __v === null || __v === false) {{ \
 node.removeAttribute(\"{attr}\"); \
 if (\"{attr}\" in node) node.{attr} = {empty}; \
 }} else {{ \
 const __a = __v === true ? '' : __v; \
 node.setAttribute(\"{attr}\", __a); \
 if (\"{attr}\" in node && node.{attr} !== __a) node.{attr} = __a; \
 }}\n"
    )
}

fn emit_reactive_binding_js(
    registry: &crate::metasystem::MetaRegistry,
    selector: &str,
    property: &str,
    value: &str,
    is_injection: bool,
    js: &mut String,
) {
    use crate::syntax::{SignalScope, collect_signal_deps};
    // Spacetime filter pipe (`expr | filterName`): split it off so deps + transpile see
    // only the value expression, then apply `ST.filter(name, v)` to the rendered result.
    // This is the SAME filter the reify content-injection path applied (template.st
    // `inj.filter`) and the file-scope derive path uses — now unified into the one
    // reactive-binding emitter (PLAN-039 FEAT-115 S3c) so `text <- $n | currency(…)`
    // filters everywhere (selector scope AND template body), not only via reify.
    // Depth- and string-aware, and never splits `||` (BUG-262). Both reactive
    // emitters route through ONE helper so the rule cannot drift between them.
    let (value, filter) = crate::syntax::split_filter_pipe(value);
    let deps = collect_signal_deps(value);
    if deps.is_empty() {
        return;
    }
    // Lexical (scope-resolved) read: `$sig` -> `ST.resolve(__node, 'sig')`. The binding is
    // applied PER MATCHED NODE (the emitter below names that node `__node`), so each node
    // reads from ITS OWN scope: a `@template` instance resolves the instance signal; a
    // file-scope node finds no element owner and resolves to SpacetimeLocal. One arc,
    // correct for both scopes (PLAN-039 Move 2b) — replacing the Global-only read that
    // silently failed inside template instances.
    let expr = lower_css_binding_value(registry, value, SignalScope::Scoped);
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let sel_js = esc(selector);

    // Route by property-name SHAPE (the `:` surface, BUG-068 / FEAT-072):
    //   `.class`  -> classList.toggle('class', !!v)   (leading dot marker, kept by the parser)
    //   `--prop`  -> style.setProperty('--prop', v)    (CSS custom property)
    //   text|content -> textContent
    //   other     -> style.setProperty(name, v)   (CSS property 2014 BUG-091)
    let apply_line: String = if let Some(class) = property.strip_prefix('.') {
        format!(
            "      node.classList.toggle(\"{cls}\", !!__v);\n",
            cls = esc(class)
        )
    } else if property.starts_with("--") {
        format!(
            "      if (__v === undefined || __v === null || __v === false) node.style.removeProperty(\"{prop}\"); else node.style.setProperty(\"{prop}\", __v);\n",
            prop = esc(property)
        )
    } else if matches!(property, "text" | "content") {
        "      node.textContent = (__v === undefined || __v === null) ? '' : __v;\n".to_string()
    } else if is_injection {
        // The `<-` arrow surface (`src <- $u;`) is a DOM-ATTRIBUTE injection.
        attribute_injection_js(&esc(property))
    } else {
        // Any other property name on the `:` surface is a CSS PROPERTY
        // (`background: $brand.scarlet;`) — emit style.setProperty, NOT
        // setAttribute (BUG-091). Attribute injection is the `<-` arrow above.
        //
        // BUG-279: a reactive SHORTHAND is applied inline, and an inline
        // shorthand resets every longhand it covers to its initial value — at
        // inline specificity, so it beats the static rule the author wrote in
        // the same block. `background: linear-gradient(… $sig …)` therefore
        // destroyed the neighbouring `background-size` and `background-clip`.
        // Route to the longhand the VALUE belongs to. Which longhand that is is
        // a question about the value's kind, and value kinds are declared in
        // stdlib/capture-types/css-values.st — see `background_route` there.
        let routed = route_reactive_shorthand(registry, property, value);
        format!(
            "      if (__v === undefined || __v === null || __v === false) node.style.removeProperty(\"{prop}\"); else node.style.setProperty(\"{prop}\", __v === true ? '' : __v);\n",
            prop = esc(routed.as_deref().unwrap_or(property))
        )
    };

    // Per-node scope-aware arc (PLAN-039 Move 2b): instead of one global `render()` that
    // queries every matching node and reads global signals, register a per-node `init`
    // (via ST.registerSelectorInit, so the dynamic-node observer also initialises nodes a
    // template factory creates per-instance). Each node's `render` reads `ST.resolve(
    // __node, ...)` — its own scope — and re-runs when the dep changes ANYWHERE in that
    // scope (ST.watchScoped: instance ancestors) or globally (`local:<dep>:updated`).
    // `node` aliases `__node` so the property-apply line (node.classList.toggle / …) is
    // unchanged. One emit, correct for file scope (resolves to SpacetimeLocal) and
    // template-instance scope (resolves to the instance signal).
    //
    // `init(__node)` is IDEMPOTENT: a unique
    // per-binding guard flag on the node (`__stBind_<id>`) ensures the `render` closure +
    // its `ST.watchScoped` subscription are installed exactly ONCE per node, no matter how
    // many times init is re-invoked (registerSelectorInit flush, the dynamic-node observer,
    // or a global `local:<dep>:updated` re-scan). Without this guard a global dep update
    // would push a fresh watcher onto the node + every ancestor on each fire — an unbounded
    // leak + quadratic fan-out. The guard id is stable per (selector, property) so distinct
    // bindings on the same node coexist.
    let guard = format!("__stBind_{:x}", {
        // Cheap stable hash of selector+property+value so each binding gets its own flag.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        selector.hash(&mut h);
        property.hash(&mut h);
        value.hash(&mut h);
        h.finish()
    });
    js.push_str("(() => {\n");
    js.push_str("  const init = (__node) => {\n");
    // Idempotency guard: stash the node's `render` under the binding id. If already
    // present, init is a no-op (no second watcher) — but the stored render is reused by
    // the global re-render channel below.
    js.push_str(&format!("    if (__node.{g}) return;\n", g = guard));
    js.push_str("    const node = __node;\n");
    js.push_str("    const render = () => {\n");
    js.push_str("      let __v;\n");
    js.push_str("      try { __v = (");
    js.push_str(&expr);
    js.push_str("); } catch (e) { return; }\n");
    if let Some(filter) = filter {
        // Apply the Spacetime filter to the rendered value (mirrors ST.filter in the
        // reify injection path + the derive path). `ST.filter` resolves the named
        // filter (`currency("$")`, `uppercase`, …) against ST.filters.
        js.push_str(&format!(
            "      if (typeof ST !== 'undefined' && ST.filter) __v = ST.filter(\"{f}\", __v);\n",
            f = esc(filter)
        ));
    }
    js.push_str("    ");
    js.push_str(&apply_line);
    js.push_str("    };\n");
    js.push_str(&format!("    __node.{g} = render;\n", g = guard));
    for dep in &deps {
        // Instance-scope channel: re-render when the dep changes anywhere in this node's
        // scope (a `@template` instance signal set via ST.set). Subscribed ONCE per node.
        js.push_str(&format!(
            "    if (typeof ST !== 'undefined' && ST.watchScoped) ST.watchScoped(__node, \"{dep}\", render);\n",
            dep = esc(dep)
        ));
    }
    js.push_str("    render();\n");
    js.push_str("  };\n");
    js.push_str(&format!(
        "  if (typeof ST !== 'undefined' && ST.registerSelectorInit) {{ ST.registerSelectorInit(\"{sel}\", init); }} else {{ document.querySelectorAll(\"{sel}\").forEach(init); }}\n",
        sel = sel_js
    ));
    // Global channel: a `local:<dep>:updated` event (page-global @data/SpacetimeLocal
    // signals, which fire a DOM event rather than ST.set) inits any NEW matching node and
    // RE-RENDERS already-wired ones via their stored render — without re-subscribing
    // (the guard makes init a no-op, so no watcher leak; re-render is the cheap stored fn).
    for dep in &deps {
        js.push_str(&format!(
            "  document.addEventListener(\"local:{dep}:updated\", () => document.querySelectorAll(\"{sel}\").forEach((__n) => {{ init(__n); if (typeof __n.{g} === 'function') __n.{g}(); }}));\n",
            dep = esc(dep), sel = sel_js, g = guard
        ));
    }
    js.push_str("  if (typeof ST !== 'undefined' && ST._scheduleInit) { ST._scheduleInit(document.body); }\n");
    js.push_str("})();\n");
}

fn emit_scope_css_inner(
    registry: &crate::metasystem::MetaRegistry,
    selector: &str,
    declarations: &[CssDeclaration],
    nested: &[NestedScope],
    css: &mut String,
    js: &mut String,
) {
    // Reactive ($-valued) declarations become runtime bindings; the rest stay
    // static CSS.
    let css_decls: Vec<&CssDeclaration> = declarations
        .iter()
        .filter(|d| !is_reactive_declaration(&d.value))
        .collect();

    if !css_decls.is_empty() {
        // PLAN-150 W6: `random()` / `random(N)` in a static value is a
        // deterministic per-element value form. The compiler cannot know the
        // runtime element instances (author `.dust i` children, `@each` output),
        // so it rewrites the value to read a custom property (`var(--st-rnd-K)`)
        // and emits ONE hydration stamp per selector that assigns each matched
        // element a value seeded from its document index — stable across builds
        // (bit-identical render), varying per element. `random(N)` is stream N
        // (an independent seed offset).
        let mut rnd_streams: Vec<u32> = Vec::new();
        css.push_str(selector);
        css.push_str(" {\n");
        for decl in css_decls {
            let value = rewrite_random_calls(&decl.value, &mut rnd_streams);
            css.push_str("  ");
            css.push_str(&decl.property);
            css.push_str(": ");
            css.push_str(&value);
            css.push_str(";\n");
        }
        css.push_str("}\n");
        if !rnd_streams.is_empty() {
            emit_random_stamp_js(selector, &rnd_streams, js);
        }
    }

    for decl in declarations
        .iter()
        .filter(|d| is_reactive_declaration(&d.value))
    {
        emit_reactive_binding_js(
            registry,
            selector,
            &decl.property,
            &decl.value,
            decl.is_injection,
            js,
        );
    }

    for nested_scope in nested {
        // Read the precomputed composed selector (BUG-206): composition is owned by
        // the scope-tree builder, not recomputed here, so CSS emit and directive
        // selector-assignment can never diverge on a nested path.
        emit_scope_css_inner(
            registry,
            &nested_scope.composed_selector,
            &nested_scope.css_declarations,
            &nested_scope.nested_scopes,
            css,
            js,
        );
    }
}

/// Convert a compile error to a structured PipelineErrorInfo.
///
/// Now unified: matches directly on `CompileErrorKind` instead of
/// two-level matching (PipelineError variant → inner error kind).
/// Build the E0930 diagnostic for a misplaced inline `@`-directive attribute (BUG-121).
/// Names the element + the offending attribute and steers the author to the correct
/// selector-rule form. `span_start` locates the enclosing scope/block (segment-precise
/// spans are a follow-up; the message already names the exact tag+attr).
fn directive_attr_error(tag: &str, attr: &str, span_start: usize) -> PipelineErrorInfo {
    PipelineErrorInfo {
        code: "E0930".to_string(),
        message: format!(
            "`{attr}` used as an inline attribute on <{tag}>. `@`-prefixed names are \
             DIRECTIVES, not attributes -- they are selector-scoped and never lower as an \
             element attribute (it would become a dead `setAttribute(\"{attr}\", ...)` \
             that nothing listens to)."
        ),
        hint: Some(format!(
            "Move it to a selector rule, e.g. give the element a class and write \
             `.your-class {{ {attr}(...) }}`. (Inline `@directive` shorthand is a planned \
             feature -- FUP-073 -- but is not available yet.)"
        )),
        span: Some(SourceSpan {
            start: span_start,
            end: span_start,
        }),
        macro_name: None,
        macro_file: None,
        selector: None,
        bind_span: None,
        bind_primitive: None,
        provided_params: None,
        expected_params: None,
        primitive_name: None,
        primitive_file: None,
    }
}

/// Convert a diagnostics-channel `Diagnostic` (e.g. a migration shim W0715 /
/// E0911 / E0912) into a `PipelineErrorInfo` (PLAN-076).
fn diag_to_pipeline_error_info(diag: &crate::diagnostics::Diagnostic) -> PipelineErrorInfo {
    PipelineErrorInfo {
        code: diag.code.to_string(),
        message: diag.message.clone(),
        hint: diag.hint.clone(),
        span: diag.span.as_ref().map(|s| SourceSpan {
            start: s.start,
            end: s.end,
        }),
        macro_name: None,
        macro_file: None,
        selector: None,
        bind_span: None,
        bind_primitive: None,
        provided_params: None,
        expected_params: None,
        primitive_name: None,
        primitive_file: None,
    }
}

fn pipeline_error_to_info(error: &pipeline::CompileError) -> PipelineErrorInfo {
    use pipeline::CompileErrorKind;
    use pipeline::{StdlibLoadErrorKind, make_error_friendly};

    let (code, message, hint) = match &error.kind {
        // Resolve layer
        CompileErrorKind::MacroNotFound(name) => (
            "E0801".to_string(),
            format!("Macro '{}' not found in stdlib", name),
            Some(format!("Check that '{}' is a valid Spacetime macro. Run `spacetime list macros` to see available macros.", name)),
        ),
        CompileErrorKind::PrimitiveNotFound(name) => (
            "E0802".to_string(),
            format!("Primitive '{}' not found", name),
            Some("This is likely an internal error. The stdlib macro references a primitive that doesn't exist.".to_string()),
        ),
        CompileErrorKind::InvalidBind(msg) => (
            "E0803".to_string(),
            format!("Invalid bind expression: {}", msg),
            None,
        ),
        CompileErrorKind::MissingParameter(name) => {
            let mut message = format!("Missing required parameter '{}'", name);
            if let Some(primitive) = &error.bind_primitive {
                message.push_str(&format!("\n  in bind: {}", primitive));
            }
            if let (Some(provided), Some(expected)) = (&error.provided_params, &error.expected_params) {
                let missing: Vec<_> = expected.iter().filter(|exp| !provided.contains(exp)).collect();
                if !provided.is_empty() {
                    message.push_str(&format!("\n  provided: {}", provided.join(", ")));
                }
                if missing.len() > 1 {
                    message.push_str(&format!("\n  missing: {}",
                        missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
                }
            }
            (
                "E0804".to_string(),
                message,
                Some(format!("The macro expects a '{}' parameter but it wasn't captured from the syntax. This may be a parser limitation.", name)),
            )
        },
        CompileErrorKind::TypeMismatch { expected, got } => (
            "E0805".to_string(),
            format!("Type mismatch: expected {}, got {}", expected, got),
            None,
        ),
        CompileErrorKind::MissingRequiredBinding { binding, child_macro, providers } => {
            let mut hint = format!("The macro @{} declares %requires {{ {} }} but this binding is not available in the parent scope.", child_macro, binding);
            if !providers.is_empty() {
                hint.push_str(&format!("\n\nMacros that provide '{}': {}", binding, providers.join(", ")));
            }
            (
                "E0806".to_string(),
                format!("Macro @{} requires binding '{}' from parent scope", child_macro, binding),
                Some(hint),
            )
        },
        // Expand layer — PrimitiveNotFound already handled above (E0802).
        CompileErrorKind::TemplateError(msg) => (
            "E0812".to_string(),
            format!("Template error: {}", msg),
            Some("Check the template syntax in the emit block".to_string()),
        ),
        // BUG-268: a resolved primitive name that resolves to nothing — neither
        // a registered `%primitive` nor a macro with `%emit` blocks. Used to be a
        // silent `// Primitive not found:` comment INSIDE the shipped bundle;
        // now a hard build failure so a `%binds` referencing a non-existent
        // `%primitive` can never vanish.
        CompileErrorKind::UnknownPrimitive(name) => (
            "E0956".to_string(),
            format!("unknown primitive: {}", name),
            Some("The primitive was neither a registered `%primitive` nor a macro with an `%emit` block. Check the `%binds`/driver that names it.".to_string()),
        ),
        CompileErrorKind::BodyScopeMissing { name, primitive } => (
            "E0925".to_string(),
            format!(
                "internal: template body scope '@template:{}' was not built (primitive '{}')",
                name, primitive
            ),
            Some("This is a compiler invariant break, not an author error: every \
                  body-bearing construct (@template, &name(){{}}, @editable-*) is named \
                  and its World-A scope is built during parsing. Please file a bug with \
                  the source that triggered it.".to_string()),
        ),
        CompileErrorKind::MissingArgument(arg) => (
            "E0813".to_string(),
            format!("Missing required argument '{}'", arg),
            Some(format!("Add the '{}' parameter when calling this primitive", arg)),
        ),
        CompileErrorKind::TypeError(msg) => (
            "E0814".to_string(),
            format!("Type error: {}", msg),
            Some("Check that argument types match the expected types".to_string()),
        ),
        CompileErrorKind::UnresolvedParam(param, available) => {
            use crate::diagnostics::find_similar;
            let available_strs: Vec<&str> = available.iter().map(|s| s.as_str()).collect();
            let suggestion = find_similar(param, &available_strs, 2);
            let hint = if let Some(similar) = suggestion {
                format!("Did you mean '%{}'? Available parameters: {}", similar, available.join(", "))
            } else if !available.is_empty() {
                format!("'%{}' was not found. Available parameters: {}", param, available.join(", "))
            } else {
                format!("'%{}' was not found in the primitive's arguments. Check that it's captured in the macro pattern or passed as an argument.", param)
            };
            ("E0815".to_string(), format!("Unresolved parameter '%{}' in emit template", param), Some(hint))
        },
        // Stdlib load
        CompileErrorKind::StdlibLoad(e) => {
            let code = match &e.kind {
                StdlibLoadErrorKind::FileNotFound => "E0810".to_string(),
                StdlibLoadErrorKind::ParseError => "E0811".to_string(),
                StdlibLoadErrorKind::DuplicateMacro(_) => "E0812".to_string(),
            };
            let source_line = if let Some(span) = &e.span {
                std::fs::read_to_string(&e.file_path)
                    .ok()
                    .map(|content| {
                        let before = &content[..span.start.min(content.len())];
                        let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let line_end = content[span.start.min(content.len())..]
                            .find('\n')
                            .map(|p| span.start + p)
                            .unwrap_or(content.len());
                        content[last_newline..line_end.min(content.len())].to_string()
                    })
            } else {
                None
            };
            let friendly = match &e.kind {
                StdlibLoadErrorKind::FileNotFound => pipeline::FriendlyError {
                    summary: format!("Cannot read stdlib file: {}", e.file_path.display()),
                    explanation: Some("The file could not be found or accessed".to_string()),
                    suggestion: Some("Ensure the stdlib directory exists and is accessible".to_string()),
                    help: None,
                },
                StdlibLoadErrorKind::DuplicateMacro(name) => pipeline::FriendlyError {
                    summary: format!("Duplicate macro definition '{}'", name),
                    explanation: Some(format!("The macro '{}' is defined multiple times", name)),
                    suggestion: Some("Remove or rename one of the duplicate definitions".to_string()),
                    help: None,
                },
                StdlibLoadErrorKind::ParseError => {
                    make_error_friendly(&e.message, source_line.as_deref())
                }
            };
            return PipelineErrorInfo {
                code,
                message: friendly.summary,
                hint: friendly.suggestion,
                span: None,
                macro_name: None,
                macro_file: Some(e.file_path.display().to_string()),
                selector: None,
                bind_span: e.span,
                bind_primitive: None,
                provided_params: None,
                expected_params: None,
                primitive_name: None,
                primitive_file: None,
            };
        },
        // Metasystem errors (E0807+) — shown as generic macro expansion errors
        other => (
            "E0807".to_string(),
            format!("{}", other),
            None,
        ),
    };

    let span = if error.span.start != 0 || error.span.end != 0 {
        Some(error.span)
    } else {
        None
    };
    PipelineErrorInfo {
        code,
        message,
        hint,
        span,
        macro_name: error.macro_name.clone(),
        macro_file: error.macro_file.clone(),
        selector: error.selector.clone(),
        bind_span: error.bind_span,
        bind_primitive: error.bind_primitive.clone(),
        provided_params: error.provided_params.clone(),
        expected_params: error.expected_params.clone(),
        primitive_name: error.primitive_name.clone(),
        primitive_file: error.primitive_file.clone(),
    }
}

// ============================================================================
// Output Validation
// ============================================================================

/// Validation result for compiled output.
#[cfg(feature = "headless")]
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// JavaScript validation errors
    pub js_errors: Vec<crate::validation::JsSyntaxError>,
    /// CSS validation errors
    pub css_errors: Vec<crate::validation::CssSyntaxError>,
}

#[cfg(feature = "headless")]
impl ValidationResult {
    /// Check if there are any validation errors.
    pub fn has_errors(&self) -> bool {
        !self.js_errors.is_empty() || !self.css_errors.is_empty()
    }

    /// Get the total number of errors.
    pub fn error_count(&self) -> usize {
        self.js_errors.len() + self.css_errors.len()
    }

    /// Convert validation errors to diagnostics.
    pub fn to_diagnostics(&self) -> Vec<crate::diagnostics::Diagnostic> {
        use crate::diagnostics::{Diagnostic, DiagnosticCode};

        let mut diagnostics = Vec::new();

        for err in &self.js_errors {
            let mut diag = Diagnostic::error(
                DiagnosticCode::E0701,
                format!("Invalid generated JavaScript: {}", err.message),
            );
            if let (Some(line), Some(col)) = (err.line, err.column) {
                diag = diag.with_note(format!("at line {}, column {}", line, col));
            }
            diagnostics.push(diag);
        }

        for err in &self.css_errors {
            let mut diag = Diagnostic::error(
                DiagnosticCode::E0702,
                format!("Invalid generated CSS: {}", err.message),
            );
            if let (Some(line), Some(col)) = (err.line, err.column) {
                diag = diag.with_note(format!("at line {}, column {}", line, col));
            }
            diagnostics.push(diag);
        }

        diagnostics
    }
}

/// Validate the syntax of compiled JS and CSS output.
///
/// This function checks that the generated JavaScript and CSS are syntactically
/// correct. It's useful for catching errors in macro-generated code or compiler bugs.
///
/// # Arguments
/// * `compiled` - The compiled output to validate
///
/// # Returns
/// A `ValidationResult` containing any JS or CSS syntax errors found.
///
/// # Example
/// ```rust,ignore
/// use spacetime::{parse, compile, validate_output, CompileOptions};
///
/// let ast = parse("body {}").unwrap();
/// let compiled = compile(&ast, CompileOptions::default());
///
/// let result = validate_output(&compiled);
/// if result.has_errors() {
///     for diag in result.to_diagnostics() {
///         eprintln!("{}", diag.message);
///     }
/// }
/// ```
#[cfg(feature = "headless")]
pub fn validate_output(compiled: &CompiledSpacetime) -> ValidationResult {
    use crate::validation::{validate_css, validate_js};

    let js_errors = match validate_js(&compiled.js) {
        Ok(()) => Vec::new(),
        Err(errors) => errors,
    };

    let css_errors = match validate_css(&compiled.css) {
        Ok(()) => Vec::new(),
        Err(errors) => errors,
    };

    ValidationResult {
        js_errors,
        css_errors,
    }
}

/// Compile and validate in one step.
///
/// This is a convenience function that compiles and validates the output,
/// returning both the compiled output and any validation errors.
///
/// # Arguments
/// * `ast` - The parsed AST
/// * `options` - Compilation options
///
/// # Returns
/// A tuple of (CompiledSpacetime, ValidationResult)
#[cfg(feature = "headless")]
pub fn compile_and_validate(
    ast: &StFile,
    options: CompileOptions,
) -> (CompiledSpacetime, ValidationResult) {
    let compiled = compile(ast, options);
    let validation = validate_output(&compiled);
    (compiled, validation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::EmitOptions;

    // === BUG-250: mixed literal+signal CSS values lower as string templates ===

    #[test]
    fn mixed_shorthand_lowers_as_string_template() {
        let out = lower_css_binding_value(
            &cached_stdlib_registry().0,
            "1px solid $brand.rule",
            crate::syntax::SignalScope::Scoped,
        );
        assert!(
            out.contains("\"1px solid \"") && out.contains("ST.resolve"),
            "literal run quoted, hole spliced: {out}"
        );
        assert!(
            out.contains(".rule"),
            "the dotted path stays member access: {out}"
        );
        assert!(
            !out.contains("1px solid ST.resolve"),
            "never raw tokens (the BUG-250 shape): {out}"
        );
    }

    #[test]
    fn single_hole_and_expressions_keep_the_expression_path() {
        let scoped = crate::syntax::SignalScope::Scoped;
        // `$items.length` — dotted read, NOT a template (a quoted
        // `.length` would break the member access).
        let out = lower_css_binding_value(&cached_stdlib_registry().0, "$items.length", scoped);
        assert!(!out.starts_with('"'), "no quoting: {out}");
        assert!(out.contains(".length"), "member access preserved: {out}");
        // `$a + 1` — digits/operators only in the remainder.
        let out2 = lower_css_binding_value(&cached_stdlib_registry().0, "$a + 1", scoped);
        assert!(!out2.contains("\" + \""), "not a template: {out2}");
        // `$open ? 'x' : 'none'` — quoted strings don't trip the discriminator.
        let out3 =
            lower_css_binding_value(&cached_stdlib_registry().0, "$open ? 'x' : 'none'", scoped);
        assert!(
            !out3.contains("\" + \""),
            "ternary stays expression: {out3}"
        );
    }

    #[test]
    fn element_ref_name_lowers_to_st_ref() {
        // BUG-333: `&$b` is an element reference, NOT a scoped signal read. It
        // must lower to `ST.ref("b")` (the __stRefs registry lookup) — before
        // the fix it emitted `&ST.resolve(__node, 'b')` (a dangling `&` that
        // failed to parse and killed the bundle).
        let scoped = crate::syntax::SignalScope::Scoped;
        let out = lower_css_binding_value(&cached_stdlib_registry().0, "&$b", scoped);
        assert_eq!(out, "ST.ref(\"b\")", "element ref lowers to ST.ref: {out}");
        // A dotted element ref keeps its member access path.
        let out2 = lower_css_binding_value(&cached_stdlib_registry().0, "&$b.rect.width", scoped);
        assert_eq!(
            out2, "ST.ref(\"b\").rect.width",
            "dotted element ref keeps member access: {out2}"
        );
    }

    #[test]
    fn calc_and_translate_lower_as_templates() {
        let scoped = crate::syntax::SignalScope::Scoped;
        let out = lower_css_binding_value(&cached_stdlib_registry().0, "calc(100% - $x)", scoped);
        assert!(out.starts_with("\"calc("), "calc is literal CSS: {out}");
        let out2 =
            lower_css_binding_value(&cached_stdlib_registry().0, "translate($x, $y)", scoped);
        assert!(
            out2.contains("\"translate(\"") && out2.contains("\", \""),
            "both holes spliced, commas literal: {out2}"
        );
    }

    // === PLAN-023 W1: file-scope HTML -> CompiledSpacetime.html ===

    fn compile_default(source: &str) -> CompiledSpacetime {
        let ast = crate::parser::parse(source).expect("parse should succeed");
        compile(&ast, CompileOptions::new().with_fresh_registry())
    }

    /// Compile a source that imports a modular stdlib MODULE (e.g. `stdlib/dnd`).
    /// Module defs ride the import path (merge_ast), not the registry scan, so
    /// these tests must go through `resolve_imports` + `from_file` — the same
    /// production path the `stdlib/md` tests use. Writes the source to a temp
    /// file (imports resolve relative to the workspace root `.`).
    fn compile_with_imports(source: &str) -> CompiledSpacetime {
        let dir = std::env::temp_dir().join(format!(
            "st_import_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(&entry, source).unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .compile();
        let _ = std::fs::remove_dir_all(&dir);
        compiled
    }

    #[test]
    fn authored_media_block_emits_to_stylesheet() {
        // BUG-087: top-level `@media` blocks were silently dropped by the CST->StFile
        // conversion (no at-rule IR variant), making breakpoint responsiveness impossible.
        // They must now pass through to the stylesheet verbatim.
        let css = compile_default(
            ".box { background: red; }\n@media (max-width: 600px) { .box { background: blue; } }",
        )
        .css;
        assert!(
            css.contains("@media (max-width: 600px)"),
            "authored @media block must reach the stylesheet, got: {}",
            css
        );
        assert!(
            css.contains("background: blue"),
            "the @media block body must be preserved, got: {}",
            css
        );
    }

    #[test]
    fn when_guard_emits_signal_watch_and_mutation() {
        // BUG-088: `@when &self.<ns>.<sig> { $state <- value; }` must lower to a real
        // runtime guard — watch the element-local signal, run the mutation body when it
        // becomes truthy. Before, the guard body was dropped entirely (the inner
        // `&self.…` greedily matched the element-ref form and `@when` matched nothing).
        let js = compile_default(
            ".sec {\n  @scroll-spy\n  @when &self.scrollSpy.isActive { $navTheme <- \"dark\"; }\n}",
        )
        .js;
        // The signal path is captured and reduced to its final segment, NOT the body.
        assert!(
            js.contains("\"scrollSpy.isActive\""),
            "the signal path must be captured verbatim (no body leak), got js without it"
        );
        assert!(
            !js.contains("\"scrollSpy.isActive { "),
            "the signal capture must NOT swallow the body brace block (BUG-088 regression)"
        );
        // The guard watches the element signal and runs the mutation body.
        assert!(
            js.contains("ST.watch(host, sig"),
            "the guard must watch the element-local signal"
        );
        assert!(
            js.contains("ST.runMutations(host, body)"),
            "the guard must run the mutation body via ST.runMutations"
        );
        assert!(
            js.contains("$navTheme <- \\\"dark\\\"") || js.contains("navTheme"),
            "the mutation body must be carried into the guard, got js without it"
        );
    }

    #[test]
    fn authored_media_string_form_normalized() {
        // BUG-087: the Spacetime `@media("query")` parenthesized-string surface must be
        // normalized to the standard `@media query` form so browsers parse it.
        let css = compile_default(
            ".box { background: red; }\n@media(\"max-width: 600px\") { .box { background: blue; } }",
        )
        .css;
        assert!(
            css.contains("@media (max-width: 600px)"),
            "the @media(\"...\") string form must normalize to standard CSS, got: {}",
            css
        );
    }

    #[test]
    fn data_registry_injects_introspection_rows_and_ssg_unrolls() {
        // FEAT-084: @data registry → @data inline(json) → @each SSG-unroll.
        // The served HTML must carry real cards built from the registry, with
        // each documented directive's @name + one-line doc, statically (no JS).
        let src = "@data registry $features;\n\
<main><div class=\"grid\"></div></main>\n\
.grid { @each($features as $f) { <article class=\"card\"><code>`$f.name`</code></article> } }\n";
        let compiled = compile_default(src);
        let n = compiled.html.matches("class=\"card\"").count();
        assert!(
            n > 20,
            "registry should unroll many cards into static HTML, got {n}: {}",
            &compiled.html[..compiled.html.len().min(200)]
        );
        // A core documented directive must appear with its single @ (no double-@).
        assert!(
            compiled.html.contains("@each"),
            "expected an @each card from the registry"
        );
        assert!(
            !compiled.html.contains("@@"),
            "directive names must not double the @"
        );
    }

    #[test]
    fn data_dispatch_injects_signature_catalog_and_ssg_unrolls() {
        // FEAT-089: @data dispatch → @data inline(json) → @each SSG-unroll. The
        // served HTML must carry one card per dispatchable macro, each showing its
        // rendered form signature.
        let src = "@data dispatch $macros;\n\
<main><div class=\"grid\"></div></main>\n\
.grid { @each($macros as $m) { <article class=\"card\"><code>`$m.signature`</code></article> } }\n";
        let compiled = compile_default(src);
        let n = compiled.html.matches("class=\"card\"").count();
        assert!(n > 50, "dispatch should unroll many macro cards, got {n}");
        // A real signature with a directive head must appear (e.g. @data fetch …).
        assert!(
            compiled.html.contains("@data fetch") || compiled.html.contains("@each"),
            "expected a rendered form signature in a card"
        );
        assert!(
            !compiled.html.contains("@@"),
            "signatures must not double the @"
        );
    }

    #[test]
    fn data_dispatch_filter_keeps_only_matching_kind() {
        let src = "@data dispatch $d from kind \"data\";\n\
<main><div class=\"g\"></div></main>\n\
.g { @each($d as $m) { <span class=\"c\" data-kind=\"`$m.kind`\">`$m.name`</span> } }\n";
        let compiled = compile_default(src);
        assert!(
            compiled.html.contains("data-kind=\"data\""),
            "data-kind rows expected"
        );
        assert!(
            !compiled.html.contains("data-kind=\"test\""),
            "a non-data kind must be filtered out of a kind=data slice"
        );
    }

    #[test]
    fn data_dispatch_carries_origin_metadata() {
        // FEAT-128: every dispatched macro row carries an `origin` object
        // ({kind,id,label}) alongside `kind`. The compiler's macros all come from
        // the stdlib prepend, so each row's origin.kind must be "stdlib".
        let src = "@data dispatch $macros;\n\
<main><div class=\"g\"></div></main>\n\
.g { @each($macros as $m) { <span class=\"c\" data-origin=\"`$m.origin.kind`\">`$m.origin.label`</span> } }\n";
        let compiled = compile_default(src);
        assert!(
            compiled.html.contains("data-origin=\"stdlib\""),
            "every stdlib macro row should carry origin.kind=stdlib"
        );
        assert!(
            compiled.html.contains("Stdlib"),
            "origin.label should render as the Stdlib chip text"
        );
        // Provenance is a SEPARATE axis from the finer `kind`: a non-stdlib origin
        // must never leak into the compiler's own (all-stdlib) macro set.
        assert!(
            !compiled.html.contains("data-origin=\"agent\""),
            "compiler macros are never agent-origin"
        );
    }

    /// BUG (found by SIP-002): `.st` string literals are captured RAW, so an
    /// inline `@doc(content: "…\n…")` reached snarkdown with a literal
    /// backslash-n and rendered as one line of noise — the spelling shown in
    /// `stdlib/md/macros/doc.st`'s own docstring. `src:` was unaffected (it
    /// injects file bytes), so the two spellings disagreed. Both must decode.
    #[test]
    fn doc_content_decodes_escape_sequences_like_src_does() {
        let dir = std::env::temp_dir().join(format!("st_doc_escapes_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@import \"stdlib/md\";\n<main><article class=\"doc\"></article></main>\n.doc { @doc(content: \"# Title\\n\\nSome **bold** prose.\") }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .with_site_dir(Some(dir.clone()))
            .fresh_registry()
            .compile();
        // The emitted JS string must carry a real newline escape (`\n` as two
        // JS chars), never the author's literal backslash-backslash-n.
        assert!(
            compiled.js.contains("# Title\\n\\nSome **bold** prose."),
            "content escapes not decoded: {}",
            &compiled.js[..compiled.js.len().min(400)]
        );
        assert!(
            !compiled.js.contains("# Title\\\\n"),
            "literal backslash-n leaked into the rendered markdown"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A markdown backslash escape the decoder does not own (`\*`) must survive
    /// verbatim — markdown has its own escapes and we must not eat them.
    #[test]
    fn doc_content_preserves_unknown_markdown_escapes() {
        let dir = std::env::temp_dir().join(format!("st_doc_mdesc_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@import \"stdlib/md\";\n<main><article class=\"doc\"></article></main>\n.doc { @doc(content: \"literal \\\\*stars\\\\* here\") }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .with_site_dir(Some(dir.clone()))
            .fresh_registry()
            .compile();
        assert!(
            compiled.js.contains("stars"),
            "markdown escape dropped the content entirely"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn doc_src_inlines_markdown_file_and_renders_via_snarkdown() {
        // @doc(src: "x.md") reads the file at build time, moves its bytes into the
        // `content` capture, and binds render-markdown — so the vendored snarkdown
        // engine is demand-injected and the file's prose ends up in the page JS.
        // Goes through the real from-file path (resolve_imports + rematch) so the
        // `stdlib/md` module's macro+primitive register exactly as in production.
        let dir = std::env::temp_dir().join(format!("st_doc_inline_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("guide.md"),
            "# Inlined Heading\n\nSome **bold** prose.",
        )
        .unwrap();
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@import \"stdlib/md\";\n<main><article class=\"doc\"></article></main>\n.doc { @doc(src: \"guide.md\") }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .with_site_dir(Some(dir.clone()))
            .fresh_registry()
            .compile();
        assert!(
            compiled.js.contains("snarkdown"),
            "render-markdown should demand-inject the snarkdown engine"
        );
        assert!(
            compiled.js.contains("Inlined Heading"),
            "the markdown file's content must be inlined into the page JS"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn doc_src_missing_file_emits_visible_error_not_blank() {
        // A @doc pointing at a moved/missing file must fail LOUDLY (a rendered
        // error string), never silently produce a blank panel.
        let dir = std::env::temp_dir().join(format!("st_doc_missing_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@import \"stdlib/md\";\n<main><article class=\"doc\"></article></main>\n.doc { @doc(src: \"does/not/exist.md\") }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .with_site_dir(Some(dir.clone()))
            .fresh_registry()
            .compile();
        assert!(
            compiled.js.contains("Document not found"),
            "a missing @doc src must inline a visible error, got: {}",
            &compiled.js[..compiled.js.len().min(400)]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FEAT-118 module system, end-to-end from disk. A two-file `@use` tree
    /// (the `examples/modules` demo, re-created in a temp dir): a `badge` module
    /// and a page that imports it `as b` and calls `@b/badge`. Exercises the
    /// full path — `resolve_imports` → retained `@use` ImportAst → ImportScope →
    /// canonical-qualifier rewrite → FQN resolution → emit. Plus the two error
    /// classes (E0926 unbound qualifier, E0927 visibility) on the same tree.
    fn write_badge_module(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("badge.st"),
            "%macro badge {\n  %form { @badge($label:string) { $styles:properties } }\n  %binds { badge-emit($label, styles: $styles) -> {} }\n}\n%primitive badge-emit {\n  %emit css { .badge::before { content: \"$label\"; } }\n}\n",
        )
        .unwrap();
    }

    #[test]
    fn module_system_alias_qualified_reference_resolves_and_emits() {
        let dir = std::env::temp_dir().join(format!("st_mod_ok_{}", std::process::id()));
        write_badge_module(&dir);
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@use \"./badge.st\" as b\n<main><span class=\"badge\"></span></main>\n.badge { @b/badge(\"NEW\") { color: red; } }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, &dir)
            .expect("compiler from_file")
            .fresh_registry()
            .compile();
        // The alias-qualified `@b/badge` resolved to the module macro and emitted.
        assert!(
            compiled.css.contains(".badge::before"),
            "@b/badge should resolve through the alias and emit badge CSS, got css: {:?} errors: {:?}",
            compiled.css,
            compiled
                .pipeline_errors
                .iter()
                .map(|e| &e.code)
                .collect::<Vec<_>>()
        );
        let e092x: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| e.code == "E0926" || e.code == "E0927")
            .collect();
        assert!(
            e092x.is_empty(),
            "clean import should not error: {:?}",
            e092x
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn module_system_unbound_qualifier_errors_e0926() {
        let dir = std::env::temp_dir().join(format!("st_mod_e0926_{}", std::process::id()));
        write_badge_module(&dir);
        let entry = dir.join("index.st");
        // `@typo/badge` — `typo` is neither the alias `b` nor a path of the module.
        std::fs::write(
            &entry,
            "@use \"./badge.st\" as b\n<main><span class=\"badge\"></span></main>\n.badge { @typo/badge(\"NEW\") { color: red; } }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, &dir)
            .expect("compiler from_file")
            .fresh_registry()
            .compile();
        assert!(
            compiled.pipeline_errors.iter().any(|e| e.code == "E0926"),
            "unbound qualifier @typo/badge should raise E0926, got: {:?}",
            compiled
                .pipeline_errors
                .iter()
                .map(|e| &e.code)
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// BUG-329 cutover: the RETIRED bare-event head must be REFUSED.
    ///
    /// `@on &.click { … }` was declared by `on-mutation-dispatcher-simple` via two
    /// capture types that never existed (`event_name`, `mutation_actions`), so it
    /// failed at USE with a grammar mismatch pointing at the author's page. The
    /// cutover deletes the head; `@on &.click` is the one spelling.
    ///
    /// This asserts the REFUSAL. Its partner asserts the replacement actually
    /// FIRES — that one cannot live here, because a listener that registers
    /// through a variable is invisible to any source assertion:
    /// `tests/lang/on-event/click-mutation.test.st` (headless).
    #[test]
    fn bare_event_on_head_is_refused_after_the_sigil_cutover() {
        let dir = std::env::temp_dir().join(format!("st_bug329_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@import \"stdlib/macros/on-event\"\n<main><button class=\"b\">go</button></main>\n.b { @on badevent { $x <- 1; } }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, &dir)
            .expect("compiler from_file")
            .fresh_registry()
            .compile();
        assert!(
            compiled.pipeline_errors.iter().any(|e| e.code == "E0946"),
            "the retired bare-event `@on click` must raise E0946, got: {:?}",
            compiled
                .pipeline_errors
                .iter()
                .map(|e| &e.code)
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn module_system_hiding_list_hides_name_e0927() {
        let dir = std::env::temp_dir().join(format!("st_mod_e0927_{}", std::process::id()));
        write_badge_module(&dir);
        let entry = dir.join("index.st");
        // `hiding (badge)` removes `@badge` from visibility; calling it errors.
        std::fs::write(
            &entry,
            "@use \"./badge.st\" as b hiding (badge)\n<main><span class=\"badge\"></span></main>\n.badge { @b/badge(\"NEW\") { color: red; } }\n",
        )
        .unwrap();
        let compiled = Compiler::from_file(&entry, &dir)
            .expect("compiler from_file")
            .fresh_registry()
            .compile();
        assert!(
            compiled.pipeline_errors.iter().any(|e| e.code == "E0927"),
            "hidden @b/badge should raise E0927, got: {:?}",
            compiled
                .pipeline_errors
                .iter()
                .map(|e| &e.code)
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn userland_emit_html_renders_into_body() {
        // FEAT-082 Piece A: a userland %emit html macro renders into the body.
        let src = "%macro note {\n  %form { @note $msg:string }\n  %emit html { <aside class=\"note\">%$msg</aside> }\n}\n<main><h1>P</h1></main>\n.x { @note \"hi there\"; }\n";
        let mut ast = crate::parser::parse(src).expect("parse");
        crate::parser::rematch_with_user_macros(&mut ast, src);
        let compiled = compile(&ast, CompileOptions::new().with_fresh_registry());
        assert!(
            compiled
                .html
                .contains("<aside class=\"note\">hi there</aside>"),
            "userland %emit html must splice into body: {}",
            compiled.html
        );
    }

    #[test]
    fn example_dual_renders_live_and_escaped_source() {
        // FEAT-082 Piece B/C: the stdlib @example shows the block live AND as
        // escaped source from ONE capture.
        let src = "@import \"stdlib/macros/docs\";\n<main><div class=\"s\"></div></main>\n.s { @example \"Btn\" { <button class=\"b\">Go</button> } }\n";
        let mut ast = crate::parser::parse(src).expect("parse");
        crate::parser::rematch_with_user_macros(&mut ast, src);
        let compiled = compile(&ast, CompileOptions::new().with_fresh_registry());
        // Live pane: real markup runs.
        assert!(
            compiled.html.contains("<button class=\"b\">Go</button>"),
            "live pane must hold real markup: {}",
            compiled.html
        );
        // Source pane: the same block, HTML-escaped (the dual-render proof).
        assert!(
            compiled
                .html
                .contains("&lt;button class=&quot;b&quot;&gt;Go&lt;/button&gt;"),
            "source pane must hold escaped source: {}",
            compiled.html
        );
        // Both panes present → the figure wraps them.
        assert!(
            compiled.html.contains("st-example"),
            "st-example figure expected"
        );
    }

    #[test]
    fn data_registry_filter_keeps_only_matching_kind() {
        let src = "@data registry $tests from kind \"test\";\n\
<main><div class=\"g\"></div></main>\n\
.g { @each($tests as $f) { <span class=\"c\">`$f.name`</span> } }\n";
        let compiled = compile_default(src);
        assert!(
            compiled.html.contains("@assert"),
            "test-kind @assert expected"
        );
        // A text-kind directive must NOT appear under the test filter.
        assert!(
            !compiled.html.contains("@balance"),
            "text-kind @balance must be filtered out of a kind=test slice"
        );
    }

    #[test]
    fn html_file_scope_element_lowers_to_html() {
        let compiled = compile_default("<main><h1>Hello</h1></main>");
        assert!(
            compiled.html.contains("<main>") && compiled.html.contains("<h1>"),
            "compiled.html should contain the file-scope markup: {:?}",
            compiled.html
        );
        assert!(
            compiled.html.contains("Hello"),
            "text content should survive: {:?}",
            compiled.html
        );
    }

    #[test]
    fn html_static_only_emits_no_js() {
        // A pure static HTML page has no reactive directives -> no JS runtime needed.
        let compiled = compile_default("<section><p>Static content</p></section>");
        assert!(
            compiled.html.contains("<section>") && compiled.html.contains("Static content"),
            "html: {:?}",
            compiled.html
        );
        assert!(
            compiled.js.is_empty(),
            "static HTML page should emit no JS: {:?}",
            compiled.js
        );
    }

    #[test]
    fn html_hole_lowers_to_placeholder() {
        // FEAT-078: a file-scope text hole is HYDRATED, not a comment placeholder. It renders a
        // stable marker (`<span data-st-hole="N">INITIAL</span>`) carrying the compile-time
        // initial value (SSG / no-JS), and the bundle JS re-renders it on `local:<dep>:updated`.
        let compiled = compile_default("$x number: 7;\n<li>`$x`</li>");
        assert!(compiled.html.contains("<li>"), "html: {:?}", compiled.html);
        // No legacy comment placeholder remains.
        assert!(
            !compiled.html.contains("st-hole:$x") && !compiled.html.contains("<!--"),
            "no comment placeholder: {:?}",
            compiled.html
        );
        // SSG: the marker carries the initial value.
        assert!(
            compiled.html.contains("data-st-hole=\"0\"") && compiled.html.contains(">7</span>"),
            "marker + initial value: {:?}",
            compiled.html
        );
        // Reactive: the bundle wires the hole to the global signal.
        assert!(
            compiled.js.contains("local:x:updated") && compiled.js.contains("SpacetimeLocal['x']"),
            "hole hydration JS: {:?}",
            compiled.js
        );
    }

    #[test]
    fn html_holes_in_separate_blocks_get_unique_ids() {
        // BUG-067 #2: hole marker ids must be unique ACROSS top-level blocks (bindings address
        // them by a document-wide querySelectorAll), else two holes collide.
        let compiled = compile_default(
            "$a string: \"A\";\n$b string: \"B\";\n<header><span>`$a`</span></header>\n<main><span>`$b`</span></main>",
        );
        assert!(
            compiled.html.contains("data-st-hole=\"0\""),
            "first hole id 0: {:?}",
            compiled.html
        );
        assert!(
            compiled.html.contains("data-st-hole=\"1\""),
            "second hole id 1 (unique): {:?}",
            compiled.html
        );
        // Both bindings present, addressing distinct selectors.
        assert!(
            compiled.js.contains("local:a:updated") && compiled.js.contains("local:b:updated"),
            "both dep listeners: {:?}",
            compiled.js
        );
    }

    #[test]
    fn html_absent_when_no_markup() {
        let compiled = compile_default(".hero { color: red; }");
        assert!(
            compiled.html.is_empty(),
            "non-HTML source must not produce body HTML: {:?}",
            compiled.html
        );
    }

    #[test]
    fn test_emit_options_with_source_maps_builder() {
        // Test the with_source_maps builder method
        let opts = EmitOptions::pretty().with_source_maps(true);
        assert!(opts.source_maps);
        assert!(opts.source_maps_include_content);

        let opts2 = EmitOptions::minified().with_source_maps(false);
        assert!(opts2.source_maps);
        assert!(!opts2.source_maps_include_content);
    }

    #[test]
    fn test_no_inline_source_maps_without_debug() {
        use crate::parser::StFile;

        let ast = StFile::default();

        // Compile without debug
        let compiled = Compiler::from_ast(&ast).compile();

        // Should NOT contain source map comments
        assert!(!compiled.js.contains("sourceMappingURL"));
        assert!(!compiled.css.contains("sourceMappingURL"));

        // Source maps should be None
        assert!(compiled.js_source_map.is_none());
        assert!(compiled.css_source_map.is_none());
    }

    #[test]
    fn test_compile_options_registry_source_default_is_cached() {
        let opts = CompileOptions::default();
        assert!(matches!(opts.registry_source, RegistrySource::Cached));
    }

    #[test]
    fn test_compile_options_with_fresh_registry() {
        let opts = CompileOptions::new().with_fresh_registry();
        assert!(matches!(opts.registry_source, RegistrySource::Fresh));
    }

    #[test]
    fn test_compile_options_with_provided_registry() {
        let registry = MetaRegistry::new();
        let opts = CompileOptions::new().with_registry(registry);
        assert!(matches!(opts.registry_source, RegistrySource::Provided(_)));
    }

    #[cfg(feature = "headless")]
    #[test]
    fn test_validation_result_to_diagnostics() {
        // Test converting validation errors to diagnostics
        let result = ValidationResult {
            js_errors: vec![crate::validation::JsSyntaxError::with_location(
                "test JS error".to_string(),
                5,
                10,
            )],
            css_errors: vec![crate::validation::CssSyntaxError::new(
                "test CSS error".to_string(),
            )],
        };

        let diagnostics = result.to_diagnostics();
        assert_eq!(diagnostics.len(), 2);

        // Check JS error diagnostic
        assert!(
            diagnostics[0]
                .message
                .contains("Invalid generated JavaScript")
        );
        assert!(matches!(
            diagnostics[0].code,
            crate::diagnostics::DiagnosticCode::E0701
        ));

        // Check CSS error diagnostic
        assert!(diagnostics[1].message.contains("Invalid generated CSS"));
        assert!(matches!(
            diagnostics[1].code,
            crate::diagnostics::DiagnosticCode::E0702
        ));
    }

    // ========================================================================
    // Tests for DX improvements: "Did you mean?" suggestions
    // ========================================================================

    #[test]
    fn test_unresolved_param_suggests_similar_name() {
        use crate::parser::SourceSpan;
        use pipeline::{CompileError, CompileErrorKind};

        // Create a CompileError with a typo in param name
        // "fsp" should suggest "fps" as similar
        let pipeline_err = CompileError::new(
            CompileErrorKind::UnresolvedParam(
                "fsp".to_string(),
                vec![
                    "fps".to_string(),
                    "autoStart".to_string(),
                    "container".to_string(),
                ],
            ),
            SourceSpan::default(),
        )
        .with_primitive_name("test-primitive")
        .with_primitive_file("test.st");
        let error_info = pipeline_error_to_info(&pipeline_err);

        // Should suggest "fps" since "fsp" is a typo
        let hint = error_info.hint.expect("Should have a hint");
        assert!(
            hint.contains("Did you mean"),
            "Hint should include 'Did you mean'"
        );
        assert!(hint.contains("fps"), "Hint should suggest 'fps'");
    }

    #[test]
    fn test_unresolved_param_shows_available_when_no_similar() {
        use crate::parser::SourceSpan;
        use pipeline::{CompileError, CompileErrorKind};

        // Create a CompileError with a param name very different from available ones
        let pipeline_err = CompileError::new(
            CompileErrorKind::UnresolvedParam(
                "xyz".to_string(),
                vec!["fps".to_string(), "autoStart".to_string()],
            ),
            SourceSpan::default(),
        )
        .with_primitive_name("test-primitive");
        let error_info = pipeline_error_to_info(&pipeline_err);

        // Should NOT suggest similar (too different), but should show available params
        let hint = error_info.hint.expect("Should have a hint");
        assert!(
            hint.contains("Available parameters:"),
            "Hint should show available params"
        );
        assert!(
            hint.contains("fps"),
            "Hint should include 'fps' in available list"
        );
        assert!(
            hint.contains("autoStart"),
            "Hint should include 'autoStart' in available list"
        );
    }

    #[test]
    fn test_unresolved_param_with_empty_available_shows_helpful_message() {
        use crate::parser::SourceSpan;
        use pipeline::{CompileError, CompileErrorKind};

        // Create a CompileError with no available params
        let pipeline_err = CompileError::new(
            CompileErrorKind::UnresolvedParam(
                "missing".to_string(),
                vec![], // No available params
            ),
            SourceSpan::default(),
        )
        .with_primitive_name("test-primitive");
        let error_info = pipeline_error_to_info(&pipeline_err);

        // Should have a helpful message even without available params
        let hint = error_info.hint.expect("Should have a hint");
        assert!(
            hint.contains("not found"),
            "Hint should indicate param was not found"
        );
        assert!(
            hint.contains("captured") || hint.contains("argument"),
            "Hint should mention capturing or arguments"
        );
    }

    #[test]
    fn test_unresolved_param_error_code() {
        use crate::parser::SourceSpan;
        use pipeline::{CompileError, CompileErrorKind};

        let pipeline_err = CompileError::new(
            CompileErrorKind::UnresolvedParam("test".to_string(), vec!["param1".to_string()]),
            SourceSpan::default(),
        );
        let error_info = pipeline_error_to_info(&pipeline_err);

        // Error code for UnresolvedParam should be E0815
        assert_eq!(error_info.code, "E0815");
    }

    // ========================================================================
    // E0402 filtering for file-level templates
    // ========================================================================

    #[test]
    fn bug121_inline_directive_attr_refused_e0930() {
        // BUG-121: `<button @mcp-action="approve">` inside a @template body must be REFUSED
        // with E0930, not silently lowered to a dead setAttribute. The message names the
        // directive + the selector-rule remedy.
        let source = "@template &panel() {\n  <button @mcp-action=\"approve\">Approve</button>\n}\n.s { &panel(); }\n";
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        let e0930 = compiled
            .pipeline_errors
            .iter()
            .find(|e| e.code == "E0930")
            .unwrap_or_else(|| {
                panic!(
                    "inline @-attr must raise E0930, got: {:?}",
                    compiled
                        .pipeline_errors
                        .iter()
                        .map(|e| &e.code)
                        .collect::<Vec<_>>()
                )
            });
        assert!(
            e0930.message.contains("@mcp-action"),
            "names the directive: {}",
            e0930.message
        );
        assert!(
            e0930.message.contains("button"),
            "names the element: {}",
            e0930.message
        );
        assert!(
            e0930
                .hint
                .as_ref()
                .is_some_and(|h| h.contains("selector rule")),
            "hint steers to selector rule: {:?}",
            e0930.hint
        );
    }

    #[test]
    fn bug121_file_scope_inline_directive_attr_refused_e0930() {
        // The guard also covers file-scope HTML (not just @template bodies).
        let source = "<main><button @on=\"x\">Go</button></main>\n";
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        assert!(
            compiled.pipeline_errors.iter().any(|e| e.code == "E0930"),
            "file-scope inline @-attr must raise E0930, got: {:?}",
            compiled
                .pipeline_errors
                .iter()
                .map(|e| &e.code)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bug121_clean_markup_no_e0930() {
        // data-* attributes and a real selector-scoped directive must NOT trip the guard.
        let source = "@template &panel() {\n  <button class=\"approve\" data-mcp-action=\"kit-choice\">Approve</button>\n}\n.approve { @on &.click { $x <- \"1\"; } }\n.s { &panel(); }\n";
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        assert!(
            !compiled.pipeline_errors.iter().any(|e| e.code == "E0930"),
            "clean markup must not raise E0930, got: {:?}",
            compiled
                .pipeline_errors
                .iter()
                .map(|e| (&e.code, &e.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_e0402_filtered_for_file_level_templates() {
        let source = r#"
@template &card($title) {
    <div class="card">`$title`</div>
}

div {
    &card("hello") {}
}
"#;
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        let e0402s: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| e.code == "E0402")
            .collect();
        assert!(
            e0402s.is_empty(),
            "Should have zero E0402 for defined template 'card', got: {:?}",
            e0402s.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_e0402_filter_does_not_remove_non_template_errors() {
        // The E0402 filter only touches E0402 codes — other pipeline errors
        // should pass through untouched
        let source = r#"
@template &card($title) {
    <div>`$title`</div>
}

div {
    &card("hi") {}
}
"#;
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        // No non-E0402 errors should be introduced or removed by the filter
        let non_e0402: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| e.code != "E0402")
            .collect();
        // This is a basic sanity check — the filter should not corrupt other errors
        // (The source compiles cleanly so we just verify no spurious errors)
        for err in &non_e0402 {
            assert!(
                !err.code.is_empty(),
                "Pipeline errors should have non-empty codes"
            );
        }
    }

    #[test]
    fn test_e0402_multiple_templates_all_filtered() {
        let source = r#"
@template &nav($label) {
    <nav>`$label`</nav>
}

@template &footer($text) {
    <footer>`$text`</footer>
}

header {
    &nav("Home") {}
}

main {
    &footer("Copyright") {}
}
"#;
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        let e0402s: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| e.code == "E0402")
            .collect();
        assert!(
            e0402s.is_empty(),
            "Should have zero E0402 for defined templates 'nav' and 'footer', got: {:?}",
            e0402s.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_e0402_mixed_defined_and_missing() {
        // Template invocations inside a @template body: defined ones should be
        // filtered, missing ones should be reported
        let source = r#"
@template &card($title) {
    <div>`$title`</div>
}

@template &page($content) {
    &card("nested") {}
    &missing("x") {}
}
"#;
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        let compiled = compile(&ast, CompileOptions::default());
        let e0402s: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| e.code == "E0402")
            .collect();
        // Should NOT have E0402 for "card"
        assert!(
            !e0402s.iter().any(|e| e.message.contains("card")),
            "Should not have E0402 for defined template 'card'"
        );
        // Should have E0402 for "missing"
        assert!(
            e0402s.iter().any(|e| e.message.contains("missing")),
            "Should have E0402 for missing template 'missing', errors: {:?}",
            e0402s.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hash_stdlib_mtimes_consistent() {
        // Calling hash_stdlib_mtimes() twice with no changes should return the same hash
        let hash1 = hash_stdlib_mtimes();
        let hash2 = hash_stdlib_mtimes();
        assert_eq!(hash1, hash2, "hash_stdlib_mtimes() should be deterministic");
        // Hash should be non-zero (stdlib directory exists and has files)
        assert_ne!(hash1, 0, "hash should be non-zero when stdlib files exist");
    }

    /// Regression test: site-local macros must not break @template resolution.
    ///
    /// When `rematch_with_user_macros` re-extracts FormMatches for user-defined
    /// macros, it must preserve the scope assignment of stdlib matches. Previously,
    /// imported scopes (with spans from a different file) could capture file-level
    /// `@template` definitions, giving them a selector that prevents the `%scope file`
    /// macro from matching. And clearing all matches destroyed correctly-assigned
    /// stdlib matches from imported files.
    #[test]
    fn test_rematch_preserves_template_resolution() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("st-test-rematch-templates");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Theme file: a scope block whose span (relative to _theme.st) could
        // collide with main-file positions if naively compared.
        std::fs::File::create(dir.join("_theme.st"))
            .unwrap()
            .write_all(b"[data-theme=\"test\"] {\n    --bg: #fff;\n}\n")
            .unwrap();

        // Site-local macro (triggers rematch_with_user_macros)
        std::fs::File::create(dir.join("_local.st"))
            .unwrap()
            .write_all(b"%primitive local-fx(&el) {\n  %emit js { console.log('fx'); }\n}\n\n%macro local-fx-macro {\n  %form { @local-fx() }\n  %binds { local-fx(&self) }\n}\n")
            .unwrap();

        // Main file: imports theme + local macro, defines a template, invokes it
        let index_content = r#"@import "./_theme.st";
@import "./_local.st";

@template &card($title) {
    <div class="card">`$title`</div>
}

.mount {
    &card("Hello");
}

body {
    @local-fx()
}
"#;
        std::fs::File::create(dir.join("index.st"))
            .unwrap()
            .write_all(index_content.as_bytes())
            .unwrap();

        let compiled = Compiler::from_file(&dir.join("index.st"), &dir)
            .expect("should parse")
            .compile();

        let e0402s: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| e.code == "E0402")
            .collect();

        assert!(
            e0402s.is_empty(),
            "@template 'card' defined in same file must be found; got E0402: {:?}",
            e0402s.iter().map(|e| &e.message).collect::<Vec<_>>()
        );

        // The local-fx macro should also emit JS (not 'Primitive not found')
        assert!(
            !compiled.js.contains("Primitive not found"),
            "Site-local primitive should resolve; JS contains 'Primitive not found'",
        );

        // Template factory should be registered
        assert!(
            compiled.js.contains("card"),
            "Template 'card' should produce JS output",
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // =========================================================================
    // Reactive output: signal/host/handle emit (PLAN-038 FUP-080/081)
    // =========================================================================

    /// The canonical http signal + handle program used by the emit assertions.
    fn signal_program() -> &'static str {
        r#"@host $api : http("https://api.example.com") { headers: { authorization: "Bearer x" } }
@data signal $add($text string) to $api {
  send POST "/api/todos" { title: $text }
  receive to AddResult {
    200 => Created($.body as Todo) final;
    422 => Invalid($.body as Errors);
    _   => Failed($.statusText);
  }
  policy queue timeout 3s retry 2
}
.app {
  @handle $add {
    optimistic { $rows <- $rows.concat({ title: "pending" }); }
    receive {
      Created(todo) => { $rows <- $rows.concat(todo); }
      Invalid(errs) => { $err <- errs; }
      Failed(msg) => { $err <- msg; }
    }
    final { $draft <- ""; }
  }
  &button "Add"
}
"#
    }

    #[test]
    fn host_emits_runtime_registration() {
        // @host must populate window.__stHosts at runtime (FUP-081 #4) — not just a
        // compile-time fact. signal-call resolveHost() reads this table.
        let js = compile_default(signal_program()).js;
        assert!(
            js.contains("window.__stHosts[key] = entry"),
            "host registration must emit"
        );
        assert!(
            js.contains("https://api.example.com"),
            "the host base url must be in the registration"
        );
        assert!(
            !js.contains("Primitive not found: host"),
            "@host must resolve to a real primitive"
        );
    }

    #[test]
    fn signal_emits_decode_table() {
        // The receive arms must lower into the runtime decode machinery (FUP-080 #1).
        let js = compile_default(signal_program()).js;
        assert!(js.contains("function decodeReply"), "decodeReply must emit");
        assert!(js.contains("function armMatches"), "armMatches must emit");
        assert!(js.contains("function evalPayload"), "evalPayload must emit");
        // The arms render as a JS structure carrying ctor + payload.
        assert!(
            js.contains("\"Created\"") || js.contains("ctor: \"Created\""),
            "the Created variant must be in the decode table"
        );
    }

    #[test]
    fn signal_emits_policy_and_lifecycle() {
        // policy queue + timeout/retry must thread to the runtime (FUP-081 #3).
        let js = compile_default(signal_program()).js;
        assert!(
            js.contains("policyMode === 'queue'"),
            "queue policy branch must emit"
        );
        assert!(
            js.contains("policyMode === 'drop'"),
            "drop policy branch must emit"
        );
        // The DURATION reaches the runtime as the author wrote it and is
        // normalized there — `parseMs("3s")` -> 3000 (signal.st's parseMs
        // handles ms/s/m). This asserted `parseMs(3000)`, i.e. that the
        // compiler pre-converts; it never did, and the runtime is the right
        // place for it since `%timeout` is substituted verbatim.
        assert!(
            js.contains("var timeoutMs = parseMs(\"3s\")"),
            "timeout 3s must reach the runtime's parseMs to be honored"
        );
        assert!(
            js.contains("var retryMax = Number(2)"),
            "retry 2 must thread"
        );
    }

    #[test]
    fn handle_emits_variant_dispatch_and_clauses() {
        // @handle must dispatch decoded variants to arms + run optimistic/final
        // (FUP-080 #2,#4). The capture-collision fix means BOTH optimistic and
        // final clause bodies surface (not just one).
        let js = compile_default(signal_program()).js;
        assert!(
            js.contains("reply.variant"),
            "handler dispatches on the decoded variant"
        );
        assert!(
            js.contains("signal:' + signalName + ':fire"),
            "fire event drives optimistic"
        );
        assert!(
            js.contains("function onFire"),
            "optimistic fire hook must emit"
        );
        assert!(
            js.contains("reply.final"),
            "final clause runs on terminal outcome"
        );
        // Both clause bodies present (collision fix): optimistic concat + final reset.
        assert!(
            js.contains("optimisticStmts = \"$rows <- $rows.concat"),
            "optimistic clause body must emit"
        );
        assert!(
            js.contains("finalStmts = \"$draft"),
            "final clause body must emit"
        );
    }

    #[test]
    fn ws_signal_emits_socket_transport() {
        // A ws @host + @data stream must emit the ws request path (FUP-081 #2).
        let src = r#"@host $mcp : ws("/ws")
@data stream $watch($id string) to $mcp {
  send emit "subscribe" { id: $id }
  receive to Event {
    "previewed" => Previewed($.payload as Preview);
    "chosen" => Chosen($.payload as Preview) final;
  }
}
.app { @handle $watch { receive { Previewed(p) => { $preview <- p; } } } &button "x" }
"#;
        let js = compile_default(src).js;
        assert!(
            js.contains("function doWsRequest"),
            "ws request path must emit"
        );
        assert!(js.contains("new WebSocket"), "ws opens a socket");
        assert!(
            js.contains("host.transport === 'ws'"),
            "transport dispatch must branch on ws"
        );
    }

    // =========================================================================
    // @drag on-drop firing (PLAN-053)
    // =========================================================================

    #[test]
    fn drag_on_drop_emits_transition_watcher() {
        // `@drag { on-drop: <action> }` must lower the macro-body `%on $active ->
        // false` clause into an ST.onTransition watcher that runs the on-drop
        // action via ST.runMutations. Before PLAN-053 the clause was dropped.
        let src = r#"@import "stdlib/dnd"
.card {
  @drag(axis: "both") {
    on-drop: $move({ card: $.dataset.id })
  }
}
"#;
        let js = compile_with_imports(src).js;
        assert!(
            js.contains("ST.onTransition"),
            "the %on $active -> false clause must emit an onTransition watcher, got: {}",
            js
        );
        assert!(
            js.contains("$move({ card: $.dataset.id })"),
            "the on-drop action must be carried into the watcher body, got: {}",
            js
        );
        // The watcher fires the action through the shared mutation rail.
        assert!(
            js.contains("ST.runMutations"),
            "the on-drop action must run via ST.runMutations"
        );
    }

    #[test]
    fn drag_on_drop_survives_preceding_states_block() {
        // A `dragging { ... }` states sub-block before `on-drop:` must NOT swallow
        // the on-drop property (the body-property boundary fix, PLAN-053). Before
        // the fix the on-drop action resolved to the form default `null`.
        let src = r#"@import "stdlib/dnd"
.card {
  @drag(axis: "both", momentum: true) {
    dragging {
      scale: 1.04
      z-index: 50
    }
    on-drop: $move({ card: $.dataset.id })
  }
}
"#;
        let js = compile_with_imports(src).js;
        assert!(
            js.contains("$move({ card: $.dataset.id })"),
            "on-drop must be captured even with a preceding dragging block, got: {}",
            js
        );
        assert!(
            !js.contains("js_statements: [\"null\"]"),
            "on-drop must NOT resolve to the null default when a states block precedes it"
        );
    }

    // =========================================================================
    // @drag %animates + reactive %derives (PLAN-054)
    // =========================================================================

    #[test]
    fn drag_animates_emits_reactive_transform() {
        // `@drag`'s `%animates { translate-x: $x; translate-y: $y }` must lower to
        // reactive ST.derive computed signals (feeding from the gesture's deltaX/
        // deltaY) + one ST.bindTransform composing the channels, so the element
        // follows the cursor. Before PLAN-054 %animates was dropped entirely.
        let src = r#"@import "stdlib/dnd"
.card {
  @drag(axis: "both") {
    on-drop: $move({ card: $.dataset.id })
  }
}
"#;
        let js = compile_with_imports(src).js;
        assert!(
            js.contains("ST.bindTransform"),
            "%animates must emit an ST.bindTransform binding, got: {}",
            js
        );
        // The x/y derives must be reactive computed signals over the gesture deltas.
        assert!(
            js.contains("ST.derive(el, \"x\""),
            "the $x derive must be lowered to a reactive ST.derive, got: {}",
            js
        );
        assert!(
            js.contains("ST.derive(el, \"rawX\", [\"deltaX\"]"),
            "rawX must derive from the runtime deltaX signal, got: {}",
            js
        );
        // The transform channels must reference the derived signals.
        assert!(
            js.contains("translateX('") || js.contains("'translateX('"),
            "a translateX transform token must be emitted, got: {}",
            js
        );
    }

    #[test]
    fn drag_animates_axis_inlines_capture() {
        // The `axis` capture is a compile-time literal: it must be INLINED into
        // the derive compute (not left as a runtime dep). For axis:\"x\", the rawY
        // derive's `$axis != \"x\"` becomes `\"x\" != \"x\"` => the y channel is pinned 0.
        let src = r#"@import "stdlib/dnd"
.card {
  @drag(axis: "x") {
    on-drop: $move({ card: $.dataset.id })
  }
}
"#;
        let js = compile_with_imports(src).js;
        assert!(
            js.contains("\"x\" != \"x\"") || js.contains("\"x\" != \"y\""),
            "the axis capture must inline as a string literal in the derive, got: {}",
            js
        );
        // axis must NOT appear as a runtime dependency of any derive.
        assert!(
            !js.contains("[\"axis\"]") && !js.contains("v.axis"),
            "the compile-time axis capture must not be a runtime derive dep, got: {}",
            js
        );
    }

    // ========================================================================
    // PLAN-123 W1 — the parse gate
    // ========================================================================

    /// THE INVARIANT of the comments feature: a `//@` comment is TRIVIA
    /// everywhere `//` is a comment — CSS scope bodies and file scope.
    ///
    /// Everything cheap about the design rests on this — the lenient scanner,
    /// the absence of `%migration` waves for comment syntax, the claim that a
    /// malformed header can only ever warn. All of it collapses the moment a
    /// comment can change emitted output, so this compares output BYTE FOR
    /// BYTE with and without the comments rather than merely checking that
    /// both compile.
    ///
    /// HTML MARKUP BLOCKS ARE EXCLUDED, and deliberately so: inside `<div>…`
    /// a `//` line is LITERAL TEXT that renders on the page — the same rule
    /// that makes a bare `$x` literal there (AGENTS.md, one sigil one
    /// meaning). That is pre-existing language behavior, not something this
    /// feature may quietly change; the sibling test pins it so nobody
    /// "fixes" it into an absorption bug later.
    #[test]
    fn inline_comments_have_zero_compile_effect() {
        let base = "@version 2026-06-09;\n\n<div class=\"stage\">\n  <h1 class=\"title\">Title</h1>\n</div>\n\n.stage { padding: 20px; }\n.title { color: red; }\n";

        // The same page, saturated with every shape of the surface at every
        // scope where `//` IS a comment: file scope and inside a CSS block.
        let with_comments = "@version 2026-06-09;\n\n//@: a file-scope note\n\n<div class=\"stage\">\n  <h1 class=\"title\">Title</h1>\n</div>\n\n//@bug(severity: high): nav overlaps below 380px\n//@> continuation lines are part of the body\n//@>\n//@> and survive a paragraph break\n.stage { padding: 20px; }\n.title { //@design: red is the brand accent\n  color: red; }\n";

        let dir = std::env::temp_dir().join(format!(
            "st_comment_parse_gate_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let compile = |name: &str, source: &str| {
            let entry = dir.join(name);
            std::fs::write(&entry, source).unwrap();
            Compiler::from_file(&entry, Path::new("."))
                .expect("compiler from_file")
                .with_site_dir(Some(dir.clone()))
                .compile()
        };

        let plain = compile("plain.st", base);
        let commented = compile("commented.st", with_comments);

        assert_eq!(
            plain.html, commented.html,
            "`//@` comments changed emitted HTML — the comment surface MUST be \
             inert. If this fails, the lenient scanner and the no-migrations \
             evolution story are both invalid."
        );
        assert_eq!(
            plain.css, commented.css,
            "`//@` comments changed emitted CSS — see above."
        );
        assert_eq!(
            plain.js, commented.js,
            "`//@` comments changed emitted JS — see above."
        );

        // And the text must not leak into output by another route (e.g. being
        // carried through as a literal comment in the emitted artifacts).
        for (label, out) in [
            ("html", &commented.html),
            ("css", &commented.css),
            ("js", &commented.js),
        ] {
            assert!(
                !out.contains("red is the brand accent"),
                "comment text leaked into emitted {label}"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The BOUNDARY of the comment surface, pinned as a fact.
    ///
    /// Inside an HTML markup block `//` is literal page text — it renders.
    /// This is pre-existing language behavior (the same reason a bare `$x` is
    /// literal there), and it is exactly why the scanner takes caller-supplied
    /// HTML exclusions instead of harvesting every `//@` it can see: a line
    /// the reader can SEE on the page must never be filed as a private note,
    /// and the pill must not offer to "resolve" visible copy.
    ///
    /// If this ever starts failing because `//` became a real comment in
    /// markup, that is a LANGUAGE change — decide it deliberately, then teach
    /// the scanner; do not let the comments feature drift it silently.
    #[test]
    fn double_slash_in_html_markup_is_literal_text_not_a_comment() {
        let dir = std::env::temp_dir().join(format!(
            "st_comment_html_boundary_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@version 2026-06-09;\n\n<div class=\"stage\">\n  // this renders\n  <h1>Title</h1>\n</div>\n",
        )
        .unwrap();

        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .with_site_dir(Some(dir.clone()))
            .compile();

        assert!(
            compiled.html.contains("// this renders"),
            "`//` inside HTML markup is literal text (pre-existing behavior). \
             Got: {}",
            compiled.html
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// THE PROJECT OVERLAY (PLAN-123), end to end: a type declared in a
    /// project's `_prelude.st` must become a first-class citizen of the
    /// roster — indistinguishable, at the point of use, from a stdlib type.
    /// This is the whole "features from syntax for free" claim: declare it,
    /// and every reader sees it with no Rust change.
    #[test]
    fn project_prelude_types_join_the_roster() {
        let dir = overlay_dir("joins");
        std::fs::write(
            dir.join(PROJECT_PRELUDE),
            "%comment_type brand-review {\n  %label \"Brand review\"\n  %docs \"Needs sign-off from brand before shipping.\"\n  %field reviewer string\n  %field due string?\n}\n",
        )
        .unwrap();

        let (mut registry, _) = load_stdlib_registry();
        let errors = load_project_overlay(&mut registry, Some(&dir)).errors;
        assert!(errors.is_empty(), "overlay must load cleanly: {errors:?}");

        let t = registry
            .comment_type("brand-review")
            .expect("the project type must be in the roster");
        assert_eq!(t.label, "Brand review");
        assert!(!t.field("reviewer").unwrap().optional);
        assert!(t.field("due").unwrap().optional);

        // Provenance is recorded so diagnostics can say WHERE a type came
        // from, while the lookup itself stays uniform.
        assert!(
            t.source_file
                .as_deref()
                .unwrap_or_default()
                .contains(PROJECT_PRELUDE),
            "project types must record their provenance"
        );

        // And the stdlib roster is still intact beside it.
        assert!(registry.comment_type("todo").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A project may NOT redefine a stdlib type id. Silently shadowing would
    /// make `//@todo` mean one thing in one project and another elsewhere,
    /// and an agent reading the hint would act on the wrong contract — so the
    /// collision is reported at LOAD time, where it is cheap to fix.
    #[test]
    fn a_duplicate_type_id_is_a_load_error() {
        let dir = overlay_dir("duplicate");
        std::fs::write(
            dir.join(PROJECT_PRELUDE),
            "%comment_type todo {\n  %label \"Our todo\"\n  %docs \"Shadowing the stdlib type.\"\n}\n",
        )
        .unwrap();

        let (mut registry, _) = load_stdlib_registry();
        let errors = load_project_overlay(&mut registry, Some(&dir)).errors;
        assert_eq!(errors.len(), 1, "the collision must be reported");
        assert!(
            errors[0].message.contains("duplicate comment type: todo"),
            "got: {}",
            errors[0].message
        );
        assert!(
            errors[0].message.contains("ambiguous"),
            "the message must say WHY it matters: {}",
            errors[0].message
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A project without a prelude is the ordinary case — it must cost
    /// nothing and produce no diagnostics.
    #[test]
    fn a_project_without_a_prelude_loads_cleanly() {
        let dir = overlay_dir("none");
        let (mut registry, _) = load_stdlib_registry();
        let before: Vec<String> = registry.comment_types().map(|t| t.id.clone()).collect();

        let errors = load_project_overlay(&mut registry, Some(&dir)).errors;
        assert!(errors.is_empty());

        let after: Vec<String> = registry.comment_types().map(|t| t.id.clone()).collect();
        assert_eq!(before, after, "absent overlay must change nothing");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A page in a sub-folder belongs to the nearest ancestor that holds a
    /// `_prelude.st`; a page with no prelude anywhere above it owns its own
    /// directory. Every single-file entry point (check/build/render/test)
    /// resolves the project this way, so `check <project>/missions/x.st.md`
    /// sees the brand forms `check <project>/` does.
    #[test]
    fn project_root_is_the_nearest_prelude_ancestor() {
        let dir = overlay_dir("root");
        std::fs::write(dir.join(PROJECT_PRELUDE), "").unwrap();
        let nested = dir.join("missions").join("deep");
        std::fs::create_dir_all(&nested).unwrap();
        let page = nested.join("x.st.md");
        std::fs::write(&page, "").unwrap();

        assert_eq!(project_root_for(&page), dir, "nested page climbs to the prelude");
        assert_eq!(project_root_for(&dir.join("index.st")), dir, "root page is its own project");

        let orphan_dir = overlay_dir("root-orphan");
        let orphan = orphan_dir.join("sub").join("y.st");
        std::fs::create_dir_all(orphan.parent().unwrap()).unwrap();
        // No prelude above (temp dir): the page's own directory.
        assert!(
            project_root_for(&orphan) == orphan_dir.join("sub")
                || project_root_for(&orphan).join(PROJECT_PRELUDE).is_file(),
            "without a prelude the page's own dir is the project"
        );

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&orphan_dir).ok();
    }

    /// The overlay must never make a page fail to compile. A prelude that
    /// cannot be read or parsed is reported, but the mechanism itself is not
    /// allowed to turn a broken side-file into a broken build for a page that
    /// never referenced it.
    #[test]
    fn a_malformed_prelude_reports_without_panicking() {
        let dir = overlay_dir("malformed");
        std::fs::write(dir.join(PROJECT_PRELUDE), "%comment_type {{{ unclosed\n").unwrap();

        let (mut registry, _) = load_stdlib_registry();
        let errors = load_project_overlay(&mut registry, Some(&dir)).errors;
        // Either it parses to nothing or it reports — both are survivable; a
        // panic or a lost stdlib roster is not.
        assert!(
            registry.comment_type("todo").is_some(),
            "stdlib roster survives"
        );
        for e in &errors {
            assert!(
                e.file_path.to_string_lossy().contains(PROJECT_PRELUDE),
                "errors must point at the prelude, got {:?}",
                e.file_path
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Editing a `%comment_type` in the overlay must invalidate the compile
    /// cache exactly as editing a stdlib entry does — otherwise an author
    /// declares a type in the dev server and the roster keeps serving the
    /// previous one until restart, which reads as "the feature is broken".
    #[test]
    fn the_overlay_participates_in_cache_invalidation() {
        let dir = overlay_dir("cachekey");

        let without = hash_stdlib_mtimes_with_overlay(Some(&dir));
        std::fs::write(
            dir.join(PROJECT_PRELUDE),
            "%comment_type a {\n  %label \"A\"\n  %docs \"d\"\n}\n",
        )
        .unwrap();
        let with = hash_stdlib_mtimes_with_overlay(Some(&dir));
        assert_ne!(
            without, with,
            "ADDING a prelude must invalidate the compile cache"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Declaring a project type in a PAGE file compiles, but the type never
    /// reaches the roster — a silent failure whose symptom ("unknown comment
    /// type") appears far from its cause. The compiler must say so at the
    /// declaration, and must NOT fail the build to do it.
    #[test]
    fn a_comment_type_in_a_page_file_warns_without_failing() {
        let dir = overlay_dir("pagedecl");
        let entry = dir.join("index.st");
        std::fs::write(
            &entry,
            "@version 2026-06-09;\n\n%comment_type stray {\n  %label \"Stray\"\n  %docs \"Declared in the wrong place.\"\n}\n\n<div class=\"stage\">Hi</div>\n",
        )
        .unwrap();

        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .with_site_dir(Some(dir.clone()))
            .compile();

        assert!(
            compiled.pipeline_errors.is_empty(),
            "a misplaced declaration must not fail the build: {:?}",
            compiled.pipeline_errors
        );
        let warning = compiled
            .migration_warnings
            .iter()
            .find(|w| w.code == "W0717")
            .expect("W0717 must be emitted for a page-declared comment type");
        assert!(warning.message.contains("stray"), "must name the type");
        assert!(
            warning
                .hint
                .as_deref()
                .unwrap_or_default()
                .contains(PROJECT_PRELUDE),
            "the hint must say WHERE the declaration belongs: {:?}",
            warning.hint
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    fn overlay_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "st_overlay_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The roster is DATA, and it must be the same data whether the stdlib is
    /// read from disk or from the embedded copy. Without this, `//@todo`
    /// resolves in a source checkout and is "unknown type" from an installed
    /// binary — a difference nobody would think to test by hand.
    #[test]
    fn comment_type_roster_loads_from_stdlib() {
        let (registry, _errors) = load_stdlib_registry();
        let ids: Vec<&str> = registry.comment_types().map(|t| t.id.as_str()).collect();

        for expected in ["note", "todo", "question", "bug", "design", "agent-task"] {
            assert!(
                ids.contains(&expected),
                "default comment type `{expected}` missing from the roster: {ids:?}"
            );
        }

        // Every type must be able to explain itself: the roster is a picker in
        // the pill and a contract for agents, and an unlabelled entry is a bad
        // affordance in both.
        for t in registry.comment_types() {
            assert!(!t.label.is_empty(), "comment type `{}` has no %label", t.id);
            assert!(!t.docs.is_empty(), "comment type `{}` has no %docs", t.id);
        }

        // agent-task's acceptance is REQUIRED by design: a task whose
        // completion cannot be checked cannot honestly be closed.
        let agent_task = registry
            .comment_type("agent-task")
            .expect("agent-task must exist");
        let acceptance = agent_task
            .field("acceptance")
            .expect("agent-task must declare `acceptance`");
        assert!(
            !acceptance.optional,
            "`acceptance` must be REQUIRED — an agent needs an observable finish line"
        );
        assert!(
            agent_task.agent_hint.is_some(),
            "agent-task must carry its follow-up contract as %agent_hint"
        );

        // A hint is the INSTRUCTION an agent executes, so its prose must
        // survive the lexer intact. Token-joining detached every comma and
        // colon ("described , then set status : resolved"), turning a
        // contract into something that reads like a ransom note.
        let hint = agent_task.agent_hint.as_deref().unwrap_or_default();
        assert!(
            !hint.contains(" ,") && !hint.contains(" .") && !hint.contains(" :"),
            "punctuation must stay attached in an agent hint: {hint}"
        );

        // And the optional marker must survive parsing (the lexer splits
        // `string?`, so this is a real hazard, not a hypothetical one).
        assert!(
            agent_task
                .field("priority")
                .expect("agent-task declares `priority`")
                .optional,
            "`priority string?` must parse as OPTIONAL"
        );
    }

    /// The scalar table is the SINGLE SOURCE for what a scalar means, so the
    /// registry must surface all 8 rows from stdlib and every row must answer
    /// the four questions (recognition, schema, zero, widget) consistently —
    /// a consumer reading the table must never find a half-populated row.
    #[test]
    fn scalar_type_roster_loads_from_stdlib() {
        let (registry, errors) = load_stdlib_registry();
        let ids: Vec<&str> = registry.scalar_types().map(|t| t.id.as_str()).collect();

        for expected in [
            "color", "length", "duration", "url", "richtext", "string", "number", "boolean",
        ] {
            assert!(
                ids.contains(&expected),
                "scalar `{expected}` missing from the roster: {ids:?}"
            );
        }

        // The whole point of the table: one row answers all four questions.
        let color = registry.get_scalar_type("color").expect("color must exist");
        assert_eq!(color.widget, "color");
        assert_eq!(color.format, "color");
        assert_eq!(color.schema, "string");

        // The one scalar whose zero is not the empty string.
        let number = registry
            .get_scalar_type("number")
            .expect("number must exist");
        assert_eq!(number.zero, "0");

        // A row missing a required key (%schema/%zero/%widget) would render as
        // a bare text box downstream — the exact failure this table closes.
        // %zero is legitimately the empty string for most scalars, so the
        // check is presence (missing_required_keys), never content.
        for t in registry.scalar_types() {
            assert!(
                t.missing_required_keys.is_empty(),
                "scalar `{}` is missing required key(s): {:?}",
                t.id,
                t.missing_required_keys
            );
        }

        let scalar_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("scalar"))
            .collect();
        assert!(
            scalar_errors.is_empty(),
            "stdlib scalars must validate cleanly: {:?}",
            scalar_errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    /// A typo'd sub-clause (`%widgt`) must be an ERROR, not a silent no-op —
    /// silently doing nothing is the exact failure class the table exists to
    /// remove.
    #[test]
    fn scalar_type_unknown_subclause_is_an_error() {
        let src = r#"
%scalar_type fizz {
  %capture "string"
  %schema "string"
  %zero ""
  %widget "text"
  %widgt "text"
}
"#;
        let mut reg = MetaRegistry::new();
        let file = crate::parser::parse(src).expect("source must parse");
        for def in file.meta_defs {
            reg.register(def).expect("register");
        }
        let errors = scalar_type_load_errors(&reg);
        assert!(
            errors.iter().any(|e| e.message.contains("widgt")),
            "unknown sub-clause must be named: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    /// A row missing a REQUIRED key (here %widget) must be an error naming the
    /// key — not a silent default that renders as a bare text box.
    #[test]
    fn scalar_type_missing_required_key_is_an_error() {
        let src = r#"
%scalar_type fizz {
  %capture "string"
  %schema "string"
  %zero ""
}
"#;
        let mut reg = MetaRegistry::new();
        let file = crate::parser::parse(src).expect("source must parse");
        for def in file.meta_defs {
            reg.register(def).expect("register");
        }
        let errors = scalar_type_load_errors(&reg);
        assert!(
            errors.iter().any(|e| e.message.contains("widget")),
            "missing required key must be named: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod snapshot_tests {
    //! Snapshot baseline for file-level directive compilation.
    //!
    //! These snapshots exist to guard every subsequent refactoring phase.
    //! If a snapshot diverges after a change, the change altered observable
    //! compiler output — intentional changes must be accepted with `cargo insta review`.
    use super::*;

    fn compile_source(source: &str) -> CompiledSpacetime {
        let ast = crate::parser::parse(source).expect("Parse should succeed");
        compile(&ast, CompileOptions::new().with_fresh_registry())
    }

    #[test]
    fn dev_template_invocations_stamp_structure_ids_but_builds_do_not() {
        let source = r#"
            @template &hero() { <section class=\"hero\"></section> }
            @template &note() { <aside class=\"note\"></aside> }
            @template &main() {
              <main></main>
              &hero();
              &note();
            }
        "#;
        let ast = crate::parser::parse(source).expect("parse template fixture");
        let dev = Compiler::from_ast(&ast)
            .inspector_node_stamps(true)
            .compile();
        assert!(dev.js.contains("data-st-node"), "dev output: {}", dev.js);
        assert!(dev.js.contains("\"0.1\""), "hero id missing: {}", dev.js);
        assert!(dev.js.contains("\"0.2\""), "note id missing: {}", dev.js);

        let build = Compiler::from_ast(&ast).compile();
        // Assert on the STAMP CALL, not the mere appearance of the attribute
        // name: the shared runtime legitimately mentions `[data-st-node]` in a
        // querySelectorAll (the pill counts how many instances one source
        // invocation rendered), so a substring check on the name alone
        // false-positives on every page. What must never appear in a production
        // build is the emission itself.
        assert!(
            !build.js.contains("setAttribute('data-st-node'"),
            "production output stamps inspector node ids: {}",
            build.js
        );
        assert!(
            !build.js.contains("\"0.1\"") && !build.js.contains("\"0.2\""),
            "production output leaked structure ids: {}",
            build.js
        );
    }

    /// BUG-154: a `%macro`'s `%binds { prim(...) -> { $export as $alias } }` output
    /// alias MUST remap the signal name a `%yield expr -> $export` in the primitive
    /// body writes, so the consuming page's `$alias` is actually populated. Before
    /// the fix the alias was parsed + stored for chaining but never threaded into
    /// `PrimitiveArgs.outputs`, so the yield wrote the RAW export name and `$alias`
    /// stayed dead (undefined) — the mechanism silently no-op'd.
    #[test]
    fn bug154_macro_binds_output_alias_remaps_yield_signal() {
        let src = concat!(
            "%primitive pulse(&el) {\n",
            "  %emit js {\n",
            "    var beat = 42;\n",
            "    %yield beat -> $beat;\n",
            "  }\n",
            "  %exports {\n",
            "    $beat: object\n",
            "  }\n",
            "}\n",
            "%macro heartbeat {\n",
            "  %scope selector\n",
            "  %order 50\n",
            "  %form {\n",
            "    @heartbeat\n",
            "  }\n",
            "  %binds {\n",
            "    pulse(&self) -> { $beat as $myBeat }\n",
            "  }\n",
            "}\n",
            ".widget { @heartbeat }\n",
        );
        // Use the from_file path (like the stdlib module tests): a user-defined
        // %primitive + %macro is resolved the same way production compiles env.st.
        let dir = std::env::temp_dir().join(format!(
            "st_bug154_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(&entry, src).unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .compile();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            compiled.pipeline_errors.is_empty(),
            "BUG-154 fixture errors: {:?}",
            compiled.pipeline_errors
        );
        let js = &compiled.js;
        // The yield must write the ALIAS name, not the raw export.
        assert!(
            js.contains("\"myBeat\""),
            "yield must remap to the alias `myBeat`; got JS:\n{js}"
        );
        // And it must NOT write the raw export name `beat` as an ST.set key
        // (a bare `beat` var reference in the body is fine; the SIGNAL key is not).
        assert!(
            !js.contains("set(el, \"beat\"") && !js.contains("set(el,\"beat\""),
            "yield must NOT write the raw export name `beat` as a signal key; got JS:\n{js}"
        );
    }

    // ── FEAT-119: World-A emit + I1 invariant (body scope totality) ───────────

    /// FEAT-119 T1 (named, local): a @template definition + invocation compiles
    /// through the World-A emit path; the rawBody (states/html) reaches the factory.
    #[test]
    fn feat119_t1_named_template_emits_from_scope() {
        let compiled = compile_source(
            "@template &counter($label) {\n  $count number: 0;\n  <div class=\"counter\"><span>`$label`</span></div>\n}\n.sidebar { &counter(\"Likes\"); }\n",
        );
        assert!(
            compiled.pipeline_errors.is_empty(),
            "T1 errors: {:?}",
            compiled.pipeline_errors
        );
        // The factory rawBody (serialized from the scope) carries the state + builder.
        assert!(
            compiled.js.contains("var_name") && compiled.js.contains("count"),
            "rawBody states must reach the factory JS"
        );
        assert!(
            compiled.js.contains("builder"),
            "reactive builder must be emitted from scope html"
        );
    }

    /// FEAT-142 WAVE A: `&name { @directive... }` lowers to a live entity scope that
    /// (a) registers the entity in the general `window.__stWorld.byName` registry via
    /// the synthesized `entity-scope-impl` primitive, and (b) still emits its interior
    /// directive against the per-entity marker selector `[data-st-entity="name"]`.
    /// DURABLE proof the dead-letter is fixed (was: 0-byte emit).
    #[test]
    fn feat142_wavea_entity_scope_registers_and_emits() {
        let compiled = compile_source(
            "@import \"stdlib\"\n&evernet { @scroll-spy }\n.probe { color: red; }\n",
        );
        assert!(
            compiled.pipeline_errors.is_empty(),
            "entity-scope compile errors: {:?}",
            compiled.pipeline_errors
        );
        assert!(
            compiled.js.contains("__stWorld"),
            "entity-scope must install the general world registry; js: {}",
            compiled.js
        );
        assert!(
            compiled.js.contains("evernet"),
            "entity-scope must register the entity by name; js: {}",
            compiled.js
        );
        // (c) the static marker element must exist in the page HTML so selector-init
        // emit can bind the interior directive to it.
        assert!(
            compiled.html.contains("st-entity-evernet"),
            "entity marker element (.st-entity-evernet) must be injected into page HTML; html: {}",
            compiled.html
        );
        // (d) the interior @scroll-spy directive must actually emit its runtime logic
        // (bound to the entity marker) — the true dead-letter fix.
        assert!(
            compiled.js.contains("IntersectionObserver"),
            "interior @scroll-spy must emit against the entity marker; js: {}",
            &compiled.js[..compiled.js.len().min(4000)]
        );
    }

    /// FEAT-119 T2 (exports payload from scope): a @template with `@exports` emits the
    /// export metadata from the World-A scope (not the retired capture field).
    #[test]
    fn feat119_t2_exports_emitted_from_scope() {
        let compiled = compile_source(
            "@template &counter($l) {\n  $count number: 0;\n  @exports { $count: mut }\n  <div><span>`$l`</span></div>\n}\n.s { &counter(\"x\"); }\n",
        );
        assert!(
            compiled.pipeline_errors.is_empty(),
            "T2 errors: {:?}",
            compiled.pipeline_errors
        );
        // The exports payload (serialized from scope.exports) reaches the factory JS.
        assert!(
            compiled.js.contains("exports") && compiled.js.contains("mutable"),
            "export metadata must be serialized from the scope, got js: {}",
            compiled.js
        );
    }

    /// FEAT-119 T5 (nested definition): a @template inside a @template — BOTH produce
    /// their own `@template:<name>` scope and emit independently.
    #[test]
    fn feat119_t5_nested_template_definitions_each_emit() {
        let compiled = compile_source(
            "@template &inner() {\n  <em>inner</em>\n}\n@template &outer() {\n  <div>outer</div>\n  &inner();\n}\n.p { &outer(); }\n",
        );
        assert!(
            compiled.pipeline_errors.is_empty(),
            "T5 errors: {:?}",
            compiled.pipeline_errors
        );
        // Both template factories registered (two distinct rawBody objects).
        assert!(
            compiled.js.matches("builder").count() >= 2,
            "both inner+outer must emit their own builder, got js: {}",
            compiled.js
        );
    }

    /// FEAT-119 T6 (the I1 tripwire): the body validator returns ONLY the validation
    /// channel (diagnostics + section_transitions) — never a payload. Proves the
    /// capture cannot smuggle html/states/exports/refs (the World-B residue is gone).
    #[test]
    fn feat119_t6_validator_returns_only_validation_channel() {
        use crate::syntax::events::form_compiler::validate_component_body;
        // A clean body: zero diagnostics.
        let clean = validate_component_body("<div class=\"c\"><b>hi</b></div>", 0, "test");
        assert!(
            clean.is_empty(),
            "clean body emits no diagnostics, got: {:?}",
            clean
        );
        // A malformed @on (no mutation): the validator emits a canonical E0906 Diagnostic
        // — never resurrects a payload field (the struct + its envelope are deleted).
        let bad = validate_component_body(
            "<div class=\"c\"></div>\n.c { @on &.click { noop(); } }",
            0,
            "test",
        );
        assert!(
            bad.iter().any(|d| d.code.as_str() == "E0906"),
            "malformed @on must surface E0906, got: {:?}",
            bad
        );
    }

    // ── FEAT-120: body diagnostics flow through the canonical channel ─────────
    //
    // These assert the FULL path (parse -> StFile.diagnostics -> output.diagnostics ->
    // CompiledSpacetime.pipeline_errors), proving the retired ComponentBodyDef capture
    // envelope + its two bridges are gone and a malformed body still surfaces loud.

    /// FEAT-120 T1: a malformed body construct surfaces E0900 via the canonical channel.
    #[test]
    fn feat120_t1_malformed_body_surfaces_e0900() {
        let compiled = compile_source(
            "@template &card($t) {\n  <div class=\"c\">`$t`</div>\n  $broken\n}\n.s { &card(\"x\"); }\n",
        );
        assert!(
            compiled.pipeline_errors.iter().any(|e| e.code == "E0900"),
            "malformed body must surface E0900 via output.diagnostics, got: {:?}",
            compiled.pipeline_errors
        );
    }

    /// FEAT-120 T2: a malformed @exports entry surfaces E0904 via the unified scope-loop site.
    #[test]
    fn feat120_t2_malformed_exports_surfaces_e0904() {
        let compiled = compile_source(
            "@template &card($t) {\n  @exports { not_a_var }\n  <div class=\"c\">`$t`</div>\n}\n.s { &card(\"x\"); }\n",
        );
        assert!(
            compiled.pipeline_errors.iter().any(|e| e.code == "E0904"),
            "malformed @exports must surface E0904, got: {:?}",
            compiled.pipeline_errors
        );
    }

    /// FEAT-120 T3: a clean body produces ZERO body diagnostics (no envelope, no noise).
    #[test]
    fn feat120_t3_clean_body_no_diagnostics() {
        let compiled = compile_source(
            "@template &card($t) {\n  <div class=\"c\">`$t`</div>\n  .c { color: red; }\n}\n.s { &card(\"x\"); }\n",
        );
        let body_codes = ["E0900", "E0904", "E0906"];
        assert!(
            !compiled
                .pipeline_errors
                .iter()
                .any(|e| body_codes.contains(&e.code.as_str())),
            "clean body must produce no body diagnostics, got: {:?}",
            compiled.pipeline_errors
        );
    }

    /// FEAT-120 T4 (the invariant): body diagnostics reach the pipeline output AS canonical
    /// `Diagnostic`s with real spans. Proves there is no second diagnostic type and the
    /// capture carries no validation payload — the span points at the offending segment, not
    /// the whole match (the old bridge's only granularity).
    #[test]
    fn feat120_t4_body_diagnostic_has_segment_span() {
        let source = "@template &card($t) {\n  <div class=\"c\">`$t`</div>\n  $broken\n}\n.s { &card(\"x\"); }\n";
        let compiled = compile_source(source);
        let e0900 = compiled
            .pipeline_errors
            .iter()
            .find(|e| e.code == "E0900")
            .expect("E0900 present");
        let span = e0900.span.expect("E0900 carries a real span");
        // The span must point at `$broken` (line 3), NOT span the whole template match.
        let sliced = &source[span.start..span.end];
        assert!(
            sliced.contains("broken"),
            "span must locate the offending segment `$broken`, got slice: {:?}",
            sliced
        );
    }

    // ── @template (definition only) ──────────────────────────────────────────

    #[test]
    fn snapshot_template_definition() {
        let source = r#"
@template &card($title, $body) {
    <div class="card">
        <h2>`$title`</h2>
        <div>`$body`</div>
    </div>
}
"#;
        let compiled = compile_source(source);
        assert!(
            compiled.pipeline_errors.is_empty(),
            "Expected no errors, got: {:?}",
            compiled.pipeline_errors
        );
        insta::assert_snapshot!("template_definition_js", compiled.js);
        insta::assert_snapshot!("template_definition_css", compiled.css);
    }

    // ── @template definition + invocation (mixed file) ───────────────────────

    #[test]
    fn snapshot_template_definition_and_invocation() {
        let source = r#"
@template &badge($label) {
    <span class="badge">`$label`</span>
}

div.widget {
    @on &.load {
        opacity: 0 -> 1;
    }
}
"#;
        let compiled = compile_source(source);
        insta::assert_snapshot!("template_def_invocation_js", compiled.js);
        insta::assert_snapshot!("template_def_invocation_css", compiled.css);
    }

    // ── @data ─────────────────────────────────────────────────────────────────

    #[test]
    fn snapshot_data_directive() {
        // Unified @data <kind> surface (PLAN-023 W5/FEAT-047): the legacy block form
        // `@data NAME : T { src:; cache: }` was removed; `@data fetch` is the surface.
        // BUG-137's body-shape resolution routes a `{ refresh: … }` options body to the
        // body-consuming `data-fetch-opts` overload, and BUG-138 makes the `:duration`
        // capture convert `5m`→ms — so this asserts the full path: the snapshot must
        // show `refreshInterval = 300000` (5 minutes in ms), unquoted, NOT "5m".
        let source = r#"
@data fetch $products Product[] : "/api/products" { refresh: 5m }
"#;
        let compiled = compile_source(source);
        insta::assert_snapshot!("data_directive_js", compiled.js);
        insta::assert_snapshot!("data_directive_css", compiled.css);
    }

    // ── @test + @fixture ──────────────────────────────────────────────────────

    #[test]
    fn snapshot_test_and_fixture() {
        let source = r#"
@test "button is visible" {
    @fixture {
        <button id="btn">Click</button>
    }
    #btn {
        @assert visible;
    }
}
"#;
        let compiled = compile_source(source);
        insta::assert_snapshot!("test_and_fixture_js", compiled.js);
        insta::assert_snapshot!("test_and_fixture_css", compiled.css);
    }

    // ── mixed: file-level + selector-scoped in same file ─────────────────────

    #[test]
    fn snapshot_mixed_file_level_and_selector_scoped() {
        let source = r#"
@template &chip($text) {
    <span class="chip">`$text`</span>
}

button.primary {
    @on &.click {
        opacity: 1 -> 0.8;
    }
}
"#;
        let compiled = compile_source(source);
        assert!(
            compiled.pipeline_errors.is_empty(),
            "Expected no errors, got: {:?}",
            compiled.pipeline_errors
        );
        insta::assert_snapshot!("mixed_file_selector_js", compiled.js);
        insta::assert_snapshot!("mixed_file_selector_css", compiled.css);
    }

    // =========================================================================
    // stdlib/3d (PLAN-050) — three.js declarative 3D surface emit tests.
    //
    // The vendored three.js engine needs a real WebGL browser, so the live scene
    // graph is only assertable on the `--cdp` paint rung (stdlib/3d/tests/*.test.st).
    // These Rust tests gate the layer that DOES verify offline: that the macros
    // resolve and the primitives emit the correct three.js calls + demand-inject
    // the engine bundle. Mirrors the stdlib/md emit tests above.
    // =========================================================================

    /// Compile an inline .st snippet through the real from-file path so the
    /// stdlib/3d module's macros + primitives register exactly as in production.
    fn compile_3d(body: &str) -> CompiledSpacetime {
        let dir = std::env::temp_dir().join(format!(
            "st_3d_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(&entry, body).unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .fresh_registry()
            .compile();
        let _ = std::fs::remove_dir_all(&dir);
        compiled
    }

    #[test]
    fn three_stage_emits_engine_and_renderer() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.hero { @stage(camZ: 5) {} }\n",
        );
        assert!(
            compiled.js.contains("THREE.REVISION") || compiled.js.contains("WebGLRenderer"),
            "@stage should demand-inject the vendored three.js engine bundle"
        );
        assert!(
            compiled.js.contains("_stThree"),
            "@stage should publish the shared three handle on the canvas (_stThree)"
        );
        assert!(
            compiled.js.contains("ACESFilmicToneMapping"),
            "@stage(tone: aces default) should set ACES tone mapping"
        );
    }

    /// Compile an inline .st snippet through the real from-file path so the
    /// stdlib data module's macros + primitives register exactly as in
    /// production (same shape as compile_3d).
    fn compile_data_module(body: &str) -> CompiledSpacetime {
        let dir = std::env::temp_dir().join(format!(
            "st_data_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("index.st");
        std::fs::write(&entry, body).unwrap();
        let compiled = Compiler::from_file(&entry, Path::new("."))
            .expect("compiler from_file")
            .fresh_registry()
            .compile();
        let _ = std::fs::remove_dir_all(&dir);
        compiled
    }

    #[test]
    fn data_stream_expands_to_event_source_primitive() {
        let compiled = compile_data_module(
            "@import \"stdlib/data\";\n@data stream $push from \"/__spacetime/host/push-events\"\ndiv { $push_state }",
        );
        assert!(
            compiled.js.contains("new SpacetimeEventSource("),
            "@data stream should emit the event-source primitive wiring, got:\n{}",
            &compiled.js[..compiled.js.len().min(2000)]
        );
        assert!(
            compiled.js.contains("/__spacetime/host/push-events"),
            "the stream URL should be embedded in the emitted wiring"
        );
        // All four yielded signals are published through the dual channel.
        for signal in ["_state", "_type", "_error"] {
            assert!(
                compiled.js.contains(signal),
                "primitive must publish the {signal} yield"
            );
        }
    }

    #[test]
    fn data_stream_demand_injects_vendor_only_when_used() {
        let with_stream = compile_data_module(
            "@import \"stdlib/data\";\n@data stream $push from \"/x\"\ndiv { $push }",
        );
        assert!(
            with_stream.js.contains("SpacetimeEventSource = "),
            "a page using @data stream should demand-inject the vendored adapter"
        );
        let without_stream =
            compile_data_module("@import \"stdlib/data\";\ndiv { \"no stream here\" }");
        assert!(
            !without_stream.js.contains("SpacetimeEventSource = "),
            "a page NOT using @data stream must ship zero EventSource bytes"
        );
    }

    #[test]
    fn three_object_glass_emits_physical_material() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @object(shape: \"icosahedron\", material: \"glass\", transmission: 1) } }\n",
        );
        assert!(
            compiled.js.contains("MeshPhysicalMaterial"),
            "material: glass should emit a MeshPhysicalMaterial"
        );
        assert!(
            compiled.js.contains("IcosahedronGeometry"),
            "shape: icosahedron should emit an IcosahedronGeometry"
        );
    }

    #[test]
    fn three_object_standard_emits_standard_material() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @object(shape: \"box\", material: \"standard\") } }\n",
        );
        assert!(
            compiled.js.contains("MeshStandardMaterial"),
            "material: standard should emit a MeshStandardMaterial"
        );
        assert!(
            compiled.js.contains("BoxGeometry"),
            "shape: box should emit a BoxGeometry"
        );
    }

    #[test]
    fn three_light_emits_directional_light() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @light(type: \"directional\", intensity: 2) } }\n",
        );
        assert!(
            compiled.js.contains("DirectionalLight"),
            "@light(type: directional) should emit a THREE.DirectionalLight"
        );
    }

    /// BUG-159/FEAT-153: `@gltf(src: "literal.glb")` must still emit a plain
    /// quoted string URL (no behavior change for the common, non-reactive case).
    #[test]
    fn three_gltf_literal_src_emits_quoted_url() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @gltf(src: \"/models/a.glb\", size: 2) } }\n",
        );
        assert!(
            compiled.js.contains("GLTFLoader"),
            "@gltf should demand-inject GLTFLoader"
        );
        assert!(
            compiled.js.contains("\"/models/a.glb\""),
            "a literal src must still emit a plain quoted string (no regression): {}",
            compiled.js
        );
    }

    /// BUG-159: before the fix, `@gltf(src: $sig)` silently failed to bind at
    /// all (the macro's `:string` capture type rejected a `$`-prefixed token,
    /// dropping the whole directive with zero diagnostic). Changing the macro
    /// capture to `:expr` (gltf.st) lets it bind; this pins that the primitive
    /// actually RECEIVES the raw signal-reference text (not a mis-parsed
    /// literal), which FEAT-153's `resolveReactive` (three-stage.st) then
    /// resolves + watches at runtime.
    #[test]
    fn three_gltf_signal_src_binds_and_carries_raw_reference() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\n@data inline $modelPath : \"/models/a.glb\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @gltf(src: $modelPath, size: 2) } }\n",
        );
        assert!(
            compiled.js.contains("GLTFLoader"),
            "@gltf(src: $signal) must still bind (BUG-159 regression) and inject GLTFLoader: {}",
            compiled.js
        );
        assert!(
            compiled.js.contains("resolveReactive"),
            "a signal-valued src must route through three-stage.st's shared resolveReactive (FEAT-153): {}",
            compiled.js
        );
    }

    #[test]
    fn three_particles_emits_points_cloud() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 6) { @particles(count: 5000) } }\n",
        );
        assert!(
            compiled.js.contains("THREE.Points"),
            "@particles should emit a THREE.Points cloud"
        );
        assert!(
            compiled.js.contains("AdditiveBlending"),
            "@particles should use additive blending for the glow build-up"
        );
        assert!(
            compiled.js.contains("_stParticleMorph"),
            "@particles should publish the _stParticleMorph hook for @scroll-3d"
        );
    }

    #[test]
    fn three_post_emits_effect_composer() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @object(shape: \"box\") @post(bloomStrength: 0.6) } }\n",
        );
        assert!(
            compiled.js.contains("EffectComposer"),
            "@post should emit an EffectComposer pipeline"
        );
        assert!(
            compiled.js.contains("UnrealBloomPass"),
            "@post(bloomStrength) should add an UnrealBloomPass"
        );
    }

    #[test]
    fn three_orbit_emits_orbit_controls() {
        let compiled = compile_3d(
            "@import \"stdlib/3d\";\nbody { margin: 0; }\ncanvas.h { @stage(camZ: 5) { @object(shape: \"box\") @orbit(damping: 0.1) } }\n",
        );
        assert!(
            compiled.js.contains("OrbitControls"),
            "@orbit should attach OrbitControls"
        );
    }

    #[test]
    fn three_unused_module_ships_no_engine() {
        // Lean law: importing stdlib/3d but rendering no 3D ships none of three's bytes.
        let compiled =
            compile_3d("@import \"stdlib/3d\";\nbody { margin: 0; }\nh1.t { color: red; }\n");
        assert!(
            !compiled.js.contains("WebGLRenderer") && !compiled.js.contains("THREE.REVISION"),
            "a page that imports stdlib/3d but uses no 3D directive must NOT ship the three.js engine"
        );
    }
}

#[cfg(test)]
mod w4_dump {
    use super::*;
    #[test]
    fn stdlib_has_zero_capture_violations() {
        // I4 / gh-18 acceptance: the capture-consumption lint (E0964) runs over
        // ALL of stdlib with zero outstanding violations — every %form capture
        // is consumed in %binds/%emit/%when/%derives/%registers/%includes, or
        // explicitly declared (a %diagnostic/%pipeline-consumed/%drops marker,
        // a `_`-prefixed intentionally-unused name). A regression here means a
        // half-implemented macro shipped.
        let (_registry, errors) = load_stdlib_registry();
        let violations: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("never consumes"))
            .map(|e| {
                format!(
                    "{} @ {}",
                    e.message.split(" — ").next().unwrap(),
                    e.file_path.display()
                )
            })
            .collect();
        assert!(
            violations.is_empty(),
            "stdlib must be free of unconsumed-capture violations, got {}:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }
}
