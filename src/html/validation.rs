//! HTML-aware validation for Spacetime selectors.
//!
//! Validates that selectors in `.st` files actually match elements in the HTML,
//! with template-aware expansion and "did you mean?" suggestions.

use super::model::HtmlContext;
use super::selector::SelectorMatch;
use crate::diagnostics::{DiagnosticCode, DiagnosticCollector};
use crate::parser::{ScopeBlock, StFile};

/// Configuration for optional recommendations/warnings.
#[derive(Debug, Clone, Default)]
pub struct RecommendationConfig {
    /// Warn about elements with no animations
    pub warn_no_animations: bool,
    /// Warn about large sections without scroll reveals
    pub warn_no_scroll_reveal: bool,
    /// Warn about non-specific selectors
    pub warn_non_specific: bool,
    /// Warn about too many stagger animations
    pub warn_stagger_count: bool,
    /// Maximum stagger animations before warning
    pub stagger_threshold: usize,
    /// Warn about below-fold content without @on visible
    pub warn_below_fold: bool,
}

impl RecommendationConfig {
    /// Create a config with all recommendations enabled
    pub fn all() -> Self {
        Self {
            warn_no_animations: true,
            warn_no_scroll_reveal: true,
            warn_non_specific: true,
            warn_stagger_count: true,
            stagger_threshold: 10,
            warn_below_fold: true,
        }
    }

    /// Create a config with no recommendations (errors only)
    pub fn errors_only() -> Self {
        Self::default()
    }
}

/// Validate a Spacetime AST against an HTML context.
///
/// This is the main entry point for HTML-aware validation.
/// It checks all selectors in the AST against the parsed HTML structure
/// and emits diagnostics for mismatches.
pub fn validate_with_html(
    ast: &StFile,
    html_ctx: &HtmlContext,
    diagnostics: &mut DiagnosticCollector,
    config: Option<RecommendationConfig>,
) {
    let config = config.unwrap_or_default();

    // Determine the "main" .st file path from the HTML context source path.
    // Convention: foo.html -> foo.st, index.html -> index.st
    let main_st_path = if html_ctx.source_path.ends_with(".html") {
        Some(html_ctx.source_path.replace(".html", ".st"))
    } else {
        None
    };

    for scope in &ast.scopes {
        validate_scope_block(
            scope,
            html_ctx,
            diagnostics,
            &config,
            main_st_path.as_deref(),
        );
    }

    // Note: File-level element refs are now validated in scope blocks

    // Optional: Check for recommendation-level warnings
    if config.warn_stagger_count {
        check_stagger_count(ast, diagnostics, config.stagger_threshold);
    }
}

/// Validate a scope block's selector and contents.
fn validate_scope_block(
    scope: &ScopeBlock,
    html_ctx: &HtmlContext,
    diagnostics: &mut DiagnosticCollector,
    _config: &RecommendationConfig,
    main_st_path: Option<&str>,
) {
    // CSS-only scopes (no directives/matches) are defensive styling — they target
    // elements that may exist in other HTML files, be dynamically created, or be
    // BEM modifiers applied at runtime. Skip selector validation for these.
    // See BUG-e0602-import-validation for full root cause analysis.
    let has_directives = !scope.matches.is_empty();

    // Validate the main scope selector
    let selector = &scope.selector;
    let result = html_ctx.matches(selector);

    if has_directives && !result.is_match() {
        // Check if this scope is from an imported module (different source file).
        // Imported scopes may target elements in other pages, so downgrade to warning.
        let is_imported = match (&scope.source_file, main_st_path) {
            (Some(scope_file), Some(main_file)) => {
                // Normalize paths for comparison (handle relative path differences)
                !scope_file.ends_with(main_file) && !main_file.ends_with(scope_file.as_str())
            }
            _ => false,
        };

        if is_imported {
            diagnostics.warning(
                DiagnosticCode::E0602,
                format!(
                    "Scope selector '{}' does not match any element (from imported module)",
                    selector
                ),
                Some(scope.span.into()),
            );
        } else {
            emit_selector_error(
                diagnostics,
                DiagnosticCode::E0602,
                &format!("Scope selector '{}' does not match any element", selector),
                Some(scope.span.into()),
                &result,
            );
        }
    }

    // Note: @each blocks and &element references are now in FormMatches, processed via macro expansion
    // They are no longer available as separate fields on ScopeBlock

    // Note: Animation target validation now happens via macros in the metasystem

    // Validate nested scopes
    for nested in &scope.nested_scopes {
        let nested_selector = if nested.selector.starts_with('>') {
            format!("{} {}", selector, nested.selector)
        } else {
            nested.selector.clone()
        };

        let nested_result = html_ctx.matches(&nested_selector);
        if !nested_result.is_match() {
            emit_selector_error(
                diagnostics,
                DiagnosticCode::E0606,
                &format!(
                    "Nested selector '{}' does not match any element",
                    nested.selector
                ),
                Some(nested.span.into()),
                &nested_result,
            );
        }

        // Note: @each blocks and element refs are now in FormMatches, processed via macro expansion
        // They are no longer available as separate fields on NestedScope
    }
}

/// Emit a selector error with suggestions if available.
fn emit_selector_error(
    diagnostics: &mut DiagnosticCollector,
    code: DiagnosticCode,
    message: &str,
    span: Option<crate::diagnostics::SourceSpan>,
    result: &SelectorMatch,
) {
    if let Some(suggestions) = result.suggestions() {
        let hint = if suggestions.len() == 1 {
            format!("Did you mean '{}'?", suggestions[0])
        } else {
            format!("Similar selectors: {}", suggestions.join(", "))
        };
        diagnostics.error_with_hint(code, message, span, hint);
    } else {
        diagnostics.error(code, message, span);
    }
}

/// Check for excessive stagger animations (performance warning).
/// Note: This now checks presets since timelines are created via macros.
fn check_stagger_count(ast: &StFile, diagnostics: &mut DiagnosticCollector, threshold: usize) {
    use crate::parser::{AnimationProperty, PresetValue};

    let mut stagger_count = 0;

    // Count staggers in animation presets
    for preset in &ast.presets {
        if let PresetValue::AnimationBlock(properties) = &preset.value {
            for prop in properties {
                if matches!(prop, AnimationProperty::Stagger(_)) {
                    stagger_count += 1;
                }
            }
        }
    }

    if stagger_count > threshold {
        diagnostics.warning(
            DiagnosticCode::W0604,
            format!(
                "File has {} stagger animations in presets which may impact performance",
                stagger_count
            ),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::{parse_html, parse_templates};
    use crate::parser::parse;

    fn create_test_context() -> HtmlContext {
        let index = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <div id="app" class="container">
                    <ik-projects>
                        <div class="ik-projects__header"></div>
                    </ik-projects>
                    <ik-services></ik-services>
                </div>
            </body>
            </html>
        "#;

        let templates = r#"
            <template data-component="ik-projects">
                <section class="ik-projects">
                    <div class="ik-projects__grid">
                        <slot></slot>
                    </div>
                </section>
            </template>
            <template data-component="ik-project">
                <article class="ik-project">
                    <div class="ik-project__image">
                        <slot name="image"></slot>
                    </div>
                    <slot name="title"></slot>
                </article>
            </template>
            <template data-component="ik-services">
                <section class="ik-services">
                    <div class="ik-services__grid">
                        <slot></slot>
                    </div>
                </section>
            </template>
        "#;

        HtmlContext {
            document: parse_html(index).unwrap(),
            templates: parse_templates(templates).unwrap(),
            source_path: "test.html".to_string(),
        }
    }

    #[test]
    fn test_each_with_valid_template() {
        let st_code = r#"
            .ik-projects__grid {
                @each(projects) {
                    template: "ik-project";
                    [slot="title"]: $.title;
                    [slot="image"] { src: $.image; }
                }
            }
        "#;

        let ast = parse(st_code).unwrap();
        let ctx = create_test_context();
        let mut diagnostics = DiagnosticCollector::new(st_code);

        validate_with_html(&ast, &ctx, &mut diagnostics, None);

        assert!(
            !diagnostics.has_errors(),
            "Valid @each should not produce errors"
        );
    }
    #[test]
    fn test_each_with_invalid_slot() {
        // @each with valid stdlib syntax — slot validation done by metasystem
        let st_code = r#"
            .ik-projects__grid {
                @each($projects as $p) {
                    &ik-project($p);
                }
            }
        "#;

        let ast = parse(st_code).unwrap();
        assert!(
            ast.scopes[0].matches.iter().any(|m| m.macro_name == "each"),
            "scope should have @each FormMatch"
        );

        // HTML validation doesn't validate macro calls directly
        let ctx = create_test_context();
        let mut diagnostics = DiagnosticCollector::new(st_code);
        validate_with_html(&ast, &ctx, &mut diagnostics, None);
    }
    #[test]
    fn test_each_with_invalid_template() {
        // @each with valid stdlib syntax — template validation done by metasystem
        let st_code = r#"
            .ik-projects__grid {
                @each($projects as $p) {
                    &nonexistent-template($p);
                }
            }
        "#;

        let ast = parse(st_code).unwrap();
        assert!(
            ast.scopes[0].matches.iter().any(|m| m.macro_name == "each"),
            "scope should have @each FormMatch"
        );

        // HTML validation doesn't validate macro calls directly
        let ctx = create_test_context();
        let mut diagnostics = DiagnosticCollector::new(st_code);
        validate_with_html(&ast, &ctx, &mut diagnostics, None);
    }

    #[test]
    fn test_e0602_imported_scope_downgrades_to_warning() {
        use crate::diagnostics::Severity;
        use crate::parser::{ScopeBlock, SourceSpan};

        let ctx = create_test_context(); // source_path: "test.html" -> main_st_path: "test.st"
        let mut diagnostics = DiagnosticCollector::new("");

        // Create an AST with a scope from an imported module (different source file)
        let ast = crate::parser::StFile {
            scopes: vec![ScopeBlock {
                kind: Default::default(),
                selector: ".nonexistent-imported".to_string(),
                behavior: Default::default(),
                css_declarations: vec![],
                form_refs: Vec::new(),
                nested_scopes: vec![],
                matches: vec![crate::syntax::FormMatch::new("on")], // has directives
                span: SourceSpan::default(),
                source_file: Some("modules/_shared.st".to_string()), // different from "test.st",
                exports: vec![],
                refs: vec![],
                states: vec![],
                html: String::new(),
            }],
            ..Default::default()
        };

        validate_with_html(&ast, &ctx, &mut diagnostics, None);

        // Should be a warning, not an error
        let diags = diagnostics.diagnostics();
        let e0602: Vec<_> = diags
            .iter()
            .filter(|d| d.code == DiagnosticCode::E0602)
            .collect();
        assert_eq!(e0602.len(), 1, "Expected one E0602 diagnostic");
        assert_eq!(
            e0602[0].severity,
            Severity::Warning,
            "Imported scope should be a warning, not error"
        );
    }

    #[test]
    fn test_e0602_local_scope_stays_error() {
        use crate::diagnostics::Severity;
        use crate::parser::{ScopeBlock, SourceSpan};

        let ctx = create_test_context(); // source_path: "test.html" -> main_st_path: "test.st"
        let mut diagnostics = DiagnosticCollector::new("");

        // Create an AST with a scope from the local file (same source file)
        let ast = crate::parser::StFile {
            scopes: vec![ScopeBlock {
                kind: Default::default(),
                selector: ".nonexistent-local".to_string(),
                behavior: Default::default(),
                css_declarations: vec![],
                form_refs: Vec::new(),
                nested_scopes: vec![],
                matches: vec![crate::syntax::FormMatch::new("on")], // has directives
                span: SourceSpan::default(),
                source_file: Some("test.st".to_string()), // matches main file,
                exports: vec![],
                refs: vec![],
                states: vec![],
                html: String::new(),
            }],
            ..Default::default()
        };

        validate_with_html(&ast, &ctx, &mut diagnostics, None);

        // Should be an error (local scope)
        let diags = diagnostics.diagnostics();
        let e0602: Vec<_> = diags
            .iter()
            .filter(|d| d.code == DiagnosticCode::E0602)
            .collect();
        assert_eq!(e0602.len(), 1, "Expected one E0602 diagnostic");
        assert_eq!(
            e0602[0].severity,
            Severity::Error,
            "Local scope should remain an error"
        );
    }
}
