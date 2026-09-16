//! Primitive JavaScript Code Generation
//!
//! Transforms %primitive definitions into optimized JavaScript code that
//! integrates with the ST minimal runtime.
//!
//! ## Transformation Rules
//!
//! | Placeholder | Replacement |
//! |-------------|-------------|
//! | `%&el` | `el` (element reference) |
//! | `%param` | Literal value from binding |
//! | `%yield expr -> $var` | `ST.set(el, 'var', expr)` |
//! | `%cleanup { code }` | Collected for cleanup function |
//!
//! ## IR Integration
//!
//! Use `generate_primitive_ir()` to get structured IR output (GeneratedPrimitiveIR).
//! Stringify at the boundary with `emit_stmts()` / `emit_all()` when strings are needed.

use crate::emit::{EmitOptions, css as css_emit, js as js_emit};
use crate::emit::{css_parser, js_parser};
use crate::ir::{CssExpr, JsStmt};
use crate::parser::meta_ast::*;
use std::collections::HashMap;

/// Arguments for primitive instantiation
#[derive(Debug, Clone, Default)]
pub struct PrimitiveArgs {
    /// Element reference mappings: param_name -> js_expression
    pub elements: HashMap<String, String>,
    /// Parameter values: param_name -> js_literal
    pub params: HashMap<String, String>,
    /// Output signal name mappings: primitive_export_name -> user_signal_name
    /// Used when %binds remaps output names (e.g., socket(...) -> { $state } with $state = "ws")
    pub outputs: HashMap<String, String>,
    /// CSS-specific parameter overrides: param_name -> raw CSS text
    /// Used when a parameter needs different representation in CSS vs JS contexts.
    /// For example, StyleProperties captured from a macro body need to be emitted
    /// as CSS declarations (key: value;) rather than JS object literals.
    pub css_overrides: HashMap<String, String>,
}

impl PrimitiveArgs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an element reference
    pub fn element(mut self, name: &str, expr: &str) -> Self {
        self.elements.insert(name.to_string(), expr.to_string());
        self
    }

    /// Add a parameter value
    pub fn param(mut self, name: &str, value: &str) -> Self {
        self.params.insert(name.to_string(), value.to_string());
        self
    }

    /// Add an output signal name mapping
    pub fn output(mut self, primitive_name: &str, user_name: &str) -> Self {
        self.outputs
            .insert(primitive_name.to_string(), user_name.to_string());
        self
    }

    /// Add a CSS-specific parameter override.
    /// When present, the CSS emit path uses this value instead of the JS-formatted one.
    pub fn css_param(mut self, name: &str, css_value: &str) -> Self {
        self.css_overrides
            .insert(name.to_string(), css_value.to_string());
        self
    }
}

/// IR-based generated code from a primitive
///
/// This structure holds IR types instead of raw strings, enabling:
/// - Introspection of the generated code structure
/// - Deferred stringification with different emit options
/// - Better debugging and tooling support
#[derive(Debug, Clone)]
pub struct GeneratedPrimitiveIR {
    /// JS statements (wrapped in JsStmt::Raw for now, will be structured later)
    pub js_stmts: Vec<JsStmt>,
    /// CSS expressions (wrapped in CssExpr::Raw for now, will be structured later)
    pub css_exprs: Vec<CssExpr>,
    /// Cleanup JS statements
    pub cleanup_stmts: Vec<JsStmt>,
    /// Build-time JS statements from %emit build-js blocks
    pub build_js_stmts: Vec<JsStmt>,
    /// Prelude JS statements — emitted once per primitive type, before per-element IIFEs
    pub prelude_js_stmts: Vec<JsStmt>,
    /// Prelude CSS expressions — emitted once per primitive type
    pub prelude_css_exprs: Vec<CssExpr>,
    /// Exported signal names
    pub exports: Vec<String>,
    /// HTML markup from `%emit html` blocks (FEAT-082), captures substituted.
    /// Spliced into `CompiledSpacetime.html` at the directive's position.
    pub html: Vec<String>,
    /// Diagnostics collected during code generation (e.g., unknown parameter errors)
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

impl GeneratedPrimitiveIR {
    /// Create a new empty GeneratedPrimitiveIR
    pub fn new() -> Self {
        Self {
            js_stmts: Vec::new(),
            css_exprs: Vec::new(),
            cleanup_stmts: Vec::new(),
            build_js_stmts: Vec::new(),
            prelude_js_stmts: Vec::new(),
            prelude_css_exprs: Vec::new(),
            exports: Vec::new(),
            html: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl Default for GeneratedPrimitiveIR {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate IR from a primitive definition
///
/// This is the IR-based version of `generate_primitive_js`. It attempts to
/// parse JS emit blocks into structured IR with Placeholder nodes, falling
/// back to Raw for complex cases or parse failures.
///
/// The resulting IR can be introspected and has deferred stringification.
pub fn generate_primitive_ir(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
) -> GeneratedPrimitiveIR {
    generate_primitive_ir_with_registry(primitive, args, None, None)
}

/// Like [`generate_primitive_ir`], but threads the live MetaRegistry into the
/// EmitContext so a `$body:block` `.js` field accessor can compile its nested
/// directives against the same macro set as the host compile (PLAN-026).
///
/// `helpers` names the page's compile-time-known filters (its `@fn`
/// definitions): `sexpr` lowering rewrites bare calls to them as
/// `ST.filters.<name>(…)`; unknown bare calls pass through verbatim.
pub fn generate_primitive_ir_with_registry(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
    registry: Option<std::sync::Arc<crate::metasystem::MetaRegistry>>,
    helpers: Option<&std::collections::HashSet<String>>,
) -> GeneratedPrimitiveIR {
    // Use the structured path which parses emit blocks into IR with Placeholders,
    // then resolves them to produce the final output
    generate_primitive_ir_structured(primitive, args, registry, helpers)
}

/// Generate IR from a primitive definition using the structured JS parser.
///
/// This function:
/// 1. Processes emit blocks using `js_parser::parse_emit_js()`
/// 2. Builds an `EmitContext` from `PrimitiveArgs`
/// 3. Calls `resolve_stmt()` on each statement to resolve Placeholders
/// 4. Returns `GeneratedPrimitiveIR` with resolved statements
///
/// Falls back gracefully to Raw for parse errors or unsupported constructs.
pub fn generate_primitive_ir_structured(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
    registry: Option<std::sync::Arc<crate::metasystem::MetaRegistry>>,
    helpers: Option<&std::collections::HashSet<String>>,
) -> GeneratedPrimitiveIR {
    let mut result = GeneratedPrimitiveIR::new();

    // Collect exports from the primitive
    for export in &primitive.body.exports {
        result.exports.push(export.name.clone());
    }

    // Fill in defaults from primitive params for any missing arguments
    let args_with_defaults = fill_default_args_structured(primitive, args);

    // Validate required params - return early with error if missing
    if let Some(error_ir) = validate_required_args_structured(primitive, &args_with_defaults) {
        return error_ir;
    }

    // BUG-252 S2: a scalar-declared param handed an object literal would
    // stringify to "[object Object]" and go inert. Refuse it here.
    validate_scalar_params_are_not_objects(primitive, &args_with_defaults, &mut result);

    // Build EmitContext from PrimitiveArgs + primitive param types
    let ctx = {
        crate::profile_span!("codegen_emit_context");
        let mut ctx = build_emit_context_structured(&args_with_defaults, primitive);
        ctx.registry = registry;
        // PLAN-133: `sexpr` params are lowered at compile time (SWC parse,
        // scope-aware name resolution) into synthetic `__fn`/`__body`/`__deps`/
        // `__step_fn` params the template splices verbatim. A lowering error
        // is a compile diagnostic, never a runtime eval failure.
        inject_sexpr_synthetics(
            primitive,
            &args_with_defaults,
            helpers,
            &mut ctx,
            &mut result,
        );
        ctx
    };
    let opts = EmitOptions::pretty();

    // Process the primitive body using the structured path
    crate::profile_span!("codegen_body");
    process_primitive_body_structured(
        &primitive.body,
        &args_with_defaults,
        &ctx,
        &opts,
        &mut result,
    );

    result
}

/// Fill in default values for missing arguments based on primitive param definitions.
fn fill_default_args_structured(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
) -> PrimitiveArgs {
    let mut args_with_defaults = args.clone();

    for param in &primitive.params {
        if let PrimitiveParam::Typed { name, ty, default } = param
            && !args_with_defaults.params.contains_key(name)
        {
            if let Some(default_val) = default {
                let default_value = match default_val {
                    ParamDefault::String(s) => s.clone(),
                    ParamDefault::Number(n) => n.to_string(),
                    ParamDefault::Bool(b) => b.to_string(),
                    ParamDefault::None => "null".to_string(),
                    ParamDefault::EmptyArray => "[]".to_string(),
                    ParamDefault::EmptyObject => "{}".to_string(),
                    ParamDefault::Length(val, unit) => format!("\"{}{}\"", val, unit),
                    ParamDefault::Array(items) => {
                        fn param_default_to_json(d: &ParamDefault) -> String {
                            match d {
                                ParamDefault::String(s) => {
                                    format!("\"{}\"", s.replace('"', "\\\""))
                                }
                                ParamDefault::Number(n) => n.to_string(),
                                ParamDefault::Bool(b) => b.to_string(),
                                ParamDefault::None => "null".to_string(),
                                ParamDefault::EmptyArray => "[]".to_string(),
                                ParamDefault::EmptyObject => "{}".to_string(),
                                ParamDefault::Length(val, unit) => {
                                    format!("\"{}{}\"", val, unit)
                                }
                                ParamDefault::Array(items) => format!(
                                    "[{}]",
                                    items
                                        .iter()
                                        .map(param_default_to_json)
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            }
                        }
                        format!(
                            "[{}]",
                            items
                                .iter()
                                .map(param_default_to_json)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                };
                args_with_defaults
                    .params
                    .insert(name.clone(), default_value);
            } else if matches!(ty, ParamType::Optional(_) | ParamType::OptionalArray(_)) {
                args_with_defaults
                    .params
                    .insert(name.clone(), "null".to_string());
            }
        }
    }

    args_with_defaults
}

/// Validate that all required parameters are provided.
/// Returns Some(error_ir) if validation fails, None if validation passes.
fn validate_required_args_structured(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
) -> Option<GeneratedPrimitiveIR> {
    for param in &primitive.params {
        match param {
            PrimitiveParam::Typed { name, ty, default } => {
                let is_required = default.is_none()
                    && !matches!(ty, ParamType::Optional(_) | ParamType::OptionalArray(_));

                if is_required && !args.params.contains_key(name) {
                    let mut ir = GeneratedPrimitiveIR::new();
                    ir.js_stmts.push(JsStmt::Raw(format!(
                        "console.error('Spacetime: Missing required parameter \"{}\" for primitive \"{}\"');\n",
                        name, primitive.name
                    )));
                    return Some(ir);
                }
            }
            PrimitiveParam::Element(name) => {
                if !args.elements.contains_key(name) {
                    let mut ir = GeneratedPrimitiveIR::new();
                    ir.js_stmts.push(JsStmt::Raw(format!(
                        "console.error('Spacetime: Missing required element \"{}\" for primitive \"{}\"');\n",
                        name, primitive.name
                    )));
                    return Some(ir);
                }
            }
            PrimitiveParam::Data(name) => {
                if !args.params.contains_key(name) {
                    let mut ir = GeneratedPrimitiveIR::new();
                    ir.js_stmts.push(JsStmt::Raw(format!(
                        "console.error('Spacetime: Missing required data binding \"{}\" for primitive \"{}\"');\n",
                        name, primitive.name
                    )));
                    return Some(ir);
                }
            }
            PrimitiveParam::TypedData { name, .. } => {
                if !args.params.contains_key(name) {
                    let mut ir = GeneratedPrimitiveIR::new();
                    ir.js_stmts.push(JsStmt::Raw(format!(
                        "console.error('Spacetime: Missing required typed data \"{}\" for primitive \"{}\"');\n",
                        name, primitive.name
                    )));
                    return Some(ir);
                }
            }
        }
    }

    None
}

/// Build an EmitContext from PrimitiveArgs for placeholder resolution.
fn build_emit_context_structured(
    args: &PrimitiveArgs,
    primitive: &PrimitiveDefAst,
) -> js_emit::EmitContext {
    let mut ctx = js_emit::EmitContext::new("el");

    // Add element mappings
    for (name, expr) in &args.elements {
        ctx = ctx.with_element(name.clone(), expr.clone());
    }

    // Add parameter values
    for (name, value) in &args.params {
        // Clean up value: strip type annotations if present
        let clean_value = clean_param_value_structured(value);
        ctx = ctx.with_param(name.clone(), clean_value);
    }

    // Add output signal name mappings
    for (primitive_name, user_name) in &args.outputs {
        ctx = ctx.with_output(primitive_name.clone(), user_name.clone());
    }

    // Add parameter types from primitive definition for type-aware resolution
    for param in &primitive.params {
        if let PrimitiveParam::Typed { name, ty, .. } = param {
            let type_str = match ty {
                ParamType::Simple(s) => s.clone(),
                ParamType::Optional(s) => s.clone(),
                _ => continue,
            };
            ctx = ctx.with_param_type(name.clone(), type_str);
        }
    }

    ctx
}

/// The query-helper vocabulary installed on `ST.filters` by the
/// computed-source prelude and rewritten to `ST.filters.<name>(…)` when a
/// `sexpr-row` param calls one bare. ONE list, two readers — this const and
/// the prelude table in stdlib/primitives/data/computed-source.st must grow
/// together. (A page `@fn` of the same name still wins: fn-registry assigns
/// ST.filters unconditionally, after preludes run.)
const QUERY_HELPERS: [&str; 8] = [
    "includes", "contains", "starts", "ends", "basename", "titleize", "lower", "upper",
];

/// Lower every `sexpr`-family param at compile time and inject synthetic
/// sibling params (`<name>__fn`, `__body`, `__deps`, `__params`, `__step_fn`)
/// into the EmitContext. The synthetic params are typed `expr` so the emitter
/// splices them as code, never string-quotes them. A lowering failure is
/// recorded as an IR diagnostic (FEAT-170 turns it into a build error).
///
/// The family is DATA, declared by the primitive's param type:
///   `sexpr`      — global context: bare `$` is an error (E0957); unbound
///                  `$sig` is a dependency. `__fn = (deps…) => body`.
///   `sexpr-row`  — row context: `$` / `$.field` is the current row (var
///                  `item`); a param named exactly `$` binds the row locally.
///                  `__fn = (item, deps…) => body`; the query-helper
///                  vocabulary (QUERY_HELPERS) is in scope alongside page
///                  filters.
///
/// Absent/empty optional params inject `null` / `[]` sentinels (never skip)
/// so the template can splice every slot unconditionally.
///
/// Two body shapes get dedicated composition:
///   - Object-map form (`map: { label: $.name, kind: "photo" }`): the leaves
///     lower individually (pipe semantics are per-leaf; constants survive
///     because quotes are intact in the raw capture) and the fn returns the
///     reassembled object literal.
///   - Top-level arrow (the reduce shape `(s, $) => …`): the step fn IS the
///     author's arrow with any discovered deps appended to its param list,
///     keeping the prelude's one calling convention `step(acc, item, …deps)`.
fn inject_sexpr_synthetics(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
    helpers: Option<&std::collections::HashSet<String>>,
    ctx: &mut js_emit::EmitContext,
    result: &mut GeneratedPrimitiveIR,
) {
    use crate::emit::st_expr::{RowCtx, lower_st_expr};

    for param in &primitive.params {
        let PrimitiveParam::Typed { name, ty, .. } = param else {
            continue;
        };
        let type_str = match ty {
            ParamType::Simple(s) | ParamType::Optional(s) => s.as_str(),
            _ => continue,
        };
        let row_var = match type_str {
            "sexpr" => None,
            "sexpr-row" => Some("item"),
            "sexpr-arms" => {
                // cond_block record (derive-match): lower every embedded
                // `expr:`/`arg:` string at build time and REPLACE the param
                // value with the rewritten record (still typed `expr`, so the
                // template's `%arms` splices are unchanged).
                let value = args.params.get(name);
                let nonempty = value.is_some_and(|v| !v.trim().is_empty() && v.trim() != "null");
                if nonempty {
                    let value = value.expect("checked");
                    match crate::emit::st_expr::lower_cond_arms(value, &|n| {
                        helpers.is_some_and(|h| h.contains(n))
                    }) {
                        Ok(rewritten) => {
                            let k = name.to_string();
                            *ctx = std::mem::take(ctx)
                                .with_param(k.clone(), rewritten)
                                .with_param_type(k, "expr");
                        }
                        Err(diag) => result.diagnostics.push(diag),
                    }
                }
                continue;
            }
            _ => continue,
        };
        let key = |suffix: &str| format!("{name}{suffix}");
        let mut inject = |suffix: &str, val: String| {
            let k = key(suffix);
            *ctx = std::mem::take(ctx)
                .with_param(k.clone(), val)
                .with_param_type(k, "expr");
        };

        let empty = args
            .params
            .get(name)
            .is_none_or(|v| v.trim().is_empty() || v.trim() == "null");
        if empty {
            // Unconditional sentinels: the template splices every slot.
            inject("__fn", "null".into());
            inject("__body", "null".into());
            inject("__deps", "[]".into());
            inject("__params", String::new());
            inject("__step_fn", "null".into());
            continue;
        }
        let value = args.params.get(name).expect("checked above");

        let row_ctx = match row_var {
            Some(v) => RowCtx::Row(v.to_string()),
            None => RowCtx::Global,
        };
        let is_helper = |n: &str| {
            helpers.is_some_and(|h| h.contains(n))
                || (row_var.is_some() && QUERY_HELPERS.contains(&n))
        };

        // Lower. Object-map form: lower each leaf, reassemble.
        let lowered = if value.trim_start().starts_with('{') {
            split_object_leaves(value).and_then(|leaves| {
                let mut body = String::from("{ ");
                let mut deps: Vec<String> = Vec::new();
                let mut first = true;
                for (k, leaf) in leaves {
                    match lower_st_expr(&leaf, &row_ctx, &is_helper) {
                        Ok(low) => {
                            for d in low.deps {
                                if !deps.contains(&d) {
                                    deps.push(d);
                                }
                            }
                            if !first {
                                body.push_str(", ");
                            }
                            first = false;
                            body.push_str(&k);
                            body.push_str(": ");
                            body.push_str(&low.body);
                        }
                        Err(diag) => return Some(Err(diag)),
                    }
                }
                body.push_str(" }");
                Some(Ok(crate::emit::st_expr::StExprLowering {
                    body: format!("({body})"),
                    deps,
                }))
            })
        } else {
            None
        };
        let lowered = match lowered {
            Some(l) => l,
            None => lower_st_expr(value, &row_ctx, &is_helper),
        };

        match lowered {
            Ok(low) => {
                let deps_json = format!(
                    "[{}]",
                    low.deps
                        .iter()
                        .map(|d| format!("\"{d}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let params = low.deps.join(", ");
                let fn_src = match row_var {
                    Some(v) if params.is_empty() => format!("({v}) => {}", low.body),
                    Some(v) => format!("({v}, {params}) => {}", low.body),
                    None => format!("({params}) => {}", low.body),
                };
                let step_fn_src = compose_step_fn(&low.body, &low.deps, row_var);
                for (suffix, val) in [
                    ("__body", low.body.clone()),
                    ("__deps", deps_json),
                    ("__params", params.clone()),
                    ("__fn", fn_src),
                    ("__step_fn", step_fn_src),
                ] {
                    inject(suffix, val);
                }
            }
            Err(diag) => result.diagnostics.push(diag),
        }
    }
}

/// Compose the fold/reduce step fn. ONE calling convention for the prelude:
/// `step(acc, item, …deps)`. A top-level arrow body (the reduce shape
/// `(s, $) => …`) keeps the author's arrow with deps appended to its param
/// list; any other body is wrapped.
fn compose_step_fn(body: &str, deps: &[String], row_var: Option<&str>) -> String {
    if let Some(with_deps) = append_params_to_arrow(body, deps) {
        return with_deps;
    }
    let extra = if deps.is_empty() {
        String::new()
    } else {
        format!(", {}", deps.join(", "))
    };
    let _ = row_var; // the row var only names the author's `$`; acc/item are the protocol
    format!("(acc, item{extra}) => {body}")
}

/// If `body` is a top-level arrow, return it with `deps` appended to its
/// param list (`(s, item) => …` → `(s, item, d1, d2) => …`; `() => …` →
/// `(d1) => …`; `x => …` → `(x, d1) => …`). Otherwise None.
fn append_params_to_arrow(body: &str, deps: &[String]) -> Option<String> {
    let t = body.trim_start();
    let lead = body.len() - t.len();
    if t.starts_with('(') {
        // Depth/quote-aware scan for the paren group that ends the params:
        // it must be immediately followed by `=>`.
        let bytes = t.as_bytes();
        let mut depth = 0i32;
        let mut i = 0usize;
        let mut close = None;
        while i < bytes.len() {
            let c = bytes[i] as char;
            match c {
                '"' | '\'' | '`' => {
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == b'\\' {
                            i += 2;
                            continue;
                        }
                        if bytes[i] as char == c {
                            break;
                        }
                        i += 1;
                    }
                }
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let close = close?;
        if t[close + 1..].trim_start().starts_with("=>") {
            if deps.is_empty() {
                return Some(body.to_string());
            }
            let inner = t[1..close].trim();
            let insert = if inner.is_empty() {
                deps.join(", ")
            } else {
                format!(", {}", deps.join(", "))
            };
            return Some(format!(
                "{}{}{}",
                &body[..lead + close],
                insert,
                &body[lead + close..]
            ));
        }
        return None;
    }
    // Bare single-param form: `ident => …` (sigils already stripped).
    let ident_len = t
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(t.len());
    if ident_len > 0 && t[ident_len..].trim_start().starts_with("=>") {
        if deps.is_empty() {
            return Some(body.to_string());
        }
        return Some(format!(
            "{}({}, {}){}",
            &body[..lead],
            &t[..ident_len],
            deps.join(", "),
            &t[ident_len..]
        ));
    }
    None
}

/// Split an object-literal capture `{ k: v, k2: v2 }` into top-level
/// (key, value) leaves — depth- and quote-aware. Returns None when the text
/// is not the object-map shape (caller falls back to whole-expr lowering).
fn split_object_leaves(src: &str) -> Option<Vec<(String, String)>> {
    let t = src.trim();
    let inner = t.strip_prefix('{')?.strip_suffix('}')?;
    let bytes = inner.as_bytes();
    let mut entries = Vec::new();
    let mut depth = 0i32;
    let mut last = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '"' | '\'' | '`' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] as char == c {
                        break;
                    }
                    i += 1;
                }
            }
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                entries.push(inner[last..i].trim().to_string());
                last = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let tail = inner[last..].trim();
    if !tail.is_empty() {
        entries.push(tail.to_string());
    }
    let mut leaves = Vec::new();
    for e in entries {
        // First top-level `:` splits key from value.
        let b = e.as_bytes();
        let mut d = 0i32;
        let mut j = 0usize;
        let mut split = None;
        while j < b.len() {
            let c = b[j] as char;
            match c {
                '"' | '\'' | '`' => {
                    j += 1;
                    while j < b.len() {
                        if b[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if b[j] as char == c {
                            break;
                        }
                        j += 1;
                    }
                }
                '(' | '[' | '{' => d += 1,
                ')' | ']' | '}' => d -= 1,
                ':' if d == 0 => {
                    split = Some(j);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let at = split?;
        leaves.push((e[..at].trim().to_string(), e[at + 1..].trim().to_string()));
    }
    Some(leaves)
}

/// Clean up a parameter value by stripping type annotations and escaping if needed.
fn clean_param_value_structured(value: &str) -> String {
    // Strip type annotations if present (e.g., "varName: Type[]" -> "varName").
    // A quoted string literal (from captured_to_js on a `$x:string` capture) is
    // OPAQUE data — its interior `:` is part of the string, never a value/type
    // separator. Exempt it (alongside object/array literals) so a test name like
    // "iso A: drives render" is NOT truncated at the first colon (BUG-109).
    if !value.starts_with('{')
        && !value.starts_with('[')
        && !value.starts_with('"')
        && !value.starts_with('\'')
        && let Some(colon_pos) = value.find(':')
    {
        let after_colon = value[colon_pos + 1..].trim();
        if after_colon
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return value[..colon_pos].trim().to_string();
        }
    }
    value.to_string()
}

/// Process a primitive body using the structured parser path.
fn process_primitive_body_structured(
    body: &PrimitiveBody,
    args: &PrimitiveArgs,
    ctx: &js_emit::EmitContext,
    opts: &EmitOptions,
    result: &mut GeneratedPrimitiveIR,
) {
    // Process direct emit blocks
    for emit_block in &body.emit_blocks {
        process_emit_block_structured(emit_block, args, ctx, opts, result);
    }

    // Process %if blocks
    for if_block in &body.if_blocks {
        if evaluate_condition(&if_block.condition, args) {
            // Condition is true - process the then body
            process_primitive_body_structured(&if_block.then_body, args, ctx, opts, result);
        } else if let Some(else_body) = &if_block.else_body {
            // Condition is false - process the else body
            process_primitive_body_structured(else_body, args, ctx, opts, result);
        }
    }

    // Handle standalone cleanup in body
    if let Some(cleanup_content) = &body.cleanup {
        let resolved = parse_and_resolve_js_with_cleanup(cleanup_content, ctx, opts);
        result.cleanup_stmts.extend(resolved.main);
        result.diagnostics.extend(resolved.diagnostics);
    }

    // Collect exports from nested bodies
    for export in &body.exports {
        if !result.exports.contains(&export.name) {
            result.exports.push(export.name.clone());
        }
    }
}

/// Process a single emit block using the structured parser.
/// Substitute `%$name`, `%$name.html`, and `%$name.source` capture references in
/// a `%emit html` template (FEAT-082). Param values arrive JS-quoted (from
/// `captured_to_js`); html wants raw text, so quotes are stripped. `.source`
/// additionally HTML-escapes the value so a captured block renders as visible
/// source code in a `<pre>` rather than as live markup. Longest-match first
/// (`.source`/`.html` before the bare `%$name`).
fn substitute_html_template(template: &str, args: &PrimitiveArgs) -> String {
    fn strip_quotes(s: &str) -> String {
        let t = s.trim();
        if t.len() >= 2
            && ((t.starts_with('"') && t.ends_with('"'))
                || (t.starts_with('\'') && t.ends_with('\'')))
        {
            // Unescape the common JS string escapes the quoting added.
            t[1..t.len() - 1]
                .replace("\\\"", "\"")
                .replace("\\'", "'")
                .replace("\\n", "\n")
                .replace("\\\\", "\\")
        } else {
            t.to_string()
        }
    }
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
    let mut out = template.to_string();
    for (name, raw) in &args.params {
        let value = strip_quotes(raw);
        // Order matters: replace the qualified forms before the bare name.
        out = out.replace(&format!("%${name}.source"), &html_escape(&value));
        out = out.replace(&format!("%${name}.html"), &value);
        out = out.replace(&format!("%${name}"), &value);
    }
    out
}
fn process_emit_block_structured(
    emit_block: &EmitBlock,
    args: &PrimitiveArgs,
    ctx: &js_emit::EmitContext,
    opts: &EmitOptions,
    result: &mut GeneratedPrimitiveIR,
) {
    match emit_block.lang {
        EmitLang::Js => {
            // BUG-252: refuse a bind that lowers to `var x = x;` (see below).
            validate_no_self_shadowing_bind(&emit_block.content, ctx, result);
            // Parse and resolve the emit block content (handles %cleanup internally)
            let resolved = parse_and_resolve_js_with_cleanup(&emit_block.content, ctx, opts);
            result.js_stmts.extend(resolved.main);
            result.cleanup_stmts.extend(resolved.cleanup);
            result.diagnostics.extend(resolved.diagnostics);
        }
        EmitLang::Css => {
            // Guard: if the CSS template references %styles but styles aren't available
            // (e.g., reactive-only $matches usage) or empty (e.g., top-level @media with
            // only child selectors), skip this emit block.
            if emit_block.content.contains("%styles") {
                let styles_available = args
                    .css_overrides
                    .get("styles")
                    .map(|s| !s.trim().is_empty())
                    .or_else(|| {
                        args.params
                            .get("styles")
                            .map(|s| !s.trim().is_empty() && s != "[]")
                    })
                    .unwrap_or(false);
                if !styles_available {
                    return;
                }
            }

            // When no selector (top-level usage), unwrap %self { ... } from the template
            // so nested selectors go directly into the @media block.
            let effective_content = if args.params.get("self").is_some_and(|s| s.is_empty())
                && emit_block.content.contains("%self")
            {
                unwrap_empty_self_block(&emit_block.content)
            } else {
                emit_block.content.clone()
            };

            // Parse CSS and resolve placeholders via structured IR
            let css_expr = css_parser::parse_emit_css(&effective_content);

            // Strip JS quoting from params — CSS doesn't use JS string formatting.
            // captured_to_js wraps strings in "..." but CSS params need raw values.
            // CSS overrides (e.g., StyleProperties as CSS declarations) take precedence.
            let mut css_params: HashMap<String, String> = args
                .params
                .iter()
                .map(|(k, v)| {
                    // Prefer CSS-specific override if available (e.g., StyleProperties → CSS declarations)
                    if let Some(css_val) = args.css_overrides.get(k) {
                        return (k.clone(), css_val.clone());
                    }
                    let trimmed = v.trim();
                    let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
                    {
                        trimmed[1..trimmed.len() - 1].to_string()
                    } else {
                        trimmed.to_string()
                    };
                    // Unescape JS string escaping for CSS context.
                    // captured_to_js / escape_js_string escapes quotes (' → \', " → \")
                    // which is correct for JS but produces invalid CSS (e.g., syntax: \'<color>\').
                    let unescaped = crate::utils::unescape_js_string(&unquoted);
                    (k.clone(), unescaped)
                })
                .collect();

            // Inject CSS overrides for keys not present in the bound params (e.g.,
            // styles captured by the macro body and threaded via css_styles).
            for (k, v) in &args.css_overrides {
                css_params.entry(k.clone()).or_insert_with(|| v.clone());
            }

            let css_ctx = css_emit::CssContext {
                params: css_params,
                elements: args.elements.clone(),
            };
            let resolved = css_emit::resolve_expr(&css_expr, &css_ctx);

            // Validate resolved CSS with lightningcss
            #[cfg(feature = "lightningcss")]
            {
                let css_string = css_emit::emit(&resolved, &EmitOptions::default());
                if let Err(err) = lightningcss::stylesheet::StyleSheet::parse(
                    &css_string,
                    lightningcss::stylesheet::ParserOptions::default(),
                ) {
                    log::warn!("Generated CSS from %emit css may be invalid: {}", err);
                }
            }

            if !matches!(&resolved, CssExpr::Raw(s) if s.trim().is_empty()) {
                result.css_exprs.push(resolved);
            }
        }
        EmitLang::Glsl => {
            // GLSL is not processed here - handled by shader compiler
        }
        EmitLang::Html => {
            // FEAT-082: userland markup macro. Substitute capture references in
            // the html template and collect the result for splicing into the
            // page body. Supported references (raw, NOT JS-quoted):
            //   %$name          -> the param's raw text value
            //   %$name.html     -> a captured block's raw markup
            //   %$name.source   -> a captured block's markup, HTML-escaped
            //                      (the dual-render "show the source" pane)
            let html = substitute_html_template(&emit_block.content, args);
            result.html.push(html);
        }
        EmitLang::BuildJs => {
            // Build-time JS: process identically to runtime JS
            // but collect into build_js_stmts for execution at build time
            let resolved = parse_and_resolve_js_with_cleanup(&emit_block.content, ctx, opts);
            result.build_js_stmts.extend(resolved.main);
            // Build-time JS has no cleanup concept, but collect diagnostics
            result.diagnostics.extend(resolved.diagnostics);
        }
        EmitLang::PreludeJs => {
            // Prelude JS: emitted once per primitive type, no element context.
            // Validate: no %&el, %yield, %cleanup allowed.
            validate_prelude_content(&emit_block.content, "prelude-js", result);
            let resolved = parse_and_resolve_js_with_cleanup(&emit_block.content, ctx, opts);
            if !resolved.cleanup.is_empty() {
                result
                    .diagnostics
                    .push(crate::diagnostics::Diagnostic::error(
                        crate::diagnostics::DiagnosticCode::E0800,
                        "%cleanup is not allowed in %emit prelude-js blocks".to_string(),
                    ));
            }
            result.prelude_js_stmts.extend(resolved.main);
            result.diagnostics.extend(resolved.diagnostics);
        }
        EmitLang::PreludeCss => {
            // Prelude CSS: emitted once per primitive type.
            validate_prelude_content(&emit_block.content, "prelude-css", result);
            let css_expr = css_parser::parse_emit_css(&emit_block.content);
            // Prelude CSS uses no params — emit as-is.
            if !matches!(&css_expr, CssExpr::Raw(s) if s.trim().is_empty()) {
                result.prelude_css_exprs.push(css_expr);
            }
        }
    }
}

/// Refuse a JS OBJECT LITERAL handed to a param declared to hold a SCALAR.
///
/// Capture types that carry structure (`signal_path` is `{base, facet?}`) lower
/// to a JS object literal. When one reaches a param declared `binding` /
/// `string` / `number`, the primitive's own code does the only thing JS can:
/// it stringifies, and `String({base:"x"})` is `"[object Object]"`. The runtime
/// then watches — or fetches, or renders — a name that cannot exist. Nothing
/// throws; the feature is simply inert.
///
/// That is BUG-252's second break: one lowering branch normalized the map to a
/// flat binding and the other spliced it raw, and the difference was invisible
/// until a browser did nothing.
///
/// The mismatch is decidable at this boundary, because the param's declared
/// type says what it can hold. Params declared `any`/`object`/`array` are the
/// legitimate homes for structured data (arms, keyframes, records) and are not
/// touched.
fn validate_scalar_params_are_not_objects(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
    result: &mut GeneratedPrimitiveIR,
) {
    for param in &primitive.params {
        let PrimitiveParam::Typed { name, ty, .. } = param else {
            continue;
        };
        let type_str = match ty {
            ParamType::Simple(s) | ParamType::Optional(s) => s.as_str(),
            // Arrays and unions are structural by declaration.
            _ => continue,
        };
        // Only types that name a single scalar value can be violated this way.
        if !matches!(type_str, "binding" | "string" | "number" | "time" | "bool") {
            continue;
        }
        let Some(value) = args.params.get(name) else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            result
                .diagnostics
                .push(crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0800,
                    format!(
                        "param `{name}` is declared `{type_str}` but received an \
                         object literal `{trimmed}`. It would stringify to \
                         \"[object Object]\" at runtime and silently do nothing. \
                         Normalize the capture to a scalar before binding it \
                         (a structured capture like `$x:signal_path` carries \
                         `{{base, facet}}` — pass `base`), or declare the param \
                         `any` if it is meant to hold structured data."
                    ),
                ));
        }
    }
}

/// Refuse an element bind that lowers to a SELF-ASSIGNMENT (`var el = el;`).
///
/// A primitive writes `var el = %&el;` to name its element locally. `%&el`
/// substitutes the SPLICE SITE's expression — and the splice site is usually
/// the selector-init closure `const init = function(el) { … }`, whose parameter
/// is also called `el`. The bind then lowers to `var el = el;`: under `var`
/// hoisting the inner declaration shadows the parameter BEFORE the assignment
/// reads it, so the variable is `undefined` and every `if (!el) return;` guard
/// below it fires. The emitted source looks perfect and the primitive does
/// nothing.
///
/// This is not hypothetical: it silently killed `@on $signal` arms and
/// `@handle` (BUG-252), and `stdlib/dnd/primitives/drag-zones.st` had carried a
/// comment warning about it for two waves — which did not stop two other
/// primitives from being written the same way. A comment cannot constrain a
/// file that does not contain it; the compiler can.
///
/// `var x = x;` has no valid meaning in any program, so this is an ERROR rather
/// than a lint: there is nothing to weigh, and the fix is always the same —
/// bind to a different name.
fn validate_no_self_shadowing_bind(
    content: &str,
    ctx: &js_emit::EmitContext,
    result: &mut GeneratedPrimitiveIR,
) {
    for line in content.lines() {
        let trimmed = line.trim();
        // `var|let|const NAME = %&PARAM;` — the only shape that can self-shadow.
        let Some(rest) = trimmed
            .strip_prefix("var ")
            .or_else(|| trimmed.strip_prefix("let "))
            .or_else(|| trimmed.strip_prefix("const "))
        else {
            continue;
        };
        let Some((lhs, rhs)) = rest.split_once('=') else {
            continue;
        };
        let bound_name = lhs.trim();
        let rhs = rhs.trim().trim_end_matches(';').trim();
        let Some(param) = rhs.strip_prefix("%&") else {
            continue;
        };
        // Resolve what `%&param` will actually become at THIS splice site.
        let Some(substituted) = ctx.elements.get(param) else {
            continue;
        };
        if bound_name == substituted.trim() {
            result
                .diagnostics
                .push(crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0800,
                    format!(
                        "`{trimmed}` lowers to `{bound_name} = {bound_name}` — a \
                         self-assignment that shadows the outer `{bound_name}` and \
                         leaves it undefined, silently disabling everything below \
                         it. Bind the element to a different name (e.g. `node`)."
                    ),
                ));
        }
    }
}

/// Validate that prelude block content doesn't use element-context features.
/// Preludes have no %&el, %param placeholders, %yield, or %cleanup.
fn validate_prelude_content(content: &str, block_kind: &str, result: &mut GeneratedPrimitiveIR) {
    let forbidden = [
        ("%&el", "%&el (element reference)"),
        ("%yield", "%yield (signal emission)"),
    ];
    for (marker, desc) in &forbidden {
        if content.contains(marker) {
            result
                .diagnostics
                .push(crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0800,
                    format!(
                        "{} is not allowed in %emit {} blocks — preludes have no element context",
                        desc, block_kind
                    ),
                ));
        }
    }
}

/// When `%self` is empty (top-level primitive usage with no parent selector),
/// unwrap the `%self { INNER }` wrapper so INNER goes directly into the output.
/// This lets `@media` at the top level emit nested selectors without an empty wrapper.
fn unwrap_empty_self_block(template: &str) -> String {
    // Find `%self` in the template
    if let Some(self_pos) = template.find("%self") {
        let after_self = &template[self_pos + 5..]; // len("%self") == 5
        // Find the opening `{` after %self (skip whitespace)
        if let Some(brace_offset) = after_self.find('{') {
            let open_pos = self_pos + 5 + brace_offset;
            // Find matching closing `}` using brace-depth counting
            let mut depth = 1;
            let mut close_pos = None;
            for (i, ch) in template[open_pos + 1..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close_pos = Some(open_pos + 1 + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(close_pos) = close_pos {
                // Replace `%self { INNER }` with just `INNER`
                let inner = &template[open_pos + 1..close_pos];
                let mut result = String::with_capacity(template.len());
                result.push_str(&template[..self_pos]);
                result.push_str(inner.trim());
                result.push_str(&template[close_pos + 1..]);
                return result;
            }
        }
    }
    // Fallback: return unchanged
    template.to_string()
}

/// Result of parsing and resolving JS content
struct ResolvedJs {
    main: Vec<JsStmt>,
    cleanup: Vec<JsStmt>,
    diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

/// Parse JavaScript emit content and resolve placeholders.
///
/// This function:
/// 1. Parses the content using js_parser::parse_emit_js_with_cleanup()
/// 2. Resolves all Placeholder nodes and expands ForLoopMeta via resolve_stmts
/// 3. Returns both main and cleanup statements along with any diagnostics
fn parse_and_resolve_js_with_cleanup(
    content: &str,
    ctx: &js_emit::EmitContext,
    opts: &EmitOptions,
) -> ResolvedJs {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};

    let mut diagnostics = Vec::new();

    match js_parser::parse_emit_js_with_cleanup(content) {
        Ok(parsed) => {
            // Resolve all placeholders and expand ForLoopMeta
            let main = match js_emit::resolve_stmts(&parsed.main, ctx, opts) {
                Ok(stmts) => stmts,
                Err(diag) => {
                    // Collect the diagnostic for the error modal
                    diagnostics.push(diag);
                    Vec::new()
                }
            };
            let cleanup = match js_emit::resolve_stmts(&parsed.cleanup, ctx, opts) {
                Ok(stmts) => stmts,
                Err(diag) => {
                    diagnostics.push(diag);
                    Vec::new()
                }
            };
            ResolvedJs {
                main,
                cleanup,
                diagnostics,
            }
        }
        Err(e) => {
            // Parse error - create a diagnostic
            let trimmed = content.trim();
            if trimmed.is_empty() {
                ResolvedJs {
                    main: Vec::new(),
                    cleanup: Vec::new(),
                    diagnostics: Vec::new(),
                }
            } else {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::E0800,
                    format!("JS emit parse error: {}", e),
                ));
                ResolvedJs {
                    main: Vec::new(),
                    cleanup: Vec::new(),
                    diagnostics,
                }
            }
        }
    }
}

/// Evaluate a MetaIfCondition against the provided args
fn evaluate_condition(condition: &MetaIfCondition, args: &PrimitiveArgs) -> bool {
    match condition {
        MetaIfCondition::Truthy(var) => {
            // Check if var exists in params and is truthy
            if let Some(value) = args.params.get(var) {
                is_truthy(value)
            } else {
                false
            }
        }
        MetaIfCondition::Falsy(var) => {
            // Check if var is falsy
            if let Some(value) = args.params.get(var) {
                !is_truthy(value)
            } else {
                true // undefined is falsy
            }
        }
        MetaIfCondition::Equals(var, expected) => {
            if let Some(value) = args.params.get(var) {
                // Compare values (strip quotes for comparison)
                let clean_value = value.trim_matches('"').trim_matches('\'');
                clean_value == expected
            } else {
                false
            }
        }
        MetaIfCondition::NotEquals(var, expected) => {
            if let Some(value) = args.params.get(var) {
                let clean_value = value.trim_matches('"').trim_matches('\'');
                clean_value != expected
            } else {
                true
            }
        }
        MetaIfCondition::LessThan(var, expected) => {
            compare_numeric(var, expected, args, |a, b| a < b)
        }
        MetaIfCondition::GreaterThan(var, expected) => {
            compare_numeric(var, expected, args, |a, b| a > b)
        }
        MetaIfCondition::LessThanOrEqual(var, expected) => {
            compare_numeric(var, expected, args, |a, b| a <= b)
        }
        MetaIfCondition::GreaterThanOrEqual(var, expected) => {
            compare_numeric(var, expected, args, |a, b| a >= b)
        }
        MetaIfCondition::And(left, right) => {
            evaluate_condition(left, args) && evaluate_condition(right, args)
        }
        MetaIfCondition::Or(left, right) => {
            evaluate_condition(left, args) || evaluate_condition(right, args)
        }
    }
}

/// Check if a JS literal value is truthy
fn is_truthy(value: &str) -> bool {
    match value.trim() {
        "false" | "null" | "undefined" | "0" | "\"\"" | "''" | "" => false,
        _ => true,
    }
}

/// Compare numeric values
fn compare_numeric<F>(var: &str, expected: &str, args: &PrimitiveArgs, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    if let Some(value) = args.params.get(var) {
        let a = value.parse::<f64>().unwrap_or(0.0);
        let b = expected.parse::<f64>().unwrap_or(0.0);
        cmp(a, b)
    } else {
        false
    }
}

/// Parsed pattern match expression
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPatternMatch {
    /// Variable name (without $)
    pub variable: String,
    /// Variant name
    pub variant: String,
    /// Binding names (without $)
    pub bindings: Vec<String>,
}

/// Parse a pattern match string format: $var:Variant{bind1,bind2} or $var:Variant
///
/// This format is produced by the parser conversion layer for `@state(when: $var is Variant { $bind1, $bind2 })`
pub fn parse_pattern_match(condition: &str) -> Option<ParsedPatternMatch> {
    // Check if it starts with $ and contains :
    if !condition.starts_with('$') || !condition.contains(':') {
        return None;
    }

    // Remove leading $
    let rest = &condition[1..];

    // Split on :
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let variable = parts[0].to_string();
    let variant_part = parts[1];

    // Check for bindings: Variant{bind1,bind2}
    if let Some(brace_start) = variant_part.find('{')
        && let Some(brace_end) = variant_part.find('}')
    {
        let variant = variant_part[..brace_start].to_string();
        let bindings_str = &variant_part[brace_start + 1..brace_end];
        let bindings: Vec<String> = bindings_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Some(ParsedPatternMatch {
            variable,
            variant,
            bindings,
        });
    }

    // No bindings: just Variant
    Some(ParsedPatternMatch {
        variable,
        variant: variant_part.to_string(),
        bindings: vec![],
    })
}

/// Generate CSS for state-based styling
pub fn generate_state_css(
    selector: &str,
    state_name: &str,
    properties: &[(String, String)],
) -> String {
    let mut css = String::new();

    css.push_str(&format!(
        "{}[data-st-state=\"{}\"] {{\n",
        selector, state_name
    ));

    for (property, value) in properties {
        css.push_str(&format!("  {}: {};\n", property, value));
    }

    css.push_str("}\n");

    css
}

/// Generate CSS for pattern match state (typed union)
///
/// For `@state(when: $ws is Connected { $send })`, generates:
/// ```css
/// .selector[data-st-ws-type="Connected"] { ... }
/// ```
pub fn generate_pattern_match_css(
    selector: &str,
    pattern: &ParsedPatternMatch,
    properties: &[(String, String)],
) -> String {
    let mut css = String::new();

    css.push_str(&format!(
        "{}[data-st-{}-type=\"{}\"] {{\n",
        selector, pattern.variable, pattern.variant
    ));

    for (property, value) in properties {
        css.push_str(&format!("  {}: {};\n", property, value));
    }

    css.push_str("}\n");

    css
}

/// Generate JS for pattern match state binding
///
/// For `@state(when: $ws is Connected { $send, $received })`, generates:
/// ```js
/// ST.watchTypedUnion(el, 'ws', 'Connected', ['send', 'received']);
/// ```
pub fn generate_pattern_match_js(pattern: &ParsedPatternMatch) -> String {
    if pattern.bindings.is_empty() {
        format!(
            "ST.watchTypedUnion(el, '{}', '{}', []);\n",
            pattern.variable, pattern.variant
        )
    } else {
        let bindings_str = pattern
            .bindings
            .iter()
            .map(|b| format!("'{}'", b))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "ST.watchTypedUnion(el, '{}', '{}', [{}]);\n",
            pattern.variable, pattern.variant, bindings_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: emits IR to strings for assertion convenience
    #[allow(dead_code)]
    struct EmittedPrimitive {
        setup: String,
        css: String,
        cleanup: Option<String>,
        exports: Vec<String>,
        diagnostics: Vec<crate::diagnostics::Diagnostic>,
    }

    fn emit_primitive(primitive: &PrimitiveDefAst, args: &PrimitiveArgs) -> EmittedPrimitive {
        let ir = generate_primitive_ir_structured(primitive, args, None, None);
        emit_ir(ir)
    }

    fn emit_ir(ir: GeneratedPrimitiveIR) -> EmittedPrimitive {
        let opts = EmitOptions::pretty();
        EmittedPrimitive {
            setup: js_emit::emit_stmts(&ir.js_stmts, &opts).unwrap_or_default(),
            css: css_emit::emit_all(&ir.css_exprs, &opts),
            cleanup: if ir.cleanup_stmts.is_empty() {
                None
            } else {
                Some(js_emit::emit_stmts(&ir.cleanup_stmts, &opts).unwrap_or_default())
            },
            exports: ir.exports,
            diagnostics: ir.diagnostics,
        }
    }

    #[test]
    fn test_generate_state_css() {
        let css = generate_state_css(
            ".card",
            "dragging",
            &[
                ("cursor".to_string(), "grabbing".to_string()),
                ("user-select".to_string(), "none".to_string()),
            ],
        );
        assert!(css.contains(".card[data-st-state=\"dragging\"]"));
        assert!(css.contains("cursor: grabbing"));
        assert!(css.contains("user-select: none"));
    }

    #[test]
    fn test_clean_param_value_strips_type_annotation() {
        // A `name: Type` annotation IS stripped to the bare name.
        assert_eq!(clean_param_value_structured("items: Item[]"), "items");
        assert_eq!(clean_param_value_structured("x: Foo"), "x");
    }

    #[test]
    fn test_clean_param_value_preserves_quoted_string_with_colon() {
        // BUG-109: a quoted string literal is opaque data — an interior `:` is
        // part of the string and MUST NOT truncate it (even when followed by an
        // uppercase token that looks like a type). Regression guard for the
        // `@test "name: ..."` whole-file SyntaxError.
        assert_eq!(
            clean_param_value_structured("\"iso A: ST.setData drives render\""),
            "\"iso A: ST.setData drives render\""
        );
        assert_eq!(
            clean_param_value_structured("\"a: b: c GET /api: 200\""),
            "\"a: b: c GET /api: 200\""
        );
        // Single-quoted likewise.
        assert_eq!(
            clean_param_value_structured("'time: 12:30 PM'"),
            "'time: 12:30 PM'"
        );
        // Lowercase-after-colon was already safe (not a "type"); keep intact.
        assert_eq!(
            clean_param_value_structured("\"key: value\""),
            "\"key: value\""
        );
    }

    #[test]
    fn test_full_primitive_transform() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![
                PrimitiveParam::Element("el".to_string()),
                PrimitiveParam::Typed {
                    name: "threshold".to_string(),
                    ty: ParamType::Simple("number".to_string()),
                    default: Some(ParamDefault::Number(0.5)),
                },
            ],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: r#"
                        const opts = { threshold: %threshold };
                        const obs = new IntersectionObserver((entries) => {
                            %yield entries[0].isIntersecting -> $visible;
                        }, opts);
                        obs.observe(%&el);
                        %cleanup { obs.disconnect(); }
                    "#
                    .to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "visible".to_string(),
                    type_expr: ExportTypeExpr::Simple("bool".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new()
            .element("el", "el")
            .param("threshold", "0.5");

        let result = emit_primitive(&primitive, &args);

        // Check for ST.set call with either quote style
        assert!(
            result
                .setup
                .contains("ST.set(el, 'visible', entries[0].isIntersecting)")
                || result
                    .setup
                    .contains("ST.set(el, \"visible\", entries[0].isIntersecting)"),
            "Expected ST.set call. Got:\n{}",
            result.setup
        );
        assert!(result.setup.contains("threshold: 0.5"));
        assert!(result.cleanup.is_some());
        assert!(result.cleanup.unwrap().contains("obs.disconnect()"));
        assert!(result.exports.contains(&"visible".to_string()));
    }

    #[test]
    fn test_css_emit_block() {
        let primitive = PrimitiveDefAst {
            name: "animate".to_string(),
            params: vec![
                PrimitiveParam::Element("el".to_string()),
                PrimitiveParam::Typed {
                    name: "property".to_string(),
                    ty: ParamType::Simple("string".to_string()),
                    default: None,
                },
            ],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Css,
                    content: r#"
                        .animated { %property: var(--st-progress); }
                    "#
                    .to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new()
            .element("el", "el")
            .param("property", "opacity");

        let result = emit_primitive(&primitive, &args);

        assert!(result.css.contains("opacity: var(--st-progress)"));
        assert!(result.setup.is_empty());
    }

    #[test]
    fn test_if_block_true_condition() {
        let primitive = PrimitiveDefAst {
            name: "animate".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "easing".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: Some(ParamDefault::String("linear".to_string())),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![PrimitiveIfBlock {
                    condition: MetaIfCondition::Equals("easing".to_string(), "linear".to_string()),
                    then_body: PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Css,
                            content: "transition: all 0.3s linear;".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    },
                    else_body: Some(PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Js,
                            content: "el.style.transition = 'all 0.3s ease';".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    }),
                    span: crate::parser::SourceSpan::default(),
                }],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Condition is true (easing == "linear")
        let args = PrimitiveArgs::new().param("easing", "\"linear\"");
        let result = emit_primitive(&primitive, &args);

        assert!(result.css.contains("transition: all 0.3s linear"));
        assert!(result.setup.is_empty());
    }

    #[test]
    fn test_if_block_false_condition() {
        let primitive = PrimitiveDefAst {
            name: "animate".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "easing".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: Some(ParamDefault::String("linear".to_string())),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![PrimitiveIfBlock {
                    condition: MetaIfCondition::Equals("easing".to_string(), "linear".to_string()),
                    then_body: PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Css,
                            content: "transition: all 0.3s linear;".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    },
                    else_body: Some(PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Js,
                            content: "el.style.transition = 'all 0.3s ease';".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    }),
                    span: crate::parser::SourceSpan::default(),
                }],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Condition is false (easing == "ease-out")
        let args = PrimitiveArgs::new().param("easing", "\"ease-out\"");
        let result = emit_primitive(&primitive, &args);

        assert!(result.css.is_empty());
        assert!(
            result
                .setup
                .contains("el.style.transition = 'all 0.3s ease'")
        );
    }

    #[test]
    fn test_if_block_truthy_condition() {
        let primitive = PrimitiveDefAst {
            name: "conditional".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "enabled".to_string(),
                ty: ParamType::Simple("bool".to_string()),
                default: Some(ParamDefault::Bool(true)),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![PrimitiveIfBlock {
                    condition: MetaIfCondition::Truthy("enabled".to_string()),
                    then_body: PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Js,
                            content: "el.classList.add('enabled');".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    },
                    else_body: None,
                    span: crate::parser::SourceSpan::default(),
                }],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // enabled = true
        let args = PrimitiveArgs::new().param("enabled", "true");
        let result = emit_primitive(&primitive, &args);
        assert!(result.setup.contains("el.classList.add('enabled')"));

        // enabled = false
        let args_false = PrimitiveArgs::new().param("enabled", "false");
        let result_false = emit_primitive(&primitive, &args_false);
        assert!(result_false.setup.is_empty());
    }

    #[test]
    fn test_for_loop_expansion() {
        use crate::emit::EmitOptions;

        let code = r#"%for $b in %bindings {
            el.querySelector('%$b.sel').textContent = item.%$b.prop;
        }"#;

        let bindings_json = r#"[{"sel":".name","prop":"name"},{"sel":".price","prop":"price"}]"#;

        // Parse with structured parser
        let stmts = js_parser::parse_emit_js(code).unwrap();

        // Resolve with context
        let ctx = js_emit::EmitContext::new("el").with_param("bindings", bindings_json);
        let opts = EmitOptions::default();
        let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();

        // Emit to string
        let result = js_emit::emit_stmts(&resolved, &opts).unwrap();

        assert!(
            result.contains("el.querySelector(\".name\").textContent = item.name;")
                || result.contains("el.querySelector('.name').textContent = item.name;"),
            "Expected .name binding, got: {}",
            result
        );
        assert!(
            result.contains("el.querySelector(\".price\").textContent = item.price;")
                || result.contains("el.querySelector('.price').textContent = item.price;"),
            "Expected .price binding, got: {}",
            result
        );
        assert!(
            !result.contains("%for"),
            "Should not contain %for, got: {}",
            result
        );
        assert!(
            !result.contains("%$b"),
            "Should not contain %$b, got: {}",
            result
        );
    }

    #[test]
    fn test_for_loop_nested_braces() {
        use crate::emit::EmitOptions;

        let code = r#"items.forEach((item, i) => {
            %for $b in %bindings {
                if (item.%$b.prop) {
                    el.querySelector('%$b.sel').textContent = item.%$b.prop;
                }
            }
        });"#;

        let bindings_json = r#"[{"sel":".name","prop":"name"}]"#;

        let stmts = js_parser::parse_emit_js(code).unwrap();
        let ctx = js_emit::EmitContext::new("el").with_param("bindings", bindings_json);
        let opts = EmitOptions::default();
        let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();
        let result = js_emit::emit_stmts(&resolved, &opts).unwrap();

        assert!(
            result.contains("item.name"),
            "Expected item.name, got: {}",
            result
        );
    }

    #[test]
    fn test_for_loop_empty_array() {
        use crate::emit::EmitOptions;

        let code = r#"%for $b in %bindings { el.textContent = item.%$b.prop; }"#;

        let stmts = js_parser::parse_emit_js(code).unwrap();
        let ctx = js_emit::EmitContext::new("el").with_param("bindings", "[]");
        let opts = EmitOptions::default();
        let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();
        let result = js_emit::emit_stmts(&resolved, &opts).unwrap();

        // Empty array should produce empty output (no iterations)
        assert!(
            !result.contains("el.textContent"),
            "Should be empty for empty array, got: {}",
            result
        );
    }

    #[test]
    fn test_for_loop_with_other_params() {
        use crate::emit::EmitOptions;

        let code = r#"const container = %&container;
%for $b in %bindings {
    container.querySelector('%$b.sel').textContent = item.%$b.prop;
}"#;

        let bindings_json = r#"[{"sel":".title","prop":"title"}]"#;

        let stmts = js_parser::parse_emit_js(code).unwrap();
        let ctx = js_emit::EmitContext::new("el")
            .with_param("bindings", bindings_json)
            .with_element("container", "containerEl");
        let opts = EmitOptions::default();
        let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();
        let result = js_emit::emit_stmts(&resolved, &opts).unwrap();

        assert!(
            result.contains("const container = containerEl;"),
            "Expected container decl, got: {}",
            result
        );
        assert!(
            result.contains(".title") && result.contains("item.title"),
            "Expected .title binding, got: {}",
            result
        );
    }

    // =========================================================================
    // Missing Required Parameter Tests
    // =========================================================================

    #[test]
    fn test_missing_required_param_errors() {
        // Primitive with required param 'url' (no default)
        let primitive = PrimitiveDefAst {
            name: "fetch".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "url".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None, // Required - no default
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "fetch('%url');".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Call without providing 'url'
        let args = PrimitiveArgs::new();
        let result = emit_primitive(&primitive, &args);

        // Should contain error, not raw %url
        assert!(
            result.setup.contains("console.error"),
            "Expected console.error but got: {}",
            result.setup
        );
        assert!(result.setup.contains("Missing required parameter"));
        assert!(result.setup.contains("url"));
        assert!(
            !result.setup.contains("fetch('%url')"),
            "Should NOT contain raw %url placeholder"
        );
    }

    #[test]
    fn test_optional_param_gets_null() {
        let primitive = PrimitiveDefAst {
            name: "maybe".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "value".to_string(),
                ty: ParamType::Optional("string".to_string()),
                default: None, // Optional type, no default
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "console.log(%value);".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new();
        let result = emit_primitive(&primitive, &args);

        // Optional param should become null, not stay as %value
        assert!(
            !result.setup.contains("%value"),
            "Should NOT contain raw %value placeholder"
        );
        assert!(
            result.setup.contains("null"),
            "Optional param should become null"
        );
    }

    #[test]
    fn test_default_param_used() {
        let primitive = PrimitiveDefAst {
            name: "greet".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "name".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: Some(ParamDefault::String("World".to_string())),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "console.log('Hello %name');".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new();
        let result = emit_primitive(&primitive, &args);

        // Default should be substituted
        assert!(
            !result.setup.contains("%name"),
            "Should NOT contain raw %name placeholder"
        );
        assert!(
            result.setup.contains("World"),
            "Default value should be used"
        );
    }

    #[test]
    fn test_missing_element_param_errors() {
        let primitive = PrimitiveDefAst {
            name: "click".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "%&el.addEventListener('click', handler);".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Call without providing element
        let args = PrimitiveArgs::new();
        let result = emit_primitive(&primitive, &args);

        // Should contain error
        assert!(
            result.setup.contains("console.error"),
            "Expected console.error but got: {}",
            result.setup
        );
        assert!(result.setup.contains("Missing required element"));
    }

    #[test]
    fn test_provided_param_works() {
        let primitive = PrimitiveDefAst {
            name: "fetch".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "url".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "fetch('%url');".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Call WITH providing 'url'
        let args = PrimitiveArgs::new().param("url", "/api/data");
        let result = emit_primitive(&primitive, &args);

        // Should contain the URL, not error
        assert!(!result.setup.contains("console.error"));
        assert!(result.setup.contains("/api/data"));
        assert!(!result.setup.contains("%url"));
    }

    // =========================================================================
    // Structured Parser Integration Tests
    // =========================================================================

    #[test]
    fn test_js_parser_simple() {
        use crate::ir::{DeclKind, JsExpr, JsLit};

        let stmts = js_parser::parse_emit_js("const x = 42;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(*kind, DeclKind::Const);
                assert_eq!(name, "x");
                assert!(matches!(init, Some(JsExpr::Lit(JsLit::Number(n))) if *n == 42.0));
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn test_js_parser_with_placeholder() {
        use crate::ir::{JsExpr, Marker};

        let stmts = js_parser::parse_emit_js("const fps = %fps;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { name, init, .. } => {
                assert_eq!(name, "fps");
                assert!(matches!(
                    init,
                    Some(JsExpr::Marker(Marker::Param { name, .. })) if name == "fps"
                ));
            }
            _ => panic!("Expected Decl with Marker"),
        }
    }

    #[test]
    fn test_js_parser_handles_cleanup() {
        // Content with %cleanup should parse into main + cleanup statements
        let parsed = js_parser::parse_emit_js_with_cleanup(
            "el.addEventListener('click', h); %cleanup { el.removeEventListener('click', h); }",
        )
        .unwrap();
        assert!(!parsed.main.is_empty());
        assert!(!parsed.cleanup.is_empty());
    }

    #[test]
    fn test_js_parser_handles_for_loop() {
        use crate::ir::JsStmt;

        // %for loops are parsed into ForLoopMeta
        let stmts =
            js_parser::parse_emit_js("%for $item in %items { console.log(item); }").unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], JsStmt::ForLoopMeta { .. }));
    }

    // =========================================================================
    // Structured IR Generation Tests (generate_primitive_ir_structured)
    // =========================================================================

    #[test]
    fn test_ir_structured_simple_param() {
        let primitive = PrimitiveDefAst {
            name: "simple".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "fps".to_string(),
                ty: ParamType::Simple("number".to_string()),
                default: Some(ParamDefault::Number(60.0)),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "const interval = 1000 / %fps;".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().param("fps", "30");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);

        // Convert to string to check output
        let result = emit_ir(ir);
        assert!(
            result.setup.contains("1000 / 30"),
            "Expected resolved param. Got: {}",
            result.setup
        );
        assert!(
            !result.setup.contains("%fps"),
            "Should not contain unresolved placeholder"
        );
    }

    #[test]
    fn test_ir_structured_element_reference() {
        let primitive = PrimitiveDefAst {
            name: "withel".to_string(),
            params: vec![PrimitiveParam::Element("container".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "const el = %&container;".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().element("container", "myContainer");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);

        assert!(
            result.setup.contains("myContainer"),
            "Expected element substitution. Got: {}",
            result.setup
        );
        assert!(
            !result.setup.contains("%&container"),
            "Should not contain unresolved placeholder"
        );
    }

    /// BUG-252 STRUCTURAL GUARD (S2). A scalar-typed param handed a JS OBJECT
    /// LITERAL is a type error, and must be reported as one.
    ///
    /// `signal_path` captures as a Named map `{base, facet?}`. One lowering
    /// branch normalized it to a flat binding; the other spliced the map. It
    /// reached `signal: binding` as `{ "base": "prPhase" }`, so the primitive's
    /// `String(%signal)` produced the literal text `"[object Object]"` and the
    /// runtime watched a signal name that can never exist. Nothing complained:
    /// JS stringifies anything.
    ///
    /// A `binding`/`string`/`number` param can never legitimately receive an
    /// object, so the mismatch is decidable here — at the boundary — rather than
    /// being discovered as inert behavior in a browser.
    #[test]
    fn object_literal_for_a_scalar_param_is_a_compile_error() {
        let primitive = PrimitiveDefAst {
            name: "watcher".to_string(),
            params: vec![
                PrimitiveParam::Element("el".to_string()),
                PrimitiveParam::Typed {
                    name: "signal".to_string(),
                    ty: ParamType::Simple("binding".to_string()),
                    default: None,
                },
            ],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "var s = String(%signal);".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // What the un-normalized `signal_path` capture actually produced.
        let args = PrimitiveArgs::new()
            .element("el", "node")
            .param("signal", "{ \"base\": \"prPhase\" }");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);

        assert!(
            ir.diagnostics
                .iter()
                .any(|d| d.message.contains("object") && d.message.contains("signal")),
            "a scalar-typed param receiving an object literal must be a compile error. Diagnostics: {:?}",
            ir.diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    /// A param genuinely declared to take structured data must NOT be flagged —
    /// object literals are the whole point there (arms, keyframes, records).
    #[test]
    fn object_literal_for_an_any_param_is_fine() {
        let primitive = PrimitiveDefAst {
            name: "taker".to_string(),
            params: vec![
                PrimitiveParam::Element("el".to_string()),
                PrimitiveParam::Typed {
                    name: "arms".to_string(),
                    ty: ParamType::Simple("any".to_string()),
                    default: None,
                },
            ],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "var a = %arms;".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new()
            .element("el", "node")
            .param("arms", "{ \"match\": \"go\" }");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);

        assert!(
            !ir.diagnostics.iter().any(|d| d.message.contains("object")),
            "a param declared `any` must accept structured data. Diagnostics: {:?}",
            ir.diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    /// BUG-252 STRUCTURAL GUARD. A primitive that binds its element with the
    /// same name the splice site uses lowers to the self-referential
    /// `var el = el;`. Under `var` hoisting the declaration shadows the outer
    /// binding BEFORE the assignment reads it, so the variable is `undefined`
    /// and every guard below it fires — silently, with correct-looking code.
    ///
    /// This killed `@on $signal` arms and `@handle` outright, and
    /// `stdlib/dnd/primitives/drag-zones.st` had documented the trap in a
    /// comment for two waves. A comment cannot constrain a different file; the
    /// compiler can. Emitting a self-assignment is ALWAYS a bug — there is no
    /// program for which `var x = x;` is the intended meaning — so it is a
    /// compile-time error, not a lint.
    #[test]
    fn self_shadowing_element_bind_is_a_compile_error() {
        let primitive = PrimitiveDefAst {
            name: "shadower".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "var el = %&el;\nif (!el) return;".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // The splice site names its element `el` — exactly what the
        // selector-init closure (`const init = function(el) { … }`) does.
        let args = PrimitiveArgs::new().element("el", "el");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);

        assert!(
            ir.diagnostics.iter().any(|d| d.message.contains("shadow")),
            "emitting `var el = el;` must be a compile error naming the shadow. Diagnostics: {:?}",
            ir.diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    /// The guard must not fire on a legitimate bind to a DIFFERENT name — the
    /// shape every correct primitive uses.
    #[test]
    fn binding_the_element_to_a_distinct_name_is_fine() {
        let primitive = PrimitiveDefAst {
            name: "wellbehaved".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "var node = %&el;\nif (!node) return;".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().element("el", "el");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);

        assert!(
            !ir.diagnostics.iter().any(|d| d.message.contains("shadow")),
            "binding to a distinct name must NOT be flagged. Diagnostics: {:?}",
            ir.diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_ir_structured_yield_statement() {
        let primitive = PrimitiveDefAst {
            name: "ticker".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "%yield now -> $time;".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "time".to_string(),
                    type_expr: ExportTypeExpr::Simple("number".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().element("el", "el");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);

        // Yield should be transformed to ST.set call
        assert!(
            result.setup.contains("ST.set"),
            "Expected ST.set call. Got: {}",
            result.setup
        );
        assert!(
            result.setup.contains("time"),
            "Expected signal name. Got: {}",
            result.setup
        );
        assert!(result.exports.contains(&"time".to_string()));
    }

    #[test]
    fn test_ir_structured_conditional_true() {
        let primitive = PrimitiveDefAst {
            name: "conditional".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "debug".to_string(),
                ty: ParamType::Simple("bool".to_string()),
                default: Some(ParamDefault::Bool(false)),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![PrimitiveIfBlock {
                    condition: MetaIfCondition::Truthy("debug".to_string()),
                    then_body: PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Js,
                            content: "console.log('debug mode');".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    },
                    else_body: Some(PrimitiveBody {
                        emit_blocks: vec![EmitBlock {
                            lang: EmitLang::Js,
                            content: "console.log('production mode');".to_string(),
                            span: crate::parser::SourceSpan::default(),
                        }],
                        cleanup: None,
                        exports: vec![],
                        if_blocks: vec![],
                    }),
                    span: crate::parser::SourceSpan::default(),
                }],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Test with debug = true
        let args = PrimitiveArgs::new().param("debug", "true");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);
        assert!(
            result.setup.contains("debug mode"),
            "Expected debug mode output. Got: {}",
            result.setup
        );
        assert!(!result.setup.contains("production mode"));

        // Test with debug = false
        let args_false = PrimitiveArgs::new().param("debug", "false");
        let ir_false = generate_primitive_ir_structured(&primitive, &args_false, None, None);
        let result_false = emit_ir(ir_false);
        assert!(
            result_false.setup.contains("production mode"),
            "Expected production mode output. Got: {}",
            result_false.setup
        );
        assert!(!result_false.setup.contains("debug mode"));
    }

    #[test]
    fn test_ir_structured_cleanup_block() {
        let primitive = PrimitiveDefAst {
            name: "withcleanup".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: r#"
                        const handler = () => {};
                        %&el.addEventListener('click', handler);
                        %cleanup {
                            %&el.removeEventListener('click', handler);
                        }
                    "#
                    .to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().element("el", "myEl");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);

        assert!(
            result.setup.contains("addEventListener"),
            "Expected addEventListener. Got: {}",
            result.setup
        );
        assert!(result.cleanup.is_some(), "Expected cleanup to be present");
        let cleanup = result.cleanup.unwrap();
        assert!(
            cleanup.contains("removeEventListener"),
            "Expected removeEventListener in cleanup. Got: {}",
            cleanup
        );
    }

    #[test]
    fn test_cleanup_extracted_from_emit_content() {
        // When %cleanup is inside %emit js content, the structured parser
        // extracts cleanup stmts — they should appear exactly once.
        let cleanup_code = "el.removeEventListener('click', handler);";
        let primitive = PrimitiveDefAst {
            name: "dupcheck".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: format!(
                        r#"
                        const handler = () => {{}};
                        %&el.addEventListener('click', handler);
                        %cleanup {{
                            %&el.{cleanup_code}
                        }}
                    "#
                    ),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().element("el", "myEl");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);

        // Count occurrences of removeEventListener in cleanup — should be exactly 1
        let count = ir
            .cleanup_stmts
            .iter()
            .filter(|s| format!("{:?}", s).contains("removeEventListener"))
            .count();
        assert_eq!(
            count, 1,
            "Expected exactly 1 cleanup stmt with removeEventListener, got {}. \
             Cleanup stmts: {:?}",
            count, ir.cleanup_stmts
        );
    }

    #[test]
    fn test_standalone_cleanup_emitted_when_no_emit_cleanup() {
        // When %cleanup is a standalone block (not inside %emit js),
        // it should still produce cleanup statements.
        let primitive = PrimitiveDefAst {
            name: "standalonecleanup".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: r#"
                        const handler = () => {};
                        %&el.addEventListener('click', handler);
                    "#
                    .to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                // Standalone cleanup at body level
                cleanup: Some("%&el.removeEventListener('click', handler);".to_string()),
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().element("el", "myEl");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);

        assert!(result.cleanup.is_some(), "Expected cleanup to be present");
        let cleanup = result.cleanup.unwrap();
        assert!(
            cleanup.contains("removeEventListener"),
            "Expected removeEventListener in standalone cleanup. Got: {}",
            cleanup
        );
    }

    #[test]
    fn test_ir_structured_missing_required_param() {
        let primitive = PrimitiveDefAst {
            name: "required".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "url".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None, // Required
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "fetch('%url');".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new(); // Missing required param
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);

        assert!(
            result.setup.contains("console.error"),
            "Expected error for missing param. Got: {}",
            result.setup
        );
        assert!(result.setup.contains("Missing required parameter"));
    }

    #[test]
    fn test_ir_structured_css_emit() {
        let primitive = PrimitiveDefAst {
            name: "withcss".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "color".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: Some(ParamDefault::String("red".to_string())),
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Css,
                    content: ".highlight { color: %color; }".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().param("color", "blue");
        let ir = generate_primitive_ir_structured(&primitive, &args, None, None);
        let result = emit_ir(ir);

        assert!(
            result.css.contains("blue"),
            "Expected CSS with color. Got: {}",
            result.css
        );
        assert!(
            !result.css.contains("%color"),
            "Should not contain unresolved placeholder"
        );
    }

    #[test]
    fn test_parse_pattern_match_with_bindings() {
        let result = parse_pattern_match("$ws:Connected{send,received}");
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.variable, "ws");
        assert_eq!(pattern.variant, "Connected");
        assert_eq!(pattern.bindings, vec!["send", "received"]);
    }

    #[test]
    fn test_parse_pattern_match_no_bindings() {
        let result = parse_pattern_match("$ws:Disconnected");
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.variable, "ws");
        assert_eq!(pattern.variant, "Disconnected");
        assert!(pattern.bindings.is_empty());
    }

    #[test]
    fn test_parse_pattern_match_single_binding() {
        let result = parse_pattern_match("$ws:Error{error}");
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.variable, "ws");
        assert_eq!(pattern.variant, "Error");
        assert_eq!(pattern.bindings, vec!["error"]);
    }

    #[test]
    fn test_parse_pattern_match_not_pattern() {
        // Plain string state - should not parse
        assert!(parse_pattern_match("idle").is_none());
        // Plain variable reference - should not parse (no :Variant)
        assert!(parse_pattern_match("$state").is_none());
    }

    #[test]
    fn test_generate_pattern_match_css() {
        let pattern = ParsedPatternMatch {
            variable: "ws".to_string(),
            variant: "Connected".to_string(),
            bindings: vec!["send".to_string()],
        };
        let css = generate_pattern_match_css(
            ".chat-app",
            &pattern,
            &[("color".to_string(), "green".to_string())],
        );
        assert!(css.contains(".chat-app[data-st-ws-type=\"Connected\"]"));
        assert!(css.contains("color: green"));
    }

    #[test]
    fn test_generate_pattern_match_js() {
        let pattern = ParsedPatternMatch {
            variable: "ws".to_string(),
            variant: "Connected".to_string(),
            bindings: vec!["send".to_string(), "received".to_string()],
        };
        let js = generate_pattern_match_js(&pattern);
        assert!(js.contains("ST.watchTypedUnion(el, 'ws', 'Connected', ['send', 'received'])"));
    }

    #[test]
    fn test_generate_pattern_match_js_no_bindings() {
        let pattern = ParsedPatternMatch {
            variable: "ws".to_string(),
            variant: "Disconnected".to_string(),
            bindings: vec![],
        };
        let js = generate_pattern_match_js(&pattern);
        assert!(js.contains("ST.watchTypedUnion(el, 'ws', 'Disconnected', [])"));
    }

    /// Test that invoke-template primitive with &container element param works
    /// when the container element is provided in PrimitiveArgs.
    /// This validates codegen emits `const container = el;` (not a console.error).
    #[test]
    fn test_invoke_template_container_bound() {
        let primitive = PrimitiveDefAst {
            name: "invoke-template".to_string(),
            params: vec![
                PrimitiveParam::Element("container".to_string()),
                PrimitiveParam::Typed {
                    name: "name".to_string(),
                    ty: ParamType::Simple("string".to_string()),
                    default: None,
                },
                PrimitiveParam::Typed {
                    name: "args".to_string(),
                    ty: ParamType::Simple("array".to_string()),
                    default: Some(ParamDefault::EmptyArray),
                },
                PrimitiveParam::Typed {
                    name: "body".to_string(),
                    ty: ParamType::Optional("string".to_string()),
                    default: Some(ParamDefault::None),
                },
            ],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "const container = %&container;\nconst templateName = %name;"
                        .to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Provide the container element (as the pipeline auto-bind should)
        let args = PrimitiveArgs::new()
            .element("container", "el")
            .param("name", "\"my-template\"");
        let result = emit_primitive(&primitive, &args);

        // Should NOT contain "Missing required element"
        assert!(
            !result.setup.contains("Missing required element"),
            "Expected no 'Missing required element' error but got: {}",
            result.setup
        );
        assert!(
            !result.setup.contains("console.error"),
            "Expected no console.error but got: {}",
            result.setup
        );

        // Should contain the container assignment
        assert!(
            result.setup.contains("const container"),
            "Expected 'const container' in output but got: {}",
            result.setup
        );
    }

    /// Regression test: CSS fallback path should unescape JS-escaped quotes.
    /// When css_overrides is NOT populated, the fallback reads from args.params
    /// which contains JS-escaped values (e.g., `\'<color>\'`). The CSS emit
    /// path must unescape these so the output is `'<color>'`, not `\'<color>\'`.
    #[test]
    fn test_css_fallback_unescapes_js_quotes() {
        // Build a minimal primitive with only a CSS emit block that uses %syntax_val
        let primitive = PrimitiveDefAst {
            name: "test-css-unescape".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "syntax_val".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Css,
                    content: "@property --test {\n  syntax: %syntax_val;\n}".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // Simulate the JS-escaped value that captured_to_js would produce
        // for the CSS value `'<color>'`: escape_js_string turns ' into \'
        let args = PrimitiveArgs::new().param("syntax_val", "\"'<color>'\""); // outer quotes from captured_to_js, inner quotes escaped
        // Note: the outer quotes get stripped, but the inner \' should be unescaped to '

        let result = emit_primitive(&primitive, &args);

        assert!(
            result.css.contains("'<color>'"),
            "CSS should contain unescaped single quotes: syntax: '<color>'\nGot: {}",
            result.css
        );
        assert!(
            !result.css.contains("\\'"),
            "CSS should NOT contain JS-escaped quotes \\'\nGot: {}",
            result.css
        );
    }

    /// Regression test: font-family with single quotes should be preserved in CSS.
    #[test]
    fn test_css_fallback_preserves_font_family_quotes() {
        let primitive = PrimitiveDefAst {
            name: "test-font-family".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "family".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Css,
                    content: ".test {\n  font-family: %family;\n}".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        // captured_to_js wraps in double quotes and escapes inner single quotes:
        // font-family: 'Helvetica Neue' → "\'Helvetica Neue\'"
        let args = PrimitiveArgs::new().param("family", "\"'Helvetica Neue'\"");

        let result = emit_primitive(&primitive, &args);

        assert!(
            result.css.contains("'Helvetica Neue'"),
            "CSS should contain font-family: 'Helvetica Neue'\nGot: {}",
            result.css
        );
        assert!(
            !result.css.contains("\\'"),
            "CSS should NOT contain escaped quotes\nGot: {}",
            result.css
        );
    }

    /// Regression test: values without JS escaping should pass through unchanged.
    #[test]
    fn test_css_fallback_plain_values_unchanged() {
        let primitive = PrimitiveDefAst {
            name: "test-plain".to_string(),
            params: vec![PrimitiveParam::Typed {
                name: "color".to_string(),
                ty: ParamType::Simple("string".to_string()),
                default: None,
            }],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Css,
                    content: ".test {\n  color: %color;\n}".to_string(),
                    span: crate::parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![],
                if_blocks: vec![],
            },
            uses: vec![],
            span: crate::parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let args = PrimitiveArgs::new().param("color", "\"red\"");

        let result = emit_primitive(&primitive, &args);

        assert!(
            result.css.contains("color: red"),
            "CSS should contain plain value 'red'\nGot: {}",
            result.css
        );
    }
}
