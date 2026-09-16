//! Tests for the diagnostics module.

use super::*;

#[test]
fn test_span_resolution_single_line() {
    let source = "opacity: 0 -> 1;";
    let span = SourceSpan::new(0, 7); // "opacity"

    let (line, col, line_text) = span.resolve(source);

    assert_eq!(line, 1);
    assert_eq!(col, 1);
    assert_eq!(line_text, "opacity: 0 -> 1;");
}

#[test]
fn test_span_resolution_multi_line() {
    let source = "first line\nsecond line\nthird line";
    let span = SourceSpan::new(11, 17); // "second"

    let (line, col, line_text) = span.resolve(source);

    assert_eq!(line, 2);
    assert_eq!(col, 1);
    assert_eq!(line_text, "second line");
}

#[test]
fn test_span_resolution_middle_of_line() {
    let source = ".hero {\n    opactiy: 0 -> 1;\n}";
    let span = SourceSpan::new(12, 19); // "opactiy"

    let (line, col, line_text) = span.resolve(source);

    assert_eq!(line, 2);
    assert_eq!(col, 5);
    assert_eq!(line_text, "    opactiy: 0 -> 1;");
}

#[test]
fn test_span_merge() {
    let span1 = SourceSpan::new(5, 10);
    let span2 = SourceSpan::new(15, 20);

    let merged = span1.merge(span2);

    assert_eq!(merged.start, 5);
    assert_eq!(merged.end, 20);
}

#[test]
fn test_span_len() {
    let span = SourceSpan::new(5, 12);
    assert_eq!(span.len(), 7);
}

#[test]
fn test_diagnostic_collector_error() {
    let source = "opacity: 0 -> 1;";
    let mut collector = DiagnosticCollector::new(source);

    collector.error(
        DiagnosticCode::E102,
        "Unknown CSS property 'opactiy'",
        Some(SourceSpan::new(0, 7)),
    );

    assert!(collector.has_errors());
    assert_eq!(collector.error_count(), 1);
    assert_eq!(collector.warning_count(), 0);
}

#[test]
fn test_diagnostic_collector_warning() {
    let source = "@preset easing &unused: ease-out;";
    let mut collector = DiagnosticCollector::new(source);

    collector.warning(
        DiagnosticCode::W001,
        "Preset '&unused' is defined but never used",
        Some(SourceSpan::new(0, 33)),
    );

    assert!(!collector.has_errors());
    assert_eq!(collector.error_count(), 0);
    assert_eq!(collector.warning_count(), 1);
}

#[test]
fn test_diagnostic_collector_multiple() {
    let source = "line1\nline2\nline3";
    let mut collector = DiagnosticCollector::new(source);

    collector.error(DiagnosticCode::E100, "Error 1", None);
    collector.error(DiagnosticCode::E101, "Error 2", None);
    collector.warning(DiagnosticCode::W001, "Warning 1", None);

    assert!(collector.has_errors());
    assert_eq!(collector.error_count(), 2);
    assert_eq!(collector.warning_count(), 1);
    assert_eq!(collector.diagnostics().len(), 3);
}

#[test]
fn test_diagnostic_render_with_span() {
    let source = ".hero {\n    opactiy: 0 -> 1;\n}";
    let mut collector = DiagnosticCollector::new(source).with_path("styles.st");

    collector.error_with_hint(
        DiagnosticCode::E102,
        "Unknown CSS property 'opactiy'",
        Some(SourceSpan::new(12, 19)),
        "did you mean 'opacity'?",
    );

    let rendered = collector.render();

    assert!(rendered.contains("error[E102]"));
    assert!(rendered.contains("Unknown CSS property 'opactiy'"));
    assert!(rendered.contains("styles.st:2:5"));
    assert!(rendered.contains("opactiy: 0 -> 1;"));
    assert!(rendered.contains("^^^^^^^"));
    assert!(rendered.contains("did you mean 'opacity'?"));
}

#[test]
fn test_diagnostic_render_without_span() {
    let source = "";
    let mut collector = DiagnosticCollector::new(source);

    collector.error(DiagnosticCode::E100, "Unknown preset '&missing'", None);

    let rendered = collector.render();

    assert!(rendered.contains("error[E100]"));
    assert!(rendered.contains("Unknown preset '&missing'"));
    assert!(!rendered.contains("-->")); // No location marker
}

#[test]
fn test_levenshtein_distance_identical() {
    assert_eq!(levenshtein_distance("opacity", "opacity"), 0);
}

#[test]
fn test_levenshtein_distance_one_char() {
    assert_eq!(levenshtein_distance("opacity", "opactiy"), 2); // swap t and i
    assert_eq!(levenshtein_distance("opacity", "opacty"), 1); // missing i
}

#[test]
fn test_levenshtein_distance_empty() {
    assert_eq!(levenshtein_distance("", "abc"), 3);
    assert_eq!(levenshtein_distance("abc", ""), 3);
    assert_eq!(levenshtein_distance("", ""), 0);
}

#[test]
fn test_find_similar() {
    let candidates = &["opacity", "transform", "scale", "rotate", "translate"];

    assert_eq!(find_similar("opactiy", candidates, 2), Some("opacity"));
    assert_eq!(find_similar("tramsform", candidates, 2), Some("transform"));
    assert_eq!(find_similar("scal", candidates, 2), Some("scale"));
    assert_eq!(find_similar("xyz", candidates, 2), None); // Too different
}

#[test]
fn test_find_similar_case_insensitive() {
    let candidates = &["opacity", "Transform", "SCALE"];

    assert_eq!(find_similar("Opacity", candidates, 1), Some("opacity"));
    assert_eq!(find_similar("transform", candidates, 1), Some("Transform"));
    assert_eq!(find_similar("scale", candidates, 1), Some("SCALE"));
}

#[test]
fn test_diagnostic_code_display() {
    assert_eq!(DiagnosticCode::E102.to_string(), "E102");
    assert_eq!(DiagnosticCode::W001.to_string(), "W001");
}

#[test]
fn test_diagnostic_code_is_error() {
    assert!(DiagnosticCode::E100.is_error());
    assert!(DiagnosticCode::E102.is_error());
    assert!(!DiagnosticCode::W001.is_error());
    assert!(!DiagnosticCode::W100.is_error());
}

#[test]
fn test_diagnostic_with_notes() {
    let diag = Diagnostic::error(DiagnosticCode::E100, "Test error")
        .with_note("Note 1")
        .with_note("Note 2");

    assert_eq!(diag.notes.len(), 2);
    assert_eq!(diag.notes[0], "Note 1");
    assert_eq!(diag.notes[1], "Note 2");
}

#[test]
fn test_summary_line() {
    let source = "";
    let mut collector = DiagnosticCollector::new(source);

    collector.error(DiagnosticCode::E100, "Error 1", None);
    collector.error(DiagnosticCode::E101, "Error 2", None);
    collector.warning(DiagnosticCode::W001, "Warning 1", None);

    let rendered = collector.render();

    assert!(rendered.contains("2 error(s), 1 warning(s)"));
}

#[test]
fn is_error_is_derived_from_the_prefix_not_a_hand_list() {
    // The rot this guards: a hand-maintained match list silently defaulted
    // every NEW code to warning in the channels that consult is_error
    // (SignalDiagnostic severity, miette severity). The refusal codes this
    // arc added (E0948 planned driver, E0949 clip consumer, E0953 wrong-sigil
    // `as`) printed "error[...]" while those channels called them warnings.
    for code in [
        DiagnosticCode::E0948,
        DiagnosticCode::E0949,
        DiagnosticCode::E0953,
        DiagnosticCode::E100,
        DiagnosticCode::E0806,
    ] {
        assert!(code.is_error(), "{code} must be an error");
    }
    // The ONLY exceptions: codes whose every construction site builds them
    // as warnings. If you add a warning-semantics E-code, you add it here —
    // the set is the documentation.
    for code in [
        DiagnosticCode::E0923,
        DiagnosticCode::E0924,
        DiagnosticCode::E150,
        DiagnosticCode::E151,
    ] {
        assert!(!code.is_error(), "{code} is warning-semantics by construction");
    }
    // W/I/H prefixes are never errors.
    assert!(!DiagnosticCode::W001.is_error());
    assert!(!DiagnosticCode::W0715.is_error());
}
