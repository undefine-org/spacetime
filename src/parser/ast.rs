//! LEAN AST types for Spacetime DSL
//!
//! This module contains syntactic AST types that capture SYNTAX only, not SEMANTICS.
//! These types are generic and not directive-specific.
//!
//! Semantic types (TimelineAst, BehaviorBlock, AnimationProperty, etc.) remain in mod.rs
//! and will be removed in Phase 4 of the modularization.

use serde::{Deserialize, Serialize};

use super::meta_ast;
use crate::syntax::{CapturedValue, FormMatch};

// =============================================================================
// Source Location Tracking
// =============================================================================

/// Source location span for AST nodes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

// =============================================================================
// Root AST Type
// =============================================================================

/// Import statement with source span for LSP features.
///
/// FEAT-118: an import is either GLOBAL (`@import`, flat-merge into the global
/// namespace — `namespace: None`) or NAMESPACED (`@use`, defs registered under a
/// module namespace). The namespaced fields default empty, so a bare `@import`
/// `ImportAst` is byte-identical to its pre-FEAT-118 shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportAst {
    /// The import path (e.g., "./shared.st").
    pub path: String,
    /// `Some(ns)` for `@use` — the namespace its defs register under (derived
    /// from the module path). `None` for `@import` (flat-global merge).
    #[serde(default)]
    pub namespace: Option<crate::metasystem::module::Namespace>,
    /// `@use "…" as <alias>` — the qualified alias bound in the importing file
    /// (for `@alias/name` references). `None` when unaliased or global.
    #[serde(default)]
    pub alias: Option<String>,
    /// `@use "…" only (a, b)` — explicit allow-list of imported names. Empty =
    /// open import (everything the module exports).
    #[serde(default)]
    pub only: Vec<String>,
    /// `@use "…" hiding (a, b)` — open-minus exclusion list.
    #[serde(default)]
    pub hiding: Vec<String>,
    /// Source span of the import statement.
    #[serde(default)]
    pub span: SourceSpan,
}

/// Root of a parsed .st file
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StFile {
    #[serde(default)]
    pub imports: Vec<ImportAst>,
    #[serde(default)]
    pub presets: Vec<PresetDef>,
    /// Pattern definitions (@pattern name($params) { body })
    #[serde(default)]
    pub patterns: Vec<PatternDef>,
    /// Metasystem definitions (%primitive, %macro)
    #[serde(default)]
    pub meta_defs: Vec<meta_ast::MetaDef>,
    #[serde(default)]
    pub scopes: Vec<ScopeBlock>,
    /// Top-level HTML element literals (PLAN-023 W1). Stored as pure-HTML skeletons +
    /// ordered hole sources; the pipeline runs the html5ever TreeSink to build ir::HtmlExpr.
    #[serde(default)]
    pub html_blocks: Vec<HtmlBlockAst>,
    /// Top-level CSS at-rule blocks (`@media`, `@supports`, `@keyframes`) authored
    /// directly in `.st` source. These are passed through to the stylesheet verbatim
    /// (the CSS emitter has no structured at-rule IR; `CssExpr::Raw` round-trips them).
    /// Without this the CST drops them entirely — breakpoint responsiveness was impossible
    /// (BUG-087).
    #[serde(default)]
    pub raw_css_blocks: Vec<RawCssBlock>,
    /// Collected FormMatches from all syntax elements (used by pipeline)
    #[serde(default, skip_serializing)]
    pub matches: Vec<FormMatch>,
    /// Form splices (`--name;`) NOT carried by any scope: file-root splices and
    /// splices inside DIRECTIVE bodies (e.g. `@form motion --reveal { --fade-in; …
    /// }` — SIP-001c's composition case). Scope-body splices live on their
    /// scope's `form_refs` instead; the sweep in `cst_to_stfile` dedups by span.
    /// Validated by `validate_form_refs`; STYLE splices expand at the compile seam (`expand_style_form_splices`, BUG-298).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_refs: Vec<FormRefUse>,
    /// Module manifest declared by a `MODULE.st` file (FEAT-118 M2): the
    /// `%module` name override, the `%public` export allow-list, and
    /// `%reexport` fçades. `None` for ordinary `.st` files.
    #[serde(default)]
    pub module_manifest: Option<ModuleManifest>,
    /// FEAT-120: parse-time diagnostics with no flat-match surface. Body validation
    /// (E0900/E0904/E0906/W0700 over a `@template`/`@editable-*` body) lands HERE as
    /// canonical `Diagnostic`s during `cst_to_stfile`, replacing the retired
    /// `ComponentBodyDef` capture envelope. The pipeline drains this into
    /// `output.diagnostics` (one extend, no per-code bridge). `#[serde(skip)]`: consumed
    /// in-process, never serialized to a layer artifact (`Diagnostic` is not Serialize).
    #[serde(skip)]
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
    /// PLAN-117 W5: the file's `@exports { $host, $session: mut }` clause — the
    /// page-global cells this file PUBLISHES to modules that `@use` it.
    ///
    /// The same clause and the same parser as a `@template` body's `@exports`
    /// (FEAT-115), lifted to file scope: learn one, you know the other. Empty
    /// means "publishes nothing explicitly", which keeps every existing file
    /// working unchanged (public-by-default, the Odin floor).
    ///
    /// This REPLACES the spec-reserved `%public (…)` manifest clause, which was
    /// parsed but never enforced and had zero uses in the repo — one export
    /// mechanism, not two.
    #[serde(default)]
    pub file_exports: Vec<crate::syntax::ExportDecl>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// A folder module's manifest, declared in its `MODULE.st` (FEAT-118 M2 /
/// `docs/specs/module-system.md` §5). All clauses optional — an absent
/// manifest means implicit-folder-name + public-by-default (the Odin floor);
/// clauses are the PureScript ceiling for explicit control.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// `%module <name>` — overrides the implicit folder-derived namespace name.
    /// `None` = use the folder name.
    #[serde(default)]
    pub name: Option<String>,
    /// `%public (a, b, …)` — the explicit export allow-list. PRESENT ⇒ only
    /// these names are importable; ABSENT (empty) ⇒ all public (Odin default).
    #[serde(default)]
    pub public: Vec<String>,
    /// `%reexport "coll:ns" (a, b)` — fçade re-exports. Each entry is the module
    /// ref plus an optional name list (empty = re-export all).
    #[serde(default)]
    pub reexports: Vec<ReexportDecl>,
    /// `%using { … }` — the Elixir-style ACTIVE extension hook (FEAT-118 M3).
    /// `None` when the module has no hook. When present, `@use`-ing this module
    /// runs the hook, injecting its contents into the importing scope
    /// ("modules as scoped dialects"). The DATA is captured here; execution +
    /// scoped activation layer on top (needs the ImportScope, FUP-055).
    #[serde(default)]
    pub using: Option<UsingHook>,
    /// Source span of the manifest's `%module` directive (or the file start).
    #[serde(default)]
    pub span: SourceSpan,
}

/// The body of a `%using { … }` active-extension hook (FEAT-118 M3 /
/// `docs/specs/module-system.md` §5). Each field captures one clause kind the
/// hook injects into an importing scope.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsingHook {
    /// `%claims @a @b` — directive names the hook installs namespace-rewrite
    /// rules for (stored without the leading `@`).
    #[serde(default)]
    pub claims: Vec<String>,
    /// `%capture_type <name> { … }` — names of scoped sub-grammars the hook
    /// brings into the importing scope. (The capture-type DEFINITIONS register
    /// normally; this records which are scope-activated by the hook.)
    #[serde(default)]
    pub capture_types: Vec<String>,
    /// `%default <key>: <value>` — scope-local defaults, as raw `key: value`
    /// strings (the value DSL is resolved at execution time).
    #[serde(default)]
    pub defaults: Vec<String>,
    /// Raw body text of the hook, preserved for the execution layer (FUP-055).
    #[serde(default)]
    pub raw_body: String,
}

/// One `%reexport` declaration in a `MODULE.st` manifest.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReexportDecl {
    /// The re-exported module reference (`"std:scene-3d"`).
    pub module: String,
    /// Specific names to re-export; empty = re-export everything the module
    /// makes public.
    #[serde(default)]
    pub names: Vec<String>,
}

/// A top-level HTML element literal captured from the CST (PLAN-023 W1).
///
/// `skeleton` is pure HTML with backtick holes replaced by `\u{E000}<idx>\u{E001}`
/// sentinels; `holes` are the ordered hole expression sources. Kept serializable (strings)
/// so the AST stays decoupled from `ir::HtmlExpr`; the pipeline builds the IR via the
/// html5ever TreeSink.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HtmlBlockAst {
    pub skeleton: String,
    #[serde(default)]
    pub holes: Vec<String>,
    #[serde(default)]
    pub span: SourceSpan,
    /// EDN-only inline injection semantics. Parsed `.st` markup keeps the default.
    #[serde(default)]
    pub injection: HtmlInjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HtmlInjection {
    #[default]
    Parsed,
    Raw,
    Reactive,
    /// `[:st/md "$expr"]` — the skeleton is a Spacetime expression whose value
    /// renders as Markdown (via snarkdown) into a reactive node. The dual of
    /// `Reactive`: where that lowers a reactive HTML string, this lowers a
    /// `HtmlExpr::Markdown` whose innerHTML tracks the expression's signals.
    Markdown,
}

/// A top-level CSS at-rule block (`@media`/`@supports`/`@keyframes`) captured verbatim
/// from `.st` source (BUG-087). `source` is the normalized CSS text ready to concatenate
/// into the stylesheet — the Spacetime `@media("query")` string form is normalized to the
/// standard `@media query` form on capture.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RawCssBlock {
    pub source: String,
    #[serde(default)]
    pub span: SourceSpan,
}

// =============================================================================
// Preset Types
// =============================================================================

/// @preset type &name: value;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetDef {
    pub preset_type: PresetType,
    pub name: String,
    pub value: PresetValue,
    #[serde(default)]
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetType {
    Easing,
    Scroll,
    Animation,
    Load,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PresetValue {
    Easing(EasingValue),
    Config(Vec<ConfigArg>),
    AnimationBlock(Vec<super::AnimationProperty>),
}

// =============================================================================
// Configuration Types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigArg {
    pub key: String,
    pub value: ConfigValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    Number(f64),
    String(String),
    Selector(String),
    Preset(String),
}

// =============================================================================
// Easing Types
// =============================================================================

/// easing: preset or function
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EasingDef {
    pub value: EasingValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EasingValue {
    Preset(String),
    CubicBezier(f64, f64, f64, f64),
    Spring {
        stiffness: f64,
        damping: f64,
        mass: f64,
    },
}

// =============================================================================
// Pattern System AST Types
// =============================================================================

/// @pattern name($params) { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternDef {
    pub name: String,
    pub params: Vec<PatternParam>,
    pub body: Vec<PatternBodyItem>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Pattern parameter: $name or $name: default
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternParam {
    pub name: String,
    pub default: Option<CapturedValue>,
}

/// Items that can appear in a pattern body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternBodyItem {
    // Behavior directives (pass-through)
    StateMachine(super::StateMachineAst),
    State(super::StateAst),
    Transition(super::TransitionAst),
    AsyncTransition(super::AsyncTransitionAst),
    Mutate(super::MutateAst),
    // Control flow
    For(PatternForAst),
    If(PatternIfAst),
    Match(PatternMatchAst),
    Include(PatternIncludeAst),
    // Nested pattern call
    Call(PatternCallAst),
}

/// @for $item in $list { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternForAst {
    pub variable: String,
    pub source: PatternForSource,
    pub body: Vec<PatternBodyItem>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternForSource {
    Variable(String),
    Literal(Vec<CapturedValue>),
}

/// @if $condition { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternIfAst {
    pub condition: PatternCondition,
    pub body: Vec<PatternBodyItem>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternCondition {
    Truthy(String),                   // @if $highlights
    Equals(String, CapturedValue),    // @if $mode == modal
    NotEquals(String, CapturedValue), // @if $persist != none
}

/// @match $var { arm => { } }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternMatchAst {
    pub variable: String,
    pub arms: Vec<PatternMatchArm>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternMatchArm {
    pub pattern: PatternMatchPattern,
    pub body: Vec<PatternBodyItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternMatchPattern {
    Literal(CapturedValue),
    Wildcard, // _
}

/// @include patternName(args)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternIncludeAst {
    pub pattern_name: String,
    pub args: Vec<PatternArg>,
    pub span: SourceSpan,
}

/// @patternName(args) - pattern invocation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternCallAst {
    pub pattern_name: String,
    pub args: Vec<PatternArg>,
    pub span: SourceSpan,
}

/// Argument in a pattern call: name: value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternArg {
    pub name: String,
    pub value: CapturedValue,
}

// =============================================================================
// Scope Block Types
// =============================================================================

/// The KIND of a scope region — a reference to the construct that introduced it
/// (PLAN-039). Scope identity is a CONSTRUCT REFERENCE, not a privileged CSS
/// selector: a `.sel {}` region is `Selector`, a template body is
/// `Construct("template")`, an `@each` body is `Construct("each")`, etc. The
/// construct name is a registry key (the macro / capture-type that owns the
/// region), so `%scope within(<kind>)` is a containment query over the scope
/// stack — kinds are DATA, never a Rust match-arm per kind. This is what lets the
/// closed `MacroScope` enum retire (its `Template`/`TimelineBody`/… variants were
/// declared-but-never-enforced; see research/spacetime-scope-kind-is-a-capture-type.org).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScopeKind {
    /// A CSS selector region: `.sel { }`, `#id { }`, `[attr] { }`, `> child { }`.
    /// The built-in default; the selector string lives in the owning struct's
    /// selector field.
    #[default]
    Selector,
    /// The file root region (no enclosing scope).
    File,
    /// A region introduced by a named construct (macro / capture-type): the name
    /// is a registry key, e.g. "template", "each", "timeline".
    Construct(String),
}

impl ScopeKind {
    /// The construct-reference name used by `%scope within(<name>)` matching.
    /// Built-ins canonicalize to `"selector"` / `"file"`; a `Construct(n)` yields `n`.
    pub fn as_ref_name(&self) -> &str {
        match self {
            ScopeKind::Selector => "selector",
            ScopeKind::File => "file",
            ScopeKind::Construct(n) => n.as_str(),
        }
    }
}

/// .selector { ... }
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScopeBlock {
    /// What construct introduced this region (PLAN-039). Defaults to `Selector`.
    #[serde(default)]
    pub kind: ScopeKind,
    pub selector: String,
    /// Behavior directives (state machines, transitions, etc.)
    #[serde(default)]
    pub behavior: super::BehaviorBlock,
    /// Plain CSS declarations (display: grid; gap: 20px; etc.)
    #[serde(default)]
    pub css_declarations: Vec<CssDeclaration>,
    /// Statement-position form splices (`--name;`) in this scope's body
    /// (SIP-001c, BUG-241). Validated after rematch; STYLE splices expand at the compile seam (BUG-298).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_refs: Vec<FormRefUse>,
    /// Nested scopes (> child-selector { }, .class { }, etc.)
    #[serde(default)]
    pub nested_scopes: Vec<NestedScope>,
    /// Collected FormMatches from scope content (used by pipeline)
    #[serde(default, skip_serializing)]
    pub matches: Vec<FormMatch>,
    #[serde(default)]
    pub span: SourceSpan,

    /// Source file where this ScopeBlock is defined (for error traces)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,

    /// FEAT-115 (Q4): body-bearing `Construct` scope payload, carried on the World-A
    /// scope instead of the retired `ComponentBodyDef` capture fields. Harvested in
    /// `cst_to_stfile` for `@template:<name>` scopes; empty for ordinary selectors.
    /// (States live in the scope's `local-state` matches; html in the factory
    /// reconstruct; diagnostics/section_transitions in the body validator.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<crate::syntax::ExportDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<crate::syntax::TemplateRef>,
    /// FEAT-119 (W3): the body's local state declarations (`$x bool: false;`),
    /// span-sorted across the body root and inner `.sel{}` regions. Sourced from
    /// the scope's `local-state` matches (`scope_states`) and back-filled here so the
    /// emit path serializes the factory `states` payload from World A, not the retired
    /// World-A scope `states` (was the retired `ComponentBodyDef.states`). Empty for ordinary selectors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<crate::syntax::ComponentStateDecl>,
    /// FEAT-119 (W2): the clean body HTML for a body-bearing `Construct` scope —
    /// the CST `reconstruct_template_body_html` (span-subtraction) output, the SOLE
    /// html source for the factory `rawBody`/`builder`. Empty for ordinary selectors
    /// and for non-body Construct scopes. Replaces the retired `ComponentBodyDef.html`
    /// capture field: the emit path (W3) reads html from HERE, not the capture.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub html: String,
}

/// Plain CSS declaration (property: value;)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CssDeclaration {
    pub property: String,
    pub value: String,
    /// True when written with the `<-` arrow (DOM injection: HTML attribute /
    /// textContent) rather than the `:` CSS surface (a CSS property). Routes a
    /// reactive binding to setAttribute vs style.setProperty (BUG-091).
    #[serde(default)]
    pub is_injection: bool,
    #[serde(default)]
    pub span: SourceSpan,
}

/// A statement-position form splice (`--card-surface;`, `--rise(12px);`) inside
/// a scope body (SIP-001c, BUG-241). Recognized by the parser as FORM_REF and
/// carried here so it is no longer silently dropped; validated against the
/// declared forms after rematch (unknown name = E0947). STYLE splices EXPAND
/// at the compile seam (`expand_style_form_splices`, BUG-298): the declared
/// body's declarations join this scope's CSS at the splice's source position,
/// and the consumed ref is cleared. Non-style kinds in this position are a
/// hard E0959.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormRefUse {
    /// The form name WITH its `--` sigil (e.g. `--card-surface`).
    pub name: String,
    /// The raw argument text inside the parens, when called with args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Nested scope: > child { } or .class { }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestedScope {
    /// What construct introduced this region (PLAN-039). Defaults to `Selector`.
    #[serde(default)]
    pub kind: ScopeKind,
    /// The authored LEAF selector (e.g., "> product-card", ".child", "#id") — the
    /// text the author wrote, before composition with ancestor scopes.
    pub selector: String,
    /// The fully-composed CSS selector down to this scope (e.g.
    /// `.counter > .controls > button.inc`), precomputed once by the scope-tree
    /// builder via [`crate::syntax::compose_selector`]. This is the SINGLE source
    /// of truth for "what element does this scope address": CSS emit reads it to
    /// emit rules, and directive FormMatch selector-assignment reads it to bind
    /// `@on`/`@scroll`/… to the right element. Pre-BUG-206 each consumer composed
    /// (or failed to compose) independently, so nested `@on` bound to the outer
    /// scope instead of the inner element (silent double-dispatch).
    ///
    /// For a scope nested directly under a `Construct` (template) region the root
    /// is the instance (implicit), so the first level composes STANDALONE (its
    /// leaf) — mirroring how template CSS emits each nested region standalone.
    #[serde(default)]
    pub composed_selector: String,
    /// Behavior directives within this nested scope
    #[serde(default)]
    pub behavior: super::BehaviorBlock,
    /// CSS declarations within this nested scope
    #[serde(default)]
    pub css_declarations: Vec<CssDeclaration>,
    /// Statement-position form splices (`--name;`) in this nested scope's body
    /// (SIP-001c, BUG-241). Validated after rematch; STYLE splices expand at the compile seam (BUG-298).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_refs: Vec<FormRefUse>,
    /// Further nested scopes (recursive)
    #[serde(default)]
    pub nested_scopes: Vec<NestedScope>,
    /// Collected FormMatches from nested scope content (used by pipeline)
    #[serde(default, skip_serializing)]
    pub matches: Vec<FormMatch>,
    /// BUG-130: when this nested scope is a COLLECTION-REF block
    /// (`&cards[] .sel { @each }`), the ref name (`cards`). The emit path installs a
    /// MutationObserver on the scope selector that mirrors its children into
    /// `ST.set(host, <name>, [children])` so `$cards` aggregates the @each output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_ref: Option<String>,
    #[serde(default)]
    pub span: SourceSpan,
}

// =============================================================================
// Type Definition Types
// =============================================================================

/// @type definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<TypeField>,
    /// Tagged-sum variants (PLAN-038 W0). Non-empty ⟺ this `@type` is a SUM
    /// (`@type R { A(T) | B(U) | C }`); empty ⟺ a product type (the `fields`
    /// form). The two are mutually exclusive — a sum body carries no fields.
    #[serde(default)]
    pub variants: Vec<VariantDef>,
    #[serde(default)]
    pub span: SourceSpan,
}

impl TypeDef {
    /// True when this type is a tagged sum (has ≥1 variant). A product type
    /// (only `fields`) returns false.
    pub fn is_sum(&self) -> bool {
        !self.variants.is_empty()
    }
}

/// One variant of a tagged sum `@type` (PLAN-038 W0). `Created(Todo)` →
/// `{ name: "Created", payload: ["Todo"] }`; a payload-less tag `Failed` →
/// `{ name: "Failed", payload: [] }`. Each payload entry is a raw type-ref
/// string (`Todo`, `Todo[]`, `Errors`), parsed downstream like a field type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantDef {
    pub name: String,
    #[serde(default)]
    pub payload: Vec<String>,
}

/// Field in a type definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeField {
    pub name: String,
    pub optional: bool,
    pub type_expr: TypeExpr,
}

/// Type expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeExpr {
    /// Primitive type (string, number, boolean, url, color)
    Primitive(String),
    /// Array type: Type[]
    Array(Box<TypeExpr>),
    /// Object type: { field: type; }
    Object(Vec<TypeField>),
    /// Union type: "a" | "b" | "c"
    Union(Vec<String>),
    /// Reference to another type by name
    Reference(String),
    /// Parameterized type application: `ctor(arg, ...)`.
    /// The CMS relation type `id(Agent)` is `Apply { ctor: "id", args: [Reference("Agent")] }`.
    /// A general node (not a special-cased `id`) so future parameterized types
    /// (`ref(T)`, `list(T)`, …) reuse the same shape.
    Apply { ctor: String, args: Vec<TypeExpr> },
}

/// JSON value for initial values and data expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

// =============================================================================
// Value Types
// =============================================================================

/// A value in transition or keyframe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Number(f64),
    NumberWithUnit(f64, String),
    Color(String),
    Calc(String),
    String(String),
    Identifier(String),
    FunctionCall(String, Vec<Value>), // function_name, args (Spacetime runtime functions)
    CssFunction(String, String),      // function_name, raw_content (CSS passthrough functions)
    ElementRef {
        name: String,
        facet: String,
        property: String,
    }, // &name.facet.property (e.g., &title.rect.right)
}

// =============================================================================
// Stagger Types
// =============================================================================

/// stagger: delay from direction;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaggerDef {
    pub delay: f64,
    pub grid: Option<(u32, u32)>,
    pub direction: StaggerDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaggerDirection {
    First,
    Last,
    Center,
    Random,
    Index(u32),
}
