//! Visitor pattern for traversing FormMatch trees.
//!
//! Provides Visitor and VisitorMut traits for read-only and mutable traversals.
//! Use these instead of manual recursive iteration.

use super::form_match::{
    CapturedValue, FormMatch, KeyframeDef, ParamDef, PropertyDef, TemplateParamDef,
};

/// Read-only visitor for traversing FormMatch trees.
///
/// Default implementations walk the tree recursively, calling the appropriate
/// visit methods. Override specific methods to perform custom operations.
///
/// # Example
/// ```ignore
/// struct MacroCounter {
///     count: usize,
/// }
///
/// impl Visitor for MacroCounter {
///     fn visit_form_match(&mut self, form: &FormMatch) {
///         self.count += 1;
///         self.walk_form_match(form);
///     }
/// }
/// ```
pub trait Visitor: Sized {
    /// Visit a FormMatch node
    fn visit_form_match(&mut self, form: &FormMatch) {
        self.walk_form_match(form);
    }

    /// Visit a captured value
    fn visit_captured_value(&mut self, name: &str, value: &CapturedValue) {
        self.walk_captured_value(name, value);
    }

    /// Visit a property definition (from :properties capture)
    fn visit_property_def(&mut self, prop: &PropertyDef) {
        let _ = prop; // Default: do nothing
    }

    /// Visit a parameter definition (from :params capture)
    fn visit_param_def(&mut self, param: &ParamDef) {
        let _ = param; // Default: do nothing
    }

    /// Visit a keyframe definition (from :keyframes capture)
    fn visit_keyframe_def(&mut self, keyframe: &KeyframeDef) {
        let _ = keyframe; // Default: do nothing
    }

    /// Visit a template parameter definition (from :param_list capture)
    fn visit_template_param_def(&mut self, param: &TemplateParamDef) {
        let _ = param; // Default: do nothing
    }

    // === Walk methods (call these from your visit methods to continue traversal) ===

    /// Walk the children of a FormMatch
    fn walk_form_match(&mut self, form: &FormMatch) {
        for (name, value) in &form.captures {
            self.visit_captured_value(name, value);
        }
    }

    /// Walk a captured value, visiting nested structures
    fn walk_captured_value(&mut self, _name: &str, value: &CapturedValue) {
        match value {
            CapturedValue::Block(forms) => {
                for form in forms {
                    self.visit_form_match(form);
                }
            }
            CapturedValue::Array(values) => {
                for (i, v) in values.iter().enumerate() {
                    self.visit_captured_value(&i.to_string(), v);
                }
            }
            CapturedValue::Named(map) => {
                for (k, v) in map {
                    self.visit_captured_value(k, v);
                }
            }
            CapturedValue::Properties(props) => {
                for prop in props {
                    self.visit_property_def(prop);
                }
            }
            CapturedValue::Params(params) => {
                for param in params {
                    self.visit_param_def(param);
                }
            }
            CapturedValue::Keyframes(keyframes) => {
                for kf in keyframes {
                    self.visit_keyframe_def(kf);
                }
            }
            CapturedValue::ParamList(params) => {
                for param in params {
                    self.visit_template_param_def(param);
                }
            }
            // Leaf values - no children to visit
            _ => {}
        }
    }
}

/// Mutable visitor for transforming FormMatch trees.
///
/// Similar to Visitor but receives mutable references, allowing modification.
pub trait VisitorMut: Sized {
    /// Visit a FormMatch node (mutable)
    fn visit_form_match_mut(&mut self, form: &mut FormMatch) {
        self.walk_form_match_mut(form);
    }

    /// Visit a captured value (mutable)
    fn visit_captured_value_mut(&mut self, name: &str, value: &mut CapturedValue) {
        self.walk_captured_value_mut(name, value);
    }

    /// Visit a property definition (mutable)
    fn visit_property_def_mut(&mut self, prop: &mut PropertyDef) {
        let _ = prop;
    }

    /// Visit a parameter definition (mutable)
    fn visit_param_def_mut(&mut self, param: &mut ParamDef) {
        let _ = param;
    }

    /// Visit a keyframe definition (mutable)
    fn visit_keyframe_def_mut(&mut self, keyframe: &mut KeyframeDef) {
        let _ = keyframe;
    }

    /// Visit a template parameter definition (mutable)
    fn visit_template_param_def_mut(&mut self, param: &mut TemplateParamDef) {
        let _ = param;
    }

    // === Walk methods (mutable) ===

    fn walk_form_match_mut(&mut self, form: &mut FormMatch) {
        // Note: we need to iterate over keys first to avoid borrow issues
        let keys: Vec<String> = form.captures.keys().cloned().collect();
        for name in keys {
            if let Some(value) = form.captures.get_mut(&name) {
                self.visit_captured_value_mut(&name, value);
            }
        }
    }

    fn walk_captured_value_mut(&mut self, _name: &str, value: &mut CapturedValue) {
        match value {
            CapturedValue::Block(forms) => {
                for form in forms {
                    self.visit_form_match_mut(form);
                }
            }
            CapturedValue::Array(values) => {
                for (i, v) in values.iter_mut().enumerate() {
                    self.visit_captured_value_mut(&i.to_string(), v);
                }
            }
            CapturedValue::Named(map) => {
                let keys: Vec<String> = map.keys().cloned().collect();
                for k in keys {
                    if let Some(v) = map.get_mut(&k) {
                        self.visit_captured_value_mut(&k, v);
                    }
                }
            }
            CapturedValue::Properties(props) => {
                for prop in props {
                    self.visit_property_def_mut(prop);
                }
            }
            CapturedValue::Params(params) => {
                for param in params {
                    self.visit_param_def_mut(param);
                }
            }
            CapturedValue::Keyframes(keyframes) => {
                for kf in keyframes {
                    self.visit_keyframe_def_mut(kf);
                }
            }
            CapturedValue::ParamList(params) => {
                for param in params {
                    self.visit_template_param_def_mut(param);
                }
            }
            _ => {}
        }
    }
}

// === Utility visitors ===

/// Counts the total number of FormMatch nodes in a tree
pub struct FormMatchCounter {
    pub count: usize,
}

impl FormMatchCounter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn count(forms: &[FormMatch]) -> usize {
        let mut counter = Self::new();
        for form in forms {
            counter.visit_form_match(form);
        }
        counter.count
    }
}

impl Default for FormMatchCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for FormMatchCounter {
    fn visit_form_match(&mut self, form: &FormMatch) {
        self.count += 1;
        self.walk_form_match(form);
    }
}

/// Collects all FormMatch nodes with a specific macro name.
///
/// Returns a vector of indices representing paths to matching FormMatches.
/// Each index tuple contains (capture_key, array_index) for navigating the tree.
pub struct MacroCollector {
    target_name: String,
    macro_names: Vec<String>,
}

impl MacroCollector {
    pub fn new(target_name: impl Into<String>) -> Self {
        Self {
            target_name: target_name.into(),
            macro_names: Vec::new(),
        }
    }

    /// Collect all macro names that match the target in the tree.
    /// Returns the collected macro names (useful for counting/verification).
    pub fn collect(target_name: &str, forms: &[FormMatch]) -> Vec<String> {
        let mut collector = Self::new(target_name);
        for form in forms {
            collector.visit_form_match(form);
        }
        collector.macro_names
    }

    /// Check if any FormMatch with the target name exists in the tree.
    pub fn contains(target_name: &str, forms: &[FormMatch]) -> bool {
        let mut collector = Self::new(target_name);
        for form in forms {
            collector.visit_form_match(form);
            if !collector.macro_names.is_empty() {
                return true;
            }
        }
        false
    }

    /// Count how many FormMatches with the target name exist in the tree.
    pub fn count(target_name: &str, forms: &[FormMatch]) -> usize {
        Self::collect(target_name, forms).len()
    }
}

impl Visitor for MacroCollector {
    fn visit_form_match(&mut self, form: &FormMatch) {
        if form.macro_name == self.target_name {
            self.macro_names.push(form.macro_name.clone());
        }
        self.walk_form_match(form);
    }
}

/// Collects all variable bindings ($name) used in a tree
pub struct BindingCollector {
    pub bindings: Vec<String>,
}

impl BindingCollector {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn collect(forms: &[FormMatch]) -> Vec<String> {
        let mut collector = Self::new();
        for form in forms {
            collector.visit_form_match(form);
        }
        collector.bindings
    }
}

impl Default for BindingCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for BindingCollector {
    fn visit_captured_value(&mut self, _name: &str, value: &CapturedValue) {
        if let CapturedValue::Binding(binding) = value
            && !self.bindings.contains(binding)
        {
            self.bindings.push(binding.clone());
        }
        self.walk_captured_value(_name, value);
    }
}

/// Collects all element references (&name) used in a tree
pub struct ElementRefCollector {
    pub elements: Vec<String>,
}

impl ElementRefCollector {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn collect(forms: &[FormMatch]) -> Vec<String> {
        let mut collector = Self::new();
        for form in forms {
            collector.visit_form_match(form);
        }
        collector.elements
    }
}

impl Default for ElementRefCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for ElementRefCollector {
    fn visit_captured_value(&mut self, _name: &str, value: &CapturedValue) {
        if let CapturedValue::Element(elem) = value
            && !self.elements.contains(elem)
        {
            self.elements.push(elem.clone());
        }
        self.walk_captured_value(_name, value);
    }
}

/// Replaces all occurrences of a binding name with a new name
pub struct BindingRenamer {
    pub old_name: String,
    pub new_name: String,
    pub renamed_count: usize,
}

impl BindingRenamer {
    pub fn new(old_name: impl Into<String>, new_name: impl Into<String>) -> Self {
        Self {
            old_name: old_name.into(),
            new_name: new_name.into(),
            renamed_count: 0,
        }
    }

    pub fn rename(old_name: &str, new_name: &str, forms: &mut [FormMatch]) -> usize {
        let mut renamer = Self::new(old_name, new_name);
        for form in forms {
            renamer.visit_form_match_mut(form);
        }
        renamer.renamed_count
    }
}

impl VisitorMut for BindingRenamer {
    fn visit_captured_value_mut(&mut self, _name: &str, value: &mut CapturedValue) {
        if let CapturedValue::Binding(binding) = value
            && (binding == &self.old_name || binding == &format!("${}", self.old_name))
        {
            *binding = if self.new_name.starts_with('$') {
                self.new_name.clone()
            } else {
                format!("${}", self.new_name)
            };
            self.renamed_count += 1;
        }
        self.walk_captured_value_mut(_name, value);
    }
}

/// Visitor that pretty-prints the tree structure (for debugging)
pub struct TreePrinter {
    indent: usize,
    output: String,
}

impl TreePrinter {
    pub fn new() -> Self {
        Self {
            indent: 0,
            output: String::new(),
        }
    }

    pub fn print(forms: &[FormMatch]) -> String {
        let mut printer = Self::new();
        for form in forms {
            printer.visit_form_match(form);
        }
        printer.output
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }
}

impl Default for TreePrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for TreePrinter {
    fn visit_form_match(&mut self, form: &FormMatch) {
        self.write_indent();
        self.output
            .push_str(&format!("FormMatch({})\n", form.macro_name));
        self.indent += 1;
        self.walk_form_match(form);
        self.indent -= 1;
    }

    fn visit_captured_value(&mut self, name: &str, value: &CapturedValue) {
        self.write_indent();
        match value {
            CapturedValue::Block(_) => {
                self.output.push_str(&format!("{}: Block\n", name));
            }
            CapturedValue::Array(arr) => {
                self.output
                    .push_str(&format!("{}: Array({})\n", name, arr.len()));
            }
            _ => {
                self.output.push_str(&format!("{}: {:?}\n", name, value));
            }
        }
        self.indent += 1;
        self.walk_captured_value(name, value);
        self.indent -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use std::collections::HashMap;

    fn make_test_form(name: &str) -> FormMatch {
        FormMatch {
            macro_name: name.to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    fn make_form_with_binding(name: &str, binding: &str) -> FormMatch {
        let mut captures = HashMap::new();
        captures.insert(
            "var".to_string(),
            CapturedValue::Binding(binding.to_string()),
        );
        FormMatch {
            macro_name: name.to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    fn make_nested_form(name: &str, children: Vec<FormMatch>) -> FormMatch {
        let mut captures = HashMap::new();
        captures.insert("body".to_string(), CapturedValue::Block(children));
        FormMatch {
            macro_name: name.to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    #[test]
    fn test_form_match_counter() {
        let forms = vec![
            make_test_form("a"),
            make_nested_form("b", vec![make_test_form("c"), make_test_form("d")]),
        ];

        assert_eq!(FormMatchCounter::count(&forms), 4);
    }

    #[test]
    fn test_macro_collector() {
        let forms = vec![
            make_test_form("data"),
            make_nested_form(
                "scope",
                vec![make_test_form("data"), make_test_form("load")],
            ),
            make_test_form("fn"),
        ];

        let data_forms = MacroCollector::collect("data", &forms);
        assert_eq!(data_forms.len(), 2);
    }

    #[test]
    fn test_binding_collector() {
        let forms = vec![
            make_form_with_binding("a", "$count"),
            make_form_with_binding("b", "$items"),
            make_form_with_binding("c", "$count"), // duplicate
        ];

        let bindings = BindingCollector::collect(&forms);
        assert_eq!(bindings.len(), 2);
        assert!(bindings.contains(&"$count".to_string()));
        assert!(bindings.contains(&"$items".to_string()));
    }

    #[test]
    fn test_binding_renamer() {
        let mut forms = vec![
            make_form_with_binding("a", "$oldName"),
            make_form_with_binding("b", "$other"),
            make_form_with_binding("c", "$oldName"),
        ];

        let count = BindingRenamer::rename("$oldName", "$newName", &mut forms);
        assert_eq!(count, 2);

        // Verify the changes
        assert_eq!(
            forms[0].captures.get("var"),
            Some(&CapturedValue::Binding("$newName".to_string()))
        );
        assert_eq!(
            forms[1].captures.get("var"),
            Some(&CapturedValue::Binding("$other".to_string()))
        );
    }

    #[test]
    fn test_tree_printer() {
        let forms = vec![make_form_with_binding("data", "$count")];

        let output = TreePrinter::print(&forms);
        assert!(output.contains("FormMatch(data)"));
        assert!(output.contains("var:"));
    }

    #[test]
    fn test_element_ref_collector() {
        let mut form1 = make_test_form("a");
        form1.captures.insert(
            "elem".to_string(),
            CapturedValue::Element("button".to_string()),
        );

        let mut form2 = make_test_form("b");
        form2.captures.insert(
            "elem".to_string(),
            CapturedValue::Element("input".to_string()),
        );

        let mut form3 = make_test_form("c");
        form3.captures.insert(
            "elem".to_string(),
            CapturedValue::Element("button".to_string()), // duplicate
        );

        let forms = vec![form1, form2, form3];

        let elements = ElementRefCollector::collect(&forms);
        assert_eq!(elements.len(), 2);
        assert!(elements.contains(&"button".to_string()));
        assert!(elements.contains(&"input".to_string()));
    }

    #[test]
    fn test_nested_traversal() {
        // Create a deeply nested structure
        let inner = make_form_with_binding("inner", "$value");
        let middle = make_nested_form("middle", vec![inner]);
        let outer = make_nested_form("outer", vec![middle]);
        let forms = vec![outer];

        // Should find the binding in the deeply nested form
        let bindings = BindingCollector::collect(&forms);
        assert_eq!(bindings.len(), 1);
        assert!(bindings.contains(&"$value".to_string()));
    }

    #[test]
    fn test_array_traversal() {
        let mut form = make_test_form("test");
        form.captures.insert(
            "items".to_string(),
            CapturedValue::Array(vec![
                CapturedValue::Binding("$first".to_string()),
                CapturedValue::Binding("$second".to_string()),
            ]),
        );

        let forms = vec![form];
        let bindings = BindingCollector::collect(&forms);
        assert_eq!(bindings.len(), 2);
        assert!(bindings.contains(&"$first".to_string()));
        assert!(bindings.contains(&"$second".to_string()));
    }

    #[test]
    fn test_named_map_traversal() {
        let mut form = make_test_form("test");
        let mut named_map = HashMap::new();
        named_map.insert(
            "key1".to_string(),
            CapturedValue::Binding("$alpha".to_string()),
        );
        named_map.insert(
            "key2".to_string(),
            CapturedValue::Binding("$beta".to_string()),
        );
        form.captures
            .insert("named".to_string(), CapturedValue::Named(named_map));

        let forms = vec![form];
        let bindings = BindingCollector::collect(&forms);
        assert_eq!(bindings.len(), 2);
        assert!(bindings.contains(&"$alpha".to_string()));
        assert!(bindings.contains(&"$beta".to_string()));
    }
}
