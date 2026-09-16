//! HTML-aware validation for Spacetime.
//!
//! This module provides compile-time validation of `.st` selectors against
//! actual HTML structure. It parses `index.html` and `templates.html` to build
//! a complete DOM model, then validates that selectors in animation files
//! actually match existing elements.
//!
//! # Features
//! - Parse HTML documents to extract IDs, classes, tags, and custom elements
//! - Parse component templates to understand slot definitions and internal structure
//! - Validate selectors with "did you mean?" suggestions for typos
//! - Template-aware matching (expands custom elements to check internal classes)

mod model;
pub mod directives;
pub mod page_shell;
mod parser;
mod selector;
pub mod ssg_filters;
pub mod ssg_unroll;
mod template;
pub mod treesink;
mod validation;

pub use model::{
    ComponentTemplate, DocumentModel, ElementInfo, HtmlContext, HtmlSpan, TemplateRegistry,
};
pub use parser::{parse_html, selector_exists};
pub use selector::{SelectorMatch, find_similar_selector};
pub use template::{
    extract_classes_from_html, extract_slots, inject_bind_classes_from_ast,
    inject_template_classes_from_ast, parse_templates,
};
pub use validation::{RecommendationConfig, validate_with_html};
pub use directives::{
    find_nested_component_call, find_nested_directive, strip_nested_component_calls,
    strip_nested_directives,
};

use std::path::Path;

/// Parse HTML files into a complete validation context.
///
/// Reads and parses both `index.html` and `templates.html` (if present)
/// to build an HtmlContext that can validate selectors.
pub fn parse_html_context(
    index_html: &str,
    templates_html: Option<&str>,
    source_path: &str,
) -> Result<HtmlContext, String> {
    let document = parse_html(index_html)?;

    let templates = match templates_html {
        Some(html) => parse_templates(html)?,
        None => TemplateRegistry::default(),
    };

    Ok(HtmlContext {
        document,
        templates,
        source_path: source_path.to_string(),
    })
}

/// Auto-detect and parse HTML files from a site directory.
///
/// Looks for:
/// - `index.html` in the site root
/// - `templates.html` in the site root or `components/` subdirectory
pub fn detect_html_context(site_dir: &Path) -> Result<Option<HtmlContext>, String> {
    let index_path = site_dir.join("index.html");

    if !index_path.exists() {
        return Ok(None);
    }

    let index_html = std::fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read index.html: {}", e))?;

    // Look for templates.html in multiple locations
    let templates_paths = [
        site_dir.join("templates.html"),
        site_dir.join("components/templates.html"),
    ];

    let templates_html = templates_paths
        .iter()
        .find(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(p).ok());

    parse_html_context(
        &index_html,
        templates_html.as_deref(),
        index_path.to_string_lossy().as_ref(),
    )
    .map(Some)
}

/// Auto-detect HTML context for a specific .st file using convention-based naming.
///
/// For `foo.st`, looks for:
/// 1. `foo.html` (convention-based pairing) - preferred
/// 2. `index.html` (fallback)
///
/// Then also looks for templates.html in standard locations.
pub fn detect_html_context_for_st(st_path: &Path) -> Result<Option<HtmlContext>, String> {
    let site_dir = st_path
        .parent()
        .ok_or("Cannot determine parent directory")?;

    // Convention-based: foo.st -> foo.html
    let st_stem = st_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("index");
    let convention_html_path = site_dir.join(format!("{}.html", st_stem));

    // Determine which HTML file to use
    let html_path = if convention_html_path.exists() {
        convention_html_path
    } else {
        let index_path = site_dir.join("index.html");
        if !index_path.exists() {
            return Ok(None);
        }
        index_path
    };

    let html_content = std::fs::read_to_string(&html_path)
        .map_err(|e| format!("Failed to read {}: {}", html_path.display(), e))?;

    // Look for templates.html in multiple locations
    let templates_paths = [
        site_dir.join("templates.html"),
        site_dir.join("components/templates.html"),
    ];

    let templates_html = templates_paths
        .iter()
        .find(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(p).ok());

    parse_html_context(
        &html_content,
        templates_html.as_deref(),
        html_path.to_string_lossy().as_ref(),
    )
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_context() {
        let index = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <div id="app" class="container">
                    <ik-hero>
                        <span slot="title">Hello</span>
                    </ik-hero>
                </div>
            </body>
            </html>
        "#;

        let templates = r#"
            <template data-component="ik-hero">
                <section class="ik-hero">
                    <h1 class="ik-hero__title">
                        <slot name="title"></slot>
                    </h1>
                </section>
            </template>
        "#;

        let ctx = parse_html_context(index, Some(templates), "test.html").unwrap();

        // Document should have container class
        assert!(ctx.document.classes.contains("container"));

        // Document should track custom element usage
        assert!(ctx.document.custom_elements.contains("ik-hero"));

        // Templates should be parsed
        assert!(ctx.templates.get("ik-hero").is_some());

        // all_classes should include template internal classes
        let all_classes = ctx.all_classes();
        assert!(all_classes.contains("ik-hero"));
        assert!(all_classes.contains("ik-hero__title"));
    }
}
