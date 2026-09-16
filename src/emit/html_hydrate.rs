//! File-scope HTML reactivity via SSG hydration (FEAT-078).
//!
//! Spacetime is a static-site generator first: file-scope `<tag>` markup must ship REAL HTML
//! (first paint / SEO / no-JS), and JS HYDRATES the `$signal` holes in place \u2014 it must NOT
//! blank-then-rebuild. This module lowers the `HtmlExpr` tree of a file-scope block into:
//!
//!   1. a STATIC HTML string where each hole is rendered as its initial value, wrapped in a
//!      stable hydration marker (`<span data-st-hole="N">INITIAL</span>` for text holes; a
//!      `data-st-hole-N` marker attribute carrying the expr for attribute holes), and
//!   2. reactive binding JS that addresses each marker and re-renders it on `local:<dep>:updated`
//!      \u2014 the SAME Global-scope arc the proven `emit_reactive_binding_js` (CSS bindings) uses.
//!
//! Scope is GLOBAL: file-scope `$x` reads `SpacetimeLocal[x]` and subscribes to
//! `local:<dep>:updated`. Initial values come from the file's `local-state` declarations
//! (compile-time), so a `$count number: 5;` hole renders `5` statically (no-JS correct).
//!
//! Contrast with `html_reactive.rs` (FEAT-077), which BUILDS fresh DOM for `@template` bodies:
//! file-scope is HYDRATE mode (attach to existing server-rendered nodes), so the two share the
//! `transpile_signal_expr_with` lowering but differ in attachment.

use crate::ir::{AttrPart, HtmlExpr, JsExpr};
use std::collections::HashMap;

use super::EmitOptions;

/// Result of lowering file-scope HTML blocks for hydration: the static HTML (initial values +
/// markers) and the reactive binding JS that hydrates the holes.
pub struct HydratedHtml {
    pub html: String,
    pub js: String,
}

/// Lower file-scope HTML blocks into static-initial HTML + hydration JS. `initials` maps a
/// global-signal name to its compile-time initial value (from `local-state` decls), used to
/// render a hole's first value statically (SSG / no-JS). A hole whose value is not statically
/// resolvable renders empty and is filled by the on-load render (still reactive).
pub fn emit_hydrated(
    exprs: &[HtmlExpr],
    initials: &HashMap<String, String>,
    opts: &EmitOptions,
) -> HydratedHtml {
    emit_hydrated_from(exprs, initials, opts, 0).0
}

/// Like [`emit_hydrated`] but seeds the marker-id counter at `start_id` and returns the next
/// free id, so the caller can thread it across a file's multiple top-level blocks — hole ids
/// MUST be unique document-wide (bindings address holes by a global `querySelectorAll`, so a
/// reused id cross-contaminates holes; BUG-067).
pub fn emit_hydrated_from(
    exprs: &[HtmlExpr],
    initials: &HashMap<String, String>,
    opts: &EmitOptions,
    start_id: usize,
) -> (HydratedHtml, usize) {
    let mut ctx = HydrateCtx {
        initials,
        js: String::new(),
        next_id: start_id,
    };
    let html = exprs
        .iter()
        .map(|e| ctx.emit(e, None))
        .collect::<Vec<_>>()
        .join(if opts.minify { "" } else { "\n" });
    (HydratedHtml { html, js: ctx.js }, ctx.next_id)
}

struct HydrateCtx<'a> {
    initials: &'a HashMap<String, String>,
    js: String,
    next_id: usize,
}

impl HydrateCtx<'_> {
    fn emit(&mut self, expr: &HtmlExpr, parent_tag: Option<&str>) -> String {
        match expr {
            HtmlExpr::Element {
                tag,
                attrs,
                children,
                ..
            } => self.emit_element(tag, attrs, children),
            HtmlExpr::Text(text) => escape_html_text(text),
            HtmlExpr::Hole(expr) => self.emit_text_hole(expr, parent_tag),
            HtmlExpr::Raw(s) => s.clone(),
            HtmlExpr::Html(s) => crate::emit::html_reactive::component_html_to_exprs(s)
                .iter()
                .map(|expr| self.emit(expr, parent_tag))
                .collect(),
            // Reactive markdown renders in the browser via snarkdown; the SSG
            // pass emits an empty display:contents span the builder fills.
            HtmlExpr::Markdown(_) => String::from("<span style=\"display:contents\"></span>"),
        }
    }

    /// A text-position hole `<span>`$x`</span>`: emit `<span data-st-hole="N">INITIAL</span>`
    /// and a reactive binding addressing `[data-st-hole="N"]`. The wrapper span carries the
    /// initial value (SSG) and is the stable hydration address.
    fn emit_text_hole(&mut self, expr: &JsExpr, parent_tag: Option<&str>) -> String {
        let JsExpr::Raw(src) = expr else {
            // Already-lowered exprs are not produced on the file-scope path; render nothing
            // rather than leak debug text.
            return String::new();
        };
        // In RCDATA (`<title>`, `<textarea>`, `<script>`, `<style>`) and table/list structural
        // parents (`<tr>`, `<table>`, `<select>`, `<ul>`, …) a `<span>` wrapper is invalid — it
        // renders as literal text or is foster-parented out, breaking both SSG and the
        // querySelector hydration. There is no valid hole-marker element in those positions, so
        // render the static initial value ONLY (SSG-correct, no reactivity there; BUG-067 #4).
        if parent_tag.is_some_and(|t| !allows_span_child(t)) {
            return escape_html_text(&self.static_initial(src));
        }
        let id = self.fresh_id();
        let selector = format!("[data-st-hole=\"{id}\"]");
        // Text hole = a single signal expression; transpile it directly. `is_text = true` is
        // passed EXPLICITLY (not sniffed from a property name, which misfires on an attribute
        // literally named "content"; BUG-067).
        let deps = crate::syntax::collect_signal_deps(src);
        if !deps.is_empty() {
            let value_js =
                crate::syntax::transpile_signal_expr_with(src, crate::syntax::SignalScope::Global);
            self.emit_binding(&selector, true, "", &value_js, &deps);
        }
        let initial = self.static_initial(src);
        format!(
            "<span data-st-hole=\"{id}\">{init}</span>",
            id = id,
            init = escape_html_text(&initial)
        )
    }

    fn emit_element(
        &mut self,
        tag: &str,
        attrs: &[(String, Vec<AttrPart>)],
        children: &[HtmlExpr],
    ) -> String {
        // Partition attributes: a value containing a hole becomes a reactive attribute binding
        // (addressed by an injected `data-st-attr-<name>="N"` marker) rendered to its initial
        // value statically; pure-literal attributes pass through.
        let mut attr_str = String::new();
        let mut hole_markers = String::new();
        for (name, parts) in attrs {
            if parts.iter().any(|p| matches!(p, AttrPart::Hole(_))) {
                let id = self.fresh_id();
                let marker = format!("data-st-attr-{name}");
                // Build a PROPER JS value expression from the parts (literals js-string-quoted,
                // holes transpiled + parenthesised, joined with `+`) — NOT a raw concat, which
                // would splice literal text into JS and break the bundle (BUG-067). Also build
                // the static initial value from the same parts.
                let initial = self.attr_initial(parts);
                let (value_js, deps) = attr_value_js(parts);
                let selector = format!("[{marker}=\"{id}\"]");
                if !deps.is_empty() {
                    self.emit_binding(&selector, false, name, &value_js, &deps);
                }
                // Static attribute = initial value; marker = hydration address.
                attr_str.push_str(&format!(" {name}=\"{}\"", escape_attr_value(&initial)));
                hole_markers.push_str(&format!(" {marker}=\"{id}\""));
            } else {
                let lit: String = parts
                    .iter()
                    .map(|p| match p {
                        AttrPart::Lit(s) => escape_attr_value(s),
                        AttrPart::Hole(_) => String::new(),
                    })
                    .collect();
                attr_str.push_str(&format!(" {name}=\"{lit}\""));
            }
        }

        if is_void_element(tag) {
            return format!("<{tag}{attr_str}{hole_markers}>");
        }
        let inner: String = children.iter().map(|c| self.emit(c, Some(tag))).collect();
        format!("<{tag}{attr_str}{hole_markers}>{inner}</{tag}>")
    }

    /// Emit the proven Global-scope reactive arc (mirrors compiler::emit_reactive_binding_js):
    /// `apply()` = transpiled expr; `render()` sets textContent / attribute on every node
    /// matching `selector`; subscribe each dep via `local:<dep>:updated`; render once on load.
    fn emit_binding(
        &mut self,
        selector: &str,
        is_text: bool,
        attr_name: &str,
        value_js: &str,
        deps: &[String],
    ) {
        if deps.is_empty() {
            return;
        }
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let sel_js = esc(selector);
        let attr_js = esc(attr_name);

        self.js.push_str("(() => {\n");
        self.js.push_str("  const apply = () => (");
        self.js.push_str(value_js);
        self.js.push_str(");\n");
        self.js.push_str("  const render = () => {\n");
        self.js.push_str("    let __v;\n");
        self.js
            .push_str("    try { __v = apply(); } catch (e) { return; }\n");
        self.js.push_str(&format!(
            "    document.querySelectorAll(\"{sel}\").forEach((node) => {{\n",
            sel = sel_js
        ));
        if is_text {
            self.js.push_str(
                "      node.textContent = (__v === undefined || __v === null) ? '' : __v;\n",
            );
        } else {
            self.js
                .push_str(&crate::compiler::attribute_injection_js(&attr_js));
        }
        self.js.push_str("    });\n");
        self.js.push_str("  };\n");
        for dep in deps {
            // Bridge element-scoped primitive exports (e.g. @measure's `%yield
            // $lines`) into the body-scope plane this binding reads (BUG-069):
            // declaring the dep as bridged makes ST.set mirror it to
            // _localState + dispatch `local:<dep>:updated`. Harmless for deps
            // that are already body-scoped (local-state / @data) — the set is
            // a no-op and the event is the one they already emit.
            self.js.push_str(&format!(
                "  if (typeof ST !== 'undefined' && ST._bridgeExport) ST._bridgeExport(\"{dep}\");\n",
                dep = esc(dep)
            ));
            self.js.push_str(&format!(
                "  document.addEventListener(\"local:{dep}:updated\", render);\n",
                dep = esc(dep)
            ));
        }
        self.js.push_str("  if (document.readyState === 'loading') { document.addEventListener('DOMContentLoaded', render); } else { render(); }\n");
        self.js.push_str("})();\n");
    }

    /// Compile-time initial value for a hole expression, as the string the runtime's first
    /// `apply()` render would produce (SSG / no-JS correct). Returns "" when the value cannot be
    /// proven at compile time — the hole then renders empty and is filled by the on-load render.
    ///
    /// CORRECTNESS INVARIANT: a non-empty result MUST equal the runtime render, else hydration
    /// would flash a wrong value before correcting it. So the evaluator is JS-faithful over the
    /// known global initials and BAILS (-> None -> "") on ANY uncertainty.
    ///
    /// Resolved (the hydrate-on-load BOUNDARY): literals (number / "string" / true|false / null),
    /// a bare `$x` from a known global initial, and `+ - * /` arithmetic / string-`+` concat
    /// chains over those (JS semantics). NOT resolved (stay empty + hydrate): function calls,
    /// filters (`|`), member access (`.`), comparisons, ternaries, an unknown signal, or any
    /// operand whose initial is itself unresolved.
    fn static_initial(&self, src: &str) -> String {
        match eval_static(src.trim(), self.initials) {
            Some(v) => v.render(),
            None => String::new(),
        }
    }

    /// Build the static INITIAL attribute value from a part list: literals verbatim, a `$x`
    /// hole contributes its compile-time static initial. (The reactive expr is built separately
    /// by `attr_value_js`, which correctly quotes literals — BUG-067.)
    fn attr_initial(&self, parts: &[AttrPart]) -> String {
        let mut initial = String::new();
        for part in parts {
            match part {
                AttrPart::Lit(s) => initial.push_str(s),
                AttrPart::Hole(JsExpr::Raw(src)) => initial.push_str(&self.static_initial(src)),
                AttrPart::Hole(_) => {}
            }
        }
        initial
    }

    fn fresh_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Build the reactive JS value expression + ordered deps for an attribute part list. Mirrors
/// `html_reactive::attr_value_expr`: literals are js-string-quoted, holes are transpiled
/// (Global scope) + parenthesised, all joined with `+`. This is the CORRECT way to interpolate
/// a mixed literal+hole attribute (`class="btn `$variant`"`) — raw-concatenating literal text
/// into JS produces a SyntaxError that breaks the whole bundle (BUG-067).
fn attr_value_js(parts: &[AttrPart]) -> (String, Vec<String>) {
    use crate::syntax::{SignalScope, collect_signal_deps, transpile_signal_expr_with};
    let mut pieces: Vec<String> = Vec::new();
    let mut deps: Vec<String> = Vec::new();
    for part in parts {
        match part {
            AttrPart::Lit(s) => pieces.push(js_string(s)),
            AttrPart::Hole(JsExpr::Raw(src)) => {
                pieces.push(format!(
                    "({})",
                    transpile_signal_expr_with(src, SignalScope::Global)
                ));
                for d in collect_signal_deps(src) {
                    if !deps.contains(&d) {
                        deps.push(d);
                    }
                }
            }
            AttrPart::Hole(_) => {}
        }
    }
    let expr = if pieces.is_empty() {
        "''".to_string()
    } else {
        pieces.join(" + ")
    };
    (expr, deps)
}

/// JS single-quoted string literal with the minimal escapes for embedding arbitrary text.
fn js_string(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("'{}'", escaped)
}

/// A compile-time-resolved value, mirroring the JS runtime types a hole initial can take.
#[derive(Clone, Debug, PartialEq)]
enum StaticValue {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
}

impl StaticValue {
    /// Render as the runtime would for textContent / an attribute: JS `String(v)` semantics.
    /// (null/undefined render as the empty string in the binding's textContent path.)
    fn render(&self) -> String {
        match self {
            StaticValue::Num(n) => fmt_js_number(*n),
            StaticValue::Str(s) => s.clone(),
            StaticValue::Bool(b) => b.to_string(),
            StaticValue::Null => String::new(),
        }
    }
}

/// JS `String(number)`: integers print without a trailing `.0`.
fn fmt_js_number(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

/// Evaluate a hole expression to a compile-time `StaticValue`, or None if it cannot be proven
/// (bail-heavy: any unhandled construct -> None). Handles `+ - * /` with JS semantics (string
/// `+` concatenates; other ops coerce to number) over literals and known global `$signals`.
fn eval_static(expr: &str, initials: &HashMap<String, String>) -> Option<StaticValue> {
    let e = expr.trim();
    if e.is_empty() {
        return None;
    }
    // Reject constructs we deliberately do NOT statically resolve (member access, calls, filters,
    // comparisons, logical/ternary). These stay empty + hydrate-on-load. (Checked on the WHOLE
    // expression; string literals containing these chars are parsed as atoms below before any
    // operator split, so a quoted `"a.b"` is unaffected — split only happens at top-level ops.)
    // Additive split first (lowest precedence among what we support).
    if let Some((l, op, r)) = split_top_binop(e, &['+']) {
        return apply_op(eval_static(l, initials)?, op, eval_static(r, initials)?);
    }
    if let Some((l, op, r)) = split_top_binop(e, &['-']) {
        return apply_op(eval_static(l, initials)?, op, eval_static(r, initials)?);
    }
    if let Some((l, op, r)) = split_top_binop(e, &['*', '/', '%']) {
        return apply_op(eval_static(l, initials)?, op, eval_static(r, initials)?);
    }
    eval_atom(e, initials)
}

/// Evaluate a non-operator atom: a parenthesised group, a literal, or a bare `$signal`.
fn eval_atom(e: &str, initials: &HashMap<String, String>) -> Option<StaticValue> {
    let e = e.trim();
    if let Some(inner) = e.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        return eval_static(inner, initials);
    }
    // String literal.
    if (e.starts_with('"') && e.ends_with('"') && e.len() >= 2)
        || (e.starts_with('\'') && e.ends_with('\'') && e.len() >= 2)
    {
        // Reject embedded quotes of the same kind (would be two literals, not one atom).
        let inner = &e[1..e.len() - 1];
        let q = e.as_bytes()[0] as char;
        if !inner.contains(q) {
            return Some(StaticValue::Str(inner.to_string()));
        }
        return None;
    }
    match e {
        "true" => return Some(StaticValue::Bool(true)),
        "false" => return Some(StaticValue::Bool(false)),
        "null" | "undefined" => return Some(StaticValue::Null),
        _ => {}
    }
    // Numeric literal.
    if let Ok(n) = e.parse::<f64>() {
        return Some(StaticValue::Num(n));
    }
    // Bare `$signal` (no member access / call — those chars make parse below fail).
    if let Some(name) = e.strip_prefix('$')
        && !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        // Resolve from the known global initial (itself an expression: 42 / "x" / true).
        let raw = initials.get(name)?;
        return eval_static(raw, initials);
    }
    None
}

/// Apply a binary operator with JS semantics: `+` concatenates when either side is a string,
/// else numeric; `- * / %` coerce both sides to number.
fn apply_op(l: StaticValue, op: char, r: StaticValue) -> Option<StaticValue> {
    if op == '+' && (matches!(l, StaticValue::Str(_)) || matches!(r, StaticValue::Str(_))) {
        return Some(StaticValue::Str(format!("{}{}", l.render(), r.render())));
    }
    let (a, b) = (to_number(&l)?, to_number(&r)?);
    let n = match op {
        '+' => a + b,
        '-' => a - b,
        '*' => a * b,
        '/' => a / b,
        '%' => a % b,
        _ => return None,
    };
    Some(StaticValue::Num(n))
}

/// JS numeric coercion for the operands we accept (number / bool / numeric-string). A non-numeric
/// string -> None (bail; the runtime would produce NaN and we must not guess).
fn to_number(v: &StaticValue) -> Option<f64> {
    match v {
        StaticValue::Num(n) => Some(*n),
        StaticValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        StaticValue::Null => Some(0.0),
        StaticValue::Str(s) => s.trim().parse::<f64>().ok(),
    }
}

/// Split `e` at the LAST top-level (depth-0, outside string literals) occurrence of one of `ops`,
/// returning (left, op, right). Last-occurrence gives left-associative evaluation. Returns None
/// if no such operator exists at depth 0. A leading-position match (unary +/-) is not split.
/// Split `e` at the LAST top-level (depth-0, outside string literals) BINARY occurrence of one
/// of `ops`, returning (left, op, right). Last-occurrence gives left-associative evaluation.
/// Returns None if no such operator exists at depth 0. A `+`/`-` in operator/leading position
/// (unary sign) is NOT a split: we require a real left operand (the previous non-space char is
/// alphanumeric / `_` / `)` / `]` / quote).
fn split_top_binop<'a>(e: &'a str, ops: &[char]) -> Option<(&'a str, char, &'a str)> {
    let bytes = e.as_bytes();
    let mut depth = 0i32;
    let mut in_q: Option<u8> = None;
    let mut found: Option<usize> = None;
    for (i, &c) in bytes.iter().enumerate() {
        match in_q {
            Some(q) => {
                if c == q && (i == 0 || bytes[i - 1] != b'\\') {
                    in_q = None;
                }
            }
            None => match c {
                b'"' | b'\'' => in_q = Some(c),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ if depth == 0 && ops.contains(&(c as char)) => {
                    // Require a real left operand: the previous NON-SPACE char must end an atom
                    // (alnum / `_` / `)` / `]` / quote). Otherwise this `+`/`-` is unary, or a
                    // `*`/`/` with no left operand — not a binary split.
                    let prev = bytes[..i].iter().rev().find(|&&b| !b.is_ascii_whitespace());
                    let is_binary = matches!(
                        prev,
                        Some(&p) if p.is_ascii_alphanumeric()
                            || p == b'_' || p == b')' || p == b']' || p == b'"' || p == b'\''
                    );
                    if is_binary {
                        found = Some(i);
                    }
                }
                _ => {}
            },
        }
    }
    let idx = found?;
    let op = bytes[idx] as char;
    Some((&e[..idx], op, &e[idx + 1..]))
}

/// Whether a `<span>` is a valid child of `tag`. False for RCDATA elements (markup not parsed)
/// and table/list structural parents that foster-parent or reject a `<span>` child (BUG-067 #4).
fn allows_span_child(tag: &str) -> bool {
    !matches!(
        tag,
        // RCDATA / raw-text: content is literal text, not markup.
        "title" | "textarea" | "script" | "style"
        // table structure: a <span> is foster-parented out of these.
        | "table" | "thead" | "tbody" | "tfoot" | "tr" | "colgroup"
        // list / select structure: only <li> / <option> / <optgroup> are valid children.
        | "ul" | "ol" | "select" | "optgroup" | "datalist"
    )
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn escape_html_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr_value(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}
#[cfg(test)]
mod feat078_tests {
    use crate::emit::EmitOptions;
    use crate::emit::html_hydrate::emit_hydrated;
    use crate::ir::{AttrPart, HtmlExpr, JsExpr};
    use std::collections::HashMap;

    fn initials() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("count".to_string(), "5".to_string());
        m.insert("name".to_string(), "\"Ada\"".to_string());
        m
    }

    #[test]
    fn text_hole_renders_initial_and_marker_and_binding() {
        // <span>`$count`</span>
        let tree = vec![HtmlExpr::Element {
            tag: "span".into(),
            attrs: vec![],
            children: vec![HtmlExpr::Hole(JsExpr::Raw("$count".into()))],
            line: None,
        }];
        let r = emit_hydrated(&tree, &initials(), &EmitOptions::default());
        // SSG: initial value baked in, wrapped in a stable hydration marker.
        assert!(
            r.html.contains("data-st-hole=\"0\""),
            "marker missing: {}",
            r.html
        );
        assert!(
            r.html.contains(">5</span>"),
            "initial value missing: {}",
            r.html
        );
        assert!(
            !r.html.contains("st-hole:"),
            "no comment placeholder: {}",
            r.html
        );
        // Reactive: Global-scope binding addressing the marker + local:count:updated listener.
        assert!(
            r.js.contains("[data-st-hole=\\\"0\\\"]"),
            "selector binding missing: {}",
            r.js
        );
        assert!(
            r.js.contains("SpacetimeLocal['count']"),
            "global expr missing: {}",
            r.js
        );
        assert!(
            r.js.contains("local:count:updated"),
            "dep listener missing: {}",
            r.js
        );
    }

    #[test]
    fn attr_hole_renders_initial_and_binding() {
        // <a href="`$name`">x</a>
        let tree = vec![HtmlExpr::Element {
            tag: "a".into(),
            attrs: vec![(
                "href".into(),
                vec![AttrPart::Hole(JsExpr::Raw("$name".into()))],
            )],
            children: vec![HtmlExpr::Text("x".into())],
            line: None,
        }];
        let r = emit_hydrated(&tree, &initials(), &EmitOptions::default());
        assert!(
            r.html.contains("href=\"Ada\""),
            "attr initial missing: {}",
            r.html
        );
        assert!(
            r.html.contains("data-st-attr-href=\"0\""),
            "attr marker missing: {}",
            r.html
        );
        assert!(
            r.js.contains("local:name:updated"),
            "attr dep listener missing: {}",
            r.js
        );
    }

    #[test]
    fn attr_hole_marker_named_by_attr() {
        // The attr-hole marker is named after the attribute (`data-st-attr-data-n`) and the
        // initial value is baked into the real attribute (SSG).
        let tree = vec![HtmlExpr::Element {
            tag: "p".into(),
            attrs: vec![(
                "data-n".into(),
                vec![AttrPart::Hole(JsExpr::Raw("$count".into()))],
            )],
            children: vec![HtmlExpr::Text("x".into())],
            line: None,
        }];
        let r = emit_hydrated(&tree, &initials(), &EmitOptions::default());
        assert!(r.html.contains("data-n=\"5\""), "attr initial: {}", r.html);
        assert!(
            r.html.contains("data-st-attr-data-n="),
            "attr marker: {}",
            r.html
        );
        assert!(
            r.js.contains("local:count:updated"),
            "attr binding: {}",
            r.js
        );
    }

    #[test]
    fn mixed_literal_hole_attr_emits_valid_js() {
        // BUG-067 #1: class="btn `$variant`" must NOT splice raw literal into JS.
        let tree = vec![HtmlExpr::Element {
            tag: "button".into(),
            attrs: vec![(
                "class".into(),
                vec![
                    AttrPart::Lit("btn ".into()),
                    AttrPart::Hole(JsExpr::Raw("$variant".into())),
                ],
            )],
            children: vec![HtmlExpr::Text("x".into())],
            line: None,
        }];
        let r = emit_hydrated(&tree, &initials(), &EmitOptions::default());
        // The reactive expr quotes the literal + concatenates the signal (no `btn $variant`).
        assert!(
            r.js.contains("'btn '"),
            "literal must be js-quoted: {}",
            r.js
        );
        assert!(
            r.js.contains("SpacetimeLocal['variant']"),
            "hole transpiled: {}",
            r.js
        );
        assert!(
            !r.js.contains("(btn "),
            "no raw literal spliced into JS: {}",
            r.js
        );
        assert!(
            r.js.contains("local:variant:updated"),
            "dep listener: {}",
            r.js
        );
    }

    #[test]
    fn content_attr_not_misrouted_to_textcontent() {
        // BUG-067 #3: <meta content="`$desc`"> must setAttribute, not textContent.
        let mut m = HashMap::new();
        m.insert("desc".to_string(), "\"hi\"".to_string());
        let tree = vec![HtmlExpr::Element {
            tag: "meta".into(),
            attrs: vec![(
                "content".into(),
                vec![AttrPart::Hole(JsExpr::Raw("$desc".into()))],
            )],
            children: vec![],
            line: None,
        }];
        let r = emit_hydrated(&tree, &m, &EmitOptions::default());
        assert!(
            r.js.contains("setAttribute(\"content\""),
            "must setAttribute content: {}",
            r.js
        );
        assert!(
            !r.js.contains("node.textContent"),
            "must NOT use textContent: {}",
            r.js
        );
    }

    #[test]
    fn rcdata_parent_renders_static_no_span() {
        // BUG-067 #4: a hole in <title> (RCDATA) must NOT wrap in <span> (would render literally).
        let tree = vec![HtmlExpr::Element {
            tag: "title".into(),
            attrs: vec![],
            children: vec![HtmlExpr::Hole(JsExpr::Raw("$name".into()))],
            line: None,
        }];
        let r = emit_hydrated(&tree, &initials(), &EmitOptions::default());
        assert!(
            r.html.contains("<title>Ada</title>"),
            "RCDATA renders static value, no span: {}",
            r.html
        );
        assert!(
            !r.html.contains("data-st-hole"),
            "no marker in RCDATA: {}",
            r.html
        );
    }

    fn ev(expr: &str) -> String {
        let mut m = HashMap::new();
        m.insert("a".to_string(), "3".to_string());
        m.insert("b".to_string(), "4".to_string());
        m.insert("label".to_string(), "\"Items\"".to_string());
        m.insert("active".to_string(), "true".to_string());
        super::eval_static(expr, &m)
            .map(|v| v.render())
            .unwrap_or_default()
    }

    #[test]
    fn static_evaluator_resolves_bounded_exprs() {
        // literals + bare signal (FUP-039 Part 1)
        assert_eq!(ev("$a"), "3");
        assert_eq!(ev("$label"), "Items");
        assert_eq!(ev("42"), "42");
        assert_eq!(ev("\"hi\""), "hi");
        assert_eq!(ev("true"), "true");
        // arithmetic (JS numeric)
        assert_eq!(ev("$a + $b"), "7");
        assert_eq!(ev("$a * 2"), "6");
        assert_eq!(ev("$b - 1"), "3");
        assert_eq!(ev("5 + $a + $b"), "12");
        assert_eq!(ev("($a + $b) * 2"), "14");
        // string concat (JS `+` with a string operand)
        assert_eq!(ev("$label + \"!\""), "Items!");
        assert_eq!(ev("\"#\" + $a"), "#3");
        // bool numeric coercion (JS: true -> 1)
        assert_eq!(ev("$active + 1"), "2");
    }

    #[test]
    fn static_evaluator_bails_on_unresolvable() {
        // unknown signal, member access, call, filter, comparison -> empty (hydrate-on-load)
        assert_eq!(ev("$unknown"), "");
        assert_eq!(ev("$a.b"), "");
        assert_eq!(ev("currency($a)"), "");
        assert_eq!(ev("$a | currency"), "");
        assert_eq!(ev("$a > 2"), "");
        assert_eq!(ev("$a ? 1 : 2"), "");
        // non-numeric string in arithmetic -> bail (would be NaN at runtime)
        assert_eq!(ev("$label * 2"), "");
    }

    #[test]
    fn no_js_correct_static_html() {
        // With JS disabled, the page shows the real value — no comment placeholders anywhere.
        let tree = vec![HtmlExpr::Element {
            tag: "p".into(),
            attrs: vec![],
            children: vec![
                HtmlExpr::Text("Count: ".into()),
                HtmlExpr::Hole(JsExpr::Raw("$count".into())),
            ],
            line: None,
        }];
        let r = emit_hydrated(&tree, &initials(), &EmitOptions::default());
        assert!(r.html.contains("Count: "), "static text: {}", r.html);
        assert!(r.html.contains(">5</span>"), "static initial: {}", r.html);
        assert!(
            !r.html.contains("<!--"),
            "no comment placeholder: {}",
            r.html
        );
    }
}
