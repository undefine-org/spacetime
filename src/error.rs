//! Unified error types for Spacetime using miette for beautiful diagnostics.
//!
//! These errors provide rich context including source snippets, span highlighting,
//! and helpful suggestions.

use miette::{Diagnostic, NamedSource, SourceSpan as MietteSpan};
use thiserror::Error;

/// Result type alias using SpacetimeError
pub type Result<T> = std::result::Result<T, SpacetimeError>;

/// Top-level error container that can hold any Spacetime error
#[derive(Debug, Error, Diagnostic)]
#[error(transparent)]
#[diagnostic(transparent)]
pub struct SpacetimeError(#[from] pub Box<dyn Diagnostic + Send + Sync>);

impl SpacetimeError {
    pub fn new<E: Diagnostic + Send + Sync + 'static>(error: E) -> Self {
        Self(Box::new(error))
    }
}

// === Parse Errors ===

#[derive(Debug, Error, Diagnostic)]
#[error("unexpected token")]
#[diagnostic(code(spacetime::parse::unexpected_token))]
pub struct UnexpectedTokenError {
    pub found: String,
    pub expected: Vec<String>,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("unexpected `{found}` here")]
    pub span: MietteSpan,

    #[help]
    pub help: Option<String>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("unclosed delimiter `{delimiter}`")]
#[diagnostic(code(spacetime::parse::unclosed_delimiter))]
pub struct UnclosedDelimiterError {
    pub delimiter: char,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("this `{delimiter}` is never closed")]
    pub open_span: MietteSpan,

    #[label("expected closing `{}`", closing_delimiter(.delimiter))]
    pub expected_span: MietteSpan,
}

fn closing_delimiter(open: &char) -> char {
    match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        _ => *open,
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("invalid number format")]
#[diagnostic(code(spacetime::parse::invalid_number))]
pub struct InvalidNumberError {
    pub value: String,
    pub reason: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("{reason}")]
    pub span: MietteSpan,
}

#[derive(Debug, Error, Diagnostic)]
#[error("invalid duration")]
#[diagnostic(code(spacetime::parse::invalid_duration))]
pub struct InvalidDurationError {
    pub value: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("invalid duration format")]
    pub span: MietteSpan,

    #[help]
    pub help: String, // e.g., "valid units are: ms, s, us, m, fps"
}

#[derive(Debug, Error, Diagnostic)]
#[error("invalid color")]
#[diagnostic(code(spacetime::parse::invalid_color))]
pub struct InvalidColorError {
    pub value: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("invalid color format")]
    pub span: MietteSpan,

    #[help]
    pub help: String, // e.g., "use #rgb, #rrggbb, or #rrggbbaa format"
}

// === Resolution Errors ===

#[derive(Debug, Error, Diagnostic)]
#[error("macro not found: @{name}")]
#[diagnostic(code(spacetime::resolve::macro_not_found))]
pub struct MacroNotFoundError {
    pub name: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("this macro doesn't exist")]
    pub span: MietteSpan,

    #[help]
    pub help: Option<String>, // e.g., "did you mean `@data`?"
}

#[derive(Debug, Error, Diagnostic)]
#[error("missing required parameter: `{param}`")]
#[diagnostic(code(spacetime::resolve::missing_param))]
pub struct MissingParameterError {
    pub param: String,
    pub macro_name: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("parameter `{param}` is required by @{macro_name}")]
    pub span: MietteSpan,

    #[help]
    pub help: String, // e.g., "add `duration: 500ms` to the argument list"
}

#[derive(Debug, Error, Diagnostic)]
#[error("unknown parameter: `{param}`")]
#[diagnostic(code(spacetime::resolve::unknown_param))]
pub struct UnknownParameterError {
    pub param: String,
    pub macro_name: String,
    pub available: Vec<String>,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("`{param}` is not a valid parameter for @{macro_name}")]
    pub span: MietteSpan,

    #[help]
    pub help: Option<String>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("type mismatch: expected {expected}, found {found}")]
#[diagnostic(code(spacetime::resolve::type_mismatch))]
pub struct TypeMismatchError {
    pub expected: String,
    pub found: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("expected {expected}")]
    pub span: MietteSpan,
}

// === Reference Errors ===

#[derive(Debug, Error, Diagnostic)]
#[error("undefined variable: ${name}")]
#[diagnostic(code(spacetime::resolve::undefined_variable))]
pub struct UndefinedVariableError {
    pub name: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("${name} is not defined")]
    pub span: MietteSpan,

    #[help]
    pub help: Option<String>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("undefined element: &{name}")]
#[diagnostic(code(spacetime::resolve::undefined_element))]
pub struct UndefinedElementError {
    pub name: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("&{name} is not defined")]
    pub span: MietteSpan,

    #[help]
    pub help: Option<String>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("undefined preset: ~{name}")]
#[diagnostic(code(spacetime::resolve::undefined_preset))]
pub struct UndefinedPresetError {
    pub name: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("~{name} is not defined")]
    pub span: MietteSpan,

    #[help]
    pub help: Option<String>,
}

// === Import Errors ===

#[derive(Debug, Error, Diagnostic)]
#[error("circular import detected")]
#[diagnostic(code(spacetime::import::circular))]
pub struct CircularImportError {
    pub chain: Vec<String>,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("this import creates a cycle")]
    pub span: MietteSpan,

    #[help]
    pub help: String, // Shows the import cycle
}

#[derive(Debug, Error, Diagnostic)]
#[error("file not found: {path}")]
#[diagnostic(code(spacetime::import::not_found))]
pub struct FileNotFoundError {
    pub path: String,

    #[source_code]
    pub src: NamedSource<String>,

    #[label("this file doesn't exist")]
    pub span: MietteSpan,
}

// === Helper Functions ===

/// Create a NamedSource for error reporting
pub fn named_source(filename: &str, content: &str) -> NamedSource<String> {
    NamedSource::new(filename, content.to_string())
}

/// Suggest similar names using Levenshtein distance
pub fn suggest_similar<'a>(
    needle: &str,
    haystack: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let needle_lower = needle.to_lowercase();
    let mut best = None;
    let mut best_dist = usize::MAX;

    for candidate in haystack {
        let dist = levenshtein(&needle_lower, &candidate.to_lowercase());
        if dist < best_dist && dist <= 3 {
            best_dist = dist;
            best = Some(candidate.to_string());
        }
    }

    best
}

/// Simple Levenshtein distance implementation
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let len_a = a.len();
    let len_b = b.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut matrix = vec![vec![0usize; len_b + 1]; len_a + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len_a + 1) {
        row[0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len_a][len_b]
}

/// Render a miette Report to a plain-text string (no ANSI codes).
/// Used for browser error modal display.
pub fn render_miette_plain(report: &miette::Report) -> String {
    use miette::{GraphicalReportHandler, GraphicalTheme};
    let mut buf = String::new();
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let _ = handler.render_report(&mut buf, report.as_ref());
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_similar() {
        let names = ["data", "each", "load", "fn", "type"];

        assert_eq!(
            suggest_similar("dataa", names.iter().copied()),
            Some("data".to_string())
        );
        assert_eq!(
            suggest_similar("eech", names.iter().copied()),
            Some("each".to_string())
        );
        assert_eq!(suggest_similar("xyzzzz", names.iter().copied()), None);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("hello", "hello"), 0);
        assert_eq!(levenshtein("hello", "hallo"), 1);
        assert_eq!(levenshtein("hello", "world"), 4);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn test_error_construction() {
        let err = MacroNotFoundError {
            name: "dataa".to_string(),
            src: named_source("test.st", "@dataa() {}"),
            span: (0..6).into(),
            help: Some("did you mean `@data`?".to_string()),
        };

        assert!(err.to_string().contains("@dataa"));
    }
}
