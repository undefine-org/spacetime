//! Pipeline Evaluate Layer (Layer 0)
//!
//! Flattens compile-time control flow in macro body items before Resolve.
//!
//! Given a `FormMatch` and its macro definition, evaluates `%when`, `%for`,
//! `%if/%elif/%else` conditionals, `%includes` (recursive macro expansion),
//! and `%derives` (parent macro inheritance). Produces a flat list of
//! `BindDecl` entries (no control flow remaining) plus any `%emit` blocks
//! and `%states` clauses as side output.
//!
//! The pipeline order becomes: **Evaluate → Resolve → Sort → Expand → Emit**

use std::collections::HashMap;

use crate::metasystem::MetaRegistry;
use crate::parser::SourceSpan;
use crate::parser::meta_ast::*;
use crate::syntax::{CapturedValue, FormMatch};

use super::types::{CompileError, CompileErrorKind};

/// Maximum recursion depth for `%includes` expansion
const MAX_EVAL_DEPTH: usize = 100;

/// Result of evaluating a macro's body items.
///
/// All control flow (`%when`, `%for`, `%if`) has been resolved into flat lists.
/// A single input `FormMatch` with a `%for` over N items may produce N bind entries.
#[derive(Debug, Clone)]
pub struct EvaluatedMatch {
    /// The original FormMatch (selector, captures, macro_name, span)
    pub form_match: FormMatch,

    /// Flattened bind declarations after control flow evaluation.
    /// These replace `macro_def.binds` + any conditional binds in the body.
    /// If empty, Resolve falls back to the macro's top-level `%binds`.
    pub bind_decls: Vec<BindDecl>,

    /// Direct `%emit` blocks with parameters already substituted.
    /// These bypass primitive resolution and produce raw JS/CSS output.
    pub emit_blocks: Vec<EvaluatedEmit>,

    /// `%states` clause from the macro, if present.
    /// Produces CSS state selectors during Expand.
    pub states: Option<MetaStatesClause>,

    /// `%on $var -> value` transition watchers from the macro body, with their
    /// action statements' params already substituted (PLAN-053). Each produces a
    /// selector-scoped `ST.watch`-driven handler during fragment generation: when
    /// `$var` transitions to `value`, the actions run through the shared mutation
    /// rail. This is how `@drag { on-drop: ... }` fires its drop action.
    pub on_clauses: Vec<EvaluatedOn>,

    /// Reactive `%derives` lowered to runtime computed signals (PLAN-054). Only
    /// populated when the macro has a `%animates` clause (the derives feed it);
    /// compile-time-only derive users (each/repeat) are untouched. Each becomes
    /// an `ST.derive(el, name, deps, compute)` fragment.
    pub derives: Vec<EvaluatedDerive>,

    /// `%animates` channels lowered to a reactive transform binding (PLAN-054).
    /// Each maps a CSS transform property to the (derived) signal that drives it;
    /// emitted as one `ST.bindTransform` so channels compose into one transform.
    pub animates: Vec<EvaluatedAnimate>,

    /// Whether body evaluation contributed any items.
    /// When false, Resolve uses the macro's top-level `%binds` as before.
    pub body_was_evaluated: bool,

    /// A lowering pass CONSUMED this match's binds (e.g. expand_score_binds
    /// dropping a score whose driver cannot drive one — the E0951 path).
    /// Distinct from "flattened to nothing": an empty bind_decls with this
    /// false still falls back to the macro's top-level `%binds` (documented
    /// on `bind_decls`); with this true, Resolve must emit NOTHING — the
    /// binds are gone on purpose and re-resolving them from the macro would
    /// resurrect what the lowering deliberately removed.
    pub binds_consumed: bool,
}

/// A `%derives` declaration lowered to a runtime computed signal (PLAN-054).
#[derive(Debug, Clone)]
pub struct EvaluatedDerive {
    /// The derived signal name (no `$`), e.g. `x`.
    pub name: String,
    /// Element signal names this derive watches (gesture outputs or other
    /// derives). Captures are inlined as literals, not deps.
    pub deps: Vec<String>,
    /// The JS compute expression, with `$sig`->`v.sig`, captures inlined,
    /// `&self.rect`->`ST.rectOf(el)`, `clamp`->`ST.clamp`, `none`->`null`.
    pub compute_js: String,
}

/// A `%animates` channel lowered to a transform binding (PLAN-054).
#[derive(Debug, Clone)]
pub struct EvaluatedAnimate {
    /// The CSS transform property, e.g. `translate-x`, `scale`, `rotate`.
    pub property: String,
    /// The (derived) signal name driving it, e.g. `x`.
    pub signal: String,
}

/// An evaluated `%on $var -> value` transition watcher.
///
/// All param substitution is already applied to `actions` (so `$onDrop` has
/// become the caller's `on-drop:` expression). Emitted as a selector-scoped JS
/// fragment that subscribes to `var` and runs the actions on the transition.
#[derive(Debug, Clone)]
pub struct EvaluatedOn {
    /// The signal name to watch (no `$`), e.g. `active`.
    pub var: String,
    /// The target value the transition fires on, e.g. `false` / `true`.
    pub value: String,
    /// The action statements to run on transition (params substituted), each a
    /// `$sig(args)` / `$x <- expr` / effect line for the shared mutation rail.
    pub actions: Vec<String>,
}

/// An evaluated `%emit` block with parameters substituted.
#[derive(Debug, Clone)]
pub struct EvaluatedEmit {
    pub lang: EmitLang,
    pub content: String,
    pub span: SourceSpan,
}

/// Context for body item evaluation — lightweight alternative to MacroContext.
/// Uses the FormMatch captures as bindings, extended by `%for` loop variables.
struct EvalCtx<'a> {
    bindings: HashMap<String, CapturedValue>,
    registry: &'a MetaRegistry,
    depth: usize,
    span: SourceSpan,
}

impl<'a> EvalCtx<'a> {
    fn new(
        captures: &HashMap<String, CapturedValue>,
        registry: &'a MetaRegistry,
        span: SourceSpan,
    ) -> Self {
        Self {
            bindings: captures.clone(),
            registry,
            depth: 0,
            span,
        }
    }

    fn resolve(&self, name: &str) -> Option<&CapturedValue> {
        let key = name.trim_start_matches('$');
        self.bindings.get(key).or_else(|| self.bindings.get(name))
    }

    fn save_bindings(&self) -> HashMap<String, CapturedValue> {
        self.bindings.clone()
    }

    fn restore_bindings(&mut self, saved: HashMap<String, CapturedValue>) {
        self.bindings = saved;
    }

    fn set_binding(&mut self, name: String, value: CapturedValue) {
        let key = name.trim_start_matches('$').to_string();
        self.bindings.insert(key, value);
    }
}

/// Accumulated output from evaluating body items.
#[derive(Debug, Default)]
struct EvalOutput {
    bind_decls: Vec<BindDecl>,
    emit_blocks: Vec<EvaluatedEmit>,
    on_clauses: Vec<EvaluatedOn>,
    animates: Vec<EvaluatedAnimate>,
}

/// Evaluate a single FormMatch against its macro definition.
///
/// Processes macro body items to flatten control flow, producing an
/// `EvaluatedMatch` with concrete bind declarations and emit blocks.
/// Resolve `$customStates`-style [`MetaStateDef::Variable`] placeholders in a
/// macro's `%states` clause against the caller's captured states block.
///
/// A macro form like `@drag(...) { $customStates:states? … }` captures the
/// caller's `dragging { scale: 1.04 } over { background } …` as raw text under
/// the capture name (`customStates`). In the macro's `%states` body the author
/// writes `$customStates?` as a placeholder where those states should land. This
/// fn replaces each placeholder with the PARSED caller states, MERGING by name:
/// a caller body for a name the macro already declares with a `when <cond>` gate
/// (e.g. dnd's `over when $hovering && $accepting { }`) folds the caller's
/// PROPERTIES into that gated state — so `over { background: red }` at the call
/// site renders as `selector[data-st-state="over"] { background: red }` while
/// keeping the macro's reactive gate. Without this the placeholder was a silent
/// no-op (the caller's custom-state bodies never reached CSS).
fn resolve_state_variables(
    states: &MetaStatesClause,
    captures: &HashMap<String, CapturedValue>,
) -> MetaStatesClause {
    fn push_merging(out: &mut Vec<MetaStateDef>, def: MetaStateDef) {
        if let MetaStateDef::Named {
            name,
            condition,
            properties,
        } = &def
            && let Some(MetaStateDef::Named {
                condition: ex_cond,
                properties: ex_props,
                ..
            }) = out
                .iter_mut()
                .find(|d| matches!(d, MetaStateDef::Named { name: n, .. } if n == name))
        {
            // Keep the macro's gate if the caller body has none; append props.
            if ex_cond.is_none() {
                *ex_cond = condition.clone();
            }
            ex_props.extend(properties.clone());
            return;
        }
        out.push(def);
    }

    let mut out: Vec<MetaStateDef> = Vec::new();
    for def in &states.states {
        match def {
            MetaStateDef::Variable { name, .. } => {
                // Resolve the placeholder from the caller's captured states text.
                if let Some(CapturedValue::Expr(text)) = captures.get(name) {
                    for parsed in crate::parser::parse_meta_states_from_text(text) {
                        push_merging(&mut out, parsed);
                    }
                }
                // Absent capture (optional, not supplied) → placeholder drops.
            }
            other => push_merging(&mut out, other.clone()),
        }
    }

    MetaStatesClause {
        states: out,
        span: states.span,
    }
}

pub fn evaluate_match(
    form_match: &FormMatch,
    registry: &MetaRegistry,
) -> Result<EvaluatedMatch, CompileError> {
    let macro_def = registry.find_macro_for_match(
        form_match.matched_macro.as_deref(),
        &form_match.macro_name,
        &form_match.captures,
        &form_match.selector,
    );

    let mut result = EvaluatedMatch {
        form_match: form_match.clone(),
        bind_decls: Vec::new(),
        emit_blocks: Vec::new(),
        states: None,
        on_clauses: Vec::new(),
        derives: Vec::new(),
        animates: Vec::new(),
        body_was_evaluated: false,
        binds_consumed: false,
    };

    let Some(macro_def) = macro_def else {
        // No macro definition found — Resolve will handle the fallback
        return Ok(result);
    };

    // Include top-level %binds from the macro definition
    if !macro_def.binds.is_empty() {
        result.bind_decls.extend(macro_def.binds.clone());
        result.body_was_evaluated = true;
    }

    // If the macro has no body items and no derives, we're done.
    if macro_def.body.is_empty() && macro_def.derives.is_empty() {
        result.states = macro_def
            .states
            .as_ref()
            .map(|s| resolve_state_variables(s, &form_match.captures));
        return Ok(result);
    }

    let mut ctx = EvalCtx::new(&form_match.captures, registry, form_match.span);

    // Process %derives — bind derived variables
    for derive in &macro_def.derives {
        // For now, bind a placeholder (matches metasystem behavior)
        ctx.set_binding(derive.name.clone(), CapturedValue::Number(0.0));
    }

    // Evaluate body items (may add more bind_decls and emit_blocks)
    let mut output = EvalOutput::default();
    for item in &macro_def.body {
        evaluate_body_item(&mut ctx, item, &mut output)?;
    }

    // Lower %animates + the %derives that feed it into reactive signals (PLAN-054).
    // Only done when the macro actually animates; compile-time-only derive users
    // (each/repeat) keep the placeholder semantics above untouched.
    if !output.animates.is_empty() {
        let runtime_signals = collect_runtime_signals(macro_def);
        let derive_map: HashMap<String, &DeriveDecl> = macro_def
            .derives
            .iter()
            .map(|d| (d.name.clone(), d))
            .collect();
        // Lower each derive transitively reachable from an animated channel.
        let mut lowered: Vec<EvaluatedDerive> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for anim in &output.animates {
            lower_derive_chain(
                &anim.signal,
                &derive_map,
                &runtime_signals,
                &form_match.captures,
                &mut seen,
                &mut lowered,
            );
        }
        result.derives = lowered;
        result.animates = output.animates;
        result.body_was_evaluated = true;
    }

    if !output.bind_decls.is_empty()
        || !output.emit_blocks.is_empty()
        || !output.on_clauses.is_empty()
    {
        result.bind_decls.extend(output.bind_decls);
        result.emit_blocks = output.emit_blocks;
        result.on_clauses = output.on_clauses;
        result.body_was_evaluated = true;
    }

    result.states = macro_def
        .states
        .as_ref()
        .map(|s| resolve_state_variables(s, &form_match.captures));

    Ok(result)
}

/// Evaluate all FormMatches, returning EvaluatedMatches for the Resolve layer.
pub fn evaluate(
    matches: &[FormMatch],
    registry: &MetaRegistry,
) -> Result<Vec<EvaluatedMatch>, CompileError> {
    matches
        .iter()
        // PLAN-039 W3-clean: a match inside a `@template` body's nested scope is emitted
        // ONCE as a page-level `registerSelectorInit('.sel', init)` — there is no
        // double-emit. The dynamic-node observer (st.js) fires `init(instanceEl)` on every
        // matching node, including the `.sel` nodes the template factory creates per
        // instance; each `init` resolves its `$`-bind via an ancestor-walk against the
        // instance scope (resolveBoundDoc), and the factory seeds the param signals on the
        // instance root (template.st param-seed). One init definition, N instance mounts.
        .map(|fm| evaluate_match(fm, registry))
        .collect()
}

/// Recursively evaluate a single macro body item, collecting results into `output`.
fn evaluate_body_item(
    ctx: &mut EvalCtx,
    item: &MacroBodyItem,
    output: &mut EvalOutput,
) -> Result<(), CompileError> {
    match item {
        MacroBodyItem::When(when) => {
            // %when $condition { body } — include body if condition is truthy
            if let Some(value) = ctx.resolve(&when.condition)
                && value.is_truthy()
            {
                for body_item in &when.body {
                    evaluate_body_item(ctx, body_item, output)?;
                }
            }
        }

        MacroBodyItem::For(for_clause) => {
            // %for $var in $source { body } — unroll loop
            let list = ctx.resolve(&for_clause.source).cloned();
            if let Some(CapturedValue::Array(items)) = list {
                let saved = ctx.save_bindings();
                for item_value in items {
                    ctx.set_binding(for_clause.variable.clone(), item_value);
                    for body_item in &for_clause.body {
                        evaluate_body_item(ctx, body_item, output)?;
                    }
                }
                ctx.restore_bindings(saved);
            }
        }

        MacroBodyItem::If(if_clause) => {
            // %if condition { body } %elif ... %else { body }
            let condition_met = evaluate_if_condition(&ctx.bindings, &if_clause.condition);
            if condition_met {
                for body_item in &if_clause.then_body {
                    evaluate_body_item(ctx, body_item, output)?;
                }
            } else {
                // Check elif clauses
                let mut elif_matched = false;
                for elif_clause in &if_clause.elif_clauses {
                    if evaluate_if_condition(&ctx.bindings, &elif_clause.condition) {
                        for body_item in &elif_clause.body {
                            evaluate_body_item(ctx, body_item, output)?;
                        }
                        elif_matched = true;
                        break;
                    }
                }
                // Fall through to else
                if !elif_matched && let Some(else_body) = &if_clause.else_body {
                    for body_item in else_body {
                        evaluate_body_item(ctx, body_item, output)?;
                    }
                }
            }
        }

        MacroBodyItem::Binds(binds) => {
            // %binds { primitive calls } — collect for Resolve
            output.bind_decls.extend(binds.clone());
        }

        MacroBodyItem::Emit(emit_block) => {
            // %emit js/css { code } — substitute parameters and collect
            let content = substitute_params_simple(&ctx.bindings, &emit_block.content);
            output.emit_blocks.push(EvaluatedEmit {
                lang: emit_block.lang,
                content,
                span: emit_block.span,
            });
        }

        MacroBodyItem::Includes(includes) => {
            // %includes { @pattern(args) } — recursive macro evaluation
            if ctx.depth >= MAX_EVAL_DEPTH {
                return Err(CompileError::new(
                    CompileErrorKind::MaxDepthExceeded {
                        max_depth: MAX_EVAL_DEPTH,
                        stack: vec![],
                    },
                    ctx.span,
                ));
            }

            for pattern in &includes.patterns {
                let included_macro = ctx.registry.get_macro(&pattern.name);
                if let Some(macro_def) = included_macro {
                    // Build captures from included pattern args
                    let mut captures = ctx.bindings.clone();
                    for arg in &pattern.args {
                        // If the arg value is a variable reference, resolve it
                        let value = if arg.value.starts_with('$') {
                            ctx.resolve(&arg.value)
                                .cloned()
                                .unwrap_or_else(|| CapturedValue::String(arg.value.clone()))
                        } else {
                            CapturedValue::String(arg.value.clone())
                        };
                        captures.insert(arg.name.clone(), value);
                    }

                    let mut child_ctx = EvalCtx {
                        bindings: captures,
                        registry: ctx.registry,
                        depth: ctx.depth + 1,
                        span: includes.span,
                    };

                    // Process derives
                    for derive in &macro_def.derives {
                        child_ctx.set_binding(derive.name.clone(), CapturedValue::Number(0.0));
                    }

                    // Process included macro's top-level binds
                    output.bind_decls.extend(macro_def.binds.clone());

                    // Process included macro's body
                    for body_item in &macro_def.body {
                        evaluate_body_item(&mut child_ctx, body_item, output)?;
                    }
                }
            }
        }

        MacroBodyItem::On(on_clause) => {
            // %on $var -> value { actions } — a state-transition watcher (PLAN-053).
            // Only the VarTransition trigger is lowered here (the `$active -> false`
            // drop signal). Collect each action statement with params substituted,
            // so a macro-param reference like `$onDrop` becomes the caller's
            // `on-drop:` expression. Event/VarEvent triggers are left for their
            // existing handlers.
            if let MetaOnTrigger::VarTransition { var, value } = &on_clause.trigger {
                let mut actions = Vec::new();
                collect_on_actions(ctx, &on_clause.body, &mut actions);
                if !actions.is_empty() {
                    output.on_clauses.push(EvaluatedOn {
                        var: var.trim_start_matches('$').to_string(),
                        value: value.trim().to_string(),
                        actions,
                    });
                }
            }
        }

        MacroBodyItem::Animates(clause) => {
            // %animates { property: $signal } — reactive transform binding (PLAN-054).
            // Collect each channel; the derive-lowering after the body loop turns
            // the driving signals into ST.derive computed signals and emits one
            // ST.bindTransform composing the channels.
            for prop in &clause.properties {
                output.animates.push(EvaluatedAnimate {
                    property: prop.property.clone(),
                    signal: prop.variable.trim_start_matches('$').to_string(),
                });
            }
        }

        // These items don't produce binds or emits at the Evaluate level.
        // They're handled by other layers or are no-ops.
        MacroBodyItem::Mutate(_) | MacroBodyItem::Trigger(_) | MacroBodyItem::Applies(_) => {}
    }

    Ok(())
}

/// Collect the action statements of a `%on` body, with macro-param references
/// substituted from the evaluation context (PLAN-053).
///
/// Each body item becomes one or more statement strings for the shared mutation
/// rail (`ST.runMutations`):
///   - `Action("$onDrop")`  -> the caller's captured `on-drop:` expression
///     (a `$onDrop` whose value resolves to e.g. `$move({...})`). A bare action
///     that is NOT a known param passes through verbatim.
///   - `Assignment { var, expr }` -> `$var <- <expr>` (params substituted in expr).
/// An action that substitutes to empty (an optional param the caller omitted) is
/// dropped, so `on-drop:` being absent yields no statement.
fn collect_on_actions(ctx: &EvalCtx, body: &[MetaOnBodyItem], out: &mut Vec<String>) {
    for item in body {
        match item {
            MetaOnBodyItem::Action(action) => {
                let stmt = substitute_on_action(ctx, action);
                if !stmt.trim().is_empty() {
                    out.push(stmt);
                }
            }
            MetaOnBodyItem::Assignment { var, expr } => {
                let expr_sub = substitute_on_action(ctx, expr);
                if !var.trim().is_empty() {
                    out.push(format!("${} <- {}", var.trim_start_matches('$'), expr_sub));
                }
            }
            // Nested macro items / inline emits in an %on body are not part of the
            // transition-watcher lowering; they are handled elsewhere or unused.
            MetaOnBodyItem::MacroItem(_) | MetaOnBodyItem::Emit(_) => {}
        }
    }
}

/// Substitute macro-param references in a `%on` action statement.
///
/// A WHOLE-WORD `$name` that names a binding in scope is replaced by the
/// binding's emit string (so a bare `$onDrop` becomes the caller's action
/// expression). Names with no binding pass through unchanged — they are page
/// signals (`$move`) or the runtime `$` target resolved later by the mutation
/// rail. An optional param that resolved to nothing yields an empty string,
/// which the caller drops.
fn substitute_on_action(ctx: &EvalCtx, action: &str) -> String {
    let re = regex::Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let mut result = action.to_string();
    // Collect distinct param names first to avoid overlapping replacements.
    let names: Vec<String> = re.captures_iter(action).map(|c| c[1].to_string()).collect();
    for name in names {
        if let Some(value) = ctx.resolve(&name) {
            let replacement = captured_value_to_emit_string(value, None);
            // Whole-word replace of `$name` (not a prefix of a longer ident).
            // NB: pass a NoExpand replacement — the substituted value can itself
            // contain `$` (e.g. `$move(...)`), which regex would otherwise read as
            // a capture-group reference and drop.
            let pat = regex::Regex::new(&format!(r"\${}\b", regex::escape(&name))).unwrap();
            result = pat
                .replace_all(&result, regex::NoExpand(&replacement))
                .into_owned();
        }
    }
    result
}

// =============================================================================
// %derives + %animates lowering (PLAN-054)
// =============================================================================

/// Collect the set of RUNTIME signal names a macro's primitives export (gesture
/// $deltaX/$active/..., etc). A `%derives` expression that references one of
/// these depends on a live signal, so it must be lowered to a reactive
/// `ST.derive`. Names NOT in this set are compile-time captures (inlined as
/// literals) or other derives (resolved transitively).
fn collect_runtime_signals(macro_def: &MacroDefAst) -> std::collections::HashSet<String> {
    let mut sigs = std::collections::HashSet::new();
    for bind in &macro_def.binds {
        for out in &bind.outputs {
            // The exported name is the alias if present, else the raw name.
            let name = out.alias.clone().unwrap_or_else(|| out.name.clone());
            sigs.insert(name.trim_start_matches('$').to_string());
        }
    }
    sigs
}

/// Recursively lower a derive (and the derives it depends on) into
/// `EvaluatedDerive`s, in dependency order (deps first), de-duplicated.
///
/// `name` is the signal an animated channel drives. If it is a `%derives`
/// declaration, its expression is translated to JS and its derive-dependencies
/// lowered first. If it is a raw runtime signal (a gesture output) there is
/// nothing to lower (the signal already exists). A capture is not lowered either.
fn lower_derive_chain(
    name: &str,
    derive_map: &HashMap<String, &DeriveDecl>,
    runtime_signals: &std::collections::HashSet<String>,
    captures: &HashMap<String, CapturedValue>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<EvaluatedDerive>,
) {
    let key = name.trim_start_matches('$');
    if seen.contains(key) {
        return;
    }
    let Some(decl) = derive_map.get(key) else {
        // Not a derive: a raw runtime signal or capture — nothing to lower.
        return;
    };
    seen.insert(key.to_string());

    // Translate the derive expression and collect its dependencies.
    let (compute_js, deps) =
        translate_derive_expr(&decl.expr, derive_map, runtime_signals, captures);

    // Lower dependency derives FIRST so they are registered before this one
    // (ST.derive over a not-yet-defined dep would never fire its first compute).
    for dep in &deps {
        lower_derive_chain(dep, derive_map, runtime_signals, captures, seen, out);
    }

    out.push(EvaluatedDerive {
        name: key.to_string(),
        deps,
        compute_js,
    });
}

/// Translate a `%derives` expression to a JS compute expression + its runtime
/// dependency signal names (PLAN-054).
///
/// Substitutions (in order):
///   - `clamp(...)`        -> `ST.clamp(...)`
///   - `&$bounds.rect.W`   -> `ST.rectOf(<boundsExpr>).W` (bounds is a capture)
///   - `&self.rect.W`      -> `ST.rectOf(el).W`
///   - `none`              -> `null`  (the Spacetime "unset" literal)
///   - `$sig`              -> `v.sig` when `sig` is a runtime signal or another
///                            derive (collected as a dep); otherwise the captured
///                            literal value (inlined).
/// The returned deps are the distinct runtime/derive signal names referenced.
fn translate_derive_expr(
    expr: &str,
    derive_map: &HashMap<String, &DeriveDecl>,
    runtime_signals: &std::collections::HashSet<String>,
    captures: &HashMap<String, CapturedValue>,
) -> (String, Vec<String>) {
    // Deps are collected across all code segments.
    let mut deps: Vec<String> = Vec::new();

    // Apply every substitution ONLY to code regions, never inside string
    // literals — a derive like `$axis == "none" ? ...` must keep the literal
    // "none" intact, and a `$`-token inside a quoted string must not become a
    // dep. `map_code_segments` walks the expr splitting string literals out, so
    // the transforms below operate only on the code between/around them.
    let out = map_code_segments(expr, |code| {
        let mut seg = code.to_string();

        // clamp(...) -> ST.clamp(...) (word-boundary so it never touches `$clamp`).
        let clamp_re = regex::Regex::new(r"\bclamp\s*\(").unwrap();
        seg = clamp_re.replace_all(&seg, "ST.clamp(").into_owned();

        // &$<cap>.rect.<field> -> ST.rectOf(<resolved-cap>).<field>
        // `<cap>` is a capture holding an element ref (`bounds: &track` ->
        // Element("track")). Inline the capture's RESOLVED JS literal
        // (`ST.ref("track")` for an Element capture, PLAN-057) so `&$bounds.rect`
        // reads the live rect of the bound element. A capture that resolves to
        // `null` keeps the bounds branch guarded-false (ST.rectOf(null) is a zero
        // rect, never reached because `$bounds != none` is then false).
        let bounds_re =
            regex::Regex::new(r"&\$([a-zA-Z_][a-zA-Z0-9_]*)\.rect\.([a-zA-Z]+)").unwrap();
        seg = bounds_re
            .replace_all(&seg, |caps: &regex::Captures| {
                let cap_name = &caps[1];
                let field = &caps[2];
                let resolved = captures
                    .get(cap_name)
                    .map(captured_value_to_js_literal)
                    .unwrap_or_else(|| "null".to_string());
                format!("ST.rectOf({}).{}", resolved, field)
            })
            .into_owned();

        // &self.rect.<field> -> ST.rectOf(el).<field>
        let self_re = regex::Regex::new(r"&self\.rect\.([a-zA-Z]+)").unwrap();
        seg = self_re.replace_all(&seg, "ST.rectOf(el).$1").into_owned();

        // `none` literal (whole word) -> null.
        let none_re = regex::Regex::new(r"\bnone\b").unwrap();
        seg = none_re.replace_all(&seg, "null").into_owned();

        // $sig references: runtime signal / derive -> v.sig (dep); capture -> literal.
        let sig_re = regex::Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let names: Vec<String> = sig_re
            .captures_iter(&seg)
            .map(|c| c[1].to_string())
            .collect();
        for name in names {
            let is_runtime = runtime_signals.contains(&name) || derive_map.contains_key(&name);
            let pat = regex::Regex::new(&format!(r"\${}\b", regex::escape(&name))).unwrap();
            if is_runtime {
                if !deps.contains(&name) {
                    deps.push(name.clone());
                }
                seg = pat
                    .replace_all(&seg, regex::NoExpand(&format!("v.{}", name)))
                    .into_owned();
            } else {
                // Compile-time capture: inline its JS literal.
                let literal = captures
                    .get(&name)
                    .map(captured_value_to_js_literal)
                    .unwrap_or_else(|| "null".to_string());
                seg = pat
                    .replace_all(&seg, regex::NoExpand(&literal))
                    .into_owned();
            }
        }
        seg
    });

    (out.trim().to_string(), deps)
}

/// Apply `f` to every CODE segment of a JS-ish expression, leaving string
/// literals (`"..."`, `'...'`, `` `...` ``) untouched (PLAN-054, audit #2).
///
/// Splits `expr` into alternating code / string-literal spans (respecting
/// backslash escapes) and runs `f` only on the code spans, re-concatenating with
/// the literals verbatim. This is what keeps `none`/`$sig`/`clamp` rewrites from
/// corrupting a string like `"none"` or a `$`-token inside quotes. Template
/// literals are treated as opaque strings (no `${}` interpolation in derive
/// exprs), which is correct for the Spacetime expression grammar.
fn map_code_segments(expr: &str, mut f: impl FnMut(&str) -> String) -> String {
    let bytes = expr.as_bytes();
    let mut out = String::with_capacity(expr.len());
    let mut code_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'"' || c == b'\'' || c == b'`' {
            // Flush the preceding code segment through the transform.
            if code_start < i {
                out.push_str(&f(&expr[code_start..i]));
            }
            // Scan the string literal verbatim (honoring backslash escapes).
            let quote = c;
            let lit_start = i;
            i += 1;
            while i < bytes.len() {
                let d = bytes[i];
                if d == b'\\' {
                    i += 2;
                    continue;
                }
                if d == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            // Append the literal (including quotes) untouched.
            out.push_str(&expr[lit_start..i.min(expr.len())]);
            code_start = i;
        } else {
            i += 1;
        }
    }
    if code_start < expr.len() {
        out.push_str(&f(&expr[code_start..]));
    }
    out
}

/// Render a captured value as a JS LITERAL for inlining into a derive compute
/// expression (PLAN-054). Strings are quoted; numbers/bools bare; null/none null.
fn captured_value_to_js_literal(value: &CapturedValue) -> String {
    match value {
        CapturedValue::String(s) | CapturedValue::Ident(s) => {
            // A captured ident like `none`/`null` means "unset".
            if s == "none" || s == "null" {
                "null".to_string()
            } else {
                format!("{:?}", s) // JSON-style quoted string
            }
        }
        CapturedValue::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        CapturedValue::Bool(b) => b.to_string(),
        CapturedValue::Expr(e) => e.clone(),
        // An `&name` element-ref capture (e.g. @drag's `bounds: &track` ->
        // Element("track")) resolves to its DOM node via the `__stRefs` registry
        // the element-ref primitive populates (PLAN-057). `ST.ref("track")` returns
        // the element (or null when no such ref is registered), so a lowered
        // bounds derive reads a REAL rect and `$bounds != none` is a genuine
        // `ST.ref(...) != null` test — the clamp is live, not inert.
        //
        // FEAT-142 WAVE C: a DOTTED capture (`&evernet.peak.summit`) is a FACET
        // PATH, not a plain element ref. It resolves against the spatial world
        // registry (`window.__stWorld.byName`) via `ST.worldFacet(entity,
        // component, facet)` instead of the DOM `__stRefs`. A bare single-segment
        // `&name` keeps the existing `ST.ref` behaviour (revB: bare unchanged).
        CapturedValue::Element(name) if crate::parser::is_facet_path(name) => {
            crate::parser::emit_facet_read_js(name)
        }
        CapturedValue::Element(name) => format!("ST.ref({:?})", name),
        // A selector/binding capture still has no scalar literal in a detached
        // derive closure — inline as null (guarded-false), as before.
        CapturedValue::Selector(_) | CapturedValue::Binding(_) => "null".to_string(),
        _ => "null".to_string(),
    }
}

/// Evaluate a `MetaIfCondition` against a captures map.
///
/// This is the single implementation used by both the Evaluate layer and
/// (transitionally) the Resolve layer's `get_conditional_binds`.
pub fn evaluate_if_condition(
    captures: &HashMap<String, CapturedValue>,
    condition: &MetaIfCondition,
) -> bool {
    match condition {
        MetaIfCondition::Truthy(var) => captures.get(var).is_some_and(|val| val.is_truthy()),
        MetaIfCondition::Falsy(var) => {
            !evaluate_if_condition(captures, &MetaIfCondition::Truthy(var.clone()))
        }
        MetaIfCondition::Equals(var, expected) => {
            if let Some(val) = captures.get(var) {
                val.to_string_value() == *expected
            } else {
                false
            }
        }
        MetaIfCondition::NotEquals(var, expected) => !evaluate_if_condition(
            captures,
            &MetaIfCondition::Equals(var.clone(), expected.clone()),
        ),
        MetaIfCondition::Or(left, right) => {
            evaluate_if_condition(captures, left) || evaluate_if_condition(captures, right)
        }
        MetaIfCondition::And(left, right) => {
            evaluate_if_condition(captures, left) && evaluate_if_condition(captures, right)
        }
        MetaIfCondition::LessThan(var, expected) => {
            if let Some(CapturedValue::Number(n)) = captures.get(var)
                && let Ok(e) = expected.parse::<f64>()
            {
                return *n < e;
            }
            false
        }
        MetaIfCondition::GreaterThan(var, expected) => {
            if let Some(CapturedValue::Number(n)) = captures.get(var)
                && let Ok(e) = expected.parse::<f64>()
            {
                return *n > e;
            }
            false
        }
        MetaIfCondition::LessThanOrEqual(var, expected) => {
            if let Some(CapturedValue::Number(n)) = captures.get(var)
                && let Ok(e) = expected.parse::<f64>()
            {
                return *n <= e;
            }
            false
        }
        MetaIfCondition::GreaterThanOrEqual(var, expected) => {
            if let Some(CapturedValue::Number(n)) = captures.get(var)
                && let Ok(e) = expected.parse::<f64>()
            {
                return *n >= e;
            }
            false
        }
    }
}

/// Simple parameter substitution for `%emit` blocks.
///
/// Replaces `%$name` references with their string values from bindings.
/// Does NOT handle `.js` field accessor or nested expansion — those
/// require the full MacroContext and are handled during Expand.
fn substitute_params_simple(bindings: &HashMap<String, CapturedValue>, content: &str) -> String {
    // Pattern: %$identifier or %$identifier.field
    let re =
        regex::Regex::new(r"%\$([a-zA-Z_][a-zA-Z0-9_]*)(?:\.([a-zA-Z_][a-zA-Z0-9_]*))?").unwrap();

    let mut result = content.to_string();

    let matches: Vec<(String, String, Option<String>)> = re
        .captures_iter(content)
        .map(|cap| {
            let full_match = cap.get(0).unwrap().as_str().to_string();
            let var_name = cap[1].to_string();
            let field_name = cap.get(2).map(|m| m.as_str().to_string());
            (full_match, var_name, field_name)
        })
        .collect();

    for (full_match, var_name, field_name) in &matches {
        if let Some(value) = bindings.get(var_name.as_str()) {
            let replacement = captured_value_to_emit_string(value, field_name.as_deref());
            result = result.replace(full_match.as_str(), &replacement);
        }
    }

    result
}

/// Convert a CapturedValue to its string representation for emit substitution.
fn captured_value_to_emit_string(value: &CapturedValue, field: Option<&str>) -> String {
    match (value, field) {
        (CapturedValue::Named(map), Some(field_name)) => map
            .get(field_name)
            .map(|v| captured_value_to_emit_string(v, None))
            .unwrap_or_default(),
        (CapturedValue::String(s), _) => s.clone(),
        (CapturedValue::Ident(s), _) => s.clone(),
        (CapturedValue::Number(n), _) => {
            if *n == (*n as i64) as f64 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        (CapturedValue::Bool(b), _) => b.to_string(),
        (CapturedValue::Time(t), _) => format!("{}ms", t),
        (CapturedValue::Selector(s), _) => s.clone(),
        (CapturedValue::Binding(s), _) => s.clone(),
        (CapturedValue::Element(e), _) => e.clone(),
        (CapturedValue::Expr(e), _) => e.clone(),
        (CapturedValue::TypeRef(t), _) => t.clone(),
        (CapturedValue::Preset(p), _) => p.clone(),
        (CapturedValue::Color(c), _) => c.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_captures(pairs: &[(&str, CapturedValue)]) -> HashMap<String, CapturedValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn test_evaluate_if_truthy() {
        let captures = make_captures(&[("enabled", CapturedValue::Bool(true))]);
        let cond = MetaIfCondition::Truthy("enabled".to_string());
        assert!(evaluate_if_condition(&captures, &cond));
    }

    #[test]
    fn facet_path_element_lowers_to_worldfacet() {
        // FEAT-142 WAVE C: a bare `&name` element ref keeps `ST.ref`; a dotted
        // facet path `&entity.component.facet` routes to `ST.worldFacet(...)`
        // against the spatial world registry.
        let bare = captured_value_to_js_literal(&CapturedValue::Element("track".to_string()));
        assert_eq!(
            bare, r#"ST.ref("track")"#,
            "bare element ref must stay ST.ref"
        );

        let facet = captured_value_to_js_literal(&CapturedValue::Element(
            "evernet.peak.summit".to_string(),
        ));
        assert_eq!(
            facet, r#"ST.worldFacet("evernet", "peak", "summit")"#,
            "dotted facet path must route to ST.worldFacet"
        );

        // Two-segment `&entity.component` resolves the component (facet empty).
        let comp =
            captured_value_to_js_literal(&CapturedValue::Element("evernet.peak".to_string()));
        assert_eq!(comp, r#"ST.worldFacet("evernet", "peak", "")"#);
    }

    #[test]
    fn test_evaluate_if_falsy() {
        let captures = make_captures(&[("enabled", CapturedValue::Bool(false))]);
        let cond = MetaIfCondition::Truthy("enabled".to_string());
        assert!(!evaluate_if_condition(&captures, &cond));
    }

    #[test]
    fn test_evaluate_if_missing_variable() {
        let captures = HashMap::new();
        let cond = MetaIfCondition::Truthy("enabled".to_string());
        assert!(!evaluate_if_condition(&captures, &cond));
    }

    #[test]
    fn test_evaluate_if_equals() {
        let captures = make_captures(&[("axis", CapturedValue::String("x".to_string()))]);
        let cond = MetaIfCondition::Equals("axis".to_string(), "x".to_string());
        assert!(evaluate_if_condition(&captures, &cond));
    }

    #[test]
    fn test_evaluate_if_not_equals() {
        let captures = make_captures(&[("axis", CapturedValue::String("x".to_string()))]);
        let cond = MetaIfCondition::NotEquals("axis".to_string(), "y".to_string());
        assert!(evaluate_if_condition(&captures, &cond));
    }

    #[test]
    fn test_evaluate_if_or() {
        let captures = make_captures(&[("mode", CapturedValue::String("fast".to_string()))]);
        let cond = MetaIfCondition::Or(
            Box::new(MetaIfCondition::Equals(
                "mode".to_string(),
                "fast".to_string(),
            )),
            Box::new(MetaIfCondition::Equals(
                "mode".to_string(),
                "turbo".to_string(),
            )),
        );
        assert!(evaluate_if_condition(&captures, &cond));
    }

    #[test]
    fn test_evaluate_if_numeric_comparison() {
        let captures = make_captures(&[("count", CapturedValue::Number(5.0))]);
        assert!(evaluate_if_condition(
            &captures,
            &MetaIfCondition::GreaterThan("count".to_string(), "3".to_string())
        ));
        assert!(!evaluate_if_condition(
            &captures,
            &MetaIfCondition::LessThan("count".to_string(), "3".to_string())
        ));
        assert!(evaluate_if_condition(
            &captures,
            &MetaIfCondition::LessThanOrEqual("count".to_string(), "5".to_string())
        ));
        assert!(evaluate_if_condition(
            &captures,
            &MetaIfCondition::GreaterThanOrEqual("count".to_string(), "5".to_string())
        ));
    }

    #[test]
    fn test_substitute_params_simple() {
        let mut bindings = HashMap::new();
        bindings.insert(
            "name".to_string(),
            CapturedValue::String("hero".to_string()),
        );
        bindings.insert("duration".to_string(), CapturedValue::Number(300.0));

        let content = "el.classList.add('%$name'); setTimeout(() => {}, %$duration);";
        let result = substitute_params_simple(&bindings, content);
        assert_eq!(
            result,
            "el.classList.add('hero'); setTimeout(() => {}, 300);"
        );
    }

    #[test]
    fn test_substitute_params_field_access() {
        let mut inner = HashMap::new();
        inner.insert("x".to_string(), CapturedValue::Number(10.0));
        let mut bindings = HashMap::new();
        bindings.insert("pos".to_string(), CapturedValue::Named(inner));

        let content = "x = %$pos.x;";
        let result = substitute_params_simple(&bindings, content);
        assert_eq!(result, "x = 10;");
    }

    #[test]
    fn test_evaluate_body_binds() {
        let binds = vec![BindDecl {
            primitive: "scroll".to_string(),
            args: vec![],
            outputs: vec![],
            span: SourceSpan::default(),
        }];

        let captures = HashMap::new();
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        evaluate_body_item(&mut ctx, &MacroBodyItem::Binds(binds.clone()), &mut output).unwrap();

        assert_eq!(output.bind_decls.len(), 1);
        assert_eq!(output.bind_decls[0].primitive, "scroll");
    }

    #[test]
    fn test_evaluate_body_when_true() {
        let captures = make_captures(&[("enabled", CapturedValue::Bool(true))]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::When(WhenClause {
            condition: "enabled".to_string(),
            body: vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "scroll".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])],
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(output.bind_decls.len(), 1);
    }

    #[test]
    fn test_evaluate_body_when_false() {
        let captures = make_captures(&[("enabled", CapturedValue::Bool(false))]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::When(WhenClause {
            condition: "enabled".to_string(),
            body: vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "scroll".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])],
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(output.bind_decls.len(), 0);
    }

    #[test]
    fn test_evaluate_body_for_loop() {
        let captures = make_captures(&[(
            "items",
            CapturedValue::Array(vec![
                CapturedValue::String("a".to_string()),
                CapturedValue::String("b".to_string()),
                CapturedValue::String("c".to_string()),
            ]),
        )]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::For(MetaForClause {
            variable: "item".to_string(),
            source: "items".to_string(),
            body: vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "setup".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])],
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        // 3 items in array → 3 bind entries
        assert_eq!(output.bind_decls.len(), 3);
    }

    #[test]
    fn test_evaluate_body_if_then() {
        let captures = make_captures(&[("axis", CapturedValue::String("x".to_string()))]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::If(MetaIfClause {
            condition: MetaIfCondition::Equals("axis".to_string(), "x".to_string()),
            then_body: vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "x-axis".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])],
            elif_clauses: vec![],
            else_body: Some(vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "both-axis".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])]),
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(output.bind_decls.len(), 1);
        assert_eq!(output.bind_decls[0].primitive, "x-axis");
    }

    #[test]
    fn test_evaluate_body_if_else() {
        let captures = make_captures(&[("axis", CapturedValue::String("both".to_string()))]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::If(MetaIfClause {
            condition: MetaIfCondition::Equals("axis".to_string(), "x".to_string()),
            then_body: vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "x-axis".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])],
            elif_clauses: vec![],
            else_body: Some(vec![MacroBodyItem::Binds(vec![BindDecl {
                primitive: "both-axis".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }])]),
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(output.bind_decls.len(), 1);
        assert_eq!(output.bind_decls[0].primitive, "both-axis");
    }

    #[test]
    fn test_evaluate_body_emit() {
        let captures = make_captures(&[("name", CapturedValue::String("hero".to_string()))]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::Emit(EmitBlock {
            lang: EmitLang::Js,
            content: "console.log('%$name');".to_string(),
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(output.emit_blocks.len(), 1);
        assert_eq!(output.emit_blocks[0].content, "console.log('hero');");
        assert!(matches!(output.emit_blocks[0].lang, EmitLang::Js));
    }

    #[test]
    fn test_evaluate_match_form_directive_fallback() {
        // Simulates @on visible: macro_name = "test" (from @test directive),
        // but the macro is registered as "test-dispatcher" with form @test ...
        let mut registry = MetaRegistry::new();

        let macro_def = MacroDefAst {
            retired: None,
            name: "test-dispatcher".to_string(),
            form: Some(FormClause {
                directive_name: "@test".to_string(),
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
                    primitive: "test-driver".to_string(),
                    args: vec![],
                    outputs: vec![],
                    span: SourceSpan::default(),
                }])],
                elif_clauses: vec![],
                else_body: None,
                span: SourceSpan::default(),
            })],
            order: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            derives: vec![],
            requires: vec![],
            registers: None,
            imports: None,
            resolves: None,
            states: None,
            source_file: None,
            span: SourceSpan::default(),
            module: None,
            doc: None,
            ..Default::default()
        };
        registry.register_macro(macro_def).unwrap();

        let form_match = FormMatch {
            macro_name: "test".to_string(), // directive-derived, NOT "test-dispatcher"
            matched_macro: None,
            captures: make_captures(&[("event", CapturedValue::Ident("visible".to_string()))]),
            capture_spans: Default::default(),
            selector: Some(".hero".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = evaluate_match(&form_match, &registry).unwrap();

        assert!(
            result.body_was_evaluated,
            "evaluate_match must find the macro via form-directive fallback"
        );
        assert_eq!(
            result.bind_decls.len(),
            1,
            "The %if $event == visible branch must produce exactly one bind_decl"
        );
        assert_eq!(
            result.bind_decls[0].primitive, "test-driver",
            "Must resolve to the conditional bind's primitive"
        );
    }

    // =========================================================================
    // %on $var -> value lowering (PLAN-053)
    // =========================================================================

    #[test]
    fn test_substitute_on_action_replaces_macro_param() {
        // A bare `$onDrop` action substitutes to the captured action expression.
        // The substituted value itself contains `$move` + `$.dataset` — neither is
        // a param, so both must pass through verbatim (no regex `$`-interpolation
        // corruption).
        let captures = make_captures(&[(
            "onDrop",
            CapturedValue::Expr("$move({ card: $.dataset.id })".to_string()),
        )]);
        let registry = MetaRegistry::new();
        let ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let out = substitute_on_action(&ctx, "$onDrop");
        assert_eq!(out, "$move({ card: $.dataset.id })");
    }

    #[test]
    fn test_substitute_on_action_passthrough_unknown() {
        // A `$name` with no binding is a page signal / runtime target — pass through.
        let captures = make_captures(&[]);
        let registry = MetaRegistry::new();
        let ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let out = substitute_on_action(&ctx, "$move({ to: $.x })");
        assert_eq!(out, "$move({ to: $.x })");
    }

    #[test]
    fn test_on_clause_lowered_not_dropped() {
        // %on $active -> false { $onDrop } must produce an EvaluatedOn (PLAN-053).
        // Before the fix this body item was a silent no-op.
        let captures = make_captures(&[(
            "onDrop",
            CapturedValue::Expr("$move({ card: $.dataset.id })".to_string()),
        )]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::On(MetaOnClause {
            trigger: MetaOnTrigger::VarTransition {
                var: "active".to_string(),
                value: "false".to_string(),
            },
            body: vec![MetaOnBodyItem::Action("$onDrop".to_string())],
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();

        assert_eq!(output.on_clauses.len(), 1, "the %on clause must be lowered");
        let on = &output.on_clauses[0];
        assert_eq!(on.var, "active");
        assert_eq!(on.value, "false");
        assert_eq!(
            on.actions,
            vec!["$move({ card: $.dataset.id })".to_string()]
        );
    }

    #[test]
    fn test_on_clause_assignment_action() {
        // A `$x <- expr` assignment in an %on body lowers to a `$x <- expr` statement.
        let captures = make_captures(&[]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::On(MetaOnClause {
            trigger: MetaOnTrigger::VarTransition {
                var: "active".to_string(),
                value: "false".to_string(),
            },
            body: vec![MetaOnBodyItem::Assignment {
                var: "dropped".to_string(),
                expr: "$.dataset.id".to_string(),
            }],
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(output.on_clauses.len(), 1);
        assert_eq!(
            output.on_clauses[0].actions,
            vec!["$dropped <- $.dataset.id".to_string()]
        );
    }

    #[test]
    fn test_on_clause_var_event_not_lowered() {
        // Only VarTransition triggers lower here; a VarEvent trigger is left alone.
        let captures = make_captures(&[]);
        let registry = MetaRegistry::new();
        let mut ctx = EvalCtx::new(&captures, &registry, SourceSpan::default());
        let mut output = EvalOutput::default();

        let item = MacroBodyItem::On(MetaOnClause {
            trigger: MetaOnTrigger::VarEvent {
                var: "x".to_string(),
                event: "change".to_string(),
            },
            body: vec![MetaOnBodyItem::Action("$y()".to_string())],
            span: SourceSpan::default(),
        });

        evaluate_body_item(&mut ctx, &item, &mut output).unwrap();
        assert_eq!(
            output.on_clauses.len(),
            0,
            "VarEvent triggers are not lowered here"
        );
    }

    // =========================================================================
    // %derives reactive lowering (PLAN-054)
    // =========================================================================

    fn runtime_set(names: &[&str]) -> std::collections::HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn audit2_string_literal_preserved() {
        let derive_map: HashMap<String, &DeriveDecl> = HashMap::new();
        let runtime = runtime_set(&["deltaX"]);
        let captures = make_captures(&[("axis", CapturedValue::String("x".to_string()))]);
        // The literal "none" and a $-token inside a string must survive untouched;
        // only the CODE $axis inlines, and the bare `none` keyword -> null.
        let (js, _d) = translate_derive_expr(
            "$axis == \"none\" ? none : \"$deltaX literal\"",
            &derive_map,
            &runtime,
            &captures,
        );
        assert_eq!(js, "\"x\" == \"none\" ? null : \"$deltaX literal\"");
    }

    #[test]
    fn test_translate_derive_signal_dep() {
        let derive_map: HashMap<String, &DeriveDecl> = HashMap::new();
        let runtime = runtime_set(&["deltaX"]);
        let captures = make_captures(&[("axis", CapturedValue::String("both".to_string()))]);
        let (js, deps) = translate_derive_expr(
            "$axis != \"y\" ? $deltaX : 0",
            &derive_map,
            &runtime,
            &captures,
        );
        assert_eq!(js, "\"both\" != \"y\" ? v.deltaX : 0");
        assert_eq!(deps, vec!["deltaX".to_string()]);
    }

    #[test]
    fn test_translate_derive_clamp_and_rect() {
        let derive_map: HashMap<String, &DeriveDecl> = HashMap::new();
        let runtime = runtime_set(&["deltaX"]);
        let captures = make_captures(&[]);
        let (js, _deps) = translate_derive_expr(
            "clamp($deltaX, 0, &self.rect.width)",
            &derive_map,
            &runtime,
            &captures,
        );
        assert_eq!(js, "ST.clamp(v.deltaX, 0, ST.rectOf(el).width)");
    }

    #[test]
    fn test_translate_derive_none_literal() {
        let derive_map: HashMap<String, &DeriveDecl> = HashMap::new();
        let runtime = runtime_set(&[]);
        let captures = make_captures(&[("bounds", CapturedValue::Ident("none".to_string()))]);
        let (js, deps) =
            translate_derive_expr("$bounds != none ? 1 : 0", &derive_map, &runtime, &captures);
        assert_eq!(js, "null != null ? 1 : 0");
        assert!(deps.is_empty(), "a capture is not a runtime dep");
    }

    #[test]
    fn test_lower_derive_chain_orders_deps_first() {
        let raw = DeriveDecl {
            name: "rawX".to_string(),
            expr: "$deltaX".to_string(),
            span: SourceSpan::default(),
        };
        let x = DeriveDecl {
            name: "x".to_string(),
            expr: "$rawX".to_string(),
            span: SourceSpan::default(),
        };
        let mut derive_map: HashMap<String, &DeriveDecl> = HashMap::new();
        derive_map.insert("rawX".to_string(), &raw);
        derive_map.insert("x".to_string(), &x);
        let runtime = runtime_set(&["deltaX"]);
        let captures = make_captures(&[]);
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        lower_derive_chain("x", &derive_map, &runtime, &captures, &mut seen, &mut out);
        let names: Vec<&str> = out.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["rawX", "x"],
            "deps must be lowered before dependents"
        );
        assert_eq!(out[0].deps, vec!["deltaX".to_string()]);
        assert_eq!(out[1].deps, vec!["rawX".to_string()]);
    }
}
