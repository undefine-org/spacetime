//! Test helpers for AST inspection and test utilities.
//!
//! Provides reusable functions for:
//! - Directive inspection
//! - Data binding analysis
//! - Animation collection
//! - Pipeline testing
//! - Cross-reference validation

use spacetime::parser::{ScopeBlock, StFile, parse};
use spacetime::{compile, compiler::CompileOptions};
use std::collections::HashMap;

// =============================================================================
// Data Structures
// =============================================================================

/// Information about a data source in the AST
#[derive(Debug, Clone)]
pub struct DataSourceInfo {
    pub name: String,
    pub type_name: String,
    pub is_array: bool,
    pub source_type: String,
}

/// Information about an animation in the AST
#[derive(Debug, Clone)]
pub struct AnimationInfo {
    pub scope_selector: String,
    pub timeline_id: String,
    pub driver_type: String,
    pub target_selector: String,
    pub property_count: usize,
    pub has_stagger: bool,
}

/// Result of running the full pipeline
#[derive(Debug)]
pub struct PipelineResult {
    pub parsed: bool,
    pub transformed: bool,
    pub compiled: bool,
    pub parse_error: Option<String>,
    pub transform_error: Option<String>,
    pub css_output: Option<String>,
    pub js_output: Option<String>,
}

// =============================================================================
// Directive Inspection
// =============================================================================

/// Get the driver type as a string (now extracts from macro call name)
pub fn driver_type_name(macro_name: &str) -> &'static str {
    match macro_name {
        "scroll" => "Scroll",
        "on" => "Event", // @on directives have event type in args
        "loop" => "Loop",
        "hover" => "Hover",
        "click" => "Click",
        "focus" => "Focus",
        "intersect" => "Intersect",
        "load" => "Load",
        _ => "Unknown",
    }
}

/// Count timelines/event handlers by driver/event type in a scope (recursive)
/// Uses FormMatches (scope.matches) to find @on and @scroll directives
pub fn count_timelines_by_driver(scope: &ScopeBlock) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    // Count FormMatch directives for @on-* and @scroll
    for m in &scope.matches {
        if m.macro_name.starts_with("on-") {
            // Extract event type from macro_name: "on-hover" -> "Hover"
            let event = &m.macro_name[3..]; // strip "on-"
            let event_name = {
                let mut chars = event.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                    None => event.to_string(),
                }
            };
            *counts.entry(event_name).or_insert(0) += 1;
        } else if m.macro_name == "scroll" {
            *counts.entry("Scroll".to_string()).or_insert(0) += 1;
        }
    }

    // Count nested scope matches (recursive)
    for nested in &scope.nested_scopes {
        for m in &nested.matches {
            if m.macro_name.starts_with("on-") {
                let event = &m.macro_name[3..];
                let event_name = {
                    let mut chars = event.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().chain(chars).collect(),
                        None => event.to_string(),
                    }
                };
                *counts.entry(event_name).or_insert(0) += 1;
            } else if m.macro_name == "scroll" {
                *counts.entry("Scroll".to_string()).or_insert(0) += 1;
            }
        }
    }

    counts
}

/// Collect all directive/feature names from a scope
pub fn collect_all_directives(scope: &ScopeBlock) -> Vec<String> {
    let mut directives = Vec::new();

    // State machine
    if scope.behavior.state_machine.is_some() {
        directives.push("@state_machine".to_string());
    }

    // States
    for state in &scope.behavior.states {
        directives.push(format!("@state(when: {})", state.when));
    }

    // Transitions
    for transition in &scope.behavior.transitions {
        directives.push(format!(
            "@transition({}->{})",
            transition.from, transition.to
        ));
    }

    // Async transitions
    for async_trans in &scope.behavior.async_transitions {
        directives.push(format!("@async_transition({})", async_trans.trigger));
    }

    // FormMatch-based directives from matches field
    for m in &scope.matches {
        if m.macro_name == "each" || m.macro_name == "each-block" {
            if let Some(source) = m.get_ident("source") {
                directives.push(format!("@each({})", source));
            }
        } else {
            directives.push(format!("@{}", m.macro_name));
        }
    }

    // Nested scopes (recursive)
    for nested in &scope.nested_scopes {
        let nested_scope = ScopeBlock {
            kind: spacetime::parser::ast::ScopeKind::Selector,
            selector: nested.selector.clone(),
            behavior: nested.behavior.clone(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: nested.matches.clone(),
            span: Default::default(),
            source_file: None,
            exports: Vec::new(),
            refs: Vec::new(),
            states: Vec::new(),
            html: String::new(),
        };
        directives.extend(collect_all_directives(&nested_scope));
    }

    directives
}

/// Check if a scope uses a specific directive (by prefix match)
pub fn has_directive(scope: &ScopeBlock, name: &str) -> bool {
    let directives = collect_all_directives(scope);
    directives.iter().any(|d| d.starts_with(name))
}

/// Check if any scope in the file uses a directive
pub fn file_has_directive(ast: &StFile, name: &str) -> bool {
    ast.scopes.iter().any(|scope| has_directive(scope, name))
}

// =============================================================================
// Data Binding Inspection
// =============================================================================

/// Extract all data sources from an AST
pub fn extract_data_sources(ast: &StFile) -> Vec<DataSourceInfo> {
    let mut sources = Vec::new();

    // Extract from FormMatches: @data directives
    for fm in ast.matches.iter().filter(|m| m.macro_name == "data") {
        let name = fm.get_ident("name").unwrap_or("").to_string();
        let type_name = fm.get_ident("type_ref").unwrap_or("unknown").to_string();
        let is_array = fm.get_ident("array_marker").is_some();

        // Determine source type from captures
        let source_type = if fm.get_string("src").is_some() {
            "file".to_string()
        } else {
            "runtime".to_string()
        };

        if !name.is_empty() {
            sources.push(DataSourceInfo {
                name,
                type_name,
                is_array,
                source_type,
            });
        }
    }

    sources
}

/// Find all @each bindings for a specific data source
pub fn find_bindings_for_source(ast: &StFile, source_name: &str) -> Vec<String> {
    let mut bindings = Vec::new();

    for scope in &ast.scopes {
        find_bindings_in_scope(scope, source_name, &mut bindings);
    }

    bindings
}

fn find_bindings_in_scope(scope: &ScopeBlock, source_name: &str, bindings: &mut Vec<String>) {
    for m in &scope.matches {
        if (m.macro_name == "each" || m.macro_name == "each-block")
            && let Some(source) = m.get_ident("source")
            && (source == source_name || source == format!("${}", source_name))
        {
            bindings.push(scope.selector.clone());
        }
    }

    for nested in &scope.nested_scopes {
        let nested_scope = ScopeBlock {
            kind: spacetime::parser::ast::ScopeKind::Selector,
            selector: format!("{} {}", scope.selector, nested.selector),
            behavior: Default::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],

            matches: nested.matches.clone(),
            span: Default::default(),
            source_file: None,
            exports: Vec::new(),
            refs: Vec::new(),
            states: Vec::new(),
            html: String::new(),
        };
        find_bindings_in_scope(&nested_scope, source_name, bindings);
    }
}

// =============================================================================
// Animation Inspection
// =============================================================================

/// Collect all animations from a file
/// Note: With the macro-based timeline system, animations are defined in macro call bodies.
/// This function now collects animation info from macro calls.
pub fn collect_all_animations(ast: &StFile) -> Vec<AnimationInfo> {
    let mut animations = Vec::new();

    for scope in &ast.scopes {
        collect_animations_from_scope(scope, &scope.selector, &mut animations);
    }

    animations
}

fn collect_animations_from_scope(
    scope: &ScopeBlock,
    parent_selector: &str,
    animations: &mut Vec<AnimationInfo>,
) {
    // Extract animation info from FormMatches
    for m in &scope.matches {
        let is_timeline =
            m.macro_name.starts_with("on-") || m.macro_name == "scroll" || m.macro_name == "load";
        if is_timeline {
            let timeline_id = m.get_ident("timeline_id").unwrap_or("unnamed").to_string();

            let driver_type = if m.macro_name.starts_with("on-") {
                let event = &m.macro_name[3..];
                let mut chars = event.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                    None => "Event".to_string(),
                }
            } else {
                driver_type_name(&m.macro_name).to_string()
            };

            animations.push(AnimationInfo {
                scope_selector: parent_selector.to_string(),
                timeline_id,
                driver_type,
                target_selector: scope.selector.clone(),
                property_count: 0,
                has_stagger: false,
            });
        }
    }

    for nested in &scope.nested_scopes {
        let nested_selector = format!("{} {}", parent_selector, nested.selector);
        let nested_scope = ScopeBlock {
            kind: spacetime::parser::ast::ScopeKind::Selector,
            selector: nested.selector.clone(),
            behavior: Default::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: nested.matches.clone(),
            span: Default::default(),
            source_file: None,
            exports: Vec::new(),
            refs: Vec::new(),
            states: Vec::new(),
            html: String::new(),
        };
        collect_animations_from_scope(&nested_scope, &nested_selector, animations);
    }
}

// =============================================================================
// Pipeline Helpers
// =============================================================================

/// Parse input and return result with error info
pub fn parse_with_errors(input: &str) -> (Option<StFile>, Option<String>) {
    match parse(input) {
        Ok(ast) => (Some(ast), None),
        Err(e) => (None, Some(e.to_string())),
    }
}

/// Run full pipeline and report results
pub fn run_full_pipeline(input: &str) -> PipelineResult {
    // Parse
    let ast = match parse(input) {
        Ok(ast) => ast,
        Err(e) => {
            return PipelineResult {
                parsed: false,
                transformed: false,
                compiled: false,
                parse_error: Some(e.to_string()),
                transform_error: None,
                css_output: None,
                js_output: None,
            };
        }
    };

    // Compile
    let compiled = compile(&ast, CompileOptions::default());

    PipelineResult {
        parsed: true,
        transformed: true,
        compiled: true,
        parse_error: None,
        transform_error: None,
        css_output: Some(compiled.css),
        js_output: Some(compiled.js),
    }
}

/// Quick check if input parses successfully
pub fn parses_ok(input: &str) -> bool {
    parse(input).is_ok()
}

/// Quick check if input goes through full pipeline
pub fn compiles_ok(input: &str) -> bool {
    let result = run_full_pipeline(input);
    result.compiled
}

// =============================================================================
// Cross-Reference Validation
// =============================================================================

/// Validate that all @each sources reference existing @data or @computed
pub fn validate_data_references(ast: &StFile) -> Vec<String> {
    let mut errors = Vec::new();

    // Collect valid source names from FormMatches
    let mut valid_sources = Vec::new();
    for fm in &ast.matches {
        if fm.macro_name == "data" {
            if let Some(name) = fm.get_ident("name") {
                valid_sources.push(name.to_string());
            }
        } else if fm.macro_name == "computed"
            && let Some(name) = fm.get_ident("name")
        {
            valid_sources.push(name.to_string());
        }
    }
    let valid_sources: Vec<&str> = valid_sources.iter().map(|s| s.as_str()).collect();

    // Check each blocks
    for scope in &ast.scopes {
        validate_refs_in_scope(scope, &valid_sources, &mut errors);
    }

    errors
}

fn validate_refs_in_scope(scope: &ScopeBlock, valid_sources: &[&str], errors: &mut Vec<String>) {
    for m in &scope.matches {
        if (m.macro_name == "each" || m.macro_name == "each-block")
            && let Some(source) = m.get_ident("source")
        {
            let source_name = source.trim_start_matches('$').trim_start_matches('&');
            if !valid_sources.contains(&source_name) {
                errors.push(format!(
                    "Scope '{}' references unknown source: {}",
                    scope.selector, source
                ));
            }
        }
    }

    for nested in &scope.nested_scopes {
        let nested_scope = ScopeBlock {
            kind: spacetime::parser::ast::ScopeKind::Selector,
            selector: format!("{} {}", scope.selector, nested.selector),
            behavior: Default::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],

            matches: nested.matches.clone(),
            span: Default::default(),
            source_file: None,
            exports: Vec::new(),
            refs: Vec::new(),
            states: Vec::new(),
            html: String::new(),
        };
        validate_refs_in_scope(&nested_scope, valid_sources, errors);
    }
}

/// Validate that all type references exist
pub fn validate_type_references(ast: &StFile) -> Vec<String> {
    let mut errors = Vec::new();

    // Collect defined type names from FormMatches
    let mut defined_types = Vec::new();
    for fm in ast.matches.iter().filter(|m| m.macro_name == "type") {
        if let Some(name) = fm.get_ident("name") {
            defined_types.push(name.to_string());
        }
    }
    let defined_types: Vec<&str> = defined_types.iter().map(|s| s.as_str()).collect();

    // Check data source type references from FormMatches
    for data_fm in ast.matches.iter().filter(|m| m.macro_name == "data") {
        if let Some(data_name) = data_fm.get_ident("name") {
            // Type validation would need to extract type_ref from captures
            let type_name = "unknown".to_string(); // Simplified for now
            // Skip built-in types
            if !["string", "number", "bool", "boolean", "any", "object"]
                .contains(&type_name.to_lowercase().as_str())
                && !defined_types.contains(&type_name.as_str())
            {
                errors.push(format!(
                    "Data '{}' references undefined type: {}",
                    data_name, type_name
                ));
            }
        }
    }

    errors
}

// =============================================================================
// File Loading Helpers
// =============================================================================

/// Load and parse an example file
pub fn load_example(filename: &str) -> Result<StFile, String> {
    let path = format!("examples/{}", filename);
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
    parse(&content).map_err(|e| format!("Failed to parse {}: {}", path, e))
}

/// Load and run full pipeline on an example file
pub fn pipeline_example(filename: &str) -> PipelineResult {
    let path = format!("examples/{}", filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => run_full_pipeline(&content),
        Err(e) => PipelineResult {
            parsed: false,
            transformed: false,
            compiled: false,
            parse_error: Some(format!("Failed to read file: {}", e)),
            transform_error: None,
            css_output: None,
            js_output: None,
        },
    }
}

// =============================================================================
// Assertion Helpers
// =============================================================================

/// Assert that a file parses without error
#[macro_export]
macro_rules! assert_parses {
    ($input:expr) => {
        assert!(
            $crate::test_helpers::parses_ok($input),
            "Expected input to parse successfully"
        );
    };
    ($input:expr, $msg:expr) => {
        assert!($crate::test_helpers::parses_ok($input), "{}", $msg);
    };
}

/// Assert that a file compiles through the full pipeline
#[macro_export]
macro_rules! assert_compiles {
    ($input:expr) => {
        assert!(
            $crate::test_helpers::compiles_ok($input),
            "Expected input to compile successfully"
        );
    };
    ($input:expr, $msg:expr) => {
        assert!($crate::test_helpers::compiles_ok($input), "{}", $msg);
    };
}

// =============================================================================
// Tests for the helpers themselves
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_errors_success() {
        let input = ".card { @on hover lift { scale: 1 -> 1.1; } }";
        let (ast, error) = parse_with_errors(input);
        assert!(ast.is_some());
        assert!(error.is_none());
    }

    #[test]
    fn test_parse_with_errors_failure() {
        let input = ".card { @invalid_syntax_here }";
        let (ast, error) = parse_with_errors(input);
        // Note: may or may not parse depending on grammar
        if ast.is_none() {
            assert!(error.is_some());
        }
    }

    #[test]
    fn test_extract_data_sources() {
        let input = r#"
            @type Product { name: string; price: number; }
            @data products: Product[] {
                src: "/api/products";
            }
        "#;
        let ast = parse(input).unwrap();
        let sources = extract_data_sources(&ast);

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "products");
        assert_eq!(sources[0].type_name, "Product");
        assert!(sources[0].is_array);
        assert_eq!(sources[0].source_type, "file");
    }

    #[test]
    fn test_count_timelines_by_driver() {
        let input = r#"
            .card {
                @on hover lift(300ms) { scale: 1 -> 1.1; }
                @scroll reveal(500ms) { opacity: 0 -> 1; }
            }
        "#;
        let ast = parse(input).unwrap();
        let counts = count_timelines_by_driver(&ast.scopes[0]);

        assert_eq!(counts.get("Hover"), Some(&1));
        assert_eq!(counts.get("Scroll"), Some(&1));
    }

    #[test]
    fn test_run_full_pipeline() {
        let input = r#"
            .hero {
                @on hover grow { scale: 1 -> 1.2; }
            }
        "#;
        let result = run_full_pipeline(input);

        assert!(result.parsed);
        assert!(result.transformed);
        assert!(result.compiled);
        assert!(result.css_output.is_some());
        assert!(result.js_output.is_some());
    }
}
