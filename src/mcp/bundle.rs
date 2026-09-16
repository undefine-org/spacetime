//! Bundle compilation (PLAN-041: Unified Live MCP Environment).
//!
//! `compile_to_bundle` reduces Spacetime source to a region-mountable
//! [`Bundle`]: every `@template` becomes a [`TemplateBundle`] whose body is
//! serialized by REUSING `component_body_to_js_from_scope` — the SAME serializer
//! the compile pipeline uses for `register-template`'s `%body` arg
//! (src/syntax/form_match.rs, called from src/pipeline/expand.rs). CSS is
//! collected from `CompiledSpacetime.css`; a minimal [`Contract`] is derived
//! from the templates' param specs.
//!
//! v1 is TEMPLATES ONLY: file-scope HTML outside `@template` is ignored (a
//! warning diagnostic is emitted when present). See
//! @research/region-model-unified-mcp.org for the region model this feeds.

use std::path::Path;

use crate::compiler::Compiler;
use crate::emit::html_reactive::{
    component_html_to_exprs, emit_builder_root_scoped, emit_builder_root_scoped_stamped,
};
use crate::syntax::{SignalScope, TemplateParamKind, component_body_to_js_from_scope};

use super::state::{
    Bundle, BundleDiagnostic, CompileError, Contract, MatchArm, ParamSpec, TemplateBody,
    TemplateBundle, TemplateContract,
};

/// Compile Spacetime `source` into a region-mountable [`Bundle`].
///
/// Reuses the SAME pipeline entry as `mcp::tools::compile_source`:
/// `parser::parse` → (optional) `parser::resolve_imports` →
/// `Compiler::from_ast(&ast).compile()`. Template bodies are serialized via
/// `component_body_to_js_from_scope` (NOT re-implemented).
///
/// # Error model
/// - **Semantic compile errors** → `Ok(Bundle)` with `has_errors = true` and
///   `diagnostics` populated. The AST is valid, so templates are still extracted.
/// - **Parse / import-resolution failures** → `Err(CompileError)`: no usable AST
///   exists, so no meaningful bundle can be produced. Never panics.
/// - **No templates** → `Ok(Bundle)` with an empty `templates` vec (chrome css
///   and diagnostics are still collected).
/// The compiled projections of one Spacetime function, produced by a SINGLE
/// pipeline run (FUP-066). Carries both the region-mountable [`Bundle`]
/// (templates + css + contract + diagnostics) AND the page `html`/`js`, so the
/// contract extraction and the page compile can never disagree on error state.
pub struct CompiledFunction {
    /// Region-mountable bundle projection (templates, css, contract, diagnostics).
    pub bundle: Bundle,
    /// Page HTML (`CompiledSpacetime.html`).
    pub html: String,
    /// Page JS (`CompiledSpacetime.js`).
    pub js: String,
}

/// The pseudo-filename `compile_pipeline` uses as the import-resolution anchor
/// when the caller gave no real `entry_path` (inline/agent source with no
/// on-disk location). `resolve_imports` tags the MAIN file's scopes with this
/// path, so it can surface as a template's `source_file` — but NO such file
/// exists on disk. [`real_source_file`] normalizes it away so a write rail can
/// never be handed a path it cannot open.
const SYNTHETIC_ENTRY_NAME: &str = "<bundle>.st";

/// Normalize a scope's `source_file` into a REAL, openable path or `None`.
///
/// `None` means "the entry source itself" — the caller already knows which file
/// it compiled, so it can resolve the address; a synthetic `<bundle>.st` anchor
/// (see [`SYNTHETIC_ENTRY_NAME`]) is indistinguishable from a real path to a
/// downstream consumer and would send a span patch to a nonexistent file, so it
/// collapses to `None` here — at the ONE place bundles are built.
fn real_source_file(source_file: Option<&str>, synthetic_anchor: Option<&Path>) -> Option<String> {
    let path = source_file?;
    // Compare against the EXACT anchor this compilation would have synthesized,
    // never the basename: a user file legitimately named `<bundle>.st` in some
    // other directory is a real, writable file, and collapsing it to `None` would
    // mis-address its rows onto the entry (W1R review finding).
    if synthetic_anchor.is_some_and(|anchor| Path::new(path) == anchor) {
        return None;
    }
    Some(path.to_string())
}

/// Run the shared compile pipeline ONCE: `parser::parse` → (optional)
/// `parser::resolve_imports` → `Compiler::from_ast(&ast).compile()`. Returns the
/// resolved AST (templates are collected from it) alongside the compile result.
///
/// This is the single entry both [`compile_to_bundle`] and [`compile_function`]
/// share, so the bundle/contract projection and the page html/js projection are
/// always derived from the same `CompiledSpacetime` (FUP-066).
fn compile_pipeline(
    source: &str,
    ws_root: &Path,
    entry_path: Option<&Path>,
) -> Result<(crate::parser::StFile, crate::compiler::CompiledSpacetime), CompileError> {
    // 1. Parse — same entry as tools::compile_source.
    //
    // PLAN-148: EDN is a second concrete syntax over the same `%form` registry,
    // so it enters HERE, at the parse step, and produces the same `StFile` the
    // `.st` parser produces. It is NOT rendered to `.st` text first: routing it
    // through the printer would put the printer's coverage on the critical path
    // (a form the reader accepts but the printer cannot render would become
    // inexpressible), and `pipeline::compile` consumes `Vec<FormMatch>` anyway —
    // which is precisely why the two syntaxes cannot diverge.
    //
    // `.st` detection is structural and `.st` never reaches the EDN arm, so this
    // adds no branch to the ordinary path.
    let ast = match crate::edn::detect(source) {
        crate::edn::SourceLang::Edn => {
            crate::edn::to_st_file(source, &crate::syntax::STDLIB_REGISTRY)
                .map_err(CompileError::parse)?
        }
        crate::edn::SourceLang::St => crate::parser::parse(source)
            .map_err(|e| CompileError::parse(e.render_all_plain(source, "<bundle>")))?,
    };

    // 2. Resolve imports relative to the entry's REAL directory when known
    // (PLAN-066: a subdirectory entry like `landing/index.st` has relative
    // imports like `../modules/_theme.st` that resolve against ITS OWN parent,
    // not the env/workspace root) — else fall back to the ws_root pseudo-path
    // (inline/agent source with no on-disk location).
    let ast = if !ast.imports.is_empty() {
        let main_path = entry_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| ws_root.join(SYNTHETIC_ENTRY_NAME));
        crate::parser::resolve_imports(&ast, &main_path, ws_root)
            .map_err(|e| CompileError::import(format!("Import resolution failed: {e}")))?
    } else {
        ast
    };

    // 3. Compile — reuse the pipeline for html + css + js + diagnostics.
    let compiled = Compiler::from_ast(&ast).compile();
    Ok((ast, compiled))
}

/// Derive the region [`Bundle`] (templates + css + contract + diagnostics) from
/// an already-compiled AST. Pure projection — no compilation here (the pipeline
/// ran once in [`compile_pipeline`]).
fn bundle_from_compiled(
    ast: &crate::parser::StFile,
    compiled: &crate::compiler::CompiledSpacetime,
    synthetic_anchor: Option<&Path>,
) -> Bundle {
    // Collect templates from the (valid) AST's World-A scopes + form matches.
    let templates = collect_template_bundles(ast, synthetic_anchor);

    // Diagnostics: pipeline errors are ERRORS (set has_errors); a v1
    // file-scope-html warning is informational only (does not set has_errors).
    let mut diagnostics: Vec<BundleDiagnostic> = Vec::new();
    let mut has_errors = false;
    for e in &compiled.pipeline_errors {
        diagnostics.push(BundleDiagnostic {
            severity: "error".to_string(),
            code: e.code.clone(),
            message: e.message.clone(),
        });
        has_errors = true;
    }
    // v1: TEMPLATES ONLY — file-scope HTML outside @template is ignored.
    if !compiled.html.trim().is_empty() {
        diagnostics.push(BundleDiagnostic {
            severity: "warning".to_string(),
            code: "BUNDLE-V1".to_string(),
            message: "file-scope HTML outside @template is ignored in v1 \
                      (templates only)."
                .to_string(),
        });
    }

    // Minimal contract derived from the collected param specs.
    let contract = Contract {
        templates: templates
            .iter()
            .map(|t| TemplateContract {
                name: t.name.clone(),
                params: t.params.clone(),
            })
            .collect(),
    };

    Bundle {
        templates,
        css: compiled.css.clone(),
        contract,
        diagnostics,
        has_errors,
    }
}

/// Compile Spacetime `source` into a region-mountable [`Bundle`].
///
/// Reuses the SAME pipeline entry as `mcp::tools::compile_source`:
/// `parser::parse` → (optional) `parser::resolve_imports` →
/// `Compiler::from_ast(&ast).compile()`. Template bodies are serialized via
/// `component_body_to_js_from_scope` (NOT re-implemented).
///
/// # Error model
/// - **Semantic compile errors** → `Ok(Bundle)` with `has_errors = true` and
///   `diagnostics` populated. The AST is valid, so templates are still extracted.
/// - **Parse / import-resolution failures** → `Err(CompileError)`: no usable AST
///   exists, so no meaningful bundle can be produced. Never panics.
/// - **No templates** → `Ok(Bundle)` with an empty `templates` vec (chrome css
///   and diagnostics are still collected).
pub fn compile_to_bundle(source: &str, ws_root: &Path) -> Result<Bundle, CompileError> {
    let (ast, compiled) = compile_pipeline(source, ws_root, None)?;
    let anchor = ws_root.join(SYNTHETIC_ENTRY_NAME);
    Ok(bundle_from_compiled(&ast, &compiled, Some(&anchor)))
}

/// [`compile_to_bundle`], but anchored at the entry's REAL on-disk path.
///
/// Import resolution is relative to the importing FILE, so an entry in a
/// subdirectory (`landing/index.st`) with `@import "../modules/_theme.st"`
/// only resolves when the pipeline knows where that entry lives — the same
/// PLAN-066 reason [`compile_function`] takes an `entry_path`. `compile_to_bundle`
/// passes `None` (correct for inline/agent source with no on-disk location), so a
/// caller that HAS a real path must use this instead or a valid nested entry
/// fails to resolve, or worse, silently resolves a same-named file from the
/// wrong directory.
///
/// It also gives every template a REAL `source_file`, which is what makes a
/// projected row a usable write address (see [`real_source_file`]).
pub fn compile_to_bundle_at(
    source: &str,
    ws_root: &Path,
    entry_path: &Path,
) -> Result<Bundle, CompileError> {
    let (ast, compiled) = compile_pipeline(source, ws_root, Some(entry_path))?;
    // A real entry path anchors import resolution, so no synthetic name is ever
    // minted and every `source_file` is already a real, openable path.
    Ok(bundle_from_compiled(&ast, &compiled, None))
}

/// Compile Spacetime `source` into BOTH the region [`Bundle`] and the page
/// `html`/`js`, in a SINGLE pipeline run (FUP-066).
///
/// `put_function_from_args` uses this so it no longer compiles the source twice
/// (once for contract params, once for the page). The page `ok` state and the
/// bundle's `has_errors` then derive from the SAME compile — they cannot
/// diverge.
///
/// `entry_path`, when given (an env-origin, file-backed tab/entry — NOT inline
/// agent source), enables two PLAN-066 completeness warnings that do not block
/// `compile_ok` but tell the truth about an incomplete MCP-composed preview:
///   - the entry uses `@locale` (non-empty `build_scripts`) — the real dev
///     server (`server.rs::try_serve_locale`) executes these via a v8 runtime
///     to materialize the routed HTML; the MCP compose path does not.
///   - the entry has NO file-scope HTML of its own but a sibling `.html` file
///     exists on disk (the `index.html`-shell composition pattern: the `.st`
///     file populates selectors like `.site-header`/`main`/`footer` that are
///     only DEFINED in that sibling shell) — MCP compose never merges it, so
///     the composed guest renders only orphaned `@each`/selector-driven
///     fragments, not the actual page.
/// Both are WARNING-severity `BundleDiagnostic`s appended to the returned
/// bundle (never setting `has_errors` — the source is valid Spacetime; the gap
/// is in what the MCP preview path can render, not the source).
///
/// Same error model as [`compile_to_bundle`]: parse/import failures → `Err`;
/// semantic compile errors → `Ok` with `bundle.has_errors = true`.
pub fn compile_function(
    source: &str,
    ws_root: &Path,
    entry_path: Option<&Path>,
) -> Result<CompiledFunction, CompileError> {
    let (ast, compiled) = compile_pipeline(source, ws_root, entry_path)?;
    let synthetic_anchor = entry_path
        .is_none()
        .then(|| ws_root.join(SYNTHETIC_ENTRY_NAME));
    let mut bundle = bundle_from_compiled(&ast, &compiled, synthetic_anchor.as_deref());
    if !bundle.has_errors {
        if !compiled.build_scripts.is_empty() {
            bundle.diagnostics.push(BundleDiagnostic {
                severity: "warning".to_string(),
                code: "MCP-LOCALE".to_string(),
                message: "this entry uses @locale build scripts; the MCP compose \
                          path does not execute them (unlike `spacetime serve`), so \
                          the composed preview will be missing the locale-routed \
                          HTML. See PLAN-066."
                    .to_string(),
            });
        }
        if compiled.html.trim().is_empty()
            && let Some(entry) = entry_path
        {
            let shell = entry.with_extension("html");
            if shell.is_file() {
                bundle.diagnostics.push(BundleDiagnostic {
                    severity: "warning".to_string(),
                    code: "MCP-SHELL".to_string(),
                    message: format!(
                        "this entry has no file-scope HTML of its own; it composes \
                             into the sibling shell {} (e.g. .site-header/main/footer \
                             selectors), which the MCP compose path does not merge. The \
                             composed preview will show only orphaned fragments, not the \
                             real page. See PLAN-066.",
                        shell.display()
                    ),
                });
            }
        }
    }
    Ok(CompiledFunction {
        bundle,
        html: compiled.html,
        js: compiled.js,
    })
}

/// Walk the parsed [`StFile`] and build one [`TemplateBundle`] per
/// `@template` / bare `&name() {}` definition.
///
/// For each template form match we: resolve its World-A scope
/// (`@template:<name>`), derive the element-param names the serializer needs,
/// and serialize the body via `component_body_to_js_from_scope` (the serializer
/// the compile pipeline uses for `register-template`'s `%body`). The reactive
/// `builder` is surfaced via the SAME `html_reactive` emit the serializer calls
/// internally — this is reuse of the shared emit layer, not a re-implementation.
pub(crate) fn collect_template_bundles(
    ast: &crate::parser::StFile,
    synthetic_anchor: Option<&Path>,
) -> Vec<TemplateBundle> {
    let mut bundles = Vec::new();
    for fm in &ast.matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }
        // Name — strip a defensive `&` prefix (matches pipeline/expand.rs).
        let Some(raw_name) = fm.get_ident("name") else {
            continue;
        };
        let name = raw_name.strip_prefix('&').unwrap_or(raw_name).to_string();

        // World-A template scope, keyed `@template:<name>`
        // (verified: src/syntax/form_match.rs::inject_provenance_into_matches).
        let scope_key = format!("@template:{name}");
        let Some(scope) = ast.scopes.iter().find(|s| s.selector == scope_key) else {
            continue;
        };

        // `param_list` capture → param specs + element-param names for the serializer
        // (the serializer's `element_params` arg, per pipeline/expand.rs ~L358).
        let params = fm.get_param_list("params").cloned().unwrap_or_default();
        let element_params: Vec<String> = params
            .iter()
            .filter(|p| matches!(p.kind, TemplateParamKind::Element))
            .map(|p| p.name.clone())
            .collect();

        // Authoritative body payload — REUSES component_body_to_js_from_scope.
        let serialized = component_body_to_js_from_scope(scope, &element_params);

        // Reactive builder — the same html_reactive emit the serializer calls
        // internally (component_payload_to_js), surfaced for inspection.
        //
        // PLAN-064 B3.2: the ENTRY template (`main`, the region-mount root) STAMPS
        // `data-st-node` on its elements under base `"0"` (the entry's structure id)
        // so a composed guest's DOM carries the SAME dotted ids the navigator shows
        // — canvas-click selection resolves to a node. Non-entry templates are built
        // unstamped: their elements appear under a `&child(…)` invocation whose base
        // varies per call site, which needs runtime invocation-base threading (a
        // documented follow-up); stamping them with a static base would mis-address
        // repeated invocations. So the single-template guest (the common case) is
        // fully selectable now; composed sub-template elements select at the
        // template-node granularity until the invocation-base rail lands.
        let builder = if scope.html.trim().is_empty() {
            String::new()
        } else {
            let exprs = component_html_to_exprs(&scope.html);
            if name == "main" {
                emit_builder_root_scoped_stamped(&exprs, SignalScope::Element, "0")
            } else {
                emit_builder_root_scoped(&exprs, SignalScope::Element)
            }
        };

        let param_specs: Vec<ParamSpec> = params.iter().map(ParamSpec::from).collect();

        bundles.push(TemplateBundle {
            name,
            source_file: real_source_file(scope.source_file.as_deref(), synthetic_anchor),
            body: TemplateBody {
                html: scope.html.clone(),
                builder,
                states: scope.states.clone(),
                exports: scope.exports.clone(),
                refs: scope.refs.clone(),
                matches: Vec::<MatchArm>::new(),
                serialized,
            },
            params: param_specs,
            animations: None,
        });
    }
    bundles
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// PLAN-066: a subdirectory entry (e.g. `landing/index.st`) with a relative
    /// import (`../modules/_theme.st`) resolves against the ENTRY's own parent
    /// directory, not the bare `ws_root`. Before this fix, `compile_function`
    /// always resolved imports against a `ws_root.join("<bundle>.st")` pseudo
    /// path regardless of the entry's real location, so any subdirectory entry
    /// with a `../`-relative import failed with a bogus "Import resolution
    /// failed" error (reproduced live against
    /// `projects/ora-ventures.com/landing/index.st`, which imports
    /// `../modules/_theme.st`).
    #[test]
    fn subdirectory_entry_resolves_relative_import_against_its_own_dir() {
        let root = std::env::temp_dir().join(format!(
            "spacetime-bundle-subdir-import-test-{}",
            std::process::id()
        ));
        let landing = root.join("landing");
        let modules = root.join("modules");
        std::fs::create_dir_all(&landing).expect("landing dir");
        std::fs::create_dir_all(&modules).expect("modules dir");
        std::fs::write(
            modules.join("_theme.st"),
            "@template &theme() { <div class=\"theme\">t</div> }",
        )
        .expect("write theme module");
        let entry_path = landing.join("index.st");
        let src = "@import \"../modules/_theme.st\"\n@template &main() { &theme(); }";
        // ws_root = the ENV root (`root`), entry_path = the real subdirectory file
        // — mirrors st_tab_open's (env_root, file) pairing exactly.
        let compiled = compile_function(src, &root, Some(&entry_path))
            .expect("relative import resolves against the entry's own directory");
        assert!(
            !compiled.bundle.has_errors,
            "subdirectory entry's relative import must resolve, not error: {:?}",
            compiled.bundle.diagnostics
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// PLAN-066: an entry using `@locale` (non-empty `compiled.build_scripts`)
    /// still compiles ok, but `compile_function` appends an MCP-LOCALE warning
    /// diagnostic — the MCP compose path does not execute build scripts (unlike
    /// `spacetime serve`), so the composed preview would silently miss the
    /// locale-routed HTML without this signal.
    #[test]
    fn locale_entry_warns_but_still_compiles() {
        let src = "\
@locale(
    default: en,
    available: [en, fr],
    src: \"locales/{locale}.json\"
)
@template &card($title) {
  <article class=\"card\"><h2>`$title`</h2></article>
}
";
        let compiled = compile_function(src, Path::new("."), None).expect("locale entry compiles");
        assert!(
            !compiled.bundle.has_errors,
            "locale directive is not a compile error"
        );
        let warning = compiled
            .bundle
            .diagnostics
            .iter()
            .find(|d| d.code == "MCP-LOCALE");
        assert!(
            warning.is_some(),
            "expected an MCP-LOCALE warning diagnostic: {:?}",
            compiled.bundle.diagnostics
        );
        assert_eq!(
            warning.unwrap().severity,
            "warning",
            "MCP-LOCALE must not block compile_ok"
        );
    }

    /// PLAN-066: an entry with NO file-scope HTML of its own (a pure @each/
    /// selector composition, like `projects/unkn/index.st`'s `.site-header`/
    /// `main`/`footer` targeting an existing `index.html` shell) warns
    /// MCP-SHELL when a sibling `<entry>.html` exists on disk — telling the
    /// truth that the MCP-composed preview will be incomplete, instead of
    /// silently reporting `html_len: 0` with no explanation.
    #[test]
    fn shell_composing_entry_warns_when_sibling_html_exists() {
        let dir = std::env::temp_dir().join(format!(
            "spacetime-bundle-shell-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let html_path = dir.join("index.html");
        std::fs::write(&html_path, "<html><body><main></main></body></html>").expect("write shell");
        let entry_path = dir.join("index.st");
        // No @template &main with root markup here — only a selector composing
        // INTO the sibling shell's `main` element (the unkn pattern). Root-level
        // html stays empty because there is no @template body producing page html.
        let src = "main { <p>hello</p> }";
        let compiled = compile_function(src, &dir, Some(&entry_path)).expect("compiles");
        assert!(
            !compiled.bundle.has_errors,
            "selector-only composition is not a compile error"
        );
        assert!(
            compiled.html.trim().is_empty(),
            "no @template main -> empty page html: {:?}",
            compiled.html
        );
        let warning = compiled
            .bundle
            .diagnostics
            .iter()
            .find(|d| d.code == "MCP-SHELL");
        assert!(
            warning.is_some(),
            "expected an MCP-SHELL warning diagnostic when a sibling .html exists: {:?}",
            compiled.bundle.diagnostics
        );
        assert_eq!(warning.unwrap().severity, "warning");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// PLAN-066 negative case: no sibling `.html` on disk (a stdlib/agent/tab
    /// with genuinely no page shell) — no MCP-SHELL warning fires even though
    /// html is empty, since there is nothing incomplete to warn about.
    #[test]
    fn no_sibling_html_yields_no_shell_warning() {
        let dir = std::env::temp_dir().join(format!(
            "spacetime-bundle-noshell-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let entry_path = dir.join("index.st");
        let src = "main { <p>hello</p> }";
        let compiled = compile_function(src, &dir, Some(&entry_path)).expect("compiles");
        assert!(compiled.html.trim().is_empty(), "empty page html");
        assert!(
            !compiled
                .bundle
                .diagnostics
                .iter()
                .any(|d| d.code == "MCP-SHELL"),
            "no sibling html on disk -> no MCP-SHELL warning: {:?}",
            compiled.bundle.diagnostics
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A single `@template &card($title)` compiles to a one-template bundle with
    /// the name, the `$title` param, non-empty html+builder, and page css.
    #[test]
    fn single_template_compiles_to_bundle() {
        let src = "\
@template &card($title) {
  <article class=\"card\"><h2>`$title`</h2></article>
}
.page { background: #ffffff; }
";
        let bundle = compile_to_bundle(src, Path::new(".")).expect("bundle");
        assert!(
            !bundle.has_errors,
            "clean source must not error: {:?}",
            bundle.diagnostics
        );
        assert_eq!(bundle.templates.len(), 1);
        let t = &bundle.templates[0];
        assert_eq!(t.name, "card");
        assert_eq!(t.params.len(), 1, "one param: {:?}", t.params);
        assert_eq!(t.params[0].name, "title");
        assert!(!t.body.html.is_empty(), "body html non-empty");
        assert!(!t.body.builder.is_empty(), "body builder non-empty");
        assert!(
            !t.body.serialized.is_empty(),
            "serialized payload non-empty"
        );
        assert!(
            t.body.serialized.contains("builder"),
            "serialized payload carries the builder key"
        );
        assert!(!bundle.css.is_empty(), "page css present: {:?}", bundle.css);
        // Contract mirrors the one template + its param.
        assert_eq!(bundle.contract.templates.len(), 1);
        assert_eq!(bundle.contract.templates[0].name, "card");
    }

    /// Two template definitions → two entries in the bundle.
    #[test]
    fn two_templates_collected() {
        let src = "\
@template &card($title) { <article>`$title`</article> }
@template &row($label) { <div>`$label`</div> }
.page { color: red; }
";
        let bundle = compile_to_bundle(src, Path::new(".")).expect("bundle");
        let names: Vec<_> = bundle.templates.iter().map(|t| t.name.clone()).collect();
        assert_eq!(bundle.templates.len(), 2, "both templates: {names:?}");
        assert!(names.contains(&"card".to_string()));
        assert!(names.contains(&"row".to_string()));
    }

    /// A semantic compile error (undeclared body `$broken` → E0900, the exact
    /// pattern from compiler.rs feat120_t4) populates diagnostics + has_errors
    /// WITHOUT panicking and WITHOUT returning Err (the AST is still valid).
    #[test]
    fn compile_error_populates_diagnostics_no_panic() {
        let src = "\
@template &card($t) {
  <div class=\"c\">`$t`</div>
  $broken
}
.s { &card(\"x\"); }
";
        let bundle = compile_to_bundle(src, Path::new("."))
            .expect("semantic errors return Ok(Bundle) with diagnostics");
        assert!(
            bundle.has_errors,
            "has_errors must be set: {:?}",
            bundle.diagnostics
        );
        assert!(
            !bundle.diagnostics.is_empty(),
            "diagnostics must be populated"
        );
    }

    /// A parse failure (no AST) returns Err(CompileError) — never panics.
    /// Uses an unclosed scope block, the same shape proven to error in
    /// src/parser/tests.rs (`parse(".x {").unwrap_err()`).
    #[test]
    fn parse_error_returns_err() {
        let src = ".broken {"; // unclosed brace — hard parse error
        let result = compile_to_bundle(src, Path::new("."));
        assert!(result.is_err(), "parse failure must be Err, got Ok");
    }

    /// No templates → empty templates vec, but chrome css + diagnostics still
    /// collected.
    #[test]
    fn no_templates_yields_empty_vec() {
        let src = ".page { color: red; }";
        let bundle = compile_to_bundle(src, Path::new(".")).expect("bundle");
        assert!(bundle.templates.is_empty(), "no templates");
        assert!(bundle.contract.templates.is_empty());
        assert!(
            !bundle.css.is_empty(),
            "chrome css still collected: {:?}",
            bundle.css
        );
        assert!(!bundle.has_errors);
    }
}
