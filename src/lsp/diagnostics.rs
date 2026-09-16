//! Diagnostics provider for Spacetime LSP.
//!
//! ONE diagnostic engine, two front-ends (gh-32): the LSP publishes the
//! COMPILER's diagnostics — the same `pipeline_errors` that `check` reports —
//! rather than a hand-maintained approximation that drifted from them. The old
//! path re-implemented directive validation against the LSP's own registry and
//! an empty `MetaRegistry`, so it could never agree with `spacetime check` on
//! the W2 cases (E0408 unknown signal; the false E0923 ambiguity). Running
//! `compile()` reuses the compiler's cached stdlib + project `_prelude.st`
//! overlay, so what the editor shows is exactly what the build resolves.

use std::path::Path;

use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Range,
};

use super::document::DocumentState;
use super::position::PositionMapper;
use crate::diagnostics::{DiagnosticCode, SourceSpan};
use crate::parser::{self, StFile};

// =============================================================================
// Main Validation Function
// =============================================================================

/// Validate a document and return LSP diagnostics, driven by the compiler's
/// own `compile()` pipeline (gh-32).
///
/// `site_dir` is the workspace/project root; when present the compiler loads
/// the project's `_prelude.st` overlay, so a project macro resolves exactly
/// when `spacetime check` resolves it.
pub fn validate_document(doc: &DocumentState, site_dir: Option<&Path>) -> Vec<LspDiagnostic> {
    let mapper = &doc.position_mapper;

    // If parsing failed, report parse errors and stop.
    let ast = match parser::parse(&doc.content) {
        Ok(ast) => ast,
        Err(errors) => return get_parse_error_diagnostics(mapper, &errors),
    };

    // Run the SAME engine `check` runs (gh-32): parse-level diagnostics + the
    // `validate()` arity pass + the full `CompileAnalysis` (E0408 ...) + the
    // compile pipeline errors. The old path ran only `compile()` and read its
    // `pipeline_errors`, silently missing E0408 and every other analysis
    // diagnostic `check` surfaces.
    let diagnostics = crate::analysis::diagnostics::collect_document_diagnostics(
        &ast,
        site_dir,
        None,
    );

    diagnostics
        .into_iter()
        // Only diagnostics that point into THIS document. Pipeline errors at
        // stdlib/registry spans (e.g. E0804 at a %binds site) are not the
        // author's file.
        .filter(|d| d.span.is_none_or(|s| s.end <= doc.content.len()))
        .map(|d| diagnostic_to_lsp(&d, mapper))
        .collect()
}

// =============================================================================
// Parse Error Handling
// =============================================================================

/// Get diagnostics for all parse errors.
fn get_parse_error_diagnostics(
    mapper: &PositionMapper,
    errors: &parser::ParseErrors,
) -> Vec<LspDiagnostic> {
    errors
        .iter()
        .map(|e| {
            let start = mapper.position_from_offset(e.offset);
            let end = mapper.position_from_offset(e.offset + e.len.max(1));
            LspDiagnostic {
                range: Range { start, end },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String(
                    DiagnosticCode::E003.as_str().to_string(),
                )),
                code_description: None,
                source: Some("spacetime".to_string()),
                message: e.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}

// =============================================================================
// Compiler-Diagnostic Conversion (gh-32)
// =============================================================================

/// Convert a compiler `Diagnostic` (the shared `check`/LSP currency) into an
/// LSP diagnostic.
fn diagnostic_to_lsp(d: &crate::diagnostics::Diagnostic, mapper: &PositionMapper) -> LspDiagnostic {
    let range = d.span.map_or_else(
        || Range::default(),
        |span| mapper.span_to_range(&SourceSpan::new(span.start, span.end)),
    );

    let message = if let Some(ref hint) = d.hint {
        format!("{}. Hint: {}", d.message, hint)
    } else {
        d.message.clone()
    };

    let severity = match d.severity {
        crate::diagnostics::Severity::Error => DiagnosticSeverity::ERROR,
        crate::diagnostics::Severity::Warning => DiagnosticSeverity::WARNING,
    };

    LspDiagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(d.code.as_str().to_string())),
        code_description: None,
        source: Some("spacetime".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}
