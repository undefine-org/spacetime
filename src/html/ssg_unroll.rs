//! Static `@each` SSG unroll (PLAN-023 W4).
//!
//! For an `@each` whose source is a STATIC inline `@data` (compile-time-constant
//! array), the rows are unrolled into the SERVED HTML at compile time — so a
//! crawler / no-JS client sees real `<li>…</li>` markup (SEO), and the runtime
//! `each-inline` hydration produces the SAME DOM (the unroll == hydrate
//! invariant). Dynamic sources (fetch / derived / mutated signals) are left to
//! the runtime hydrate path untouched.
//!
//! The pass is intentionally string-level and conservative: it only fires when
//! (a) the source resolves to a compile-time JSON array, AND (b) the enclosing
//! scope's selector matches exactly one empty container element in the captured
//! HTML blocks. Anything it cannot prove static/unambiguous is left for runtime.

use std::collections::HashMap;

use serde_json::Value;

use crate::parser::ast::{HtmlBlockAst, ScopeBlock, StFile};
use crate::syntax::{CapturedValue, FormMatch, JsQuoting};
use std::path::Path;

/// Collect compile-time-constant array sources: `@data inline $name : [ … ]`.
/// Returns name (no `$`) -> parsed JSON array. Non-array or non-parseable inline
/// values are skipped (they are not unrollable).
pub fn collect_static_arrays(ast: &StFile) -> HashMap<String, Vec<Value>> {
    collect_arrays(ast, None)
}

/// Like `collect_static_arrays`, but when `site_dir` is provided ALSO reads
/// file-sourced `@data fetch … : "/data/x.json"` arrays from disk at build time
/// (Stage-3 Wave-6 compile-time hydration). The on-disk JSON is the same content
/// the runtime would fetch, so prerender == hydrate. Missing/unparseable files
/// are skipped (left to runtime). Path is resolved under `site_dir`, never above
/// it (traversal-guarded).
pub fn collect_arrays(ast: &StFile, site_dir: Option<&Path>) -> HashMap<String, Vec<Value>> {
    let mut out = HashMap::new();
    for fm in &ast.matches {
        if is_inline_data(fm) {
            let Some(name) = data_name(fm) else { continue };
            let Some(value) = fm.get("value") else {
                continue;
            };
            let json = value.to_js(JsQuoting::DoubleQuoted);
            if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&json) {
                out.insert(name, items);
            }
        } else if let Some(base) = site_dir {
            // File-sourced @data: read the JSON array from disk at build time.
            if fm.macro_name != "data" {
                continue;
            }
            let Some(name) = data_name(fm) else { continue };
            let Some(src) = fm.get("src").and_then(|v| v.as_string_literal()) else {
                continue;
            };
            if let Some(items) = read_file_array(base, src) {
                out.insert(name, items);
            }
        }
    }
    out
}

/// Read a `@data` file source's JSON array from disk, resolved under `base`.
/// Returns None for non-arrays, missing files, or any path that escapes `base`.
fn read_file_array(base: &Path, src: &str) -> Option<Vec<Value>> {
    // Source paths are site-absolute ("/data/x.json") or relative; strip a
    // leading '/' and join under base. Reject traversal.
    let rel = src.trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") {
        return None;
    }
    let path = base.join(rel);
    // Containment guard: the resolved path must stay under base.
    match (path.canonicalize(), base.canonicalize()) {
        (Ok(p), Ok(b)) if p.starts_with(&b) => {}
        _ => return None,
    }
    let text = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(items)) => Some(items),
        _ => None,
    }
}

fn is_inline_data(fm: &FormMatch) -> bool {
    // Unified surface: matched_macro == "data-inline"; or a legacy `@data` whose
    // `src`/kind resolves to inline. We key on the kind word captured as `kind`,
    // falling back to the matched macro name.
    if fm.matched_macro.as_deref() == Some("data-inline") {
        return true;
    }
    matches!(fm.get("kind"), Some(CapturedValue::Ident(k)) if k == "inline")
}

fn data_name(fm: &FormMatch) -> Option<String> {
    match fm.get("name") {
        Some(CapturedValue::Binding(s)) => Some(s.trim_start_matches('$').to_string()),
        Some(CapturedValue::Ident(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Unroll static `@each` rows into the captured HTML blocks. Mutates `blocks`
/// in place: for every scope whose `@each` source is static and whose selector
/// matches an empty container in the blocks, the rendered rows are injected.
pub fn unroll_static_each(
    ast: &StFile,
    blocks: &mut [HtmlBlockAst],
    site_dir: Option<&Path>,
    registry: Option<&crate::metasystem::MetaRegistry>,
) {
    let statics = collect_arrays(ast, site_dir);
    if statics.is_empty() {
        return;
    }
    // Collected once: an `@each` body spelled as `&row($i);` resolves against the
    // file's named templates (BUG-348).
    let templates = collect_static_templates(ast);
    for scope in &ast.scopes {
        unroll_scope(scope, &statics, &templates, blocks, registry);
    }
}

fn unroll_scope(
    scope: &ScopeBlock,
    statics: &HashMap<String, Vec<Value>>,
    templates: &HashMap<String, StaticTemplate>,
    blocks: &mut [HtmlBlockAst],
    registry: Option<&crate::metasystem::MetaRegistry>,
) {
    for fm in &scope.matches {
        if fm.macro_name != "each" {
            continue;
        }
        let Some(source) = each_source(fm) else {
            continue;
        };
        let Some(items) = statics.get(&source) else {
            continue;
        };
        let Some(item_var) = each_item(fm) else {
            continue;
        };
        // Two body grammars, one unroll: an inline `@template { … }` block, or
        // one or more named `&row($i);` invocations (BUG-348). Falling back to the
        // invocation form keeps SSG and runtime agreeing about what an `@each`
        // body may contain.
        let Some(row_html) =
            each_body_html(fm).or_else(|| each_invocations_html(fm, &item_var, templates))
        else {
            continue;
        };

        let rendered: String = items
            .iter()
            .enumerate()
            .map(|(i, item)| render_row_with(&row_html, &item_var, item, i, registry))
            .collect();
        // BUG-203: restore sentinel-protected backticks in substituted
        // values at the OUTERMOST level (recursive inner render_rows keep
        // their protection until now).
        let rendered = rendered.replace('\u{E000}', "`");

        inject_rows(&scope.selector, &rendered, blocks);
    }
}

fn each_source(fm: &FormMatch) -> Option<String> {
    match fm.get("source") {
        Some(CapturedValue::Binding(s)) => Some(s.trim_start_matches('$').to_string()),
        Some(CapturedValue::Ident(s)) => Some(s.trim_start_matches('$').to_string()),
        _ => None,
    }
}

fn each_item(fm: &FormMatch) -> Option<String> {
    match fm.get("item") {
        Some(CapturedValue::Binding(s)) => Some(s.trim_start_matches('$').to_string()),
        Some(CapturedValue::Ident(s)) => Some(s.clone()),
        _ => None,
    }
}

fn each_body_html(fm: &FormMatch) -> Option<String> {
    match fm.get("body") {
        Some(CapturedValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Resolve an `@each` body spelled as NAMED template invocations
/// (`@each($items as $i) { &row($i); }`) into the row HTML to render per item.
///
/// `@each` has two body grammars: `$body:html_block` (an inline `@template { … }`)
/// and `$invocations:template_invocation+` (one or more `&name(args);`). Only the
/// first was unrolled, so extracting a row into a named template — the refactor
/// the language teaches for reuse — silently emptied the server-rendered HTML
/// while the runtime bundle stayed complete (BUG-348).
///
/// Each invocation contributes its template's HTML with the invocation's own
/// parameter names rebound to the loop variable, so the returned string is a
/// normal row body that [`render_row_with`] fills exactly like the inline form.
/// Anything not statically resolvable (unknown template, element/slot params, an
/// arg that is not the loop variable) returns None so the caller skips unrolling
/// and the runtime stays authoritative — the same conservatism as
/// [`render_static_template`].
fn each_invocations_html(
    fm: &FormMatch,
    item_var: &str,
    templates: &HashMap<String, StaticTemplate>,
) -> Option<String> {
    use crate::syntax::CapturedValue as CV;
    let CV::Array(invocations) = fm.get("invocations")? else {
        return None;
    };
    if invocations.is_empty() {
        return None;
    }

    let mut out = String::new();
    for inv in invocations {
        let CV::Named(fields) = inv else {
            return None;
        };
        let name = match fields.get("name") {
            Some(CV::String(s) | CV::Ident(s)) => s.strip_prefix('&').unwrap_or(s).to_string(),
            _ => return None,
        };
        let tpl = templates.get(&name)?;

        // An element/slot param has no literal HTML source here, exactly as in
        // `render_static_template` — defer the whole unroll to the runtime.
        if tpl
            .params
            .iter()
            .any(|p| p.kind == crate::syntax::TemplateParamKind::Element)
        {
            return None;
        }

        let args: Vec<String> = match fields.get("args") {
            Some(CV::Array(list)) => list
                .iter()
                .map(|a| match a {
                    CV::String(s) | CV::Ident(s) | CV::Binding(s) | CV::Expr(s) => s.clone(),
                    other => format!("{other:?}"),
                })
                .collect(),
            None => Vec::new(),
            _ => return None,
        };
        if args.len() > tpl.params.len() {
            return None;
        }

        // Rebind the template's parameter holes to the caller's argument. The only
        // statically renderable argument inside a loop is the loop variable itself
        // (`&row($i)`): the row is then filled per item by `render_row_with`, which
        // already knows how to resolve `$i` / `$i.field` / `$index`.
        let mut html = tpl.html.clone();
        for (i, param) in tpl.params.iter().enumerate() {
            match args.get(i) {
                Some(arg) => {
                    let arg_name = arg.trim_start_matches('$');
                    if arg_name != item_var {
                        // A literal or an unrelated binding: `render_row_with`
                        // cannot resolve it against the item, so refuse rather
                        // than render a hole that silently disagrees with hydrate.
                        return None;
                    }
                    if param.name != arg_name {
                        html = rebind_param(&html, &param.name, arg_name);
                    }
                }
                None if param.optional => {}
                None => return None,
            }
        }
        out.push_str(&html);
    }
    Some(out)
}

/// Rewrite `` `$from` `` / `` `$from.field` `` holes to `$to`, so a template
/// parameter named differently from the loop variable still resolves. Word-bounded
/// so `$item` never matches inside `$itemCount`.
fn rebind_param(html: &str, from: &str, to: &str) -> String {
    let needle = format!("${from}");
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(pos) = rest.find(&needle) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + needle.len()..];
        let boundary = after
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != '-');
        if boundary {
            out.push('$');
            out.push_str(to);
        } else {
            out.push_str(&needle);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Render one row by substituting backtick holes `` `$item.field` `` / `` `$.field` ``
/// / `` `$index` `` and bare `$.field` tokens against `item`, AND expanding any
/// NESTED `@each(<path> as <var>) { … }` over a field of `item` (BUG-081).
/// Mirrors the runtime `each-inline` interpolation so SSG-unroll == hydrate.
/// [`render_row`] with the meta-registry available, so a filter pipe following a
/// hole (`` `$x` | uppercase ``) is EVALUATED at build time instead of surviving
/// into the served HTML as literal text (GH-11).
///
/// The evaluation runs the stdlib primitive's own `%emit js` body — the same
/// bytes the browser runs — so build output and hydrated output cannot disagree.
/// Without a registry (or for a filter that refuses: unknown, impure, or not a
/// simple transform) the pipe is left exactly as it was, and the runtime remains
/// authoritative. Degrading to the previous behavior is always safe; inventing a
/// build-time answer is not.
pub fn render_row_with(
    row_html: &str,
    item_var: &str,
    item: &Value,
    index: usize,
    registry: Option<&crate::metasystem::MetaRegistry>,
) -> String {
    let expanded = expand_nested_each(row_html, item_var, item);
    let filled = fill_holes_with(&expanded, item_var, item, index, registry);
    fill_bare(&filled, item_var, item, index)
}

pub fn render_row(row_html: &str, item_var: &str, item: &Value, index: usize) -> String {
    // Expand nested @each first: each nested region renders its inner HTML once
    // per inner item (recursively), against the inner var. The surrounding row is
    // then field-substituted against the OUTER item as before. Inner field holes
    // are resolved during the recursive render_row over the inner items, so they
    // are already concrete text by the time the outer substitution runs.
    //
    // BUG-203: substituted VALUES carry sentinel-protected backticks (\uE000)
    // so the outer pass never re-scans inner data as holes. The restore happens
    // in `render_rows` — the TOP-LEVEL caller — NOT here: inner (recursive)
    // results must stay protected until the outermost pass is done.
    let expanded = expand_nested_each(row_html, item_var, item);
    let filled = fill_holes(&expanded, item_var, item, index);
    fill_bare(&filled, item_var, item, index)
}

/// A nested `@each(<src> as <var>) { <inner> }` found inside a row body.
struct NestedEach {
    /// Byte range of the whole `@each … { … }` region in the source.
    start: usize,
    end: usize,
    /// Source path as written, e.g. `$r.tags` or `$.tags`.
    src: String,
    /// Inner loop variable (no `$`), e.g. `t`.
    var: String,
    /// Inner row HTML (between the braces).
    inner: String,
}

/// Find the FIRST top-level nested `@each(<src> as <var>) { <inner> }` in `html`.
/// Depth-aware over `{ }` so the inner body may itself contain braces / a further
/// nested `@each`. Returns None when there is no nested each.
fn find_nested_each(html: &str) -> Option<NestedEach> {
    let bytes = html.as_bytes();
    let mut search = 0;
    while let Some(rel) = html[search..].find("@each") {
        let at = search + rel;
        // Parse `@each` SP* `(` … `as` … `)` SP* `{`.
        let after = at + "@each".len();
        let rest = &html[after..];
        let Some(paren_rel) = rest.find('(') else {
            search = after;
            continue;
        };
        // Only whitespace allowed between `@each` and `(`.
        if !rest[..paren_rel].chars().all(|c| c.is_whitespace()) {
            search = after;
            continue;
        }
        let paren_open = after + paren_rel;
        // Find the matching close paren (depth-aware).
        let Some(paren_close) = match_delim(bytes, paren_open, b'(', b')') else {
            return None;
        };
        let head = &html[paren_open + 1..paren_close];
        // head is `<src> as <var>` (ignore any `, key:`/`, when` tail — nested
        // loops in a reference are the simple form).
        let head_main = head.split(',').next().unwrap_or(head);
        let mut parts = head_main.splitn(2, " as ");
        let src = parts.next().map(|s| s.trim().to_string());
        let var = parts
            .next()
            .map(|s| s.trim().trim_start_matches('$').to_string());
        let (Some(src), Some(var)) = (src, var) else {
            search = paren_close + 1;
            continue;
        };
        // After `)`, only whitespace then `{`.
        let after_paren = &html[paren_close + 1..];
        let Some(brace_rel) = after_paren.find('{') else {
            return None;
        };
        if !after_paren[..brace_rel].chars().all(|c| c.is_whitespace()) {
            search = paren_close + 1;
            continue;
        }
        let brace_open = paren_close + 1 + brace_rel;
        let Some(brace_close) = match_delim(bytes, brace_open, b'{', b'}') else {
            return None;
        };
        let inner = html[brace_open + 1..brace_close].trim().to_string();
        return Some(NestedEach {
            start: at,
            end: brace_close + 1,
            src,
            var,
            inner,
        });
    }
    None
}

/// Index of the delimiter matching the `open` byte at `from` (which must equal
/// `open_ch`), respecting nesting. Returns None if unbalanced.
fn match_delim(bytes: &[u8], from: usize, open_ch: u8, close_ch: u8) -> Option<usize> {
    debug_assert_eq!(bytes[from], open_ch);
    let mut depth = 0usize;
    let mut i = from;
    while i < bytes.len() {
        let c = bytes[i];
        if c == open_ch {
            depth += 1;
        } else if c == close_ch {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Resolve a nested-each `src` (e.g. `$r.tags`, `$.tags`) to the inner array on
/// `item`. Returns the array's elements, or empty when the path is absent / not
/// an array.
fn resolve_nested_array(src: &str, item_var: &str, item: &Value) -> Vec<Value> {
    let e = src.trim().strip_prefix('$').unwrap_or(src.trim());
    let path = if let Some(rest) = e.strip_prefix('.') {
        rest
    } else if let Some(rest) = e.strip_prefix(&format!("{}.", item_var)) {
        rest
    } else if e == item_var {
        "" // the whole item is the array
    } else {
        return Vec::new();
    };
    let mut cur = item;
    if !path.is_empty() {
        for part in path.split('.') {
            match cur.get(part) {
                Some(v) => cur = v,
                None => return Vec::new(),
            }
        }
    }
    match cur {
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    }
}

/// Replace every top-level nested `@each` region in `html` with its inner rows
/// rendered against `item`'s corresponding array. Recurses via `render_row` so
/// arbitrarily deep nesting and inner field holes both work.
fn expand_nested_each(html: &str, item_var: &str, item: &Value) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest_start = 0;
    while let Some(ne) = find_nested_each(&html[rest_start..]) {
        let abs_start = rest_start + ne.start;
        let abs_end = rest_start + ne.end;
        out.push_str(&html[rest_start..abs_start]);
        let inner_items = resolve_nested_array(&ne.src, item_var, item);
        for (i, inner_item) in inner_items.iter().enumerate() {
            out.push_str(&render_row(&ne.inner, &ne.var, inner_item, i));
        }
        rest_start = abs_end;
    }
    out.push_str(&html[rest_start..]);
    out
}

/// Replace `` `…` `` holes. Iterates by char (UTF-8 safe — never re-byte-casts).
fn fill_holes(html: &str, item_var: &str, item: &Value, index: usize) -> String {
    fill_holes_with(html, item_var, item, index, None)
}

/// `fill_holes`, plus build-time evaluation of a trailing filter pipe.
///
/// After a hole resolves to a value, a following ` | filter` run is scanned and
/// folded: the value is evaluated through the filter chain and the pipe text is
/// consumed. When anything about that is not provable — no registry, unknown
/// filter, impure body — the pipe is left in place verbatim, which is exactly
/// the pre-GH-11 behavior and lets the runtime answer.
fn fill_holes_with(
    html: &str,
    item_var: &str,
    item: &Value,
    index: usize,
    registry: Option<&crate::metasystem::MetaRegistry>,
) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('`') {
            Some(close) => {
                let inner = &after[..close];
                match resolve_token(inner, item_var, item, index) {
                    // BUG-203: substituted VALUES carry their backticks
                    // sentinel-protected so a LATER hole pass (nested @each)
                    // never re-scans data text; restored at render_row's end.
                    Some(v) => {
                        // A ` | filter` run immediately after the hole belongs to
                        // the hole, not to the page text. Fold it when we can
                        // prove the result; otherwise leave both value and pipe
                        // untouched for the runtime.
                        let (v, consumed) =
                            apply_trailing_filters(&v, &after[close + 1..], registry);
                        out.push_str(&escape_html(&v).replace('`', "\u{E000}"));
                        rest = &after[close + 1 + consumed..];
                        continue;
                    }
                    None => {
                        out.push('`');
                        out.push_str(inner);
                        out.push('`');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                // Unterminated backtick — emit the rest verbatim.
                out.push('`');
                out.push_str(after);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Scan a ` | filter | filter(arg)` run at the start of `tail` and apply it to
/// `value` at build time.
///
/// Returns the (possibly filtered) value and how many bytes of `tail` were
/// consumed. Consuming 0 means "left for the runtime" — the caller then emits
/// the pipe text verbatim, which is the honest fallback whenever the build
/// cannot prove the answer:
///
/// * no registry threaded through (a caller that never had one),
/// * the filter is not a declared `%primitive` (a real error, surfaced by the
///   normal unknown-primitive path rather than silently swallowed here),
/// * the body is impure (`Date.now`) — a build-time value would contradict the
///   browser on the next render,
/// * the JS evaluator is unavailable in this build.
///
/// The filter body executed is the stdlib primitive's own `%emit js`, so a
/// build-time answer and a runtime answer come from ONE definition.
fn apply_trailing_filters(
    value: &str,
    tail: &str,
    registry: Option<&crate::metasystem::MetaRegistry>,
) -> (String, usize) {
    let Some(registry) = registry else {
        return (value.to_string(), 0);
    };
    let Some((pipe_src, consumed)) = scan_pipe_run(tail) else {
        return (value.to_string(), 0);
    };
    let chain = crate::html::ssg_filters::parse_filter_chain(&pipe_src);
    if chain.is_empty() {
        return (value.to_string(), 0);
    }

    // Build one nested JS expression for the whole chain, innermost first, then
    // evaluate ONCE — a chain is a composition, not N round trips.
    let mut expr = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string());
    for call in &chain {
        let Some(def) = registry.get_primitive(&call.name) else {
            return (value.to_string(), 0);
        };
        match crate::html::ssg_filters::render_filter_expr(def, call, &expr) {
            Ok(next) => expr = next,
            Err(_) => return (value.to_string(), 0),
        }
    }

    match crate::html::ssg_filters::eval_js_string(&expr) {
        Some(result) => (result, consumed),
        None => (value.to_string(), 0),
    }
}

/// At the start of `tail`, scan `[ws] | [ws] filter[(args)]` repeatedly.
/// Returns the pipe source (without the leading `|`) and bytes consumed.
/// Mirrors `emit::html_reactive::scan_filter_pipe` in what counts as a pipe, so
/// the SSG path and the hydration path agree on where a filter run ends.
fn scan_pipe_run(tail: &str) -> Option<(String, usize)> {
    let bytes = tail.as_bytes();
    let mut i = 0usize;
    let mut collected = String::new();
    loop {
        let mut j = i;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        // `||` is a logical or, never a filter pipe.
        if j >= bytes.len() || bytes[j] != b'|' || bytes.get(j + 1) == Some(&b'|') {
            break;
        }
        j += 1;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        let start = j;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'-')
        {
            j += 1;
        }
        if j == start {
            break; // `|` not followed by an identifier: literal text.
        }
        let mut stage = tail[start..j].to_string();
        // Optional balanced, quote-aware argument list.
        if bytes.get(j) == Some(&b'(') {
            let mut depth = 0i32;
            let mut k = j;
            let mut quote: Option<u8> = None;
            let mut escaped = false;
            while k < bytes.len() {
                let c = bytes[k];
                if escaped {
                    escaped = false;
                } else if let Some(q) = quote {
                    if c == b'\\' {
                        escaped = true;
                    } else if c == q {
                        quote = None;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = Some(c);
                } else if c == b'(' {
                    depth += 1;
                } else if c == b')' {
                    depth -= 1;
                    if depth == 0 {
                        k += 1;
                        break;
                    }
                }
                k += 1;
            }
            if depth != 0 {
                break; // unbalanced: not a well-formed pipe.
            }
            stage.push_str(&tail[j..k]);
            j = k;
        }
        if !collected.is_empty() {
            collected.push_str(" | ");
        }
        collected.push_str(&stage);
        i = j;
    }
    if collected.is_empty() {
        None
    } else {
        Some((collected, i))
    }
}

/// Replace bare `$.field` / `$item.field` / `$index` tokens (non-hole positions).
/// Byte-indexing is safe here because the only bytes inspected (`$`, ASCII
/// alphanumerics, `_`, `.`) are single-byte in UTF-8; any multibyte char ends the
/// token scan and is copied through verbatim via the `&html[..]` slices.
fn fill_bare(html: &str, item_var: &str, item: &Value, index: usize) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    let mut copy_from = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_alphanumeric() || c == b'_' || c == b'.' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let tok = &html[start..j];
                if let Some(v) = resolve_token(&format!("${}", tok), item_var, item, index) {
                    out.push_str(&html[copy_from..i]); // flush verbatim run (UTF-8 safe)
                    // BUG-203: sentinel-protect substituted values' backticks
                    // (a parent nested-@each pass must not re-scan data text).
                    out.push_str(&escape_html(&v).replace('`', "\u{E000}"));
                    i = j;
                    copy_from = j;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&html[copy_from..]);
    out
}

/// Resolve a `$…`-shaped token to a string against the item, or None when it is
/// not an item reference (left verbatim).
fn resolve_token(raw: &str, item_var: &str, item: &Value, index: usize) -> Option<String> {
    let e = raw.trim();
    let e = e.strip_prefix('$').unwrap_or(e);
    if e == "index" {
        return Some(index.to_string());
    }
    if e == item_var {
        return Some(json_to_string(item));
    }
    let path = if let Some(rest) = e.strip_prefix('.') {
        rest // $.field
    } else if let Some(rest) = e.strip_prefix(&format!("{}.", item_var)) {
        rest // $item.field
    } else {
        return None;
    };
    let mut cur = item;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(json_to_string(cur))
}

fn json_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Escape a substituted value. Mirrors the runtime each-inline `escapeAttr`
/// (& " ' < >) so SSG-unrolled attribute holes match the hydrated DOM. In text
/// position the extra `&quot;`/`&#x27;` are DOM-equivalent to the raw chars, so
/// a single escaper is safe for both positions (unroll == hydrate).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Inject `rows` into the first EMPTY element in `blocks` whose tag carries the
/// scope selector (a `.class` or `#id`). Conservative: only an empty container
/// (`<tag …></tag>`) is filled, so a hand-authored body is never clobbered.
fn inject_rows(selector: &str, rows: &str, blocks: &mut [HtmlBlockAst]) {
    let Some(matcher) = SelectorMatch::parse(selector) else {
        return;
    };
    // SAFETY/UNROLL==HYDRATE: only inject when the selector's container token matches
    // EXACTLY ONE empty element across all blocks. The runtime each binds to the FULL
    // scope selector; if the last-token match is ambiguous (multiple same-class/id empty
    // containers, or any non-empty one), we cannot prove SSG would target the same element
    // the runtime does, so we leave ALL of them to the runtime hydrate path (rows still
    // render — just not pre-rendered into HTML). PLAN-023 W4 reviewer fix.
    let total_matches: usize = blocks
        .iter()
        .map(|b| matcher.count_matches(&b.skeleton))
        .sum();
    if total_matches != 1 {
        return;
    }
    for block in blocks.iter_mut() {
        if let Some(injected) = matcher.inject_into_empty(&block.skeleton, rows) {
            block.skeleton = injected;
            return;
        }
    }
}

/// A minimal selector matcher for `.class` / `#id` / `[attr="v"]` single-token
/// selectors.
struct SelectorMatch {
    attr: String,
    value: String,
}

impl SelectorMatch {
    fn parse(selector: &str) -> Option<Self> {
        let s = selector.trim();
        // Use the LAST simple token (e.g. ".panel .list" -> ".list").
        let last = s.split_whitespace().last().unwrap_or(s);
        if let Some(cls) = last.strip_prefix('.') {
            Some(Self {
                attr: "class".to_string(),
                value: cls.to_string(),
            })
        } else if let Some(id) = last.strip_prefix('#') {
            Some(Self {
                attr: "id".to_string(),
                value: id.to_string(),
            })
        } else {
            // `[data-lit-seg="seed-0"]` — the literate prose mount (SIP-002).
            // One attribute, one quoted value; anything richer is not a
            // container shape this matcher can prove unique.
            let inner = last.strip_prefix('[')?.strip_suffix(']')?;
            let (attr, raw) = inner.split_once('=')?;
            let value = raw.trim().trim_matches('"').trim_matches('\'');
            if attr.is_empty() || value.is_empty() {
                return None;
            }
            Some(Self {
                attr: attr.trim().to_string(),
                value: value.to_string(),
            })
        }
    }

    /// Count EMPTY elements in `html` whose `attr` carries `value` (the unroll target
    /// shape). Used to enforce a unique injection target across blocks.
    fn count_matches(&self, html: &str) -> usize {
        let mut probe = html.to_string();
        let mut n = 0;
        // Inject a unique sentinel repeatedly; each successful inject == one empty match.
        while let Some(next) = self.inject_into_empty(&probe, "\u{0}STSENTINEL\u{0}") {
            n += 1;
            probe = next;
            if n > 64 {
                break; // safety
            }
        }
        n
    }

    /// Find `<tag … attr="… value …" …></tag>` (empty) and inject rows before the
    /// close tag. Returns the modified HTML, or None if no empty match is found.
    fn inject_into_empty(&self, html: &str, rows: &str) -> Option<String> {
        let needle = format!("{}=\"", self.attr);
        let mut search = 0;
        while let Some(rel) = html[search..].find(&needle) {
            let attr_start = search + rel;
            let val_start = attr_start + needle.len();
            let Some(val_end_rel) = html[val_start..].find('"') else {
                break;
            };
            let val_end = val_start + val_end_rel;
            let attr_val = &html[val_start..val_end];
            let matches = attr_val.split_whitespace().any(|t| t == self.value);
            if matches {
                // Find the enclosing tag's `>` and the matching `</tag>` immediately after
                // (empty element). Walk back to the `<` that opens this tag.
                if let Some(open_lt) = html[..attr_start].rfind('<') {
                    let tag = read_tag_name(&html[open_lt + 1..]);
                    if let Some(gt_rel) = html[val_end..].find('>') {
                        let gt = val_end + gt_rel;
                        let close = format!("</{}>", tag);
                        // Empty container: `>` immediately followed (modulo whitespace) by </tag>
                        let after = html[gt + 1..].trim_start();
                        if after.starts_with(&close) {
                            let ws_len = html[gt + 1..].len() - after.len();
                            let insert_at = gt + 1 + ws_len;
                            let mut s = String::with_capacity(html.len() + rows.len());
                            s.push_str(&html[..insert_at]);
                            s.push_str(rows);
                            s.push_str(&html[insert_at..]);
                            return Some(s);
                        }
                    }
                }
            }
            search = val_end + 1;
        }
        None
    }
}

fn read_tag_name(s: &str) -> String {
    s.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

// =============================================================================
// SSG-render static `@doc(content:)` markdown (literate prose, `@doc(src:)`)
// =============================================================================
//
// A `.st.md` document tangles every prose segment into an empty
// `<section class="lit-prose" data-lit-seg="…">` plus a
// `[data-lit-seg="…"] { @doc(content: "…") }` scope, and `@doc(src:)` inlines
// a file's bytes into the same shape. Both used to render ONLY in the browser:
// the served HTML carried an empty section, so a crawler, a no-JS client, and
// the first paint all saw a page with no prose — for a document format whose
// whole point is the prose.
//
// The fix follows the SAME rule as the filter pipes above (GH-11): never a
// second markdown implementation in Rust. The vendored snarkdown IIFE — the
// exact bytes the browser runs — is evaluated in the build-time V8 runtime
// against the captured content, and the HTML it returns is injected into the
// empty container. The runtime's `render-markdown` primitive then re-renders
// the same string into the same element on mount: unroll == hydrate.
//
// Conservative on every edge: no engine (non-`headless` build), no blob on
// disk (embedded stdlib), an ambiguous container, or a throwing evaluation
// all leave the container empty and the runtime authoritative.

/// Render every static `@doc(content: …)` scope's markdown into its (unique,
/// empty) container in `blocks`, using the vendored engine from `registry`.
pub fn unroll_static_docs(
    ast: &StFile,
    blocks: &mut [HtmlBlockAst],
    registry: Option<&crate::metasystem::MetaRegistry>,
) {
    let Some(registry) = registry else {
        return;
    };
    let docs: Vec<(&str, String)> = ast
        .scopes
        .iter()
        .flat_map(|scope| {
            scope
                .matches
                .iter()
                .filter(|m| m.matched_macro.as_deref() == Some("doc"))
                .filter_map(|m| {
                    m.captures
                        .get("content")
                        .and_then(|v| v.as_string_literal())
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| (scope.selector.as_str(), s.to_string()))
                })
        })
        .collect();
    if docs.is_empty() {
        return;
    }
    let Some(engine) = crate::vendor::vendor_blob_source(registry, "snarkdown") else {
        return;
    };
    for (selector, content) in docs {
        let Some(html) = render_markdown_at_build(&engine, &content) else {
            continue;
        };
        inject_rows(selector, &html, blocks);
    }
}

/// Evaluate `snarkdown(content)` with the vendored IIFE in build-time V8.
/// Returns None when there is no engine or the evaluation throws.
fn render_markdown_at_build(engine: &str, content: &str) -> Option<String> {
    let expr = format!(
        "(() => {{ {engine}\n return globalThis.snarkdown({}); }})()",
        serde_json::to_string(content).ok()?
    );
    crate::html::ssg_filters::eval_js_string(&expr)
}

// =============================================================================
// FEAT-154: SSG-unroll static @template invocations
// =============================================================================
//
// Extends the `unroll == hydrate` invariant above (docs/language/data-and-rendering.md
// §6) to `@template` invocations: `&card("Title");` with LITERAL args unrolls into
// the served HTML at compile time — a crawler/no-JS client sees real markup instead
// of an empty container — mirroring exactly how a static `@each` unrolls above.
//
// v1 scope (deliberately conservative, matching the FEAT's own staticity rule):
// - every arg must be a LITERAL (string/number/bool) — a `$`-ref arg (a dynamic
//   signal read) is NOT static, so the whole invocation is left to the runtime.
// - the invoking scope's selector must match exactly ONE empty container across
//   the captured HTML blocks (same uniqueness proof `inject_rows` already enforces
//   for `@each` — ambiguous targets are never guessed at).
// - the template's body itself is HTML + `` `$param` `` holes only for v1: a param
//   whose value can't be resolved (an unknown name, or an ELEMENT param — `&slot`
//   — with no static HTML equivalent) makes the WHOLE invocation non-static, so it
//   safely falls through to the runtime-only path (v1 does not attempt slot
//   unrolling; a `body`-slot invocation and any `&param` element-typed template are
//   left dynamic).
// - nested static invocations (a static template invoking ANOTHER static template)
//   are recursively resolved through the same `render_static_template` entry point.
//
// Anything outside this v1 envelope keeps TODAY's behavior unchanged (empty
// container + runtime `invoke-template` mount) — this pass only ADDS unrolling for
// the provably-static case; it never removes or alters the dynamic path.

/// One registered template's static-render inputs: its param list (ordered,
/// matching factory positional-arg order) and its raw body HTML (with `` `$name` ``
/// holes, exactly as captured on the `@template:<name>` Construct scope).
struct StaticTemplate {
    params: Vec<crate::syntax::TemplateParamDef>,
    html: String,
    /// The template's own local `$name type: value;` state declarations — their
    /// INITIAL values render into a static unroll's holes exactly like a page-level
    /// `$signal` hole's compile-time initial (FEAT-078's existing precedent):
    /// `data-and-rendering.md` §6's `unroll == hydrate` invariant only holds if the
    /// static render shows the SAME first-paint value the runtime factory seeds.
    states: Vec<crate::syntax::ComponentStateDecl>,
}

/// Collect every registered `@template &name(...)`'s static-render inputs, keyed
/// by name (no `&`). Sourced from `ast.matches` (`macro_name == "template" |
/// "template-inline"`, `params` capture) joined with the template's Construct scope
/// html (`@template:<name>`) — the SAME two sources the runtime factory payload is
/// built from (`component_body_to_js_from_scope` + the `params` capture threaded via
/// `expand.rs`), so a static render and the runtime factory can never disagree about
/// what a template's params or body ARE, only about WHEN they're evaluated.
fn collect_static_templates(ast: &StFile) -> HashMap<String, StaticTemplate> {
    use crate::syntax::CapturedValue as CV;
    let mut out = HashMap::new();
    for fm in &ast.matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }
        let (CV::Ident(name) | CV::String(name)) =
            fm.get("name").cloned().unwrap_or(CV::String(String::new()))
        else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let name = name.strip_prefix('&').unwrap_or(&name).to_string();
        let params = match fm.get("params") {
            Some(CV::ParamList(defs)) => defs.clone(),
            _ => Vec::new(),
        };
        let Some(scope) = ast
            .scopes
            .iter()
            .find(|s| s.selector == format!("@template:{}", name))
        else {
            continue;
        };
        out.insert(
            name,
            StaticTemplate {
                params,
                html: scope.html.clone(),
                states: scope.states.clone(),
            },
        );
    }
    out
}

/// A single top-level (page-scope) template invocation: the invoking scope's
/// selector (to locate its unique empty container), the template name, and its
/// raw arg-expression strings (as captured — a literal like `"Hello"` or `42`, or
/// a `$`-prefixed dynamic reference).
struct StaticInvocation<'a> {
    scope_selector: &'a str,
    template_name: String,
    args: Vec<String>,
    /// A body-slot invocation (`&card("T") { <p>Body</p> }`) carries inline body
    /// HTML that fills the first unbound ELEMENT param. v1 treats ANY body-slot
    /// invocation as non-static (element-param unrolling is out of v1 scope — see
    /// module doc) — tracked here only so the caller can skip it explicitly rather
    /// than silently mis-rendering.
    has_body_slot: bool,
}

/// Harvest every top-level `&name(args);` / `&name(args) { body }` invocation from
/// ordinary (non-template) scopes. Mirrors `unroll_scope`'s `@each` harvest above,
/// but keyed on the `template-invoke*` macro family instead of `each`.
fn collect_static_invocations(ast: &StFile) -> Vec<StaticInvocation<'_>> {
    use crate::syntax::CapturedValue as CV;
    let mut out = Vec::new();
    for scope in &ast.scopes {
        // Only page-scope (ordinary selector) invocations are unrolled in v1 — an
        // invocation INSIDE a template body renders per-INSTANCE (it needs the
        // instance's own resolved params as inputs), which is the transitive-nesting
        // follow-up this module's doc defers, not a v1 case.
        if !matches!(scope.kind, crate::parser::ast::ScopeKind::Selector) {
            continue;
        }
        for m in &scope.matches {
            if !matches!(
                m.macro_name.as_str(),
                "template-invoke-bare" | "template-invoke" | "template-invoke-named"
            ) {
                continue;
            }
            let (CV::Ident(name) | CV::String(name)) =
                m.get("name").cloned().unwrap_or(CV::String(String::new()))
            else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let args = match m.get("args") {
                Some(CV::Array(items)) => items
                    .iter()
                    .filter_map(|v| match v {
                        CV::Expr(s) | CV::String(s) | CV::Ident(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let has_body_slot = m.get("body").is_some();
            out.push(StaticInvocation {
                scope_selector: &scope.selector,
                template_name: name.strip_prefix('&').unwrap_or(&name).to_string(),
                args,
                has_body_slot,
            });
        }
    }
    out
}

/// Is `arg` a compile-time literal (string/number/bool)? A `$`-prefixed name is a
/// dynamic signal reference — NOT static. Bare `true`/`false`/a numeric-looking
/// token/a quoted string are literals; anything else (a function call, an object,
/// an operator expression) is conservatively treated as non-static too — v1 only
/// unrolls the unambiguous literal case.
fn arg_is_static_literal(arg: &str) -> bool {
    let a = arg.trim();
    if a.starts_with('$') {
        return false;
    }
    if a.starts_with('"') && a.ends_with('"') && a.len() >= 2 {
        return true;
    }
    if a == "true" || a == "false" {
        return true;
    }
    a.parse::<f64>().is_ok()
}

/// Parse a literal arg's VALUE (unquoted string / bool / number) as a `Value`, for
/// hole substitution (mirrors how `@each`'s row rendering resolves an item field).
fn parse_static_literal(arg: &str) -> Value {
    let a = arg.trim();
    if let Some(inner) = a.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Value::String(inner.to_string());
    }
    if a == "true" {
        return Value::Bool(true);
    }
    if a == "false" {
        return Value::Bool(false);
    }
    if let Ok(n) = a.parse::<f64>() {
        return number_to_json(n);
    }
    Value::String(a.to_string())
}

/// Shared float -> JSON-Number conversion: prefers an INTEGER representation for
/// a whole value (`0.0` -> `0`), matching how the runtime factory's own JS number
/// formatting renders a whole float (`String(0)` not `String(0.0)`).
fn number_to_json(n: f64) -> Value {
    if n.fract() == 0.0 && n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        Value::Number((n as i64).into())
    } else {
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// Convert a state declaration's typed initial `CapturedValue` (Bool/Number/String
/// — the shapes `cb_state`'s reifier produces, see
/// stdlib/capture-types/component-body.st's `cb_state`) into a JSON `Value` for
/// hole substitution. An `Expr`/`Ident` initial (a non-literal expression like
/// `$other + 1`) has no static value to seed — falls through to an empty string,
/// same as an unresolved hole elsewhere in this module.
fn captured_value_to_json(v: &crate::syntax::CapturedValue) -> Value {
    use crate::syntax::CapturedValue as CV;
    match v {
        CV::Bool(b) => Value::Bool(*b),
        CV::Number(n) => number_to_json(*n),
        CV::String(s) | CV::Ident(s) => Value::String(s.clone()),
        _ => Value::String(String::new()),
    }
}

/// Render a template's HTML with its params bound to STATIC literal values.
/// Returns `None` when the invocation cannot be proven fully static: an ELEMENT
/// param (no static HTML equivalent in v1), an arg-count/param-count mismatch
/// where a REQUIRED param has no arg (v1 defers to the runtime factory's own
/// defaulting/undefined semantics rather than reimplementing them), or the
/// template is unknown.
fn render_static_template(
    templates: &HashMap<String, StaticTemplate>,
    name: &str,
    args: &[String],
) -> Option<String> {
    let tpl = templates.get(name)?;
    // Any ELEMENT param disqualifies v1 static rendering entirely for this
    // template (a `&slot` hole has no literal HTML source without a body-slot
    // arg, which v1 doesn't unroll — see StaticInvocation::has_body_slot).
    if tpl
        .params
        .iter()
        .any(|p| p.kind == crate::syntax::TemplateParamKind::Element)
    {
        return None;
    }
    if args.len() > tpl.params.len() {
        return None;
    }
    let mut bound: HashMap<String, Value> = HashMap::new();
    // Seed local state initials FIRST (a `$count number: 0;` hole renders its
    // declared initial, matching the runtime factory's own first-paint value —
    // FEAT-078's precedent for page-level `$signal` holes). Params bound below can
    // shadow a same-named state (the grammar doesn't allow a name collision in
    // practice, but params are the more specific/authoritative binding if it ever did).
    for state in &tpl.states {
        bound.insert(
            state.var_name.clone(),
            captured_value_to_json(&state.initial),
        );
    }
    for (i, param) in tpl.params.iter().enumerate() {
        match args.get(i) {
            Some(a) => {
                if !arg_is_static_literal(a) {
                    return None;
                }
                bound.insert(param.name.clone(), parse_static_literal(a));
            }
            None => {
                if !param.optional {
                    // A required param with no arg: defer to the runtime factory's
                    // own semantics rather than guessing (v1 conservatism).
                    return None;
                }
                // Optional + omitted: renders as empty, matching the documented
                // factory contract (undefined -> "" in HTML interpolation).
            }
        }
    }
    Some(fill_template_holes(&tpl.html, &bound))
}

/// Substitute `` `$param` `` holes in `html` against `bound` (param name -> static
/// value). A hole naming an unbound (omitted optional) param renders empty. A
/// nested `&other("lit");` invocation inside this HTML is NOT expanded in v1 (it
/// stays a dead literal string in served HTML — no `&name(...)` construct exists
/// in plain HTML the browser could act on); nested composition depth is explicitly
/// named as the transitive-staticity follow-up in the module doc, not a silent
/// correctness gap.
fn fill_template_holes(html: &str, bound: &HashMap<String, Value>) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('`') {
            Some(close) => {
                let inner = after[..close].trim();
                let resolved = if let Some(name) = inner.strip_prefix('$') {
                    bound.get(name).map(json_to_string)
                } else {
                    None
                };
                match resolved {
                    Some(v) => out.push_str(&escape_html(&v)),
                    None => {
                        // Unbound/unknown hole (an omitted optional param, or a
                        // non-`$param` expression v1 doesn't evaluate statically):
                        // render empty rather than leaking the raw `` `$x` `` text
                        // into served markup — matches the runtime factory's own
                        // "undefined -> empty string" contract for an omitted param.
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push('`');
                out.push_str(after);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// SSG-unroll every provably-static top-level `@template` invocation into the
/// captured HTML blocks. Mutates `blocks` in place, exactly like
/// `unroll_static_each`. Called from `src/compiler.rs` immediately alongside it.
pub fn unroll_static_invocations(ast: &StFile, blocks: &mut [HtmlBlockAst]) {
    let templates = collect_static_templates(ast);
    if templates.is_empty() {
        return;
    }
    for inv in collect_static_invocations(ast) {
        if inv.has_body_slot {
            continue; // v1: element-param/body-slot unrolling out of scope (see doc)
        }
        if !inv.args.iter().all(|a| arg_is_static_literal(a)) {
            continue; // a dynamic ($-ref) arg disqualifies this invocation
        }
        let Some(rendered) = render_static_template(&templates, &inv.template_name, &inv.args)
        else {
            continue;
        };
        // A rendered STATIC template carries a `data-st-ssg="1"` marker so the
        // runtime hydrate path can ADOPT the existing DOM instead of re-appending
        // a duplicate instance (the `unroll == hydrate` invariant — identical DOM
        // shape either way, but the marker lets the runtime skip its OWN insert).
        let marked = mark_ssg_root(&rendered);
        inject_rows(inv.scope_selector, &marked, blocks);
    }
}

/// Add a `data-st-ssg="1"` attribute to a rendered template's ROOT element (the
/// first opening tag), so the runtime `invoke-template` primitive can detect an
/// already-unrolled instance and adopt it instead of appending a duplicate.
fn mark_ssg_root(html: &str) -> String {
    let Some(lt) = html.find('<') else {
        return html.to_string();
    };
    let Some(gt_rel) = html[lt..].find('>') else {
        return html.to_string();
    };
    let gt = lt + gt_rel;
    // Self-closing / void tag (`<br/>`) — insert before the trailing `/`.
    let insert_at = if html[..gt].ends_with('/') {
        gt - 1
    } else {
        gt
    };
    let mut s = String::with_capacity(html.len() + 20);
    s.push_str(&html[..insert_at]);
    s.push_str(" data-st-ssg=\"1\"");
    s.push_str(&html[insert_at..]);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_row_fills_hole() {
        let item = json!("Alpha");
        let out = render_row("<li>`$i`</li>", "i", &item, 0);
        assert_eq!(out, "<li>Alpha</li>");
    }

    // Stage-3 Wave-6: file-sourced @data arrays are read from disk at build time.
    #[test]
    fn read_file_array_reads_json_under_base() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("data")).unwrap();
        std::fs::write(
            dir.path().join("data/items.json"),
            r#"[{"id":"a","label":"Alpha"},{"id":"b","label":"Beta"}]"#,
        )
        .unwrap();
        // site-absolute path resolves under base
        let got = read_file_array(dir.path(), "/data/items.json").expect("array");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["label"], json!("Alpha"));
    }

    #[test]
    fn read_file_array_rejects_traversal_and_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        // traversal is rejected
        assert!(read_file_array(dir.path(), "/../etc/passwd").is_none());
        assert!(read_file_array(dir.path(), "../secret.json").is_none());
        // missing file -> None (left to runtime)
        assert!(read_file_array(dir.path(), "/data/nope.json").is_none());
    }

    #[test]
    fn read_file_array_non_array_is_none() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("obj.json"), r#"{"not":"an array"}"#).unwrap();
        assert!(read_file_array(dir.path(), "/obj.json").is_none());
    }

    #[test]
    fn render_row_fills_field() {
        let item = json!({"title": "Hello"});
        let out = render_row("<li>`$i.title`</li>", "i", &item, 0);
        assert_eq!(out, "<li>Hello</li>");
    }

    #[test]
    fn render_row_expands_nested_each() {
        // BUG-081: a nested @each over a field of the outer item renders inner rows.
        let item = json!({"name": "a", "tags": [{"t": "x"}, {"t": "y"}]});
        let row = "<article><code>`$r.name`</code><span>@each($r.tags as $t) { <em>`$t.t`</em> }</span></article>";
        let out = render_row(row, "r", &item, 0);
        assert_eq!(
            out,
            "<article><code>a</code><span><em>x</em><em>y</em></span></article>"
        );
    }

    #[test]
    fn render_row_nested_each_empty_array() {
        let item = json!({"tags": []});
        let row = "<span>@each($r.tags as $t) { <em>`$t.t`</em> }</span>";
        assert_eq!(render_row(row, "r", &item, 0), "<span></span>");
    }

    #[test]
    fn render_row_two_level_nested_each() {
        let item = json!({"groups": [{"items": [{"v": "1"}, {"v": "2"}]}]});
        let row = "@each($r.groups as $g) { <ul>@each($g.items as $i) { <li>`$i.v`</li> }</ul> }";
        assert_eq!(
            render_row(row, "r", &item, 0),
            "<ul><li>1</li><li>2</li></ul>"
        );
    }

    #[test]
    fn inject_into_empty_container() {
        let m = SelectorMatch::parse(".list").unwrap();
        let html = r#"<ul class="list"></ul>"#;
        let out = m.inject_into_empty(html, "<li>X</li>").unwrap();
        assert_eq!(out, r#"<ul class="list"><li>X</li></ul>"#);
    }

    #[test]
    fn inject_skips_nonempty_container() {
        let m = SelectorMatch::parse(".list").unwrap();
        let html = r#"<ul class="list"><li>existing</li></ul>"#;
        assert!(m.inject_into_empty(html, "<li>X</li>").is_none());
    }

    /// The literate prose mount is addressed by attribute, not class: the
    /// tangler emits `[data-lit-seg="seed-0"] { @doc(content: …) }`, so the
    /// matcher must resolve that selector shape or every prose segment stays
    /// empty in the served HTML.
    #[test]
    fn inject_into_attribute_selected_container() {
        let m = SelectorMatch::parse(r#"[data-lit-seg="seed-0"]"#).unwrap();
        let html = r#"<section class="lit-prose" data-lit-seg="seed-0"></section><section class="lit-prose" data-lit-seg="seed-1"></section>"#;
        let out = m.inject_into_empty(html, "<p>P</p>").unwrap();
        assert_eq!(
            out,
            r#"<section class="lit-prose" data-lit-seg="seed-0"><p>P</p></section><section class="lit-prose" data-lit-seg="seed-1"></section>"#
        );
        assert_eq!(m.count_matches(html), 1, "exactly one segment carries that id");
        assert!(SelectorMatch::parse("[data-x]").is_none(), "a bare presence selector has no value to match");
    }

    /// The build-time markdown render runs the VENDORED engine — the same bytes
    /// the browser executes — so unroll == hydrate by construction.
    #[cfg(feature = "headless")]
    #[test]
    fn static_doc_renders_with_the_vendored_engine() {
        let engine = std::fs::read_to_string("stdlib/md/vendor/snarkdown.bundle.js")
            .expect("the vendored snarkdown blob is committed");
        let html = render_markdown_at_build(&engine, "# Title\n\nSome **prose**.").unwrap();
        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(html.contains("<strong>prose</strong>"), "{html}");
    }
}

#[cfg(test)]
mod tests_review {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_row_utf8_safe() {
        let item = json!("café ★");
        let out = render_row("<li>`$i` €</li>", "i", &item, 0);
        assert_eq!(out, "<li>café ★ €</li>");
    }

    #[test]
    fn nested_each_preserves_backticked_inner_values() {
        // BUG-203: the outer hole pass must not re-scan already-substituted
        // inner-row data (hint text carrying backticks used to be erased).
        // Production restores the sentinel at the outermost caller — same here.
        let item = json!({
            "date": "2026-06-09",
            "entries": [
                { "id": "bind-text", "docs": "became `text <- $x;` ok", "hint": "" },
                { "id": "show", "docs": "docs", "hint": "e.g. `.hidden: !$c;` — plus a `.hidden { display: none }` rule (or `visibility`)" }
            ]
        });
        let row = r#"<div>`$w.date` @each($w.entries as $e) { <p>`$e.docs` `$e.hint` <b>`$e.id`</b></p> }</div>"#;
        let out = render_row(row, "w", &item, 0).replace('\u{E000}', "`");
        assert!(out.contains("`text &lt;- $x;`"), "out: {out}");
        assert!(out.contains("`.hidden: !$c;`"), "out: {out}");
        assert!(out.contains("`.hidden { display: none }`"), "out: {out}");
        assert!(out.contains("`visibility`"), "out: {out}");
        assert!(
            !out.contains('\u{E000}'),
            "sentinel must be restored: {out}"
        );
        assert!(out.contains("<b>bind-text</b>"), "out: {out}");
        assert!(out.contains("2026-06-09"), "out: {out}");
    }

    #[test]
    fn escape_quotes_in_attr_hole() {
        let item = json!("a\"b");
        let out = render_row("<li title=\"`$i`\">x</li>", "i", &item, 0);
        assert!(out.contains("&quot;"), "got: {}", out);
        assert!(!out.contains("title=\"a\"b\""), "broken attr: {}", out);
    }

    #[test]
    fn ambiguous_container_not_injected() {
        // two empty .list containers -> ambiguous -> no injection
        let mut blocks = vec![HtmlBlockAst {
            skeleton: "<ul class=\"list\"></ul><ul class=\"list\"></ul>".into(),
            holes: vec![],
            span: Default::default(),
            injection: Default::default(),
        }];
        let m = SelectorMatch::parse(".list").unwrap();
        assert_eq!(m.count_matches(&blocks[0].skeleton), 2);
        // inject_rows requires exactly 1 -> leaves unchanged
        super::inject_rows(".list", "<li>X</li>", &mut blocks);
        assert!(
            !blocks[0].skeleton.contains("<li>X</li>"),
            "should not inject into ambiguous: {}",
            blocks[0].skeleton
        );
    }

    // === FEAT-154: static @template invocation unroll ===

    #[test]
    fn arg_is_static_literal_classifies_correctly() {
        assert!(arg_is_static_literal("\"hello\""));
        assert!(arg_is_static_literal("42"));
        assert!(arg_is_static_literal("true"));
        assert!(arg_is_static_literal("false"));
        assert!(!arg_is_static_literal("$dynamic"));
        assert!(!arg_is_static_literal("someFn()"));
    }

    #[test]
    fn number_to_json_prefers_integer_form() {
        assert_eq!(number_to_json(0.0).to_string(), "0");
        assert_eq!(number_to_json(42.0).to_string(), "42");
        assert_eq!(number_to_json(3.5).to_string(), "3.5");
    }

    #[test]
    fn fill_template_holes_substitutes_bound_params() {
        let mut bound = HashMap::new();
        bound.insert("title".to_string(), json!("Hello"));
        let out = fill_template_holes("<h3>`$title`</h3>", &bound);
        assert_eq!(out, "<h3>Hello</h3>");
    }

    #[test]
    fn fill_template_holes_renders_unbound_as_empty() {
        let bound: HashMap<String, Value> = HashMap::new();
        let out = fill_template_holes("<p>`$subtitle`</p>", &bound);
        assert_eq!(
            out, "<p></p>",
            "unbound optional param hole must render empty, not leak raw syntax"
        );
    }

    #[test]
    fn mark_ssg_root_inserts_attribute_on_first_tag() {
        let out = mark_ssg_root("<article class=\"card\"><h3>x</h3></article>");
        assert!(out.starts_with("<article class=\"card\" data-st-ssg=\"1\">"));
    }

    #[test]
    fn render_static_template_rejects_element_param() {
        let mut templates = HashMap::new();
        templates.insert(
            "card".to_string(),
            StaticTemplate {
                params: vec![crate::syntax::TemplateParamDef {
                    name: "content".to_string(),
                    kind: crate::syntax::TemplateParamKind::Element,
                    optional: false,
                    type_ref: None,
                    default: None,
                    collection: false,
                }],
                html: "<div>`&content`</div>".to_string(),
                states: vec![],
            },
        );
        assert!(render_static_template(&templates, "card", &["\"x\"".to_string()]).is_none());
    }

    #[test]
    fn render_static_template_rejects_dynamic_arg() {
        let mut templates = HashMap::new();
        templates.insert(
            "card".to_string(),
            StaticTemplate {
                params: vec![crate::syntax::TemplateParamDef {
                    name: "title".to_string(),
                    kind: crate::syntax::TemplateParamKind::Binding,
                    optional: false,
                    type_ref: None,
                    default: None,
                    collection: false,
                }],
                html: "<h3>`$title`</h3>".to_string(),
                states: vec![],
            },
        );
        assert!(render_static_template(&templates, "card", &["$dynamic".to_string()]).is_none());
    }

    #[test]
    fn render_static_template_renders_literal_arg() {
        let mut templates = HashMap::new();
        templates.insert(
            "card".to_string(),
            StaticTemplate {
                params: vec![crate::syntax::TemplateParamDef {
                    name: "title".to_string(),
                    kind: crate::syntax::TemplateParamKind::Binding,
                    optional: false,
                    type_ref: None,
                    default: None,
                    collection: false,
                }],
                html: "<h3>`$title`</h3>".to_string(),
                states: vec![],
            },
        );
        let out = render_static_template(&templates, "card", &["\"Hello\"".to_string()]);
        assert_eq!(out, Some("<h3>Hello</h3>".to_string()));
    }
}

// ── PLAN-144 W1: a component hole expands ────────────────────────────────
//
// `` `&card("Title")` `` in file-scope markup is a COMPONENT hole. It already
// parses — `parse_hole_inner` re-parses a hole as a full Spacetime expression
// and emits an ELEMENT_REF — but nothing downstream expanded it: the hole
// reached `emit_text_hole`, which treats every hole as a SIGNAL expression,
// evaluated it to nothing, and emitted `<span data-st-hole="N"></span>`.
// Build green, `Pages: 1`, zero output.
//
// The fix keeps ONE meaning for the hole form: interpolate the thing named
// here. A value hole interpolates a value; a component hole interpolates a
// component. It gains no second role — which is why this is an expansion pass
// and not a new surface.
//
// Expansion happens in the SKELETON, before `emit_html_blocks` turns holes
// into spans, and reuses `render_static_template` — the same renderer the
// selector-scoped `&name(…);` invocation path uses. One expansion path, two
// spellings.

/// Split `&name(arg, arg)` into its template name and raw arg sources.
/// Returns `None` for anything that is not a call — a BARE `&name` is a scope
/// reference (a score subject like `&showcase-hero-mtn__title for 35%`), never
/// an invocation, so it must not be touched (PLAN-144 Q2).
pub(crate) fn parse_component_hole(src: &str) -> Option<(String, Vec<String>)> {
    let s = src.trim();
    let rest = s.strip_prefix('&')?;
    let open = rest.find('(')?;
    let name = rest[..open].trim();
    if name.is_empty() || !rest.trim_end().ends_with(')') {
        return None;
    }
    // A name must be a plain identifier; anything else is not a component call.
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let inner = &rest.trim_end()[open + 1..rest.trim_end().len() - 1];
    Some((name.to_string(), split_top_level_args(inner)))
}

/// Split an argument list on top-level commas (respecting quotes and nesting),
/// so `&card("a, b", 2)` yields two args rather than three.
fn split_top_level_args(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut cur = String::new();
    let mut prev_escape = false;
    for c in inner.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                if c == q && !prev_escape {
                    quote = None;
                }
                prev_escape = c == '\\' && !prev_escape;
                continue;
            }
            None => {}
        }
        match c {
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
            ',' if depth == 0 => {
                let t = cur.trim();
                if !t.is_empty() {
                    out.push(t.to_string());
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
        prev_escape = false;
    }
    let t = cur.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
    out
}

/// Expand every COMPONENT hole in each block's skeleton into the template's
/// rendered HTML, in place.
///
/// A hole whose source is not a component call is left exactly as it was, so
/// value holes keep their existing reactive path untouched. A component call
/// naming a template that cannot be statically rendered (a dynamic `$arg`, an
/// element param) is also left alone — it falls through to the existing
/// diagnostic rather than being silently dropped here.
pub fn expand_component_holes(ast: &StFile, blocks: &mut [HtmlBlockAst]) {
    let templates = collect_static_templates(ast);
    if templates.is_empty() {
        return;
    }
    for block in blocks.iter_mut() {
        for (idx, hole) in block.holes.clone().iter().enumerate() {
            let Some((name, args)) = parse_component_hole(hole) else {
                continue;
            };
            if !templates.contains_key(&name) {
                continue; // unknown name — refused upstream, never silently dropped
            }
            if !args.iter().all(|a| arg_is_static_literal(a)) {
                continue;
            }
            let Some(rendered) = render_static_template(&templates, &name, &args) else {
                continue;
            };
            let sentinel = format!("\u{E000}{idx}\u{E001}");
            if block.skeleton.contains(&sentinel) {
                block.skeleton = block.skeleton.replace(&sentinel, &mark_ssg_root(&rendered));
                // The hole is consumed; blank its source so the emitter cannot
                // also render it as an (empty) value hole.
                block.holes[idx] = String::new();
            }
        }
    }
}
