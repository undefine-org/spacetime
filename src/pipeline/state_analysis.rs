//! Page-global state provenance (PLAN-117 W2).
//!
//! Answers, for every page-global `$cell`: **who declared it, who reads it, who
//! writes it, and what can the compiler prove about it?**
//!
//! Why this exists. `@import` merges many files into one page, so a page-global
//! `$host` may be declared anywhere in the import graph. Before this pass:
//!
//! - two files declaring the same `$host` merged **silently** into one cell,
//!   while every other registry refuses a duplicate definition (`DuplicateMacro`,
//!   `DuplicatePrimitive`). `$` was the only exempt kind, and that exemption was
//!   the bug. E0938 closes it.
//! - nothing recorded WHERE a cell came from, so a diagnostic could not name the
//!   conflicting files and `inspect` could not show provenance.
//!
//! The verdict is **proven, not inferred**. Spacetime has no custom JavaScript —
//! no `eval`, no foreign call, no escape hatch — so a cell's write-set is CLOSED
//! and settled by scanning the page's own matches. That closure is what lets W4
//! const-fold soundly where a general-purpose language could only guess.
//!
//! One structure, three consumers: the diagnostic (E0938 reads `declared_in`),
//! the documentation (`inspect --layer state`), and the W4 optimisation
//! (`verdict`).

use std::collections::BTreeMap;

use crate::parser::{NestedScope, ScopeBlock, StFile};
use crate::pipeline::scope::{StateProvenance, StateVerdict};
use crate::syntax::{CapturedValue, FormMatch, collect_signal_deps};

/// Label for a match/scope whose `source_file` is `None` — i.e. the entry file
/// the compiler was pointed at, as opposed to something it pulled in.
pub const ENTRY_FILE_LABEL: &str = "<entry>";

/// Normalise a `source_file` into a stable, comparable label.
///
/// Two normalisations, both load-bearing rather than cosmetic:
///
/// 1. `None` means "the entry file" — import resolution stamps `source_file`
///    only on matches it PULLED IN, leaving the entry's own matches unstamped.
/// 2. Paths are reduced to a workspace-relative form. Imported matches carry an
///    ABSOLUTE path while the entry file is known by the relative path the user
///    typed; without this, the same file could be counted under two labels and
///    a single writer would be reported twice.
fn file_of(source_file: &Option<String>) -> String {
    match source_file {
        None => ENTRY_FILE_LABEL.to_string(),
        Some(p) => relativise(p),
    }
}

/// Reduce an absolute path to a workspace-relative one when possible, so labels
/// from different pipeline stages compare equal.
fn relativise(path: &str) -> String {
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(stripped) = std::path::Path::new(path).strip_prefix(&cwd)
    {
        return stripped.to_string_lossy().to_string();
    }
    path.to_string()
}

/// True when an initial value is a compile-time literal — the precondition for
/// a `Const` verdict. An expression initial (`$b: $a + 1`) is a computation and
/// stays reactive even with no writer, because folding it would require
/// evaluating arbitrary expressions at build time.
fn is_literal_initial(initial: &str) -> bool {
    let t = initial.trim();
    if t.is_empty() || t == "null" {
        return false;
    }
    // A quoted string, a number, or a boolean.
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        return true;
    }
    if t == "true" || t == "false" {
        return true;
    }
    t.parse::<f64>().is_ok()
}

/// Accumulator keyed by cell name.
#[derive(Default)]
struct Acc {
    declared_in: Vec<String>,
    type_name: String,
    initial: String,
    readers: BTreeMap<String, usize>,
    writers: BTreeMap<String, usize>,
}

/// Analyse every page-global cell on a fully-import-resolved page.
///
/// `ast` MUST be post-`resolve_imports`: this pass reasons about the WHOLE page,
/// which is exactly what makes cross-file duplicates and closed write-sets
/// visible. Element-scoped state (declared inside a selector block) is
/// deliberately excluded — a selector matches 0..N elements, so that state is
/// plural by construction and has no single address to report.
pub fn analyze_state(ast: &StFile) -> Vec<StateProvenance> {
    let mut acc: BTreeMap<String, Acc> = BTreeMap::new();

    // ---- declarations -----------------------------------------------------
    // Page-global == a `local-state` match with NO selector (file level).
    for fm in &ast.matches {
        if fm.macro_name != "local-state" && fm.macro_name != "local-state-uninitialized" {
            continue;
        }
        if fm.selector.is_some() {
            continue; // element-scoped: plural, not addressable
        }
        let name = fm.get_ident("name").unwrap_or_default().to_string();
        if name.is_empty() {
            continue;
        }
        let e = acc.entry(name).or_default();
        e.declared_in.push(file_of(&fm.source_file));
        if e.type_name.is_empty() {
            e.type_name = fm
                .get_ident("type")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "any".to_string());
            e.initial = match fm.captures.get("value") {
                Some(CapturedValue::Ident(s)) => s.clone(),
                Some(CapturedValue::Bool(b)) => b.to_string(),
                Some(CapturedValue::Number(n)) => n.to_string(),
                Some(CapturedValue::String(s)) => format!("\"{}\"", s),
                Some(CapturedValue::Expr(x)) => x.clone(),
                _ => "null".to_string(),
            };
        }
    }

    if acc.is_empty() {
        return Vec::new();
    }

    // ---- usages -----------------------------------------------------------
    // Reads and writes are counted per FILE, so a diagnostic can say where.
    let mut record = |name: &str, file: &str, is_write: bool, acc: &mut BTreeMap<String, Acc>| {
        if let Some(e) = acc.get_mut(name) {
            let bucket = if is_write {
                &mut e.writers
            } else {
                &mut e.readers
            };
            *bucket.entry(file.to_string()).or_insert(0) += 1;
        }
    };

    // A directive inside a scope is reachable from BOTH `ast.matches` (the flat
    // list the pipeline consumes) and `ast.scopes` (the tree the emitter walks).
    // Visiting both is required — a css_declaration read exists only in the tree,
    // a file-level directive only in the flat list — so the same `@on` handler is
    // seen twice and its write would be counted twice. Deduplicate by SPAN, which
    // is the identity of a usage SITE regardless of how it was reached.
    let mut seen_spans: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();

    for fm in &ast.matches {
        if !seen_spans.insert((fm.span.start, fm.span.end)) {
            continue;
        }
        collect_from_match(fm, &file_of(&fm.source_file), &mut acc, &mut record);
    }
    for scope in &ast.scopes {
        collect_from_scope(scope, &mut acc, &mut record, &mut seen_spans);
    }

    // File-scope MARKUP HOLES are reads too: `<li>`$x`</li>` renders the cell
    // (FEAT-078 hydrates it and re-renders on `local:x:updated`). They live on
    // `html_blocks`, not in `matches` or `scopes`, so a pass that walked only
    // those two would judge a hole-only cell DEAD and eliminate it — which is
    // exactly what happened: two suites went red with an empty hydration
    // marker. A reader the analysis cannot see is the one way this pass can
    // delete a working page.
    for block in &ast.html_blocks {
        for hole in &block.holes {
            scan_text(hole, ENTRY_FILE_LABEL, &mut acc, &mut record);
        }
    }

    // ---- verdicts ---------------------------------------------------------
    acc.into_iter()
        .map(|(var_name, e)| {
            let readers: Vec<(String, usize)> = e.readers.into_iter().collect();
            let writers: Vec<(String, usize)> = e.writers.into_iter().collect();
            let verdict = if readers.is_empty() {
                StateVerdict::Dead
            } else if writers.is_empty() && is_literal_initial(&e.initial) {
                StateVerdict::Const
            } else {
                StateVerdict::Reactive
            };
            StateProvenance {
                var_name,
                type_name: e.type_name,
                initial: e.initial,
                declared_in: e.declared_in,
                readers,
                writers,
                verdict,
            }
        })
        .collect()
}

/// W4 — inline every read of a write-free literal cell, then drop its
/// declaration.
///
/// A cell nobody writes, initialised to a literal, is a VALUE wearing a signal's
/// clothes. Folding it removes the cell, its subscription plumbing, and the
/// update path that could never fire — the page renders the same bytes with less
/// machinery.
///
/// Soundness rests on the same closure the rest of this module depends on: with
/// no custom JavaScript there is no `eval`, no foreign call, and no escape hatch
/// that could write the cell behind the compiler's back, so "write-free" is
/// PROVEN by a whole-page scan rather than assumed. A general-purpose language
/// cannot make this substitution safely; Spacetime can, because it closed the
/// hatch.
///
/// Conservative by construction: only LITERAL initials fold (a computed initial
/// like `$b: $a + 1` stays reactive — see `is_literal_initial`), and a single
/// writer anywhere on the page disqualifies the cell.
///
/// Returns the folded names, for the build report.
pub fn fold_const_state(ast: &mut StFile, provenance: &[StateProvenance]) -> Vec<String> {
    let foldable: Vec<(String, String)> = provenance
        .iter()
        .filter(|p| p.verdict == StateVerdict::Const)
        .map(|p| (p.var_name.clone(), p.initial.clone()))
        .collect();

    if foldable.is_empty() {
        return Vec::new();
    }

    for fm in &mut ast.matches {
        fold_in_match(fm, &foldable);
    }
    for scope in &mut ast.scopes {
        fold_in_scope(scope, &foldable);
    }

    // The DECLARATION is deliberately kept.
    //
    // Removing it looks like the natural finish — every read is now a literal,
    // so who needs the cell? — but the binding pipeline registers a reactive
    // binding BY its declared cell: drop the declaration and `text <- $host`
    // loses its registration and renders nothing. (Observed: the folded page
    // emitted no text binding at all and rendered an empty element.)
    //
    // Keeping it costs one initialiser and yields the win that matters: every
    // READ is now a literal, so no subscription is created and no update path
    // is emitted for a cell that could never change. Removing the declaration
    // too belongs with the binding-registration rework, not here — an
    // optimisation that renders the wrong page is not an optimisation.
    foldable.iter().map(|(n, _)| n.clone()).collect()
}

fn fold_in_match(fm: &mut FormMatch, foldable: &[(String, String)]) {
    let is_decl = fm.macro_name == "local-state" || fm.macro_name == "local-state-uninitialized";
    for (key, value) in fm.captures.iter_mut() {
        // A declaration's own `value` is the literal being folded IN; rewriting
        // it would be circular.
        if is_decl && (key == "value" || key == "name" || key == "type") {
            continue;
        }
        fold_in_value(value, foldable);
    }
}

fn fold_in_value(value: &mut CapturedValue, foldable: &[(String, String)]) {
    match value {
        CapturedValue::Block(children) => {
            for child in children {
                fold_in_match(child, foldable);
            }
        }
        CapturedValue::Array(items) => {
            for item in items {
                fold_in_value(item, foldable);
            }
        }
        CapturedValue::Named(map) => {
            for v in map.values_mut() {
                fold_in_value(v, foldable);
            }
        }
        CapturedValue::StyleProperties(pairs) => {
            for (_, v) in pairs.iter_mut() {
                fold_in_text(v, foldable);
            }
        }
        CapturedValue::Expr(e) => fold_in_text(e, foldable),
        CapturedValue::Binding(b) => fold_in_text(b, foldable),
        _ => {}
    }
}

fn fold_in_scope(scope: &mut ScopeBlock, foldable: &[(String, String)]) {
    for cd in &mut scope.css_declarations {
        fold_in_text(&mut cd.value, foldable);
    }
    for fm in &mut scope.matches {
        fold_in_match(fm, foldable);
    }
    for ns in &mut scope.nested_scopes {
        fold_in_nested(ns, foldable);
    }
}

fn fold_in_nested(ns: &mut NestedScope, foldable: &[(String, String)]) {
    for cd in &mut ns.css_declarations {
        fold_in_text(&mut cd.value, foldable);
    }
    for fm in &mut ns.matches {
        fold_in_match(fm, foldable);
    }
    for child in &mut ns.nested_scopes {
        fold_in_nested(child, foldable);
    }
}

/// Substitute `$name` -> its literal, on WHOLE-TOKEN boundaries only.
///
/// `$host` must not match inside `$hostname`, and a `$host.field` read keeps its
/// member access (`"https://…".field`), which is what the reader asked for.
fn fold_in_text(text: &mut String, foldable: &[(String, String)]) {
    for (name, literal) in foldable {
        let needle = format!("${name}");
        let mut out = String::with_capacity(text.len());
        let mut rest: &str = text;
        while let Some(idx) = rest.find(&needle) {
            let after = &rest[idx + needle.len()..];
            let boundary_ok = after
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-');
            out.push_str(&rest[..idx]);
            if boundary_ok {
                out.push_str(literal);
            } else {
                out.push_str(&needle);
            }
            rest = after;
        }
        out.push_str(rest);
        *text = out;
    }
}

/// W4 — eliminate page-global cells the page cannot observe.
///
/// Removes the DECLARATION of every cell with verdict [`StateVerdict::Dead`]:
/// nothing on the page reads it, so nothing can tell it existed. The cell's
/// initialiser, its slot in `SpacetimeLocal`, and its update plumbing all stop
/// being emitted.
///
/// Why this is SOUND here and merely hopeful elsewhere: Spacetime has no custom
/// JavaScript — no `eval`, no foreign call, no escape hatch — so "nothing reads
/// it" is a fact settled by scanning the assembled page, not a guess about what
/// some other script might do at runtime.
///
/// Deliberately CONSERVATIVE about what counts as unobservable:
///   - a cell with any reader survives, even if the read is dynamic;
///   - element-scoped state is never considered (it is plural by construction
///     and never enters this table);
///   - a cell that is written but never read is still dead — a write nobody can
///     observe changes nothing — but it keeps its writer's side effects, since
///     only the DECLARATION is removed, not the handler.
///
/// Returns the names eliminated, for the build report.
pub fn eliminate_dead_state(ast: &mut StFile, provenance: &[StateProvenance]) -> Vec<String> {
    let dead: Vec<String> = provenance
        .iter()
        .filter(|p| p.verdict == StateVerdict::Dead)
        .map(|p| p.var_name.clone())
        .collect();

    if dead.is_empty() {
        return dead;
    }

    ast.matches.retain(|fm| {
        let is_decl =
            fm.macro_name == "local-state" || fm.macro_name == "local-state-uninitialized";
        if !is_decl || fm.selector.is_some() {
            return true;
        }
        match fm.get_ident("name") {
            Some(n) => !dead.iter().any(|d| d == n),
            None => true,
        }
    });

    dead
}

/// E0938 — refuse a page that declares the same page-global cell twice.
///
/// A page holds ONE cell per name, exactly as it holds one macro per name
/// (`DuplicateMacro`) and one primitive per name (`DuplicatePrimitive`). Before
/// this check `$` was the only registry that merged duplicates silently: the
/// last declaration processed won, the other file's initial value vanished, and
/// nothing was reported.
///
/// The message NAMES every declaring file, because the whole difficulty of the
/// bug is that `@import` puts the conflict somewhere you are not looking.
pub fn check_duplicate_state(
    provenance: &[StateProvenance],
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode};

    provenance
        .iter()
        .filter(|p| p.declared_in.len() > 1)
        .map(|p| {
            let files = p.declared_in.join(", ");
            Diagnostic::error(
                DiagnosticCode::E0938,
                format!(
                    "${} is declared {} times on this page (in {})",
                    p.var_name,
                    p.declared_in.len(),
                    files
                ),
            )
            .with_hint(format!(
                "a page holds one cell per name, like one macro per name — the \
                 declarations were being merged silently. Rename one of them, \
                 e.g. `${}Fallback`, so each file owns a distinct cell.",
                p.var_name
            ))
        })
        .collect()
}

type Recorder<'a> = dyn FnMut(&str, &str, bool, &mut BTreeMap<String, Acc>) + 'a;

/// Walk a FormMatch for cell reads and writes.
///
/// A `@on` handler body arrives as raw JS-ish text, where `$x <- v` is a WRITE
/// of `x` and every other `$y` is a READ. Splitting on the arrow is what
/// separates the two: the left of `<-` is the target, the right is an
/// expression that may itself read other cells (`$n <- $n + 1` is both).
fn collect_from_match(
    fm: &FormMatch,
    file: &str,
    acc: &mut BTreeMap<String, Acc>,
    record: &mut Recorder<'_>,
) {
    // A declaration's own initial value is not a read of itself.
    let is_decl = fm.macro_name == "local-state" || fm.macro_name == "local-state-uninitialized";

    for (key, value) in &fm.captures {
        if is_decl && (key == "value" || key == "name" || key == "type") {
            continue;
        }
        collect_from_value(value, file, acc, record);
    }
}

/// Walk a captured value for cell reads and writes.
///
/// Containers must be DESCENDED into, not stringified: an `@on` handler body
/// arrives as `Named` (verified via `inspect --layer parse`: `body: [named]`),
/// so flattening it to text loses the `<-` structure that distinguishes a write
/// from a read. Every container variant recurses; every leaf is scanned as text.
fn collect_from_value(
    value: &CapturedValue,
    file: &str,
    acc: &mut BTreeMap<String, Acc>,
    record: &mut Recorder<'_>,
) {
    match value {
        CapturedValue::Block(children) => {
            for child in children {
                collect_from_match(child, file, acc, record);
            }
        }
        CapturedValue::Named(map) => {
            // A MUTATION (`$target <- $expr`) arrives DECOMPOSED into a Named
            // map keyed `target` + `expr` — the `<-` is stripped, so the text
            // scan below can never see the write. The sigil `@on &.<driver> { … }`
            // body (on_motion_body -> `$mut:mutation`) is the surviving form that
            // carries one; the colon form (`@on &.click: $x <- v;`) reaches it via
            // on_mutation. Recognised by STRUCTURE (both keys present) rather than
            // a macro-name allowlist, so a newly declared driver macro cannot
            // silently reintroduce the blind spot.
            if let (Some(target), Some(expr)) = (map.get("target"), map.get("expr")) {
                let target_text = target.to_js(crate::syntax::JsQuoting::Raw);
                let cell = target_text.trim_start_matches('$').to_string();
                if !cell.is_empty() {
                    record(&cell, file, true, acc);
                }
                // Reads come from the RHS expression (`$n <- $n + 1` reads `n`).
                // Scan it VERBATIM: `Expr::to_js(Raw)` is an emission transform
                // that strips the leading `$`, which would hide the read here.
                let expr_text = match expr {
                    CapturedValue::Expr(e) => e.clone(),
                    other => other.to_js(crate::syntax::JsQuoting::Raw),
                };
                scan_text(&expr_text, file, acc, record);
                return;
            }
            for (k, v) in map {
                // A handler body's statements can arrive as `key: value` pairs
                // where the KEY carries the write target (`$session` -> `"ada"`).
                // Scan the key too, joined by the arrow so the write is seen.
                if k.trim_start().starts_with('$') {
                    let rhs = v.to_js(crate::syntax::JsQuoting::Raw);
                    scan_text(&format!("{k} <- {rhs}"), file, acc, record);
                } else {
                    collect_from_value(v, file, acc, record);
                }
            }
        }
        CapturedValue::Array(items) => {
            for item in items {
                collect_from_value(item, file, acc, record);
            }
        }
        CapturedValue::StyleProperties(pairs) => {
            for (_, v) in pairs {
                scan_text(v, file, acc, record);
            }
        }
        other => {
            let text = other.to_js(crate::syntax::JsQuoting::Raw);
            scan_text(&text, file, acc, record);
        }
    }
}

/// Extract the contents of every backtick HOLE in a run of markup.
///
/// In EXPRESSION text a backtick run is a quoted string and `collect_signal_deps`
/// rightly skips it. In MARKUP the backtick is the hole form — the one place a
/// `$cell` may be read from inside a template body — so the same skip would hide
/// every reader. Pull the hole bodies out first, then scan those as expressions.
fn markup_holes(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        match after.find('`') {
            Some(close) => {
                out.push(after[..close].to_string());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    out
}

/// Scan a chunk of expression / handler-body text for reads and writes.
fn scan_text(text: &str, file: &str, acc: &mut BTreeMap<String, Acc>, record: &mut Recorder<'_>) {
    for stmt in text.split(';') {
        match stmt.split_once("<-") {
            Some((lhs, rhs)) => {
                // Write target: the FIRST binding on the left of the arrow.
                // `$user.name <- v` writes the cell `user` (member assignment).
                if let Some(target) = collect_signal_deps(lhs).into_iter().next() {
                    record(&target, file, true, acc);
                }
                for dep in collect_signal_deps(rhs) {
                    record(&dep, file, false, acc);
                }
            }
            None => {
                for dep in collect_signal_deps(stmt) {
                    record(&dep, file, false, acc);
                }
            }
        }
    }
}

fn collect_from_scope(
    scope: &ScopeBlock,
    acc: &mut BTreeMap<String, Acc>,
    record: &mut Recorder<'_>,
    seen_spans: &mut std::collections::HashSet<(usize, usize)>,
) {
    let file = file_of(&scope.source_file);
    for cd in &scope.css_declarations {
        scan_text(&cd.value, &file, acc, record);
    }
    // A `@template` body's markup is a READ site too. FUP-094: a hole inside a
    // template resolves LEXICALLY — `` `$selected` `` reaches the page-global
    // cell from inside the instance — so the body's html is a genuine reader of
    // page-global state, not just of its own params.
    //
    // It lives in `ScopeKind::Construct`'s `html` (FEAT-119), a third reader
    // channel beside scope bindings and file-scope `html_blocks`. Missing it
    // eliminated a cell that a template was rendering, and the page showed an
    // empty span (caught by `fup094_template_hole_resolves_param_and_outer_signal`).
    if !scope.html.is_empty() {
        for hole in markup_holes(&scope.html) {
            scan_text(&hole, &file, acc, record);
        }
    }
    for fm in &scope.matches {
        if !seen_spans.insert((fm.span.start, fm.span.end)) {
            continue;
        }
        collect_from_match(fm, &file_of(&fm.source_file), acc, record);
    }
    for ns in &scope.nested_scopes {
        collect_from_nested(ns, &file, acc, record, seen_spans);
    }
}

fn collect_from_nested(
    ns: &NestedScope,
    file: &str,
    acc: &mut BTreeMap<String, Acc>,
    record: &mut Recorder<'_>,
    seen_spans: &mut std::collections::HashSet<(usize, usize)>,
) {
    for cd in &ns.css_declarations {
        scan_text(&cd.value, file, acc, record);
    }
    for fm in &ns.matches {
        if !seen_spans.insert((fm.span.start, fm.span.end)) {
            continue;
        }
        collect_from_match(fm, &file_of(&fm.source_file), acc, record);
    }
    for child in &ns.nested_scopes {
        collect_from_nested(child, file, acc, record, seen_spans);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(src: &str) -> Vec<StateProvenance> {
        let ast = crate::parse(src).expect("parse");
        analyze_state(&ast)
    }

    fn find<'a>(v: &'a [StateProvenance], name: &str) -> &'a StateProvenance {
        v.iter()
            .find(|p| p.var_name == name)
            .unwrap_or_else(|| panic!("no cell named {name} in {v:?}"))
    }

    #[test]
    fn write_free_literal_cell_is_const() {
        let v = analyze(
            "$host string: \"https://api.example.com\";\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
        );
        let host = find(&v, "host");
        assert_eq!(host.verdict, StateVerdict::Const);
        assert!(host.writers.is_empty(), "no writers: {:?}", host.writers);
        assert_eq!(host.readers.len(), 1, "one reading file");
    }

    #[test]
    fn written_cell_is_reactive() {
        let v = analyze(
            "$session string: \"anon\";\n<main><p class=\"x\"></p><button class=\"b\">go</button></main>\n.x { text <- $session; }\n.b { @on &.click { $session <- \"ada\"; } }\n",
        );
        let s = find(&v, "session");
        assert_eq!(s.verdict, StateVerdict::Reactive, "prov: {s:?}");
        assert!(!s.writers.is_empty(), "the @on handler writes it");
    }

    #[test]
    fn unread_cell_is_dead() {
        let v = analyze("$retryBudget number: 3;\n<main><p class=\"x\">hi</p></main>\n");
        assert_eq!(find(&v, "retryBudget").verdict, StateVerdict::Dead);
    }

    #[test]
    fn self_increment_counts_as_both_read_and_write() {
        // `$n <- $n + 1` is the classic counter: a write of `n` whose value
        // expression READS `n`. Both sides must be recorded, or a counter would
        // look write-only and be wrongly considered unread.
        let v = analyze(
            "$n number: 0;\n<main><button class=\"b\">+</button></main>\n.b { @on &.click { $n <- $n + 1; } }\n",
        );
        let n = find(&v, "n");
        assert!(!n.writers.is_empty(), "the mutation is a write");
        assert!(!n.readers.is_empty(), "the RHS is a read");
        assert_eq!(n.verdict, StateVerdict::Reactive);
    }

    #[test]
    fn expression_initial_stays_reactive_even_without_writers() {
        // Folding would require evaluating an arbitrary expression at build
        // time. Only literals are foldable; a computed initial stays a cell.
        let v = analyze(
            "$a number: 2;\n$b number: $a + 1;\n<main><p class=\"x\"></p></main>\n.x { text <- $b; }\n",
        );
        assert_eq!(find(&v, "b").verdict, StateVerdict::Reactive);
    }

    #[test]
    fn element_scoped_state_is_not_page_global() {
        // A selector matches 0..N elements, so `.card { $count … }` is plural by
        // construction and must not appear in a page-global provenance table.
        let v = analyze("<main><div class=\"card\"></div></main>\n.card { $count number: 0; }\n");
        assert!(
            v.iter().all(|p| p.var_name != "count"),
            "element-scoped state must be excluded: {v:?}"
        );
    }

    #[test]
    fn a_declaration_initial_is_not_a_read_of_itself() {
        // Guards the Dead verdict: if a declaration counted as its own reader,
        // nothing would ever be dead.
        let v = analyze("$x number: 3;\n<main><p>hi</p></main>\n");
        assert_eq!(find(&v, "x").verdict, StateVerdict::Dead);
    }

    #[test]
    fn duplicate_declarations_are_recorded_not_merged() {
        // The E0938 precondition: BOTH declaration sites survive into the
        // provenance so the diagnostic can name them.
        let v = analyze("$host string: \"a\";\n$host string: \"b\";\n<main></main>\n");
        assert_eq!(
            find(&v, "host").declared_in.len(),
            2,
            "both declarations must be recorded"
        );
    }

    // --- fold + eliminate ---------------------------------------------------

    #[test]
    fn dead_cell_declaration_is_removed() {
        let mut ast = crate::parse("$gone number: 3;\n<main><p>hi</p></main>\n").expect("parse");
        let prov = analyze_state(&ast);
        let removed = eliminate_dead_state(&mut ast, &prov);
        assert_eq!(removed, vec!["gone".to_string()]);
        assert!(
            analyze_state(&ast).is_empty(),
            "the declaration must be gone from the AST"
        );
    }

    #[test]
    fn a_read_cell_survives_elimination() {
        // The discriminating half: a pass that dropped every declaration would
        // satisfy the test above while destroying the page.
        let mut ast = crate::parse(
            "$kept string: \"v\";\n$gone number: 3;\n<main><p class=\"x\"></p></main>\n.x { text <- $kept; }\n",
        )
        .expect("parse");
        let prov = analyze_state(&ast);
        eliminate_dead_state(&mut ast, &prov);
        let after = analyze_state(&ast);
        assert!(
            after.iter().any(|p| p.var_name == "kept"),
            "read cell stays"
        );
        assert!(
            !after.iter().any(|p| p.var_name == "gone"),
            "dead cell goes"
        );
    }

    #[test]
    fn a_file_scope_markup_hole_counts_as_a_reader() {
        // Regression guard. `<li>`$x`</li>` renders the cell via a file-scope
        // hole (FEAT-078), but holes live on `html_blocks` — not in `matches`
        // or `scopes`. Walking only those two judged this cell DEAD and
        // eliminated it, emptying the hydration marker and turning two suites
        // red. A reader the analysis cannot see is the one way elimination can
        // delete a working page.
        let ast = crate::parse("$x number: 7;\n<li>`$x`</li>").expect("parse");
        let v = analyze_state(&ast);
        let x = find(&v, "x");
        assert!(!x.readers.is_empty(), "the hole is a read: {x:?}");
        assert_ne!(
            x.verdict,
            StateVerdict::Dead,
            "a cell read only by a markup hole must NOT be eliminated"
        );
    }

    #[test]
    fn a_template_body_hole_counts_as_a_reader() {
        // Regression guard, and the THIRD reader channel this pass had to learn.
        //
        // FUP-094: a hole inside a `@template` resolves LEXICALLY, so
        // `` `$selected` `` in a template body reads the page-global cell. That
        // body lives in `ScopeKind::Construct`'s `html` — neither in `matches`
        // nor in file-scope `html_blocks`.
        //
        // Two distinct misses had to be fixed to see it: the scope html was not
        // scanned at all, and then `collect_signal_deps` SKIPS backtick runs
        // (correct for expressions, where a backtick is a quoted string; wrong
        // for markup, where the backtick IS the hole). Caught by
        // `fup094_template_hole_resolves_param_and_outer_signal` rendering an
        // empty span.
        let ast = crate::parse(
            "$selected string: \"b\";\n@template &row($label) {\n  <li class=\"row\"><span class=\"sel\">`$selected`</span></li>\n}\n.list { &row(\"Beta\"); }\n",
        )
        .expect("parse");
        let v = analyze_state(&ast);
        let sel = find(&v, "selected");
        assert!(
            !sel.readers.is_empty(),
            "the template hole is a read: {sel:?}"
        );
        assert_ne!(
            sel.verdict,
            StateVerdict::Dead,
            "a cell read only from a template body must NOT be eliminated"
        );
    }

    #[test]
    fn markup_holes_are_extracted_from_a_body() {
        assert_eq!(
            markup_holes("<li><span>`$a`</span><b>`$b + 1`</b></li>"),
            vec!["$a".to_string(), "$b + 1".to_string()]
        );
        assert!(markup_holes("<p>no holes</p>").is_empty());
        // An unterminated backtick must not panic or loop.
        assert!(markup_holes("<p>`dangling</p>").is_empty());
    }

    #[test]
    fn fold_substitutes_on_token_boundaries_only() {
        // `$host` must not be substituted inside `$hostname`. Without the
        // boundary check the longer name silently reads the shorter one's value
        // — a bug that surfaces as mystery text far from its cause.
        let foldable = vec![("host".to_string(), "\"H\"".to_string())];
        let mut text = "$host + $hostname".to_string();
        fold_in_text(&mut text, &foldable);
        assert_eq!(text, "\"H\" + $hostname");
    }

    #[test]
    fn fold_leaves_a_member_access_intact() {
        let foldable = vec![("cfg".to_string(), "{a:1}".to_string())];
        let mut text = "$cfg.a".to_string();
        fold_in_text(&mut text, &foldable);
        assert_eq!(
            text, "{a:1}.a",
            "the member access the author wrote is kept"
        );
    }

    #[test]
    fn fold_reports_the_names_it_folded() {
        // NB `fold_const_state` is analysis-complete but NOT wired into the
        // compiler — substituting the reference token breaks the binding
        // registration that keys on it (see the note in src/compiler.rs). These
        // tests keep the pass honest and ready for the binding-aware rework.
        let mut ast = crate::parse(
            "$host string: \"https://x\";\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
        )
        .expect("parse");
        let prov = analyze_state(&ast);
        let folded = fold_const_state(&mut ast, &prov);
        assert_eq!(folded, vec!["host".to_string()]);
    }

    #[test]
    fn fold_leaves_a_written_cell_alone() {
        let mut ast = crate::parse(
            "$n number: 0;\n<main><p class=\"x\"></p><button class=\"b\">+</button></main>\n.x { text <- $n; }\n.b { @on &.click { $n <- $n + 1; } }\n",
        )
        .expect("parse");
        let prov = analyze_state(&ast);
        assert!(
            fold_const_state(&mut ast, &prov).is_empty(),
            "a cell with a writer is a REAL signal and must not be folded"
        );
    }

    #[test]
    fn a_sigil_handler_write_blocks_folding() {
        // Regression guard (PLAN-124 sigil cutover). A cell whose ONLY writer is
        // a `@on &.<driver> { … }` handler must stay REACTIVE and never be
        // const-folded: the mutation arrives as a DECOMPOSED `target`/`expr`
        // Named capture (no `<-` to scan), and if the writer walk misses it the
        // cell folds to its initial literal and the handler's write is silently
        // discarded — a working page compiled into a dead one.
        //
        // This exercises the FOLD path, not just the verdict field: folding
        // would report `session` as folded, and folding only happens when the
        // verdict wrongly came out Const.
        let src = "$session string: \"anon\";\n\
                   <main><p class=\"x\"></p><button class=\"b\">go</button></main>\n\
                   .x { text <- $session; }\n\
                   .b { @on &.click { $session <- \"ada\"; } }\n";
        let mut ast = crate::parse(src).expect("parse");
        let prov = analyze_state(&ast);
        let s = find(&prov, "session");
        assert_eq!(
            s.verdict,
            StateVerdict::Reactive,
            "a sigil-written cell must not be Const: {s:?}"
        );
        assert!(
            fold_const_state(&mut ast, &prov).is_empty(),
            "a cell written only by a sigil handler must NOT be const-folded"
        );
        assert!(
            eliminate_dead_state(&mut ast, &prov).is_empty(),
            "a read cell written by a sigil handler must NOT be dead-eliminated"
        );
    }

    #[test]
    fn literal_initial_detection() {
        assert!(is_literal_initial("\"hello\""));
        assert!(is_literal_initial("42"));
        assert!(is_literal_initial("3.5"));
        assert!(is_literal_initial("true"));
        assert!(!is_literal_initial("null"));
        assert!(!is_literal_initial(""));
        assert!(!is_literal_initial("$a + 1"));
    }
}
