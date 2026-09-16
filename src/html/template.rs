//! Template parsing for component definitions.
//!
//! Parses `<template data-component="...">` elements from templates.html
//! to extract slot definitions and internal structure.

use super::model::{ComponentTemplate, ElementInfo, HtmlSpan, TemplateRegistry};
use super::treesink::visit_elements;
use std::collections::HashSet;

/// Parse templates.html to extract component definitions.
///
/// Looks for `<template data-component="name">` elements and extracts:
/// - Slot definitions (`<slot name="...">`)
/// - Internal classes and IDs
/// - Nested custom elements
pub fn parse_templates(html: &str) -> Result<TemplateRegistry, String> {
    let mut registry = TemplateRegistry::default();
    let visited = visit_elements(html);

    // Map each visited element index -> the index of the enclosing
    // `template[data-component]` (if any), by walking the parent chain. This is
    // the tree-true replacement for lol_html's in_template/depth stack.
    let template_of = |idx: usize| -> Option<usize> {
        let mut p = visited[idx].parent_index;
        while let Some(pi) = p {
            let anc = &visited[pi];
            if anc.tag == "template" && anc.attr("data-component").is_some_and(|c| !c.is_empty()) {
                return Some(pi);
            }
            p = anc.parent_index;
        }
        None
    };

    // First pass: create a ComponentTemplate for every template[data-component],
    // keyed by its visited index so descendants attach to the right one.
    use std::collections::HashMap;
    let mut by_index: HashMap<usize, ComponentTemplate> = HashMap::new();
    let mut order: Vec<usize> = Vec::new();
    for (i, el) in visited.iter().enumerate() {
        if el.tag == "template"
            && let Some(name) = el.attr("data-component")
            && !name.is_empty()
        {
            by_index.insert(i, ComponentTemplate::new(name.to_string()));
            order.push(i);
        }
    }

    // Second pass: attach each descendant element to its enclosing template.
    for (i, el) in visited.iter().enumerate() {
        // The template element itself contributes nothing to its own body.
        if by_index.contains_key(&i) {
            continue;
        }
        let Some(tpl_idx) = template_of(i) else {
            continue;
        };
        let Some(template) = by_index.get_mut(&tpl_idx) else {
            continue;
        };
        let tag = el.tag.clone();

        // Slot elements: collect named/default slots.
        if tag == "slot" {
            if let Some(name) = el.attr("name") {
                template.slots.insert(name.to_string());
            } else {
                template.has_default_slot = true;
            }
        }

        // Internal classes
        if let Some(class_attr) = el.attr("class") {
            for class in class_attr.split_whitespace() {
                template.internal_classes.insert(class.to_string());
            }
        }
        // Internal IDs
        if let Some(id) = el.attr("id") {
            template.internal_ids.insert(id.to_string());
        }
        // Nested custom elements
        if tag.contains('-') && tag != "slot" {
            template.nested_components.insert(tag.clone());
        }

        let elem_info = ElementInfo {
            tag,
            id: el.attr("id").map(str::to_string),
            classes: el
                .attr("class")
                .map(|c| c.split_whitespace().map(String::from).collect())
                .unwrap_or_default(),
            slot: el.attr("slot").map(str::to_string),
            data_component: None,
            attributes: std::collections::HashMap::new(),
            span: HtmlSpan::default(),
            parent_index: None,
            depth: el.depth,
        };
        template.elements.push(elem_info);
    }

    for idx in order {
        if let Some(template) = by_index.remove(&idx) {
            registry.components.insert(template.name.clone(), template);
        }
    }

    Ok(registry)
}

/// Extract slot names from a template's HTML content.
///
/// Helper function for parsing slot definitions.
pub fn extract_slots(template_html: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    let mut has_default = false;
    for el in visit_elements(template_html) {
        if el.tag == "slot" {
            if let Some(name) = el.attr("name") {
                result.insert(name.to_string());
            } else {
                has_default = true;
            }
        }
    }
    if has_default {
        result.insert(String::new()); // Empty string represents default slot
    }
    result
}

/// Extract class names from an HTML string via the html5ever element walker.
///
/// Used to register classes from `@template` body HTML into the HtmlContext
/// so that scope selectors targeting template-created elements pass validation.
pub fn extract_classes_from_html(html: &str) -> HashSet<String> {
    let mut classes = HashSet::new();
    for el in visit_elements(html) {
        if let Some(class_attr) = el.attr("class") {
            for class in class_attr.split_whitespace() {
                classes.insert(class.to_string());
            }
        }
    }
    classes
}

/// Inject classes from `@template` bodies and inline HTML in the AST into the HtmlContext.
///
/// Scans all file-level FormMatches for `@template` definitions and extracts
/// class names from their HTML body content. Also scans scope-level source
/// for `class="..."` patterns to catch classes from template invocations
/// (e.g., `&ora-footer(...) { <div class="ora-footer__links">... }`).
///
/// This ensures scope selectors targeting template-created or invocation-created
/// elements pass E0602 validation.
pub fn inject_template_classes_from_ast(
    html_ctx: &mut super::model::HtmlContext,
    ast: &crate::parser::StFile,
    source: &str,
) {
    use std::collections::HashMap;

    // Cache imported file source by path so we only read each file once.
    let mut import_source_cache: HashMap<String, String> = HashMap::new();
    let mut load_source = |source_file: Option<&str>| -> Option<String> {
        match source_file {
            None => Some(source.to_string()),
            Some(path) => {
                if let Some(cached) = import_source_cache.get(path) {
                    return Some(cached.clone());
                }
                match std::fs::read_to_string(path) {
                    Ok(s) => {
                        import_source_cache.insert(path.to_string(), s.clone());
                        Some(s)
                    }
                    Err(_) => None,
                }
            }
        }
    };

    for fm in ast.matches.iter().filter(|m| m.selector.is_none()) {
        if fm.macro_name == "template" {
            let start = fm.span.start;
            let end = fm.span.end;
            // Spans are scoped to their source file. Resolve via fm.source_file when set,
            // and fall back to the current source string for inline templates.
            let snippet_owned = load_source(fm.source_file.as_deref());
            if let Some(snippet_src) = snippet_owned
                && start < end
                && end <= snippet_src.len()
                && snippet_src.is_char_boundary(start)
                && snippet_src.is_char_boundary(end)
            {
                let snippet = &snippet_src[start..end];
                for class in extract_classes_from_html(snippet) {
                    html_ctx.document.classes.insert(class);
                }
            }
        }
    }

    // Also scan scope source spans for inline HTML class attributes.
    // This catches classes from template invocation bodies (e.g., &body slots)
    // that aren't part of @template definitions themselves.
    for scope in &ast.scopes {
        let start = scope.span.start;
        let end = scope.span.end;
        let snippet_owned = load_source(scope.source_file.as_deref());
        if let Some(snippet_src) = snippet_owned
            && start < end
            && end <= snippet_src.len()
            && snippet_src.is_char_boundary(start)
            && snippet_src.is_char_boundary(end)
        {
            let snippet = &snippet_src[start..end];
            for class in extract_classes_from_html(snippet) {
                html_ctx.document.classes.insert(class);
            }
        }
    }
}

/// Inject dynamic classes from `@bind(class: "...")` directives into the HtmlContext.
///
/// Scans all scope-level FormMatches for `bind` directives with a `class` capture
/// and registers those class names as known, so E0602 validation doesn't flag
/// scope selectors that target dynamically-bound classes.
pub fn inject_bind_classes_from_ast(
    html_ctx: &mut super::model::HtmlContext,
    ast: &crate::parser::StFile,
) {
    for scope in &ast.scopes {
        for fm in &scope.matches {
            if fm.macro_name == "bind"
                && let Some(class) = fm.get_string("class")
            {
                html_ctx.document.classes.insert(class.to_string());
            }
        }
        // Also check nested scopes
        for ns in &scope.nested_scopes {
            for fm in &ns.matches {
                if fm.macro_name == "bind"
                    && let Some(class) = fm.get_string("class")
                {
                    html_ctx.document.classes.insert(class.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_template() {
        let html = r#"
            <template data-component="my-card">
                <div class="card">
                    <slot name="title"></slot>
                    <slot name="content"></slot>
                    <slot></slot>
                </div>
            </template>
        "#;

        let registry = parse_templates(html).unwrap();
        let template = registry.get("my-card").unwrap();

        assert!(template.slots.contains("title"));
        assert!(template.slots.contains("content"));
        assert!(template.has_default_slot);
        assert!(template.internal_classes.contains("card"));
    }

    #[test]
    fn test_parse_nested_components() {
        let html = r#"
            <template data-component="ik-projects">
                <section class="ik-projects">
                    <div class="ik-projects__grid">
                        <slot></slot>
                    </div>
                </section>
            </template>
        "#;

        let registry = parse_templates(html).unwrap();
        let template = registry.get("ik-projects").unwrap();

        assert!(template.internal_classes.contains("ik-projects"));
        assert!(template.internal_classes.contains("ik-projects__grid"));
        assert!(template.has_default_slot);
    }

    #[test]
    fn test_extract_slots() {
        let html = r#"
            <div>
                <slot name="header"></slot>
                <slot name="body"></slot>
                <slot></slot>
            </div>
        "#;

        let slots = extract_slots(html);
        assert!(slots.contains("header"));
        assert!(slots.contains("body"));
        assert!(slots.contains("")); // Default slot
    }

    #[test]
    fn test_extract_classes_from_html() {
        let html = r#"<nav class="zypsy-nav"><a class="zypsy-nav__logo">Brand</a></nav>"#;
        let classes = extract_classes_from_html(html);
        assert!(classes.contains("zypsy-nav"));
        assert!(classes.contains("zypsy-nav__logo"));
    }

    #[test]
    fn test_extract_classes_multiple_classes() {
        let html = r#"<div class="card card--featured"><span class="card__title">T</span></div>"#;
        let classes = extract_classes_from_html(html);
        assert!(classes.contains("card"));
        assert!(classes.contains("card--featured"));
        assert!(classes.contains("card__title"));
    }

    #[test]
    fn test_extract_classes_empty_html() {
        let classes = extract_classes_from_html("");
        assert!(classes.is_empty());
    }

    #[test]
    fn test_inject_template_classes_from_source_span() {
        use crate::parser::SourceSpan;

        let source = r#"@template &card($label) {
  <nav class="zypsy-nav"><div class="zypsy-nav__links"></div></nav>
}"#;
        let mut html_ctx = super::super::model::HtmlContext::new();
        let fm = crate::syntax::FormMatch {
            macro_name: "template".to_string(),
            matched_macro: None,
            captures: std::collections::HashMap::new(),
            capture_spans: std::collections::HashMap::new(),
            selector: None,
            span: SourceSpan::new(0, source.len()),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let ast = crate::parser::StFile {
            matches: vec![fm],
            form_refs: Vec::new(),
            ..Default::default()
        };

        inject_template_classes_from_ast(&mut html_ctx, &ast, source);
        assert!(html_ctx.document.classes.contains("zypsy-nav"));
        assert!(html_ctx.document.classes.contains("zypsy-nav__links"));
    }

    #[test]
    fn test_inject_bind_class_registers_dynamic_class() {
        use crate::parser::{ScopeBlock, SourceSpan};
        use crate::syntax::CapturedValue;
        use std::collections::HashMap;

        let mut html_ctx = super::super::model::HtmlContext::new();

        // Create a scope with @bind(class: "foo", when: $x)
        let mut captures = HashMap::new();
        captures.insert(
            "class".to_string(),
            CapturedValue::String("foo".to_string()),
        );
        let bind_fm = crate::syntax::FormMatch {
            macro_name: "bind".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let ast = crate::parser::StFile {
            scopes: vec![ScopeBlock {
                kind: Default::default(),
                selector: ".container".to_string(),
                behavior: Default::default(),
                css_declarations: vec![],
                form_refs: Vec::new(),
                nested_scopes: vec![],
                matches: vec![bind_fm],
                span: SourceSpan::default(),
                source_file: None,
                exports: vec![],
                refs: vec![],
                states: vec![],
                html: String::new(),
            }],
            ..Default::default()
        };

        inject_bind_classes_from_ast(&mut html_ctx, &ast);
        assert!(
            html_ctx.document.classes.contains("foo"),
            "Expected 'foo' to be registered as a dynamic class"
        );
    }

    #[test]
    fn test_inject_bind_class_does_not_add_without_bind() {
        let html_ctx = super::super::model::HtmlContext::new();

        let ast = crate::parser::StFile::default();
        let mut ctx = html_ctx;
        inject_bind_classes_from_ast(&mut ctx, &ast);
        assert!(
            !ctx.document.classes.contains("foo"),
            "Expected no dynamic classes without @bind"
        );
    }

    #[test]
    fn test_inject_template_invocation_registers_internal_classes() {
        use crate::parser::{ScopeBlock, SourceSpan};

        // Simulates: [data-footer] { &ora-footer("Ora") { <div class="inner-class">...</div> } }
        let source = r#"[data-footer] {
    &ora-footer("Ora") {
        <div class="inner-class">links</div>
    }
}"#;
        let mut html_ctx = super::super::model::HtmlContext::new();
        let ast = crate::parser::StFile {
            scopes: vec![ScopeBlock {
                kind: Default::default(),
                selector: "[data-footer]".to_string(),
                behavior: Default::default(),
                css_declarations: vec![],
                form_refs: Vec::new(),
                nested_scopes: vec![],
                matches: vec![],
                span: SourceSpan::new(0, source.len()),
                source_file: None,
                exports: vec![],
                refs: vec![],
                states: vec![],
                html: String::new(),
            }],
            ..Default::default()
        };

        inject_template_classes_from_ast(&mut html_ctx, &ast, source);
        assert!(
            html_ctx.document.classes.contains("inner-class"),
            "Expected 'inner-class' from template invocation body to be registered"
        );
    }
}
