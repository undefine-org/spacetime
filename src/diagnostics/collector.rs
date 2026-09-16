//! Diagnostic collection and rendering.

use super::codes::DiagnosticCode;
use super::span::SourceSpan;

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Fatal error that prevents compilation
    Error,
    /// Warning that doesn't prevent compilation
    Warning,
}

/// A single diagnostic message.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// The diagnostic code (e.g., E102, W001)
    pub code: DiagnosticCode,
    /// Severity level
    pub severity: Severity,
    /// Primary message describing the issue
    pub message: String,
    /// Source location (if available)
    pub span: Option<SourceSpan>,
    /// Suggested fix or hint
    pub hint: Option<String>,
    /// Additional notes
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create a new error diagnostic.
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span: None,
            hint: None,
            notes: Vec::new(),
        }
    }

    /// Create a new warning diagnostic.
    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            hint: None,
            notes: Vec::new(),
        }
    }

    /// Add a source span to this diagnostic.
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Add a hint to this diagnostic.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add a note to this diagnostic.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Convert to a miette-compatible report.
    ///
    /// This bridges the existing Diagnostic system to miette, enabling
    /// gradual migration. The returned report can be displayed with
    /// miette's fancy rendering.
    pub fn to_miette_report(&self, source: &str, file_path: Option<&str>) -> miette::Report {
        let severity = self.code.to_miette_severity();
        let code = self.code.as_miette_code();

        let mut labels = Vec::new();
        if let Some(span) = &self.span {
            labels.push(miette::LabeledSpan::at(span.start..span.end, &self.message));
        }

        let source_name = file_path.unwrap_or("<input>");
        let named_source = miette::NamedSource::new(source_name, source.to_string());

        let mut builder = miette::MietteDiagnostic::new(&self.message)
            .with_code(code)
            .with_severity(severity);

        if let Some(hint) = &self.hint {
            builder = builder.with_help(hint.clone());
        }

        if let Some(span) = &self.span {
            builder =
                builder.with_label(miette::LabeledSpan::at(span.start..span.end, &self.message));
        }

        miette::Report::new(builder).with_source_code(named_source)
    }
}

/// Accumulates diagnostics during compilation.
#[derive(Debug, Default)]
pub struct DiagnosticCollector {
    /// The source code being compiled
    source: String,
    /// File path (optional)
    file_path: Option<String>,
    /// Accumulated diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollector {
    /// Create a new collector for the given source.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            file_path: None,
            diagnostics: Vec::new(),
        }
    }

    /// Set the file path for error messages.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Emit a diagnostic.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        // Collapse an identical diagnostic already collected.
        //
        // One authoring mistake can reach the collector by several routes: a
        // construct is carried by BOTH the flat `matches` list and the `scopes`
        // tree, and a whole-page pass's findings arrive both on
        // `StFile.diagnostics` and (after compilation) on `pipeline_errors`.
        // Without this the same error prints two or three times, which trains
        // authors to skim errors — and skimming is how a real error gets missed.
        //
        // Identity is (code, message, span): two DIFFERENT sites that happen to
        // produce the same message still both report, because their spans
        // differ.
        let is_dupe = self.diagnostics.iter().any(|d| {
            d.code == diagnostic.code
                && d.message == diagnostic.message
                && d.span == diagnostic.span
        });
        if !is_dupe {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Emit an error diagnostic.
    pub fn error(
        &mut self,
        code: DiagnosticCode,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) {
        let mut diag = Diagnostic::error(code, message);
        if let Some(s) = span {
            diag = diag.with_span(s);
        }
        self.emit(diag);
    }

    /// Emit a warning diagnostic.
    pub fn warning(
        &mut self,
        code: DiagnosticCode,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) {
        let mut diag = Diagnostic::warning(code, message);
        if let Some(s) = span {
            diag = diag.with_span(s);
        }
        self.emit(diag);
    }

    /// Emit an error with a hint.
    pub fn error_with_hint(
        &mut self,
        code: DiagnosticCode,
        message: impl Into<String>,
        span: Option<SourceSpan>,
        hint: impl Into<String>,
    ) {
        let mut diag = Diagnostic::error(code, message).with_hint(hint);
        if let Some(s) = span {
            diag = diag.with_span(s);
        }
        self.emit(diag);
    }

    /// Check if any errors have been collected.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Get the number of warnings.
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Get all diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Render all diagnostics as miette-formatted strings.
    ///
    /// Uses miette's built-in rendering for rich error output with
    /// source context, labels, and help text.
    pub fn render_miette(&self) -> String {
        use std::fmt::Write;
        let mut output = String::new();
        for diag in &self.diagnostics {
            let report = diag.to_miette_report(&self.source, self.file_path.as_deref());
            let _ = writeln!(output, "{:?}", report);
        }
        output
    }

    /// Render all diagnostics as a formatted string.
    pub fn render(&self) -> String {
        let mut output = String::new();

        for diag in &self.diagnostics {
            output.push_str(&self.render_diagnostic(diag));
            output.push('\n');
        }

        // Summary
        let errors = self.error_count();
        let warnings = self.warning_count();
        if errors > 0 || warnings > 0 {
            output.push_str(&format!("\n{} error(s), {} warning(s)\n", errors, warnings));
        }

        output
    }

    /// Render a single diagnostic.
    fn render_diagnostic(&self, diag: &Diagnostic) -> String {
        let mut output = String::new();

        // Header: error[E102]: message
        let severity_str = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        output.push_str(&format!(
            "{}[{}]: {}\n",
            severity_str, diag.code, diag.message
        ));

        // Location and source context
        if let Some(span) = &diag.span {
            let (line, col, line_text) = span.resolve(&self.source);
            let file = self.file_path.as_deref().unwrap_or("<input>");

            // Location line
            output.push_str(&format!("  --> {}:{}:{}\n", file, line, col));

            // Source context
            let line_num_width = line.to_string().len();
            output.push_str(&format!("{:width$} |\n", "", width = line_num_width));
            output.push_str(&format!("{} | {}\n", line, line_text));

            // Underline
            let underline_start = col.saturating_sub(1);
            let underline_len = span
                .len()
                .min(line_text.len().saturating_sub(underline_start))
                .max(1);
            output.push_str(&format!(
                "{:width$} | {:>start$}{}\n",
                "",
                "",
                "^".repeat(underline_len),
                width = line_num_width,
                start = underline_start
            ));
        }

        // Hint
        if let Some(hint) = &diag.hint {
            output.push_str(&format!("  = hint: {}\n", hint));
        }

        // Notes
        for note in &diag.notes {
            output.push_str(&format!("  = note: {}\n", note));
        }

        output
    }
}

/// Calculate Levenshtein distance between two strings.
/// Used for "did you mean?" suggestions.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for (i, a_char) in a.chars().enumerate() {
        for (j, b_char) in b.chars().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Find the best match for a string from a list of candidates.
/// Returns None if no close match found (distance > threshold).
pub fn find_similar<'a>(needle: &str, candidates: &[&'a str], threshold: usize) -> Option<&'a str> {
    let needle_lower = needle.to_lowercase();
    candidates
        .iter()
        .map(|&c| (c, levenshtein_distance(&needle_lower, &c.to_lowercase())))
        .filter(|(_, dist)| *dist <= threshold)
        .min_by_key(|(_, dist)| *dist)
        .map(|(c, _)| c)
}

/// Find up to `max_results` similar matches, ordered by distance.
///
/// Uses a smart threshold: distance <= 3 AND distance < half the needle length.
/// This avoids nonsensical suggestions for short names (e.g., "on" suggesting "fn").
pub fn find_similar_multiple<'a>(
    needle: &str,
    candidates: &[&'a str],
    max_results: usize,
) -> Vec<(&'a str, usize)> {
    let needle_lower = needle.to_lowercase();
    let needle_len = needle_lower.chars().count();
    let max_dist = 3.min(if needle_len > 1 { needle_len / 2 } else { 1 });

    let mut matches: Vec<(&str, usize)> = candidates
        .iter()
        .map(|&c| (c, levenshtein_distance(&needle_lower, &c.to_lowercase())))
        .filter(|(_, dist)| *dist <= max_dist && *dist > 0)
        .collect();

    matches.sort_by_key(|(_, dist)| *dist);
    matches.truncate(max_results);
    matches
}

/// Format a "did you mean?" suggestion string from similar matches.
///
/// Returns `None` if no matches. Examples:
/// - Single match: `"did you mean 'opacity'?"`
/// - Multiple: `"did you mean one of: 'opacity' (1 edit), 'opaque' (2 edits)?"`
pub fn format_suggestion(matches: &[(&str, usize)]) -> Option<String> {
    match matches.len() {
        0 => None,
        1 => Some(format!("did you mean '{}'?", matches[0].0)),
        _ => {
            let items: Vec<String> = matches
                .iter()
                .map(|(name, dist)| {
                    let edit_word = if *dist == 1 { "edit" } else { "edits" };
                    format!("'{}' ({} {})", name, dist, edit_word)
                })
                .collect();
            Some(format!("did you mean one of: {}?", items.join(", ")))
        }
    }
}
