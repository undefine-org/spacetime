//! JavaScript Emitter
//!
//! Converts JsExpr and JsStmt IR types to JavaScript strings.

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ir::{BinOp, DeclKind, JsExpr, JsLit, JsPart, JsStmt, Marker, TemplatePart, UnaryOp};
use crate::utils::unescape_js_string;

use super::EmitOptions;

/// Emit a JavaScript expression
pub fn emit_expr(expr: &JsExpr, opts: &EmitOptions) -> Result<String, Diagnostic> {
    match expr {
        JsExpr::Lit(lit) => Ok(emit_lit(lit)),
        JsExpr::Var(name) => Ok(name.clone()),
        JsExpr::Prop { obj, prop } => {
            let obj_str = emit_expr(obj, opts)?;
            // Use bracket notation for reserved words or invalid identifiers
            if needs_bracket_notation(prop) {
                Ok(format!("{}[\"{}\"]", obj_str, escape_string(prop)))
            } else {
                Ok(format!("{}.{}", obj_str, prop))
            }
        }
        JsExpr::Index { arr, idx } => Ok(format!(
            "{}[{}]",
            emit_expr(arr, opts)?,
            emit_expr(idx, opts)?
        )),
        JsExpr::Call { callee, args } => {
            let mut args_strs = Vec::new();
            for a in args {
                args_strs.push(emit_expr(a, opts)?);
            }
            Ok(format!(
                "{}({})",
                emit_expr(callee, opts)?,
                args_strs.join(", ")
            ))
        }
        JsExpr::Method { obj, method, args } => {
            let mut args_strs = Vec::new();
            for a in args {
                args_strs.push(emit_expr(a, opts)?);
            }
            Ok(format!(
                "{}.{}({})",
                emit_expr(obj, opts)?,
                method,
                args_strs.join(", ")
            ))
        }
        JsExpr::Arrow { params, body } => {
            let params_str = if params.len() == 1 && is_simple_ident(&params[0]) {
                params[0].clone()
            } else {
                format!("({})", params.join(", "))
            };
            let body_str = emit_expr(body, opts)?;
            // Wrap object literals in parens to avoid ambiguity
            if matches!(**body, JsExpr::Object(_)) {
                Ok(format!("{} => ({})", params_str, body_str))
            } else {
                Ok(format!("{} => {}", params_str, body_str))
            }
        }
        JsExpr::Block { stmts, expr } => {
            let nl = &opts.newline;
            let indent = &opts.indent;
            let mut parts = Vec::new();
            for stmt in stmts {
                parts.push(format!("{}{}", indent, emit_stmt(stmt, opts, 1)?));
            }
            if let Some(e) = expr {
                parts.push(format!("{}return {};", indent, emit_expr(e, opts)?));
            }
            Ok(format!("{{{}{}{}}}", nl, parts.join(nl), nl))
        }
        JsExpr::Template { parts } => {
            let mut out = String::from("`");
            for part in parts {
                match part {
                    TemplatePart::Text(t) => out.push_str(&escape_template_string(t)),
                    TemplatePart::Expr(e) => {
                        out.push_str("${");
                        out.push_str(&emit_expr(e, opts)?);
                        out.push('}');
                    }
                }
            }
            out.push('`');
            Ok(out)
        }
        JsExpr::Binary { left, op, right } => {
            let left_str = emit_expr_maybe_parens(left, expr, true, opts)?;
            let right_str = emit_expr_maybe_parens(right, expr, false, opts)?;
            Ok(format!("{} {} {}", left_str, emit_binop(op), right_str))
        }
        JsExpr::Unary { op, expr: inner } => {
            let inner_str = emit_expr(inner, opts)?;
            Ok(match op {
                UnaryOp::Not => format!("!{}", inner_str),
                UnaryOp::Neg => format!("-{}", inner_str),
                UnaryOp::Typeof => format!("typeof {}", inner_str),
                UnaryOp::Void => format!("void {}", inner_str),
            })
        }
        JsExpr::Ternary { cond, then_, else_ } => Ok(format!(
            "{} ? {} : {}",
            emit_expr(cond, opts)?,
            emit_expr(then_, opts)?,
            emit_expr(else_, opts)?
        )),
        JsExpr::Object(entries) => {
            if entries.is_empty() {
                return Ok("{}".to_string());
            }
            if opts.minify {
                let mut pairs = Vec::new();
                for (k, v) in entries {
                    pairs.push(format!("{}: {}", emit_object_key(k), emit_expr(v, opts)?));
                }
                Ok(format!("{{ {} }}", pairs.join(", ")))
            } else {
                let mut pairs = Vec::new();
                for (k, v) in entries {
                    pairs.push(format!("  {}: {}", emit_object_key(k), emit_expr(v, opts)?));
                }
                Ok(format!("{{\n{}\n}}", pairs.join(",\n")))
            }
        }
        JsExpr::Array(items) => {
            let mut items_strs = Vec::new();
            for i in items {
                items_strs.push(emit_expr(i, opts)?);
            }
            Ok(format!("[{}]", items_strs.join(", ")))
        }
        JsExpr::Raw(s) => Ok(s.clone()),
        JsExpr::New { callee, args } => {
            let mut args_strs = Vec::new();
            for a in args {
                args_strs.push(emit_expr(a, opts)?);
            }
            Ok(format!(
                "new {}({})",
                emit_expr(callee, opts)?,
                args_strs.join(", ")
            ))
        }
        JsExpr::Marker(_) => {
            // Markers MUST be resolved before emission.
            // If we reach here, it's a compiler bug.
            Err(Diagnostic::error(
                DiagnosticCode::E0807,
                "unresolved placeholder/marker reached JS emission - must be resolved before emission".to_string(),
            )
            .with_note("This is a compiler bug - please file an issue at https://github.com/anthropics/spacetime/issues"))
        }
        JsExpr::Composite(parts) => {
            // Emit each part in sequence
            let mut result = String::new();
            for part in parts {
                result.push_str(&emit_part(part, opts)?);
            }
            Ok(result)
        }
    }
}

/// Emit a JsPart
fn emit_part(part: &JsPart, opts: &EmitOptions) -> Result<String, Diagnostic> {
    match part {
        JsPart::Raw(s) => Ok(s.clone()),
        JsPart::Expr(e) => emit_expr(e, opts),
        JsPart::Marker(m) => emit_marker(m),
    }
}

/// Emit a Marker - markers MUST be resolved before emission
fn emit_marker(marker: &Marker<JsExpr>) -> Result<String, Diagnostic> {
    // Markers MUST be resolved before emission.
    // If we reach here, it's a compiler bug.
    Err(Diagnostic::error(
        DiagnosticCode::E0808,
        format!("unresolved marker {:?} reached JS emission", marker),
    )
    .with_note("This is a compiler bug - please file an issue at https://github.com/anthropics/spacetime/issues"))
}

/// Emit a JavaScript statement
pub fn emit_stmt(stmt: &JsStmt, opts: &EmitOptions, depth: usize) -> Result<String, Diagnostic> {
    let nl = if opts.minify { "" } else { &opts.newline };

    match stmt {
        JsStmt::Decl { kind, name, init } => {
            let kind_str = match kind {
                DeclKind::Const => "const",
                DeclKind::Let => "let",
                DeclKind::Var => "var",
            };
            match init {
                Some(expr) => Ok(format!(
                    "{} {} = {};",
                    kind_str,
                    name,
                    emit_expr(expr, opts)?
                )),
                None => Ok(format!("{} {};", kind_str, name)),
            }
        }
        JsStmt::Expr(expr) => Ok(format!("{};", emit_expr(expr, opts)?)),
        JsStmt::If { cond, then_, else_ } => {
            let cond_str = emit_expr(cond, opts)?;
            let then_str = emit_stmts_block(then_, opts, depth)?;
            match else_ {
                Some(else_stmts) => {
                    let else_str = emit_stmts_block(else_stmts, opts, depth)?;
                    let sep = if opts.minify { " " } else { nl };
                    Ok(format!(
                        "if ({}) {}{}else {}",
                        cond_str, then_str, sep, else_str
                    ))
                }
                None => Ok(format!("if ({}) {}", cond_str, then_str)),
            }
        }
        JsStmt::ForOf { var, iter, body } => {
            let body_str = emit_stmts_block(body, opts, depth)?;
            Ok(format!(
                "for (const {} of {}) {}",
                var,
                emit_expr(iter, opts)?,
                body_str
            ))
        }
        JsStmt::For {
            init,
            cond,
            update,
            body,
        } => {
            let init_str = match init {
                Some(s) => emit_stmt(s, opts, 0)?.trim_end_matches(';').to_string(),
                None => String::new(),
            };
            let cond_str = match cond {
                Some(e) => emit_expr(e, opts)?,
                None => String::new(),
            };
            let update_str = match update {
                Some(e) => emit_expr(e, opts)?,
                None => String::new(),
            };
            let body_str = emit_stmts_block(body, opts, depth)?;
            Ok(format!(
                "for ({}; {}; {}) {}",
                init_str, cond_str, update_str, body_str
            ))
        }
        JsStmt::Return(expr) => match expr {
            Some(e) => Ok(format!("return {};", emit_expr(e, opts)?)),
            None => Ok("return;".to_string()),
        },
        JsStmt::Break => Ok("break;".to_string()),
        JsStmt::Continue => Ok("continue;".to_string()),
        JsStmt::Raw(s) => Ok(s.clone()),
        JsStmt::ForLoopMeta { var, array, body } => {
            // Meta-level for loops should be expanded during codegen, not emitted directly.
            // If we reach here, emit as a comment with the body statements.
            let body_str = emit_stmts(body, opts)?;
            Ok(format!("/* %for ${} in %{} */ {}", var, array, body_str))
        }
    }
}

/// Emit multiple statements
pub fn emit_stmts(stmts: &[JsStmt], opts: &EmitOptions) -> Result<String, Diagnostic> {
    let nl = if opts.minify { " " } else { &opts.newline };
    let mut results = Vec::new();
    for s in stmts {
        results.push(emit_stmt(s, opts, 0)?);
    }
    Ok(results.join(nl))
}

/// Emit statements as a block with braces
fn emit_stmts_block(
    stmts: &[JsStmt],
    opts: &EmitOptions,
    depth: usize,
) -> Result<String, Diagnostic> {
    if stmts.is_empty() {
        return Ok("{}".to_string());
    }

    let nl = if opts.minify { " " } else { &opts.newline };
    let indent = if opts.minify {
        String::new()
    } else {
        opts.indent.repeat(depth + 1)
    };
    let close_indent = if opts.minify {
        String::new()
    } else {
        opts.indent.repeat(depth)
    };

    let mut body = Vec::new();
    for s in stmts {
        body.push(format!("{}{}", indent, emit_stmt(s, opts, depth + 1)?));
    }

    Ok(format!("{{{}{}{}{}}}", nl, body.join(nl), nl, close_indent))
}

/// Emit a literal value
fn emit_lit(lit: &JsLit) -> String {
    match lit {
        JsLit::String(s) => format!("\"{}\"", escape_string(s)),
        JsLit::Number(n) => {
            if n.is_infinite() {
                if *n > 0.0 {
                    "Infinity".to_string()
                } else {
                    "-Infinity".to_string()
                }
            } else if n.is_nan() {
                "NaN".to_string()
            } else if *n == n.trunc() && n.abs() < 1e15 {
                // Integer-like numbers
                format!("{:.0}", n)
            } else {
                format!("{}", n)
            }
        }
        JsLit::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        JsLit::Null => "null".to_string(),
        JsLit::Undefined => "undefined".to_string(),
    }
}

/// Emit a binary operator
fn emit_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::EqStrict => "===",
        BinOp::NeStrict => "!==",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>>",
        BinOp::Assign => "=",
    }
}

/// Get operator precedence (higher = binds tighter)
fn precedence(expr: &JsExpr) -> u8 {
    match expr {
        JsExpr::Binary { op, .. } => match op {
            BinOp::Assign => 2,
            BinOp::Or => 4,
            BinOp::And => 5,
            BinOp::BitOr => 6,
            BinOp::BitXor => 7,
            BinOp::BitAnd => 8,
            BinOp::Eq | BinOp::Ne | BinOp::EqStrict | BinOp::NeStrict => 9,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 10,
            BinOp::Shl | BinOp::Shr | BinOp::UShr => 11,
            BinOp::Add | BinOp::Sub => 12,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 13,
        },
        JsExpr::Ternary { .. } => 3,
        JsExpr::Unary { .. } => 14,
        _ => 15,
    }
}

/// Emit expression with parentheses if needed
fn emit_expr_maybe_parens(
    inner: &JsExpr,
    outer: &JsExpr,
    is_left: bool,
    opts: &EmitOptions,
) -> Result<String, Diagnostic> {
    let inner_prec = precedence(inner);
    let outer_prec = precedence(outer);

    // Need parens if inner has lower precedence
    // Or if same precedence and on the right (for left-associative ops)
    let needs_parens = inner_prec < outer_prec || (inner_prec == outer_prec && !is_left);

    if needs_parens {
        Ok(format!("({})", emit_expr(inner, opts)?))
    } else {
        emit_expr(inner, opts)
    }
}

/// Check if a property name needs bracket notation
fn needs_bracket_notation(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    let first = name.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return true;
    }
    name.chars()
        .any(|c| !c.is_alphanumeric() && c != '_' && c != '$')
}

/// Check if identifier is simple (single param arrow can omit parens)
fn is_simple_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().unwrap().is_alphabetic()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Emit an object key (quote if needed)
fn emit_object_key(key: &str) -> String {
    if is_simple_ident(key) && !is_reserved_word(key) {
        key.to_string()
    } else {
        format!("\"{}\"", escape_string(key))
    }
}

/// Check if a word is a JavaScript reserved word
fn is_reserved_word(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "catch"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "in"
            | "instanceof"
            | "new"
            | "return"
            | "switch"
            | "this"
            | "throw"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "class"
            | "const"
            | "enum"
            | "export"
            | "extends"
            | "import"
            | "super"
            | "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
}

/// Escape a string for JS string literals
fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Escape a string for JS template literals
fn escape_template_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '`' => out.push_str("\\`"),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            c => out.push(c),
        }
    }
    out
}

// ============================================================================
// Marker Resolution
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;

use crate::metasystem::MetaRegistry;

/// Context for resolving Spacetime placeholders during code emission.
///
/// This context provides the mapping from placeholder names to their concrete
/// values at code generation time.
#[derive(Debug, Clone, Default)]
pub struct EmitContext {
    /// Variable name for the current element (usually "el")
    pub element_var: String,
    /// Parameter values: name -> value string
    pub params: HashMap<String, String>,
    /// Element references: name -> variable name
    pub elements: HashMap<String, String>,
    /// Loop variable bindings: var_name -> current item JSON value
    pub loop_vars: HashMap<String, serde_json::Value>,
    /// Output signal name mappings: primitive_export_name -> user_signal_name
    /// Used when %binds remaps output names (e.g., socket(...) -> { $state } with $state = "ws")
    pub outputs: HashMap<String, String>,
    /// Parameter types from primitive definition: name -> type string (e.g., "expr", "string", "ident")
    /// Used to determine how parameter values should be resolved (raw JS vs quoted string)
    pub param_types: HashMap<String, String>,
    /// The live MetaRegistry, when available. Used to compile a `$body:block` /
    /// `$content:block` capture's nested directives via the `.js` field accessor
    /// (PLAN-026) against the SAME macro set as the host compile (so testing
    /// macros like @assert/@then resolve). `Arc` keeps EmitContext cheap to clone.
    pub registry: Option<Arc<MetaRegistry>>,
}

impl EmitContext {
    /// Create a new emit context with the given element variable name
    pub fn new(element_var: impl Into<String>) -> Self {
        Self {
            element_var: element_var.into(),
            params: HashMap::new(),
            elements: HashMap::new(),
            loop_vars: HashMap::new(),
            outputs: HashMap::new(),
            param_types: HashMap::new(),
            registry: None,
        }
    }

    /// Add a parameter value
    pub fn with_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name.into(), value.into());
        self
    }

    /// Add an element reference
    pub fn with_element(mut self, name: impl Into<String>, var_name: impl Into<String>) -> Self {
        self.elements.insert(name.into(), var_name.into());
        self
    }

    /// Create a new context with a loop variable binding
    pub fn with_loop_var(&self, var: impl Into<String>, value: serde_json::Value) -> Self {
        let mut ctx = self.clone();
        ctx.loop_vars.insert(var.into(), value);
        ctx
    }

    /// Add an output signal name mapping
    /// Maps from primitive's export name to user's alias
    pub fn with_output(
        mut self,
        primitive_name: impl Into<String>,
        user_name: impl Into<String>,
    ) -> Self {
        self.outputs.insert(primitive_name.into(), user_name.into());
        self
    }

    /// Add a parameter type from the primitive definition
    pub fn with_param_type(mut self, name: impl Into<String>, ty: impl Into<String>) -> Self {
        self.param_types.insert(name.into(), ty.into());
        self
    }
}

/// Resolve a signal name to its final output name.
///
/// Handles two cases:
/// - Direct signal name: looks up in outputs mapping
/// - Param reference: looks up param value, then looks up in outputs mapping
///
/// Used by Marker::Directive resolution.
fn resolve_signal_name(
    signal: Option<&str>,
    signal_param: Option<&str>,
    ctx: &EmitContext,
) -> String {
    if let Some(s) = signal {
        // Direct signal name - check outputs mapping for remapping
        ctx.outputs.get(s).cloned().unwrap_or_else(|| s.to_string())
    } else if let Some(param) = signal_param {
        // Param reference - look up param value, then check outputs mapping
        let param_signal = ctx
            .params
            .get(param)
            .cloned()
            .unwrap_or_else(|| param.to_string());
        // Strip outer JS quotes — param values from captured_to_js() are stored as
        // JS literals (e.g., "\"name\"") but signal names should be bare identifiers
        let unquoted = strip_js_quotes(&param_signal);
        ctx.outputs.get(&unquoted).cloned().unwrap_or(unquoted)
    } else {
        "unknown".to_string()
    }
}

/// Strip outer JS quote characters from a string value.
///
/// Param values from `captured_to_js()` are stored with JS-level quotes
/// (e.g., `"\"journey-header\""`) but signal names need bare identifiers.
fn strip_js_quotes(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Resolve a %$var or %$var.field reference against loop_vars and params.
/// Returns the resolved JsExpr, or an E0805 diagnostic if not found.
fn resolve_var_ref(
    var: &str,
    field: &Option<String>,
    ctx: &EmitContext,
) -> Result<JsExpr, Diagnostic> {
    if let Some(value) = ctx.loop_vars.get(var) {
        let target_value = if let Some(field_name) = field {
            value
                .get(field_name)
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        } else {
            value.clone()
        };
        Ok(json_value_to_js_expr(&target_value))
    } else if let Some(param_value) = ctx.params.get(var) {
        if let Some(field_name) = field {
            // Route field access through the single field-accessor authority so
            // `%$body.js` (a VarRef) gets the same block-body compilation as the
            // Marker::Param path — e.g. `.js` compiles nested directives instead
            // of emitting a dead `value.js` property access (PLAN-026 / BUG-051).
            apply_param_field_accessor(
                var,
                field_name,
                param_value,
                &JsExpr::Var(param_value.clone()),
                ctx,
            )
        } else {
            Ok(JsExpr::Var(param_value.clone()))
        }
    } else {
        let field_str = field
            .as_ref()
            .map(|f| format!(".{}", f))
            .unwrap_or_default();
        Err(Diagnostic::error(
            DiagnosticCode::E0805,
            format!(
                "unresolved variable '%${}{}' during marker resolution",
                var, field_str
            ),
        )
        .with_hint(format!(
            "available loop vars: {:?}, params: {:?}",
            ctx.loop_vars.keys().collect::<Vec<_>>(),
            ctx.params.keys().collect::<Vec<_>>()
        )))
    }
}

/// Convert a JsExpr string literal to Raw for in-string interpolation.
///
/// When a placeholder is inside a string literal like `'Hello %name'`,
/// we need to emit the value as raw text (without quotes) since the
/// surrounding quotes are already part of the string.
fn unwrap_string_to_raw(expr: JsExpr) -> JsExpr {
    match expr {
        JsExpr::Lit(JsLit::String(s)) => JsExpr::Raw(s),
        // For other types (numbers, bools), emit their string representation
        JsExpr::Lit(JsLit::Number(n)) => {
            if n == n.trunc() && n.abs() < 1e15 {
                JsExpr::Raw(format!("{:.0}", n))
            } else {
                JsExpr::Raw(format!("{}", n))
            }
        }
        JsExpr::Lit(JsLit::Bool(b)) => JsExpr::Raw(if b { "true" } else { "false" }.to_string()),
        JsExpr::Lit(JsLit::Null) => JsExpr::Raw("null".to_string()),
        JsExpr::Lit(JsLit::Undefined) => JsExpr::Raw("undefined".to_string()),
        // Raw is already raw
        JsExpr::Raw(_) => expr,
        // For complex expressions, keep as-is (shouldn't happen for simple params)
        _ => expr,
    }
}

/// Resolve any placeholders in a JsExpr tree
pub fn resolve_expr(
    expr: &JsExpr,
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<JsExpr, Diagnostic> {
    match expr {
        JsExpr::Binary { left, op, right } => Ok(JsExpr::Binary {
            left: Box::new(resolve_expr(left, ctx, opts)?),
            op: *op,
            right: Box::new(resolve_expr(right, ctx, opts)?),
        }),
        JsExpr::Unary { op, expr: inner } => Ok(JsExpr::Unary {
            op: *op,
            expr: Box::new(resolve_expr(inner, ctx, opts)?),
        }),
        JsExpr::Call { callee, args } => {
            let mut resolved_args = Vec::new();
            for a in args {
                resolved_args.push(resolve_expr(a, ctx, opts)?);
            }
            Ok(JsExpr::Call {
                callee: Box::new(resolve_expr(callee, ctx, opts)?),
                args: resolved_args,
            })
        }
        JsExpr::Method { obj, method, args } => {
            let mut resolved_args = Vec::new();
            for a in args {
                resolved_args.push(resolve_expr(a, ctx, opts)?);
            }
            Ok(JsExpr::Method {
                obj: Box::new(resolve_expr(obj, ctx, opts)?),
                method: method.clone(),
                args: resolved_args,
            })
        }
        JsExpr::Arrow { params, body } => Ok(JsExpr::Arrow {
            params: params.clone(),
            body: Box::new(resolve_expr(body, ctx, opts)?),
        }),
        JsExpr::Ternary { cond, then_, else_ } => Ok(JsExpr::Ternary {
            cond: Box::new(resolve_expr(cond, ctx, opts)?),
            then_: Box::new(resolve_expr(then_, ctx, opts)?),
            else_: Box::new(resolve_expr(else_, ctx, opts)?),
        }),
        JsExpr::Prop { obj, prop } => Ok(JsExpr::Prop {
            obj: Box::new(resolve_expr(obj, ctx, opts)?),
            prop: prop.clone(),
        }),
        JsExpr::Index { arr, idx } => Ok(JsExpr::Index {
            arr: Box::new(resolve_expr(arr, ctx, opts)?),
            idx: Box::new(resolve_expr(idx, ctx, opts)?),
        }),
        JsExpr::Array(items) => {
            let mut resolved = Vec::new();
            for i in items {
                resolved.push(resolve_expr(i, ctx, opts)?);
            }
            Ok(JsExpr::Array(resolved))
        }
        JsExpr::Object(entries) => {
            let mut resolved = Vec::new();
            for (k, v) in entries {
                resolved.push((k.clone(), resolve_expr(v, ctx, opts)?));
            }
            Ok(JsExpr::Object(resolved))
        }
        JsExpr::Template { parts } => {
            let mut resolved_parts = Vec::new();
            for p in parts {
                match p {
                    TemplatePart::Text(t) => resolved_parts.push(TemplatePart::Text(t.clone())),
                    TemplatePart::Expr(e) => {
                        resolved_parts.push(TemplatePart::Expr(resolve_expr(e, ctx, opts)?))
                    }
                }
            }
            Ok(JsExpr::Template {
                parts: resolved_parts,
            })
        }
        JsExpr::Block { stmts, expr } => {
            let mut resolved_stmts = Vec::new();
            for s in stmts {
                resolved_stmts.push(resolve_stmt(s, ctx, opts)?);
            }
            let resolved_expr = match expr {
                Some(e) => Some(Box::new(resolve_expr(e, ctx, opts)?)),
                None => None,
            };
            Ok(JsExpr::Block {
                stmts: resolved_stmts,
                expr: resolved_expr,
            })
        }
        JsExpr::New { callee, args } => {
            let mut resolved_args = Vec::new();
            for a in args {
                resolved_args.push(resolve_expr(a, ctx, opts)?);
            }
            Ok(JsExpr::New {
                callee: Box::new(resolve_expr(callee, ctx, opts)?),
                args: resolved_args,
            })
        }
        // Leaf nodes that don't contain placeholders
        JsExpr::Lit(_) | JsExpr::Var(_) => Ok(expr.clone()),
        // Raw is truly raw - no placeholders to resolve
        // (Markers should be in Composite, not Raw)
        JsExpr::Raw(_) => Ok(expr.clone()),
        // Composite: resolve each part
        JsExpr::Composite(parts) => {
            let mut resolved = Vec::new();
            for p in parts {
                resolved.push(resolve_part(p, ctx, opts)?);
            }
            Ok(JsExpr::Composite(resolved))
        }
        JsExpr::Marker(m) => {
            let resolved = resolve_marker(m, ctx, opts)?;
            resolve_expr(&resolved, ctx, opts)
        }
    }
}

/// Resolve markers in a JsPart
fn resolve_part(
    part: &JsPart,
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<JsPart, Diagnostic> {
    match part {
        JsPart::Raw(s) => Ok(JsPart::Raw(s.clone())),
        JsPart::Expr(e) => Ok(JsPart::Expr(resolve_expr(e, ctx, opts)?)),
        JsPart::Marker(m) => {
            // Resolve marker to an expression, then wrap in JsPart::Expr
            let resolved = resolve_marker(m, ctx, opts)?;
            Ok(JsPart::Expr(resolved))
        }
    }
}

/// Resolve a Marker to a concrete JsExpr
fn resolve_marker(
    marker: &Marker<JsExpr>,
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<JsExpr, Diagnostic> {
    match marker {
        Marker::Param {
            name,
            field,
            in_string,
        } => {
            match ctx.params.get(name) {
                Some(value) => {
                    // Check if this parameter is declared as `expr` type.
                    // Expr params should be parsed as JS (handles $var → Signal resolution)
                    // rather than treated as string literals by parse_param_value.
                    let is_expr_type = ctx
                        .param_types
                        .get(name)
                        .map(|t| t == "expr")
                        .unwrap_or(false);
                    let parsed = if is_expr_type && !value.is_empty() {
                        match crate::emit::js_parser::parse_emit_js(value) {
                            Ok(stmts) => {
                                if let Some(JsStmt::Expr(expr)) = stmts.first() {
                                    match resolve_expr(expr, ctx, opts) {
                                        Ok(resolved) => resolved,
                                        Err(_) => JsExpr::Raw(value.clone()),
                                    }
                                } else {
                                    JsExpr::Raw(value.clone())
                                }
                            }
                            Err(_) => JsExpr::Raw(value.clone()),
                        }
                    } else {
                        parse_param_value(value)
                    };
                    // Apply compile-time field accessor if present
                    let parsed = if let Some(field_name) = field {
                        apply_param_field_accessor(name, field_name, value, &parsed, ctx)?
                    } else {
                        parsed
                    };
                    if in_string.is_some() {
                        Ok(unwrap_string_to_raw(parsed))
                    } else {
                        Ok(parsed)
                    }
                }
                None => {
                    let available: Vec<&str> = ctx.params.keys().map(|s| s.as_str()).collect();
                    let hint = if available.is_empty() {
                        "no parameters available".to_string()
                    } else {
                        format!("available params: {}", available.join(", "))
                    };
                    Err(Diagnostic::error(
                        DiagnosticCode::E0800,
                        format!("unknown parameter '{}' during marker resolution", name),
                    )
                    .with_hint(hint))
                }
            }
        }
        Marker::Element(name) => match ctx.elements.get(name) {
            Some(var_name) => Ok(JsExpr::Var(var_name.clone())),
            None => Ok(JsExpr::Var(name.clone())),
        },
        Marker::Signal(name) => Ok(JsExpr::Method {
            obj: Box::new(JsExpr::Var("ST".into())),
            method: "get".into(),
            args: vec![
                JsExpr::Var(ctx.element_var.clone()),
                JsExpr::Lit(JsLit::String(name.clone())),
            ],
        }),
        Marker::VarRef {
            var,
            field,
            in_string,
        } => {
            let expr = resolve_var_ref(var, field, ctx)?;
            if in_string.is_some() {
                Ok(unwrap_string_to_raw(expr))
            } else {
                Ok(expr)
            }
        }
        Marker::Directive {
            keyword,
            expr,
            target,
        } => {
            match keyword.as_str() {
                "yield" => {
                    // %yield expr -> $signal becomes ST.set(el, 'signal', expr)
                    let resolved_expr = match expr {
                        Some(e) => resolve_expr(e, ctx, opts)?,
                        None => JsExpr::Lit(JsLit::Undefined),
                    };

                    // Parse target to determine if it's a direct signal or param reference
                    let signal = target.clone().unwrap_or_else(|| "unknown".into());
                    let (direct_signal, param_signal) = if signal.starts_with('%') {
                        (None, Some(&signal[1..]))
                    } else {
                        (Some(signal.as_str()), None)
                    };
                    let resolved_signal = resolve_signal_name(direct_signal, param_signal, ctx);

                    Ok(JsExpr::Method {
                        obj: Box::new(JsExpr::Var("ST".into())),
                        method: "set".into(),
                        args: vec![
                            JsExpr::Var(ctx.element_var.clone()),
                            JsExpr::Lit(JsLit::String(resolved_signal)),
                            resolved_expr,
                        ],
                    })
                }
                _ => Err(Diagnostic::error(
                    DiagnosticCode::E0806,
                    format!("unknown directive '%{}' during marker resolution", keyword),
                )),
            }
        }
    }
}

/// Apply a compile-time field accessor to a resolved parameter value.
/// For typed params (e.g., color), this resolves at compile time.
/// For untyped params, falls back to runtime property access.
fn apply_param_field_accessor(
    param_name: &str,
    field: &str,
    raw_value: &str,
    parsed: &JsExpr,
    ctx: &EmitContext,
) -> Result<JsExpr, Diagnostic> {
    let param_type = ctx.param_types.get(param_name).map(|s| s.as_str());
    match (param_type, field) {
        (Some("color"), "rgba") => {
            // Strip JS string quotes if present — macro_value_to_string wraps
            // Color values in quotes for JS contexts (e.g., '"transparent"'),
            // but the color parser needs the bare value.
            let color_str = raw_value.trim();
            let color_str = if (color_str.starts_with('"') && color_str.ends_with('"'))
                || (color_str.starts_with('\'') && color_str.ends_with('\''))
            {
                &color_str[1..color_str.len() - 1]
            } else {
                color_str
            };
            match crate::color::parse_color_to_normalized_rgba(color_str) {
                Ok([r, g, b, a]) => Ok(JsExpr::Array(vec![
                    JsExpr::Lit(JsLit::Number(r)),
                    JsExpr::Lit(JsLit::Number(g)),
                    JsExpr::Lit(JsLit::Number(b)),
                    JsExpr::Lit(JsLit::Number(a)),
                ])),
                Err(e) => Err(Diagnostic::error(
                    DiagnosticCode::E0800,
                    format!(
                        "cannot resolve color '{}' at compile time: {}",
                        color_str, e
                    ),
                )),
            }
        }
        // Block-body field accessors (PLAN-026 / BUG-051). A `$body:block` /
        // `$content:block` capture is the RAW inner source of a `{ ... }` body
        // (e.g. a @test or @fixture body). `.js` compiles its nested Spacetime
        // directives to JavaScript; `.html` yields the raw markup string. Without
        // this, `%$body.js` fell through to runtime property access and emitted
        // the body as a dead string literal (`"@assert(...)".js`) so test bodies
        // never executed.
        (_, "js") => Ok(JsExpr::Raw(compile_block_body_to_js(raw_value, ctx))),
        // `.scoped_js` (BUG-119 §1): like `.js`, but the body is recompiled WRAPPED
        // in the sibling `target` selector — `target { <body> }` — so a body that is a
        // CSS scope (`.cls: $sig`) or carries scoped-state decls (`$x bool: false`)
        // compiles as a real ScopeBlock bound to that element, not as a selectorless
        // fragment that drops to nothing. Used by @given, whose author writes
        // `@given .sel { $open bool: false; .is-open: $open }` meaning "this element
        // has this scoped state + reactive class". Falls back to bare `.js` when no
        // `target` param is in scope.
        (_, "scoped_js") => {
            let target = ctx.params.get("target").map(|s| strip_js_quotes(s));
            match target {
                Some(sel) if !sel.trim().is_empty() => Ok(JsExpr::Raw(
                    compile_scoped_block_body_to_js(raw_value, &sel, ctx),
                )),
                _ => Ok(JsExpr::Raw(compile_block_body_to_js(raw_value, ctx))),
            }
        }
        // `.state_props` (BUG-217): the DOM-state `name: value` pairs a block body
        // declares, as a JS object literal.
        //
        // `@given <sel> { data-state: "modified" }` means "put the element in this
        // state". Compiled as a scope (`.scoped_js`), those pairs are CSS
        // declarations — they emit a CSS rule and NO JavaScript, so nothing ever
        // touched the element. Eight of the test framework's own self-tests
        // asserted on state that was never set, and had been red long enough to be
        // taken as normal.
        //
        // The sibling `%macro given` declaring `$state:properties` cannot fix this:
        // its capture is INLINE, so its form has no body_capture and it never
        // competes for a braced body at all.
        //
        // A body with no `name: value` pairs yields `{}`, so the caller's loop is a
        // no-op and the scope path is untouched — the two shapes compose instead of
        // one shadowing the other.
        (_, "state_props") => Ok(JsExpr::Raw(block_body_state_props_json(raw_value))),
        // I4 / gh-8: a directive inside markup is not a nested bind (directives
        // attach through selectors) and its source must not render as text.
        (_, "html") => {
            let unquoted = strip_js_quotes(raw_value);
            Ok(JsExpr::Lit(JsLit::String(strip_js_quotes(
                &crate::html::strip_nested_directives(&unquoted),
            ))))
        }
        // BUG-105 (P3): `.fixture_html` is `.html` made safe to embed inside a JS
        // BACKTICK template literal — the form `@mount`/`@fixture` use
        // (`innerHTML = `%$content.fixture_html``) to preserve multi-line markup. A
        // block body that is a `@template` fixture contains a `` `$hole` `` (or a
        // literal `${`) which would otherwise terminate the template literal early →
        // SyntaxError. Escape backslashes, backticks, and `${` so the markup embeds
        // verbatim. A bare raw fragment (no surrounding quotes): the host macro
        // supplies the backticks. (`.html` stays raw for HTML contexts like docs.st
        // where backticks are content, e.g. markdown code spans.)
        (_, "fixture_html") => {
            // I4 / gh-8: a directive written INSIDE the markup (`<div> @scroll … { … }
            // </div>`) is not a nested bind — directives attach through selectors. The
            // source text must NOT be painted onto the page (it would render as visible
            // compiler input), so strip every nested-directive invocation first.
            let sanitized = crate::html::strip_nested_directives(&strip_js_quotes(raw_value));
            // The raw_value is ALREADY JS-escaped for a string literal (its `\"`, `\n`
            // sequences are valid inside a backtick literal unchanged). Only the two
            // chars special to a TEMPLATE literal but not yet escaped need handling:
            // a backtick (closes the literal) and `${` (opens interpolation). Do NOT
            // re-escape backslashes — that would double the existing valid escapes.
            let escaped = sanitized
                .replace('`', "\\`")
                .replace("${", "\\${");
            Ok(JsExpr::Raw(escaped))
        }
        _ => {
            // Unknown field accessor — emit as runtime property access (fallback)
            Ok(JsExpr::Prop {
                obj: Box::new(parsed.clone()),
                prop: field.to_string(),
            })
        }
    }
}

/// Compile a block-body's raw inner source into the JavaScript of its nested
/// Spacetime directives (PLAN-026). The body arrives as a (possibly quoted) raw
/// string; we parse it as a standalone fragment and compile WITHOUT the runtime
/// wrapper (the host page already has it). On parse/compile failure, returns an
/// empty string so a malformed body degrades to a no-op rather than leaking the
/// raw source as a dead expression.
/// BUG-216: a test/directive body that FAILS TO PARSE must not compile to
/// silence.
///
/// `compile_block_body_to_js` and its scoped sibling recompile an opaque `:block`
/// as a sub-program (FUP-069). Both bailed with `let Ok(ast) = parse(..) else {
/// return String::new() }` — so a body the parser rejected produced an EMPTY
/// string, which is indistinguishable from a body that legitimately emits
/// nothing. The directive vanished, the build stayed green, and `@test` bodies
/// containing it REPORTED PASS: `@assert (false) ("m " + rows.length)` — an
/// assertion that names the failing value, the one you most want to write —
/// asserted nothing at all.
///
/// `EmitContext` carries no diagnostics channel, so this cannot become a compile
/// error here without threading one through every emit path. What it CAN do is
/// refuse to be silent: emit JS that throws with the parse error and its offset.
/// A dropped body then fails loudly at the first moment it runs, and a test
/// containing one FAILS instead of passing vacuously.
///
/// Returning a throw rather than `String::new()` is the whole fix: the caller
/// splices this in place of the body it expected.
fn block_body_parse_failure_js(err: &impl std::fmt::Debug, source: &str) -> String {
    // A parse failure is a BUILD defect, so the message names the construct and
    // shows the body, not just an offset into a fragment the author never sees.
    let detail = format!("{err:?}");
    let preview: String = source.chars().take(400).collect();
    let msg = format!(
        "Spacetime: a directive body failed to parse and was dropped (BUG-216). \
         This body compiled to nothing, so anything inside it - including \
         assertions - did not run. Parse error: {detail}\n\nBody:\n{preview}"
    );
    format!(
        "throw new Error(\"{}\");\n",
        crate::utils::escape_js_string(&msg)
    )
}

/// Like `compile_block_body_to_js`, but the body is wrapped in `selector { ... }`
/// before recompiling, so a CSS-scope body (reactive `.cls: $sig`, scoped-state
/// `$x bool: …`) compiles as a real ScopeBlock bound to that element instead of a
/// selectorless fragment that drops to nothing (BUG-119 §1, the @given path).
/// Directive bodies (`@on &.click { … }`) wrap harmlessly too — a directive inside a
/// selector scope resolves against that selector, the same as file scope.
fn compile_scoped_block_body_to_js(raw_value: &str, selector: &str, ctx: &EmitContext) -> String {
    let inner = strip_js_quotes(raw_value)
        .replace("\\n", "\n")
        .replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\\\", "\\");
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let wrapped = format!("{} {{\n{}\n}}", selector.trim(), trimmed);
    let mut ast = match crate::parser::parse_body_fragment(&wrapped) {
        Ok(ast) => ast,
        // BUG-216: never silently drop a body the parser rejected.
        Err(e) => return block_body_parse_failure_js(&e, trimmed),
    };
    let Some(reg) = &ctx.registry else {
        return String::new();
    };
    crate::parser::rematch_with_registry(&mut ast, &wrapped, reg);
    let compiled = crate::compiler::Compiler::from_ast(&ast)
        .registry((**reg).clone())
        .without_runtime()
        .bindings_first(true)
        .compile()
        .js;
    asyncify_body_scopes(&compiled)
}

/// Extract a block body's DOM-state declarations as a JS object literal (BUG-217).
///
/// Parses the body as a CSS-ish declaration list — `name: value;` pairs — and emits
/// `{"name": "value", ...}`. Only pairs are taken; anything else in the body (a
/// nested directive, a scoped-state decl) is ignored here and still handled by the
/// scope compile, so a mixed body works and a body with no pairs yields `{}`.
///
/// Values keep their source form minus surrounding quotes, so `"modified"` becomes
/// the string `modified` and a bare `true` stays the string `true` (the caller
/// coerces for `checked`/`disabled`, where both spellings are natural to write).
fn block_body_state_props_json(raw_value: &str) -> String {
    let src = strip_js_quotes(raw_value)
        .replace("\\n", "\n")
        .replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\\\", "\\");

    let mut pairs: Vec<String> = Vec::new();
    // Declarations separate on `;` OR `,` — both spellings occur in real bodies
    // (`{ data-value: "42", data-name: "test" }` and `{ opacity: "0.5"; }`), and
    // splitting on `;` alone silently kept only the first pair of a comma list.
    //
    // The split must be QUOTE- AND BRACKET-AWARE. A naive `split([';', ','])` cut
    // `textContent: "a, b"` in half and wrote the truncated `"a` to the element —
    // a WRONG write, which is far worse than a skipped one: the test then asserts
    // against silently corrupted content. Same for a value carrying a separator
    // inside parens, e.g. `background: rgba(0, 0, 0, .5)`.
    let decls = {
        let mut out: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut quote: Option<char> = None;
        let mut depth = 0i32;
        let mut chars = src.chars().peekable();
        while let Some(c) = chars.next() {
            match quote {
                Some(q) => {
                    cur.push(c);
                    if c == '\\' {
                        if let Some(esc) = chars.next() {
                            cur.push(esc);
                        }
                    } else if c == q {
                        quote = None;
                    }
                }
                None => match c {
                    '"' | '\'' => {
                        quote = Some(c);
                        cur.push(c);
                    }
                    '(' | '[' | '{' => {
                        depth += 1;
                        cur.push(c);
                    }
                    ')' | ']' | '}' => {
                        depth -= 1;
                        cur.push(c);
                    }
                    ';' | ',' if depth == 0 => {
                        out.push(std::mem::take(&mut cur));
                    }
                    _ => cur.push(c),
                },
            }
        }
        out.push(cur);
        out
    };
    for raw_decl in decls {
        let decl = raw_decl.trim();
        if decl.is_empty() || decl.starts_with('@') || decl.starts_with('$') {
            continue;
        }
        // Split on the FIRST colon: a value may itself contain one (a URL, a time).
        let Some((name, value)) = decl.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        // A property name is a plain ident / data-attr. Anything else (a selector,
        // a nested block's leftovers) is not ours.
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            continue;
        }
        if value.is_empty() {
            continue;
        }
        let value = value.trim_matches(|c| c == '"' || c == '\'');
        pairs.push(format!(
            "{}: {}",
            serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into()),
            serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
        ));
    }
    format!("{{{}}}", pairs.join(", "))
}

fn compile_block_body_to_js(raw_value: &str, ctx: &EmitContext) -> String {
    // The body text may carry escaped quotes/newlines from string-literal
    // wrapping; unescape the common ones so it parses as source.
    let src = strip_js_quotes(raw_value)
        .replace("\\n", "\n")
        .replace("\\\"", "\"")
        .replace("\\'", "'")
        .replace("\\\\", "\\");
    let trimmed = src.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut ast = match crate::parser::parse_body_fragment(trimmed) {
        Ok(ast) => ast,
        // BUG-216: never silently drop a body the parser rejected.
        Err(e) => return block_body_parse_failure_js(&e, trimmed),
    };
    // The body fragment's directives (@assert/@then/@eval/@fixture) live in the
    // registry, not in the fragment, so the stdlib-only parse matched nothing.
    // Re-extract matches against the live registry's macros before compiling.
    // Without the live registry we cannot resolve those macros — bail to empty.
    let Some(reg) = &ctx.registry else {
        return String::new();
    };
    crate::parser::rematch_with_registry(&mut ast, trimmed, reg);
    // Compile WITHOUT the runtime wrapper (host page already has it).
    let compiled = crate::compiler::Compiler::from_ast(&ast)
        .registry((**reg).clone())
        .without_runtime()
        // BUG-119 §2: a test body's reactive `.sel { .cls: $sig }` binding listener
        // must be wired BEFORE the body's inline `@when click` fires; otherwise the
        // click runs first and the class never reacts. Page builds load synchronously
        // so they don't need this; a test body executes its directives in sequence.
        .bindings_first(true)
        .compile()
        .js;
    // Make each top-level test-body directive IIFE that uses `await` (e.g. @when
    // flushing microtasks / rAF) into an AWAITED async IIFE so the `await` is
    // valid and directives run in order. Per-directive IIFE isolation is kept
    // (nested `const init` blocks from @mount/@on must not collide), so this does
    // NOT merge scopes — cross-directive `@let`/`$error` sharing stays out of scope
    // (tests use window.__X globals for that). (PLAN-027 W3.)
    asyncify_body_scopes(&compiled)
}

/// Rewrite each TOP-LEVEL test-body directive IIFE that contains `await` into an
/// AWAITED async IIFE, preserving per-directive scope isolation. The enclosing
/// test fn is async, so `await (async function(){…})()` is valid and ordered.
/// IIFEs without `await` are left untouched. Nested IIFEs inside a directive's
/// body are NOT descended into (we skip past a directive's matching closer), so
/// their `const init` etc. keep their own scope. Unrecognised output is returned
/// unchanged.
fn asyncify_body_scopes(js: &str) -> String {
    let mut out = String::new();
    let mut rest = js;
    const OPENER: &str = "(function() {";
    const CLOSER: &str = "})();";

    loop {
        let Some(start) = rest.find(OPENER) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_open = &rest[start + OPENER.len()..];
        let Some(close_rel) = find_matching_close(after_open, CLOSER) else {
            // Can't match this directive's closer — emit the rest verbatim.
            out.push_str(&rest[start..]);
            break;
        };
        let inner = &after_open[..close_rel];
        // Only the directive's OWN body decides async-ness; check the top level of
        // `inner` for `await ` that is not inside a deeper nested IIFE. A simple
        // contains() is sufficient here because a nested IIFE's own await would
        // also (correctly) require this directive to be awaited to preserve order.
        if inner.contains("await ") {
            out.push_str("await (async function() {");
        } else {
            out.push_str(OPENER);
        }
        out.push_str(inner);
        out.push_str(CLOSER);
        rest = &after_open[close_rel + CLOSER.len()..];
    }
    out
}

/// Given text starting just after an IIFE's `{`, return the byte offset of the
/// matching `})();` closer (the offset of `}` measured from the slice start),
/// tracking brace depth.
///
/// The closer is located with the vendored swc lexer rather than a hand-rolled
/// byte scanner. The old scanner drifted on real JS shapes the emitters produce
/// — a regex char class like `[.#\[\]=">~+\s]` embedded a `"` it read as a
/// string opener, and template interpolation `${…}` unbalanced the count — so
/// it failed to find a `@mount` directive's closer, `asyncify_body_scopes`
/// bailed, and `@wait`/`@when` `await`s stayed in a non-async IIFE, making the
/// whole bundle fail to parse (E0952). swc's lexer classifies strings, templates
/// (incl. `${…}`), regexes and comments as their own tokens, so a `}` inside any
/// of them never disturbs the depth count.
fn find_matching_close(s: &str, _closer: &str) -> Option<usize> {
    use swc_common::{FileName, SourceMap, input::StringInput};
    use swc_ecma_ast::EsVersion;
    use swc_ecma_parser::{EsSyntax, Syntax, lexer::Lexer, token::Token};

    const OPENER: &str = "(function() {";
    // Lex `s` as the body of a synthetic IIFE so a leading `{` (or bare code)
    // balances against the `})();` closer we are looking for.
    let combined = format!("{OPENER}{s}");
    let cm = SourceMap::default();
    let fm = cm.new_source_file(FileName::Anon.into(), combined.clone());
    let lexer = Lexer::new(
        Syntax::Es(EsSyntax::default()),
        EsVersion::Es2022,
        StringInput::from(&*fm),
        None,
    );
    let mut depth = 0i32;
    for tok in lexer {
        let ts = tok;
        match ts.token {
            // swc's lexer tokenizes strings, templates (incl. `${…}`), regex
            // literals and comments as their own tokens, so a `}` inside any of
            // them never disturbs this count.
            Token::LBrace | Token::DollarLBrace => depth += 1,
            Token::RBrace => {
                depth -= 1;
                if depth == 0 {
                    // span.lo is a 1-based byte offset into `combined`; the
                    // closer lives in `s`, everything after OPENER.
                    let off = (ts.span.lo.0 as usize).saturating_sub(1 + OPENER.len());
                    if s.get(off..).is_some_and(|tail| tail.starts_with("})();")) {
                        return Some(off);
                    }
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

pub fn resolve_stmt(
    stmt: &JsStmt,
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<JsStmt, Diagnostic> {
    match stmt {
        JsStmt::Decl { kind, name, init } => {
            let resolved_init = match init {
                Some(e) => Some(resolve_expr(e, ctx, opts)?),
                None => None,
            };
            Ok(JsStmt::Decl {
                kind: *kind,
                name: name.clone(),
                init: resolved_init,
            })
        }
        JsStmt::Expr(expr) => Ok(JsStmt::Expr(resolve_expr(expr, ctx, opts)?)),
        JsStmt::If { cond, then_, else_ } => {
            let mut resolved_then = Vec::new();
            for s in then_ {
                resolved_then.push(resolve_stmt(s, ctx, opts)?);
            }
            let resolved_else = match else_ {
                Some(stmts) => {
                    let mut resolved = Vec::new();
                    for s in stmts {
                        resolved.push(resolve_stmt(s, ctx, opts)?);
                    }
                    Some(resolved)
                }
                None => None,
            };
            Ok(JsStmt::If {
                cond: resolve_expr(cond, ctx, opts)?,
                then_: resolved_then,
                else_: resolved_else,
            })
        }
        JsStmt::ForOf { var, iter, body } => {
            let mut resolved_body = Vec::new();
            for s in body {
                resolved_body.push(resolve_stmt(s, ctx, opts)?);
            }
            Ok(JsStmt::ForOf {
                var: var.clone(),
                iter: resolve_expr(iter, ctx, opts)?,
                body: resolved_body,
            })
        }
        JsStmt::For {
            init,
            cond,
            update,
            body,
        } => {
            let resolved_init = match init {
                Some(s) => Some(Box::new(resolve_stmt(s, ctx, opts)?)),
                None => None,
            };
            let resolved_cond = match cond {
                Some(e) => Some(resolve_expr(e, ctx, opts)?),
                None => None,
            };
            let resolved_update = match update {
                Some(e) => Some(resolve_expr(e, ctx, opts)?),
                None => None,
            };
            let mut resolved_body = Vec::new();
            for s in body {
                resolved_body.push(resolve_stmt(s, ctx, opts)?);
            }
            Ok(JsStmt::For {
                init: resolved_init,
                cond: resolved_cond,
                update: resolved_update,
                body: resolved_body,
            })
        }
        JsStmt::Return(expr) => {
            let resolved = match expr {
                Some(e) => Some(resolve_expr(e, ctx, opts)?),
                None => None,
            };
            Ok(JsStmt::Return(resolved))
        }
        JsStmt::ForLoopMeta { var, array, body } => {
            let mut resolved_body = Vec::new();
            for s in body {
                resolved_body.push(resolve_stmt(s, ctx, opts)?);
            }
            Ok(JsStmt::ForLoopMeta {
                var: var.clone(),
                array: array.clone(),
                body: resolved_body,
            })
        }
        JsStmt::Break | JsStmt::Continue => Ok(stmt.clone()),
        // Raw is truly raw - no placeholders to resolve
        JsStmt::Raw(_) => Ok(stmt.clone()),
    }
}

/// Parse a parameter value string into an appropriate JsExpr
fn parse_param_value(value: &str) -> JsExpr {
    let trimmed = value.trim();

    // Backtick hole as an arg VALUE (`one sigil, one meaning`): `` `$expr` `` is THE hole
    // form everywhere, including a macro/primitive argument position. When a `string?`
    // param receives a backtick hole (e.g. `@editable(bind: `$f.value`)` in a template
    // body, or `bind: `$post.body`` on a page), the wrapper is the hole sigil and the
    // inner text is the value reference. Normalize to the inner expression as a STRING
    // path literal so the consuming primitive resolves it as a `$`-path (the editable
    // surface's resolveBoundDoc case (b) — instance-scope walk then SpacetimeLocal).
    // Single outer pair only; never touch JS template literals inside `%js` bodies
    // (those reach emission as parsed JsExpr, not through param-value parsing).
    if trimmed.len() >= 2 && trimmed.starts_with('`') && trimmed.ends_with('`') {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        // Only unwrap a `$`-reference hole; leave other backtick content untouched so
        // this stays a targeted normalization of the value-hole form, not a blanket strip.
        if inner.starts_with('$') {
            return JsExpr::Lit(JsLit::String(inner.to_string()));
        }
    }

    // Try parsing as number
    if let Ok(n) = trimmed.parse::<f64>() {
        return JsExpr::Lit(JsLit::Number(n));
    }

    // Try parsing as bool
    if trimmed == "true" {
        return JsExpr::Lit(JsLit::Bool(true));
    }
    if trimmed == "false" {
        return JsExpr::Lit(JsLit::Bool(false));
    }

    // Try parsing as null/undefined
    if trimmed == "null" {
        return JsExpr::Lit(JsLit::Null);
    }
    if trimmed == "undefined" {
        return JsExpr::Lit(JsLit::Undefined);
    }

    // Array literal
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return parse_array_literal(trimmed);
    }

    // Object literal
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return parse_object_literal(trimmed);
    }

    // Quoted string - strip the outer quotes and unescape JS escape sequences.
    // Values from captured_to_js arrive already JS-escaped (e.g. `"hello \"world\"\nfoo"`).
    // Stripping quotes gives `hello \"world\"\nfoo`. Since emit will re-escape when
    // producing the final JS string literal, we must unescape here to avoid double-escaping.
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        return JsExpr::Lit(JsLit::String(unescape_js_string(inner)));
    }

    // Default to string literal (unquoted identifier or value)
    JsExpr::Lit(JsLit::String(trimmed.to_string()))
}

/// Parse an array literal like `[]`, `[1, 2, 3]`, etc.
fn parse_array_literal(s: &str) -> JsExpr {
    // Strip only the outermost [ and ] (not all leading/trailing brackets)
    let inner = if s.starts_with('[') && s.ends_with(']') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    let inner = inner.trim();

    // Empty array
    if inner.is_empty() {
        return JsExpr::Array(Vec::new());
    }

    // Split on commas (simple case - doesn't handle nested arrays/objects with commas)
    let elements: Vec<JsExpr> = split_on_commas(inner)
        .iter()
        .map(|item| parse_param_value(item.trim()))
        .collect();

    JsExpr::Array(elements)
}

/// Parse an object literal like `{}`, `{key: value}`, etc.
fn parse_object_literal(s: &str) -> JsExpr {
    // Strip only the outermost { and } (not all leading/trailing braces)
    let inner = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    let inner = inner.trim();

    // Empty object
    if inner.is_empty() {
        return JsExpr::Object(Vec::new());
    }

    // Split on commas and parse key-value pairs
    let pairs: Vec<(String, JsExpr)> = split_on_commas(inner)
        .iter()
        .filter_map(|item| {
            let item = item.trim();
            // Find the first colon (key: value)
            if let Some(colon_pos) = item.find(':') {
                let key = item[..colon_pos]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                let value = parse_param_value(item[colon_pos + 1..].trim());
                Some((key, value))
            } else {
                None
            }
        })
        .collect();

    JsExpr::Object(pairs)
}

/// Split a string on commas, respecting nested structures and escaped characters
fn split_on_commas(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut escape = false;

    for c in s.chars() {
        if in_string {
            current.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == string_char {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    string_char = c;
                    current.push(c);
                }
                '[' | '{' | '(' => {
                    depth += 1;
                    current.push(c);
                }
                ']' | '}' | ')' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    result.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Convert a serde_json::Value to a JsExpr for loop variable substitution.
///
/// Unlike parse_param_value, this emits strings as Raw (unquoted) because
/// loop variable values are substituted directly into code (e.g., %$b.sel -> .name)
fn json_value_to_js_expr(value: &serde_json::Value) -> JsExpr {
    match value {
        serde_json::Value::Null => JsExpr::Lit(JsLit::Null),
        serde_json::Value::Bool(b) => JsExpr::Lit(JsLit::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                JsExpr::Lit(JsLit::Number(f))
            } else {
                JsExpr::Raw(n.to_string())
            }
        }
        // Emit strings as Raw - they're substituted directly into code
        serde_json::Value::String(s) => JsExpr::Raw(s.clone()),
        serde_json::Value::Array(arr) => {
            JsExpr::Array(arr.iter().map(json_value_to_js_expr).collect())
        }
        serde_json::Value::Object(obj) => {
            let pairs: Vec<(String, JsExpr)> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_value_to_js_expr(v)))
                .collect();
            JsExpr::Object(pairs)
        }
    }
}

/// Expand a ForLoopMeta by iterating over the array and resolving body for each item.
///
/// This is called during resolve_stmts when we encounter a ForLoopMeta.
fn expand_for_loop_meta(
    var: &str,
    array: &str,
    body: &[JsStmt],
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<Vec<JsStmt>, Diagnostic> {
    // Get the array value from params
    let array_json = match ctx.params.get(array) {
        Some(json) => json.clone(),
        None => {
            // Array not found
            return Ok(vec![JsStmt::Raw(format!(
                "/* %for ${} in %{}: array not found */",
                var, array
            ))]);
        }
    };

    // Parse as JSON array
    let items: Vec<serde_json::Value> = match serde_json::from_str(&array_json) {
        Ok(arr) => arr,
        Err(_) => {
            // Failed to parse
            return Ok(vec![JsStmt::Raw(format!(
                "/* %for ${} in %{}: failed to parse array */",
                var, array
            ))]);
        }
    };

    // Empty array = no output
    if items.is_empty() {
        return Ok(Vec::new());
    }

    // Expand body for each item
    let mut result = Vec::new();
    for item in items {
        // Create new context with loop variable bound
        let loop_ctx = ctx.with_loop_var(var, item);
        // Resolve body statements with the new context
        for stmt in body {
            result.extend(resolve_stmts_inner(
                std::slice::from_ref(stmt),
                &loop_ctx,
                opts,
            )?);
        }
    }

    Ok(result)
}

/// Resolve statements, expanding ForLoopMeta and flattening results.
fn resolve_stmts_inner(
    stmts: &[JsStmt],
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<Vec<JsStmt>, Diagnostic> {
    let mut result = Vec::new();
    for stmt in stmts {
        match stmt {
            JsStmt::ForLoopMeta { var, array, body } => {
                // Expand the for loop
                result.extend(expand_for_loop_meta(var, array, body, ctx, opts)?);
            }
            _ => {
                // For all other statements, resolve normally
                result.push(resolve_stmt(stmt, ctx, opts)?);
            }
        }
    }
    Ok(result)
}

/// Resolve statements, expanding ForLoopMeta
pub fn resolve_stmts(
    stmts: &[JsStmt],
    ctx: &EmitContext,
    opts: &EmitOptions,
) -> Result<Vec<JsStmt>, Diagnostic> {
    resolve_stmts_inner(stmts, ctx, opts)
}

// ============================================================================
// Writer-Based Emission (for Source Maps)
// ============================================================================

use super::sourcemap::SourceSpan;
use super::writer::SourceMapWriter;

/// Emit a JavaScript expression to a writer with optional source mapping.
pub fn emit_expr_to_writer(
    expr: &JsExpr,
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
    span: Option<&SourceSpan>,
) -> Result<(), Diagnostic> {
    // Mark the start of this expression if span provided
    if let Some(s) = span {
        writer.mark(s);
    }

    match expr {
        JsExpr::Lit(lit) => {
            writer.write(&emit_lit(lit));
        }
        JsExpr::Var(name) => {
            writer.write(name);
        }
        JsExpr::Prop { obj, prop } => {
            emit_expr_to_writer(obj, opts, writer, None)?;
            if needs_bracket_notation(prop) {
                writer.write("[\"");
                writer.write(&escape_string(prop));
                writer.write("\"]");
            } else {
                writer.write(".");
                writer.write(prop);
            }
        }
        JsExpr::Index { arr, idx } => {
            emit_expr_to_writer(arr, opts, writer, None)?;
            writer.write("[");
            emit_expr_to_writer(idx, opts, writer, None)?;
            writer.write("]");
        }
        JsExpr::Call { callee, args } => {
            emit_expr_to_writer(callee, opts, writer, None)?;
            writer.write("(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    writer.write(", ");
                }
                emit_expr_to_writer(arg, opts, writer, None)?;
            }
            writer.write(")");
        }
        JsExpr::Method { obj, method, args } => {
            emit_expr_to_writer(obj, opts, writer, None)?;
            writer.write(".");
            writer.write(method);
            writer.write("(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    writer.write(", ");
                }
                emit_expr_to_writer(arg, opts, writer, None)?;
            }
            writer.write(")");
        }
        JsExpr::Arrow { params, body } => {
            if params.len() == 1 && is_simple_ident(&params[0]) {
                writer.write(&params[0]);
            } else {
                writer.write("(");
                writer.write(&params.join(", "));
                writer.write(")");
            }
            writer.write(" => ");
            if matches!(**body, JsExpr::Object(_)) {
                writer.write("(");
                emit_expr_to_writer(body, opts, writer, None)?;
                writer.write(")");
            } else {
                emit_expr_to_writer(body, opts, writer, None)?;
            }
        }
        JsExpr::Block {
            stmts,
            expr: block_expr,
        } => {
            let nl = &opts.newline;
            let indent = &opts.indent;
            writer.write("{");
            writer.write(nl);
            for stmt in stmts {
                writer.write(indent);
                emit_stmt_to_writer(stmt, opts, writer, 1, None)?;
                writer.write(nl);
            }
            if let Some(e) = block_expr {
                writer.write(indent);
                writer.write("return ");
                emit_expr_to_writer(e, opts, writer, None)?;
                writer.write(";");
                writer.write(nl);
            }
            writer.write("}");
        }
        JsExpr::Template { parts } => {
            writer.write("`");
            for part in parts {
                match part {
                    TemplatePart::Text(t) => writer.write(&escape_template_string(t)),
                    TemplatePart::Expr(e) => {
                        writer.write("${");
                        emit_expr_to_writer(e, opts, writer, None)?;
                        writer.write("}");
                    }
                }
            }
            writer.write("`");
        }
        JsExpr::Binary { .. } => {
            // For simplicity, use the string-based version for precedence handling
            let s = emit_expr(expr, opts)?;
            writer.write(&s);
        }
        JsExpr::Unary { op, expr: inner } => {
            match op {
                UnaryOp::Not => writer.write("!"),
                UnaryOp::Neg => writer.write("-"),
                UnaryOp::Typeof => writer.write("typeof "),
                UnaryOp::Void => writer.write("void "),
            }
            emit_expr_to_writer(inner, opts, writer, None)?;
        }
        JsExpr::Ternary { cond, then_, else_ } => {
            emit_expr_to_writer(cond, opts, writer, None)?;
            writer.write(" ? ");
            emit_expr_to_writer(then_, opts, writer, None)?;
            writer.write(" : ");
            emit_expr_to_writer(else_, opts, writer, None)?;
        }
        JsExpr::Object(entries) => {
            if entries.is_empty() {
                writer.write("{}");
            } else if opts.minify {
                writer.write("{ ");
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        writer.write(", ");
                    }
                    writer.write(&emit_object_key(k));
                    writer.write(": ");
                    emit_expr_to_writer(v, opts, writer, None)?;
                }
                writer.write(" }");
            } else {
                writer.write("{\n");
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        writer.write(",\n");
                    }
                    writer.write("  ");
                    writer.write(&emit_object_key(k));
                    writer.write(": ");
                    emit_expr_to_writer(v, opts, writer, None)?;
                }
                writer.write("\n}");
            }
        }
        JsExpr::Array(items) => {
            writer.write("[");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    writer.write(", ");
                }
                emit_expr_to_writer(item, opts, writer, None)?;
            }
            writer.write("]");
        }
        JsExpr::Raw(s) => {
            writer.write(s);
        }
        JsExpr::New { callee, args } => {
            writer.write("new ");
            emit_expr_to_writer(callee, opts, writer, None)?;
            writer.write("(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    writer.write(", ");
                }
                emit_expr_to_writer(arg, opts, writer, None)?;
            }
            writer.write(")");
        }
        JsExpr::Marker(_) => {
            return Err(Diagnostic::error(
                DiagnosticCode::E0807,
                "unresolved placeholder/marker reached JS emission".to_string(),
            ));
        }
        JsExpr::Composite(parts) => {
            for part in parts {
                match part {
                    JsPart::Raw(s) => writer.write(s),
                    JsPart::Expr(e) => emit_expr_to_writer(e, opts, writer, None)?,
                    JsPart::Marker(_) => {
                        return Err(Diagnostic::error(
                            DiagnosticCode::E0808,
                            "unresolved marker reached JS emission".to_string(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Emit a JavaScript statement to a writer with optional source mapping.
pub fn emit_stmt_to_writer(
    stmt: &JsStmt,
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
    depth: usize,
    span: Option<&SourceSpan>,
) -> Result<(), Diagnostic> {
    // Mark the start of this statement if span provided
    if let Some(s) = span {
        writer.mark(s);
    }

    let nl = if opts.minify { "" } else { &opts.newline };

    match stmt {
        JsStmt::Decl { kind, name, init } => {
            let kind_str = match kind {
                DeclKind::Const => "const",
                DeclKind::Let => "let",
                DeclKind::Var => "var",
            };
            writer.write(kind_str);
            writer.write(" ");
            writer.write(name);
            if let Some(expr) = init {
                writer.write(" = ");
                emit_expr_to_writer(expr, opts, writer, None)?;
            }
            writer.write(";");
        }
        JsStmt::Expr(expr) => {
            emit_expr_to_writer(expr, opts, writer, None)?;
            writer.write(";");
        }
        JsStmt::If { cond, then_, else_ } => {
            writer.write("if (");
            emit_expr_to_writer(cond, opts, writer, None)?;
            writer.write(") ");
            emit_stmts_block_to_writer(then_, opts, writer, depth)?;
            if let Some(else_stmts) = else_ {
                if !opts.minify {
                    writer.write(nl);
                } else {
                    writer.write(" ");
                }
                writer.write("else ");
                emit_stmts_block_to_writer(else_stmts, opts, writer, depth)?;
            }
        }
        JsStmt::ForOf { var, iter, body } => {
            writer.write("for (const ");
            writer.write(var);
            writer.write(" of ");
            emit_expr_to_writer(iter, opts, writer, None)?;
            writer.write(") ");
            emit_stmts_block_to_writer(body, opts, writer, depth)?;
        }
        JsStmt::For {
            init,
            cond,
            update,
            body,
        } => {
            writer.write("for (");
            if let Some(init_stmt) = init {
                // Emit without trailing semicolon
                let s = emit_stmt(init_stmt, opts, 0)?;
                writer.write(s.trim_end_matches(';'));
            }
            writer.write("; ");
            if let Some(cond_expr) = cond {
                emit_expr_to_writer(cond_expr, opts, writer, None)?;
            }
            writer.write("; ");
            if let Some(update_expr) = update {
                emit_expr_to_writer(update_expr, opts, writer, None)?;
            }
            writer.write(") ");
            emit_stmts_block_to_writer(body, opts, writer, depth)?;
        }
        JsStmt::Return(expr) => {
            writer.write("return");
            if let Some(e) = expr {
                writer.write(" ");
                emit_expr_to_writer(e, opts, writer, None)?;
            }
            writer.write(";");
        }
        JsStmt::Break => {
            writer.write("break;");
        }
        JsStmt::Continue => {
            writer.write("continue;");
        }
        JsStmt::Raw(s) => {
            writer.write(s);
        }
        JsStmt::ForLoopMeta { var, array, body } => {
            // Should be expanded before emission
            writer.write(&format!("/* %for ${} in %{} */ ", var, array));
            emit_stmts_to_writer(body, opts, writer)?;
        }
    }

    Ok(())
}

/// Emit multiple statements to a writer.
pub fn emit_stmts_to_writer(
    stmts: &[JsStmt],
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
) -> Result<(), Diagnostic> {
    let nl = if opts.minify { " " } else { &opts.newline };
    for (i, stmt) in stmts.iter().enumerate() {
        if i > 0 {
            writer.write(nl);
        }
        emit_stmt_to_writer(stmt, opts, writer, 0, None)?;
    }
    Ok(())
}

/// Emit statements as a block with braces to a writer.
fn emit_stmts_block_to_writer(
    stmts: &[JsStmt],
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
    depth: usize,
) -> Result<(), Diagnostic> {
    if stmts.is_empty() {
        writer.write("{}");
        return Ok(());
    }

    let nl = if opts.minify { " " } else { &opts.newline };
    let indent = if opts.minify {
        String::new()
    } else {
        opts.indent.repeat(depth + 1)
    };
    let close_indent = if opts.minify {
        String::new()
    } else {
        opts.indent.repeat(depth)
    };

    writer.write("{");
    writer.write(nl);
    for (i, stmt) in stmts.iter().enumerate() {
        if i > 0 {
            writer.write(nl);
        }
        writer.write(&indent);
        emit_stmt_to_writer(stmt, opts, writer, depth + 1, None)?;
    }
    writer.write(nl);
    writer.write(&close_indent);
    writer.write("}");

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_opts() -> EmitOptions {
        EmitOptions::default()
    }

    fn test_ctx() -> EmitContext {
        EmitContext::new("el")
            .with_param("fps", "60")
            .with_param("autoStart", "true")
            .with_element("container", "containerEl")
    }

    #[test]
    fn resolve_param_to_number() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::Param {
            name: "fps".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();

        assert!(matches!(resolved, JsExpr::Lit(JsLit::Number(n)) if n == 60.0));
    }

    #[test]
    fn resolve_param_to_bool() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::Param {
            name: "autoStart".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();

        assert!(matches!(resolved, JsExpr::Lit(JsLit::Bool(true))));
    }

    #[test]
    fn resolve_unknown_param_returns_error() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::Param {
            name: "unknown".into(),
            field: None,
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagnosticCode::E0800);
    }

    #[test]
    fn resolve_element_to_var() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::Element("container".into());
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();

        assert!(matches!(resolved, JsExpr::Var(name) if name == "containerEl"));
    }

    #[test]
    fn resolve_signal_to_st_get() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::Signal("progress".into());
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();

        match resolved {
            JsExpr::Method { obj, method, args } => {
                assert!(matches!(obj.as_ref(), JsExpr::Var(n) if n == "ST"));
                assert_eq!(method, "get");
                assert!(matches!(&args[0], JsExpr::Var(n) if n == "el"));
                assert!(matches!(&args[1], JsExpr::Lit(JsLit::String(s)) if s == "progress"));
            }
            _ => panic!("Expected Method call"),
        }
    }

    #[test]
    fn resolve_yield_to_st_set() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::Directive {
            keyword: "yield".into(),
            expr: Some(Box::new(JsExpr::Var("now".into()))),
            target: Some("t".into()),
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();

        match resolved {
            JsExpr::Method { obj, method, args } => {
                assert!(matches!(obj.as_ref(), JsExpr::Var(n) if n == "ST"));
                assert_eq!(method, "set");
                assert!(matches!(&args[0], JsExpr::Var(n) if n == "el"));
                assert!(matches!(&args[1], JsExpr::Lit(JsLit::String(s)) if s == "t"));
                assert!(matches!(&args[2], JsExpr::Var(n) if n == "now"));
            }
            _ => panic!("Expected Method call"),
        }
    }

    #[test]
    fn resolve_expr_with_binary_placeholder() {
        let ctx = test_ctx();
        let opts = default_opts();

        // 1000 / %fps
        let expr = JsExpr::Binary {
            left: Box::new(JsExpr::Lit(JsLit::Number(1000.0))),
            op: BinOp::Div,
            right: Box::new(JsExpr::Marker(Marker::Param {
                name: "fps".into(),
                field: None,
                in_string: None,
            })),
        };

        let resolved = resolve_expr(&expr, &ctx, &opts).unwrap();

        match resolved {
            JsExpr::Binary { left, op, right } => {
                assert!(matches!(left.as_ref(), JsExpr::Lit(JsLit::Number(n)) if *n == 1000.0));
                assert_eq!(op, BinOp::Div);
                assert!(matches!(right.as_ref(), JsExpr::Lit(JsLit::Number(n)) if *n == 60.0));
            }
            _ => panic!("Expected Binary"),
        }
    }

    #[test]
    fn emit_resolved_binary_expr() {
        let ctx = test_ctx();
        let opts = default_opts();

        // const interval = 1000 / %fps;
        let stmt = JsStmt::Decl {
            kind: DeclKind::Const,
            name: "interval".into(),
            init: Some(JsExpr::Binary {
                left: Box::new(JsExpr::Lit(JsLit::Number(1000.0))),
                op: BinOp::Div,
                right: Box::new(JsExpr::Marker(Marker::Param {
                    name: "fps".into(),
                    field: None,
                    in_string: None,
                })),
            }),
        };

        let resolved = resolve_stmt(&stmt, &ctx, &opts).unwrap();
        let output = emit_stmt(&resolved, &opts, 0).unwrap();

        assert_eq!(output, "const interval = 1000 / 60;");
    }

    #[test]
    fn emit_resolved_yield() {
        let ctx = test_ctx();
        let opts = default_opts();

        // %yield now -> $t;
        let stmt = JsStmt::Expr(JsExpr::Marker(Marker::Directive {
            keyword: "yield".into(),
            expr: Some(Box::new(JsExpr::Var("now".into()))),
            target: Some("t".into()),
        }));

        let resolved = resolve_stmt(&stmt, &ctx, &opts).unwrap();
        let output = emit_stmt(&resolved, &opts, 0).unwrap();

        assert_eq!(output, "ST.set(el, \"t\", now);");
    }

    #[test]
    fn resolve_yield_signal_param_strips_quotes() {
        // Simulates the real pipeline: captured_to_js() wraps ident values in JS quotes,
        // so ctx.params["name"] = "\"journey-header\"". The signal name in ST.set()
        // should be the bare identifier, not double-quoted.
        let ctx = EmitContext::new("el").with_param("name", "\"journey-header\"");
        let opts = default_opts();
        let marker = Marker::Directive {
            keyword: "yield".into(),
            expr: Some(Box::new(JsExpr::Var("progress".into()))),
            target: Some("%name".into()),
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        assert_eq!(output, "ST.set(el, \"journey-header\", progress)");
    }

    #[test]
    fn resolve_yield_signal_param_unquoted_passthrough() {
        // When the param value has no quotes, it should pass through unchanged
        let ctx = EmitContext::new("el").with_param("name", "fade");
        let opts = default_opts();
        let marker = Marker::Directive {
            keyword: "yield".into(),
            expr: Some(Box::new(JsExpr::Var("progress".into()))),
            target: Some("%name".into()),
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        assert_eq!(output, "ST.set(el, \"fade\", progress)");
    }

    #[test]
    fn strip_js_quotes_tests() {
        assert_eq!(strip_js_quotes("\"hello\""), "hello");
        assert_eq!(strip_js_quotes("'hello'"), "hello");
        assert_eq!(strip_js_quotes("hello"), "hello");
        assert_eq!(strip_js_quotes("\"journey-header\""), "journey-header");
        assert_eq!(strip_js_quotes(""), "");
        assert_eq!(strip_js_quotes("\"\""), "");
    }

    #[test]
    fn parse_param_value_tests() {
        assert!(matches!(parse_param_value("42"), JsExpr::Lit(JsLit::Number(n)) if n == 42.0));
        // 3.14 here is an arbitrary decimal test fixture (verifying
        // generic number-literal parsing), not an intended pi
        // approximation -- clippy::approx_constant is a false positive.
        #[allow(clippy::approx_constant)]
        assert!(
            matches!(parse_param_value("3.14"), JsExpr::Lit(JsLit::Number(n)) if (n - 3.14).abs() < 0.001)
        );
        assert!(matches!(
            parse_param_value("true"),
            JsExpr::Lit(JsLit::Bool(true))
        ));
        assert!(matches!(
            parse_param_value("false"),
            JsExpr::Lit(JsLit::Bool(false))
        ));
        assert!(matches!(
            parse_param_value("null"),
            JsExpr::Lit(JsLit::Null)
        ));
        assert!(
            matches!(parse_param_value("hello"), JsExpr::Lit(JsLit::String(s)) if s == "hello")
        );
    }

    #[test]
    fn parse_param_value_nested_object_not_stringified() {
        // Verify that nested objects in parse_param_value are correctly parsed as JsExpr::Object
        // not as JsExpr::Lit(JsLit::String(...))
        let input = r#"{type: "OnEvent", selector: ".btn", event: "click", action: {type: "Toggle", var: "navOpen"}}"#;
        let result = parse_param_value(input);
        // Should be an Object, not a String
        assert!(
            matches!(result, JsExpr::Object(_)),
            "Expected Object, got {:?}",
            result
        );
        // Emit it and verify action is an object, not a string
        let emitted = emit_expr(&result, &default_opts()).unwrap();
        assert!(emitted.contains("action:"), "Should contain action field");
        // The action should NOT be a quoted string
        assert!(
            !emitted.contains(r#"action: ""#),
            "action should not be a quoted string, got: {}",
            emitted
        );
    }

    #[test]
    fn parse_param_value_unescapes_quoted_strings() {
        // Value from captured_to_js: already JS-escaped and quoted
        let input = r##""<a href=\"#work\">Work</a>\n<a href=\"#growth\">Growth</a>""##;
        let result = parse_param_value(input);
        match result {
            JsExpr::Lit(JsLit::String(s)) => {
                // After stripping quotes + unescaping, should have raw content
                assert!(
                    s.contains("<a href=\"#work\">"),
                    "Should have unescaped quotes: got {:?}",
                    s
                );
                assert!(
                    s.contains('\n'),
                    "Should have real newline, not literal \\n: got {:?}",
                    s
                );
                assert!(
                    !s.contains("\\n"),
                    "Should not have literal \\n: got {:?}",
                    s
                );
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn parse_param_value_quoted_string_roundtrip() {
        use crate::utils::escape_js_string;
        // Simulate what captured_to_js does: escape + wrap in quotes
        let raw_html = "<a href=\"#work\">Work</a>\n<a href=\"#growth\">Growth</a>";
        let js_literal = format!("\"{}\"", escape_js_string(raw_html));
        // parse_param_value should recover the original raw content
        let result = parse_param_value(&js_literal);
        match result {
            JsExpr::Lit(JsLit::String(s)) => {
                assert_eq!(s, raw_html, "Round-trip should preserve original content");
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn parse_param_value_no_double_escape_on_emit() {
        use crate::utils::escape_js_string;
        // Simulate the full pipeline: captured_to_js -> parse_param_value -> emit
        let raw_html = "<a href=\"#work\">Work</a>\n<a>Growth</a>";
        let js_literal = format!("\"{}\"", escape_js_string(raw_html));
        let result = parse_param_value(&js_literal);
        let emitted = emit_expr(&result, &default_opts()).unwrap();
        // The emitted JS should match what captured_to_js originally produced
        assert_eq!(
            emitted, js_literal,
            "Emit should produce the same JS literal as captured_to_js"
        );
    }

    #[test]
    fn emit_unresolved_placeholder_returns_error() {
        let expr = JsExpr::Marker(Marker::Param {
            name: "test".into(),
            field: None,
            in_string: None,
        });
        let result = emit_expr(&expr, &default_opts());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagnosticCode::E0807);
    }

    #[test]
    fn emit_unresolved_marker_returns_error() {
        let expr = JsExpr::Composite(vec![JsPart::Marker(Marker::Param {
            name: "test".into(),
            field: None,
            in_string: None,
        })]);
        let result = emit_expr(&expr, &default_opts());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagnosticCode::E0808);
    }

    #[test]
    fn resolve_unknown_varref_returns_error() {
        let ctx = test_ctx();
        let opts = default_opts();
        let marker = Marker::VarRef {
            var: "unknown".into(),
            field: None,
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagnosticCode::E0805);
    }

    // ========================================================================
    // Tests for DX improvements: available params in error hints
    // ========================================================================

    #[test]
    fn unknown_param_error_includes_available_params_in_hint() {
        let ctx = test_ctx(); // has params: fps, autoStart
        let opts = default_opts();
        let marker = Marker::Param {
            name: "unknown".into(),
            field: None,
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagnosticCode::E0800);

        // Check that hint contains available params
        let hint = err.hint.expect("Error should have a hint");
        assert!(
            hint.contains("available params:"),
            "Hint should mention available params"
        );
        assert!(hint.contains("fps"), "Hint should list 'fps' as available");
        assert!(
            hint.contains("autoStart"),
            "Hint should list 'autoStart' as available"
        );
    }

    #[test]
    fn unknown_param_error_with_empty_params_shows_no_params_message() {
        let ctx = EmitContext::new("el"); // Empty params
        let opts = default_opts();
        let marker = Marker::Param {
            name: "missing".into(),
            field: None,
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts);

        assert!(result.is_err());
        let err = result.unwrap_err();

        let hint = err.hint.expect("Error should have a hint");
        assert!(
            hint.contains("no parameters available"),
            "Hint should say no parameters available"
        );
    }

    #[test]
    fn unknown_marker_param_error_includes_available_params_in_hint() {
        let ctx = test_ctx(); // has params: fps, autoStart
        let opts = default_opts();
        let marker = Marker::Param {
            name: "unknown".into(),
            field: None,
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagnosticCode::E0800);

        // Check that hint contains available params
        let hint = err.hint.expect("Error should have a hint");
        assert!(
            hint.contains("available params:"),
            "Hint should mention available params"
        );
        assert!(hint.contains("fps"), "Hint should list 'fps' as available");
    }

    // ========================================================================
    // Tests for expr param type-aware resolution
    // ========================================================================

    #[test]
    fn resolve_expr_param_not_quoted_as_string() {
        // When param_type is "expr", the value should NOT be wrapped in quotes
        let ctx = EmitContext::new("el")
            .with_param("condition", r#"$navTheme === "dark""#)
            .with_param_type("condition", "expr");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "condition".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        // Must NOT be a quoted string literal (the old buggy behavior)
        assert!(
            !output.starts_with('"'),
            "expr param should not be quoted as string: {}",
            output
        );
        // Should reference navTheme (via signal accessor or raw)
        assert!(
            output.contains("navTheme"),
            "should reference navTheme: {}",
            output
        );
        // Should contain the comparison operator
        assert!(
            output.contains("==="),
            "should contain comparison: {}",
            output
        );
    }

    #[test]
    fn resolve_expr_param_resolves_dollar_var_as_signal() {
        // $varName in expr params should be resolved to ST.get(el, "varName")
        let ctx = EmitContext::new("el")
            .with_param("value", "$progress")
            .with_param_type("value", "expr");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "value".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        // Should be a signal accessor, not a quoted string
        assert!(!output.starts_with('"'), "should not be quoted: {}", output);
        assert!(
            output.contains("progress"),
            "should reference progress: {}",
            output
        );
    }

    #[test]
    fn resolve_string_param_still_quoted() {
        // When param_type is "string", values should still be quoted as before
        let ctx = EmitContext::new("el")
            .with_param("className", r#""my-class""#)
            .with_param_type("className", "string");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "className".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        assert!(
            output.contains("my-class"),
            "should contain class name: {}",
            output
        );
    }

    #[test]
    fn resolve_param_without_type_uses_parse_param_value() {
        // Without param_type, behavior should be unchanged (backward compat)
        let ctx = EmitContext::new("el").with_param("count", "42");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "count".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        assert!(matches!(resolved, JsExpr::Lit(JsLit::Number(n)) if n == 42.0));
    }

    #[test]
    fn resolve_expr_param_with_simple_number() {
        // Even with expr type, simple values should still work
        let ctx = EmitContext::new("el")
            .with_param("threshold", "0.5")
            .with_param_type("threshold", "expr");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "threshold".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        assert!(
            output.contains("0.5"),
            "should contain the number: {}",
            output
        );
    }

    #[test]
    fn resolve_expr_param_mutation_syntax_fallback() {
        // Mutation syntax (contains <-) isn't valid JS, should fall back to Raw
        let ctx = EmitContext::new("el")
            .with_param("actions", r#"$navTheme <- "default""#)
            .with_param_type("actions", "expr");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "actions".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        // Should contain the raw mutation text (not quoted as string)
        assert!(
            output.contains("<-"),
            "should preserve mutation syntax: {}",
            output
        );
    }

    #[test]
    fn resolve_marker_expr_param_not_quoted() {
        // Marker::Param with expr type should also resolve as raw JS, not quoted string.
        // This is the code path used when %condition appears inside arrow functions / Composites.
        let ctx = EmitContext::new("el")
            .with_param("condition", r#"$navTheme === "dark""#)
            .with_param_type("condition", "expr");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "condition".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        assert!(
            !output.starts_with('"'),
            "marker expr param should not be quoted: {}",
            output
        );
        assert!(
            output.contains("navTheme"),
            "should reference navTheme: {}",
            output
        );
        assert!(
            output.contains("==="),
            "should contain comparison: {}",
            output
        );
    }

    #[test]
    fn resolve_marker_string_param_still_quoted() {
        // Marker::Param without expr type should still go through parse_param_value
        let ctx = EmitContext::new("el")
            .with_param("className", r#""my-class""#)
            .with_param_type("className", "string");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "className".into(),
            field: None,
            in_string: None,
        };
        let resolved = resolve_marker(&marker, &ctx, &opts).unwrap();
        let output = emit_expr(&resolved, &opts).unwrap();
        assert!(
            output.contains("my-class"),
            "should contain class name: {}",
            output
        );
    }

    #[test]
    fn test_resolve_param_with_rgba_field() {
        let ctx = EmitContext::new("el")
            .with_param("clearColor", "transparent")
            .with_param_type("clearColor", "color");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "clearColor".into(),
            field: Some("rgba".into()),
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts).unwrap();
        match result {
            JsExpr::Array(items) => {
                assert_eq!(items.len(), 4);
                assert_eq!(items[0], JsExpr::Lit(JsLit::Number(0.0)));
                assert_eq!(items[1], JsExpr::Lit(JsLit::Number(0.0)));
                assert_eq!(items[2], JsExpr::Lit(JsLit::Number(0.0)));
                assert_eq!(items[3], JsExpr::Lit(JsLit::Number(0.0)));
            }
            other => panic!("Expected JsExpr::Array for color.rgba, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_param_with_rgba_field_quoted_value() {
        let ctx = EmitContext::new("el")
            .with_param("clearColor", "\"transparent\"")
            .with_param_type("clearColor", "color");
        let opts = default_opts();
        let marker = Marker::Param {
            name: "clearColor".into(),
            field: Some("rgba".into()),
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx, &opts).unwrap();
        match result {
            JsExpr::Array(items) => {
                assert_eq!(items.len(), 4);
                assert_eq!(items[0], JsExpr::Lit(JsLit::Number(0.0)));
                assert_eq!(items[1], JsExpr::Lit(JsLit::Number(0.0)));
                assert_eq!(items[2], JsExpr::Lit(JsLit::Number(0.0)));
                assert_eq!(items[3], JsExpr::Lit(JsLit::Number(0.0)));
            }
            other => panic!(
                "Expected JsExpr::Array for quoted color.rgba, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn find_matching_close_skips_regex_char_class_quote() {
        // The regex char class `[.#\[\]=">~+\s]` embeds a `"`; the hand-rolled
        // scanner it replaces read that as a string opener, swallowed the rest of
        // the directive and failed to find the closer (BUG-262/E0952). The swc
        // lexer tokenizes the regex atomically, so the closer is found.
        let s = "\n{\n  const n = sel.replace(/[.#\\[\\]=\">~+\\s]/g, '-');\n};\n})();";
        let off = find_matching_close(s, "})();").expect("must find closer");
        assert_eq!(&s[off..off + 5], "})();");
    }

    #[test]
    fn find_matching_close_handles_template_interpolation() {
        // A template literal with multiple `${…}` interpolations must not skew
        // the brace depth (the interpolation braces cancel).
        let s = "\n{\n  return `${k}(${typeof v === 'number' ? v + 'px' : fnArgs(v)})`;\n};\n})();";
        let off = find_matching_close(s, "})();").expect("must find closer");
        assert_eq!(&s[off..off + 5], "})();");
    }

    #[test]
    fn find_matching_close_handles_block_comment_with_brace() {
        // A `/* … */` comment containing a `}` must not close the directive early.
        let s = "\n{\n  const x = 1; /* brace } here */\n};\n})();";
        let off = find_matching_close(s, "})();").expect("must find closer");
        assert_eq!(&s[off..off + 5], "})();");
    }

    #[test]
    fn asyncify_asyncifies_wait_after_regex_mount() {
        // A `@mount` whose body embeds the regex char-class quote, followed by a
        // `@wait`-style directive with `await`: asyncify must still wrap the wait
        // directive in an awaited async IIFE (it used to bail on the mount and
        // leave the await in a non-async IIFE → E0952).
        let js = "(function() {\n{\n  const n = sel.replace(/[.#\\[\\]=\">~+\\s]/g, '-');\n};\n})();\n(function() {\n{\n  await new Promise(r => setTimeout(r, 10));\n};\n})();";
        let out = asyncify_body_scopes(js);
        assert!(out.contains("await (async function() {"), "wait directive must be asyncified");
        assert!(out.contains("await new Promise(r => setTimeout(r, 10))"));
        // The mount directive (no await) stays a plain IIFE.
        assert!(out.contains("(function() {\n{\n  const n = sel.replace"));
    }
}
