//! Syntax migrations (PLAN-076) — the ONE rewrite engine.
//!
//! Old syntax → new syntax, driven by stdlib `%migration` entries (registry
//! DATA, never per-retirement Rust). This module holds the shared mechanics:
//!
//! - `apply_migration_shim` — the compile-time COMPAT SHIM: rewrite-kind
//!   matches are rewritten IN MEMORY (with a W0715 diagnostic per match) so
//!   old syntax keeps compiling. Runs in `Compiler::from_file` (the one place
//!   holding source + AST + registry).
//! - `read_syntax_version` — the project's `@version <date>;` fact: waves at
//!   or before it are INERT for this project.
//! - `instantiate_template` — `%rewrite` template + capture slices → text.
//!
//! The pill's persisted apply and `spacetime migrate` (W3) run the SAME
//! functions over on-disk sources — no parallel implementations.
//!
//! Soundness rule: a rewrite-kind match is only rewritten when the call's
//! arg NAMES are a subset of the `%match` form's params — a call carrying
//! MORE args than the form claims (e.g. multi-shape `@bind(text: x, class:
//! c)` against the single-shape `@bind(text:)` migration) would silently
//! lose the unclaimed args when the whole directive span is replaced. Those
//! degrade to E0910 with a split-or-migrate-manually hint.

use std::collections::HashSet;

use crate::diagnostics::{Diagnostic, DiagnosticCode, SourceSpan as DiagSpan};
use crate::metasystem::MetaRegistry;
use crate::parser::StFile;
use crate::parser::meta_ast::{FormClause, MigrationDefAst, MigrationRewriteAst};
use crate::syntax::FormMatch;

/// A migration hit recorded on the compile output — the single source the
/// pill status route, `check`, and the CLI read (no second scanner).
#[derive(Debug, Clone)]
pub struct PendingMigration {
    /// Owning migration id
    pub migration_id: String,
    /// The rewrite rule that produced this row (`None` = a manual row from
    /// an embedded-macro match — hint-covered or rule-less shape)
    pub rule: Option<String>,
    /// Wave date (YYYY-MM-DD)
    pub date: String,
    /// `%docs` line (for the pill panel / check output)
    pub docs: String,
    /// Span of the matched (old-syntax) directive in the CURRENT on-disk source
    pub span: DiagSpan,
    /// The matched directive's source text
    pub old_text: String,
    /// The instantiated rewrite (automatic rows only)
    pub new_text: Option<String>,
    /// The `%hint` text (manual rows, or degraded multi-shape rewrites)
    pub hint: Option<String>,
}

impl PendingMigration {
    /// Rewrite-kind (auto-appliable) vs hint/manual row.
    pub fn is_automatic(&self) -> bool {
        self.new_text.is_some()
    }
}

/// Outcome of shimming one source file.
pub struct ShimOutcome {
    /// The (possibly rewritten) source.
    pub source: String,
    /// Parse of `source` after the final iteration.
    pub ast: StFile,
    /// W0715 diagnostics (one per applied match) + any E0912 cycle error.
    pub diagnostics: Vec<Diagnostic>,
    /// Pending entries for matches present in the ORIGINAL on-disk source
    /// (iteration 0) — i.e. exactly what a persisted apply will change.
    pub pending: Vec<PendingMigration>,
}

/// A read of the project's `@version` fact (PLAN-076).
#[derive(Debug, Default, Clone)]
pub struct VersionFact {
    /// Normalized `YYYY-MM-DD` (None = undeclared = date-zero)
    pub version: Option<String>,
    /// Number of `@version` matches seen
    pub decls: usize,
    /// Number of DISTINCT files those matches came from (post-import merge;
    /// >1 means `@version` leaked out of the root entry — an E0911 case)
    pub files: usize,
}

/// Strict ISO `YYYY-MM-DD` shape (valid month/day ranges; lexicographic ==
/// chronological). Shared by `%date` validation (metasystem) and `@version`
/// validation (compiler) — ONE definition.
pub fn is_iso_wave_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
        && (1..=12).contains(&parts[1].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&parts[2].parse::<u32>().unwrap_or(0))
}

/// Read the project's `@version <date>;` fact from parsed matches.
///
/// The `@version` directive is an ordinary stdlib `%macro` form
/// (`stdlib/syntax/version.st`); its `$date:expr` capture carries the date
/// text. Whitespace-insensitive (`2026 -06 -09` token splitting, same
/// normalization as `%date`). See `VersionFact` for the duplicate/non-root
/// counts the callers turn into E0911s.
pub fn read_syntax_version(matches: &[FormMatch]) -> VersionFact {
    let mut fact = VersionFact::default();
    let mut files = HashSet::new();
    for fm in matches {
        if fm.macro_name != "version" {
            continue;
        }
        fact.decls += 1;
        files.insert(fm.source_file.clone());
        let raw = fm
            .captures
            .get("date")
            .map(|v| match v {
                crate::syntax::CapturedValue::Expr(s)
                | crate::syntax::CapturedValue::Ident(s)
                | crate::syntax::CapturedValue::String(s) => s.clone(),
                other => format!("{:?}", other),
            })
            .unwrap_or_default();
        let normalized: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        if fact.version.is_none() {
            fact.version = Some(normalized);
        }
    }
    fact.files = files.len();
    fact
}

/// Instantiate a `%rewrite` template against a match: every backtick-quoted
/// hole (`` `$name` `` — THE hole form) splices the capture's text. For a
/// STRING-LITERAL capture the splice is its semantic value (unquoted) —
/// `@bind(attr: "src")` migrating to `src <- …` wants the bare name, not a
/// quoted literal. Every other capture splices its RAW SOURCE SLICE (via
/// `capture_spans`), verbatim. A bare `$name` in the template is literal
/// output. Unknown holes are left as-is (validated at registration, so
/// unreachable for stdlib entries; defensive for user-defined ones).
///
/// (see `reindent_splice` below for how a multi-line capture is aligned)

/// Re-indent a spliced block to the column the hole sits at.
///
/// A capture's source span starts at its first TOKEN, so the first line of a
/// multi-line block carries no leading whitespace while later lines keep their
/// original indentation verbatim. Splicing that in raw lands line 1 at the
/// template's column and everything after it at the ORIGINAL depth — which is
/// how a migrated body ended up with its opening line flush against column 0
/// and its siblings still indented (BUG-337).
///
/// So: shift the whole block as a unit. Take the common indentation of the
/// continuation lines (line 1 has none to contribute), strip it, and re-apply
/// the hole's own column to every line. A single-line splice is returned
/// untouched — there is nothing to align it against.
fn reindent_splice(text: &str, column: usize) -> String {
    if !text.contains('\n') {
        return text.to_string();
    }
    let mut lines = text.lines();
    let Some(first) = lines.next() else {
        return text.to_string();
    };
    let rest: Vec<&str> = lines.collect();

    // Common leading whitespace across the non-blank continuation lines: the
    // block's original depth, which we are about to replace wholesale.
    let base = rest
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    let pad = " ".repeat(column);
    let mut out = String::with_capacity(text.len() + rest.len() * column);
    out.push_str(first.trim_start());
    for line in rest {
        out.push('\n');
        if line.trim().is_empty() {
            continue; // keep blank lines blank rather than padding them
        }
        out.push_str(&pad);
        // Preserve depth RELATIVE to the block: a nested line stays nested,
        // but the block as a whole now hangs from the hole's column. Line 1
        // contributed no indentation (its span starts at the token), so `base`
        // comes from the continuation lines and is stripped from them here —
        // leaving every line of the block sharing exactly one origin.
        let own = line.len() - line.trim_start().len();
        out.push_str(&line[base.min(own)..]);
    }
    out
}

/// Column of the current output line — how far past the last newline we are.
fn current_column(out: &str) -> usize {
    match out.rfind('\n') {
        Some(nl) => out.len() - nl - 1,
        None => out.len(),
    }
}

pub fn instantiate_template(template: &str, fm: &FormMatch, source: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        if let Some(hole) = rest.strip_prefix("`$") {
            // Candidate hole: `` `$name` `` — ASCII ident then a closing backtick.
            let name_len = hole
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            if name_len > 0 && hole[name_len..].starts_with('`') {
                let name = &hole[..name_len];
                if let Some(crate::syntax::CapturedValue::String(s)) = fm.captures.get(name) {
                    // A String capture can still be a multi-line BLOCK (a
                    // keyframes body arrives here), so it needs the same
                    // alignment as the span path — this branch is checked first
                    // and was splicing raw, which is where BUG-337's ragged
                    // first line actually came from.
                    let col = current_column(&out);
                    out.push_str(&reindent_splice(s, col));
                } else if let Some(span) = fm.capture_spans.get(name) {
                    let col = current_column(&out);
                    out.push_str(&reindent_splice(&source[span.start..span.end], col));
                } else if let Some(value) = fm.captures.get(name) {
                    // Span-less captures: defaults the matcher FILLED (no
                    // source slice exists) and structured captures (the
                    // body). Render them back to surface text — the cutover's
                    // templates splice defaulted params and whole keyframes
                    // bodies (W3.4), which the String/span-only paths cannot
                    // reach.
                    match value {
                        crate::syntax::CapturedValue::String(s)
                        | crate::syntax::CapturedValue::Ident(s)
                        | crate::syntax::CapturedValue::Expr(s) => out.push_str(s),
                        crate::syntax::CapturedValue::Number(n) => {
                            if n.fract() == 0.0 {
                                out.push_str(&format!("{}", *n as i64));
                            } else {
                                out.push_str(&format!("{n}"));
                            }
                        }
                        crate::syntax::CapturedValue::Bool(b) => {
                            out.push_str(if *b { "true" } else { "false" });
                        }
                        // Time captures are stored as MILLISECONDS (unit
                        // normalized at capture). Render back with an
                        // unambiguous unit — a defaulted `:time` param
                        // (e.g. `@load reveal { }` → 1000ms) must NOT fall
                        // through to the verbatim-hole fallback, which would
                        // leave an unresolved ` `$duration` ` in the output.
                        crate::syntax::CapturedValue::Time(ms) => {
                            out.push_str(&format!("{ms}ms"));
                        }
                        crate::syntax::CapturedValue::Keyframes(kfs) => {
                            // Indent to the hole's own column so a spliced block
                            // sits at its nesting depth. A hardcoded width put
                            // every keyframe at a fixed 8 columns regardless of
                            // where the `%into` placed the hole (BUG-337).
                            let pad = " ".repeat(current_column(&out));
                            for (i, k) in kfs.iter().enumerate() {
                                if i > 0 {
                                    out.push('\n');
                                    out.push_str(&pad);
                                }
                                out.push_str(&format!(
                                    "{}: {};",
                                    k.property,
                                    k.values.join(" -> ")
                                ));
                            }
                        }
                        // Raw statement runs (e.g. a mutation_actions body):
                        // rejoin as statements. MOTION-BODY items (lines,
                        // splices, mutations, scopes — the on_motion_body
                        // capture) render back to surface text.
                        crate::syntax::CapturedValue::Array(items) => {
                            if items.iter().any(|i| matches!(i, crate::syntax::CapturedValue::Named(m) if m.contains_key("line") || m.contains_key("splice") || m.contains_key("mut") || m.contains_key("scope") || m.contains_key("sel"))) {
                                out.push_str(&render_motion_body(items, 8));
                            } else {
                                let texts: Vec<&str> = items
                                    .iter()
                                    .filter_map(|i| match i {
                                        crate::syntax::CapturedValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .collect();
                                out.push_str(&texts.join(";\n    "));
                            }
                        }
                        _ => {
                            // Unknown shape: emit verbatim (never silently drop).
                            out.push_str("`$");
                            out.push_str(name);
                            out.push('`');
                        }
                    }
                } else {
                    // Unknown hole: emit verbatim (load-time validation should
                    // have caught this; never silently drop).
                    out.push_str("`$");
                    out.push_str(name);
                    out.push('`');
                }
                i += 2 + name_len + 1;
                continue;
            }
        }
        // Plain template text — char-safe (templates may carry non-ASCII).
        let ch = rest
            .chars()
            .next()
            .expect("i < template.len() checked above");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Render an on_motion_body capture back to surface text (the cutover's
/// `$body` splice): lines, splices, mutations, and nested scopes.
fn render_motion_body(items: &[crate::syntax::CapturedValue], indent: usize) -> String {
    use crate::syntax::CapturedValue;
    let pad = " ".repeat(indent);
    let mut out = String::new();
    let text = |v: Option<&CapturedValue>| -> String {
        match v {
            Some(CapturedValue::String(s))
            | Some(CapturedValue::Ident(s))
            | Some(CapturedValue::Expr(s)) => s.clone(),
            _ => String::new(),
        }
    };
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let CapturedValue::Named(entry) = item else {
            continue;
        };
        if let Some(CapturedValue::Named(line)) = entry.get("line") {
            out.push_str(&format!(
                "{pad}{}: {};",
                text(line.get("prop")),
                text(line.get("value"))
            ));
        } else if let Some(CapturedValue::Named(splice)) = entry.get("splice") {
            let form_map = match splice.get("form") {
                Some(CapturedValue::Named(m)) => m.clone(),
                _ => splice.clone(),
            };
            let name = text(form_map.get("form"));
            let args = match form_map.get("args") {
                Some(CapturedValue::Array(args)) if !args.is_empty() => {
                    let parts: Vec<String> = args
                        .iter()
                        .filter_map(|a| {
                            if let CapturedValue::Named(arg) = a {
                                let value = text(arg.get("value"));
                                if value.is_empty() {
                                    None
                                } else if let Some(n) =
                                    arg.get("name").and_then(|n| text_of_capture(n))
                                {
                                    Some(format!("{n}: {value}"))
                                } else {
                                    Some(value)
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    format!("({})", parts.join(", "))
                }
                _ => String::new(),
            };
            out.push_str(&format!("{pad}{name}{args};"));
        } else if let Some(CapturedValue::Named(mutation)) = entry.get("mut") {
            out.push_str(&format!(
                "{pad}${} <- {};",
                text(mutation.get("target")),
                text(mutation.get("expr"))
            ));
        } else if let Some(CapturedValue::Array(nested)) = entry.get("scope") {
            let sel = entry
                .get("sel")
                .and_then(text_of_capture)
                .unwrap_or_else(|| "&".to_string());
            out.push_str(&format!("{pad}{sel} {{\n"));
            out.push_str(&render_motion_body(nested, indent + 4));
            out.push_str(&format!("\n{pad}}}\n"));
        }
    }
    out
}

fn text_of_capture(v: &crate::syntax::CapturedValue) -> Option<String> {
    match v {
        crate::syntax::CapturedValue::String(s)
        | crate::syntax::CapturedValue::Ident(s)
        | crate::syntax::CapturedValue::Expr(s) => Some(s.clone()),
        crate::syntax::CapturedValue::Selector(s) => Some(s.clone()),
        _ => None,
    }
}
/// Collect matches against a migration's EMBEDDED retired macros as pending
/// "manual" rows (PLAN-076/PLAN-079).
///
/// Hint-covered directives (`@show`, `@input`) never enter the shim — they
/// are not mechanically rewritable; they compile through the window via the
/// embedded macro's own `%binds` (W0715). Rule-covered directives land here
/// only when NO rule matched the call shape (e.g. `@bind(when:)` alone) —
/// the E0910 degrade path. Either way the pill's status and `check` list
/// them. Spans come from the FormMatch (no source text at that layer, so
/// `old_text` is empty).
pub fn collect_hint_pending(
    matches: &[FormMatch],
    registry: &MetaRegistry,
    version: Option<&str>,
) -> Vec<PendingMigration> {
    matches
        .iter()
        .filter_map(|fm| {
            let def_name = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
            let (mig_id, rule_id) = registry.retired_by_macro(def_name)?;
            if rule_id.is_some() {
                return None; // rule-def matches are the shim's channel
            }
            let mig = registry.get_migration(mig_id)?;
            if version.is_some_and(|v| mig.date.as_str() <= v) {
                return None; // inert wave
            }
            let directive = fm.macro_name.trim_start_matches('@');
            let hint = mig.hint_for(directive).map(|s| s.to_string()).or_else(|| {
                Some(format!(
                    "no rewrite rule covers this @{directive} shape — migrate it manually"
                ))
            });
            Some(PendingMigration {
                migration_id: mig.id.clone(),
                rule: None,
                date: mig.date.clone(),
                docs: mig.docs.clone(),
                span: DiagSpan::new(fm.span.start, fm.span.end),
                old_text: String::new(),
                new_text: None,
                hint,
            })
        })
        .collect()
}

/// Trim ASCII whitespace off both ends of a span (char-boundary safe).
/// FormMatch spans can include flanking whitespace; the splice must replace
/// the directive proper only.
fn trim_ws_span(source: &str, start: usize, end: usize) -> (usize, usize) {
    let slice = &source[start..end];
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    (start + lead, end - trail)
}

// =============================================================================
// Persistence rail (PLAN-076 W3) — the disk-facing half of the ONE engine.
// The pill's apply route and `spacetime migrate` both ride these functions;
// the compile-time shim and they share `apply_migration_shim_for_wave`.
// =============================================================================

/// What one file looks like after a (possibly wave-filtered) apply.
#[derive(Debug)]
pub struct FileApplyPlan {
    /// The file scanned
    pub path: std::path::PathBuf,
    /// The ORIGINAL source as scanned (pending spans index THIS text)
    pub old_source: String,
    /// Rewritten source (== original when nothing applied)
    pub new_source: String,
    /// Whether the rewrite changed the file's bytes
    pub changed: bool,
    /// Iteration-0 pending entries (what changed / what needs a human)
    pub pending: Vec<PendingMigration>,
    /// Shim diagnostics (E0912 etc.)
    pub diagnostics: Vec<Diagnostic>,
    /// Content hash at plan time — the staleness guard for the write
    /// (R5 P2): a file saved between scan and write must refuse, never be
    /// silently overwritten by stale-span rewrites.
    pub content_hash: u64,
}

impl FileApplyPlan {
    /// True when the rewrite changes the file's bytes.
    pub fn is_changed(&self) -> bool {
        self.changed
    }
}

/// Files this run could not read, in encounter order.
///
/// AUD-010 — `plan_file_apply` returns None both for "filtered out" and for
/// "could not parse", so an unreadable file silently left the plan list and
/// `migrate` printed "No pending migrations — the project is current." It cannot
/// know that: a file it failed to parse is a file it never inspected, and the
/// one holding retired syntax is exactly the one most likely not to parse.
///
/// A side channel rather than a signature change: every caller of
/// `plan_file_apply` uses it inside `filter_map`, and the CLI needs the list at
/// the END of the walk, not per file.
pub static UNREADABLE_FILES: std::sync::Mutex<Vec<std::path::PathBuf>> =
    std::sync::Mutex::new(Vec::new());

/// Record a file that could not be parsed, so the CLI can name it.
fn note_unreadable(path: &std::path::Path) {
    if let Ok(mut v) = UNREADABLE_FILES.lock()
        && !v.iter().any(|p| p == path)
    {
        v.push(path.to_path_buf());
    }
}

/// Compute the apply plan for ONE file: parse, run the shim (optionally
/// restricted to `wave`) with the PROJECT's `version` (the root entry's
/// fact, supplied by the caller). Returns None when the file does not parse,
/// and records it in [`UNREADABLE_FILES`] so the CLI can report it rather than
/// counting it as "nothing to do".
pub fn plan_file_apply(
    path: &std::path::Path,
    registry: &MetaRegistry,
    version: Option<&str>,
    wave: Option<&str>,
) -> Option<FileApplyPlan> {
    let Ok(content) = std::fs::read_to_string(path) else {
        note_unreadable(path);
        return None;
    };
    let Ok(ast) = crate::parser::parse(&content) else {
        note_unreadable(path);
        return None;
    };
    let outcome = apply_migration_shim_for_wave(content.clone(), ast, registry, version, wave);
    let changed = outcome.source != content;
    Some(FileApplyPlan {
        path: path.to_path_buf(),
        old_source: content.clone(),
        new_source: outcome.source,
        changed,
        pending: outcome.pending,
        diagnostics: outcome.diagnostics,
        content_hash: hash_content(&content),
    })
}

/// Content hash for the staleness guard (same DefaultHasher shape the
/// compiler's cache uses).
pub(crate) fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Insert or update the `@version <date>;` fact in a root entry source.
///
/// - An existing `@version …;` directive (located via its FormMatch span) is
///   rewritten in place to the new date.
/// - Otherwise the declaration is inserted after any leading doc/comment
///   run (top of file), followed by a blank line.
pub fn upsert_version_decl(source: &str, date: &str) -> String {
    let decl = format!("@version {date};");
    if let Ok(ast) = crate::parser::parse(source) {
        let fact = read_syntax_version(&ast.matches);
        if fact.decls >= 1 {
            // Replace the FIRST @version directive's span (whitespace-trimmed).
            for fm in &ast.matches {
                if fm.macro_name == "version" {
                    let (s, e) = trim_ws_span(source, fm.span.start, fm.span.end);
                    return format!("{}{}{}", &source[..s], decl, &source[e..]);
                }
            }
        }
    }
    // Insert after the leading comment/blank run. Scan BYTES for the real
    // line terminators (`str::lines()` + `line.len() + 1` miscounts CRLF and
    // overruns a comment-only file with no trailing newline — R3 P1).
    let bytes = source.as_bytes();
    let mut insert_at = 0;
    let mut pos = 0;
    // A leading `/* … */` block comment is leading trivia too — skip past it
    // (incl. any trailing newline) so the decl doesn't land BEFORE it (R4).
    {
        let trimmed = source.trim_start();
        if trimmed.starts_with("/*")
            && let Some(end) = trimmed.find("*/")
        {
            let mut after = end + 2;
            if trimmed[after..].starts_with("\r\n") {
                after += 2;
            } else if trimmed[after..].starts_with('\n') {
                after += 1;
            }
            let skipped = source.len() - trimmed.len() + after;
            insert_at = skipped;
            pos = skipped;
        }
    }
    while pos < bytes.len() {
        let nl = source[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(source.len());
        let line = source[pos..nl].trim_end_matches('\r');
        let t = line.trim_start();
        if t.starts_with("//") || t.is_empty() {
            // Advance past the line INCLUDING its terminator (\n if present).
            insert_at = if nl < source.len() {
                nl + 1
            } else {
                source.len()
            };
            pos = insert_at;
        } else {
            break;
        }
    }
    format!(
        "{}{}{}\n\n{}",
        &source[..insert_at],
        // The leading run's last line may lack its terminator (EOF without
        // trailing newline) — guarantee the decl starts on its own line.
        if insert_at > 0 && !source[..insert_at].ends_with('\n') {
            "\n"
        } else {
            ""
        },
        decl,
        &source[insert_at..]
    )
}

/// Outcome of persisting a set of apply plans to disk.
#[derive(Debug, Clone)]
pub struct ApplySummary {
    /// Files actually rewritten
    pub files_changed: usize,
    /// Automatic entries applied
    pub entries_applied: usize,
    /// The @version date written to the root entry (None when the root
    /// entry file doesn't exist — the bump is skipped, not fabricated)
    pub version: Option<String>,
}

/// Persist apply plans: write each changed file atomically (temp + rename),
/// then bump the root entry's `@version` to `applied_wave`. The ONE write
/// path — `spacetime migrate` and the dev-server apply route both ride it.
///
/// Ordering: ALL file writes complete before the @version bump, so a failed
/// write leaves the version fact consistent with what's actually on disk
/// (rerun converges).
pub fn write_apply_plans(
    plans: &[&FileApplyPlan],
    root: &std::path::Path,
    applied_wave: Option<&str>,
) -> Result<ApplySummary, String> {
    let mut files_changed = 0;
    let mut entries_applied = 0;
    for plan in plans {
        if !plan.is_changed() {
            continue;
        }
        // Staleness guard (PLAN-076 Safety): the dev server is multi-actor —
        // a save landing between the scan and this write must refuse, never
        // be silently overwritten by stale-span rewrites.
        let current = std::fs::read_to_string(&plan.path)
            .map_err(|e| format!("failed to re-read {}: {e}", plan.path.display()))?;
        if hash_content(&current) != plan.content_hash {
            return Err(format!(
                "{} changed on disk since the migration scan — refusing to overwrite; re-run to rescan",
                plan.path.display()
            ));
        }
        let tmp = plan.path.with_extension("st.migrate-tmp");
        std::fs::write(&tmp, &plan.new_source)
            .and_then(|_| std::fs::rename(&tmp, &plan.path))
            .map_err(|e| format!("failed to write {}: {e}", plan.path.display()))?;
        files_changed += 1;
        entries_applied += plan.pending.iter().filter(|e| e.is_automatic()).count();
    }

    let version = match applied_wave {
        Some(d) if root.is_file() => {
            let current = std::fs::read_to_string(root).unwrap_or_default();
            let bumped = upsert_version_decl(&current, d);
            if bumped != current {
                let tmp = root.with_extension("st.migrate-tmp");
                std::fs::write(&tmp, &bumped)
                    .and_then(|_| std::fs::rename(&tmp, root))
                    .map_err(|e| format!("failed to bump @version in {}: {e}", root.display()))?;
            }
            Some(d.to_string())
        }
        // No root entry — the bump is skipped (never fabricated).
        Some(_) => None,
        None => None,
    };

    Ok(ApplySummary {
        files_changed,
        entries_applied,
        version,
    })
}

/// Collect the project's .st files for a migrate run: recursive walk of
/// `root`, skipping build output, vendored dirs, hidden dirs, and ANY path
/// with a `stdlib` component (migrations never rewrite the stdlib itself).
pub fn collect_project_st_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                if matches!(
                    name.as_ref(),
                    "dist" | "node_modules" | "target" | "stdlib" | "vendor"
                ) {
                    continue;
                }
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("st") {
                out.push(path);
            }
        }
    }
    if root.is_file() {
        if root.extension().and_then(|e| e.to_str()) == Some("st") {
            return vec![root.to_path_buf()];
        }
        return Vec::new();
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

/// Render a unified-ish hunk for one pending entry (dry-run display):
/// ±2 lines of context around the span, `-` old lines, `+` new lines.
/// Hint-kind entries render as a note instead.
pub fn render_pending_hunk(source: &str, entry: &PendingMigration) -> String {
    let Some(new_text) = &entry.new_text else {
        return format!(
            "  ! manual: {} ({})\n    {}\n",
            entry.migration_id,
            entry.docs,
            entry.hint.as_deref().unwrap_or_default()
        );
    };
    let start = entry.span.start;
    let end = entry.span.end;
    // Line boundaries around the span.
    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[end..]
        .find('\n')
        .map(|i| end + i)
        .unwrap_or(source.len());
    let line_no = source[..line_start].matches('\n').count() + 1;
    // Context: up to 2 lines before/after.
    let ctx_start = {
        let mut s = line_start;
        for _ in 0..2 {
            s = source[..s.saturating_sub(1)]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            if s == 0 {
                break;
            }
        }
        s
    };
    let ctx_end = {
        // line_end is the INDEX of the span line's terminating '\n' (or EOF);
        // step past it first or the probe finds the same newline forever
        // (R3 P2: trailing context never rendered).
        let mut e = if line_end < source.len() {
            line_end + 1
        } else {
            line_end
        };
        for _ in 0..2 {
            if e >= source.len() {
                e = source.len();
                break;
            }
            match source[e..].find('\n') {
                Some(i) => e += i,
                None => {
                    e = source.len();
                    break;
                }
            }
        }
        e
    };
    let mut out = format!("  @@ line {} ({}) @@\n", line_no, entry.migration_id);
    for line in source[ctx_start..line_start].lines() {
        out.push_str(&format!("    {}\n", line));
    }
    for line in source[line_start..line_end].lines() {
        out.push_str(&format!("  - {}\n", line));
    }
    for line in new_text.lines() {
        out.push_str(&format!("  + {}\n", line));
    }
    // Trailing context starts AFTER the span line's newline (skip the
    // newline itself so no spurious blank line prints).
    let trail_start = usize::min(line_end + 1, source.len());
    for line in source[trail_start..ctx_end].lines() {
        out.push_str(&format!("    {}\n", line));
    }
    out
}

/// Arg names a call site carries BEYOND what the rule's `%match` form claims.
/// Non-empty ⇒ the rewrite is unsound (whole-span replacement would drop
/// them) ⇒ degrade to E0910 instead of rewriting.
fn unconsumed_arg_names(fm: &FormMatch, form: &FormClause, source: &str) -> Vec<(String, bool)> {
    let claimed: HashSet<String> = form.params.iter().map(|p| p.name.clone()).collect();
    // Positional call args bind UNNAMED params only, in declaration order,
    // when the group's shape fits the param's capture type — the real
    // matcher's rule (measured: `@on visible x(600ms)` binds the unnamed
    // `$duration:time`; `@time x(2s)` against an all-NAMED form leaves `2s`
    // unbound — tolerated-and-dropped). Approximate exactly that: an earlier
    // version fitted positionals into NAMED slots too, which read real
    // extras as consumed and let rules apply without the required %drops.
    let mut positional_slots: Vec<(String, String)> = form
        .params
        .iter()
        .filter(|p| p.name.is_empty())
        .map(|p| (p.name.clone(), param_capture_type_name(p)))
        .collect();
    let span_text = &source[fm.span.start..fm.span.end];
    let Some(open) = span_text.find('(') else {
        return Vec::new(); // no arg list — nothing extra possible
    };
    let bytes = span_text.as_bytes();
    let Some(close) = matching_paren(bytes, open) else {
        return Vec::new();
    };
    let args = &span_text[open + 1..close];

    // Top-level comma groups; a group's arg name is the leading `ident:`
    // (mirrors the form matcher's name-first grouping). Bare positional
    // groups count as unconsumed when the form declares no unnamed params.
    let has_unnamed_form_param = form.params.iter().any(|p| p.name.is_empty());
    let mut extras = Vec::new();
    for group in split_arg_groups(args) {
        let group = group.trim();
        if group.is_empty() {
            continue;
        }
        let name = group
            .split(':')
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('$');
        let looks_named = group.contains(':')
            && !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if looks_named {
            if !claimed.contains(name) {
                extras.push((name.to_string(), false));
            } else {
                // A named group fills its slot — it can't take a positional too.
                positional_slots.retain(|(n, _)| n != name);
            }
        } else if has_unnamed_form_param {
            // Positional: bind the next unnamed slot whose type fits, else extra.
            let slot_idx = positional_slots
                .iter()
                .position(|(_, ty)| positional_group_fits(group, ty));
            match slot_idx {
                Some(idx) => {
                    positional_slots.remove(idx);
                }
                None => extras.push((group.to_string(), true)),
            }
        } else {
            // No unnamed params: a positional can NEVER bind — the real
            // matcher tolerates-and-drops it. Always an extra (%drops `_`
            // must declare it, else the rule degrades to E0910).
            extras.push((group.to_string(), true));
        }
    }
    extras
}

/// The capture type NAME of a param's first capture element (e.g. "time",
/// "ident", "number") — used only for the positional-fit approximation.
fn param_capture_type_name(param: &crate::parser::meta_ast::FormParam) -> String {
    use crate::parser::meta_ast::FormInlineElement;
    for el in &param.elements {
        if let FormInlineElement::Capture(cap, _) = el {
            return format!("{:?}", cap.capture_type).to_lowercase();
        }
    }
    String::new()
}

/// Conservative positional-fit: a group binds a slot ONLY on an exact shape
/// match. `300ms` → time/duration; `0.2` → number; anything else →
/// ident/string/selector/bool. Everything else (notably `600ms` against an
/// `:ident` slot — the tolerated-drop case) does NOT fit.
fn positional_group_fits(group: &str, type_name: &str) -> bool {
    // Share the extractor's own duration parser — a hand-rolled suffix check
    // under-approximates (`1m`, `1000us`, `60fps` are all valid `:time`) and
    // would misclassify a LIVE positional capture as a droppable extra.
    let is_time = crate::syntax::conversions::parse_duration_with_unit(group).is_some();
    let is_number = group.parse::<f64>().is_ok();
    let is_bool = group == "true" || group == "false";
    match type_name {
        t if t.contains("time") || t.contains("duration") => is_time,
        t if t.contains("number") => is_number,
        t if t.contains("bool") => is_bool,
        t if t.contains("ident") || t.contains("string") || t.contains("selector") => {
            !is_time && !is_number
        }
        _ => false,
    }
}

/// Scan bytes over string literals (`"…"`, `'…'`, backtick holes) and `//`
/// line comments, invoking `on` only for STRUCTURAL characters. Both the
/// paren matcher and the comma splitter ride this — a `)` or `,` inside a
/// string arg must never truncate a scan (R2: silent-truncation correctness).
fn for_structural_bytes(bytes: &[u8], mut on: impl FnMut(usize, u8) -> bool) {
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' | b'`' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    let done = bytes[i] == quote;
                    i += 1;
                    if done {
                        break;
                    }
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            // Block comments (R4: a bracket inside `/* … */` must never
            // desync the paren/comma scan — that would fail the
            // unconsumed-args guard OPEN into an unsound arg-dropping
            // rewrite). Unterminated: skip to EOF (the scan simply ends).
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = usize::min(i + 2, bytes.len());
            }
            b => {
                if !on(i, b) {
                    return;
                }
                i += 1;
            }
        }
    }
}

/// Depth-aware close-paren for the `(` at `open`, string/comment-aware.
fn matching_paren(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut close = None;
    for_structural_bytes(&bytes[open..], |idx, b| {
        match b {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + idx);
                    return false;
                }
            }
            _ => {}
        }
        true
    });
    close
}

/// Top-level (depth-0) comma groups of an arg list, string/comment-aware.
fn split_arg_groups(args: &str) -> Vec<&str> {
    let bytes = args.as_bytes();
    let mut groups = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for_structural_bytes(bytes, |idx, b| {
        match b {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            b',' if depth == 0 => {
                groups.push(&args[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
        true
    });
    groups.push(&args[start..]);
    groups
}

/// The `@version` a real source file compiles AT: its own declared fact, or —
/// when it declares none — the newest registered wave.
///
/// HARD CUTOVER. An absent `@version` used to mean "older than every wave", so
/// the migration window (W0715) stood open forever: retired syntax was
/// rewritten in memory and compiled clean in a file written long after the wave
/// that retired it. That is a permanent amnesty rather than a transition, and
/// it is how ~100 sites accumulated unnoticed (AUD-009).
///
/// Absent now means CURRENT: new code is held to the current surface, and a
/// project genuinely mid-migration opts INTO the window by declaring the older
/// `@version` it is migrating FROM — exactly what `spacetime migrate` writes
/// before bumping it forward.
///
/// This is POLICY about real source files, which is why it lives here and not
/// inside `apply_migration_shim_for_wave`: the shim is a mechanism, and `None`
/// must keep meaning "no version constraint" for a caller that means it.
/// Derived from the registry, so shipping a new wave moves the default with it
/// — no date to keep in sync in Rust.
pub fn effective_syntax_version(
    declared: Option<String>,
    registry: &MetaRegistry,
) -> Option<String> {
    declared.or_else(|| registry.newest_migration_wave().map(String::from))
}

/// The compat shim: rewrite old syntax IN MEMORY, wave by wave.
///
/// `source`/`ast` are the freshly-parsed main file (pre-import-resolution).
/// Returns the rewritten source + fresh AST + diagnostics + pending entries.
/// Waves apply oldest-first; fuel = distinct participating wave dates + 1,
/// so forward chains (A→B→C) terminate and a cycle reports E0912.
pub fn apply_migration_shim(
    source: String,
    ast: StFile,
    registry: &MetaRegistry,
    version: Option<&str>,
) -> ShimOutcome {
    apply_migration_shim_for_wave(source, ast, registry, version, None)
}

/// The shim restricted to a single wave (`--wave`): only migrations dated
/// exactly `wave` participate. `None` = every participating wave (the
/// compile-time behavior).
pub fn apply_migration_shim_for_wave(
    mut source: String,
    mut ast: StFile,
    registry: &MetaRegistry,
    version: Option<&str>,
    wave: Option<&str>,
) -> ShimOutcome {
    // HARD CUTOVER: an ABSENT `@version` means CURRENT, not ancient.
    //
    // `migrations_pending_since(None)` treats a file with no `@version` fact as
    // predating EVERY wave, so the migration window stood open forever: retired
    // syntax was silently rewritten in memory and compiled clean, in a file
    // written long after the wave that retired it. That is a permanent amnesty
    // rather than a transition, and it is how ~100 sites accumulated unnoticed
    // (AUD-009).
    //
    // Defaulting to the newest registered wave inverts it: new code is held to
    // the current surface, and a project genuinely mid-migration opts INTO the
    // window by declaring the older `@version` it is migrating FROM — exactly
    // what `spacetime migrate` writes before bumping it.
    //
    // Applied at the COMPILE entry point (`effective_syntax_version`), not
    // here: the shim is a mechanism and `None` must keep meaning "no version
    // constraint" for a caller that means it (the unit tests drive synthetic
    // registries and would otherwise close their own window). The default is a
    // POLICY about real source files, so it belongs where a real file's
    // version fact is read.
    let participating: Vec<&MigrationDefAst> = registry
        .migrations_pending_since(version)
        .into_iter()
        .filter(|m| m.is_rewrite())
        .filter(|m| wave.is_none_or(|w| m.date == w))
        .collect();

    let mut diagnostics = Vec::new();
    let mut pending = Vec::new();

    // Iteration-0 embedded-macro scan (R4: span-drift fix). Matches against a
    // retired DEFINITION (rule: None) — hint-covered directives (@show/@input)
    // and rule-covered shapes no rule matched (@bind(when:) alone) — are
    // recorded HERE, against the ORIGINAL on-disk source, so their pending
    // spans never drift from the rewrites spliced below. (Every migration
    // pending_since(version) participates in this channel — including
    // hint-only capsules that carry no %rewrite rules.)
    {
        let embedded_participating: Vec<&MigrationDefAst> = registry
            .migrations_pending_since(version)
            .into_iter()
            .filter(|m| wave.is_none_or(|w| m.date == w))
            .collect();
        for fm in &ast.matches {
            let def_name = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
            let Some((mig_id, None)) = registry.retired_by_macro(def_name) else {
                continue;
            };
            let Some(mig) = embedded_participating.iter().find(|m| &m.id == mig_id) else {
                continue;
            };
            let directive = fm.macro_name.trim_start_matches('@');
            let (span_start, span_end) = trim_ws_span(&source, fm.span.start, fm.span.end);
            pending.push(PendingMigration {
                migration_id: mig.id.clone(),
                rule: None,
                date: mig.date.clone(),
                docs: mig.docs.clone(),
                span: DiagSpan::new(span_start, span_end),
                old_text: source[span_start..span_end].to_string(),
                new_text: None,
                hint: Some(
                    mig.hint_for(directive).map(|s| s.to_string()).unwrap_or_else(|| {
                        format!(
                            "no rewrite rule covers this @{directive} shape — migrate it manually"
                        )
                    }),
                ),
            });
        }
    }

    if participating.is_empty() {
        return ShimOutcome {
            source,
            ast,
            diagnostics,
            pending,
        };
    }

    // Fuel = one application per participating RULE + 1: chains (A→B→C,
    // including same-capsule rule→rule hops the load validator permits) need
    // one iteration per hop. Anything still progressing past that cycles.
    let rule_count: usize = participating.iter().map(|m| m.rewrites.len()).sum();
    let fuel = rule_count + 1;

    for iteration in 0..fuel {
        // Collect this iteration's rewrite-rule hits. A rule claims a match
        // via `matched_macro` (the exact def parse selected — the match-only
        // rule def `<migration>#<rule>`); embedded-macro matches (rule:
        // None) are the manual channel, never spliced here.
        let hits: Vec<(&FormMatch, &MigrationDefAst, &MigrationRewriteAst)> = ast
            .matches
            .iter()
            .filter_map(|fm| {
                let def_name = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
                let (mig_id, rule_id) = registry.retired_by_macro(def_name)?;
                let rule_id = rule_id.as_ref()?;
                participating
                    .iter()
                    .find(|m| &m.id == mig_id)
                    .and_then(|m| {
                        m.rewrites
                            .iter()
                            .find(|r| &r.id == rule_id)
                            .map(|r| (*m, r))
                    })
                    .map(|(m, r)| (fm, m, r))
            })
            .collect();
        if hits.is_empty() {
            break;
        }

        // Partition: unsound matches (extra args) are NEVER rewritten — they
        // stay in the AST and the pipeline's retired-syntax check turns them
        // into E0910 (with the multi-shape note recorded as pending). Only
        // spliceable hits count as forward progress: an iteration with none
        // is the natural end of the chain, not a cycle. A rule may declare
        // `%drops`: extras it may silently drop because they never reached
        // the old `%binds` (measured — e.g. a positional duration a
        // tolerated-arg form ignored on every compile).
        let mut spliceable: Vec<(&FormMatch, &MigrationDefAst, &MigrationRewriteAst)> = Vec::new();
        for (fm, mig, rule) in &hits {
            let extras = unconsumed_arg_names(fm, &rule.match_form, &source);
            let droppable = |(extra, positional): &(String, bool)| {
                rule.drops.iter().any(|d| d == extra)
                    || (*positional && rule.drops.iter().any(|d| d == "_"))
            };
            if extras.is_empty() || extras.iter().all(droppable) {
                spliceable.push((fm, mig, rule));
            } else if iteration == 0 {
                let (span_start, span_end) = trim_ws_span(&source, fm.span.start, fm.span.end);
                pending.push(PendingMigration {
                    migration_id: mig.id.clone(),
                    rule: Some(rule.id.clone()),
                    date: mig.date.clone(),
                    docs: mig.docs.clone(),
                    span: DiagSpan::new(span_start, span_end),
                    old_text: source[span_start..span_end].to_string(),
                    new_text: None,
                    hint: Some(format!(
                        "call carries args beyond the migration's shape ({}) — \
                         split them or migrate manually",
                        extras
                            .iter()
                            .map(|(e, _)| e.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                });
            }
        }
        if spliceable.is_empty() {
            break; // only degraded matches remain — pipeline reports them
        }
        if iteration == fuel - 1 {
            // Fuel exhausted with REAL progress remaining: a chain cycle
            // slipped past load-time validation. Loud, never a hang.
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0912,
                    format!(
                        "syntax migration apply exceeded its rule fuel ({fuel}) — \
                         a migration cycle exists in the stdlib entries"
                    ),
                )
                .with_span(DiagSpan::new(
                    spliceable[0].0.span.start,
                    spliceable[0].0.span.end,
                )),
            );
            break;
        }

        // Build replacements. FormMatch spans may include flanking WHITESPACE
        // (the parse records the statement extent); trim whitespace at both
        // ends so the splice replaces the directive proper and leaves
        // surrounding formatting untouched. Comments are never trimmed — only
        // ASCII whitespace.
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        for (fm, mig, rule) in &spliceable {
            let (span_start, span_end) = trim_ws_span(&source, fm.span.start, fm.span.end);
            let old_text = source[span_start..span_end].to_string();
            let new_text = instantiate_template(&rule.template, fm, &source);
            if iteration == 0 {
                pending.push(PendingMigration {
                    migration_id: mig.id.clone(),
                    rule: Some(rule.id.clone()),
                    date: mig.date.clone(),
                    docs: mig.docs.clone(),
                    span: DiagSpan::new(span_start, span_end),
                    old_text,
                    new_text: Some(new_text.clone()),
                    hint: None,
                });
            }
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::W0715,
                    format!(
                        "`{}` rule `{}` ({}) auto-migrated in memory: {}",
                        mig.id, rule.id, mig.date, mig.docs
                    ),
                )
                .with_hint(
                    "persist the rewrite via the migrations pill or `spacetime migrate`"
                        .to_string(),
                )
                .with_span(DiagSpan::new(span_start, span_end)),
            );
            replacements.push((span_start, span_end, new_text));
        }

        // Reverse-span-order splicing (the established edit_collector /
        // apply_text_patch pattern): earlier offsets stay valid.
        replacements.sort_by_key(|(start, _, _)| usize::MAX - start);
        for (start, end, text) in replacements {
            source.replace_range(start..end, &text);
        }

        // Re-parse for the next wave. A stdlib entry whose template produces
        // non-parsing output surfaces here, attributed to the migration.
        match crate::parser::parse(&source) {
            Ok(new_ast) => ast = new_ast,
            Err(e) => {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0912,
                        format!(
                            "a %migration %rewrite produced source that no longer parses: {}",
                            e.first().message
                        ),
                    )
                    .with_hint(
                        "this is a stdlib migration-authoring error — the template must \
                         produce valid .st syntax"
                            .to_string(),
                    ),
                );
                break;
            }
        }
    }

    ShimOutcome {
        source,
        ast,
        diagnostics,
        pending,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metasystem::MetaRegistry;
    use crate::parser::meta_ast::MetaDef;
    use crate::syntax::CapturedValue;
    use std::collections::HashMap;

    /// Register the on-disk seed entries (stdlib/migrations/entries/*.st) +
    /// any extra %migration sources, into a fresh MetaRegistry.
    fn registry_with_seeds(extra: &[&str]) -> MetaRegistry {
        let mut registry = MetaRegistry::new();
        let dir = std::path::Path::new("stdlib/migrations/entries");
        for entry in std::fs::read_dir(dir).expect("entries dir") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("st") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap();
            let parsed = crate::parser::parse(&content).expect("seed parses");
            for def in parsed.meta_defs {
                registry.register(def).expect("seed registers");
            }
        }
        for src in extra {
            let parsed = crate::parser::parse(src).expect("extra migration parses");
            for def in parsed.meta_defs {
                if let MetaDef::Migration(m) = def {
                    registry.register_migration(m).expect("extra registers");
                }
            }
        }
        crate::metasystem::validate_migration_chains(&registry).expect("chains valid");
        registry
    }

    /// A hand-built FormMatch for `macro_name` covering `source[span]`, with
    /// one capture (name -> span) — the shape the real parse produces.
    fn fake_match(
        macro_name: &str,
        source: &str,
        span: (usize, usize),
        captures: &[(&str, (usize, usize))],
    ) -> FormMatch {
        let mut capture_spans = HashMap::new();
        let mut capture_values = HashMap::new();
        for (name, (s, e)) in captures {
            capture_spans.insert(
                name.to_string(),
                crate::parser::SourceSpan { start: *s, end: *e },
            );
            capture_values.insert(
                name.to_string(),
                CapturedValue::Expr(source[*s..*e].to_string()),
            );
        }
        FormMatch {
            macro_name: macro_name.to_string(),
            matched_macro: Some(macro_name.to_string()),
            captures: capture_values,
            capture_spans,
            selector: Some(".card".to_string()),
            span: crate::parser::SourceSpan {
                start: span.0,
                end: span.1,
            },
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    #[test]
    fn instantiate_template_splices_raw_slices_and_keeps_literal_dollar() {
        let source = ".card { @bind(text: $user.name | uppercase) }";
        let open = source.find("@bind").unwrap();
        let close = source.find(')').unwrap() + 1;
        let expr_start = source.find("$user.name").unwrap();
        let expr_end = source.find(')').unwrap();
        let fm = fake_match(
            "bind-text-to-arrow",
            source,
            (open, close),
            &[("x", (expr_start, expr_end))],
        );
        // Template with a hole AND a literal bare `$sig` (must stay verbatim).
        let out = instantiate_template("text <- `$x`; $sig <- 1;", &fm, source);
        assert_eq!(out, "text <- $user.name | uppercase; $sig <- 1;");
    }

    #[test]
    fn read_syntax_version_normalizes_and_counts() {
        let mut fm = FormMatch::new("version");
        fm.captures.insert(
            "date".to_string(),
            CapturedValue::Expr("2026 -06 -09".to_string()),
        );
        let fact = read_syntax_version(&[fm]);
        assert_eq!(fact.version.as_deref(), Some("2026-06-09"));
        assert_eq!(fact.decls, 1);
        assert!(crate::migrate::is_iso_wave_date("2026-06-09"));
        assert!(!crate::migrate::is_iso_wave_date("2026-13-09"));
        assert!(!crate::migrate::is_iso_wave_date("banana"));
    }

    #[test]
    fn shim_rewrites_seed_bind_text() {
        let registry = registry_with_seeds(&[]);
        let source = ".card { @bind(text: $user.name) }".to_string();
        let open = source.find("@bind").unwrap();
        let close = source.find(')').unwrap() + 1;
        let expr_start = source.find("$user.name").unwrap();
        let expr_end = source.find(')').unwrap();
        let fm = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open, close),
            &[("x", (expr_start, expr_end))],
        );
        let ast = StFile {
            matches: vec![fm],
            form_refs: Vec::new(),
            ..Default::default()
        };
        let out = apply_migration_shim(source, ast, &registry, None);
        assert_eq!(out.source, ".card { text <- $user.name; }");
        assert_eq!(out.pending.len(), 1);
        assert_eq!(
            out.pending[0].new_text.as_deref(),
            Some("text <- $user.name;")
        );
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::W0715)
        );
    }

    #[test]
    fn shim_chains_forward_across_waves() {
        // Wave 2025-01-01: @legacy(text:) -> @bind(text:); the SEED wave
        // 2026-06-09 then takes @bind(text:) -> text <-. Two W0715s total;
        // pending covers only the iteration-0 (on-disk) match.
        let legacy = r#"%migration legacy-to-bind {
  %date 2025-01-01
  %docs "@legacy(text:) became @bind(text:)."

  %macro legacy {
    %form { @legacy(text: $x:expr) }
  }

  %rewrite legacy-text {
    %match { @legacy(text: $x:expr) }
    %into { @bind(text: `$x`) }
  }
}"#;
        let registry = registry_with_seeds(&[legacy]);
        let source = ".card { @legacy(text: $user.name) }".to_string();
        let open = source.find("@legacy").unwrap();
        let close = source.find(')').unwrap() + 1;
        let expr_start = source.find("$user.name").unwrap();
        let expr_end = source.find(')').unwrap();
        let fm = fake_match(
            "legacy-to-bind#legacy-text",
            &source,
            (open, close),
            &[("x", (expr_start, expr_end))],
        );
        let ast = StFile {
            matches: vec![fm],
            form_refs: Vec::new(),
            ..Default::default()
        };
        let out = apply_migration_shim(source, ast, &registry, None);
        assert_eq!(out.source, ".card { text <- $user.name; }");
        assert_eq!(out.pending.len(), 1);
        assert_eq!(out.pending[0].migration_id, "legacy-to-bind");
        let warnings = out
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::W0715)
            .count();
        assert_eq!(warnings, 2, "one W0715 per applied wave");
    }

    #[test]
    fn shim_degrades_multi_shape_calls() {
        let registry = registry_with_seeds(&[]);
        // @bind with BOTH text and class — bind-text matches (text claimed),
        // but class/when are unconsumed: rewriting would silently drop them.
        let source = ".card { @bind(text: $user.name, class: \"open\", when: $flag) }".to_string();
        let open = source.find("@bind").unwrap();
        let close = source.rfind(')').unwrap() + 1;
        let expr_start = source.find("$user.name").unwrap();
        let expr_end = expr_start + "$user.name".len();
        let fm = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open, close),
            &[("x", (expr_start, expr_end))],
        );
        let ast = StFile {
            matches: vec![fm],
            form_refs: Vec::new(),
            ..Default::default()
        };
        let out = apply_migration_shim(source.clone(), ast, &registry, None);
        assert_eq!(out.source, source, "unsound rewrite must NOT happen");
        assert_eq!(out.pending.len(), 1);
        assert!(out.pending[0].new_text.is_none());
        assert!(
            out.pending[0]
                .hint
                .as_deref()
                .unwrap_or_default()
                .contains("class"),
            "hint should name the extra args: {:?}",
            out.pending[0].hint
        );
    }

    #[test]
    fn upsert_version_decl_crlf_inserts_after_comments() {
        // R3 P1: CRLF line endings must not corrupt the insert point — the
        // decl must land AFTER the comment run, outside any comment.
        let crlf = "// project docs\r\n\r\n$flag bool: true;\r\n";
        let out = upsert_version_decl(crlf, "2026-06-09");
        let decl_line = out.lines().find(|l| l.contains("@version")).unwrap();
        assert_eq!(decl_line, "@version 2026-06-09;");
        // And it parses back as a version fact (not swallowed by a comment).
        let ast = crate::parser::parse(&out).expect("reparse");
        let fact = read_syntax_version(&ast.matches);
        assert_eq!(fact.version.as_deref(), Some("2026-06-09"));
    }

    #[test]
    fn upsert_version_decl_comment_only_file_no_trailing_newline() {
        // R3 P1: a comment-only file with NO trailing newline must not panic
        // or produce an out-of-bounds slice.
        let src = "// only a comment";
        let out = upsert_version_decl(src, "2026-06-09");
        assert!(out.starts_with("// only a comment\n@version 2026-06-09;"));
    }

    #[test]
    fn upsert_version_decl_updates_existing_in_place() {
        let src = "@version 2026-06-09;\n$flag bool: true;\n";
        let out = upsert_version_decl(src, "2027-01-15");
        assert!(out.contains("@version 2027-01-15;"));
        assert!(!out.contains("2026-06-09"));
        assert_eq!(out.matches("@version").count(), 1);
    }

    #[test]
    fn render_pending_hunk_shows_trailing_context() {
        // R3 P2: the trailing context lines must actually render.
        let source = "line one\n.card { @bind(text: $x) }\nline three\nline four\nline five\n";
        let entry = PendingMigration {
            migration_id: "bind-text-to-arrow".to_string(),
            rule: None,
            date: "2026-06-09".to_string(),
            docs: "docs".to_string(),
            span: DiagSpan::new(18, 37),
            old_text: "@bind(text: $x)".to_string(),
            new_text: Some("text <- $x;".to_string()),
            hint: None,
        };
        let hunk = render_pending_hunk(source, &entry);
        assert!(hunk.contains("line three"), "hunk: {hunk}");
        assert!(hunk.contains("line four"), "hunk: {hunk}");
        assert!(hunk.contains("- .card { @bind(text: $x) }"), "hunk: {hunk}");
        assert!(hunk.contains("+ text <- $x;"), "hunk: {hunk}");
    }

    #[test]
    fn write_apply_plans_refuses_stale_files() {
        // R5 P2: a save landing between the scan and the write must refuse,
        // never be silently overwritten by stale-span rewrites.
        let registry = registry_with_seeds(&[]);
        let dir = std::env::temp_dir().join(format!("stale-guard-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("index.st");
        std::fs::write(&file, ".card { @bind(text: $user.name) }\n").unwrap();

        let plan = plan_file_apply(&file, &registry, None, None).expect("plan");
        assert!(plan.is_changed());

        // The user saves an unrelated edit AFTER the scan.
        std::fs::write(&file, "// user edit\n.card { @bind(text: $user.name) }\n").unwrap();

        let err = write_apply_plans(&[&plan], &file, Some("2026-06-09")).unwrap_err();
        assert!(err.contains("changed on disk"), "err: {err}");
        // The user's edit is untouched.
        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(on_disk.starts_with("// user edit"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// R4 P2 (fuel): intra-capsule rule→rule chains (@a → @b → new syntax in
    /// ONE migration) are permitted by load validation and served by the
    /// shim. NOTE: the shim re-parses via the GLOBAL stdlib registry
    /// (production: byte-identical to the one passed in), so a unit test can
    /// only exercise multi-hop chains whose later hops match GLOBAL (seed)
    /// rule defs — the intra-capsule case is pinned at the validator level
    /// (metasystem::tests::migrations) and by the multi-wave chain test
    /// above; the fuel arithmetic (rules + 1, degraded ≠ progress) is pinned
    /// by shim_degraded_leftover_is_not_a_cycle.

    /// R4 P2 (fuel): one SOUND match + one permanently-DEGRADED match in the
    /// same wave must not trip the cycle guard — the degraded match is not
    /// forward progress.
    #[test]
    fn shim_degraded_leftover_is_not_a_cycle() {
        let registry = registry_with_seeds(&[]);
        let source = ".a { @bind(text: $x) }\n.b { @bind(text: $y, class: \"open\") }".to_string();
        let open1 = source.find("@bind").unwrap();
        let close1 = source.find(')').unwrap() + 1;
        let fm1 = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open1, close1),
            &[(
                "x",
                (source.find("$x").unwrap(), source.find("$x").unwrap() + 2),
            )],
        );
        let open2 = source.rfind("@bind").unwrap();
        let close2 = source.rfind(')').unwrap() + 1;
        let fm2 = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open2, close2),
            &[(
                "x",
                (source.find("$y").unwrap(), source.find("$y").unwrap() + 2),
            )],
        );
        let ast = StFile {
            matches: vec![fm1, fm2],
            ..Default::default()
        };
        let out = apply_migration_shim(source, ast, &registry, None);
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::E0912),
            "degraded leftovers are not cycles: {:?}",
            out.diagnostics
        );
        assert!(out.source.contains("text <- $x;"));
        assert!(
            out.source.contains("@bind(text: $y, class:"),
            "degraded untouched"
        );
    }

    /// R4 P2 (soundness): a block comment carrying an unbalanced bracket in
    /// the arg list must not fail the unconsumed-args guard OPEN — the extras
    /// are still seen, the rewrite refused.
    #[test]
    fn shim_block_comment_in_args_does_not_fail_open() {
        let registry = registry_with_seeds(&[]);
        let source = ".card { @bind(text: $x /* ( */, class: \"open\") }".to_string();
        let open = source.find("@bind").unwrap();
        let close = source.rfind(')').unwrap() + 1;
        let fm = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open, close),
            &[(
                "x",
                (source.find("$x").unwrap(), source.find("$x").unwrap() + 2),
            )],
        );
        let ast = StFile {
            matches: vec![fm],
            form_refs: Vec::new(),
            ..Default::default()
        };
        let out = apply_migration_shim(source.clone(), ast, &registry, None);
        assert_eq!(out.source, source, "unsound rewrite must NOT happen");
        assert_eq!(out.pending.len(), 1);
        assert!(out.pending[0].new_text.is_none());
        assert!(
            out.pending[0]
                .hint
                .as_deref()
                .unwrap_or_default()
                .contains("class"),
            "hint names the extra arg: {:?}",
            out.pending[0].hint
        );
    }

    /// R4 P2 (span drift): the shim records EMBEDDED-macro matches (hint
    /// channel) at iteration 0 — pre-shim on-disk spans — so a rewrite
    /// earlier in the file can't shift the @show row's coordinates.
    #[test]
    fn shim_records_embedded_matches_with_pre_shim_spans() {
        let registry = registry_with_seeds(&[]);
        let source =
            "$flag bool: true;\n.card { @bind(text: $user.name) }\n.spinner { @show(when: $flag) }\n"
                .to_string();
        let open_b = source.find("@bind").unwrap();
        let close_b = source.find(')').unwrap() + 1;
        let fm_bind = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open_b, close_b),
            &[("x", (source.find("$user.name").unwrap(), close_b - 1))],
        );
        let open_s = source.find("@show").unwrap();
        let close_s = source.rfind(')').unwrap() + 1;
        let mut fm_show = fake_match(
            "reactive-surface#@show",
            &source,
            (open_s, close_s),
            &[("condition", (source.find("$flag").unwrap(), close_s - 1))],
        );
        // Real embedded matches carry the DIRECTIVE as macro_name (the def
        // identity lives in matched_macro).
        fm_show.macro_name = "show".to_string();
        let ast = StFile {
            matches: vec![fm_bind, fm_show],
            ..Default::default()
        };
        let out = apply_migration_shim(source.clone(), ast, &registry, None);
        // The @bind was rewritten (shortening the text before @show)…
        assert!(out.source.contains("text <- $user.name;"));
        // …and the @show pending row still points at the REAL on-disk text.
        let show_row = out
            .pending
            .iter()
            .find(|p| p.migration_id == "reactive-surface" && p.rule.is_none())
            .expect("embedded pending row");
        assert_eq!(show_row.old_text, "@show(when: $flag)");
        assert_eq!(
            &source[show_row.span.start..show_row.span.end],
            "@show(when: $flag)",
            "span indexes the pre-shim source"
        );
        assert!(
            show_row
                .hint
                .as_deref()
                .unwrap_or_default()
                .contains(".hidden:"),
            "hint from the entry's %hint: {:?}",
            show_row.hint
        );
    }

    #[test]
    fn shim_respects_version_inertness() {
        let registry = registry_with_seeds(&[]);
        let source = ".card { @bind(text: $user.name) }".to_string();
        let open = source.find("@bind").unwrap();
        let close = source.find(')').unwrap() + 1;
        let expr_start = source.find("$user.name").unwrap();
        let expr_end = source.find(')').unwrap();
        let fm = fake_match(
            "reactive-surface#bind-text",
            &source,
            (open, close),
            &[("x", (expr_start, expr_end))],
        );
        let ast = StFile {
            matches: vec![fm],
            form_refs: Vec::new(),
            ..Default::default()
        };
        // Project already crossed the 2026-06-09 wave: nothing participates.
        let out = apply_migration_shim(source.clone(), ast, &registry, Some("2026-06-09"));
        assert_eq!(out.source, source);
        assert!(out.pending.is_empty());
        assert!(out.diagnostics.is_empty());
    }
}

#[cfg(test)]
mod bug337_indentation {
    use super::reindent_splice;

    /// A capture's source span starts at its first TOKEN, so line 1 arrives
    /// with no leading whitespace while its siblings keep the original depth.
    /// Splicing that raw is what put a migrated body's opening line at column 0
    /// with the rest still indented (BUG-337). Every line must share one origin.
    #[test]
    fn a_multiline_block_hangs_entirely_from_the_hole_column() {
        let src = "translate-y: -100% -> 0\n            opacity: 0 -> 1\n            easing: linear";
        let got = reindent_splice(src, 10);
        let cols: Vec<usize> = got
            .lines()
            .skip(1)
            .map(|l| l.len() - l.trim_start().len())
            .collect();
        assert_eq!(cols, vec![10, 10], "continuation lines must sit at the hole column: {got:?}");
        assert!(
            !got.lines().next().unwrap().starts_with(' '),
            "line 1 is emitted AT the hole, so it carries no pad of its own: {got:?}"
        );
    }

    /// Relative nesting inside the block survives the shift.
    #[test]
    fn nesting_inside_the_block_is_preserved() {
        let src = "a: 1\n    b: 2\n        c: 3";
        let got = reindent_splice(src, 4);
        let cols: Vec<usize> = got
            .lines()
            .skip(1)
            .map(|l| l.len() - l.trim_start().len())
            .collect();
        assert_eq!(cols, vec![4, 8], "deeper lines stay deeper: {got:?}");
    }

    /// A single-line splice has nothing to align against and must pass through.
    #[test]
    fn a_single_line_splice_is_untouched() {
        assert_eq!(reindent_splice("opacity: 1", 12), "opacity: 1");
    }

    /// A blank line stays blank rather than collecting trailing whitespace.
    #[test]
    fn blank_lines_are_not_padded() {
        let got = reindent_splice("a: 1\n\n    b: 2", 6);
        assert_eq!(got, "a: 1\n\n      b: 2", "got: {got:?}");
    }
}
