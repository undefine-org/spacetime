//! Reactive HtmlExpr → DOM builder emitter (FEAT-077, completes PLAN-023 W3/W4).
//!
//! The static [`super::html`] emitter turns an `HtmlExpr` into an HTML *string* with
//! `<!--st-hole-->` placeholders (SSG). This module instead lowers an `HtmlExpr` into a
//! JavaScript **DOM-builder function** that constructs the live element tree at runtime and
//! wires every interpolation `Hole` reactively to the signal store (`ST.get/watch`).
//!
//! It is the ONE reactive renderer for BOTH:
//!   - `@template` bodies (scope = [`SignalScope::Element`]; holes read `ST.get(__el, …)`)
//!   - file-scope HTML (scope = [`SignalScope::Global`]; holes read `SpacetimeLocal[…]`)
//!
//! The emitted builder has the shape (proven by the FEAT-077 spike):
//! ```js
//! (function (__el) {
//!   const n0 = document.createElement('div');
//!   n0.setAttribute('class', 'card');           // static attr
//!   const n1 = document.createElement('span');
//!   const applyN1class = () => { n1.setAttribute('class', ("v-" + (ST.get(__el,'open')))); };
//!   ST.watch(__el, 'open', applyN1class); applyN1class();   // reactive attr
//!   const t0 = document.createTextNode('');
//!   const applyT0 = () => { const __v = (ST.get(__el,'count')); t0.textContent = __v == null ? '' : String(__v); };
//!   ST.watch(__el, 'count', applyT0); applyT0();            // reactive text hole
//!   n1.appendChild(t0); n0.appendChild(n1);
//!   return n0;
//! })
//! ```
//! For [`SignalScope::Global`] holes, deps subscribe via `document.addEventListener(
//! 'local:<dep>:updated', render)` (mirroring `emit_reactive_binding_js`) instead of
//! `ST.watch(__el, …)`, and the scope element parameter is `document`.

use crate::ir::{AttrPart, HtmlExpr, JsExpr};
use crate::syntax::cst::{HOLE_CLOSE, HOLE_OPEN};
use crate::syntax::{SignalScope, collect_signal_deps, transpile_signal_expr_with};

/// Convert a legacy component-body HTML string (with bare `$x` / `$x.path` interpolation in
/// text and attribute positions, e.g. `<div data-n="$count">$title</div>`) into a list of
/// `HtmlExpr` by NORMALIZING bare `$x` into hole sentinels and parsing via html5ever
/// (`parse_html_skeleton`). This is the B-Wave 2 bridge that lets the legacy string form feed
/// the reactive renderer: authoring stays `$x` (no backticks), but lowers to reactive holes.
///
/// `element_params` is the set of DECLARED element-param names (the `&name` params in the
/// template signature). Only an `&name` whose `name` is in this set is normalized into an
/// element-substitution hole; every other `&` in text (prose like `AT&T`, entities `&amp;`)
/// is left untouched. Pass an empty slice when there are no element params.
pub fn component_html_to_exprs(html: &str) -> Vec<HtmlExpr> {
    let (skeleton, holes) = normalize_bare_holes(html);
    let exprs = crate::html::treesink::parse_html_skeleton(&skeleton, &holes);
    fold_markdown_nodes(exprs)
}

/// Fold `<st-md>` sentinel elements into `HtmlExpr::Markdown`, recursively.
///
/// A template body carries a reactive-markdown node as `<st-md>` `$expr` `</st-md>`
/// — the ONE way to reach the markdown rail through the HTML-string path
/// (`component_html_to_exprs`) that template bodies take, since treesink parses
/// plain HTML and knows no markdown. The element's single text-hole child is the
/// expression whose value snarkdown renders. This runs AFTER treesink so the
/// parser stays markdown-unaware (one HTML parser, one concern); the fold is a
/// small structural rewrite over its output, the same shape `Html`/`Raw` already
/// take as post-parse IR nodes.
///
/// An `<st-md>` with no hole child (or a non-hole child) is left as a plain
/// element rather than guessed at — a malformed sentinel renders visibly wrong
/// rather than silently empty.
fn fold_markdown_nodes(exprs: Vec<HtmlExpr>) -> Vec<HtmlExpr> {
    exprs.into_iter().map(fold_one).collect()
}

fn fold_one(expr: HtmlExpr) -> HtmlExpr {
    match expr {
        HtmlExpr::Element {
            tag,
            attrs,
            children,
            line,
        } if tag == "st-md" => {
            // The expression is the element's single text-position hole.
            let src = children.iter().find_map(|c| match c {
                HtmlExpr::Hole(JsExpr::Raw(s)) => Some(s.clone()),
                _ => None,
            });
            match src {
                Some(s) => HtmlExpr::Markdown(s),
                // Malformed sentinel: keep it as an element so the mistake shows.
                None => HtmlExpr::Element {
                    tag,
                    attrs,
                    children: fold_markdown_nodes(children),
                    line,
                },
            }
        }
        HtmlExpr::Element {
            tag,
            attrs,
            children,
            line,
        } => HtmlExpr::Element {
            tag,
            attrs,
            children: fold_markdown_nodes(children),
            line,
        },
        other => other,
    }
}

/// Scan an HTML string and replace every backtick hole `` `expr` `` with a
/// `\u{E000}<idx>\u{E001}` hole sentinel, returning the skeleton + ordered hole sources.
/// Tag structure is respected: a hole is normalized in BOTH text content and
/// attribute-value position (the two places the factory interpolates), but NOT inside a
/// tag name or an attribute name. Quoted attribute strings are handled so
/// `data-n="`$count`"` normalizes the value. FUP-041: backtick is the ONE hole form;
/// bare `$x`/`&param` are literal text.
fn normalize_bare_holes(html: &str) -> (String, Vec<String>) {
    let bytes = html.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n + 16);
    let mut holes: Vec<String> = Vec::new();
    let mut i = 0;
    let mut in_tag = false; // between `<` and `>`
    let mut in_str: Option<u8> = None; // inside a quoted attr value

    while i < n {
        let c = bytes[i];
        // Track tag / string context so we don't treat `$` in a tag NAME position oddly,
        // and so attribute-value strings are still normalized.
        if in_tag {
            match in_str {
                Some(q) => {
                    if c == q {
                        in_str = None;
                    }
                }
                None => match c {
                    b'"' | b'\'' => in_str = Some(c),
                    b'>' => in_tag = false,
                    _ => {}
                },
            }
        } else if c == b'<' {
            in_tag = true;
        }

        // A `$` begins a bare interpolation when followed by an identifier start, in text
        // position OR inside a quoted attribute value (not in a bare tag region).
        let normalizable = !in_tag || in_str.is_some();
        // `$$` is a LITERAL dollar — the one escape the emit tokenizer (BUG-112)
        // and the runtime interpolator (public/runtime/templates.js) both honor,
        // so the builder path must collapse it too or the two render paths
        // diverge. Without it a template that DISPLAYS st source as text
        // (`.clock { text <- $now; }`) loses `$now` to interpolation instead.
        if c == b'$' && i + 1 < n && bytes[i + 1] == b'$' && normalizable {
            out.push('$');
            i += 2;
            continue;
        }
        // Escaped backtick `` \` `` is literal text, not a hole delimiter (docs/language
        // §2). Consume the backslash, emit the backtick, and advance past both.
        if c == b'\\' && i + 1 < n && bytes[i + 1] == b'`' && normalizable {
            out.push('`');
            i += 2;
            continue;
        }
        // Backtick hole — THE one interpolation form (FUP-041). `` `expr` `` in text or
        // a quoted attribute value lowers to a hole sentinel; the inner is a full
        // expression source, INCLUDING the `&name` element-substitution form
        // (`` `&sel` `` flattens a node/sequence at this position). Bare `$x`/`&param`
        // recognition was REMOVED in FUP-041 — backtick is the single, unambiguous
        // hole delimiter, so `$5` / `AT&T` / `&amp;` in prose are always literal text.
        if c == b'`'
            && normalizable
            && let Some((inner, next)) = scan_backtick_hole(bytes, i)
        {
            // Filter pipe (BUG-188): in TEXT position, a ` | filter(...)` run
            // immediately following the hole is the template/@each filter form
            // (the SAME pipe the `text <-` binding path honors). Fold it into
            // the hole source so it applies to the rendered value via ST.filter
            // instead of leaking as a literal ` | currency("$")` text node.
            // Attribute values keep `|` literal (attr filter pipes are out of
            // scope; the injection path owns attr filtering).
            let (inner, next) = if !in_tag && in_str.is_none() {
                match scan_filter_pipe(bytes, next) {
                    Some((pipe, after)) => (format!("{} | {}", inner, pipe), after),
                    None => (inner, next),
                }
            } else {
                (inner, next)
            };
            let idx = holes.len();
            holes.push(inner);
            out.push(HOLE_OPEN);
            out.push_str(&idx.to_string());
            out.push(HOLE_CLOSE);
            i = next;
            continue;
        }
        // ASCII byte: push directly. A non-ASCII byte (>= 0x80) is the lead of a
        // multibyte UTF-8 sequence — `c as char` would reinterpret each byte as a
        // separate code point and corrupt it (e.g. ‘↑’ e2 86 91 → \u00e2\u0086\u0091
        // mojibake, BUG-079). Copy the WHOLE char from the original valid-UTF-8
        // string and advance past all its bytes.
        if c < 0x80 {
            out.push(c as char);
            i += 1;
        } else {
            let ch = html[i..].chars().next().expect("valid UTF-8");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    (out, holes)
}

/// At `bytes[start]`, try to scan a filter pipe run: whitespace, `|`, whitespace,
/// then a filter call — an identifier (`[A-Za-z_][A-Za-z0-9_-]*`) with an optional
/// balanced-paren argument list (quote-aware, so `if("a)b", "c")` scans whole).
/// Returns the trimmed pipe source (e.g. `currency("$")`) and the index just past
/// it. Returns `None` when no well-formed pipe follows (the `|` then stays literal
/// text, e.g. prose like "yes | no").
fn scan_filter_pipe(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let n = bytes.len();
    let mut j = start;
    while j < n && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j >= n || bytes[j] != b'|' {
        return None;
    }
    j += 1;
    while j < n && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    let name_start = j;
    if j >= n || !(bytes[j].is_ascii_alphabetic() || bytes[j] == b'_') {
        return None;
    }
    while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'-') {
        j += 1;
    }
    let mut end = j;
    // Optional argument list: balanced parens, skipping quoted runs so a `)`
    // inside a string arg doesn't close early.
    if j < n && bytes[j] == b'(' {
        let mut depth = 0i32;
        let mut k = j;
        while k < n {
            match bytes[k] {
                b'\'' | b'"' => {
                    let q = bytes[k];
                    k += 1;
                    while k < n && bytes[k] != q {
                        if bytes[k] == b'\\' {
                            k += 1;
                        }
                        k += 1;
                    }
                }
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = k + 1;
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }
        if depth != 0 {
            return None; // unbalanced parens — not a well-formed filter call
        }
    }
    let pipe = std::str::from_utf8(&bytes[name_start..end])
        .ok()?
        .trim()
        .to_string();
    if pipe.is_empty() {
        return None;
    }
    Some((pipe, end))
}

/// At a backtick (`bytes[start] == b'`'`), scan a `` `…` `` hole. Returns the INNER
/// expression source (backticks stripped, trimmed) and the index past the closing
/// backtick. An escaped `` \` `` inside the run contributes a literal backtick to the
/// inner and does NOT close the hole. Returns `None` if the run is never closed (a lone
/// backtick is then treated as literal text by the caller).
fn scan_backtick_hole(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let n = bytes.len();
    let mut j = start + 1;
    let mut inner = String::new();
    while j < n {
        if bytes[j] == b'\\' && j + 1 < n && bytes[j + 1] == b'`' {
            inner.push('`');
            j += 2;
            continue;
        }
        if bytes[j] == b'`' {
            // Closing backtick: hole complete.
            return Some((inner.trim().to_string(), j + 1));
        }
        // Copy one byte (inner is expression source; non-ASCII is rare here but
        // preserved verbatim via the original UTF-8 bytes).
        if bytes[j] < 0x80 {
            inner.push(bytes[j] as char);
            j += 1;
        } else {
            // Decode the full char from the original slice to keep UTF-8 intact.
            let s = std::str::from_utf8(&bytes[j..]).ok()?;
            let ch = s.chars().next()?;
            inner.push(ch);
            j += ch.len_utf8();
        }
    }
    None
}

/// Emit a reactive DOM-builder function expression for a list of top-level HtmlExprs.
///
/// Returns a JS expression `(function (__el) { …; return <fragment-or-node>; })`. When the
/// list has a single root the builder returns that node; otherwise it returns a
/// `DocumentFragment` holding all roots. `scope` selects how holes read signals.

/// Scan an `HtmlExpr` tree for `@`-prefixed ATTRIBUTE names — the totality-of-lowering
/// guard (BUG-121). `@` is THE directive sigil (AGENTS harmony rule: one sigil, one
/// meaning); a directive is selector-scoped (`.foo { @mcp-action(...) }`) and never an
/// element attribute. So an `@`-named attribute reaching the DOM emitter is, by definition,
/// a MISPLACED directive or a typo — there is no legitimate `@`-named HTML attribute. Left
/// alone it would lower to a dead `setAttribute("@mcp-action", "approve")` that nothing
/// listens to (the silent no-op BUG-121 fixes). This walker collects every such name so the
/// caller can REFUSE with a diagnostic instead of emitting the dead attribute (fidelity
/// ladder: refuse, don't fake-green). Returns the offending `(tag, attr_name)` pairs in
/// document order; an empty vec means the tree is clean.
pub fn find_directive_attrs(exprs: &[HtmlExpr]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for expr in exprs {
        collect_directive_attrs(expr, &mut found);
    }
    found
}

fn collect_directive_attrs(expr: &HtmlExpr, found: &mut Vec<(String, String)>) {
    if let HtmlExpr::Element {
        tag,
        attrs,
        children,
        ..
    } = expr
    {
        for (name, _parts) in attrs {
            if name.starts_with('@') {
                found.push((tag.clone(), name.clone()));
            }
        }
        for child in children {
            collect_directive_attrs(child, found);
        }
    }
}
pub fn emit_builder(exprs: &[HtmlExpr], scope: SignalScope) -> String {
    emit_builder_opts(exprs, scope, false, None)
}

/// Like [`emit_builder`] but the single root element IS the signal scope: holes wire to the
/// root the builder creates (not the passed `__el`). Used for `@template` bodies, where the
/// root element owns the instance signals (params + state) that its own holes read. Falls
/// back to the passed `__el` for multi-root bodies (no single root to scope to).
pub fn emit_builder_root_scoped(exprs: &[HtmlExpr], scope: SignalScope) -> String {
    emit_builder_opts(exprs, scope, true, None)
}

/// Like [`emit_builder_root_scoped`], but STAMPS `data-st-node` on every guest
/// element under `base` (PLAN-064 B3.2). Used ONLY by the MCP region-bundle path
/// (`src/mcp/bundle.rs`) so a composed workbench guest's DOM elements carry the
/// SAME dotted ids the navigator shows — the address that makes a canvas click
/// and a tree click resolve to one node. `base` is the template's structure id
/// (the entry template is `"0"`); its top-level element is stamped `"<base>.0"`.
/// Normal Spacetime pages never call this, so their DOM stays stamp-free.
pub fn emit_builder_root_scoped_stamped(
    exprs: &[HtmlExpr],
    scope: SignalScope,
    base: &str,
) -> String {
    emit_builder_opts(exprs, scope, true, Some(base.to_string()))
}

fn emit_builder_opts(
    exprs: &[HtmlExpr],
    scope: SignalScope,
    root_is_scope: bool,
    stamp_base: Option<String>,
) -> String {
    let mut b = match stamp_base {
        Some(base) => Builder::new_stamped(scope, base),
        None => Builder::new(scope),
    };
    b.line("(function (__el) {");
    b.indent();
    if root_is_scope {
        // Single-root bodies: defer hole wiring until after the root exists, then wire holes
        // to the root itself. We achieve this by emitting the root first, reassigning __el to
        // it, and only THEN running the deferred hole-apply subscriptions. Implemented by
        // building into a buffer where __el is rebound; simplest correct form: build the tree
        // with a placeholder scope variable that we point at the root post-creation.
        //
        // Concretely: emit the root element creation, set `__el = <root>` when there's a
        // single root, so every subsequent ST.watch(__el, …) binds to the root.
        let mut roots = Vec::new();
        // Pre-scan: if exactly one top-level Element, emit it but capture its var and rebind.
        let single_root_element = exprs.len() == 1 && matches!(exprs[0], HtmlExpr::Element { .. });
        if single_root_element {
            // Emit the root element's createElement + static attrs FIRST, rebind __el, then
            // its reactive attrs + children (whose holes now wire to the root).
            if let HtmlExpr::Element {
                tag,
                attrs,
                children,
                ..
            } = &exprs[0]
            {
                // B3.2: the single root element is the FIRST structural slot under
                // `base` -> `base.0` (matching the shared allocator + navigator).
                let root_id = b.stamp_base.clone().map(|base| format!("{base}.0"));
                let var =
                    b.emit_element_root_scoped_with_id(tag, attrs, children, root_id.as_deref());
                roots.push(var);
            }
        } else {
            // Multi-root (or non-element root) body: wrap in a `display:contents` scope element
            // so there IS a single root to own the instance signals (params/state) the holes
            // read. `display:contents` makes the wrapper invisible in layout — children render
            // as if direct. Rebind __el to it BEFORE emitting children so holes wire to it.
            let wrapper = b.fresh("n");
            b.line(&format!(
                "const {} = document.createElement('div');",
                wrapper
            ));
            b.line(&format!("{}.style.display = 'contents';", wrapper));
            b.line(&format!("__el = {};", wrapper));
            // FUP-094: alias __node to the wrapper scope root (see single-root path).
            if matches!(scope, SignalScope::Scoped) {
                b.line(&format!("var __node = {};", wrapper));
            }
            // B3.2: the synthetic `display:contents` wrapper is NOT stamped (it has
            // no navigator node); its children are the TOP-LEVEL template-body list,
            // so a bare hole here BURNS an index (matching html_walk's top-level
            // slots) via `emit_children_toplevel`. Top-level guest elements stamp
            // `base.<idx>`.
            b.emit_children_toplevel(&wrapper, exprs, b.stamp_base.clone().as_deref());
            roots.push(wrapper);
        }
        match roots.len() {
            0 => b.line("return null;"),
            _ => b.line(&format!("return {};", roots[0])),
        }
    } else {
        let mut roots = Vec::new();
        for expr in exprs {
            if let Some(var) = b.emit_node(expr) {
                roots.push(var);
            }
        }
        match roots.len() {
            0 => b.line("return null;"),
            1 => b.line(&format!("return {};", roots[0])),
            _ => {
                b.line("const __frag = document.createDocumentFragment();");
                for r in &roots {
                    b.line(&format!("__frag.appendChild({});", r));
                }
                b.line("return __frag;");
            }
        }
    }
    b.dedent();
    b.line("})");
    b.out
}

/// Internal codegen state: an indented JS line buffer + a fresh-name counter + the signal
/// scope for hole lowering.
struct Builder {
    out: String,
    depth: usize,
    counter: usize,
    scope: SignalScope,
    /// BUG-082: >0 while emitting inside an <svg> subtree — elements there must be
    /// created with createElementNS (the SVG namespace) or the browser never paints
    /// them. <foreignObject> children revert to 0 (HTML namespace).
    svg_depth: usize,
    /// PLAN-064 B3.2: when `Some`, the builder STAMPS `data-st-node="<dotted-id>"`
    /// on each real element, using the SHARED structural-id allocator
    /// (`crate::introspect::html_ids`) so the stamps match the navigator's node
    /// ids EXACTLY. `None` (the default, all normal pages) stamps nothing — this
    /// is workbench-guest-only, gated at the `src/mcp/bundle.rs` call site.
    stamp_base: Option<String>,
}

impl Builder {
    fn new(scope: SignalScope) -> Self {
        Builder {
            out: String::new(),
            depth: 0,
            counter: 0,
            scope,
            svg_depth: 0,
            stamp_base: None,
        }
    }

    /// A Builder that stamps `data-st-node` under the given base id (B3.2).
    fn new_stamped(scope: SignalScope, base: String) -> Self {
        Builder {
            stamp_base: Some(base),
            ..Builder::new(scope)
        }
    }

    /// Emit the `data-st-node` stamp for an element var, if this builder stamps and
    /// an id is known for the element. Called immediately after createElement.
    fn stamp(&mut self, var: &str, node_id: Option<&str>) {
        if self.stamp_base.is_some()
            && let Some(id) = node_id
        {
            self.line(&format!(
                "{}.setAttribute('data-st-node', {});",
                var,
                js_string(id)
            ));
        }
    }

    fn indent(&mut self) {
        self.depth += 1;
    }
    fn dedent(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
    fn line(&mut self, s: &str) {
        // Single-line emission: the builder is embedded as a value in the register-template
        // payload object literal, which is spliced into a `%emit js` block; multi-line values
        // would be newline-escaped into a string. Emit compact, space-separated statements so
        // the builder stays a real (single-line) function expression. Indentation is dropped.
        if !self.out.is_empty() && !self.out.ends_with(' ') {
            self.out.push(' ');
        }
        self.out.push_str(s.trim());
    }
    fn fresh(&mut self, prefix: &str) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("__{}{}", prefix, n)
    }

    /// Emit code that builds one HtmlExpr; return the JS variable name bound to its node,
    /// or None when the expr produces nothing (e.g. an empty raw).
    fn emit_node(&mut self, expr: &HtmlExpr) -> Option<String> {
        match expr {
            HtmlExpr::Element {
                tag,
                attrs,
                children,
                ..
            } => Some(self.emit_element(tag, attrs, children)),
            HtmlExpr::Text(text) => {
                let var = self.fresh("t");
                self.line(&format!(
                    "const {} = document.createTextNode({});",
                    var,
                    js_string(text)
                ));
                Some(var)
            }
            HtmlExpr::Hole(JsExpr::Raw(src)) => {
                // An empty/whitespace hole (e.g. an empty backtick `` `` ``) has no
                // expression to evaluate; emit a plain empty text node rather than
                // `const __v = ();` (a SyntaxError that would invalidate the whole builder).
                if src.trim().is_empty() {
                    let var = self.fresh("t");
                    self.line(&format!("const {} = document.createTextNode('');", var));
                    return Some(var);
                }
                // `&param` element substitution: a trusted-element insertion hole.
                // Already flattens a multi-node sequence (an HTML-string value is parsed
                // into a fragment and ALL its childNodes inserted), so an @mark/@block
                // body wraps a whole selection with `` `&sel` `` — no spread needed.
                if let Some(param) = src.trim().strip_prefix('&') {
                    return Some(self.emit_element_hole(param));
                }
                Some(self.emit_text_hole(src))
            }
            HtmlExpr::Hole(other) => {
                // Already-lowered exprs are not expected on the reactive path yet; emit a
                // best-effort static text node carrying the debug form (never silently drop).
                let var = self.fresh("t");
                self.line(&format!(
                    "const {} = document.createTextNode({});",
                    var,
                    js_string(&format!("{:?}", other))
                ));
                Some(var)
            }
            HtmlExpr::Html(s) => {
                let exprs = component_html_to_exprs(s);
                let nodes = exprs
                    .iter()
                    .filter_map(|expr| self.emit_node(expr))
                    .collect::<Vec<_>>();
                match nodes.len() {
                    0 => None,
                    1 => nodes.into_iter().next(),
                    _ => {
                        let fragment = self.fresh("html");
                        self.line(&format!(
                            "const {} = document.createDocumentFragment();",
                            fragment
                        ));
                        for node in nodes {
                            self.line(&format!("{}.appendChild({});", fragment, node));
                        }
                        Some(fragment)
                    }
                }
            }
            HtmlExpr::Markdown(src) => Some(self.emit_markdown_hole(src)),
            HtmlExpr::Raw(s) => {
                if s.trim().is_empty() {
                    return None;
                }
                // Trusted raw HTML (e.g. &param element substitution): build via a template
                // and return its FULL content fragment so multi-node raw
                // (`<a></a><b></b>`) keeps every sibling (firstChild-only would drop them).
                // In SVG context (BUG-082) wrap-parse in <svg> so the parser assigns the
                // SVG namespace, then lift the children into a fragment.
                let var = self.fresh("raw");
                self.line(&format!(
                    "const {} = document.createElement('template');",
                    var
                ));
                let node = self.fresh("rawnode");
                if self.svg_depth > 0 {
                    self.line(&format!(
                        "{}.innerHTML = '<svg>' + {} + '</svg>';",
                        var,
                        js_string(s)
                    ));
                    self.line(&format!(
                        "const {} = document.createDocumentFragment();",
                        node
                    ));
                    let wrap = self.fresh("svgwrap");
                    self.line(&format!(
                        "const {} = {}.content.firstElementChild;",
                        wrap, var
                    ));
                    self.line(&format!(
                        "while ({w} && {w}.firstChild) {n}.appendChild({w}.firstChild);",
                        w = wrap,
                        n = node
                    ));
                } else {
                    self.line(&format!("{}.innerHTML = {};", var, js_string(s)));
                    self.line(&format!("const {} = {}.content;", node, var));
                }
                Some(node)
            }
        }
    }

    /// Build an element: createElement / createElementNS (SVG context, BUG-082),
    /// wire attrs (static or reactive), recurse children.
    fn emit_element(
        &mut self,
        tag: &str,
        attrs: &[(String, Vec<AttrPart>)],
        children: &[HtmlExpr],
    ) -> String {
        self.emit_element_with_id(tag, attrs, children, None)
    }

    /// Like [`emit_element`], but `node_id` (when this is a stamping builder) is the
    /// dotted structure id to stamp on THIS element; its structural children get
    /// their ids from the SHARED allocator (`crate::introspect::html_ids`) so the
    /// stamps match the navigator. `node_id` is `None` on the unstamped path (every
    /// normal page), making this byte-identical to the pre-B3.2 emit.
    fn emit_element_with_id(
        &mut self,
        tag: &str,
        attrs: &[(String, Vec<AttrPart>)],
        children: &[HtmlExpr],
        node_id: Option<&str>,
    ) -> String {
        let var = self.fresh("n");
        self.line(&self.create_element_line(&var, tag));
        self.stamp(&var, node_id);
        let child_svg = self.child_svg_depth(tag);
        for (name, parts) in attrs {
            self.emit_attr(&var, name, parts);
        }
        let saved = self.svg_depth;
        self.svg_depth = child_svg;
        self.emit_children(&var, children, node_id);
        self.svg_depth = saved;
        var
    }

    /// Append an element's children (unstamped path — every normal page). Exactly
    /// the old `for child { emit_node; appendChild }` loop, byte-identical.
    fn emit_children(&mut self, parent_var: &str, children: &[HtmlExpr], parent_id: Option<&str>) {
        // Element children ABSORB their direct text-position holes (the IR does this
        // via `recurse_children` before recursing), so a hole does NOT burn a child
        // index. `emit_element_*` calls this; the top-level template-body list uses
        // `emit_children_toplevel` (holes counted).
        self.emit_children_indexed(parent_var, children, parent_id, true);
    }

    /// Append the TOP-LEVEL template-body children. Unlike an element's children, a
    /// bare hole here is a first-class structure node (it burns an index + gets a
    /// navigator Hole node), matching `html_walk`'s top-level `structural_slots`.
    fn emit_children_toplevel(
        &mut self,
        parent_var: &str,
        children: &[HtmlExpr],
        parent_id: Option<&str>,
    ) {
        self.emit_children_indexed(parent_var, children, parent_id, false);
    }

    /// The shared child-emit loop. Always emits every child's DOM node in source
    /// order (unstamped path is byte-identical to the legacy loop). When stamping,
    /// allocates a dotted id per STRUCTURAL child and stamps ELEMENTS with it; the
    /// id counter mirrors `html_walk` EXACTLY:
    ///   - an Element always burns an index (and is stamped);
    ///   - a Hole burns an index ONLY when `!absorb_holes` (top-level bare holes are
    ///     structure nodes; an element's direct holes are absorbed → no index);
    ///   - Text / Raw never burn an index.
    /// A Hole/Text/Raw still emits its DOM node, just without a `data-st-node`.
    fn emit_children_indexed(
        &mut self,
        parent_var: &str,
        children: &[HtmlExpr],
        parent_id: Option<&str>,
        absorb_holes: bool,
    ) {
        let stamping = self.stamp_base.is_some();
        let mut idx = 0usize;
        for child in children {
            // Does this child occupy a structural index? (Mirrors html_walk.)
            let id_bearing = match child {
                HtmlExpr::Element { .. } => true,
                HtmlExpr::Hole(_) => !absorb_holes,
                HtmlExpr::Text(_)
                | HtmlExpr::Raw(_)
                | HtmlExpr::Html(_)
                | HtmlExpr::Markdown(_) => false,
            };
            let child_id = if stamping && id_bearing {
                Some(match parent_id {
                    Some(pid) => format!("{pid}.{idx}"),
                    None => idx.to_string(),
                })
            } else {
                None
            };
            if id_bearing {
                idx += 1;
            }
            // Only an Element receives the stamp; a bare hole burns its index (so a
            // following element gets the right one) but emits a plain text node.
            let emitted = match child {
                HtmlExpr::Element {
                    tag,
                    attrs,
                    children,
                    ..
                } => Some(self.emit_element_with_id(tag, attrs, children, child_id.as_deref())),
                other => self.emit_node(other),
            };
            if let Some(child_var) = emitted {
                self.line(&format!("{}.appendChild({});", parent_var, child_var));
            }
        }
    }

    /// The createElement(NS) statement for a tag in the CURRENT namespace context:
    /// inside an <svg> subtree, for <svg> itself, OR for an unambiguously-SVG tag
    /// (`path`/`circle`/`g`/… — see `is_svg_tag`) even at depth 0 → createElementNS.
    /// The depth-0 SVG-tag case matters for STANDALONE @template bodies whose root
    /// is an SVG element (e.g. a graph-node `&g(…)` template): the compiler can't
    /// see the eventual <svg> mount parent, but the tag name alone disambiguates.
    /// SVG elements created in the HTML namespace are HTMLUnknownElements that
    /// never paint (BUG-082).
    fn create_element_line(&self, var: &str, tag: &str) -> String {
        if self.svg_depth > 0 || tag.eq_ignore_ascii_case("svg") || is_svg_tag(tag) {
            format!(
                "const {} = document.createElementNS('http://www.w3.org/2000/svg', {});",
                var,
                js_string(tag)
            )
        } else {
            format!(
                "const {} = document.createElement({});",
                var,
                js_string(tag)
            )
        }
    }

    /// The svg_depth the CHILDREN of `tag` should see: entering <svg> OR any
    /// unambiguously-SVG container tag increments (so its children inherit the SVG
    /// namespace even in a standalone template body); <foreignObject> children
    /// revert to the HTML namespace (0); otherwise inherit the current depth.
    fn child_svg_depth(&self, tag: &str) -> usize {
        if tag.eq_ignore_ascii_case("foreignObject") {
            0
        } else if tag.eq_ignore_ascii_case("svg") || is_svg_tag(tag) {
            self.svg_depth + 1
        } else {
            self.svg_depth
        }
    }

    /// Root-scoped element emit that also STAMPS (B3.2): `node_id` is the root
    /// element's dotted id; children ids flow from the shared allocator.
    fn emit_element_root_scoped_with_id(
        &mut self,
        tag: &str,
        attrs: &[(String, Vec<AttrPart>)],
        children: &[HtmlExpr],
        node_id: Option<&str>,
    ) -> String {
        let var = self.fresh("n");
        self.line(&self.create_element_line(&var, tag));
        self.stamp(&var, node_id);
        let child_svg = self.child_svg_depth(tag);
        // Rebind the signal scope to the root element: subsequent ST.watch(__el, …) and
        // ST.get(__el, …) in this body now read the root's own instance signals.
        self.line(&format!("__el = {};", var));
        // FUP-094: alias __node to the scope root so Scoped-scope holes (which the
        // shared transpile emits as `ST.resolve(__node, …)` and watches via
        // ST.watchScoped) resolve against THIS instance root — lexical resolution
        // (instance → ancestors → global) for outer signals in a template hole.
        if matches!(self.scope, SignalScope::Scoped) {
            self.line(&format!("var __node = {};", var));
        }
        for (name, parts) in attrs {
            self.emit_attr(&var, name, parts);
        }
        let saved = self.svg_depth;
        self.svg_depth = child_svg;
        self.emit_children(&var, children, node_id);
        self.svg_depth = saved;
        var
    }

    /// Wire an attribute. All-literal → a single setAttribute. Any hole → a reactive
    /// apply()+watch arc recomputing the concatenated value string.
    fn emit_attr(&mut self, el_var: &str, name: &str, parts: &[AttrPart]) {
        let has_hole = parts.iter().any(|p| matches!(p, AttrPart::Hole(_)));
        if !has_hole {
            // Static: concatenate literal parts.
            let lit: String = parts
                .iter()
                .map(|p| match p {
                    AttrPart::Lit(s) => s.clone(),
                    AttrPart::Hole(_) => String::new(),
                })
                .collect();
            self.line(&format!(
                "{}.setAttribute({}, {});",
                el_var,
                js_string(name),
                js_string(&lit)
            ));
            return;
        }
        // Reactive: build the value expression `("lit" + (expr) + "lit" + …)` and collect deps.
        let (value_expr, deps) = self.attr_value_expr(parts);
        let apply = self.fresh("applyAttr");
        self.line(&format!("const {} = () => {{", apply));
        self.indent();
        // try/catch: a throwing hole (null dotted intermediate) degrades to removing the
        // attribute rather than aborting the builder (parity with factory + reactive-binding).
        self.line("let __v;");
        self.line(&format!(
            "try {{ __v = ({}); }} catch (e) {{ __v = null; }}",
            value_expr
        ));
        self.line(&format!(
            "if (__v === undefined || __v === null || __v === false) {{ {}.removeAttribute({}); }}",
            el_var,
            js_string(name)
        ));
        self.line(&format!(
            "else {{ {}.setAttribute({}, __v === true ? '' : String(__v)); }}",
            el_var,
            js_string(name)
        ));
        self.dedent();
        self.line("};");
        self.emit_subscribe(&apply, &deps);
    }

    /// Build a reactive text node for a text-position hole.
    fn emit_text_hole(&mut self, src: &str) -> String {
        let var = self.fresh("t");
        self.line(&format!("const {} = document.createTextNode('');", var));
        // Filter pipe (BUG-188): `expr | filter(...)` — the SAME split the file-scope
        // reactive-binding emitter (emit_reactive_binding_js) and the reify injection
        // path do. Deps + transpile see only the value expression; ST.filter resolves
        // the named filter against ST.filters at render time (unknown names no-op).
        // Depth- and string-aware, and never splits `||` (BUG-262). A naive
        // `split_once('|')` here emitted `__v = (($a ` for `text <- ($a || $b)`.
        let (value_src, filter) = crate::syntax::split_filter_pipe(src);
        let expr = transpile_signal_expr_with(value_src, self.scope);
        let deps = collect_signal_deps(value_src);
        let apply = self.fresh("applyText");
        self.line(&format!("const {} = () => {{", apply));
        self.indent();
        // try/catch degrades to '' so a null intermediate in a dotted path
        // (`$item.user.name` when item/user is null) does NOT throw and abort the
        // whole builder — parity with the factory's per-step null-guard and with
        // emit_reactive_binding_js's try/catch.
        self.line("let __v;");
        self.line(&format!(
            "try {{ __v = ({}); }} catch (e) {{ __v = ''; }}",
            expr
        ));
        if let Some(filter) = filter {
            self.line(&format!(
                "if (typeof ST !== 'undefined' && ST.filter) __v = ST.filter({}, __v);",
                js_string(filter)
            ));
        }
        self.line(&format!(
            "{}.textContent = (__v === undefined || __v === null) ? '' : String(__v);",
            var
        ));
        self.dedent();
        self.line("};");
        self.emit_subscribe(&apply, &deps);
        var
    }

    /// Build a reactive MARKDOWN node: the dual of `emit_text_hole`. Where a text
    /// hole writes `textContent = String(v)`, this renders `v` as Markdown into a
    /// container's `innerHTML` via the vendored `snarkdown` global, re-rendering
    /// whenever a dependency signal changes.
    ///
    /// The container is a `display:contents` span so it adds no box of its own —
    /// the rendered markdown flows exactly where the node sits (a chat bubble, a
    /// doc section). `display:contents` because snarkdown emits block-level HTML
    /// (`<p>`, `<ul>`) that must not be trapped inside an inline wrapper.
    ///
    /// Dependency collection and transpilation are shared with the text rail
    /// (`collect_signal_deps` / `transpile_signal_expr_with`), so a markdown node
    /// tracks `$m.text` exactly as a text hole would — one signal semantics, two
    /// sinks. The `snarkdown(...)` reference here is what the demand-driven vendor
    /// pass (`vendor::inject_vendor_preludes`) keys on to inject the engine, so a
    /// page with no markdown node ships none of snarkdown's bytes.
    ///
    /// TRUST: `innerHTML` is a trusted sink, and this is deliberate — the author
    /// asked to render Markdown, and snarkdown is the rendering (and escaping)
    /// engine. It is the same trust `emit_element_hole` and `Raw` already carry.
    /// The `try/catch` degrades a null/throwing expression to empty rather than
    /// aborting the whole builder, matching the text rail.
    fn emit_markdown_hole(&mut self, src: &str) -> String {
        let var = self.fresh("md");
        self.line(&format!(
            "const {} = document.createElement('span');",
            var
        ));
        self.line(&format!("{}.style.display = 'contents';", var));
        let expr = transpile_signal_expr_with(src, self.scope);
        let deps = collect_signal_deps(src);
        let apply = self.fresh("applyMd");
        self.line(&format!("const {} = () => {{", apply));
        self.indent();
        self.line("let __v;");
        self.line(&format!(
            "try {{ __v = ({}); }} catch (e) {{ __v = ''; }}",
            expr
        ));
        self.line(&format!(
            "const __md = (__v === undefined || __v === null) ? '' : String(__v);"
        ));
        self.line(&format!(
            "{}.innerHTML = (typeof snarkdown !== 'undefined') ? snarkdown(__md) : __md;",
            var
        ));
        self.dedent();
        self.line("};");
        self.emit_subscribe(&apply, &deps);
        var
    }

    /// Build a reactive element-substitution hole for `&param`. The param value (seeded as a
    /// signal on the scope element) is inserted as TRUSTED content: a string becomes parsed
    /// HTML, a Node is inserted directly. A stable comment anchor marks the insertion point so
    /// re-renders replace prior content. Returns the anchor node var (appended by parent).
    fn emit_element_hole(&mut self, param: &str) -> String {
        let anchor = self.fresh("anchor");
        self.line(&format!(
            "const {} = document.createComment({});",
            anchor,
            js_string(&format!("&{}", param))
        ));
        let inserted = self.fresh("ins");
        self.line(&format!("let {} = [];", inserted));
        let apply = self.fresh("applyEl");
        self.line(&format!("const {} = () => {{", apply));
        self.indent();
        self.line(&format!("{}.forEach(nd => nd.remove());", inserted));
        self.line(&format!("{} = [];", inserted));
        let read = match self.scope {
            SignalScope::Element => format!("ST.get(__el, {})", js_string(param)),
            SignalScope::Global => format!("SpacetimeLocal[{}]", js_string(param)),
            // Builder runs with `__el` = the instance root; lexical resolution from there.
            SignalScope::Scoped => format!("ST.resolve(__el, {})", js_string(param)),
        };
        self.line(&format!(
            "let __c; try {{ __c = ({}); }} catch (e) {{ __c = null; }}",
            read
        ));
        self.line(&format!(
            "if (__c == null || !{}.parentNode) {{ return; }}",
            anchor
        ));
        self.line("if (__c instanceof Node) {");
        self.indent();
        self.line(&format!(
            "{}.parentNode.insertBefore(__c, {}); {}.push(__c);",
            anchor, anchor, inserted
        ));
        self.dedent();
        self.line("} else {");
        self.indent();
        self.line(
            "const __tpl = document.createElement('template'); __tpl.innerHTML = String(__c);",
        );
        self.line(&format!(
            "Array.from(__tpl.content.childNodes).forEach(nd => {{ {}.parentNode.insertBefore(nd, {}); {}.push(nd); }});",
            anchor, anchor, inserted
        ));
        self.dedent();
        self.line("}");
        self.dedent();
        self.line("};");
        self.emit_subscribe(&apply, &[param.to_string()]);
        anchor
    }

    /// Build the `("lit" + (expr) + …)` value expression for an attribute part-list, plus
    /// the ordered, de-duplicated dependency list across all holes.
    fn attr_value_expr(&self, parts: &[AttrPart]) -> (String, Vec<String>) {
        let mut pieces: Vec<String> = Vec::new();
        let mut deps: Vec<String> = Vec::new();
        for part in parts {
            match part {
                AttrPart::Lit(s) => pieces.push(js_string(s)),
                AttrPart::Hole(JsExpr::Raw(src)) => {
                    pieces.push(format!("({})", transpile_signal_expr_with(src, self.scope)));
                    for d in collect_signal_deps(src) {
                        if !deps.contains(&d) {
                            deps.push(d);
                        }
                    }
                }
                AttrPart::Hole(other) => pieces.push(format!("({:?})", other)),
            }
        }
        let expr = if pieces.is_empty() {
            "''".to_string()
        } else {
            pieces.join(" + ")
        };
        (expr, deps)
    }

    /// Emit the dependency subscription + initial apply for a reactive `apply` fn, per scope.
    /// Element scope: `ST.watch(__el, dep, apply)` (ST.watch fires immediately if defined).
    /// Global scope: `document.addEventListener('local:<dep>:updated', apply)` + explicit apply.
    fn emit_subscribe(&mut self, apply: &str, deps: &[String]) {
        match self.scope {
            SignalScope::Element | SignalScope::Scoped => {
                let scoped = matches!(self.scope, SignalScope::Scoped);
                for dep in deps {
                    // Element: watch the instance root. Scoped: watch across the instance
                    // scope chain (ST.watchScoped) so an ancestor-owned signal also drives.
                    if scoped {
                        self.line(&format!(
                            "if (ST.watchScoped) ST.watchScoped(__el, {}, {});",
                            js_string(dep),
                            apply
                        ));
                    } else {
                        self.line(&format!("ST.watch(__el, {}, {});", js_string(dep), apply));
                    }
                }
                // ST.watch fires immediately when the signal already has a value; call once
                // more to cover the undefined-initial case (idempotent).
                self.line(&format!("{}();", apply));
            }
            SignalScope::Global => {
                for dep in deps {
                    // Bridge element-scoped primitive exports (a signal's
                    // `%yield pending`) into the body-scope plane this
                    // listener reads (BUG-069 rail; missed here until
                    // BUG-197) -- a no-op for deps already global.
                    self.line(&format!(
                        "if (typeof ST !== 'undefined' && ST._bridgeExport) ST._bridgeExport({});",
                        js_string(dep)
                    ));
                    self.line(&format!(
                        "document.addEventListener({}, {});",
                        js_string(&format!("local:{}:updated", dep)),
                        apply
                    ));
                }
                self.line(&format!("{}();", apply));
            }
        }
    }
}

/// JSON-encode a string as a JS string literal (double-quoted, escaped). Reuses serde_json
/// for correctness (handles quotes, backslashes, control chars, unicode).
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Whether `tag` is an UNAMBIGUOUSLY-SVG element name — one that only exists in
/// the SVG namespace, so the reactive builder must `createElementNS` it even at
/// svg_depth 0 (a standalone @template body whose mount parent the compiler
/// can't see). HTML-ambiguous names that exist in BOTH namespaces (`a`, `title`,
/// `style`, `script`) are deliberately EXCLUDED — they default to HTML and only
/// become SVG via an actual <svg> ancestor (the svg_depth path handles those).
/// (BUG-082 follow-up: declarative SVG graph templates.)
fn is_svg_tag(tag: &str) -> bool {
    const SVG_TAGS: &[&str] = &[
        "g",
        "path",
        "circle",
        "ellipse",
        "line",
        "polyline",
        "polygon",
        "rect",
        "text",
        "tspan",
        "textPath",
        "defs",
        "use",
        "symbol",
        "marker",
        "mask",
        "pattern",
        "clipPath",
        "linearGradient",
        "radialGradient",
        "stop",
        "foreignObject",
        "image",
        "switch",
        "desc",
        "metadata",
        "filter",
        "feGaussianBlur",
        "feOffset",
        "feBlend",
        "feColorMatrix",
        "feComposite",
        "feFlood",
        "feImage",
        "feMerge",
        "feMergeNode",
        "feMorphology",
        "feTile",
        "feTurbulence",
        "feDropShadow",
    ];
    SVG_TAGS.iter().any(|t| tag.eq_ignore_ascii_case(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{AttrPart, HtmlExpr, JsExpr};

    fn hole(src: &str) -> JsExpr {
        JsExpr::Raw(src.to_string())
    }

    #[test]
    fn stamped_builder_ids_match_structure_ir() {
        // PLAN-064 B3.2 (the selection-spine invariant): the `data-st-node` ids the
        // stamped builder writes onto guest elements MUST equal the ids the shared
        // Structure IR (`structure_from_html`) assigns — so a canvas click and a
        // navigator click resolve to ONE node. Compare them element-for-element.
        let html =
            "<section class=\"hero\"><h1>`$t`</h1><p>x</p><span>`$a` and `$b`</span></section>";
        let exprs = component_html_to_exprs(html);
        // Stamped builder under base "0" (the entry template's structure id).
        let js = emit_builder_root_scoped_stamped(&exprs, SignalScope::Element, "0");
        // The IR ids: structure_from_html walks the SAME html under a base id "0".
        // (structure_from_html seeds at IdPath::root() rendering "", so its top
        // element is "0"; to compare against base "0" we prefix.) Simplest: assert
        // the builder stamped EXACTLY the element ids the IR produced, by extracting
        // every `data-st-node` literal from the emitted JS.
        let mut stamped: Vec<String> = Vec::new();
        let mut rest = js.as_str();
        while let Some(i) = rest.find("data-st-node', \"") {
            let after = &rest[i + "data-st-node', \"".len()..];
            if let Some(end) = after.find('"') {
                stamped.push(after[..end].to_string());
                rest = &after[end..];
            } else {
                break;
            }
        }
        // Expected element ids under base "0": section=0.0, h1=0.0.0, p=0.0.1,
        // span=0.0.2. (Holes/text burn no element stamp; the span's two holes are
        // absorbed as bindings, not stamped.)
        assert_eq!(
            stamped,
            vec!["0.0", "0.0.0", "0.0.1", "0.0.2"],
            "stamped ids must be the dotted element paths: {js}"
        );
    }

    #[test]
    fn stamped_builder_absorbs_direct_hole_before_element() {
        // Reviewer B3 P1: a direct text-hole child of an element must be ABSORBED
        // (no index burned), so a FOLLOWING element gets the id the IR assigns. For
        // `<div>`$x`<section></section></div>`: div=0.0, then inside div the hole is
        // absorbed and section is the FIRST structural child -> 0.0.0 (NOT 0.0.1).
        let html = "<div>`$x`<section></section></div>";
        let exprs = component_html_to_exprs(html);
        let js = emit_builder_root_scoped_stamped(&exprs, SignalScope::Element, "0");
        assert!(
            js.contains("data-st-node', \"0.0\""),
            "div stamped 0.0: {js}"
        );
        assert!(
            js.contains("data-st-node', \"0.0.0\""),
            "section (direct hole absorbed) stamped 0.0.0, not 0.0.1: {js}"
        );
        assert!(
            !js.contains("data-st-node', \"0.0.1\""),
            "no phantom 0.0.1 from the absorbed hole: {js}"
        );
    }

    #[test]
    fn stamped_builder_ids_match_ir_for_mixed_children() {
        // Reviewer B3 P1 parity: for markup mixing text, direct holes, and multiple
        // elements, the stamped ids must EQUAL the shared IR element ids exactly.
        let html = "<div>text`$x`<p></p><span>`$a` `$b`</span></div>";
        let exprs = component_html_to_exprs(html);
        let js = emit_builder_root_scoped_stamped(&exprs, SignalScope::Element, "0");
        // IR (structure_from_html under root->"0" base via structure_from_html_nodes):
        // div=0.0, p=0.0.0, span=0.0.1 (text + direct hole absorbed, burn no index).
        let ir = crate::introspect::structure_from_html_nodes(html);
        // The IR nodes are relative to root ""; prefix the base "0" the builder used.
        for n in ir
            .iter()
            .filter(|n| n.kind == crate::introspect::NodeKind::Element)
        {
            let expected = format!("data-st-node', \"0.{}\"", n.id);
            assert!(
                js.contains(&expected),
                "builder stamps IR element id 0.{} : {js}",
                n.id
            );
        }
    }

    #[test]
    fn unstamped_builder_emits_no_data_st_node() {
        // The DEFAULT (unstamped) path — every normal Spacetime page — must NEVER
        // emit `data-st-node` (no attribute noise on real sites).
        let html = "<section class=\"hero\"><h1>`$t`</h1></section>";
        let exprs = component_html_to_exprs(html);
        let js = emit_builder_root_scoped(&exprs, SignalScope::Element);
        assert!(
            !js.contains("data-st-node"),
            "unstamped builder must not stamp: {js}"
        );
    }

    #[test]
    fn stamped_builder_toplevel_bare_hole_burns_index() {
        // A bare hole at the TOP LEVEL of a template body (a sibling of the root,
        // not a direct child of an element) IS a structure node and burns an index,
        // so a following top-level element is `base.1` not `base.0`. (Contrast with
        // a direct element-child hole, which is absorbed — see the absorb test.)
        // Body: `` `$x` `` then `<section>` — two top-level siblings; multi-root path.
        let html = "`$x`<section></section>";
        let exprs = component_html_to_exprs(html);
        let js = emit_builder_root_scoped_stamped(&exprs, SignalScope::Element, "0");
        // Top-level: bare hole burns index 0, section = base.1 = 0.1.
        assert!(
            js.contains("data-st-node', \"0.1\""),
            "top-level section after a bare hole is 0.1: {js}"
        );
    }

    #[test]
    fn svg_subtree_uses_createelementns() {
        // BUG-082: elements inside <svg> must be created in the SVG namespace
        // (createElementNS) or the browser never paints them. A sibling <div>
        // stays on plain createElement, and <foreignObject> children revert to HTML.
        let tree = vec![
            HtmlExpr::Element {
                tag: "svg".to_string(),
                attrs: vec![(
                    "viewBox".to_string(),
                    vec![AttrPart::Lit("0 0 10 10".to_string())],
                )],
                children: vec![
                    HtmlExpr::Element {
                        tag: "circle".to_string(),
                        attrs: vec![("cx".to_string(), vec![AttrPart::Lit("5".to_string())])],
                        children: vec![],
                        line: None,
                    },
                    HtmlExpr::Element {
                        tag: "foreignObject".to_string(),
                        attrs: vec![],
                        children: vec![HtmlExpr::Element {
                            tag: "p".to_string(),
                            attrs: vec![],
                            children: vec![],
                            line: None,
                        }],
                        line: None,
                    },
                ],
                line: None,
            },
            HtmlExpr::Element {
                tag: "div".to_string(),
                attrs: vec![],
                children: vec![],
                line: None,
            },
        ];
        let js = emit_builder(&tree, SignalScope::Element);
        let svg_ns = "http://www.w3.org/2000/svg";
        assert!(
            js.contains(&format!("document.createElementNS('{svg_ns}', \"svg\")")),
            "<svg> itself is namespaced: {js}"
        );
        assert!(
            js.contains(&format!("document.createElementNS('{svg_ns}', \"circle\")")),
            "<circle> inherits the SVG namespace: {js}"
        );
        assert!(
            js.contains(&format!(
                "document.createElementNS('{svg_ns}', \"foreignObject\")"
            )),
            "<foreignObject> is an SVG element: {js}"
        );
        assert!(
            js.contains("document.createElement(\"p\")"),
            "<p> inside foreignObject reverts to the HTML namespace: {js}"
        );
        assert!(
            js.contains("document.createElement(\"div\")"),
            "the sibling <div> outside <svg> stays plain createElement: {js}"
        );
    }

    #[test]
    fn standalone_svg_template_root_is_namespaced() {
        // BUG-082 follow-up: a standalone @template body whose ROOT is an
        // unambiguously-SVG tag (a graph-node `<g>…` template, mounted later into
        // an <svg>) must createElementNS even at svg_depth 0 — the compiler can't
        // see the mount parent, but `g`/`circle`/`text`/`path` only exist in SVG.
        let tree = vec![HtmlExpr::Element {
            tag: "g".to_string(),
            attrs: vec![],
            children: vec![
                HtmlExpr::Element {
                    tag: "circle".to_string(),
                    attrs: vec![("cx".to_string(), vec![AttrPart::Lit("5".to_string())])],
                    children: vec![],
                    line: None,
                },
                HtmlExpr::Element {
                    tag: "text".to_string(),
                    attrs: vec![],
                    children: vec![],
                    line: None,
                },
            ],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        let svg_ns = "http://www.w3.org/2000/svg";
        assert!(
            js.contains(&format!("document.createElementNS('{svg_ns}', \"g\")")),
            "standalone <g> root is SVG-namespaced: {js}"
        );
        assert!(
            js.contains(&format!("document.createElementNS('{svg_ns}', \"circle\")")),
            "its <circle> child inherits the SVG namespace: {js}"
        );
        assert!(
            js.contains(&format!("document.createElementNS('{svg_ns}', \"text\")")),
            "its <text> child is SVG-namespaced: {js}"
        );
        // A standalone <div> root is still plain HTML.
        let html = emit_builder(
            &[HtmlExpr::Element {
                tag: "div".to_string(),
                attrs: vec![],
                children: vec![],
                line: None,
            }],
            SignalScope::Element,
        );
        assert!(
            html.contains("document.createElement(\"div\")"),
            "a plain HTML root stays createElement: {html}"
        );
    }

    #[test]
    fn normalize_preserves_multibyte_utf8() {
        // BUG-079: the hole-normalizer scanned bytes but rebuilt the skeleton with
        // `c as char`, splitting a multibyte UTF-8 char (e.g. ‘↑’ e2 86 91) into three
        // bogus code points (mojibake). The skeleton must preserve the char verbatim.
        let (skeleton, holes) = normalize_bare_holes("<span>\u{2191}\u{22ee}\u{00b7} `$x`</span>");
        assert!(
            skeleton.contains('\u{2191}'),
            "up-arrow preserved: {skeleton:?}"
        );
        assert!(
            skeleton.contains('\u{22ee}'),
            "vellip preserved: {skeleton:?}"
        );
        assert!(
            skeleton.contains('\u{00b7}'),
            "middot preserved: {skeleton:?}"
        );
        assert_eq!(
            holes,
            vec!["$x".to_string()],
            "the `$x` backtick hole normalizes"
        );
        // And the emitted builder carries the char into createTextNode verbatim.
        let tree = vec![HtmlExpr::Element {
            tag: "span".to_string(),
            attrs: vec![],
            children: vec![HtmlExpr::Text("\u{2191}ok".to_string())],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        assert!(
            js.contains('\u{2191}'),
            "emitted JS keeps the multibyte char: {js}"
        );
        assert!(
            !js.contains("\\u0086"),
            "no broken byte-escape in the emit: {js}"
        );
    }

    #[test]
    fn emits_static_element() {
        let tree = vec![HtmlExpr::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), vec![AttrPart::Lit("card".to_string())])],
            children: vec![HtmlExpr::Text("hi".to_string())],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        assert!(js.contains("document.createElement(\"div\")"));
        assert!(js.contains("setAttribute(\"class\", \"card\")"));
        assert!(js.contains("createTextNode(\"hi\")"));
        assert!(js.starts_with("(function (__el) {"));
        assert!(js.trim_end().ends_with("})"));
    }

    #[test]
    fn emits_reactive_text_hole_element_scope() {
        let tree = vec![HtmlExpr::Element {
            tag: "span".to_string(),
            attrs: vec![],
            children: vec![HtmlExpr::Hole(hole("$count"))],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        // text hole lowers to ST.get(__el,'count') + ST.watch on dep 'count'
        assert!(js.contains("ST.get(__el, 'count')"), "js: {js}");
        assert!(js.contains("ST.watch(__el, \"count\""), "js: {js}");
        assert!(js.contains(".textContent ="), "js: {js}");
    }

    #[test]
    fn emits_reactive_attr_hole_element_scope() {
        let tree = vec![HtmlExpr::Element {
            tag: "span".to_string(),
            attrs: vec![(
                "class".to_string(),
                vec![
                    AttrPart::Lit("v-".to_string()),
                    AttrPart::Hole(hole("$open")),
                ],
            )],
            children: vec![],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        assert!(js.contains("\"v-\" + (ST.get(__el, 'open'))"), "js: {js}");
        assert!(js.contains("ST.watch(__el, \"open\""), "js: {js}");
        assert!(js.contains("setAttribute"), "js: {js}");
    }

    #[test]
    fn empty_hole_emits_valid_text_node_not_invalid_js() {
        // R-wave P2-a: an empty hole must NOT produce `const __v = ();`.
        let tree = vec![HtmlExpr::Element {
            tag: "span".to_string(),
            attrs: vec![],
            children: vec![HtmlExpr::Hole(hole(""))],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        assert!(
            !js.contains("= ();"),
            "empty hole must not emit invalid JS: {js}"
        );
        assert!(
            js.contains("createTextNode('')"),
            "empty hole -> empty text node: {js}"
        );
    }

    #[test]
    fn dotted_hole_apply_is_try_wrapped() {
        // R-wave P2-c: dotted-path holes must be null-safe (try/catch), not throw.
        let tree = vec![HtmlExpr::Element {
            tag: "span".to_string(),
            attrs: vec![],
            children: vec![HtmlExpr::Hole(hole("$item.user.name"))],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Element);
        assert!(
            js.contains("try {") && js.contains("catch"),
            "dotted hole apply must be try-wrapped: {js}"
        );
        // dep is the base name only
        assert!(
            js.contains("ST.watch(__el, \"item\""),
            "dep is base name 'item': {js}"
        );
    }

    #[test]
    fn multinode_raw_keeps_all_siblings() {
        // R-wave P2-b: Raw must use the full content fragment, not firstChild.
        let tree = vec![HtmlExpr::Raw("<a></a><b></b>".to_string())];
        let js = emit_builder(&tree, SignalScope::Element);
        assert!(
            js.contains(".content;"),
            "raw must take full content fragment: {js}"
        );
        assert!(
            !js.contains(".content.firstChild"),
            "raw must not take firstChild only: {js}"
        );
    }

    #[test]
    fn normalizes_bare_text_and_attr_holes() {
        // The backtick hole form `<div data-n="`$count`">`$title`</div>` normalizes in
        // BOTH attr and text position into reactive holes (FUP-041: backtick is THE form).
        let exprs = component_html_to_exprs("<div data-n=\"`$count`\"><h2>`$title`</h2></div>");
        // Lower to a builder and assert both holes wired reactively.
        let js = emit_builder(&exprs, SignalScope::Element);
        assert!(
            js.contains("ST.get(__el, 'count')"),
            "attr hole $count wired: {js}"
        );
        assert!(
            js.contains("ST.get(__el, 'title')"),
            "text hole $title wired: {js}"
        );
        assert!(
            js.contains("setAttribute(\"data-n\""),
            "data-n attr present: {js}"
        );
    }

    #[test]
    fn normalizes_dotted_bare_hole() {
        let exprs = component_html_to_exprs("<span>`$item.title`</span>");
        let js = emit_builder(&exprs, SignalScope::Element);
        // dotted path lowered; dep is base name 'item'
        assert!(
            js.contains("ST.get(__el, 'item').title"),
            "dotted lowered: {js}"
        );
        assert!(
            js.contains("ST.watch(__el, \"item\""),
            "dep is base 'item': {js}"
        );
    }

    #[test]
    fn root_scoped_rebinds_el_to_root() {
        // Root-scoped builder rebinds __el to the root element so holes wire to it.
        let body = component_html_to_exprs("<div class=\"card\"><span>`$count`</span></div>");
        let js = emit_builder_root_scoped(&body, SignalScope::Element);
        assert!(js.contains("__el = "), "root-scoped must rebind __el: {js}");
        // The rebind must occur BEFORE the child hole's ST.watch.
        let rebind = js.find("__el = ").unwrap();
        let watch = js.find("ST.watch(__el").unwrap();
        assert!(rebind < watch, "__el rebind must precede hole watch: {js}");
    }

    #[test]
    fn amp_param_element_substitution_lowers() {
        // `&param` in text position lowers to an element-substitution hole (insertBefore +
        // trusted-HTML/Node), reactive on the param signal. FUP-041: written as the
        // backtick element-hole `` `&content` `` (bare `&content` is now literal text).
        let exprs = component_html_to_exprs("<div class=\"wrap\">`&content`</div>");
        let js = emit_builder_root_scoped(&exprs, SignalScope::Element);
        assert!(
            js.contains("createComment"),
            "&param uses a comment anchor: {js}"
        );
        assert!(js.contains("insertBefore"), "&param inserts content: {js}");
        assert!(
            js.contains("ST.watch(__el, \"content\""),
            "&param reactive on 'content': {js}"
        );
        assert!(
            !js.contains("createTextNode(\"&content\")"),
            "&content must not be literal text: {js}"
        );
    }

    #[test]
    fn amp_prose_not_treated_as_param() {
        // R-wave P1: literal `&word` prose (AT&T, R&D) must NOT be normalized when `word`
        // is not a declared element param. No element params here -> all & is literal.
        let exprs = component_html_to_exprs("<span>AT&T and R&D win</span>");
        let js = emit_builder_root_scoped(&exprs, SignalScope::Element);
        assert!(
            !js.contains("createComment"),
            "prose &word must not become a param hole: {js}"
        );
        // The text content must preserve the full prose (html5ever keeps &T/&D as text).
        assert!(
            js.contains("AT&T") || js.contains("AT&amp;T") || js.contains("T and R"),
            "prose preserved: {js}"
        );
    }

    #[test]
    fn amp_entity_not_treated_as_param() {
        // `&amp;` is an HTML entity, NOT a param — left as literal text.
        let exprs = component_html_to_exprs("<span>a &amp; b</span>");
        let js = emit_builder_root_scoped(&exprs, SignalScope::Element);
        assert!(
            !js.contains("createComment"),
            "&amp; entity must not become a param hole: {js}"
        );
    }

    #[test]
    fn literal_dollar_not_normalized() {
        // A `$` not followed by an identifier is left as literal text.
        let exprs = component_html_to_exprs("<span>price: $5</span>");
        let js = emit_builder(&exprs, SignalScope::Element);
        assert!(
            !js.contains("ST.get"),
            "bare $5 must not become a hole: {js}"
        );
        assert!(
            js.contains("price: $5") || js.contains("$5"),
            "literal $5 preserved: {js}"
        );
    }

    #[test]
    fn double_dollar_collapses_to_literal() {
        // `$$` is the escape for a literal `$` before an identifier — the same
        // rule the emit tokenizer (BUG-112) and the runtime interpolator
        // (templates.js) honor. A template DISPLAYING st source as text
        // (`.clock { text <- $now; }`) writes `$$now` and must render `$now`
        // on the builder path exactly as on the string-interpolation path.
        let exprs = component_html_to_exprs("<code>.clock { text &lt;- $$now; }</code>");
        let js = emit_builder(&exprs, SignalScope::Element);
        assert!(
            js.contains("$now"),
            "$$ must collapse to a single literal $: {js}"
        );
        assert!(
            !js.contains("$$now"),
            "the escape marker must not survive to the DOM: {js}"
        );
    }

    #[test]
    fn directive_attr_is_detected_for_refusal() {
        // BUG-121: an `@`-prefixed attribute (the misplaced-directive / typo case) must be
        // FOUND so the compiler can refuse it, never silently lowered to a dead setAttribute.
        // `<button @mcp-action="approve">` is the exact authoring that started this.
        let exprs = component_html_to_exprs("<button @mcp-action=\"approve\">x</button>");
        let found = find_directive_attrs(&exprs);
        assert_eq!(found.len(), 1, "one offending @-attr found: {found:?}");
        assert_eq!(found[0].0, "button", "tag is button");
        assert_eq!(found[0].1, "@mcp-action", "name is @mcp-action");
    }

    #[test]
    fn directive_attr_detected_nested_and_multiple() {
        // Nested + multiple @-attrs are all collected (so the diagnostic can name each).
        let exprs = component_html_to_exprs(
            "<div @on=\"x\"><span class=\"ok\"><input @mcp-action=\"y\"></span></div>",
        );
        let found = find_directive_attrs(&exprs);
        let names: Vec<&str> = found.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"@on"), "outer @on found: {found:?}");
        assert!(
            names.contains(&"@mcp-action"),
            "nested @mcp-action found: {found:?}"
        );
        assert_eq!(found.len(), 2, "exactly the two @-attrs: {found:?}");
    }

    #[test]
    fn clean_tree_has_no_directive_attrs() {
        // Ordinary attributes (incl. data-*, class, reactive holes) are NOT flagged.
        let exprs = component_html_to_exprs(
            "<button class=\"approve\" data-mcp-action=\"kit-choice\" data-n=\"`$id`\">x</button>",
        );
        let found = find_directive_attrs(&exprs);
        assert!(found.is_empty(), "no @-attrs in a clean tree: {found:?}");
    }

    #[test]
    fn emits_global_scope_listeners() {
        let tree = vec![HtmlExpr::Element {
            tag: "p".to_string(),
            attrs: vec![],
            children: vec![HtmlExpr::Hole(hole("$total"))],
            line: None,
        }];
        let js = emit_builder(&tree, SignalScope::Global);
        assert!(js.contains("SpacetimeLocal['total']"), "js: {js}");
        assert!(js.contains("local:total:updated"), "js: {js}");
    }

    #[test]
    fn folds_st_md_element_to_markdown_node() {
        // `<st-md>`$m.text`</st-md>` in a template body becomes a Markdown node.
        let exprs = component_html_to_exprs("<st-md>`$m.text`</st-md>");
        assert_eq!(exprs.len(), 1, "exprs: {exprs:?}");
        match &exprs[0] {
            HtmlExpr::Markdown(src) => assert_eq!(src.trim(), "$m.text"),
            other => panic!("expected Markdown, got {other:?}"),
        }
    }

    #[test]
    fn folds_nested_st_md() {
        // An `<st-md>` nested inside another element is folded too.
        let exprs = component_html_to_exprs("<div class=\"b\"><st-md>`$x`</st-md></div>");
        match &exprs[0] {
            HtmlExpr::Element { children, .. } => match &children[0] {
                HtmlExpr::Markdown(src) => assert_eq!(src.trim(), "$x"),
                other => panic!("expected nested Markdown, got {other:?}"),
            },
            other => panic!("expected Element, got {other:?}"),
        }
    }

    #[test]
    fn markdown_node_emits_reactive_snarkdown() {
        // The Markdown node's builder renders via snarkdown, dep-watching its signal.
        let tree = vec![HtmlExpr::Markdown("$m.text".to_string())];
        let js = emit_builder(&tree, SignalScope::Global);
        assert!(js.contains("snarkdown("), "js: {js}");
        assert!(js.contains("innerHTML"), "js: {js}");
        assert!(js.contains("applyMd"), "js: {js}");
    }
}
