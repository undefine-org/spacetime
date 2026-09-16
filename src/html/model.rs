//! Data structures for HTML-aware validation.
//!
//! Provides types for representing parsed HTML structure, template definitions,
//! and selector matching results.

use std::collections::{HashMap, HashSet};

/// Complete HTML context for validation.
///
/// Combines document structure from index.html with template definitions
/// from templates.html to enable comprehensive selector validation.
#[derive(Debug, Clone, Default)]
pub struct HtmlContext {
    /// Elements directly in index.html
    pub document: DocumentModel,
    /// Parsed component templates
    pub templates: TemplateRegistry,
    /// Source file path for error messages
    pub source_path: String,
}

impl HtmlContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all classes that would exist in the expanded DOM
    /// (includes both document classes and template internal classes)
    pub fn all_classes(&self) -> HashSet<String> {
        let mut classes = self.document.classes.clone();

        // Expand custom elements to include their internal classes
        for custom_el in &self.document.custom_elements {
            if let Some(template) = self.templates.get(custom_el) {
                classes.extend(template.internal_classes.iter().cloned());
            }
        }

        classes
    }

    /// Get all IDs that would exist in the expanded DOM
    pub fn all_ids(&self) -> HashSet<String> {
        let mut ids = self.document.ids.clone();

        for custom_el in &self.document.custom_elements {
            if let Some(template) = self.templates.get(custom_el) {
                ids.extend(template.internal_ids.iter().cloned());
            }
        }

        ids
    }

    /// Check if a slot exists in a component
    pub fn validate_slot(&self, component: &str, slot: &str) -> Result<(), String> {
        match self.templates.get(component) {
            Some(template) => {
                if slot.is_empty() {
                    if template.has_default_slot {
                        Ok(())
                    } else {
                        Err(format!("Component '{}' has no default slot", component))
                    }
                } else if template.slots.contains(slot) {
                    Ok(())
                } else {
                    let available: Vec<&str> = template.slots.iter().map(|s| s.as_str()).collect();
                    Err(format!(
                        "Slot '{}' not found in '{}'. Available: {}",
                        slot,
                        component,
                        available.join(", ")
                    ))
                }
            }
            None => Err(format!("Unknown component '{}'", component)),
        }
    }
}

/// Representation of the document's DOM structure.
///
/// Tracks all elements, IDs, classes, and custom elements found in the HTML.
#[derive(Debug, Clone, Default)]
pub struct DocumentModel {
    /// All element IDs found (without #)
    pub ids: HashSet<String>,
    /// All class names found (without .)
    pub classes: HashSet<String>,
    /// All tag names found (lowercase)
    pub tags: HashSet<String>,
    /// Custom elements (hyphenated tags like ik-nav, ik-hero)
    pub custom_elements: HashSet<String>,
    /// Slot usage: maps custom element tag -> slots used within it
    /// e.g., "ik-hero" -> {"logo", "title", "tagline"}
    pub slot_usage: HashMap<String, HashSet<String>>,
    /// Hierarchical element data for complex queries
    pub elements: Vec<ElementInfo>,
}

/// Information about a single element in the DOM.
#[derive(Debug, Clone)]
pub struct ElementInfo {
    /// Tag name (lowercase)
    pub tag: String,
    /// Element ID if present (without #)
    pub id: Option<String>,
    /// Class names (without .)
    pub classes: Vec<String>,
    /// Slot attribute value if present
    pub slot: Option<String>,
    /// data-component attribute if this is a template
    pub data_component: Option<String>,
    /// Other attributes (data-*, href, etc.)
    pub attributes: HashMap<String, String>,
    /// Source span in HTML file
    pub span: HtmlSpan,
    /// Parent element index (for hierarchy)
    pub parent_index: Option<usize>,
    /// Depth in DOM tree (0 = root)
    pub depth: usize,
}

impl ElementInfo {
    /// Create a new ElementInfo with just a tag name
    pub fn new(tag: String) -> Self {
        Self {
            tag,
            id: None,
            classes: Vec::new(),
            slot: None,
            data_component: None,
            attributes: HashMap::new(),
            span: HtmlSpan::default(),
            parent_index: None,
            depth: 0,
        }
    }
}

/// Source location in HTML file.
#[derive(Debug, Clone, Copy, Default)]
pub struct HtmlSpan {
    /// Byte offset start
    pub start: usize,
    /// Byte offset end
    pub end: usize,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

/// Registry of all component templates.
#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    /// Maps component name (e.g., "ik-projects") to its definition
    pub components: HashMap<String, ComponentTemplate>,
}

impl TemplateRegistry {
    /// Get a component by its tag name
    pub fn get(&self, name: &str) -> Option<&ComponentTemplate> {
        self.components.get(name)
    }

    /// Check if a slot exists in a component
    pub fn has_slot(&self, component: &str, slot: &str) -> bool {
        self.components
            .get(component)
            .map(|t| t.slots.contains(slot) || (slot.is_empty() && t.has_default_slot))
            .unwrap_or(false)
    }

    /// Get all component names
    pub fn component_names(&self) -> Vec<&str> {
        self.components.keys().map(|s| s.as_str()).collect()
    }
}

/// A parsed component template.
///
/// Represents a `<template data-component="...">` definition.
#[derive(Debug, Clone)]
pub struct ComponentTemplate {
    /// Component name from data-component attribute
    pub name: String,
    /// Available slot names (from <slot name="...">)
    pub slots: HashSet<String>,
    /// Has default slot (<slot> without name)?
    pub has_default_slot: bool,
    /// Internal classes this component produces
    /// e.g., ik-projects produces ["ik-projects__grid", "ik-projects__header", ...]
    pub internal_classes: HashSet<String>,
    /// Internal IDs this component produces
    pub internal_ids: HashSet<String>,
    /// Nested custom elements within this template
    pub nested_components: HashSet<String>,
    /// Internal element structure (for complex queries)
    pub elements: Vec<ElementInfo>,
    /// Source span in templates.html
    pub span: HtmlSpan,
}

impl ComponentTemplate {
    /// Create a new template with just a name
    pub fn new(name: String) -> Self {
        Self {
            name,
            slots: HashSet::new(),
            has_default_slot: false,
            internal_classes: HashSet::new(),
            internal_ids: HashSet::new(),
            nested_components: HashSet::new(),
            elements: Vec::new(),
            span: HtmlSpan::default(),
        }
    }
}
