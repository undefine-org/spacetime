//! Scope Tree (PROJ-102)
//!
//! Builds a compile-time scope tree from template definitions, mapping
//! each template to its declared state variables. This enables:
//! - Instance-scoped state via ST.set(rootEl, name, value)
//! - Diagnostics for undefined/shadowed/unused $var references
//! - Inspect output via `--layer scopes`

use std::collections::HashMap;

use serde::Serialize;

use crate::syntax::{CapturedValue, FormMatch};

// =============================================================================
// Types
// =============================================================================

/// A single state declaration within a scope (template or global).
#[derive(Debug, Clone, Serialize)]
pub struct StateDecl {
    /// Variable name (without `$` prefix)
    pub var_name: String,
    /// Type annotation (e.g., "number", "bool", "string", "Product[]")
    pub type_name: String,
    /// Initial value as a string (e.g., "0", "false", "\"hello\"")
    pub initial: String,
    /// PLAN-117 W2: the file that DECLARED this cell.
    ///
    /// `@import` merges many files into one page, so a page-global `$host` may
    /// come from anywhere in the import graph. Without this, two files declaring
    /// the same name merge SILENTLY — the defect E0938 exists to refuse. Read
    /// from `FormMatch.source_file`; `None` for the entry file itself (which the
    /// display renders as the entry path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
}

/// PLAN-117 W2: what the compiler can PROVE about a page-global cell.
///
/// The verdict is not a heuristic. Spacetime has no custom JavaScript — no
/// `eval`, no foreign call, no escape hatch — so a cell's write-set is CLOSED
/// and is settled by scanning the page's own matches. That closure is what makes
/// `Const` sound rather than opportunistic, and it is the input W4 folds on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StateVerdict {
    /// Written by nobody and initialised to a literal: a value wearing a
    /// signal's clothes. W4 inlines it at every read and emits no cell.
    Const,
    /// Has at least one writer, or a non-literal initial: a real reactive cell.
    Reactive,
    /// Read by nobody: emits nothing at all.
    Dead,
}

impl StateVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            StateVerdict::Const => "const",
            StateVerdict::Reactive => "reactive",
            StateVerdict::Dead => "dead",
        }
    }
}

/// PLAN-117 W2: provenance for ONE page-global cell — who declared it, who reads
/// it, who writes it, and what the compiler concludes.
///
/// This single structure is simultaneously the diagnostic (E0938 reads
/// `declared_in`), the documentation (`inspect --layer state` prints it), and
/// the input to the W4 optimisation (`verdict`). One registry, three views.
#[derive(Debug, Clone, Serialize)]
pub struct StateProvenance {
    /// Variable name (without `$`)
    pub var_name: String,
    pub type_name: String,
    pub initial: String,
    /// Every file that declares this name at page-global scope. More than one
    /// entry IS the E0938 condition — a page holds one cell per name, exactly as
    /// it holds one macro per name.
    pub declared_in: Vec<String>,
    /// Files (with counts) that READ the cell.
    pub readers: Vec<(String, usize)>,
    /// Files (with counts) that WRITE the cell.
    pub writers: Vec<(String, usize)>,
    pub verdict: StateVerdict,
}

/// Scope for a single template definition.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateScope {
    /// Template name (without `&` prefix)
    pub name: String,
    /// State variables declared in this template
    pub states: Vec<StateDecl>,
    /// Directives declared in this template (class toggles, @on actions, content
    /// bindings/injections) — string display, sourced from the World-A scope tree.
    pub directives: Vec<String>,
    /// Export declarations from @exports block
    pub exports: Vec<crate::syntax::ExportDecl>,
    /// Template ref invocations (child template instances)
    pub refs: Vec<crate::syntax::TemplateRef>,
    /// Parent template name (for nested templates), or None for top-level
    pub parent: Option<String>,
}

/// The full scope tree for a compilation unit.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeTree {
    /// State variables declared at body/global scope (SpacetimeLocal)
    pub global_states: Vec<StateDecl>,
    /// Template scopes keyed by template name
    pub templates: HashMap<String, TemplateScope>,
}

impl ScopeTree {
    pub fn new() -> Self {
        Self {
            global_states: Vec::new(),
            templates: HashMap::new(),
        }
    }
}

impl Default for ScopeTree {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Builder
// =============================================================================

/// Format a CapturedValue initial value to a display string for StateDecl.
fn format_state_initial(value: &CapturedValue) -> String {
    match value {
        CapturedValue::Bool(b) => b.to_string(),
        CapturedValue::Number(n) => n.to_string(),
        CapturedValue::String(s) => format!("\"{}\"", s),
        CapturedValue::Expr(e) => e.clone(),
        _ => "null".to_string(),
    }
}

/// FEAT-115 (Q4): the factory `states` for a body-bearing `Construct` scope, read
/// from the scope's surfaced `local-state` matches (World A) — not the retired
/// `ComponentBody` capture. Walks the scope's own matches and its nested scopes (a
/// state decl can sit at the body root or inside a `.sel {}` region). Mirrors the
/// global-state conversion above.
fn scope_state_decls(scope: &crate::parser::ScopeBlock) -> Vec<StateDecl> {
    fn convert(fm: &FormMatch, out: &mut Vec<StateDecl>) {
        if fm.macro_name != "local-state" && fm.macro_name != "local-state-uninitialized" {
            return;
        }
        let var_name = fm.get_ident("name").unwrap_or_default().to_string();
        let type_name = fm
            .get_ident("type")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "any".to_string());
        let initial = match fm.captures.get("value") {
            Some(CapturedValue::Ident(s)) => s.clone(),
            Some(other) => format_state_initial(other),
            None => "null".to_string(),
        };
        out.push(StateDecl {
            var_name,
            type_name,
            initial,
            source_file: fm.source_file.clone(),
        });
    }
    fn walk_nested(nested: &[crate::parser::ast::NestedScope], out: &mut Vec<StateDecl>) {
        for ns in nested {
            for fm in &ns.matches {
                convert(fm, out);
            }
            walk_nested(&ns.nested_scopes, out);
        }
    }
    let mut out = Vec::new();
    for fm in &scope.matches {
        convert(fm, &mut out);
    }
    walk_nested(&scope.nested_scopes, &mut out);
    out
}

/// Parse a state declaration string like "$open bool: false" into a StateDecl.
/// Retained for unit tests; production code reads the World-A scope directly.
#[cfg(test)]
fn parse_state_decl(s: &str) -> Option<StateDecl> {
    let s = s.trim().trim_end_matches(';').trim();

    // Pattern: $name type: initial
    if let Some(rest) = s.strip_prefix('$') {
        let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            return None;
        }
        let var_name = parts[0].to_string();
        let type_and_init = parts[1].trim();

        // Split on `: ` to separate type from initial value
        if let Some(colon_pos) = type_and_init.find(':') {
            let type_name = type_and_init[..colon_pos].trim().to_string();
            let initial = type_and_init[colon_pos + 1..].trim().to_string();
            Some(StateDecl {
                var_name,
                type_name,
                initial,
                source_file: None,
            })
        } else {
            // Uninitialized: $name type
            let type_name = type_and_init.trim().to_string();
            Some(StateDecl {
                var_name,
                type_name,
                initial: "null".to_string(),
                source_file: None,
            })
        }
    } else {
        None
    }
}

/// Build a scope tree from parsed FormMatches.
///
/// Walks all form matches looking for:
/// - `local-state` macros at file level (global scope)
/// - `template` / `template-inline` macros with component_body states
/// FEAT-115 S5c: format a template Construct scope's directives for the inspect
/// display (was reify's `body.directives`). Walks the scope's nested scopes:
/// reactive css_declarations (class toggle `.x--y: $z`, content binding/injection
/// `text: $z` / `text <- $z`, self-prop `attr: $z`) and `@on` matches.
fn template_scope_directive_display(scope: &crate::parser::ScopeBlock) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(
        matches: &[crate::syntax::FormMatch],
        nested: &[crate::parser::ast::NestedScope],
        out: &mut Vec<String>,
    ) {
        for m in matches {
            if m.macro_name == "on" {
                out.push(format!("@on {}", m.get_ident("event").unwrap_or("event")));
            }
        }
        for n in nested {
            for cd in &n.css_declarations {
                if cd.value.trim_start().starts_with('$') {
                    out.push(format!("{}: {}", cd.property, cd.value));
                }
            }
            walk(&n.matches, &n.nested_scopes, out);
        }
    }
    walk(&scope.matches, &scope.nested_scopes, &mut out);
    out
}

pub fn build_scope_tree(matches: &[FormMatch], scopes: &[crate::parser::ScopeBlock]) -> ScopeTree {
    let mut tree = ScopeTree::new();

    for fm in matches {
        match fm.macro_name.as_str() {
            // Global state declarations (body-level $var)
            "local-state" | "local-state-uninitialized" => {
                if fm.selector.is_none() {
                    // File-level = global scope
                    let var_name = fm.get_ident("name").unwrap_or_default().to_string();
                    let type_name = fm
                        .get_ident("type")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "any".to_string());
                    // Use the shared typed formatter so number/bool/expr initials are kept
                    // (not coerced to "null") — file-scope holes (FEAT-078) render these as their
                    // static initial value. Ident (a bare enum-ish token) stays verbatim.
                    let initial = match fm.captures.get("value") {
                        Some(CapturedValue::Ident(s)) => s.clone(),
                        Some(other) => format_state_initial(other),
                        None => "null".to_string(),
                    };
                    tree.global_states.push(StateDecl {
                        var_name,
                        type_name,
                        initial,
                        source_file: fm.source_file.clone(),
                    });
                }
            }

            // Template definitions with component_body
            "template" | "template-inline" => {
                let template_name = fm.get_ident("name").unwrap_or_default().to_string();

                // FEAT-115 (Q4): states / exports / refs / directives are ALL read
                // from this template's World-A `Construct` scope — not the retired
                // `ComponentBody` capture fields. The scope is the single source of
                // truth: states from its surfaced `local-state` matches, exports/refs
                // from the scope payload harvested in cst_to_stfile, directives from
                // the nested scope display.
                let construct = scopes.iter().find(|s| {
                    matches!(&s.kind, crate::parser::ast::ScopeKind::Construct(_))
                        && s.selector == format!("@template:{}", template_name)
                });

                let mut states = Vec::new();
                let mut exports = Vec::new();
                let mut refs = Vec::new();
                if let Some(s) = construct {
                    states = scope_state_decls(s);
                    exports = s.exports.clone();
                    refs = s.refs.clone();
                }

                let directives = construct
                    .map(template_scope_directive_display)
                    .unwrap_or_default();

                tree.templates.insert(
                    template_name.clone(),
                    TemplateScope {
                        name: template_name,
                        states,
                        directives,
                        exports,
                        refs,
                        parent: None, // Nested template resolution is deferred
                    },
                );
            }

            _ => {}
        }
    }

    tree
}

// =============================================================================
// Display
// =============================================================================

impl ScopeTree {
    /// Pretty-print the scope tree for --layer scopes output.
    pub fn display_pretty(&self) -> String {
        let mut out = String::new();

        out.push_str("=== Scope Tree ===\n\n");

        // Global scope
        out.push_str("body (global) → SpacetimeLocal\n");
        if self.global_states.is_empty() {
            out.push_str("  (no state declarations)\n");
        } else {
            for s in &self.global_states {
                out.push_str(&format!(
                    "  ${}: {} = {}\n",
                    s.var_name, s.type_name, s.initial
                ));
            }
        }
        out.push('\n');

        // Template scopes
        if self.templates.is_empty() {
            out.push_str("(no template scopes)\n");
        } else {
            let mut names: Vec<&String> = self.templates.keys().collect();
            names.sort();
            for name in names {
                let scope = &self.templates[name];
                let parent_info = scope
                    .parent
                    .as_ref()
                    .map(|p| format!(" (parent: {})", p))
                    .unwrap_or_default();

                out.push_str(&format!(
                    "@template &{}{} → ST.set(rootEl, ...)\n",
                    name, parent_info
                ));

                if scope.states.is_empty() {
                    out.push_str("  (no state declarations)\n");
                } else {
                    for s in &scope.states {
                        out.push_str(&format!(
                            "  ${}: {} = {}\n",
                            s.var_name, s.type_name, s.initial
                        ));
                    }
                }

                if !scope.directives.is_empty() {
                    out.push_str(&format!("  directives: {}\n", scope.directives.len()));
                }
                out.push('\n');
            }
        }

        out
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use std::collections::HashMap;

    #[test]
    fn test_parse_state_decl_with_initial() {
        let decl = parse_state_decl("$count number: 0").unwrap();
        assert_eq!(decl.var_name, "count");
        assert_eq!(decl.type_name, "number");
        assert_eq!(decl.initial, "0");
    }

    #[test]
    fn test_parse_state_decl_bool() {
        let decl = parse_state_decl("$open bool: false").unwrap();
        assert_eq!(decl.var_name, "open");
        assert_eq!(decl.type_name, "bool");
        assert_eq!(decl.initial, "false");
    }

    #[test]
    fn test_parse_state_decl_string() {
        let decl = parse_state_decl("$name string: \"hello\"").unwrap();
        assert_eq!(decl.var_name, "name");
        assert_eq!(decl.type_name, "string");
        assert_eq!(decl.initial, "\"hello\"");
    }

    #[test]
    fn test_parse_state_decl_uninitialized() {
        let decl = parse_state_decl("$error string").unwrap();
        assert_eq!(decl.var_name, "error");
        assert_eq!(decl.type_name, "string");
        assert_eq!(decl.initial, "null");
    }

    #[test]
    fn test_parse_state_decl_array_type() {
        let decl = parse_state_decl("$items Product[]: []").unwrap();
        assert_eq!(decl.var_name, "items");
        assert_eq!(decl.type_name, "Product[]");
        assert_eq!(decl.initial, "[]");
    }

    #[test]
    fn test_parse_state_decl_trailing_semicolon() {
        let decl = parse_state_decl("$visible bool: true;").unwrap();
        assert_eq!(decl.var_name, "visible");
        assert_eq!(decl.type_name, "bool");
        assert_eq!(decl.initial, "true");
    }

    #[test]
    fn test_build_scope_tree_empty() {
        let tree = build_scope_tree(&[], &[]);
        assert!(tree.global_states.is_empty());
        assert!(tree.templates.is_empty());
    }

    #[test]
    fn test_build_scope_tree_global_state() {
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("count".to_string()),
        );
        captures.insert(
            "type".to_string(),
            CapturedValue::Ident("number".to_string()),
        );
        captures.insert("value".to_string(), CapturedValue::String("0".to_string()));

        let fm = FormMatch {
            macro_name: "local-state".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let tree = build_scope_tree(&[fm], &[]);
        assert_eq!(tree.global_states.len(), 1);
        assert_eq!(tree.global_states[0].var_name, "count");
        assert!(tree.templates.is_empty());
    }

    #[test]
    fn test_build_scope_tree_template_with_states() {
        // FEAT-115 S5c: states come from the World-A-overwritten reify struct;
        // directives are sourced from the World-A scope tree. Parse real source so
        // both the matches and scopes are present.
        let f = crate::parse(
            "@template &counter() { <button class=\"c\"></button>\n$count number: 0;\n$open bool: false;\n.c { @on &.click { $count <- $count + 1; } } }\n<main></main>",
        )
        .expect("parse");
        let tree = build_scope_tree(&f.matches, &f.scopes);
        assert!(tree.global_states.is_empty());
        assert_eq!(tree.templates.len(), 1);

        let scope = &tree.templates["counter"];
        assert_eq!(scope.name, "counter");
        assert_eq!(scope.states.len(), 2);
        let names: Vec<&str> = scope.states.iter().map(|s| s.var_name.as_str()).collect();
        assert!(
            names.contains(&"count") && names.contains(&"open"),
            "{:?}",
            names
        );
        assert_eq!(
            scope.directives.len(),
            1,
            "one @on directive: {:?}",
            scope.directives
        );
    }

    #[test]
    fn test_scope_tree_pretty_display() {
        let mut tree = ScopeTree::new();
        tree.global_states.push(StateDecl {
            var_name: "menuOpen".to_string(),
            type_name: "bool".to_string(),
            initial: "false".to_string(),
            source_file: None,
        });
        tree.templates.insert(
            "counter".to_string(),
            TemplateScope {
                name: "counter".to_string(),
                states: vec![StateDecl {
                    var_name: "count".to_string(),
                    type_name: "number".to_string(),
                    initial: "0".to_string(),
                    source_file: None,
                }],
                directives: vec![],
                exports: vec![],
                refs: vec![],
                parent: None,
            },
        );

        let output = tree.display_pretty();
        assert!(output.contains("body (global)"));
        assert!(output.contains("$menuOpen: bool = false"));
        assert!(output.contains("@template &counter"));
        assert!(output.contains("$count: number = 0"));
    }
}
