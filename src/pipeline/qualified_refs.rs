//! Qualified cell references — `app/$host` (PLAN-117 W3).
//!
//! One reference shape across every registry:
//!
//! ```text
//! @b/badge        directive from module b     (FEAT-118, shipped)
//! %b/tokens       macro from module b         (FEAT-118, shipped)
//! app/$host       cell from module app        (this file)
//! ```
//!
//! The sigil selects the registry; `/` addresses within it; the sigil stays
//! glued to its leaf. `@`/`%` LEAD their path because their leaf IS the whole
//! invocation (`@scene/camera` is one registry name). `$` is a REFERENT, so the
//! qualifier sits in front and `$host` stays intact.
//!
//! # Why the qualifier goes in front
//!
//! `$app` is a valid OPERAND — it reads a cell — so `$app/host` is genuinely
//! ambiguous with division and no lexing rule settles it without whitespace
//! significance. `@app` is not an operand, which is why `@scene/camera` has been
//! safe since FEAT-118. Putting the qualifier in front restores that property:
//! **after `/`, a `$` never begins a divisor.**
//!
//! # The disambiguation rule
//!
//! > After `/`, a `$`-led leaf opens the QUALIFIED production **only when the
//! > head before the `/` is a bare identifier** and no trivia surrounds the `/`.
//!
//! A head that is an operand (`$x`, a number, a paren group) keeps `/` as
//! division. This mirrors the tight-adjacency rule already implemented for `@`
//! (`src/syntax/cst/parser.rs:636-652`, `ast.rs:283`: "Trivia ends the name
//! run").
//!
//! Counted evidence that this is safe: real division in expression position is
//! 4 lines in one file (all `/ 100`, spaced); division inside backtick holes is
//! ZERO; CSS slash values are 9 lines (all `font: 15px/1.6`, numeric head).
//!
//! # The one refusal
//!
//! `1/$denominator` — a TIGHT numeric head before a `$` leaf. A number can never
//! be a qualifier, so this IS division, but it reads exactly like the qualified
//! form. The compiler REFUSES (E0939) and names the fix rather than silently
//! picking. Refusing beats guessing.
//!
//! # How resolution works
//!
//! An alias is a COMPILE-TIME rename with zero runtime footprint (Elixir `alias`
//! semantics). This pass runs immediately after import resolution and rewrites
//! `app/$host` to plain `$host` in place, so every downstream consumer — emit,
//! analysis, fold, LSP — sees exactly what it would have seen had the author
//! written the bare reference in the owning file. Nothing downstream changes.

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::parser::ast::{ImportAst, NestedScope, ScopeBlock, StFile};
use crate::syntax::{CapturedValue, FormMatch};
use std::collections::HashMap;

/// Everything the rewrite needs to resolve and check ONE reference.
///
/// Bundled rather than passed as four parameters through five recursive walkers:
/// the walkers exist to reach every text position, and a signature that grows
/// with each new check obscures that.
struct Ctx<'a> {
    /// alias / module-name -> module path, from this file's `@use` entries.
    env: &'a HashMap<String, String>,
    /// Every page-global cell the assembled page declares.
    declared: &'a [String],
    /// Cells published by an `@exports` clause. EMPTY = nobody stated an
    /// interface, so everything is public and no visibility check runs.
    published: &'a [String],
}

/// A qualified reference found in source text.
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRef {
    /// Byte offset of the qualifier's first character.
    pub start: usize,
    /// Byte offset one past the `$leaf`.
    pub end: usize,
    /// The qualifier path (`app`, or `themes/dark` for a multi-segment path).
    pub qualifier: String,
    /// The cell name, without `$`.
    pub leaf: String,
}

/// Scan a chunk of expression text for qualified cell references.
///
/// Returns every `ident/$leaf` (or `a/b/$leaf`) whose head is a bare identifier
/// and whose slashes are tight. Quoted strings are skipped wholesale — a `/$`
/// inside a string literal is text, not an address.
pub fn scan_qualified_refs(text: &str) -> Vec<QualifiedRef> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i] as char;

        // Skip string literals wholesale.
        if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                i += 1;
                if ch == '\\' && i < bytes.len() {
                    i += 1;
                    continue;
                }
                if ch == quote {
                    break;
                }
            }
            continue;
        }

        // A qualified reference starts at an identifier head. It must be a
        // WORD BOUNDARY: `foo.bar/$x` must not treat `bar` as a qualifier, and
        // `$a/$b` must not treat `a` as one (that is division of two cells).
        if is_ident_start(c) && (i == 0 || !is_ident_continue_or_sigil(bytes[i - 1] as char)) {
            if let Some(found) = try_parse_at(text, bytes, i) {
                i = found.end;
                out.push(found);
                continue;
            }
        }

        i += 1;
    }

    out
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// A preceding char that would make this identifier a CONTINUATION rather than a
/// fresh head: an identifier char, a `.` (member access), or a sigil.
fn is_ident_continue_or_sigil(c: char) -> bool {
    is_ident_continue(c) || c == '.' || c == '$' || c == '@' || c == '%' || c == '&'
}

/// Try to parse `ident(/ident)*/$leaf` starting at `start`. All slashes must be
/// TIGHT (no surrounding trivia) — a spaced slash is division.
fn try_parse_at(text: &str, bytes: &[u8], start: usize) -> Option<QualifiedRef> {
    let mut i = start;
    let mut segments: Vec<String> = Vec::new();

    loop {
        // One identifier segment.
        let seg_start = i;
        while i < bytes.len() && is_ident_continue(bytes[i] as char) {
            i += 1;
        }
        if i == seg_start {
            return None;
        }

        // Must be followed IMMEDIATELY by `/` — no trivia.
        if i >= bytes.len() || bytes[i] != b'/' {
            return None;
        }
        segments.push(text[seg_start..i].to_string());
        i += 1; // consume '/'

        if i >= bytes.len() {
            return None;
        }

        // `$leaf` ends the path; another identifier continues it.
        if bytes[i] == b'$' {
            let leaf_start = i + 1;
            let mut j = leaf_start;
            while j < bytes.len() && is_ident_continue(bytes[j] as char) {
                j += 1;
            }
            if j == leaf_start {
                return None;
            }
            return Some(QualifiedRef {
                start,
                end: j,
                qualifier: segments.join("/"),
                leaf: text[leaf_start..j].to_string(),
            });
        }

        if !is_ident_start(bytes[i] as char) {
            return None;
        }
    }
}

/// Detect the AMBIGUOUS shape the design deliberately refuses: a tight `/$`
/// whose head is a NUMBER (`1/$denominator`).
///
/// A number can never be a qualifier, so this is division — but it reads like
/// the qualified form, and silently picking either reading is the failure mode
/// this whole design exists to avoid. Returns the offending source snippets.
pub fn scan_ambiguous_slash(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                i += 1;
                if ch == '\\' && i < bytes.len() {
                    i += 1;
                    continue;
                }
                if ch == quote {
                    break;
                }
            }
            continue;
        }

        if c.is_ascii_digit() && (i == 0 || !is_ident_continue_or_sigil(bytes[i - 1] as char)) {
            let num_start = i;
            while i < bytes.len() && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            // Tight `/$` immediately after the number.
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'$' {
                let mut j = i + 2;
                while j < bytes.len() && is_ident_continue(bytes[j] as char) {
                    j += 1;
                }
                if j > i + 2 {
                    out.push(text[num_start..j].to_string());
                    i = j;
                    continue;
                }
            }
            continue;
        }

        i += 1;
    }

    out
}

/// The file-local alias environment: `alias -> module path`, built from the
/// `@use` entries import resolution retained.
///
/// Both spellings resolve, mirroring how `@scene/camera` works under an open
/// `@use "std:scene"`:
///   - the explicit alias (`@use "./config.st" as app` -> `app`)
///   - the module's own NAME (`./config.st` -> `config`), which is the tail of
///     the path the author already typed at the import site (Odin convention:
///     `import "core:fmt"` -> `fmt.println`).
pub fn build_alias_env(imports: &[ImportAst]) -> HashMap<String, String> {
    let mut env = HashMap::new();
    for imp in imports {
        let module_name = module_name_of(&imp.path);
        if let Some(alias) = &imp.alias {
            env.insert(alias.clone(), imp.path.clone());
        }
        if !module_name.is_empty() {
            env.entry(module_name).or_insert_with(|| imp.path.clone());
        }
    }
    env
}

/// The module NAME for an import path: the file stem, or the folder name for a
/// directory import. `"./config.st"` -> `config`; `"local:app/state"` -> `state`.
fn module_name_of(path: &str) -> String {
    let tail = path.rsplit('/').next().unwrap_or(path);
    tail.strip_suffix(".st")
        .unwrap_or(tail)
        .rsplit(':')
        .next()
        .unwrap_or(tail)
        .to_string()
}

/// Rewrite every qualified reference in the page to its bare form, and report
/// what could not be resolved.
///
/// Runs immediately after import resolution, so the rewrite is invisible to
/// every downstream consumer: emit, analysis, fold and LSP all see exactly the
/// text the author would have written inside the owning file. That is the whole
/// content of "an alias is a compile-time rename with zero runtime footprint".
pub fn resolve_qualified_refs(ast: &mut StFile) -> Vec<Diagnostic> {
    let alias_env = build_alias_env(&ast.imports);
    // Every cell the page declares — the set a qualified reference may name.
    let declared: Vec<String> = ast
        .matches
        .iter()
        .filter(|m| {
            (m.macro_name == "local-state" || m.macro_name == "local-state-uninitialized")
                && m.selector.is_none()
        })
        .filter_map(|m| m.get_ident("name").map(|s| s.to_string()))
        .collect();

    // PLAN-117 W5: the page's published set, from every `@exports` clause in the
    // import graph. EMPTY means nobody stated an interface, so everything is
    // public (the Odin floor) and no visibility check runs — which is exactly
    // what keeps every pre-existing file working unchanged.
    // Harvested from the `@exports` FormMatch (the registry path a stdlib macro
    // produces), not from a bespoke parser branch: the clause is a `%macro` like
    // any other, so it arrives as a match with a `body` capture.
    let mut published: Vec<String> = Vec::new();
    for fm in ast.matches.iter().filter(|m| m.macro_name == "exports") {
        let body = fm
            .captures
            .get("body")
            .map(|v| v.to_js(crate::syntax::JsQuoting::Raw))
            .unwrap_or_default();
        for entry in body.split([',', ';']) {
            // `$name` or `$name: mut` — the `: mut` marker is FEAT-115's, reused
            // verbatim so the file-scope clause reads like the template one.
            let name = entry
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches('$')
                .trim();
            if !name.is_empty() {
                published.push(name.to_string());
            }
        }
    }
    published.extend(ast.file_exports.iter().map(|e| e.var_name.clone()));

    let mut diags = Vec::new();

    let ctx = Ctx {
        env: &alias_env,
        declared: &declared,
        published: &published,
    };

    let mut matches = std::mem::take(&mut ast.matches);
    for fm in &mut matches {
        rewrite_match(fm, &ctx, &mut diags);
    }
    ast.matches = matches;

    let mut scopes = std::mem::take(&mut ast.scopes);
    for scope in &mut scopes {
        rewrite_scope(scope, &ctx, &mut diags);
    }
    ast.scopes = scopes;

    // MARKUP HOLES are the third reference channel, and they are a READ site
    // just as much as `text <- app/$host` is:
    //
    //     <p>`app/$host`</p>
    //
    // They live on `ast.html_blocks` (FEAT-078) — not in `matches`, not in
    // `scopes`. Skipping them meant a qualified read in a hole was never
    // resolved AND never checked: `app/$internal` silently read an unpublished
    // cell (no E0944) and `nope/$host` silently resolved nothing (no E0940),
    // while the identical reference in a scope binding was correctly refused.
    //
    // This is the SAME channel the state analysis had to learn twice. Any pass
    // that reasons about `$` references must walk all three.
    let mut blocks = std::mem::take(&mut ast.html_blocks);
    for block in &mut blocks {
        for hole in &mut block.holes {
            rewrite_text(hole, &ctx, &mut diags);
        }
    }
    ast.html_blocks = blocks;

    dedupe(diags)
}

/// Scan a directive NAME for a colon used where a slash belongs (`@b:badge`).
///
/// BUG-232: the CST lexer terminates a directive-name run at `:`, so `@b:badge`
/// matched no `%form` and was DROPPED — silently. Verified by differential
/// build: `@b/badge` emits `.badge::before` into spacetime.css; `@b:badge`
/// reports success and emits an empty stylesheet.
///
/// `:` belongs inside a module ADDRESS, and a module address only ever appears
/// inside a quoted import string (`@use "local:config"`). It never enters
/// reference position, where six other roles already claim it.
fn colon_qualifier_in_name(name: &str) -> Option<(String, String)> {
    let (head, leaf) = name.split_once(':')?;
    if head.is_empty() || leaf.is_empty() {
        return None;
    }
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    };
    if ok(head) && ok(leaf) {
        Some((head.to_string(), leaf.to_string()))
    } else {
        None
    }
}

/// True when the reference ending at `end` is the TARGET of a `<-` write.
///
/// Only trivia may sit between the reference and the arrow: `app/$x <- v` is a
/// write, but `app/$x + 1 <- v` is not this reference's write (and would be
/// nonsense anyway).
fn is_write_target(text: &str, end: usize) -> bool {
    text[end..].trim_start().starts_with("<-")
}

/// Collapse diagnostics that describe the SAME mistake.
///
/// A directive inside a scope is reachable from both `ast.matches` (the flat
/// list) and `ast.scopes` (the tree), and a css_declaration carrying a reference
/// is reachable from the scope tree as well — so ONE bad reference can yield the
/// same diagnostic two or three times. Reporting a mistake more than once trains
/// authors to skim errors, which is how real errors get missed.
fn dedupe(mut diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut seen = std::collections::HashSet::new();
    diags.retain(|d| seen.insert(format!("{}|{}", d.code.as_str(), d.message)));
    diags
}

fn rewrite_match(fm: &mut FormMatch, ctx: &Ctx<'_>, diags: &mut Vec<Diagnostic>) {
    // BUG-232: `@b:badge` — a colon where a slash belongs. The name run ends at
    // the `:`, so the directive matches no form and vanishes without a word.
    if let Some((head, leaf)) = colon_qualifier_in_name(&fm.macro_name) {
        diags.push(
            Diagnostic::error(
                DiagnosticCode::E0943,
                format!("`@{head}:{leaf}` uses `:` where a module qualifier needs `/`"),
            )
            .with_hint(format!(
                "write `@{head}/{leaf}`. `:` belongs inside a module address in an \
                 import string (`@use \"local:{head}\"`); in a reference, `/` \
                 addresses within a registry."
            )),
        );
    }

    for value in fm.captures.values_mut() {
        rewrite_value(value, ctx, diags);
    }
}

fn rewrite_value(value: &mut CapturedValue, ctx: &Ctx<'_>, diags: &mut Vec<Diagnostic>) {
    match value {
        CapturedValue::Block(children) => {
            for child in children {
                rewrite_match(child, ctx, diags);
            }
        }
        CapturedValue::Array(items) => {
            for item in items {
                rewrite_value(item, ctx, diags);
            }
        }
        CapturedValue::Named(map) => {
            // A handler body arrives as key -> value pairs, with the WRITE
            // TARGET as the key and the arrow already consumed by the parser.
            // So a qualified ref in key position is a write even though no `<-`
            // survives in the text — append one so the shared rewrite sees the
            // same shape it would in raw handler text.
            let rewritten: Vec<(String, CapturedValue)> = map
                .iter()
                .map(|(k, v)| {
                    let mut probe = format!("{k} <-");
                    rewrite_text(&mut probe, ctx, diags);
                    let nk = probe
                        .trim_end()
                        .trim_end_matches("<-")
                        .trim_end()
                        .to_string();
                    (nk, v.clone())
                })
                .collect();
            map.clear();
            for (k, mut v) in rewritten {
                rewrite_value(&mut v, ctx, diags);
                map.insert(k, v);
            }
        }
        CapturedValue::StyleProperties(pairs) => {
            for (_, v) in pairs.iter_mut() {
                rewrite_text(v, ctx, diags);
            }
        }
        CapturedValue::Expr(e) => rewrite_text(e, ctx, diags),
        CapturedValue::String(s) => rewrite_text(s, ctx, diags),
        CapturedValue::Binding(b) => rewrite_text(b, ctx, diags),
        _ => {}
    }
}

fn rewrite_scope(scope: &mut ScopeBlock, ctx: &Ctx<'_>, diags: &mut Vec<Diagnostic>) {
    for cd in &mut scope.css_declarations {
        rewrite_text(&mut cd.value, ctx, diags);
    }
    for fm in &mut scope.matches {
        rewrite_match(fm, ctx, diags);
    }
    for ns in &mut scope.nested_scopes {
        rewrite_nested(ns, ctx, diags);
    }
}

fn rewrite_nested(ns: &mut NestedScope, ctx: &Ctx<'_>, diags: &mut Vec<Diagnostic>) {
    for cd in &mut ns.css_declarations {
        rewrite_text(&mut cd.value, ctx, diags);
    }
    for fm in &mut ns.matches {
        rewrite_match(fm, ctx, diags);
    }
    for child in &mut ns.nested_scopes {
        rewrite_nested(child, ctx, diags);
    }
}

/// Rewrite one chunk of text in place, emitting a diagnostic per unresolvable
/// reference. Rewrites back-to-front so earlier offsets stay valid.
fn rewrite_text(text: &mut String, ctx: &Ctx<'_>, diags: &mut Vec<Diagnostic>) {
    for snippet in scan_ambiguous_slash(text) {
        let spaced = snippet.replacen("/$", " / $", 1);
        diags.push(
            Diagnostic::error(
                DiagnosticCode::E0939,
                format!("`{snippet}` is ambiguous: a tight `/$` reads as a registry address"),
            )
            .with_hint(format!(
                "a number cannot be a qualifier, so this must be division — write `{spaced}`. \
                 (A qualified cell reference looks like `app/$host`, where the head names a module.)"
            )),
        );
    }

    let refs = scan_qualified_refs(text);
    if refs.is_empty() {
        return;
    }

    for r in refs.iter().rev() {
        // A qualified WRITE (`app/$session <- v`) is refused. Reads may cross a
        // module boundary; writes stay in the owning file, which is exactly what
        // keeps a cell's write-set settleable by a ONE-FILE scan and therefore
        // keeps const-folding sound.
        if is_write_target(text, r.end) {
            diags.push(
                Diagnostic::error(
                    DiagnosticCode::E0942,
                    format!(
                        "cannot write `${}` through the qualifier `{}`",
                        r.leaf, r.qualifier
                    ),
                )
                .with_hint(format!(
                    "`${}` is owned by the module `{}` — reads may cross a module \
                     boundary, writes may not. Let the owner mutate its own cell: \
                     emit an event here (`@emit {}-changed(…)`) and write `${}` in \
                     the file that declares it.",
                    r.leaf, r.qualifier, r.leaf, r.leaf
                )),
            );
            continue;
        }

        // The qualifier's LAST segment is the alias/module name; leading
        // segments are the namespace path (`themes/dark/$x`).
        let head = r.qualifier.rsplit('/').next().unwrap_or(&r.qualifier);

        if !ctx.env.contains_key(head) {
            let mut known: Vec<&str> = ctx.env.keys().map(|s| s.as_str()).collect();
            known.sort();
            diags.push(
                Diagnostic::error(
                    DiagnosticCode::E0940,
                    format!(
                        "`{}` is not a known module qualifier in `{}/${}`",
                        head, r.qualifier, r.leaf
                    ),
                )
                .with_hint(if known.is_empty() {
                    "no modules are in scope here — add `@use \"./module.st\" as <alias>`"
                        .to_string()
                } else {
                    format!("qualifiers in scope: {}", known.join(", "))
                }),
            );
            continue;
        }

        if !ctx.declared.is_empty() && !ctx.declared.contains(&r.leaf) {
            let mut names: Vec<&str> = ctx.declared.iter().map(|s| s.as_str()).collect();
            names.sort();
            diags.push(
                Diagnostic::error(
                    DiagnosticCode::E0941,
                    format!(
                        "`{}` does not publish a cell named `${}`",
                        r.qualifier, r.leaf
                    ),
                )
                .with_hint(format!(
                    "page-global cells in scope: {}",
                    names
                        .iter()
                        .map(|n| format!("${n}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            );
            continue;
        }

        // W5: the owner's interface, when it stated one. An EMPTY published set
        // means no module on this page declared `@exports`, so everything is
        // public (the Odin floor) and this check does not run — which is what
        // keeps every pre-existing file working unchanged.
        if !ctx.published.is_empty() && !ctx.published.contains(&r.leaf) {
            let mut names: Vec<&str> = ctx.published.iter().map(|s| s.as_str()).collect();
            names.sort();
            diags.push(
                Diagnostic::error(
                    DiagnosticCode::E0944,
                    format!("`{}` does not publish `${}`", r.qualifier, r.leaf),
                )
                .with_hint(format!(
                    "published cells: {}. Add `${}` to the owning file's \
                     `@exports {{ … }}` clause to make it readable here.",
                    names
                        .iter()
                        .map(|n| format!("${n}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    r.leaf
                )),
            );
            continue;
        }

        // Resolved: rewrite to the bare form. Byte-identical to what the owning
        // file would have emitted.
        text.replace_range(r.start..r.end, &format!("${}", r.leaf));
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn qualifiers(text: &str) -> Vec<(String, String)> {
        scan_qualified_refs(text)
            .into_iter()
            .map(|r| (r.qualifier, r.leaf))
            .collect()
    }

    #[test]
    fn recognises_a_simple_qualified_reference() {
        assert_eq!(
            qualifiers("app/$host"),
            vec![("app".to_string(), "host".to_string())]
        );
    }

    #[test]
    fn recognises_a_multi_segment_qualifier() {
        assert_eq!(
            qualifiers("themes/dark/$theme"),
            vec![("themes/dark".to_string(), "theme".to_string())]
        );
    }

    // --- the division guard: every one of these must find NOTHING -----------

    #[test]
    fn spaced_slash_between_bindings_is_division() {
        assert!(qualifiers("$total / $count").is_empty());
    }

    #[test]
    fn spaced_slash_with_numeric_head_is_division() {
        // The case named in design review: `1 / $denominator`.
        assert!(qualifiers("1 / $denominator").is_empty());
    }

    #[test]
    fn tight_slash_after_a_binding_is_division() {
        // `$total/$part` — the head is an OPERAND, not a bare identifier, so
        // the qualified production must not open. This is precisely why the
        // qualifier goes in front rather than after the sigil.
        assert!(qualifiers("$total/$part").is_empty());
    }

    #[test]
    fn tight_numeric_slash_is_not_a_qualifier() {
        assert!(qualifiers("9/3").is_empty());
        assert!(qualifiers("1/$denominator").is_empty());
    }

    #[test]
    fn css_slash_values_are_untouched() {
        assert!(qualifiers("15px/1.6").is_empty());
        assert!(qualifiers("1 / 3").is_empty());
        assert!(qualifiers("16/9").is_empty());
    }

    #[test]
    fn member_access_head_is_not_a_qualifier() {
        // `item.price/$x` — `price` follows a `.`, so it is a member, not a head.
        assert!(qualifiers("item.price/$x").is_empty());
    }

    #[test]
    fn a_reference_inside_a_string_is_text() {
        assert!(qualifiers("\"app/$host\"").is_empty());
    }

    #[test]
    fn finds_a_reference_amongst_other_text() {
        assert_eq!(
            qualifiers("prefix app/$host suffix"),
            vec![("app".to_string(), "host".to_string())]
        );
    }

    // --- the deliberate refusal --------------------------------------------

    #[test]
    fn tight_numeric_head_is_flagged_ambiguous() {
        assert_eq!(
            scan_ambiguous_slash("1/$denominator"),
            vec!["1/$denominator"]
        );
    }

    #[test]
    fn spaced_numeric_head_is_not_flagged() {
        assert!(scan_ambiguous_slash("1 / $denominator").is_empty());
    }

    #[test]
    fn a_qualified_reference_is_not_flagged_ambiguous() {
        assert!(scan_ambiguous_slash("app/$host").is_empty());
    }

    // --- alias environment --------------------------------------------------

    #[test]
    fn module_names_derive_from_the_import_path() {
        assert_eq!(module_name_of("./config.st"), "config");
        assert_eq!(module_name_of("local:app/state"), "state");
        assert_eq!(module_name_of("stdlib/3d"), "3d");
    }
}
