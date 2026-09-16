//! Compile-time analysis report for Spacetime.
//!
//! Collects and reports on:
//! - Type definitions with field counts
//! - Data sources with item counts
//! - Computed data with dependencies
//! - Functions with signatures
//! - Template references and slots
//! - Binding counts per selector
//! - State machines with transition validation
//! - Signal definitions and usage tracking
//! - Orphan detection (unused code)
//! - Cross-reference validation
//! - Element dependency tracking

pub mod diagnostics;
pub mod element_deps;
pub mod liveview_contract;
pub mod signals;
pub mod timeline_lint;
pub mod visual_lint;

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::parser::{CssDeclaration, NestedScope, ScopeBlock, StateMachineAst};
use crate::syntax::{
    CapturedValue, FormMatch, collect_signal_deps, collect_st_var_deps, handler_body_signal_deps,
    on_motion_body_signal_deps,
};
use crate::type_system::TypeRegistry;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub use element_deps::{ElementDepGraph, ElementDependency};
pub use signals::{SignalAnalysis, SignalAnalyzer, analyze_signals_with_diagnostics};
pub use timeline_lint::{TimelineLintConfig, analyze_timelines};

#[cfg(test)]
mod tests;

/// Reduce a raw `from:` source capture to just its leading identifier,
/// stripping any `$` sigil and discarding trailing clause text the
/// form-matcher may have folded in (e.g. `items; reduce: ...` -> `items`).
fn leading_source_ident(raw: &str) -> String {
    let s = raw.trim().strip_prefix('$').unwrap_or(raw.trim());
    s.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// Base source of a possibly-dotted, possibly-`$`-sigiled read: `$chat.status`
/// → `chat`, `$new-session` → `new-session`. Distinct from
/// [`leading_source_ident`], whose identifier charset stops at `-` — legal in
/// signal names — and would truncate `new-session` to `new`.
fn read_base(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('$')
        .split('.')
        .next()
        .unwrap_or("")
        .to_string()
}

// =============================================================================
// Analysis Data Structures
// =============================================================================

/// Complete compile-time analysis of a Spacetime file.
#[derive(Debug, Clone)]
pub struct CompileAnalysis {
    pub types: Vec<TypeAnalysis>,
    pub data_sources: Vec<DataAnalysis>,
    pub computed: Vec<ComputedAnalysis>,
    pub functions: Vec<FunctionAnalysis>,
    pub templates: Vec<TemplateAnalysis>,
    pub bindings: Vec<BindingAnalysis>,
    pub state_machines: Vec<StateMachineAnalysis>,
    pub locals: Vec<LocalAnalysis>,
    pub signals: Vec<SignalAnalysis>,
    pub diagnostics: Vec<Diagnostic>,
    /// `@data` source names that are READ (consumed) outside a subscriber-like
    /// position (`@each`/`@cycle`/computed) — collected from `text:`/attribute/
    /// class-toggle bindings and `@handle` arms. `detect_orphans` unions this into
    /// its used-set so a source read only via `text: $error` / `@handle $error` is
    /// no longer falsely flagged W0201 (BUG-207).
    pub read_usages: HashSet<String>,
}

/// Analysis of a scoped value definition ($name type: value).
#[derive(Debug, Clone)]
pub struct LocalAnalysis {
    pub name: String,
    pub type_name: String,
    pub is_array: bool,
}

/// Analysis of a type definition.
#[derive(Debug, Clone)]
pub struct TypeAnalysis {
    pub name: String,
    pub total_fields: usize,
    pub optional_fields: usize,
    pub used: bool,
}

/// Analysis of a data source.
#[derive(Debug, Clone)]
pub struct DataAnalysis {
    pub name: String,
    pub type_name: String,
    pub is_array: bool,
    pub source: DataSource,
    pub item_count: Option<usize>,
    pub validated: bool,
    pub used: bool,
}

#[derive(Debug, Clone)]
pub enum DataSource {
    File(String),
    /// Inline literal data: `src: inline; value: [...]`. Carries the raw
    /// `value:` expression text (a JSON-ish array/object literal) so no file
    /// resolution is attempted.
    Inline(String),
    LocalStorage,
    Runtime,
}

/// Analysis of computed data.
#[derive(Debug, Clone)]
pub struct ComputedAnalysis {
    pub name: String,
    pub type_name: String,
    pub dependencies: Vec<String>,
    pub has_where: bool,
    pub has_sort: bool,
    pub has_limit: bool,
    pub has_reduce: bool,
}

/// Analysis of a function definition.
#[derive(Debug, Clone)]
pub struct FunctionAnalysis {
    pub name: String,
    pub signature: String,
    pub used: bool,
}

/// Analysis of a template.
#[derive(Debug, Clone)]
pub struct TemplateAnalysis {
    pub name: String,
    pub slots: Vec<String>,
    pub used: bool,
}

/// Analysis of bindings in a selector.
#[derive(Debug, Clone)]
pub struct BindingAnalysis {
    pub selector: String,
    pub data_source: String,
    pub instance_count: Option<usize>,
}

/// Analysis of a state machine.
#[derive(Debug, Clone)]
pub struct StateMachineAnalysis {
    pub selector: String,
    pub initial: String,
    pub states: Vec<StateInfo>,
    pub transitions: Vec<TransitionInfo>,
}

#[derive(Debug, Clone)]
pub struct StateInfo {
    pub name: String,
    pub has_incoming: bool,
    pub has_outgoing: bool,
}

#[derive(Debug, Clone)]
pub struct TransitionInfo {
    pub from: String,
    pub to: String,
    pub on: String,
}

// =============================================================================
// Analysis Collector
// =============================================================================

impl CompileAnalysis {
    /// Create a new empty analysis.
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            data_sources: Vec::new(),
            computed: Vec::new(),
            functions: Vec::new(),
            templates: Vec::new(),
            bindings: Vec::new(),
            state_machines: Vec::new(),
            locals: Vec::new(),
            signals: Vec::new(),
            diagnostics: Vec::new(),
            read_usages: HashSet::new(),
        }
    }

    /// Analyze $ value declarations from FormMatch.
    /// Ambiguous-dispatch lint (FEAT-088): warn when a directive call's top two
    /// candidate macros tie on score, so resolution silently picks one by
    /// iteration order. Warning, not error — first-match still resolves; the fix
    /// is a disambiguating literal/type in one of the competing forms.
    pub fn analyze_dispatch(
        &mut self,
        matches: &[FormMatch],
        registry: &crate::metasystem::MetaRegistry,
    ) {
        for fm in matches {
            // GH-27: if parse already disambiguated via literal kind-words
            // (`matched_macro`, e.g. `data-inline` for `@data inline $x : []`),
            // the capture scorer is BLIND to the literal that settled it and
            // re-scores a false tie. Honor parse's decision; only re-score when
            // the matcher genuinely did not decide (matched_macro is None).
            if fm.matched_macro.is_some() {
                continue;
            }
            if let Some((winner, rival, score)) =
                registry.dispatch_ambiguity(&fm.macro_name, &fm.captures, &fm.selector)
            {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::E0923,
                        format!(
                            "ambiguous dispatch: @{} resolves to `{}` and `{}` (both score {}); \
                             add a disambiguating literal or typed capture to one form",
                            fm.macro_name.trim_start_matches('@'),
                            winner,
                            rival,
                            score
                        ),
                    )
                    .with_span(fm.span.into()),
                );
            }
        }
    }

    pub fn analyze_locals(&mut self, matches: &[FormMatch]) {
        for fm in matches.iter().filter(|m| m.macro_name == "local-state") {
            let name = fm.get_ident("name").unwrap_or("").to_string();

            let type_name = fm.type_name("type").unwrap_or("any").to_string();

            let is_array = type_name.ends_with("[]");

            self.locals.push(LocalAnalysis {
                name,
                type_name,
                is_array,
            });
        }
    }

    /// Analyze type definitions from FormMatch.
    pub fn analyze_types(&mut self, matches: &[FormMatch], _registry: &TypeRegistry) {
        for fm in matches.iter().filter(|m| m.macro_name == "type") {
            let name = fm.get_ident("name").unwrap_or("").to_string();

            // Get fields from captures - can be Block with field definitions
            // or Named with field name -> type mappings
            let (total_fields, optional_fields) = match fm.get("fields") {
                Some(CapturedValue::Block(fields)) => {
                    let total = fields.len();
                    // Count optional fields by checking for "optional" capture or "?" suffix
                    let optional = fields
                        .iter()
                        .filter(|f| f.get_bool("optional").unwrap_or(false))
                        .count();
                    (total, optional)
                }
                Some(CapturedValue::Named(fields)) => {
                    (fields.len(), 0) // Named doesn't track optionality directly
                }
                Some(CapturedValue::Array(fields)) => {
                    let total = fields.len();
                    let optional = fields
                        .iter()
                        .filter(|f| {
                            if let CapturedValue::Named(map) = f {
                                map.get("optional")
                                    .and_then(|v| match v {
                                        CapturedValue::Bool(b) => Some(*b),
                                        _ => None,
                                    })
                                    .unwrap_or(false)
                            } else {
                                false
                            }
                        })
                        .count();
                    (total, optional)
                }
                _ => (0, 0),
            };

            self.types.push(TypeAnalysis {
                name,
                total_fields,
                optional_fields,
                used: false, // Will be updated in orphan detection
            });
        }
    }

    /// Analyze data sources from FormMatch.
    pub fn analyze_data_sources(&mut self, matches: &[FormMatch], _registry: &TypeRegistry) {
        for fm in matches.iter().filter(|m| m.macro_name == "data") {
            // The binding name may be captured as an Ident (legacy `@data …:type{}`) OR a
            // Binding (`$name:binding`, the unified `@data <kind>` surface, PLAN-023 W3).
            // Strip the leading `$` so the source name matches `@each($name …)` refs.
            let name = fm
                .get_ident("name")
                .or_else(|| {
                    fm.get_binding("name")
                        .map(|s| s.strip_prefix('$').unwrap_or(s))
                })
                .or_else(|| fm.get_ident("as"))
                .unwrap_or("")
                .to_string();

            let type_name = fm.type_name("type").unwrap_or("any").to_string();

            let is_array = type_name.ends_with("[]");

            // Determine the data source from captures. `src: inline` (with a
            // `value:` literal) is an inline data source — no file resolution.
            // The unified `@data <kind>` surface captures `src` via `$src:expr`,
            // so a fetch URL arrives as a quoted `Expr`, not a bare `String`;
            // `as_string_literal` unwraps both shapes.
            //
            // Drift guard: a `src` capture that is PRESENT but does not yield a
            // string literal (and is not localStorage) is the fingerprint of a
            // capture-shape mismatch between a `@data <kind>` macro and this
            // reader (the exact failure mode of BUG-admin-data-source-expr-src).
            // Such a form silently degraded to `Runtime`, emptying the
            // dataSourceMap. Flag it (W0205) instead of failing silently.
            let has_src = fm.has("src");
            let source = match fm.get("src").and_then(|v| v.as_string_literal()) {
                Some("inline") => {
                    let value = fm
                        .get("value")
                        .map(|v| v.to_js(crate::syntax::JsQuoting::DoubleQuoted))
                        .unwrap_or_else(|| "[]".to_string());
                    DataSource::Inline(value)
                }
                Some(src) => DataSource::File(src.to_string()),
                None => {
                    if fm.has("localStorage") {
                        DataSource::LocalStorage
                    } else {
                        if has_src {
                            let raw = fm
                                .get("src")
                                .map(|v| v.to_js(crate::syntax::JsQuoting::Raw))
                                .unwrap_or_default();
                            self.diagnostics.push(
                                Diagnostic::warning(
                                    DiagnosticCode::W0205,
                                    format!(
                                        "@data source '{}' declares a source but it did not resolve to a file, inline, or localStorage value",
                                        if name.is_empty() { "<anonymous>" } else { &name }
                                    ),
                                )
                                .with_hint(format!(
                                    "src captured as `{}` — expected a quoted URL string literal. This usually means a `@data <kind>` macro's capture shape drifted from the analysis reader.",
                                    raw.trim()
                                )),
                            );
                        }
                        DataSource::Runtime
                    }
                }
            };

            self.data_sources.push(DataAnalysis {
                name,
                type_name,
                is_array,
                source,
                item_count: None, // Will be populated if JSON is loaded
                validated: false, // Will be updated if schema validation runs
                used: false,      // Will be updated in orphan detection
            });
        }
    }

    /// Analyze computed data from FormMatch.
    pub fn analyze_computed(&mut self, matches: &[FormMatch]) {
        for fm in matches.iter().filter(|m| m.macro_name == "computed") {
            let name = fm.get_ident("name").unwrap_or("").to_string();

            let type_name = fm.type_name("type").unwrap_or("any").to_string();

            // Extract dependencies from captures (from/source)
            let dependencies: Vec<String> = fm
                .captures
                .get("from")
                .or_else(|| fm.captures.get("source"))
                .and_then(|v| match v {
                    CapturedValue::Ident(id) => Some(vec![id.clone()]),
                    CapturedValue::Binding(b) => {
                        // Strip $ prefix if present
                        let dep = b.strip_prefix('$').unwrap_or(b);
                        Some(vec![dep.to_string()])
                    }
                    _ => None,
                })
                .unwrap_or_default();
            // Keep only the leading identifier of each `from:` source so that
            // trailing clauses (where/sort/reduce/...) the form-matcher may
            // fold into the capture never leak into the dependency name.
            let dependencies: Vec<String> = dependencies
                .iter()
                .map(|raw| leading_source_ident(raw))
                .filter(|d| !d.is_empty())
                .collect();

            // Detect which operations are used
            let has_where = fm.has("where") || fm.has("filter");
            let has_sort = fm.has("sort") || fm.has("sortBy");
            let has_limit = fm.has("limit");
            let has_reduce = fm.has("reduce");

            self.computed.push(ComputedAnalysis {
                name,
                type_name,
                dependencies,
                has_where,
                has_sort,
                has_limit,
                has_reduce,
            });
        }
    }

    /// Analyze function definitions from FormMatch.
    pub fn analyze_functions(&mut self, matches: &[FormMatch]) {
        for fm in matches.iter().filter(|m| m.macro_name == "fn") {
            let name = fm.get_ident("name").unwrap_or("").to_string();

            // Get params from captures - can be Array of Named or Block
            let params_str = match fm.get("params") {
                Some(CapturedValue::Array(params)) => params
                    .iter()
                    .filter_map(|p| {
                        if let CapturedValue::Named(map) = p {
                            let pname = map
                                .get("name")
                                .and_then(|v| match v {
                                    CapturedValue::Ident(s) => Some(s.as_str()),
                                    _ => None,
                                })
                                .unwrap_or("_");
                            let ptype = map
                                .get("type")
                                .and_then(|v| v.as_type_name())
                                .unwrap_or("any");
                            Some(format!("{}: {}", pname, ptype))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
                Some(CapturedValue::Block(params)) => params
                    .iter()
                    .filter_map(|p| {
                        let pname = p.get_ident("name").unwrap_or("_");
                        let ptype = p.type_name("type").unwrap_or("any");
                        Some(format!("{}: {}", pname, ptype))
                    })
                    .collect::<Vec<_>>(),
                _ => vec![],
            };

            // Get return type
            let return_type = fm
                .type_name("returnType")
                .or_else(|| fm.type_name("returns"))
                .unwrap_or("void")
                .to_string();

            let signature = format!("{}({}): {}", name, params_str.join(", "), return_type);

            self.functions.push(FunctionAnalysis {
                name,
                signature,
                used: false,
            });
        }
    }

    /// Analyze signals in a parsed AST file.
    /// This method analyzes signal definitions and usages, adding any diagnostics
    /// for unused signals or undefined references.
    /// Analyze signals in a parsed AST file. MERGES (never clobbers) its
    /// findings into `self.signals` so it can run AFTER
    /// `register_macro_yield_signals` without wiping the %binds yields, and
    /// suppresses E0408 for every name the enclosing analysis already knows
    /// (data/computed/locals/yields) — a signal published by an imported
    /// module, a template, or a parent scope is not "undefined" (GH-26).
    pub fn analyze_signals_from_ast(&mut self, ast: &crate::parser::StFile) {
        let known = self.reactive_source_names();
        let (signals, diagnostics) = analyze_signals_with_diagnostics(ast, &known);
        for s in signals {
            if !self.signals.iter().any(|e| e.name == s.name) {
                self.signals.push(s);
            }
        }
        for d in diagnostics {
            self.diagnostics.push(d.into_diagnostic());
        }
    }

    /// Register the signals yielded by a macro's `%binds` clause as reactive
    /// sources, read from the meta-registry (NOT a hardcoded list). BUG-126: the
    /// signal analyzer only knew a frozen set of primitive yields, so any macro
    /// whose outputs come solely from `%binds { prim(..) -> { $a, $b } }`
    /// (@realtime, @presence, @data collection/signal/stream, …) had its yields
    /// rejected with E0405. The `%binds … -> { $out }` clause already declares
    /// the yields as data; this consults that, resolving `${$cap}` alias patterns
    /// against each match's captures so `$x_loading`-style yields resolve too.
    pub fn register_macro_yield_signals(
        &mut self,
        matches: &[FormMatch],
        registry: &crate::metasystem::MetaRegistry,
    ) {
        for fm in matches {
            self.register_yields_for_form_match(fm, registry);
        }
    }

    fn register_yields_for_form_match(
        &mut self,
        fm: &FormMatch,
        registry: &crate::metasystem::MetaRegistry,
    ) {
        // GH-26 (same class as #27): honor the EXACT macro parse selected
        // (matched_macro, e.g. "data-signal") over the shared `%creates`
        // directive name ("data"). `get_macro_by_form_directive` returns only
        // the FIRST form sharing the directive, so `@data signal`'s yields
        // (`${$name}_pending` / `_error`) were never registered — an undefined-
        // signal false positive on legitimate auto-yields.
        let macro_def = fm
            .matched_macro
            .as_deref()
            .and_then(|exact| registry.get_macro(exact))
            .or_else(|| registry.get_macro_by_form_directive(&fm.macro_name));
        if let Some(macro_def) = macro_def {
            for bind_decl in &macro_def.binds {
                for output in &bind_decl.outputs {
                    let raw = output.alias.as_deref().unwrap_or(&output.name);
                    if let Some(name) = resolve_yield_name(raw, &fm.captures) {
                        let clean = name.trim_start_matches('$').to_string();
                        if clean.is_empty() {
                            continue;
                        }
                        if !self.signals.iter().any(|s| s.name == clean) {
                            self.signals.push(crate::analysis::SignalAnalysis::new(
                                clean,
                                fm.span,
                                format!("@{}", fm.macro_name),
                            ));
                        }
                    }
                }
            }
        }
        // Recurse into Block captures so nested directives' yields register too.
        for value in fm.captures.values() {
            if let crate::syntax::CapturedValue::Block(children) = value {
                for child in children {
                    self.register_yields_for_form_match(child, registry);
                }
            }
        }
    }

    /// Analyze scopes for bindings, templates, and state machines.
    pub fn analyze_scopes(&mut self, scopes: &[ScopeBlock]) {
        for scope in scopes {
            // Process @each blocks from FormMatches
            for fm in &scope.matches {
                if fm.macro_name == "each" {
                    self.analyze_each_form_match(&scope.selector, fm);
                }

                // Track @cycle as a data consumer
                if fm.macro_name == "cycle"
                    && let Some(source) = fm.get_ident("source")
                {
                    self.bindings.push(BindingAnalysis {
                        selector: scope.selector.clone(),
                        data_source: source.to_string(),
                        instance_count: None,
                    });
                }
            }

            // Analyze state machines
            if let Some(state_machine) = &scope.behavior.state_machine {
                self.analyze_state_machine(&scope.selector, state_machine, &scope.behavior);
            }
        }
    }

    /// Analyze scroll timelines for timing issues (W101, W110-W114).
    pub fn analyze_timelines(&mut self, scopes: &[ScopeBlock]) {
        let config = TimelineLintConfig::default();
        let diags = analyze_timelines(scopes, &config);
        self.diagnostics.extend(diags);
    }

    /// Analyze scroll timelines with custom configuration.
    /// Analyze visual consistency issues (W201-W205).
    pub fn analyze_visual(
        &mut self,
        scopes: &[ScopeBlock],
        matches: &[FormMatch],
        html_content: Option<&str>,
    ) {
        let diags = visual_lint::analyze_visual(scopes, matches, html_content);
        self.diagnostics.extend(diags);
    }

    /// Analyze an @each block from FormMatch for templates and bindings.
    fn analyze_each_form_match(&mut self, selector: &str, fm: &FormMatch) {
        // Extract the data source from captures
        // The source can be either a Binding ($name) or an Ident (name)
        if let Some(source) = fm.get_binding("source").or_else(|| fm.get_ident("source")) {
            // Strip the $ prefix if present
            let source_name = source.strip_prefix('$').unwrap_or(source);

            // Only record bindings for top-level data sources (not nested $.field references)
            if !source_name.is_empty() && !source_name.starts_with('.') {
                self.bindings.push(BindingAnalysis {
                    selector: selector.to_string(),
                    data_source: source_name.to_string(),
                    instance_count: None, // Will be populated if we count JSON items
                });
            }
        }

        // Extract template name if present
        if let Some(template_name) = fm.get_ident("template") {
            // Collect slots from the template invocations/bindings
            let slots = Self::collect_slots_from_form_match(fm);

            // Check if we already have this template
            if let Some(existing) = self.templates.iter_mut().find(|t| t.name == template_name) {
                existing.used = true;
                // Merge slots
                for slot in slots {
                    if !existing.slots.contains(&slot) {
                        existing.slots.push(slot);
                    }
                }
            } else {
                self.templates.push(TemplateAnalysis {
                    name: template_name.to_string(),
                    slots,
                    used: true,
                });
            }
        }

        // Handle nested @each blocks in a Block capture
        if let Some(CapturedValue::Block(nested_matches)) = fm.get("nested") {
            for nested_fm in nested_matches {
                if nested_fm.macro_name == "each" {
                    self.analyze_each_form_match(selector, nested_fm);
                }
            }
        }
    }

    /// Collect slot names from FormMatch captures.
    fn collect_slots_from_form_match(fm: &FormMatch) -> Vec<String> {
        let mut slots = Vec::new();

        // Check for invocations array which contains slot bindings
        if let Some(CapturedValue::Array(invocations)) = fm.get("invocations") {
            for invocation in invocations {
                if let CapturedValue::Named(map) = invocation {
                    // Extract slot name from the invocation
                    if let Some(CapturedValue::Ident(slot_name)) = map.get("slot")
                        && !slots.contains(slot_name)
                    {
                        slots.push(slot_name.clone());
                    }
                }
            }
        }

        // Also check for direct slot captures
        if let Some(CapturedValue::Block(bindings)) = fm.get("bindings") {
            for binding in bindings {
                if let Some(slot_name) = binding.get_ident("slot")
                    && !slots.contains(&slot_name.to_string())
                {
                    slots.push(slot_name.to_string());
                }
            }
        }

        slots
    }

    /// Analyze a state machine.
    fn analyze_state_machine(
        &mut self,
        selector: &str,
        state_machine: &StateMachineAst,
        behavior: &crate::parser::BehaviorBlock,
    ) {
        // Collect all state names
        let mut state_info: HashMap<String, StateInfo> = HashMap::new();

        // Initialize with all defined states
        for state in &behavior.states {
            state_info.insert(
                state.when.clone(),
                StateInfo {
                    name: state.when.clone(),
                    has_incoming: state.when == state_machine.initial, // Initial state has "incoming"
                    has_outgoing: false,
                },
            );
        }

        // Ensure initial state exists
        if !state_info.contains_key(&state_machine.initial) {
            state_info.insert(
                state_machine.initial.clone(),
                StateInfo {
                    name: state_machine.initial.clone(),
                    has_incoming: true,
                    has_outgoing: false,
                },
            );
        }

        // Collect transitions
        let mut transitions = Vec::new();
        for trans in &behavior.transitions {
            transitions.push(TransitionInfo {
                from: trans.from.clone(),
                to: trans.to.clone(),
                on: trans.on.clone(),
            });

            // Mark outgoing for 'from' state
            if let Some(info) = state_info.get_mut(&trans.from) {
                info.has_outgoing = true;
            }

            // Mark incoming for 'to' state
            if let Some(info) = state_info.get_mut(&trans.to) {
                info.has_incoming = true;
            } else {
                // State referenced but not defined
                state_info.insert(
                    trans.to.clone(),
                    StateInfo {
                        name: trans.to.clone(),
                        has_incoming: true,
                        has_outgoing: false,
                    },
                );
            }
        }

        // Also check async transitions
        for async_trans in &behavior.async_transitions {
            transitions.push(TransitionInfo {
                from: async_trans.from.clone(),
                to: async_trans.on_success.clone(),
                on: format!("async:{}", async_trans.trigger),
            });

            if let Some(info) = state_info.get_mut(&async_trans.from) {
                info.has_outgoing = true;
            }

            // Mark success state
            if let Some(info) = state_info.get_mut(&async_trans.on_success) {
                info.has_incoming = true;
            } else {
                state_info.insert(
                    async_trans.on_success.clone(),
                    StateInfo {
                        name: async_trans.on_success.clone(),
                        has_incoming: true,
                        has_outgoing: false,
                    },
                );
            }

            // Mark error state
            if let Some(info) = state_info.get_mut(&async_trans.on_error) {
                info.has_incoming = true;
            } else {
                state_info.insert(
                    async_trans.on_error.clone(),
                    StateInfo {
                        name: async_trans.on_error.clone(),
                        has_incoming: true,
                        has_outgoing: false,
                    },
                );
            }
        }

        self.state_machines.push(StateMachineAnalysis {
            selector: selector.to_string(),
            initial: state_machine.initial.clone(),
            states: state_info.into_values().collect(),
            transitions,
        });
    }

    /// `$name` reads inside a template's HTML body — the bundle interpolates them
    /// at render, so each is a READ of that signal. `$$` is the literal-dollar
    /// escape (BUG-112), never a read. Field access reads its base (`$m.text` →
    /// `m`), matching `read_base`.
    fn template_html_signal_deps(html: &str) -> Vec<String> {
        let bytes = html.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'$' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'$' {
                    i += 2;
                    continue;
                }
                let start = i + 1;
                if start < bytes.len()
                    && (bytes[start].is_ascii_alphabetic() || bytes[start] == b'_')
                {
                    let mut j = start;
                    while j < bytes.len()
                        && (bytes[j].is_ascii_alphanumeric()
                            || bytes[j] == b'_'
                            || bytes[j] == b'-'
                            || bytes[j] == b'.')
                    {
                        j += 1;
                    }
                    out.push(read_base(&html[start..j]));
                    i = j;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        out
    }

    /// Collect READ positions that mark a `@data` source as used but are NOT
    /// subscriber-like (`@each`/`@cycle`/computed) — the positions `detect_orphans`
    /// historically missed (BUG-207):
    ///   - `text:` / `text <-` / attribute / class-toggle bindings — CSS
    ///     declarations whose value reads a `$var` (`text: $error`).
    ///   - `@handle $signal { … }` — the consumed `signal` names the source.
    /// Without these, a source read only via `text: $error` / `@handle $error` was
    /// falsely flagged W0201 while an identical source that ALSO feeds an `@each`
    /// stayed silent — a linter crying wolf on the diagnostic channel that shares
    /// its pass with the E0928 contract check.
    pub fn analyze_read_usages(&mut self, scopes: &[ScopeBlock], matches: &[FormMatch]) {
        fn walk_decls(css: &[CssDeclaration], out: &mut HashSet<String>) {
            for d in css {
                for dep in collect_signal_deps(&d.value) {
                    out.insert(dep);
                }
                // GH-26/W0.3: a CSS read of the signal's published property
                // (`var(--st-x)`) is a use of `x`. Without this a source consumed
                // only from CSS was falsely flagged W0201 "defined but never used".
                for dep in collect_st_var_deps(&d.value) {
                    out.insert(dep);
                }
            }
        }
        fn walk_matches(ms: &[FormMatch], out: &mut HashSet<String>) {
            for m in ms {
                // A handler body (`@on &.click { $x <- 2 }`) both READS and WRITES
                // its signals; a write keeps the signal alive, so the body's refs
                // count as uses. Without this a source written by handlers was
                // falsely flagged W0201.
                for dep in handler_body_signal_deps(&m.captures) {
                    out.insert(dep);
                }
                // BUG-380: an @on body is an `on_motion_body` — STRUCTURED
                // stmt/mut/call/line entries, never the `js_statements` map the
                // reader above looks for (that shape belongs to
                // `mutation_actions` bodies, and to the emit pipeline's later
                // rewrite). Without this, a fired signal (`$send($draft)`), its
                // arguments, and mutation targets were all invisible here.
                for dep in on_motion_body_signal_deps(&m.captures) {
                    out.insert(dep);
                }
                // BUG-381: `@view`/`@match` lower to `dispatch-mount(subject:…)`,
                // which READS the subject — reactively for @view, once at mount
                // for @match. The subject binding names the consumed source.
                if m.macro_name == "view" || m.macro_name == "match" {
                    if let Some(subj) = m.get_binding("subject") {
                        out.insert(read_base(subj));
                    }
                }
            }
        }
        fn walk_nested(nested: &[NestedScope], out: &mut HashSet<String>) {
            for n in nested {
                walk_decls(&n.css_declarations, out);
                walk_matches(&n.matches, out);
                walk_nested(&n.nested_scopes, out);
            }
        }
        for scope in scopes {
            walk_decls(&scope.css_declarations, &mut self.read_usages);
            walk_matches(&scope.matches, &mut self.read_usages);
            walk_nested(&scope.nested_scopes, &mut self.read_usages);
        }
        // File-scope forms live ONLY in the flat list (an @on with no selector
        // never enters a ScopeBlock); EDN ingress dual-registers scoped forms
        // into both, and the HashSet dedups.
        walk_matches(matches, &mut self.read_usages);
        for fm in matches.iter().filter(|m| m.macro_name == "handle") {
            if let Some(sig) = fm.get_binding("signal") {
                self.read_usages
                    .insert(sig.trim_start_matches('$').to_string());
            }
        }
        // A `$name` in a template's HTML body is a runtime READ — the bundle
        // interpolates it where it renders. Without this, a source consumed
        // ONLY by template text (a view whose whole answer to "show @partial"
        // is `{ $partial }` in a template) was falsely flagged W0201 "defined
        // but never used" — the same false-positive family as BUG-207/380/381,
        // one render position over.
        for scope in scopes {
            if scope.selector.starts_with("@template:") {
                for dep in Self::template_html_signal_deps(&scope.html) {
                    self.read_usages.insert(dep);
                }
            }
        }
    }

    /// Detect orphans (unused definitions) and add warnings.
    pub fn detect_orphans(&mut self) {
        // Track usage from bindings (subscriber-like: @each / @cycle)
        let mut used_data_sources = HashSet::new();
        for binding in &self.bindings {
            used_data_sources.insert(binding.data_source.clone());
        }

        // Track usage from computed dependencies
        for comp in &self.computed {
            for dep in &comp.dependencies {
                used_data_sources.insert(dep.clone());
            }
        }

        // Track usage from READ positions — text:/attribute bindings + @handle arms
        // (BUG-207). A source consumed outside a subscriber-like position is still
        // used; failing to count it here was the W0201 false positive.
        for dep in &self.read_usages {
            used_data_sources.insert(dep.clone());
        }

        // Mark data sources as used/unused and emit warnings
        for data in &mut self.data_sources {
            data.used = used_data_sources.contains(&data.name);
            if !data.used {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W0201,
                        format!("Data source '{}' is defined but never used", data.name),
                    )
                    .with_hint("Remove unused data source or add @each binding".to_string()),
                );
            }
        }

        // Check for unused templates (templates that exist but aren't referenced)
        for template in &mut self.templates {
            if !template.used {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W0202,
                        format!(
                            "Template '{}' is defined but never instantiated",
                            template.name
                        ),
                    )
                    .with_hint(
                        "This template is never referenced in any @each or template: directive"
                            .to_string(),
                    ),
                );
            }
        }

        // Check for unused types (types that are never referenced)
        let mut used_types = HashSet::new();
        for data in &self.data_sources {
            // Strip array suffix: "HeroTerm[]" -> "HeroTerm"
            let base_type = data.type_name.trim_end_matches("[]");
            used_types.insert(base_type.to_string());
        }
        for comp in &self.computed {
            let base_type = comp.type_name.trim_end_matches("[]");
            used_types.insert(base_type.to_string());
        }

        for type_analysis in &mut self.types {
            type_analysis.used = used_types.contains(&type_analysis.name);
            if !type_analysis.used {
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W0203,
                        format!("Type '{}' is defined but never used", type_analysis.name),
                    )
                    .with_hint(
                        "Remove unused type definition or reference it in @data or @computed"
                            .to_string(),
                    ),
                );
            }
        }

        // Check for unused functions
        // (Would need to scan function calls in bindings - simplified for now)
        for func in &mut self.functions {
            // For now, assume all functions are used unless we implement call tracking
            func.used = true;
        }

        // Check for unreachable states
        for state_machine in &self.state_machines {
            for state in &state_machine.states {
                if !state.has_incoming && state.name != state_machine.initial {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            DiagnosticCode::W0301,
                            format!(
                                "State '{}' in {} has no incoming transitions",
                                state.name, state_machine.selector
                            ),
                        )
                        .with_hint("Add a transition to this state or remove it".to_string())
                        .with_note(format!(
                            "State machine starts at '{}'",
                            state_machine.initial
                        )),
                    );
                }

                if !state.has_outgoing {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            DiagnosticCode::W0302,
                            format!(
                                "State '{}' in {} has no outgoing transitions",
                                state.name, state_machine.selector
                            ),
                        )
                        .with_hint("Add a transition from this state".to_string()),
                    );
                }
            }
        }
    }

    /// Update data source item counts from loaded JSON.
    pub fn update_item_counts(&mut self, json_data: &HashMap<String, serde_json::Value>) {
        for data in &mut self.data_sources {
            if let Some(value) = json_data.get(&data.name)
                && let Some(arr) = value.as_array()
            {
                data.item_count = Some(arr.len());
            }
        }

        // Also update binding instance counts
        for binding in &mut self.bindings {
            if let Some(value) = json_data.get(&binding.data_source)
                && let Some(arr) = value.as_array()
            {
                binding.instance_count = Some(arr.len());
            }
        }
    }

    /// Mark data sources as validated.
    pub fn mark_validated(&mut self, validated_sources: &HashSet<String>) {
        for data in &mut self.data_sources {
            if validated_sources.contains(&data.name) {
                data.validated = true;
            }
        }
    }

    /// Validate cross-references between data sources.
    ///
    /// For example, validates that cart.printId references exist in prints.id
    pub fn validate_cross_references(
        &mut self,
        json_data: &HashMap<String, serde_json::Value>,
        registry: &TypeRegistry,
    ) {
        // This is a simplified implementation that looks for common ID reference patterns
        // A full implementation would need to parse the type definitions to find
        // which fields are references to other data sources

        // For now, we'll validate known patterns like:
        // - cart.printId -> prints.id
        // - curations.prints -> prints.id

        for data in &self.data_sources {
            if let Some(items) = json_data.get(&data.name).and_then(|v| v.as_array()) {
                // Get the type definition
                if let Some(type_def) = registry.get_type(&data.type_name) {
                    for field in &type_def.fields {
                        // Check if field name ends with "Id" or contains "Ref"
                        if field.name.ends_with("Id") || field.name.contains("Ref") {
                            // Try to infer the target data source
                            let target_name = field
                                .name
                                .strip_suffix("Id")
                                .or_else(|| field.name.strip_suffix("Ref"))
                                .map(|s| format!("{}s", s)) // e.g., "print" -> "prints"
                                .unwrap_or_else(|| field.name.clone());

                            if let Some(target_data) =
                                json_data.get(&target_name).and_then(|v| v.as_array())
                            {
                                // Collect all IDs from target
                                let target_ids: HashSet<String> = target_data
                                    .iter()
                                    .filter_map(|item| {
                                        item.get("id").and_then(|v| v.as_str()).map(String::from)
                                    })
                                    .collect();

                                // Validate references
                                for (i, item) in items.iter().enumerate() {
                                    if let Some(ref_id) =
                                        item.get(&field.name).and_then(|v| v.as_str())
                                        && !target_ids.contains(ref_id)
                                    {
                                        self.diagnostics.push(
                                                Diagnostic::error(
                                                    DiagnosticCode::E0501,
                                                    format!(
                                                        "Invalid reference in {}.{}: '{}' does not exist in {}",
                                                        data.name, field.name, ref_id, target_name
                                                    ),
                                                )
                                                .with_note(format!("Item at index {}", i)),
                                            );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate that computed data dependencies exist.
    pub fn validate_computed_dependencies(&mut self) {
        let known = self.reactive_source_names();
        for comp in &self.computed {
            for dep in &comp.dependencies {
                if !known.contains(dep) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0405,
                            format!(
                                "Computed '{}' depends on '{}', which does not exist",
                                comp.name, dep
                            ),
                        )
                        .with_hint(format!(
                            "Define a @data, @computed, or scoped state source named '{}' or fix the dependency",
                            dep
                        )),
                    );
                }
            }
        }
    }

    /// The set of names that can back a reactive binding or computed: data
    /// sources, computed values, scoped `$state` (locals, both bare and
    /// `&`-prefixed forms), and analyzed signals. One lookup, one mental
    /// model — any reactive value is referenceable by name.
    fn reactive_source_names(&self) -> HashSet<String> {
        let mut names: HashSet<String> = HashSet::new();
        names.extend(self.data_sources.iter().map(|d| d.name.clone()));
        names.extend(self.computed.iter().map(|c| c.name.clone()));
        for l in &self.locals {
            names.insert(l.name.clone());
            names.insert(format!("&{}", l.name));
        }
        names.extend(self.signals.iter().map(|s| s.name.clone()));
        names
    }

    /// Validate that binding data sources exist.
    ///
    /// `binding.data_source` may be a DOTTED PATH (`"wrap.items"` from
    /// `@each($wrap.items as $x)` / any dotted binding source) -- the runtime
    /// correctly resolves this by treating the HEAD segment as the actual signal
    /// and walking the rest as property access (see BUG-155:
    /// `ST.resolvePath` in `each-with-templates` and friends). This static check
    /// must mirror that: validate the HEAD name against known reactive sources,
    /// not the full dotted string, else every dotted-path binding source is
    /// (falsely) flagged as referencing a nonexistent signal — the compile-time
    /// analysis and the runtime resolver must agree on what a valid source is.
    pub fn validate_binding_sources(&mut self) {
        let known = self.reactive_source_names();
        for binding in &self.bindings {
            let head = binding
                .data_source
                .split('.')
                .next()
                .unwrap_or(&binding.data_source);
            if !known.contains(&binding.data_source) && !known.contains(head) {
                self.diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0405,
                        format!(
                            "Binding in '{}' references '{}', which does not exist",
                            binding.selector, binding.data_source
                        ),
                    )
                    .with_hint(format!(
                        "Define a @data, @computed, or scoped state source named '{}'",
                        binding.data_source
                    )),
                );
            }
        }
    }

    /// Field-check every dotted read against its binding's declared type
    /// (FUP-150 / PLAN-119 W2). For each `$base.a.b` read whose `$base` is a
    /// data source declared WITH a concrete `@type` (e.g. a FUP-144 typed
    /// `@data subscribe $form FormEnvelope …`), walk the access path against the
    /// registered type; an unknown field surfaces as E0401 with a
    /// "did you mean" hint — the within-payload sibling of E0928 (which checks
    /// event names across the seam) and E0405 (which checks the base exists).
    ///
    /// Purely additive: a read whose base is UNTYPED (`type_name == "any"`, the
    /// pre-FUP-144 default) is skipped — there is no shape to check against, so
    /// today's behavior is preserved. A bare `$base` with no path is trivially
    /// valid (nothing to walk).
    pub fn validate_typed_reads(&mut self, registry: &TypeRegistry, scopes: &[ScopeBlock]) {
        use crate::syntax::collect_dotted_signal_reads;
        use crate::type_system::parse_type_ref_to_expr;

        // Map each typed data source to its declared type expression. Skip the
        // untyped default and any type the registry can't resolve (a dangling
        // reference is E0102's job, not ours — we stay silent there).
        let mut typed: HashMap<String, crate::parser::TypeExpr> = HashMap::new();
        for data in &self.data_sources {
            if data.type_name == "any" || data.type_name.is_empty() {
                continue;
            }
            typed.insert(data.name.clone(), parse_type_ref_to_expr(&data.type_name));
        }
        if typed.is_empty() {
            return;
        }

        fn check_decls(
            css: &[CssDeclaration],
            typed: &HashMap<String, crate::parser::TypeExpr>,
            registry: &TypeRegistry,
            out: &mut Vec<Diagnostic>,
        ) {
            for d in css {
                for (base, path) in collect_dotted_signal_reads(&d.value) {
                    if path.is_empty() {
                        continue; // bare `$base` — nothing to field-check
                    }
                    let Some(type_expr) = typed.get(&base) else {
                        continue; // untyped base — additive skip
                    };
                    if let Err(diag) = registry.check_read_path(type_expr, &path, d.span.into()) {
                        out.push(diag);
                    }
                }
            }
        }
        fn check_nested(
            nested: &[NestedScope],
            typed: &HashMap<String, crate::parser::TypeExpr>,
            registry: &TypeRegistry,
            out: &mut Vec<Diagnostic>,
        ) {
            for n in nested {
                check_decls(&n.css_declarations, typed, registry, out);
                check_nested(&n.nested_scopes, typed, registry, out);
            }
        }

        let mut found: Vec<Diagnostic> = Vec::new();
        for scope in scopes {
            check_decls(&scope.css_declarations, &typed, registry, &mut found);
            check_nested(&scope.nested_scopes, &typed, registry, &mut found);
        }
        self.diagnostics.extend(found);
    }

    /// Validate that data source files exist on disk.
    ///
    /// For data sources with file paths (like "/data/packs.json"), checks if the
    /// file exists relative to the site directory. Reports E0504 for missing files.
    pub fn validate_data_source_files(&mut self, site_dir: &Path) {
        for data in &self.data_sources {
            if let DataSource::File(path) = &data.source {
                // Skip non-local paths (e.g., http://, https://)
                if path.starts_with("http://") || path.starts_with("https://") {
                    continue;
                }

                // Skip paths with {locale} placeholder — resolved at build time
                if path.contains("{locale}") {
                    continue;
                }

                // Skip server-runtime endpoints — the /__spacetime/ namespace is
                // generated by the dev server (e.g. /__spacetime/dev/site.json),
                // never a file on disk. The CMS admin fetches these at runtime.
                if path.starts_with("/__spacetime/") {
                    continue;
                }

                // Handle absolute paths (starting with /) relative to site dir
                let file_path = if path.starts_with('/') {
                    site_dir.join(&path[1..])
                } else {
                    site_dir.join(path)
                };

                if !file_path.exists() {
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0504,
                            format!("Data source file not found: {}", path),
                        )
                        .with_hint(format!("Expected file at: {}", file_path.display())),
                    );
                }
            }
        }
    }
}

impl Default for CompileAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Report Generator
// =============================================================================

impl CompileAnalysis {
    /// Generate a formatted compile report.
    pub fn generate_report(&self, project_name: &str) -> String {
        let mut report = String::new();
        let sep = "══════════════════════════════════════════════════════════════";

        // Header
        report.push_str(sep);
        report.push('\n');
        report.push_str(&format!("  SPACETIME COMPILE REPORT: {}\n", project_name));
        report.push_str(sep);
        report.push_str("\n\n");

        // Types section
        if !self.types.is_empty() {
            report.push_str("Types:\n");
            for type_analysis in &self.types {
                let symbol = if type_analysis.used { "✓" } else { "⚠" };
                let optional_info = if type_analysis.optional_fields > 0 {
                    format!(", {} optional", type_analysis.optional_fields)
                } else {
                    String::new()
                };
                report.push_str(&format!(
                    "  {} {} ({} fields{})\n",
                    symbol, type_analysis.name, type_analysis.total_fields, optional_info
                ));
            }
            report.push('\n');
        }

        // Data Sources section
        if !self.data_sources.is_empty() {
            report.push_str("Data Sources:\n");
            for data in &self.data_sources {
                let symbol = if data.used { "✓" } else { "⚠" };
                let type_str = if data.is_array {
                    format!("{}[]", data.type_name)
                } else {
                    data.type_name.clone()
                };

                let source_str = match &data.source {
                    DataSource::File(path) => {
                        let validation = if data.validated {
                            "validated"
                        } else {
                            "not validated"
                        };
                        let count_str = data
                            .item_count
                            .map(|c| format!(", {} items", c))
                            .unwrap_or_default();
                        format!("{} ({}{})", path, validation, count_str)
                    }
                    DataSource::Inline(_) => "inline".to_string(),
                    DataSource::LocalStorage => "localStorage (runtime)".to_string(),
                    DataSource::Runtime => "runtime".to_string(),
                };

                report.push_str(&format!(
                    "  {} {}: {} → {}\n",
                    symbol, data.name, type_str, source_str
                ));
            }
            report.push('\n');
        }

        // Computed section
        if !self.computed.is_empty() {
            report.push_str("Computed:\n");
            for comp in &self.computed {
                let deps_str = if !comp.dependencies.is_empty() {
                    format!(" (from {})", comp.dependencies.join(", "))
                } else {
                    String::new()
                };

                // Show operations used
                let mut ops = Vec::new();
                if comp.has_where {
                    ops.push("filter");
                }
                if comp.has_sort {
                    ops.push("sort");
                }
                if comp.has_limit {
                    ops.push("limit");
                }
                if comp.has_reduce {
                    ops.push("reduce");
                }
                let ops_str = if !ops.is_empty() {
                    format!(" [{}]", ops.join(", "))
                } else {
                    String::new()
                };

                report.push_str(&format!(
                    "  ✓ {}: {}{}{}\n",
                    comp.name, comp.type_name, deps_str, ops_str
                ));
            }
            report.push('\n');
        }

        // Functions section
        if !self.functions.is_empty() {
            report.push_str("Functions:\n");
            for func in &self.functions {
                let symbol = if func.used { "✓" } else { "⚠" };
                report.push_str(&format!("  {} {}\n", symbol, func.signature));
            }
            report.push('\n');
        }

        // Templates section
        if !self.templates.is_empty() {
            report.push_str("Templates:\n");
            for template in &self.templates {
                let symbol = if template.used { "✓" } else { "⚠" };
                let slots_str = if !template.slots.is_empty() {
                    format!(" (slots: {})", template.slots.join(", "))
                } else {
                    String::new()
                };
                report.push_str(&format!("  {} {}{}\n", symbol, template.name, slots_str));
            }
            report.push('\n');
        }

        // Bindings section
        if !self.bindings.is_empty() {
            report.push_str("Bindings:\n");
            for binding in &self.bindings {
                let count_str = binding
                    .instance_count
                    .map(|c| format!(" ({} instances)", c))
                    .unwrap_or_else(|| " (runtime)".to_string());
                report.push_str(&format!(
                    "  ✓ {} → {}{}\n",
                    binding.selector, binding.data_source, count_str
                ));
            }
            report.push('\n');
        }

        // Signals section
        if !self.signals.is_empty() {
            report.push_str("Signals:\n");
            for signal in &self.signals {
                let symbol = if signal.is_used() || signal.is_exported {
                    "✓"
                } else {
                    "⚠"
                };
                let usage_count = signal.used_at.len();
                let usage_str = if usage_count > 0 {
                    format!(" ({} usages)", usage_count)
                } else if signal.is_exported {
                    " (exported)".to_string()
                } else {
                    " (unused)".to_string()
                };
                report.push_str(&format!(
                    "  {} ${}: defined by {}{}\n",
                    symbol, signal.name, signal.defined_by, usage_str
                ));
            }
            report.push('\n');
        }

        // State Machines section
        if !self.state_machines.is_empty() {
            report.push_str("State Machines:\n");
            for sm in &self.state_machines {
                let all_valid = sm
                    .states
                    .iter()
                    .all(|s| s.has_incoming || s.name == sm.initial);
                let symbol = if all_valid { "✓" } else { "⚠" };

                // Build transition summary
                let mut reachable_states: HashSet<String> = HashSet::new();
                reachable_states.insert(sm.initial.clone());
                for trans in &sm.transitions {
                    if reachable_states.contains(&trans.from) {
                        reachable_states.insert(trans.to.clone());
                    }
                }

                let state_list: Vec<String> = reachable_states.into_iter().collect();
                let status = if all_valid {
                    "all transitions valid"
                } else {
                    "has warnings"
                };

                report.push_str(&format!(
                    "  {} {}: {} → {} ({})\n",
                    symbol,
                    sm.selector,
                    sm.initial,
                    state_list.join(" | "),
                    status
                ));
            }
            report.push('\n');
        }

        // Summary
        report.push_str(sep);
        report.push('\n');
        let warning_count = self
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, crate::diagnostics::Severity::Warning))
            .count();
        let error_count = self
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, crate::diagnostics::Severity::Error))
            .count();
        report.push_str(&format!(
            "  WARNINGS: {} | ERRORS: {}\n",
            warning_count, error_count
        ));
        report.push_str(sep);
        report.push('\n');

        // Detailed diagnostics
        if !self.diagnostics.is_empty() {
            report.push('\n');
            for diag in &self.diagnostics {
                let severity_str = match diag.severity {
                    crate::diagnostics::Severity::Error => "error",
                    crate::diagnostics::Severity::Warning => "warning",
                };
                report.push_str(&format!(
                    "{}[{}]: {}\n",
                    severity_str, diag.code, diag.message
                ));
                if let Some(hint) = &diag.hint {
                    report.push_str(&format!("  = help: {}\n", hint));
                }
                for note in &diag.notes {
                    report.push_str(&format!("  = note: {}\n", note));
                }
                report.push('\n');
            }
        }

        // Final status
        report.push_str(sep);
        report.push('\n');
        if error_count == 0 {
            report.push_str("  BUILD SUCCEEDED\n");
        } else {
            report.push_str("  BUILD FAILED\n");
        }
        report.push_str(sep);
        report.push('\n');

        report
    }
}

/// Resolve a `%binds` output name to its concrete signal name for a given match,
/// substituting `${$cap}` / `${cap}` interpolations (and a bare leading `$cap`)
/// with the match's capture values. Returns None if a referenced capture is
/// absent (the yield can't be named for this invocation). BUG-126 helper.
pub(crate) fn resolve_yield_name(
    raw: &str,
    captures: &HashMap<String, CapturedValue>,
) -> Option<String> {
    // Fast path: a literal yield like `$items` / `users` with no interpolation.
    if !raw.contains("${") {
        return Some(raw.to_string());
    }
    // Resolve `${ $cap }` / `${cap}` segments against the captures.
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // find closing brace
            let close = raw[i + 2..].find('}')?;
            let inner = raw[i + 2..i + 2 + close]
                .trim()
                .trim_start_matches('$')
                .trim();
            let val = capture_scalar(captures.get(inner)?)?;
            out.push_str(&val);
            i = i + 2 + close + 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    Some(out)
}

/// Extract a scalar identifier-ish string from a capture (binding/ident), used to
/// fill `${$name}` alias slots. Strips a leading `$` from binding captures.
fn capture_scalar(value: &CapturedValue) -> Option<String> {
    match value {
        CapturedValue::Binding(b) => Some(b.trim_start_matches('$').to_string()),
        CapturedValue::Ident(s) => Some(s.clone()),
        CapturedValue::String(s) => Some(s.clone()),
        _ => None,
    }
}
