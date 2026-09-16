//! HTML parsing for document structure extraction.
//!
//! Uses the shared html5ever element walker ([`crate::html::treesink::visit_elements`])
//! to extract elements, IDs, classes, and custom-element usage from a document.
//! Pre-order DFS reproduces document order; each element carries depth + parent
//! index so slot-usage nesting (slot inside the nearest custom element) is
//! resolved by walking the parent chain.

use super::model::{DocumentModel, ElementInfo, HtmlSpan};
use super::treesink::visit_elements;
use std::collections::HashMap;

/// Parse an HTML document and extract element information.
///
/// Builds a DocumentModel containing all IDs, classes, tags, custom elements,
/// and slot usage found in the HTML.
pub fn parse_html(html: &str) -> Result<DocumentModel, String> {
    let mut model = DocumentModel::default();
    let visited = visit_elements(html);

    for (i, el) in visited.iter().enumerate() {
        let tag = el.tag.clone();

        // Extract ID
        let id = el.attr("id").map(str::to_string);
        if let Some(ref id_val) = id {
            model.ids.insert(id_val.clone());
        }

        // Extract classes
        let classes: Vec<String> = el
            .attr("class")
            .map(|c| c.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        for class in &classes {
            model.classes.insert(class.clone());
        }

        // Track tag name
        model.tags.insert(tag.clone());

        // Track custom elements (hyphenated names)
        if tag.contains('-') {
            model.custom_elements.insert(tag.clone());
        }

        // Track slot usage within custom elements: walk the parent chain to the
        // NEAREST ancestor whose tag is a custom element (lol_html tracked this
        // via a custom-element stack; the parent-index chain is the tree-true
        // equivalent).
        if let Some(slot) = el.attr("slot") {
            let mut p = el.parent_index;
            while let Some(pi) = p {
                let parent = &visited[pi];
                if parent.tag.contains('-') {
                    model
                        .slot_usage
                        .entry(parent.tag.clone())
                        .or_default()
                        .insert(slot.to_string());
                    break;
                }
                p = parent.parent_index;
            }
        }

        // Extract a fixed set of data-* attributes inline
        let mut attributes = HashMap::new();
        for attr in [
            "data-id",
            "data-state",
            "data-theme",
            "data-component",
            "data-st-id",
        ] {
            if let Some(value) = el.attr(attr) {
                attributes.insert(attr.to_string(), value.to_string());
            }
        }

        let info = ElementInfo {
            tag: tag.clone(),
            id,
            classes,
            slot: el.attr("slot").map(str::to_string),
            data_component: el.attr("data-component").map(str::to_string),
            attributes,
            span: HtmlSpan::default(),
            // Map the parent ELEMENT index (visit order == element insertion order,
            // so the visited parent index equals the model.elements index).
            parent_index: el.parent_index,
            depth: el.depth,
        };
        debug_assert_eq!(model.elements.len(), i);
        model.elements.push(info);
    }

    Ok(model)
}

/// Check if a selector would match any element in the document model.
///
/// This is a simplified matcher that handles common selector patterns:
/// - `#id` - ID selector
/// - `.class` - Class selector
/// - `tag` - Tag selector
/// - `[slot="name"]` - Attribute selector for slots
/// - Compound selectors like `.parent .child`
pub fn selector_exists(model: &DocumentModel, selector: &str) -> bool {
    let selector = selector.trim();

    // Universal selector (*) always matches
    if selector == "*" || selector.starts_with("*:") || selector.starts_with("*,") {
        return true;
    }

    // Handle compound selectors (space-separated)
    if selector.contains(' ') {
        // For compound selectors, check if the final part exists
        // This is a simplification - a full implementation would trace the hierarchy
        let parts: Vec<&str> = selector.split_whitespace().collect();
        if let Some(last) = parts.last() {
            return selector_exists(model, last);
        }
    }

    // Handle child combinator
    if selector.contains('>') {
        let parts: Vec<&str> = selector.split('>').map(|s| s.trim()).collect();
        if let Some(last) = parts.last() {
            return selector_exists(model, last);
        }
    }

    // ID selector
    if let Some(id) = selector.strip_prefix('#') {
        return model.ids.contains(id);
    }

    // Class selector (including compound like .class1.class2)
    if selector.starts_with('.') {
        // Strip pseudo-classes/elements before matching (e.g., .foo:first-child -> .foo)
        let without_pseudo = strip_pseudo(selector);
        let classes: Vec<&str> = without_pseudo[1..]
            .split('.')
            .filter(|s| !s.is_empty())
            .collect();
        return classes.iter().all(|c| model.classes.contains(*c));
    }

    // Attribute selector for slots
    if selector.starts_with("[slot=") {
        // Extract slot value - handle both quoted and unquoted forms
        // Must try the longer prefix first to avoid partial matches
        let slot_value = selector
            .trim_start_matches("[slot=\"")
            .trim_start_matches("[slot='")
            .trim_start_matches("[slot=")
            .trim_end_matches(']')
            .trim_end_matches('"')
            .trim_end_matches('\'');

        // Check if any element uses this slot
        return model
            .slot_usage
            .values()
            .any(|slots| slots.contains(slot_value));
    }

    // Generic attribute selector
    if selector.starts_with('[') && selector.ends_with(']') {
        // For now, return true for complex attribute selectors to avoid false positives
        return true;
    }

    // Tag + class compound selector (e.g., canvas.showcase, div.container)
    if selector.contains('.') && !selector.starts_with('.') {
        let without_pseudo = strip_pseudo(selector);
        let dot_pos = without_pseudo.find('.').unwrap();
        let tag = &without_pseudo[..dot_pos];
        let class_part = &without_pseudo[dot_pos + 1..];

        // Check tag exists
        if !model.tags.contains(tag) {
            return false;
        }

        // Check all classes exist (handles tag.class1.class2)
        let classes: Vec<&str> = class_part.split('.').filter(|s| !s.is_empty()).collect();
        return classes.iter().all(|c| model.classes.contains(*c));
    }

    // Tag selector (including custom elements)
    if !selector.contains('[') && !selector.contains(':') {
        return model.tags.contains(selector);
    }

    // Pseudo-class selectors - can't validate statically, assume true
    if selector.contains(':') {
        return true;
    }

    // For any other complex selectors, assume they might match
    true
}

/// Strip pseudo-classes and pseudo-elements from a selector.
///
/// Examples:
/// - `.foo:first-child` -> `.foo`
/// - `.foo::before` -> `.foo`
/// - `.foo:hover:focus` -> `.foo`
/// - `.foo` -> `.foo` (unchanged)
fn strip_pseudo(selector: &str) -> &str {
    if let Some(pos) = selector.find(':') {
        &selector[..pos]
    } else {
        selector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <div id="main" class="container">
                    <h1 class="title">Hello</h1>
                </div>
            </body>
            </html>
        "#;

        let model = parse_html(html).unwrap();

        assert!(model.ids.contains("main"));
        assert!(model.classes.contains("container"));
        assert!(model.classes.contains("title"));
        assert!(model.tags.contains("div"));
        assert!(model.tags.contains("h1"));
    }

    #[test]
    fn slot_usage_resolves_through_nested_non_custom_wrappers() {
        // FEAT-095: lol_html tracked the enclosing custom element via a stack;
        // the html5ever port walks the parent chain. A slotted element nested
        // under plain wrappers inside a custom element must still attribute its
        // slot to that custom element.
        let html = r#"<ik-hero><div class="wrap"><section><span slot="title">Hi</span></section></div></ik-hero>"#;
        let model = parse_html(html).unwrap();
        assert!(
            model.slot_usage.get("ik-hero").unwrap().contains("title"),
            "slot must attribute to the nearest custom-element ancestor through plain wrappers"
        );
    }

    #[test]
    fn parse_preserves_document_order_depth_and_parent() {
        // FEAT-095: pre-order DFS must reproduce lol_html's document order, with
        // correct depth and parent_index linkage.
        let html = r#"<main><h1 class="t">A</h1><ul><li>x</li><li>y</li></ul></main>"#;
        let model = parse_html(html).unwrap();
        let tags: Vec<&str> = model.elements.iter().map(|e| e.tag.as_str()).collect();
        assert_eq!(tags, vec!["main", "h1", "ul", "li", "li"]);
        // depths: main=0, h1=1, ul=1, li=2, li=2
        let depths: Vec<usize> = model.elements.iter().map(|e| e.depth).collect();
        assert_eq!(depths, vec![0, 1, 1, 2, 2]);
        // parents: main=None, h1=main(0), ul=main(0), li=ul(2), li=ul(2)
        let parents: Vec<Option<usize>> = model.elements.iter().map(|e| e.parent_index).collect();
        assert_eq!(parents, vec![None, Some(0), Some(0), Some(2), Some(2)]);
    }

    #[test]
    fn test_parse_custom_elements() {
        let html = r#"
            <ik-hero>
                <span slot="title">Hello</span>
                <span slot="subtitle">World</span>
            </ik-hero>
        "#;

        let model = parse_html(html).unwrap();

        assert!(model.custom_elements.contains("ik-hero"));
        assert!(model.slot_usage.get("ik-hero").unwrap().contains("title"));
        assert!(
            model
                .slot_usage
                .get("ik-hero")
                .unwrap()
                .contains("subtitle")
        );
    }

    #[test]
    fn test_selector_exists_id() {
        let html = r#"<div id="main"></div>"#;
        let model = parse_html(html).unwrap();

        assert!(selector_exists(&model, "#main"));
        assert!(!selector_exists(&model, "#other"));
    }

    #[test]
    fn test_selector_exists_class() {
        let html = r#"<div class="container active"></div>"#;
        let model = parse_html(html).unwrap();

        assert!(selector_exists(&model, ".container"));
        assert!(selector_exists(&model, ".active"));
        assert!(!selector_exists(&model, ".other"));
    }

    #[test]
    fn test_selector_exists_compound() {
        let html = r#"
            <div class="parent">
                <div class="child"></div>
            </div>
        "#;
        let model = parse_html(html).unwrap();

        assert!(selector_exists(&model, ".parent .child"));
        assert!(selector_exists(&model, ".parent > .child"));
    }

    #[test]
    fn test_selector_exists_tag_class() {
        let html = r#"<canvas class="showcase"></canvas>"#;
        let model = parse_html(html).unwrap();

        assert!(selector_exists(&model, "canvas.showcase"));
        assert!(selector_exists(&model, "canvas"));
        assert!(selector_exists(&model, ".showcase"));
        assert!(!selector_exists(&model, "div.showcase")); // wrong tag
        assert!(!selector_exists(&model, "canvas.other")); // wrong class
    }

    #[test]
    fn test_selector_exists_tag_multiple_classes() {
        let html = r#"<div class="container active large"></div>"#;
        let model = parse_html(html).unwrap();

        assert!(selector_exists(&model, "div.container"));
        assert!(selector_exists(&model, "div.active"));
        assert!(selector_exists(&model, "div.container.active"));
        assert!(!selector_exists(&model, "div.container.missing"));
    }

    #[test]
    fn test_selector_exists_slot() {
        let html = r#"
            <ik-hero>
                <span slot="title">Hello</span>
            </ik-hero>
        "#;
        let model = parse_html(html).unwrap();

        assert!(selector_exists(&model, "[slot=\"title\"]"));
        assert!(selector_exists(&model, "[slot=title]"));
    }
}
