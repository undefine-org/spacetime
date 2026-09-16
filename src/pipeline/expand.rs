//! Expand Layer (Layer 4)
//!
//! Expands ResolvedPrimitives to typed IR fragments.
//!
//! This layer:
//! 1. Takes a ResolvedPrimitive
//! 2. Looks up the primitive definition in the MetaRegistry
//! 3. Evaluates the primitive's %emit blocks with bound arguments
//! 4. Returns ExpandedPrimitive instances with typed JsFragment/CssFragment

use log::trace;

use crate::ir::CssExpr;
use crate::metasystem::MetaRegistry;

#[allow(unused_imports)]
use crate::parser::SourceSpan; // used in tests
use crate::syntax::{CapturedValue, TemplateParamKind};
use crate::utils::escape_js_string;

use super::types::{
    CompileError, CssFragment, ElInit, ExpandedPrimitive, JsFragment, JsScope, Phase,
    ResolvedPrimitive,
};

/// Type aliases for backward compatibility — expand layer errors are now CompileError.
pub type ExpandError = CompileError;
pub type ExpandErrorKind = super::types::CompileErrorKind;

// =============================================================================
// Expansion Logic
// =============================================================================

/// Expand ResolvedPrimitives to typed ExpandedPrimitives.
///
/// This is the primary entry point for the expand layer. Uses typed IR
/// (`Vec<JsStmt>`, `Vec<CssExpr>`) through the pipeline — strings are
/// only produced at the final emit step.
pub fn expand_typed(
    primitives: &[ResolvedPrimitive],
    meta_registry: &MetaRegistry,
    scopes_by_name: &std::collections::HashMap<String, &crate::parser::ScopeBlock>,
) -> Result<Vec<ExpandedPrimitive>, ExpandError> {
    let mut results = Vec::new();

    // Wrap the registry ONCE per pass so block-body `.js` compilation (PLAN-026)
    // can reuse the live macro set without re-cloning per primitive.
    let registry_arc = std::sync::Arc::new(meta_registry.clone());

    // PLAN-133: the page's compile-time-known filters are its `@fn` definitions
    // (each lowers to a `fn-registry` invocation). Collected ONCE from the full
    // primitive list so `sexpr` lowering can rewrite bare filter calls to
    // `ST.filters.<name>(…)` — the page knows its functions, as data.
    let helpers: std::sync::Arc<std::collections::HashSet<String>> = std::sync::Arc::new(
        primitives
            .iter()
            .filter(|p| p.primitive_name == "fn-registry")
            .filter_map(|p| p.args.get("name"))
            .filter_map(|v| match v {
                crate::syntax::CapturedValue::Ident(s)
                | crate::syntax::CapturedValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
    );

    // PLAN-133: `%uses` closure — a primitive declares cross-primitive prelude
    // dependencies in its header; the dependency's PRELUDE must reach the page
    // even when the dependency itself is never invoked. Pulled deps expand
    // prelude-only, emit before their dependent (dependency-before-dependent),
    // and dedupe by primitive name via the existing first-occurrence-wins
    // collector. Unknown targets and cycles are compile errors.
    let ordered = order_with_uses(primitives, meta_registry)?;

    for (primitive, prelude_only) in &ordered {
        let mut expanded = expand_single_typed(
            primitive,
            meta_registry,
            &registry_arc,
            scopes_by_name,
            &helpers,
        )?;
        if *prelude_only {
            // A `%uses`-pulled dependency contributes its preludes ONLY — its
            // per-usage code, exports, and html must not run (it was never
            // invoked).
            expanded.js = None;
            expanded.css = None;
            expanded.exports = Vec::new();
            expanded.build_scripts = Vec::new();
            expanded.html = Vec::new();
        }
        results.push(expanded);
    }

    Ok(results)
}

/// Compute the expansion order with `%uses` dependencies pulled in.
///
/// Each entry is the primitive plus a `prelude_only` flag: pulled dependencies
/// expand for their prelude fragments alone. A dependency the page ALREADY
/// expands directly is not re-pulled — its prelude ships via its own instance
/// (preludes all emit before any per-usage code, so the only ordering
/// guarantee a pulled dep must provide is "before the dependent's prelude",
/// which direct list order does not disturb in practice).
fn order_with_uses(
    primitives: &[ResolvedPrimitive],
    registry: &MetaRegistry,
) -> Result<Vec<(ResolvedPrimitive, bool)>, ExpandError> {
    let present: std::collections::HashSet<&str> = primitives
        .iter()
        .map(|p| p.primitive_name.as_str())
        .collect();
    let mut out: Vec<(ResolvedPrimitive, bool)> = Vec::new();
    let mut pulled: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn pull(
        name: &str,
        dependent_span: crate::parser::SourceSpan,
        registry: &MetaRegistry,
        present: &std::collections::HashSet<&str>,
        pulled: &mut std::collections::HashSet<String>,
        stack: &mut Vec<String>,
        out: &mut Vec<(ResolvedPrimitive, bool)>,
    ) -> Result<(), ExpandError> {
        // Cycle check FIRST: a dep already on the path is a cycle even when
        // the page also expands it directly (the present short-circuit below
        // would otherwise mask a → b → a).
        if stack.iter().any(|s| s == name) {
            let mut cycle = stack.clone();
            cycle.push(name.to_string());
            return Err(super::types::CompileError::new(
                super::types::CompileErrorKind::CircularDependency(cycle),
                dependent_span,
            ));
        }
        if present.contains(name) || pulled.contains(name) {
            return Ok(());
        }
        let def = registry.get_primitive(name).ok_or_else(|| {
            super::types::CompileError::new(
                super::types::CompileErrorKind::UnknownPrimitive(format!(
                    "{name} (named in a %uses clause)"
                )),
                dependent_span,
            )
        })?;
        stack.push(name.to_string());
        for dep in def.uses.clone() {
            pull(&dep, dependent_span, registry, present, pulled, stack, out)?;
        }
        stack.pop();
        pulled.insert(name.to_string());
        // A `%uses` target is invoked with no arguments: it must be param-free
        // or all-defaults (required-arg validation rejects it otherwise, with
        // the usual missing-param diagnostic).
        out.push((
            ResolvedPrimitive {
                primitive_name: name.to_string(),
                args: Default::default(),
                phase: super::types::Phase::Global,
                order: 0,
                selector: None,
                css_styles: None,
                outputs: Default::default(),
                span: dependent_span,
            },
            true,
        ));
        Ok(())
    }

    for primitive in primitives {
        if let Some(def) = registry.get_primitive(&primitive.primitive_name) {
            for dep in def.uses.clone() {
                let mut stack = vec![primitive.primitive_name.clone()];
                pull(
                    &dep,
                    primitive.span,
                    registry,
                    &present,
                    &mut pulled,
                    &mut stack,
                    &mut out,
                )?;
            }
        }
        out.push((primitive.clone(), false));
    }
    Ok(out)
}

/// Expand a single ResolvedPrimitive to typed ExpandedPrimitive.
///
/// Uses `generate_primitive_ir()` (not `generate_primitive_js()`) to keep
/// typed IR flowing through the pipeline without premature stringification.
fn expand_single_typed(
    primitive: &ResolvedPrimitive,
    meta_registry: &MetaRegistry,
    registry_arc: &std::sync::Arc<MetaRegistry>,
    scopes_by_name: &std::collections::HashMap<String, &crate::parser::ScopeBlock>,
    helpers: &std::sync::Arc<std::collections::HashSet<String>>,
) -> Result<ExpandedPrimitive, ExpandError> {
    crate::profile_span!("expand_primitive", name = %primitive.primitive_name);
    #[cfg(debug_assertions)]
    trace!(
        "[expand] Expanding primitive '{}' with args: {:?}",
        primitive.primitive_name,
        primitive.args.keys().collect::<Vec<_>>()
    );

    // Look up primitive definition
    if let Some(primitive_def) = meta_registry.get_primitive(&primitive.primitive_name) {
        let prim_args =
            resolved_to_primitive_args(primitive, primitive_def, meta_registry, scopes_by_name)?;
        let mut ir = crate::metasystem::generate_primitive_ir_with_registry(
            primitive_def,
            &prim_args,
            Some(registry_arc.clone()),
            Some(helpers.as_ref()),
        );

        for diag in &ir.diagnostics {
            trace!(
                "[expand] Diagnostic in '{}': {:?}",
                primitive.primitive_name, diag
            );
        }

        // FEAT-170 (BUG-256): an error the IR recorded while parsing an `%emit`
        // block must FAIL the build, never vanish. Previously these diagnostics
        // were only `trace!`d here, so a primitive whose emit block failed to
        // parse emitted ZERO statements and the build reported green — the page
        // shipped without the primitive while every layer (register, bind,
        // resolve) reported success. That is the banned silent-drop class
        // living in the compiler's own plumbing. Reuse the exact error channel
        // every other directive parse error uses: surface as a CompileError,
        // which `pipeline::compile` turns into a counted E0812 error.
        if let Some(err) = ir_emit_parse_error(&ir, primitive) {
            return Err(err);
        }

        // Generate initial-state CSS for apply-animations to prevent FOUC
        if primitive.primitive_name == "apply-animations"
            && let Some(raw_body) = get_raw_keyframe_body(primitive)
            && let Some(selector) = &primitive.selector
        {
            let css_exprs = generate_initial_state_css(selector, &raw_body);
            ir.css_exprs.extend(css_exprs);
        }

        return Ok(ir_to_expanded(ir, primitive));
    }

    // Not a registered primitive — check if it's a macro with %emit blocks
    trace!(
        "[expand] No primitive '{}' found, checking macros",
        primitive.primitive_name
    );
    if let Some(macro_def) = meta_registry.get_macro(&primitive.primitive_name) {
        trace!(
            "[expand] Found macro '{}' with {} body items, args: {:?}",
            primitive.primitive_name,
            macro_def.body.len(),
            primitive.args.keys().collect::<Vec<_>>()
        );

        let emit_blocks: Vec<_> = macro_def
            .body
            .iter()
            .filter_map(|item| match item {
                crate::parser::meta_ast::MacroBodyItem::Emit(eb) => Some(eb.clone()),
                _ => None,
            })
            .collect();

        if !emit_blocks.is_empty() {
            let synthetic = crate::parser::meta_ast::PrimitiveDefAst {
                name: primitive.primitive_name.clone(),
                params: vec![],
                body: crate::parser::meta_ast::PrimitiveBody {
                    emit_blocks,
                    cleanup: None,
                    exports: vec![],
                    if_blocks: vec![],
                },
                uses: vec![],
                span: macro_def.span,
                source_file: macro_def.source_file.clone(),
                doc: None,
            };

            let mut prim_args = crate::metasystem::PrimitiveArgs::new();
            for (name, value) in &primitive.args {
                let js_value = coerce_to_declared_param_type(
                    &primitive.primitive_name,
                    name,
                    value,
                    meta_registry,
                );
                prim_args = prim_args.param(name, &js_value);
            }

            // Inject contextual CSS params (same as resolved_to_primitive_args does)
            if let Some(ref sel) = primitive.selector
                && !sel.is_empty()
            {
                prim_args = prim_args.param("self", sel);
            }
            if let Some(ref styles) = primitive.css_styles {
                prim_args = prim_args.param("styles", styles);
            }

            let ir = crate::metasystem::generate_primitive_ir_with_registry(
                &synthetic,
                &prim_args,
                Some(registry_arc.clone()),
                Some(helpers.as_ref()),
            );
            // Same FEAT-170 guard as the primitive branch above: a macro whose
            // `%emit` block fails to parse must fail the build, not emit a
            // silently-empty body.
            if let Some(err) = ir_emit_parse_error(&ir, primitive) {
                return Err(err);
            }
            return Ok(ir_to_expanded(ir, primitive));
        }
    }

    // BUG-268: a primitive name that resolves to nothing — neither a registered
    // `%primitive` nor a macro with `%emit` blocks — is a genuine authoring
    // error (e.g. a `%binds` naming a `%primitive` that does not exist). It used
    // to emit a `// Primitive not found: <name>` comment INTO THE SHIPPED BUNDLE
    // and report success (the banned silent-drop class). Fail the build instead:
    // `compile` counts this as a hard E0956.
    return Err(CompileError::new(
        super::types::CompileErrorKind::UnknownPrimitive(primitive.primitive_name.clone()),
        primitive.span,
    )
    .with_primitive_name(primitive.primitive_name.clone()));
}

/// Promote a JS-EMIT PARSE ERROR the IR recorded into a `CompileError`, or
/// `None` when the IR is clean.
///
/// FEAT-170 (BUG-256): `parse_and_resolve_js_with_cleanup` reports an
/// `%emit`-block that fails to PARSE as an error-severity diagnostic whose
/// message begins "JS emit parse error". The expand layer used to DROP
/// `ir.diagnostics` (a `trace!` only), so a broken emit block emitted zero
/// statements and the build stayed green — the page shipped without the
/// primitive while every layer (register, bind, resolve) reported success.
/// That is the banned silent-drop class living in the compiler's own plumbing.
///
/// We scope to the PARSE-error class specifically: a marker-RESOLUTION failure
/// ("unknown parameter 'name-width' during marker resolution", e.g. the
/// `--st-%name-width` greedy-marker false positive in `element-ref-impl`)
/// coexists with valid partial output and is a separate, tolerated defect —
/// promoting it would break existing primitives. A block that failed to parse
/// contributes ZERO statements, so it is always fatal to the primitive's
/// correctness and must fail the build through the same channel every other
/// directive parse error does (a `CompileError`, which `pipeline::compile`
/// turns into a counted E0812 error).
fn ir_emit_parse_error(
    ir: &crate::metasystem::GeneratedPrimitiveIR,
    primitive: &ResolvedPrimitive,
) -> Option<CompileError> {
    let diag = ir.diagnostics.iter().find(|d| {
        d.severity == crate::diagnostics::Severity::Error
            && d.message.starts_with("JS emit parse error")
    })?;
    Some(CompileError::new(
        super::types::CompileErrorKind::TemplateError(format!(
            "primitive '{}' emit block: {}",
            primitive.primitive_name, diag.message
        )),
        primitive.span,
    ))
}

/// Convert a `GeneratedPrimitiveIR` to `ExpandedPrimitive`, applying scope
/// and element init based on the primitive's phase and selector.
fn ir_to_expanded(
    ir: crate::metasystem::GeneratedPrimitiveIR,
    primitive: &ResolvedPrimitive,
) -> ExpandedPrimitive {
    // If CSS was generated with styles content, suppress JS —
    // the styling is handled by pure CSS, no matchMedia needed.
    // PLAN-077 W6: this optimization is MEDIA's (the %binds media(...) case,
    // where the JS is only a matchMedia fallback) — it must not fire for any
    // CSS-emitting primitive whose JS does independent runtime work
    // (state-reflect's JS is the reactive writer; suppressing it killed
    // @state reflection entirely). Gate it to media's primitive name.
    let suppress_js = !ir.css_exprs.is_empty()
        && primitive.css_styles.is_some()
        && primitive.primitive_name == "media";

    let js = if !ir.js_stmts.is_empty() && !suppress_js {
        // Determine element initialization based on phase and selector
        let el_init = match primitive.phase {
            // An empty-string selector is semantically "no selector" (a file-level
            // directive resolves through `.with_selector(…unwrap_or_default())`, which
            // turns `None` into `""`). Treat `Some("")` exactly like `None`: only
            // bind `el` to `document.body` when the primitive actually needs an
            // element reference (it %yields exports or registers %cleanup). Without
            // this, EVERY selectorless test-body directive (@eval/@let/@assert/@then)
            // got a spurious `const el = document.body` preamble — which both bloats
            // the body and collides with an eval'd `var el` inside @eval (BUG-101),
            // and is the per-step `const el` that FUP-072 set out to remove.
            Phase::Global => match primitive.selector.as_deref() {
                Some(sel) if !sel.is_empty() => Some(ElInit::Selector(sel.to_string())),
                _ => {
                    if !ir.exports.is_empty() || !ir.cleanup_stmts.is_empty() {
                        Some(ElInit::Body)
                    } else {
                        None
                    }
                }
            },
            Phase::Selector => match primitive.selector.as_deref() {
                Some(sel) if !sel.is_empty() => Some(ElInit::Selector(sel.to_string())),
                _ => Some(ElInit::Body),
            },
        };

        Some(JsFragment {
            stmts: ir.js_stmts,
            cleanup: ir.cleanup_stmts,
            scope: JsScope::IIFE, // Always IIFE: prevents const redeclaration across primitives
            el_init,
            source: primitive.span,
        })
    } else {
        None
    };

    let css = if !ir.css_exprs.is_empty() {
        Some(CssFragment {
            exprs: ir.css_exprs,
            source: primitive.span,
        })
    } else {
        None
    };

    // Stringify build-time JS statements
    let build_scripts = if !ir.build_js_stmts.is_empty() {
        let opts = crate::emit::EmitOptions::pretty();
        match crate::emit::js::emit_stmts(&ir.build_js_stmts, &opts) {
            Ok(s) if !s.trim().is_empty() => vec![s],
            _ => vec![],
        }
    } else {
        vec![]
    };

    // Build prelude JS fragment (inline scope, no element context)
    let prelude_js = if !ir.prelude_js_stmts.is_empty() {
        Some(JsFragment {
            stmts: ir.prelude_js_stmts,
            cleanup: vec![],
            scope: JsScope::Inline,
            el_init: None,
            source: primitive.span,
        })
    } else {
        None
    };

    // Build prelude CSS fragment
    let prelude_css = if !ir.prelude_css_exprs.is_empty() {
        Some(CssFragment {
            exprs: ir.prelude_css_exprs,
            source: primitive.span,
        })
    } else {
        None
    };

    ExpandedPrimitive {
        js,
        css,
        exports: ir.exports,
        build_scripts,
        prelude_js,
        prelude_css,
        primitive_name: primitive.primitive_name.clone(),
        html: ir.html,
    }
}

/// Convert ResolvedPrimitive args (BoundArgs) to PrimitiveArgs for codegen.
///
/// Uses the primitive definition to distinguish element refs from params.
fn resolved_to_primitive_args(
    primitive: &ResolvedPrimitive,
    primitive_def: &crate::parser::meta_ast::PrimitiveDefAst,
    meta_registry: &MetaRegistry,
    scopes_by_name: &std::collections::HashMap<String, &crate::parser::ScopeBlock>,
) -> Result<crate::metasystem::PrimitiveArgs, ExpandError> {
    use crate::parser::meta_ast::PrimitiveParam;

    let mut prim_args = crate::metasystem::PrimitiveArgs::new();

    // Build a set of element param names from the definition
    let element_params: std::collections::HashSet<String> = primitive_def
        .params
        .iter()
        .filter_map(|p| match p {
            PrimitiveParam::Element(name) => Some(name.clone()),
            _ => None,
        })
        .collect();

    for (name, value) in &primitive.args {
        if element_params.contains(name) {
            // Element reference — for selector-phase primitives, the element is
            // the `el` callback parameter from wrap_with_element_scope, so use "el".
            // For global-phase, convert to a querySelector expression.
            // EXCEPTION: a NAMED element ref (`&hero` — an IDENTITY, one sigil
            // one meaning) is not the scope element. Resolve it through the
            // ref registry; forcing "el" would silently retarget the
            // primitive to the scope element (measured: `@on &hero.visible`
            // observed the card itself, the subject dropped).
            let js_expr = if primitive.phase == Phase::Selector {
                match value {
                    CapturedValue::Element(name) if name != "self" => {
                        format!("ST.ref({:?})", name)
                    }
                    _ => "el".to_string(),
                }
            } else {
                match value {
                    CapturedValue::Selector(sel) | CapturedValue::Element(sel)
                        if primitive.selector.as_deref() == Some(sel.as_str()) =>
                    {
                        // Same selector as the IIFE's `const el = ...` — reuse it
                        "el".to_string()
                    }
                    CapturedValue::Selector(sel) => {
                        format!("document.querySelector('{}')", escape_js_string(sel))
                    }
                    // FEAT-142 (revC P2): a DOTTED element capture in an
                    // `element`-typed param is a FACET PATH, not a DOM selector —
                    // `document.querySelector('e.c.f')` would be nonsense. Resolve it
                    // against the world registry instead (the facet may be an
                    // element handle, e.g. an entity's marker). A future typed
                    // diagnostic can reject a facet path in a param that strictly
                    // wants a DOM element once component facet TYPES exist (Wave D).
                    CapturedValue::Element(sel) if crate::parser::is_facet_path(sel) => {
                        crate::parser::emit_facet_read_js(sel)
                    }
                    CapturedValue::Element(sel) => {
                        format!("document.querySelector('{}')", escape_js_string(sel))
                    }
                    _ => captured_to_js_with_registry(value, meta_registry),
                }
            };
            prim_args = prim_args.element(name, &js_expr);
        } else if name == "body" {
            // FEAT-119/120: the factory `body` payload (html/states/exports/refs +
            // reactive builder) is serialized from the World-A template SCOPE keyed
            // `@template:<name>`. The `ComponentBody` capture is a bare presence-marker
            // carrying NOTHING (body-validation diagnostics land in `StFile.diagnostics`);
            // it only flags that this match HAS a body so we lower via the scope path.
            // element-param names come from the sibling `params` capture so `&param`
            // holes lower.
            let is_component_body = matches!(value, CapturedValue::ComponentBody);
            let js_value = if is_component_body {
                let tmpl_element_params: Vec<String> = primitive
                    .args
                    .get("params")
                    .and_then(|v| match v {
                        CapturedValue::ParamList(defs) => Some(
                            defs.iter()
                                .filter(|d| {
                                    matches!(d.kind, crate::syntax::TemplateParamKind::Element)
                                })
                                .map(|d| d.name.clone())
                                .collect(),
                        ),
                        _ => None,
                    })
                    .unwrap_or_default();
                // Resolve the template name (captured as `&name` Ident, with prefix).
                let tmpl_name = primitive.args.get("name").and_then(|v| match v {
                    CapturedValue::Ident(n) | CapturedValue::Element(n) => {
                        Some(n.strip_prefix('&').unwrap_or(n).to_string())
                    }
                    _ => None,
                });
                match tmpl_name.as_deref().and_then(|n| scopes_by_name.get(n)) {
                    Some(scope) => {
                        crate::syntax::component_body_to_js_from_scope(scope, &tmpl_element_params)
                    }
                    None => {
                        // INVARIANT I1 (FEAT-119): every body-bearing construct resolves to
                        // exactly one `@template:<name>` scope. A miss is an internal
                        // compiler invariant break, never author error. Surface it loudly
                        // (E0925) instead of silently emitting an empty body.
                        return Err(ExpandError::new(
                            crate::pipeline::types::CompileErrorKind::BodyScopeMissing {
                                name: tmpl_name.unwrap_or_else(|| "<anonymous>".to_string()),
                                primitive: primitive.primitive_name.clone(),
                            },
                            primitive.span,
                        ));
                    }
                }
            } else {
                captured_to_js_with_registry(value, meta_registry)
            };
            prim_args = prim_args.param(name, &js_value);
        } else {
            // Regular parameter — convert to JS literal, honouring the type the
            // primitive DECLARED for it (BUG-294: a `duration` capture reaching a
            // `number` param must arrive as milliseconds, not as `"5m"`).
            let js_value = coerce_to_declared_param_type(
                &primitive.primitive_name,
                name,
                value,
                meta_registry,
            );
            prim_args = prim_args.param(name, &js_value);
        }
    }

    // Auto-bind missing element params for selector-phase primitives.
    // Mirrors metasystem expand.rs:1958 — if a primitive declares &container
    // but the macro %binds didn't explicitly include &self, default to "el"
    // (the selector scope callback parameter).
    if primitive.phase == Phase::Selector {
        for param in &primitive_def.params {
            if let PrimitiveParam::Element(name) = param
                && !prim_args.elements.contains_key(name)
            {
                trace!(
                    "Auto-binding missing element param '{}' to 'el' for selector-phase primitive '{}'",
                    name, primitive.primitive_name
                );
                prim_args = prim_args.element(name, "el");
            }
        }
    }

    // Inject contextual CSS params for %emit css templates.
    // These are not declared primitive params — they're context from the calling macro.
    if let Some(sel) = &primitive.selector {
        prim_args = prim_args.param("self", sel);
    }
    if let Some(styles) = &primitive.css_styles {
        prim_args = prim_args.css_param("styles", styles);
    }

    // BUG-154: thread the `%binds` output aliases (export -> alias) into
    // PrimitiveArgs.outputs so build_emit_context_structured applies them via
    // `ctx.with_output`, and a `%yield expr -> $export` in the primitive body
    // emits `ST.set(el, "alias", …)` (the name the page reads). Without this the
    // aliased signal is never written.
    for (export_name, alias) in &primitive.outputs {
        prim_args = prim_args.output(export_name, alias);
    }

    Ok(prim_args)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert a CapturedValue to a JavaScript expression, with macro expansion support
///
/// This function handles expression-level macros (macros with %binds that return values).
/// When an expression like `localStorage("key", [])` is encountered, it checks if
/// `localStorage` is a macro with %binds and expands it to the appropriate primitive.
/// Nested-in-record rendering (PLAN-077 W2): inside a Named/Array record an
/// `Expr` capture is DATA — source text the runtime re-evaluates (a cond-mode
/// guard, a variant payload arg) — not a code splice. Verbatim, the emit pass
/// re-tokenizes its bare `$sig` into `ST.get(el, …)` (unbound `el` in the IIFE
/// scope), silently breaking the record contract (`{ expr: "<guard text>" }`).
/// Quote it, keeping the `$` intact for the runtime's dep-scan/eval rewrite.
/// Top-level param splices keep `captured_to_js_with_registry` (Expr verbatim
/// + macro/keyframe expansion).
fn captured_to_js_data_with_registry(
    value: &CapturedValue,
    meta_registry: &MetaRegistry,
) -> String {
    match value {
        CapturedValue::Expr(e) => format!("\"{}\"", escape_js_string(e)),
        CapturedValue::Array(items) => {
            // Same template-invocation contract as the top-level serializer:
            // an array of invocation maps renders as `[{ name, args }]` with
            // args as CODE (template factories evaluate them positionally).
            if items
                .iter()
                .all(|v| matches!(v, CapturedValue::Named(map) if is_template_invocation_map(map)))
            {
                let items_str: Vec<String> = items
                    .iter()
                    .filter_map(|v| {
                        if let CapturedValue::Named(map) = v {
                            let name = map.get("name")?;
                            let args = map.get("args")?;
                            Some(format!(
                                "{{ name: {}, args: {} }}",
                                captured_to_js_with_registry(name, meta_registry),
                                captured_to_js_with_registry(args, meta_registry)
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                return format!("[{}]", items_str.join(", "));
            }
            let items_str: Vec<String> = items
                .iter()
                .map(|v| captured_to_js_data_with_registry(v, meta_registry))
                .collect();
            format!("[{}]", items_str.join(", "))
        }
        CapturedValue::Named(map) => {
            // A single invocation map nested in a record (e.g. a match arm's
            // `inv:`) renders as a one-element array — the runtime contract
            // (`a.inv[0]`), mirroring the top-level serializer.
            if is_template_invocation_map(map) {
                let name = map
                    .get("name")
                    .map(|v| captured_to_js_with_registry(v, meta_registry))
                    .unwrap_or_else(|| "\"\"".to_string());
                let args = map
                    .get("args")
                    .map(|v| captured_to_js_with_registry(v, meta_registry))
                    .unwrap_or_else(|| "[]".to_string());
                return format!("[{{ name: {}, args: {} }}]", name, args);
            }
            let mut pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let key = if k.contains('-')
                        || k.contains(' ')
                        || k.starts_with(|c: char| c.is_numeric())
                    {
                        format!("\"{}\"", escape_js_string(k))
                    } else {
                        k.clone()
                    };
                    format!(
                        "{}: {}",
                        key,
                        captured_to_js_data_with_registry(v, meta_registry)
                    )
                })
                .collect();
            pairs.sort();
            format!("{{ {} }}", pairs.join(", "))
        }
        other => captured_to_js_with_registry(other, meta_registry),
    }
}

/// Render a captured value as JS for a primitive parameter, honouring the type
/// that primitive DECLARED for it.
///
/// A macro and the primitive it binds to can disagree about representation, and
/// the declaration is what settles it. `@data fetch`'s form captures
/// `$refresh:duration`, whose value is its SOURCE TEXT under PLAN-122
/// (`"5m"`), while `stdlib/primitives/data/source.st:20` declares
/// `refresh: number = 0` and its body computes
/// `if (refreshInterval > 0) setInterval(load, refreshInterval)`. Passing the
/// string through makes `("5m" > 0)` false in JS, so the polling silently never
/// starts — no error, a data source that simply never refreshes (BUG-294).
///
/// The conversion happens HERE because this is the only place that knows BOTH
/// halves: what the value is, and what the receiver asked for. Doing it in the
/// extractor would make duration the one scalar with a different representation
/// from its siblings; doing it in codegen would mean re-deriving a value's type
/// from its spelling, which is the shadow-classifier pattern PLAN-122 removes.
fn coerce_to_declared_param_type(
    primitive_name: &str,
    param_name: &str,
    value: &CapturedValue,
    meta_registry: &MetaRegistry,
) -> String {
    use crate::parser::meta_ast::{ParamType, PrimitiveParam};

    let declared_number = meta_registry
        .get_primitive(primitive_name)
        .map(|def| {
            def.params.iter().any(|p| match p {
                PrimitiveParam::Typed { name, ty, .. } => {
                    name == param_name
                        && matches!(
                            ty,
                            ParamType::Simple(t) | ParamType::Optional(t) if t == "number"
                        )
                }
                PrimitiveParam::TypedData { name, ty } => name == param_name && ty == "number",
                _ => false,
            })
        })
        .unwrap_or(false);

    if declared_number {
        // A duration in a numeric slot converts to milliseconds. Only a value the
        // duration converter RECOGNISES is rewritten — anything else falls through
        // unchanged, so a genuinely non-numeric value still reaches codegen as
        // itself rather than being coerced into a plausible lie.
        // `Expr` is included because a body-param capture (`{ refresh: 5m }`)
        // arrives as an unparsed expression rather than as a typed scalar; the
        // declared param type is what tells us it should be a number either way.
        let text = match value {
            CapturedValue::String(s) => Some(s.trim().trim_matches('"').to_string()),
            CapturedValue::Ident(s) | CapturedValue::Expr(s) => Some(s.trim().to_string()),
            _ => None,
        };
        if let Some(text) = text
            && let Some((ms, _)) = crate::syntax::conversions::parse_duration_with_unit(&text)
        {
            return ms.to_string();
        }
    }

    captured_to_js_with_registry(value, meta_registry)
}

fn captured_to_js_with_registry(value: &CapturedValue, meta_registry: &MetaRegistry) -> String {
    match value {
        CapturedValue::Expr(e) => {
            // Check if this expression is a macro call that needs expansion
            if let Some(expanded) = try_expand_expression_macro(e, meta_registry) {
                return expanded;
            }
            // Check if this is a keyframe body (contains -> transitions)
            // Keyframe bodies contain Spacetime animation syntax like:
            //   .dot { scale: 0 -> 1; opacity: 0 -> 1; }
            // These need to be converted to structured JS objects for apply-animations.
            // BUG-194: the conversion is STRICT — only content that fully parses as
            // a keyframes body is converted; anything else (e.g. a @test body that
            // merely CONTAINS a keyframe directive) falls through untouched.
            if e.contains("->")
                && (e.contains('{') || e.contains(':'))
                && let Some(converted) = try_convert_keyframe_body_to_js(e)
            {
                return converted;
            }
            // Check if value is empty/comment-only (e.g., keyframe body with just a comment)
            // These would produce invalid JS if emitted as-is (e.g., `const anims = // comment;`)
            let trimmed = e.trim();
            if trimmed.is_empty()
                || trimmed
                    .lines()
                    .all(|l| l.trim().is_empty() || l.trim().starts_with("//"))
            {
                return "null".to_string();
            }
            // Not a macro or keyframe body — check if it's a CSS literal that needs quoting
            if looks_like_css_literal(e) {
                format!("\"{}\"", e)
            } else {
                e.clone()
            }
        }
        CapturedValue::Array(items) => {
            if items
                .iter()
                .all(|v| matches!(v, CapturedValue::Named(map) if is_template_invocation_map(map)))
            {
                let items_str: Vec<String> = items
                    .iter()
                    .filter_map(|v| {
                        if let CapturedValue::Named(map) = v {
                            let name = map.get("name")?;
                            let args = map.get("args")?;
                            Some(format!(
                                "{{ name: {}, args: {} }}",
                                captured_to_js_with_registry(name, meta_registry),
                                captured_to_js_with_registry(args, meta_registry)
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                return format!("[{}]", items_str.join(", "));
            }

            let items_str: Vec<String> = items
                .iter()
                .map(|v| captured_to_js_data_with_registry(v, meta_registry))
                .collect();
            format!("[{}]", items_str.join(", "))
        }
        CapturedValue::Named(map) => {
            if is_template_invocation_map(map) {
                let name = map
                    .get("name")
                    .map(|v| captured_to_js_with_registry(v, meta_registry))
                    .unwrap_or_else(|| "\"\"".to_string());
                let args = map
                    .get("args")
                    .map(|v| captured_to_js_with_registry(v, meta_registry))
                    .unwrap_or_else(|| "[]".to_string());
                return format!("[{{ name: {}, args: {} }}]", name, args);
            }

            // Sort keys for deterministic output — HashMap iteration order is
            // random per instance, which changes _st_init_ content hashes.
            let mut pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let key = if k.contains('-')
                        || k.contains(' ')
                        || k.starts_with(|c: char| c.is_numeric())
                    {
                        format!("\"{}\"", escape_js_string(k))
                    } else {
                        k.clone()
                    };
                    format!(
                        "{}: {}",
                        key,
                        captured_to_js_data_with_registry(v, meta_registry)
                    )
                })
                .collect();
            pairs.sort();
            format!("{{ {} }}", pairs.join(", "))
        }
        CapturedValue::String(s) => {
            let trimmed = s.trim();
            // A matched pair of quotes requires len >= 2; a lone `"` (len 1) would
            // slice `[1..0]` and panic. Guard the length so single-char or empty
            // values pass through verbatim.
            let unquoted = if trimmed.len() >= 2
                && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                    || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
            {
                &trimmed[1..trimmed.len() - 1]
            } else {
                trimmed
            };
            // Detect keyframe body: contains -> transitions with scoped blocks or property syntax.
            // BUG-194: strict — refuse to mangle content that isn't fully a keyframes body.
            if unquoted.contains("->")
                && (unquoted.contains('{') || unquoted.contains(':'))
                && let Some(converted) = try_convert_keyframe_body_to_js(unquoted)
            {
                return converted;
            }
            // Non-keyframe string: delegate to standard conversion
            captured_to_js(value)
        }
        // For other variants, delegate to the non-registry version
        _ => captured_to_js(value),
    }
}

// =============================================================================
// Initial State CSS Generation (FOUC Prevention)
// =============================================================================

/// Extract the raw keyframe body string from the primitive's "animations" arg.
/// Returns `None` if the arg is missing or doesn't contain `->` transitions.
fn get_raw_keyframe_body(primitive: &ResolvedPrimitive) -> Option<String> {
    primitive.args.get("animations").and_then(|v| match v {
        CapturedValue::Expr(e) if e.contains("->") => Some(e.clone()),
        CapturedValue::String(s) => {
            let unquoted = s.trim().trim_matches(|c| c == '"' || c == '\'');
            if unquoted.contains("->") {
                Some(unquoted.to_string())
            } else {
                None
            }
        }
        // Structured flat keyframes (per-element @on visible/@scroll) reconstitute a raw
        // body so initial-state (progress=0) FOUC CSS still generates — the element must
        // start at its `val0` (e.g. opacity:0) before JS hydrates (BUG-086).
        //
        // A nested-scope entry (`.child { opacity: 0 -> 1; }`, BUG-201) carries its
        // selector on the KeyframeDef. It must be reconstituted INSIDE its scope
        // block, never flattened onto the root: flattening put every child's
        // initial value on the PARENT (a `.title { scale: 0.15 -> 1 }` became
        // `transform: scale(0.15)` on the whole scene), so the page shipped with
        // the wrong element shrunk/blurred/hidden until the driver's first tick —
        // and under `render`, which captures frame 0 at exactly that instant, the
        // wrong element stayed wrong for the whole film.
        CapturedValue::Keyframes(kfs) if !kfs.is_empty() => {
            let body = keyframes_to_raw_body(kfs);
            if body.contains("->") {
                Some(body)
            } else {
                None
            }
        }
        _ => None,
    })
}
/// Reconstitute a `Keyframes` capture into the raw `prop: a -> b;` body shape that
/// `generate_initial_state_css` parses, preserving nested scopes: consecutive
/// entries sharing a `selector` are grouped into one `sel { … }` block, in first-
/// appearance order; root-level entries (no selector) stay at the top level.
fn keyframes_to_raw_body(kfs: &[crate::syntax::KeyframeDef]) -> String {
    let mut root = Vec::new();
    let mut scopes: Vec<(String, Vec<String>)> = Vec::new();
    for k in kfs {
        let line = format!("{}: {};", k.property, k.values.join(" -> "));
        match &k.selector {
            None => root.push(line),
            Some(sel) => match scopes.iter_mut().find(|(s, _)| s == sel) {
                Some((_, lines)) => lines.push(line),
                None => scopes.push((sel.clone(), vec![line])),
            },
        }
    }
    let mut out = root;
    for (sel, lines) in scopes {
        out.push(format!("{} {{ {} }}", sel, lines.join(" ")));
    }
    out.join(" ")
}
/// Generate CSS rules that set the initial state (progress=0 values) for animated
/// properties. This prevents FOUC where elements flash visible before JS initializes.
///
/// For each property with `val0 -> val1 -> ...`, extracts ONLY `val0` (the initial
/// state at progress=0). Transform sub-properties are composed into a single
/// `transform` declaration. All output is wrapped in `@media (prefers-reduced-motion:
/// no-preference) { ... }`.
///
/// # Arguments
/// * `parent_selector` - The parent element selector (e.g., ".hero")
/// * `raw_body` - The raw keyframe body string with `->` transitions
///
/// # Returns
/// A `Vec<CssExpr>` containing a single `CssExpr::Raw` with the complete CSS block,
/// or an empty vec if no animated properties are found.
pub fn generate_initial_state_css(parent_selector: &str, raw_body: &str) -> Vec<CssExpr> {
    // Control properties that are NOT CSS properties
    const CONTROL_PROPS: &[&str] = &["easing", "range", "stagger"];

    // Transform sub-property names (Spacetime syntax -> CSS function names)
    const TRANSFORM_PROPS: &[(&str, &str)] = &[
        ("translate-x", "translateX"),
        ("translateX", "translateX"),
        ("translate-y", "translateY"),
        ("translateY", "translateY"),
        ("translate-z", "translateZ"),
        ("translateZ", "translateZ"),
        ("scale", "scale"),
        ("scale-x", "scaleX"),
        ("scaleX", "scaleX"),
        ("scale-y", "scaleY"),
        ("scaleY", "scaleY"),
        ("rotate", "rotate"),
        ("rotate-x", "rotateX"),
        ("rotateX", "rotateX"),
        ("rotate-y", "rotateY"),
        ("rotateY", "rotateY"),
        ("skew", "skew"),
        ("skew-x", "skewX"),
        ("skewX", "skewX"),
        ("skew-y", "skewY"),
        ("skewY", "skewY"),
    ];

    // Filter sub-property names
    const FILTER_PROPS: &[(&str, &str)] = &[
        ("blur", "blur"),
        ("brightness", "brightness"),
        ("contrast", "contrast"),
        ("grayscale", "grayscale"),
        ("saturate", "saturate"),
        ("sepia", "sepia"),
        ("hue-rotate", "hue-rotate"),
        ("invert", "invert"),
    ];

    /// Extract the initial value (first value before first `->`) from an animation value.
    /// Returns `None` for static properties (no `->`) or var() values.
    fn extract_initial_value(value: &str) -> Option<String> {
        // PLAN-150 W1: strip film-surface stop syntax (`at`, per-step `--easing`)
        // from the first stop so it never leaks into the initial-state CSS.
        crate::metasystem::expand::first_stop_value(value)
    }

    /// Classify a property and extract its initial value into the appropriate bucket.
    fn classify_property(
        name: &str,
        value: &str,
        simple_decls: &mut Vec<(String, String)>,
        transform_parts: &mut Vec<(String, String)>,
        filter_parts: &mut Vec<(String, String)>,
    ) {
        if CONTROL_PROPS.contains(&name) {
            return;
        }
        let initial = match extract_initial_value(value) {
            Some(v) => v,
            None => return,
        };

        // Check transform sub-properties
        for &(st_name, css_fn) in TRANSFORM_PROPS {
            if name == st_name {
                transform_parts.push((css_fn.to_string(), initial));
                return;
            }
        }

        // Check filter sub-properties
        for &(st_name, css_fn) in FILTER_PROPS {
            if name == st_name {
                filter_parts.push((css_fn.to_string(), initial));
                return;
            }
        }

        // Simple CSS property
        simple_decls.push((name.to_string(), initial));
    }

    /// Build CSS declarations string from classified properties.
    fn build_declarations(
        simple_decls: &[(String, String)],
        transform_parts: &[(String, String)],
        filter_parts: &[(String, String)],
    ) -> String {
        let mut decls = Vec::new();
        for (prop, val) in simple_decls {
            decls.push(format!("    {}: {};", prop, val));
        }
        if !transform_parts.is_empty() {
            let transform_val: Vec<String> = transform_parts
                .iter()
                .map(|(func, val)| format!("{}({})", func, val))
                .collect();
            decls.push(format!("    transform: {};", transform_val.join(" ")));
        }
        if !filter_parts.is_empty() {
            let filter_val: Vec<String> = filter_parts
                .iter()
                .map(|(func, val)| format!("{}({})", func, val))
                .collect();
            decls.push(format!("    filter: {};", filter_val.join(" ")));
        }
        decls.join("\n")
    }

    // Parse the raw body into scoped blocks and root-level properties
    let trimmed = raw_body.trim();
    let mut scopes: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut root_props: Vec<(String, String)> = Vec::new();

    let mut pos = 0;
    while pos < trimmed.len() {
        while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= trimmed.len() {
            break;
        }
        // Skip line comments
        if trimmed[pos..].starts_with("//") {
            if let Some(nl) = trimmed[pos..].find('\n') {
                pos += nl + 1;
            } else {
                break;
            }
            continue;
        }
        let rest = &trimmed[pos..];
        if let Some(scope_result) = try_parse_scope_block(rest) {
            scopes.push((scope_result.selector, scope_result.properties));
            pos += scope_result.consumed;
        } else if let Some(prop_result) = try_parse_property(rest) {
            root_props.push((prop_result.name, prop_result.value));
            pos += prop_result.consumed;
        } else {
            pos += 1;
        }
    }

    // Collect CSS rule blocks
    let mut css_rules: Vec<String> = Vec::new();

    // Process root-level properties (applied to parent_selector)
    {
        let mut simple = Vec::new();
        let mut transforms = Vec::new();
        let mut filters = Vec::new();
        for (name, value) in &root_props {
            classify_property(name, value, &mut simple, &mut transforms, &mut filters);
        }
        let decls = build_declarations(&simple, &transforms, &filters);
        if !decls.is_empty() {
            css_rules.push(format!("  {} {{\n{}\n  }}", parent_selector, decls));
        }
    }

    // Process scoped blocks (applied to parent_selector + child selector)
    for (child_selector, props) in &scopes {
        let mut simple = Vec::new();
        let mut transforms = Vec::new();
        let mut filters = Vec::new();
        for (name, value) in props {
            classify_property(name, value, &mut simple, &mut transforms, &mut filters);
        }
        let decls = build_declarations(&simple, &transforms, &filters);
        if !decls.is_empty() {
            let combined = if child_selector == "&self" || child_selector == "&" {
                parent_selector.to_string()
            } else {
                format!("{} {}", parent_selector, child_selector)
            };
            css_rules.push(format!("  {} {{\n{}\n  }}", combined, decls));
        }
    }

    if css_rules.is_empty() {
        return vec![];
    }

    // Wrap everything in @media (prefers-reduced-motion: no-preference)
    let css = format!(
        "@media (prefers-reduced-motion: no-preference) {{\n{}\n}}",
        css_rules.join("\n")
    );

    vec![CssExpr::Raw(css)]
}

// =============================================================================
// Keyframe Body Conversion
// =============================================================================

/// Convert a Spacetime keyframe body to a structured JavaScript object
///
/// Input: raw Spacetime animation body like:
///   `.dot { scale: 0 -> 1; opacity: 0 -> 1; easing: ~ease-out-back; }`
///
/// Output: structured JS object like:
///   `{ scopes: [{ selector: '.dot', properties: [...], easing: 'ease-out-back' }] }`
///
/// Ported from `convert_properties_to_keyframes_js` in `src/metasystem/expand.rs`
#[cfg_attr(not(test), allow(dead_code))]
fn convert_keyframe_body_to_js(raw: &str) -> String {
    let mut scopes: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut root_props: Vec<(String, String)> = Vec::new();

    // Parse scoped blocks and root-level properties
    let trimmed = raw.trim();

    // State machine to parse: `.selector { prop: val; ... }` and `prop: val;`
    let mut pos = 0;

    while pos < trimmed.len() {
        // Skip whitespace
        while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= trimmed.len() {
            break;
        }

        // Skip line comments (// ... until newline)
        if trimmed[pos..].starts_with("//") {
            if let Some(nl) = trimmed[pos..].find('\n') {
                pos += nl + 1;
            } else {
                break; // comment extends to end of input
            }
            continue;
        }

        // Check if this looks like a scoped block (starts with . # & or alphabetic selector)
        // We need to find if there's a `{` before a `:` to distinguish selector blocks from properties
        let rest = &trimmed[pos..];

        if let Some(scope_result) = try_parse_scope_block(rest) {
            scopes.push((scope_result.selector, scope_result.properties));
            pos += scope_result.consumed;
        } else if let Some(prop_result) = try_parse_property(rest) {
            root_props.push((prop_result.name, prop_result.value));
            pos += prop_result.consumed;
        } else {
            // Skip unrecognized character
            pos += 1;
        }
    }

    // Build JS object from parsed data
    build_keyframes_js_object(&root_props, &scopes)
}

/// Strict, fallible sibling of [`convert_keyframe_body_to_js`] (BUG-194).
///
/// The lenient converter force-parses ANY text containing `->` plus `{`/`:` —
/// including @test/@fixture/@given/@mount bodies that merely CONTAIN a keyframe
/// directive (`@scroll d(...) { opacity: 0 -> 1; }`) — into a garbage
/// `{ properties: [{ property: '@scroll d(start', … }] }` object. That garbage
/// then fails the block-body fragment re-parse in emit/js.rs and the WHOLE
/// test body silently compiles to an empty function (false-green tests; all
/// six tests in tests/unit/animations/mouse-tilt.test.st were vacuous because
/// of this). This strict variant returns `None` unless the ENTIRE input is
/// consumed by the keyframes grammar — scope blocks (`sel { … }`) and
/// `ident: value (-> value)* ;` property lines — so non-keyframes content
/// falls through to the default (raw) conversion untouched.
fn try_convert_keyframe_body_to_js(raw: &str) -> Option<String> {
    let mut scopes: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut root_props: Vec<(String, String)> = Vec::new();
    let trimmed = raw.trim();
    let mut pos = 0;

    while pos < trimmed.len() {
        // Skip whitespace
        while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= trimmed.len() {
            break;
        }
        // Skip line comments (// ... until newline)
        if trimmed[pos..].starts_with("//") {
            match trimmed[pos..].find('\n') {
                Some(nl) => {
                    pos += nl + 1;
                    continue;
                }
                // Comment extends to end of input: consumed, valid.
                None => break,
            }
        }

        let rest = &trimmed[pos..];
        if let Some(scope_result) = try_parse_scope_block(rest) {
            // Strict: a keyframes selector never contains '@' or ';' — this is
            // what rejects `@scroll d(start: 0, end: 1) { … }` & friends.
            if scope_result.selector.contains('@') || scope_result.selector.contains(';') {
                return None;
            }
            scopes.push((scope_result.selector, scope_result.properties));
            pos += scope_result.consumed;
        } else if let Some(prop_result) = try_parse_property(rest) {
            // Strict: property names are CSS idents (incl. --custom-props and
            // dashed animation props like translate-x), never
            // `@directive d(start` fragments.
            if !is_css_ident(&prop_result.name) {
                return None;
            }
            root_props.push((prop_result.name, prop_result.value));
            pos += prop_result.consumed;
        } else {
            // The lenient machine skips one char and mangles on; strict refuses.
            return None;
        }
    }

    if scopes.is_empty() && root_props.is_empty() {
        return None;
    }
    Some(build_keyframes_js_object(&root_props, &scopes))
}

/// CSS ident: `[-a-zA-Z_][-a-zA-Z0-9_]*` — covers `--custom` properties and
/// dashed animation properties (`translate-x`, `rotate-y`).
fn is_css_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '-' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

struct ScopeParseResult {
    selector: String,
    properties: Vec<(String, String)>,
    consumed: usize,
}

struct PropertyParseResult {
    name: String,
    value: String,
    consumed: usize,
}

/// Try to parse a scoped block like `.selector { prop: val; ... }`
fn try_parse_scope_block(input: &str) -> Option<ScopeParseResult> {
    // Find the opening brace
    let brace_pos = input.find('{')?;
    let colon_pos = input.find(':');

    // If a "property-style" colon comes before the brace, this is a property not a scope.
    // Pseudo-selectors (:first-child, ::before, :not(...)) have colons that are part of
    // the selector, NOT property separators. Distinguish them:
    //   - Pseudo-selector colon: followed by alphanumeric, another colon, or hyphen
    //   - Property colon: followed by whitespace, digit, or nothing
    if colon_pos.is_some() {
        let pre_brace = &input[..brace_pos];
        let has_property_colon = pre_brace.char_indices().any(|(i, c)| {
            if c != ':' {
                return false;
            }
            match pre_brace[i + 1..].chars().next() {
                // Pseudo-selector: directly followed by word char, another colon, or hyphen
                Some(ch) if ch.is_alphanumeric() || ch == ':' || ch == '-' => false,
                // Property separator: followed by whitespace, end, or anything else
                _ => true,
            }
        });
        if has_property_colon {
            return None;
        }
    }

    // Strip leading line comments from selector text
    let selector: String = input[..brace_pos]
        .lines()
        .filter(|line| !line.trim().starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ");
    let selector = selector.trim().to_string();
    if selector.is_empty() {
        return None;
    }

    // Find matching closing brace
    let mut depth = 0;
    let mut close_pos = None;
    for (i, c) in input[brace_pos..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close_pos = Some(brace_pos + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_pos = close_pos?;
    let body = &input[brace_pos + 1..close_pos];

    // Parse properties within the scope body
    let properties = parse_properties_from_body(body);

    Some(ScopeParseResult {
        selector,
        properties,
        consumed: close_pos + 1,
    })
}

/// Try to parse a single property like `opacity: 0 -> 1;` or `easing: ~ease-out;`
fn try_parse_property(input: &str) -> Option<PropertyParseResult> {
    let colon_pos = input.find(':')?;
    let name = input[..colon_pos].trim().to_string();

    if name.is_empty() || name.contains('{') || name.contains('}') {
        return None;
    }

    // Find the end of the value (semicolon or end of input)
    let value_start = colon_pos + 1;
    let rest = &input[value_start..];

    // Find semicolon, but not inside parentheses or braces
    let mut paren_depth = 0;
    let mut brace_depth = 0;
    let mut end_pos = rest.len();
    for (i, c) in rest.char_indices() {
        match c {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '{' => brace_depth += 1,
            '}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                } else {
                    // Hit a closing brace at depth 0 — stop before consuming
                    // into a scope block's territory
                    end_pos = i;
                    break;
                }
            }
            ';' if paren_depth == 0 && brace_depth == 0 => {
                end_pos = i;
                break;
            }
            _ => {}
        }
    }

    let value = rest[..end_pos].trim().to_string();
    let consumed = value_start + end_pos + if end_pos < rest.len() { 1 } else { 0 };

    Some(PropertyParseResult {
        name,
        value,
        consumed,
    })
}

/// Parse properties from a scope body string
fn parse_properties_from_body(body: &str) -> Vec<(String, String)> {
    let mut props = Vec::new();
    let mut pos = 0;
    let trimmed = body.trim();

    while pos < trimmed.len() {
        // Skip whitespace
        while pos < trimmed.len() && trimmed.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= trimmed.len() {
            break;
        }

        let rest = &trimmed[pos..];
        if let Some(prop) = try_parse_property(rest) {
            props.push((prop.name, prop.value));
            pos += prop.consumed;
        } else {
            pos += 1;
        }
    }

    props
}

/// Parse keyframe value like "0 -> 0.35" or "0 -> 0.5 -> 1" into JS array string
// PLAN-150 W1: positioned stops (`at 25%`) and per-step easing (`--form`) are
// parsed by the single implementation in `metasystem::expand` — this path
// (the `@on` inline-body / build_keyframes_js_object route) delegates to it so
// both keyframe serializers stay byte-identical. One grammar, one parser.
use crate::metasystem::expand::parse_keyframes_value;

/// Parse stagger value like "0.006 grid(8 6) center" into JS object string
fn parse_stagger_value(val: &str) -> String {
    let trimmed = val.trim();
    let mut parts_iter = trimmed.splitn(2, char::is_whitespace);

    // First part is always the delay. calcStagger consumes MILLISECONDS
    // (the root-level `defaultStaggerDelay` rail already passes ms), so a
    // time literal is normalized here: `120ms` -> 120, `0.12s` -> 120. A
    // bare number is already ms. Before this, `stagger: 120ms` parsed as
    // f64, failed, and silently emitted `delay: 0` — no stagger, green build.
    let delay_str = parts_iter.next().unwrap_or("0");
    let delay: f64 = {
        let s = delay_str.trim();
        if let Some(ms) = s.strip_suffix("ms") {
            ms.trim().parse().unwrap_or(0.0)
        } else if let Some(sec) = s.strip_suffix('s') {
            sec.trim().parse::<f64>().unwrap_or(0.0) * 1000.0
        } else {
            s.parse().unwrap_or(0.0)
        }
    };

    let rest = parts_iter.next().unwrap_or("").trim();

    // Check for grid pattern: grid(N M)
    let (grid, from) = if rest.starts_with("grid(") {
        if let Some(paren_end) = rest.find(')') {
            let grid_inner = &rest[5..paren_end];
            let grid_parts: Vec<&str> = grid_inner.split_whitespace().collect();
            let cols: u32 = grid_parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            let rows: u32 = grid_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let from_str = rest[paren_end + 1..].trim();
            let from = if from_str.is_empty() {
                "first"
            } else {
                from_str
            };
            (Some((cols, rows)), from.to_string())
        } else {
            (None, "first".to_string())
        }
    } else if rest.is_empty() {
        (None, "first".to_string())
    } else {
        // Just a direction like "center"
        (None, rest.to_string())
    };

    let mut result = format!("{{ delay: {}", delay);
    if let Some((cols, rows)) = grid {
        result.push_str(&format!(", grid: [{}, {}]", cols, rows));
    }
    result.push_str(&format!(", from: '{}' }}", from));
    result
}

/// Build the final JS object from parsed root properties and scopes
fn build_keyframes_js_object(
    root_props: &[(String, String)],
    scopes: &[(String, Vec<(String, String)>)],
) -> String {
    let mut js_parts: Vec<String> = Vec::new();

    // Process root-level properties
    let mut root_properties: Vec<String> = Vec::new();
    let mut root_static: Vec<String> = Vec::new();

    for (prop, val) in root_props {
        if prop == "easing" || prop == "range" || prop == "stagger" {
            // These are handled at scope/root level, skip as properties
            continue;
        }
        if val.contains("->") {
            let keyframes = parse_keyframes_value(val);
            root_properties.push(format!(
                "{{ property: '{}', keyframes: {} }}",
                prop, keyframes
            ));
        } else {
            root_static.push(format!(
                "{{ property: '{}', value: '{}' }}",
                prop,
                val.replace('\'', "\\'")
            ));
        }
    }

    if !root_properties.is_empty() {
        js_parts.push(format!("properties: [{}]", root_properties.join(", ")));
    }
    if !root_static.is_empty() {
        js_parts.push(format!("staticProperties: [{}]", root_static.join(", ")));
    }

    // Process scopes
    let mut js_scopes: Vec<String> = Vec::new();

    for (selector, scope_props) in scopes {
        let mut properties: Vec<String> = Vec::new();
        let mut static_properties: Vec<String> = Vec::new();
        let mut scope_easing: Option<String> = None;
        let mut scope_range: Option<(f64, f64)> = None;
        let mut scope_stagger: Option<String> = None;

        for (prop, val) in scope_props {
            if prop == "easing" {
                // Strip ~ prefix from easing names (Spacetime syntax convention)
                scope_easing = Some(val.trim_start_matches('~').to_string());
            } else if prop == "range" {
                // Parse "0 to 0.5" or "0.2 to 0.8"
                let parts: Vec<&str> = val.split(" to ").collect();
                if parts.len() == 2
                    && let (Ok(start), Ok(end)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                {
                    scope_range = Some((start, end));
                }
            } else if prop == "stagger" {
                scope_stagger = Some(parse_stagger_value(val));
            } else if val.contains("->") {
                let keyframes = parse_keyframes_value(val);
                properties.push(format!(
                    "{{ property: '{}', keyframes: {} }}",
                    prop, keyframes
                ));
            } else {
                static_properties.push(format!(
                    "{{ property: '{}', value: '{}' }}",
                    prop,
                    val.replace('\'', "\\'")
                ));
            }
        }

        let mut scope_obj = format!("{{ selector: '{}'", selector);

        if !properties.is_empty() {
            scope_obj.push_str(&format!(", properties: [{}]", properties.join(", ")));
        }
        if !static_properties.is_empty() {
            scope_obj.push_str(&format!(
                ", staticProperties: [{}]",
                static_properties.join(", ")
            ));
        }
        if let Some(easing) = scope_easing {
            scope_obj.push_str(&format!(", easing: '{}'", easing));
        }
        if let Some((start, end)) = scope_range {
            scope_obj.push_str(&format!(", range: {{ start: {}, end: {} }}", start, end));
        }
        if let Some(stagger) = scope_stagger {
            scope_obj.push_str(&format!(", stagger: {}", stagger));
        }
        scope_obj.push_str(" }");

        js_scopes.push(scope_obj);
    }

    if !js_scopes.is_empty() {
        js_parts.push(format!("scopes: [{}]", js_scopes.join(", ")));
    }

    // Handle root-level easing, range, stagger
    for (prop, val) in root_props {
        if prop == "easing" {
            js_parts.push(format!("easing: '{}'", val.trim_start_matches('~')));
        } else if prop == "range" {
            let parts: Vec<&str> = val.split(" to ").collect();
            if parts.len() == 2
                && let (Ok(start), Ok(end)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>())
            {
                js_parts.push(format!("range: {{ start: {}, end: {} }}", start, end));
            }
        } else if prop == "stagger" {
            js_parts.push(format!("stagger: {}", parse_stagger_value(val)));
        }
    }

    if js_parts.is_empty() {
        "{}".to_string()
    } else {
        format!("{{ {} }}", js_parts.join(", "))
    }
}

// =============================================================================
// Expression Macro Expansion
// =============================================================================

/// Try to expand an expression string as a macro call
///
/// Parses expressions like `macroName("arg1", arg2)` and checks if macroName
/// is a macro with %binds. If so, expands it to JavaScript.
fn try_expand_expression_macro(expr: &str, meta_registry: &MetaRegistry) -> Option<String> {
    // Parse the expression to extract function name and arguments
    // Pattern: `identifier(args...)`
    let expr = expr.trim();
    let paren_pos = expr.find('(')?;
    let name = expr[..paren_pos].trim();

    // Validate the name is a valid identifier
    if name.is_empty() || !name.chars().next()?.is_alphabetic() {
        return None;
    }

    // Check if this is a macro with %binds
    let macro_def = meta_registry.get_macro(name)?;
    if macro_def.binds.is_empty() {
        return None;
    }

    // Extract arguments from the parentheses
    // Find the matching closing paren
    let mut depth = 0;
    let mut close_paren_pos = None;
    for (i, c) in expr.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_paren_pos = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_paren = close_paren_pos?;
    let args_str = &expr[paren_pos + 1..close_paren];

    // Parse arguments (simple comma-split, respecting quotes and parens)
    let args = parse_macro_args(args_str);

    // Now expand the macro via %binds
    // The %binds clause should reference a primitive
    // For localStorage, it's: local-storage-source(key: $key, default: $default)
    if let Some(bind) = macro_def.binds.first() {
        // Build the primitive call
        let prim_name = &bind.primitive;

        // Get the primitive definition (just to verify it exists)
        let _primitive_def = meta_registry.get_primitive(prim_name)?;

        // Map macro form params to args
        // FormParam.name is the syntax name (may be empty for positional params)
        // We need to get the capture names from the elements
        use crate::parser::meta_ast::FormInlineElement;
        let form_params: Vec<String> = if let Some(form) = &macro_def.form {
            form.params
                .iter()
                .filter_map(|p| {
                    // Try to get the capture name from elements
                    for elem in &p.elements {
                        if let FormInlineElement::Capture(capture, _) = elem {
                            return Some(capture.var_name.clone());
                        }
                    }
                    // Fallback to param name if it's not empty
                    if !p.name.is_empty() {
                        Some(p.name.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        // Build bound args for the primitive
        use crate::parser::meta_ast::BindArg;
        let mut bound_args = super::types::BoundArgs::new();

        for bind_arg in &bind.args {
            match bind_arg {
                BindArg::Named {
                    name: bind_name,
                    value,
                } => {
                    // The value references form params like $key, $default
                    // We need to substitute with actual args
                    let resolved_value = resolve_bind_value(value, &form_params, &args);
                    bound_args.insert(bind_name.clone(), resolved_value);
                }
                BindArg::Positional(value) => {
                    // Positional args - just add by index
                    let resolved_value = resolve_bind_value(value, &form_params, &args);
                    bound_args.insert(format!("_arg{}", bound_args.len()), resolved_value);
                }
                BindArg::Element {
                    name,
                    child_selector,
                } => {
                    // Element reference - keep as variable reference
                    let selector = if let Some(sel) = child_selector {
                        format!("{} {}", name, sel)
                    } else {
                        name.clone()
                    };
                    bound_args.insert(name.clone(), CapturedValue::Element(selector));
                }
            }
        }

        // Create a ResolvedPrimitive and expand it
        let resolved_prim = super::types::ResolvedPrimitive::new(
            prim_name.clone(),
            bound_args,
            super::types::Phase::Global, // Expression macros typically run at global phase
            0,
        );

        // Expand the primitive using typed path
        if let Ok(expanded) = expand_single_typed(
            &resolved_prim,
            meta_registry,
            &std::sync::Arc::new(meta_registry.clone()),
            &std::collections::HashMap::new(),
            &std::sync::Arc::new(std::collections::HashSet::new()),
        ) {
            // Emit JS from the typed fragment
            if let Some(js_frag) = &expanded.js {
                let opts = crate::emit::EmitOptions::pretty();
                if let Ok(js) = crate::emit::js::emit_stmts(&js_frag.stmts, &opts)
                    && !js.is_empty()
                {
                    // Wrap in IIFE for expression context - must RETURN the result!
                    return Some(format!("(() => {{ return (\n{}\n); }})()", js));
                }
            }
        }
    }

    None
}

/// Resolve a BindValue to a CapturedValue using form params and args
fn resolve_bind_value(
    value: &crate::parser::meta_ast::BindValue,
    form_params: &[String],
    args: &[String],
) -> CapturedValue {
    use crate::parser::meta_ast::BindValue as BindVal;

    match value {
        BindVal::Variable(var_name) => {
            // Find the position of this param in the form
            if let Some(pos) = form_params.iter().position(|p| p == var_name) {
                if let Some(arg) = args.get(pos) {
                    // Parse the arg as a CapturedValue
                    parse_arg_to_captured_value(arg)
                } else {
                    // Param not provided, use empty/null
                    CapturedValue::Expr("null".to_string())
                }
            } else {
                // Unknown var, keep as variable reference
                CapturedValue::Expr(format!("${}", var_name))
            }
        }
        BindVal::String(s) => CapturedValue::String(s.clone()),
        BindVal::Number(n) => CapturedValue::Number(*n),
        BindVal::Ident(s) => CapturedValue::Ident(s.clone()),
        BindVal::Array(arr) => {
            let items: Vec<CapturedValue> = arr
                .iter()
                .map(|s| CapturedValue::String(s.clone()))
                .collect();
            CapturedValue::Array(items)
        }
        BindVal::FunctionCall {
            name,
            args: call_args,
        } => {
            // Recursively resolve function call args
            let resolved_args: Vec<String> = call_args
                .iter()
                .map(|a| captured_to_js(&resolve_bind_value(a, form_params, args)))
                .collect();
            CapturedValue::Expr(format!("{}({})", name, resolved_args.join(", ")))
        }
        BindVal::ElementRef(s) => CapturedValue::Element(s.clone()),
    }
}

/// Parse macro arguments from a string, respecting quotes and nested parens
fn parse_macro_args(args_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev_char = ' ';

    for c in args_str.chars() {
        match c {
            '"' | '\'' if !in_string => {
                in_string = true;
                string_char = c;
                current_arg.push(c);
            }
            c if c == string_char && in_string && prev_char != '\\' => {
                in_string = false;
                current_arg.push(c);
            }
            '(' | '[' | '{' if !in_string => {
                depth += 1;
                current_arg.push(c);
            }
            ')' | ']' | '}' if !in_string => {
                depth -= 1;
                current_arg.push(c);
            }
            ',' if depth == 0 && !in_string => {
                let trimmed = current_arg.trim().to_string();
                if !trimmed.is_empty() {
                    args.push(trimmed);
                }
                current_arg = String::new();
            }
            _ => {
                current_arg.push(c);
            }
        }
        prev_char = c;
    }

    // Don't forget the last argument
    let trimmed = current_arg.trim().to_string();
    if !trimmed.is_empty() {
        args.push(trimmed);
    }

    args
}

/// Parse an argument string into a CapturedValue
fn parse_arg_to_captured_value(arg: &str) -> CapturedValue {
    let arg = arg.trim();

    // Check for string literal (len >= 2 so a lone quote can't slice [1..0]).
    if arg.len() >= 2
        && ((arg.starts_with('"') && arg.ends_with('"'))
            || (arg.starts_with('\'') && arg.ends_with('\'')))
    {
        let inner = &arg[1..arg.len() - 1];
        return CapturedValue::String(inner.to_string());
    }

    // Check for number
    if let Ok(n) = arg.parse::<f64>() {
        return CapturedValue::Number(n);
    }

    // Check for boolean
    if arg == "true" {
        return CapturedValue::Bool(true);
    }
    if arg == "false" {
        return CapturedValue::Bool(false);
    }

    // Check for array literal (len >= 2 guards a lone bracket).
    if arg.len() >= 2 && arg.starts_with('[') && arg.ends_with(']') {
        let inner = &arg[1..arg.len() - 1];
        let items = parse_macro_args(inner);
        let values: Vec<CapturedValue> = items
            .iter()
            .map(|s| parse_arg_to_captured_value(s))
            .collect();
        return CapturedValue::Array(values);
    }

    // Check for null/undefined
    if arg == "null" || arg == "undefined" {
        return CapturedValue::Expr("null".to_string());
    }

    // Default to expression
    CapturedValue::Expr(arg.to_string())
}

/// Check if a string looks like a CSS literal value (hex color, CSS dimension)
/// that would be invalid as bare JavaScript and needs quoting.
fn looks_like_css_literal(s: &str) -> bool {
    let trimmed = s.trim();
    // Hex color
    if trimmed.starts_with('#') {
        return true;
    }
    // CSS dimension: digits (optional decimal) followed by a CSS unit
    let css_units = [
        "px", "em", "rem", "%", "vh", "vw", "vmin", "vmax", "deg", "rad", "turn", "ms", "s",
    ];
    for unit in &css_units {
        if let Some(prefix) = trimmed.strip_suffix(unit)
            && !prefix.is_empty()
            && prefix.chars().all(|c| c.is_ascii_digit() || c == '.')
            && prefix.chars().any(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

fn is_template_invocation_map(map: &std::collections::HashMap<String, CapturedValue>) -> bool {
    map.len() == 2 && map.contains_key("name") && map.contains_key("args")
}

/// Convert a CapturedValue to a JavaScript expression
/// Nested-in-record rendering (PLAN-077 W2): inside a Named/Array/Block
/// structure an `Expr` capture is DATA — source text the runtime re-evaluates
/// (a cond-mode guard, a variant payload arg) — not a code splice. Rendered
/// verbatim it would be re-tokenized by the emit pass and its bare `$sig`
/// lowered to `ST.get(el, …)` (an unbound `el` in an IIFE scope), silently
/// breaking the record's contract (`{ expr: "<guard text>" }`). Quote it
/// instead, keeping the `$` intact for the runtime's dep-scan/eval rewrite.
/// Top-level param splices still go through `captured_to_js` (Expr verbatim).
fn captured_to_js_data(value: &CapturedValue) -> String {
    match value {
        CapturedValue::Expr(e) => format!("\"{}\"", escape_js_string(e)),
        other => captured_to_js(other),
    }
}

fn captured_to_js(value: &CapturedValue) -> String {
    match value {
        // Idents are literal identifier values (like animation names, keywords)
        // They should be quoted when used in JavaScript contexts
        // NOTE: Sometimes bindings like "$packs" get captured as Ident instead of Binding
        // due to form matching quirks, so we strip $ prefix here too
        CapturedValue::Ident(s) => {
            let name = s.strip_prefix('$').unwrap_or(s);
            format!("\"{}\"", escape_js_string(name))
        }
        CapturedValue::String(s) => {
            // Defensive: strip outer quotes if they weren't stripped earlier in the pipeline
            // This can happen if body params parsing doesn't properly process string values
            let s = s.trim();
            let unquoted = if s.len() >= 2
                && ((s.starts_with('"') && s.ends_with('"'))
                    || (s.starts_with('\'') && s.ends_with('\'')))
            {
                &s[1..s.len() - 1]
            } else {
                s
            };
            format!("\"{}\"", escape_js_string(unquoted))
        }
        CapturedValue::Number(n) => n.to_string(),
        CapturedValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        CapturedValue::Time(ms) => ms.to_string(),
        CapturedValue::Length(l) => format!("\"{}{}\"", l.value, l.unit),
        CapturedValue::Selector(s) => format!("\"{}\"", escape_js_string(s)),
        CapturedValue::Color(s) => format!("\"{}\"", escape_js_string(s)),
        // Bindings like "$faq" represent data source names - strip the $ prefix
        // The $ is syntax indicating "this is a binding", not part of the name
        CapturedValue::Binding(s) => {
            let name = s.strip_prefix('$').unwrap_or(s);
            format!("\"{}\"", name)
        }
        // FEAT-142 WAVE C (revC P1): a DOTTED element capture
        // (`&evernet.peak.summit`) reaching a PRIMITIVE PARAMETER (e.g.
        // `@anchor(to: &e.c.f)` -> three-anchor's `to:`) is a FACET PATH — route it
        // through the single facet-read emitter (`ST.worldFacet(...)` + tail), not
        // the raw string. A bare single-segment `&name` still emits its plain name
        // (existing element-ref / selector behaviour, revB: bare unchanged).
        CapturedValue::Element(s) if crate::parser::is_facet_path(s) => {
            crate::parser::emit_facet_read_js(s)
        }
        CapturedValue::Element(s) => s.clone(),
        CapturedValue::Expr(e) => {
            if looks_like_css_literal(e) {
                format!("\"{}\"", e)
            } else {
                e.clone()
            }
        }
        CapturedValue::TypeRef(t) => format!("\"{}\"", t),
        CapturedValue::Preset(p) => format!("\"{}\"", p),
        CapturedValue::Json(j) => format!("{:?}", j),
        CapturedValue::Block(matches) => {
            // Serialize Block (Vec<FormMatch>) as array of objects
            // Each FormMatch becomes { __macro: "macro_name", ...captures }
            // Using __macro to avoid conflicts with common capture names like "name"
            let items: Vec<String> = matches
                .iter()
                .map(|fm| {
                    let mut pairs =
                        vec![format!("__macro: \"{}\"", escape_js_string(&fm.macro_name))];

                    // Add captures as additional properties
                    for (k, v) in &fm.captures {
                        let key = if k.contains('-') || k.contains(' ') {
                            format!("\"{}\"", escape_js_string(k))
                        } else {
                            k.clone()
                        };
                        pairs.push(format!("{}: {}", key, captured_to_js_data(v)));
                    }

                    format!("{{ {} }}", pairs.join(", "))
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        CapturedValue::Array(items) => {
            if items
                .iter()
                .all(|v| matches!(v, CapturedValue::Named(map) if is_template_invocation_map(map)))
            {
                let items_str: Vec<String> = items
                    .iter()
                    .filter_map(|v| {
                        if let CapturedValue::Named(map) = v {
                            let name = map.get("name")?;
                            let args = map.get("args")?;
                            Some(format!(
                                "{{ name: {}, args: {} }}",
                                captured_to_js(name),
                                captured_to_js(args)
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                return format!("[{}]", items_str.join(", "));
            }

            let items_str: Vec<String> = items.iter().map(captured_to_js_data).collect();
            format!("[{}]", items_str.join(", "))
        }
        CapturedValue::Named(map) => {
            if is_template_invocation_map(map) {
                let name = map
                    .get("name")
                    .map(captured_to_js)
                    .unwrap_or_else(|| "\"\"".to_string());
                let args = map
                    .get("args")
                    .map(captured_to_js)
                    .unwrap_or_else(|| "[]".to_string());
                return format!("[{{ name: {}, args: {} }}]", name, args);
            }

            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| {
                let rank = |key: &str| match key {
                    "name" => 0,
                    "args" => 1,
                    _ => 2,
                };
                rank(a.as_str())
                    .cmp(&rank(b.as_str()))
                    .then_with(|| a.cmp(b))
            });

            let pairs: Vec<String> = keys
                .into_iter()
                .filter_map(|k| map.get(k).map(|v| (k, v)))
                .map(|(k, v)| {
                    // Quote keys that contain non-identifier characters (like hyphens)
                    let key = if k.contains('-')
                        || k.contains(' ')
                        || k.starts_with(|c: char| c.is_numeric())
                    {
                        format!("\"{}\"", escape_js_string(k))
                    } else {
                        k.clone()
                    };
                    format!("{}: {}", key, captured_to_js_data(v))
                })
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        CapturedValue::Properties(props) => {
            let props_str: Vec<String> = props
                .iter()
                .map(|p| {
                    format!(
                        "{}{}: {}",
                        p.name,
                        if p.optional { "?" } else { "" },
                        p.type_ref
                    )
                })
                .collect();
            format!("[{}]", props_str.join(", "))
        }
        CapturedValue::Params(params) => {
            let params_str: Vec<String> = params
                .iter()
                .map(|p| match &p.default {
                    Some(d) => format!("{}: {} = {}", p.name, p.type_ref, d),
                    None => format!("{}: {}", p.name, p.type_ref),
                })
                .collect();
            format!("[{}]", params_str.join(", "))
        }
        CapturedValue::Keyframes(kfs) => {
            // A structured `:keyframes` capture (flat per-element form, e.g.
            //   @on visible reveal(900ms) { opacity: 0 -> 1; translate-y: 44px -> 0; }
            // ) must serialize to the `{ properties, staticProperties, ... }` object the
            // `apply-animations` runtime consumes — NOT a raw `["opacity: 0 -> 1"]` string
            // array (which leaves `anims.properties` undefined so applyStyle never runs).
            // KeyframeDef is flat (property + value-chain); route it through the same
            // builder the raw-body path uses so both representations converge (BUG-086).
            // BUG-201: entries carrying a nested `selector` become `anims.scopes`
            // (grouped per selector, source order preserved) instead of being dropped.
            let mut root_props: Vec<(String, String)> = Vec::new();
            let mut scopes: Vec<(String, Vec<(String, String)>)> = Vec::new();
            for k in kfs {
                let pair = (k.property.clone(), k.values.join(" -> "));
                match &k.selector {
                    Some(sel) => match scopes.iter_mut().find(|(s, _)| s == sel) {
                        Some((_, props)) => props.push(pair),
                        None => scopes.push((sel.clone(), vec![pair])),
                    },
                    None => root_props.push(pair),
                }
            }
            build_keyframes_js_object(&root_props, &scopes)
        }
        CapturedValue::ParamList(params) => {
            // Convert template param list to JSON array
            // Each param becomes: { name: "...", kind: "binding"|"element", optional: bool }
            let params_str: Vec<String> = params
                .iter()
                .map(|p| {
                    let kind = match p.kind {
                        TemplateParamKind::Binding => "binding",
                        TemplateParamKind::Element => "element",
                    };
                    // Optional type annotation (`$href url`) surfaces to runtime as a
                    // `type` key so editable schema + inline-edit widgets can dispatch
                    // on it; absent for untyped params (byte-identical legacy emit).
                    let type_part = match &p.type_ref {
                        Some(t) => format!(r#", type: "{}""#, t),
                        None => String::new(),
                    };
                    // Default value (`$title = "x"` -> Some("\"x\"")). The raw source
                    // is already a valid JS expression (a quoted string literal, a
                    // number, a bool), so it is emitted VERBATIM as the `default` key:
                    // `"x"` stays a JS string literal evaluating to `x`, `0` stays
                    // numeric. The runtime factory (buildTemplateFactory) seeds
                    // `data[name] = spec.default` for every param with a default before
                    // applying provided args, so an autostaged guest (no args) renders
                    // its declared defaults instead of `[object Object]` (BUG: this key
                    // was previously dropped, so `spec.default` always arrived
                    // `undefined`). Absent for a param with no default (legacy emit).
                    let default_part = match &p.default {
                        Some(d) => format!(r#", default: {}"#, d),
                        None => String::new(),
                    };
                    format!(
                        r#"{{ name: "{}", kind: "{}", optional: {}{}{} }}"#,
                        p.name, kind, p.optional, type_part, default_part
                    )
                })
                .collect();
            format!("[{}]", params_str.join(", "))
        }
        CapturedValue::StyleProperties(props) => {
            let props_str: Vec<String> = props
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{{ \"{}\": \"{}\" }}",
                        escape_js_string(k),
                        escape_js_string(v)
                    )
                })
                .collect();
            format!("[{}]", props_str.join(", "))
        }
        CapturedValue::PatternMatch {
            signal, variant, ..
        } => {
            format!(
                "\"{}:{}\"",
                escape_js_string(signal),
                escape_js_string(variant)
            )
        }
        CapturedValue::ComponentBody => {
            // FEAT-119 (W5): the factory body arm (above) serializes from the World-A
            // scope; this generic fallback is unreachable for a real register-template
            // body. Emit an empty marker for safety.
            "{}".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::pipeline::types::{BoundArgs, Phase};
    use crate::syntax::{CapturedValue, LengthValue};

    #[test]
    fn captured_to_js_routes_dotted_facet_path() {
        // FEAT-142 WAVE C (revC P1): the PRIMITIVE-PARAMETER lowering path (the one
        // `@anchor(to: &e.c.f)` actually flows through) must route a dotted element
        // capture to the facet resolver, NOT emit the raw string. A bare element
        // ref is unchanged.
        assert_eq!(
            captured_to_js(&CapturedValue::Element("track".to_string())),
            "track",
            "bare element ref stays a plain name"
        );
        assert_eq!(
            captured_to_js(&CapturedValue::Element("evernet.peak.summit".to_string())),
            r#"ST.worldFacet("evernet", "peak", "summit")"#,
            "dotted facet path routes to ST.worldFacet in the primitive-arg path"
        );
    }

    /// Helper to stringify an ExpandedPrimitive's JS stmts for assertions
    fn js_string(expanded: &[ExpandedPrimitive]) -> Vec<String> {
        let opts = crate::emit::EmitOptions::pretty();
        expanded
            .iter()
            .map(|e| {
                if let Some(js_frag) = &e.js {
                    crate::emit::js::emit_stmts(&js_frag.stmts, &opts).unwrap_or_default()
                } else {
                    String::new()
                }
            })
            .collect()
    }

    #[test]
    fn test_expand_empty() {
        let meta_registry = MetaRegistry::new();
        let result = expand_typed(&[], &meta_registry, &std::collections::HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    // ==== PLAN-133 W2.5: %uses closure ====

    fn uses_prim_def(name: &str, uses: &[&str]) -> crate::parser::meta_ast::PrimitiveDefAst {
        crate::parser::meta_ast::PrimitiveDefAst {
            name: name.to_string(),
            params: vec![],
            body: crate::parser::meta_ast::PrimitiveBody::default(),
            uses: uses.iter().map(|u| u.to_string()).collect(),
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        }
    }

    fn uses_registry(defs: Vec<crate::parser::meta_ast::PrimitiveDefAst>) -> MetaRegistry {
        let mut registry = MetaRegistry::new();
        for def in defs {
            registry.register_primitive(def).unwrap();
        }
        registry
    }

    fn uses_resolved(name: &str) -> ResolvedPrimitive {
        ResolvedPrimitive::new(name, BoundArgs::new(), Phase::Global, 0)
    }

    #[test]
    fn uses_unknown_target_is_a_compile_error() {
        let registry = uses_registry(vec![uses_prim_def("needs-ghost", &["ghost-helper"])]);
        let primitives = vec![uses_resolved("needs-ghost")];
        let err = order_with_uses(&primitives, &registry)
            .expect_err("unknown %uses target must error");
        assert!(
            format!("{err:?}").contains("ghost-helper"),
            "error must name the missing dependency; got: {err:?}"
        );
    }

    #[test]
    fn uses_cycle_is_a_compile_error() {
        let registry = uses_registry(vec![
            uses_prim_def("cycle-a", &["cycle-b"]),
            uses_prim_def("cycle-b", &["cycle-a"]),
        ]);
        let primitives = vec![uses_resolved("cycle-a")];
        let err = order_with_uses(&primitives, &registry).expect_err("%uses cycle must error");
        assert!(
            format!("{err:?}").contains("Circular"),
            "error must be a circular dependency; got: {err:?}"
        );
    }

    #[test]
    fn uses_pulls_dependency_before_dependent_prelude_only_once() {
        let registry = uses_registry(vec![
            uses_prim_def("helper", &[]),
            uses_prim_def("consumer", &["helper"]),
        ]);
        // Two consumer instances: the dependency must still pull ONCE.
        let primitives = vec![uses_resolved("consumer"), uses_resolved("consumer")];
        let ordered = order_with_uses(&primitives, &registry).expect("order");
        let pulled: Vec<_> = ordered.iter().filter(|(_, prelude_only)| *prelude_only).collect();
        assert_eq!(pulled.len(), 1, "dependency pulls exactly once");
        assert_eq!(pulled[0].0.primitive_name, "helper");
        // Dependency position precedes every dependent.
        let dep_pos = ordered
            .iter()
            .position(|(p, _)| p.primitive_name == "helper")
            .unwrap();
        let consumer_pos = ordered
            .iter()
            .position(|(p, _)| p.primitive_name == "consumer")
            .unwrap();
        assert!(dep_pos < consumer_pos, "dependency before dependent");
    }

    #[test]
    fn uses_transitive_closure_orders_leaves_first() {
        let registry = uses_registry(vec![
            uses_prim_def("c", &[]),
            uses_prim_def("b", &["c"]),
            uses_prim_def("a", &["b"]),
        ]);
        let primitives = vec![uses_resolved("a")];
        let ordered = order_with_uses(&primitives, &registry).expect("order");
        let names: Vec<&str> = ordered
            .iter()
            .map(|(p, _)| p.primitive_name.as_str())
            .collect();
        assert_eq!(names, vec!["c", "b", "a"], "transitive deps emit leaves-first");
    }

    #[test]
    fn uses_directly_present_dependency_is_not_re_pulled() {
        let registry = uses_registry(vec![
            uses_prim_def("helper", &[]),
            uses_prim_def("consumer", &["helper"]),
        ]);
        // The page expands `helper` directly: no synthetic pull needed.
        let primitives = vec![uses_resolved("consumer"), uses_resolved("helper")];
        let ordered = order_with_uses(&primitives, &registry).expect("order");
        let helpers = ordered
            .iter()
            .filter(|(p, _)| p.primitive_name == "helper")
            .count();
        assert_eq!(helpers, 1, "direct instance covers the dependency");
        assert!(
            ordered.iter().all(|(p, prelude_only)| {
                p.primitive_name != "helper" || !*prelude_only
            }),
            "the direct instance keeps its full expansion"
        );
    }

    #[test]
    fn test_expand_unknown_primitive_errors() {
        let meta_registry = MetaRegistry::new();
        let mut args = BoundArgs::new();
        args.insert(
            "name".to_string(),
            CapturedValue::Ident("myData".to_string()),
        );

        let primitive = ResolvedPrimitive::new("fetch-data", args, Phase::Global, 0);

        // BUG-268: an unknown primitive must FAIL the build (E0956), never emit
        // a `// Primitive not found:` comment into the shipped bundle.
        let result = expand_typed(
            &[primitive],
            &meta_registry,
            &std::collections::HashMap::new(),
        );
        assert!(result.is_err(), "unknown primitive must fail the build");
        let err = result.unwrap_err();
        assert!(
            matches!(&err.kind, ExpandErrorKind::UnknownPrimitive(n) if n == "fetch-data"),
            "expected UnknownPrimitive(fetch-data), got {:?}",
            err.kind
        );
    }

    #[test]
    fn test_expand_multiple_unknown_primitives() {
        let meta_registry = MetaRegistry::new();
        let primitives = vec![
            ResolvedPrimitive::new("fetch-data", BoundArgs::new(), Phase::Global, 0),
            ResolvedPrimitive::new("create-state", BoundArgs::new(), Phase::Global, 1),
            ResolvedPrimitive::new("get-element", BoundArgs::new(), Phase::Selector, 0)
                .with_selector(".button"),
        ];

        // BUG-268: the first unresolvable primitive fails the build; expand does
        // not degrade any of them to a comment stub.
        let result = expand_typed(
            &primitives,
            &meta_registry,
            &std::collections::HashMap::new(),
        );
        assert!(result.is_err(), "unknown primitives must fail the build");
        assert!(
            matches!(result.unwrap_err().kind, ExpandErrorKind::UnknownPrimitive(_)),
            "expected UnknownPrimitive error"
        );
    }

    #[test]
    fn test_missing_param_produces_output_with_unresolved_placeholder() {
        use crate::parser::SourceSpan;
        use crate::parser::meta_ast::{
            EmitBlock, EmitLang, ParamType, PrimitiveBody, PrimitiveDefAst, PrimitiveParam,
        };

        // Create a primitive definition with an emit block that references %missing
        let primitive_def = PrimitiveDefAst {
            name: "test-unresolved".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "missing".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "console.log(%missing);".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Register the primitive
        let mut meta_registry = MetaRegistry::new();
        meta_registry.register_primitive(primitive_def).unwrap();

        // Create a ResolvedPrimitive WITHOUT providing the 'missing' arg
        let primitive = ResolvedPrimitive::new(
            "test-unresolved",
            BoundArgs::new(), // Empty args - missing required param
            Phase::Global,
            0,
        );

        // The codegen path produces output (with unresolved placeholders)
        // rather than returning an error
        let result = expand_typed(
            &[primitive],
            &meta_registry,
            &std::collections::HashMap::new(),
        );
        assert!(
            result.is_ok(),
            "Codegen path should not hard-error on missing params"
        );
    }

    #[test]
    fn test_resolved_template_param_succeeds() {
        use crate::parser::SourceSpan;
        use crate::parser::meta_ast::{
            EmitBlock, EmitLang, ParamType, PrimitiveBody, PrimitiveDefAst, PrimitiveParam,
        };

        // Create a primitive definition with an emit block that references %value
        let primitive_def = PrimitiveDefAst {
            name: "test-resolved".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "value".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "console.log(%value);".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Register the primitive
        let mut meta_registry = MetaRegistry::new();
        meta_registry.register_primitive(primitive_def).unwrap();

        // Create a ResolvedPrimitive WITH the 'value' arg provided
        let mut args = BoundArgs::new();
        args.insert(
            "value".to_string(),
            CapturedValue::String("test".to_string()),
        );

        let primitive = ResolvedPrimitive::new("test-resolved", args, Phase::Global, 0);

        let result = expand_typed(
            &[primitive],
            &meta_registry,
            &std::collections::HashMap::new(),
        );

        assert!(result.is_ok());
        let expanded = result.unwrap();
        assert_eq!(expanded.len(), 1);
        // The generated JS should have the value substituted
        let js = js_string(&expanded);
        assert!(js[0].contains("test"));
    }

    #[test]
    fn test_missing_for_loop_array_produces_output() {
        use crate::parser::SourceSpan;
        use crate::parser::meta_ast::{EmitBlock, EmitLang, PrimitiveBody, PrimitiveDefAst};

        // Create a primitive with a %for loop that references a missing array param
        let primitive_def = PrimitiveDefAst {
            name: "test-for-unresolved".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "%for $item in %items { console.log($%item); }".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let mut meta_registry = MetaRegistry::new();
        meta_registry.register_primitive(primitive_def).unwrap();

        // Create primitive without the 'items' array param
        let primitive =
            ResolvedPrimitive::new("test-for-unresolved", BoundArgs::new(), Phase::Global, 0);

        // The codegen path produces output rather than returning an error
        let result = expand_typed(
            &[primitive],
            &meta_registry,
            &std::collections::HashMap::new(),
        );
        assert!(
            result.is_ok(),
            "Codegen path should not hard-error on missing for-loop array"
        );
    }

    // ========================================================================
    // Tests for DX improvements: available params in error hints
    // ========================================================================

    #[test]
    fn test_missing_param_with_partial_args_produces_output() {
        use crate::parser::SourceSpan;
        use crate::parser::meta_ast::{
            EmitBlock, EmitLang, ParamType, PrimitiveBody, PrimitiveDefAst, PrimitiveParam,
        };

        // Create a primitive that references %missing_param but we only provide %url
        let primitive_def = PrimitiveDefAst {
            name: "test-available-params".to_string(),
            params: vec![
                PrimitiveParam::Typed {
                    name: "url".to_string(),
                    ty: ParamType::Simple("string".to_string()),
                    default: None,
                },
                PrimitiveParam::Typed {
                    name: "missing_param".to_string(),
                    ty: ParamType::Simple("string".to_string()),
                    default: None,
                },
            ],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "fetch(%missing_param);".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let mut meta_registry = MetaRegistry::new();
        meta_registry.register_primitive(primitive_def).unwrap();

        // Create a ResolvedPrimitive WITH 'url' but WITHOUT 'missing_param'
        let mut args = BoundArgs::new();
        args.insert(
            "url".to_string(),
            CapturedValue::String("/api/data".to_string()),
        );

        let primitive = ResolvedPrimitive::new("test-available-params", args, Phase::Global, 0);

        // Codegen path produces output with unresolved placeholders
        let result = expand_typed(
            &[primitive],
            &meta_registry,
            &std::collections::HashMap::new(),
        );
        assert!(
            result.is_ok(),
            "Codegen path should not hard-error on missing params"
        );
    }

    // ========================================================================
    // Tests for comment-only / empty keyframe body edge case
    // ========================================================================
    //
    // When a keyframe body contains only comments (e.g., `@scroll page-progress { // comment }`),
    // captured_to_js_with_registry must return "null" instead of emitting the raw text,
    // which would produce invalid JS like `const anims = // comment;`.

    #[test]
    fn test_captured_expr_comment_only_returns_null() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr(
            "// Exposes --st-page-progress-progress for CSS scaleX".to_string(),
        );
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(result, "null", "Comment-only expression should return null");
    }

    #[test]
    fn test_captured_expr_multiline_comments_returns_null() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr("// first comment\n  // second comment\n".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(
            result, "null",
            "Multi-line comment-only expression should return null"
        );
    }

    #[test]
    fn test_captured_expr_empty_returns_null() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr("".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(result, "null", "Empty expression should return null");
    }

    #[test]
    fn test_captured_expr_whitespace_only_returns_null() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr("   \n  \n  ".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(
            result, "null",
            "Whitespace-only expression should return null"
        );
    }

    #[test]
    fn test_captured_expr_mixed_blank_and_comment_returns_null() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr("\n  // a comment\n\n  // another\n  ".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(
            result, "null",
            "Blank lines mixed with comments should return null"
        );
    }

    #[test]
    fn test_captured_expr_normal_code_passes_through() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr("document.title".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(
            result, "document.title",
            "Normal expressions should pass through as-is"
        );
    }

    #[test]
    fn test_captured_expr_code_with_comment_passes_through() {
        let registry = MetaRegistry::new();
        // A body that has real code plus a comment should NOT be treated as comment-only
        let value = CapturedValue::Expr("someFunction() // inline comment".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(
            result, "someFunction() // inline comment",
            "Expression with code + trailing comment should pass through"
        );
    }

    #[test]
    fn test_captured_expr_keyframe_body_converts_to_js_object() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::Expr(".dot { scale: 0 -> 1; opacity: 0 -> 1; }".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        // Should be a structured JS object, not the raw text
        assert!(
            result.contains("properties"),
            "Keyframe body should be converted to JS object with properties"
        );
        assert!(
            result.contains("scale"),
            "Keyframe body should contain 'scale' property"
        );
        assert!(
            result.contains("opacity"),
            "Keyframe body should contain 'opacity' property"
        );
        assert!(
            !result.contains("->"),
            "Keyframe body should not contain raw '->' syntax"
        );
    }

    #[test]
    fn test_captured_keyframes_struct_converts_to_js_object() {
        // BUG-086: a structured `:keyframes` capture (flat per-element @on visible/@scroll,
        // e.g. `{ opacity: 0 -> 1; translate-y: 44px -> 0; }`) must serialize to the
        // `{ properties: [...] }` object the apply-animations runtime consumes — NOT a raw
        // `["opacity: 0 -> 1"]` string array (which left `anims.properties` undefined so
        // applyStyle never ran, killing every reveal/scrollytelling animation).
        use crate::syntax::KeyframeDef;
        let value = CapturedValue::Keyframes(vec![
            KeyframeDef {
                property: "opacity".to_string(),
                values: vec!["0".to_string(), "1".to_string()],
                selector: None,
            },
            KeyframeDef {
                property: "translate-y".to_string(),
                values: vec!["44px".to_string(), "0".to_string()],
                selector: None,
            },
        ]);
        let result = captured_to_js(&value);
        assert!(
            result.contains("properties"),
            "Keyframes struct must produce a JS object with `properties`, got: {}",
            result
        );
        assert!(
            result.contains("keyframes"),
            "Each property must carry a `keyframes` array, got: {}",
            result
        );
        assert!(
            result.contains("opacity") && result.contains("translate-y"),
            "Both animated properties must survive, got: {}",
            result
        );
        assert!(
            !result.trim_start().starts_with('['),
            "Must NOT emit a raw string array (the BUG-086 shape), got: {}",
            result
        );
    }

    #[test]
    fn test_captured_string_keyframe_body_converts_to_js_object() {
        let registry = MetaRegistry::new();
        // Same content as the Expr test, but as CapturedValue::String
        let value = CapturedValue::String(".dot { scale: 0 -> 1; opacity: 0 -> 1; }".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert!(
            result.contains("properties"),
            "String keyframe body should be converted to JS object with properties, got: {}",
            result
        );
        assert!(
            result.contains("scale"),
            "Should contain 'scale' property, got: {}",
            result
        );
        assert!(
            result.contains("opacity"),
            "Should contain 'opacity' property, got: {}",
            result
        );
        assert!(
            !result.contains("->"),
            "Should not contain raw '->' syntax, got: {}",
            result
        );
    }

    #[test]
    fn test_captured_string_with_scoped_selectors_converts() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::String(
            ".line--h1 { opacity: 0 -> 0.35; scale-x: 0 -> 1; transform-origin: left; easing: &ease-out-expo; range: 0 to 0.5; }".to_string()
        );
        let result = captured_to_js_with_registry(&value, &registry);
        assert!(
            result.contains("scopes"),
            "Scoped keyframe body should produce JS with scopes array, got: {}",
            result
        );
        assert!(
            result.contains(".line--h1"),
            "Should contain selector, got: {}",
            result
        );
    }

    #[test]
    fn test_captured_string_plain_text_unchanged() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::String("just a label".to_string());
        let result = captured_to_js_with_registry(&value, &registry);
        assert_eq!(
            result, "\"just a label\"",
            "Non-keyframe strings should be quoted normally"
        );
    }

    #[test]
    fn test_keyframe_body_comment_before_scope_stripped() {
        let registry = MetaRegistry::new();
        let value = CapturedValue::String(
            "// Comment line\n        .selector { opacity: 0 -> 1; }".to_string(),
        );
        let result = captured_to_js_with_registry(&value, &registry);
        assert!(
            !result.contains("// Comment"),
            "Comment text should not appear in JS output, got: {}",
            result
        );
        assert!(
            result.contains(".selector"),
            "Selector should still be present, got: {}",
            result
        );
        assert!(
            result.contains("scopes"),
            "Should produce scoped output, got: {}",
            result
        );
    }

    #[test]
    fn test_keyframe_body_multiple_scopes_with_comments() {
        let registry = MetaRegistry::new();
        // Mirror ikarchitecte pattern: multiple scopes with comment before the second
        let value = CapturedValue::String(
            ".ik-contact__info {\n    opacity: 0 -> 1;\n    easing: &ease-out-expo;\n    range: 0.35 to 0.95;\n}\n\n// Closing line draws\n.ik-contact__line {\n    stroke-dashoffset: 1200 -> 0;\n    easing: &ease-out-expo;\n    range: 0.5 to 1.5;\n}".to_string()
        );
        let result = captured_to_js_with_registry(&value, &registry);
        assert!(
            !result.contains("// Closing"),
            "Comment should not leak into output, got: {}",
            result
        );
        assert!(
            result.contains(".ik-contact__info"),
            "First selector should be present, got: {}",
            result
        );
        assert!(
            result.contains(".ik-contact__line"),
            "Second selector should be present, got: {}",
            result
        );
        // Make sure selector values are clean (no comment text)
        assert!(
            !result.contains("Closing line draws"),
            "Comment text should not be in selector, got: {}",
            result
        );
    }

    // =========================================================================
    // Pseudo-selector scope block tests
    // =========================================================================

    /// Extract selector strings from convert_keyframe_body_to_js output.
    /// Looks for `selector: '...'` patterns in the JS object string.
    fn extract_selectors(js_output: &str) -> Vec<String> {
        let mut selectors = Vec::new();
        let needle = "selector: '";
        let mut search_from = 0;
        while let Some(start) = js_output[search_from..].find(needle) {
            let abs_start = search_from + start + needle.len();
            if let Some(end) = js_output[abs_start..].find('\'') {
                selectors.push(js_output[abs_start..abs_start + end].to_string());
                search_from = abs_start + end + 1;
            } else {
                break;
            }
        }
        selectors
    }

    /// Validate that a selector is valid CSS using lightningcss.
    fn assert_valid_css_selector(selector: &str) {
        let css = format!("{} {{}}", selector);
        let result = lightningcss::stylesheet::StyleSheet::parse(
            &css,
            lightningcss::stylesheet::ParserOptions::default(),
        );
        assert!(
            result.is_ok(),
            "Invalid CSS selector '{}': {:?}",
            selector,
            result.err()
        );
    }

    /// Count scopes in the JS output
    fn count_scopes(js_output: &str) -> usize {
        extract_selectors(js_output).len()
    }

    // --- Pseudo-class selectors (the bug) ---

    #[test]
    fn test_pseudo_first_child() {
        let result = convert_keyframe_body_to_js(".foo:first-child { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
        assert_eq!(selectors[0], ".foo:first-child");
        assert_valid_css_selector(&selectors[0]);
        assert!(
            result.contains("opacity"),
            "Should contain opacity property"
        );
    }

    #[test]
    fn test_pseudo_last_child() {
        let result = convert_keyframe_body_to_js(".foo:last-child { scale: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
        assert_eq!(selectors[0], ".foo:last-child");
        assert_valid_css_selector(&selectors[0]);
    }

    #[test]
    fn test_pseudo_nth_child() {
        let result = convert_keyframe_body_to_js(".foo:nth-child(2) { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
        assert_eq!(selectors[0], ".foo:nth-child(2)");
        assert_valid_css_selector(&selectors[0]);
    }

    #[test]
    fn test_pseudo_hover() {
        let result = convert_keyframe_body_to_js(".foo:hover { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
        assert_eq!(selectors[0], ".foo:hover");
        assert_valid_css_selector(&selectors[0]);
    }

    #[test]
    fn test_pseudo_focus() {
        let result = convert_keyframe_body_to_js(".foo:focus { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_valid_css_selector(&selectors[0]);
    }

    // --- Pseudo-element selectors ---

    #[test]
    fn test_pseudo_element_before() {
        let result = convert_keyframe_body_to_js(".foo::before { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
        assert_eq!(selectors[0], ".foo::before");
        assert_valid_css_selector(&selectors[0]);
    }

    #[test]
    fn test_pseudo_element_after() {
        let result = convert_keyframe_body_to_js(".foo::after { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_valid_css_selector(&selectors[0]);
    }

    // --- Compound pseudo-selectors ---

    #[test]
    fn test_chained_pseudo() {
        let result = convert_keyframe_body_to_js(".foo:first-child:hover { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_valid_css_selector(&selectors[0]);
    }

    #[test]
    fn test_descendant_pseudo() {
        let result = convert_keyframe_body_to_js(".parent .child:first-child { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0], ".parent .child:first-child");
        assert_valid_css_selector(&selectors[0]);
    }

    #[test]
    fn test_child_combinator_pseudo() {
        let result =
            convert_keyframe_body_to_js(".parent > .child:last-child { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_valid_css_selector(&selectors[0]);
    }

    // --- :not() functional pseudo ---

    #[test]
    fn test_pseudo_not() {
        let result = convert_keyframe_body_to_js(".foo:not(.bar) { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_valid_css_selector(&selectors[0]);
    }

    // --- Mixed scopes: exact oraventures hero-entrance pattern ---

    #[test]
    fn test_oraventures_hero_entrance() {
        let input = "\
.ora-hero__line:first-child {
    opacity: 0 -> 1;
    translate-y: 40px -> 0;
    easing: &ease-out-expo;
    range: 0 to 0.35;
}

.ora-hero__flap {
    opacity: 0 -> 1;
    translate-y: 60px -> 0;
    easing: &ease-out-expo;
    range: 0.15 to 0.5;
}

.ora-hero__line:last-child {
    opacity: 0 -> 1;
    translate-y: 40px -> 0;
    easing: &ease-out-expo;
    range: 0.3 to 0.65;
}

.ora-hero__sub {
    opacity: 0 -> 1;
    translate-y: 30px -> 0;
    easing: &ease-out-expo;
    range: 0.5 to 0.8;
}

.ora-cta {
    opacity: 0 -> 1;
    translate-y: 20px -> 0;
    easing: &ease-out-back;
    range: 0.65 to 1;
}";
        let result = convert_keyframe_body_to_js(input);
        let selectors = extract_selectors(&result);
        assert_eq!(
            selectors.len(),
            5,
            "Expected 5 scopes, got {}: {}",
            selectors.len(),
            result
        );

        // All selectors must be valid CSS
        for sel in &selectors {
            assert_valid_css_selector(sel);
            assert!(
                !sel.contains('}'),
                "Selector '{}' contains leaked closing brace",
                sel
            );
        }

        // Check specific selector values
        assert_eq!(selectors[0], ".ora-hero__line:first-child");
        assert_eq!(selectors[1], ".ora-hero__flap");
        assert_eq!(selectors[2], ".ora-hero__line:last-child");
        assert_eq!(selectors[3], ".ora-hero__sub");
        assert_eq!(selectors[4], ".ora-cta");
    }

    // --- Property/scope disambiguation (no false positives) ---

    #[test]
    fn test_property_not_misidentified_as_scope() {
        let result = convert_keyframe_body_to_js("opacity: 0 -> 1; scale: 0 -> 1;");
        assert_eq!(
            count_scopes(&result),
            0,
            "Properties should not create scopes: {}",
            result
        );
        assert!(result.contains("opacity"));
        assert!(result.contains("scale"));
    }

    #[test]
    fn test_mixed_properties_and_scopes() {
        let result = convert_keyframe_body_to_js("easing: &ease-out; .child { opacity: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
        assert_eq!(selectors[0], ".child");
        assert!(result.contains("easing"));
    }

    #[test]
    fn test_colon_in_easing_value() {
        let result = convert_keyframe_body_to_js(
            ".scope { easing: cubic-bezier(0, 0, 0.2, 1); opacity: 0 -> 1; }",
        );
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1, "Expected 1 scope, got: {}", result);
    }

    // --- Edge cases ---

    #[test]
    fn test_pseudo_with_range_and_easing() {
        let result = convert_keyframe_body_to_js(
            ".foo:first-child { opacity: 0 -> 1; easing: &ease-out-expo; range: 0 to 0.35; }",
        );
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0], ".foo:first-child");
        assert!(
            result.contains("easing"),
            "Should capture easing: {}",
            result
        );
        assert!(result.contains("range"), "Should capture range: {}", result);
    }

    #[test]
    fn test_no_brace_leak_between_scopes() {
        let result =
            convert_keyframe_body_to_js(".a:first-child { x: 0 -> 1; }\n.b { y: 0 -> 1; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 2, "Expected 2 scopes, got: {}", result);
        for sel in &selectors {
            assert_valid_css_selector(sel);
            assert!(
                !sel.contains('}'),
                "Selector '{}' contains leaked brace",
                sel
            );
            // No excessive whitespace
            assert!(
                !sel.contains("  "),
                "Selector '{}' has excessive whitespace",
                sel
            );
        }
    }

    #[test]
    fn test_pseudo_selector_with_stagger() {
        let result =
            convert_keyframe_body_to_js(".foo:first-child { opacity: 0 -> 1; stagger: 50ms; }");
        let selectors = extract_selectors(&result);
        assert_eq!(selectors.len(), 1);
        assert!(
            result.contains("stagger"),
            "Should capture stagger: {}",
            result
        );
    }

    #[test]
    fn test_captured_to_js_length_is_quoted() {
        let value = CapturedValue::Length(LengthValue {
            value: 300.0,
            unit: "px".into(),
        });
        let result = captured_to_js(&value);
        assert_eq!(
            result, "\"300px\"",
            "Length values must be quoted for valid JS"
        );
    }

    #[test]
    fn test_captured_to_js_expr_hex_color_is_quoted() {
        let value = CapturedValue::Expr("#E85D4A".to_string());
        let result = captured_to_js(&value);
        assert_eq!(
            result, "\"#E85D4A\"",
            "Hex color expressions must be quoted for valid JS"
        );
    }

    #[test]
    fn test_captured_to_js_expr_normal_js_not_quoted() {
        let value = CapturedValue::Expr("document.title".to_string());
        let result = captured_to_js(&value);
        assert_eq!(
            result, "document.title",
            "Normal JS expressions must NOT be quoted"
        );
    }

    #[test]
    fn test_captured_to_js_expr_dimension_is_quoted() {
        let value = CapturedValue::Expr("1.5em".to_string());
        let result = captured_to_js(&value);
        assert_eq!(
            result, "\"1.5em\"",
            "CSS dimension expressions must be quoted for valid JS"
        );
    }

    #[test]
    fn test_captured_to_js_color_hex() {
        let value = CapturedValue::Color("#E85D4A".to_string());
        let result = captured_to_js(&value);
        assert_eq!(result, "\"#E85D4A\"", "Color values must be quoted in JS");
    }

    #[test]
    fn test_captured_to_js_color_named() {
        let value = CapturedValue::Color("cyan".to_string());
        let result = captured_to_js(&value);
        assert_eq!(result, "\"cyan\"", "Named colors must be quoted in JS");
    }

    #[test]
    fn test_captured_to_js_color_rgb() {
        let value = CapturedValue::Color("rgb(232, 93, 74)".to_string());
        let result = captured_to_js(&value);
        assert_eq!(
            result, "\"rgb(232, 93, 74)\"",
            "RGB colors must be quoted in JS"
        );
    }

    #[test]
    fn test_captured_to_js_color_oklch() {
        let value = CapturedValue::Color("oklch(0.7 0.15 180)".to_string());
        let result = captured_to_js(&value);
        assert_eq!(
            result, "\"oklch(0.7 0.15 180)\"",
            "OKLCH colors must be quoted in JS"
        );
    }

    #[test]
    fn test_captured_to_js_color_color_mix() {
        let value = CapturedValue::Color("color-mix(in srgb, red, blue)".to_string());
        let result = captured_to_js(&value);
        assert_eq!(
            result, "\"color-mix(in srgb, red, blue)\"",
            "color-mix() must be quoted in JS"
        );
    }

    #[test]
    fn test_captured_to_js_template_invocation_array() {
        let mut invocation = HashMap::new();
        invocation.insert(
            "name".to_string(),
            CapturedValue::String("ado-project".to_string()),
        );
        invocation.insert(
            "args".to_string(),
            CapturedValue::Array(vec![CapturedValue::String("$p".to_string())]),
        );

        let value = CapturedValue::Array(vec![CapturedValue::Named(invocation)]);
        let result = captured_to_js(&value);

        assert_eq!(
            result, "[{ name: \"ado-project\", args: [\"$p\"] }]",
            "Template invocation capture should serialize to array of invocation objects"
        );
    }

    #[test]
    fn test_captured_to_js_template_invocation_named_wraps_array() {
        let mut invocation = HashMap::new();
        invocation.insert(
            "name".to_string(),
            CapturedValue::String("ado-project".to_string()),
        );
        invocation.insert(
            "args".to_string(),
            CapturedValue::Array(vec![CapturedValue::String("$p".to_string())]),
        );

        let value = CapturedValue::Named(invocation);
        let result = captured_to_js(&value);

        assert_eq!(
            result, "[{ name: \"ado-project\", args: [\"$p\"] }]",
            "Single template invocation capture should still serialize as invocation array"
        );
    }

    #[test]
    fn test_captured_to_js_paramlist_emits_default_field() {
        // B1 Site 1: a param's `= default` MUST serialize into the runtime spec as a
        // `default:` key (emitted VERBATIM — the raw source is already a valid JS
        // expression). Previously dropped, so the factory's default-seeding got
        // `undefined` and an autostaged guest rendered `[object Object]`.
        use crate::syntax::{TemplateParamDef, TemplateParamKind};
        let params = vec![
            TemplateParamDef {
                name: "title".to_string(),
                kind: TemplateParamKind::Binding,
                optional: false,
                type_ref: None,
                default: Some("\"Launch faster\"".to_string()),
                collection: false,
            },
            TemplateParamDef {
                name: "count".to_string(),
                kind: TemplateParamKind::Binding,
                optional: false,
                type_ref: Some("number".to_string()),
                default: Some("0".to_string()),
                collection: false,
            },
            TemplateParamDef {
                name: "plain".to_string(),
                kind: TemplateParamKind::Binding,
                optional: false,
                type_ref: None,
                default: None,
                collection: false,
            },
        ];
        let result = captured_to_js(&CapturedValue::ParamList(params));
        assert_eq!(
            result,
            "[{ name: \"title\", kind: \"binding\", optional: false, default: \"Launch faster\" }, \
             { name: \"count\", kind: \"binding\", optional: false, type: \"number\", default: 0 }, \
             { name: \"plain\", kind: \"binding\", optional: false }]",
            "ParamList must emit `default:` verbatim for params that have one, and omit it otherwise"
        );
    }

    // =========================================================================
    // Initial state CSS generation tests (FOUC prevention)
    // =========================================================================

    #[test]
    fn test_initial_state_css_simple_properties() {
        let css = generate_initial_state_css(".hero", "opacity: 0 -> 1; scale: 0.9 -> 1;");
        assert!(!css.is_empty(), "Should generate CSS");
        if let CssExpr::Raw(s) = &css[0] {
            assert!(
                s.contains("opacity: 0"),
                "Should contain opacity initial value, got: {}",
                s
            );
            assert!(
                s.contains("transform: scale(0.9)"),
                "Should contain transform initial value, got: {}",
                s
            );
            assert!(
                s.contains(".hero"),
                "Should contain parent selector, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    #[test]
    fn test_initial_state_css_scoped_selectors() {
        let css = generate_initial_state_css(
            ".hero",
            ".child { opacity: 0 -> 1; translate-y: 20px -> 0; }",
        );
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            assert!(
                s.contains(".hero .child"),
                "Should combine parent + child selector, got: {}",
                s
            );
            assert!(
                s.contains("opacity: 0"),
                "Should set opacity initial, got: {}",
                s
            );
            assert!(
                s.contains("translateY(20px)"),
                "Should set translateY initial, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    #[test]
    fn test_initial_state_css_var_values_skipped() {
        let css =
            generate_initial_state_css(".hero", "color: var(--my-color) -> var(--other-color);");
        // var() values should be skipped entirely
        assert!(css.is_empty(), "var() values should produce no CSS output");
    }

    #[test]
    fn test_initial_state_css_reduced_motion_wrapper() {
        let css = generate_initial_state_css(".hero", "opacity: 0 -> 1;");
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            assert!(
                s.contains("prefers-reduced-motion"),
                "Should wrap in @media query, got: {}",
                s
            );
            assert!(
                s.contains("no-preference"),
                "Should use no-preference, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    /// BUG (found rendering demos/promo): a nested-scope keyframe entry
    /// (`.child { scale: 0.15 -> 1 }`, BUG-201's shape) reaches the FOUC
    /// generator through the `Keyframes` capture, whose reconstitution used to
    /// drop `selector` — so the child's initial value was emitted on the ROOT.
    /// The scene shrank to 0.15 and blurred while the child it was meant for
    /// sat untouched. The initial value must land on `root child`, never root.
    #[test]
    fn test_initial_state_css_keyframes_capture_keeps_nested_scope() {
        use crate::syntax::KeyframeDef;
        let kfs = vec![
            KeyframeDef { property: "opacity".into(), values: vec!["0".into(), "1".into()], selector: None },
            KeyframeDef { property: "scale".into(), values: vec!["0.15".into(), "1".into()], selector: Some(".orb".into()) },
            KeyframeDef { property: "blur".into(), values: vec!["30px".into(), "0px".into()], selector: Some(".title".into()) },
            KeyframeDef { property: "translate-y".into(), values: vec!["16px".into(), "0px".into()], selector: Some(".title".into()) },
        ];
        let body = keyframes_to_raw_body(&kfs);
        let css = generate_initial_state_css(".s1", &body);
        let CssExpr::Raw(s) = &css[0] else { panic!("Expected CssExpr::Raw") };
        let root_block = s.split(".s1 {").nth(1).and_then(|r| r.split('}').next()).expect("root block");
        assert!(root_block.contains("opacity: 0"), "root keeps its own initial: {s}");
        assert!(!root_block.contains("scale("), "child scale must NOT land on the root: {s}");
        assert!(!root_block.contains("blur("), "child blur must NOT land on the root: {s}");
        assert!(s.contains(".s1 .orb {") && s.contains("scale(0.15)"), "orb keeps its scale: {s}");
        let title_block = s.split(".s1 .title {").nth(1).and_then(|r| r.split('}').next()).expect("title block");
        assert!(title_block.contains("blur(30px)") && title_block.contains("translateY(16px)"), "title block groups both of its lines: {s}");
    }

    #[test]
    fn test_initial_state_css_skips_non_animated() {
        let css = generate_initial_state_css(".hero", "transform-origin: center;");
        assert!(
            css.is_empty(),
            "Should not generate CSS for static properties"
        );
    }

    #[test]
    fn test_initial_state_css_transform_composition() {
        let css = generate_initial_state_css(
            ".hero",
            "translate-y: 20px -> 0; scale: 0 -> 1; opacity: 0 -> 1;",
        );
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            assert!(s.contains("opacity: 0"), "Should have opacity, got: {}", s);
            // Transforms should be composed into single declaration
            assert!(
                s.contains("transform:"),
                "Should have transform declaration, got: {}",
                s
            );
            assert!(
                s.contains("translateY(20px)"),
                "Should have translateY, got: {}",
                s
            );
            assert!(s.contains("scale(0)"), "Should have scale, got: {}", s);
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    #[test]
    fn test_initial_state_css_skips_control_props() {
        let css = generate_initial_state_css(
            ".hero",
            "opacity: 0 -> 1; easing: ease-out; range: 0 to 0.5; stagger: 0.1;",
        );
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            assert!(s.contains("opacity: 0"), "Should have opacity");
            assert!(
                !s.contains("easing"),
                "Should not contain easing, got: {}",
                s
            );
            assert!(!s.contains("range"), "Should not contain range, got: {}", s);
            assert!(
                !s.contains("stagger"),
                "Should not contain stagger, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    #[test]
    fn test_initial_state_css_filter_composition() {
        let css = generate_initial_state_css(".hero", "blur: 10px -> 0; brightness: 0.5 -> 1;");
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            assert!(
                s.contains("filter:"),
                "Should have filter declaration, got: {}",
                s
            );
            assert!(s.contains("blur(10px)"), "Should have blur, got: {}", s);
            assert!(
                s.contains("brightness(0.5)"),
                "Should have brightness, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    #[test]
    fn test_initial_state_css_self_selector() {
        let css = generate_initial_state_css(".hero", "&self { opacity: 0 -> 1; }");
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            // &self should resolve to just the parent selector, not ".hero &self"
            assert!(
                s.contains(".hero"),
                "Should contain parent selector, got: {}",
                s
            );
            assert!(
                !s.contains("&self"),
                "Should not contain raw &self, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }

    #[test]
    fn test_initial_state_css_multiple_scopes() {
        let css = generate_initial_state_css(
            ".hero",
            ".title { opacity: 0 -> 1; } .subtitle { translate-y: 30px -> 0; }",
        );
        assert!(!css.is_empty());
        if let CssExpr::Raw(s) = &css[0] {
            assert!(
                s.contains(".hero .title"),
                "Should contain first scope, got: {}",
                s
            );
            assert!(
                s.contains(".hero .subtitle"),
                "Should contain second scope, got: {}",
                s
            );
            assert!(s.contains("opacity: 0"), "Should have opacity, got: {}", s);
            assert!(
                s.contains("translateY(30px)"),
                "Should have translateY, got: {}",
                s
            );
        } else {
            panic!("Expected CssExpr::Raw");
        }
    }
}
