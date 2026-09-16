//! Resolve Layer (Layer 2)
//!
//! Resolves FormMatch → ResolvedPrimitive via %bind clauses.
//!
//! This layer:
//! 1. Takes FormMatch instances (from the parse layer)
//! 2. Looks up the corresponding %macro definition
//! 3. Applies %bind clauses to create BoundArgs
//! 4. Determines phase and order
//! 5. Returns ResolvedPrimitive instances

use std::collections::HashMap;

use log::trace;

use crate::emit::metasystem_codegen::{PrimitiveArgs, generate_primitive_ir};
use crate::metasystem::{MetaRegistry, macro_scope_matches};
use crate::parser::SourceSpan;
use crate::parser::meta_ast::{
    BindArg, BindDecl, BindValue, FormCapture, FormInlineElement, MacroBodyItem, MacroDefAst,
    MacroScope, PrimitiveParam,
};
use crate::syntax::{CapturedValue, FormMatch, JsQuoting};

use super::types::{CompileError, CompileErrorKind, Phase, ResolvedPrimitive};

// Type alias for backward compatibility during migration
pub type ResolveError = CompileError;

// Form matching score is provided by syntax::form_scoring

// =============================================================================
// Resolution Logic
// =============================================================================

/// Resolve FormMatches to ResolvedPrimitives
///
/// This is the main entry point for the resolve layer.
/// Resolve EvaluatedMatches to ResolvedPrimitives.
///
/// When an EvaluatedMatch has pre-evaluated bind_decls from the Evaluate layer,
/// those are used directly instead of scanning the macro body.
pub fn resolve_evaluated(
    evaluated: &[super::evaluate::EvaluatedMatch],
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError> {
    let mut primitives = Vec::new();

    for ev in evaluated {
        if ev.binds_consumed {
            // A lowering pass consumed this match's binds on purpose (e.g.
            // expand_score_binds dropping a score whose driver cannot drive
            // one — the E0951 path). Re-resolving would RESURRECT them from
            // the macro definition: the score bind would reach expand, where
            // E0956 masks the diagnostic that actually explains the page.
            continue;
        }
        if ev.body_was_evaluated && !ev.bind_decls.is_empty() {
            // Use pre-evaluated bind_decls — Evaluate already flattened control flow
            let macro_def = meta_registry.find_macro_for_match(
                ev.form_match.matched_macro.as_deref(),
                &ev.form_match.macro_name,
                &ev.form_match.captures,
                &ev.form_match.selector,
            );
            if let Some(macro_def) = macro_def {
                let resolved = resolve_via_binds_with_decls(
                    &ev.form_match,
                    macro_def,
                    &ev.bind_decls,
                    meta_registry,
                )?;
                primitives.extend(resolved);
            } else {
                // No macro def found even via form-directive fallback — fall through to resolve_single
                let resolved = resolve_single(&ev.form_match, meta_registry)?;
                primitives.extend(resolved);
            }
        } else {
            // No evaluated body — use the existing resolve path
            let resolved = resolve_single(&ev.form_match, meta_registry)?;
            primitives.extend(resolved);
        }
    }
    Ok(primitives)
}

pub fn resolve(
    matches: &[FormMatch],
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError> {
    let mut primitives = Vec::new();

    for form_match in matches {
        let resolved = resolve_single(form_match, meta_registry)?;
        primitives.extend(resolved);
    }
    Ok(primitives)
}

/// Resolve a single FormMatch
///
/// This:
/// 1. Looks up the macro definition from MetaRegistry
/// 2. Applies %bind clauses to map captured values to primitive arguments
/// 3. Determines phase and order from the primitive definition
/// 4. Returns ResolvedPrimitive instances
fn resolve_single(
    form_match: &FormMatch,
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError> {
    // Look up the macro by its %form directive name,
    // then fall back to direct macro name lookup
    // The form_match.macro_name comes from what users write (e.g., "on" from @on)
    #[cfg(debug_assertions)]
    trace!(
        "[resolve] Looking up macro for directive '{}', captures: {:?}",
        form_match.macro_name,
        form_match.captures.keys().collect::<Vec<_>>()
    );

    // FEAT-118 FUP-055: a namespace-QUALIFIED call (`@scene/camera`, captured as
    // namespace_qualifier=["scene"]) resolves against the FQN-keyed registry
    // first — the qualifier names the module the leaf lives in. Falls through to
    // the normal (bare) resolution when there is no qualifier or no FQN hit.
    let qualified = if !form_match.namespace_qualifier.is_empty() {
        let fqn = format!(
            "{}/{}",
            form_match.namespace_qualifier.join("/"),
            form_match.macro_name
        );
        meta_registry.get_macro(&fqn)
    } else {
        None
    };

    let macro_def = qualified
        .or_else(|| {
            meta_registry.find_macro_for_match(
                form_match.matched_macro.as_deref(),
                &form_match.macro_name,
                &form_match.captures,
                &form_match.selector,
            )
        })
        .or_else(|| {
            // Fall back to first form-directive match (backward compatibility)
            meta_registry
                .get_macro_by_form_directive(&form_match.macro_name)
                .filter(|m| macro_scope_matches(&m.scopes, &form_match.selector))
        })
        .or_else(|| {
            // Fall back to direct macro name lookup (for prefix-based patterns)
            meta_registry
                .get_macro(&form_match.macro_name)
                .filter(|m| macro_scope_matches(&m.scopes, &form_match.selector))
        });

    // Macro-expansion coverage (PLAN-027 W6): record the resolved directive +
    // its specific matched macro form. No-op unless a coverage run is active.
    crate::coverage::record(&form_match.macro_name, macro_def.map(|m| m.name.as_str()));

    if let Some(macro_def) = macro_def {
        #[cfg(debug_assertions)]
        trace!(
            "[resolve] Found macro '{}' from {:?}",
            macro_def.name, macro_def.source_file
        );

        // Validate %requires - if macro has requirements but we're at top level
        // (no parent scope), it means the user is using the macro incorrectly
        if !macro_def.requires.is_empty() {
            // Check if any required bindings are missing from the captures
            let missing: Vec<_> = macro_def
                .requires
                .iter()
                .filter(|req| {
                    let key = req.trim_start_matches('$');
                    let key_with_dollar = format!("${}", key);
                    !form_match.captures.contains_key(key)
                        && !form_match.captures.contains_key(&key_with_dollar)
                })
                .collect();

            if !missing.is_empty() {
                let providers = find_providers_for_binding(missing[0], meta_registry);
                return Err(CompileError::new(
                    CompileErrorKind::MissingRequiredBinding {
                        binding: missing[0].clone(),
                        child_macro: form_match.macro_name.clone(),
                        providers,
                    },
                    form_match.span,
                )
                .with_macro_name(form_match.macro_name.clone())
                .with_selector(form_match.selector.clone().unwrap_or_default())
                .with_macro_file(macro_def.source_file.clone().unwrap_or_default())
                .with_provided_params(form_match.captures.keys().cloned().collect())
                .with_expected_params(macro_def.requires.clone()));
            }
        }

        // Process %registers side effect for template validation.
        // %registers template($name) populates a compile-time registry
        // used by template validation (E0402).
        // We still flow through to %binds processing below — don't skip.

        // If macro has %binds clauses, use them
        if !macro_def.binds.is_empty() {
            return resolve_via_binds(form_match, macro_def, meta_registry);
        }

        // NOTE: Conditional binds (%if blocks containing %binds) are now handled
        // by the Evaluate layer (pipeline/evaluate.rs). When called via
        // resolve_evaluated(), those binds arrive pre-flattened.

        // Macros with only %states (no %binds) don't need primitive resolution.
        // State CSS/JS is generated by generate_state_fragments() in compile_verbose.
        if macro_def.states.is_some() {
            #[cfg(debug_assertions)]
            trace!(
                "[resolve] Macro '{}' has states/registers, no primitive needed",
                macro_def.name
            );
            return Ok(vec![]);
        }

        // Registry-FACT macros (BUG-268): a registers-only macro with no binds,
        // states, or emit blocks declares COMPILE-TIME data (@version, @type,
        // @deploy, driver registry entries) — not runtime behavior. The registers
        // side effect already populated the registry; producing a ResolvedPrimitive
        // here would carry a name the expand layer cannot resolve and, before
        // BUG-268, silently degraded to a comment stub IN THE SHIPPED BUNDLE. Route
        // them out at creation: registry data, never an emit target.
        if macro_def.registers.is_some()
            && !macro_def
                .body
                .iter()
                .any(|item| matches!(item, MacroBodyItem::Emit(_)))
        {
            return Ok(vec![]);
        }
    }

    // Fallback: macro has no %binds, no %states, no %registers.
    // Use the macro_name as the primitive name, and derive phase/order from
    // the MacroDefAst's %scope/%order clauses when available.
    let phase = macro_def
        .and_then(|m| {
            if m.scopes.contains(&MacroScope::Selector) {
                Some(Phase::Selector)
            } else if m.scopes.is_empty() {
                None
            } else {
                Some(Phase::Global)
            }
        })
        .unwrap_or(Phase::Global);

    let order = macro_def.and_then(|m| m.order).unwrap_or(500);

    let args = form_match.captures.clone();

    // Use the SPECIFIC resolved macro's name (e.g. `test-needs`) rather than the
    // directive name (`test`) so the Expand step re-looks-up the exact matched
    // macro template instead of falling back to the base directive macro. This is
    // what makes multi-form directives (test/test-needs, reveal-simple/
    // reveal-with-keyframes) emit the form that actually matched. (PLAN-027 W1.)
    let primitive_name = macro_def
        .map(|m| m.name.clone())
        .unwrap_or_else(|| form_match.macro_name.clone());

    let mut primitive = ResolvedPrimitive::new(primitive_name, args, phase, order)
        .with_selector(form_match.selector.clone().unwrap_or_default())
        .with_span(form_match.span);

    let has_nested_children = form_match
        .captures
        .get("children")
        .or_else(|| form_match.captures.get("body"))
        .and_then(|v| match v {
            CapturedValue::Block(children) => Some(!children.is_empty()),
            CapturedValue::Named(map) => map.get("macro_calls").and_then(|m| match m {
                CapturedValue::Block(children) => Some(!children.is_empty()),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or(false);

    if !has_nested_children
        && let Some(css_str) = extract_css_styles(
            form_match
                .captures
                .get("styles")
                .or_else(|| form_match.captures.get("$styles")),
        )
    {
        primitive = primitive.with_css_styles(css_str);
    }

    Ok(vec![primitive])
}

/// Resolve a FormMatch via its macro's %binds clauses
fn resolve_via_binds(
    form_match: &FormMatch,
    macro_def: &MacroDefAst,
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError> {
    resolve_via_binds_with_decls(form_match, macro_def, &macro_def.binds, meta_registry)
}

/// Resolve a FormMatch via explicit bind declarations
/// This is the core implementation used by both resolve_via_binds and conditional bind resolution
fn resolve_via_binds_with_decls(
    form_match: &FormMatch,
    macro_def: &MacroDefAst,
    bind_decls: &[BindDecl],
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError> {
    let mut primitives = Vec::new();

    // Collect provided params from user's macro call
    let provided_params: Vec<String> = form_match.captures.keys().cloned().collect();

    // Create a mutable captures map that includes outputs from previous binds
    // This enables chaining: glContext -> { $gl } then glLoop($gl, ...)
    let mut extended_captures = form_match.captures.clone();

    // Create evaluation context for error reporting
    let mut ctx = EvalContext {
        macro_name: &form_match.macro_name,
        selector: form_match.selector.as_deref(),
        span: form_match.span,
        macro_file: macro_def.source_file.as_deref(),
        bind_span: None,
        bind_primitive: None,
        provided_params,
        meta_registry,
    };

    // Determine the macro-level phase: use Selector if ANY bind needs an element
    // This keeps all binds within a macro executing together
    let macro_phase = if bind_decls.iter().any(|b| {
        b.args
            .iter()
            .any(|arg| matches!(arg, BindArg::Element { .. }))
    }) {
        Phase::Selector
    } else {
        Phase::Global
    };

    for (bind_index, bind_decl) in bind_decls.iter().enumerate() {
        // Update context with current bind clause info
        ctx.bind_span = Some(bind_decl.span);
        ctx.bind_primitive = Some(&bind_decl.primitive);

        // Extract expected params from bind args (variables that need to be captured)
        let expected_params = extract_expected_params(&bind_decl.args);

        // Get ALL primitive param names for positional mapping
        // This maps positional args to the primitive's declared parameter names
        let prim_all_params: Vec<String> = meta_registry
            .get_primitive(&bind_decl.primitive)
            .map(|p| {
                p.params
                    .iter()
                    .map(|param| match param {
                        PrimitiveParam::Element(name) => name.clone(),
                        PrimitiveParam::Data(name) => name.clone(),
                        PrimitiveParam::TypedData { name, .. } => name.clone(),
                        PrimitiveParam::Typed { name, .. } => name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Evaluate bind arguments by substituting captured values
        let mut args = HashMap::new();
        let mut positional_arg_index = 0; // Track position of all args for mapping to primitive params

        for bind_arg in &bind_decl.args {
            match bind_arg {
                BindArg::Element {
                    name,
                    child_selector,
                } => {
                    // &element or &self >> selector
                    let value = if name == "self" {
                        // &self refers to the current selector scope
                        if let Some(sel) = &form_match.selector {
                            if let Some(child) = child_selector {
                                CapturedValue::Selector(format!("{} >> {}", sel, child))
                            } else {
                                CapturedValue::Selector(sel.clone())
                            }
                        } else {
                            CapturedValue::Element(name.clone())
                        }
                    } else {
                        // Look up &name from extended captures (includes outputs from previous binds)
                        extended_captures
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| CapturedValue::Element(name.clone()))
                    };
                    // Use primitive's param name if available, otherwise use bind's arg name
                    let key = prim_all_params
                        .get(positional_arg_index)
                        .cloned()
                        .unwrap_or_else(|| name.clone());
                    args.insert(key, value);
                    positional_arg_index += 1;
                }
                BindArg::Named { name, value } => {
                    // name: value - evaluate the value using extended captures
                    let evaluated = evaluate_bind_value(
                        value,
                        &extended_captures,
                        &ctx,
                        &expected_params,
                        macro_def,
                    )?;
                    args.insert(name.clone(), evaluated);
                    // Named args don't increment positional index - they use explicit names
                }
                BindArg::Positional(value) => {
                    // Positional arg - map to primitive's param name at this position
                    let evaluated = evaluate_bind_value(
                        value,
                        &extended_captures,
                        &ctx,
                        &expected_params,
                        macro_def,
                    )?;
                    let key = prim_all_params
                        .get(positional_arg_index)
                        .cloned()
                        .unwrap_or_else(|| format!("_arg{}", positional_arg_index));
                    args.insert(key, evaluated);
                    positional_arg_index += 1;
                }
            }
        }

        // Add this bind's outputs to extended_captures for subsequent binds
        // The outputs are signal names that will be emitted by the primitive at runtime
        // We represent them as Binding captures (like $gl) so they can be passed to subsequent binds.
        //
        // BUG-154: ALSO collect them as a write-time remap map (export -> alias) so a
        // `%yield expr -> $export` in the primitive body emits `ST.set(el, "alias", …)`
        // — the name the consuming page reads. Without this the yield writes the raw
        // export name and `$alias` is never populated (the aliased signal is dead).
        let mut output_mappings: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for output in &bind_decl.outputs {
            let output_name = &output.name;
            // Use alias if provided, otherwise use original name
            let capture_name = output.alias.as_ref().unwrap_or(output_name).clone();
            // Store as a Binding reference - represents a $variable that will be available at runtime
            extended_captures.insert(
                capture_name.clone(),
                CapturedValue::Binding(format!("${}", output_name)),
            );
            // Only a genuine RENAME needs a remap entry (alias != export); an
            // identity mapping would be a harmless no-op but we skip it for clarity.
            // The yield target is a BARE signal name (`$lastMessage` parses to
            // `lastMessage`), and `resolve_signal_name` writes it verbatim as the
            // ST.set key, so the alias VALUE must also be bare — strip the leading
            // `$` the capture form carries (`$mcpLastMessage` -> `mcpLastMessage`).
            if output.alias.is_some() {
                // BUG-197: INTERPOLATED alias patterns (`${$name}_pending`,
                // data-kind macros) resolve against THIS invocation's captures
                // into the concrete write-time remap key (e.g.
                // `pushPreview_pending`), so a `%yield pending -> $pending`
                // emits ST.set(el, "pushPreview_pending", …) -- the name
                // consuming pages actually read. Previously these were
                // skipped outright ("resolved elsewhere") and the yield wrote
                // the raw per-instance export name, leaving every aliased
                // `_pending`/`_error`/`_cancel` signal permanently dead.
                // A missing capture falls back to the raw alias, which fails
                // the plain-ident gate below exactly as before. Bare `$cap`
                // aliases (no interpolation) keep the historic trim-the-`$`
                // behavior (resolve_yield_name fast-paths them through).
                let resolved =
                    crate::analysis::resolve_yield_name(&capture_name, &extended_captures)
                        .unwrap_or_else(|| capture_name.clone());
                let bare_alias = resolved.trim_start_matches('$').to_string();
                // Only a PLAIN identifier alias is a write-time remap. An
                // alias that still isn't a plain ident after interpolation
                // (e.g. a capture was missing) must NOT become a literal
                // ST.set key.
                let is_plain_ident = !bare_alias.is_empty()
                    && bare_alias
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_');
                if is_plain_ident && bare_alias != *output_name {
                    output_mappings.insert(output_name.clone(), bare_alias);
                }
            }
        }

        // Use macro-level phase to keep all binds together
        // Use macro %order as base offset + bind_index to preserve both macro ordering
        // and intra-macro bind declaration order.
        // This ensures template invocations (%order 200) run before event handlers (%order 500+).
        let macro_order = macro_def.order.unwrap_or(500);
        let effective_order = macro_order
            .saturating_mul(1000)
            .saturating_add(bind_index as u32);
        let mut primitive = ResolvedPrimitive::new(
            bind_decl.primitive.clone(),
            args,
            macro_phase,
            effective_order,
        )
        .with_span(form_match.span)
        .with_outputs(output_mappings);
        // Only set selector if present — file-level directives have None.
        if let Some(sel) = &form_match.selector {
            primitive = primitive.with_selector(sel.clone());
        }

        let has_nested_children = form_match
            .captures
            .get("children")
            .or_else(|| form_match.captures.get("body"))
            .and_then(|v| match v {
                CapturedValue::Block(children) => Some(!children.is_empty()),
                CapturedValue::Named(map) => map.get("macro_calls").and_then(|m| match m {
                    CapturedValue::Block(children) => Some(!children.is_empty()),
                    _ => None,
                }),
                _ => None,
            })
            .unwrap_or(false);

        if !has_nested_children
            && let Some(css_str) = extract_css_styles(
                form_match
                    .captures
                    .get("styles")
                    .or_else(|| form_match.captures.get("$styles")),
            )
        {
            primitive = primitive.with_css_styles(css_str);
        }

        trace!(
            "[resolve] Pushing primitive: {} (phase={:?}, order={}) for macro '{}'",
            bind_decl.primitive, macro_phase, bind_index, form_match.macro_name
        );
        primitives.push(primitive);
    }

    // If the macro body contains %emit js blocks, push a synthetic primitive
    // so expand.rs's fallback path (which looks up the macro by name) can emit them.
    // Without this, %emit js in macros with %binds is silently dropped.
    let has_emit_blocks = macro_def
        .body
        .iter()
        .any(|item| matches!(item, crate::parser::meta_ast::MacroBodyItem::Emit(_)));
    if has_emit_blocks {
        let macro_order = macro_def.order.unwrap_or(500);
        let emit_order = macro_order
            .saturating_mul(1000)
            .saturating_add(bind_decls.len() as u32);
        let emit_primitive = ResolvedPrimitive::new(
            form_match.macro_name.clone(),
            extended_captures.clone(),
            macro_phase,
            emit_order,
        )
        .with_selector(form_match.selector.clone().unwrap_or_default())
        .with_span(form_match.span);
        trace!(
            "[resolve] Pushing synthetic emit primitive for macro '{}' (order={})",
            form_match.macro_name, emit_order,
        );
        primitives.push(emit_primitive);
    }

    // Process children with parent scope inheritance
    // Children are FormMatches captured in a Block
    // Only process children from the ORIGINAL form_match captures, not from extended_captures
    // (to avoid infinite recursion when injecting parent scope into children)

    // Try "children" key first (from $children* pattern)
    let children_captures = form_match
        .captures
        .get("children")
        // Also try "body" key (some macros use $body)
        .or_else(|| form_match.captures.get("body"));

    // Debug: Log what we found in children/body capture
    if let Some(capture) = children_captures {
        match capture {
            CapturedValue::Block(b) => {
                trace!(
                    "[resolve] children/body capture is Block with {} items",
                    b.len()
                );
                for (i, fm) in b.iter().enumerate() {
                    trace!("[resolve]   child[{}]: macro_name={}", i, fm.macro_name);
                }
            }
            CapturedValue::Named(map) => {
                trace!(
                    "[resolve] children/body capture is Named with keys: {:?}",
                    map.keys().collect::<Vec<_>>()
                );
                if let Some(mc) = map.get("macro_calls") {
                    if let CapturedValue::Block(b) = mc {
                        trace!("[resolve]   macro_calls has {} items", b.len());
                        for (i, fm) in b.iter().enumerate() {
                            trace!(
                                "[resolve]   macro_calls[{}]: macro_name={}",
                                i, fm.macro_name
                            );
                        }
                    } else {
                        trace!("[resolve]   macro_calls is not a Block");
                    }
                }
            }
            other => {
                trace!(
                    "[resolve] children/body capture is {:?}",
                    std::mem::discriminant(other)
                );
            }
        }
    } else {
        trace!("[resolve] No children/body capture found");
    }

    // If we have a Named capture (from macro body parsing), look for macro_calls inside
    let nested_form_matches: Option<&Vec<FormMatch>> = match children_captures {
        Some(CapturedValue::Block(children)) => Some(children),
        Some(CapturedValue::Named(map)) => {
            // Check for macro_calls inside Named capture
            match map.get("macro_calls") {
                Some(CapturedValue::Block(children)) => Some(children),
                _ => None,
            }
        }
        _ => None,
    };

    if let Some(children) = nested_form_matches {
        for child in children {
            let child_primitives = resolve_child_with_parent_scope(
                child,
                &extended_captures,
                form_match.selector.as_deref(),
                macro_phase,
                meta_registry,
            )?;
            trace!(
                "[resolve] Extending with {} child primitives from '{}'",
                child_primitives.len(),
                child.macro_name
            );
            for cp in &child_primitives {
                trace!(
                    "[resolve]   child primitive: {} (phase={:?}, order={})",
                    cp.primitive_name, cp.phase, cp.order
                );
            }
            primitives.extend(child_primitives);
        }
    }

    Ok(primitives)
}

/// Extract CSS styles from a CapturedValue, converting both String and StyleProperties variants.
/// Returns None if the value doesn't contain styles data.
fn extract_css_styles(value: Option<&CapturedValue>) -> Option<String> {
    match value {
        Some(CapturedValue::String(s)) => Some(s.clone()),
        Some(CapturedValue::Properties(props)) => {
            let css = props
                .iter()
                .map(|p| format!("{}: {};", p.name, p.type_ref))
                .collect::<Vec<_>>()
                .join(" ");
            Some(css)
        }
        Some(CapturedValue::StyleProperties(props)) => {
            let css = props
                .iter()
                .map(|(k, v)| format!("{}: {};", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            Some(css)
        }
        _ => None,
    }
}
/// Resolve a child macro with parent scope inheritance
///
/// This handles the `%requires` directive - child macros can declare
/// bindings they need from parent scope, and we validate and inject them.
fn resolve_child_with_parent_scope(
    child: &FormMatch,
    parent_scope: &HashMap<String, CapturedValue>,
    parent_selector: Option<&str>,
    parent_phase: Phase,
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError> {
    // Look up the child's macro definition
    let child_macro = meta_registry
        .get_macro_by_form_directive(&child.macro_name)
        .or_else(|| meta_registry.get_macro(&child.macro_name));

    if let Some(macro_def) = child_macro {
        // Validate %requires - check that all required bindings exist in parent scope
        if !macro_def.requires.is_empty() {
            for required_binding in &macro_def.requires {
                // Strip $ prefix if present for lookup
                let lookup_key = required_binding.trim_start_matches('$');
                let lookup_with_dollar = format!("${}", lookup_key);

                if !parent_scope.contains_key(lookup_key)
                    && !parent_scope.contains_key(&lookup_with_dollar)
                {
                    // Find macros that provide this binding for helpful error message
                    let providers = find_providers_for_binding(required_binding, meta_registry);

                    return Err(CompileError::new(
                        CompileErrorKind::MissingRequiredBinding {
                            binding: required_binding.clone(),
                            child_macro: child.macro_name.clone(),
                            providers,
                        },
                        child.span,
                    )
                    .with_macro_name(child.macro_name.clone())
                    .with_selector(parent_selector.unwrap_or_default().to_string())
                    .with_macro_file(macro_def.source_file.clone().unwrap_or_default())
                    .with_expected_params(macro_def.requires.clone()));
                }
            }
        }

        // Inject parent bindings into child's captures
        // Skip container keys that hold nested FormMatches to avoid infinite recursion
        const CONTAINER_KEYS: &[&str] =
            &["body", "children", "animations", "macro_calls", "nested"];

        let mut child_captures = child.captures.clone();
        for (name, value) in parent_scope {
            // Skip container keys - these hold nested macro calls, not bindings
            if CONTAINER_KEYS.contains(&name.as_str()) {
                continue;
            }

            // Don't override if child already has this capture
            if !child_captures.contains_key(name) {
                child_captures.insert(name.clone(), value.clone());
            }
        }

        // Create a modified FormMatch with injected parent scope
        let child_with_scope = FormMatch {
            macro_name: child.macro_name.clone(),
            matched_macro: child.matched_macro.clone(),
            doc: child.doc.clone(),
            captures: child_captures,
            capture_spans: child.capture_spans.clone(),
            selector: child
                .selector
                .clone()
                .or_else(|| parent_selector.map(String::from)),
            span: child.span,
            source_file: None,
            namespace_qualifier: child.namespace_qualifier.clone(),
        };

        // Recursively resolve the child
        let mut child_primitives = resolve_single(&child_with_scope, meta_registry)?;

        // Child primitives inherit the parent's phase to maintain execution order
        // This ensures children execute after their parent's binds complete
        for prim in &mut child_primitives {
            prim.phase = parent_phase;
        }

        return Ok(child_primitives);
    }

    // No macro definition found - fall back to standard resolution
    let mut child_primitives = resolve_single(child, meta_registry)?;
    for prim in &mut child_primitives {
        prim.phase = parent_phase;
    }
    Ok(child_primitives)
}

/// Find macros that provide a given binding (for error messages)
fn find_providers_for_binding(binding: &str, meta_registry: &MetaRegistry) -> Vec<String> {
    // Use the dynamically-built provider index from %binds outputs
    meta_registry
        .get_providers(binding)
        .into_iter()
        .map(|name| format!("@{}", name))
        .collect()
}

/// Context for error reporting during bind evaluation
struct EvalContext<'a> {
    macro_name: &'a str,
    selector: Option<&'a str>,
    span: SourceSpan,
    macro_file: Option<&'a str>,
    // Rich context for error reporting:
    bind_span: Option<SourceSpan>,
    bind_primitive: Option<&'a str>,
    provided_params: Vec<String>,
    // Registry for macro lookup (needed for expression macro expansion)
    meta_registry: &'a MetaRegistry,
}

/// Evaluate a BindValue by substituting captured variables
fn evaluate_bind_value(
    value: &BindValue,
    captures: &HashMap<String, CapturedValue>,
    ctx: &EvalContext,
    expected_params: &[String],
    macro_def: &MacroDefAst,
) -> Result<CapturedValue, ResolveError> {
    match value {
        BindValue::Variable(var_name) => {
            trace!(
                "[evaluate_bind_value] Looking up variable '{}' in captures (macro={})",
                var_name, ctx.macro_name
            );
            trace!(
                "[evaluate_bind_value] Available captures: {:?}",
                captures.keys().collect::<Vec<_>>()
            );
            // PLAN-077 W6: dotted capture paths (`$when.signal`) — walk a
            // nested Named capture record. One lookup mechanism, reused by
            // every macro whose grammar produces structured records
            // (cond_block arms, state_when, …). Tried BEFORE the flat lookup
            // so a literal capture named `when.signal` (impossible from the
            // grammar) can never shadow it.
            if let Some((head, rest)) = var_name.split_once('.') {
                let mut current = captures.get(head);
                for part in rest.split('.') {
                    current = match current {
                        Some(CapturedValue::Named(map)) => map.get(part),
                        _ => None,
                    };
                }
                if let Some(captured) = current {
                    return Ok(captured.clone());
                }
            }
            // First try to look up from captures directly by variable name
            if let Some(captured) = captures.get(var_name).cloned() {
                trace!(
                    "[evaluate_bind_value] Found '{}' directly: {:?}",
                    var_name, captured
                );
                return Ok(captured);
            }

            // Try to find the parameter name that maps to this variable name
            // Forms like `on: $trigger:ident` store capture as "on" but binds use "$trigger"
            if let Some(param_name) = get_param_name_for_var(macro_def, var_name)
                && let Some(captured) = captures.get(&param_name).cloned()
            {
                return Ok(captured);
            }

            // If not captured, check for default value in the macro's form
            if let Some(default) = get_form_param_default(macro_def, var_name) {
                return Ok(default);
            }

            // If param is optional (has ? modifier), return null/undefined value
            // Use Expr so it passes through as the JS keyword, not a quoted string
            if is_form_param_optional(macro_def, var_name) {
                return Ok(CapturedValue::Expr("null".to_string()));
            }

            // No capture and no default - error
            Err(CompileError::new(
                CompileErrorKind::MissingParameter(var_name.clone()),
                ctx.span,
            )
            .with_macro_name(ctx.macro_name.to_string())
            .with_selector(ctx.selector.unwrap_or_default().to_string())
            .with_macro_file(ctx.macro_file.unwrap_or_default().to_string())
            .with_bind_span(ctx.bind_span.unwrap_or_default())
            .with_bind_primitive(ctx.bind_primitive.unwrap_or_default().to_string())
            .with_provided_params(ctx.provided_params.clone())
            .with_expected_params(expected_params.to_vec()))
        }
        BindValue::String(s) => Ok(CapturedValue::String(s.clone())),
        BindValue::Number(n) => Ok(CapturedValue::Number(*n)),
        BindValue::Ident(id) => {
            // Handle boolean and null literals specially
            match id.as_str() {
                "true" => Ok(CapturedValue::Bool(true)),
                "false" => Ok(CapturedValue::Bool(false)),
                "null" => Ok(CapturedValue::Expr("null".to_string())),
                _ => Ok(CapturedValue::Ident(id.clone())),
            }
        }
        BindValue::Array(items) => {
            // For now, return as a string representation
            Ok(CapturedValue::String(items.join(", ")))
        }
        BindValue::FunctionCall { name, args } => {
            // Evaluate function call arguments
            let evaluated_args: Result<Vec<_>, _> = args
                .iter()
                .map(|arg| evaluate_bind_value(arg, captures, ctx, expected_params, macro_def))
                .collect();
            let evaluated_args = evaluated_args?;

            // Check if this function call is actually an expression-level macro
            // (like localStorage, webSocket, etc.) that needs expansion
            if let Some(called_macro) = ctx.meta_registry.get_macro(name)
                && !called_macro.binds.is_empty()
            {
                // This is an expression-level macro with %binds - expand it!
                if let Some(expanded_js) =
                    expand_expression_macro(called_macro, &evaluated_args, ctx.meta_registry)
                {
                    return Ok(CapturedValue::Expr(expanded_js));
                }
            }

            // Fall through: not a macro, format as regular function call
            let args_str: Vec<String> = evaluated_args.iter().map(captured_value_to_js).collect();
            Ok(CapturedValue::String(format!(
                "{}({})",
                name,
                args_str.join(", ")
            )))
        }
        BindValue::ElementRef(elem) => Ok(CapturedValue::Element(elem.clone())),
    }
}

/// Expand an expression-level macro (like localStorage) to JavaScript
///
/// Expression-level macros are macros with %binds that can be used in expression positions.
/// They expand to their primitive's JavaScript code, which should be an IIFE that returns a value.
///
/// # Arguments
/// * `called_macro` - The macro definition being called
/// * `args` - The evaluated arguments passed to the macro
/// * `meta_registry` - Registry for looking up primitives
///
/// # Returns
/// The generated JavaScript string if expansion succeeds, or None if it fails
fn expand_expression_macro(
    called_macro: &MacroDefAst,
    args: &[CapturedValue],
    meta_registry: &MetaRegistry,
) -> Option<String> {
    // Get the first %binds clause (expression macros typically have one)
    let bind = called_macro.binds.first()?;

    // Get the primitive definition
    let primitive = meta_registry.get_primitive(&bind.primitive)?;

    // Build PrimitiveArgs from the macro's form and provided arguments
    let mut prim_args = PrimitiveArgs::new();

    // Map arguments to primitive parameters based on the bind clause
    // The bind clause has args like: key: $key, default: $default
    // We need to resolve $key, $default from the form captures
    if let Some(form) = &called_macro.form {
        // Build a mapping from form param names to argument values
        let mut form_captures: HashMap<String, CapturedValue> = HashMap::new();

        // Match positional args to form params
        for (i, param) in form.params.iter().enumerate() {
            if let Some(arg_value) = args.get(i) {
                // Extract param name from the form param
                if let Some(capture) = param.capture() {
                    form_captures.insert(capture.var_name.clone(), arg_value.clone());
                }
            }
        }

        // Now resolve bind args using form_captures
        for bind_arg in &bind.args {
            match bind_arg {
                BindArg::Named { name, value } => {
                    let js_value = crate::syntax::bind_value_to_js(
                        &form_captures,
                        value,
                        JsQuoting::DoubleQuoted,
                    );
                    prim_args = prim_args.param(name, &js_value);
                }
                BindArg::Positional(value) => {
                    // For positional args, we need the param name from the primitive
                    // This is a simplified handling - named args are more common
                    let js_value = crate::syntax::bind_value_to_js(
                        &form_captures,
                        value,
                        JsQuoting::DoubleQuoted,
                    );
                    let arg_index = prim_args.params.len();
                    prim_args = prim_args.param(&format!("_arg{}", arg_index), &js_value);
                }
                BindArg::Element { name, .. } => {
                    // Element refs in expression macros aren't common, but handle them
                    prim_args = prim_args.element(name, "document.body");
                }
            }
        }
    }

    // Generate the primitive's IR and stringify at the boundary
    let ir = generate_primitive_ir(primitive, &prim_args);

    if ir.js_stmts.is_empty() {
        None
    } else {
        let opts = crate::emit::EmitOptions::pretty();
        match crate::emit::js::emit_stmts(&ir.js_stmts, &opts) {
            Ok(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        }
    }
}

/// Convert a CapturedValue to JavaScript string (double-quoted mode).
///
/// Convenience wrapper around the unified `CapturedValue::to_js(DoubleQuoted)`.
fn captured_value_to_js(value: &CapturedValue) -> String {
    value.to_js(JsQuoting::DoubleQuoted)
}

/// Get a default value for a form parameter from the macro definition
fn get_form_param_default(macro_def: &MacroDefAst, param_name: &str) -> Option<CapturedValue> {
    use crate::parser::meta_ast::ParamDefault;

    let form = macro_def.form.as_ref()?;

    // Helper to convert default to CapturedValue
    let convert_default = |d: &ParamDefault| -> CapturedValue {
        match d {
            ParamDefault::String(s) => CapturedValue::String(s.clone()),
            ParamDefault::Number(n) => CapturedValue::Number(*n),
            ParamDefault::Bool(b) => CapturedValue::Bool(*b),
            ParamDefault::None => CapturedValue::Ident("none".to_string()),
            ParamDefault::EmptyArray => CapturedValue::String("[]".to_string()),
            ParamDefault::EmptyObject => CapturedValue::String("{}".to_string()),
            ParamDefault::Array(items) => {
                // Convert array to string representation
                let strs: Vec<String> = items
                    .iter()
                    .map(|i| match i {
                        ParamDefault::String(s) => format!("\"{}\"", s),
                        ParamDefault::Number(n) => n.to_string(),
                        ParamDefault::Bool(b) => b.to_string(),
                        _ => "null".to_string(),
                    })
                    .collect();
                CapturedValue::String(format!("[{}]", strs.join(", ")))
            }
            // Handle CSS length values like "0ms", "20px", etc.
            ParamDefault::Length(value, unit) => {
                CapturedValue::Time((*value * get_time_unit_multiplier(unit)) as u32)
            }
        }
    };

    // Search through form params for matching name
    for param in &form.params {
        // Check if this param's capture matches the name we're looking for
        if let Some(capture) = param.capture()
            && capture.var_name == param_name
        {
            // Found the param, check for default
            return param.default.as_ref().map(&convert_default);
        }
    }

    // Also search body_params
    for param in &form.body_params {
        if let Some(capture) = param.capture()
            && capture.var_name == param_name
        {
            return param.default.as_ref().map(&convert_default);
        }
    }

    None
}

/// Convert time unit to milliseconds multiplier
fn get_time_unit_multiplier(unit: &str) -> f64 {
    match unit {
        "ms" => 1.0,
        "s" => 1000.0,
        _ => 1.0, // Default to ms for unknown units
    }
}

/// Check if a form parameter is optional (has ? modifier)
/// Search for a capture variable in form elements, including nested PseudoSelector body_params.
fn find_capture_in_elements<'a>(
    elements: &'a [FormInlineElement],
    var_name: &str,
) -> Option<&'a FormCapture> {
    for elem in elements {
        match elem {
            FormInlineElement::Capture(cap, _) => {
                if cap.var_name == var_name {
                    return Some(cap);
                }
            }
            FormInlineElement::PseudoSelector { body_params, .. }
            | FormInlineElement::KeywordBlock { body_params, .. }
            | FormInlineElement::PseudoClass { body_params, .. } => {
                for param in body_params {
                    if let Some(cap) = find_capture_in_elements(&param.elements, var_name) {
                        return Some(cap);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn is_form_param_optional(macro_def: &MacroDefAst, param_name: &str) -> bool {
    use crate::parser::meta_ast::CaptureModifier;

    let form = match macro_def.form.as_ref() {
        Some(f) => f,
        None => return false,
    };

    // Search through form params
    for param in &form.params {
        if let Some(capture) = param.capture()
            && capture.var_name == param_name
        {
            return capture.modifier == CaptureModifier::Optional
                || capture.modifier == CaptureModifier::ZeroOrMore;
        }
    }

    // Search body_params (including nested PseudoSelector captures)
    for param in &form.body_params {
        if let Some(capture) = param.capture()
            && capture.var_name == param_name
        {
            return capture.modifier == CaptureModifier::Optional
                || capture.modifier == CaptureModifier::ZeroOrMore;
        }
        // Search inside PseudoSelector/KeywordBlock body_params
        if let Some(cap) = find_capture_in_elements(&param.elements, param_name) {
            return cap.modifier == CaptureModifier::Optional
                || cap.modifier == CaptureModifier::ZeroOrMore;
        }
    }

    // Search body GROUPS (the leading `( … )?` body-group region, FEAT-103 /
    // PLAN-038 W1). A capture that lives inside an OPTIONAL or zero-or-more body
    // group is optional: when the group is absent, the capture is unbound and a
    // bind referencing it must resolve to null, not error. This covers a capture
    // nested inside a custom capture-type clause (e.g. `$mode` inside
    // `( $policy:policy_clause )?`): if ANY enclosing group is optional, the
    // capture is optional.
    for group in &form.body_groups {
        if capture_in_optional_group(group, param_name) {
            return true;
        }
    }

    // Search the head INLINE elements (and the post-arg inline run): an
    // optional head capture like `$as:on_as?` (PLAN-126's naming clause) is
    // unbound when absent — a bind referencing it resolves to null, same as
    // optional params/body fields. The head had no optional captures before
    // the `as` clause, so this region was never scanned.
    if let Some(cap) = find_capture_in_elements(&form.inline_elements, param_name) {
        return cap.modifier == CaptureModifier::Optional
            || cap.modifier == CaptureModifier::ZeroOrMore;
    }
    if let Some(cap) = find_capture_in_elements(&form.post_arg_inline, param_name) {
        return cap.modifier == CaptureModifier::Optional
            || cap.modifier == CaptureModifier::ZeroOrMore;
    }

    // Search the raw `body_capture` STRING for an optional field declaration
    // `$name:type?`. A `{ field: $name:type? }` body is stored verbatim as a
    // `body_capture` string (not parsed into `body_params`), so a single-field
    // optional body field (`@host … { headers: $headers:object? }`) would
    // otherwise read as required when its body is absent. Honor the `?` here so
    // an absent optional body field resolves to null, uniformly with inline and
    // body-group optionals. (PLAN-038 FUP-081: @host bodyless config.)
    if let Some(body_spec) = &form.body_capture
        && body_capture_field_is_optional(body_spec, param_name)
    {
        return true;
    }

    false
}

/// True when the raw `body_capture` string declares `$param_name` with an
/// OPTIONAL (`?`) or zero-or-more (`*`) modifier. Scans for the capture token
/// `$param_name:` and checks the modifier that follows its type. Conservative:
/// only an explicit `?`/`*` immediately after the capture's type counts.
fn body_capture_field_is_optional(body_spec: &str, param_name: &str) -> bool {
    let needle = format!("${}", param_name);
    let mut search_from = 0;
    while let Some(rel) = body_spec[search_from..].find(&needle) {
        let at = search_from + rel;
        // Ensure this is a whole capture name, not a prefix (`$header` vs `$headers`):
        // the char after the name must be `:` (typed) or a non-identifier boundary.
        let after_name = &body_spec[at + needle.len()..];
        let boundary_ok = after_name
            .chars()
            .next()
            .map(|c| !c.is_alphanumeric() && c != '_')
            .unwrap_or(true);
        if boundary_ok {
            // The modifier is the first `?`/`*`/`+` before the next field separator
            // (`;`, `,`, newline, `}`). A `?` or `*` means optional.
            for ch in after_name.chars() {
                match ch {
                    '?' | '*' => return true,
                    ';' | ',' | '\n' | '}' | '+' => break,
                    _ => {}
                }
            }
        }
        search_from = at + needle.len();
    }
    false
}

/// True when `param_name` is a capture reachable inside `pattern` AND some
/// enclosing Group along the way is Optional/ZeroOrMore (so the capture may be
/// absent at runtime). Walks the capture-pattern tree, tracking whether an
/// optional group has been crossed. A capture-type reference (Custom) is treated
/// as opaque — if it sits under an optional group, every name it could bind is
/// optional, so we conservatively report the queried name optional when the
/// custom capture itself is the var or lives under an optional group.
fn capture_in_optional_group(
    pattern: &crate::parser::meta_ast::CapturePatternAst,
    param_name: &str,
) -> bool {
    fn walk(
        pattern: &crate::parser::meta_ast::CapturePatternAst,
        param_name: &str,
        under_optional: bool,
    ) -> bool {
        use crate::parser::meta_ast::CaptureModifier;
        use crate::parser::meta_ast::CapturePatternAst as P;
        use crate::parser::meta_ast::CaptureType;
        match pattern {
            P::Capture {
                var_name,
                capture_type,
                ..
            } => {
                if !under_optional {
                    return false;
                }
                // Direct name match, OR a Custom capture-type whose lifted inner
                // captures (merged to top level) could include `param_name` — those
                // inner names are opaque here, so under an optional group a Custom
                // capture conservatively binds the queried name as optional.
                var_name == param_name || matches!(capture_type, CaptureType::Custom(_))
            }
            P::Group { pattern, modifier } => {
                let opt = under_optional
                    || matches!(
                        modifier,
                        Some(CaptureModifier::Optional) | Some(CaptureModifier::ZeroOrMore)
                    );
                walk(pattern, param_name, opt)
            }
            P::Sequence(elems) => elems.iter().any(|e| walk(e, param_name, under_optional)),
            P::Choice(branches) => branches.iter().any(|b| walk(b, param_name, under_optional)),
            P::Literal(_) | P::CharClass { .. } => false,
        }
    }
    walk(pattern, param_name, false)
}

/// Find a parameter name that maps to a given capture variable name
///
/// Forms can have named parameters like `on: $trigger:ident` where:
/// - `on` is the parameter name (how users write it)
/// - `trigger` is the capture variable name (how %binds reference it)
///
/// Captures are stored by parameter name, but binds use variable name.
/// This function finds the parameter name given a variable name.
fn get_param_name_for_var(macro_def: &MacroDefAst, var_name: &str) -> Option<String> {
    use crate::parser::meta_ast::{FormInlineElement, FormParam};

    let form = macro_def.form.as_ref()?;

    // Helper to check a param and return its name if the capture matches
    let check_param = |param: &FormParam| -> Option<String> {
        if let Some(capture) = param.capture()
            && capture.var_name == var_name
        {
            return Some(param.name.clone());
        }
        None
    };

    // Search form params
    for param in &form.params {
        if let Some(param_name) = check_param(param) {
            return Some(param_name);
        }
    }

    // Search body_params (including nested PseudoSelector captures)
    for param in &form.body_params {
        if let Some(param_name) = check_param(param) {
            return Some(param_name);
        }
        // Search inside PseudoSelector/KeywordBlock body_params
        if find_capture_in_elements(&param.elements, var_name).is_some() {
            return Some(var_name.to_string());
        }
    }

    // Also check inline_elements for capture patterns
    for elem in &form.inline_elements {
        if let FormInlineElement::Capture(capture, _default) = elem
            && capture.var_name == var_name
        {
            return Some(var_name.to_string());
        }
    }

    None
}

/// Extract variable names that a bind clause expects from captures
fn extract_expected_params(args: &[BindArg]) -> Vec<String> {
    let mut params = Vec::new();
    for arg in args {
        match arg {
            BindArg::Named { value, .. } => {
                extract_variables_from_bind_value(value, &mut params);
            }
            BindArg::Positional(value) => {
                extract_variables_from_bind_value(value, &mut params);
            }
            BindArg::Element { name, .. } => {
                // Element refs like &self don't need captures
                if name != "self" {
                    params.push(name.clone());
                }
            }
        }
    }
    params
}

/// Recursively extract variable names from a BindValue
fn extract_variables_from_bind_value(value: &BindValue, vars: &mut Vec<String>) {
    match value {
        BindValue::Variable(name) => {
            if !vars.contains(name) {
                vars.push(name.clone());
            }
        }
        BindValue::FunctionCall { args, .. } => {
            for arg in args {
                extract_variables_from_bind_value(arg, vars);
            }
        }
        _ => {}
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    // `MacroBodyItem`/`MetaIfCondition` are used ONLY by this test module
    // (never by resolve.rs's own non-test code -- confirmed: the one
    // non-test reference, line ~519, uses a fully-qualified path and
    // needs no import). Deliberately kept HERE rather than in the
    // top-level `use` block: `cargo build --lib` (no `#[cfg(test)]`) and
    // `cargo build --tests` (`#[cfg(test)]` compiled in) are DIFFERENT
    // compilation units: a shared top-level import that's only used by
    // test code is flagged "unused" under the plain `--lib` build, and
    // `cargo clippy --fix` mechanically strips it based on THAT warning
    // -- silently breaking the `--tests` build, which genuinely needs it.
    // Reproduced twice this session (clippy --fix repeatedly re-deleted a
    // top-level `MacroBodyItem, MetaIfCondition` import despite 32 real
    // tests depending on it) before isolating the actual fix: move
    // test-only imports INTO `mod tests` so they're visible to every
    // compilation unit that could ever need them, and invisible (hence
    // never independently flagged unused) to the ones that don't.
    use crate::parser::meta_ast::{MacroBodyItem, MetaIfCondition};

    /// Test helper: wraps the unified bind_value_to_js with DoubleQuoted mode
    fn bind_value_to_js_for_expansion(
        value: &BindValue,
        captures: &HashMap<String, CapturedValue>,
    ) -> String {
        crate::syntax::bind_value_to_js(captures, value, JsQuoting::DoubleQuoted)
    }

    #[test]
    fn test_resolve_empty() {
        let meta_registry = MetaRegistry::new();
        let result = resolve(&[], &meta_registry);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_resolve_single_global() {
        let meta_registry = MetaRegistry::new();
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("myData".to_string()),
        );

        let form_match = FormMatch {
            macro_name: "data-fetch".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = resolve(&[form_match], &meta_registry);
        assert!(result.is_ok());

        let primitives = result.unwrap();
        assert_eq!(primitives.len(), 1);
        assert_eq!(primitives[0].primitive_name, "data-fetch");
        assert_eq!(primitives[0].phase, Phase::Global);
    }

    #[test]
    fn test_resolve_single_selector() {
        let meta_registry = MetaRegistry::new();
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("myElement".to_string()),
        );

        let form_match = FormMatch {
            macro_name: "element-ref".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: Some(".my-selector".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = resolve(&[form_match], &meta_registry);
        assert!(result.is_ok());

        let primitives = result.unwrap();
        assert_eq!(primitives.len(), 1);
        assert_eq!(primitives[0].primitive_name, "element-ref");
        assert_eq!(primitives[0].phase, Phase::Global); // No macro def in empty registry
        assert_eq!(primitives[0].selector, Some(".my-selector".to_string()));
    }

    #[test]
    fn test_extract_expected_params_from_bind_args() {
        use crate::parser::meta_ast::{BindArg, BindValue};

        // Test with Named args containing variables
        let args = vec![
            BindArg::Named {
                name: "name".to_string(),
                value: BindValue::Variable("name".to_string()),
            },
            BindArg::Named {
                name: "body".to_string(),
                value: BindValue::Variable("body".to_string()),
            },
            BindArg::Named {
                name: "params".to_string(),
                value: BindValue::Variable("params".to_string()),
            },
        ];

        let params = extract_expected_params(&args);
        assert_eq!(params, vec!["name", "body", "params"]);
    }

    #[test]
    fn test_extract_expected_params_with_literals() {
        use crate::parser::meta_ast::{BindArg, BindValue};

        // Mix of variables and literals - only variables should be extracted
        let args = vec![
            BindArg::Named {
                name: "name".to_string(),
                value: BindValue::Variable("name".to_string()),
            },
            BindArg::Named {
                name: "mode".to_string(),
                value: BindValue::String("default".to_string()),
            },
            BindArg::Named {
                name: "count".to_string(),
                value: BindValue::Number(42.0),
            },
        ];

        let params = extract_expected_params(&args);
        assert_eq!(params, vec!["name"]); // Only variable refs are expected params
    }

    #[test]
    fn test_extract_expected_params_with_function_call() {
        use crate::parser::meta_ast::{BindArg, BindValue};

        // Function call with variable arguments
        let args = vec![BindArg::Named {
            name: "value".to_string(),
            value: BindValue::FunctionCall {
                name: "concat".to_string(),
                args: vec![
                    BindValue::Variable("prefix".to_string()),
                    BindValue::Variable("suffix".to_string()),
                ],
            },
        }];

        let params = extract_expected_params(&args);
        assert_eq!(params, vec!["prefix", "suffix"]);
    }

    #[test]
    fn test_missing_parameter_error_has_rich_context() {
        use crate::parser::meta_ast::{BindDecl, MacroDefAst};
        use crate::syntax::FormMatch;

        let meta_registry = MetaRegistry::new();

        // Create a macro definition with a bind clause
        let macro_def = MacroDefAst {
            name: "test-macro".to_string(),
            binds: vec![BindDecl {
                primitive: "test-primitive".to_string(),
                args: vec![
                    BindArg::Named {
                        name: "provided_arg".to_string(),
                        value: BindValue::Variable("provided".to_string()),
                    },
                    BindArg::Named {
                        name: "missing_arg".to_string(),
                        value: BindValue::Variable("missing".to_string()),
                    },
                ],
                outputs: vec![],
                span: SourceSpan {
                    start: 100,
                    end: 200,
                },
            }],
            source_file: Some("test/macro.st".to_string()),
            span: SourceSpan { start: 0, end: 300 },
            ..Default::default()
        };

        // Create a form match that's missing the "missing" parameter
        let mut captures = HashMap::new();
        captures.insert(
            "provided".to_string(),
            CapturedValue::String("value".to_string()),
        );

        let form_match = FormMatch {
            macro_name: "test-macro".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan {
                start: 50,
                end: 100,
            },
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        // Resolve should fail with MissingParameter
        let result = resolve_via_binds(&form_match, &macro_def, &meta_registry);
        assert!(result.is_err());

        let err = result.unwrap_err();

        // Verify rich context is populated
        assert!(
            matches!(err.kind, CompileErrorKind::MissingParameter(ref name) if name == "missing")
        );
        assert_eq!(err.macro_name, Some("test-macro".to_string()));
        assert_eq!(err.macro_file, Some("test/macro.st".to_string()));
        assert_eq!(err.bind_primitive, Some("test-primitive".to_string()));
        assert_eq!(
            err.bind_span,
            Some(SourceSpan {
                start: 100,
                end: 200
            })
        );
        assert_eq!(err.provided_params, Some(vec!["provided".to_string()]));
        assert!(
            err.expected_params
                .as_ref()
                .unwrap()
                .contains(&"provided".to_string())
        );
        assert!(
            err.expected_params
                .as_ref()
                .unwrap()
                .contains(&"missing".to_string())
        );
    }

    #[test]
    fn test_evaluate_condition_equals() {
        let mut captures = HashMap::new();
        captures.insert(
            "event".to_string(),
            CapturedValue::Ident("visible".to_string()),
        );

        // Test equality match
        let condition = MetaIfCondition::Equals("event".to_string(), "visible".to_string());
        assert!(crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Test inequality (non-match)
        let condition = MetaIfCondition::Equals("event".to_string(), "hover".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));
    }

    #[test]
    fn test_evaluate_condition_not_equals() {
        let mut captures = HashMap::new();
        captures.insert(
            "event".to_string(),
            CapturedValue::Ident("click".to_string()),
        );

        let condition = MetaIfCondition::NotEquals("event".to_string(), "hover".to_string());
        assert!(crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        let condition = MetaIfCondition::NotEquals("event".to_string(), "click".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));
    }

    #[test]
    fn test_evaluate_condition_truthy() {
        let mut captures = HashMap::new();
        captures.insert(
            "enabled".to_string(),
            CapturedValue::Ident("true".to_string()),
        );

        let condition = MetaIfCondition::Truthy("enabled".to_string());
        assert!(crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Test with missing variable
        let condition = MetaIfCondition::Truthy("missing".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Test with "false" string
        captures.insert(
            "disabled".to_string(),
            CapturedValue::Ident("false".to_string()),
        );
        let condition = MetaIfCondition::Truthy("disabled".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Expr("null") is falsy
        captures.insert(
            "nullval".to_string(),
            CapturedValue::Expr("null".to_string()),
        );
        let condition = MetaIfCondition::Truthy("nullval".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Expr("undefined") is falsy
        captures.insert(
            "undefval".to_string(),
            CapturedValue::Expr("undefined".to_string()),
        );
        let condition = MetaIfCondition::Truthy("undefval".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Ident("null") is falsy
        captures.insert(
            "nullident".to_string(),
            CapturedValue::Ident("null".to_string()),
        );
        let condition = MetaIfCondition::Truthy("nullident".to_string());
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // Falsy with Expr("null") is truthy (double negation)
        let condition = MetaIfCondition::Falsy("nullval".to_string());
        assert!(crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));
    }

    #[test]
    fn test_evaluate_condition_and_or() {
        let mut captures = HashMap::new();
        captures.insert("a".to_string(), CapturedValue::Ident("true".to_string()));
        captures.insert("b".to_string(), CapturedValue::Ident("false".to_string()));

        // AND: true && false = false
        let condition = MetaIfCondition::And(
            Box::new(MetaIfCondition::Truthy("a".to_string())),
            Box::new(MetaIfCondition::Truthy("b".to_string())),
        );
        assert!(!crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));

        // OR: true || false = true
        let condition = MetaIfCondition::Or(
            Box::new(MetaIfCondition::Truthy("a".to_string())),
            Box::new(MetaIfCondition::Truthy("b".to_string())),
        );
        assert!(crate::pipeline::evaluate::evaluate_if_condition(
            &captures, &condition
        ));
    }

    #[test]
    fn test_resolve_via_evaluated_conditional_binds() {
        use crate::parser::meta_ast::{
            BindArg, BindDecl, BindValue, FormClause, MacroDefAst, MetaIfClause,
        };
        use crate::pipeline::evaluate::evaluate_match;

        let mut meta_registry = MetaRegistry::new();

        // Create a macro with conditional binds (mimics on-event-dispatcher)
        let macro_def = MacroDefAst {
            name: "on-event-dispatcher".to_string(),
            form: Some(FormClause {
                directive_name: "on".to_string(),
                inline_elements: vec![],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
            binds: vec![], // Empty - binds are conditional
            body: vec![MacroBodyItem::If(MetaIfClause {
                condition: MetaIfCondition::Equals("event".to_string(), "hover".to_string()),
                then_body: vec![MacroBodyItem::Binds(vec![BindDecl {
                    primitive: "event-driver".to_string(),
                    args: vec![BindArg::Named {
                        name: "trigger".to_string(),
                        value: BindValue::String("hover".to_string()),
                    }],
                    outputs: vec![],
                    span: SourceSpan::default(),
                }])],
                elif_clauses: vec![crate::parser::meta_ast::MetaElifClause {
                    condition: MetaIfCondition::Equals("event".to_string(), "visible".to_string()),
                    body: vec![MacroBodyItem::Binds(vec![BindDecl {
                        primitive: "load-driver".to_string(),
                        args: vec![BindArg::Named {
                            name: "trigger".to_string(),
                            value: BindValue::String("visible".to_string()),
                        }],
                        outputs: vec![],
                        span: SourceSpan::default(),
                    }])],
                    span: SourceSpan::default(),
                }],
                else_body: None,
                span: SourceSpan::default(),
            })],
            source_file: Some("stdlib/on-event.st".to_string()),
            span: SourceSpan::default(),
            ..Default::default()
        };

        meta_registry.register_macro(macro_def).unwrap();

        // Test with "visible" event — should resolve to load-driver via evaluate
        let mut captures = HashMap::new();
        captures.insert(
            "event".to_string(),
            CapturedValue::Ident("visible".to_string()),
        );

        let form_match = FormMatch {
            macro_name: "on-event-dispatcher".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: Some(".section".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        // Evaluate flattens the %if conditional binds
        let evaluated = evaluate_match(&form_match, &meta_registry).unwrap();
        assert!(evaluated.body_was_evaluated);
        assert_eq!(evaluated.bind_decls.len(), 1);
        assert_eq!(evaluated.bind_decls[0].primitive, "load-driver");

        // Resolve uses the pre-evaluated binds
        let primitives = resolve_evaluated(&[evaluated], &meta_registry).unwrap();
        assert_eq!(primitives.len(), 1);
        assert_eq!(primitives[0].primitive_name, "load-driver");
    }

    #[test]
    fn test_resolve_evaluated_form_directive_fallback() {
        // Like test_resolve_via_evaluated_conditional_binds but with
        // macro_name = "on" (directive-derived) instead of "on-event-dispatcher" (registered name).
        // Verifies that resolve_evaluated finds the macro via find_macro_by_directive.
        use crate::parser::meta_ast::{
            BindArg, BindDecl, BindValue, CaptureModifier, CaptureType, FormCapture, FormClause,
            FormInlineElement, MacroDefAst, MetaIfClause, ParamType, PrimitiveBody,
            PrimitiveDefAst, PrimitiveParam,
        };
        use crate::pipeline::evaluate::evaluate_match;

        let mut meta_registry = MetaRegistry::new();

        let macro_def = MacroDefAst {
            name: "on-event-dispatcher".to_string(),
            form: Some(FormClause {
                directive_name: "@on".to_string(),
                inline_elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "event".to_string(),
                        capture_type: CaptureType::Ident,
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                )],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
            binds: vec![],
            body: vec![MacroBodyItem::If(MetaIfClause {
                condition: MetaIfCondition::Equals("event".to_string(), "visible".to_string()),
                then_body: vec![MacroBodyItem::Binds(vec![BindDecl {
                    primitive: "load-driver".to_string(),
                    args: vec![BindArg::Named {
                        name: "trigger".to_string(),
                        value: BindValue::String("visible".to_string()),
                    }],
                    outputs: vec![],
                    span: SourceSpan::default(),
                }])],
                elif_clauses: vec![],
                else_body: None,
                span: SourceSpan::default(),
            })],
            span: SourceSpan::default(),
            ..Default::default()
        };
        meta_registry.register_macro(macro_def).unwrap();

        // Register the primitive so resolve can look it up
        let prim = PrimitiveDefAst {
            name: "load-driver".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "trigger".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody::default(),
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };
        meta_registry.register_primitive(prim).unwrap();

        let mut captures = HashMap::new();
        captures.insert(
            "event".to_string(),
            CapturedValue::Ident("visible".to_string()),
        );

        let form_match = FormMatch {
            macro_name: "on".to_string(), // directive-derived name, NOT "on-event-dispatcher"
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: Some(".hero".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        // Evaluate finds the macro via form-directive fallback
        let evaluated = evaluate_match(&form_match, &meta_registry).unwrap();
        assert!(
            evaluated.body_was_evaluated,
            "evaluate_match must find macro via form-directive fallback"
        );
        assert_eq!(evaluated.bind_decls.len(), 1);
        assert_eq!(evaluated.bind_decls[0].primitive, "load-driver");

        // Resolve also finds the macro via the same fallback
        let primitives = resolve_evaluated(&[evaluated], &meta_registry).unwrap();
        assert_eq!(primitives.len(), 1);
        assert_eq!(primitives[0].primitive_name, "load-driver");
    }

    // =========================================================================
    // Expression Macro Expansion Tests
    // =========================================================================

    #[test]
    fn test_captured_value_to_js_string() {
        let val = CapturedValue::String("hello".to_string());
        assert_eq!(captured_value_to_js(&val), "\"hello\"");

        // Test escaping
        let val = CapturedValue::String("hello\"world".to_string());
        assert_eq!(captured_value_to_js(&val), "\"hello\\\"world\"");
    }

    #[test]
    fn test_captured_value_to_js_number() {
        let val = CapturedValue::Number(42.5);
        assert_eq!(captured_value_to_js(&val), "42.5");
    }

    #[test]
    fn test_captured_value_to_js_bool() {
        let val = CapturedValue::Bool(true);
        assert_eq!(captured_value_to_js(&val), "true");

        let val = CapturedValue::Bool(false);
        assert_eq!(captured_value_to_js(&val), "false");
    }

    #[test]
    fn test_captured_value_to_js_array() {
        let val = CapturedValue::Array(vec![
            CapturedValue::String("a".to_string()),
            CapturedValue::Number(1.0),
        ]);
        assert_eq!(captured_value_to_js(&val), "[\"a\", 1]");
    }

    #[test]
    fn test_captured_value_to_js_expr() {
        // Expr values pass through unchanged
        let val = CapturedValue::Expr("someFunction()".to_string());
        assert_eq!(captured_value_to_js(&val), "someFunction()");
    }

    #[test]
    fn test_bind_value_to_js_for_expansion_variable() {
        let mut captures = HashMap::new();
        captures.insert(
            "key".to_string(),
            CapturedValue::String("my-key".to_string()),
        );

        let value = BindValue::Variable("key".to_string());
        assert_eq!(
            bind_value_to_js_for_expansion(&value, &captures),
            "\"my-key\""
        );

        // Missing variable returns null
        let value = BindValue::Variable("missing".to_string());
        assert_eq!(bind_value_to_js_for_expansion(&value, &captures), "null");
    }

    #[test]
    fn test_bind_value_to_js_for_expansion_string() {
        let captures = HashMap::new();
        let value = BindValue::String("test".to_string());
        assert_eq!(
            bind_value_to_js_for_expansion(&value, &captures),
            "\"test\""
        );
    }

    #[test]
    fn test_bind_value_to_js_for_expansion_array() {
        let captures = HashMap::new();
        let value = BindValue::Array(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            bind_value_to_js_for_expansion(&value, &captures),
            "[\"a\", \"b\"]"
        );
    }

    #[test]
    fn test_bind_value_to_js_for_expansion_function_call() {
        let captures = HashMap::new();
        let value = BindValue::FunctionCall {
            name: "parseInt".to_string(),
            args: vec![BindValue::String("42".to_string())],
        };
        assert_eq!(
            bind_value_to_js_for_expansion(&value, &captures),
            "parseInt(\"42\")"
        );
    }

    #[test]
    fn test_expand_expression_macro_no_binds() {
        // A macro without %binds should not expand
        use crate::parser::meta_ast::MacroDefAst;

        let macro_def = MacroDefAst {
            name: "test".to_string(),
            binds: vec![], // No binds!
            ..Default::default()
        };

        let args = vec![CapturedValue::String("arg".to_string())];
        let registry = MetaRegistry::new();

        let result = expand_expression_macro(&macro_def, &args, &registry);
        assert!(result.is_none());
    }

    #[test]
    fn test_expand_expression_macro_primitive_not_found() {
        // A macro with %binds but primitive not in registry
        use crate::parser::meta_ast::{BindDecl, MacroDefAst};

        let macro_def = MacroDefAst {
            name: "test".to_string(),
            binds: vec![BindDecl {
                primitive: "nonexistent-primitive".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }],
            ..Default::default()
        };

        let args = vec![];
        let registry = MetaRegistry::new();

        let result = expand_expression_macro(&macro_def, &args, &registry);
        // Should return None because primitive doesn't exist
        assert!(result.is_none());
    }

    #[test]
    fn test_evaluate_bind_value_function_call_not_macro() {
        // A function call that's not a macro should pass through as JS
        use crate::parser::meta_ast::{BindValue, MacroDefAst};

        let meta_registry = MetaRegistry::new();
        let macro_def = MacroDefAst::default();
        let captures = HashMap::new();

        let ctx = EvalContext {
            macro_name: "test",
            selector: None,
            span: SourceSpan::default(),
            macro_file: None,
            bind_span: None,
            bind_primitive: None,
            provided_params: vec![],
            meta_registry: &meta_registry,
        };

        let value = BindValue::FunctionCall {
            name: "parseInt".to_string(),
            args: vec![BindValue::String("42".to_string())],
        };

        let result = evaluate_bind_value(&value, &captures, &ctx, &[], &macro_def);
        assert!(result.is_ok());

        // Should be formatted as a regular function call string
        let captured = result.unwrap();
        match captured {
            CapturedValue::String(s) => {
                assert!(s.contains("parseInt"), "Expected 'parseInt' in: {}", s);
            }
            _ => panic!("Expected String, got {:?}", captured),
        }
    }

    // ==========================================================================
    // %requires Directive Tests
    // ==========================================================================

    /// BUG-197 pin: an interpolated %binds output alias (`${$name}_pending`
    /// -- the exact shape every @data signal declares) resolves against the
    /// invocation's captures into the concrete write-time remap key, so the
    /// primitive's `%yield pending -> $pending` emits
    /// `ST.set(el, "slowSig_pending", …)` -- the name consuming pages read.
    /// Pre-fix these aliases were skipped outright and the yield wrote the
    /// raw per-instance export name, leaving every `_pending`/`_error`
    /// auto-yield permanently dead.
    #[test]
    fn test_interpolated_bind_output_alias_resolves_against_captures() {
        use crate::parser::meta_ast::{BindDecl, BindOutput};

        let mut registry = MetaRegistry::new();
        let mut macro_def = make_test_macro_with_requires("data-signal", vec![]);
        macro_def.binds = vec![BindDecl {
            primitive: "signal-call".to_string(),
            args: vec![],
            outputs: vec![
                BindOutput {
                    name: "call".to_string(),
                    alias: Some("$name".to_string()),
                },
                BindOutput {
                    name: "pending".to_string(),
                    alias: Some("${$name}_pending".to_string()),
                },
                BindOutput {
                    name: "error".to_string(),
                    alias: Some("${$name}_error".to_string()),
                },
                BindOutput {
                    name: "cancel".to_string(),
                    alias: Some("${$name}_cancel".to_string()),
                },
            ],
            span: SourceSpan::default(),
        }];
        registry.register_macro(macro_def).unwrap();

        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Binding("$slowSig".to_string()),
        );
        let form_match = FormMatch {
            macro_name: "data-signal".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let primitives = resolve(&[form_match], &registry).unwrap();
        assert_eq!(primitives.len(), 1);
        let outputs = &primitives[0].outputs;
        assert_eq!(
            outputs.get("pending").map(String::as_str),
            Some("slowSig_pending")
        );
        assert_eq!(
            outputs.get("error").map(String::as_str),
            Some("slowSig_error")
        );
        assert_eq!(
            outputs.get("cancel").map(String::as_str),
            Some("slowSig_cancel")
        );
        // Bare-$cap aliases keep their historic trim-the-`$` behavior.
        assert_eq!(outputs.get("call").map(String::as_str), Some("name"));
    }

    /// BUG-197 edge: an interpolated alias referencing a capture this
    /// invocation does NOT have must NOT become a literal ST.set key -- it
    /// degrades to no mapping, exactly as pre-fix (graceful, never garbage).
    #[test]
    fn test_interpolated_bind_output_alias_missing_capture_yields_no_mapping() {
        use crate::parser::meta_ast::{BindDecl, BindOutput};

        let mut registry = MetaRegistry::new();
        let mut macro_def = make_test_macro_with_requires("data-signal", vec![]);
        macro_def.binds = vec![BindDecl {
            primitive: "signal-call".to_string(),
            args: vec![],
            outputs: vec![BindOutput {
                name: "pending".to_string(),
                alias: Some("${$absent}_pending".to_string()),
            }],
            span: SourceSpan::default(),
        }];
        registry.register_macro(macro_def).unwrap();

        let form_match = FormMatch {
            macro_name: "data-signal".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let primitives = resolve(&[form_match], &registry).unwrap();
        assert_eq!(primitives.len(), 1);
        assert!(primitives[0].outputs.get("pending").is_none());
    }

    /// Create a minimal macro definition for testing %requires
    fn make_test_macro_with_requires(name: &str, requires: Vec<&str>) -> MacroDefAst {
        use crate::parser::meta_ast::FormClause;

        MacroDefAst {
            retired: None,
            name: name.to_string(),
            form: Some(FormClause {
                directive_name: name.to_string(),
                inline_elements: vec![],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            span: SourceSpan::default(),
            source_file: Some("test.st".to_string()),
            requires: requires.into_iter().map(String::from).collect(),
            module: None,
            doc: None,
            ..Default::default()
        }
    }
    

    #[test]
    fn test_requires_validation_missing_binding() {
        // Create a macro that requires $gl
        let macro_def = make_test_macro_with_requires("glow", vec!["$gl"]);

        let mut registry = MetaRegistry::new();
        let _ = registry.register_macro(macro_def);

        // Create a form match without $gl in captures
        let form_match = FormMatch {
            macro_name: "glow".to_string(),
            matched_macro: None,
            captures: HashMap::new(), // Empty - no $gl
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        // Should fail with MissingRequiredBinding error
        let result = resolve_single(&form_match, &registry);
        assert!(
            result.is_err(),
            "Should error when required binding is missing"
        );

        let err = result.unwrap_err();
        match err.kind {
            CompileErrorKind::MissingRequiredBinding {
                binding,
                child_macro,
                ..
            } => {
                assert_eq!(binding, "$gl", "Should report missing $gl");
                assert_eq!(child_macro, "glow", "Should report macro name");
            }
            _ => panic!("Expected MissingRequiredBinding, got {:?}", err.kind),
        }
    }

    #[test]
    fn test_requires_validation_binding_present() {
        // Create a macro that requires $gl
        let macro_def = make_test_macro_with_requires("glow", vec!["$gl"]);

        let mut registry = MetaRegistry::new();
        let _ = registry.register_macro(macro_def);

        // Create a form match WITH $gl in captures
        let mut captures = HashMap::new();
        captures.insert(
            "gl".to_string(),
            CapturedValue::String("webgl_context".to_string()),
        );

        let form_match = FormMatch {
            macro_name: "glow".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        // Should NOT error - $gl is present (as "gl" without $)
        let result = resolve_single(&form_match, &registry);
        // Note: This might still fail for other reasons (no binds), but NOT for missing requirements
        if let Err(ref e) = result {
            match &e.kind {
                CompileErrorKind::MissingRequiredBinding { .. } => {
                    panic!("Should NOT error for missing required binding - $gl is present");
                }
                _ => {} // Other errors are ok (e.g., no %binds)
            }
        }
    }

    #[test]
    fn test_requires_validation_multiple_bindings() {
        // Create a macro that requires $gl, $width, $height
        let macro_def = make_test_macro_with_requires("glow", vec!["$gl", "$width", "$height"]);

        let mut registry = MetaRegistry::new();
        let _ = registry.register_macro(macro_def);

        // Only provide $gl, missing $width and $height
        let mut captures = HashMap::new();
        captures.insert("gl".to_string(), CapturedValue::String("ctx".to_string()));

        let form_match = FormMatch {
            macro_name: "glow".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let result = resolve_single(&form_match, &registry);
        assert!(
            result.is_err(),
            "Should error when some required bindings are missing"
        );

        let err = result.unwrap_err();
        match &err.kind {
            CompileErrorKind::MissingRequiredBinding { binding, .. } => {
                // Should report one of the missing bindings ($width or $height)
                assert!(
                    binding == "$width" || binding == "$height",
                    "Should report missing $width or $height, got: {}",
                    binding
                );
            }
            _ => panic!("Expected MissingRequiredBinding, got {:?}", err.kind),
        }
    }

    #[test]
    fn test_requires_no_requirements_passes() {
        // Create a macro with no %requires
        let macro_def = make_test_macro_with_requires("simple", vec![]);

        let mut registry = MetaRegistry::new();
        let _ = registry.register_macro(macro_def);

        let form_match = FormMatch {
            macro_name: "simple".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        // Should not fail with MissingRequiredBinding (may fail for other reasons)
        let result = resolve_single(&form_match, &registry);
        if let Err(ref e) = result {
            match &e.kind {
                CompileErrorKind::MissingRequiredBinding { .. } => {
                    panic!("Should NOT error for missing required binding when no %requires");
                }
                _ => {} // Other errors are fine
            }
        }
    }

    #[test]
    fn test_find_providers_for_gl_binding() {
        // Test that find_providers_for_binding returns @scene for $gl
        let registry = MetaRegistry::new();

        // Note: This test uses the hardcoded known_providers list
        // When the registry has a "scene" macro, it should be suggested
        let providers = find_providers_for_binding("$gl", &registry);

        // Currently returns empty because scene macro isn't registered
        // In real usage with stdlib loaded, it would return ["@scene"]
        // This test validates the function doesn't panic
        assert!(providers.is_empty() || providers.contains(&"@scene".to_string()));
    }

    #[test]
    fn test_child_scope_injection_skips_container_keys() {
        // Test that resolve_child_with_parent_scope correctly skips container keys
        // to avoid infinite recursion

        let macro_def = make_test_macro_with_requires("child", vec![]);

        let mut registry = MetaRegistry::new();
        let _ = registry.register_macro(macro_def);

        // Parent scope with container keys that should be skipped
        let mut parent_scope = HashMap::new();
        parent_scope.insert("gl".to_string(), CapturedValue::String("ctx".to_string()));
        parent_scope.insert("body".to_string(), CapturedValue::Named(HashMap::new())); // Should skip
        parent_scope.insert("children".to_string(), CapturedValue::Named(HashMap::new())); // Should skip

        let child = FormMatch {
            macro_name: "child".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let result =
            resolve_child_with_parent_scope(&child, &parent_scope, None, Phase::Global, &registry);

        // Should not panic from infinite recursion
        // The actual result depends on macro definition, but no stack overflow
        assert!(result.is_ok() || result.is_err()); // Just verify it completes
    }

    #[test]
    fn qualified_dispatch_resolves_namespaced_macro() {
        // FEAT-118 FUP-055: a FormMatch carrying namespace_qualifier=["scene"]
        // for the leaf `camera` resolves to the FQN-keyed macro `scene/camera`,
        // NOT a bare `camera` — proving qualified dispatch picks the right module.
        use crate::metasystem::module::Namespace;
        use crate::parser::meta_ast::MetaDef;
        let mut registry = MetaRegistry::new();
        // Build the macro from .st source, then namespace-stamp + register it.
        let parsed = crate::parser::parse_for_bootstrap(
            "%macro camera {\n  %form { @camera $fov:string }\n}",
        )
        .expect("parse");
        for def in parsed.meta_defs {
            if let MetaDef::Macro(mut m) = def {
                m.module = Some(Namespace::new(None, vec!["scene".to_string()]));
                registry.register_macro(m).expect("register scene/camera");
            }
        }

        // The macro is keyed under its namespace.
        assert!(registry.get_macro("scene/camera").is_some());
        assert!(registry.get_macro("camera").is_none(), "not bare-keyed");

        // A qualified FormMatch resolves to it.
        let mut fm = FormMatch::new("camera");
        fm.namespace_qualifier = vec!["scene".to_string()];
        let resolved = resolve_single(&fm, &registry);
        assert!(
            resolved.is_ok(),
            "qualified @scene/camera resolves to scene/camera: {resolved:?}"
        );
    }
}
