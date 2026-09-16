//! Tests for LSP completion provider.

use std::path::PathBuf;

use spacetime::lsp::form_registry::{DirectiveParam, DirectiveSignature, FormRegistry};
use spacetime::lsp::{
    CompletionContext, DocumentState, extract_context_at_position, provide_completions,
};
use spacetime::parser::SourceSpan;
use spacetime::parser::meta_ast::{CaptureModifier, CaptureType};

use tower_lsp::lsp_types::Position;

/// Create a test FormRegistry with sample directives.
fn create_test_registry() -> FormRegistry {
    let mut registry = FormRegistry::new();

    // @scroll directive
    registry.register(DirectiveSignature {
        name: "scroll".to_string(),
        macro_name: "ScrollMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "duration".to_string(),
                capture_var: "dur".to_string(),
                capture_type: CaptureType::Duration,
                modifier: CaptureModifier::Optional,
                default_value: Some("0.3s".to_string()),
            },
            DirectiveParam {
                name: "easing".to_string(),
                capture_var: "ease".to_string(),
                capture_type: CaptureType::Easing,
                modifier: CaptureModifier::Optional,
                default_value: Some("ease-out".to_string()),
            },
        ],
        body_type: Some(CaptureType::Properties),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/scroll.st"),
        documentation: Some("Animate properties on scroll.".to_string()),
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @screen directive
    registry.register(DirectiveSignature {
        name: "screen".to_string(),
        macro_name: "ScreenMacro".to_string(),
        params: vec![DirectiveParam {
            name: "min-width".to_string(),
            capture_var: "min".to_string(),
            capture_type: CaptureType::Length,
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: Some(CaptureType::Properties),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/screen.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @fade-in directive
    registry.register(DirectiveSignature {
        name: "fade-in".to_string(),
        macro_name: "FadeInMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/fade.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @toggle directive with union type
    registry.register(DirectiveSignature {
        name: "toggle".to_string(),
        macro_name: "ToggleMacro".to_string(),
        params: vec![DirectiveParam {
            name: "axis".to_string(),
            capture_var: "axis".to_string(),
            capture_type: CaptureType::Union(vec![
                "x".to_string(),
                "y".to_string(),
                "both".to_string(),
            ]),
            modifier: CaptureModifier::Optional,
            default_value: Some("y".to_string()),
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/toggle.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    registry
}

/// Create a DocumentState for testing.
fn document_from(content: &str) -> DocumentState {
    DocumentState::new(content.to_string(), 1)
}

// =============================================================================
// Context Extraction Tests
// =============================================================================

#[test]
fn test_extract_context_after_at_empty() {
    let content = "@";
    let ctx = extract_context_at_position(content, 1);
    assert_eq!(
        ctx,
        CompletionContext::AfterAt {
            prefix: "".to_string()
        }
    );
}

#[test]
fn test_extract_context_after_at_partial() {
    let content = "@scr";
    let ctx = extract_context_at_position(content, 4);
    assert_eq!(
        ctx,
        CompletionContext::AfterAt {
            prefix: "scr".to_string()
        }
    );
}

#[test]
fn test_extract_context_inside_params_empty() {
    let content = "@scroll(";
    let ctx = extract_context_at_position(content, 8);
    assert_eq!(
        ctx,
        CompletionContext::InsideParams {
            directive_name: "scroll".to_string(),
            provided_params: vec![]
        }
    );
}

#[test]
fn test_extract_context_inside_params_with_existing() {
    let content = "@scroll(duration: 0.3s, ";
    let ctx = extract_context_at_position(content, 24);
    assert_eq!(
        ctx,
        CompletionContext::InsideParams {
            directive_name: "scroll".to_string(),
            provided_params: vec!["duration".to_string()]
        }
    );
}

#[test]
fn test_extract_context_param_value() {
    let content = "@scroll(duration: ";
    let ctx = extract_context_at_position(content, 18);
    assert_eq!(
        ctx,
        CompletionContext::ParamValue {
            directive_name: "scroll".to_string(),
            param_name: "duration".to_string()
        }
    );
}

#[test]
fn test_extract_context_no_context() {
    let content = "some text without directive";
    let ctx = extract_context_at_position(content, 10);
    assert_eq!(ctx, CompletionContext::None);
}

#[test]
fn test_extract_context_with_hyphen_directive() {
    let content = "@fade-";
    let ctx = extract_context_at_position(content, 6);
    assert_eq!(
        ctx,
        CompletionContext::AfterAt {
            prefix: "fade-".to_string()
        }
    );
}

// =============================================================================
// Completion Provider Tests
// =============================================================================

#[test]
fn completes_directives_after_at() {
    let doc = document_from(".hero { @ }");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 9,
        },
        &registry,
    );

    assert!(!completions.is_empty());
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"scroll"));
    assert!(labels.contains(&"screen"));
    assert!(labels.contains(&"fade-in"));
    assert!(labels.contains(&"toggle"));
}

#[test]
fn completes_directives_with_prefix() {
    let doc = document_from(".hero { @scr }");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 12,
        },
        &registry,
    );

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"scroll"));
    assert!(labels.contains(&"screen"));
    assert!(!labels.contains(&"fade-in"));
}

#[test]
fn completes_params_inside_parens() {
    let doc = document_from("@scroll(");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 8,
        },
        &registry,
    );

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"duration"));
    assert!(labels.contains(&"easing"));
}

#[test]
fn excludes_already_provided_params() {
    let doc = document_from("@scroll(duration: 0.3s, ");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 24,
        },
        &registry,
    );

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(!labels.contains(&"duration")); // Already provided
    assert!(labels.contains(&"easing")); // Not yet provided
}

#[test]
fn no_completions_outside_context() {
    let doc = document_from("some plain text");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 5,
        },
        &registry,
    );

    assert!(completions.is_empty());
}

#[test]
fn completion_item_has_documentation() {
    let doc = document_from("@scro");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 5,
        },
        &registry,
    );

    let scroll_completion = completions.iter().find(|c| c.label == "scroll").unwrap();
    assert!(scroll_completion.documentation.is_some());
    assert!(scroll_completion.detail.is_some());
}

#[test]
fn completion_for_unknown_directive_returns_empty() {
    let doc = document_from("@unknowndirective(");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 18,
        },
        &registry,
    );

    // Should return empty since directive doesn't exist
    assert!(completions.is_empty());
}

// =============================================================================
// Sigil Completion Context Tests
// =============================================================================

#[test]
fn context_after_dollar() {
    let content = "$nav";
    let ctx = extract_context_at_position(content, content.len());
    assert_eq!(
        ctx,
        CompletionContext::AfterDollar {
            prefix: "nav".to_string()
        }
    );
}

#[test]
fn context_after_ampersand() {
    let content = "&ora-";
    let ctx = extract_context_at_position(content, content.len());
    assert_eq!(
        ctx,
        CompletionContext::AfterAmpersand {
            prefix: "ora-".to_string()
        }
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn handles_empty_document() {
    let doc = document_from("");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 0,
        },
        &registry,
    );

    assert!(completions.is_empty());
}

#[test]
fn handles_cursor_at_start() {
    let doc = document_from("@scroll");
    let registry = create_test_registry();
    let completions = provide_completions(
        &doc,
        Position {
            line: 0,
            character: 0,
        },
        &registry,
    );

    // Cursor before @, no completions
    assert!(completions.is_empty());
}

#[test]
fn handles_multiline_document() {
    let doc = document_from(
        r#"
.hero {
    @
}
"#,
    );
    let registry = create_test_registry();
    // Line 2 (0-indexed), after the @
    let completions = provide_completions(
        &doc,
        Position {
            line: 2,
            character: 5,
        },
        &registry,
    );

    assert!(!completions.is_empty());
}



// =============================================================================
// Grammar-driven snippets (gh-29): a snippet built from the real %form grammar
// must be STRUCTURALLY valid — literal words, sigils and positional captures
// come from the grammar, so filling the placeholders yields a shape the
// compiler actually recognizes (never a silently-dropped or E0946 shape).
// =============================================================================

/// Replace snippet tabstops with their default labels and drop final-cursor
/// `$N` markers, yielding the literal text a user who accepts every default
/// would write.
fn fill_snippet(snippet: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = snippet.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            if let Some(rel) = chars[i + 2..].iter().position(|&c| c == '}') {
                let inner: String = chars[i + 2..i + 2 + rel].iter().collect();
                if let Some(colon) = inner.find(':') {
                    out.push_str(&inner[colon + 1..]);
                }
                i += 2 + rel + 1;
                continue;
            }
        }
        // Bare `$N` tabstop (final cursor `$0`, body `$1`, …) — drop it.
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[test]
fn grammar_snippets_are_structurally_valid_in_context() {
    use spacetime::lsp::completion::build_directive_snippet;
    use spacetime::lsp::FormRegistry;

    let registry = FormRegistry::from_compiler();

    // Directive name → how it is used. `scope` directives live inside a
    // `.selector { … }` block; `file` directives sit at file scope.
    let cases: &[(&str, &str)] = &[
        ("data", "file"),
        ("type", "file"),
        ("form", "file"),
        ("each", "scope"),
        ("on", "scope"),
        ("scroll", "scope"),
        ("loop", "scope"),
        ("state", "scope"),
        ("presence", "scope"),
    ];

    let mut checked = 0;
    let mut bad = Vec::new();
    for (name, ctx) in cases {
        let sig = match registry.get_directive(name) {
            Some(s) => s,
            None => continue,
        };
        let (snippet, _fmt) = build_directive_snippet(name, sig);
        let filled = fill_snippet(&snippet);
        let directive = format!("@{}", filled);
        let source = match *ctx {
            "file" => directive.clone(),
            _ => format!(".x {{\n    {}\n}}", directive),
        };
        // The structural guarantee of gh-29: a grammar-driven snippet, its
        // placeholders filled with defaults, must PARSE as valid Spacetime.
        // (Whether it then MATCHES a specific %form depends on the values the
        // author types — e.g. a valid `@data` kind word, a declared binding —
        // which no default placeholder can know. Malformed structure is what
        // the old flat snippet produced; that is what we forbid here.)
        match spacetime::parser::parse(&source) {
            Ok(_) => {}
            Err(e) => {
                bad.push(format!(
                    "@{name} -> `{snippet}` failed to parse: {}",
                    e.render_all_plain(&source, "test.st")
                ));
            }
        }
        checked += 1;
    }

    assert_eq!(checked, cases.len(), "every curated directive should resolve");
    assert!(
        bad.is_empty(),
        "grammar-driven snippets were not structurally valid:\n{}",
        bad.join("\n")
    );
}

/// gh-29 COMPILE gate: a grammar-driven snippet, its placeholders filled with
/// VALID values for the capture types, must COMPILE — i.e. the shared
/// `validate_document` engine (the exact `check` pipeline) reports no
/// structural (E0946/E0910 "does not match grammar") diagnostic. This is the
/// loop the plan closes: the old flat `name: ${n}` snippet produced shapes
/// that became E0946 build errors after W3. A snippet built from the `%form`
/// cannot be structurally wrong.
#[test]
fn grammar_driven_snippets_compile_with_valid_values() {
    use spacetime::lsp::completion::build_directive_snippet;
    use spacetime::lsp::{DocumentState, validate_document};

    // A curated real use for each core directive — the shape the grammar-
    // driven snippet leads the author to type, with concrete values.
    let cases: &[(&str, &str, &str)] = &[
        // (name, context, fully-authored directive text)
        ("data", "file", "@data derive $items : 1;"),
        ("type", "file", "@type Product {\n    name: string;\n}"),
        ("state", "scope", "@state(when: \"done\") {\n    :active {\n        opacity: 1;\n    }\n}"),
        ("on", "scope", "@on &.click {\n    $x <- 1;\n}"),
    ];

    for (name, ctx, authored) in cases {
        let source = match *ctx {
            "file" => authored.to_string(),
            _ => format!(".x {{\n    {}\n}}", authored),
        };
        let ds = DocumentState::new(source, 1);
        let diags = validate_document(&ds, None);
        for d in &diags {
            let code = d
                .code
                .as_ref()
                .map(|c| match c {
                    tower_lsp::lsp_types::NumberOrString::String(s) => s.clone(),
                    tower_lsp::lsp_types::NumberOrString::Number(n) => n.to_string(),
                })
                .unwrap_or_default();
            assert!(
                !(code.contains("E0946") || code.contains("E0910")),
                "@{name} with a valid authored shape must not be structurally wrong: {code} {message}",
                code = code,
                message = d.message
            );
        }
    }
}
