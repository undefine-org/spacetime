//! CLI inspect command - visualize compilation layers
//!
//! This module provides introspection into the Spacetime 5-layer compilation pipeline,
//! allowing developers to examine:
//!
//! 1. **Parse layer** - Raw AST from parsing (scopes, directives, patterns, etc.)
//! 2. **Registry layer** - Loaded primitives and macros from stdlib
//! 3. **Keyframes layer** - Captured animation properties from @scroll/@load/@on bodies
//! 4. **Expansion layer** - Macro expansion to %binds/%derives/%states
//! 5. **IR layer** - Intermediate representation (JsStmt, CssExpr IR types)
//! 6. **Emit layer** - Final emitted JS/CSS code

use crate::compiler::Compiler;
use crate::metasystem::MetaRegistry;
use crate::parser::{StFile, parse};
use crate::pipeline::scope::build_scope_tree;

use serde::Serialize;
use std::path::{Path, PathBuf};

// =============================================================================
// Output Format
// =============================================================================

/// Output format for inspect command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Pretty-printed colored terminal output (default)
    Pretty,
    /// JSON output for machine consumption
    Json,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pretty" => Some(OutputFormat::Pretty),
            "json" => Some(OutputFormat::Json),
            _ => None,
        }
    }
}

// =============================================================================
// JSON Output Structures
// =============================================================================

/// JSON output for parse layer
#[derive(Debug, Clone, Serialize)]
pub struct ParseLayerOutput {
    pub layer: String,
    pub summary: ParseSummary,
    pub imports: Vec<String>,
    pub presets: Vec<PresetSummary>,
    pub patterns: Vec<PatternSummary>,
    pub types: Vec<TypeSummary>,
    pub data: Vec<DataSummary>,
    pub computed: Vec<ComputedSummary>,
    pub functions: Vec<FunctionSummary>,
    pub scopes: Vec<ScopeSummary>,
    pub file_level_directives: Vec<DirectiveSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParseSummary {
    pub imports: usize,
    pub presets: usize,
    pub patterns: usize,
    pub meta_defs: usize,
    pub types: usize,
    pub data: usize,
    pub computed: usize,
    pub functions: usize,
    pub scopes: usize,
    pub file_level_directives: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PresetSummary {
    pub name: String,
    pub preset_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternSummary {
    pub name: String,
    pub params: Vec<String>,
    pub body_items: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeSummary {
    pub name: String,
    pub field_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataSummary {
    pub name: String,
    pub option_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComputedSummary {
    pub name: String,
    pub expression: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionSummary {
    pub name: String,
    pub param_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScopeSummary {
    pub selector: String,
    pub directives: Vec<DirectiveSummary>,
    pub css_declarations: usize,
    pub each_blocks: usize,
    pub nested_scopes: usize,
    pub on_mutations: usize,
    pub element_refs: usize,
    pub value_declarations: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectiveSummary {
    pub name: String,
    pub captures: Vec<String>,
}

/// JSON output for registry layer
#[derive(Debug, Clone, Serialize)]
pub struct RegistryLayerOutput {
    pub layer: String,
    pub summary: RegistrySummary,
    pub primitives: Vec<PrimitiveSummary>,
    pub macros: Vec<MacroDefSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrySummary {
    pub primitives: usize,
    pub macros: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrimitiveSummary {
    pub name: String,
    pub params: Vec<String>,
    pub js_emit_count: usize,
    pub css_emit_count: usize,
    pub exports: Vec<String>,
    /// Leading `///` doc-comment, or `None` when undocumented (FEAT-083).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MacroDefSummary {
    pub name: String,
    pub form: Option<String>,
    pub binds: Vec<String>,
    pub derives_count: usize,
    pub has_states: bool,
    pub scopes: Vec<String>,
    /// Leading `///` doc-comment, or `None` when undocumented (FEAT-083).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

// =============================================================================
// Dispatch layer (FEAT-087): predicate-dispatch introspection
// =============================================================================

/// JSON output for the dispatch layer: every dispatchable macro grouped by the
/// directive it competes under, plus an optional resolution table when a call is
/// probed with `--probe`.
#[derive(Debug, Clone, Serialize)]
pub struct DispatchLayerOutput {
    pub layer: String,
    /// Directives, each with the macro overloads that compete under it.
    pub directives: Vec<DispatchDirective>,
    /// Present only when `--probe "<call>"` was given: the scored resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe: Option<DispatchProbe>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DispatchDirective {
    /// e.g. "@data" — the directive macros compete under.
    pub directive: String,
    pub overloads: Vec<DispatchOverload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DispatchOverload {
    /// %macro name (e.g. "data-fetch").
    pub name: String,
    /// Rendered form signature (minimal in FEAT-087; FEAT-089 upgrades it).
    pub signature: String,
    pub scopes: Vec<String>,
    pub binds: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// The scored resolution of one probed call across the candidate overloads.
#[derive(Debug, Clone, Serialize)]
pub struct DispatchProbe {
    pub call: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directive: Option<String>,
    pub candidates: Vec<DispatchCandidate>,
    /// True when the top two viable candidates tie on score (FEAT-088 territory).
    pub ambiguous: bool,
    /// winner.total − best-viable-runner-up.total (None when <2 viable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<u32>,
    /// Set when the call did not parse to any directive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DispatchCandidate {
    pub macro_name: String,
    /// Final score; `None` = hard-rejected (a required element missing / type mismatch).
    pub total: Option<u32>,
    pub winner: bool,
    pub terms: Vec<crate::syntax::form_scoring::ScoreTerm>,
}

/// JSON output for keyframes layer
#[derive(Debug, Clone, Serialize)]
pub struct KeyframesLayerOutput {
    pub layer: String,
    pub timelines: Vec<TimelineSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineSummary {
    pub name: String,
    pub timeline_type: String,
    pub selector: String,
    pub properties: Vec<PropertySummary>,
    pub nested_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertySummary {
    pub name: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub raw_value: Option<String>,
}

/// JSON output for expansion layer
#[derive(Debug, Clone, Serialize)]
pub struct ExpansionLayerOutput {
    pub layer: String,
    pub expansions: Vec<ExpansionSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpansionSummary {
    pub selector: String,
    pub macro_call: DirectiveSummary,
    pub expands_to: Option<ExpansionDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpansionDetail {
    pub binds: Vec<BindSummary>,
    pub derives: Vec<DeriveSummary>,
    pub has_states: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BindSummary {
    pub primitive: String,
    pub arg_count: usize,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeriveSummary {
    pub name: String,
    pub expression: String,
}

/// JSON output for emit layer
#[derive(Debug, Clone, Serialize)]
pub struct EmitLayerOutput {
    pub layer: String,
    pub css: EmitOutput,
    pub js: EmitOutput,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmitOutput {
    pub size_bytes: usize,
    pub content: String,
}

/// JSON output for IR layer
#[derive(Debug, Clone, Serialize)]
pub struct IrLayerOutput {
    pub layer: String,
    pub summary: IrSummary,
    pub js_stmts: Vec<JsStmtSummary>,
    pub css_exprs: Vec<CssExprSummary>,
}

/// Summary of IR structure
#[derive(Debug, Clone, Serialize)]
pub struct IrSummary {
    pub js_stmt_count: usize,
    pub css_expr_count: usize,
    pub js_raw_count: usize,
    pub css_raw_count: usize,
    pub js_structured_count: usize,
    pub css_structured_count: usize,
}

/// Summary of a JS statement in the IR
#[derive(Debug, Clone, Serialize)]
pub struct JsStmtSummary {
    pub kind: String,
    pub preview: String,
}

/// Summary of a CSS expression in the IR
#[derive(Debug, Clone, Serialize)]
pub struct CssExprSummary {
    pub kind: String,
    pub preview: String,
}

/// Create a MetaRegistry with stdlib loaded.
///
/// BUG-372: this used to load exactly three directories — `runtime`, `macros`,
/// `primitives`. Twenty stdlib directories define `%macro`s. Everything in
/// `enum` (`@match`/`@view`), `mobile`, `3d`, `text`, `dnd`, `fields`, … was
/// therefore INVISIBLE here, and `inspect --layer expansion` reported
/// `"expands_to": null` for a directive the compiler expands perfectly well —
/// indistinguishable, to a reader or a tool, from "this macro has no bindings".
///
/// The compiler does not hardcode a list: it follows the file's `@import`s
/// (`resolve_imports`). Inspect cannot always do that (the `registry` layer has
/// no file at all), so it loads every stdlib directory instead — a superset,
/// which is the safe direction for a read-only view.
fn create_registry_with_stdlib() -> MetaRegistry {
    let mut registry = MetaRegistry::new();
    // Runtime registries first: they define the namespaces macros register into.
    let _ = registry.load_stdlib_from_dir(Path::new("stdlib/runtime"));
    let _ = registry.load_stdlib_from_dir(Path::new("stdlib/macros"));
    let _ = registry.load_stdlib_from_dir(Path::new("stdlib/primitives"));

    // Then every other stdlib subdirectory, so a macro's home directory is not
    // the thing that decides whether tooling can see it.
    if let Ok(entries) = std::fs::read_dir("stdlib") {
        let mut dirs: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .filter(|p| {
                !matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("runtime") | Some("macros") | Some("primitives")
                )
            })
            .collect();
        // Deterministic order: `read_dir` is filesystem-ordered, and this
        // registry decides which macro wins a name collision.
        dirs.sort();
        for dir in dirs {
            let _ = registry.load_stdlib_from_dir(&dir);
        }
    }

    registry
}

/// Layer to inspect in the compilation pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectLayer {
    /// Raw AST from parsing
    Parse,
    /// Loaded primitives and macros from stdlib
    Registry,
    /// Captured animation properties from timeline bodies
    Keyframes,
    /// Macro expansion to %binds/%derives/%states
    Expansion,
    /// Intermediate representation (placeholder)
    Ir,
    /// Final emitted JS/CSS code
    Emit,
    /// Compile-time scope tree (template state declarations)
    Scopes,
    /// PLAN-117 W2: page-global state provenance — who declares each `$cell`,
    /// who reads it, who writes it, and what the compiler can PROVE about it
    /// (const / reactive / dead). One registry serving three consumers: this
    /// table, the E0938 diagnostic, and the W4 fold pass.
    State,
    /// Predicate-dispatch introspection: macros grouped by directive + `--probe`
    /// resolution table (FEAT-087).
    Dispatch,
}

impl InspectLayer {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "parse" => Some(InspectLayer::Parse),
            "registry" => Some(InspectLayer::Registry),
            "keyframes" => Some(InspectLayer::Keyframes),
            "expansion" => Some(InspectLayer::Expansion),
            "ir" => Some(InspectLayer::Ir),
            "emit" => Some(InspectLayer::Emit),
            // Backwards compatibility
            "emission" => Some(InspectLayer::Emit),
            "scopes" => Some(InspectLayer::Scopes),
            "state" => Some(InspectLayer::State),
            "dispatch" => Some(InspectLayer::Dispatch),
            _ => None,
        }
    }

    /// List all available layer names
    pub fn all_names() -> Vec<&'static str> {
        vec![
            "parse",
            "registry",
            "keyframes",
            "expansion",
            "ir",
            "emit",
            "scopes",
            "state",
            "dispatch",
        ]
    }

    /// Get the string representation of this layer
    pub fn as_str(&self) -> &'static str {
        match self {
            InspectLayer::Parse => "parse",
            InspectLayer::Registry => "registry",
            InspectLayer::Keyframes => "keyframes",
            InspectLayer::Expansion => "expansion",
            InspectLayer::Ir => "ir",
            InspectLayer::Emit => "emit",
            InspectLayer::Scopes => "scopes",
            InspectLayer::State => "state",
            InspectLayer::Dispatch => "dispatch",
        }
    }
}

/// Filter options for inspect output
#[derive(Debug, Clone, Default)]
pub struct InspectFilter {
    /// Filter to specific CSS selector (e.g., ".hero")
    pub selector: Option<String>,
    /// Filter to specific timeline name (e.g., "nav-transition")
    pub timeline: Option<String>,
    /// Dispatch layer: a call string to resolve against candidate forms
    /// (e.g. `@data fetch $api : "/url"`). FEAT-087.
    pub probe: Option<String>,
}

/// Run the inspect command
pub fn run_inspect(
    file: Option<PathBuf>,
    layer: InspectLayer,
    filter: InspectFilter,
    format: OutputFormat,
) -> Result<(), String> {
    // Registry layer doesn't need a file
    if layer == InspectLayer::Registry {
        return inspect_registry(&filter, format);
    }

    // Dispatch layer is fileless too: it introspects the stdlib registry's forms.
    if layer == InspectLayer::Dispatch {
        return inspect_dispatch(&filter, format);
    }

    // All other layers require a file
    let file = file.ok_or_else(|| format!("Layer '{}' requires a file", layer.as_str()))?;

    // Read the file
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read {}: {}", file.display(), e))?;

    // Parse the file — EDN entries (PLAN-148) lift through the same
    // extension-gated ingress `Compiler::from_file` uses, so `inspect` reads
    // exactly what a compile would.
    let parsed_ast = match crate::compiler::Compiler::edn_ingress(&file, &content) {
        Some(r) => r.map_err(|e| format!("EDN source {}: {}", file.display(), e))?,
        None => parse(&content)
            .map_err(|e| format!("Failed to parse {}: {}", file.display(), e))?,
    };

    // For parse layer, show raw AST without import resolution
    if layer == InspectLayer::Parse {
        return inspect_parse(&parsed_ast, &filter, format);
    }

    // For all other layers, resolve imports first (mirrors Compiler::from_file behavior)
    let mut ast = if !parsed_ast.imports.is_empty() {
        // Determine workspace root: walk up from file to find site root
        let workspace_root = file
            .parent()
            .and_then(|p| {
                // Walk up to find the directory that looks like a workspace root
                // (contains themes.css, index.html, or is the site directory)
                let mut dir = p;
                loop {
                    if dir.join("themes.css").exists() || dir.join("index.html").exists() {
                        return Some(dir.to_path_buf());
                    }
                    dir = dir.parent()?;
                }
            })
            .unwrap_or_else(|| {
                file.parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf()
            });

        crate::parser::resolve_imports(&parsed_ast, &file, &workspace_root)
            .map_err(|e| format!("Import resolution failed: {}", e))?
    } else {
        parsed_ast
    };

    // Re-extract FormMatches if imports brought user-defined macros
    if !ast.meta_defs.is_empty() {
        crate::parser::rematch_with_user_macros(&mut ast, &content);
    }

    match layer {
        InspectLayer::Parse => unreachable!(),    // Handled above
        InspectLayer::Registry => unreachable!(), // Handled above
        InspectLayer::Dispatch => unreachable!(), // Handled above
        InspectLayer::Keyframes => inspect_keyframes(&ast, &filter, format),
        InspectLayer::Expansion => inspect_expansion(&ast, &filter, format),
        InspectLayer::Ir => inspect_ir(&ast, &filter, format),
        InspectLayer::Emit => inspect_emit(&ast, &filter, &content, format),
        InspectLayer::Scopes => inspect_scopes(&ast, &filter, format),
        InspectLayer::State => inspect_state(&ast, format),
    }
}

/// PLAN-117 W2: print page-global state provenance.
///
/// This table is the deliverable of W2. It is simultaneously:
///   - the DIAGNOSTIC input (E0938 reads `declared_in`),
///   - the DOCUMENTATION (what does this page's state actually consist of?),
///   - and the OPTIMISATION input (W4 folds every `const`, drops every `dead`).
///
/// The verdict is proven, not guessed: Spacetime has no custom JavaScript, so a
/// cell's write-set is closed and a whole-page scan settles it.
fn inspect_state(ast: &StFile, format: OutputFormat) -> Result<(), String> {
    let provenance = crate::pipeline::state_analysis::analyze_state(ast);

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "layer": "state",
                    "cells": provenance,
                }))
                .unwrap()
            );
        }
        OutputFormat::Pretty => {
            println!("=== Page-global State ===\n");
            if provenance.is_empty() {
                println!("(no page-global state declarations)");
                println!(
                    "\n\x1b[90mNB element-scoped state (`.card {{ $n … }}`) is plural by \
                     construction — a selector matches 0..N elements — so it has no single \
                     address and is not listed here.\x1b[0m"
                );
                return Ok(());
            }

            for p in &provenance {
                let (colour, verdict) = match p.verdict {
                    crate::pipeline::scope::StateVerdict::Const => ("\x1b[32m", "const"),
                    crate::pipeline::scope::StateVerdict::Reactive => ("\x1b[36m", "reactive"),
                    crate::pipeline::scope::StateVerdict::Dead => ("\x1b[33m", "dead"),
                };
                println!(
                    "\x1b[1m${}\x1b[0m  \x1b[90m{}\x1b[0m  = {}",
                    p.var_name, p.type_name, p.initial
                );
                println!(
                    "    \x1b[90mdeclared\x1b[0m {}{}",
                    p.declared_in.join(", "),
                    if p.declared_in.len() > 1 {
                        "   \x1b[31m← E0938: declared more than once\x1b[0m"
                    } else {
                        ""
                    }
                );
                println!("    \x1b[90mreaders\x1b[0m  {}", fmt_usage(&p.readers));
                println!("    \x1b[90mwriters\x1b[0m  {}", fmt_usage(&p.writers));
                println!(
                    "    \x1b[90mverdict\x1b[0m  {colour}{verdict}\x1b[0m {}",
                    match p.verdict {
                        crate::pipeline::scope::StateVerdict::Const =>
                            "\x1b[90m— write-free literal; foldable at every read\x1b[0m",
                        crate::pipeline::scope::StateVerdict::Reactive =>
                            "\x1b[90m— has a writer or a computed initial; stays a live cell\x1b[0m",
                        crate::pipeline::scope::StateVerdict::Dead =>
                            "\x1b[90m— read by nobody; eliminable\x1b[0m",
                    }
                );
                println!();
            }
        }
    }

    Ok(())
}

/// Render a per-file usage list as `file (n) · file (n)`, or ∅ when empty.
fn fmt_usage(usage: &[(String, usize)]) -> String {
    if usage.is_empty() {
        return "∅".to_string();
    }
    usage
        .iter()
        .map(|(f, n)| format!("{f} ({n})"))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Inspect the parse layer - show raw AST structure
fn inspect_parse(ast: &StFile, filter: &InspectFilter, format: OutputFormat) -> Result<(), String> {
    // Build output data structure
    let output = build_parse_output(ast, filter);

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => {
            print_parse_pretty(ast, filter);
        }
    }

    Ok(())
}

/// Build parse layer output structure
fn build_parse_output(ast: &StFile, filter: &InspectFilter) -> ParseLayerOutput {
    // Count types, data, computed, functions from FormMatches instead of legacy fields
    let types_count = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "type")
        .count();
    let data_count = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "data")
        .count();
    let computed_count = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "computed")
        .count();
    let functions_count = ast.matches.iter().filter(|m| m.macro_name == "fn").count();

    let summary = ParseSummary {
        imports: ast.imports.len(),
        presets: ast.presets.len(),
        patterns: ast.patterns.len(),
        meta_defs: ast.meta_defs.len(),
        types: types_count,
        data: data_count,
        computed: computed_count,
        functions: functions_count,
        scopes: ast.scopes.len(),
        file_level_directives: ast
            .matches
            .iter()
            .filter(|m| m.selector.is_none())
            .filter(|m| {
                m.macro_name != "type"
                    && m.macro_name != "data"
                    && m.macro_name != "computed"
                    && m.macro_name != "fn"
            })
            .count(),
    };

    let presets: Vec<PresetSummary> = ast
        .presets
        .iter()
        .map(|p| PresetSummary {
            name: p.name.clone(),
            preset_type: format!("{:?}", p.preset_type),
        })
        .collect();

    let patterns: Vec<PatternSummary> = ast
        .patterns
        .iter()
        .map(|p| PatternSummary {
            name: p.name.clone(),
            params: p
                .params
                .iter()
                .map(|param| format!("${}", param.name))
                .collect(),
            body_items: p.body.len(),
        })
        .collect();

    // Extract types from FormMatches
    let types: Vec<TypeSummary> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "type")
        .filter_map(|m| {
            m.get_ident("name").map(|name| TypeSummary {
                name: name.to_string(),
                field_count: m.get_properties("fields").map(|f| f.len()).unwrap_or(0),
            })
        })
        .collect();

    // Extract data from FormMatches
    let data: Vec<DataSummary> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "data")
        .filter_map(|m| {
            m.get_ident("name").map(|name| DataSummary {
                name: name.to_string(),
                option_count: 0, // Options are captured differently in FormMatch
            })
        })
        .collect();

    // Extract computed from FormMatches
    let computed: Vec<ComputedSummary> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "computed")
        .filter_map(|m| {
            m.get_ident("name").map(|name| ComputedSummary {
                name: name.to_string(),
                expression: "computed".to_string(), // Type expression not easily accessible from FormMatch
            })
        })
        .collect();

    // Extract functions from FormMatches
    let functions: Vec<FunctionSummary> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "fn")
        .filter_map(|m| {
            m.get_ident("name").map(|name| FunctionSummary {
                name: name.to_string(),
                param_count: 0, // Param count not easily accessible from FormMatch
            })
        })
        .collect();

    let scopes: Vec<ScopeSummary> = ast
        .scopes
        .iter()
        .filter(|scope| {
            if let Some(ref filter_selector) = filter.selector {
                &scope.selector == filter_selector
            } else {
                true
            }
        })
        .map(|scope| ScopeSummary {
            selector: scope.selector.clone(),
            directives: scope.matches.iter().map(summarize_directive).collect(),
            css_declarations: scope.css_declarations.len(),
            each_blocks: scope
                .matches
                .iter()
                .filter(|m| m.macro_name == "each")
                .count(),
            nested_scopes: scope.nested_scopes.len(),
            on_mutations: scope
                .matches
                .iter()
                .filter(|m| m.macro_name.starts_with("on-"))
                .count(),
            element_refs: scope
                .matches
                .iter()
                .filter(|m| m.macro_name == "element-ref")
                .count(),
            value_declarations: scope
                .matches
                .iter()
                .filter(|m| {
                    m.macro_name == "local-state" || m.macro_name == "local-state-uninitialized"
                })
                .count(),
        })
        .collect();

    // File-level directives from FormMatches (type, data, computed, fn, etc.)
    let file_level_directives: Vec<DirectiveSummary> = ast
        .matches
        .iter()
        .filter(|m| m.selector.is_none())
        .filter(|m| {
            m.macro_name != "type"
                && m.macro_name != "data"
                && m.macro_name != "computed"
                && m.macro_name != "fn"
        })
        .map(summarize_directive)
        .collect();

    ParseLayerOutput {
        layer: "parse".to_string(),
        summary,
        imports: ast.imports.iter().map(|i| i.path.clone()).collect(),
        presets,
        patterns,
        types,
        data,
        computed,
        functions,
        scopes,
        file_level_directives,
    }
}

/// Build a DirectiveSummary from a FormMatch
fn summarize_directive(fm: &crate::syntax::FormMatch) -> DirectiveSummary {
    let mut captures: Vec<String> = fm
        .captures
        .iter()
        .filter(|(k, _)| !k.starts_with("__"))
        .map(|(k, v)| format!("{}: {}", k, format_captured_value(v)))
        .collect();
    captures.sort();
    DirectiveSummary {
        name: fm.macro_name.clone(),
        captures,
    }
}

/// Format a CapturedValue for display
fn format_captured_value(v: &crate::syntax::CapturedValue) -> String {
    v.to_string_value()
}

/// Print parse layer in pretty format
fn print_parse_pretty(ast: &StFile, filter: &InspectFilter) {
    println!("\n\x1b[1;36m=== Parse Layer ===\x1b[0m");
    println!("\x1b[90mShowing raw AST structure from parser\x1b[0m\n");

    // Summary statistics
    println!("\x1b[1;33mAST Summary:\x1b[0m");
    println!("  \x1b[32mimports:\x1b[0m {}", ast.imports.len());
    println!("  \x1b[32mpresets:\x1b[0m {}", ast.presets.len());
    println!("  \x1b[32mpatterns:\x1b[0m {}", ast.patterns.len());
    println!("  \x1b[32mmeta_defs:\x1b[0m {}", ast.meta_defs.len());
    println!(
        "  \x1b[32mtypes:\x1b[0m {}",
        ast.matches
            .iter()
            .filter(|m| m.macro_name == "type")
            .count()
    );
    println!(
        "  \x1b[32mdata:\x1b[0m {}",
        ast.matches
            .iter()
            .filter(|m| m.macro_name == "data")
            .count()
    );
    println!(
        "  \x1b[32mcomputed:\x1b[0m {}",
        ast.matches
            .iter()
            .filter(|m| m.macro_name == "computed")
            .count()
    );
    println!(
        "  \x1b[32mfunctions:\x1b[0m {}",
        ast.matches.iter().filter(|m| m.macro_name == "fn").count()
    );
    println!("  \x1b[32mscopes:\x1b[0m {}", ast.scopes.len());
    let other_file_directives = ast
        .matches
        .iter()
        .filter(|m| m.selector.is_none())
        .filter(|m| {
            m.macro_name != "type"
                && m.macro_name != "data"
                && m.macro_name != "computed"
                && m.macro_name != "fn"
        })
        .count();
    println!(
        "  \x1b[32mfile-level directives:\x1b[0m {}",
        other_file_directives
    );
    println!();

    // Show imports if any
    if !ast.imports.is_empty() {
        println!("\x1b[1;33mImports:\x1b[0m");
        for import in &ast.imports {
            println!("  \x1b[36m@import\x1b[0m \"{}\"", import.path);
        }
        println!();
    }

    // Show presets if any
    if !ast.presets.is_empty() {
        println!("\x1b[1;33mPresets:\x1b[0m");
        for preset in &ast.presets {
            println!(
                "  \x1b[36m@preset\x1b[0m {:?} {}",
                preset.preset_type, preset.name
            );
        }
        println!();
    }

    // Show patterns if any
    if !ast.patterns.is_empty() {
        println!("\x1b[1;33mPatterns:\x1b[0m");
        for pattern in &ast.patterns {
            let params: Vec<String> = pattern
                .params
                .iter()
                .map(|p| format!("${}", p.name))
                .collect();
            println!(
                "  \x1b[36m@pattern\x1b[0m {}({}) {{ {} items }}",
                pattern.name,
                params.join(", "),
                pattern.body.len()
            );
        }
        println!();
    }

    // Show types if any (from FormMatches)
    let type_matches: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "type")
        .collect();
    if !type_matches.is_empty() {
        println!("\x1b[1;33mTypes:\x1b[0m");
        for fm in type_matches {
            if let Some(name) = fm.get_ident("name") {
                let field_count = fm.get_properties("fields").map(|f| f.len()).unwrap_or(0);
                println!(
                    "  \x1b[36m@type\x1b[0m {} {{ {} fields }}",
                    name, field_count
                );
            }
        }
        println!();
    }

    // Show data definitions if any (from FormMatches)
    let data_matches: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "data")
        .collect();
    if !data_matches.is_empty() {
        println!("\x1b[1;33mData Definitions:\x1b[0m");
        for fm in data_matches {
            if let Some(name) = fm.get_ident("name") {
                println!("  \x1b[36m@data\x1b[0m ${}", name);
            }
        }
        println!();
    }

    // Show scopes with details
    if !ast.scopes.is_empty() {
        println!("\x1b[1;33mScopes:\x1b[0m");
        for scope in &ast.scopes {
            // Apply selector filter if specified
            if let Some(ref filter_selector) = filter.selector
                && &scope.selector != filter_selector
            {
                continue;
            }

            println!("  \x1b[1;35m{}\x1b[0m", scope.selector);

            // Count items in scope
            let directive_count = scope.matches.len();
            let css_count = scope.css_declarations.len();
            let each_count = scope
                .matches
                .iter()
                .filter(|m| m.macro_name == "each")
                .count();
            let nested_count = scope.nested_scopes.len();
            let mutation_count = scope
                .matches
                .iter()
                .filter(|m| m.macro_name.starts_with("on-"))
                .count();
            let elem_ref_count = scope
                .matches
                .iter()
                .filter(|m| m.macro_name == "element-ref")
                .count();
            let value_decl_count = scope
                .matches
                .iter()
                .filter(|m| {
                    m.macro_name == "local-state" || m.macro_name == "local-state-uninitialized"
                })
                .count();
            if directive_count > 0 {
                println!("    \x1b[90mdirectives:\x1b[0m {}", directive_count);
                for fm in &scope.matches {
                    let capture_keys: Vec<&str> = fm
                        .captures
                        .keys()
                        .filter(|k| !k.starts_with("__"))
                        .map(|k| k.as_str())
                        .collect();
                    println!(
                        "      - \x1b[36m@{}\x1b[0m ({})",
                        fm.macro_name,
                        if capture_keys.is_empty() {
                            "no captures".to_string()
                        } else {
                            capture_keys.join(", ")
                        }
                    );
                }
            }
            if css_count > 0 {
                println!("    \x1b[90mcss_declarations:\x1b[0m {}", css_count);
            }
            if each_count > 0 {
                println!("    \x1b[90meach_blocks:\x1b[0m {}", each_count);
            }
            if nested_count > 0 {
                println!("    \x1b[90mnested_scopes:\x1b[0m {}", nested_count);
            }
            if mutation_count > 0 {
                println!("    \x1b[90mon_mutations:\x1b[0m {}", mutation_count);
            }
            if elem_ref_count > 0 {
                println!("    \x1b[90melement_ref_decls:\x1b[0m {}", elem_ref_count);
            }
            if value_decl_count > 0 {
                println!(
                    "    \x1b[90mvalue_declarations:\x1b[0m {}",
                    value_decl_count
                );
            }

            println!();
        }
    }

    // Show file-level directives (not type/data/computed/fn, which are shown above)
    let other_directives: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.selector.is_none())
        .filter(|m| {
            m.macro_name != "type"
                && m.macro_name != "data"
                && m.macro_name != "computed"
                && m.macro_name != "fn"
        })
        .collect();
    if !other_directives.is_empty() {
        println!("\x1b[1;33mFile-level Directives:\x1b[0m");
        for fm in other_directives {
            let capture_keys: Vec<&str> = fm
                .captures
                .keys()
                .filter(|k| !k.starts_with("__"))
                .map(|k| k.as_str())
                .collect();
            println!(
                "  \x1b[36m@{}\x1b[0m ({})",
                fm.macro_name,
                if capture_keys.is_empty() {
                    "no captures".to_string()
                } else {
                    capture_keys.join(", ")
                }
            );
        }
        println!();
    }
}

/// Inspect the registry layer - show loaded primitives and macros from stdlib
fn inspect_registry(filter: &InspectFilter, format: OutputFormat) -> Result<(), String> {
    let registry = create_registry_with_stdlib();

    match format {
        OutputFormat::Json => {
            let output = build_registry_output(&registry, filter);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => {
            print_registry_pretty(&registry, filter);
        }
    }

    Ok(())
}

/// Build registry layer output structure
fn build_registry_output(registry: &MetaRegistry, filter: &InspectFilter) -> RegistryLayerOutput {
    let mut primitive_names: Vec<&str> = registry.primitive_names().collect();
    primitive_names.sort();

    let primitives: Vec<PrimitiveSummary> = primitive_names
        .iter()
        .filter(|name| {
            if let Some(ref filter_name) = filter.selector {
                name.contains(filter_name.as_str())
            } else {
                true
            }
        })
        .filter_map(|name| registry.get_primitive(name))
        .map(|prim| {
            let params: Vec<String> = prim.params.iter().map(format_primitive_param).collect();

            let js_count = prim
                .body
                .emit_blocks
                .iter()
                .filter(|e| e.lang == crate::parser::meta_ast::EmitLang::Js)
                .count();
            let css_count = prim
                .body
                .emit_blocks
                .iter()
                .filter(|e| e.lang == crate::parser::meta_ast::EmitLang::Css)
                .count();

            let exports: Vec<String> = prim
                .body
                .exports
                .iter()
                .map(|e| format!("${}: {}", e.name, e.type_expr.format()))
                .collect();

            PrimitiveSummary {
                name: prim.name.clone(),
                params,
                js_emit_count: js_count,
                css_emit_count: css_count,
                exports,
                doc: prim.doc.clone(),
            }
        })
        .collect();

    let mut macro_names: Vec<&str> = registry.macro_names().collect();
    macro_names.sort();

    let macros: Vec<MacroDefSummary> = macro_names
        .iter()
        .filter(|name| {
            if let Some(ref filter_name) = filter.selector {
                name.contains(filter_name.as_str())
            } else {
                true
            }
        })
        .filter_map(|name| registry.get_macro(name))
        .map(|mac| {
            let binds: Vec<String> = mac.binds.iter().map(|b| b.primitive.clone()).collect();

            let scopes: Vec<String> = mac.scopes.iter().map(format_macro_scope).collect();

            MacroDefSummary {
                name: mac.name.clone(),
                form: mac.form.as_ref().map(|f| f.directive_name.clone()),
                binds,
                derives_count: mac.derives.len(),
                has_states: mac.states.is_some(),
                scopes,
                doc: mac.doc.clone(),
            }
        })
        .collect();

    RegistryLayerOutput {
        layer: "registry".to_string(),
        summary: RegistrySummary {
            primitives: primitives.len(),
            macros: macros.len(),
        },
        primitives,
        macros,
    }
}

/// Format primitive parameter for display
fn format_primitive_param(p: &crate::parser::meta_ast::PrimitiveParam) -> String {
    match p {
        crate::parser::meta_ast::PrimitiveParam::Element(e) => format!("&{}", e),
        crate::parser::meta_ast::PrimitiveParam::Data(d) => format!("${}", d),
        crate::parser::meta_ast::PrimitiveParam::TypedData { name, ty } => {
            format!("${}: {}", name, ty)
        }
        crate::parser::meta_ast::PrimitiveParam::Typed { name, ty, default } => {
            let type_str = match ty {
                crate::parser::meta_ast::ParamType::Simple(s) => s.clone(),
                crate::parser::meta_ast::ParamType::Union(vals) => {
                    format!("({})", vals.join(" | "))
                }
                crate::parser::meta_ast::ParamType::Array(s) => format!("{}[]", s),
                crate::parser::meta_ast::ParamType::Optional(s) => format!("{}?", s),
                crate::parser::meta_ast::ParamType::OptionalArray(s) => format!("{}[]?", s),
            };
            if default.is_some() {
                format!("{}: {} = ...", name, type_str)
            } else {
                format!("{}: {}", name, type_str)
            }
        }
    }
}

/// Format macro scope for display
fn format_macro_scope(s: &crate::parser::meta_ast::MacroScope) -> String {
    s.as_str().to_string()
}

// =============================================================================
// Dispatch layer (FEAT-087)
// =============================================================================

fn inspect_dispatch(filter: &InspectFilter, format: OutputFormat) -> Result<(), String> {
    let registry = create_registry_with_stdlib();
    let output = build_dispatch_output(&registry, filter);
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => print_dispatch_pretty(&output),
    }
    Ok(())
}

fn overload_of(mac: &crate::parser::meta_ast::MacroDefAst) -> DispatchOverload {
    // Canonical signature rendering lives in metasystem::signature (FEAT-089), so the
    // dispatch catalog and the @data dispatch source render identically.
    let signature = mac
        .form
        .as_ref()
        .map(crate::metasystem::signature::format_form_signature)
        .unwrap_or_else(|| mac.name.clone());
    DispatchOverload {
        name: mac.name.clone(),
        signature,
        scopes: mac.scopes.iter().map(format_macro_scope).collect(),
        binds: mac.binds.iter().map(|b| b.primitive.clone()).collect(),
        doc: mac
            .doc
            .as_ref()
            .map(|d| d.lines().next().unwrap_or("").trim().to_string()),
    }
}

fn build_dispatch_output(registry: &MetaRegistry, filter: &InspectFilter) -> DispatchLayerOutput {
    use std::collections::BTreeMap;

    // Group every %form-carrying macro under its directive.
    let mut by_directive: BTreeMap<String, Vec<DispatchOverload>> = BTreeMap::new();
    let mut names: Vec<&str> = registry.macro_names().collect();
    names.sort();
    for name in names {
        let Some(mac) = registry.get_macro(name) else {
            continue;
        };
        let Some(form) = &mac.form else { continue };
        let dir = if form.directive_name.starts_with('@') {
            form.directive_name.clone()
        } else {
            format!("@{}", form.directive_name)
        };
        if let Some(ref f) = filter.selector
            && !dir.contains(f.as_str())
            && !mac.name.contains(f.as_str())
        {
            continue;
        }
        by_directive.entry(dir).or_default().push(overload_of(mac));
    }

    let directives: Vec<DispatchDirective> = by_directive
        .into_iter()
        .map(|(directive, overloads)| DispatchDirective {
            directive,
            overloads,
        })
        .collect();

    let probe = filter
        .probe
        .as_ref()
        .map(|call| build_dispatch_probe(registry, call));

    DispatchLayerOutput {
        layer: "dispatch".to_string(),
        directives,
        probe,
    }
}

/// Resolve a probed call across the candidate overloads for its directive,
/// returning the full scored table (FEAT-087). Reuses the SAME explainer the
/// compiler's scorer delegates to — no reimplementation. Public so the dev
/// server's `/__spacetime/dispatch` endpoint (FEAT-091) shares this exact logic.
pub fn build_dispatch_probe(registry: &MetaRegistry, call: &str) -> DispatchProbe {
    use crate::syntax::STDLIB_REGISTRY;
    use crate::syntax::form_scoring::{ProbeInput, explain_form_match, explain_macro_form};

    // Parse the call into a FormMatch using the global syntax registry, so the
    // captures the scorer reads are the real ones the parser would produce.
    let (matches, _diags) = crate::syntax::events::parse_matches(call, &STDLIB_REGISTRY);
    let Some(fm) = matches.into_iter().next() else {
        return DispatchProbe {
            call: call.to_string(),
            directive: None,
            candidates: Vec::new(),
            ambiguous: false,
            margin: None,
            error: Some("no directive matched".to_string()),
        };
    };

    // Derive the directive the call dispatches under: prefer the matched macro's
    // own form directive (authoritative), else the match's macro_name.
    let directive = fm
        .matched_macro
        .as_deref()
        .and_then(|m| registry.get_macro(m))
        .and_then(|m| m.form.as_ref())
        .map(|f| f.directive_name.clone())
        .or_else(|| {
            registry
                .get_macro(&fm.macro_name)
                .and_then(|m| m.form.as_ref())
                .map(|f| f.directive_name.clone())
        })
        .unwrap_or_else(|| fm.macro_name.clone());

    let candidates_macros = registry.get_all_macros_by_form_directive(&directive);

    // Score each candidate with a literal-aware ProbeInput built for ITS form, so a
    // keyword literal scores +3 only when it appears in the call text — a faithful,
    // non-misleading resolution table (FEAT-087). Formless macros fall back to the
    // capture-only explainer.
    let mut candidates: Vec<DispatchCandidate> = candidates_macros
        .iter()
        .map(|mac| {
            let ex = match &mac.form {
                Some(form) => {
                    let input = ProbeInput::new(&fm.captures, form, call);
                    explain_form_match(form, &input)
                }
                None => explain_macro_form(
                    mac,
                    &ProbeInput::new(&fm.captures, &Default::default(), call),
                ),
            };
            DispatchCandidate {
                macro_name: mac.name.clone(),
                total: ex.total,
                winner: false,
                terms: ex.terms,
            }
        })
        .collect();

    // Winner = max viable total (None sorts lowest). Stable: first max wins,
    // mirroring find_macro_for_match's max_by_key.
    let best_idx = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.total.map(|t| (i, t)))
        .max_by_key(|(_, t)| *t)
        .map(|(i, _)| i);

    // Margin + ambiguity from the top two viable totals.
    let mut viable: Vec<u32> = candidates.iter().filter_map(|c| c.total).collect();
    viable.sort_unstable_by(|a, b| b.cmp(a));
    let (ambiguous, margin) = match viable.as_slice() {
        [top, second, ..] => (top == second, Some(top - second)),
        _ => (false, None),
    };

    if let Some(i) = best_idx {
        candidates[i].winner = true;
    }
    // Sort for display: winner first, then by total desc (None last), then name.
    candidates.sort_by(|a, b| {
        b.winner
            .cmp(&a.winner)
            .then(b.total.unwrap_or(0).cmp(&a.total.unwrap_or(0)))
            .then(a.total.is_none().cmp(&b.total.is_none()))
            .then(a.macro_name.cmp(&b.macro_name))
    });

    DispatchProbe {
        call: call.to_string(),
        directive: Some(if directive.starts_with('@') {
            directive
        } else {
            format!("@{directive}")
        }),
        candidates,
        ambiguous,
        margin,
        error: None,
    }
}

fn print_dispatch_pretty(out: &DispatchLayerOutput) {
    println!(
        "\x1b[1mDispatch catalog\x1b[0m — {} directives\n",
        out.directives.len()
    );
    for d in &out.directives {
        let n = d.overloads.len();
        println!(
            "\x1b[36m{}\x1b[0m ▸ {} overload{}",
            d.directive,
            n,
            if n == 1 { "" } else { "s" }
        );
        for o in &d.overloads {
            println!("  \x1b[90m{:<22}\x1b[0m {}", o.name, o.signature);
            if let Some(doc) = &o.doc
                && !doc.is_empty()
            {
                println!("  {:<22} \x1b[2m{}\x1b[0m", "", doc);
            }
        }
        println!();
    }
    if let Some(p) = &out.probe {
        println!("\x1b[1mProbe\x1b[0m  {}", p.call);
        if let Some(err) = &p.error {
            println!("  \x1b[31m✗\x1b[0m {err}");
            return;
        }
        if let Some(dir) = &p.directive {
            println!("  directive: {dir}");
        }
        for c in &p.candidates {
            let mark = if c.winner {
                "\x1b[32m✓\x1b[0m"
            } else if c.total.is_none() {
                "\x1b[31m✗\x1b[0m"
            } else {
                " "
            };
            let score = match c.total {
                Some(t) => format!("+{t}"),
                None => "∅".to_string(),
            };
            let breakdown: Vec<String> = c
                .terms
                .iter()
                .map(|t| {
                    // Surface the verdict for anything that is not a plain match, so a
                    // reject reason (missing / type-mismatch / literal-miss) is visible.
                    if t.verdict == "matched" {
                        format!("{} {:+}", t.element_label, t.points)
                    } else {
                        format!("{} {:+} ⟨{}⟩", t.element_label, t.points, t.verdict)
                    }
                })
                .collect();
            println!(
                "  {mark} \x1b[1m{:>4}\x1b[0m {:<22} \x1b[2m{}\x1b[0m",
                score,
                c.macro_name,
                breakdown.join(" · ")
            );
        }
        if p.ambiguous {
            println!("  \x1b[33m⚠ ambiguous\x1b[0m — top two candidates tie");
        } else if let Some(m) = p.margin {
            println!("  won by +{m}");
        }
    }
}

/// Print registry layer in pretty format
fn print_registry_pretty(registry: &MetaRegistry, filter: &InspectFilter) {
    println!("\n\x1b[1;36m=== Registry Layer ===\x1b[0m");
    println!("\x1b[90mShowing loaded primitives and macros from stdlib\x1b[0m\n");

    // Show summary
    println!("\x1b[1;33mRegistry Summary:\x1b[0m");
    println!(
        "  \x1b[32mprimitives:\x1b[0m {}",
        registry.primitive_count()
    );
    println!("  \x1b[32mmacros:\x1b[0m {}", registry.macro_count());
    println!();

    // Collect and sort primitive names
    let mut primitive_names: Vec<&str> = registry.primitive_names().collect();
    primitive_names.sort();

    // Show primitives
    println!("\x1b[1;33mPrimitives:\x1b[0m");
    for name in &primitive_names {
        // Apply filter if specified (filter.selector can match primitive name)
        if let Some(ref filter_name) = filter.selector
            && !name.contains(filter_name.as_str())
        {
            continue;
        }

        if let Some(prim) = registry.get_primitive(name) {
            // Format parameters
            let params: Vec<String> = prim.params.iter().map(format_primitive_param).collect();

            // Count emit blocks
            let js_count = prim
                .body
                .emit_blocks
                .iter()
                .filter(|e| e.lang == crate::parser::meta_ast::EmitLang::Js)
                .count();
            let css_count = prim
                .body
                .emit_blocks
                .iter()
                .filter(|e| e.lang == crate::parser::meta_ast::EmitLang::Css)
                .count();
            let export_count = prim.body.exports.len();

            println!(
                "  \x1b[35m%primitive\x1b[0m \x1b[1m{}\x1b[0m({})",
                name,
                params.join(", ")
            );
            if js_count > 0 || css_count > 0 {
                println!("    \x1b[90memits:\x1b[0m {}js, {}css", js_count, css_count);
            }
            if export_count > 0 {
                let exports: Vec<String> = prim
                    .body
                    .exports
                    .iter()
                    .map(|e| format!("${}: {}", e.name, e.type_expr.format()))
                    .collect();
                println!("    \x1b[90mexports:\x1b[0m {{ {} }}", exports.join(", "));
            }
        }
    }
    println!();

    // Collect and sort macro names
    let mut macro_names: Vec<&str> = registry.macro_names().collect();
    macro_names.sort();

    // Show macros
    println!("\x1b[1;33mMacros:\x1b[0m");
    for name in &macro_names {
        // Apply filter if specified
        if let Some(ref filter_name) = filter.selector
            && !name.contains(filter_name.as_str())
        {
            continue;
        }

        if let Some(mac) = registry.get_macro(name) {
            println!("  \x1b[35m%macro\x1b[0m \x1b[1m{}\x1b[0m", name);

            // Show form if present
            if let Some(ref form) = mac.form {
                println!("    \x1b[90mform:\x1b[0m {}", form.directive_name);
            }

            // Show binds count
            if !mac.binds.is_empty() {
                let primitives: Vec<&str> =
                    mac.binds.iter().map(|b| b.primitive.as_str()).collect();
                println!("    \x1b[90mbinds:\x1b[0m {}", primitives.join(", "));
            }

            // Show derives count
            if !mac.derives.is_empty() {
                println!(
                    "    \x1b[90mderives:\x1b[0m {} variables",
                    mac.derives.len()
                );
            }

            // Show states if present
            if mac.states.is_some() {
                println!("    \x1b[90mstates:\x1b[0m defined");
            }

            // Show scopes if present
            if !mac.scopes.is_empty() {
                let scope_strs: Vec<String> = mac.scopes.iter().map(format_macro_scope).collect();
                println!("    \x1b[90mscopes:\x1b[0m {}", scope_strs.join(", "));
            }
        }
    }
    println!();
}

/// Inspect the IR layer - show typed ExpandedPrimitives from the pipeline
fn inspect_ir(ast: &StFile, _filter: &InspectFilter, format: OutputFormat) -> Result<(), String> {
    use crate::compiler::load_stdlib_registry;
    use crate::metasystem::MetaRegistryErrorKind;
    use crate::pipeline::{self, CompileContext};

    let (mut registry, stdlib_errors) = load_stdlib_registry();
    if !stdlib_errors.is_empty() {
        return Err(format!(
            "Stdlib load errors: {:?}",
            stdlib_errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        ));
    }

    // Register user-defined primitives and macros from the AST
    for def in ast.meta_defs.clone() {
        match registry.register(def) {
            Ok(()) => {}
            Err(e) => match &e.kind {
                MetaRegistryErrorKind::DuplicatePrimitive(_)
                | MetaRegistryErrorKind::DuplicateMacro(_) => {
                    // Already loaded from stdlib — skip silently
                }
                _ => {
                    return Err(format!("User meta def error: {:?}", e.kind));
                }
            },
        }
    }

    let context = CompileContext::new(registry);
    let verbose = pipeline::compile_verbose(&ast.matches, &context)
        .map_err(|e| format!("Pipeline error: {:?}", e))?;

    match format {
        OutputFormat::Json => {
            let output = build_ir_output_from_expanded(&verbose.expanded);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => {
            print_ir_pretty_from_expanded(&verbose.expanded);
        }
    }

    Ok(())
}

/// Build IR layer output from typed ExpandedPrimitives
fn build_ir_output_from_expanded(expanded: &[crate::pipeline::ExpandedPrimitive]) -> IrLayerOutput {
    use crate::emit::{EmitOptions, css as css_emit, js as js_emit};
    let opts = EmitOptions::pretty();

    let mut js_stmts = Vec::new();
    let mut css_exprs = Vec::new();
    let mut js_raw_count = 0;
    let mut js_structured_count = 0;
    let mut css_raw_count = 0;
    let mut css_structured_count = 0;

    for (i, ep) in expanded.iter().enumerate() {
        if let Some(js_frag) = &ep.js {
            for stmt in &js_frag.stmts {
                let preview = if let Ok(s) = js_emit::emit_stmts(&[stmt.clone()], &opts) {
                    truncate_preview(&s, 80)
                } else {
                    format!("{:?}", stmt)
                };
                let kind = match stmt {
                    crate::ir::JsStmt::Raw(_) => {
                        js_raw_count += 1;
                        "Raw"
                    }
                    _ => {
                        js_structured_count += 1;
                        "Structured"
                    }
                };
                js_stmts.push(JsStmtSummary {
                    kind: format!("Expanded[{}]::{}", i, kind),
                    preview,
                });
            }
        }
        if let Some(css_frag) = &ep.css {
            for expr in &css_frag.exprs {
                let preview = truncate_preview(&css_emit::emit_all(&[expr.clone()], &opts), 80);
                let kind = match expr {
                    crate::ir::CssExpr::Raw(_) => {
                        css_raw_count += 1;
                        "Raw"
                    }
                    _ => {
                        css_structured_count += 1;
                        "Structured"
                    }
                };
                css_exprs.push(CssExprSummary {
                    kind: format!("Expanded[{}]::{}", i, kind),
                    preview,
                });
            }
        }
    }

    IrLayerOutput {
        layer: "ir".to_string(),
        summary: IrSummary {
            js_stmt_count: js_stmts.len(),
            css_expr_count: css_exprs.len(),
            js_raw_count,
            css_raw_count,
            js_structured_count,
            css_structured_count,
        },
        js_stmts,
        css_exprs,
    }
}

/// Truncate a string for preview display
fn truncate_preview(s: &str, max_len: usize) -> String {
    let first_line = s.lines().next().unwrap_or("");
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len])
    } else if s.lines().count() > 1 {
        format!("{}... ({} lines)", first_line, s.lines().count())
    } else {
        first_line.to_string()
    }
}

/// Print IR layer in pretty format using typed ExpandedPrimitives
fn print_ir_pretty_from_expanded(expanded: &[crate::pipeline::ExpandedPrimitive]) {
    use crate::emit::{EmitOptions, css as css_emit, js as js_emit};
    let opts = EmitOptions::pretty();

    println!("\n\x1b[1;36m=== IR Layer (Typed Fragments) ===\x1b[0m");
    println!("\x1b[90mExpandedPrimitives from expand stage, before final emission\x1b[0m\n");

    let total_js: usize = expanded
        .iter()
        .filter_map(|e| e.js.as_ref())
        .map(|j| j.stmts.len())
        .sum();
    let total_css: usize = expanded
        .iter()
        .filter_map(|e| e.css.as_ref())
        .map(|c| c.exprs.len())
        .sum();
    let total_cleanup: usize = expanded
        .iter()
        .filter_map(|e| e.js.as_ref())
        .map(|j| j.cleanup.len())
        .sum();

    println!("\x1b[1;33mSummary:\x1b[0m");
    println!("  \x1b[32mPrimitives:\x1b[0m {}", expanded.len());
    println!("  \x1b[32mJS stmts:\x1b[0m {}", total_js);
    println!("  \x1b[32mCSS exprs:\x1b[0m {}", total_css);
    println!("  \x1b[32mCleanup stmts:\x1b[0m {}", total_cleanup);
    println!();

    for (i, ep) in expanded.iter().enumerate() {
        println!("\x1b[1;33mExpanded [{}]:\x1b[0m", i);
        if let Some(js_frag) = &ep.js {
            println!("  \x1b[36mscope:\x1b[0m {:?}", js_frag.scope);
            if let Some(el_init) = &js_frag.el_init {
                println!("  \x1b[36mel_init:\x1b[0m {:?}", el_init);
            }
            for (j, stmt) in js_frag.stmts.iter().enumerate() {
                let preview = if let Ok(s) = js_emit::emit_stmts(&[stmt.clone()], &opts) {
                    truncate_preview(&s, 80)
                } else {
                    format!("{:?}", stmt)
                };
                println!("  \x1b[35mJS[{}]\x1b[0m \x1b[90m{}\x1b[0m", j, preview);
            }
            for (j, cl) in js_frag.cleanup.iter().enumerate() {
                let preview = if let Ok(s) = js_emit::emit_stmts(&[cl.clone()], &opts) {
                    truncate_preview(&s, 80)
                } else {
                    format!("{:?}", cl)
                };
                println!("  \x1b[35mCleanup[{}]\x1b[0m \x1b[90m{}\x1b[0m", j, preview);
            }
        }
        if let Some(css_frag) = &ep.css {
            for (j, expr) in css_frag.exprs.iter().enumerate() {
                let preview = truncate_preview(&css_emit::emit_all(&[expr.clone()], &opts), 80);
                println!("  \x1b[35mCSS[{}]\x1b[0m \x1b[90m{}\x1b[0m", j, preview);
            }
        }
    }
    println!();
}

/// Inspect the keyframes layer - show captured animation properties
fn inspect_keyframes(
    ast: &StFile,
    filter: &InspectFilter,
    format: OutputFormat,
) -> Result<(), String> {
    match format {
        OutputFormat::Json => {
            let output = build_keyframes_output(ast, filter);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => {
            print_keyframes_pretty(ast, filter);
        }
    }

    Ok(())
}

/// Build keyframes layer output structure
fn build_keyframes_output(ast: &StFile, filter: &InspectFilter) -> KeyframesLayerOutput {
    let mut timelines = Vec::new();

    for scope in &ast.scopes {
        if let Some(ref filter_selector) = filter.selector
            && &scope.selector != filter_selector
        {
            continue;
        }

        for fm in &scope.matches {
            let is_timeline = matches!(
                fm.macro_name.as_str(),
                "scroll"
                    | "time"
                    | "loop"
                    | "hover"
                    | "on-visible"
                    | "load"
                    | "click"
                    | "mouse"
                    | "value-change"
                    | "after"
                    | "sequence"
            );

            if !is_timeline {
                continue;
            }

            let timeline_name = fm
                .get_ident("name")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unnamed".to_string());

            if let Some(ref filter_timeline) = filter.timeline
                && &timeline_name != filter_timeline
            {
                continue;
            }

            let mut properties = Vec::new();

            // Extract keyframes from the body capture
            if let Some(keyframes) = fm.get_keyframes("body") {
                for kf in keyframes {
                    if let Some((from, to)) = parse_animation_property(&kf.values.join(" -> ")) {
                        properties.push(PropertySummary {
                            name: kf.property.clone(),
                            from: Some(from),
                            to: Some(to),
                            raw_value: None,
                        });
                    } else {
                        properties.push(PropertySummary {
                            name: kf.property.clone(),
                            from: None,
                            to: None,
                            raw_value: Some(kf.values.join(" -> ").clone()),
                        });
                    }
                }
            }

            timelines.push(TimelineSummary {
                name: timeline_name,
                timeline_type: fm.macro_name.clone(),
                selector: scope.selector.clone(),
                properties,
                nested_scopes: Vec::new(),
            });
        }
    }

    KeyframesLayerOutput {
        layer: "keyframes".to_string(),
        timelines,
    }
}

/// Print keyframes layer in pretty format
fn print_keyframes_pretty(ast: &StFile, filter: &InspectFilter) {
    println!("\n\x1b[1;36m=== Keyframes Layer ===\x1b[0m");
    println!("\x1b[90mShowing captured $body:keyframes structure\x1b[0m\n");

    let mut found_any = false;

    // Iterate through scopes to find @scroll and other timeline macros
    for scope in &ast.scopes {
        // Apply selector filter
        if let Some(ref filter_selector) = filter.selector
            && &scope.selector != filter_selector
        {
            continue;
        }

        // Look for timeline directives in FormMatches
        for fm in &scope.matches {
            let is_timeline = matches!(
                fm.macro_name.as_str(),
                "scroll"
                    | "time"
                    | "loop"
                    | "hover"
                    | "on-visible"
                    | "load"
                    | "click"
                    | "mouse"
                    | "value-change"
                    | "after"
                    | "sequence"
            );

            if !is_timeline {
                continue;
            }

            let timeline_name = fm
                .get_ident("name")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unnamed".to_string());

            // Apply timeline filter
            if let Some(ref filter_timeline) = filter.timeline
                && &timeline_name != filter_timeline
            {
                continue;
            }

            found_any = true;

            // Print timeline header
            println!("\x1b[1;33m@{} {}:\x1b[0m", fm.macro_name, timeline_name);
            println!("  \x1b[90mSelector:\x1b[0m {}", scope.selector);

            // Extract keyframes from body capture
            if let Some(keyframes) = fm.get_keyframes("body") {
                if !keyframes.is_empty() {
                    println!("  \x1b[32mproperties:\x1b[0m");
                    for kf in keyframes {
                        if let Some((from, to)) = parse_animation_property(&kf.values.join(" -> "))
                        {
                            println!("    - \x1b[36m{}\x1b[0m: {} -> {}", kf.property, from, to);
                        } else {
                            println!(
                                "    - \x1b[36m{}\x1b[0m: {}",
                                kf.property,
                                kf.values.join(" -> ")
                            );
                        }
                    }
                } else {
                    println!("  \x1b[90mproperties: []\x1b[0m");
                }
            } else {
                println!("  \x1b[90m(no body)\x1b[0m");
            }

            println!();
        }
    }

    if !found_any {
        println!("\x1b[33mNo timelines found matching filters\x1b[0m");
    }
}

/// Parse animation property value to extract from -> to
fn parse_animation_property(value: &str) -> Option<(String, String)> {
    // Look for " -> " separator
    if let Some(pos) = value.find("->") {
        let from = value[..pos].trim().to_string();
        let to = value[pos + 2..].trim().to_string();
        Some((from, to))
    } else {
        None
    }
}

/// Inspect the expansion layer - show macro expansion to %binds/%derives/%states
fn inspect_expansion(
    ast: &StFile,
    filter: &InspectFilter,
    format: OutputFormat,
) -> Result<(), String> {
    match format {
        OutputFormat::Json => {
            let output = build_expansion_output(ast, filter);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => {
            print_expansion_pretty(ast, filter);
        }
    }

    Ok(())
}

/// Build expansion layer output structure
fn build_expansion_output(ast: &StFile, filter: &InspectFilter) -> ExpansionLayerOutput {
    let registry = create_registry_with_stdlib();
    let mut expansions = Vec::new();

    for scope in &ast.scopes {
        if let Some(ref filter_selector) = filter.selector
            && &scope.selector != filter_selector
        {
            continue;
        }

        for fm in &scope.matches {
            if let Some(ref filter_timeline) = filter.timeline {
                let timeline_name = fm
                    .get_ident("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed".to_string());

                if &timeline_name != filter_timeline {
                    continue;
                }
            }

            let directive_summary = summarize_directive(fm);

            // BUG-371: honour the macro PARSE already selected. `macro_name` is the
            // directive (`on`), which several `%macro` siblings share — `on-driver-body`,
            // `on-driver-form-as-invalid`, `on-driver-form-legacy-space`. Resolving by
            // directive alone re-runs a scorer that is blind to the literals which
            // settled it, and the fallback iterates a HashMap: the reported primitive
            // flipped between `drive`, `drive-body`, `drive-arms`, `drive-legacy-space`
            // and the ERROR sibling `drive-as-error` across identical runs of one file.
            // `matched_macro` is deterministic and authoritative; compile already uses it
            // (`find_macro_for_match`'s `preferred`), which is why emit stayed stable
            // while inspect lied.
            let expands_to = registry
                .get_macro_for_match(fm.matched_macro.as_deref(), &fm.macro_name)
                .map(|macro_def| {
                    let binds: Vec<BindSummary> = macro_def
                        .binds
                        .iter()
                        .map(|binding| {
                            let outputs: Vec<String> = binding
                                .outputs
                                .iter()
                                .map(|o| {
                                    if let Some(ref alias) = o.alias {
                                        format!("{} as {}", o.name, alias)
                                    } else {
                                        o.name.clone()
                                    }
                                })
                                .collect();

                            BindSummary {
                                primitive: binding.primitive.clone(),
                                arg_count: binding.args.len(),
                                outputs,
                            }
                        })
                        .collect();

                    let derives: Vec<DeriveSummary> = macro_def
                        .derives
                        .iter()
                        .map(|d| DeriveSummary {
                            name: d.name.clone(),
                            expression: d.expr.clone(),
                        })
                        .collect();

                    ExpansionDetail {
                        binds,
                        derives,
                        has_states: macro_def.states.is_some(),
                    }
                });

            expansions.push(ExpansionSummary {
                selector: scope.selector.clone(),
                macro_call: directive_summary,
                expands_to,
            });
        }
    }

    ExpansionLayerOutput {
        layer: "expansion".to_string(),
        expansions,
    }
}

/// Print expansion layer in pretty format
fn print_expansion_pretty(ast: &StFile, filter: &InspectFilter) {
    println!("\n\x1b[1;36m=== Expansion Layer ===\x1b[0m");
    println!("\x1b[90mShowing macro expansion to %binds/%derives/%states\x1b[0m\n");

    let registry = create_registry_with_stdlib();
    let mut found_any = false;

    // Iterate through scopes
    for scope in &ast.scopes {
        // Apply selector filter
        if let Some(ref filter_selector) = filter.selector
            && &scope.selector != filter_selector
        {
            continue;
        }

        let mut scope_has_directives = false;

        // Look for directives in FormMatches
        for fm in &scope.matches {
            // Apply timeline filter if specified
            if let Some(ref filter_timeline) = filter.timeline {
                let timeline_name = fm
                    .get_ident("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed".to_string());

                if &timeline_name != filter_timeline {
                    continue;
                }
            }

            if !scope_has_directives {
                println!("\x1b[1;33m{}:\x1b[0m", scope.selector);
                scope_has_directives = true;
                found_any = true;
            }

            // Show the directive with its captures
            let capture_keys: Vec<String> = fm
                .captures
                .keys()
                .filter(|k| !k.starts_with("__"))
                .map(|k| {
                    format!(
                        "{}: {}",
                        k,
                        format_captured_value(fm.captures.get(k).unwrap())
                    )
                })
                .collect();
            println!(
                "  \x1b[36m@{}\x1b[0m({})",
                fm.macro_name,
                if capture_keys.is_empty() {
                    String::new()
                } else {
                    capture_keys.join(", ")
                }
            );

            // Try to expand the macro - first by name, then by form directive
            if let Some(macro_def) = registry
                .get_macro(&fm.macro_name)
                .or_else(|| registry.get_macro_by_form_directive(&fm.macro_name))
            {
                println!("    \x1b[90m-> Expands to:\x1b[0m");

                for binding in &macro_def.binds {
                    let primitive_name = &binding.primitive;
                    let binding_args = if binding.args.is_empty() {
                        String::new()
                    } else {
                        format!("{} args", binding.args.len())
                    };

                    let outputs = if !binding.outputs.is_empty() {
                        let output_names: Vec<String> = binding
                            .outputs
                            .iter()
                            .map(|o| {
                                if let Some(ref alias) = o.alias {
                                    format!("{} as {}", o.name, alias)
                                } else {
                                    o.name.clone()
                                }
                            })
                            .collect();
                        format!(" -> {{ {} }}", output_names.join(", "))
                    } else {
                        String::new()
                    };

                    println!(
                        "      \x1b[35m%binds:\x1b[0m {}({}){}",
                        primitive_name, binding_args, outputs
                    );
                }

                for derive in &macro_def.derives {
                    println!(
                        "      \x1b[35m%derives:\x1b[0m ${} = {}",
                        derive.name, derive.expr
                    );
                }

                if let Some(ref _states) = macro_def.states {
                    println!("      \x1b[35m%states:\x1b[0m [states defined]");
                }
            } else {
                println!("    \x1b[90m(macro not found in registry)\x1b[0m");
            }

            println!();
        }

        if scope_has_directives {
            println!();
        }
    }

    if !found_any {
        println!("\x1b[33mNo macro expansions found matching filters\x1b[0m");
    }
}

/// Inspect the emit layer - show final emitted JS/CSS code
fn inspect_emit(
    ast: &StFile,
    filter: &InspectFilter,
    _content: &str,
    format: OutputFormat,
) -> Result<(), String> {
    // Compile through the pipeline
    let compiled = Compiler::from_ast(ast).compile();

    match format {
        OutputFormat::Json => {
            let output = EmitLayerOutput {
                layer: "emit".to_string(),
                css: EmitOutput {
                    size_bytes: compiled.css.len(),
                    content: compiled.css.clone(),
                },
                js: EmitOutput {
                    size_bytes: compiled.js.len(),
                    content: compiled.js.clone(),
                },
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Pretty => {
            print_emit_pretty(&compiled, filter);
        }
    }

    Ok(())
}

/// Print emit layer in pretty format
fn print_emit_pretty(compiled: &crate::compiler::CompiledSpacetime, filter: &InspectFilter) {
    println!("\n\x1b[1;36m=== Emit Layer ===\x1b[0m");
    println!("\x1b[90mShowing emitted JS/CSS code\x1b[0m\n");

    let mut found_any = false;

    if let Some(ref selector_filter) = filter.selector {
        println!(
            "\x1b[1;33mFiltered for selector: {}\x1b[0m\n",
            selector_filter
        );

        // Show CSS for this selector
        println!("\x1b[32m--- CSS for {} ---\x1b[0m", selector_filter);
        for line in compiled.css.lines() {
            if line.contains(selector_filter) || line.contains("--st-") {
                println!("{}", line);
                found_any = true;
            }
        }
        println!();

        // Show JS for this selector (approximate matching)
        println!("\x1b[32m--- JS for {} ---\x1b[0m", selector_filter);
        let selector_slug = selector_filter.trim_start_matches('.').replace('.', "-");
        for line in compiled.js.lines() {
            if line.contains(selector_filter) || line.contains(&selector_slug) {
                println!("{}", line);
                found_any = true;
            }
        }
        println!();
    } else if let Some(ref timeline_filter) = filter.timeline {
        println!(
            "\x1b[1;33mFiltered for timeline: {}\x1b[0m\n",
            timeline_filter
        );

        // Show CSS for this timeline
        println!(
            "\x1b[32m--- CSS for timeline {} ---\x1b[0m",
            timeline_filter
        );
        let timeline_var = format!("--st-{}", timeline_filter);
        for line in compiled.css.lines() {
            if line.contains(&timeline_var) {
                println!("{}", line);
                found_any = true;
            }
        }
        println!();

        // Show JS for this timeline
        println!("\x1b[32m--- JS for timeline {} ---\x1b[0m", timeline_filter);
        for line in compiled.js.lines() {
            if line.contains(timeline_filter) {
                println!("{}", line);
                found_any = true;
            }
        }
        println!();
    } else {
        // Show full output
        println!(
            "\x1b[32m--- Full CSS Output ({} bytes) ---\x1b[0m",
            compiled.css.len()
        );
        println!("{}", compiled.css);
        println!();

        println!(
            "\x1b[32m--- Full JS Output ({} bytes) ---\x1b[0m",
            compiled.js.len()
        );
        println!("{}", compiled.js);
        println!();

        found_any = true;
    }

    if !found_any {
        println!("\x1b[33mNo emitted code found matching filters\x1b[0m");
    }
}

/// Inspect the scopes layer - show compile-time scope tree (PROJ-102)
fn inspect_scopes(
    ast: &StFile,
    _filter: &InspectFilter,
    format: OutputFormat,
) -> Result<(), String> {
    let scope_tree = build_scope_tree(&ast.matches, &ast.scopes);

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&scope_tree).unwrap());
        }
        OutputFormat::Pretty => {
            print!("{}", scope_tree.display_pretty());
        }
    }

    Ok(())
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;

    fn registry() -> MetaRegistry {
        create_registry_with_stdlib()
    }

    #[test]
    fn catalog_groups_data_overloads_under_directive() {
        let reg = registry();
        let filter = InspectFilter {
            selector: Some("@data".to_string()),
            ..Default::default()
        };
        let out = build_dispatch_output(&reg, &filter);
        let data = out
            .directives
            .iter()
            .find(|d| d.directive == "@data")
            .expect("@data directive present");
        // The canonical data overloads must all appear under @data.
        let names: Vec<&str> = data.overloads.iter().map(|o| o.name.as_str()).collect();
        for expected in [
            "data-inline",
            "data-fetch-kind",
            "data-derive",
            "data-query",
        ] {
            assert!(
                names.contains(&expected),
                "missing overload {expected} in {names:?}"
            );
        }
        // Signatures carry the directive + a discriminating keyword.
        let inline = data
            .overloads
            .iter()
            .find(|o| o.name == "data-inline")
            .unwrap();
        assert!(inline.signature.contains("@data"));
        assert!(inline.signature.contains("inline"));
    }

    #[test]
    fn probe_picks_fetch_winner_with_breakdown() {
        let reg = registry();
        let probe = build_dispatch_probe(&reg, "@data fetch $api : \"/users\";");
        assert_eq!(probe.directive.as_deref(), Some("@data"));
        let winner = probe
            .candidates
            .iter()
            .find(|c| c.winner)
            .expect("a winner");
        assert_eq!(winner.macro_name, "data-fetch-kind");
        // data-fetch-kind beats the inline/derive overloads (they hard-reject).
        let inline = probe
            .candidates
            .iter()
            .find(|c| c.macro_name == "data-inline")
            .unwrap();
        assert_eq!(
            inline.total, None,
            "data-inline must hard-reject (no $value)"
        );
        // The winner's `fetch` literal scored as a real match.
        assert!(
            winner
                .terms
                .iter()
                .any(|t| t.element_label.contains("fetch") && t.verdict == "matched")
        );
        // A losing overload's wrong keyword is recorded as a literal-miss (honest).
        assert!(
            inline
                .terms
                .iter()
                .any(|t| t.element_label.contains("inline") && t.verdict == "literal-miss")
        );
    }

    #[test]
    fn probe_garbage_call_reports_error_not_panic() {
        let reg = registry();
        let probe = build_dispatch_probe(&reg, "zzz nonsense");
        assert!(probe.directive.is_none());
        assert!(probe.candidates.is_empty());
        assert!(probe.error.is_some());
    }
}
