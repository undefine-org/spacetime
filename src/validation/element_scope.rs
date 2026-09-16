//! GH-12 / PLAN-137 W6 (I6): `%scope element(<tag>)` constraint enforcement.
//!
//! A macro that declares `%scope element(<tag>)` (e.g. `@stage` on
//! `stdlib/3d/macros/stage.st` with `element(canvas)`) is only valid when the
//! selector it binds to implies the required element. Before this check, such a
//! macro compiled clean against ANY selector and failed only at runtime (three.js
//! `canvas.getContext` on a div). Nothing in the compiler's output could be
//! searched for.
//!
//! This is kinds-as-DATA, mirroring `%scope within(<construct>)`: the required tag
//! is declared on the macro and read back through
//! [`MetaRegistry::macro_scope_elements`]; no Rust match-arm per tag. The bound
//! selector is the MATCH's composed selector (`FormMatch.selector`), not the
//! enclosing scope's, because that is the element the directive actually resolves
//! to after nested-selector composition.
//!
//! Three-tier outcome (the unknowable case is DECIDED — see evidence.txt):
//! - implied tag == required         → OK (`canvas.hero`)
//! - implied tag != required         → ERROR `E0963` (`div.hero`)
//! - implied tag unknowable          → WARNING `W0963` (`.hero` — the element may
//!   be a runtime-inserted canvas, so it is not refused; but it is never SILENT).

use crate::diagnostics::{Diagnostic, DiagnosticCode, SourceSpan};
use crate::metasystem::MetaRegistry;
use crate::parser::StFile;

/// Resolve the SUBJECT element's type selector from a CSS selector string.
///
/// The subject compound is the last one (after the last combinator). Its leading
/// identifier — before the first `.`/`#`/`[`/`:` — is the element tag. Returns
/// `None` when the subject carries no tag (class/id/attribute/universal-only),
/// i.e. the element is unknowable from the selector alone.
pub fn subject_type_selector(selector: &str) -> Option<String> {
    let subject = selector
        .split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~')
        .filter(|part| !part.is_empty())
        .last()?;
    let mut tag = String::new();
    for ch in subject.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            tag.push(ch);
        } else {
            break;
        }
    }
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

/// Collect `E0963` (implied element != required tag) and `W0963` (implied element
/// unknowable) diagnostics for every match whose macro declares an element
/// restriction. ERROR-severity diagnostics make the compile FAIL; WARNING-severity
/// ones are surfaced but do not. Callers filter by severity for their channel.
pub fn scope_element_diagnostics(
    ast: &StFile,
    registry: &MetaRegistry,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for fm in &ast.matches {
        let required = registry.macro_scope_elements(&fm.macro_name);
        if required.is_empty() {
            continue;
        }
        let required_tags = required.join(", ");
        let span = SourceSpan::new(fm.span.start, fm.span.end);
        let sel = fm.selector.clone().unwrap_or_default();
        let Some(implied) = subject_type_selector(&sel) else {
            // Unknowable: warn-not-error (documented in evidence.txt). The element
            // may be the required tag created at runtime (a `<canvas>` JS-inserted
            // into `.hero`), so this is not refused — but it must NEVER pass
            // silently, because a wrong element is exactly how the runtime failed
            // with no compile signal (the GH-23-class silence).
            let hint = format!(
                "Selector '{}' carries no element tag, so the element cannot be \
                 verified at compile time. If the element is created at runtime this \
                 is fine; otherwise add an element (e.g. 'canvas{}') so the mismatch \
                 is caught here instead of at runtime.",
                sel,
                suffix_hint(&sel)
            );
            out.push(
                Diagnostic::warning(
                    DiagnosticCode::W0963,
                    format!(
                        "@{} requires a <{}> element, but its selector '{}' carries no \
                         element tag — cannot verify it will be one.",
                        fm.macro_name, required_tags, sel
                    ),
                )
                .with_span(span)
                .with_hint(hint),
            );
            continue;
        };
        if required.iter().any(|r| r.eq_ignore_ascii_case(&implied)) {
            // Implied tag matches a required tag — OK.
            continue;
        }
        // Clear mismatch: refuse at compile time.
        let hint = format!(
            "The selector '{}' resolves to a <{}> element, but @{} only renders into a \
             <{}>. Change the selector's element to '{}' (e.g. '{}') or bind @{} to the \
             element the primitive needs.",
            sel,
            implied,
            fm.macro_name,
            required_tags,
            required[0],
            replaced_subject(&sel, &required[0]),
            fm.macro_name,
        );
        out.push(
            Diagnostic::error(
                DiagnosticCode::E0963,
                format!(
                    "@{} binds to a <{}> element, but requires a <{}> element — it would \
                     fail only at runtime.",
                    fm.macro_name, implied, required_tags
                ),
            )
            .with_span(span)
            .with_hint(hint),
        );
    }
    out
}

/// A one-token hint for a bare compound: append the element to the selector's
/// subject (`hero` → `canvas.hero`). Empty when the selector is empty.
fn suffix_hint(sel: &str) -> String {
    let subject = sel
        .split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~')
        .filter(|part| !part.is_empty())
        .last();
    match subject {
        Some(s) if !s.is_empty() => format!(".{}", s.trim_start_matches(['.', '#', '[', ':', '*'])),
        _ => String::new(),
    }
}

/// Replace the subject compound's leading tag (or prepend one) so the hint shows
/// the corrected spelling. `div.hero` + `canvas` → `canvas.hero`; `.hero` +
/// `canvas` → `canvas.hero`.
fn replaced_subject(sel: &str, new_tag: &str) -> String {
    let subject = sel
        .split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~')
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(sel);
    // Strip any leading universal `*` or tag-like run, then the first `.`/`#`/`[`/`:`.
    let rest = subject
        .trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        .trim_start_matches('*');
    if rest.is_empty() {
        format!("{}{}", new_tag, suffix_hint(sel))
    } else {
        // `div.hero` -> `canvas.hero`; `div:hover` -> `canvas:hover`
        let body = rest.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        format!("{}{}", new_tag, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_type_selector_reads_the_subject_tag() {
        assert_eq!(subject_type_selector("div.hero").as_deref(), Some("div"));
        assert_eq!(subject_type_selector("canvas.hero").as_deref(), Some("canvas"));
        assert_eq!(subject_type_selector("canvas").as_deref(), Some("canvas"));
        assert_eq!(subject_type_selector("button.btn > canvas.hero").as_deref(), Some("canvas"));
        assert_eq!(subject_type_selector("canvas.hero:hover").as_deref(), Some("canvas"));
        assert_eq!(subject_type_selector("my-widget").as_deref(), Some("my-widget"));
        // Unknowable — no tag in the subject.
        assert_eq!(subject_type_selector(".hero"), None);
        assert_eq!(subject_type_selector("#hero"), None);
        assert_eq!(subject_type_selector(""), None);
        assert_eq!(subject_type_selector("*"), None);
    }

    /// GH-12 / PLAN-137 W6 enforcement: a macro declaring `element(canvas)` is
    /// ERRORED on a `div.hero` selector, accepted on `canvas.hero`, and WARNED
    /// (never silent) on the unknowable `.hero`.
    #[test]
    fn scope_element_errors_on_wrong_tag_and_warns_on_unknowable() {
        use crate::diagnostics::Severity;
        use crate::metasystem::MetaRegistry;
        use crate::parser::meta_ast::{MacroDefAst, MacroScope};
        use crate::parser::StFile;
        use crate::syntax::FormMatch;

        let mut reg = MetaRegistry::new();
        let stage = MacroDefAst {
            name: "stage".to_string(),
            scopes: vec![MacroScope::Selector],
            scope_element: vec!["canvas".to_string()],
            ..Default::default()
        };
        reg.register_macro(stage).unwrap();

        let fm = |sel: &str| {
            let mut m = FormMatch::new("stage");
            m.selector = Some(sel.to_string());
            m
        };
        let ast = StFile {
            matches: vec![fm("div.hero"), fm("canvas.hero"), fm(".hero")],
            ..Default::default()
        };

        let diags = scope_element_diagnostics(&ast, &reg);
        // div.hero -> ERROR E0963; canvas.hero -> accepted; .hero -> WARNING W0963.
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(errors.len(), 1, "expected exactly one E0963 for div.hero");
        assert_eq!(
            errors[0].code,
            DiagnosticCode::E0963,
            "wrong-element case must be an ERROR"
        );
        assert!(
            errors[0].message.contains("canvas"),
            "E0963 must name the required element, got: {}",
            errors[0].message
        );
        assert_eq!(warnings.len(), 1, "expected exactly one W0963 for .hero");
        assert_eq!(
            warnings[0].code,
            DiagnosticCode::W0963,
            "unknowable-element case must be a WARNING, not silent"
        );
        // canvas.hero produced neither — accepted.
        assert_eq!(diags.len(), 2);
    }
}
