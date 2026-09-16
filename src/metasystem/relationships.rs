//! Macro Relationship Analysis (Level 1 Inference)
//!
//! This module analyzes `%includes` clauses to infer relationships between macros.
//! When macro A uses `%includes @B(...)`, we can determine:
//! - Which params are forwarded vs pre-filled
//! - Whether A is a partial application, wrapper, or override of B
//! - Type compatibility across the %includes boundary

use std::collections::{HashMap, HashSet};

use crate::parser::meta_ast::{IncludedPattern, IncludesClause, MacroBodyItem, MacroDefAst};

/// Relationship type between child and parent macro
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
    /// Child exposes subset of parent params with some pre-filled
    Partial,
    /// Child adds behavior on top of parent (all parent params exposed + extras)
    Wrapper,
    /// Child reimplements (no %includes to this parent)
    Override,
}

/// Represents a relationship between a child macro and its parent
#[derive(Debug, Clone)]
pub struct MacroRelationship {
    /// Name of the child macro (e.g., "fade-in-up")
    pub child: String,
    /// Name of the parent macro (e.g., "fade-in")
    pub parent: String,
    /// Type of relationship
    pub relationship: RelationshipType,
    /// Params that are forwarded from child to parent
    pub forwarded_params: Vec<String>,
    /// Params that are pre-filled with values (name, value)
    pub prefilled_params: Vec<(String, String)>,
}

impl MacroRelationship {
    /// Format for display in LSP hover
    pub fn format_for_hover(&self) -> String {
        let rel_str = match self.relationship {
            RelationshipType::Partial => "partial of",
            RelationshipType::Wrapper => "wrapper of",
            RelationshipType::Override => "override of",
        };

        let mut lines = vec![format!("({} @{})", rel_str, self.parent)];

        if !self.forwarded_params.is_empty() {
            lines.push(String::new());
            lines.push("Forwarded params:".to_string());
            for param in &self.forwarded_params {
                lines.push(format!("  {}", param));
            }
        }

        if !self.prefilled_params.is_empty() {
            lines.push(String::new());
            lines.push(format!("Pre-filled from @{}:", self.parent));
            for (name, value) in &self.prefilled_params {
                lines.push(format!("  {} = {}", name, value));
            }
        }

        lines.join("\n")
    }
}

/// Registry of macro relationships
#[derive(Debug, Clone, Default)]
pub struct RelationshipRegistry {
    /// Map from child macro name to its relationships
    relationships: HashMap<String, Vec<MacroRelationship>>,
}

impl RelationshipRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a macro definition and extract relationships from %includes
    pub fn analyze_macro(
        &mut self,
        macro_def: &MacroDefAst,
        parent_forms: &HashMap<String, Vec<String>>,
    ) {
        // Find all %includes in the macro body
        for item in &macro_def.body {
            if let MacroBodyItem::Includes(includes) = item {
                self.analyze_includes(&macro_def.name, macro_def, includes, parent_forms);
            }
        }
    }

    /// Analyze an %includes clause to extract relationships
    fn analyze_includes(
        &mut self,
        child_name: &str,
        child_def: &MacroDefAst,
        includes: &IncludesClause,
        parent_forms: &HashMap<String, Vec<String>>,
    ) {
        // Get the child's form params
        let child_params: HashSet<String> = if let Some(ref form) = child_def.form {
            form.params.iter().map(|p| p.name.clone()).collect()
        } else {
            HashSet::new()
        };

        // Analyze each included pattern
        for pattern in &includes.patterns {
            let parent_name = &pattern.name;

            // Get parent's expected params (if known)
            let parent_params: HashSet<String> = parent_forms
                .get(parent_name)
                .map(|params| params.iter().cloned().collect())
                .unwrap_or_default();

            // Analyze which params are forwarded vs pre-filled
            let (forwarded, prefilled) = Self::analyze_param_forwarding(pattern, &child_params);

            // Determine relationship type
            let relationship = if forwarded.len() < parent_params.len() || !prefilled.is_empty() {
                // Child exposes fewer params than parent -> Partial
                RelationshipType::Partial
            } else if child_params.len() > parent_params.len() {
                // Child has more params than parent -> Wrapper
                RelationshipType::Wrapper
            } else {
                // Same params -> Could be either, default to Partial
                RelationshipType::Partial
            };

            let rel = MacroRelationship {
                child: child_name.to_string(),
                parent: parent_name.clone(),
                relationship,
                forwarded_params: forwarded,
                prefilled_params: prefilled,
            };

            self.relationships
                .entry(child_name.to_string())
                .or_default()
                .push(rel);
        }
    }

    /// Analyze which params are forwarded (using $var) vs pre-filled (literal values)
    fn analyze_param_forwarding(
        pattern: &IncludedPattern,
        child_params: &HashSet<String>,
    ) -> (Vec<String>, Vec<(String, String)>) {
        let mut forwarded = Vec::new();
        let mut prefilled = Vec::new();

        for arg in &pattern.args {
            let is_var = arg.value.starts_with('$');

            if is_var {
                // Extract variable name (strip $)
                let var_name = arg.value.trim_start_matches('$');
                if child_params.contains(var_name) || child_params.contains(&arg.name) {
                    forwarded.push(arg.name.clone());
                } else {
                    // Variable not in child's params - might be derived or pre-filled
                    prefilled.push((arg.name.clone(), arg.value.clone()));
                }
            } else {
                // Literal value - pre-filled
                prefilled.push((arg.name.clone(), arg.value.clone()));
            }
        }

        (forwarded, prefilled)
    }

    /// Get relationships for a macro
    pub fn get_relationships(&self, macro_name: &str) -> Option<&Vec<MacroRelationship>> {
        self.relationships.get(macro_name)
    }

    /// Get the first relationship for a macro (for simple hover display)
    pub fn get_primary_relationship(&self, macro_name: &str) -> Option<&MacroRelationship> {
        self.relationships.get(macro_name).and_then(|r| r.first())
    }

    /// Check if a macro has any relationships
    pub fn has_relationships(&self, macro_name: &str) -> bool {
        self.relationships.contains_key(macro_name)
    }

    /// Get all macro names that have relationships
    pub fn macro_names(&self) -> impl Iterator<Item = &str> {
        self.relationships.keys().map(|s| s.as_str())
    }

    /// Register a relationship directly
    pub fn register(&mut self, relationship: MacroRelationship) {
        self.relationships
            .entry(relationship.child.clone())
            .or_default()
            .push(relationship);
    }
}

/// Extract param names from a macro's form clause
pub fn extract_form_params(macro_def: &MacroDefAst) -> Vec<String> {
    macro_def
        .form
        .as_ref()
        .map(|form| form.params.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use crate::parser::meta_ast::*;

    fn make_form_param(name: &str) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: name.to_string(),
                    capture_type: CaptureType::Duration,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            default: None,
        }
    }

    fn make_included_pattern(name: &str, args: Vec<(&str, &str)>) -> IncludedPattern {
        IncludedPattern {
            name: name.to_string(),
            args: args
                .into_iter()
                .map(|(n, v)| IncludedPatternArg {
                    name: n.to_string(),
                    value: v.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_partial_application_detection() {
        let mut registry = RelationshipRegistry::new();

        // Simulate fade-in-up which is a partial of fade-in
        let fade_in_up = MacroDefAst {
            retired: None,
            name: "fade-in-up".to_string(),
            form: Some(FormClause {
                directive_name: "fade-in-up".to_string(),
                inline_elements: vec![],
                params: vec![make_form_param("duration"), make_form_param("distance")],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![MacroBodyItem::Includes(IncludesClause {
                patterns: vec![make_included_pattern(
                    "fade-in",
                    vec![
                        ("duration", "$duration"),
                        ("distance", "$distance"),
                        ("threshold", "0.1"), // Pre-filled
                        ("delay", "0"),       // Pre-filled
                    ],
                )],
                span: SourceSpan::default(),
            })],
            requires: vec![],
            span: SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        // Parent macro forms
        let mut parent_forms = HashMap::new();
        parent_forms.insert(
            "fade-in".to_string(),
            vec![
                "duration".to_string(),
                "distance".to_string(),
                "threshold".to_string(),
                "delay".to_string(),
            ],
        );

        registry.analyze_macro(&fade_in_up, &parent_forms);

        let rel = registry.get_primary_relationship("fade-in-up").unwrap();
        assert_eq!(rel.parent, "fade-in");
        assert_eq!(rel.relationship, RelationshipType::Partial);
        assert!(rel.forwarded_params.contains(&"duration".to_string()));
        assert!(rel.forwarded_params.contains(&"distance".to_string()));
        assert!(rel.prefilled_params.iter().any(|(n, _)| n == "threshold"));
        assert!(rel.prefilled_params.iter().any(|(n, _)| n == "delay"));
    }

    #[test]
    fn test_format_for_hover() {
        let rel = MacroRelationship {
            child: "fade-in-up".to_string(),
            parent: "fade-in".to_string(),
            relationship: RelationshipType::Partial,
            forwarded_params: vec!["duration".to_string(), "distance".to_string()],
            prefilled_params: vec![
                ("threshold".to_string(), "0.1".to_string()),
                ("delay".to_string(), "0".to_string()),
            ],
        };

        let hover = rel.format_for_hover();
        assert!(hover.contains("partial of @fade-in"));
        assert!(hover.contains("Forwarded params:"));
        assert!(hover.contains("duration"));
        assert!(hover.contains("Pre-filled from @fade-in:"));
        assert!(hover.contains("threshold = 0.1"));
    }
}
