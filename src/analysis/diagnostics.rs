//! ONE diagnostic engine, two front-ends (gh-32).
//!
//! `check` (the CLI) and the LSP both publish the compiler's diagnostics for a
//! document. This is the shared core they call: parse-level diagnostics, the
//! `validate()` function-arity pass, the full `CompileAnalysis` chain (E0408
//! unknown signal, orphans, binding sources, typed reads, …), the W6
//! `%scope element(<tag>)` warning half, and the `compile()` pipeline errors
//! (E0946, E0923, E0900, …).
//!
//! The LSP previously ran only `compile()` and read its `pipeline_errors`,
//! missing E0408 and every other analysis diagnostic that `check` surfaces —
//! two sources of truth that drifted silently. This module is the cure: one
//! function, both consumers.
//!
//! What it deliberately does NOT include: site-level channels that need the
//! surrounding project rather than the document alone — comment-roster
//! diagnostics (`_prelude.st`), HTML-pair validation, and the LiveView
//! contract sidecar. Those are checked only by `check` on a full site; a
//! single open document cannot judge them.

use std::collections::HashSet;
use std::path::Path;

use crate::analysis::CompileAnalysis;
use crate::diagnostics::{Diagnostic, DiagnosticCollector, DiagnosticCode};
use crate::parser::StFile;
use crate::type_system::TypeRegistry;

/// Run the full compiler diagnostic chain for one document, returning the same
/// diagnostics `check` reports for it (pipeline errors + analysis + validate +
/// parse-level). Site channels that need project context are excluded; see the
/// module docs.
///
/// `meta_registry` seeds dispatch/yield signal analysis with the compiler's
/// registry (cached stdlib + this file's own `meta_defs`); pass
/// `&cached_stdlib_registry().0` augmented with `ast.meta_defs`, or `None` to
/// let this function build it.
pub fn collect_document_diagnostics(
    ast: &StFile,
    site_dir: Option<&Path>,
    meta_registry: Option<&crate::metasystem::MetaRegistry>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    // Parse-level diagnostics on the AST itself, deduplicated (the flat
    // `matches` list and the scopes tree can both carry one construct).
    let mut emitted = HashSet::new();
    for diag in &ast.diagnostics {
        if emitted.insert(format!("{}|{}", diag.code.as_str(), diag.message)) {
            out.push(diag.clone());
        }
    }

    // The full compile-time analysis chain — this is where E0408 (unknown
    // signal), orphans, binding-source and typed-read diagnostics come from.
    let analysis = build_analysis(ast, site_dir, meta_registry);
    out.extend(analysis.diagnostics.iter().cloned());

    // W6 (I6): the `%scope element(<tag>)` WARNING half. The E0963 error half
    // fails the compile via pipeline_errors below; this emits only the warning
    // (element unknowable, e.g. `.hero { @stage }`). Same function, filtered by
    // severity per channel — no duplicated detection logic.
    let meta = meta_registry
        .map(|m| m.clone())
        .unwrap_or_else(|| crate::compiler::cached_stdlib_registry().0);
    for d in crate::validation::element_scope::scope_element_diagnostics(ast, &meta) {
        if d.severity == crate::diagnostics::Severity::Warning {
            out.push(d);
        }
    }

    // The compile pipeline's own errors (E0946, E0923, E0900, …).
    let options = crate::compiler::CompileOptions {
        site_dir: site_dir.map(|p| p.to_path_buf()),
        ..Default::default()
    };
    let compiled = crate::compiler::compile(ast, options);
    for w in &compiled.migration_warnings {
        let mut d = Diagnostic::warning(DiagnosticCode::W0715, w.message.clone());
        if let Some(s) = w.span {
            d = d.with_span(crate::diagnostics::SourceSpan::new(s.start, s.end));
        }
        if let Some(h) = &w.hint {
            d = d.with_hint(h.clone());
        }
        out.push(d);
    }
    for pe in &compiled.pipeline_errors {
        let code = DiagnosticCode::from_code_str(&pe.code).unwrap_or(DiagnosticCode::E0806);
        let span = pe.span.map(|s| crate::diagnostics::SourceSpan::new(s.start, s.end));
        let mut d = Diagnostic::error(code, pe.message.clone());
        if let Some(sp) = span {
            d = d.with_span(sp);
        }
        if let Some(h) = &pe.hint {
            d = d.with_hint(h.clone());
        }
        out.push(d);
    }

    out
}

/// Build the `CompileAnalysis` for an AST, mirroring the exact chain `check`
/// runs (main.rs `check_file`). `meta_registry` seeds dispatch + yield-signal
/// analysis with the compiler's registry; when `None`, the cached stdlib
/// registry is used.
fn build_analysis(
    ast: &StFile,
    site_dir: Option<&Path>,
    meta_registry: Option<&crate::metasystem::MetaRegistry>,
) -> CompileAnalysis {
    let mut analysis = CompileAnalysis::new();
    let matches = &ast.matches;
    let registry = TypeRegistry::from_form_matches(matches);

    analysis.analyze_types(matches, &registry);
    analysis.analyze_data_sources(matches, &registry);
    analysis.analyze_locals(matches);
    analysis.analyze_computed(matches);
    analysis.analyze_functions(matches);
    analysis.analyze_scopes(&ast.scopes);
    analysis.analyze_read_usages(&ast.scopes, matches);
    analysis.analyze_timelines(&ast.scopes);

    // Dispatch + yield analysis need the meta registry. Mirror `check_file`:
    // seed from the cached stdlib registry, then register this file's own
    // meta_defs so non-core-module macros (`stdlib/__mcp__`, `stdlib/3d`) are
    // visible (BUG-155-adjacent).
    let owned_meta = if let Some(m) = meta_registry {
        None
    } else {
        let (mut meta, _) = crate::compiler::cached_stdlib_registry();
        for def in ast.meta_defs.clone() {
            let _ = meta.register(def);
        }
        Some(meta)
    };
    let meta = meta_registry.unwrap_or_else(|| owned_meta.as_ref().expect("built above"));
    analysis.analyze_dispatch(matches, meta);

    // Visual lints (W201-W205): the LSP has no paired HTML here, so pass None.
    analysis.analyze_visual(&ast.scopes, matches, None);

    analysis.register_macro_yield_signals(matches, meta);
    analysis.analyze_signals_from_ast(ast);
    analysis.detect_orphans();
    analysis.validate_computed_dependencies();
    analysis.validate_binding_sources();
    analysis.validate_typed_reads(&registry, &ast.scopes);

    if let Some(dir) = site_dir {
        analysis.validate_data_source_files(dir);
    }

    analysis
}
