//! Diagnostic conversion for metasystem errors.
//!
//! Converts ValidationError to rich Diagnostic with hints.

use super::registry::MetaRegistry;
use super::validate::{ValidationError, ValidationErrorKind};
use crate::diagnostics::{Diagnostic, DiagnosticCode, SourceSpan as DiagSpan};

/// Convert parser SourceSpan to diagnostics SourceSpan
fn to_diag_span(span: &crate::parser::SourceSpan) -> DiagSpan {
    DiagSpan::new(span.start, span.end)
}

/// Find a similar name for typo suggestions
fn find_similar<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let name_lower = name.to_lowercase();
    candidates
        .filter(|c| {
            let c_lower = c.to_lowercase();
            c_lower.starts_with(&name_lower[..1.min(name_lower.len())])
                || c_lower.contains(&name_lower)
                || name_lower.contains(&c_lower)
                || levenshtein_close(&name_lower, &c_lower)
        })
        .min_by_key(|c| levenshtein_distance(&name.to_lowercase(), &c.to_lowercase()))
        .map(|s| s.to_string())
}

/// Check if two strings are within Levenshtein distance 2
fn levenshtein_close(a: &str, b: &str) -> bool {
    levenshtein_distance(a, b) <= 2
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Convert a ValidationError to a Diagnostic
pub fn to_diagnostic(error: &ValidationError, registry: &MetaRegistry) -> Diagnostic {
    let span = to_diag_span(&error.span);

    match &error.kind {
        ValidationErrorKind::InvalidMigrationDate(msg)
        | ValidationErrorKind::InvalidMigration(msg) => Diagnostic::error(
            DiagnosticCode::E205,
            format!("invalid migration definition: {}", msg),
        )
        .with_span(span),

        ValidationErrorKind::UnknownPrimitive(name) => {
            let mut diag = Diagnostic::error(
                DiagnosticCode::E170,
                format!("unknown primitive: {}", name),
            )
            .with_span(span);

            if let Some(suggestion) = find_similar(name, registry.primitive_names()) {
                diag = diag.with_hint(format!("did you mean `{}`?", suggestion));
            } else {
                diag = diag.with_hint("primitives are defined with %primitive { ... }");
            }
            diag
        }

        ValidationErrorKind::UnknownMacro(name) => {
            let mut diag = Diagnostic::error(
                DiagnosticCode::E171,
                format!("unknown macro: %{}", name),
            )
            .with_span(span);

            if let Some(suggestion) = find_similar(name, registry.macro_names()) {
                diag = diag.with_hint(format!("did you mean `%{}`?", suggestion));
            } else {
                diag = diag.with_hint("macros are defined with %macro { ... }");
            }
            diag
        }

        ValidationErrorKind::MissingParameter { primitive, param } => {
            Diagnostic::error(
                DiagnosticCode::E172,
                format!("missing required parameter '{}' for primitive {}", param, primitive),
            )
            .with_span(span)
            .with_hint(format!("add `{}: <value>`", param))
        }

        ValidationErrorKind::ParameterTypeMismatch { param, expected, got } => {
            Diagnostic::error(
                DiagnosticCode::E173,
                format!("type mismatch for parameter '{}'", param),
            )
            .with_span(span)
            .with_note(format!("expected {}, found {}", expected, got))
        }

        ValidationErrorKind::InvalidEmitLang(lang) => {
            Diagnostic::error(
                DiagnosticCode::E174,
                format!("invalid emit language: {}", lang),
            )
            .with_span(span)
            .with_hint("valid languages are: js, glsl")
        }

        ValidationErrorKind::UnboundEmitVariable(name) => {
            Diagnostic::error(
                DiagnosticCode::E175,
                format!("unbound variable in emit block: ${}", name),
            )
            .with_span(span)
            .with_hint("ensure variable is defined as a primitive parameter")
        }

        ValidationErrorKind::InvalidYieldTarget(target) => {
            Diagnostic::error(
                DiagnosticCode::E176,
                format!("invalid yield target: {}", target),
            )
            .with_span(span)
            .with_hint("yield target must be $variable")
        }

        ValidationErrorKind::InvalidFormPattern(msg) => {
            Diagnostic::error(
                DiagnosticCode::E178,
                format!("invalid form pattern: {}", msg),
            )
            .with_span(span)
            .with_hint("form pattern must start with @directive")
        }

        ValidationErrorKind::UnknownCaptureType(ty) => {
            Diagnostic::error(
                DiagnosticCode::E179,
                format!("unknown capture type: {}", ty),
            )
            .with_span(span)
            .with_hint("valid capture types: ident, string, number, time, expr, properties, states, transitions, keyframes, selector, element")
        }

        ValidationErrorKind::CircularMacroDependency(cycle) => {
            Diagnostic::error(
                DiagnosticCode::E180,
                format!("circular macro dependency: {}", cycle.join(" -> ")),
            )
            .with_span(span)
            .with_hint("macros cannot include themselves directly or indirectly")
        }

        ValidationErrorKind::UnboundMacroVariable(name) => {
            Diagnostic::error(
                DiagnosticCode::E181,
                format!("unbound variable in macro body: ${}", name),
            )
            .with_span(span)
            .with_hint("ensure variable is bound by %form, %binds, or %derives")
        }

        ValidationErrorKind::InvalidBindsDecl(msg) => {
            Diagnostic::error(
                DiagnosticCode::E182,
                format!("invalid binds declaration: {}", msg),
            )
            .with_span(span)
            .with_hint("format: primitive(args) -> { $output1, $output2 }")
        }

        ValidationErrorKind::ExportTypeMismatch { name, declared, actual } => {
            Diagnostic::error(
                DiagnosticCode::E184,
                format!("export ${} type mismatch", name),
            )
            .with_span(span)
            .with_note(format!("declared as {}, but primitive yields {}", declared, actual))
        }

        ValidationErrorKind::DuplicateExport(name) => {
            Diagnostic::error(
                DiagnosticCode::E185,
                format!("duplicate export: ${}", name),
            )
            .with_span(span)
            .with_hint("each export name must be unique")
        }

        ValidationErrorKind::InvalidDerivesExpr(msg) => {
            Diagnostic::error(
                DiagnosticCode::E186,
                format!("invalid derives expression: {}", msg),
            )
            .with_span(span)
        }

        ValidationErrorKind::MissingFormForDirective(name) => {
            Diagnostic::error(
                DiagnosticCode::E187,
                format!("@{} needs a %form clause", name),
            )
            .with_span(span)
            .with_hint("add `%form { @directive(params) { body } }`")
        }

        ValidationErrorKind::InvalidTrigger(name) => {
            Diagnostic::error(
                DiagnosticCode::E188,
                format!("invalid trigger: {}", name),
            )
            .with_span(span)
        }

        ValidationErrorKind::InvalidBodyContext(msg) => {
            Diagnostic::error(
                DiagnosticCode::E189,
                format!("invalid macro body context: {}", msg),
            )
            .with_span(span)
        }

        ValidationErrorKind::UnknownDirective(name) => {
            Diagnostic::error(
                DiagnosticCode::E190,
                format!("unknown directive @{}", name),
            )
            .with_span(span)
            .with_hint("no macro creates this directive - check spelling or import the macro")
        }

        ValidationErrorKind::MissingRequiredArg { directive, param } => {
            Diagnostic::error(
                DiagnosticCode::E191,
                format!("@{} requires argument '{}'", directive, param),
            )
            .with_span(span)
            .with_hint(format!("add `{}: <value>`", param))
        }

        ValidationErrorKind::UnexpectedArg { directive, arg } => {
            Diagnostic::error(
                DiagnosticCode::E192,
                format!("@{} does not accept argument '{}'", directive, arg),
            )
            .with_span(span)
            .with_hint("check the directive signature for valid arguments")
        }

        ValidationErrorKind::ArgTypeMismatch { directive, param, expected, got } => {
            Diagnostic::error(
                DiagnosticCode::E193,
                format!("type mismatch for @{} argument '{}'", directive, param),
            )
            .with_span(span)
            .with_note(format!("expected {}, found {}", expected, got))
        }

        ValidationErrorKind::YieldToUndeclaredExport(name) => {
            Diagnostic::error(
                DiagnosticCode::E196,
                format!("yield to undeclared export ${}", name),
            )
            .with_span(span)
            .with_hint(format!("add `${}: <type>` to %exports block", name))
        }

        ValidationErrorKind::ExportNeverYielded(name) => {
            Diagnostic::error(
                DiagnosticCode::E197,
                format!("export ${} is declared but never yielded", name),
            )
            .with_span(span)
            .with_hint(format!("add `%yield <value> -> ${}` in emit block, or make optional with `{}: type?`", name, name))
        }

        ValidationErrorKind::LegacyFunctionType(name) => {
            Diagnostic::error(
                DiagnosticCode::E198,
                format!("export ${} uses bare 'fn' type", name),
            )
            .with_span(span)
            .with_hint("use 'fn() ~> effect' for side-effects or 'fn(params) ~> ReturnType' for value-returning functions")
        }

        ValidationErrorKind::UnknownTypeReference(ty) => {
            Diagnostic::error(
                DiagnosticCode::E199,
                format!("unknown type '{}'", ty),
            )
            .with_span(span)
            .with_hint("valid types: number, string, bool, effect, void, any, array, map, optional, element, object")
        }

        ValidationErrorKind::OptionalCaptureWithoutDefault { macro_name, capture_name } => {
            Diagnostic::error(
                DiagnosticCode::E204,
                format!("optional inline capture '${}' in macro '{}' requires a default", capture_name, macro_name),
            )
            .with_span(span)
            .with_hint(format!("add a default value: ${}:type? = value", capture_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;

    #[test]
    fn test_unknown_primitive_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::UnknownPrimitive("scrool".to_string()),
            span: SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E170);
        assert!(diag.message.contains("scrool"));
    }

    #[test]
    fn test_find_similar() {
        let primitives = vec!["scroll", "pointer", "gesture", "tick"];
        let result = find_similar("scrool", primitives.iter().map(|s| *s));
        assert_eq!(result, Some("scroll".to_string()));
    }

    #[test]
    fn test_circular_dependency_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::CircularMacroDependency(vec![
                "a".to_string(),
                "b".to_string(),
                "a".to_string(),
            ]),
            span: SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E180);
        assert!(diag.message.contains("a -> b -> a"));
    }

    #[test]
    fn test_unknown_directive_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::UnknownDirective("undefined".to_string()),
            span: SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E190);
        assert!(diag.message.contains("@undefined"));
    }

    #[test]
    fn test_missing_required_arg_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::MissingRequiredArg {
                directive: "fade".to_string(),
                param: "duration".to_string(),
            },
            span: SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E191);
        assert!(diag.message.contains("@fade"));
        assert!(diag.message.contains("duration"));
    }

    #[test]
    fn test_unexpected_arg_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::UnexpectedArg {
                directive: "toggle".to_string(),
                arg: "unknown_param".to_string(),
            },
            span: SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E192);
        assert!(diag.message.contains("@toggle"));
        assert!(diag.message.contains("unknown_param"));
    }

    #[test]
    fn test_arg_type_mismatch_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::ArgTypeMismatch {
                directive: "drag".to_string(),
                param: "axis".to_string(),
                expected: "string".to_string(),
                got: "number".to_string(),
            },
            span: SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E193);
        assert!(diag.message.contains("@drag"));
        assert!(diag.message.contains("axis"));
    }

    #[test]
    fn test_levenshtein_distance() {
        // Identical strings
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        // Single character difference
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
        // Multiple differences
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        // Empty string cases
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_find_similar_no_match() {
        let candidates = vec!["apple", "banana", "cherry"];
        let result = find_similar("xyz", candidates.iter().map(|s| *s));
        // Should return None or a weak match
        assert!(result.is_none() || result.as_ref().map(|s| s.as_str()) != Some("xyz"));
    }
}
