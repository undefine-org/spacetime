//! Signal usage analysis for Spacetime.
//!
//! Tracks signal definitions and usages to detect:
//! - W0401: Unused signal definitions (signal defined but never used)
//! - E0408: Undefined signal references (signal used but not defined)
//! - W0403: Signal shadowing (signal shadows outer signal)

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::parser::meta_ast::{ExportDecl, MetaDef, PrimitiveDefAst};
use crate::parser::{NestedScope, ScopeBlock, SourceSpan, StFile};
use std::collections::{HashMap, HashSet};

// Use the diagnostics SourceSpan for diagnostics output
use crate::diagnostics::SourceSpan as DiagnosticSpan;

// =============================================================================
// Signal Analysis Data Structures
// =============================================================================

/// Analysis of a single signal definition and its usage.
#[derive(Debug, Clone)]
pub struct SignalAnalysis {
    /// Signal name (without $ prefix)
    pub name: String,
    /// Where the signal is defined
    pub defined_at: SourceSpan,
    /// What directive/primitive defines this signal
    pub defined_by: String,
    /// All locations where this signal is used
    pub used_at: Vec<SourceSpan>,
    /// Whether this signal is exported to parent scope
    pub is_exported: bool,
}

impl SignalAnalysis {
    /// Create a new signal analysis entry.
    pub fn new(name: String, defined_at: SourceSpan, defined_by: String) -> Self {
        Self {
            name,
            defined_at,
            defined_by,
            used_at: Vec::new(),
            is_exported: false,
        }
    }

    /// Record a usage of this signal.
    pub fn record_usage(&mut self, span: SourceSpan) {
        self.used_at.push(span);
    }

    /// Check if this signal has any usages.
    pub fn is_used(&self) -> bool {
        !self.used_at.is_empty()
    }
}

/// Diagnostic for signal-related issues.
#[derive(Debug, Clone)]
pub struct SignalDiagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub hint: Option<String>,
    /// The signal name this diagnostic concerns (E0408/W0401), so the caller can
    /// prune false positives against the full reactive-source registry.
    pub name: Option<String>,
}

impl SignalDiagnostic {
    /// Convert to a general Diagnostic.
    pub fn into_diagnostic(self) -> Diagnostic {
        let mut diag = if self.code.is_error() {
            Diagnostic::error(self.code, self.message)
        } else {
            Diagnostic::warning(self.code, self.message)
        };
        if let Some(span) = self.span {
            // Convert parser SourceSpan to diagnostics SourceSpan
            diag = diag.with_span(DiagnosticSpan::from(span));
        }
        if let Some(hint) = self.hint {
            diag = diag.with_hint(hint);
        }
        diag
    }
}

// =============================================================================
// Signal Analyzer
// =============================================================================

/// Analyzes signal definitions and usages in a Spacetime file.
pub struct SignalAnalyzer {
    /// All known signals, keyed by name
    signals: HashMap<String, SignalAnalysis>,
    /// Signals referenced but not yet defined (for forward references)
    undefined_references: Vec<(String, SourceSpan)>,
    /// Track signals in current scope for shadowing detection
    scope_stack: Vec<HashSet<String>>,
    /// Shadowing warnings collected
    shadow_warnings: Vec<SignalDiagnostic>,
}

impl SignalAnalyzer {
    /// Create a new signal analyzer.
    pub fn new() -> Self {
        Self {
            signals: HashMap::new(),
            undefined_references: Vec::new(),
            scope_stack: vec![HashSet::new()],
            shadow_warnings: Vec::new(),
        }
    }

    /// Track a signal definition.
    pub fn track_definition(&mut self, name: &str, span: SourceSpan, defined_by: &str) {
        // Check for shadowing
        if self.is_signal_in_outer_scope(name) {
            self.shadow_warnings.push(SignalDiagnostic {
                code: DiagnosticCode::W0403,
                message: format!("Signal '{}' shadows an outer signal", name),
                span: Some(span),
                hint: Some("Consider using a different name to avoid confusion".to_string()),
                name: Some(name.to_string()),
            });
        }

        // Add to current scope
        if let Some(current_scope) = self.scope_stack.last_mut() {
            current_scope.insert(name.to_string());
        }

        // Record the signal definition
        self.signals.insert(
            name.to_string(),
            SignalAnalysis::new(name.to_string(), span, defined_by.to_string()),
        );
    }

    /// Track a signal usage.
    pub fn track_usage(&mut self, name: &str, span: SourceSpan) {
        if let Some(signal) = self.signals.get_mut(name) {
            signal.record_usage(span);
        } else {
            // Signal not yet defined - might be a forward reference or undefined
            self.undefined_references.push((name.to_string(), span));
        }
    }

    /// Track a WRITE to a signal (`$x <- v`). A write both USES the signal and —
    /// in Spacetime's dynamic model — CREATES it if it was not declared (`$x <- v`
    /// on first run allocates x). So a write must not be an E0408 (undefined
    /// reference) and it must keep the signal from being W0401-unused; a later
    /// READ of the same name then resolves against this implicit definition.
    /// (Without this, a signal written by handlers but read only from CSS was
    /// flagged "defined but never used" AND "used but not defined".)
    pub fn track_write(&mut self, name: &str, span: SourceSpan) {
        if let Some(signal) = self.signals.get_mut(name) {
            signal.record_usage(span);
        } else {
            let mut signal = SignalAnalysis::new(name.to_string(), span, "handler write".to_string());
            signal.record_usage(span);
            signal.is_exported = true;
            self.signals.insert(name.to_string(), signal);
        }
    }

    /// Mark a signal as exported to parent scope.
    pub fn mark_exported(&mut self, name: &str) {
        if let Some(signal) = self.signals.get_mut(name) {
            signal.is_exported = true;
        }
    }

    /// Push a new scope for shadowing detection.
    pub fn push_scope(&mut self) {
        self.scope_stack.push(HashSet::new());
    }

    /// Pop the current scope.
    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Check if a signal is defined in an outer scope.
    fn is_signal_in_outer_scope(&self, name: &str) -> bool {
        // Check all scopes except the current one
        for scope in self.scope_stack.iter().rev().skip(1) {
            if scope.contains(name) {
                return true;
            }
        }
        false
    }

    /// Analyze and produce diagnostics. `already_known` names the reactive
    /// sources the enclosing CompileAnalysis already knows (data sources,
    /// computed, locals, %binds yields): a reference to one of THOSE is not an
    /// E0408 even if this standalone analyzer never saw its definition
    /// (GH-26 — a signal published by an imported module, a template, or a
    /// parent scope must not be flagged as undefined).
    pub fn analyze(&self, already_known: &HashSet<String>) -> Vec<SignalDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check for unused signals
        for signal in self.signals.values() {
            if !signal.is_used() && !signal.is_exported {
                diagnostics.push(SignalDiagnostic {
                    code: DiagnosticCode::W0401,
                    message: format!("Signal '{}' is defined but never used", signal.name),
                    span: Some(signal.defined_at),
                    hint: Some(format!(
                        "Remove unused signal or use it in a binding expression. Defined by: {}",
                        signal.defined_by
                    )),
                    name: Some(signal.name.clone()),
                });
            }
        }

        // Check for undefined references
        for (name, span) in &self.undefined_references {
            if !self.signals.contains_key(name) && !already_known.contains(name) {
                diagnostics.push(SignalDiagnostic {
                    code: DiagnosticCode::E0408,
                    message: format!("Signal '{}' is used but not defined", name),
                    span: Some(*span),
                    hint: Some(
                        // The old text named `@pointer`/`@scroll` — directives RETIRED by the
                        // on-cutover wave, so the hint taught the syntax the
                        // compiler now refuses. A hint that recommends removed
                        // syntax is worse than no hint.
                        "Define the signal with a `$name type: value;` declaration, or bind it \
                         from a driver — `@on &.pointer(…) { … }`, `@on &.scroll(…) { … }`"
                            .to_string(),
                    ),
                    name: Some(name.clone()),
                });
            }
        }

        // Add shadowing warnings
        diagnostics.extend(self.shadow_warnings.clone());

        diagnostics
    }

    /// Get all signals for reporting.
    pub fn signals(&self) -> impl Iterator<Item = &SignalAnalysis> {
        self.signals.values()
    }
}

impl Default for SignalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// High-Level Analysis Functions
// =============================================================================

/// Analyze signals and return diagnostics. `already_known` names the reactive
/// sources the CompileAnalysis already registered (data/computed/locals/%binds
/// yields) so a reference to one is not reported as an E0408 undefined signal.
/// Diagnostics are returned UNCONVERTED (SignalDiagnostic) so the caller can
/// prune them before they become user-facing Diagnostic objects.
pub fn analyze_signals_with_diagnostics(
    ast: &StFile,
    already_known: &HashSet<String>,
) -> (Vec<SignalAnalysis>, Vec<SignalDiagnostic>) {
    let mut analyzer = SignalAnalyzer::new();

    // 1. Collect signal definitions from primitives in meta_defs
    for meta_def in &ast.meta_defs {
        if let MetaDef::Primitive(primitive) = meta_def {
            collect_primitive_exports(&mut analyzer, primitive);
        }
    }

    // 2. Collect signal definitions from file-level FormMatches (which bind to primitives)
    for fm in ast.matches.iter().filter(|m| m.selector.is_none()) {
        collect_form_match_signals(&mut analyzer, fm);
    }

    // 3. Scan scopes for signal usages and definitions
    for scope in &ast.scopes {
        collect_scope_signals(&mut analyzer, scope);
    }

    // 3b. Scan `@test` bodies (a String capture, not structured scopes). The
    // test harness mounts fixtures and toggles state inside these — a
    // `$x <- !$x` WRITE there defines `x` (dynamic model), so a top-level
    // `.sel { .is-active: $x }` read must not be an E0408. And a READ of a
    // never-written signal inside a test body IS the undefined-signal case
    // (GH-26: `text <- $definitelyMissing` in a fixture).
    for fm in ast.matches.iter().filter(|m| {
        matches!(m.macro_name.as_str(), "test" | "template" | "template-inline")
    }) {
        if let Some(crate::syntax::CapturedValue::String(body)) = fm.captures.get("body") {
            collect_text_signal_refs(&mut analyzer, body);
        }
    }

    // 4. Generate diagnostics
    let diagnostics = analyzer.analyze(already_known);

    (analyzer.signals().cloned().collect(), diagnostics)
}

/// Collect signal exports from a primitive definition.
fn collect_primitive_exports(analyzer: &mut SignalAnalyzer, primitive: &PrimitiveDefAst) {
    for export in &primitive.body.exports {
        analyzer.track_definition(
            &export.name,
            primitive.span,
            &format!("@{}", primitive.name),
        );
        // Primitive exports are generally available to users, so mark them exported
        analyzer.mark_exported(&export.name);
    }

    // Also check if_blocks for conditional exports
    for if_block in &primitive.body.if_blocks {
        collect_exports_from_list(
            analyzer,
            &if_block.then_body.exports,
            &primitive.name,
            primitive.span,
        );
        if let Some(else_body) = &if_block.else_body {
            collect_exports_from_list(
                analyzer,
                &else_body.exports,
                &primitive.name,
                primitive.span,
            );
        }
    }
}

fn collect_exports_from_list(
    analyzer: &mut SignalAnalyzer,
    exports: &[ExportDecl],
    primitive_name: &str,
    span: SourceSpan,
) {
    for export in exports {
        analyzer.track_definition(&export.name, span, &format!("@{}", primitive_name));
        analyzer.mark_exported(&export.name);
    }
}

/// Collect signals from FormMatches (directives like @pointer, @scroll).
fn collect_form_match_signals(analyzer: &mut SignalAnalyzer, fm: &crate::syntax::FormMatch) {
    // `@state $x : v` / `$x string: "v"` is a local-state DECLARATION — it
    // defines signal `x`. Registered here (not just in analyze_locals, which
    // only sees top-level matches) so a template- or scope-scoped state is a
    // known signal: `text <- $x` must not be an E0408 false positive.
    if fm.macro_name == "local-state" {
        let name: Option<String> = fm
            .get_ident("name")
            .map(|s| s.to_string())
            .or_else(|| fm.get_binding("name").map(|s| s.to_string()));
        if let Some(name) = name {
            let clean = name.trim_start_matches('$').to_string();
            if !clean.is_empty() {
                analyzer.track_definition(&clean, fm.span, "@state");
                analyzer.mark_exported(&clean);
            }
        }
    }

    // `@on &.visible(600ms) as $reveal` DEFINES signal `reveal` — the alias
    // names the driver's progress signal, readable as `var(--st-reveal)`. The
    // `%capture_type on_as { "as" $name:binding }` puts it in a `name` binding
    // capture, and without registering it here E0408 called correct, current
    // syntax an undefined signal (BUG-345). That is the shape the on-cutover
    // migration steers people INTO, so the false positive landed squarely on
    // the recommended surface — it broke demos/starter-pack the moment that
    // tutorial was migrated off the retired positional-name form.
    //
    // Keyed on the CAPTURE, not on a macro-name allowlist: any form declaring
    // an `as` alias is covered, including ones added later.
    //
    // The alias nests one level — `@on(as: {name: $reveal}, driver: {…})` — so
    // it is read out of the `as` capture's own `name` field, not the match's
    // top level.
    if let Some(crate::syntax::CapturedValue::Named(fields)) = fm.captures.get("as") {
        for (field, value) in fields {
            if field != "name" {
                continue;
            }
            let raw = match value {
                crate::syntax::CapturedValue::Binding(b) => b.clone(),
                crate::syntax::CapturedValue::Ident(i) => i.clone(),
                _ => continue,
            };
            let clean = raw.trim_start_matches('$');
            if !clean.is_empty() {
                analyzer.track_definition(clean, fm.span, &format!("@{}", fm.macro_name));
                // The alias is a CAPABILITY like a primitive's yields: binding
                // it and not reading it is not a "defined but never used"
                // warning.
                analyzer.mark_exported(clean);
            }
        }
    }

    // Common primitives and their exported signals
    let primitive_signals: HashMap<&str, Vec<&str>> = [
        ("pointer", vec!["x", "y", "isOver", "isPressed"]),
        (
            "scroll",
            vec![
                "x",
                "y",
                "progress",
                "progressX",
                "progressY",
                "direction",
                "velocity",
            ],
        ),
        (
            "gesture",
            vec![
                "active",
                "deltaX",
                "deltaY",
                "velocityX",
                "velocityY",
                "scale",
                "rotation",
            ],
        ),
        ("tick", vec!["time", "delta", "elapsed"]),
        ("resize", vec!["width", "height", "ratio"]),
        ("intersection", vec!["isIntersecting", "ratio"]),
        ("media", vec!["matches"]),
        ("mutation", vec!["records"]),
        ("fetch", vec!["data", "loading", "error"]),
    ]
    .iter()
    .cloned()
    .collect();

    // Check if this FormMatch is a known primitive
    if let Some(signals) = primitive_signals.get(fm.macro_name.as_str()) {
        for signal_name in signals {
            analyzer.track_definition(signal_name, fm.span, &format!("@{}", fm.macro_name));
            // A primitive's yields are CAPABILITIES (available, not declared):
            // instantiating `@pointer` and reading only `$x` must not warn that
            // `y`/`isOver` are "defined but never used". Mark exported so W0401
            // skips them.
            analyzer.mark_exported(signal_name);
        }
    }

    // Recursively check Block captures for nested directives
    for value in fm.captures.values() {
        if let crate::syntax::CapturedValue::Block(children) = value {
            for child in children {
                collect_form_match_signals(analyzer, child);
            }
        }
    }
}

/// Collect signals from a scope block.
fn collect_scope_signals(analyzer: &mut SignalAnalyzer, scope: &ScopeBlock) {
    analyzer.push_scope();

    // Check FormMatches in the scope for signal definitions
    for fm in &scope.matches {
        collect_form_match_signals(analyzer, fm);
    }

    // CSS reads are uses: a `text <- $events` injection and a `var(--st-x)`
    // read both keep the referenced signal alive. Without this a signal written
    // by handlers and read from CSS was falsely flagged "defined but never used"
    // (GH-26 W0.3).
    for d in &scope.css_declarations {
        collect_signal_refs_in_expr(analyzer, &d.value, d.span);
        collect_st_var_refs_in_expr(analyzer, &d.value, d.span);
    }

    // Handler bodies (@on etc.) both read and WRITE signals; a write is a use.
    for fm in &scope.matches {
        collect_handler_body_refs(analyzer, fm);
    }

    // Check behavior block for signal usages in mutate operations
    for mutate in &scope.behavior.mutates {
        // Check mutate operations for signal references in their arguments
        for op in &mutate.operations {
            for value in op.args.values() {
                collect_signal_refs_in_expr(analyzer, value, SourceSpan::default());
            }
        }
    }

    // Check nested scopes
    for nested_scope in &scope.nested_scopes {
        collect_nested_scope_signals(analyzer, nested_scope);
    }

    analyzer.pop_scope();
}

/// Collect signals from a nested scope.
fn collect_nested_scope_signals(analyzer: &mut SignalAnalyzer, nested_scope: &NestedScope) {
    analyzer.push_scope();

    // Check FormMatches in the nested scope for signal definitions
    for fm in &nested_scope.matches {
        collect_form_match_signals(analyzer, fm);
    }

    // CSS reads are uses (see collect_scope_signals).
    for d in &nested_scope.css_declarations {
        collect_signal_refs_in_expr(analyzer, &d.value, d.span);
        collect_st_var_refs_in_expr(analyzer, &d.value, d.span);
    }

    // Handler bodies in the nested scope.
    for fm in &nested_scope.matches {
        collect_handler_body_refs(analyzer, fm);
    }

    // Check behavior block for signal usages in mutate operations
    for mutate in &nested_scope.behavior.mutates {
        for op in &mutate.operations {
            for value in op.args.values() {
                collect_signal_refs_in_expr(analyzer, value, SourceSpan::default());
            }
        }
    }

    // Recursively check nested scopes
    for inner_scope in &nested_scope.nested_scopes {
        collect_nested_scope_signals(analyzer, inner_scope);
    }

    analyzer.pop_scope();
}

/// Scan an expression string for $signal references.
fn collect_signal_refs_in_expr(analyzer: &mut SignalAnalyzer, expr: &str, span: SourceSpan) {
    // Scan for $identifier patterns, skipping quoted strings (`"…"` `'…'` `` `…` ``)
    // so a `$sig` mention inside a string literal or message is not a signal read
    // (GH-26: `@assert "… $cond …"` was flagged "used but not defined").
    let bytes = expr.as_bytes();
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
        if c == '$' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                // ASCII only: a UTF-8 continuation byte cast to `char` reads as
                // a Latin-1 letter and would extend the slice into the middle
                // of a multi-byte character. Signal names are ASCII idents.
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let name = &expr[start..j];
                if name != "." {
                    // Skip $.property (item reference) patterns
                    analyzer.track_usage(name, span);
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

/// Track a `var(--st-NAME)` CSS read as a use of signal `NAME`. The runtime
/// publishes every signal as the custom property `--st-<name>`, so reading it
/// in CSS is a use — not an E0408 undefined reference, and not W0401-unused.
fn collect_st_var_refs_in_expr(analyzer: &mut SignalAnalyzer, expr: &str, span: SourceSpan) {
    let bytes = expr.as_bytes();
    const MARK: &str = "var(--st-";
    let mut i = 0;
    while i < bytes.len() {
        // Byte-wise advance, char-boundary slicing: a multi-byte glyph in a
        // value (`content: "·"`) must not panic the analyzer.
        if !expr.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if expr[i..].starts_with(MARK) {
            let start = i + MARK.len();
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                analyzer.track_usage(&expr[start..j], span);
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

/// Track every signal a handler body references (reads AND writes) as used. The
/// body capture is a `Named` map with `js_statements`; each statement like
/// `$count <- $count + 1` both writes `count` (LHS) and reads it (RHS) — either
/// keeps the signal alive, so neither E0408 nor W0401 may fire for it.
fn collect_handler_body_refs(analyzer: &mut SignalAnalyzer, fm: &crate::syntax::FormMatch) {
    let Some(crate::syntax::CapturedValue::Named(body)) = fm.captures.get("body") else {
        return;
    };
    let Some(crate::syntax::CapturedValue::Array(stmts)) = body.get("js_statements") else {
        return;
    };
    for st in stmts {
        if let crate::syntax::CapturedValue::String(statement) = st {
            collect_handler_statement_refs(analyzer, statement, fm.span);
        }
    }
}

/// Scan ONE handler statement. A `$lhs <- rhs` write defines (and uses) the
/// LHS signal and only the RHS is a READ subject to E0408 — writing an
/// undefined signal creates it (dynamic model), so `$x <- !$x` toggling a
/// freshly-declared-by-write state must not be flagged undefined.
fn collect_handler_statement_refs(analyzer: &mut SignalAnalyzer, statement: &str, span: SourceSpan) {
    if let Some((lhs, rhs)) = statement.split_once("<-") {
        if let Some(name) = first_signal_ref(lhs) {
            analyzer.track_write(&name, span);
        }
        collect_signal_refs_in_expr(analyzer, rhs, span);
    } else {
        collect_signal_refs_in_expr(analyzer, statement, span);
    }
}

/// Scan raw source TEXT (a @test body, template body, or similar String
/// capture) for signal traffic that the structured walk cannot see:
///   - `as $name` binds a loop/param variable → defined (no E0408 for reads);
///   - `$name <- …` WRITES a signal → defines it (dynamic model) + uses it;
///   - every other `$name` is a READ → E0408 candidate.
/// Comments are skipped so a `// $doc` mention cannot raise a false E0408.
fn collect_text_signal_refs(analyzer: &mut SignalAnalyzer, text: &str) {
    // Pass 1: `as $name` bindings and `$name <-` writes (definitions).
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip comments.
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] as char != '\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        // `as $name`
        if bytes[i] == b'a' && text[i..].starts_with("as ") {
            let after = i + 3;
            if after < bytes.len() && bytes[after] == b'$' {
                if let Some(name) = read_signal_at(text, after) {
                    analyzer.track_definition(&name, SourceSpan::default(), "binding");
                    analyzer.mark_exported(&name);
                }
            }
            i = after;
            continue;
        }
        // `$name <-`
        if bytes[i] == b'$' {
            if let Some(name) = read_signal_at(text, i) {
                let end = i + 1 + name.len();
                let rest = &text[end..];
                if rest.trim_start().starts_with("<-") {
                    // `$x <- v` — a write defines (and uses) the signal.
                    analyzer.track_write(&name, SourceSpan::default());
                } else if text_signal_declared(rest) {
                    // `$x string: "hello"` / `@state $x : 0` — a declaration.
                    analyzer.track_definition(&name, SourceSpan::default(), "state");
                    analyzer.mark_exported(&name);
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    // Pass 2: a `<- $name` read (the signal is the VALUE of a binding, e.g.
    // `text <- $definitelyMissing`) is an E0408 candidate — it renders empty
    // at runtime. This is deliberately NARROW: `@then $x on …`, `on-drop: $f(…)`,
    // `@computed $x`, and `ST.set(…, 'x', …)` all read signals the test harness
    // or runtime creates dynamically, so a blanket `$name` scan would flag a
    // wall of false positives on legitimate dynamic signals.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Byte comparison, NOT `text[i..].starts_with("<-")`: this loop walks
        // byte indices, so slicing the &str panics the moment `i` lands inside a
        // multi-byte char. An em dash in a comment was enough to kill `check`.
        if bytes[i] == b'<' && bytes.get(i + 1) == Some(&b'-') {
            // Skip whitespace between `<-` and the signal: `text <- $x`.
            let mut after = i + 2;
            // `is_ascii_whitespace`, not `(b as char).is_whitespace()`: a UTF-8
            // continuation byte like 0xA0 casts to U+00A0 NO-BREAK SPACE, which
            // reports as whitespace and would walk the cursor through the middle
            // of a character.
            while after < bytes.len() && bytes[after].is_ascii_whitespace() {
                after += 1;
            }
            if after < bytes.len() && bytes[after] == b'$' {
                if let Some(name) = read_signal_at(text, after) {
                    analyzer.track_usage(&name, SourceSpan::default());
                }
            }
            i = after;
            continue;
        }
        i += 1;
    }
}

/// Does the text AFTER a `$name` mark it as a DECLARATION (`$x : v` or
/// `$x type: v`)? This is the local-state surface (`$label string: "hello"`).
/// A bare read (`text <- $x;`, `$x + $y`) is not — the distinguishing token is
/// a `:` following the name (optionally after a type word). `on-drop: $f(…)`
/// has `$f` PRECEDED by `:`, so it is never a declaration here.
fn text_signal_declared(rest: &str) -> bool {
    let rest = rest.trim_start();
    // `$x : v`
    if let Some(after_colon) = rest.strip_prefix(':') {
        return !after_colon.trim_start().is_empty();
    }
    // `$x type : v` — one type word then a colon.
    // Iterate CHARS, not bytes cast to chars: `bytes[j] as char` decodes a UTF-8
    // lead byte as Latin-1, so 0xC3 (first byte of an accented letter) becomes an
    // alphanumeric char, advancing the cursor into the middle of a character and
    // panicking on the `rest[j..]` slice below.
    //
    // `char_indices` keeps the Unicode-aware `is_alphanumeric` intent (a type
    // word may legitimately be non-ASCII) while staying on char boundaries.
    let mut j = 0;
    for (idx, ch) in rest.char_indices() {
        if ch.is_alphanumeric() {
            j = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if j == 0 {
        return false;
    }
    let after_type = rest[j..].trim_start();
    after_type.strip_prefix(':').map(|t| !t.trim_start().is_empty()).unwrap_or(false)
}

/// Read the `$ident` starting at `at` (which must be a `$` byte). Returns the
/// name without the sigil, or None if it is `$.field` (item ref, not a signal).
fn read_signal_at(text: &str, at: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let start = at + 1;
    let mut j = start;
    while j < bytes.len() {
        let ch = bytes[j] as char;
        if ch.is_ascii_alphanumeric() || ch == '_' {
            j += 1;
        } else {
            break;
        }
    }
    if j > start {
        let name = text[start..j].to_string();
        if name != "." {
            return Some(name);
        }
    }
    None
}

/// First `$ident` in a string, if any.
fn first_signal_ref(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let name = text[start..j].to_string();
                if name != "." {
                    return Some(name);
                }
            }
        }
        i += 1;
    }
    None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// The SAME class of crash in the two expression scanners: a CSS value
    /// such as `content: "·"` (a multi-byte glyph) panicked
    /// `collect_st_var_refs_in_expr` (byte index inside a char), and a `$`
    /// followed by an accented letter walked `collect_signal_refs_in_expr`
    /// into the middle of a char. Both took `check` AND `build` down for the
    /// whole project.
    #[test]
    fn expression_scanners_survive_multibyte_values() {
        let span = SourceSpan::new(0, 0);
        for text in ["\"·\"", "$café + 1", "éé var(--st-x) ·", "$é"] {
            let mut a = SignalAnalyzer::new();
            collect_signal_refs_in_expr(&mut a, text, span);
            collect_st_var_refs_in_expr(&mut a, text, span);
        }
        let mut a = SignalAnalyzer::new();
        collect_st_var_refs_in_expr(&mut a, "éé var(--st-x) ·", span);
        assert!(
            a.undefined_references.iter().any(|(name, _)| name == "x"),
            "the read after the multi-byte text must still be tracked"
        );
    }

    /// A non-ASCII character anywhere before a `<-` must not crash the scan.
    ///
    /// This pass walked BYTE indices but sliced the string with `text[i..]`, so
    /// the moment `i` landed inside a multi-byte char Rust panicked. An em dash
    /// in a comment was enough.
    ///
    /// The blast radius was much larger than a crashed file: `spacetime check
    /// tests` died on the first such file and reported "0 files", which reads
    /// exactly like a clean scan. A fallout measurement run against that output
    /// concluded "zero hits" from a crash — the panic laundered itself into a
    /// green result. Hence the gate: a scanner that cannot survive punctuation
    /// silently invalidates every measurement taken through it.
    #[test]
    fn a_multibyte_char_does_not_panic_the_signal_scan() {
        let mut analyzer = SignalAnalyzer::new();
        // Em dash before the `<-`, mid-char byte index lands inside it.
        collect_text_signal_refs(&mut analyzer, "@eval (a \u{2014} b <- 1)");

        // Non-ASCII in many shapes: accents, CJK, emoji, RTL.
        for text in [
            "caf\u{e9} <- $x",
            "\u{6f22}\u{5b57} <- $y",
            "\u{1f680}\u{1f680} <- $z",
            "\u{5b57}",
            "\u{2014}",
            "$caf\u{e9}",
        ] {
            let mut a = SignalAnalyzer::new();
            collect_text_signal_refs(&mut a, text);
        }
    }

    /// The scan still SEES a signal read that sits after a multi-byte char —
    /// not-panicking is only half the contract; skipping the text would be a
    /// silent regression that a crash-only gate would happily accept.
    #[test]
    fn a_signal_read_after_a_multibyte_char_is_still_tracked() {
        let mut analyzer = SignalAnalyzer::new();
        collect_text_signal_refs(&mut analyzer, "\u{2014} text <- $afterDash");
        // An undefined read lands in `undefined_references` — that IS the
        // tracking, and is what E0408 later reports.
        assert!(
            analyzer
                .undefined_references
                .iter()
                .any(|(name, _)| name == "afterDash"),
            "signal read after an em dash must still be tracked, got: {:?}",
            analyzer.undefined_references
        );
    }

    #[test]
    fn test_signal_analyzer_basic() {
        let mut analyzer = SignalAnalyzer::new();

        // Define a signal
        analyzer.track_definition("x", SourceSpan::new(0, 10), "@pointer");

        // Use it
        analyzer.track_usage("x", SourceSpan::new(20, 25));

        let diagnostics = analyzer.analyze(&HashSet::new());
        assert!(
            diagnostics.is_empty(),
            "No diagnostics expected for used signal"
        );
    }

    #[test]
    fn test_unused_signal_warning() {
        let mut analyzer = SignalAnalyzer::new();

        // Define a signal but don't use it
        analyzer.track_definition("unused", SourceSpan::new(0, 10), "@pointer");

        let diagnostics = analyzer.analyze(&HashSet::new());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::W0401);
        assert!(diagnostics[0].message.contains("unused"));
    }

    #[test]
    fn test_undefined_signal_error() {
        let mut analyzer = SignalAnalyzer::new();

        // Use a signal without defining it
        analyzer.track_usage("missing", SourceSpan::new(0, 10));

        let diagnostics = analyzer.analyze(&HashSet::new());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::E0408);
        assert!(diagnostics[0].message.contains("missing"));
    }

    #[test]
    fn test_exported_signal_no_warning() {
        let mut analyzer = SignalAnalyzer::new();

        // Define an exported signal (no usage required)
        analyzer.track_definition("exported", SourceSpan::new(0, 10), "@pointer");
        analyzer.mark_exported("exported");

        let diagnostics = analyzer.analyze(&HashSet::new());
        assert!(diagnostics.is_empty(), "No warning for exported signals");
    }

    #[test]
    fn test_signal_shadowing_warning() {
        let mut analyzer = SignalAnalyzer::new();

        // Define signal in outer scope and mark as exported (to avoid unused warning)
        analyzer.track_definition("x", SourceSpan::new(0, 10), "@pointer");
        analyzer.mark_exported("x");

        // Push new scope and define same signal (also mark exported)
        analyzer.push_scope();
        analyzer.track_definition("x", SourceSpan::new(20, 30), "@scroll");
        analyzer.mark_exported("x");

        let diagnostics = analyzer.analyze(&HashSet::new());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::W0403);
        assert!(diagnostics[0].message.contains("shadows"));
    }

    #[test]
    fn test_collect_signal_refs_in_expr() {
        let mut analyzer = SignalAnalyzer::new();

        // Define some signals
        analyzer.track_definition("x", SourceSpan::new(0, 5), "@pointer");
        analyzer.track_definition("y", SourceSpan::new(5, 10), "@pointer");

        // Scan an expression with signal references
        collect_signal_refs_in_expr(&mut analyzer, "$x + $y * 2", SourceSpan::new(20, 30));

        // Both signals should be marked as used
        let diagnostics = analyzer.analyze(&HashSet::new());
        assert!(diagnostics.is_empty(), "Both signals should be used");
    }

    #[test]
    fn test_skip_item_reference() {
        let mut analyzer = SignalAnalyzer::new();

        // Scan expression with $.field (item reference, not signal)
        collect_signal_refs_in_expr(&mut analyzer, "$.name + $.price", SourceSpan::new(0, 20));

        // No undefined signal errors for $. patterns
        let diagnostics = analyzer.analyze(&HashSet::new());
        assert!(
            diagnostics.is_empty(),
            "$.field should not be treated as signal"
        );
    }
}
