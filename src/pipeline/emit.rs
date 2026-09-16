//! Emit Layer (Layer 5)
//!
//! Emits typed IR fragments to final output strings (JS and CSS).
//!
//! This layer:
//! 1. Takes typed JsFragment/CssFragment instances (scope-aware)
//! 2. Wraps JS in IIFE/Block scopes for variable isolation
//! 3. Adds runtime boilerplate if needed
//! 4. Returns final PipelineOutput

use super::types::{CssFragment, ElInit, JsFragment, JsScope, PipelineOutput};
use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::emit::{EmitOptions, css as css_emit, js as js_emit};
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// =============================================================================
// Runtime Module Includes (same pattern as metasystem/timeline.rs)
// =============================================================================

const RUNTIME_CORE: &str = include_str!("../../public/runtime/st.js");
const RUNTIME_RAF: &str = include_str!("../../public/runtime/raf-coordinator.js");
const RUNTIME_EASING: &str = include_str!("../../public/runtime/easing.js");
const RUNTIME_COLOR: &str = include_str!("../../public/runtime/color.js");
const RUNTIME_INTERPOLATE: &str = include_str!("../../public/runtime/interpolate.js");
const RUNTIME_STAGGER: &str = include_str!("../../public/runtime/stagger.js");
const RUNTIME_VALUE_FUNCTIONS: &str = include_str!("../../public/runtime/value-functions.js");
const RUNTIME_TEMPLATES: &str = include_str!("../../public/runtime/templates.js");
const RUNTIME_EDITABLE: &str = include_str!("../../public/runtime/editable.js");

// =============================================================================
// Emission Logic
// =============================================================================

/// Emit typed JS and CSS fragments to final output.
///
/// JS fragments are wrapped according to their `JsScope`:
/// - `IIFE`: `(function() { el_init; stmts; cleanup; })();`
/// - `Block`: `{ el_init; stmts; cleanup; }`
/// - `Inline`: raw statements, cleanup goes to global handler
///
/// Cleanup is emitted INSIDE the scope for IIFE/Block (because it
/// references scoped variables like `mql`, `el`).
pub fn emit_typed(
    prelude_js_fragments: &[JsFragment],
    js_fragments: &[JsFragment],
    prelude_css_fragments: &[CssFragment],
    css_fragments: &[CssFragment],
) -> PipelineOutput {
    let opts = EmitOptions::pretty();
    let mut js_parts: Vec<String> = Vec::new();
    let mut global_cleanup: Vec<String> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut has_selector_inits = false;

    for frag in js_fragments {
        if frag.stmts.is_empty() && frag.cleanup.is_empty() {
            continue;
        }

        let stmts_str = match js_emit::emit_stmts(&frag.stmts, &opts) {
            Ok(s) => s,
            Err(diag) => {
                diagnostics.push(diag);
                continue;
            }
        };

        let (el_line, multi_selector) = match &frag.el_init {
            Some(ElInit::Selector(sel)) => {
                let escaped = sel.replace('\\', "\\\\").replace('\'', "\\'");
                if escaped.starts_with('#') {
                    // ID selector — unique, use querySelector
                    (
                        format!(
                            "const el = document.querySelector('{}');\nif (!el) return;\n",
                            escaped
                        ),
                        None,
                    )
                } else {
                    // Class/attribute/tag selector — may match multiple elements
                    (String::new(), Some(escaped))
                }
            }
            Some(ElInit::Body) => {
                // If the fragment REASSIGNS `el` (e.g. a test `@let (el = ...)`
                // emits `(el = document.querySelector(...))`), the preamble must
                // be `let`, not `const` — otherwise the reassignment throws
                // "Assignment to constant variable". A plain body fragment keeps
                // `const` (immutable, the common case). Detect a top-level
                // assignment to `el` in the emitted statements.
                let reassigns_el = stmts_str.lines().map(str::trim_start).any(|l| {
                    l.starts_with("el =")
                        || l.starts_with("el=")
                        || l.starts_with("(el =")
                        || l.starts_with("(el=")
                });
                let kw = if reassigns_el { "let" } else { "const" };
                (format!("{kw} el = document.body;\n"), None)
            }
            None => (String::new(), None),
        };

        // Safety guard: if pipeline already provides `const el = ...` via el_init,
        // strip any redundant `const el = el;` from the primitive body.
        // This prevents SyntaxError from duplicate const declarations in the same IIFE.
        let stmts_str = if !el_line.is_empty() || multi_selector.is_some() {
            let trimmed = stmts_str.trim_start();
            if let Some(rest) = trimmed.strip_prefix("const el = el;\n") {
                rest.to_string()
            } else if let Some(rest) = trimmed.strip_prefix("const el = el;") {
                rest.to_string()
            } else {
                stmts_str
            }
        } else {
            stmts_str
        };

        let cleanup_str = if !frag.cleanup.is_empty() {
            match js_emit::emit_stmts(&frag.cleanup, &opts) {
                Ok(c) => {
                    match frag.scope {
                        JsScope::IIFE | JsScope::Block => {
                            format!("\nST.onCleanup(el, () => {{\n{}\n}});", c)
                        }
                        JsScope::Inline => {
                            // Inline cleanup goes to global handler
                            global_cleanup.push(c);
                            String::new()
                        }
                    }
                }
                Err(diag) => {
                    diagnostics.push(diag);
                    String::new()
                }
            }
        } else {
            String::new()
        };

        if let Some(ref selector) = multi_selector {
            // Multi-element selector: use registerSelectorInit + querySelectorAll().forEach()
            // registerSelectorInit ensures elements created dynamically by @each/@template
            // are also initialized via the MutationObserver in st.js.
            let mut hasher = DefaultHasher::new();
            selector.hash(&mut hasher);
            stmts_str.hash(&mut hasher);
            let hash = format!("{:x}", hasher.finish());
            let shadows_selector_element = Regex::new(r"\b(?:const|let|var)\s+el\b")
                .expect("valid selector-init shadow regex")
                .is_match(&stmts_str);
            let inner_body = format!("{}{}", stmts_str, cleanup_str);
            if shadows_selector_element {
                diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W0115,
                        "primitive-body local 'el' shadows the selector-init element param; rename to 'node'/'self'",
                    )
                    .with_span(frag.source.clone().into()),
                );
            }
            // Indent inner_body for readability inside the init function
            let indented = inner_body
                .lines()
                .map(|l| {
                    if l.trim().is_empty() {
                        String::new()
                    } else {
                        format!("  {}", l)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            js_parts.push(format!(
                "(function() {{\n  const init = function(el) {{\n    if (el._st_init_{hash}) return;\n    el._st_init_{hash} = true;\n{body}\n  }};\n  ST.registerSelectorInit('{sel}', init);\n}})();",
                hash = hash,
                sel = selector,
                body = indented
            ));
            has_selector_inits = true;
        } else {
            let body = format!("{}{}{}", el_line, stmts_str, cleanup_str);

            match frag.scope {
                JsScope::IIFE => {
                    js_parts.push(format!("(function() {{\n{}\n}})();", body));
                }
                JsScope::Block => {
                    js_parts.push(format!("{{\n{}\n}}", body));
                }
                JsScope::Inline => {
                    js_parts.push(stmts_str);
                }
            }
        }
    }

    // Add global cleanup registration if any Inline fragments had cleanup
    if !global_cleanup.is_empty() {
        let mut cleanup_block = String::from(
            "\n\n// Cleanup registration\nwindow.addEventListener('beforeunload', () => {\n",
        );
        for c in &global_cleanup {
            for l in c.lines() {
                if !l.trim().is_empty() {
                    cleanup_block.push_str("  ");
                    cleanup_block.push_str(l);
                    cleanup_block.push('\n');
                }
            }
        }
        cleanup_block.push_str("});");
        js_parts.push(cleanup_block);
    }

    // Emit prelude JS first (before per-element IIFEs)
    let mut all_js_parts: Vec<String> = Vec::new();
    if !prelude_js_fragments.is_empty() {
        let opts_prelude = EmitOptions::pretty();
        for frag in prelude_js_fragments {
            if frag.stmts.is_empty() {
                continue;
            }
            match js_emit::emit_stmts(&frag.stmts, &opts_prelude) {
                Ok(s) if !s.trim().is_empty() => {
                    all_js_parts.push(s);
                }
                Ok(_) => {}
                Err(diag) => {
                    diagnostics.push(diag);
                }
            }
        }
    }
    all_js_parts.extend(js_parts);
    // After all selector-init IIFEs, schedule a single batched init pass
    // instead of 78 individual querySelectorAll calls.
    if has_selector_inits {
        all_js_parts.push("ST._scheduleInit(document.body);".to_string());
    }
    let js = all_js_parts.join("\n");

    // Emit CSS (preludes first, then per-element)
    let all_css_exprs: Vec<_> = prelude_css_fragments
        .iter()
        .flat_map(|f| f.exprs.iter().cloned())
        .chain(css_fragments.iter().flat_map(|f| f.exprs.iter().cloned()))
        .collect();
    let css = if all_css_exprs.is_empty() {
        String::new()
    } else {
        css_emit::emit_all(&all_css_exprs, &opts)
    };

    PipelineOutput {
        js,
        css,
        build_scripts: Vec::new(),
        html: Vec::new(),
        diagnostics,
    }
}

/// Emit typed fragments with optional runtime boilerplate.
pub fn emit_typed_with_runtime(
    prelude_js_fragments: &[JsFragment],
    js_fragments: &[JsFragment],
    prelude_css_fragments: &[CssFragment],
    css_fragments: &[CssFragment],
    include_runtime: bool,
) -> PipelineOutput {
    let mut output = emit_typed(
        prelude_js_fragments,
        js_fragments,
        prelude_css_fragments,
        css_fragments,
    );

    if include_runtime && !output.js.is_empty() {
        output.js = format_js_with_runtime(&output.js);
    }

    output
}

/// Wrap JS code with the full Spacetime runtime
///
/// The runtime modules provide:
/// - st.js: Core signals, cleanup, watch, data registry (defines window.ST)
/// - raf-coordinator.js: RAF loop management
/// - easing.js: 28 easing functions
/// - color.js: OKLCH-first color parsing and interpolation
/// - interpolate.js: Keyframe interpolation
/// - stagger.js: Stagger delay calculation
/// - value-functions.js: Wave, noise, random, parallax
/// - templates.js: Template factory utilities
pub fn format_js_with_runtime(js: &str) -> String {
    format!(
        "// Spacetime Runtime v2.0 (Modular)\n\
         // Core + RAF + Easing + Color + Interpolate + Stagger + ValueFunctions + Templates\n\n\
         {}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n\
         // =============================================================================\n\
         // Site Code\n\
         // =============================================================================\n\n\
         {}",
        RUNTIME_CORE,
        RUNTIME_RAF,
        RUNTIME_EASING,
        RUNTIME_COLOR,
        RUNTIME_INTERPOLATE,
        RUNTIME_STAGGER,
        RUNTIME_VALUE_FUNCTIONS,
        RUNTIME_TEMPLATES,
        RUNTIME_EDITABLE,
        js
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::{CssFragment, ElInit, JsFragment, JsScope};
    use super::*;
    use crate::ir::{CssExpr, DeclKind, JsExpr, JsLit, JsStmt};

    // =========================================================================
    // Typed emit tests (scope-aware)
    // =========================================================================

    /// Helper to create a JsStmt::Decl(Const, name, expr)
    fn const_decl(name: &str, init: JsExpr) -> JsStmt {
        JsStmt::Decl {
            kind: DeclKind::Const,
            name: name.to_string(),
            init: Some(init),
        }
    }

    /// Helper to create a JsExpr::Raw
    fn raw_expr(s: &str) -> JsExpr {
        JsExpr::Raw(s.to_string())
    }

    #[test]
    fn two_iife_fragments_same_const_no_redecl() {
        // THE original bug: two fragments both declaring `const mql`
        let frag1 = JsFragment {
            stmts: vec![const_decl(
                "mql",
                raw_expr("window.matchMedia('(max-width: 1290px)')"),
            )],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };
        let frag2 = JsFragment {
            stmts: vec![const_decl(
                "mql",
                raw_expr("window.matchMedia('(max-width: 1023px)')"),
            )],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag1, frag2], &[], &[]);
        // Both should be in separate IIFEs
        let iife_count = output.js.matches("(function()").count();
        assert_eq!(iife_count, 2, "Expected 2 IIFEs, got {}", iife_count);
        assert!(
            output.js.contains("1290px"),
            "Should contain first media query"
        );
        assert!(
            output.js.contains("1023px"),
            "Should contain second media query"
        );
    }

    #[test]
    fn three_media_fragments_isolated() {
        let queries = ["1290px", "1023px", "743px"];
        let frags: Vec<JsFragment> = queries
            .iter()
            .map(|q| JsFragment {
                stmts: vec![const_decl(
                    "mql",
                    raw_expr(&format!("window.matchMedia('(max-width: {})')", q)),
                )],
                cleanup: vec![],
                scope: JsScope::IIFE,
                el_init: Some(ElInit::Body),
                source: Default::default(),
            })
            .collect();

        let output = emit_typed(&[], &frags, &[], &[]);
        let iife_count = output.js.matches("(function()").count();
        assert_eq!(iife_count, 3, "Expected 3 IIFEs for 3 media queries");
        for q in &queries {
            assert!(output.js.contains(q), "Should contain {}", q);
        }
    }

    #[test]
    fn cleanup_inside_iife_scope() {
        let frag = JsFragment {
            stmts: vec![const_decl(
                "mql",
                raw_expr("window.matchMedia('(max-width: 768px)')"),
            )],
            cleanup: vec![JsStmt::Expr(raw_expr(
                "mql.removeEventListener('change', handler)",
            ))],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        // Cleanup should be inside the IIFE via ST.onCleanup
        assert!(
            output.js.contains("ST.onCleanup(el"),
            "Cleanup should use ST.onCleanup"
        );
        assert!(
            output.js.contains("removeEventListener"),
            "Should contain cleanup code"
        );
        // The cleanup should appear inside the IIFE, not as a global handler
        assert!(
            !output.js.contains("beforeunload"),
            "Should NOT have global cleanup handler"
        );
    }

    #[test]
    fn cleanup_inside_block_scope() {
        let frag = JsFragment {
            stmts: vec![const_decl("x", JsExpr::Lit(JsLit::Number(1.0)))],
            cleanup: vec![JsStmt::Expr(raw_expr("console.log('cleanup')"))],
            scope: JsScope::Block,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            output.js.starts_with("{"),
            "Block scope should start with {{"
        );
        assert!(
            output.js.contains("ST.onCleanup"),
            "Block cleanup uses ST.onCleanup"
        );
    }

    #[test]
    fn inline_scope_cleanup_goes_to_global() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("console.log('inline')"))],
            cleanup: vec![JsStmt::Expr(raw_expr("console.log('global cleanup')"))],
            scope: JsScope::Inline,
            el_init: None,
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            !output.js.contains("(function()"),
            "Inline should not have IIFE"
        );
        assert!(
            output.js.contains("beforeunload"),
            "Inline cleanup goes to global handler"
        );
        assert!(
            output.js.contains("global cleanup"),
            "Should contain cleanup text"
        );
    }

    #[test]
    fn empty_js_fragment_emits_nothing() {
        let frag = JsFragment {
            stmts: vec![],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            output.js.is_empty(),
            "Empty fragment should produce no output"
        );
    }

    #[test]
    fn js_only_no_css_output() {
        let frag = JsFragment::new(vec![JsStmt::Expr(raw_expr("console.log(1)"))]);
        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(!output.js.is_empty(), "Should have JS");
        assert!(output.css.is_empty(), "Should have no CSS");
    }

    #[test]
    fn css_only_no_js_output() {
        let css_frag = CssFragment::new(vec![CssExpr::Raw(".x { color: red; }".to_string())]);
        let output = emit_typed(&[], &[], &[], &[css_frag]);
        assert!(output.js.is_empty(), "Should have no JS");
        assert!(output.css.contains("color: red"), "Should have CSS");
    }

    #[test]
    fn el_init_body_emits_document_body() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("console.log(el)"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            output.js.contains("const el = document.body;"),
            "Should init el as document.body"
        );
    }

    #[test]
    fn el_init_class_selector_emits_query_selector_all() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("console.log(el)"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Selector(".header".to_string())),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        // No eager querySelectorAll — replaced by single batched ST._scheduleInit
        assert!(
            !output.js.contains("document.querySelectorAll"),
            "Should NOT have eager querySelectorAll, got: {}",
            output.js
        );
        assert!(
            output
                .js
                .contains("ST.registerSelectorInit('.header', init)"),
            "Class selector should register with ST for dynamic elements, got: {}",
            output.js
        );
        assert!(
            output.js.contains("ST._scheduleInit(document.body)"),
            "Should have batched scheduleInit at end, got: {}",
            output.js
        );
        assert!(
            output.js.contains("_st_init_"),
            "Class selector should have dedup guard, got: {}",
            output.js
        );
    }

    #[test]
    fn selector_init_warns_when_primitive_body_shadows_el() {
        // `%&el` has already been substituted to `el` when this reaches emit_typed.
        let shadowing = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("(() => { const el = el; })()"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Selector(".field".to_string())),
            source: Default::default(),
        };
        let safe = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("(() => { const node = el; })()"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Selector(".safe-field".to_string())),
            source: Default::default(),
        };

        let non_selector = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr(
                "(() => { const el = document.body; })()",
            ))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: None,
            source: Default::default(),
        };

        let output = emit_typed(&[], &[shadowing, safe, non_selector], &[], &[]);
        let warning = output.diagnostics.iter().find(|diagnostic| {
            diagnostic
                .message
                .contains("primitive-body local 'el' shadows")
        });

        assert!(
            warning.is_some(),
            "expected selector-init shadow warning: {:?}",
            output.diagnostics
        );
        assert_eq!(
            warning.unwrap().severity,
            crate::diagnostics::Severity::Warning
        );
        assert_eq!(
            warning.unwrap().code,
            crate::diagnostics::DiagnosticCode::W0115
        );
        assert_eq!(
            output.diagnostics.len(),
            1,
            "only the `el` declaration warns"
        );
    }

    #[test]
    fn el_init_id_selector_emits_query_selector() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("console.log(el)"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Selector("#header".to_string())),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            output.js.contains("document.querySelector('#header')"),
            "ID selector should use querySelector, got: {}",
            output.js
        );
        assert!(
            output.js.contains("if (!el) return;"),
            "ID selector should have null guard, got: {}",
            output.js
        );
    }

    #[test]
    fn el_init_body_has_no_null_guard() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("console.log(el)"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Body),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            output.js.contains("const el = document.body;"),
            "Should init el as body"
        );
        assert!(
            !output.js.contains("if (!el) return;"),
            "Body init should NOT have null guard (body is always present)"
        );
    }

    #[test]
    fn el_init_none_omits_el_binding() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("console.log('no el')"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: None,
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            !output.js.contains("const el ="),
            "Should NOT have el binding"
        );
    }

    #[test]
    fn selector_with_special_chars_escaped() {
        let frag = JsFragment {
            stmts: vec![JsStmt::Expr(raw_expr("1"))],
            cleanup: vec![],
            scope: JsScope::IIFE,
            el_init: Some(ElInit::Selector("a'b".to_string())),
            source: Default::default(),
        };

        let output = emit_typed(&[], &[frag], &[], &[]);
        assert!(
            output.js.contains("a\\'b"),
            "Quote in selector should be escaped"
        );
    }

    #[test]
    fn fragment_ordering_preserved() {
        let frags: Vec<JsFragment> = ["A", "B", "C"]
            .iter()
            .map(|name| JsFragment {
                stmts: vec![JsStmt::Expr(raw_expr(&format!("/* {} */", name)))],
                cleanup: vec![],
                scope: JsScope::IIFE,
                el_init: None,
                source: Default::default(),
            })
            .collect();

        let output = emit_typed(&[], &frags, &[], &[]);
        let a_pos = output.js.find("/* A */").unwrap();
        let b_pos = output.js.find("/* B */").unwrap();
        let c_pos = output.js.find("/* C */").unwrap();
        assert!(
            a_pos < b_pos && b_pos < c_pos,
            "Fragment order should be preserved"
        );
    }

    #[test]
    fn mixed_scopes_in_same_output() {
        let frags = vec![
            JsFragment {
                stmts: vec![JsStmt::Expr(raw_expr("/* iife */"))],
                cleanup: vec![],
                scope: JsScope::IIFE,
                el_init: None,
                source: Default::default(),
            },
            JsFragment {
                stmts: vec![JsStmt::Expr(raw_expr("/* inline */"))],
                cleanup: vec![],
                scope: JsScope::Inline,
                el_init: None,
                source: Default::default(),
            },
            JsFragment {
                stmts: vec![JsStmt::Expr(raw_expr("/* block */"))],
                cleanup: vec![],
                scope: JsScope::Block,
                el_init: None,
                source: Default::default(),
            },
        ];

        let output = emit_typed(&[], &frags, &[], &[]);
        assert!(output.js.contains("(function()"), "Should have IIFE");
        assert!(output.js.contains("/* inline */"), "Should have inline");
        assert!(
            output.js.contains("{\n/* block */"),
            "Should have block scope"
        );
    }

    #[test]
    fn emit_typed_with_runtime_includes_runtime() {
        let frag = JsFragment::new(vec![JsStmt::Expr(raw_expr("console.log(1)"))]);
        let output = emit_typed_with_runtime(&[], &[frag], &[], &[], true);
        assert!(
            output.js.contains("Spacetime Core Runtime"),
            "Should include runtime"
        );
        assert!(
            output.js.contains("console.log(1)"),
            "Should include user code"
        );
    }
}
