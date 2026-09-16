//! Spacetime expression lowering (PLAN-133).
//!
//! A `sexpr` is a Spacetime signal expression: JS-expression syntax plus
//! signal references. It is parsed by a REAL parser (SWC, already vendored for
//! output validation) and lowered to plain JS at compile time — the compiler
//! owns name resolution, ending the runtime regex/char-scan rewriters that
//! produced BUG-273 (lambda params mangled into `SpacetimeLocal['r']`).
//!
//! Language rules (one meaning per sigil):
//! - `$name`   — a SIGNAL, unless bound by an enclosing arrow parameter.
//!               Unbound: recorded as a dependency and renamed to the bare
//!               positional-parameter slot `name`. Arrow params are the ONLY
//!               local `$`-binding form; a `$x` that is not arrow-bound is
//!               always a signal. Bound arrow params (and their uses) are
//!               STRIPPED to plain JS idents (`($r) => $r.x` → `(r) => r.x`):
//!               generated JS carries no `$` locals, so the emit layer's
//!               invariant "`$` = signal" holds for every byte it sees.
//! - `$`       — the current ROW (`$.field`), only meaningful in a Row
//!               context (@data query); an E0957 error in a derivation.
//! - `a | f(b)` — left-insertion pipe, desugared to `f(a, b)` BEFORE parsing
//!               (surface-syntax desugar, ported from the two JS copies —
//!               derived-signal.st and computed-source.st — it replaces).
//! - bare `f(…)` where `f` names a compile-time-known filter/helper →
//!               rewritten to `ST.filters.f(…)`. Unknown bare calls pass
//!               through verbatim (JS globals keep working).
//!
//! Rewrites are SPAN EDITS against the parsed AST applied to the desugared
//! source text: the structure comes from the parse, the text keeps its
//! original formatting — no pretty-printer, no codegen dependency.

use std::collections::HashSet;

use swc_common::{FileName, SourceMap, input::SourceFileInput};
use swc_ecma_ast::*;
use swc_ecma_parser::{EsSyntax, Parser, Syntax};

use crate::diagnostics::{Diagnostic, DiagnosticCode};

/// Result of lowering one Spacetime expression.
#[derive(Debug, Clone, PartialEq)]
pub struct StExprLowering {
    /// The lowered JS expression source (span edits applied).
    pub body: String,
    /// Signal dependencies, first-appearance order, deduped (bare names).
    pub deps: Vec<String>,
}

/// Whether the expression has a current-row context (`$.field`).
#[derive(Debug, Clone, PartialEq)]
pub enum RowCtx {
    /// Derivation context: no row. A bare `$` is an E0957 error.
    Global,
    /// Per-item query context: `$` lowers to the given JS variable (e.g. "item").
    Row(String),
}

/// Lower a Spacetime signal expression to plain JS.
///
/// `is_helper` names compile-time-known filters (page-defined `@fn`s): a bare
/// call `f(…)` with `is_helper(f)` rewrites to `ST.filters.f(…)`.
pub fn lower_st_expr(
    src: &str,
    row: &RowCtx,
    is_helper: &dyn Fn(&str) -> bool,
) -> Result<StExprLowering, Diagnostic> {
    let desugared = desugar_pipes(src);
    let (expr, base) = parse_expr(&desugared).map_err(|msg| {
        Diagnostic::error(
            DiagnosticCode::E0957,
            format!("sexpr does not parse: {msg}\n  in: {src}"),
        )
    })?;

    let mut edits;
    let deps;
    {
        let mut walk = Walk {
            src: &desugared,
            base,
            scopes: vec![HashSet::new()],
            edits: Vec::new(),
            deps: Vec::new(),
            row,
            is_helper,
            row_error: None,
        };
        walk.expr(&expr);

        if walk.row_error.is_some() {
            return Err(Diagnostic::error(
                DiagnosticCode::E0957,
                format!(
                    "bare `$` row reference in a derivation expression — `$` row refs only exist inside per-item query expressions (@data query)\n  in: {src}"
                ),
            ));
        }
        edits = walk.edits;
        deps = walk.deps;
    }

    let mut body = desugared;
    // Apply right-to-left so earlier offsets stay valid.
    edits.sort_by_key(|(lo, _, _)| std::cmp::Reverse(*lo));
    for (lo, hi, replacement) in edits {
        body.replace_range(lo..hi, &replacement);
    }

    Ok(StExprLowering { body, deps })
}

/// Lower the embedded expressions of a cond_block arms record (derive-match).
///
/// The record arrives as serialized JS data with EXPRESSION STRINGS inside:
/// `{ arms: [{ guard: "_" | { expr: "…" }, cons: { name, payload: [{ arg: "…" }] } }] }`.
/// For every `expr:`/`arg:` string field, the CONTENT is lowered with
/// [`lower_st_expr`] (Global ctx — guards and payload args are derivation
/// expressions) and the object gains `f` + `deps` siblings:
/// `{ expr: "$isA >= 1" }` → `{ expr: "$isA >= 1", f: (isA) => isA >= 1, deps: ["isA"] }`.
/// The original string stays — the runtime needs it for catch-all detection,
/// payload field naming, and error messages. The catch-all guard `"_"` (a
/// bare string, not an object) carries no expr and is skipped.
///
/// Returns the rewritten record WRAPPED IN PARENS (spliceable as an
/// expression; the parens are what lets `{ … }` parse as an object literal).
pub fn lower_cond_arms(src: &str, is_helper: &dyn Fn(&str) -> bool) -> Result<String, Diagnostic> {
    let wrapped = format!("({src})");
    let (expr, base) = parse_expr(&wrapped).map_err(|msg| {
        Diagnostic::error(
            DiagnosticCode::E0957,
            format!("cond-arms record does not parse: {msg}\n  in: {src}"),
        )
    })?;

    // Collect (insert_offset, field_text) pairs: every `expr: "…"` / `arg: "…"`
    // string-literal field in the object/array tree.
    let mut hits: Vec<(usize, String)> = Vec::new();
    collect_cond_arm_fields(&expr, &wrapped, base, &mut hits);

    let mut edits: Vec<SpanEdit> = Vec::new();
    for (at, content) in hits {
        let low = lower_st_expr(&content, &RowCtx::Global, is_helper)?;
        let params = low.deps.join(", ");
        let deps_json = format!(
            "[{}]",
            low.deps
                .iter()
                .map(|d| format!("\"{d}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        edits.push((
            at,
            at,
            format!(", f: ({params}) => {}, deps: {deps_json}", low.body),
        ));
    }

    let mut out = wrapped;
    edits.sort_by_key(|(lo, _, _)| std::cmp::Reverse(*lo));
    for (lo, hi, replacement) in edits {
        out.replace_range(lo..hi, &replacement);
    }
    Ok(out)
}

/// Recursively collect `expr:`/`arg:` string-literal fields from the
/// object/array tree of a cond_block record. Each hit is (insert_offset,
/// cooked_string_content) — the offset sits right after the closing quote.
fn collect_cond_arm_fields(expr: &Expr, src: &str, base: usize, hits: &mut Vec<(usize, String)>) {
    match expr {
        Expr::Paren(p) => collect_cond_arm_fields(&p.expr, src, base, hits),
        Expr::Object(obj) => {
            for prop in &obj.props {
                let PropOrSpread::Prop(prop) = prop else { continue };
                let Prop::KeyValue(kv) = &**prop else { continue };
                let key_matches = match &kv.key {
                    PropName::Ident(id) => id.sym.as_ref() == "expr" || id.sym.as_ref() == "arg",
                    PropName::Str(s) => s.value.as_ref() == "expr" || s.value.as_ref() == "arg",
                    _ => false,
                };
                if key_matches {
                    if let Expr::Lit(Lit::Str(s)) = &*kv.value {
                        let at = (s.span.hi.0 as usize) - base;
                        hits.push((at, s.value.to_string()));
                    }
                } else {
                    collect_cond_arm_fields(&kv.value, src, base, hits);
                }
            }
        }
        Expr::Array(arr) => {
            for el in arr.elems.iter().flatten() {
                collect_cond_arm_fields(&el.expr, src, base, hits);
            }
        }
        _ => {}
    }
}

/// Desugar the left-insertion pipe `a | f(b)` → `f(a, b)`, `a | f` → `f(a)`.
/// Splits ONLY top-level single `|` (not `||`), respecting (), [], {}, and
/// quotes — a byte-for-byte port of the JS `desugarPipes` in
/// derived-signal.st / computed-source.st (one implementation now, tested).
pub fn desugar_pipes(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut segs: Vec<String> = Vec::new();
    let mut depth = 0i32;
    let mut last = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == '"' || c == '\'' || c == '`' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == '\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == c {
                    break;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '|' if depth == 0
                && (i == 0 || bytes[i - 1] != '|')
                && (i + 1 >= bytes.len() || bytes[i + 1] != '|') =>
            {
                segs.push(bytes[last..i].iter().collect());
                last = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    segs.push(bytes[last..].iter().collect());
    if segs.len() == 1 {
        return text.to_string();
    }

    let mut acc = segs[0].trim().to_string();
    for raw in &segs[1..] {
        let seg = raw.trim();
        // `f(rest)` → f(acc, rest); bare `f` → f(acc)
        if let Some(open) = seg.find('(')
            && seg.ends_with(')')
        {
            let name = seg[..open].trim();
            if is_js_ident(name) {
                let args = seg[open + 1..seg.len() - 1].trim();
                acc = if args.is_empty() {
                    format!("{name}({acc})")
                } else {
                    format!("{name}({acc}, {args})")
                };
                continue;
            }
        }
        if is_js_ident(seg) {
            acc = format!("{seg}({acc})");
            continue;
        }
        // Non-call segment: keep the old behavior (seg(acc)).
        acc = format!("{seg}({acc})");
    }
    acc
}

fn is_js_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Parse a JS expression, returning the AST and the byte offset of the
/// source's first byte within the SourceMap file (span base).
fn parse_expr(src: &str) -> Result<(Box<Expr>, usize), String> {
    let cm = SourceMap::default();
    let fm = cm.new_source_file(FileName::Anon.into(), src.to_string());
    let base = fm.start_pos.0 as usize;
    let mut parser = Parser::new(
        Syntax::Es(EsSyntax::default()),
        SourceFileInput::from(&*fm),
        None,
    );
    let expr = parser.parse_expr().map_err(|e| format!("{:?}", e.into_kind()))?;
    let errors = parser.take_errors();
    if let Some(first) = errors.first() {
        return Err(format!("{:?}", first.clone().into_kind()));
    }
    Ok((expr, base))
}

/// One pending span edit: replace `src[lo..hi]` with `replacement`.
type SpanEdit = (usize, usize, String);

struct Walk<'a> {
    src: &'a str,
    base: usize,
    /// Lexical scope stack: each frame holds locally-bound names AS WRITTEN
    /// (arrow params keep any `$` prefix — `$r` and `r` are distinct idents).
    scopes: Vec<HashSet<String>>,
    edits: Vec<SpanEdit>,
    deps: Vec<String>,
    row: &'a RowCtx,
    is_helper: &'a dyn Fn(&str) -> bool,
    /// Deferred `$`-in-Global error (the walk stays infallible; reported after).
    row_error: Option<swc_common::Span>,
}

impl<'a> Walk<'a> {
    fn is_bound(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|frame| frame.contains(name))
    }

    fn offset(&self, pos: swc_common::BytePos) -> usize {
        pos.0 as usize - self.base
    }

    fn ident_text(&self, span: swc_common::Span) -> &str {
        &self.src[self.offset(span.lo)..self.offset(span.hi)]
    }

    fn record_dep(&mut self, name: &str) {
        if !self.deps.iter().any(|d| d == name) {
            self.deps.push(name.to_string());
        }
    }

    /// A `$name` occurrence outside arrow binding: signal dependency.
    /// Renames the span to the bare positional slot; records the dep.
    fn lower_signal_ident(&mut self, ident: &Ident) {
        let text = self.ident_text(ident.span).to_string();
        let name = text[1..].to_string(); // strip `$`
        self.record_dep(&name);
        self.edits.push((
            self.offset(ident.span.lo),
            self.offset(ident.span.hi),
            name,
        ));
    }

    /// Visit an identifier in VALUE position (not a binding site, not a
    /// property name after `.`).
    fn value_ident(&mut self, ident: &Ident) {
        let text = self.ident_text(ident.span).to_string();
        if text == "$" {
            match self.row {
                RowCtx::Row(var) => {
                    let var = var.clone();
                    self.edits.push((
                        self.offset(ident.span.lo),
                        self.offset(ident.span.hi),
                        var,
                    ));
                }
                RowCtx::Global => {
                    // Recorded as an edit-error below; see `row_error`.
                    self.row_error = Some(ident.span);
                }
            }
            return;
        }
        if text.starts_with('$') {
            if self.is_bound(&text) {
                // Arrow-bound local: strip the sigil — generated JS carries
                // plain idents only (`$r` → `r`, matching the stripped
                // binding site).
                self.edits.push((
                    self.offset(ident.span.lo),
                    self.offset(ident.span.hi),
                    text[1..].to_string(),
                ));
            } else {
                self.lower_signal_ident(ident);
            }
        }
    }
}

// The recursive walker itself.
impl<'a> Walk<'a> {
    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(ident) => self.value_ident(ident),

            Expr::Arrow(arrow) => {
                // Binding form: params introduce a scope over the body.
                // `$`-prefixed params are STRIPPED at the binding site, so
                // the emitted JS holds plain idents (`($r) => …` → `(r) => …`).
                let mut frame = HashSet::new();
                for pat in &arrow.params {
                    collect_pat_idents(pat, &mut frame);
                    self.strip_pat_sigils(pat);
                }
                self.scopes.push(frame);
                match &*arrow.body {
                    BlockStmtOrExpr::BlockStmt(block) => {
                        for stmt in &block.stmts {
                            self.stmt(stmt);
                        }
                    }
                    BlockStmtOrExpr::Expr(body) => self.expr(body),
                }
                self.scopes.pop();
            }

            Expr::Fn(fn_expr) => {
                let mut frame = HashSet::new();
                for param in &fn_expr.function.params {
                    collect_pat_idents(&param.pat, &mut frame);
                }
                self.scopes.push(frame);
                if let Some(body) = &fn_expr.function.body {
                    for stmt in &body.stmts {
                        self.stmt(stmt);
                    }
                }
                self.scopes.pop();
            }

            Expr::Call(call) => {
                // Bare helper call: `darken(…)` → `ST.filters.darken(…)` when
                // the callee is a compile-time-known filter and not a local.
                if let Callee::Expr(callee) = &call.callee
                    && let Expr::Ident(ident) = &**callee
                {
                    let name = self.ident_text(ident.span).to_string();
                    if !name.starts_with('$')
                        && !self.is_bound(&name)
                        && (self.is_helper)(&name)
                    {
                        self.edits.push((
                            self.offset(ident.span.lo),
                            self.offset(ident.span.hi),
                            format!("ST.filters.{name}"),
                        ));
                    }
                }
                if let Callee::Expr(callee) = &call.callee {
                    self.expr(callee);
                }
                for arg in &call.args {
                    self.expr(&arg.expr);
                }
            }

            Expr::Member(member) => {
                self.expr(&member.obj);
                if let MemberProp::Computed(computed) = &member.prop {
                    self.expr(&computed.expr);
                }
                // Non-computed property names are not value idents.
            }

            Expr::Bin(bin) => {
                self.expr(&bin.left);
                self.expr(&bin.right);
            }
            Expr::Unary(un) => self.expr(&un.arg),
            Expr::Cond(cond) => {
                self.expr(&cond.test);
                self.expr(&cond.cons);
                self.expr(&cond.alt);
            }
            Expr::Paren(paren) => self.expr(&paren.expr),
            Expr::Tpl(tpl) => {
                for e in &tpl.exprs {
                    self.expr(e);
                }
            }
            Expr::Array(arr) => {
                for el in arr.elems.iter().flatten() {
                    self.expr(&el.expr);
                }
            }
            Expr::Object(obj) => {
                for prop in &obj.props {
                    match prop {
                        PropOrSpread::Spread(spread) => self.expr(&spread.expr),
                        PropOrSpread::Prop(p) => match &**p {
                            Prop::KeyValue(kv) => {
                                if let PropName::Computed(c) = &kv.key {
                                    self.expr(&c.expr);
                                }
                                self.expr(&kv.value);
                            }
                            Prop::Shorthand(ident) => self.value_ident(ident),
                            Prop::Method(m) => {
                                if let PropName::Computed(c) = &m.key {
                                    self.expr(&c.expr);
                                }
                            }
                            Prop::Getter(g) => {
                                if let PropName::Computed(c) = &g.key {
                                    self.expr(&c.expr);
                                }
                            }
                            Prop::Setter(s) => {
                                if let PropName::Computed(c) = &s.key {
                                    self.expr(&c.expr);
                                }
                            }
                            Prop::Assign(_) => {}
                        },
                    }
                }
            }
            Expr::Seq(seq) => {
                for e in &seq.exprs {
                    self.expr(e);
                }
            }
            Expr::Assign(assign) => {
                self.expr(&assign.right);
                // assignment targets in derivations are out of scope
            }
            Expr::Await(a) => self.expr(&a.arg),
            Expr::Update(u) => self.expr(&u.arg),
            Expr::OptChain(chain) => match &*chain.base {
                OptChainBase::Member(member) => {
                    self.expr(&member.obj);
                    if let MemberProp::Computed(computed) = &member.prop {
                        self.expr(&computed.expr);
                    }
                }
                OptChainBase::Call(call) => {
                    self.expr(&call.callee);
                    for arg in &call.args {
                        self.expr(&arg.expr);
                    }
                }
            },
            Expr::New(new_expr) => {
                self.expr(&new_expr.callee);
                for arg in new_expr.args.iter().flatten() {
                    self.expr(&arg.expr);
                }
            }
            Expr::TaggedTpl(tagged) => {
                self.expr(&tagged.tag);
                for e in &tagged.tpl.exprs {
                    self.expr(e);
                }
            }
            Expr::Class(_) | Expr::Yield(_) | Expr::MetaProp(_) | Expr::SuperProp(_)
            | Expr::Invalid(_) | Expr::TsAs(_) | Expr::TsSatisfies(_) | Expr::TsConstAssertion(_)
            | Expr::TsTypeAssertion(_) | Expr::TsNonNull(_) | Expr::TsInstantiation(_)
            | Expr::PrivateName(_) | Expr::JSXMember(_) | Expr::JSXNamespacedName(_)
            | Expr::JSXEmpty(_) | Expr::JSXElement(_) | Expr::JSXFragment(_)
            | Expr::Lit(_) | Expr::This(_) => {}
        }
    }

    /// Strip `$` sigils from arrow/function param BINDING SITES, so the
    /// emitted JS holds plain idents (`($r) => …` → `(r) => …`).
    fn strip_pat_sigils(&mut self, pat: &Pat) {
        match pat {
            Pat::Ident(binding) => {
                let text = self.ident_text(binding.id.span).to_string();
                if text == "$" {
                    // A param named exactly `$` binds the ROW locally
                    // (`(s, $) => $.price`). Rename the binding to the row
                    // var so value-position `$` (rewritten to the same var)
                    // resolves to it. Global ctx has no row — same E0957 as
                    // a value-position bare `$`.
                    match self.row {
                        RowCtx::Row(var) => {
                            let var = var.clone();
                            self.edits.push((
                                self.offset(binding.id.span.lo),
                                self.offset(binding.id.span.hi),
                                var,
                            ));
                        }
                        RowCtx::Global => {
                            self.row_error = Some(binding.id.span);
                        }
                    }
                } else if text.starts_with('$') {
                    self.edits.push((
                        self.offset(binding.id.span.lo),
                        self.offset(binding.id.span.hi),
                        text[1..].to_string(),
                    ));
                }
            }
            Pat::Array(arr) => {
                for el in arr.elems.iter().flatten() {
                    self.strip_pat_sigils(el);
                }
            }
            Pat::Object(obj) => {
                for prop in &obj.props {
                    match prop {
                        ObjectPatProp::KeyValue(kv) => self.strip_pat_sigils(&kv.value),
                        ObjectPatProp::Assign(assign) => {
                            let text = self.ident_text(assign.key.id.span).to_string();
                            if text.starts_with('$') {
                                self.edits.push((
                                    self.offset(assign.key.id.span.lo),
                                    self.offset(assign.key.id.span.hi),
                                    text[1..].to_string(),
                                ));
                            }
                        }
                        ObjectPatProp::Rest(rest) => self.strip_pat_sigils(&rest.arg),
                    }
                }
            }
            Pat::Rest(rest) => self.strip_pat_sigils(&rest.arg),
            Pat::Assign(assign) => self.strip_pat_sigils(&assign.left),
            _ => {}
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        // Block-body arrows: expression positions inside statements. Local
        // `const x = …` declarations bind only BARE names (the `$` namespace
        // is signals-only), so no scope tracking is needed here.
        match stmt {
            Stmt::Expr(e) => self.expr(&e.expr),
            Stmt::Return(r) => {
                if let Some(arg) = &r.arg {
                    self.expr(arg);
                }
            }
            Stmt::Decl(Decl::Var(v)) => {
                for decl in &v.decls {
                    if let Some(init) = &decl.init {
                        self.expr(init);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Collect simple identifier bindings from a pattern (arrow/function params).
/// Destructuring patterns bind their leaf idents; rest/default unwrap.
fn collect_pat_idents(pat: &Pat, out: &mut HashSet<String>) {
    match pat {
        Pat::Ident(binding) => {
            out.insert(binding.id.sym.to_string());
        }
        Pat::Array(arr) => {
            for el in arr.elems.iter().flatten() {
                collect_pat_idents(el, out);
            }
        }
        Pat::Object(obj) => {
            for prop in &obj.props {
                match prop {
                    ObjectPatProp::KeyValue(kv) => collect_pat_idents(&kv.value, out),
                    ObjectPatProp::Assign(assign) => {
                        out.insert(assign.key.id.sym.to_string());
                    }
                    ObjectPatProp::Rest(rest) => collect_pat_idents(&rest.arg, out),
                }
            }
        }
        Pat::Rest(rest) => collect_pat_idents(&rest.arg, out),
        Pat::Assign(assign) => collect_pat_idents(&assign.left, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_helpers(_: &str) -> bool {
        false
    }

    fn lower(src: &str) -> StExprLowering {
        lower_st_expr(src, &RowCtx::Global, &no_helpers).expect(src)
    }

    #[test]
    fn plain_signals_become_positional_slots() {
        let r = lower("$a + $b");
        assert_eq!(r.body, "a + b");
        assert_eq!(r.deps, vec!["a", "b"]);
    }

    #[test]
    fn deps_dedup_first_appearance() {
        let r = lower("$a + $a * $b + $a");
        assert_eq!(r.deps, vec!["a", "b"]);
    }

    #[test]
    fn lambda_params_are_locals_not_signals() {
        // BUG-273's case, structurally: $r is arrow-bound → stripped to a
        // plain JS local; never a dep, never a signal accessor.
        let r = lower("$comRecords.filter(($r) => $r.status == \"open\").length");
        assert_eq!(
            r.body,
            "comRecords.filter((r) => r.status == \"open\").length"
        );
        assert_eq!(r.deps, vec!["comRecords"]);
    }

    #[test]
    fn bare_lambda_params_also_bind() {
        let r = lower("$xs.map(x => x * $factor)");
        assert_eq!(r.body, "xs.map(x => x * factor)");
        assert_eq!(r.deps, vec!["xs", "factor"]);
    }

    #[test]
    fn nested_arrows_scope_independently() {
        let r = lower("$xs.flatMap(($g) => $g.items.filter(($i) => $i.qty > $min))");
        assert_eq!(
            r.body,
            "xs.flatMap((g) => g.items.filter((i) => i.qty > min))"
        );
        assert_eq!(r.deps, vec!["xs", "min"]);
    }

    #[test]
    fn shadowing_a_signal_name_inside_an_arrow() {
        // `$y` outside is a signal (dep y); `($y) =>` binds a LOCAL y inside.
        // After stripping, the arrow param `y` JS-shadows the dep slot `y` —
        // which is exactly the language rule (the arrow binding is more local),
        // so both meanings survive the shared name.
        let r = lower("[$y].map(($y) => $y * 2)");
        assert_eq!(r.body, "[y].map((y) => y * 2)");
        assert_eq!(r.deps, vec!["y"]);
    }

    #[test]
    fn dollar_inside_string_literals_is_not_a_signal() {
        // The old char-scan needed a quote-aware special case; the parse gives
        // it for free.
        let r = lower("\"$notASignal\" + $real");
        assert_eq!(r.body, "\"$notASignal\" + real");
        assert_eq!(r.deps, vec!["real"]);
    }

    #[test]
    fn template_literal_expressions_lower() {
        let r = lower("`total: ${$total}`");
        assert_eq!(r.body, "`total: ${total}`");
        assert_eq!(r.deps, vec!["total"]);
    }

    #[test]
    fn pipe_desugars_then_lowers() {
        let is_filter = |name: &str| name == "darken";
        let r = lower_st_expr("$brand.scarlet | darken(0.08)", &RowCtx::Global, &is_filter)
            .expect("pipe");
        assert_eq!(r.body, "ST.filters.darken(brand.scarlet, 0.08)");
        assert_eq!(r.deps, vec!["brand"]);
    }

    #[test]
    fn pipe_bare_fn_form() {
        let is_filter = |name: &str| name == "titleize";
        let r = lower_st_expr("$slug | titleize", &RowCtx::Global, &is_filter).expect("pipe");
        assert_eq!(r.body, "ST.filters.titleize(slug)");
        assert_eq!(r.deps, vec!["slug"]);
    }

    #[test]
    fn unknown_bare_calls_pass_through() {
        let r = lower("Math.max($a, 1) + Number($b)");
        assert_eq!(r.body, "Math.max(a, 1) + Number(b)");
        assert_eq!(r.deps, vec!["a", "b"]);
    }

    #[test]
    fn bound_bare_param_is_not_a_helper_call() {
        // An arrow param named like a filter must not rewrite.
        let is_filter = |name: &str| name == "darken";
        let r = lower_st_expr("$xs.map(darken => darken($v))", &RowCtx::Global, &is_filter)
            .expect("shadow");
        assert_eq!(r.body, "xs.map(darken => darken(v))");
        assert_eq!(r.deps, vec!["xs", "v"]);
    }

    #[test]
    fn facet_style_parenthesized_comparison() {
        let r = lower("($cartTotal >= 1)");
        assert_eq!(r.body, "(cartTotal >= 1)");
        assert_eq!(r.deps, vec!["cartTotal"]);
    }

    #[test]
    fn optional_chain_and_nullish() {
        let r = lower("$a?.x ?? $b");
        assert_eq!(r.body, "a?.x ?? b");
        assert_eq!(r.deps, vec!["a", "b"]);
    }

    #[test]
    fn row_ref_errors_in_global_context() {
        let err = lower_st_expr("$.price * $qty", &RowCtx::Global, &no_helpers);
        assert!(err.is_err());
    }

    #[test]
    fn bare_dollar_arrow_param_binds_the_row() {
        // `(s, $) => s + $.price` — a param named exactly `$` is the row,
        // bound locally: binding site and value-position `$` both rename to
        // the row var. The reduce/fold author shape.
        let r = lower_st_expr("(s, $) => s + $.price * $.qty", &RowCtx::Row("item".into()), &no_helpers)
            .expect("row arrow");
        assert_eq!(r.body, "(s, item) => s + item.price * item.qty");
        assert_eq!(r.deps, Vec::<String>::new());
    }

    #[test]
    fn bare_dollar_arrow_param_errors_in_global_context() {
        let err = lower_st_expr("($) => $.x", &RowCtx::Global, &no_helpers);
        assert!(err.is_err());
    }

    #[test]
    fn cond_arms_lowers_guard_and_payload_fields() {
        let src = r##"{ arms: [{ guard: { expr: "$isOrange" }, cons: { name: "Orange", payload: [] } }, { guard: { expr: "$isErr" }, cons: { name: "Failed", payload: [{ arg: "$error" }] } }, { guard: "_", cons: { name: "Green", payload: [] } }] }"##;
        let out = lower_cond_arms(src, &no_helpers).expect("arms lower");
        assert!(
            out.contains(r##"{ expr: "$isOrange", f: (isOrange) => isOrange, deps: ["isOrange"] }"##),
            "guard gains f + deps: {out}"
        );
        assert!(
            out.contains(r##"{ arg: "$error", f: (error) => error, deps: ["error"] }"##),
            "payload arg gains f + deps: {out}"
        );
        // The catch-all string is untouched (no f inserted after a bare "_").
        assert!(
            out.contains(r##"guard: "_", cons: { name: "Green", payload"##),
            "catch-all untouched: {out}"
        );
        // Original strings stay (catch-all detection + field naming read them).
        assert!(out.contains(r##""$isOrange""##));
    }

    #[test]
    fn cond_arms_constant_guard_gets_nullary_fn() {
        let src = r##"{ arms: [{ guard: { expr: "1 >= 1" }, cons: { name: "A", payload: [] } }] }"##;
        let out = lower_cond_arms(src, &no_helpers).expect("lower");
        assert!(out.contains("f: () => 1 >= 1, deps: []"), "nullary fn: {out}");
    }

    #[test]
    fn cond_arms_row_ref_is_an_error() {
        let src = r##"{ arms: [{ guard: { expr: "$.x" }, cons: { name: "A", payload: [] } }] }"##;
        assert!(lower_cond_arms(src, &no_helpers).is_err());
    }

    #[test]
    fn row_ref_lowers_in_row_context() {
        let r = lower_st_expr("$.price * $qty", &RowCtx::Row("item".into()), &no_helpers)
            .expect("row");
        assert_eq!(r.body, "item.price * qty");
        assert_eq!(r.deps, vec!["qty"]);
    }

    #[test]
    fn pipe_desugar_unit() {
        assert_eq!(desugar_pipes("$a"), "$a");
        assert_eq!(desugar_pipes("$a | f"), "f($a)");
        assert_eq!(desugar_pipes("$a | f(1)"), "f($a, 1)");
        assert_eq!(desugar_pipes("$a | f | g(2)"), "g(f($a), 2)");
        // `||` is not a pipe
        assert_eq!(desugar_pipes("$a || $b"), "$a || $b");
        // nested pipes inside parens are not top-level
        assert_eq!(desugar_pipes("f($a | g)"), "f($a | g)");
        // quoted pipe chars are not pipes
        assert_eq!(desugar_pipes("$a | f(\"|\")"), "f($a, \"|\")");
    }
}
