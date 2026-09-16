//! Spacetime DSL Parser
//!
//! Parses `.st` files into an AST.

pub mod ast;
pub mod entity;

pub mod comments;
pub mod meta_ast;

// Re-export all AST types for backward compatibility
pub use ast::*;

/// The diagnostic for a directive the registry does not know.
///
/// ONE constructor for both surfaces — top level and inside a selector scope.
/// They previously built their own copies and had already drifted in hint text;
/// the in-scope site's own comment warns against exactly that drift, so the
/// answer belongs in one place.
///
/// A QUALIFIED name is deliberately NOT judged here.
///
/// `@b/badge` is only wrong if nothing binds `b`, and this runs BEFORE import
/// resolution — the parser cannot see `@use "./badge.st" as b`. Erroring here
/// flagged three legitimate files (`examples/modules/index.st` and two
/// `tests/repro/gh-22-*`) whose qualifier was properly bound.
///
/// The real check lives at `src/pipeline/mod.rs:3285`, after the import scope
/// exists, and already emits E0926 (unbound qualifier) and E0927 (hidden name).
/// BUG-301 is that this pass never RUNS under `check`, not that it is missing:
/// the fix belongs at that gate, not in a second copy of the rule here.
/// (AUD-011: a rule enforced where its evidence is absent will be wrong
/// whenever the evidence would have said yes.)
fn unknown_directive_diagnostic(
    name: &str,
    span: SourceSpan,
) -> Option<crate::diagnostics::Diagnostic> {
    // A qualified name is not judged here — see the doc comment. Returning None
    // means "no verdict from this surface", not "fine".
    if name.contains('/') {
        return None;
    }
    // PLAN-150 W0: the film surface (docs/language/film.st.md) RESERVES some
    // directive words whose implementation lands in a later wave. Using one
    // today is not a typo (W0714's assumption) — it is a known feature that is
    // not built. Say so, and name the wave, instead of "unknown, ignored": a
    // reserved word that emitted nothing on a green build is the silent-drop
    // class the film-doc gate exists to close.
    let reserved_wave: Option<(&str, &str)> = match name {
        // `scatter` landed in W5, `camera` in W3 (a `@form camera` kind), `audio`
        // in W7 (a `@audio(…) as &name at <t>;` SCORE ENTRY — valid only inside
        // a `@score` body, where the score grammar matches it). Reaching this
        // fallback means `@audio` was written OUTSIDE a score, which is
        // meaningless: sound is ON the score's transport. No reserved words
        // remain; the arm is kept as the seam for the next one.
        _ => None,
    };
    if name == "audio" {
        return Some(
            crate::diagnostics::Diagnostic::error(
                crate::diagnostics::DiagnosticCode::E0970,
                "`@audio` is a SCORE ENTRY — write it inside a `@score { … }` body \
                 (`@audio(src: \"…\") as &bed at 0s;`), not as a standalone directive"
                    .to_string(),
            )
            .with_span(span.into())
            .with_hint(
                "sound is on the score's ONE transport (film.st.md §8); it has no meaning outside a score"
                    .to_string(),
            ),
        );
    }
    if let Some((wave, what)) = reserved_wave {
        return Some(
            crate::diagnostics::Diagnostic::error(
                crate::diagnostics::DiagnosticCode::E0970,
                format!(
                    "`@{name}` is reserved by the film surface ({what}) but not yet built — it lands in PLAN-150 {wave}"
                ),
            )
            .with_span(span.into())
            .with_hint(format!(
                "see docs/language/film.st.md; until {wave} lands, `@{name}` has no runtime and would emit nothing"
            )),
        );
    }
    Some(
        crate::diagnostics::Diagnostic::warning(
            crate::diagnostics::DiagnosticCode::W0714,
            format!("`@{name}` is not a known directive and was ignored"),
        )
        .with_span(span.into())
        .with_hint(
            "check the spelling, or add the `@import` that declares it — a directive the \
             registry does not know emits nothing"
                .to_string(),
        ),
    )
}
pub use entity::{
    FacetPath, emit_facet_read_js, entity_name_from_selector, entity_selector, is_facet_path,
};

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::syntax::{
    CapturedValue, ComponentStateDecl, FormMatch, STDLIB_REGISTRY, compose_selector,
};

/// Host-language CSS at-rules that Spacetime passes through VERBATIM into the stylesheet
/// (no structured IR, no `%form` requirement). `@media`/`@supports`/`@keyframes` are the
/// long-standing set (BUG-087). `@font-face` joined in W7 (GH-20): the design rule in
/// `docs/language/css-interop.md` is that when a construct exists in the host language
/// (CSS/HTML) AND Spacetime has its own spelling, the host spelling must either work or
/// explain itself — vanishing is off the table. Raw `@font-face` is what CSS authors type
/// first, so it is passed through like the other host at-rules, converging with the
/// `@font("Family")` macro on the same `@font-face` rule in the emitted stylesheet.
///
/// This is the ONE registry for that decision — every consumer must read this const,
/// never an inline `"media" | "supports" | ...` string list that can drift apart.
pub const HOST_CSS_PASSTHROUGH_AT_RULES: &[&str] = &["media", "supports", "keyframes", "font-face"];

/// All behavior directives parsed from a scope block
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BehaviorBlock {
    pub state_machine: Option<StateMachineAst>,
    #[serde(default)]
    pub states: Vec<StateAst>,
    #[serde(default)]
    pub transitions: Vec<TransitionAst>,
    #[serde(default)]
    pub async_transitions: Vec<AsyncTransitionAst>,
    #[serde(default)]
    pub mutates: Vec<MutateAst>,
}

/// State machine directive.
///
/// ```text
/// @state_machine(initial: "viewing")
/// OR with block form:
/// @state_machine(initial: "viewing") {
///     viewing { }
///     editing { outline: 2px dashed teal; }
///
///     viewing -> editing on dblclick;
///     editing -> saving on blur, run: save_content;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineAst {
    pub initial: String,
    /// Inline state definitions (from block form)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inline_states: Vec<InlineStateAst>,
    /// Arrow transitions (from block form)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrow_transitions: Vec<TransitionAst>,
}

/// Inline state definition within a state_machine block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineStateAst {
    pub name: String,
    pub properties: Vec<StatePropertyAst>,
}

/// @state(when: "editing") { ... }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateAst {
    pub when: String,
    pub properties: Vec<StatePropertyAst>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatePropertyAst {
    pub property: String,
    pub value: String,
}

/// @transition(from: "viewing", to: "editing", on: "dblclick")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionAst {
    pub from: String,
    pub to: String,
    pub on: String,
    pub run: Option<String>,
    pub debounce_ms: Option<u32>,
}

/// @async_transition(trigger: ..., from: ..., on_success: ..., on_error: ...)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncTransitionAst {
    pub trigger: String,
    pub from: String,
    pub on_success: String,
    pub on_error: String,
    pub effect: Option<EffectAst>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EffectAst {
    Persist(String),
    Fetch(String),
}

/// @mutate selection { ... }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateAst {
    pub context: MutateContextAst,
    pub operations: Vec<MutateOpAst>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MutateContextAst {
    Selection,
    OnEnter(String),
    OnExit(String),
    OnEvent(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateOpAst {
    pub op: String,
    pub args: HashMap<String, String>,
}

// Note: OnMutationBlock, MutationStatement, SignalMethodCall are now in ast.rs

// =============================================================================
// ANIMATION PROPERTY TYPES
// =============================================================================
//
// These types are used by preset blocks (@preset animation) and the macro system.
// Timeline types (TimelineBlock, TimelineDriver, etc.) were removed in Phase 4 cleanup.
// Timelines are now created via macros in stdlib/macros/timeline.st.
//
// Note: ConfigArg, ConfigValue are in ast.rs
// =============================================================================

/// Animation property within a preset or animation block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimationProperty {
    Apply(Vec<String>),
    Transition(TransitionProperty),
    Keyframes(KeyframeProperty),
    Easing(EasingDef),
    Timing { offset: f64, span: Option<f64> },
    Stagger(StaggerDef),
    ColorSpace(String),
    Static(StaticProperty),
}

/// Static CSS property (no animation, just set once)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticProperty {
    pub property: String,
    pub value: Value,
    #[serde(default)]
    pub span: SourceSpan,
}

/// property: from -> to [in colorspace] [with easing];
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionProperty {
    pub property: String,
    pub values: Vec<Value>,
    pub inline_color_space: Option<String>,
    pub inline_easing: Option<EasingDef>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// property: { 0%: v; 50%: v; 100%: v; };
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyframeProperty {
    pub property: String,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub percentage: u8,
    pub value: Value,
    pub easing: Option<EasingDef>,
}

// Note: EasingDef, EasingValue, StaggerDef, StaggerDirection, Value are now in ast.rs

// =============================================================================
// Data Binding AST Types (moved to ast.rs)
// =============================================================================

// Note: TypeDef, TypeField, TypeExpr, JsonValue are in ast.rs
// =============================================================================
// Parser Implementation (CST-based)
// =============================================================================

/// Parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Byte offset in the source
    pub offset: usize,
    /// Length of the error span
    pub len: usize,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: impl Into<String>, offset: usize, len: usize) -> Self {
        Self {
            message: message.into(),
            offset,
            len,
        }
    }
}

impl ParseError {
    /// Convert this parse error into a rich miette::Report with source context.
    pub fn to_miette_report(&self, source: &str, filename: &str) -> miette::Report {
        let label =
            miette::LabeledSpan::at(self.offset..self.offset + self.len.max(1), &self.message);
        miette::miette!(labels = vec![label], "{}", self.message,)
            .with_source_code(miette::NamedSource::new(filename, source.to_string()))
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at offset {}", self.message, self.offset)
    }
}

impl std::error::Error for ParseError {}

/// A collection of parse errors.
///
/// Wraps `Vec<ParseError>` to expose all errors from a single parse,
/// instead of only the first one.
#[derive(Debug, Clone)]
pub struct ParseErrors(Vec<ParseError>);

impl ParseErrors {
    pub fn new(errors: Vec<ParseError>) -> Self {
        debug_assert!(!errors.is_empty());
        Self(errors)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn first(&self) -> &ParseError {
        &self.0[0]
    }
    pub fn iter(&self) -> impl Iterator<Item = &ParseError> {
        self.0.iter()
    }
    pub fn to_miette_reports(&self, source: &str, filename: &str) -> Vec<miette::Report> {
        self.0
            .iter()
            .map(|e| e.to_miette_report(source, filename))
            .collect()
    }
    pub fn render_all_plain(&self, source: &str, filename: &str) -> String {
        self.to_miette_reports(source, filename)
            .iter()
            .map(crate::error::render_miette_plain)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl std::fmt::Display for ParseErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() == 1 {
            write!(f, "{}", self.0[0])
        } else {
            writeln!(f, "{} parse errors:", self.0.len())?;
            for (i, e) in self.0.iter().enumerate() {
                write!(f, "  {}: {}", i + 1, e)?;
                if i < self.0.len() - 1 {
                    writeln!(f)?;
                }
            }
            Ok(())
        }
    }
}

impl std::error::Error for ParseErrors {}

/// Parse a .st file into an AST
///
/// This is the main entry point for parsing Spacetime DSL files.
/// It uses the new Rowan-based CST parser.
///
/// # Example
///
/// ```ignore
/// use spacetime::parser::parse;
///
/// let source = r#"
/// .hero {
///     @animate(fade-in, 500ms) {
///         opacity: 0 -> 1;
///     }
/// }
/// "#;
///
/// match parse(source) {
///     Ok(file) => println!("Parsed {} scopes", file.scopes.len()),
///     Err(e) => eprintln!("Parse error: {}", e),
/// }
/// ```
pub fn parse(input: &str) -> Result<StFile, ParseErrors> {
    parse_impl(input, true)
}

/// Parse a block BODY FRAGMENT (template/test body lowering, `emit/js.rs`),
/// skipping form-splice validation. A fragment is validated where it is
/// AUTHORED — the enclosing file's own parse — and the forms it may splice
/// are declared in that enclosing file, which a fragment parse cannot see:
/// validating here would report every legitimate splice as unknown (E0947)
/// and carry a spurious diagnostic into the host page's output.
pub fn parse_body_fragment(input: &str) -> Result<StFile, ParseErrors> {
    parse_impl(input, false)
}

fn parse_impl(input: &str, validate: bool) -> Result<StFile, ParseErrors> {
    use crate::syntax::cst;

    let parse_result = cst::parse(input);

    // Check for parse errors — return all of them
    if !parse_result.errors.is_empty() {
        let errors = parse_result
            .errors
            .iter()
            .map(|e| ParseError::new(&e.message, e.offset, e.len))
            .collect();
        return Err(ParseErrors::new(errors));
    }

    // Convert CST to StFile
    let mut file = cst_to_stfile(&parse_result.root, input);

    // PLAN-135 W4: attach each match's contiguous `///` doc block. Parse has
    // the source and every match has its span — the one seam that covers the
    // main file, `@import`ed files (each parsed here), and the project
    // overlay. The `@data forms` catalog serves `doc`; everything else
    // ignores it. Runs before validation so a doc never changes a verdict
    // (comments are trivia — AGENTS).
    fill_match_docs(&mut file, input);

    // Matches are populated via CST to StFile conversion
    if validate {
        validate_form_refs(&mut file);
        validate_declaration_entities(&mut file);
        validate_declaration_entities(&mut file);
    }

    Ok(file)
}

/// Parse input without accessing the stdlib registry.
///
/// This is used during bootstrap to avoid circular dependency when
/// the STDLIB_REGISTRY is being initialized. Form parameter names
/// will use heuristics instead of the proper names from form patterns.
pub fn parse_for_bootstrap(input: &str) -> Result<StFile, ParseErrors> {
    use crate::syntax::cst;

    let parse_result = cst::parse(input);

    // Check for parse errors — return all of them
    if !parse_result.errors.is_empty() {
        let errors = parse_result
            .errors
            .iter()
            .map(|e| ParseError::new(&e.message, e.offset, e.len))
            .collect();
        return Err(ParseErrors::new(errors));
    }

    // Convert CST to StFile without registry lookup
    let mut file = cst_to_stfile_with_registry(&parse_result.root, input, None);
    validate_form_refs(&mut file);

    Ok(file)
}

/// Convert a CST root node to an StFile AST
fn cst_to_stfile(root: &crate::syntax::cst::SyntaxNode, source: &str) -> StFile {
    // Access the stdlib registry for proper form pattern lookup
    // This is safe here because cst_to_stfile is only called from parse(),
    // not during bootstrap_stdlib() which would cause circular initialization.
    let registry = &*STDLIB_REGISTRY;
    cst_to_stfile_with_registry(root, source, Some(registry))
}

/// FEAT-116 / Q3: whether a construct's `$body:component_body` is lowered to a
/// World-A `Construct` SCOPE (states/exports/refs/directives sourced from the
/// scope, not reify) and mounts per-instance. Registry-DERIVED: a construct is
/// body-bearing iff its `%form` declares a `component_body` body capture — there
/// is no hardcoded name set. Adding a new body-bearing construct is therefore a
/// pure-stdlib act (write a `%macro` with `$body:component_body`); no Rust edit.
/// Kept in lockstep with `Context::is_template_directive` (the inner-match-
/// surfacing seam), which derives the same predicate from the same registry.
fn is_body_bearing_construct(name: &str, registry: Option<&crate::syntax::SyntaxRegistry>) -> bool {
    // Registry-DERIVED: a construct is body-bearing iff its `%form` declares a
    // `$body:component_body` capture. Read ONLY from the registry threaded into
    // this parse — NEVER the global `STDLIB_REGISTRY`. `registry` is `None` only on
    // the `parse_for_bootstrap` path, which runs WHILE the global LazyLock is still
    // initializing (bootstrap parses every stdlib macro through here); touching the
    // global there re-enters its own init and deadlocks. During bootstrap there is
    // also nothing to detect — a stdlib macro *definition* never nests a body-
    // bearing construct — so `None` safely means "not body-bearing".
    registry.is_some_and(|r| r.is_body_bearing(name))
}

/// Convert a CST root node to an StFile AST with an optional registry for form lookup
fn cst_to_stfile_with_registry(
    root: &crate::syntax::cst::SyntaxNode,
    source: &str,
    registry: Option<&crate::syntax::SyntaxRegistry>,
) -> StFile {
    use crate::syntax::cst::{AstNode, Root};

    let mut file = StFile::default();
    file.span = SourceSpan::new(0, source.len());

    // Try to cast to Root
    let Some(root_node) = Root::cast(root.clone()) else {
        return file;
    };

    // --- FormMatch extraction via event-based parser ---
    // Single pass: lex + parse + match all directives, variables, and element refs.
    // Replaces convert_cst_directive_to_form_match, convert_cst_element_ref_stmt_to_form_match,
    // and convert_cst_variable_decl_to_form_match.
    let all_matches = if let Some(reg) = registry {
        let (matches, match_diagnostics) = crate::syntax::events::parse_matches(source, reg);
        // BUG-234: `subscribe` is a discriminator, not an optional word. A form
        // failure after it must be reported rather than silently dropped with the
        // generic unmatched directive diagnostics.
        // BUG-229: every retained match diagnostic is surfaced, not just the one
        // hardcoded shape. `match_sink` has already applied the judgement about
        // WHICH failures are real (a committed grammar that then failed) vs which
        // are the normal negative result of trying every form against every
        // directive — so anything that arrives here is a genuine author error and
        // dropping it is what made malformed bodies compile green.
        file.diagnostics
            .extend(match_diagnostics_to_errors(source, &match_diagnostics));
        matches
    } else {
        Vec::new()
    };

    // FUP-069: a directive whose macro captures its body as an OPAQUE `:block`
    // (the `@test`/`@mount`/`@fixture`/`@given`/`@try` testing surface) re-compiles
    // that body as a SEPARATE sub-program via `%$content.js`. Its interior
    // FormMatches (`@on`, `local-state`, nested `.sel{}` directives) therefore
    // belong to the sub-program ONLY — emitting them ALSO as page-level matches
    // double-binds every interior `@on` (once correctly via the scoped recompile,
    // once wrongly at file scope on `document.body`, an ancestor → a click fires
    // both, doubling the mutation). Collect the BYTE SPANS of such bodies from the
    // (correctly-nested) CST and drop any match strictly inside one. The opaque
    // directive's OWN match (which spans `@name { … }`, starting at the `@`) is kept
    // — only its interior is suppressed.
    let opaque_body_spans: Vec<(usize, usize)> = match registry {
        Some(reg) => root_node
            .syntax()
            .descendants()
            .filter_map(crate::syntax::cst::Directive::cast)
            .filter(|d| {
                d.name_text()
                    .as_deref()
                    .is_some_and(|n| reg.has_opaque_block_body(n))
            })
            .filter_map(|d| d.body())
            .map(|b| {
                let r = b.syntax().text_range();
                (usize::from(r.start()), usize::from(r.end()))
            })
            .collect(),
        None => Vec::new(),
    };
    let mut all_matches: Vec<_> = all_matches
        .into_iter()
        .filter(|fm| {
            !opaque_body_spans
                .iter()
                .any(|&(bs, be)| bs <= fm.span.start && fm.span.end <= be)
        })
        .collect();

    // Collect scope spans for associating matches with scopes
    let mut scope_spans: Vec<(SourceSpan, String)> = Vec::new();

    // Qualified directive names no form claimed. Judged AFTER imports are
    // collected (BUG-301) — see the loop near the end of this function.
    let mut unclaimed_qualified: Vec<(String, SourceSpan)> = Vec::new();

    // Process scope blocks (structural extraction — not FormMatch)
    for scope in root_node.scope_blocks() {
        // W2: an unknown directive INSIDE a selector scope must be diagnosed.
        //
        // The top-level case already warns (W0714, a few hundred lines below),
        // and a component body already errors (E0900, form_compiler.rs). The
        // selector scope — where directives actually live — was the one surface
        // still silent, so `.card { @notarealdirective(x: 1) }` compiled clean
        // and emitted nothing. That is the GH-13/GH-22 experience exactly:
        // forget an import, get a blank page and a green build.
        //
        // Checked HERE rather than in `convert_cst_scope` because this is where
        // the registry is in hand; the conversion is deliberately registry-free.
        // The predicate is the same one the top-level check uses — zero forms
        // registered for `@name` — so the two cannot drift into disagreeing
        // about what "unknown" means.
        if let Some(r) = registry
            && let Some(body) = scope.body()
        {
            for directive in body.directives() {
                let Some(name) = directive.name_text() else {
                    continue;
                };
                let name = name.trim_start_matches('@').to_string();
                if name.is_empty() || !r.get_forms_for_directive(&name).is_empty() {
                    continue;
                }
                let range = directive.syntax().text_range();
                let span = SourceSpan::new(range.start().into(), range.end().into());
                match unknown_directive_diagnostic(&name, span) {
                    Some(d) => file.diagnostics.push(d),
                    None => unclaimed_qualified.push((name, span)),
                }
            }
        }
        if let Some(scope_ast) = convert_cst_scope(&scope, source) {
            scope_spans.push((scope_ast.span, scope_ast.selector.clone()));
            file.scopes.push(scope_ast);
        }
    }

    // FEAT-142 WAVE A: top-level entity scopes `&name { @directive... }` are
    // selector-less, body-bearing element-ref statements. Harvest each as a
    // World-A entity scope (a plain `Selector` scope) whose `selector` is the marker
    // selector `[data-st-entity="name"]`. Interior directives are assigned to it
    // by span containment and bind to the marker. Also synthesize a FormMatch that
    // drives the `entity-scope-impl` primitive so the marker is emitted and the
    // entity is registered in `window.__stWorld.byName`.
    for ers in root_node.element_ref_stmts() {
        let Some(ers_body) = ers.body() else { continue };
        if ers.arg_list().is_some() {
            continue; // template invocation `&name(){}` — not an entity scope
        }
        let selector_text = ers.selector_text().unwrap_or_default().trim().to_string();
        if !selector_text.is_empty() {
            continue; // `&name .sel {}` / collection-ref — not an entity scope
        }
        // A DOTTED ref (`&name.component { … }`) is a FACET PATH, NOT a base entity
        // declaration. Its `.component` lives in the ELEMENT_REF node (not the
        // selector), so `selector_text` is empty and it would otherwise be
        // mis-harvested as the base entity `name` — silently dropping the facet
        // segment. Require exactly ONE path segment (a bare `&name`) here; dotted
        // facet-path refs are resolved by the Wave-B/C facet machinery, not lowered
        // as an entity scope (reviewer P2, FEAT-142).
        if ers.element_ref().map(|e| e.path().len()).unwrap_or(1) > 1 {
            continue;
        }
        let Some(name) = ers.name_text() else {
            continue;
        };
        let body_range = ers_body.text_range();
        let span = SourceSpan::new(body_range.start().into(), body_range.end().into());
        let synthetic_selector = entity_selector(&name);

        // An entity scope is a plain SELECTOR scope whose selector is the synthetic
        // per-entity marker class `.st-entity-<name>`: its interior component
        // directives bind to that class exactly like any `.sel { … }` scope (the
        // ordinary selector-assignment path). There is NO dedicated `ScopeKind`
        // arm — "entity-ness" is carried as DATA by the synthesized
        // `entity-scope-impl` FormMatch below (which drives marker injection +
        // runtime registration), per the AGENTS elegance bar (kinds-as-data, not a
        // closed Rust enum arm). (reviewer P2, FEAT-142.)
        file.scopes.push(ScopeBlock {
            kind: ScopeKind::Selector,
            selector: synthetic_selector.clone(),
            behavior: Default::default(),
            css_declarations: Vec::new(),
            form_refs: Vec::new(),
            nested_scopes: Vec::new(),
            matches: Vec::new(),
            span,
            source_file: None,
            exports: Vec::new(),
            refs: Vec::new(),
            states: Vec::new(),
            html: String::new(),
        });
        scope_spans.push((span, synthetic_selector.clone()));

        let mut entity_captures = HashMap::new();
        entity_captures.insert("name".to_string(), CapturedValue::Ident(name));
        let mut entity_fm = FormMatch::with_captures("entity-scope-impl", entity_captures);
        entity_fm.span = span;
        all_matches.push(entity_fm);
    }

    // PLAN-039 S1b: a `@template &name($p){ … }` body is a SCOPE, not an opaque
    // `component_body` capture. Build ONE `ScopeBlock` per template (kind
    // `Construct("template")`, span = the body span, selector = a synthetic per-template
    // key `@template:<name>`) and hang the body's inner `.sel{}` regions under it as
    // `nested_scopes`. The span-containment assignment below then routes EVERY body match
    // — `local-state` ($x), `@on`, `@each`, `@editable`, class-toggles — into this scope
    // (directly, or into a nested `.sel` first), exactly like a `.sel{}` body. This is the
    // structural truth ("AST is truth") that lets the cutover (S2) source the factory
    // payload (states/exports/matches) from the scope instead of the parallel
    // `component_body` reify parser, and lets `%scope within(template)` match.
    //
    // FEAT-115 S2: `@exports { $x[: mut] }` has NO flat-match surface (unlike
    // `local-state`), so harvest it straight from each template body's `@exports`
    // directive here — via `parse_exports_block`, keyed by template name. FEAT-119:
    // this populates `scope.exports` (World A), the SOLE source the emit path reads;
    // there is no longer a `body.exports` capture field to overwrite.
    let mut template_exports: HashMap<String, Vec<crate::syntax::ExportDecl>> = HashMap::new();
    // FEAT-115 S3: the factory `html`/`builder` come from reify's text-SEGMENTATION,
    // which LEAKS trailing non-HTML body segments (a `.sel { @on … }` block, a
    // `text <- $x` line) into `html` — a live bug: they render as a spurious
    // `display:contents` wrapper + a literal CSS text node in the instance DOM.
    // The CST already separates the body's HTML element nodes from its construct
    // segments, so harvest the verbatim HTML element text directly (World A) — clean
    // by construction. Keyed by template name; populates `scope.html` (FEAT-119), the
    // SOLE html source the emit path + reactive builder read.
    let mut template_html: HashMap<String, String> = HashMap::new();
    // FEAT-115 S3d: per-template instance-ROOT selector (keyed by the synthetic
    // `@template:<name>` scope selector). A body-ROOT behavioral construct (a
    // `@match` not wrapped in a `.sel{}`) has no nested selector to adopt; it binds
    // to the instance root via this selector so it mounts per-instance (where the
    // factory seeds the params it resolves) instead of leaking to page scope.
    let mut template_root_sel: HashMap<String, String> = HashMap::new();
    for directive in root_node.directives() {
        // FEAT-116: build a World-A Construct scope for every body-bearing directive
        // form (@template + @editable-mark/@editable-block), not just @template.
        match directive.name_text().as_deref() {
            Some(n) if is_body_bearing_construct(n, registry) => {}
            _ => continue,
        }
        let Some(body) = directive.body() else {
            continue;
        };
        let body_range = body.syntax().text_range();
        let body_span = SourceSpan::new(body_range.start().into(), body_range.end().into());
        // The template's ref name (`&card` -> "card"): the directive contains an
        // ElementRef node for the `&name`. Fall back to a positional key if absent.
        let tmpl_name = directive
            .syntax()
            .descendants()
            .find_map(crate::syntax::cst::ElementRef::cast)
            .and_then(|e| e.name_text())
            .unwrap_or_else(|| format!("anon{}", body_span.start));
        let selector = format!("@template:{}", tmpl_name);
        // E0929 (fail-loud): a `:` inside a `@template`/`@editable-*` param list is
        // never valid — a param declares its TYPE with a space (`$price number`)
        // and its DEFAULT with `=` (`$title = "x"`). A stray `:` (`$a: "one", $b:
        // "two"`) previously truncated the list SILENTLY at the first param (the
        // param_list PEG cannot consume `:`, so its `( … )*` loop stops and every
        // later param is dropped with no diagnostic). Scan the arg-list source for a
        // TOP-LEVEL `:` (ignoring any inside string literals / brackets, e.g. a
        // union type or a default string value) and surface it instead of losing
        // data. The return type's `:` lives AFTER the `)`, outside the arg list, so
        // it is never scanned here.
        if let Some(arg_list) = directive.arg_list() {
            let al_range = arg_list.syntax().text_range();
            let al_text = arg_list.syntax().text().to_string();
            if let Some(rel) = first_top_level_colon(&al_text) {
                let at = usize::from(al_range.start()) + rel;
                file.diagnostics.push(
                    crate::diagnostics::Diagnostic::error(
                        crate::diagnostics::DiagnosticCode::E0929,
                        format!("`:` is not valid in the parameter list of @template &{tmpl_name}"),
                    )
                    .with_span(crate::diagnostics::SourceSpan::new(at, at + 1))
                    .with_hint(
                        "a parameter uses a SPACE for its type (`$price number`) and `=` for a \
                         default (`$title = \"x\"`) — remove the `:` or replace it with `=`"
                            .to_string(),
                    ),
                );
            }
        }
        // FEAT-115 S2: harvest this template's `@exports` declarations from its body
        // directive(s), parsed by the canonical `parse_exports_block` (single source
        // of export value-semantics + E0904 diagnostics). Keyed by template name to
        // populate `scope.exports` (World A) after scope construction.
        {
            let mut exports = Vec::new();
            for d in body.syntax().descendants() {
                let Some(dir) = crate::syntax::cst::Directive::cast(d) else {
                    continue;
                };
                if dir.name_text().as_deref() != Some("exports") {
                    continue;
                }
                let dir_range = dir.syntax().text_range();
                let dir_span = SourceSpan::new(dir_range.start().into(), dir_range.end().into());
                let block = dir.syntax().text().to_string();
                // FEAT-120: E0904 export diagnostics land in `file.diagnostics` (the
                // canonical channel) — no longer smuggled through the body capture.
                crate::syntax::events::form_compiler::parse_exports_block(
                    &block,
                    &mut exports,
                    &mut file.diagnostics,
                    dir_span.into(),
                );
            }
            if !exports.is_empty() {
                template_exports.insert(tmpl_name.clone(), exports);
            }
        }
        // FEAT-120: validate the body HERE — the single site where the template name,
        // body span, and source text all coexist. The validator emits canonical
        // `Diagnostic`s (E0900/E0906/W0700) straight into `file.diagnostics`, replacing
        // the retired `ComponentBodyDef` capture envelope + its two pipeline bridges.
        // `inner` is the brace-stripped body text (the extractor's old input shape),
        // with `inner_offset` its absolute start so spans are real.
        {
            let body_text = &source[body_range.start().into()..body_range.end().into()];
            // Strip the outer `{ }` to match the extractor's `inner`. The CST body is
            // always brace-delimited; fall back to the whole slice if not.
            let (inner, inner_offset) = match (body_text.find('{'), body_text.rfind('}')) {
                (Some(o), Some(c)) if c > o => (
                    &body_text[o + 1..c],
                    usize::from(body_range.start()) + o + 1,
                ),
                _ => (body_text, body_range.start().into()),
            };
            file.diagnostics.extend(
                crate::syntax::events::form_compiler::validate_component_body_with_registry(
                    inner,
                    inner_offset,
                    &tmpl_name,
                    registry,
                ),
            );
        }
        // FEAT-115 S3: harvest the body's HTML element nodes verbatim from the CST
        // (World A), joined by newline — the clean HTML, with no construct segments
        // leaked in (the reify-segmentation bug). Direct children only: a `.sel {}`
        // block's inner HTML belongs to that nested scope, not the body root.
        let body_html = reconstruct_template_body_html(&body);
        template_html.insert(tmpl_name.clone(), body_html.clone());
        // BUG-167: a root `@media (…) { … }` (or `@supports`/`@keyframes`) inside a
        // @template body was previously SILENTLY DROPPED — no component_item arm
        // matched it (validate_component_body is diagnostics-only and never builds
        // CSS output), and it isn't a body-bearing construct captured elsewhere, so
        // it vanished with no error and no emitted rule. Mirror the top-level
        // `"media" | "supports" | "keyframes" => file.raw_css_blocks` handling
        // (~L979): harvest each such directive DIRECTLY in this template's body
        // (not nested inside a `.sel{}` — those are a DIFFERENT Directive parent and
        // are walked separately if ever added) and hoist its raw source verbatim to
        // the page's global stylesheet. Template CSS is already globally-namespaced
        // (no per-template scoping yet — FUP-062 tracks proper `@scope` support), so
        // this hoist is consistent with every other CSS rule a template body emits.
        for at_rule_dir in body.directives() {
            let Some(at_name) = at_rule_dir.name_text() else {
                continue;
            };
            if !HOST_CSS_PASSTHROUGH_AT_RULES.contains(&at_name.as_str()) {
                continue;
            }
            let dir_range = at_rule_dir.syntax().text_range();
            let dir_span = SourceSpan::new(dir_range.start().into(), dir_range.end().into());
            let raw = at_rule_dir.syntax().text().to_string();
            let normalized = normalize_raw_css_at_rule(&raw);
            file.raw_css_blocks.push(crate::parser::ast::RawCssBlock {
                source: normalized,
                span: dir_span,
            });
        }
        // Inner `.sel{}` regions become nested scopes of the template scope.
        let mut nested_scopes = Vec::new();
        // FEAT-115 S3c: body-root content injections (`target <- $x;`) become
        // synthesized nested scopes on the UNIFIED reactive-binding path (the same
        // one serving `.sel { text <- $x }`), retiring the parallel reify injection
        // field. Root-targeting forms bind against the instance root selector.
        let root_sel = template_root_selector(&body_html);
        template_root_sel.insert(selector.clone(), root_sel.clone());
        nested_scopes.extend(injection_scopes_from_body(&body, &root_sel));
        // FEAT-115 S5c: body-root reactive `:` decls (class-toggle / self-prop)
        // ride the same synthesized-scope path (was reify's `directives`).
        nested_scopes.extend(directive_scopes_from_body(&body, &root_sel));
        // BUG-130: collection-ref selector blocks `&name[] .sel { @each }` are
        // ELEMENT_REF_STMTs (not SCOPE_BLOCKs), so harvest their bodies as nested
        // scopes too — the interior @each then assigns by span containment.
        nested_scopes.extend(collection_ref_scopes_from_body(&body));
        // FEAT-142 WAVE E: entity scopes `&name { @c }` nested INSIDE this
        // construct body (a `@template` delegate, so `&name{}` works per-row under
        // `@each`, and inside `@stage`-like children). Harvest each as a nested
        // selector scope on `.st-entity-<name>` AND synthesize its
        // `entity-scope-impl` registration match (the nested analog of the
        // top-level harvest). The interior @directives assign into the nested scope
        // by span containment, exactly like a `.sel{}` region.
        for (ns, ent_name, ent_span) in entity_scopes_from_body(&body) {
            nested_scopes.push(ns);
            let mut ent_caps = HashMap::new();
            ent_caps.insert("name".to_string(), CapturedValue::Ident(ent_name));
            let mut ent_fm = FormMatch::with_captures("entity-scope-impl", ent_caps);
            ent_fm.span = ent_span;
            all_matches.push(ent_fm);
        }
        for nested in body.nested_scopes() {
            if let Some(mut ns) = convert_cst_scope(&nested, source) {
                ns.kind = crate::parser::ast::ScopeKind::Construct("template".to_string());
                nested_scopes.push(crate::parser::ast::NestedScope {
                    kind: ns.kind,
                    selector: ns.selector,
                    composed_selector: String::new(),
                    behavior: ns.behavior,
                    css_declarations: ns.css_declarations,
                    form_refs: ns.form_refs,
                    nested_scopes: ns.nested_scopes,
                    matches: ns.matches,
                    collection_ref: None,
                    span: ns.span,
                });
            }
        }
        // BUG-130: a data-rendering directive (`@each`/`@view`) placed DIRECTLY in a
        // @template body's MARKUP is silently dropped — the body parser keeps HTML +
        // nested `.sel{}` scopes + root behavioral directives, but a body-root
        // `@each`/`@view` renders NOTHING (its interior invocations never compile).
        // The supported idiom wraps it in a selector scope: `.sel { @each(…){…} }`.
        // Warn so the silent failure becomes a clear, actionable author error.
        // An inline `@each(`/`@view(` in template-body MARKUP is tokenized as raw HTML
        // text (NOT a CST Directive node), so it cannot be found structurally. Detect
        // it heuristically: the body's HTML (after construct spans are cut) still
        // contains the directive text `@each(` / `@view(`. Selector-scoped + body-root
        // directive forms are CUT from body_html (they are SCOPE_BLOCK / DIRECTIVE
        // children), so if the marker survives into body_html it is the swallowed
        // inline form. Low false-positive: `@each(` with the opening paren is specific
        // to the directive call (prose `@each` without `(` does not match).
        // `@if` joins `@each`/`@view` here: HTML content is lexed as flat
        // HTML_RAW tokens, so a directive written INSIDE the template's markup is
        // never a CST child of the body and the construct-cut above cannot see it.
        // Its source survives into the template HTML and compiles to a TEXT NODE —
        // the author sees `@if &slot {` rendered on the page.
        //
        // Both spellings are listed because both leak: `@if(` is the
        // `%macro conditional` form and `@if ` the CST form. `stdlib/macros/template.st`
        // documents the paren form inside a template body in three examples, so the
        // documented shape rendered text with no diagnostic at all (BUG-358).
        for marker in ["@each(", "@view(", "@if(", "@if "] {
            if body_html.contains(marker) {
                let bare = marker
                    .trim_start_matches('@')
                    .trim_end_matches('(')
                    .trim_end();
                let rel = body_html.find(marker).unwrap_or(0);
                let start = body_span.start + rel;
                file.diagnostics.push(
                    crate::diagnostics::Diagnostic::warning(
                        crate::diagnostics::DiagnosticCode::W0712,
                        format!(
                            "`@{bare}` directly in the body markup of @template &{tmpl_name} is dropped and renders nothing"
                        ),
                    )
                    .with_span(crate::diagnostics::SourceSpan::new(start, start + marker.len()))
                    .with_hint(if bare == "if" {
                        // A conditional guards MARKUP, so the `&child(…)` shape
                        // suggested for @each/@view would be wrong advice here.
                        "wrap it in a selector scope: a `.your-class { @if $cond { … } }` \
                         block inside the template body"
                            .to_string()
                    } else {
                        format!(
                            "wrap it in a selector scope: a `.your-class {{ @{bare}(…) {{ &child(…); }} }}` block inside the template body"
                        )
                    }),
                );
            }
        }
        let tmpl_scope = ScopeBlock {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector: selector.clone(),
            behavior: Default::default(),
            css_declarations: Vec::new(),
            form_refs: Vec::new(),
            nested_scopes,
            matches: Vec::new(),
            span: body_span,
            source_file: None,
            // FEAT-115 (Q4): exports harvested above (template_exports); refs are
            // back-filled after `template_scope_refs` is computed below. The factory
            // payload reads them HERE (scope.rs), not from the ComponentBody capture.
            exports: template_exports
                .get(&tmpl_name)
                .cloned()
                .unwrap_or_default(),
            refs: Vec::new(),
            // FEAT-119 W3: states back-filled below (after template_scope_states).
            states: Vec::new(),
            // FEAT-119 W2: the clean CST body html, the SOLE factory html source.
            html: body_html.clone(),
        };
        scope_spans.push((body_span, selector));
        file.scopes.push(tmpl_scope);
    }

    // Precompute each nested scope's fully-composed selector (BUG-206). This is the
    // SINGLE composition pass: `composed_selector` becomes the source of truth that
    // BOTH CSS emit and directive selector-assignment read, so a nested `@on` binds
    // the inner element (`.counter > .controls > button.inc`) instead of being
    // hoisted to the outer scope. A `Construct` (template) region's nested scopes
    // compose STANDALONE — the instance root is implicit — mirroring template CSS.
    for scope in &mut file.scopes {
        let root = if matches!(scope.kind, crate::parser::ast::ScopeKind::Construct(_)) {
            ""
        } else {
            scope.selector.as_str()
        };
        compose_nested_selectors(root, &mut scope.nested_scopes);
    }

    // Assign matches to scopes by span containment (including nested scopes).
    for mut fm in all_matches {
        let mut assigned = false;
        for scope_ast in file.scopes.iter_mut() {
            if scope_ast.span.start <= fm.span.start && fm.span.end <= scope_ast.span.end {
                // A `Construct("template")` scope's selector is a SYNTHETIC association key
                // (`@template:name`), NOT a CSS/DOM selector. It must never become a
                // match's emit selector. Instead, descend to the inner `.sel{}` nested
                // scopes for a REAL selector; a match sitting DIRECTLY in the template body
                // (e.g. a `$state` decl) keeps its own selector (typically None) and is
                // sourced into the factory by the cutover, not emitted at page scope.
                let is_construct =
                    matches!(scope_ast.kind, crate::parser::ast::ScopeKind::Construct(_));
                if is_construct {
                    // Find the deepest containing nested (real-selector) scope; adopt its
                    // COMPOSED selector (BUG-206). Pre-fix this returned the LEAF alone.
                    if let Some(sel) =
                        nearest_nested_composed_selector(&fm, &scope_ast.nested_scopes)
                    {
                        fm.selector = Some(sel);
                    } else if fm.selector.is_none()
                        && (fm.macro_name == "match" || fm.macro_name == "on")
                    {
                        // A body-ROOT behavioral construct (not wrapped in a `.sel{}`)
                        // must bind to the instance ROOT so it mounts per-instance (where
                        // the factory seeded the state/subject it resolves), NOT at page
                        // scope. Without a root selector an `@on` lowers to `document.body`
                        // (a single page-global listener that never re-binds to instances
                        // created via invokeTemplate/@each) and a `@match` mounts once at
                        // page scope. Adopt the template's root selector for both.
                        //   - `@match`: render-once dispatch (FEAT-115 S3d).
                        //   - `@on`:    per-instance event handler (the body-root form
                        //               `@on click { … }`; the wrapped `.sel { @on … }`
                        //               form already gets a real selector above).
                        // Scoped to these two ONLY: template invocations `&name()` keep
                        // their reify-refs path + recursion-depth guard — a root selector
                        // would reroute them to the unguarded invoke primitive.
                        if let Some(root) = template_root_sel.get(&scope_ast.selector) {
                            fm.selector = Some(root.clone());
                        }
                    }
                    // A `local-state` ($x decl) sitting in a template body is the INSTANCE's
                    // state — the factory seeds it per-instance (ST.set on the instance
                    // root). It must NOT also page-emit `window._localState[x]` (a global
                    // leak that the instance signal merely shadows). So keep it ONLY in the
                    // scope (where the cutover sources the factory `states`) and do NOT add
                    // it to the flat `file.matches` that drives page-level emit. Behavioral
                    // constructs (@on/@each/@editable/class-toggle) DO stay flat — their
                    // page selector-init mounts per-instance via the observer.
                    let is_state = fm.macro_name == "local-state";
                    if !is_state {
                        file.matches.push(fm.clone());
                    }
                    if !assign_match_to_nested_scopes(&fm, &mut scope_ast.nested_scopes) {
                        scope_ast.matches.push(fm.clone());
                    }
                } else {
                    // The COMPOSED selector of the deepest nested scope containing this
                    // match, falling back to this scope's own selector at the body root.
                    // Pre-BUG-206 this unconditionally took the OUTER scope's selector,
                    // hoisting a nested `@on` to `.counter` so one click fired every
                    // handler bound under it (inc + dec → net zero; server: double-dispatch).
                    fm.selector = Some(
                        nearest_nested_composed_selector(&fm, &scope_ast.nested_scopes)
                            .unwrap_or_else(|| scope_ast.selector.clone()),
                    );
                    file.matches.push(fm.clone());
                    if !assign_match_to_nested_scopes(&fm, &mut scope_ast.nested_scopes) {
                        scope_ast.matches.push(fm.clone());
                    }
                }
                assigned = true;
                break;
            }
        }
        if !assigned {
            file.matches.push(fm);
        }
    }

    // PLAN-039 FEAT-115 S1: source the factory `states` payload from the template
    // SCOPE (World A) rather than the parallel `reify_state` (World B). Each
    // `template` match carries a `body: ComponentBody` whose `states` were filled
    // by the reify classifier; overwrite them with the decls surfaced on the
    // matching `Construct("template")` scope's `local-state` matches (keyed by the
    // synthetic `@template:<name>` selector). Same AST, one producer: the structured
    // `ComponentBodyDef` payload (html/states/exports/refs) is all scope-sourced here;
    // `validate_component_body` (formerly `reify_component_body`) is a pure validator
    // that builds none of it. Behavior is unchanged: the factory seeds the same
    // `{var_name,type_name,initial}` per instance (stdlib/primitives/template.st).
    let template_scope_states: HashMap<String, Vec<ComponentStateDecl>> = file
        .scopes
        .iter()
        .filter_map(|s| match &s.kind {
            ScopeKind::Construct(_) => {
                let name = s.selector.strip_prefix("@template:")?.to_string();
                Some((name, scope_states(s)))
            }
            _ => None,
        })
        .collect();
    // FEAT-115 S3d: refs (body template invocations) from the scope's invoke matches
    // (bare/named), span-sorted, merged with dynamic `&$w(…)` harvested from the body
    // source (no flat match for a `$`-name). Populates `scope.refs` (World A);
    // the factory renders them via its existing mount-time forEach.
    let template_scope_refs: HashMap<String, Vec<crate::syntax::TemplateRef>> = file
        .scopes
        .iter()
        .filter_map(|s| match &s.kind {
            ScopeKind::Construct(_) => {
                let name = s.selector.strip_prefix("@template:")?.to_string();
                let mut spanned = scope_refs(s);
                spanned.extend(harvest_dynamic_refs(source, s.span));
                spanned.sort_by_key(|(start, _)| *start);
                Some((name, spanned.into_iter().map(|(_, r)| r).collect()))
            }
            _ => None,
        })
        .collect();
    // FEAT-119 (W3): back-fill each Construct scope's `states` + `refs` from the
    // harvested maps, so the EMIT path serializes the factory payload entirely from
    // the SCOPE (World A) — `html` (W2), `exports` (built at scope construction),
    // `states` + `refs` (here). The retired `ComponentBodyDef` capture fields are no
    // longer the source; this is the single producer.
    for s in file.scopes.iter_mut() {
        if let ScopeKind::Construct(_) = &s.kind
            && let Some(name) = s.selector.strip_prefix("@template:")
        {
            if let Some(states) = template_scope_states.get(name) {
                s.states = states.clone();
            }
            if let Some(refs) = template_scope_refs.get(name) {
                s.refs = refs.clone();
            }
        }
    }
    // FEAT-120: the body-capture diagnostic-injection block is GONE. Body validation
    // (E0900/E0904/E0906/W0700) now runs in the scope-building loop above and lands in
    // `file.diagnostics` (the canonical channel); the `ComponentBody` capture is a bare
    // presence-marker carrying nothing. The factory payload (states/html/exports/refs)
    // is read from the SCOPE at emit time (World A, single producer).

    // Process top-level directives (non-FormMatch structural extraction)
    for directive in root_node.directives() {
        let name = directive.name_text().unwrap_or_default();
        let text_range = directive.syntax().text_range();
        let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

        // PLAN-117 W5: `@exports { $host, $session: mut }` at FILE scope declares
        // which page-global cells this file publishes. Same clause, same parser
        // (`parse_exports_block`) and the same `: mut` marker as a `@template`
        // body's `@exports` (FEAT-115) — learn one, you know the other.
        //
        // Before this the clause parsed at file scope and did NOTHING: no
        // FormMatch, no error, silently dropped.
        if name == "exports" {
            let block = directive.syntax().text().to_string();
            crate::syntax::events::form_compiler::parse_exports_block(
                &block,
                &mut file.file_exports,
                &mut file.diagnostics,
                span.into(),
            );
            continue;
        }

        // Global imports are registry-DERIVED (FEAT-118): a directive whose macro
        // declares `%imports { global: true }` produces an ImportAst. `@import` is
        // no longer privileged in Rust — `%macro import` (presets.st) declares the
        // effect. The literal `"import"` fallback covers the bootstrap parse
        // (`registry: None`), where stdlib's own `@import`s must still resolve
        // before the registry that would describe them exists. Mirrors the
        // registry-derived `is_body_bearing` predicate.
        let is_global_import =
            name == "import" || registry.is_some_and(|r| r.is_global_import(&name));
        let is_namespaced_import = registry.is_some_and(|r| r.is_namespaced_import(&name));
        if is_global_import || is_namespaced_import {
            if let Some(path) = directive.inline_args().next().and_then(|a| a.value_text()) {
                let path = path.trim_matches('"').trim_matches('\'').to_string();
                // `@use` (namespaced) derives the namespace its loaded defs
                // register under from the module ref; `@import` (global) merges
                // flat (`namespace: None`). `@use` additionally parses its
                // invocation tail (`as <alias>`, `only (...)`, `hiding (...)`)
                // for the per-file ImportScope (FEAT-118 FUP-053).
                let namespace = if is_namespaced_import {
                    Some(crate::metasystem::module::Namespace::from_module_ref(&path))
                } else {
                    None
                };
                let (alias, only, hiding) = if is_namespaced_import {
                    parse_use_clause_tail(directive.inline_tail_text().as_deref())
                } else {
                    (None, Vec::new(), Vec::new())
                };
                file.imports.push(ImportAst {
                    path,
                    namespace,
                    alias,
                    only,
                    hiding,
                    span,
                });
            }
            continue;
        }

        match name.as_str() {
            "pattern" => {
                if let Some(pattern) = convert_cst_directive_to_pattern(&directive, source) {
                    file.patterns.push(pattern);
                }
            }
            "preset" | "easing" | "scroll" | "animation" => {
                if let Some(preset) = convert_cst_directive_to_preset(&directive, source) {
                    file.presets.push(preset);
                }
            }
            // Plain CSS at-rule blocks authored at top level. The CSS emitter has no
            // structured at-rule IR, so capture verbatim source and pass it through as raw
            // CSS (BUG-087). Without this they were silently dropped, making breakpoint
            // responsiveness impossible.
            n if HOST_CSS_PASSTHROUGH_AT_RULES.contains(&n) => {
                let raw = directive.syntax().text().to_string();
                let normalized = normalize_raw_css_at_rule(&raw);
                file.raw_css_blocks.push(crate::parser::ast::RawCssBlock {
                    source: normalized,
                    span,
                });
            }
            _ => {
                // FormMatch extraction handled by event parser above. If the name is
                // not known to the registry either, the directive is silently dropped —
                // surface a warning so authors catch typos.
                if registry.is_some_and(|r| r.get_forms_for_directive(&name).is_empty()) {
                    // BUG-301 — a QUALIFIED name is an error, not a warning.
                    //
                    // `@typo/badge` says "the directive `badge`, from the module
                    // `typo`". The qualifier is a checkable claim: either that
                    // module was imported or it was not. Nothing was, so this
                    // cannot be anything but a mistake — unlike a bare `@foo`,
                    // which a not-yet-loaded registry might still explain.
                    //
                    // As a warning it shipped: `build` exited 0 and WROTE the
                    // page with the directive silently removed, so whatever the
                    // author asked for simply did not happen and the deploy
                    // reported success. There is no program for which "I did not
                    // recognize this, so I deleted it" is the intended outcome.
                    match unknown_directive_diagnostic(&name, span) {
                        Some(d) => file.diagnostics.push(d),
                        None => unclaimed_qualified.push((name.clone(), span)),
                    }
                }
            }
        }
    }

    // Process meta definitions (%primitive, %macro, etc.)
    for meta_def in root_node.meta_defs() {
        // FEAT-118 M2: `%module` / `%public` / `%reexport` are MODULE.st manifest
        // clauses, not registry defs — route them to the file's module_manifest.
        if let Some(keyword) = meta_def.keyword_text() {
            match keyword.as_str() {
                "module" | "public" | "reexport" => {
                    apply_manifest_clause(&mut file, &meta_def, &keyword, source);
                    continue;
                }
                "using" => {
                    apply_using_hook(&mut file, &meta_def, source);
                    continue;
                }
                _ => {}
            }
        }
        if let Some(meta) = convert_cst_meta_def(&meta_def, source) {
            file.meta_defs.push(meta);
        }
    }

    // BUG-301 — a qualified directive whose qualifier binds NOTHING.
    //
    // Deferred to here on purpose: the checks above run before `file.imports`
    // is populated, so they cannot tell `@b/badge` (bound by `@use "…" as b`,
    // a working feature) from `@nosuch/badge` (a typo). Judging there flagged
    // three legitimate files; not judging at all let the typo through in
    // silence, and `build` shipped a page with the directive deleted.
    //
    // By this point the imports ARE known, so the question is answerable:
    // either some import binds the qualifier or none does.
    //
    // NB this deliberately does NOT duplicate the visibility rules in
    // `check_import_visibility` (E0926/E0927) — that pass runs over matched
    // forms and never sees a directive that matched nothing, which is exactly
    // the case here. This answers only the binding question, and only for names
    // no form claimed.
    for (name, span) in std::mem::take(&mut unclaimed_qualified) {
        let Some((qualifier, leaf)) = name.split_once('/') else {
            continue;
        };
        // Bound by an `as` alias, or by the trailing segment of an imported
        // path (`@use "stdlib/md"` binds `md`) — the two spellings
        // `ImportScope::resolve_qualifier` accepts.
        let bound = file.imports.iter().any(|i| {
            i.alias.as_deref() == Some(qualifier)
                || i.path
                    .trim_end_matches(".st")
                    .rsplit('/')
                    .next()
                    .is_some_and(|seg| seg == qualifier)
        });
        if bound {
            // The qualifier resolves; any remaining problem (a missing leaf, a
            // `hiding` violation) belongs to the visibility pass, which has the
            // module contents this one does not.
            continue;
        }
        file.diagnostics.push(
            crate::diagnostics::Diagnostic::error(
                crate::diagnostics::DiagnosticCode::E0926,
                format!("`{qualifier}` is not a known module qualifier in `@{name}`"),
            )
            .with_span(span.into())
            .with_hint(format!(
                "`@{name}` reads as `{leaf}` from a module `{qualifier}`, but no import binds \
                 that name — so the directive would be dropped and the page would build \
                 without it. Add `@use \"…\" as {qualifier}`, or fix the spelling."
            )),
        );
    }

    // PLAN-144 W1 / BUG-344: a bare `&name(…)` at FILE scope — no body, no
    // enclosing selector — is consumed by nothing. It used to vanish without a
    // trace while the build reported success, which is the same silence the
    // markup spelling produced. Refuse it, and point at the two spellings that
    // work.
    for eref in root_node.element_ref_stmts() {
        if eref.arg_list().is_none() {
            continue; // a bare `&name` is a scope reference, never a call (Q2)
        }
        // `&name(…) { … }` with a real CSS selector is a legitimate body-slot
        // invocation. But a call written as a STATEMENT swallows whatever
        // follows it as its "selector": the CST for `&c("x")\n<main>…</main>\n
        // .page { … }` puts the entire page — markup AND the next rule — inside
        // this node. So the orphan call does not merely vanish; it eats the rest
        // of the file. A selector containing markup is the tell.
        let selector_text = eref.selector_text().unwrap_or_default();
        let swallowed_markup = selector_text.contains('<');
        if eref.body().is_some() && !swallowed_markup {
            continue; // `&name(…) { … }` is a body-slot invocation, handled above
        }
        let name = eref.name_text().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let range = eref.syntax().text_range();
        file.diagnostics.push(
            crate::diagnostics::Diagnostic::error(
                crate::diagnostics::DiagnosticCode::E0900,
                format!(
                    "template invocation `&{name}(…)` at file scope is consumed by \
                     nothing — a directive binds through a SELECTOR"
                ),
            )
            .with_span(crate::diagnostics::SourceSpan::new(
                range.start().into(),
                range.end().into(),
            ))
            .with_hint(format!(
                "put it where it should render: `` `&{name}(…)` `` inside markup, \
                 or `.x {{ &{name}(…); }}` in a selector scope"
            )),
        );
    }

    // Process top-level HTML element literals (PLAN-023 W1).
    for html in root_node.html_elements() {
        let (skeleton, holes) = html.skeleton();
        let text_range = html.syntax().text_range();
        let span = SourceSpan::new(text_range.start().into(), text_range.end().into());
        // I4 / gh-8: a directive written INSIDE an HTML literal (`<div> @scroll …
        // </div>`) cannot bind — directives attach through SELECTORS, never through
        // nesting (a directive must target elements that may not exist yet). Emit
        // E0900 naming the shape + showing the selector rewrite, and strip the
        // directive from the skeleton so compiler input is never painted as text.
        if let Some((rel, end_rel, name)) = crate::html::find_nested_directive(&skeleton) {
            let dir_start = span.start + rel;
            let dir_end = span.start + end_rel;
            file.diagnostics.push(
                crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0900,
                    format!(
                        "directive `@{name}` cannot be nested inside HTML markup — a directive \
                         binds through a SELECTOR, never through nesting"
                    ),
                )
                .with_span(crate::diagnostics::SourceSpan::new(dir_start, dir_end))
                .with_hint(format!(
                    "move it into a selector scope instead: `.x {{ @{name} … }} ` — \
                     the element already exists once its selector is bound, so the directive \
                     can attach to it"
                )),
            );
        }
        // PLAN-144 W1 / BUG-344: a template invocation is a directive too, so
        // the same rule applies — it binds through a SELECTOR, never through
        // nesting. Written into markup it used to be HTML-escaped and shipped
        // as body copy with the build reporting success. The COMPONENT HOLE
        // (`` `&card("T")` ``) is the spelling that works in a markup
        // position, so the hint offers both rewrites.
        if let Some((rel, end_rel, name)) = crate::html::find_nested_component_call(&skeleton) {
            let call_start = span.start + rel;
            let call_end = span.start + end_rel;
            file.diagnostics.push(
                crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0900,
                    format!(
                        "template invocation `&{name}(…)` cannot be nested inside HTML \
                         markup — a directive binds through a SELECTOR, never through nesting"
                    ),
                )
                .with_span(crate::diagnostics::SourceSpan::new(call_start, call_end))
                .with_hint(format!(
                    "interpolate it as a component hole: `` `&{name}(…)` `` — or move it \
                     into a selector scope: `.x {{ &{name}(…); }}`"
                )),
            );
        }
        let skeleton = crate::html::strip_nested_component_calls(&skeleton);
        let skeleton = crate::html::strip_nested_directives(&skeleton);
        file.html_blocks.push(crate::parser::ast::HtmlBlockAst {
            skeleton,
            holes,
            span,
            injection: Default::default(),
        });
    }

    // Sweep: every FORM_REF the scope conversion did NOT carry — file-root
    // splices and splices inside DIRECTIVE bodies (`@form motion --reveal {
    // --fade-in; … }`, SIP-001c's composition case). Without this, a typo'd
    // nested splice is recognized by the parser and then accepted with no
    // validation — the same silent-accept class as the original bug, one
    // level down. Dedup by span against what the scopes already carry.
    {
        fn carried_spans(
            scopes: &[crate::parser::ast::NestedScope],
            out: &mut std::collections::HashSet<(usize, usize)>,
        ) {
            for scope in scopes {
                out.extend(scope.form_refs.iter().map(|r| (r.span.start, r.span.end)));
                carried_spans(&scope.nested_scopes, out);
            }
        }
        let mut carried = std::collections::HashSet::new();
        for scope in &file.scopes {
            carried.extend(scope.form_refs.iter().map(|r| (r.span.start, r.span.end)));
            carried_spans(&scope.nested_scopes, &mut carried);
        }
        for node in root_node
            .syntax()
            .descendants()
            .filter(|n| n.kind() == crate::syntax::cst::SyntaxKind::FORM_REF)
        {
            let range = node.text_range();
            let key = (usize::from(range.start()), usize::from(range.end()));
            if carried.contains(&key) {
                continue;
            }
            let name = crate::syntax::cst::FormRef::cast(node.clone())
                .and_then(|fr| fr.name())
                .map(|t| t.text().to_string());
            let Some(name) = name else { continue };
            let args = node
                .children()
                .find(|n| n.kind() == crate::syntax::cst::SyntaxKind::ARG_LIST)
                .map(|n| {
                    let t = n.text().to_string();
                    t.trim_start_matches('(')
                        .trim_end_matches(')')
                        .trim()
                        .to_string()
                });
            file.form_refs.push(crate::parser::ast::FormRefUse {
                name,
                args,
                span: SourceSpan::new(key.0, key.1),
            });
        }
    }

    // NOTE: form-splice validation (BUG-241) does NOT run here — it runs in
    // the parse entries (`parse`, `parse_without_registry`), because body
    // FRAGMENTS (`parse_body_fragment`) reach this conversion too and cannot
    // see the enclosing file's form declarations.

    file
}

/// Normalize a captured CSS at-rule block (BUG-087) for verbatim emission.
///
/// Spacetime accepts two `@media` surfaces:
///   `@media (max-width: 768px) { ... }`   (standard CSS)
///   `@media("max-width: 768px") { ... }`  (parenthesized-string form)
/// The second must be normalized to the first so the browser parses it. We also strip a
/// trailing `;` the directive lexer may have appended. The inner declarations are already
/// valid CSS text and pass through unchanged.
fn normalize_raw_css_at_rule(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches(';').trim_end();

    // Find the at-rule head (everything up to the first `{`).
    let Some(brace) = trimmed.find('{') else {
        return trimmed.to_string();
    };
    let (head, body) = trimmed.split_at(brace);
    let head = head.trim();

    // Normalize the `@media("query")` / `@media('query')` string form to `@media query`.
    let normalized_head = if let Some(open) = head.find('(') {
        let prefix = head[..open].trim_end(); // e.g. `@media`
        let inner = head[open + 1..].trim_end();
        let inner = inner.strip_suffix(')').unwrap_or(inner).trim();
        // Unwrap a single quoted string argument: "max-width: 768px" -> max-width: 768px
        let unquoted = inner
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| inner.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')));
        match unquoted {
            // String form: re-wrap in standard parens.
            Some(q) => format!("{} ({})", prefix, q.trim()),
            // Already standard `@media (cond)` form: keep as authored.
            None => head.to_string(),
        }
    } else {
        head.to_string()
    };

    format!("{} {}", normalized_head, body)
}

/// Recursively assign a FormMatch to the most specific nested scope by span containment.
/// Returns true if the match was assigned to a nested scope.
/// Recursively count the FormMatches nested inside a match's captures.
///
/// A parent macro (e.g. `@stage { … }`) captures its body directives as a
/// `CapturedValue::Block` (the `$children*` capture) or as `macro_calls` inside a
/// `Named` capture. When the stdlib-only parse runs, project-local child directives
/// (e.g. a site's `@layer`) are NOT yet known forms, so they never enter the block;
/// the augmented parse (stdlib + user macros) DOES capture them. Comparing this count
/// between the two parses is how `rematch_with_user_macros_in` detects a parent match
/// whose children were ENRICHED by user macros and must replace the stale one.
fn count_nested_matches(fm: &FormMatch) -> usize {
    fn count_in_value(v: &CapturedValue) -> usize {
        match v {
            CapturedValue::Block(children) => {
                children.iter().map(|c| 1 + count_nested_matches(c)).sum()
            }
            CapturedValue::Named(map) => map.values().map(count_in_value).sum(),
            CapturedValue::Array(items) => items.iter().map(count_in_value).sum(),
            _ => 0,
        }
    }
    fm.captures.values().map(count_in_value).sum()
}

/// Replace, by exact span, an existing FormMatch with an enriched one throughout the
/// AST (flat `matches` list + every scope/nested-scope `matches`). Returns true if a
/// match was replaced anywhere. Used to swap a parent match (e.g. `@stage`) whose
/// stdlib-parse children block was missing project-local child directives for the
/// augmented-parse version that captured them. The selector is preserved from the
/// match being replaced.
fn replace_match_by_span(span: SourceSpan, enriched: &FormMatch, ast: &mut StFile) -> bool {
    fn replace_in(matches: &mut [FormMatch], span: SourceSpan, enriched: &FormMatch) -> bool {
        let mut replaced = false;
        for m in matches.iter_mut() {
            if m.span.start == span.start && m.span.end == span.end {
                let selector = m.selector.clone();
                *m = enriched.clone();
                m.selector = selector;
                replaced = true;
            }
        }
        replaced
    }
    fn replace_in_nested(
        nested: &mut [NestedScope],
        span: SourceSpan,
        enriched: &FormMatch,
    ) -> bool {
        let mut replaced = false;
        for n in nested.iter_mut() {
            replaced |= replace_in(&mut n.matches, span, enriched);
            replaced |= replace_in_nested(&mut n.nested_scopes, span, enriched);
        }
        replaced
    }

    let mut replaced = replace_in(&mut ast.matches, span, enriched);
    for scope_ast in ast.scopes.iter_mut() {
        replaced |= replace_in(&mut scope_ast.matches, span, enriched);
        replaced |= replace_in_nested(&mut scope_ast.nested_scopes, span, enriched);
    }
    replaced
}

fn assign_match_to_nested_scopes(fm: &FormMatch, nested_scopes: &mut [NestedScope]) -> bool {
    for nested in nested_scopes.iter_mut() {
        if nested.span.start <= fm.span.start && fm.span.end <= nested.span.end {
            // Try deeper nesting first
            if !assign_match_to_nested_scopes(fm, &mut nested.nested_scopes) {
                nested.matches.push(fm.clone());
            }
            return true;
        }
    }
    false
}

/// Precompute `composed_selector` on every nested scope in a subtree (BUG-206).
///
/// Walks the nested-scope chain composing each leaf with its parent via
/// [`crate::syntax::compose_selector`], storing the full path (e.g.
/// `.counter > .controls > button.inc`) on `composed_selector`. An empty `root`
/// means the parent is a `Construct` (template) region: its nested scopes address
/// the INSTANCE (implicit), so the first level composes standalone (its leaf) —
/// exactly how template CSS emits each nested region standalone.
fn compose_nested_selectors(root: &str, nested: &mut [NestedScope]) {
    for n in nested {
        let composed = if root.is_empty() {
            n.selector.clone()
        } else {
            compose_selector(root, &n.selector)
        };
        n.composed_selector = composed.clone();
        compose_nested_selectors(&composed, &mut n.nested_scopes);
    }
}

/// The `composed_selector` of the DEEPEST nested scope containing `fm` (by span),
/// or `None` when the match sits directly in the parent (not in any nested
/// `.sel{}`). Reads the precomputed field — never recomposes — so directive
/// selector-assignment and CSS emit share one composition truth (BUG-206).
fn nearest_nested_composed_selector(
    fm: &FormMatch,
    nested_scopes: &[NestedScope],
) -> Option<String> {
    for nested in nested_scopes {
        if nested.span.start <= fm.span.start && fm.span.end <= nested.span.end {
            return nearest_nested_composed_selector(fm, &nested.nested_scopes)
                .or_else(|| Some(nested.composed_selector.clone()));
        }
    }
    None
}
/// FEAT-115 S3: reconstruct a `@template` body's HTML verbatim from the CST,
/// keeping only the HTML element runs (and bare `$x`/`` `$x` `` holes that sit
/// inside HTML) and DROPPING the construct children — state declarations
/// (`$x type: v;`), nested `.sel {}` scope blocks, and body directives
/// (`@exports`, `@each`, root injections). This replaces reify's text-SEGMENTATION
/// `html`, which leaked those construct segments into the rendered DOM (a live bug:
/// a `.sel { @on … }` block rendered as a literal CSS text node + a spurious
/// `display:contents` wrapper). Source order is preserved by walking children.
///
/// A bare hole (`$x` / `` `$x` ``) can surface as a body-direct `VARIABLE_REF`
/// child only when the CST mis-splits adjacent sibling elements; it is KEPT
/// (it is HTML content), distinguished from a state decl by shape (a decl carries
/// a type/`:`/trailing `;`).
/// Find the byte offset of the first TOP-LEVEL `:` in a parameter-list source
/// slice (the `(…)` after a `@template`/`@editable-*` name), or `None`.
///
/// "Top-level" = not inside a string literal (`"…"` / `'…'`) and not nested inside
/// brackets (`(` `[` `{`). This skips a `:` that legitimately appears inside a
/// default string value (`$t = "a:b"`) or a bracketed expression, so only a
/// genuine param-separator `:` (the `$a: "x"` mistake) is reported. Backs E0929.
fn first_top_level_colon(s: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_str: Option<char> = None;
    let mut prev_backslash = false;
    for (i, c) in s.char_indices() {
        match in_str {
            Some(q) => {
                if c == q && !prev_backslash {
                    in_str = None;
                }
                prev_backslash = c == '\\' && !prev_backslash;
            }
            None => {
                match c {
                    '"' | '\'' => {
                        in_str = Some(c);
                        prev_backslash = false;
                    }
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => depth -= 1,
                    // Ignore the outer arg-list parens: the whole slice is wrapped in
                    // `( … )`, so the param body sits at depth 1. A separator `:` is
                    // therefore top-level when depth <= 1.
                    ':' if depth <= 1 => return Some(i),
                    _ => {}
                }
            }
        }
    }
    None
}

fn reconstruct_template_body_html(body: &crate::syntax::cst::Body) -> String {
    use crate::syntax::cst::{AstNode, SyntaxKind};
    let node = body.syntax();
    let range = node.text_range();
    let base: usize = range.start().into();
    let full = node.text().to_string(); // includes the outer `{` … `}`

    // Collect the byte ranges (relative to `full`) of every CONSTRUCT child — the
    // segments that are NOT HTML and must be removed: state declarations
    // (`$x type: v;`), nested `.sel {}` scope blocks, and body directives
    // (`@exports`, `@each`, root injections). Everything else (HTML element runs and
    // bare `$x`/`` `$x` `` holes inside HTML) is HTML content and is kept verbatim.
    //
    // Span-subtraction on the verbatim source (rather than re-joining CST child text)
    // is robust to the CST mis-splitting adjacent sibling elements: a `<dd ...>` open
    // tag the CST drops into trivia is still present in `full` and survives, because
    // we only ever CUT construct spans — we never reconstruct HTML from child nodes.
    let mut cuts: Vec<(usize, usize)> = Vec::new();

    for ch in node.children() {
        let keep = match ch.kind() {
            SyntaxKind::HTML_ELEMENT => true,
            SyntaxKind::VARIABLE_REF => {
                // A bare hole (`$x`) is HTML content; a state decl (`$x type: v;`) is a
                // construct. Distinguish by shape (a decl carries a type/`:`/trailing `;`).
                let t = ch.text().to_string();
                !(t.contains(':')
                    || t.trim_end().ends_with(';')
                    || t.split_whitespace().count() > 1)
            }
            _ => false, // SCOPE_BLOCK, DIRECTIVE, and any other construct child
        };
        if !keep {
            let r = ch.text_range();
            let s = usize::from(r.start()).saturating_sub(base);
            let e = usize::from(r.end()).saturating_sub(base);
            if e <= full.len() {
                cuts.push((s, e));
            }
        }
    }
    cuts.sort_unstable();

    // Emit `full` minus the cut ranges (byte-indexed; cuts fall on token boundaries
    // so they are UTF-8 safe), then strip the enclosing braces + trim.
    let bytes = full.as_bytes();
    let mut kept: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(&(_, e)) = cuts.iter().find(|&&(s, e)| i >= s && i < e) {
            i = e;
            continue;
        }
        kept.push(bytes[i]);
        i += 1;
    }
    let out = String::from_utf8_lossy(&kept);
    out.trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim()
        .to_string()
}
/// PLAN-039 FEAT-115 S3c: a body-root content injection (`target <- $x;`) is sugar
/// for a selector-scoped reactive binding on the SAME unified path that serves
/// `.sel { text <- $x }` everywhere else — so transform it into a synthesized
/// `NestedScope` on the template scope, retiring the parallel `ContentInjection`
/// reify field + the factory's bodyInjections forEach.
///
/// Target classification mirrors `reify_inject` exactly:
///  - `[slot="v"] <- $x`      → nested scope `[slot="v"]`, `text <- $x` (textContent)
///  - `[class="v"] <- $x`     → nested scope `[class="v"]` (any bracket selector,
///                              the DESCENDANT it matches), `text <- $x`
///  - `text <- $x`            → the root element's textContent (`<root> { text <- $x }`)
///  - `attr <- $x` (e.g. src) → the root element's attribute (`<root> { attr <- $x }`,
///                              is_injection so it routes to setAttribute)
///
/// `root_sel` is a selector that matches the instance root (the template scope's
/// first HTML element's leading class/tag); root-targeting forms bind against it.
/// The value keeps any trailing `| filter` pipe verbatim (the binding emitter
/// honours `expr | filter` via ST.filter), matching the injection filter path.
/// BUG-130: a COLLECTION-REF selector block `&name[] .sel { @each(…){ &child } }`
/// inside a @template body is parsed as an ELEMENT_REF_STMT (with a body), NOT a
/// SCOPE_BLOCK — so `body.nested_scopes()` (SCOPE_BLOCK only) never harvests it and
/// the interior @each is dropped (renders nothing). Harvest each ELEMENT_REF_STMT
/// that carries a `{ body }` + a selector into a nested scope keyed by its selector
/// (e.g. `.cards`), so the span-containment match assignment routes the interior
/// @each FormMatch into it exactly like the plain `.sel { @each }` form. The
/// collection-ref binding itself (`&name[]`) is recorded as the scope's
/// `collection_ref` so the builder can wire the MutationObserver state array.
/// A bodyless element-ref (`&hdr .header;`) is left untouched (no body to descend).
fn collection_ref_scopes_from_body(body: &crate::syntax::cst::Body) -> Vec<NestedScope> {
    use crate::syntax::cst::AstNode;
    let mut scopes = Vec::new();
    for ers in body.element_ref_stmts() {
        // Only the body-bearing form is a scope; `&name sel;` (no body) is a plain ref.
        let Some(ers_body) = ers.body() else { continue };
        let Some(selector) = ers.selector_text() else {
            continue;
        };
        // The `[]` collection marker is stripped by `ElementRefStmt::selector_text`
        // itself, so this is the real CSS target (`.cards`) the @each renders into.
        let selector = selector.trim().to_string();
        if selector.is_empty() {
            continue;
        }
        // The collection-ref name (`&cards[]` -> "cards"); is_collection is implied by
        // the `[]` marker, which the selector text carries as a leading `[]` token run.
        let ref_name = ers.name_text();
        let body_range = ers_body.text_range();
        let span = SourceSpan::new(body_range.start().into(), body_range.end().into());
        scopes.push(NestedScope {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector,
            composed_selector: String::new(),
            behavior: Default::default(),
            css_declarations: Vec::new(),
            form_refs: Vec::new(),
            nested_scopes: Vec::new(),
            matches: Vec::new(),
            collection_ref: ref_name,
            span,
        });
    }
    scopes
}

/// FEAT-142 WAVE A: harvest a selector-less, body-bearing element-ref statement
/// (`&name { @directive... }`) as an entity scope. The entity name is stored in the
/// scope's `selector` field (e.g. "evernet") and its component directives are
/// assigned to `matches` by span containment in `cst_to_stfile`. Bodyless refs,
/// refs with selectors, and template invocations (`&name(){}`) are left untouched.
// `entity_selector` / `entity_name_from_selector` now live in `parser::entity`
// (the single typed home for the entity + facet-path model) and are re-exported
// at the top of this module.

/// FEAT-142 WAVE A: harvest a selector-less, body-bearing element-ref statement
/// (`&name { @directive... }`) as an entity scope. The entity name is preserved
/// Harvest entity scopes (`&name { @c }`) from a construct body (a `@template` /
/// body-bearing construct). Returns, per entity, its synthetic-selector NestedScope
/// AND the name+span so the caller can synthesize the `entity-scope-impl`
/// registration FormMatch (identical to the top-level path). This is the nested
/// analog of the top-level harvest — it lets `&name { @peak }` live INSIDE a
/// `@template`/`@each` delegate body (row-scoped entities, FEAT-142 Wave E).
fn entity_scopes_from_body(
    body: &crate::syntax::cst::Body,
) -> Vec<(NestedScope, String, SourceSpan)> {
    use crate::syntax::cst::AstNode;
    let mut scopes = Vec::new();
    for ers in body.element_ref_stmts() {
        // Must have a `{ ... }` body.
        let Some(ers_body) = ers.body() else { continue };
        // Must NOT have parenthesized args (that's a template invocation).
        if ers.arg_list().is_some() {
            continue;
        }
        // Must be selector-less (empty or absent selector).
        let selector = ers.selector_text().unwrap_or_default().trim().to_string();
        if !selector.is_empty() {
            continue;
        }
        // A DOTTED ref (`&name.component { … }`) is a facet path, not a base entity
        // (reviewer P2 — same guard as the top-level harvest).
        if ers.element_ref().map(|e| e.path().len()).unwrap_or(1) > 1 {
            continue;
        }
        let Some(name) = ers.name_text() else {
            continue;
        };
        let body_range = ers_body.text_range();
        let span = SourceSpan::new(body_range.start().into(), body_range.end().into());
        let ns = NestedScope {
            kind: crate::parser::ast::ScopeKind::Selector,
            selector: entity_selector(&name),
            composed_selector: String::new(),
            behavior: Default::default(),
            css_declarations: Vec::new(),
            form_refs: Vec::new(),
            nested_scopes: Vec::new(),
            matches: Vec::new(),
            collection_ref: None,
            span,
        };
        scopes.push((ns, name, span));
    }
    scopes
}

fn injection_scopes_from_body(body: &crate::syntax::cst::Body, root_sel: &str) -> Vec<NestedScope> {
    use crate::syntax::cst::{AstNode, SyntaxKind};
    let mut scopes = Vec::new();
    for ch in body.syntax().children() {
        if ch.kind() != SyntaxKind::CSS_PROPERTY {
            continue;
        }
        let text = ch.text().to_string();
        // Only `<-` arrow declarations are injections; a `:` declaration is a CSS prop.
        let Some((target_raw, value_raw)) = text.split_once("<-") else {
            continue;
        };
        let target = target_raw.trim();
        let value = value_raw.trim().trim_end_matches(';').trim();
        if target.is_empty() || value.is_empty() {
            continue;
        }
        // (selector, property, is_injection) per the reify_inject classification.
        let (selector, property, is_injection): (String, &str, bool) =
            if target == "text" || target == "content" {
                (root_sel.to_string(), "text", false)
            } else if target.starts_with('[') && target.ends_with(']') {
                // [slot="v"] / [class="v"] / [attr="v"] — the DESCENDANT it matches, its
                // textContent (verbatim selector preserved; BUG-061/063).
                (target.to_string(), "text", false)
            } else {
                // A bare attribute name on the root (`src <- $u`) — setAttribute.
                (root_sel.to_string(), "", true)
            };
        let property = if property.is_empty() {
            target
        } else {
            property
        };
        let span = {
            let r = ch.text_range();
            SourceSpan::new(r.start().into(), r.end().into())
        };
        scopes.push(NestedScope {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector,
            composed_selector: String::new(),
            behavior: Default::default(),
            css_declarations: vec![CssDeclaration {
                property: property.to_string(),
                value: value.to_string(),
                is_injection,
                span,
            }],
            form_refs: Vec::new(),
            nested_scopes: Vec::new(),
            matches: Vec::new(),
            collection_ref: None,
            span,
        });
    }
    scopes
}
/// FEAT-115 S5c: body-ROOT reactive `:` declarations (`​.c--open: $open;` class
/// toggle, `aria-expanded: $open;` self-prop) become synthesized nested scopes on
/// the SAME unified scope path that already serves the `.sel`-wrapped forms
/// (`.c { .c--open: $open }`) — retiring reify's parallel `directives` field AND
/// fixing the body-root render, which otherwise leaks into `staticProperties` as
/// a mangled static prop (the body-directives.test.st failures).
///
/// Only REACTIVE declarations (value is a `$signal`) are lifted; a plain CSS prop
/// (`color: red;`) stays HTML/CSS and is handled by the html/css path. Mirrors the
/// working wrapped shape exactly: a nested scope whose `selector` is the instance
/// ROOT and whose css_declaration is `{ property, value }` (class toggle when
/// `property` starts with `.`, else an attribute self-prop via `is_injection`).
fn directive_scopes_from_body(body: &crate::syntax::cst::Body, root_sel: &str) -> Vec<NestedScope> {
    use crate::syntax::cst::{AstNode, SyntaxKind};
    let mut scopes = Vec::new();
    for ch in body.syntax().children() {
        if ch.kind() != SyntaxKind::CSS_PROPERTY {
            continue;
        }
        let text = ch.text().to_string();
        // A `<-` arrow is an injection (handled by injection_scopes_from_body); a `:`
        // declaration is a CSS-surface prop. Split on the FIRST `:` only.
        if text.contains("<-") {
            continue;
        }
        let Some((prop_raw, value_raw)) = text.split_once(':') else {
            continue;
        };
        let property = prop_raw.trim();
        let value = value_raw.trim().trim_end_matches(';').trim();
        if property.is_empty() || value.is_empty() {
            continue;
        }
        // Only REACTIVE declarations are directives. A plain CSS value (`red`, `12px`)
        // is left to the html/css path. A `$signal` (optionally `| filter`) is reactive.
        if !value.starts_with('$') {
            continue;
        }
        // Class toggle: `.cls--mod: $sig` (property is a class selector). Self-prop:
        // `aria-expanded: $sig` (property is an attribute name) — routed via setAttribute
        // like an injection. Both bind on the instance ROOT selector.
        let is_injection = !property.starts_with('.');
        let span = {
            let r = ch.text_range();
            SourceSpan::new(r.start().into(), r.end().into())
        };
        scopes.push(NestedScope {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector: root_sel.to_string(),
            composed_selector: String::new(),
            behavior: Default::default(),
            css_declarations: vec![CssDeclaration {
                property: property.to_string(),
                value: value.to_string(),
                is_injection,
                span,
            }],
            form_refs: Vec::new(),
            nested_scopes: Vec::new(),
            matches: Vec::new(),
            collection_ref: None,
            span,
        });
    }
    scopes
}
/// The selector that matches a template instance's ROOT element — used to bind
/// root-targeting injections (`text <- $x`, `src <- $x`). Derived from the first
/// HTML element of the reconstructed body: its leading `class="…"` first token, else
/// its tag name. Falls back to `*` (matches the root the factory mounts).
fn template_root_selector(html: &str) -> String {
    let trimmed = html.trim_start();
    // class="foo bar" → `.foo`
    if let Some(ci) = trimmed.find("class=\"") {
        let rest = &trimmed[ci + 7..];
        if let Some(end) = rest.find('"')
            && let Some(first) = rest[..end].split_whitespace().next()
            && !first.is_empty()
        {
            return format!(".{}", first);
        }
    }
    // else the opening tag name
    if let Some(stripped) = trimmed.strip_prefix('<') {
        let tag: String = stripped
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if !tag.is_empty() {
            return tag;
        }
    }
    "*".to_string()
}
/// PLAN-039 FEAT-115 S1: derive the factory `states` payload from a template
/// scope's surfaced `local-state` matches (World A — the AST truth) instead of
/// the parallel `reify_state` (World B) classifier. A `local-state` match carries
/// captures `name` (Ident, `$`-stripped), `type` (TypeRef), `value` (Expr); the
/// typed-initial conversion is byte-for-byte the value-semantics of `reify_state`
/// (bool → Bool, number → Number(parse|0), else → String(unquoted)); a missing
/// value becomes `Expr("null")`. Walks the scope's own matches AND its nested
/// scopes (a state decl can sit directly in the body or inside a `.sel{}` region).
fn scope_states(scope: &ScopeBlock) -> Vec<ComponentStateDecl> {
    // Collect (source_span_start, decl) so the payload preserves authored order
    // even when state decls are split between the body root and inner `.sel{}`
    // regions — byte-for-byte the source order `reify_state` produced.
    let mut spanned: Vec<(usize, ComponentStateDecl)> = Vec::new();
    collect_scope_states_from_matches(&scope.matches, &mut spanned);
    collect_scope_states_nested(&scope.nested_scopes, &mut spanned);
    spanned.sort_by_key(|(start, _)| *start);
    spanned.into_iter().map(|(_, d)| d).collect()
}
/// PLAN-039 FEAT-115 S3d: derive the factory `refs` payload (template invocations
/// in a template body) from the template scope's surfaced invoke matches
/// (`template-invoke-bare` / `template-invoke-named`) — World A — instead of the
/// reify `parse_template_ref` classifier (World B). The matches carry `name`
/// (Ident), `args` (Array of Expr), and for the named form `ref` (Ident); the
/// `TemplateRef` shape is byte-for-byte what reify produced (args as raw Expr
/// strings). Source-span sorted to preserve authored order. The factory renders
/// these via its existing mount-time forEach (the correct render time for an
/// invocation that CREATES a subtree); only the data source moves off reify.
///
/// NB: dynamic dispatch `&$w(…)` has no flat invoke match (`&$name:ident` rejects
/// a `$`-name), so it is harvested separately from the body source text by the
/// caller (via `parse_template_ref`) and merged in.
/// Harvest the template-invocation refs from a slice of FormMatches (the
/// `template-invoke-bare` / `template-invoke-named` matches), as
/// `(span_start, TemplateRef)`. Shared by the scope-root and nested-scope walks.
fn invoke_refs_from_matches(
    matches: &[crate::syntax::FormMatch],
) -> Vec<(usize, crate::syntax::TemplateRef)> {
    use crate::syntax::TemplateRef;
    let mut out: Vec<(usize, TemplateRef)> = Vec::new();
    // Extract the comma-split arg expressions from an `args` capture (an Array of
    // Expr/String/Ident values). Shared by the direct-invoke and @each-invocation
    // paths.
    fn args_of(v: Option<&CapturedValue>) -> Vec<String> {
        match v {
            Some(CapturedValue::Array(items)) => items
                .iter()
                .filter_map(|x| match x {
                    CapturedValue::Expr(s) | CapturedValue::String(s) | CapturedValue::Ident(s) => {
                        Some(s.clone())
                    }
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
    for m in matches {
        match m.macro_name.as_str() {
            "template-invoke-bare" | "template-invoke-named" => {
                let (ref_name, is_collection) = if m.macro_name == "template-invoke-named" {
                    match cap_str(m, "ref") {
                        Some(r) => {
                            let coll = r.ends_with("[]");
                            (Some(r.trim_end_matches("[]").to_string()), coll)
                        }
                        None => (None, false),
                    }
                } else {
                    (None, false)
                };
                let Some(template_name) = cap_str(m, "name") else {
                    continue;
                };
                let (args, arg_names) =
                    crate::syntax::normalize_invocation_args(&args_of(m.captures.get("args")));
                out.push((
                    m.span.start,
                    TemplateRef {
                        ref_name,
                        template_name,
                        args,
                        is_collection,
                        arg_names,
                        span: m.span,
                    },
                ));
            }
            // BUG-130: a SELECTOR-scoped `@each($src as $x) { &item($x); }` stores
            // its child invocations in the @each match's `invocations` capture (an
            // Array of Named{name, args, ...}), NOT as separate template-invoke
            // matches. Harvest them so the parent template's refs include @each-
            // driven children (the builder navigator's structure tree). Marked
            // is_collection (they render once per data row).
            "each" | "each-filtered" => {
                if let Some(CapturedValue::Array(invocations)) = m.captures.get("invocations") {
                    for inv in invocations {
                        if let CapturedValue::Named(map) = inv {
                            let name = match map.get("name") {
                                Some(CapturedValue::String(s))
                                | Some(CapturedValue::Ident(s))
                                | Some(CapturedValue::Expr(s)) => Some(s.clone()),
                                _ => None,
                            };
                            if let Some(template_name) = name {
                                let ref_name = match map.get("ref") {
                                    Some(CapturedValue::Ident(s))
                                    | Some(CapturedValue::String(s)) => Some(s.clone()),
                                    _ => None,
                                };
                                let (args, arg_names) = crate::syntax::normalize_invocation_args(
                                    &args_of(map.get("args")),
                                );
                                out.push((
                                    m.span.start,
                                    TemplateRef {
                                        ref_name,
                                        template_name,
                                        args,
                                        is_collection: true,
                                        arg_names,
                                        span: m.span,
                                    },
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Recursively harvest invoke refs from a [`NestedScope`] subtree (its own
/// matches + every descendant nested scope). A `&child(…)` invocation inside an
/// `@each`/conditional/`.sel{}` block within a template body lands in a NESTED
/// scope, not the template's root matches — so without this descent the parent
/// template's `refs` would miss every `@each`-driven child (the builder
/// navigator would show an incomplete tree).
fn nested_scope_refs(
    ns: &crate::parser::ast::NestedScope,
) -> Vec<(usize, crate::syntax::TemplateRef)> {
    let mut out = invoke_refs_from_matches(&ns.matches);
    for child in &ns.nested_scopes {
        out.extend(nested_scope_refs(child));
    }
    out
}

fn scope_refs(scope: &ScopeBlock) -> Vec<(usize, crate::syntax::TemplateRef)> {
    // Root matches PLUS every nested scope (recursively): an @each body's
    // `&item($x)` is a template-invoke match in a NESTED scope of the template
    // body, which the root-only harvest missed (the navigator tree then dropped
    // every @each-driven child).
    let mut out = invoke_refs_from_matches(&scope.matches);
    for ns in &scope.nested_scopes {
        out.extend(nested_scope_refs(ns));
    }
    out
}
/// FEAT-115 S3d: harvest DYNAMIC template invocations `&$name(args)` from a span of
/// source text — the dispatch form whose template name is a `$`-signal, which has
/// no flat invoke match (the `&$name:ident` grammar rejects a `$`-name). Returns
/// `(span_start, TemplateRef{ template_name: "$name" … })` so the caller merges +
/// span-sorts them with the static/named refs. Mirrors reify's `&$`-name handling:
/// the `$name` is kept verbatim (the factory resolves it at mount); args are the
/// comma-split raw expressions. Only matches `&$` (NOT `&name` or `&a &b`), so it
/// never double-counts a static/named ref the scope matches already cover.
fn harvest_dynamic_refs(
    source: &str,
    span: SourceSpan,
) -> Vec<(usize, crate::syntax::TemplateRef)> {
    use crate::syntax::TemplateRef;
    let start = span.start.min(source.len());
    let end = span.end.min(source.len());
    let text = &source[start..end];
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] != b'&' {
            i += 1;
            continue;
        }
        // (1) DYNAMIC dispatch `&$name(args)` — a `$`-signal template name (no flat match).
        if bytes[i + 1] == b'$' {
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'.')
            {
                j += 1;
            }
            if j > name_start
                && j < bytes.len()
                && bytes[j] == b'('
                && let Some(close) = text[j..].find(')')
            {
                let (args, arg_names) = crate::syntax::normalize_invocation_args(
                    &split_invocation_args(&text[j + 1..j + close]),
                );
                out.push((
                    start + i,
                    TemplateRef {
                        ref_name: None,
                        template_name: format!("${}", &text[name_start..j]),
                        args,
                        is_collection: false,
                        arg_names,
                        span: SourceSpan::new(start + i, start + j + close + 1),
                    },
                ));
                i = j + close + 1;
                continue;
            }
        }
        // (2) COLLECTION ref `&name[] &tmpl(args)` — no flat match (the `[]` collection
        //     form). A NAMED ref `&name &tmpl(…)` (no `[]`) IS a flat invoke match
        //     (scope_refs covers it), so only harvest when a `[]` is present.
        let rn_start = i + 1;
        let mut k = rn_start;
        while k < bytes.len() && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_') {
            k += 1;
        }
        if k > rn_start && k + 1 < bytes.len() && bytes[k] == b'[' && bytes[k + 1] == b']' {
            let ref_name = text[rn_start..k].to_string();
            // After `[]`, skip whitespace, expect `&tmpl(args)`.
            let mut m = k + 2;
            while m < bytes.len() && bytes[m].is_ascii_whitespace() {
                m += 1;
            }
            if m < bytes.len() && bytes[m] == b'&' {
                let tn_start = m + 1;
                let mut t = tn_start;
                while t < bytes.len()
                    && (bytes[t].is_ascii_alphanumeric() || bytes[t] == b'_' || bytes[t] == b'-')
                {
                    t += 1;
                }
                if t > tn_start
                    && t < bytes.len()
                    && bytes[t] == b'('
                    && let Some(close) = text[t..].find(')')
                {
                    let (args, arg_names) = crate::syntax::normalize_invocation_args(
                        &split_invocation_args(&text[t + 1..t + close]),
                    );
                    out.push((
                        start + i,
                        TemplateRef {
                            ref_name: Some(ref_name),
                            template_name: text[tn_start..t].to_string(),
                            args,
                            is_collection: true,
                            arg_names,
                            span: SourceSpan::new(start + i, start + t + close + 1),
                        },
                    ));
                    i = t + close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Split a raw invocation arg list (the text between `(` and `)`) into the
/// comma-separated raw argument expressions, trimmed — the same shape reify's
/// `parse_template_call` produced (`&row(1, "x", $d)` → `["1", "\"x\"", "$d"]`).
fn split_invocation_args(arg_str: &str) -> Vec<String> {
    if arg_str.trim().is_empty() {
        Vec::new()
    } else {
        arg_str.split(',').map(|a| a.trim().to_string()).collect()
    }
}
fn collect_scope_states_nested(nested: &[NestedScope], out: &mut Vec<(usize, ComponentStateDecl)>) {
    for ns in nested {
        collect_scope_states_from_matches(&ns.matches, out);
        collect_scope_states_nested(&ns.nested_scopes, out);
    }
}

fn collect_scope_states_from_matches(
    matches: &[FormMatch],
    out: &mut Vec<(usize, ComponentStateDecl)>,
) {
    for m in matches {
        if m.macro_name != "local-state" {
            continue;
        }
        let Some(var_name) = cap_str(m, "name").map(|s| s.trim_start_matches('$').to_string())
        else {
            continue;
        };
        let Some(type_name) = cap_str(m, "type") else {
            continue;
        };
        let value = cap_str(m, "value");
        let initial = match value.as_deref() {
            Some(v) => match type_name.as_str() {
                "bool" => CapturedValue::Bool(v == "true"),
                "number" => v
                    .parse::<f64>()
                    .map(CapturedValue::Number)
                    .unwrap_or(CapturedValue::Number(0.0)),
                _ => {
                    let unquoted = v
                        .trim_start_matches('"')
                        .trim_end_matches('"')
                        .trim_start_matches('\'')
                        .trim_end_matches('\'');
                    CapturedValue::String(unquoted.to_string())
                }
            },
            None => CapturedValue::Expr("null".to_string()),
        };
        out.push((
            m.span.start,
            ComponentStateDecl {
                var_name,
                type_name,
                initial,
            },
        ));
    }
}

/// Read a `local-state` capture as a plain string, unwrapping the value-bearing
/// `CapturedValue` variants (Ident/TypeRef/Expr/String/Binding/Selector).

/// True when `macro_name` DECLARES forms: its `%registers` clause names
/// category `form` — the stdlib `@form <kind>` macros (form.st, via
/// STDLIB_REGISTRY) or a same-file user `%macro` carrying its own
/// `%registers form(...)`. The set of form-declaring macros is registry
/// DATA, not a Rust list — the one predicate shared by validation
/// ([`validate_form_refs`]), expansion ([`expand_style_form_splices`]), and
/// the easing-registration injection (BUG-297, compiler.rs).
pub(crate) fn is_form_declaring_macro(ast: &StFile, macro_name: &str) -> bool {
    declares_entity(ast, macro_name, "form")
}

/// Does `macro_name` declare an instance of `entity`?
///
/// `%registers <entity>(…)` is the generic rail — the registry already carries
/// 7 entities (driver, binding, form, host, template, type, import). Asking it
/// a form-shaped question is what made `@template` invisible to `@data forms`
/// (BUG-343): templates are not excluded by design, they register a DIFFERENT
/// entity. Parameterising the question is what lets one catalog surface serve
/// every entity instead of growing a sibling per kind.
pub(crate) fn declares_entity(ast: &StFile, macro_name: &str, entity: &str) -> bool {
    use crate::parser::meta_ast::MetaDef;
    if STDLIB_REGISTRY
        .get_by_macro_name(macro_name)
        .and_then(|f| f.macro_def.registers.as_ref())
        .is_some_and(|r| r.name == entity)
    {
        return true;
    }
    ast.meta_defs.iter().any(|def| match def {
        MetaDef::Macro(m) => {
            m.name == macro_name && m.registers.as_ref().is_some_and(|r| r.name == entity)
        }
        _ => false,
    })
}

/// Every entity name the registry knows — the valid right-hand sides of
/// `@data declarations … from <entity>`. Derived from the registry, never a
/// hardcoded list, so a new `%registers` entity is catalogable the day it
/// lands. Used to REFUSE an unknown entity rather than return an empty list
/// (a silent empty reads exactly like "you have none").
pub(crate) fn known_entities(ast: &StFile) -> std::collections::BTreeSet<String> {
    use crate::parser::meta_ast::MetaDef;
    let mut out = std::collections::BTreeSet::new();
    for f in STDLIB_REGISTRY.all_forms() {
        if let Some(r) = f.macro_def.registers.as_ref() {
            out.insert(r.name.clone());
        }
    }
    for def in &ast.meta_defs {
        if let MetaDef::Macro(m) = def
            && let Some(r) = m.registers.as_ref()
        {
            out.insert(r.name.clone());
        }
    }
    out
}

/// Every `@form` declaration in `ast.matches` (any kind), as the matches
/// themselves — captures carry the name (WITH `--` sigil), kind, params,
/// and body.
pub(crate) fn form_declaration_matches(ast: &StFile) -> Vec<&FormMatch> {
    ast.matches
        .iter()
        .filter(|m| {
            is_form_declaring_macro(ast, m.matched_macro.as_deref().unwrap_or(&m.macro_name))
        })
        .collect()
}

fn cap_str(m: &FormMatch, key: &str) -> Option<String> {
    match m.captures.get(key)? {
        CapturedValue::Ident(s)
        | CapturedValue::TypeRef(s)
        | CapturedValue::Expr(s)
        | CapturedValue::String(s)
        | CapturedValue::Binding(s)
        | CapturedValue::Selector(s) => Some(s.trim().to_string()),
        _ => None,
    }
}
/// Re-extract FormMatches using an augmented registry that includes user-defined macros.
///
/// When a site imports files containing `%macro` definitions, those macros need to be
/// recognized during FormMatch extraction. The initial parse only uses STDLIB_REGISTRY,
/// so user macros produce zero matches. This function clones the stdlib registry,
/// registers user macro forms, re-runs the event-based matcher, and reassigns matches
/// to the existing scope structure.

/// Convert retained match diagnostics into author-facing errors (BUG-229/BUG-234).
///
/// `match_sink` has already decided WHICH match failures are real: a directive
/// that committed to a grammar and then failed inside it, versus the ordinary
/// negative result of trying every form against every directive. Everything that
/// reaches here is a genuine author error, so this only chooses the code and the
/// wording.
///
/// Shared by both the initial parse and the user-macro rematch, because a user
/// grammar can only be judged by the rematch (the stdlib-only parse has never
/// heard of it) while a stdlib grammar is judged by the first parse.
fn match_diagnostics_to_errors(
    source: &str,
    diagnostics: &[crate::syntax::events::MatchDiagnostic],
) -> Vec<crate::diagnostics::Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            // A CSS at-rule that Spacetime passes through VERBATIM as raw CSS
            // (`media` | `supports` | `keyframes`, see the `raw_css_blocks` arm in
            // `cst_to_stfile`) is not required to match any `%form`. Valid CSS like
            // `@media (prefers-reduced-motion: reduce)` fails `@media($query:string)`
            // on its query — correctly, since the form wants a QUOTED string — and is
            // then handled by the CSS layer. Reporting that failure turns plain CSS
            // into a compile error, so these names are excluded at the reporting
            // boundary rather than by weakening the matcher.
            // `HOST_CSS_PASSTHROUGH_AT_RULES` (module const) is the ONE registry for the
            // host-at-rule decision — `@media` | `@supports` | `@keyframes` | `@font-face`.
            let clause = source.get(diagnostic.offset..).unwrap_or("");
            let at_name: String = clause
                .trim_start()
                .strip_prefix('@')
                .map(|rest| {
                    rest.chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '-')
                        .collect()
                })
                .unwrap_or_default();
            !HOST_CSS_PASSTHROUGH_AT_RULES.contains(&at_name.as_str())
        })
        .map(|diagnostic| {
            let clause = source.get(diagnostic.offset..).unwrap_or("");
            let span = crate::diagnostics::SourceSpan::new(
                diagnostic.offset,
                diagnostic.offset + clause.find(';').unwrap_or(clause.len()),
            );
            // `@data subscribe` keeps its OWN code and wording, because `subscribe`
            // names a specific contract an author can be told exactly how to fix.
            // This is a better MESSAGE for a case the general rule already catches,
            // not a second mechanism deciding WHETHER to report (BUG-234 → BUG-229).
            if clause.split_whitespace().take(2).eq(["@data", "subscribe"]) {
                crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0945,
                    "malformed @data subscribe: expected `$name <type>? from $host : <assign>`",
                )
                .with_span(span)
                .with_hint("add the required `from $host` clause before `: <assign>`")
            } else {
                crate::diagnostics::Diagnostic::error(
                    crate::diagnostics::DiagnosticCode::E0946,
                    format!(
                        "directive does not match its declared grammar: {}",
                        diagnostic.message
                    ),
                )
                .with_span(span)
                .with_hint(
                    "check the `%form` this directive declares (and any `%capture_type` \
                     it names) — the directive must match that grammar exactly",
                )
            }
        })
        .collect()
}

pub fn rematch_with_user_macros(ast: &mut StFile, source: &str) {
    rematch_with_user_macros_in(ast, source, None);
}

/// Validate statement-position form splices against the declared forms (BUG-241).
///
/// The `parse_body_item` dispatch arm RECOGNIZES `--name;` as a FORM_REF (no
/// longer silently dropped); this is the second half: a name no `@form`
/// declaration registered is a HARD error (E0947), never another silent drop.
/// "Declared" means a FormMatch of a macro that registers into category
/// `form` — the stdlib `@form <kind>` macros (form.st, via STDLIB_REGISTRY) or
/// a same-file user `%macro` carrying its own `%registers form(...)` — so the
/// set of form-declaring macros is registry DATA, not a Rust list.
///
/// Idempotent: it runs at the end of the initial parse AND at the end of the
/// user-macro rematch (a form declared by a same-file `%macro` is only visible
/// to the second pass), so prior E0947s are cleared before re-validating.
/// PLAN-135 W4: fill every match's `doc` from the contiguous `///` block
/// above it (see `doc_comment_before`). File-scope and selector-scope
/// matches alike; idempotent (a match that already carries a doc — e.g. an
/// imported AST merged after its own parse — is left alone).
pub fn fill_match_docs(ast: &mut StFile, source: &str) {
    // A single-declaration module documents ITSELF at the top: every
    // `stdlib/showcases/*` pattern opens with `/// scenario:` / `/// school:`
    // at line 1 and declares its `@template` at line 15, with an `@import` and
    // a block comment in between. There is no CONTIGUOUS doc block above the
    // declaration, so without this fallback the metadata describing the
    // pattern is invisible to the catalog (PLAN-144 W2 / BUG-343).
    //
    // The declaration's OWN block always wins; the header is consulted only
    // when it has none, so a multi-declaration file is never mislabelled.
    let header = crate::syntax::file_header_doc(source);
    let fill = |m: &mut crate::syntax::FormMatch| {
        if m.doc.is_none() {
            m.doc = crate::syntax::doc_comment_before(source, m.span.start as usize)
                .or_else(|| header.clone());
        }
    };
    for m in ast.matches.iter_mut() {
        fill(m);
    }
    for scope in ast.scopes.iter_mut() {
        for m in scope.matches.iter_mut() {
            fill(m);
        }
    }
}

/// Refuse `@data declarations … from <entity>` when `<entity>` is not a
/// registered entity (PLAN-144 W2).
///
/// The alternative — returning an empty row set — is the banned failure class:
/// "you have no templates" and "you misspelled `template`" would render
/// identically, as an empty page that built clean. The valid names come from
/// the registry itself, so a new `%registers` entity is catalogable the day it
/// lands and this list can never drift.
pub fn validate_declaration_entities(ast: &mut StFile) {
    let known = known_entities(ast);
    let mut diags = Vec::new();
    let mut check = |m: &FormMatch| {
        if m.matched_macro.as_deref() != Some("data-declarations") {
            return;
        }
        let Some(cap) = m.captures.get("entity") else {
            return;
        };
        let Some(entity) = cap.as_type_name().or_else(|| cap.as_string_literal()) else {
            return;
        };
        if known.contains(entity) {
            return;
        }
        let mut names: Vec<&str> = known.iter().map(|s| s.as_str()).collect();
        names.sort_unstable();
        diags.push(
            crate::diagnostics::Diagnostic::error(
                crate::diagnostics::DiagnosticCode::E0959,
                format!("unknown declaration entity `{entity}`"),
            )
            .with_span(m.span.into())
            .with_hint(format!(
                "`%registers` knows: {}. An unknown entity is refused rather \
                 than returning an empty list, which would read exactly like \
                 having none.",
                names.join(", ")
            )),
        );
    };
    for m in &ast.matches {
        check(m);
    }
    for scope in &ast.scopes {
        for m in &scope.matches {
            check(m);
        }
    }
    ast.diagnostics.extend(diags);
}

pub fn validate_form_refs(ast: &mut StFile) {
    ast.diagnostics
        .retain(|d| d.code != crate::diagnostics::DiagnosticCode::E0947);

    // Macros that DECLARE forms: their %registers clause names category "form".
    let is_form_macro = |macro_name: &str| is_form_declaring_macro(ast, macro_name);

    // Every match of a form-declaring macro contributes its captured `name`
    // (the dashed ident, sigil included). Identity comes from `matched_macro`
    // (the %macro actually selected — "form-style") — NOT `macro_name`, which
    // is the directive's public name ("form" for all six kind macros).
    let mut declared: Vec<String> = ast
        .matches
        .iter()
        .filter(|m| is_form_macro(m.matched_macro.as_deref().unwrap_or(&m.macro_name)))
        .filter_map(|m| cap_str(m, "name"))
        .collect();
    declared.sort();
    declared.dedup();

    // Collect (name, span) of every splice in every scope, recursively.
    fn collect<'a>(
        scopes: &'a [crate::parser::ast::NestedScope],
        out: &mut Vec<&'a crate::parser::ast::FormRefUse>,
    ) {
        for scope in scopes {
            out.extend(scope.form_refs.iter());
            collect(&scope.nested_scopes, out);
        }
    }
    let mut refs: Vec<&crate::parser::ast::FormRefUse> = Vec::new();
    // File-level refs: root splices and directive-body splices (the sweep).
    refs.extend(ast.form_refs.iter());
    for scope in &ast.scopes {
        refs.extend(scope.form_refs.iter());
        collect(&scope.nested_scopes, &mut refs);
    }

    for form_ref in refs {
        if declared.contains(&form_ref.name) {
            continue;
        }
        let hint = if declared.is_empty() {
            format!(
                "declare it first: `@form style {} {{ … }}` (SIP-001c) — or check the spelling",
                form_ref.name
            )
        } else {
            format!(
                "declared forms: {} · or declare it: `@form style {} {{ … }}`",
                declared.join(", "),
                form_ref.name
            )
        };
        ast.diagnostics.push(
            crate::diagnostics::Diagnostic::error(
                crate::diagnostics::DiagnosticCode::E0947,
                format!(
                    "unknown form `{}` in statement position — no `@form` declaration registers it",
                    form_ref.name
                ),
            )
            .with_span(crate::diagnostics::SourceSpan::new(
                form_ref.span.start,
                form_ref.span.end,
            ))
            .with_hint(hint),
        );
    }
}

/// BUG-298 — the EXPANSION half of statement-position form splices. Where
/// [`validate_form_refs`] proves every `--name;` names a declared form, this
/// pass splices a STYLE form's declarations into the scope's CSS at the
/// splice's source position (CSS order is semantic), substituting declared
/// params with call-site args (named or positional) falling back to
/// declaration defaults.
///
/// Runs POST-import-merge (the compile pipeline), where the authoritative
/// declaration set is the merged AST's matches — a form declared in an
/// `@import`ed file or the project's `_prelude.st` resolves here. The
/// parse-time validation stays as the file-local fast path; the pipeline
/// re-runs `validate_form_refs` first (it is idempotent), so a cross-file
/// form's stale E0947 clears and unknown names still error exactly once.
///
/// Kind discipline: only STYLE forms expand. A non-style form in bare-scope
/// statement position is E0959 (an easing/motion/score/value/markup form has
/// no declarations to contribute — motion belongs to `@on`'s form slot, score
/// to `@score` bodies). A call-site argument mismatch is E0960: a NAMED arg
/// the declaration does not have (silently ignoring it would drop author
/// intent), or a required param left unfilled (an unsubstituted `$hole` in
/// emitted CSS is the same silent-drop class).
/// True when the page carries any statement-position form splice
/// (`--name;`, BUG-298): file-root, top-level scope, or nested. Gates the
/// form-expansion clone at the pipeline seam (the common no-splice case is
/// byte-identical to before).
pub fn ast_has_form_refs(ast: &StFile) -> bool {
    fn nested(scopes: &[crate::parser::ast::NestedScope]) -> bool {
        scopes
            .iter()
            .any(|s| !s.form_refs.is_empty() || nested(&s.nested_scopes))
    }
    if !ast.form_refs.is_empty() {
        return true;
    }
    ast.scopes
        .iter()
        .any(|s| !s.form_refs.is_empty() || nested(&s.nested_scopes))
}

pub fn expand_style_form_splices(ast: &mut StFile) {
    use crate::syntax::CapturedValue;

    // Same discovery as validate_form_refs: macros that DECLARE forms are
    // stdlib's six `@form <kind>` macros or a same-file user `%macro` whose
    // %registers names category `form` — registry DATA, not a Rust list.
    let is_form_macro = |macro_name: &str| is_form_declaring_macro(ast, macro_name);

    struct StyleForm {
        body: Vec<crate::syntax::PropertyDef>,
        params: Vec<crate::syntax::TemplateParamDef>,
    }

    // Declared STYLE forms, keyed by name WITH the `--` sigil (both the
    // declaration capture and the splice carry it — one normalization, no
    // guessing about spelling, matching score_fragment_body's rule).
    let mut declared: std::collections::HashMap<String, StyleForm> =
        std::collections::HashMap::new();
    for m in &ast.matches {
        if !is_form_macro(m.matched_macro.as_deref().unwrap_or(&m.macro_name)) {
            continue;
        }
        let Some(name) = cap_str(m, "name") else {
            continue;
        };
        let kind = cap_str(m, "kind").unwrap_or_default();
        if kind != "style" {
            continue;
        }
        let body = match m.captures.get("body") {
            Some(CapturedValue::Properties(p)) => p.clone(),
            _ => Vec::new(),
        };
        let params = match m.captures.get("params") {
            Some(CapturedValue::ParamList(p)) => p.clone(),
            _ => Vec::new(),
        };
        declared.insert(name, StyleForm { body, params });
    }

    // Resolve one scope's refs; returns the spliced declarations in splice
    // order. Diagnostics for kind/arg errors accumulate into `diags`.
    fn expand_scope(
        form_refs: &[crate::parser::ast::FormRefUse],
        declared: &std::collections::HashMap<String, StyleForm>,
        diags: &mut Vec<crate::diagnostics::Diagnostic>,
    ) -> Vec<crate::parser::ast::CssDeclaration> {
        expand_refs(form_refs, declared, diags, &[])
    }

    /// Resolve a list of splices, carrying the chain of forms currently being
    /// expanded so a cycle can be named rather than looped.
    fn expand_refs(
        form_refs: &[crate::parser::ast::FormRefUse],
        declared: &std::collections::HashMap<String, StyleForm>,
        diags: &mut Vec<crate::diagnostics::Diagnostic>,
        seen: &[String],
    ) -> Vec<crate::parser::ast::CssDeclaration> {
        let mut out = Vec::new();
        for form_ref in form_refs {
            let Some(form) = declared.get(&form_ref.name) else {
                // Unknown names are validate_form_refs' E0947 — reported
                // there exactly once, skipped here.
                continue;
            };
            let args = parse_form_call_args(form_ref.args.as_deref());
            // Named args the declaration does not have → E0960.
            let mut named: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            let mut positional: Vec<String> = Vec::new();
            for (arg_name, value) in args {
                match arg_name {
                    Some(n) => {
                        if !form.params.iter().any(|p| p.name == n) {
                            diags.push(
                                crate::diagnostics::Diagnostic::error(
                                    crate::diagnostics::DiagnosticCode::E0960,
                                    format!(
                                        "unknown argument `{n}` for form `{}` — declared parameters: {}",
                                        form_ref.name,
                                        form.params
                                            .iter()
                                            .map(|p| format!("${}", p.name))
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    ),
                                )
                                .with_span(crate::diagnostics::SourceSpan::new(
                                    form_ref.span.start,
                                    form_ref.span.end,
                                ))
                                .with_hint(
                                    "declare the parameter in the `@form`, or check the spelling",
                                ),
                            );
                            continue;
                        }
                        named.insert(n, value);
                    }
                    None => positional.push(value),
                }
            }
            // Per-param resolution: named override → positional → default.
            let mut positionals = positional.iter();
            let mut missing: Option<String> = None;
            let values: Vec<(String, String)> = form
                .params
                .iter()
                .filter_map(|p| {
                    let value = named
                        .get(&p.name)
                        .cloned()
                        .or_else(|| positionals.next().cloned())
                        .or_else(|| p.default.clone());
                    match value {
                        Some(v) => Some((p.name.clone(), v)),
                        None => {
                            missing.get_or_insert_with(|| p.name.clone());
                            None
                        }
                    }
                })
                .collect();
            if let Some(param) = missing {
                diags.push(
                    crate::diagnostics::Diagnostic::error(
                        crate::diagnostics::DiagnosticCode::E0960,
                        format!(
                            "form `{}` requires `${param}` — no default and no call-site argument",
                            form_ref.name
                        ),
                    )
                    .with_span(crate::diagnostics::SourceSpan::new(
                        form_ref.span.start,
                        form_ref.span.end,
                    ))
                    .with_hint(format!(
                        "pass it (`{}({param}: …)`), or give the parameter a default",
                        form_ref.name
                    )),
                );
                continue;
            }
            for prop in &form.body {
                let mut value = prop.type_ref.clone();
                for (name, replacement) in &values {
                    value = crate::syntax::replace_param_token(
                        &value,
                        &format!("${name}"),
                        replacement,
                    );
                }
                // A form body may SPLICE another form (BUG-327): the nested
                // reference rides in the body as a PropertyDef whose name is
                // the form reference. Resolve it here, so composition works to
                // any depth and a school can bind a role
                // (`@form style --role-title { --sc-min-title; }`) instead of
                // every pattern naming the school directly.
                if prop.name.starts_with("--") && declared.contains_key(&prop.name) {
                    let nested = crate::parser::ast::FormRefUse {
                        name: prop.name.clone(),
                        args: if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.clone())
                        },
                        span: form_ref.span,
                    };
                    // Depth-guard: a cycle (`--a { --b; }` / `--b { --a; }`)
                    // must be a loud diagnostic, never a hang or a silently
                    // empty rule. `seen` carries the chain being expanded.
                    if seen.iter().any(|n| n == &prop.name) {
                        let mut chain = seen.join(" -> ");
                        chain.push_str(" -> ");
                        chain.push_str(&prop.name);
                        diags.push(
                            crate::diagnostics::Diagnostic::error(
                                crate::diagnostics::DiagnosticCode::E0966,
                                format!("form splice cycle: {chain}"),
                            )
                            .with_span(crate::diagnostics::SourceSpan::new(
                                form_ref.span.start,
                                form_ref.span.end,
                            ))
                            .with_hint(
                                "a form cannot splice itself, directly or through another form — break the loop by inlining one of the declarations",
                            ),
                        );
                        continue;
                    }
                    let mut deeper = seen.to_vec();
                    deeper.push(prop.name.clone());
                    out.extend(expand_refs(
                        std::slice::from_ref(&nested),
                        declared,
                        diags,
                        &deeper,
                    ));
                    continue;
                }
                out.push(crate::parser::ast::CssDeclaration {
                    property: prop.name.clone(),
                    value,
                    is_injection: false,
                    span: form_ref.span,
                });
            }
        }
        out
    }

    // Kind errors for refs naming a NON-style form (E0959), then expansion.
    // The declared set above holds only style forms, so kind discovery is a
    // separate, cheaper pass over the same matches.
    let mut non_style: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for m in &ast.matches {
        if !is_form_macro(m.matched_macro.as_deref().unwrap_or(&m.macro_name)) {
            continue;
        }
        let (Some(name), Some(kind)) = (cap_str(m, "name"), cap_str(m, "kind")) else {
            continue;
        };
        if kind != "style" {
            non_style.insert(name, kind);
        }
    }

    let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();

    fn kind_error(
        form_refs: &[crate::parser::ast::FormRefUse],
        non_style: &std::collections::HashMap<String, String>,
        diags: &mut Vec<crate::diagnostics::Diagnostic>,
    ) {
        for form_ref in form_refs {
            if let Some(kind) = non_style.get(&form_ref.name) {
                diags.push(
                    crate::diagnostics::Diagnostic::error(
                        crate::diagnostics::DiagnosticCode::E0959,
                        format!(
                            "`{}` is a `{kind}` form — a statement-position splice takes a STYLE form",
                            form_ref.name
                        ),
                    )
                    .with_span(crate::diagnostics::SourceSpan::new(
                        form_ref.span.start,
                        form_ref.span.end,
                    ))
                    .with_hint(
                        "motion forms drive (`@on &.driver: --form;`), score forms splice into `@score` bodies; style forms splice into scopes",
                    ),
                );
            }
        }
    }

    fn walk_nested(
        scopes: &mut [crate::parser::ast::NestedScope],
        declared: &std::collections::HashMap<String, StyleForm>,
        non_style: &std::collections::HashMap<String, String>,
        diags: &mut Vec<crate::diagnostics::Diagnostic>,
    ) {
        for scope in scopes.iter_mut() {
            kind_error(&scope.form_refs, non_style, diags);
            if !scope.form_refs.is_empty() {
                let spliced = expand_scope(&scope.form_refs, declared, diags);
                scope.css_declarations.extend(spliced);
                scope.css_declarations.sort_by_key(|d| d.span.start);
                scope.form_refs.clear();
            }
            walk_nested(&mut scope.nested_scopes, declared, non_style, diags);
        }
    }

    for scope in &mut ast.scopes {
        kind_error(&scope.form_refs, &non_style, &mut diags);
        if !scope.form_refs.is_empty() {
            let spliced = expand_scope(&scope.form_refs, &declared, &mut diags);
            scope.css_declarations.extend(spliced);
            scope.css_declarations.sort_by_key(|d| d.span.start);
            scope.form_refs.clear();
        }
        walk_nested(&mut scope.nested_scopes, &declared, &non_style, &mut diags);
    }

    ast.diagnostics.extend(diags);
}

/// Parse a form application's raw argument text (`"12px, pad: 3rem"`) into
/// `(name, value)` pairs: `None` = positional, `Some(n)` = named. Splits
/// commas and reads the first `:` only at paren depth 0, so CSS values with
/// parens (`cubic-bezier(0.4, 0, 0.2, 1)`) survive intact.
fn parse_form_call_args(raw: Option<&str>) -> Vec<(Option<String>, String)> {
    let Some(raw) = raw else { return Vec::new() };
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for ch in raw.chars() {
        match ch {
            '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .map(|part| {
            let mut depth = 0i32;
            for (i, ch) in part.char_indices() {
                match ch {
                    '(' | '[' => depth += 1,
                    ')' | ']' => depth -= 1,
                    ':' if depth == 0 => {
                        let name = part[..i].trim().trim_start_matches('$').to_string();
                        let value = part[i + 1..].trim().to_string();
                        return (Some(name), value);
                    }
                    _ => {}
                }
            }
            (None, part)
        })
        .collect()
}

/// Like [`rematch_with_user_macros`], but scoped to a specific main file.
///
/// After `resolve_imports`, the main file's scopes are tagged with its path
/// (not `None`). `source` is the main file content, so re-parsed matches must be
/// assigned to scopes belonging to the main file: those with `source_file == None`
/// (inline / no-import path) OR equal to `main_source_file` (post-import path).
pub fn rematch_with_user_macros_in(ast: &mut StFile, source: &str, main_source_file: Option<&str>) {
    use crate::parser::meta_ast::MetaDef;

    // Collect user-defined macros from meta_defs
    let user_macros: Vec<_> = ast
        .meta_defs
        .iter()
        .filter_map(|def| {
            if let MetaDef::Macro(m) = def {
                Some(m)
            } else {
                None
            }
        })
        .collect();

    // BUG-229 (A): a same-file `%capture_type` was NEVER registered here, only
    // `%macro` was. A macro whose `%form` captures a body with its own custom
    // capture type therefore matched against a registry that did not contain the
    // grammar — the extractor was always absent, and the body fell through to the
    // lenient raw-text fallback in `form_compiler`. Register both, so a user
    // grammar is available to the parse that must enforce it.
    let user_capture_types: Vec<_> = ast
        .meta_defs
        .iter()
        .filter_map(|def| {
            if let MetaDef::CaptureType(ct) = def {
                Some(ct)
            } else {
                None
            }
        })
        .collect();

    if user_macros.is_empty() && user_capture_types.is_empty() {
        return;
    }

    // Clone stdlib registry and register user macro forms + capture types.
    let mut augmented = (*STDLIB_REGISTRY).clone();
    for macro_def in &user_macros {
        augmented.register(macro_def);
    }
    for ct in &user_capture_types {
        augmented.register_capture_type((*ct).clone());
    }

    // Re-extract FormMatches from the main file with the augmented registry.
    // This picks up directives that the stdlib-only parse missed.
    //
    // BUG-229: these diagnostics are the ONLY ones that can judge a user grammar.
    // The stdlib-only parse below cannot — it has never heard of the user's
    // `%capture_type`, so its failure for `@spike` is a meaningless
    // `DirectivePrefixMismatch`. Discarding the augmented diagnostics (`_diags`)
    // meant a body that failed the grammar its own file declared produced NOTHING:
    // the directive silently vanished and `check` reported success.
    let (augmented_matches, augmented_diags) =
        crate::syntax::events::parse_matches(source, &augmented);
    ast.diagnostics
        .extend(match_diagnostics_to_errors(source, &augmented_diags));

    // Also extract with stdlib-only to find what's genuinely new.
    let (stdlib_matches, _) = crate::syntax::events::parse_matches(source, &STDLIB_REGISTRY);

    // Index stdlib matches by span: value = how many nested matches the stdlib parse
    // captured for that span. A span is "already matched" by stdlib; comparing the
    // nested-match COUNT detects a parent (e.g. `@stage`) whose children block was
    // ENRICHED by user macros (a nested project-local `@layer` the stdlib parse, not
    // knowing that form, silently dropped from the block).
    let stdlib_by_span: std::collections::HashMap<(usize, usize), usize> = stdlib_matches
        .iter()
        .map(|m| ((m.span.start, m.span.end), count_nested_matches(m)))
        .collect();

    // Partition augmented matches:
    //   - genuinely NEW (span the stdlib parse never produced) -> assign to scopes below
    //   - ENRICHED (same span as a stdlib match, but with MORE nested matches) -> replace
    //     the stale match in-place so its now-captured project-local children survive.
    //     Without this, a project macro nested inside a stdlib macro body is dropped:
    //     the enriched parent shares the stdlib parent's span and would be filtered out,
    //     discarding the children block the augmented parse built (BUG: nested user macros).
    let mut new_matches: Vec<FormMatch> = Vec::new();
    for m in augmented_matches {
        let key = (m.span.start, m.span.end);
        match stdlib_by_span.get(&key) {
            None => new_matches.push(m),
            Some(&stdlib_count) => {
                if count_nested_matches(&m) > stdlib_count {
                    // Parent match gained children from a user macro -> swap it in place.
                    replace_match_by_span(m.span, &m, ast);
                }
                // else: identical stdlib match, nothing new to add.
            }
        }
    }

    // Assign new matches to scopes from the main file only (imported scopes
    // have spans from their own file content, not the main file's source).
    for mut fm in new_matches {
        let mut assigned = false;
        for scope_ast in ast
            .scopes
            .iter_mut()
            .filter(|s| s.source_file.is_none() || s.source_file.as_deref() == main_source_file)
        {
            if scope_ast.span.start <= fm.span.start && fm.span.end <= scope_ast.span.end {
                fm.selector = Some(
                    nearest_nested_composed_selector(&fm, &scope_ast.nested_scopes)
                        .unwrap_or_else(|| scope_ast.selector.clone()),
                );
                ast.matches.push(fm.clone());
                if !assign_match_to_nested_scopes(&fm, &mut scope_ast.nested_scopes) {
                    scope_ast.matches.push(fm.clone());
                }
                assigned = true;
                break;
            }
        }
        if !assigned {
            ast.matches.push(fm);
        }
    }

    // BUG-241: re-validate form splices — a form declared by a same-file
    // `%macro` is only visible to THIS pass, so a splice that errored as
    // unknown at the initial parse may now resolve (and a still-unknown one
    // re-errors). Idempotent: clears its own E0947s first.
    validate_form_refs(ast);
}

/// Re-extract FormMatches using ALL macros from a MetaRegistry (PLAN-026).
///
/// Unlike [`rematch_with_user_macros`], which sources macros from the AST's own
/// `meta_defs`, this registers every macro in `registry` into the augmented
/// syntax registry. Used when compiling a `$body:block` capture's inner source
/// in isolation: the body uses macros (`@assert`, `@then`, `@eval`, …) that live
/// in the registry, not in the tiny body fragment, so the stdlib-only parse
/// produced zero matches. Assigns new matches to the fragment's own scopes.
pub fn rematch_with_registry(
    ast: &mut StFile,
    source: &str,
    registry: &crate::metasystem::MetaRegistry,
) {
    let mut augmented = (*STDLIB_REGISTRY).clone();
    for (_, macro_def) in registry.iter_macros() {
        augmented.register(macro_def);
    }

    let (augmented_matches, _diags) = crate::syntax::events::parse_matches(source, &augmented);
    let (stdlib_matches, _) = crate::syntax::events::parse_matches(source, &STDLIB_REGISTRY);
    // Index by span -> nested-match count, so an ENRICHED parent (a stdlib macro body that
    // gained project-local children in the augmented parse) is replaced rather than
    // discarded by the span diff. See `rematch_with_user_macros_in` for the rationale.
    let stdlib_by_span: std::collections::HashMap<(usize, usize), usize> = stdlib_matches
        .iter()
        .map(|m| ((m.span.start, m.span.end), count_nested_matches(m)))
        .collect();

    let mut new_matches: Vec<FormMatch> = Vec::new();
    for m in augmented_matches {
        let key = (m.span.start, m.span.end);
        match stdlib_by_span.get(&key) {
            None => new_matches.push(m),
            Some(&stdlib_count) => {
                if count_nested_matches(&m) > stdlib_count {
                    replace_match_by_span(m.span, &m, ast);
                }
            }
        }
    }

    for mut fm in new_matches {
        let mut assigned = false;
        for scope_ast in ast.scopes.iter_mut() {
            if scope_ast.span.start <= fm.span.start && fm.span.end <= scope_ast.span.end {
                fm.selector = Some(
                    nearest_nested_composed_selector(&fm, &scope_ast.nested_scopes)
                        .unwrap_or_else(|| scope_ast.selector.clone()),
                );
                ast.matches.push(fm.clone());
                if !assign_match_to_nested_scopes(&fm, &mut scope_ast.nested_scopes) {
                    scope_ast.matches.push(fm.clone());
                }
                assigned = true;
                break;
            }
        }
        if !assigned {
            ast.matches.push(fm);
        }
    }
}

/// Convert a CST directive to a pattern definition
fn convert_cst_directive_to_pattern(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<PatternDef> {
    use crate::syntax::cst::{Arg, AstNode, NamedArg, SyntaxKind};

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Extract pattern name from inline args
    let name = directive
        .inline_args()
        .next()
        .and_then(|a| a.value_text())?;

    // Extract parameters from arg list
    let mut params = Vec::new();
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::NAMED_ARG {
                // Named argument: $name: default
                if let Some(named_arg) = NamedArg::cast(arg_node) {
                    let param_name = named_arg.name_text().unwrap_or_default();
                    let default = named_arg
                        .value_text()
                        .map(|v| parse_pattern_value(v.trim()));
                    params.push(PatternParam {
                        name: param_name,
                        default,
                    });
                }
            } else if arg_node.kind() == SyntaxKind::ARG {
                // Positional argument: $name or $name?
                if let Some(arg) = Arg::cast(arg_node) {
                    let arg_text = arg.value_text().unwrap_or_default();
                    let arg_text = arg_text.trim();

                    // Parse parameter: $name, $name?, $name*
                    if arg_text.starts_with('$') {
                        let param_name = arg_text
                            .trim_start_matches('$')
                            .trim_end_matches('?')
                            .trim_end_matches('*')
                            .to_string();

                        params.push(PatternParam {
                            name: param_name,
                            default: None,
                        });
                    }
                }
            }
        }
    }

    // Convert pattern body
    let body = convert_cst_body_to_pattern_items(directive.body(), source);

    Some(PatternDef {
        name,
        params,
        body,
        span,
    })
}

/// Convert a CST body to pattern body items
fn convert_cst_body_to_pattern_items(
    body: Option<crate::syntax::cst::Body>,
    source: &str,
) -> Vec<PatternBodyItem> {
    use crate::syntax::cst::AstNode;

    let Some(body) = body else {
        return Vec::new();
    };

    let mut items = Vec::new();

    for directive in body.directives() {
        let text_range = directive.syntax().text_range();
        let _span = SourceSpan::new(text_range.start().into(), text_range.end().into());

        let name = directive.name_text().unwrap_or_default();

        match name.as_str() {
            // Pattern primitive directives
            "state_machine" => {
                if let Some(sm) = convert_cst_directive_to_state_machine(&directive, source) {
                    items.push(PatternBodyItem::StateMachine(sm));
                }
            }
            "state" => {
                if let Some(state) = convert_cst_directive_to_state(&directive, source) {
                    items.push(PatternBodyItem::State(state));
                }
            }
            "transition" => {
                if let Some(trans) = convert_cst_directive_to_transition(&directive, source) {
                    items.push(PatternBodyItem::Transition(trans));
                }
            }
            "async_transition" => {
                if let Some(async_trans) =
                    convert_cst_directive_to_async_transition(&directive, source)
                {
                    items.push(PatternBodyItem::AsyncTransition(async_trans));
                }
            }
            "mutate" => {
                if let Some(mutate) = convert_cst_directive_to_mutate(&directive, source) {
                    items.push(PatternBodyItem::Mutate(mutate));
                }
            }
            // Control flow directives
            "for" => {
                if let Some(for_item) = convert_cst_directive_to_pattern_for(&directive, source) {
                    items.push(PatternBodyItem::For(for_item));
                }
            }
            "if" => {
                if let Some(if_item) = convert_cst_directive_to_pattern_if(&directive, source) {
                    items.push(PatternBodyItem::If(if_item));
                }
            }
            "match" => {
                if let Some(match_item) = convert_cst_directive_to_pattern_match(&directive, source)
                {
                    items.push(PatternBodyItem::Match(match_item));
                }
            }
            "include" => {
                if let Some(include_item) =
                    convert_cst_directive_to_pattern_include(&directive, source)
                {
                    items.push(PatternBodyItem::Include(include_item));
                }
            }
            _ => {
                // Generic pattern call (nested pattern invocation)
                if let Some(call_item) = convert_cst_directive_to_pattern_call(&directive, source) {
                    items.push(PatternBodyItem::Call(call_item));
                }
            }
        }
    }

    items
}

/// Convert a CST @if directive to a PatternIfAst
fn convert_cst_directive_to_pattern_if(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<PatternIfAst> {
    use crate::syntax::cst::AstNode;

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Parse condition from inline args
    // e.g. @if $persist != none { ... }
    let mut condition_parts: Vec<String> = Vec::new();
    for arg in directive.inline_args() {
        if let Some(text) = arg.value_text() {
            condition_parts.push(text);
        }
    }

    // Also check parenthesized args
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            let text = arg_node.text().to_string().trim().to_string();
            condition_parts.push(text);
        }
    }

    let condition_str = condition_parts.join(" ");

    // Parse condition: $var, $var == value, $var != value
    let condition = if condition_str.contains("!=") {
        let parts: Vec<_> = condition_str.split("!=").collect();
        if parts.len() == 2 {
            let var = parts[0].trim().trim_start_matches('$').to_string();
            let value = parse_pattern_value(parts[1].trim());
            PatternCondition::NotEquals(var, value)
        } else {
            PatternCondition::Truthy(condition_str.trim_start_matches('$').to_string())
        }
    } else if condition_str.contains("==") {
        let parts: Vec<_> = condition_str.split("==").collect();
        if parts.len() == 2 {
            let var = parts[0].trim().trim_start_matches('$').to_string();
            let value = parse_pattern_value(parts[1].trim());
            PatternCondition::Equals(var, value)
        } else {
            PatternCondition::Truthy(condition_str.trim_start_matches('$').to_string())
        }
    } else {
        PatternCondition::Truthy(condition_str.trim_start_matches('$').to_string())
    };

    // Parse body recursively
    let body = convert_cst_body_to_pattern_items(directive.body(), source);

    Some(PatternIfAst {
        condition,
        body,
        span,
    })
}

/// Convert a CST @state_machine directive to a StateMachineAst
fn convert_cst_directive_to_state_machine(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<StateMachineAst> {
    use crate::syntax::cst::{AstNode, NamedArg, SyntaxKind};

    let mut initial = String::new();

    // Extract "initial" arg from arg list
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::NAMED_ARG
                && let Some(named_arg) = NamedArg::cast(arg_node)
            {
                let name = named_arg.name_text().unwrap_or_default();
                if name == "initial"
                    && let Some(value) = named_arg.value_text()
                {
                    initial = value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    // Handle variable references
                    if initial.starts_with('$') {
                        initial = initial.to_string(); // Keep as variable reference
                    }
                }
            }
        }
    }

    Some(StateMachineAst {
        initial,
        inline_states: Vec::new(),
        arrow_transitions: Vec::new(),
    })
}

/// Convert a CST @state directive to a StateAst
fn convert_cst_directive_to_state(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<StateAst> {
    use crate::syntax::cst::{AstNode, NamedArg, SyntaxKind};

    let mut when = String::new();

    // Extract "when" arg from arg list
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::NAMED_ARG
                && let Some(named_arg) = NamedArg::cast(arg_node)
            {
                let name = named_arg.name_text().unwrap_or_default();
                if name == "when"
                    && let Some(value) = named_arg.value_text()
                {
                    when = value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                }
            }
        }
    }

    // Extract CSS properties from body
    let mut properties = Vec::new();
    if let Some(body) = directive.body() {
        for css_prop in body.properties() {
            let prop_name = css_prop.name_text().unwrap_or_default();
            let prop_value = css_prop
                .value()
                .map(|v| v.syntax().text().to_string())
                .unwrap_or_default();
            properties.push(StatePropertyAst {
                property: prop_name,
                value: prop_value.trim().to_string(),
            });
        }
    }

    Some(StateAst { when, properties })
}

/// Convert a CST @transition directive to a TransitionAst
fn convert_cst_directive_to_transition(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<TransitionAst> {
    use crate::syntax::cst::{AstNode, NamedArg, SyntaxKind};

    let mut from = String::new();
    let mut to = String::new();
    let mut on = String::new();
    let mut run = None;
    let mut debounce_ms = None;

    // Extract args from arg list
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::NAMED_ARG
                && let Some(named_arg) = NamedArg::cast(arg_node)
            {
                let name = named_arg.name_text().unwrap_or_default();
                if let Some(value) = named_arg.value_text() {
                    let value = value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    match name.as_str() {
                        "from" => from = value,
                        "to" => to = value,
                        "on" => on = value,
                        "run" => run = Some(value),
                        "debounce_ms" => debounce_ms = value.parse().ok(),
                        _ => {}
                    }
                }
            }
        }
    }

    Some(TransitionAst {
        from,
        to,
        on,
        run,
        debounce_ms,
    })
}

/// Convert a CST @async_transition directive to an AsyncTransitionAst
fn convert_cst_directive_to_async_transition(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<AsyncTransitionAst> {
    use crate::syntax::cst::{AstNode, NamedArg, SyntaxKind};

    let mut trigger = String::new();
    let mut from = String::new();
    let mut on_success = String::new();
    let mut on_error = String::new();
    let mut effect = None;

    // Extract args from arg list
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::NAMED_ARG
                && let Some(named_arg) = NamedArg::cast(arg_node)
            {
                let name = named_arg.name_text().unwrap_or_default();
                if let Some(value) = named_arg.value_text() {
                    let value = value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    match name.as_str() {
                        "trigger" => trigger = value,
                        "from" => from = value,
                        "on_success" => on_success = value,
                        "on_error" => on_error = value,
                        "persist" => effect = Some(EffectAst::Persist(value)),
                        "fetch" => effect = Some(EffectAst::Fetch(value)),
                        _ => {}
                    }
                }
            }
        }
    }

    Some(AsyncTransitionAst {
        trigger,
        from,
        on_success,
        on_error,
        effect,
    })
}

/// Convert a CST @mutate directive to a MutateAst
fn convert_cst_directive_to_mutate(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<MutateAst> {
    use crate::syntax::cst::{Arg, AstNode, SyntaxKind};

    // Determine context from inline args or arg list
    let mut context = MutateContextAst::Selection;

    // Check inline args first (e.g., @mutate selection { ... })
    for inline_arg in directive.inline_args() {
        if let Some(text) = inline_arg.value_text() {
            let text = text.trim();
            context = match text {
                "selection" => MutateContextAst::Selection,
                _ if text.starts_with("on_enter:") => MutateContextAst::OnEnter(
                    text.trim_start_matches("on_enter:").trim().to_string(),
                ),
                _ if text.starts_with("on_exit:") => {
                    MutateContextAst::OnExit(text.trim_start_matches("on_exit:").trim().to_string())
                }
                _ if text.starts_with("on_event:") => MutateContextAst::OnEvent(
                    text.trim_start_matches("on_event:").trim().to_string(),
                ),
                _ => MutateContextAst::Selection,
            };
        }
    }

    // Check arg list (e.g., @mutate(on_enter: "editing") { ... })
    if let Some(arg_list) = directive.arg_list() {
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::ARG
                && let Some(arg) = Arg::cast(arg_node)
                && let Some(text) = arg.value_text()
            {
                let text = text.trim();
                if text == "selection" {
                    context = MutateContextAst::Selection;
                }
            }
        }
    }

    // Extract operations from body
    let mut operations = Vec::new();
    if let Some(body) = directive.body() {
        for css_prop in body.properties() {
            let op = css_prop.name_text().unwrap_or_default();
            let mut args = std::collections::HashMap::new();

            if let Some(value) = css_prop.value() {
                let value_text = value.syntax().text().to_string();
                // Simple parsing: "key: value" pairs or just a value
                args.insert("value".to_string(), value_text.trim().to_string());
            }

            operations.push(MutateOpAst { op, args });
        }
    }

    Some(MutateAst {
        context,
        operations,
    })
}

/// Convert a CST @for directive to a PatternForAst
fn convert_cst_directive_to_pattern_for(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<PatternForAst> {
    use crate::syntax::cst::AstNode;

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Parse @for $item in $list { body } or @for $item in [a, b, c] { body }
    // Collect inline args
    let mut variable = String::new();
    let mut source_val = PatternForSource::Variable(String::new());
    let mut seen_in = false;

    for inline_arg in directive.inline_args() {
        if let Some(text) = inline_arg.value_text() {
            let text = text.trim();
            if text == "in" {
                seen_in = true;
            } else if !seen_in {
                // Before "in" - this is the variable
                variable = text.trim_start_matches('$').to_string();
            } else {
                // After "in" - this is the source
                if text.starts_with('$') {
                    source_val =
                        PatternForSource::Variable(text.trim_start_matches('$').to_string());
                } else if text.starts_with('[') {
                    // List literal
                    let inner = text.trim_start_matches('[').trim_end_matches(']').trim();
                    if inner.is_empty() {
                        source_val = PatternForSource::Literal(Vec::new());
                    } else {
                        let items: Vec<CapturedValue> = inner
                            .split(',')
                            .map(|item| parse_pattern_value(item.trim()))
                            .collect();
                        source_val = PatternForSource::Literal(items);
                    }
                } else {
                    source_val = PatternForSource::Variable(text.to_string());
                }
            }
        }
    }

    // Parse body recursively
    let body = convert_cst_body_to_pattern_items(directive.body(), source);

    Some(PatternForAst {
        variable,
        source: source_val,
        body,
        span,
    })
}

/// Convert a CST @match directive to a PatternMatchAst
fn convert_cst_directive_to_pattern_match(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<PatternMatchAst> {
    use crate::syntax::cst::AstNode;

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Parse @match $var { pattern => { body }, ... }
    // Extract variable from inline args
    let mut variable = String::new();
    for inline_arg in directive.inline_args() {
        if let Some(text) = inline_arg.value_text() {
            variable = text.trim().trim_start_matches('$').to_string();
            break;
        }
    }

    // Parse match arms from body
    // Each arm is a directive with the pattern as inline arg and body as the arm body
    let mut arms = Vec::new();
    if let Some(body) = directive.body() {
        for arm_directive in body.directives() {
            // Pattern is in inline args or the directive name
            let pattern_text = arm_directive.name_text().unwrap_or_default();
            let pattern = if pattern_text == "_" {
                PatternMatchPattern::Wildcard
            } else {
                PatternMatchPattern::Literal(parse_pattern_value(&pattern_text))
            };

            // Body is nested
            let arm_body = convert_cst_body_to_pattern_items(arm_directive.body(), source);

            arms.push(PatternMatchArm {
                pattern,
                body: arm_body,
            });
        }
    }

    Some(PatternMatchAst {
        variable,
        arms,
        span,
    })
}

/// Convert a CST @include directive to a PatternIncludeAst
fn convert_cst_directive_to_pattern_include(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<PatternIncludeAst> {
    use crate::syntax::cst::AstNode;

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Extract pattern name from inline args
    let pattern_name = directive
        .inline_args()
        .next()
        .and_then(|a| a.value_text())?;

    // Extract args from arg list
    let args = convert_cst_args_to_pattern_args(directive);

    Some(PatternIncludeAst {
        pattern_name,
        args,
        span,
    })
}

/// Convert a CST directive to a PatternCallAst
fn convert_cst_directive_to_pattern_call(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<PatternCallAst> {
    use crate::syntax::cst::AstNode;

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    let pattern_name = directive.name_text()?;

    // Extract args from arg list
    let args = convert_cst_args_to_pattern_args(directive);

    Some(PatternCallAst {
        pattern_name,
        args,
        span,
    })
}

/// Convert CST directive args to PatternArg vector
fn convert_cst_args_to_pattern_args(directive: &crate::syntax::cst::Directive) -> Vec<PatternArg> {
    use crate::syntax::cst::{Arg, AstNode, NamedArg, SyntaxKind};

    let mut args = Vec::new();

    if let Some(arg_list) = directive.arg_list() {
        let mut pos_index = 0;
        for arg_node in arg_list.args() {
            if arg_node.kind() == SyntaxKind::NAMED_ARG {
                if let Some(named_arg) = NamedArg::cast(arg_node) {
                    let name = named_arg.name_text().unwrap_or_default();
                    let value = named_arg
                        .value_text()
                        .map(|v| parse_pattern_value(v.trim()))
                        .unwrap_or(CapturedValue::String(String::new()));
                    args.push(PatternArg { name, value });
                }
            } else if arg_node.kind() == SyntaxKind::ARG
                && let Some(arg) = Arg::cast(arg_node)
            {
                let value_text = arg.value_text().unwrap_or_default();
                let value = parse_pattern_value(value_text.trim());
                // Use positional name
                let name = format!("arg{}", pos_index);
                args.push(PatternArg { name, value });
                pos_index += 1;
            }
        }
    }

    args
}

/// Parse a string into a CapturedValue
fn parse_pattern_value(s: &str) -> CapturedValue {
    let s = s.trim();
    if s.is_empty() {
        // Empty string -> empty identifier (shouldn't normally happen)
        CapturedValue::Ident(String::new())
    } else if s.starts_with('"') || s.starts_with('\'') {
        CapturedValue::String(s.trim_matches('"').trim_matches('\'').to_string())
    } else if s.starts_with('$') {
        CapturedValue::Binding(s.trim_start_matches('$').to_string())
    } else if s.starts_with('[') {
        // List value
        let inner = s.trim_start_matches('[').trim_end_matches(']').trim();
        if inner.is_empty() {
            // Empty list: []
            CapturedValue::Array(Vec::new())
        } else {
            let items: Vec<CapturedValue> = inner
                .split(',')
                .map(|item| parse_pattern_value(item.trim()))
                .collect();
            CapturedValue::Array(items)
        }
    } else {
        // Try to parse as number, otherwise identifier
        if s.parse::<f64>().is_ok() {
            CapturedValue::String(s.to_string())
        } else {
            CapturedValue::Ident(s.to_string())
        }
    }
}

/// Convert a CST directive to a preset definition
fn convert_cst_directive_to_preset(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<PresetDef> {
    use crate::syntax::cst::AstNode;

    let directive_name = directive.name_text()?;
    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Determine preset type from directive name
    let preset_type = match directive_name.as_str() {
        "easing" => PresetType::Easing,
        "scroll" => PresetType::Scroll,
        "animation" => PresetType::Animation,
        "preset" => {
            // Check first arg for type
            directive
                .inline_args()
                .next()
                .and_then(|a| a.value_text())
                .map(|t| match t.as_str() {
                    "easing" => PresetType::Easing,
                    "scroll" => PresetType::Scroll,
                    "animation" => PresetType::Animation,
                    _ => PresetType::Easing,
                })
                .unwrap_or(PresetType::Easing)
        }
        _ => PresetType::Easing,
    };

    // Get the full inline tokens text for config parsing
    let inline_text = directive.inline_tokens_text().unwrap_or_default();

    // Extract preset name and config from inline args
    let mut name = String::new();
    let mut config_parts = Vec::new();
    let mut found_name = false;
    let mut after_name_colon = false;

    for arg in directive.inline_args() {
        if let Some(text) = arg.value_text() {
            if text.starts_with('~') && !found_name {
                name = text;
                found_name = true;
            } else if found_name {
                if text == ":" && !after_name_colon {
                    after_name_colon = true;
                } else if after_name_colon {
                    config_parts.push(text);
                }
            }
        }
    }

    // Build config string from inline_args (after name's colon) + inline_text
    // inline_args may contain: "start", ":", "0.2"
    // inline_text may contain: ", end : 0.8 ;"
    let mut config_part = if !config_parts.is_empty() {
        // Join with spaces, but reconstruct colon-separated key-value pairs
        let mut result = String::new();
        for (i, part) in config_parts.iter().enumerate() {
            if i > 0
                && *part != ":"
                && *part != ","
                && !result.ends_with(':')
                && !result.ends_with(',')
            {
                result.push(' ');
            }
            result.push_str(part);
        }
        result
    } else {
        String::new()
    };

    // Append inline_text (which may have the rest of the config after a comma)
    let inline_rest = inline_text.trim_end_matches(';').trim();
    if !inline_rest.is_empty() {
        config_part.push_str(inline_rest);
    }

    let mut config_part = config_part.trim().to_string();

    if name.is_empty() {
        return None;
    }

    // If config_part is a function name (spring, cubic-bezier) and there's an arg_list,
    // combine them into a full function call
    if let Some(arg_list) = directive.arg_list() {
        let args: Vec<String> = arg_list
            .args()
            .map(|a| a.text().to_string().trim().to_string())
            .collect();
        if !args.is_empty() && !config_part.is_empty() && !config_part.contains('(') {
            // It's a function name followed by arg_list: spring(350, 15, 1)
            config_part = format!("{}({})", config_part, args.join(", "));
        }
    }

    // Parse the preset value - check if there's a body for animation block
    let value = if let Some(body) = directive.body() {
        // Animation block with properties
        let props = convert_cst_body_to_animation_properties(&body, source);
        PresetValue::AnimationBlock(props)
    } else if !config_part.is_empty() && config_part.contains(':') {
        // Config format: "from: 0, to: 1" or "key: value, key2: value2"
        let config_args = parse_preset_config_args(&config_part);
        if !config_args.is_empty() {
            PresetValue::Config(config_args)
        } else {
            // Fallback for complex value patterns
            parse_preset_value_from_text(&config_part)
        }
    } else if !config_part.is_empty() {
        // Simple value without colons - spring, cubic-bezier, or preset name
        parse_preset_value_from_text(&config_part)
    } else {
        PresetValue::Easing(EasingValue::Preset("ease".to_string()))
    };

    Some(PresetDef {
        preset_type,
        name,
        value,
        span,
    })
}

/// Parse config args from text like "from: 0, to: 1"
fn parse_preset_config_args(text: &str) -> Vec<ConfigArg> {
    let mut args = Vec::new();

    // Split by comma, respecting nesting
    for part in split_respecting_nesting(text, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Split on the first colon only
        if let Some(colon_pos) = part.find(':') {
            let key = part[..colon_pos].trim().to_string();
            let value_str = part[colon_pos + 1..].trim();

            // Parse the value
            let value = if let Ok(num) = value_str.parse::<f64>() {
                ConfigValue::Number(num)
            } else if value_str.starts_with('~') {
                ConfigValue::Preset(value_str.to_string())
            } else if value_str.starts_with('.') || value_str.starts_with('#') {
                ConfigValue::Selector(value_str.to_string())
            } else {
                ConfigValue::String(value_str.to_string())
            };

            args.push(ConfigArg { key, value });
        }
    }

    args
}

/// Parse a preset value from text (spring, cubic-bezier, or preset name)
fn parse_preset_value_from_text(text: &str) -> PresetValue {
    let text = text.trim();

    if text.contains("spring") {
        // Parse spring parameters
        if let Some(params) = parse_function_call_args(text)
            && params.len() >= 3
        {
            return PresetValue::Easing(EasingValue::Spring {
                stiffness: params[0],
                damping: params[1],
                mass: params[2],
            });
        }
        PresetValue::Easing(EasingValue::Spring {
            mass: 1.0,
            stiffness: 100.0,
            damping: 10.0,
        })
    } else if text.contains("cubic-bezier") {
        // Parse cubic bezier
        if let Some(params) = parse_function_call_args(text)
            && params.len() >= 4
        {
            return PresetValue::Easing(EasingValue::CubicBezier(
                params[0], params[1], params[2], params[3],
            ));
        }
        PresetValue::Easing(EasingValue::Preset("ease".to_string()))
    } else {
        // Simple preset name
        PresetValue::Easing(EasingValue::Preset(text.to_string()))
    }
}

/// Convert a CST body to animation properties
fn convert_cst_body_to_animation_properties(
    body: &crate::syntax::cst::Body,
    _source: &str,
) -> Vec<AnimationProperty> {
    use crate::parser::ast::{StaggerDef, StaggerDirection, Value};
    use crate::syntax::cst::AstNode;

    let mut props = Vec::new();

    for css_prop in body.properties() {
        let prop_name = css_prop.name_text().unwrap_or_default();
        let text_range = css_prop.syntax().text_range();
        let prop_span = SourceSpan::new(text_range.start().into(), text_range.end().into());

        if let Some(value) = css_prop.value() {
            let value_text = value.syntax().text().to_string();

            // Check if this is a transition (contains ->)
            if value_text.contains("->") {
                let parts: Vec<_> = value_text.split("->").collect();
                if parts.len() == 2 {
                    // Convert string parts to Value enum
                    let values: Vec<Value> = parts
                        .iter()
                        .map(|s| parse_css_value_to_ast_value(s.trim()))
                        .collect();
                    props.push(AnimationProperty::Transition(TransitionProperty {
                        property: prop_name,
                        values,
                        inline_color_space: None,
                        inline_easing: None,
                        span: prop_span,
                    }));
                }
            } else if prop_name == "range" || prop_name == "offset" {
                // Timing property: range: 0.2 + 0.5 means offset 0.2, span 0.5
                let parts: Vec<_> = value_text.split('+').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    let offset = parts[0].parse::<f64>().unwrap_or(0.0);
                    let span = parts[1].parse::<f64>().ok();
                    props.push(AnimationProperty::Timing { offset, span });
                } else if let Ok(offset) = value_text.trim().parse::<f64>() {
                    props.push(AnimationProperty::Timing { offset, span: None });
                }
            } else if prop_name == "stagger" {
                // Stagger property: delay [grid(cols rows)] [direction]
                // Examples: "0.1 first", "0.005 grid(8 4) first", "0.003 grid(13 13) center"
                let stagger_text = value_text.trim();
                let mut delay = 50.0_f64;
                let mut grid = None;
                let mut direction = StaggerDirection::First;

                // Split on whitespace but keep grid(...) together
                let mut remaining = stagger_text;

                // Parse delay (first token should be a number)
                if let Some(end) = remaining.find(|c: char| c.is_whitespace()) {
                    if let Ok(d) = remaining[..end].parse::<f64>() {
                        delay = d;
                    }
                    remaining = remaining[end..].trim_start();
                } else if let Ok(d) = remaining.parse::<f64>() {
                    delay = d;
                    remaining = "";
                }

                // Parse optional grid(cols rows)
                if remaining.starts_with("grid(")
                    && let Some(close) = remaining.find(')')
                {
                    let inner = &remaining[5..close]; // contents inside grid(...)
                    let nums: Vec<u32> = inner
                        .split_whitespace()
                        .filter_map(|s| s.trim_matches(',').parse::<u32>().ok())
                        .collect();
                    if nums.len() == 2 {
                        grid = Some((nums[0], nums[1]));
                    }
                    remaining = remaining[close + 1..].trim_start();
                }

                // Parse optional direction
                match remaining {
                    "first" => direction = StaggerDirection::First,
                    "last" => direction = StaggerDirection::Last,
                    "center" => direction = StaggerDirection::Center,
                    "random" => direction = StaggerDirection::Random,
                    _ => {
                        if let Ok(idx) = remaining.parse::<u32>() {
                            direction = StaggerDirection::Index(idx);
                        }
                    }
                }

                props.push(AnimationProperty::Stagger(StaggerDef {
                    delay,
                    grid,
                    direction,
                }));
            } else if prop_name == "easing" {
                // Easing property: ~preset-name or cubic-bezier(...)
                let value_text = value_text.trim();
                let easing_value = if value_text.starts_with('~') {
                    EasingValue::Preset(value_text.to_string())
                } else if value_text.contains("cubic-bezier") {
                    if let Some(params) = parse_function_call_args(value_text) {
                        if params.len() >= 4 {
                            EasingValue::CubicBezier(params[0], params[1], params[2], params[3])
                        } else {
                            EasingValue::Preset("ease".to_string())
                        }
                    } else {
                        EasingValue::Preset("ease".to_string())
                    }
                } else if value_text.contains("spring") {
                    if let Some(params) = parse_function_call_args(value_text) {
                        if params.len() >= 3 {
                            EasingValue::Spring {
                                stiffness: params[0],
                                damping: params[1],
                                mass: params[2],
                            }
                        } else {
                            EasingValue::Spring {
                                mass: 1.0,
                                stiffness: 100.0,
                                damping: 10.0,
                            }
                        }
                    } else {
                        EasingValue::Spring {
                            mass: 1.0,
                            stiffness: 100.0,
                            damping: 10.0,
                        }
                    }
                } else {
                    EasingValue::Preset(value_text.to_string())
                };
                props.push(AnimationProperty::Easing(EasingDef {
                    value: easing_value,
                }));
            } else {
                // Regular static property
                props.push(AnimationProperty::Static(StaticProperty {
                    property: prop_name,
                    value: parse_css_value_to_ast_value(value_text.trim()),
                    span: prop_span,
                }));
            }
        }
    }

    props
}

/// Parse a CSS value string to an AST Value
fn parse_css_value_to_ast_value(s: &str) -> crate::parser::ast::Value {
    use crate::parser::ast::Value;

    let s = s.trim();

    // Check for number with unit (simple inline parsing)
    if let Some(unit_start) = s.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        && unit_start > 0
    {
        let num_str = &s[..unit_start];
        let unit = &s[unit_start..];
        if let Ok(num) = num_str.parse::<f64>() {
            let valid_units = [
                "ms", "s", "px", "em", "rem", "%", "vh", "vw", "vmin", "vmax", "deg", "rad",
            ];
            if valid_units.contains(&unit) {
                return Value::NumberWithUnit(num, unit.to_string());
            }
        }
    }

    // Check for number
    if let Ok(n) = s.parse::<f64>() {
        return Value::Number(n);
    }

    // Check for color
    if s.starts_with('#') {
        return Value::Color(s.to_string());
    }

    // Check for string
    if s.starts_with('"') || s.starts_with('\'') {
        return Value::String(s.trim_matches('"').trim_matches('\'').to_string());
    }

    // Default to identifier
    Value::Identifier(s.to_string())
}

/// Parse function call arguments from strings like "spring(350, 15, 1)"
fn parse_function_call_args(s: &str) -> Option<Vec<f64>> {
    let start = s.find('(')?;
    let end = s.rfind(')')?;
    if start >= end {
        return None;
    }
    let args_str = &s[start + 1..end];
    let args: Vec<f64> = args_str
        .split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();
    if args.is_empty() { None } else { Some(args) }
}

/// Convert a CST scope block to an AST scope block
fn convert_cst_scope(scope: &crate::syntax::cst::ScopeBlock, source: &str) -> Option<ScopeBlock> {
    use crate::syntax::cst::AstNode;

    let selector_node = scope.selector()?;
    let selector = selector_node.selector_text().trim().to_string();

    let text_range = scope.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    let mut scope_ast = ScopeBlock {
        // A `.sel { }` CST scope is a CSS selector region (PLAN-039). Construct-introduced
        // regions (template/each bodies) set their own kind where they are lowered to scopes.
        kind: crate::parser::ast::ScopeKind::Selector,
        selector,
        behavior: BehaviorBlock::default(),
        nested_scopes: Vec::new(),
        css_declarations: Vec::new(),
        form_refs: Vec::new(),
        matches: Vec::new(),
        span,
        source_file: None,
        exports: Vec::new(),
        refs: Vec::new(),
        states: Vec::new(),
        html: String::new(),
    };

    // Process body content
    if let Some(body) = scope.body() {
        // Scope directives are now captured as FormMatches by the event parser
        // and assigned to scope_ast.matches via span containment.

        // Process CSS properties
        for prop in body.properties() {
            if let (Some(name), Some(value)) = (prop.name_text(), prop.value()) {
                scope_ast.css_declarations.push(CssDeclaration {
                    property: name,
                    value: value.syntax().text().to_string().trim().to_string(),
                    is_injection: prop.is_arrow_injection(),
                    span: SourceSpan::new(
                        value.syntax().text_range().start().into(),
                        value.syntax().text_range().end().into(),
                    ),
                });
            }
        }

        // Process form splices (`--card-surface;`, `--rise(12px);`) — recognized
        // by the parser as FORM_REF; carried onto the scope so the splice is no
        // longer silently dropped (BUG-241). Validated after rematch; expansion
        // is W3 scope.
        for form_ref in body.form_refs() {
            let Some(name) = form_ref.name() else {
                continue;
            };
            let range = form_ref.syntax().text_range();
            let args = form_ref
                .syntax()
                .children()
                .find(|n| n.kind() == crate::syntax::cst::SyntaxKind::ARG_LIST)
                .map(|n| {
                    let t = n.text().to_string();
                    t.trim_start_matches('(')
                        .trim_end_matches(')')
                        .trim()
                        .to_string()
                });
            scope_ast.form_refs.push(crate::parser::ast::FormRefUse {
                name: name.text().to_string(),
                args,
                span: SourceSpan::new(range.start().into(), range.end().into()),
            });
        }

        // Process nested scopes (reusing ScopeBlock in CST)
        for nested in body.nested_scopes() {
            if let Some(nested_scope) = convert_cst_nested_scope(&nested, source) {
                scope_ast.nested_scopes.push(nested_scope);
            }
        }

        // Emit CSS for @state_machine inline state blocks.
        // Each state-name block generates: [data-st-state="name"] .child-selector { properties }
        for directive in body.directives() {
            if directive.name_text().as_deref() != Some("state_machine") {
                continue;
            }
            let Some(sm_body) = directive.body() else {
                continue;
            };
            for state_block in sm_body.nested_scopes() {
                let Some(state_sel) = state_block.selector() else {
                    continue;
                };
                let state_name = state_sel.selector_text();
                let Some(state_body) = state_block.body() else {
                    continue;
                };
                for child_block in state_body.nested_scopes() {
                    let Some(child_sel_node) = child_block.selector() else {
                        continue;
                    };
                    let child_selector = child_sel_node.selector_text();
                    let scoped_selector = format!(
                        "[data-st-state=\"{}\"] {}",
                        state_name.trim(),
                        child_selector.trim()
                    );
                    if let Some(mut nested_scope) = convert_cst_nested_scope(&child_block, source) {
                        nested_scope.selector = scoped_selector;
                        scope_ast.nested_scopes.push(nested_scope);
                    }
                }
            }
        }
    }

    Some(scope_ast)
}

/// Convert a CST nested scope to an AST nested scope
fn convert_cst_nested_scope(
    nested: &crate::syntax::cst::ScopeBlock,
    source: &str,
) -> Option<NestedScope> {
    use crate::syntax::cst::AstNode;

    let selector_node = nested.selector()?;
    let selector = selector_node.selector_text().trim().to_string();

    let text_range = nested.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    let mut nested_ast = NestedScope {
        kind: crate::parser::ast::ScopeKind::Selector,
        selector,
        composed_selector: String::new(),
        behavior: BehaviorBlock::default(),
        nested_scopes: Vec::new(),
        css_declarations: Vec::new(),
        form_refs: Vec::new(),
        matches: Vec::new(),
        collection_ref: None,
        span,
    };

    // Process body content
    if let Some(body) = nested.body() {
        // Nested scope directives are now captured as FormMatches by the event
        // parser and assigned to nested_ast.matches via span containment.

        for prop in body.properties() {
            if let (Some(name), Some(value)) = (prop.name_text(), prop.value()) {
                nested_ast.css_declarations.push(CssDeclaration {
                    property: name,
                    value: value.syntax().text().to_string().trim().to_string(),
                    is_injection: prop.is_arrow_injection(),
                    span: SourceSpan::new(
                        value.syntax().text_range().start().into(),
                        value.syntax().text_range().end().into(),
                    ),
                });
            }
        }

        // Form splices in a NESTED scope's body — same recognition, same
        // carry (BUG-241). Validated after rematch; STYLE splices expand at the compile seam (BUG-298).
        for form_ref in body.form_refs() {
            let Some(name) = form_ref.name() else {
                continue;
            };
            let range = form_ref.syntax().text_range();
            let args = form_ref
                .syntax()
                .children()
                .find(|n| n.kind() == crate::syntax::cst::SyntaxKind::ARG_LIST)
                .map(|n| {
                    let t = n.text().to_string();
                    t.trim_start_matches('(')
                        .trim_end_matches(')')
                        .trim()
                        .to_string()
                });
            nested_ast.form_refs.push(crate::parser::ast::FormRefUse {
                name: name.text().to_string(),
                args,
                span: SourceSpan::new(range.start().into(), range.end().into()),
            });
        }

        for child in body.nested_scopes() {
            if let Some(child_scope) = convert_cst_nested_scope(&child, source) {
                nested_ast.nested_scopes.push(child_scope);
            }
        }
    }

    Some(nested_ast)
}

/// Convert a CST meta definition to an AST meta definition
/// Collect the contiguous `///` doc-comment block immediately preceding a meta
/// definition node. Walks previous siblings (tokens included): allows only
/// whitespace between comment lines, stops at the first blank line (two
/// newlines), a non-`///` comment, or any real token. Returns the joined lines
/// with the `///` marker and one optional leading space stripped, or `None`
/// when there is no doc block. Shared by FEAT-083 consumers.
fn collect_doc_comment(node: &crate::syntax::cst::SyntaxNode) -> Option<String> {
    use crate::syntax::cst::SyntaxKind;
    let mut lines: Vec<String> = Vec::new();
    let mut started = false;
    let mut el = node.prev_sibling_or_token();
    while let Some(cur) = el {
        match &cur {
            rowan::NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::WHITESPACE => {
                    // A blank line (>=2 newlines) BETWEEN comment lines detaches
                    // the block. A single blank line directly before the
                    // definition (the common `///\n\n%macro` gap) is allowed: it
                    // only matters once we have started collecting `///` lines.
                    if started && t.text().matches('\n').count() >= 2 {
                        break;
                    }
                }
                SyntaxKind::COMMENT if t.text().starts_with("///") => {
                    started = true;
                    let stripped = t
                        .text()
                        .strip_prefix("///")
                        .unwrap_or("")
                        .strip_prefix(' ')
                        .unwrap_or_else(|| t.text().strip_prefix("///").unwrap_or(""));
                    lines.push(stripped.trim_end().to_string());
                }
                // A non-doc comment or any real token ends the block.
                _ => break,
            },
            rowan::NodeOrToken::Node(_) => break,
        }
        el = cur.prev_sibling_or_token();
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

fn convert_cst_meta_def(
    meta: &crate::syntax::cst::MetaDef,
    source: &str,
) -> Option<meta_ast::MetaDef> {
    use crate::syntax::cst::AstNode;
    let doc = collect_doc_comment(meta.syntax());

    let keyword = meta.keyword_text()?;
    // Use explicit_name for primitives/macros which have "keyword name" format
    // Fall back to keyword as name for "%my-macro" style
    let name = meta
        .explicit_name()
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| keyword.clone());
    let text_range = meta.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    match keyword.as_str() {
        "primitive" => {
            let params = convert_cst_meta_params(meta);
            let body = meta
                .body()
                .map(|b| convert_cst_primitive_body(&b, source))
                .unwrap_or_default();
            Some(meta_ast::MetaDef::Primitive(meta_ast::PrimitiveDefAst {
                name,
                params,
                body,
                uses: meta.uses_names(),
                span,
                source_file: None,
                doc,
            }))
        }
        "macro" => {
            let mut mac = meta_ast::MacroDefAst {
                name,
                span,
                doc,
                ..Default::default()
            };

            // Parse body clauses
            if let Some(body) = meta.body() {
                populate_macro_from_clauses(&mut mac, &body, source);
            }

            Some(meta_ast::MetaDef::Macro(mac))
        }

        "preset" => {
            // %preset category name { value }
            // For now, skip preset definitions as they're typically in stdlib
            None
        }
        "capture_type" | "captureType" => {
            // %capture_type name { pattern }
            parse_capture_type_def(meta, name, span)
        }
        "migration" => {
            // %migration id { %date; %docs; %macro…; %rewrite…; %hint… }
            parse_migration_def(meta, source, name, span, doc)
        }
        "comment_type" | "commentType" => {
            // %comment_type id { %label; %docs; %field…; %color; %agent_hint }
            parse_comment_type_def(meta, name, span, doc)
        }
        "scalar_type" | "scalarType" => {
            // %scalar_type id { %capture; %schema; %format; %zero; %widget; %docs }
            parse_scalar_type_def(meta, name, span, doc)
        }
        "runtime-registry" => {
            // %runtime-registry name { %target js { ... } }
            parse_runtime_registry(meta, source, name, span)
        }
        "vendor" => {
            // %vendor name { source: ...; entry: ...; bundle: ...; exports: { ... }; out: ...; license: ... }
            parse_vendor_def(meta, name, span)
        }
        _ => None,
    }
}

/// Populate a `%macro` def from its body clauses — shared by the top-level
/// `%macro` arm and a `%macro` EMBEDDED in a `%migration` capsule (PLAN-079),
/// where the nested macro arrives as a DIRECTIVE named "macro" whose body has
/// the identical clause shape.
fn populate_macro_from_clauses(
    mac: &mut meta_ast::MacroDefAst,
    body: &crate::syntax::cst::Body,
    source: &str,
) {
    use crate::syntax::cst::AstNode;
    for directive in body.directives() {
        let dir_name = directive.name_text().unwrap_or_default();
        match dir_name.as_str() {
            "creates" => {
                // %creates is deprecated and ignored — %form is the source of truth
            }
            "form" => {
                // %form { pattern }
                mac.form = convert_cst_directive_to_form_clause(&directive);
            }
            "binds" => {
                // %binds { primitive(...) -> { outputs } ... }
                // A single %binds block can contain multiple bind declarations
                if let Some(binds_body) = directive.body() {
                    let binds = parse_binds_from_body_text(&binds_body);
                    mac.binds.extend(binds);
                }
            }
            "scope" => {
                // %scope file | selector | property-value
                // Get all inline tokens text and split on |
                if let Some(inline_text) = directive.inline_tokens_text() {
                    for scope_part in inline_text.split('|') {
                        let part = scope_part.trim();
                        // The CST reconstructs inline text with SINGLE SPACES
                        // between tokens, so `within(template)` and
                        // `element(canvas)` arrive as `within ( template )` /
                        // `element ( canvas )`. Compact whitespace before
                        // matching the parenthesised kinds so both the spaceful
                        // and space-free spellings work.
                        let compact: String = part.chars().filter(|c| !c.is_whitespace()).collect();
                        // %scope within(<construct>) — kinds-as-data restriction
                        // (PLAN-039). The construct name is a registry key; matched
                        // by a containment query over the scope stack.
                        if let Some(inner) = compact
                            .strip_prefix("within(")
                            .and_then(|s| s.strip_suffix(')'))
                        {
                            let name = inner.trim();
                            if !name.is_empty() {
                                mac.scope_within.push(name.to_string());
                            }
                            continue;
                        }
                        // %scope element(<tag>) — the bound selector scope's
                        // implied element must be the declared tag (GH-12). Same
                        // kinds-as-data shape as within(<construct>); enforced by
                        // validate_scope_element_constraints in the pipeline.
                        if let Some(inner) = compact
                            .strip_prefix("element(")
                            .and_then(|s| s.strip_suffix(')'))
                        {
                            let tag = inner.trim();
                            if !tag.is_empty() {
                                mac.scope_element.push(tag.to_string());
                            }
                            continue;
                        }
                        match part {
                            "file" => mac.scopes.push(meta_ast::MacroScope::File),
                            "selector" => mac.scopes.push(meta_ast::MacroScope::Selector),
                            // Only file|selector are enforced by
                            // macro_scope_matches; any other scope token is
                            // ignored (use `%scope within(<construct>)` for
                            // construct-containment — macro_within_matches).
                            _ => {}
                        }
                    }
                } else if let Some(first_arg) = directive.inline_args().next() {
                    // Fallback to single arg
                    let scope_str = first_arg.value_text().unwrap_or_default();
                    match scope_str.as_str() {
                        "file" => mac.scopes.push(meta_ast::MacroScope::File),
                        "selector" => mac.scopes.push(meta_ast::MacroScope::Selector),
                        _ => {}
                    }
                }
            }
            "resolves" => {
                // %resolves { $symbol -> registry }
                if let Some(resolves_body) = directive.body() {
                    let body_text = extract_body_content(&resolves_body);
                    let mappings = parse_resolves_mappings(&body_text);
                    if !mappings.is_empty() {
                        let body_range = resolves_body.syntax().text_range();
                        mac.resolves = Some(meta_ast::ResolvesClause {
                            mappings,
                            span: SourceSpan::new(
                                body_range.start().into(),
                                body_range.end().into(),
                            ),
                        });
                    }
                }
            }
            "derives" => {
                // %derives { name: expr }
                // Parse from raw body text: the CST property walker
                // truncates multi-line ternary values (a `$x` whose
                // `? : ` arms span lines would swallow the next decl)
                // and drops hyphenated keys. parse_clause_decls keeps
                // each multi-line value intact. PLAN-054.
                if let Some(derives_body) = directive.body() {
                    let body_range = derives_body.syntax().text_range();
                    let inner = extract_body_content(&derives_body);
                    for (name, expr) in parse_clause_decls(&inner) {
                        if name.is_empty() {
                            continue;
                        }
                        mac.derives.push(meta_ast::DeriveDecl {
                            name,
                            expr: expr.trim().to_string(),
                            span: SourceSpan::new(
                                body_range.start().into(),
                                body_range.end().into(),
                            ),
                        });
                    }
                }
            }
            "requires" => {
                // %requires { $gl, $width, ... }
                if let Some(req_body) = directive.body() {
                    for var in req_body.variables() {
                        if let Some(var_name) = var.name_text() {
                            mac.requires.push(var_name);
                        }
                    }
                }
            }
            "order" => {
                // %order N — execution priority (lower = earlier)
                if let Some(inline_text) = directive.inline_tokens_text()
                    && let Ok(n) = inline_text.trim().parse::<u32>()
                {
                    mac.order = Some(n);
                }
            }
            "drops" => {
                // %drops ( a b _ ) — capture names this macro deliberately
                // does NOT forward (a documented legacy drop). The
                // capture-consumption lint (E0964) treats each as consumed by
                // declaration (I4 / gh-18). Same grammar as a %rewrite rule's
                // %drops clause.
                mac.drops = directive
                    .inline_tokens_text()
                    .unwrap_or_default()
                    .trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .split_whitespace()
                    .map(|s| s.trim_start_matches('$').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "diagnostic" => {
                // %diagnostic — declared marker: this macro's sole job is to
                // DETECT an invalid shape and emit a diagnostic. Its %form
                // captures are consumed by the diagnostic, not forwarded; the
                // capture-consumption lint (E0964) exempts them (I4 / gh-18).
                mac.diagnostic_only = true;
            }
            "pipeline-consumed" => {
                // %pipeline-consumed — declared marker: the macro's captures
                // are consumed by the compile pipeline or by a custom capture
                // type's decomposed record, neither of which the lint's
                // name-scan can see. The author asserts it (I4 / gh-18).
                mac.pipeline_consumed = true;
            }
            "states" => {
                // %states {
                //   idle { cursor: grab }
                //   dragging when $active { cursor: grabbing }
                //   $customStates?
                // }
                if let Some(states_body) = directive.body() {
                    let body_range = states_body.syntax().text_range();
                    let body_text = extract_body_content(&states_body);

                    // Parse states from the body text directly
                    // The CST doesn't properly parse bare identifier { } blocks as scope blocks
                    let state_defs = parse_meta_states_from_text(&body_text);

                    if !state_defs.is_empty() {
                        mac.states = Some(meta_ast::MetaStatesClause {
                            states: state_defs,
                            span: SourceSpan::new(
                                body_range.start().into(),
                                body_range.end().into(),
                            ),
                        });
                    }
                }
            }
            "registers" => {
                // %registers category(args) { items }
                //
                // `inline_tail_text` (not `inline_tokens_text`): a `$capture` arg
                // (`%registers form($name, kind: "style")`, `binding($name, type:
                // $type)`) parses into a VARIABLE_REF *child node*, whose tokens the
                // token-only walker skips — so every captured key and every captured
                // field value was SILENTLY DROPPED from the clause's args, leaving
                // `binding($name, type: $type)` reading back as `[Named { name:
                // "type", var: "" }]`. The descending walker keeps them.
                // `entry_key` treats a `$capture` positional as "no static key"
                // (its documented contract), so retaining them widens what is
                // READABLE without changing what is statically keyed.
                if let Some(inline_text) = directive.inline_tail_text() {
                    let inline_text = inline_text.trim().to_string();
                    let body_text = directive
                        .body()
                        .map(|b| extract_body_content(&b))
                        .unwrap_or_default();
                    if let Some(clause) =
                        parse_registers_clause(&inline_text, &body_text, &directive)
                    {
                        mac.registers = Some(clause);
                    }
                }
            }
            "imports" => {
                // %imports { module: $path, as: $alias, only: (...),
                //            hiding: (...), global: bool }
                // A declared compile-time module-load effect (FEAT-118).
                let body_text = directive
                    .body()
                    .map(|b| extract_body_content(&b))
                    .unwrap_or_default();
                let span = SourceSpan::new(
                    directive.syntax().text_range().start().into(),
                    directive.syntax().text_range().end().into(),
                );
                if let Some(clause) = parse_imports_clause(&body_text, span) {
                    mac.imports = Some(clause);
                }
            }
            _ => {
                // Other body items (when, on, for, if, etc.)
                if let Some(item) = convert_cst_directive_to_macro_body_item(&directive, source) {
                    mac.body.push(item);
                }
            }
        }
    }
}
/// Parse a syntax migration definition (PLAN-079 capsule):
/// `%migration <id> { %date YYYY-MM-DD; %docs "…";
///   %macro <old> { … }…  %rewrite <rule> { %match {…} %into {…} }…
///   %hint for @directive "…"… }`
///
/// A nested `%macro` arrives as a DIRECTIVE named "macro" (the meta-body
/// clause parser treats every `%x` uniformly); its body clauses populate a
/// real `MacroDefAst` through the SAME `populate_macro_from_clauses` path as
/// top-level `%macro` — no parallel grammar. `%match` patterns parse through
/// the SAME form-pattern path as `%form` (`convert_cst_directive_to_form_clause`).
/// Structural validation (ISO date, coverage, hole names) lives in
/// `metasystem::validate::validate_migration`, enforced at registry load.
fn parse_migration_def(
    meta: &crate::syntax::cst::MetaDef,
    source: &str,
    name: String,
    span: SourceSpan,
    doc: Option<String>,
) -> Option<meta_ast::MetaDef> {
    use crate::syntax::cst::AstNode;
    let mut mig = meta_ast::MigrationDefAst {
        id: name,
        span,
        doc,
        ..Default::default()
    };
    let directive_span = |d: &crate::syntax::cst::Directive| {
        SourceSpan::new(
            d.syntax().text_range().start().into(),
            d.syntax().text_range().end().into(),
        )
    };
    let body = meta.body()?;
    for directive in body.directives() {
        match directive.name_text().as_deref() {
            Some("date") => {
                // `%date 2026-06-09` — inline tokens arrive SPACE-JOINED and
                // the lexer may split the date (`2026 -06 -09`); the ISO
                // shape is what matters, so strip ALL whitespace and let
                // load-time validation judge the shape.
                mig.date = directive
                    .inline_tokens_text()
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
            }
            Some("docs") => {
                mig.docs = directive
                    .inline_tokens_text()
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }
            Some("macro") => {
                // Embedded retired definition (the capsule's old grammar +
                // semantics). `%creates` lines inside are ignored (deprecated)
                // — the %form is the directive source of truth.
                let mac_name = directive
                    .inline_tokens_text()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let mut mac = meta_ast::MacroDefAst {
                    name: mac_name,
                    span: directive_span(&directive),
                    ..Default::default()
                };
                if let Some(mac_body) = directive.body() {
                    populate_macro_from_clauses(&mut mac, &mac_body, source);
                }
                mig.macros.push(mac);
            }
            Some("rewrite") => {
                // %rewrite <rule-id> { %match { <pattern> } %into { <template> } }
                let mut rule = meta_ast::MigrationRewriteAst {
                    id: directive
                        .inline_tokens_text()
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                    span: directive_span(&directive),
                    ..Default::default()
                };
                if let Some(rule_body) = directive.body() {
                    for sub in rule_body.directives() {
                        match sub.name_text().as_deref() {
                            Some("match") => {
                                if let Some(fc) = convert_cst_directive_to_form_clause(&sub) {
                                    rule.match_form = fc;
                                }
                                rule.match_source = sub
                                    .body()
                                    .map(|b| extract_body_content(&b))
                                    .unwrap_or_default();
                            }
                            Some("into") => {
                                rule.template = sub
                                    .body()
                                    .map(|b| extract_body_content(&b))
                                    .unwrap_or_default()
                            }
                            Some("drops") => {
                                // %drops ( a b _ ) — extras the rule may drop
                                // because the old %binds never received them.
                                rule.drops = sub
                                    .inline_tokens_text()
                                    .unwrap_or_default()
                                    .trim()
                                    .trim_start_matches('(')
                                    .trim_end_matches(')')
                                    .split_whitespace()
                                    .map(|s| s.trim_start_matches('$').to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                            }
                            _ => {}
                        }
                    }
                }
                mig.rewrites.push(rule);
            }
            Some("hint") => {
                // %hint for @directive "text" — parse from the directive's
                // raw text (the inline @directive tokenizes as a nested
                // directive node, so inline_tokens_text can drop it).
                let raw = directive.syntax().text().to_string();
                if let Some(hint) = parse_migration_hint(&raw, directive_span(&directive)) {
                    mig.hints.push(hint);
                }
            }
            _ => {}
        }
    }
    Some(meta_ast::MetaDef::Migration(mig))
}
/// Parse a `%comment_type` declaration (PLAN-123) — the DATA behind one kind
/// of `//@` comment.
///
/// ```text
/// %comment_type agent-task {
///   %label "Agent task"
///   %docs  "Work item for an agent: do it, then mark resolved."
///   %field acceptance string      // required — a SPACE introduces the type
///   %field priority   string?     // optional — trailing `?`
///   %color "#e8a13d"
///   %agent_hint "Implement, run the acceptance check, then resolve."
/// }
/// ```
///
/// Deliberately the same shape as `parse_migration_def`: one clause namespace,
/// clauses read off the SAME CST directive machinery. No bespoke scanner is
/// introduced for comment types — a `%`-declaration is a `%`-declaration.
///
/// Structural validation (required clauses present, field kinds known,
/// required-before-optional, unique ids) is deliberately NOT done here: it is
/// ORDER-DEPENDENT across files, so it runs post-load with every entry
/// registered — exactly like `validate_migration_chains`. Parsing stays total,
/// and an unparsable field kind is RECORDED (`unknown_field_kinds`) rather
/// than dropped, so validation can name it instead of the type silently
/// losing a field.
fn parse_comment_type_def(
    meta: &crate::syntax::cst::MetaDef,
    name: String,
    span: SourceSpan,
    doc: Option<String>,
) -> Option<meta_ast::MetaDef> {
    use crate::syntax::cst::AstNode;
    let mut def = meta_ast::CommentTypeDefAst {
        id: name,
        span,
        doc,
        ..Default::default()
    };

    let unquote = |s: String| s.trim().trim_matches('"').to_string();

    let body = meta.body()?;
    for directive in body.directives() {
        let inline = directive.inline_tokens_text().unwrap_or_default();
        let dir_span = SourceSpan::new(
            directive.syntax().text_range().start().into(),
            directive.syntax().text_range().end().into(),
        );
        match directive.name_text().as_deref() {
            Some("label") => def.label = unquote(inline),
            Some("docs") => def.docs = unquote(inline),
            Some("color") => {
                let c = unquote(inline);
                if !c.is_empty() {
                    def.color = Some(c);
                }
            }
            Some("agent_hint") | Some("agent-hint") => {
                // A hint is PROSE that an agent reads as its instructions, so
                // punctuation has to survive. Inline tokens arrive
                // space-joined, which detaches every comma and period
                // ("described , then set status : resolved"), so the quoted
                // span is recovered from the directive's RAW text instead and
                // only the line-wrap whitespace is collapsed.
                let raw = directive.syntax().text().to_string();
                let quoted = raw
                    .find('"')
                    .and_then(|start| {
                        raw.rfind('"')
                            .filter(|end| *end > start)
                            .map(|end| &raw[start + 1..end])
                    })
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| unquote(inline));
                let collapsed = quoted.split_whitespace().collect::<Vec<_>>().join(" ");
                if !collapsed.is_empty() {
                    def.agent_hint = Some(collapsed);
                }
            }
            Some("field") => {
                // `%field <name> <kind>` / `%field <name> <kind>?`
                //
                // The lexer may split `string?` into two tokens (same hazard
                // `%date` documents), and inline tokens arrive space-joined —
                // so the KIND is everything after the name with whitespace
                // removed, not merely the next token. Reading only the second
                // token would silently drop every `?` and make optional
                // fields required.
                let mut parts = inline.split_whitespace();
                let Some(fname) = parts.next() else { continue };
                let rest: String = parts.collect::<Vec<_>>().concat();
                let optional = rest.ends_with('?');
                let kind_text = rest.trim_end_matches('?');
                let kind = match meta_ast::CommentFieldKind::parse(kind_text) {
                    Some(k) => k,
                    None => {
                        // Record and carry on: the type keeps its shape for
                        // every other field, and load-time validation reports
                        // this one by name against `span`.
                        def.unknown_field_kinds
                            .push((fname.to_string(), kind_text.to_string()));
                        continue;
                    }
                };
                def.fields.push(meta_ast::CommentFieldAst {
                    name: fname.to_string(),
                    kind,
                    optional,
                    span: dir_span,
                });
            }
            _ => {}
        }
    }

    Some(meta_ast::MetaDef::CommentType(def))
}

/// Populate a `%scalar_type` def from its body clauses (FEAT-168 / PLAN-122
/// W4). One row of the scalar type table: `%capture`, `%schema`, `%format`,
/// `%zero`, `%widget`, `%docs`. `%schema`, `%zero`, and `%widget` are REQUIRED
/// (validated at load, where the whole roster is in view); `%format`/`%docs`/
/// `%capture` may be empty — the base scalars deliberately carry no format and
/// no domain grammar.
///
/// An unknown sub-clause is RECORDED rather than silently ignored, so a typo'd
/// `%widgt` surfaces at load-time validation instead of quietly doing nothing —
/// the exact failure class this table exists to remove.
fn parse_scalar_type_def(
    meta: &crate::syntax::cst::MetaDef,
    name: String,
    span: SourceSpan,
    doc: Option<String>,
) -> Option<meta_ast::MetaDef> {
    let mut def = meta_ast::ScalarTypeDefAst {
        id: name,
        span,
        doc,
        ..Default::default()
    };

    let unquote = |s: String| s.trim().trim_matches('"').to_string();

    // Presence of the REQUIRED sub-clauses. `%zero` is legitimately the empty
    // string for most scalars, so a required key is checked by whether it was
    // DECLARED, never by whether its value is empty.
    let mut seen_schema = false;
    let mut seen_zero = false;
    let mut seen_widget = false;

    let body = meta.body()?;
    for directive in body.directives() {
        let inline = directive.inline_tokens_text().unwrap_or_default();
        match directive.name_text().as_deref() {
            Some("capture") => def.capture = unquote(inline),
            Some("schema") => {
                def.schema = unquote(inline);
                seen_schema = true;
            }
            Some("format") => def.format = unquote(inline),
            Some("zero") => {
                def.zero = unquote(inline);
                seen_zero = true;
            }
            Some("widget") => {
                def.widget = unquote(inline);
                seen_widget = true;
            }
            Some("docs") => def.docs = unquote(inline),
            // FEAT-109 W5. Which scalar wins when this one ALSO matches a value.
            // Space-separated, most-preferred first; a name here is a claim that
            // this scalar is a FALLBACK for that one.
            //
            // This lives in the table rather than in Rust because it is a fact
            // about the language, and the arc that produced it has deleted eight
            // hand-synced Rust lists for exactly that reason. It cannot be
            // derived: `string` accepts `chartreuse` and refuses `#FF0020`,
            // `color` the reverse — the languages OVERLAP, so there is no
            // containment to compute. What makes `color` beat `string` is that a
            // MEANING beats a SHAPE, which is a judgement and must be declared.
            Some("loses_to") => {
                def.loses_to = unquote(inline)
                    .split_whitespace()
                    .map(|s| s.trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            Some(other) => {
                // Record and carry on: the row keeps its shape for every other
                // clause, and load-time validation reports this one by name
                // against `span`.
                def.unknown_subclauses.push(other.to_string());
            }
            None => {}
        }
    }

    for (seen, key) in [
        (seen_schema, "schema"),
        (seen_zero, "zero"),
        (seen_widget, "widget"),
    ] {
        if !seen {
            def.missing_required_keys.push(key.to_string());
        }
    }

    Some(meta_ast::MetaDef::ScalarType(def))
}

/// Parse `%hint for @directive "text"` from the directive's raw text.
fn parse_migration_hint(raw: &str, span: SourceSpan) -> Option<meta_ast::MigrationHintAst> {
    let rest = raw.trim().strip_prefix("%hint")?.trim_start();
    let rest = rest.strip_prefix("for")?.trim_start();
    let rest = rest.strip_prefix('@')?;
    let (directive, text) = rest.split_once(char::is_whitespace)?;
    let text = text.trim().trim_matches('"');
    // The raw slice keeps the source's escapes — fold `\"` back to `"` so
    // diagnostics/pill rows don't show backslashes (R4).
    let text = text.replace("\\\"", "\"");
    if directive.is_empty() || text.is_empty() {
        return None;
    }
    let text = text.to_string();
    Some(meta_ast::MigrationHintAst {
        directive: directive.trim().to_string(),
        text,
        span,
    })
}

/// Parse a runtime registry definition
fn parse_runtime_registry(
    meta: &crate::syntax::cst::MetaDef,
    source: &str,
    name: String,
    span: SourceSpan,
) -> Option<meta_ast::MetaDef> {
    let body = meta.body()?;
    let mut targets = Vec::new();

    for directive in body.directives() {
        if directive.name_text().as_deref() == Some("target")
            && let Some(target) = parse_registry_target(&directive, source)
        {
            targets.push(target);
        }
    }

    Some(meta_ast::MetaDef::RuntimeRegistry(
        meta_ast::RuntimeRegistryDef {
            name,
            targets,
            span,
            source_file: None,
        },
    ))
}

/// Parse a capture type definition: %capture_type name { pattern }
/// Parse a vendor definition: %vendor name { source: ...; entry: ...; bundle: bun|direct;
/// exports: { a, b }; out: ...; license: MIT }
///
/// The body is a field DSL of `key: value` declarations (`;`- or newline-separated).
/// `exports` takes a `{ comma, list }`; all other fields take a bareword or string.
fn parse_vendor_def(
    meta: &crate::syntax::cst::MetaDef,
    name: String,
    span: SourceSpan,
) -> Option<meta_ast::MetaDef> {
    let body = meta.body()?;
    let body_content = extract_body_content(&body);
    let fields = parse_vendor_fields(&body_content);

    let get = |k: &str| {
        fields
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    };

    // `source: submodule "path"` — strip the leading `submodule` keyword if present.
    let source_raw = get("source")?;
    let source = source_raw
        .strip_prefix("submodule")
        .unwrap_or(&source_raw)
        .trim()
        .trim_matches('"')
        .to_string();

    let entry = get("entry")?.trim_matches('"').to_string();

    let bundle = match get("bundle")?.trim() {
        "bun" => meta_ast::VendorBundleStrategy::Bun,
        "direct" => meta_ast::VendorBundleStrategy::Direct,
        _ => return None,
    };

    let out = get("out")?.trim_matches('"').to_string();
    let license = get("license").map(|l| l.trim_matches('"').to_string());

    let exports = get("exports")
        .map(|e| {
            e.trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(meta_ast::MetaDef::Vendor(meta_ast::VendorDefAst {
        name,
        source,
        entry,
        bundle,
        exports,
        out,
        license,
        span,
        source_file: None,
    }))
}

/// Parse the `%vendor` body's `key: value` field declarations.
///
/// Splits on top-level `;` and newlines, but keeps `{ … }` groups (the `exports`
/// list) intact. Returns `(key, value)` pairs with the key lowercased/trimmed.
fn parse_vendor_fields(body: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    let flush = |seg: &str, fields: &mut Vec<(String, String)>| {
        let seg = seg.trim();
        if seg.is_empty() {
            return;
        }
        // Strip line comments (`// …`) before the value.
        let seg = seg.split("//").next().unwrap_or(seg).trim();
        if seg.is_empty() {
            return;
        }
        if let Some(colon) = seg.find(':') {
            let key = seg[..colon].trim().to_string();
            let value = seg[colon + 1..].trim().to_string();
            if !key.is_empty() {
                fields.push((key, value));
            }
        }
    };

    for ch in body.chars() {
        match ch {
            '{' => {
                depth += 1;
                current.push(ch);
            }
            '}' => {
                depth -= 1;
                current.push(ch);
            }
            ';' | '\n' if depth == 0 => {
                flush(&current, &mut fields);
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    flush(&current, &mut fields);
    fields
}
fn parse_capture_type_def(
    meta: &crate::syntax::cst::MetaDef,
    name: String,
    span: SourceSpan,
) -> Option<meta_ast::MetaDef> {
    let body = meta.body()?;
    let body_content = extract_body_content(&body);
    let pattern = parse_capture_pattern(&body_content)?;

    Some(meta_ast::MetaDef::CaptureType(
        meta_ast::CaptureTypeDefAst {
            name,
            pattern,
            span,
            source_file: None,
        },
    ))
}

/// Parse a capture pattern from a string
/// Pattern DSL:
/// - $name:type or &name:type - capture with modifier
/// - "literal" - literal string match
/// - [chars] or [^chars] - character class
/// - ( ... )* ( ... )+ ( ... )? - group with modifier
/// - ... | ... - choice (alternation)
pub(crate) fn parse_capture_pattern(input: &str) -> Option<meta_ast::CapturePatternAst> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let mut parser = CapturePatternParser::new(input);
    parser.parse_pattern()
}

/// Parser for capture patterns
struct CapturePatternParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> CapturePatternParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    /// Parse top-level pattern (handles alternation)
    fn parse_pattern(&mut self) -> Option<meta_ast::CapturePatternAst> {
        self.skip_whitespace();

        let first = self.parse_sequence()?;

        self.skip_whitespace();

        // Check for alternation
        if self.peek() == Some('|') {
            let mut choices = vec![first];
            while self.consume('|') {
                self.skip_whitespace();
                if let Some(alt) = self.parse_sequence() {
                    choices.push(alt);
                }
                self.skip_whitespace();
            }
            if choices.len() == 1 {
                Some(choices.remove(0))
            } else {
                Some(meta_ast::CapturePatternAst::Choice(choices))
            }
        } else {
            Some(first)
        }
    }

    /// Parse a sequence of elements
    fn parse_sequence(&mut self) -> Option<meta_ast::CapturePatternAst> {
        let mut elements = Vec::new();

        while let Some(elem) = self.parse_element() {
            elements.push(elem);
            self.skip_whitespace();

            // Stop at alternation or group close
            if matches!(self.peek(), Some('|') | Some(')') | None) {
                break;
            }
        }

        match elements.len() {
            0 => None,
            1 => Some(elements.remove(0)),
            _ => Some(meta_ast::CapturePatternAst::Sequence(elements)),
        }
    }

    /// Parse a single element with optional modifier
    fn parse_element(&mut self) -> Option<meta_ast::CapturePatternAst> {
        self.skip_whitespace();

        let elem = match self.peek()? {
            '$' | '&' => self.parse_capture()?,
            '"' => self.parse_literal()?,
            '[' => self.parse_char_class()?,
            '(' => self.parse_group()?,
            ')' | '|' => return None, // End of current context
            _ => return None,         // Unknown
        };

        Some(elem)
    }

    /// Parse a capture: $name:type or &name:type with optional modifier
    fn parse_capture(&mut self) -> Option<meta_ast::CapturePatternAst> {
        let prefix = self.peek()?;
        if prefix != '$' && prefix != '&' {
            return None;
        }
        self.pos += 1;

        // Parse name
        let name_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        let var_name = self.input[name_start..self.pos].to_string();

        // Expect colon and type
        self.skip_whitespace();
        if !self.consume(':') {
            return None;
        }

        // Parse type name (bare alphanumerics)
        let type_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        let type_name = self.input[type_start..self.pos].to_string();

        // Parameterized terminal: `balanced(';')` — consume the `(arg)` suffix so the
        // delimiter becomes part of the type (PLAN-023 W2). Other types ignore a trailing
        // `(` (it belongs to the surrounding pattern grammar).
        let capture_type = if type_name == "balanced" && self.peek() == Some('(') {
            self.pos += 1; // consume '('
            let delim = self.parse_balanced_delim_arg();
            meta_ast::CaptureType::Balanced(delim.unwrap_or(';'))
        } else {
            parse_capture_type(&type_name)
        };

        // Check for modifier
        self.skip_whitespace();
        let modifier = self.parse_modifier();

        Some(meta_ast::CapturePatternAst::Capture {
            var_name,
            capture_type,
            modifier,
        })
    }

    /// Parse the delimiter argument of `balanced(';')` / `balanced(";")`. Reads a single
    /// quoted char and consumes the closing `)`. Returns the delimiter char, or None on a
    /// malformed arg (caller defaults to ';').
    fn parse_balanced_delim_arg(&mut self) -> Option<char> {
        self.skip_whitespace();
        let quote = self.peek()?;
        if quote != '\'' && quote != '"' {
            return None;
        }
        self.pos += 1; // opening quote
        let delim = self.peek()?;
        self.pos += delim.len_utf8();
        if self.peek() == Some(quote) {
            self.pos += 1; // closing quote
        }
        self.skip_whitespace();
        if self.peek() == Some(')') {
            self.pos += 1; // closing paren
        }
        Some(delim)
    }

    /// Parse a string literal: "text"
    fn parse_literal(&mut self) -> Option<meta_ast::CapturePatternAst> {
        if !self.consume('"') {
            return None;
        }

        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '"' {
                let content = self.input[start..self.pos].to_string();
                self.pos += 1; // consume closing quote

                // Check for modifier
                self.skip_whitespace();
                let modifier = self.parse_modifier();

                // Return as a group with modifier if modifier is present
                let literal = meta_ast::CapturePatternAst::Literal(content);
                if modifier != meta_ast::CaptureModifier::Required {
                    return Some(crate::parser::meta_ast::CapturePatternAst::Group {
                        pattern: Box::new(literal),
                        modifier: Some(modifier),
                    });
                }
                return Some(literal);
            } else if c == '\\' {
                self.pos += 1;
                if self.peek().is_some() {
                    self.pos += self.peek().unwrap().len_utf8();
                }
            } else {
                self.pos += c.len_utf8();
            }
        }
        None
    }

    /// Parse a character class: [chars] or [^chars]
    fn parse_char_class(&mut self) -> Option<meta_ast::CapturePatternAst> {
        if !self.consume('[') {
            return None;
        }

        let negated = self.consume('^');

        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == ']' {
                let chars = self.input[start..self.pos].to_string();
                self.pos += 1; // consume ]

                // Check for modifier
                let modifier = self.parse_modifier();

                // A COUNTED modifier belongs to the class itself, not to a wrapping
                // group: `[0-9a-f]{3|4|6|8}` is one terminal matching a run of a
                // permitted length, whereas a Group would compose per-character
                // matches and lose the length constraint that IS the token's shape.
                if let meta_ast::CaptureModifier::Counted(counts) = modifier {
                    return Some(meta_ast::CapturePatternAst::CharClass {
                        chars,
                        negated,
                        counts: Some(counts),
                    });
                }

                let char_class = meta_ast::CapturePatternAst::CharClass {
                    chars,
                    negated,
                    counts: None,
                };
                if modifier != meta_ast::CaptureModifier::Required {
                    return Some(crate::parser::meta_ast::CapturePatternAst::Group {
                        pattern: Box::new(char_class),
                        modifier: Some(modifier),
                    });
                }
                return Some(char_class);
            } else if c == '\\' {
                self.pos += 1;
                if self.peek().is_some() {
                    self.pos += self.peek().unwrap().len_utf8();
                }
            } else {
                self.pos += c.len_utf8();
            }
        }
        None
    }

    /// Parse a group: ( pattern )modifier
    fn parse_group(&mut self) -> Option<meta_ast::CapturePatternAst> {
        if !self.consume('(') {
            return None;
        }

        self.skip_whitespace();
        let inner = self.parse_pattern()?;
        self.skip_whitespace();

        if !self.consume(')') {
            return None;
        }

        // Check for modifier
        let modifier = self.parse_modifier();

        Some(crate::parser::meta_ast::CapturePatternAst::Group {
            pattern: Box::new(inner),
            modifier: if modifier == meta_ast::CaptureModifier::Required {
                None
            } else {
                Some(modifier)
            },
        })
    }

    /// Parse an optional modifier: *, +, ?
    fn parse_modifier(&mut self) -> meta_ast::CaptureModifier {
        match self.peek() {
            Some('*') => {
                self.pos += 1;
                crate::parser::meta_ast::CaptureModifier::ZeroOrMore
            }
            Some('+') => {
                self.pos += 1;
                meta_ast::CaptureModifier::OneOrMore
            }
            Some('?') => {
                self.pos += 1;
                crate::parser::meta_ast::CaptureModifier::Optional
            }
            // `{n}` / `{a|b|c}` — counted repetition. A token shape whose LENGTH is
            // the discriminator (hex colour `{3|4|6|8}`, a 4-digit year) cannot be
            // written with `*`/`+`, which are length-blind.
            Some('{') => self.parse_counted_modifier(),
            _ => meta_ast::CaptureModifier::Required,
        }
    }

    /// Parse `{n}` or `{a|b|c}` into a `Counted` modifier.
    ///
    /// A malformed brace run (no digits, unterminated, a count that does not fit a
    /// `u8`) leaves the cursor untouched and yields `Required`, so the `{` falls
    /// through to the surrounding pattern grammar rather than silently becoming a
    /// repetition that matches nothing.
    fn parse_counted_modifier(&mut self) -> meta_ast::CaptureModifier {
        let start = self.pos;
        self.pos += 1; // consume '{'

        let mut counts: Vec<u8> = Vec::new();
        loop {
            self.skip_whitespace();
            let digits_start = self.pos;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
            if self.pos == digits_start {
                self.pos = start;
                return meta_ast::CaptureModifier::Required;
            }
            match self.input[digits_start..self.pos].parse::<u8>() {
                Ok(n) => counts.push(n),
                Err(_) => {
                    self.pos = start;
                    return meta_ast::CaptureModifier::Required;
                }
            }
            self.skip_whitespace();
            match self.peek() {
                Some('|') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                _ => {
                    self.pos = start;
                    return meta_ast::CaptureModifier::Required;
                }
            }
        }

        let set = meta_ast::CountSet::new(counts);
        if set.is_empty() {
            self.pos = start;
            return meta_ast::CaptureModifier::Required;
        }
        meta_ast::CaptureModifier::Counted(set)
    }
}

/// Parse a registry target: %target js { ... }
fn parse_registry_target(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<meta_ast::RegistryTarget> {
    use crate::syntax::cst::AstNode;

    // Get target name from inline tokens text (e.g., "js")
    let target_name = directive
        .inline_tokens_text()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    let body = directive.body()?;
    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    let mut namespace = String::new();
    let mut init = String::new();
    let mut operations = Vec::new();

    for sub_directive in body.directives() {
        let sub_name = sub_directive.name_text().unwrap_or_default();
        match sub_name.as_str() {
            "namespace" => {
                if let Some(ns_body) = sub_directive.body() {
                    namespace = extract_body_content(&ns_body).trim().to_string();
                }
            }
            "init" => {
                if let Some(init_body) = sub_directive.body() {
                    init = extract_body_content(&init_body).trim().to_string();
                }
            }
            "call" => {
                let (params, template) = parse_registry_operation(&sub_directive, source);
                operations.push(meta_ast::RegistryOperation::Call { params, template });
            }
            "register" => {
                let (params, template) = parse_registry_operation(&sub_directive, source);
                operations.push(meta_ast::RegistryOperation::Register { params, template });
            }
            "access" => {
                let (params, template) = parse_registry_operation(&sub_directive, source);
                operations.push(meta_ast::RegistryOperation::Access { params, template });
            }
            _ => {}
        }
    }

    Some(meta_ast::RegistryTarget {
        target_name,
        namespace,
        init,
        operations,
        span,
    })
}

/// Parse registry operation params and template: %call($name, $args) { template }
fn parse_registry_operation(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> (Vec<String>, String) {
    use crate::syntax::cst::AstNode;

    let mut params = Vec::new();

    // Get params from arg list: ($name, $args)
    if let Some(arg_list) = directive.arg_list() {
        for arg in arg_list.args() {
            let text = arg.text().to_string().trim().to_string();
            if text.starts_with('$') {
                params.push(text);
            }
        }
    }

    // If no params from arg_list, extract from child VARIABLE_REF nodes
    if params.is_empty() {
        for child in directive.syntax().children() {
            if child.kind() == crate::syntax::cst::SyntaxKind::VARIABLE_REF {
                let var_text = child.text().to_string().trim().to_string();
                if var_text.starts_with('$') {
                    params.push(var_text);
                }
            }
        }
    }

    // Get template from body
    let template = directive
        .body()
        .map(|b| extract_body_content(&b).trim().to_string())
        .unwrap_or_default();

    (params, template)
}

/// Convert CST meta params to PrimitiveParam
fn convert_cst_meta_params(meta: &crate::syntax::cst::MetaDef) -> Vec<meta_ast::PrimitiveParam> {
    let mut params = Vec::new();

    let Some(arg_list) = meta.arg_list() else {
        return params;
    };

    for arg in arg_list.args() {
        let arg_text = arg.text().to_string();
        let arg_text = arg_text.trim();

        // Parse different param types
        if arg_text.starts_with('&') {
            // &el - element reference
            let name = arg_text[1..].trim().to_string();
            params.push(meta_ast::PrimitiveParam::Element(name));
        } else if let Some(rest) = arg_text.strip_prefix('$') {
            // $name or $name: type - data reference
            if let Some(colon_pos) = rest.find(':') {
                let name = rest[..colon_pos].trim().to_string();
                let ty = rest[colon_pos + 1..].trim().to_string();
                params.push(meta_ast::PrimitiveParam::TypedData { name, ty });
            } else {
                params.push(meta_ast::PrimitiveParam::Data(rest.trim().to_string()));
            }
        } else if let Some(colon_pos) = arg_text.find(':') {
            // name: type = default
            let name = arg_text[..colon_pos].trim().to_string();
            let rest = &arg_text[colon_pos + 1..];

            // Check for default value
            let (type_str, default) = if let Some(eq_pos) = rest.find('=') {
                (rest[..eq_pos].trim(), Some(rest[eq_pos + 1..].trim()))
            } else {
                (rest.trim(), None)
            };

            let ty = parse_meta_param_type(type_str);
            let default = default.map(parse_meta_param_default);

            params.push(meta_ast::PrimitiveParam::Typed { name, ty, default });
        }
    }

    params
}

/// Parse a type string into ParamType
fn parse_meta_param_type(type_str: &str) -> meta_ast::ParamType {
    let type_str = type_str.trim();

    // Check for optional type: type?
    if type_str.ends_with('?') {
        let inner = type_str[..type_str.len() - 1].trim();
        if inner.ends_with("[]") {
            // string[]?
            let base = inner[..inner.len() - 2].trim();
            return meta_ast::ParamType::OptionalArray(base.to_string());
        }
        return meta_ast::ParamType::Optional(inner.to_string());
    }

    // Check for array type
    if type_str.ends_with("[]") {
        let inner = type_str[..type_str.len() - 2].trim();
        return meta_ast::ParamType::Array(inner.to_string());
    }

    // Check for union type with parentheses: ("x" | "y")
    if type_str.starts_with('(') && type_str.ends_with(')') {
        let inner = &type_str[1..type_str.len() - 1];
        let alternatives: Vec<_> = inner
            .split('|')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect();
        return meta_ast::ParamType::Union(alternatives);
    }

    // Check for union type without parentheses: string | null
    if type_str.contains('|') {
        let alternatives: Vec<_> = type_str.split('|').map(|s| s.trim().to_string()).collect();
        return meta_ast::ParamType::Union(alternatives);
    }

    // Simple type - use ParamType::Simple for all
    meta_ast::ParamType::Simple(type_str.to_string())
}

/// Parse a default value string into ParamDefault
fn parse_meta_param_default(default_str: &str) -> meta_ast::ParamDefault {
    let default_str = default_str.trim();

    // String literal
    if (default_str.starts_with('"') && default_str.ends_with('"'))
        || (default_str.starts_with('\'') && default_str.ends_with('\''))
    {
        let inner = &default_str[1..default_str.len() - 1];
        return meta_ast::ParamDefault::String(inner.to_string());
    }

    // Boolean
    if default_str == "true" {
        return meta_ast::ParamDefault::Bool(true);
    }
    if default_str == "false" {
        return meta_ast::ParamDefault::Bool(false);
    }

    // Number
    if let Ok(num) = default_str.parse::<f64>() {
        return meta_ast::ParamDefault::Number(num);
    }

    // Empty array
    if default_str == "[]" {
        return meta_ast::ParamDefault::EmptyArray;
    }

    // Empty object
    if default_str == "{}" {
        return meta_ast::ParamDefault::EmptyObject;
    }

    // None/null/undefined
    if default_str == "none" || default_str == "null" || default_str == "undefined" {
        return meta_ast::ParamDefault::None;
    }

    // Variable reference or other - treat as string
    meta_ast::ParamDefault::String(default_str.to_string())
}

/// Convert CST body to PrimitiveBody
fn convert_cst_primitive_body(
    body: &crate::syntax::cst::Body,
    source: &str,
) -> meta_ast::PrimitiveBody {
    let mut result = meta_ast::PrimitiveBody::default();

    for directive in body.directives() {
        let dir_name = directive.name_text().unwrap_or_default();
        match dir_name.as_str() {
            "emit" => {
                if let Some(emit) = convert_cst_emit_block(&directive, source) {
                    result.emit_blocks.push(emit);
                }
            }
            "cleanup" => {
                // %cleanup { code }
                if let Some(cleanup_body) = directive.body() {
                    let code = extract_body_content(&cleanup_body);
                    result.cleanup = Some(code);
                }
            }
            "exports" => {
                // %exports { $name: type ... }
                if let Some(exports_body) = directive.body() {
                    result.exports = parse_exports(&exports_body, source);
                }
            }
            "if" => {
                if let Some(if_block) = convert_cst_primitive_if(&directive, source) {
                    result.if_blocks.push(if_block);
                }
            }
            _ => {}
        }
    }

    result
}

/// Convert CST %emit directive to EmitBlock
fn convert_cst_emit_block(
    directive: &crate::syntax::cst::Directive,
    _source: &str,
) -> Option<meta_ast::EmitBlock> {
    use crate::syntax::cst::AstNode;

    // Get the language - try inline_args first (for @ directives)
    // then try inline_tokens_text (for % meta clauses)
    let lang_str = directive
        .inline_args()
        .next()
        .and_then(|a| a.value_text())
        .or_else(|| directive.inline_tokens_text())
        .unwrap_or_default();

    let lang = match lang_str.trim() {
        "js" | "javascript" => meta_ast::EmitLang::Js,
        "css" => meta_ast::EmitLang::Css,
        "glsl" => meta_ast::EmitLang::Glsl,
        "build-js" => meta_ast::EmitLang::BuildJs,
        "prelude-js" => meta_ast::EmitLang::PreludeJs,
        "prelude-css" => meta_ast::EmitLang::PreludeCss,
        "html" => meta_ast::EmitLang::Html,
        _ => return None,
    };

    let body = directive.body()?;
    let content = extract_body_content(&body);

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    Some(meta_ast::EmitBlock {
        lang,
        content,
        span,
    })
}

/// Extract the raw content inside a body (between { and })
fn extract_body_content(body: &crate::syntax::cst::Body) -> String {
    use crate::syntax::cst::AstNode;
    let text = body.syntax().text().to_string();
    // Remove leading { and trailing }
    let text = text.trim();
    if text.starts_with('{') && text.ends_with('}') {
        text[1..text.len() - 1].trim().to_string()
    } else {
        text.to_string()
    }
}

/// Split a `%derives` / `%animates` clause body into `(key, value)` declarations.
///
/// Each declaration is `name: value`; the value MAY span multiple lines and
/// contain nested `?:` ternaries, `( )`, `{ }`, `[ ]`, and `:` (inside those
/// brackets). A new declaration begins at a line whose FIRST depth-0
/// `key:` appears with `key` being a bare identifier (letters/digits/`-`/`_`,
/// optionally `$`-prefixed) — this lets a multi-line ternary value (which
/// contains its own `:`) stay attached to its declaration. Comments (`//`) and
/// blank lines are skipped. PLAN-054.
///
/// Why not the CST `properties()` walker: it parses `translate-x: $x` as a bare
/// VARIABLE_REF (dropping the key) and truncates multi-line ternary values, so
/// both `%animates` and `%derives` need this raw-text splitter to be robust.
fn parse_clause_decls(inner: &str) -> Vec<(String, String)> {
    let mut decls: Vec<(String, String)> = Vec::new();
    let mut cur_key: Option<String> = None;
    let mut cur_val = String::new();
    // Bracket depth carried ACROSS lines so a multi-line ternary value is not
    // split at an inner `key:`-looking fragment.
    let mut depth: i32 = 0;
    // String-literal state carried across lines so a value containing a
    // multi-line string (or an unbalanced bracket inside one) is not miscounted
    // and a line that merely CONTINUES a string is not read as a new decl (#8).
    let mut scan = ClauseScanState::default();

    let flush = |decls: &mut Vec<(String, String)>, key: &mut Option<String>, val: &mut String| {
        if let Some(k) = key.take() {
            decls.push((k, val.trim().to_string()));
        }
        val.clear();
    };

    for raw_line in inner.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        // A new declaration starts only at bracket depth 0, NOT mid-string, AND
        // when the line leads with `<ident>:` (the key). Otherwise the line is a
        // continuation of the current value (e.g. a ternary's `? ... : ...` arms,
        // or the tail of a multi-line string literal).
        let starts_decl = depth == 0 && !scan.in_str && leading_decl_key(line).is_some();
        if starts_decl {
            flush(&mut decls, &mut cur_key, &mut cur_val);
            let (key, rest) = leading_decl_key(line).unwrap();
            cur_key = Some(key);
            cur_val.push_str(rest.trim());
        } else if cur_key.is_some() {
            if !cur_val.is_empty() {
                cur_val.push(' ');
            }
            cur_val.push_str(line);
        }
        // Update bracket depth with this line's net delta, threading string state.
        depth += bracket_delta(line, &mut scan);
        if depth < 0 {
            depth = 0;
        }
    }
    flush(&mut decls, &mut cur_key, &mut cur_val);
    decls
}

/// If `line` begins with `<ident>:` at depth 0, return `(key, remainder)`.
/// `key` is a bare identifier (letters/digits/`-`/`_`), optionally `$`-prefixed,
/// and is stripped of a leading `$`. The `:` must be the FIRST top-level colon
/// (a `::` or a `:` inside no brackets — here we only look at the line's head, so
/// a ternary `a ? b : c` line will not match because it doesn't START with an
/// `ident:`).
fn leading_decl_key(line: &str) -> Option<(String, String)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    // optional leading $
    if i < bytes.len() && bytes[i] == b'$' {
        i += 1;
    }
    let key_start = i;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' {
            i += 1;
        } else {
            break;
        }
    }
    if i == key_start {
        return None;
    }
    // skip spaces before the colon
    let mut j = i;
    while j < bytes.len() && bytes[j] == b' ' {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b':' {
        return None;
    }
    // not a `::` (member/path) — that's not a declaration colon
    if j + 1 < bytes.len() && bytes[j + 1] == b':' {
        return None;
    }
    let key = line[key_start..i].to_string();
    let rest = line[j + 1..].to_string();
    Some((key, rest))
}

/// Net bracket nesting delta of a line, ignoring brackets inside string literals.
/// Lexer state carried ACROSS lines while scanning a clause body (audit #8).
/// `in_str` = currently inside a string literal; `quote` = its opening delimiter.
#[derive(Clone, Copy, Default)]
struct ClauseScanState {
    in_str: bool,
    quote: u8,
}

/// Net bracket nesting delta of a line, threading string-literal state THROUGH
/// `state` so a string spanning multiple lines (or carrying an unbalanced
/// bracket) is not miscounted (PLAN-054 audit #8). Brackets inside a string
/// literal are ignored; `state` is updated in place for the next line.
fn bracket_delta(line: &str, state: &mut ClauseScanState) -> i32 {
    let bytes = line.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if state.in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == state.quote {
                state.in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'\'' | b'`' => {
                state.in_str = true;
                state.quote = c;
            }
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    depth
}

/// Parse meta states from body text
/// Parse a %registers clause from inline text and body text.
///
/// Format: `category(args) { items }`
/// Examples:
///   `type($name)` with body `{ $fields }`
///   `binding($name, type: $type)` with body `{ value: $initial }`
fn parse_registers_clause(
    inline_text: &str,
    body_text: &str,
    directive: &crate::syntax::cst::Directive,
) -> Option<meta_ast::RegistersClause> {
    use crate::syntax::cst::AstNode;

    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Parse name and optional args: "category(args)" or "category"
    let (name, args) = if let Some(paren_pos) = inline_text.find('(') {
        let name = inline_text[..paren_pos].trim().to_string();
        let args_str = &inline_text[paren_pos + 1..];
        let args_str = args_str.trim_end_matches(')').trim();
        let args = parse_registers_args(args_str);
        (name, args)
    } else {
        (inline_text.to_string(), vec![])
    };

    if name.is_empty() {
        return None;
    }

    // Parse body items
    let items = parse_registers_body(body_text);

    Some(meta_ast::RegistersClause {
        name,
        args,
        items,
        span,
    })
}

/// Apply one `MODULE.st` manifest clause (`%module` / `%public` / `%reexport`)
/// to the file's `module_manifest` (FEAT-118 M2). Lazily initializes it.
fn apply_manifest_clause(
    file: &mut StFile,
    meta: &crate::syntax::cst::MetaDef,
    keyword: &str,
    _source: &str,
) {
    use crate::syntax::cst::AstNode;
    let manifest = file.module_manifest.get_or_insert_with(Default::default);
    // The meta node's full source text minus the leading `%keyword`, e.g.
    // `(camera, light)` for `%public (camera, light)` or `"std:scene-3d" (form)`
    // for `%reexport "std:scene-3d" (form)`. MetaDef has no inline-token helper
    // (that lives on Directive), so reconstruct from the node text.
    let full = meta.syntax().text().to_string();
    let inline: Option<String> = full
        .trim_start()
        .strip_prefix('%')
        .map(|rest| rest.trim_start().to_string())
        .map(|rest| {
            // drop the leading keyword word
            match rest.find(char::is_whitespace) {
                Some(i) => rest[i..].trim().to_string(),
                None => String::new(),
            }
        })
        .filter(|s| !s.is_empty());

    // Split a `( a, b, c )` (or bare `a b c`) token run into identifier names.
    let names_from = |s: &str| -> Vec<String> {
        s.replace(['(', ')', ','], " ")
            .split_whitespace()
            .map(|t| t.trim_matches('"').trim_matches('\'').to_string())
            .filter(|t| !t.is_empty())
            .collect()
    };

    match keyword {
        "module" => {
            if let Some(name) = meta.explicit_name().map(|t| t.text().to_string()) {
                manifest.name = Some(name);
            } else if let Some(first) = inline
                .as_deref()
                .and_then(|t| names_from(t).into_iter().next())
            {
                manifest.name = Some(first);
            }
            let r = meta.syntax().text_range();
            manifest.span = SourceSpan::new(r.start().into(), r.end().into());
        }
        "public" => {
            if let Some(text) = inline.as_deref() {
                manifest.public.extend(names_from(text));
            }
        }
        "reexport" => {
            if let Some(text) = inline.as_deref() {
                let module = text
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_matches('"').trim_matches('\'').to_string())
                    .unwrap_or_default();
                let names = text
                    .find('(')
                    .map(|i| names_from(&text[i..]))
                    .unwrap_or_default();
                if !module.is_empty() {
                    manifest
                        .reexports
                        .push(crate::parser::ast::ReexportDecl { module, names });
                }
            }
        }
        _ => {}
    }
}

/// Parse a `%using { … }` active-extension hook (FEAT-118 M3) into the file's
/// `module_manifest.using`. Captures the clause DATA (`%claims`,
/// `%capture_type` names, `%default` lines) + the raw body for the execution
/// layer (FUP-055). Does NOT execute the hook — that needs the ImportScope.
fn apply_using_hook(file: &mut StFile, meta: &crate::syntax::cst::MetaDef, _source: &str) {
    use crate::syntax::cst::AstNode;
    let manifest = file.module_manifest.get_or_insert_with(Default::default);
    let mut hook = crate::parser::ast::UsingHook::default();

    // The hook's body text (between the outer braces).
    let body_text = meta
        .body()
        .map(|b| b.syntax().text().to_string())
        .unwrap_or_default();
    hook.raw_body = body_text.clone();

    // Scan the body line-by-line for the three injected clause kinds. (The body
    // is the metasystem's own surface, so a light line scan suffices here; the
    // execution layer re-parses the raw body when it actually injects.)
    for raw in body_text.lines() {
        let line = raw
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        if let Some(rest) = line.strip_prefix("%claims") {
            // `%claims @camera @light` → [camera, light]
            for tok in rest.split_whitespace() {
                let name = tok.trim_start_matches('@').trim();
                if !name.is_empty() {
                    hook.claims.push(name.to_string());
                }
            }
        } else if let Some(rest) = line.strip_prefix("%capture_type") {
            if let Some(name) = rest.split_whitespace().next() {
                hook.capture_types.push(name.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("%default") {
            let d = rest.trim().trim_end_matches([',', ';']).trim();
            if !d.is_empty() {
                hook.defaults.push(d.to_string());
            }
        }
    }

    manifest.using = Some(hook);
}

/// Parse the invocation TAIL of a `@use` directive into (alias, only, hiding)
/// (FEAT-118 FUP-053). Input is the directive's inline-token text — the run
/// after the directive name, e.g. `"std:scene" as s only (camera, light)`.
///
/// Grammar (all optional, any order after the module string):
///   `as <ident>`            → qualified alias
///   `only ( a, b, … )`     → explicit allow-list
///   `hiding ( a, b, … )`   → open-minus exclusion list
fn parse_use_clause_tail(inline: Option<&str>) -> (Option<String>, Vec<String>, Vec<String>) {
    let Some(text) = inline else {
        return (None, Vec::new(), Vec::new());
    };
    // Tokenize on whitespace and parens; keep `(`/`)`/`,` as delimiters we skip.
    let normalized = text
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace(',', " ");
    let mut toks = normalized.split_whitespace().peekable();
    let mut alias = None;
    let mut only = Vec::new();
    let mut hiding = Vec::new();

    let strip = |s: &str| s.trim_matches('"').trim_matches('\'').to_string();
    // Collect identifiers until the matching `)` after an `only`/`hiding` keyword.
    let collect_list = |toks: &mut std::iter::Peekable<std::str::SplitWhitespace>| -> Vec<String> {
        let mut out = Vec::new();
        // optional leading `(`
        if toks.peek() == Some(&"(") {
            toks.next();
        }
        for t in toks.by_ref() {
            if t == ")" {
                break;
            }
            out.push(t.to_string());
        }
        out
    };

    while let Some(tok) = toks.next() {
        match tok {
            "as" => alias = toks.next().map(strip),
            "only" => only = collect_list(&mut toks),
            "hiding" => hiding = collect_list(&mut toks),
            _ => {} // the leading module string and any stray tokens
        }
    }
    (alias, only, hiding)
}

/// Parse a `%imports` clause body (FEAT-118 / `docs/specs/declared-effects.md`).
///
/// Body is a set of `key: value` lines:
///   `module: $path`        (required) capture holding the module reference
///   `as: $alias`           (optional) capture for the qualified alias
///   `only: ($a, $b)`       (optional) explicit import list captures
///   `hiding: ($a, $b)`     (optional) open-minus list captures
///   `global: true|false`   (optional, default false) `@import` sets true
///
/// Capture sigils (`$`) and surrounding parens are stripped; we store bare
/// capture NAMES (the enactor resolves them against the match's captures).
fn parse_imports_clause(body_text: &str, span: SourceSpan) -> Option<meta_ast::ImportsClause> {
    let strip_cap = |s: &str| s.trim().trim_start_matches('$').trim().to_string();
    let strip_list = |s: &str| -> Vec<String> {
        s.trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .map(&strip_cap)
            .filter(|p| !p.is_empty())
            .collect()
    };

    let mut module: Option<String> = None;
    let mut alias: Option<String> = None;
    let mut only: Vec<String> = Vec::new();
    let mut hiding: Vec<String> = Vec::new();
    let mut global = false;

    // Fields are separated by newlines OR commas — but a comma INSIDE a
    // `(...)` list (`only: ($a, $b)`) is not a field separator. Split on
    // top-level separators (newline + paren-depth-0 comma + semicolon).
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth: i32 = 0;
    for ch in body_text.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            '\n' | ';' => {
                fields.push(std::mem::take(&mut cur));
            }
            ',' if depth == 0 => {
                fields.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
    }
    fields.push(cur);

    for field in fields {
        let line = field.trim();
        if line.is_empty() {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let key = line[..colon].trim();
        let val = line[colon + 1..].trim();
        match key {
            "module" => module = Some(strip_cap(val)),
            "as" => alias = Some(strip_cap(val)),
            "only" => only = strip_list(val),
            "hiding" => hiding = strip_list(val),
            "global" => global = val == "true",
            _ => {}
        }
    }

    // `module` is the one required field — a %imports clause without it is malformed.
    let module = module?;
    Some(meta_ast::ImportsClause {
        module,
        alias,
        only,
        hiding,
        global,
        span,
    })
}

/// Parse registers clause args: "$name" or "$name, type: $type"
fn parse_registers_args(args_str: &str) -> Vec<meta_ast::RegistersArg> {
    let mut args = Vec::new();

    for part in args_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(colon_pos) = part.find(':') {
            // Named arg: "type: $type"
            let name = part[..colon_pos].trim().to_string();
            let var = normalize_sigil_spacing(part[colon_pos + 1..].trim());
            args.push(meta_ast::RegistersArg::Named { name, var });
        } else {
            // Positional arg: "$name"
            args.push(meta_ast::RegistersArg::Positional(normalize_sigil_spacing(
                part,
            )));
        }
    }

    args
}

/// Re-join a sigil to the identifier it binds: `"$ name"` → `"$name"`.
///
/// The clause text is rebuilt by JOINING CST tokens with a space, and `$name`
/// lexes as two tokens (`$` + `name`). Every consumer compares an arg against a
/// capture name (`$name`), so a stray space silently turns a live capture
/// reference into a string that matches nothing. Applied at the one place args
/// are constructed, so no consumer has to re-derive it.
fn normalize_sigil_spacing(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if matches!(c, '$' | '&' | '~') {
            while chars.peek() == Some(&' ') {
                chars.next();
            }
        }
    }
    out
}

/// Parse registers body items: "field: $var", "field: value", "$varref"
fn parse_registers_body(body_text: &str) -> Vec<meta_ast::RegisterItem> {
    let mut items = Vec::new();

    for line in body_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // Variable reference: $fields
        if line.starts_with('$') && !line.contains(':') {
            items.push(meta_ast::RegisterItem::VarRef(line.to_string()));
            continue;
        }

        // Field: name: value
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value_str = line[colon_pos + 1..].trim();

            let value = if value_str.starts_with('$') {
                meta_ast::RegisterValue::Var(value_str.to_string())
            } else if value_str == "true" {
                meta_ast::RegisterValue::Bool(true)
            } else if value_str == "false" {
                meta_ast::RegisterValue::Bool(false)
            } else if value_str.starts_with('[') {
                // Array value
                let inner = value_str.trim_start_matches('[').trim_end_matches(']');
                let arr_items: Vec<meta_ast::RegisterArrayItem> = inner
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        if s.starts_with('$') {
                            meta_ast::RegisterArrayItem::Var(s.to_string())
                        } else if s.starts_with('"') || s.starts_with('\'') {
                            meta_ast::RegisterArrayItem::String(
                                s.trim_matches(|c| c == '"' || c == '\'').to_string(),
                            )
                        } else {
                            meta_ast::RegisterArrayItem::Ident(s.to_string())
                        }
                    })
                    .collect();
                meta_ast::RegisterValue::Array(arr_items)
            } else if value_str.starts_with('"') || value_str.starts_with('\'') {
                meta_ast::RegisterValue::String(
                    value_str
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string(),
                )
            } else {
                // Treat as ident — covers cases like defaultValue($type), ${$name}_loading, etc.
                meta_ast::RegisterValue::Ident(value_str.to_string())
            };

            items.push(meta_ast::RegisterItem::Field(meta_ast::RegisterField {
                name,
                value,
            }));
        }
    }

    items
}

/// Handles patterns like:
///   idle { cursor: grab }
///   dragging when $active { cursor: grabbing }
///   $customStates?
pub(crate) fn parse_meta_states_from_text(text: &str) -> Vec<meta_ast::MetaStateDef> {
    let mut states = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(&c) = chars.peek() {
        // Skip whitespace and comments
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        // Comment
        if c == '/' && chars.clone().nth(1) == Some('/') {
            // Skip to end of line
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == '\n' {
                    break;
                }
            }
            continue;
        }

        // Variable reference: $name or $name?
        if c == '$' {
            chars.next();
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var_name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let is_optional = chars.peek() == Some(&'?');
            if is_optional {
                chars.next();
            }
            if !var_name.is_empty() {
                states.push(meta_ast::MetaStateDef::Variable {
                    name: var_name,
                    optional: is_optional,
                    properties: Vec::new(),
                });
            }
            continue;
        }

        // Named state: name (when condition)? { properties }
        if c.is_alphabetic() || c == '_' {
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Skip whitespace
            while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                chars.next();
            }

            // A `name :` that is NOT followed by a state block is a PROPERTY line
            // (`accepts: $x;`, `on-drop: $f`) that shares the body with the states
            // capture (e.g. `@drop-zone { accepts: …; over { … } }`). Skip it to the
            // terminating `;` (or to the next top-level `}`/EOF) so it never becomes
            // a spurious empty state. A real state uses `name { … }` or
            // `name when … { … }`, never `name :`.
            if chars.peek() == Some(&':') {
                let mut depth = 0i32;
                while let Some(&c) = chars.peek() {
                    match c {
                        '(' | '[' | '{' => depth += 1,
                        ')' | ']' => depth -= 1,
                        '}' if depth == 0 => break,
                        '}' => depth -= 1,
                        ';' if depth == 0 => {
                            chars.next();
                            break;
                        }
                        _ => {}
                    }
                    chars.next();
                }
                continue;
            }

            // Check for "when" condition
            let mut condition = None;
            let remaining: String = chars.clone().take(5).collect();
            if remaining.starts_with("when ") {
                // Consume "when "
                for _ in 0..5 {
                    chars.next();
                }
                // Parse condition until {
                let mut cond = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '{' {
                        break;
                    }
                    cond.push(c);
                    chars.next();
                }
                condition = Some(cond.trim().to_string());
            }

            // Skip whitespace
            while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                chars.next();
            }

            // Parse body { ... }
            let mut properties = Vec::new();
            if chars.peek() == Some(&'{') {
                chars.next(); // consume {

                // Parse properties until }
                let mut brace_depth = 1;
                let mut body_text = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '{' {
                        brace_depth += 1;
                    } else if c == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            chars.next();
                            break;
                        }
                    }
                    body_text.push(c);
                    chars.next();
                }

                // Parse properties from body_text
                properties = parse_state_properties_from_text(&body_text);
            }

            if !name.is_empty() {
                states.push(meta_ast::MetaStateDef::Named {
                    name,
                    condition,
                    properties,
                });
            }
            continue;
        }

        // Skip unknown characters
        chars.next();
    }

    states
}

/// Parse properties from state body text
fn parse_state_properties_from_text(text: &str) -> Vec<meta_ast::MetaStateProperty> {
    let mut properties = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        // Parse property: name: value
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..]
                .trim()
                .trim_end_matches(';')
                .to_string();
            if !name.is_empty() {
                properties.push(meta_ast::MetaStateProperty { name, value });
            }
        }
    }

    properties
}

/// Parse exports from a body
fn parse_exports(body: &crate::syntax::cst::Body, _source: &str) -> Vec<meta_ast::ExportDecl> {
    let mut exports = Vec::new();
    let body_text = extract_body_content(body);

    // Parse $name: type patterns
    // Simple regex-like parsing: find $word then : then type
    let mut chars = body_text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            // Found a variable
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    name.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            // Skip whitespace
            while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                chars.next();
            }

            // Check for colon
            if chars.peek() == Some(&':') {
                chars.next(); // consume :

                // Skip whitespace
                while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                    chars.next();
                }

                // Read type - handle nested parens for fn(params)
                let mut type_str = String::new();
                let mut optional = false;
                let mut paren_depth = 0;

                while let Some(&c) = chars.peek() {
                    if c == '$' || c == '\n' || c == '}' {
                        break;
                    }
                    if c == '(' {
                        paren_depth += 1;
                    } else if c == ')' {
                        paren_depth -= 1;
                    }
                    // Only treat ? as optional marker when outside parens
                    if c == '?' && paren_depth == 0 {
                        optional = true;
                        chars.next();
                        break;
                    }
                    type_str.push(chars.next().unwrap());
                }

                let type_str = type_str.trim();
                let type_expr = parse_export_type_expr(type_str);

                exports.push(meta_ast::ExportDecl {
                    name,
                    type_expr,
                    optional,
                });
            }
        }
    }

    exports
}

/// Parse export type expression
fn parse_export_type_expr(type_str: &str) -> meta_ast::ExportTypeExpr {
    let type_str = type_str.trim();

    // Array type
    if type_str.ends_with("[]") {
        let inner = type_str[..type_str.len() - 2].trim();
        return meta_ast::ExportTypeExpr::Array(inner.to_string());
    }

    // Function type: fn(params) ~> return_type
    if type_str.starts_with("fn") {
        // Check for full function type: fn(params) ~> return
        if let Some(arrow_pos) = type_str.find("~>") {
            let params_part = &type_str[2..arrow_pos].trim();
            let return_type = type_str[arrow_pos + 2..].trim().to_string();

            // Parse params from fn(param_list)
            let params = if params_part.starts_with('(') && params_part.ends_with(')') {
                let params_inner = &params_part[1..params_part.len() - 1];
                parse_fn_type_params(params_inner)
            } else {
                Vec::new()
            };

            return meta_ast::ExportTypeExpr::Function(meta_ast::FunctionTypeExpr {
                params,
                return_type,
            });
        } else if type_str.starts_with("fn(") {
            // Legacy fn with params but no return: fn(type)
            if let Some(close_paren) = type_str.find(')') {
                let params_str = &type_str[3..close_paren];
                return meta_ast::ExportTypeExpr::LegacyFn(Some(params_str.to_string()));
            }
        }
        // Plain fn without params
        return meta_ast::ExportTypeExpr::LegacyFn(None);
    }

    // Union type with |
    if type_str.contains('|') {
        let variants: Vec<_> = type_str
            .split('|')
            .map(|s| meta_ast::UnionVariant {
                name: s.trim().to_string(),
                bindings: vec![],
            })
            .collect();
        return meta_ast::ExportTypeExpr::Union(variants);
    }

    // Simple type
    meta_ast::ExportTypeExpr::Simple(type_str.to_string())
}

/// Parse function type parameters: "number, string?" -> [FunctionTypeParam, ...]
fn parse_fn_type_params(params_str: &str) -> Vec<meta_ast::FunctionTypeParam> {
    let mut params = Vec::new();

    for part in split_respecting_nesting(params_str, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Check for optional: type?
        let (param_type, optional) = if part.ends_with('?') {
            (part[..part.len() - 1].trim().to_string(), true)
        } else {
            (part.to_string(), false)
        };

        params.push(meta_ast::FunctionTypeParam {
            name: None, // No named params in this syntax
            param_type,
            optional,
        });
    }

    params
}

/// Convert CST %if directive to PrimitiveIfBlock
fn convert_cst_primitive_if(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<meta_ast::PrimitiveIfBlock> {
    use crate::syntax::cst::AstNode;
    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    // Parse condition from inline args
    let condition = parse_meta_if_condition(directive)?;

    // Parse then body
    let then_body = directive
        .body()
        .map(|b| convert_cst_primitive_body(&b, source))
        .unwrap_or_default();

    // Look for %else
    let else_body = find_else_body(directive, source);

    Some(meta_ast::PrimitiveIfBlock {
        condition,
        then_body,
        else_body,
        span,
    })
}

/// Parse meta if condition from directive
fn parse_meta_if_condition(
    directive: &crate::syntax::cst::Directive,
) -> Option<meta_ast::MetaIfCondition> {
    use crate::syntax::cst::{AstNode, SyntaxKind, VariableRef};

    // For %if directives, the condition is in the children between the name and body
    // Structure: % if VARIABLE_REF EQ_EQ STRING BODY

    // First, try to find a VariableRef child
    let var_ref = directive.syntax().children().find_map(VariableRef::cast);

    // Then look at tokens for the operator and value
    let name = directive.name()?;
    let name_end = name.text_range().end();

    // Collect all tokens after the name but before the body
    let tokens: Vec<_> = directive
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .filter(|t| t.text_range().start() >= name_end)
        .take_while(|t| t.kind() != SyntaxKind::L_BRACE)
        .collect();

    // Also collect nodes (like VARIABLE_REF) that contain our variable
    let var_name = var_ref.and_then(|v| v.name_text());

    // Build args_text from tokens
    let args_text: String = if let Some(ref vn) = var_name {
        // Prepend the variable name
        let rest: String = tokens
            .iter()
            .map(|t| t.text().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        format!("${} {}", vn, rest)
    } else {
        tokens
            .iter()
            .map(|t| t.text().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };

    let args_text = args_text.trim();

    // Handle negation: !$var
    if args_text.starts_with('!') {
        let rest = args_text[1..].trim();
        if rest.starts_with('$') {
            let var_name = rest[1..].trim().to_string();
            return Some(meta_ast::MetaIfCondition::Falsy(var_name));
        }
    }

    // Handle comparison operators
    for (op, constructor) in [
        (
            "==",
            meta_ast::MetaIfCondition::Equals as fn(String, String) -> meta_ast::MetaIfCondition,
        ),
        (
            "!=",
            meta_ast::MetaIfCondition::NotEquals as fn(String, String) -> meta_ast::MetaIfCondition,
        ),
        (
            "<=",
            meta_ast::MetaIfCondition::LessThanOrEqual
                as fn(String, String) -> meta_ast::MetaIfCondition,
        ),
        (
            ">=",
            meta_ast::MetaIfCondition::GreaterThanOrEqual
                as fn(String, String) -> meta_ast::MetaIfCondition,
        ),
        (
            "<",
            meta_ast::MetaIfCondition::LessThan as fn(String, String) -> meta_ast::MetaIfCondition,
        ),
        (
            ">",
            meta_ast::MetaIfCondition::GreaterThan
                as fn(String, String) -> meta_ast::MetaIfCondition,
        ),
    ] {
        if let Some(pos) = args_text.find(op) {
            let left = args_text[..pos].trim();
            let right = args_text[pos + op.len()..].trim();

            // Extract variable name (remove $)
            let var_name = if left.starts_with('$') {
                left[1..].to_string()
            } else {
                left.to_string()
            };

            // Extract value (remove quotes if present)
            let value = right.trim_matches('"').trim_matches('\'').to_string();

            return Some(constructor(var_name, value));
        }
    }

    // Simple truthy check: $var
    if args_text.starts_with('$') {
        let var_name = args_text[1..].trim().to_string();
        return Some(meta_ast::MetaIfCondition::Truthy(var_name));
    }

    None
}

/// Find and convert %else body
fn find_else_body(
    if_directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<meta_ast::PrimitiveBody> {
    use crate::syntax::cst::AstNode;
    // Look for a sibling %else directive after this %if
    let parent = if_directive.syntax().parent()?;
    let mut found_if = false;

    for child in parent.children() {
        if child == *if_directive.syntax() {
            found_if = true;
            continue;
        }
        if found_if
            && let Some(dir) = crate::syntax::cst::Directive::cast(child)
            && dir.name_text().as_deref() == Some("else")
        {
            return dir.body().map(|b| convert_cst_primitive_body(&b, source));
        }
    }

    None
}

/// Convert directive to FormClause (for %form)
fn convert_cst_directive_to_form_clause(
    directive: &crate::syntax::cst::Directive,
) -> Option<meta_ast::FormClause> {
    use crate::syntax::cst::AstNode;

    // Get the body and extract its text content
    let body = directive.body()?;
    let body_text = extract_body_content(&body);
    let body_text = body_text.trim();

    // Parse the form pattern text
    parse_form_pattern(body_text, body.syntax().text_range())
}

/// Parse a form pattern string into a FormClause
/// Example: "@toggle(initial: $initial:ident = off)"
/// Example: "$name:ident $type:typeref : $value:expr ;"
fn parse_form_pattern(pattern: &str, range: rowan::TextRange) -> Option<meta_ast::FormClause> {
    let pattern = pattern.trim();

    // Must start with a prefix character (@ for directives, $ for variables, & for element refs)
    let first_char = pattern.chars().next()?;
    if !matches!(first_char, '@' | '$' | '&') {
        return None;
    }

    // Find directive name
    // For @ directives: name includes prefix + identifier (e.g., "@on" from "@on $event:ident")
    // For $ and & directives: name is just the prefix character, because the text
    //   after the prefix is a capture variable, not a literal name
    //   (e.g., "$name:ident" → prefix "$", capture "name:ident")
    let name_end = if first_char == '$' || first_char == '&' {
        1 // directive_name is just the prefix character
    } else {
        pattern[1..]
            .find(|c: char| c == '(' || c == '{' || c == ':' || c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(pattern.len())
    };

    let directive_name = pattern[0..name_end].trim().to_string();

    // For $ and & prefix patterns, the text after the prefix may need a $ marker.
    // - $ prefix: the $ was consumed, so remaining text always needs $ re-added.
    // - & prefix: the & was consumed. If the form is `&name:type` (no $), we need
    //   to add $. But if `&$name:type` (already has $), we must NOT double it.
    let prefix_needs_dollar = first_char == '$' || first_char == '&';

    // Parse inline elements (between name and parentheses)
    let mut inline_elements = Vec::new();
    let mut params = Vec::new();
    let mut post_arg_inline = Vec::new();
    let mut body_capture = None;

    // Check for parentheses — skip union type parens like $timing:("a" | "b")
    // Union type parens are preceded by ':' (capture type annotation)
    let paren_start_opt = {
        let mut search_from = name_end;
        let mut found = None;
        while let Some(offset) = pattern[search_from..].find('(') {
            let abs_pos = search_from + offset;
            // A `(` that appears AFTER the body-opening `{` is inside the body region
            // (e.g. a FEAT-103 body group `( "shortcut" ":" $s:string )?`), NOT a
            // parenthesized params list. Stop searching — this form has no params paren.
            if let Some(brace_pos) = pattern[name_end..abs_pos].find('{') {
                let _ = brace_pos;
                break;
            }
            // A `( … )?` / `( … )*` / `( … )+` is an inline GROUP (W3 / PLAN-137:
            // an optional trailing clause like `( timeout : $t:time = 5000 )?`),
            // NOT a parenthesized params list. Its closing `)` is immediately
            // followed by the repetition modifier, so skip it — the form has no
            // params paren of its own.
            if let Some(close) = find_matching_delimiter(&pattern[abs_pos..], '(', ')') {
                let after_close = pattern[abs_pos..].as_bytes().get(close + 1);
                if matches!(after_close, Some(b'?') | Some(b'*') | Some(b'+')) {
                    break;
                }
            }
            // Check if this ( is part of a TYPE annotation, not a params list:
            //   - union type:        `$t:("a" | "b")`  — `(` directly preceded by `:`
            //   - parameterized type: `$v:balanced(';')` — `(` preceded by `:<ident>`
            // Both belong to a capture's type, so the `(` is NOT a parenthesized params
            // opener and must be skipped past its matching `)`. (PLAN-023 W3: balanced in forms.)
            let before = pattern[..abs_pos].trim_end();
            // Union type `$t:("a"|"b")` — `(` directly after `:`. Parameterized terminal
            // `$v:balanced(';')` — `(` directly after the `balanced` type keyword. Only these
            // two shapes are type-paren; a generic `:ident(` is a params list (e.g.
            // `@fn $name:ident($params:params)`), so we must NOT broaden to all `:ident(`.
            // `:balanced(` (possibly spaced: `$v : balanced(';')`) is a parameterized
            // type annotation, not a params list. Strip a trailing `balanced` keyword
            // then require a `:` before it (whitespace-tolerant).
            let is_type_paren = before.ends_with(':') || {
                let s = before.strip_suffix("balanced").map(|p| p.trim_end());
                matches!(s, Some(p) if p.ends_with(':'))
            };
            if is_type_paren {
                // This is a type-annotation paren — skip past matching )
                if let Some(close) = find_matching_delimiter(&pattern[abs_pos..], '(', ')') {
                    search_from = abs_pos + close + 1;
                    continue;
                }
                break; // Unmatched paren — give up
            }
            found = Some(abs_pos);
            break;
        }
        found
    };
    if let Some(paren_start) = paren_start_opt {
        // Parse inline elements before (
        let raw_inline = pattern[name_end..paren_start].trim();
        let inline_text =
            if prefix_needs_dollar && !raw_inline.is_empty() && !raw_inline.starts_with('$') {
                format!("${}", raw_inline)
            } else {
                raw_inline.to_string()
            };
        inline_elements.extend(parse_form_inline_elements(&inline_text));

        // Find matching closing paren
        if let Some(paren_end) = find_matching_delimiter(&pattern[paren_start..], '(', ')') {
            let params_str = &pattern[paren_start + 1..paren_start + paren_end];
            params = parse_form_params(params_str);

            // Check for body after params. The run between `)` and `{` (e.g. the
            // `: $returnType:typeref` of `@fn $n($p): $rt { … }`) is its own inline
            // segment — parse it into post_arg_inline so the matcher can capture it
            // (PLAN-025 / BUG-045). Without a body, the whole tail is post-arg inline.
            let after_params = &pattern[paren_start + paren_end + 1..];
            if let Some(brace_start) = after_params.find('{') {
                let between = after_params[..brace_start].trim();
                if !between.is_empty() {
                    post_arg_inline.extend(parse_form_inline_elements(between));
                }
                body_capture = Some(after_params[brace_start..].trim().to_string());
            } else {
                let tail = after_params.trim();
                if !tail.is_empty() {
                    post_arg_inline.extend(parse_form_inline_elements(tail));
                }
            }
        }
    } else if let Some(brace_start) = pattern.find('{') {
        // No params, but has body
        let raw_inline = pattern[name_end..brace_start].trim();
        let inline_text = if prefix_needs_dollar && !raw_inline.is_empty() {
            format!("${}", raw_inline)
        } else {
            raw_inline.to_string()
        };
        inline_elements.extend(parse_form_inline_elements(&inline_text));
        body_capture = Some(pattern[brace_start..].trim().to_string());
    } else {
        // Just inline elements, no params or body
        let raw_inline = pattern[name_end..].trim();
        let inline_text =
            if prefix_needs_dollar && !raw_inline.is_empty() && !raw_inline.starts_with('$') {
                format!("${}", raw_inline)
            } else {
                raw_inline.to_string()
            };
        inline_elements.extend(parse_form_inline_elements(&inline_text));
    }

    // Extract leading parenthesized body GROUPS (FEAT-103) before the rest of the
    // body capture. A `( ... )modifier` run at the head of the body region parses
    // via the SAME PEG as %capture_type (CapturePatternParser), and the remaining
    // tail stays as `body_capture` (e.g. `$body:component_body`).
    let body_groups = extract_body_groups(&mut body_capture);

    let mut form = meta_ast::FormClause {
        directive_name,
        inline_elements,
        params,
        post_arg_inline,
        body_capture,
        body_params: Vec::new(),
        body_groups,
        span: SourceSpan::new(range.start().into(), range.end().into()),
    };

    refine_body_capture(&mut form);

    Some(form)
}

/// Find the byte offset of a matching closing delimiter, handling nesting.
/// `s` must start with the opening delimiter.
fn find_matching_delimiter(s: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Zero-copy parser for pseudo-selector patterns in body captures.
///
/// Operates entirely on byte offsets into the input `&str`, avoiding the
/// `Vec<char>` + char/byte conversion dance that previously caused regressions.
///
/// Handles two patterns found in stdlib forms:
/// - Bare: `:name { $var:type? }` (used by @value-change)
/// - Parenthesized: `(:name { $var:type })?` (used by @each)
struct PseudoParser<'a> {
    input: &'a str,
    pos: usize, // byte offset — always on a char boundary
}

impl<'a> PseudoParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// True when the current position is a pseudo-selector boundary for `:`.
    /// A `:` is a pseudo boundary at start-of-input or after whitespace
    /// (not inside `$var:type`).
    fn at_pseudo_boundary(&self) -> bool {
        self.pos == 0 || self.input[..self.pos].ends_with(char::is_whitespace)
    }

    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    fn advance_one(&mut self) {
        self.pos += self.peek().map_or(0, |c| c.len_utf8());
    }

    /// Try to parse a parenthesized pseudo-selector: `(:name { $var:type })modifier?`
    fn try_paren_pseudo(&mut self) -> Option<meta_ast::FormParam> {
        let rest = self.remaining();
        if !rest.starts_with('(') {
            return None;
        }

        // Peek inside — must contain `:name`
        let after_paren = rest[1..].trim_start();
        if !after_paren.starts_with(':') {
            return None;
        }

        let close_paren = find_matching_delimiter(rest, '(', ')')?;
        let inner = rest[1..close_paren].trim();

        let (pseudo_name, body_params) = Self::parse_pseudo_inner(inner)?;

        // Check for trailing modifier after closing paren
        let after = &rest[close_paren + 1..];
        let (modifier, extra) = if after.starts_with('?') {
            (crate::parser::meta_ast::CaptureModifier::Optional, 1)
        } else if after.starts_with('*') {
            (crate::parser::meta_ast::CaptureModifier::ZeroOrMore, 1)
        } else if after.starts_with('+') {
            (meta_ast::CaptureModifier::OneOrMore, 1)
        } else {
            (meta_ast::CaptureModifier::Required, 0)
        };

        self.advance(close_paren + 1 + extra);

        Some(meta_ast::FormParam {
            name: String::new(),
            elements: vec![meta_ast::FormInlineElement::PseudoSelector {
                name: pseudo_name,
                body_params,
                modifier,
            }],
            default: None,
        })
    }

    /// Try to parse a bare pseudo-selector: `:name { $var:type? }`
    fn try_bare_pseudo(&mut self) -> Option<meta_ast::FormParam> {
        let rest = self.remaining();
        if !rest.starts_with(':') {
            return None;
        }

        let (pseudo_name, body_params) = Self::parse_pseudo_inner(rest)?;

        // Find total bytes consumed: from ':' through closing '}'
        let brace_start = rest.find('{')?;
        let brace_end = find_matching_delimiter(&rest[brace_start..], '{', '}')? + brace_start;
        self.advance(brace_end + 1);

        Some(meta_ast::FormParam {
            name: String::new(),
            elements: vec![meta_ast::FormInlineElement::PseudoSelector {
                name: pseudo_name,
                body_params,
                modifier: crate::parser::meta_ast::CaptureModifier::Optional,
            }],
            default: None,
        })
    }

    /// Parse the inner content of a pseudo-selector pattern: `:name { $var:type }`
    /// Returns (name, body_params).
    fn parse_pseudo_inner(s: &str) -> Option<(String, Vec<meta_ast::FormParam>)> {
        let s = s.trim_start_matches(':');

        // Find the name (identifier chars before whitespace or '{')
        let name_end = s.find(|c: char| c.is_whitespace() || c == '{')?;
        let pseudo_name = s[..name_end].trim().to_string();

        if pseudo_name.is_empty() {
            return None;
        }

        // Validate: pseudo-selector names are simple identifiers (letters, digits, hyphens)
        // Reject capture type annotations like "template_invocation+" or "string?"
        if !pseudo_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return None;
        }

        // Find the brace block
        let after_name = &s[name_end..];
        let brace_start = after_name.find('{')?;
        let brace_content_start = brace_start + 1;
        let brace_end = find_matching_delimiter(&after_name[brace_start..], '{', '}')?;
        let inner_content = after_name[brace_content_start..brace_start + brace_end].trim();

        // Parse the inner content as captures
        let mut body_params = Vec::new();
        if !inner_content.is_empty() {
            for token in inner_content.split_whitespace() {
                if token.starts_with('$')
                    && let Some(capture) = parse_form_capture(token)
                {
                    body_params.push(meta_ast::FormParam {
                        name: String::new(),
                        elements: vec![meta_ast::FormInlineElement::Capture(capture, None)],
                        default: None,
                    });
                }
            }
        }

        Some((pseudo_name, body_params))
    }

    /// Extract pseudo-selector patterns from a body capture string.
    /// Returns (pseudo_params, remaining_non_pseudo_content).
    fn parse(input: &'a str) -> (Vec<meta_ast::FormParam>, Option<String>) {
        let mut parser = Self::new(input);
        let mut pseudo_params = Vec::new();
        let mut remaining = String::new();

        while parser.peek().is_some() {
            // Try parenthesized pseudo: (:name { ... })modifier?
            if parser.peek() == Some('(')
                && let Some(param) = parser.try_paren_pseudo()
            {
                pseudo_params.push(param);
                continue;
            }

            // Try bare pseudo: :name { ... }
            if parser.peek() == Some(':')
                && parser.at_pseudo_boundary()
                && let Some(param) = parser.try_bare_pseudo()
            {
                pseudo_params.push(param);
                continue;
            }

            remaining.push(parser.peek().unwrap());
            parser.advance_one();
        }

        let remaining = remaining.trim();
        let remaining = if remaining.is_empty() {
            None
        } else {
            Some(format!("{{ {} }}", remaining))
        };

        (pseudo_params, remaining)
    }
}

/// Extract leading parenthesized GROUPS from the head of a form body (FEAT-103).
///
/// `body_capture` arrives as the raw body string including its outer `{ }`. A
/// `( ... )modifier` run at the head of the body — e.g. `( shortcut: $s:string ; )?`
/// — is a PEG group (parsed by the SAME `CapturePatternParser` that powers
/// `%capture_type`), to be matched against the body's token prefix BEFORE a greedy
/// body capture (`$body:component_body`). This peels each leading group, parses it,
/// and rewrites `*body_capture` to `{ <remaining tail> }` so the existing
/// body-capture path handles the rest unchanged.
///
/// A group is recognized ONLY at the head and ONLY when its inner is non-empty and
/// it is followed (after an optional `? * +` modifier) by more body content. A
/// `(:pseudo {...})` form is NOT a body group — it is left for `refine_body_capture`
/// (the leading `:` after `(` disqualifies it here). Returns the parsed groups in
/// source order; empty when the body has no leading group (the legacy path).
/// Find the closing `)` of a body group, quote- and char-class-aware (FEAT-104 #2).
///
/// `s` begins at the opening `(`. Parens inside `"..."` / `'...'` string literals and
/// `[...]` char classes are NOT delimiters (a grammar literal may legitimately be a
/// paren, e.g. `( "(" ")" )`). Returns the byte index of the matching `)` within `s`,
/// or `None` if unbalanced — in which case the caller declines to treat it as a group.
fn find_group_close(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'(') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_str: Option<u8> = None; // Some(quote) while inside a string literal
    let mut in_class = false; // inside [...]
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            // Inside a string literal: end on the matching unescaped quote.
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'\'' => in_str = Some(c),
            b'[' => in_class = true,
            b']' => in_class = false,
            b'(' if !in_class => depth += 1,
            b')' if !in_class => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Is a capture pattern LITERAL-LED — i.e. does its first matchable element anchor on a
/// literal token (FEAT-104 #4)? A Sequence is literal-led iff its first element is; a
/// Choice iff EVERY branch is (any non-literal branch is an unanchored alternative); a
/// bare Literal/CharClass is; a bare Capture or Group is not (recursing into the group).
fn pattern_is_literal_led(pattern: &meta_ast::CapturePatternAst) -> bool {
    use meta_ast::CapturePatternAst as P;
    use meta_ast::CaptureType;
    match pattern {
        P::Literal(_) | P::CharClass { .. } => true,
        // A bare builtin capture (`$x:expr`, `$x:balanced`, `$x:ident`, …) is
        // greedy/unanchored — it has no sentinel, so as an OPTIONAL leading body
        // group it would steal the first body token. NOT literal-led.
        //
        // EXCEPTION: a Custom (stdlib %capture_type) capture runs its OWN grammar,
        // which self-anchors — it matches only its specific production and fails
        // cleanly otherwise (consuming nothing), exactly like a literal sentinel.
        // So `( $policy:policy_clause )?` (policy_clause = `"policy" …`) is a safe
        // optional leading group even though the outer node is a Capture. This lets
        // a %form body sequence optional grammar-typed clauses (PLAN-038 W1
        // @data signal: `( $policy:policy_clause )? ( optimistic { … } )?`).
        P::Capture { capture_type, .. } => matches!(capture_type, CaptureType::Custom(_)),
        P::Sequence(elems) => elems.first().is_some_and(pattern_is_literal_led),
        P::Choice(branches) => !branches.is_empty() && branches.iter().all(pattern_is_literal_led),
        P::Group { pattern, .. } => pattern_is_literal_led(pattern),
    }
}

fn extract_body_groups(body_capture: &mut Option<String>) -> Vec<meta_ast::CapturePatternAst> {
    let Some(raw) = body_capture.as_ref() else {
        return Vec::new();
    };
    // Work on the brace-inner content; preserve the outer braces on rewrite.
    let trimmed = raw.trim();
    let had_braces = trimmed.starts_with('{') && trimmed.ends_with('}');
    let inner_full = if had_braces {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    };

    use crate::parser::meta_ast::{CaptureModifier, CapturePatternAst};
    let mut groups = Vec::new();
    let mut rest = inner_full.as_str();
    loop {
        let head = rest.trim_start();
        let leading_ws = rest.len() - head.len();
        if !head.starts_with('(') {
            break;
        }
        // A pseudo-selector `(:name { ... })` is NOT a body group; leave it for
        // refine_body_capture (PseudoParser).
        if head[1..].trim_start().starts_with(':') {
            break;
        }
        // QUOTE-AWARE close-paren scan (FEAT-104 #2): a group literal may itself
        // contain a paren (`( "(" ")" )?`), so raw `find_matching_delimiter` would
        // mis-bound. `find_group_close` ignores parens inside "..."/'...' string
        // literals and [...] char classes. An unresolved leading `(` (unbalanced or
        // literal-confused) is NOT a body group — leave it in body_capture so the
        // downstream body-capture path reports it, rather than silently swallowing.
        let Some(close) = find_group_close(head) else {
            break;
        };
        let group_inner = &head[1..close];
        if group_inner.trim().is_empty() {
            break;
        }
        // Trailing modifier (`?`/`*`/`+`), tolerating whitespace before it (FEAT-104
        // #3): the inner PEG skips whitespace before a modifier, so the group level
        // must too — otherwise `( ... ) ?` silently degrades optional→required and
        // leaks the stray `?`.
        let after_close = &head[close + 1..];
        let ws_before_mod = after_close.len() - after_close.trim_start().len();
        let after_mod_ws = after_close.trim_start();
        let (modifier, modifier_len) = match after_mod_ws.chars().next() {
            Some('?') => (Some(CaptureModifier::Optional), ws_before_mod + 1),
            Some('*') => (Some(CaptureModifier::ZeroOrMore), ws_before_mod + 1),
            Some('+') => (Some(CaptureModifier::OneOrMore), ws_before_mod + 1),
            _ => (None, 0),
        };
        // Parse the group inner as a capture pattern (the %capture_type PEG).
        let Some(pattern) = parse_capture_pattern(group_inner) else {
            break;
        };
        // LITERAL-LED discipline (FEAT-104 #4): an OPTIONAL / zero-or-more body group
        // placed ahead of a greedy body capture MUST be literal-led, else it has no
        // anchoring sentinel and greedily steals the first body token. Such a group
        // is NOT recognized here (left in body_capture to fail loudly downstream),
        // rather than silently stealing content. A REQUIRED group is self-anchoring
        // (its absence fails the form), so it is exempt.
        let is_optional_like = matches!(
            modifier,
            Some(CaptureModifier::Optional) | Some(CaptureModifier::ZeroOrMore)
        );
        if is_optional_like && !pattern_is_literal_led(&pattern) {
            break;
        }
        groups.push(CapturePatternAst::Group {
            pattern: Box::new(pattern),
            modifier,
        });
        // Advance past this group in `rest`.
        let consumed = leading_ws + close + 1 + modifier_len;
        rest = &rest[consumed..];
    }

    if groups.is_empty() {
        return Vec::new();
    }

    // Rewrite body_capture to the remaining tail (re-wrapped in braces so the
    // body-capture path keeps its brace-trimming invariant).
    let tail = rest.trim();
    *body_capture = if tail.is_empty() {
        None
    } else if had_braces {
        Some(format!("{{ {tail} }}"))
    } else {
        Some(tail.to_string())
    };

    groups
}

/// Extract pseudo-selector patterns from body_capture into body_params.
/// After extraction, remaining non-pseudo-selector content stays in body_capture.
fn refine_body_capture(form: &mut meta_ast::FormClause) {
    let body_capture = match &form.body_capture {
        Some(bc) => bc.clone(),
        None => return,
    };

    let inner = body_capture
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();

    if inner.is_empty() {
        return;
    }

    let (pseudo_params, remaining) = PseudoParser::parse(inner);

    if pseudo_params.is_empty() {
        return;
    }

    form.body_params = pseudo_params;
    form.body_capture = remaining;
}

/// Split a string on whitespace, but keep parenthesized groups together.
/// e.g. `$timing:("on_enter" | "on_exit") $other:ident` → [`$timing:("on_enter" | "on_exit")`, `$other:ident`]
fn split_whitespace_respecting_parens(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut depth = 0u32;
    let mut in_string = false;
    let mut string_char = '"';
    let mut start = 0;
    let mut in_token = false;

    for (i, c) in text.char_indices() {
        match c {
            '"' | '\'' if depth > 0 && !in_string => {
                in_string = true;
                string_char = c;
            }
            c if in_string && c == string_char => {
                in_string = false;
            }
            '(' if !in_string => {
                if !in_token {
                    start = i;
                    in_token = true;
                }
                depth += 1;
            }
            ')' if !in_string && depth > 0 => {
                depth -= 1;
            }
            c if c.is_whitespace() && depth == 0 && !in_string => {
                if in_token {
                    tokens.push(&text[start..i]);
                    in_token = false;
                }
                continue;
            }
            _ => {
                if !in_token {
                    start = i;
                    in_token = true;
                }
            }
        }
    }
    if in_token {
        tokens.push(&text[start..]);
    }
    tokens
}

/// Parse form inline elements (captures, literals, etc.)
fn parse_form_inline_elements(text: &str) -> Vec<meta_ast::FormInlineElement> {
    parse_form_inline_elements_raw(text)
}

fn parse_form_inline_elements_raw(text: &str) -> Vec<meta_ast::FormInlineElement> {
    let mut elements = Vec::new();
    let text = text.trim();
    if text.is_empty() {
        return elements;
    }

    // Tokenize on whitespace, respecting paren nesting for union types like $timing:("a" | "b")
    let tokens: Vec<&str> = split_whitespace_respecting_parens(text);
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        // An inline GROUP: `( ... )?` / `( ... )*` / `( ... )+` / `( ... )`.
        // `split_whitespace_respecting_parens` keeps the whole `( a b )?` run as
        // one token, so peel the parens, parse the inner elements recursively,
        // and read the repetition modifier off the closing `)` (W3 / PLAN-137:
        // a genuinely-optional trailing clause is grammar DATA, not a Rust
        // special case).
        // Strip a trailing repetition suffix (`.?`, `.*`, `.+`) off the closing
        // paren before testing the balanced-paren shape, so `( a b )?` is seen
        // as a group with modifier Optional.
        let (group_mod, inner_no_paren) = {
            let stripped = token
                .strip_suffix("?")
                .or_else(|| token.strip_suffix("*"))
                .or_else(|| token.strip_suffix("+"));
            match stripped {
                Some(s) if s.ends_with(')') => {
                    let m = if token.ends_with('?') {
                        meta_ast::CaptureModifier::Optional
                    } else if token.ends_with('*') {
                        meta_ast::CaptureModifier::ZeroOrMore
                    } else {
                        meta_ast::CaptureModifier::OneOrMore
                    };
                    (m, Some(&s[..s.len() - 1]))
                }
                Some(_) => (meta_ast::CaptureModifier::Required, None),
                None => (meta_ast::CaptureModifier::Required, None),
            }
        };
        if token.starts_with('(') && inner_no_paren.is_some() {
            let inner = inner_no_paren.expect("guarded by inner_no_paren.is_some()");
            let inner = &inner[1..]; // strip the opening `(`
            let modifier = group_mod;
            let inner = inner.trim();
            // `_raw`, not the wrapper: optionality is propagated ONCE from the
            // top of the tree, so recursing through the wrapper would re-walk
            // every nested group for each level of nesting.
            let group_elements = if inner.is_empty() {
                Vec::new()
            } else {
                parse_form_inline_elements_raw(inner)
            };
            elements.push(meta_ast::FormInlineElement::Group {
                elements: group_elements,
                modifier,
            });
            i += 1;
            continue;
        } else if token.starts_with("$$") && token.len() > 2 {
            // Literal $ prefix + capture: $$name:type (e.g., $$name:ident in @computed)
            elements.push(meta_ast::FormInlineElement::Literal("$".to_string()));
            if let Some(cap) = parse_form_capture(&token[1..]) {
                let default = if i + 2 < tokens.len() && tokens[i + 1] == "=" {
                    let default_val = parse_param_default(tokens[i + 2]);
                    i += 2;
                    Some(default_val)
                } else {
                    None
                };
                elements.push(meta_ast::FormInlineElement::Capture(cap, default));
            }
        } else if token.starts_with('$') {
            // Capture: $name:type, possibly followed by `= default`
            if let Some(cap) = parse_form_capture(token) {
                // Check for `= default_value` after this capture
                let default = if i + 2 < tokens.len() && tokens[i + 1] == "=" {
                    let default_val = parse_param_default(tokens[i + 2]);
                    i += 2; // skip `=` and value
                    Some(default_val)
                } else {
                    None
                };
                elements.push(meta_ast::FormInlineElement::Capture(cap, default));
            }
        } else if token.starts_with("&$") {
            // Element capture: &$name:type (e.g., &$name:ident in @template)
            elements.push(meta_ast::FormInlineElement::Literal("&".to_string()));
            if let Some(cap) = parse_form_capture(&token[1..]) {
                elements.push(meta_ast::FormInlineElement::Capture(cap, None));
            }
        } else if let Some(rest) = token.strip_prefix("&self.").filter(|r| r.starts_with('$')) {
            // Self-signal reference + capture: `&self.$sig:expr` (BUG-088 @when guard).
            // The `&self.` prefix is a literal that reads a signal off THIS element;
            // the trailing `$sig:expr` captures the (possibly dotted) signal path.
            elements.push(meta_ast::FormInlineElement::Literal("&self.".to_string()));
            if let Some(cap) = parse_form_capture(rest) {
                let default = if i + 2 < tokens.len() && tokens[i + 1] == "=" {
                    let default_val = parse_param_default(tokens[i + 2]);
                    i += 2;
                    Some(default_val)
                } else {
                    None
                };
                elements.push(meta_ast::FormInlineElement::Capture(cap, default));
            }
        } else {
            // Literal token
            elements.push(meta_ast::FormInlineElement::Literal(token.to_string()));
        }
        i += 1;
    }

    elements
}

/// Parse form parameters: "name: $var:type = default, name2: $var2:type2"
fn parse_form_params(params_str: &str) -> Vec<meta_ast::FormParam> {
    let mut params = Vec::new();

    // Split by comma, respecting nesting
    for param_str in split_respecting_nesting(params_str, ',') {
        let param_str = param_str.trim();
        if param_str.is_empty() {
            continue;
        }

        // Find the parameter name (before first $)
        if let Some(dollar_pos) = param_str.find('$') {
            // Check for "name:" before the $
            let before_dollar = &param_str[..dollar_pos];
            let name = if let Some(colon_pos) = before_dollar.find(':') {
                before_dollar[..colon_pos].trim().to_string()
            } else {
                // Positional parameter - use capture name
                String::new()
            };

            // Parse the capture portion
            let capture_str = &param_str[dollar_pos..];

            // Handle "as" clause: $source:binding as $item:ident
            let (primary_str, alias_capture) = if let Some(as_pos) = capture_str.find(" as ") {
                let primary = capture_str[..as_pos].trim();
                let alias_str = capture_str[as_pos + 4..].trim(); // skip " as "
                let alias = parse_form_capture(alias_str);
                (primary, alias)
            } else {
                (capture_str.trim(), None)
            };

            // Check for default value: $var:type = default
            let (capture_part, default_value) = if let Some(eq_pos) = primary_str.find('=') {
                let cap_part = primary_str[..eq_pos].trim();
                let default_part = primary_str[eq_pos + 1..].trim();
                (cap_part, Some(parse_param_default(default_part)))
            } else {
                (primary_str, None)
            };

            if let Some(mut capture) = parse_form_capture(capture_part) {
                // Attach the alias capture if present
                capture.alias_capture = alias_capture.map(Box::new);
                // Build elements list, prepending literal tokens from before_dollar
                // e.g. "when $filter:expr" -> [Literal("when"), Capture($filter:expr)]
                let mut elements: Vec<meta_ast::FormInlineElement> = Vec::new();
                if name.is_empty() {
                    let literal_prefix = before_dollar.trim();
                    if !literal_prefix.is_empty() {
                        for word in literal_prefix.split_whitespace() {
                            elements.push(meta_ast::FormInlineElement::Literal(word.to_string()));
                        }
                    }
                }
                elements.push(meta_ast::FormInlineElement::Capture(capture, None));
                // Keep name empty for positional params - the var_name is in the capture
                // and bind_args uses positional matching when param.name is empty
                params.push(meta_ast::FormParam {
                    name,
                    elements,
                    default: default_value,
                });
            }
        }
    }

    params
}

/// Parse a form capture: $name:type or $name:type* or $name:type?
fn parse_form_capture(text: &str) -> Option<meta_ast::FormCapture> {
    let text = text.trim();
    if !text.starts_with('$') {
        return None;
    }

    let text = &text[1..]; // Remove $

    // Check for modifier at the end
    let (text, modifier) = if text.ends_with('*') {
        (
            &text[..text.len() - 1],
            crate::parser::meta_ast::CaptureModifier::ZeroOrMore,
        )
    } else if text.ends_with('?') {
        (
            &text[..text.len() - 1],
            crate::parser::meta_ast::CaptureModifier::Optional,
        )
    } else if text.ends_with('+') {
        (
            &text[..text.len() - 1],
            meta_ast::CaptureModifier::OneOrMore,
        )
    } else {
        (text, meta_ast::CaptureModifier::Required)
    };

    // Split on : to get name and type
    let (var_name, capture_type) = if let Some(colon_pos) = text.find(':') {
        let name = text[..colon_pos].trim().to_string();
        let type_str = text[colon_pos + 1..].trim();
        (name, parse_capture_type(type_str))
    } else {
        // No type specified, default to expr
        (text.to_string(), meta_ast::CaptureType::Expr)
    };

    Some(meta_ast::FormCapture {
        var_name,
        capture_type,
        modifier,
        alias_capture: None,
    })
}

/// Resolve a capture-type NAME to its `CaptureType`, for callers outside the
/// parser.
///
/// `Custom(name)` is the answer for anything not builtin — either a stdlib
/// `%capture_type` or a name nobody has defined. The caller tells those apart
/// by looking the name up in the registry (PLAN-136 W1).
pub fn parse_capture_type_public(type_str: &str) -> meta_ast::CaptureType {
    parse_capture_type(type_str)
}

/// Parse capture type string to CaptureType enum
fn parse_capture_type(type_str: &str) -> meta_ast::CaptureType {
    let type_str = type_str.trim();
    // Parameterized terminal `balanced(';')` / `balanced(",")` — also valid in %form
    // inline captures (not just %capture_type bodies). Extract the single quoted delimiter
    // char; default to ';' on a malformed arg. (PLAN-023 W3: @data derive/fold value capture.)
    if let Some(rest) = type_str.strip_prefix("balanced") {
        let rest = rest.trim();
        if let Some(inner) = rest.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            let delim = inner
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .chars()
                .next();
            return meta_ast::CaptureType::Balanced(delim.unwrap_or(';'));
        }
    }
    // Check for union type: ("a" | "b" | "c")
    if type_str.starts_with('(') && type_str.ends_with(')') {
        let inner = &type_str[1..type_str.len() - 1];
        let variants: Vec<String> = inner
            .split('|')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !variants.is_empty() {
            return meta_ast::CaptureType::Union(variants);
        }
    }
    match type_str {
        "ident" => meta_ast::CaptureType::Ident,
        "dashed_ident" => meta_ast::CaptureType::DashedIdent,
        "event_name" => meta_ast::CaptureType::EventName,
        "string" => meta_ast::CaptureType::String,
        "number" => meta_ast::CaptureType::Number,
        "bool" => meta_ast::CaptureType::Bool,
        // color/length/duration/time/easing migrated to stdlib (PLAN-122 W1.2):
        // fall through to Custom, resolving against the grammars in
        // stdlib/capture-types/css-values.st. Same route properties/params/
        // keyframes/param_list took in PLAN-023 W2 — the arm is removed so the
        // stdlib production is reachable at all. A hex colour's legal lengths
        // are now DATA (`[0-9a-fA-F]{3|4|6|8}`) rather than a hand-written
        // extractor, which is what makes `#e8ee1` a diagnostic (BUG-257).
        "typeref" => meta_ast::CaptureType::Typeref,
        "binding" => meta_ast::CaptureType::Binding,
        "event" => meta_ast::CaptureType::Event,
        "expr" => meta_ast::CaptureType::Expr,
        // properties/fields migrated to stdlib (PLAN-023 W2): fall through to Custom.
        // (kept out of the hardcoded list so they resolve to the stdlib grammar + reifier)
        // params migrated to stdlib (PLAN-023 W2): falls through to Custom + reify_params.
        "states" => meta_ast::CaptureType::States,
        "transitions" => meta_ast::CaptureType::Transitions,
        // keyframes migrated to stdlib (PLAN-023 W2): Custom + reify_keyframes.
        "selector" => meta_ast::CaptureType::Selector,
        "element" => meta_ast::CaptureType::Element,
        "preset" => meta_ast::CaptureType::Preset,
        // "color" migrated to stdlib with the other scalars above (PLAN-122 W1.2).
        "mutation_actions" => meta_ast::CaptureType::MutationActions,
        "template" => meta_ast::CaptureType::Template,
        "skip_block" => meta_ast::CaptureType::SkipBlock,
        // param_list lives in stdlib now (PLAN-023 W2): falls through to Custom below.
        "html_block" => meta_ast::CaptureType::HtmlBlock,
        "component_body" => meta_ast::CaptureType::ComponentBody,
        // `template_invocation` (`&name(args)`) is a builtin leaf used by match_arm arms,
        // each.st bodies, and cb_nested_each — map it to its variant so the %capture_type engine
        // dispatches TemplateInvocationExtractor (Named{name,args}), not Custom -> Expr (BUG-066).
        "template_invocation" => meta_ast::CaptureType::TemplateInvocation,
        // A clean identifier names a Custom (stdlib %capture_type) production (PLAN-023 W2,
        // e.g. param_list, optmark). Anything else (empty, spaces, punctuation — which can
        // arise from lenient form-capture splitting like `number ..`) keeps the historical
        // Expr default so existing forms are unaffected.
        other if is_capture_type_ident(other) => meta_ast::CaptureType::Custom(other.to_string()),
        _ => meta_ast::CaptureType::Expr,
    }
}

/// True if `s` is a clean capture-type identifier (letters, digits, `_`, `-`; non-empty;
/// starts with a letter). Guards the Custom fallthrough against malformed type strings.
fn is_capture_type_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Parse parameter default value
fn parse_param_default(text: &str) -> meta_ast::ParamDefault {
    let text = text.trim();

    // String literal
    if (text.starts_with('"') && text.ends_with('"'))
        || (text.starts_with('\'') && text.ends_with('\''))
    {
        return meta_ast::ParamDefault::String(text[1..text.len() - 1].to_string());
    }

    // Number
    if let Ok(num) = text.parse::<f64>() {
        return meta_ast::ParamDefault::Number(num);
    }

    // Bool
    if text == "true" {
        return meta_ast::ParamDefault::Bool(true);
    }
    if text == "false" {
        return meta_ast::ParamDefault::Bool(false);
    }

    // None/null/undefined
    if text == "none" || text == "null" || text == "undefined" {
        return meta_ast::ParamDefault::None;
    }

    // Default to identifier stored as string
    meta_ast::ParamDefault::String(text.to_string())
}

/// Split string on delimiter, respecting nested parens/brackets/braces and quoted strings
fn split_respecting_nesting(s: &str, delim: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth: usize = 0;
    let mut start = 0;
    let mut in_quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => in_quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            c if c == delim && depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(&s[start..]);
    result
}

/// Parse bind arguments
fn parse_bind_args(args_text: &str) -> Vec<meta_ast::BindArg> {
    let mut args = Vec::new();

    // Split by comma, respecting nested parens (e.g., lerp($a, $b, 0.5) stays together)
    for arg in split_respecting_nesting(args_text, ',') {
        let arg = arg.trim();
        if arg.is_empty() {
            continue;
        }

        // &name - element reference
        if arg.starts_with('&') {
            args.push(meta_ast::BindArg::Element {
                name: arg[1..].trim().to_string(),
                child_selector: None,
            });
        } else if (arg.starts_with('"') && arg.ends_with('"'))
            || (arg.starts_with('\'') && arg.ends_with('\''))
        {
            // Quoted string - treat as positional (may contain colons)
            args.push(meta_ast::BindArg::Positional(parse_bind_value(arg)));
        } else if let Some(colon_pos) = arg.find(':') {
            // name: value - named argument
            let name = arg[..colon_pos].trim().to_string();
            let value_str = arg[colon_pos + 1..].trim();
            let value = parse_bind_value(value_str);
            args.push(meta_ast::BindArg::Named { name, value });
        } else if arg.starts_with('$') {
            // $var - positional variable
            args.push(meta_ast::BindArg::Positional(
                meta_ast::BindValue::Variable(arg[1..].to_string()),
            ));
        } else {
            // Other positional value
            args.push(meta_ast::BindArg::Positional(parse_bind_value(arg)));
        }
    }

    args
}

/// Parse bind value
fn parse_bind_value(value_str: &str) -> meta_ast::BindValue {
    let value_str = value_str.trim();

    // Variable reference
    if value_str.starts_with('$') {
        return meta_ast::BindValue::Variable(value_str[1..].to_string());
    }

    // String literal
    if (value_str.starts_with('"') && value_str.ends_with('"'))
        || (value_str.starts_with('\'') && value_str.ends_with('\''))
    {
        return meta_ast::BindValue::String(value_str[1..value_str.len() - 1].to_string());
    }

    // Number
    if let Ok(num) = value_str.parse::<f64>() {
        return meta_ast::BindValue::Number(num);
    }

    // Boolean - store as Ident (no Bool variant)
    if value_str == "true" || value_str == "false" {
        return meta_ast::BindValue::Ident(value_str.to_string());
    }

    // Function call: name(args)
    if let Some(paren_pos) = value_str.find('(')
        && value_str.ends_with(')')
    {
        let fn_name = value_str[..paren_pos].to_string();
        let fn_args_str = &value_str[paren_pos + 1..value_str.len() - 1];
        let fn_args: Vec<_> = split_respecting_nesting(fn_args_str, ',')
            .iter()
            .map(|s| parse_bind_value(s.trim()))
            .collect();
        return meta_ast::BindValue::FunctionCall {
            name: fn_name,
            args: fn_args,
        };
    }

    // Default to identifier
    meta_ast::BindValue::Ident(value_str.to_string())
}

/// Parse bind outputs
/// Split a `%binds` output list into entries.
///
/// Entries are comma- or newline-separated, but the bind-line accumulator joins
/// physical lines with a space before this point, so a one-per-line block looks
/// like `$a as $x $b as $y`. A bare space cannot be the separator (` as ` is
/// itself space-delimited), so after the comma/newline pass each run is split
/// again at every `$` that STARTS a new entry — i.e. a `$` at a token boundary
/// that is not the one immediately following an `as`.
fn split_outputs(outputs_str: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in outputs_str.split([',', '\n']) {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        // Walk tokens, starting a new entry at a `$` that is not an alias target.
        let mut current = String::new();
        let mut prev_was_as = false;
        for tok in chunk.split_whitespace() {
            if tok.starts_with('$') && !prev_was_as && !current.trim().is_empty() {
                out.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(tok);
            current.push(' ');
            prev_was_as = tok == "as";
        }
        if !current.trim().is_empty() {
            out.push(current.trim().to_string());
        }
    }
    out
}

fn parse_bind_outputs(outputs_str: &str) -> Vec<meta_ast::BindOutput> {
    let mut outputs = Vec::new();

    // Remove { } if present
    let outputs_str = outputs_str
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();

    // Split on comma OR newline. A `%binds` output list is written both ways —
    // `{ $a, $b }` on one line and one-per-line in a multi-line block — and the
    // line accumulator upstream joins physical lines with a SPACE, so by the
    // time the text arrives here a one-per-line block is a single run. Splitting
    // on the comma alone therefore parsed the whole block as ONE output whose
    // ALIAS was the rest of the text (`insWriteError $status as $insWriteStatus`),
    // which fails the plain-ident gate in resolve.rs: no remap was recorded, so
    // `%yield` wrote the RAW export name and every export after the first stayed
    // permanently dead. `split_outputs` below restores the boundary the
    // accumulator erased; newline is kept as a separator for any caller that
    // preserves it.
    for output in split_outputs(outputs_str) {
        let output = output.trim();
        if output.is_empty() {
            continue;
        }

        // Check for alias: $name as alias
        if let Some(as_pos) = output.find(" as ") {
            let name_part = output[..as_pos].trim();
            let alias = output[as_pos + 4..].trim().to_string();
            let name = if name_part.starts_with('$') {
                name_part[1..].to_string()
            } else {
                name_part.to_string()
            };
            outputs.push(meta_ast::BindOutput {
                name,
                alias: Some(alias),
            });
        } else if output.starts_with('$') {
            outputs.push(meta_ast::BindOutput {
                name: output[1..].to_string(),
                alias: None,
            });
        }
    }

    outputs
}

/// Parse resolves mappings from body text
/// Format: $symbol -> registry
fn parse_resolves_mappings(body_text: &str) -> Vec<meta_ast::ResolveMapping> {
    body_text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if let Some(arrow_idx) = line.find("->") {
                let symbol = line[..arrow_idx].trim().to_string();
                let registry = line[arrow_idx + 2..].trim().to_string();
                if !symbol.is_empty() && !registry.is_empty() {
                    return Some(meta_ast::ResolveMapping { symbol, registry });
                }
            }
            None
        })
        .collect()
}

/// Convert directive to MacroBodyItem
fn convert_cst_directive_to_macro_body_item(
    directive: &crate::syntax::cst::Directive,
    source: &str,
) -> Option<meta_ast::MacroBodyItem> {
    use crate::syntax::cst::AstNode;

    let name = directive.name_text()?;
    let text_range = directive.syntax().text_range();
    let span = SourceSpan::new(text_range.start().into(), text_range.end().into());

    match name.as_str() {
        "if" => {
            // %if condition { body } %elif condition { body } %else { body }
            let condition = parse_meta_if_condition(directive)?;

            // Parse then body - recursively parse nested directives
            let then_body = directive
                .body()
                .map(|b| parse_macro_body_items_from_cst_body(&b, source))
                .unwrap_or_default();

            // Look for sibling %elif and %else directives
            let (elif_clauses, else_body) = find_macro_elif_else_clauses(directive, source);

            Some(meta_ast::MacroBodyItem::If(meta_ast::MetaIfClause {
                condition,
                then_body,
                elif_clauses,
                else_body,
                span,
            }))
        }
        "elif" | "else" => {
            // These are handled by the %if handler when it looks for siblings
            // Return None to skip them as standalone items
            None
        }
        "for" => {
            // %for $item in $list { body }
            let (variable, source_var) = parse_meta_for_header(directive)?;

            let body = directive
                .body()
                .map(|b| parse_macro_body_items_from_cst_body(&b, source))
                .unwrap_or_default();

            Some(meta_ast::MacroBodyItem::For(meta_ast::MetaForClause {
                variable,
                source: source_var,
                body,
                span,
            }))
        }
        "binds" => {
            // %binds { primitive(...) -> { outputs } }
            if let Some(binds_body) = directive.body() {
                let binds = parse_binds_from_body_text(&binds_body);
                if !binds.is_empty() {
                    return Some(meta_ast::MacroBodyItem::Binds(binds));
                }
            }
            None
        }
        "when" => {
            // %when condition { body }
            // Parse condition from inline args
            let condition = directive
                .inline_tokens_text()
                .unwrap_or_default()
                .trim()
                .to_string();

            let body = directive
                .body()
                .map(|b| parse_macro_body_items_from_cst_body(&b, source))
                .unwrap_or_default();

            Some(meta_ast::MacroBodyItem::When(meta_ast::WhenClause {
                condition,
                body,
                span,
            }))
        }
        "on" => {
            // %on trigger { body }
            let trigger = parse_meta_on_trigger(directive)?;
            let body = directive
                .body()
                .map(|b| parse_meta_on_body_items(&b, source))
                .unwrap_or_default();

            Some(meta_ast::MacroBodyItem::On(meta_ast::MetaOnClause {
                trigger,
                body,
                span,
            }))
        }
        "emit" => {
            // %emit js/css { code }
            convert_cst_emit_block(directive, source).map(meta_ast::MacroBodyItem::Emit)
        }
        "animates" => {
            // %animates { property: $variable }
            // Each property reactively binds to a (derived) signal so the element
            // follows the value live (e.g. @drag's translate-x: $x). PLAN-054.
            // The CST parses `translate-x: $x` lines as bare VARIABLE_REFs (losing
            // the property key), so parse from raw body text via the shared
            // `name: value` splitter.
            let mut properties = Vec::new();
            if let Some(body) = directive.body() {
                let inner = extract_body_content(&body);
                for (key, value) in parse_clause_decls(&inner) {
                    let variable = value.trim().trim_start_matches('$').to_string();
                    if !key.is_empty() && !variable.is_empty() {
                        properties.push(meta_ast::AnimateProp {
                            property: key,
                            variable,
                        });
                    }
                }
            }
            if properties.is_empty() {
                None
            } else {
                Some(meta_ast::MacroBodyItem::Animates(
                    meta_ast::AnimatesClause { properties, span },
                ))
            }
        }
        "trigger" => {
            // %trigger event_name
            let event = directive
                .inline_tokens_text()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !event.is_empty() {
                Some(meta_ast::MacroBodyItem::Trigger(event))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse macro body items from a CST body
fn parse_macro_body_items_from_cst_body(
    body: &crate::syntax::cst::Body,
    source: &str,
) -> Vec<meta_ast::MacroBodyItem> {
    let mut items = Vec::new();

    for directive in body.directives() {
        if let Some(item) = convert_cst_directive_to_macro_body_item(&directive, source) {
            items.push(item);
        }
    }

    items
}

/// Find %elif and %else clauses following a %if directive
fn find_macro_elif_else_clauses(
    if_directive: &crate::syntax::cst::Directive,
    source: &str,
) -> (
    Vec<meta_ast::MetaElifClause>,
    Option<Vec<meta_ast::MacroBodyItem>>,
) {
    use crate::syntax::cst::AstNode;

    let mut elif_clauses = Vec::new();
    let mut else_body = None;

    let parent = match if_directive.syntax().parent() {
        Some(p) => p,
        None => return (elif_clauses, else_body),
    };

    let mut found_if = false;

    for child in parent.children() {
        if child == *if_directive.syntax() {
            found_if = true;
            continue;
        }

        if found_if && let Some(dir) = crate::syntax::cst::Directive::cast(child) {
            match dir.name_text().as_deref() {
                Some("elif") => {
                    if let Some(condition) = parse_meta_if_condition(&dir) {
                        let text_range = dir.syntax().text_range();
                        let span =
                            SourceSpan::new(text_range.start().into(), text_range.end().into());

                        let body = dir
                            .body()
                            .map(|b| parse_macro_body_items_from_cst_body(&b, source))
                            .unwrap_or_default();

                        elif_clauses.push(meta_ast::MetaElifClause {
                            condition,
                            body,
                            span,
                        });
                    }
                }
                Some("else") => {
                    else_body = dir
                        .body()
                        .map(|b| parse_macro_body_items_from_cst_body(&b, source));
                    // Stop after finding %else
                    break;
                }
                Some("if") => {
                    // New %if starts a new conditional chain, stop here
                    break;
                }
                _ => {
                    // Any other directive breaks the chain
                    break;
                }
            }
        }
    }

    (elif_clauses, else_body)
}

/// Parse %for header to extract variable and source
fn parse_meta_for_header(directive: &crate::syntax::cst::Directive) -> Option<(String, String)> {
    // Format: %for $item in $list
    let tokens_text = directive.inline_tokens_text()?;
    let tokens_text = tokens_text.trim();

    // Find "in" keyword
    if let Some(in_pos) = tokens_text.find(" in ") {
        let variable = tokens_text[..in_pos].trim();
        let source_var = tokens_text[in_pos + 4..].trim();

        // Remove $ prefix if present
        let variable = variable.trim_start_matches('$').to_string();
        let source_var = source_var.trim_start_matches('$').to_string();

        if !variable.is_empty() && !source_var.is_empty() {
            return Some((variable, source_var));
        }
    }

    None
}

/// Parse multiple bind declarations from a %binds body
fn parse_binds_from_body_text(body: &crate::syntax::cst::Body) -> Vec<meta_ast::BindDecl> {
    let body_text = extract_body_content(body);
    let mut binds = Vec::new();

    // Split by lines that contain "->" to find individual bind declarations
    // Each bind is: primitive-name(args) -> { outputs }
    let mut current_bind = String::new();
    let mut brace_depth: usize = 0;
    let mut paren_depth: usize = 0;

    for line in body_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // Track brace and paren depth
        for c in line.chars() {
            match c {
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                '(' => paren_depth += 1,
                ')' => paren_depth = paren_depth.saturating_sub(1),
                _ => {}
            }
        }

        current_bind.push_str(line);
        current_bind.push(' ');

        // A bind is complete when all braces AND parens are balanced.
        // We must track paren depth because bind args can contain nested function calls
        // like `color: colorToArray($clearColor)` — we can't split on `)` alone.
        if brace_depth == 0 && paren_depth == 0 && current_bind.contains('(') {
            // Check if this is a complete bind: either has -> (with outputs) or
            // has a closing paren (output-less bind like `apply-animations(...)`)
            let trimmed = current_bind.trim();
            let has_outputs = trimmed.contains("->");
            let has_complete_call = trimmed.contains(')');

            if has_outputs || has_complete_call {
                if let Some(bind) = parse_single_bind_decl(&current_bind, body) {
                    binds.push(bind);
                }
                current_bind.clear();
            }
        }
    }

    // Handle case where last bind doesn't end with closing brace on new line
    if !current_bind.trim().is_empty()
        && let Some(bind) = parse_single_bind_decl(&current_bind, body)
    {
        binds.push(bind);
    }

    binds
}

/// Parse a single bind declaration from text
///
/// Supports two forms:
/// - With outputs: `primitive-name(args) -> { $output1, $output2 }`
/// - Without outputs: `primitive-name(args)`
fn parse_single_bind_decl(
    text: &str,
    body: &crate::syntax::cst::Body,
) -> Option<meta_ast::BindDecl> {
    use crate::syntax::cst::AstNode;

    let text = text.trim();

    // Determine if this bind has outputs (contains ->)
    let (call_part, outputs) = if let Some(arrow_pos) = text.find("->") {
        let call = text[..arrow_pos].trim();
        let outputs_part = text[arrow_pos + 2..].trim();
        (call, parse_bind_outputs(outputs_part))
    } else {
        // Output-less bind (e.g., apply-animations(...))
        (text, Vec::new())
    };

    // Parse call: name(args)
    let paren_pos = call_part.find('(')?;
    let primitive = call_part[..paren_pos].trim().to_string();
    let args_text = &call_part[paren_pos + 1..call_part.rfind(')')?];

    // Parse args
    let args = parse_bind_args(args_text);

    let text_range = body.syntax().text_range();
    Some(meta_ast::BindDecl {
        primitive,
        args,
        outputs,
        span: SourceSpan::new(text_range.start().into(), text_range.end().into()),
    })
}

/// Parse %on trigger from directive
fn parse_meta_on_trigger(
    directive: &crate::syntax::cst::Directive,
) -> Option<meta_ast::MetaOnTrigger> {
    // Use the descending tail walker: a `$var` trigger parses into a child node
    // (like a VAR), which the token-only `inline_tokens_text` skips — so
    // `$active -> false` would lose `$active`. `inline_tail_text` collects the
    // structured children too (PLAN-053).
    let tokens_text = directive
        .inline_tail_text()
        .or_else(|| directive.inline_tokens_text())?;
    let tokens_text = tokens_text.trim();

    // Check for $var -> value (transition)
    if let Some(arrow_pos) = tokens_text.find("->") {
        // The tail walker may join the sigil and name with a space (`$ active`),
        // so strip `$` then re-trim to get the bare signal name.
        let var = tokens_text[..arrow_pos]
            .trim()
            .trim_start_matches('$')
            .trim()
            .to_string();
        let value = tokens_text[arrow_pos + 2..].trim().to_string();
        return Some(meta_ast::MetaOnTrigger::VarTransition { var, value });
    }

    // Check for $var.event
    if tokens_text.starts_with('$')
        && let Some(dot_pos) = tokens_text.find('.')
    {
        let var = tokens_text[1..dot_pos].to_string();
        let event = tokens_text[dot_pos + 1..].to_string();
        return Some(meta_ast::MetaOnTrigger::VarEvent { var, event });
    }

    // Plain event name
    if !tokens_text.is_empty() {
        return Some(meta_ast::MetaOnTrigger::Event(tokens_text.to_string()));
    }

    None
}

/// Parse %on body items
fn parse_meta_on_body_items(
    body: &crate::syntax::cst::Body,
    source: &str,
) -> Vec<meta_ast::MetaOnBodyItem> {
    let mut items = Vec::new();

    // First check for directives
    for directive in body.directives() {
        if directive.name_text().as_deref() == Some("emit") {
            if let Some(emit) = convert_cst_emit_block(&directive, source) {
                items.push(meta_ast::MetaOnBodyItem::Emit(emit));
            }
        } else if let Some(macro_item) =
            convert_cst_directive_to_macro_body_item(&directive, source)
        {
            items.push(meta_ast::MetaOnBodyItem::MacroItem(Box::new(macro_item)));
        }
    }

    // Check body text for assignments (`$var <- expr`) and bare action statements
    // (a signal call `$move({...})`, an effect, or a macro-param action reference
    // like `$onDrop`). An assignment is recognized by `<-`; any other non-empty,
    // non-comment line is an Action run through the shared mutation rail (PLAN-053).
    let body_text = extract_body_content(body);
    for line in body_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some(arrow_pos) = line.find("<-") {
            let var = line[..arrow_pos].trim().trim_start_matches('$').to_string();
            let expr = line[arrow_pos + 2..]
                .trim()
                .trim_end_matches(';')
                .to_string();
            if !var.is_empty() {
                items.push(meta_ast::MetaOnBodyItem::Assignment { var, expr });
            }
        } else {
            let action = line.trim_end_matches(';').trim().to_string();
            if !action.is_empty() {
                items.push(meta_ast::MetaOnBodyItem::Action(action));
            }
        }
    }

    items
}

// =============================================================================
// =============================================================================
// Import Resolution
// =============================================================================

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Resolve all imports recursively and merge into a single AST.
/// Resolve a page's imports, then normalise its qualified cell references.
///
/// PLAN-117 W3: `app/$host` is rewritten to bare `$host` at the END of this
/// function, once the whole page is assembled. Doing it here — rather than in
/// each consumer — is what makes an alias a pure COMPILE-TIME rename with zero
/// runtime footprint (Elixir `alias` semantics): every downstream stage (emit,
/// analysis, fold, LSP) sees exactly the text the author would have written
/// inside the owning file, so none of them needs to know qualification exists.
///
/// Diagnostics from that pass land in `StFile.diagnostics`.
pub fn resolve_imports(
    main_file: &StFile,
    main_path: &Path,
    workspace_root: &Path,
) -> Result<StFile, String> {
    use crate::lsp::workspace::{
        ImportResolver, IndexingStrategy, ResolvedImport, WorkspaceConfig,
    };

    let config = WorkspaceConfig {
        root: workspace_root.to_path_buf(),
        exclude_patterns: vec![],
        indexing_strategy: IndexingStrategy::Hybrid,
        import_aliases: HashMap::new(),
        stdlib_path: Some(workspace_root.join("stdlib")),
    };

    let resolver = ImportResolver::new(config);
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Track file -> imports for cycle detection
    let mut import_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    // Start with main file
    visited.insert(main_path.to_path_buf());
    queue.push_back((main_path.to_path_buf(), main_file.clone()));

    // Merged result: start from main file, strip @import FormMatches
    // Tag main file's matches and scopes with source_file
    let mut merged = main_file.clone();
    // Drop the global `@import` entries (their content is flat-merged below) but
    // RETAIN the namespaced `@use` entries: downstream the pipeline builds the
    // per-file ImportScope (FEAT-118 FUP-057) from them for qualified-alias
    // resolution and import validation.
    merged.imports.retain(|i| i.namespace.is_some());
    merged.matches.retain(|m| m.macro_name != "import"); // Strip @import FormMatches too

    // Tag main file's matches and scopes with source_file
    let main_path_str = main_path.to_string_lossy().to_string();
    for m in &mut merged.matches {
        m.source_file = Some(main_path_str.clone());
    }
    for s in &mut merged.scopes {
        s.source_file = Some(main_path_str.clone());
    }
    while let Some((current_path, current_file)) = queue.pop_front() {
        let current_imports = &current_file.imports;

        for import_ast in current_imports {
            let resolved = resolver.resolve(&import_ast.path, &current_path);

            match resolved {
                ResolvedImport::File(path) => {
                    // Cycle detection
                    import_graph
                        .entry(current_path.clone())
                        .or_default()
                        .push(path.clone());

                    if has_cycle(&import_graph, &path) {
                        return Err(format!(
                            "Circular import detected: {} imports {}",
                            current_path.display(),
                            path.display()
                        ));
                    }

                    if visited.insert(path.clone()) {
                        // Load and parse the imported file. SIP-002: an
                        // imported `.st.md` is TANGLED first, so a literate
                        // document can be imported exactly like a `.st` module
                        // (and a `.st` page can import a literate one). Without
                        // this the import would parse markdown as Spacetime and
                        // silently contribute nothing.
                        let (content, _line_map) = crate::literate::read_source(&path)
                            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

                        // Parse UNVALIDATED: form-splice validation is only
                        // meaningful against the ASSEMBLED page — a module's
                        // `--name;` refs are declared by its siblings
                        // (`_schools.st` beside `hero/minimal.st`), invisible
                        // to this file-local parse. Validating here attaches
                        // stale E0947s that `check` then reports against the
                        // merged file even though the post-merge seam
                        // (`validate_form_refs` + `expand_style_form_splices`)
                        // clears and re-judges them. Same rule as
                        // `parse_body_fragment`: a fragment is validated where
                        // it is ASSEMBLED, not where it is stored.
                        let imported_ast = parse_impl(&content, false)
                            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

                        // Merge into result (imports processed first, so dependencies come before dependents)
                        let path_str = path.to_string_lossy().to_string();
                        merge_ast(
                            &mut merged,
                            &imported_ast,
                            Some(&path_str),
                            import_ast.namespace.as_ref(),
                        );

                        // Queue for processing
                        queue.push_back((path, imported_ast));
                    }
                }
                ResolvedImport::Stdlib { .. } => {
                    // Stdlib imports are handled by MetaRegistry, skip
                    continue;
                }
                ResolvedImport::Unresolved {
                    original_path,
                    searched_paths,
                } => {
                    return Err(format!(
                        "Could not resolve import '{}' from {}. Searched: {:?}",
                        original_path,
                        current_path.display(),
                        searched_paths
                    ));
                }
            }
        }
    }

    // PLAN-117 W3: the page is now fully assembled, so every module a
    // qualified reference could name is known. Rewrite `app/$host` -> `$host`
    // and report what could not be resolved (E0939 ambiguous tight `/$`, E0940
    // unknown qualifier, E0941 no such cell in that module).
    let qualified_diags = crate::pipeline::qualified_refs::resolve_qualified_refs(&mut merged);
    merged.diagnostics.extend(qualified_diags);

    Ok(merged)
}

/// Detect cycles in import graph using DFS
fn has_cycle(graph: &HashMap<PathBuf, Vec<PathBuf>>, start: &Path) -> bool {
    fn dfs(
        node: &Path,
        graph: &HashMap<PathBuf, Vec<PathBuf>>,
        visited: &mut HashSet<PathBuf>,
        stack: &mut HashSet<PathBuf>,
    ) -> bool {
        if stack.contains(node) {
            return true; // Cycle detected
        }
        if visited.contains(node) {
            return false; // Already processed, no cycle from here
        }

        visited.insert(node.to_path_buf());
        stack.insert(node.to_path_buf());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if dfs(neighbor, graph, visited, stack) {
                    return true;
                }
            }
        }

        stack.remove(node);
        false
    }

    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    dfs(start, graph, &mut visited, &mut stack)
}

/// Merge imported AST into main AST
/// Order matters: presets, patterns, meta_defs, then matches/scopes
/// Tag a MetaDef with its originating source file (for source-relative
/// resolution such as %vendor blob paths and incremental-cache invalidation).
///
/// `pub` because loaders OUTSIDE this module need it — the PLAN-123 project
/// overlay tags `_prelude.st` declarations with their provenance, and a second
/// copy of this match would be one more place to forget a variant.
pub fn set_meta_def_source_file(def: &mut meta_ast::MetaDef, source_file: &str) {
    use meta_ast::MetaDef;
    match def {
        MetaDef::Macro(m) => m.source_file = Some(source_file.to_string()),
        MetaDef::Primitive(p) => p.source_file = Some(source_file.to_string()),
        MetaDef::CaptureType(ct) => ct.source_file = Some(source_file.to_string()),
        MetaDef::RuntimeRegistry(r) => r.source_file = Some(source_file.to_string()),
        MetaDef::Vendor(v) => v.source_file = Some(source_file.to_string()),
        MetaDef::Migration(m) => m.source_file = Some(source_file.to_string()),
        MetaDef::CommentType(c) => c.source_file = Some(source_file.to_string()),
        MetaDef::ScalarType(s) => s.source_file = Some(source_file.to_string()),
        MetaDef::Preset(_) => {}
    }
}
fn merge_ast(
    target: &mut StFile,
    source: &StFile,
    source_file_path: Option<&str>,
    namespace: Option<&crate::metasystem::module::Namespace>,
) {
    // Presets: imported first (can be overridden)
    target.presets.extend_from_slice(&source.presets);

    // Patterns: imported first (can be shadowed)
    target.patterns.extend_from_slice(&source.patterns);

    // PLAN-117 W5: an imported module's file-scope `@exports` join the page's
    // published set, so a qualified read can be checked against what its owner
    // actually publishes. A module with no clause contributes nothing, which is
    // what keeps public-by-default (the Odin floor) true for every existing file.
    target.file_exports.extend_from_slice(&source.file_exports);

    // Meta definitions: imported first. Tag each with the importing file path so
    // source-relative resolution works downstream — notably %vendor, whose blob
    // (`out`) is resolved relative to the declaring file's directory.
    //
    // FEAT-118: for a NAMESPACED import (`@use`, `namespace: Some(ns)`), stamp
    // each macro def's `module` so it registers under the module namespace
    // (M0 Fqn keying turns `module` into the registry key). A GLOBAL import
    // (`@import`, `namespace: None`) leaves `module` untouched — flat-global,
    // byte-identical to pre-FEAT-118 behaviour.
    for def in &source.meta_defs {
        let mut d = def.clone();
        if let Some(sf) = source_file_path {
            set_meta_def_source_file(&mut d, sf);
        }
        if let (Some(ns), meta_ast::MetaDef::Macro(m)) = (namespace, &mut d) {
            m.module = Some(ns.clone());
        }
        target.meta_defs.push(d);
    }

    // FormMatches: the pipeline's primary input. Merge all except @import directives
    // (imports are already resolved recursively by resolve_imports, so we skip them
    // to avoid the pipeline re-processing them as no-op macros).
    target.matches.extend(
        source
            .matches
            .iter()
            .filter(|m| m.macro_name != "import")
            .map(|m| {
                let mut fm = m.clone();
                if let Some(path) = source_file_path {
                    fm.source_file = Some(path.to_string());
                }
                fm
            }),
    );

    // Scopes (legacy representation, kept for compatibility)
    // Tag each ScopeBlock with source_file if provided.
    target.scopes.extend(source.scopes.iter().map(|scope| {
        let mut s = scope.clone();
        if let Some(path) = source_file_path {
            s.source_file = Some(path.to_string());
        }
        s
    }));

    // File-level form splices (BUG-241) and parse-time diagnostics travel
    // WITH the imported file. Dropping diagnostics here meant an imported
    // file's E0947 (unknown form splice) vanished whenever the merged AST had
    // no user macros to trigger a rematch — the same silent-drop class the
    // code exists to close. When a rematch DOES run later, validate_form_refs
    // clears and recomputes its own E0947s, so carrying them cannot double.
    target.form_refs.extend_from_slice(&source.form_refs);
    target
        .diagnostics
        .extend(source.diagnostics.iter().cloned());

    // File-scope HTML literal blocks: an imported file may contribute page markup
    // (the `<tag>` sigil), exactly like the entry file. Without this merge,
    // imported markup is silently dropped from compiled.html and multi-file
    // Spacetime apps cannot split their shell across files (BUG-075). Imported
    // blocks append in import (dependency-first) order, after any already-merged
    // markup — consistent with how presets/matches merge imported content.
    target.html_blocks.extend_from_slice(&source.html_blocks);

    // Authored CSS at-rule blocks (@media/@supports/@keyframes) from imported modules,
    // so a section module can carry its own responsive rules (BUG-087). Imported blocks
    // are appended in import order (before the entry file's later-merged blocks).
    target
        .raw_css_blocks
        .extend_from_slice(&source.raw_css_blocks);
}

// =============================================================================
// Built-in Presets
// =============================================================================

/// Get built-in preset definitions (easings, scroll presets, etc.)
pub fn get_builtin_presets() -> HashMap<String, PresetDef> {
    let mut presets = HashMap::new();

    // Easing presets
    let easings = [
        ("&linear", EasingValue::CubicBezier(0.0, 0.0, 1.0, 1.0)),
        (
            "&ease-in-quad",
            EasingValue::CubicBezier(0.55, 0.085, 0.68, 0.53),
        ),
        (
            "&ease-out-quad",
            EasingValue::CubicBezier(0.25, 0.46, 0.45, 0.94),
        ),
        (
            "&ease-in-out-quad",
            EasingValue::CubicBezier(0.455, 0.03, 0.515, 0.955),
        ),
        (
            "&ease-out-expo",
            EasingValue::CubicBezier(0.19, 1.0, 0.22, 1.0),
        ),
        (
            "&ease-out-back",
            EasingValue::CubicBezier(0.175, 0.885, 0.32, 1.275),
        ),
        (
            "&ease-out-elastic",
            EasingValue::Spring {
                stiffness: 400.0,
                damping: 10.0,
                mass: 1.0,
            },
        ),
        (
            "&spring-bouncy",
            EasingValue::Spring {
                stiffness: 400.0,
                damping: 10.0,
                mass: 1.0,
            },
        ),
        (
            "&spring-smooth",
            EasingValue::Spring {
                stiffness: 200.0,
                damping: 30.0,
                mass: 1.0,
            },
        ),
        (
            "&spring-snappy",
            EasingValue::Spring {
                stiffness: 500.0,
                damping: 25.0,
                mass: 1.0,
            },
        ),
    ];

    for (name, easing) in easings {
        presets.insert(
            name.to_string(),
            PresetDef {
                preset_type: PresetType::Easing,
                name: name.to_string(),
                value: PresetValue::Easing(easing),
                span: SourceSpan::default(),
            },
        );
    }

    // Scroll presets
    let scroll_presets = [
        ("&reveal", vec![("start", 0.0), ("end", 0.0)]),
        ("&quick-reveal", vec![("start", 0.0), ("end", 0.5)]),
        ("&through-center", vec![("start", 0.0), ("end", 1.0)]),
        ("&parallax", vec![("start", 0.2), ("end", 0.8)]),
    ];

    for (name, config) in scroll_presets {
        let args: Vec<ConfigArg> = config
            .into_iter()
            .map(|(k, v)| ConfigArg {
                key: k.to_string(),
                value: ConfigValue::Number(v),
            })
            .collect();

        presets.insert(
            name.to_string(),
            PresetDef {
                preset_type: PresetType::Scroll,
                name: name.to_string(),
                value: PresetValue::Config(args),
                span: SourceSpan::default(),
            },
        );
    }

    presets
}

#[cfg(test)]
mod test_doc_capture {
    use crate::parser::meta_ast::MetaDef;
    use crate::parser::parse;

    fn macro_doc(file: &crate::parser::ast::StFile, name: &str) -> Option<String> {
        file.meta_defs.iter().find_map(|d| match d {
            MetaDef::Macro(m) if m.name == name => m.doc.clone(),
            _ => None,
        })
    }
    fn prim_doc(file: &crate::parser::ast::StFile, name: &str) -> Option<String> {
        file.meta_defs.iter().find_map(|d| match d {
            MetaDef::Primitive(p) if p.name == name => p.doc.clone(),
            _ => None,
        })
    }

    #[test]
    fn test_doc_comment_captured_on_macro() {
        let src = "/// First line.\n/// Second line.\n%macro foo {\n  %form { @foo }\n}\n";
        let file = parse(src).expect("parse");
        assert_eq!(
            macro_doc(&file, "foo").as_deref(),
            Some("First line.\nSecond line."),
            "contiguous /// block must be captured, markers + one space stripped"
        );
    }

    #[test]
    fn test_doc_comment_single_blank_gap_before_def_allowed() {
        // The common stdlib shape: doc block, ONE blank line, then the def.
        let src = "/// Docs here.\n\n%primitive bar(&el) {\n  %emit js { }\n}\n";
        let file = parse(src).expect("parse");
        assert_eq!(prim_doc(&file, "bar").as_deref(), Some("Docs here."));
    }

    #[test]
    fn test_no_doc_when_undocumented() {
        let src = "%macro baz {\n  %form { @baz }\n}\n";
        let file = parse(src).expect("parse");
        assert_eq!(macro_doc(&file, "baz"), None);
    }

    #[test]
    fn test_plain_double_slash_is_not_doc() {
        // `//` (not `///`) is an ordinary comment, never a doc-comment.
        let src = "// just a note\n%macro qux {\n  %form { @qux }\n}\n";
        let file = parse(src).expect("parse");
        assert_eq!(macro_doc(&file, "qux"), None);
    }
}

#[cfg(test)]
mod test_sourcespan {
    use crate::parser::parse;

    #[test]
    fn test_spans_are_captured() {
        let input = r#"
.hero {
    @load intro(duration: 500ms) {
        opacity: 0 -> 1;
    }
}
"#;

        let ast = parse(input).expect("Should parse successfully");

        // File span should cover the entire input
        assert_eq!(ast.span.start, 0);
        assert_eq!(ast.span.end, input.len());

        // Scope should have a span
        assert_eq!(ast.scopes.len(), 1);
        let scope = &ast.scopes[0];
        assert!(scope.span.start > 0);
        assert!(scope.span.end > scope.span.start);

        // FormMatch in scope should have a span
        let load_fm = scope
            .matches
            .iter()
            .find(|m| m.macro_name == "load")
            .expect("scope should have @load FormMatch");
        assert!(load_fm.span.start > 0);
        assert!(load_fm.span.end > load_fm.span.start);
    }

    #[test]
    fn test_json_serialization() {
        let input = r#"
.hero {
    @load intro(duration: 500ms) {
        opacity: 0 -> 1;
    }
}
"#;

        let ast = parse(input).expect("Should parse successfully");

        // Test that the AST can be serialized to JSON
        let json = serde_json::to_string(&ast).expect("Should serialize to JSON");
        assert!(json.contains("\"span\""));
        assert!(json.contains("\"start\""));
        assert!(json.contains("\"end\""));

        // Test that it can be deserialized back
        let _deserialized: crate::parser::StFile =
            serde_json::from_str(&json).expect("Should deserialize from JSON");
    }

    // (removed) test_nested_transition_named_args — exercised the @state_machine/
    // @transition construct retired in PLAN-047. The surviving declarative state
    // surface (@state(when:), @state-match, @socket) is covered elsewhere.

    #[test]
    fn test_bind_in_scope_produces_file_match() {
        let input = r#"
.nav {
    @bind(class: "nav--dark", when: $theme === "dark")
}
"#;
        let ast = parse(input).expect("Should parse");
        let macro_names: Vec<&str> = ast.matches.iter().map(|m| m.macro_name.as_str()).collect();
        assert!(
            macro_names.contains(&"bind"),
            "ast.matches should include @bind from scope block. Got: {:?}",
            macro_names
        );
    }

    #[test]
    fn test_import_plus_bind_produces_both_matches() {
        let input = r#"
@import "./faq.st";

.nav {
    @bind(class: "nav--dark", when: $theme === "dark")
}
"#;
        let ast = parse(input).expect("Should parse");
        let macro_names: Vec<&str> = ast.matches.iter().map(|m| m.macro_name.as_str()).collect();
        assert!(
            macro_names.contains(&"import"),
            "Should have import. Got: {:?}",
            macro_names
        );
        assert!(
            macro_names.contains(&"bind"),
            "Should have bind. Got: {:?}",
            macro_names
        );
    }

    /// FUP-069: interior directives of an opaque `:block` body (`@test`/`@mount`)
    /// must NOT leak to the page-level `ast.matches`. The macro recompiles that
    /// body as its own sub-program (`%$content.js`); a leaked file-scope `@on`
    /// would bind a SECOND handler on `document.body` (an ancestor of the mounted
    /// node) and double-fire every click. Pin: a `.b1{ @on click }` inside
    /// `@mount{}` inside `@test{}` yields NO top-level `on` / `local-state` match.
    #[test]
    fn test_opaque_block_body_interior_matches_suppressed() {
        let input = r#"@import "stdlib/testing/test"
@test "drive" {
  @mount { <button class="b1">+</button>
    .b1 { $n number: 0; @on click { $n <- $n + 1; } } }
  @when .b1 click
  @then .b1 { $n == 1; }
}
"#;
        let ast = parse(input).expect("should parse");
        let macro_names: Vec<&str> = ast.matches.iter().map(|m| m.macro_name.as_str()).collect();
        assert!(
            macro_names.contains(&"test"),
            "the @test directive itself must survive. Got: {:?}",
            macro_names
        );
        assert!(
            !macro_names.contains(&"on"),
            "interior @on must NOT leak to page-level matches. Got: {:?}",
            macro_names
        );
        assert!(
            !macro_names.contains(&"local-state"),
            "interior local-state must NOT leak to page-level matches. Got: {:?}",
            macro_names
        );
    }

    #[test]
    fn test_matches_include_scoped_directives() {
        let input = r#"
body {
    $activeQ number: -1;
}

.faq-q0 {
    @on &.click { $activeQ <- 0; }
}

.faq-item-0 {
    @portal(when: $activeQ);
}
"#;
        let ast = parse(input).expect("Should parse");

        // ast.matches should contain ALL FormMatches, including those inside scopes
        let macro_names: Vec<&str> = ast.matches.iter().map(|m| m.macro_name.as_str()).collect();
        assert!(
            macro_names.contains(&"local-state"),
            "ast.matches should include $activeQ state decl, got: {:?}",
            macro_names
        );
        assert!(
            macro_names.contains(&"on"),
            "ast.matches should include @on click, got: {:?}",
            macro_names
        );
        assert!(
            macro_names.contains(&"portal"),
            "ast.matches should include @portal, got: {:?}",
            macro_names
        );
    }

    /// PLAN-122 W1.2 — the four CSS scalar names resolve to the STDLIB grammars
    /// in `stdlib/capture-types/css-values.st`, not to closed Rust enum arms.
    ///
    /// This test previously asserted the opposite (`"color"` -> `CaptureType::Color`).
    /// Inverting it IS the wave: while the arm existed, a stdlib `%capture_type
    /// color` was dead text — the registry loaded it and `parse_capture_type`
    /// never asked. `Custom(name)` is what sends the lookup to the registry.
    ///
    /// This is not a new mechanism; it is the migration route already taken by
    /// `properties`, `params`, `keyframes` and `param_list` (PLAN-023 W2), whose
    /// comments sit beside these arms explaining that they were REMOVED so they
    /// would "resolve to the stdlib grammar + reifier". Four more names, same road.
    ///
    /// Why it matters beyond tidiness: the Rust arms and the grammar disagree.
    /// The enum arm accepts whatever its hand-written extractor accepts; the
    /// grammar states the legal hex lengths as DATA (`{3|4|6|8}`), so `#e8ee1`
    /// becomes a spanned error instead of passing `check` and landing verbatim
    /// in the emitted stylesheet (BUG-257).
    #[test]
    fn css_scalar_names_resolve_to_stdlib_grammars_not_rust_arms() {
        for name in ["color", "length", "duration", "time", "easing"] {
            assert_eq!(
                super::parse_capture_type(name),
                super::meta_ast::CaptureType::Custom(name.to_string()),
                "`{name}` still resolves to a hardcoded Rust arm, so the stdlib \
                 %capture_type of the same name in css-values.st is shadowed and \
                 can never match. Delete the arm in parse_capture_type."
            );
        }
    }

    /// The names must not fall to the lenient `Expr` default either — that is the
    /// silent failure mode this wave is most at risk of. `Expr` swallows nearly
    /// anything, so a typo'd or unregistered scalar would still "work" while
    /// validating nothing at all.
    #[test]
    fn css_scalar_names_do_not_fall_through_to_expr() {
        for name in ["color", "length", "duration", "time", "easing"] {
            assert_ne!(
                super::parse_capture_type(name),
                super::meta_ast::CaptureType::Expr,
                "`{name}` fell through to the Expr default — it would match \
                 anything and validate nothing"
            );
        }
    }
    // ── FEAT-103: body-group PEG in %form ──────────────────────────────────────

    #[test]
    fn feat103_optional_group_extracted_from_body() {
        // An optional literal-led group at the head of a body is peeled into
        // body_groups; the remaining tail stays as body_capture.
        let mut bc =
            Some("{ ( \"shortcut\" \":\" $s:string \";\"? )? $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert_eq!(groups.len(), 1, "one leading optional group");
        assert_eq!(bc.as_deref(), Some("{ $body:component_body }"));
        assert!(matches!(
            groups[0],
            crate::parser::meta_ast::CapturePatternAst::Group {
                modifier: Some(crate::parser::meta_ast::CaptureModifier::Optional),
                ..
            }
        ));
    }

    #[test]
    fn feat103_repeat_group_extracted() {
        // A `( ... )*` repeated run is a body group with ZeroOrMore. It must be
        // LITERAL-LED (FEAT-104 #4) to anchor ahead of the greedy body — here the
        // `"attr"` literal is the sentinel.
        let mut bc = Some(
            "{ ( \"attr\" $k:ident \"=\" $v:string \";\" )* $body:component_body }".to_string(),
        );
        let groups = super::extract_body_groups(&mut bc);
        assert_eq!(groups.len(), 1);
        assert!(matches!(
            groups[0],
            crate::parser::meta_ast::CapturePatternAst::Group {
                modifier: Some(crate::parser::meta_ast::CaptureModifier::ZeroOrMore),
                ..
            }
        ));
        assert_eq!(bc.as_deref(), Some("{ $body:component_body }"));
    }

    #[test]
    fn feat103_no_group_leaves_body_untouched() {
        // The legacy path: a body with no leading group yields zero groups and an
        // unchanged body_capture.
        let mut bc = Some("{ $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert!(groups.is_empty());
        assert_eq!(bc.as_deref(), Some("{ $body:component_body }"));
    }

    #[test]
    fn feat103_pseudo_selector_not_treated_as_group() {
        // `(:name { ... })` is a pseudo-selector, NOT a body group; it stays in
        // body_capture for refine_body_capture/PseudoParser.
        let mut bc = Some("{ (:entering { $a:keyframes })? $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert!(
            groups.is_empty(),
            "pseudo-selector must not be a body group"
        );
        assert!(bc.as_deref().unwrap().contains(":entering"));
    }

    #[test]
    fn feat103_form_pattern_carries_body_groups() {
        // End-to-end through parse_form_pattern: a directive form with a leading
        // optional group exposes it on FormClause.body_groups.
        let pat = "@thing { ( \"meta\" \":\" $m:string \";\"? )? $body:component_body }";
        let form = super::parse_form_pattern(
            pat,
            rowan::TextRange::new(0.into(), (pat.len() as u32).into()),
        )
        .expect("form parses");
        assert_eq!(
            form.body_groups.len(),
            1,
            "leading group captured on the form"
        );
        assert_eq!(
            form.body_capture.as_deref(),
            Some("{ $body:component_body }")
        );
    }

    #[test]
    fn feat104_paren_literal_group_bounded() {
        // #2: a group whose literal contains a paren must be bounded quote-aware,
        // not truncated at the inner paren.
        let mut bc = Some("{ ( \"(\" \":\" $s:string \")\" )? $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert_eq!(groups.len(), 1, "paren-literal group must bound correctly");
        assert_eq!(bc.as_deref(), Some("{ $body:component_body }"));
    }

    #[test]
    fn feat104_unbalanced_paren_not_a_group() {
        // #2: an unbalanced leading `(` is NOT a body group; it stays in body_capture
        // (to fail loudly downstream) rather than silently swallowing the body.
        let mut bc = Some("{ ( \"x\" $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert!(groups.is_empty(), "unbalanced paren must not be a group");
        assert!(bc.as_deref().unwrap().contains("$body"));
    }

    #[test]
    fn feat104_whitespace_before_modifier_honored() {
        // #3: `( ... ) ?` (space before `?`) must read as Optional, not Required,
        // and must not leak the stray `?`.
        let mut bc =
            Some("{ ( \"shortcut\" \":\" $s:string ) ? $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert_eq!(groups.len(), 1);
        assert!(matches!(
            groups[0],
            crate::parser::meta_ast::CapturePatternAst::Group {
                modifier: Some(crate::parser::meta_ast::CaptureModifier::Optional),
                ..
            }
        ));
        assert_eq!(
            bc.as_deref(),
            Some("{ $body:component_body }"),
            "no leaked ?"
        );
    }

    #[test]
    fn feat104_non_literal_led_optional_rejected() {
        // #4: a non-literal-led OPTIONAL group ahead of a greedy body has no anchor;
        // it must NOT be treated as a body group (left in body_capture).
        let mut bc = Some("{ ( $x:ident )? $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert!(
            groups.is_empty(),
            "non-literal-led optional group must be rejected"
        );
        assert!(bc.as_deref().unwrap().contains("$x:ident"));
    }

    #[test]
    fn feat104_literal_led_optional_accepted() {
        // #4 control: a literal-led optional group IS accepted.
        let mut bc = Some("{ ( \"meta\" $x:ident )? $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert_eq!(groups.len(), 1, "literal-led optional group accepted");
    }

    #[test]
    fn feat104_required_group_exempt_from_literal_led() {
        // #4: a REQUIRED group is self-anchoring (its absence fails the form), so it
        // need not be literal-led.
        let mut bc = Some("{ ( $x:ident ) $body:component_body }".to_string());
        let groups = super::extract_body_groups(&mut bc);
        assert_eq!(
            groups.len(),
            1,
            "required group exempt from literal-led rule"
        );
    }

    // ---- FEAT-118: %imports declared-effect clause parsing ----

    fn imports_of(src: &str) -> Option<crate::parser::meta_ast::ImportsClause> {
        let file = super::parse(src).expect("parse");
        file.meta_defs.into_iter().find_map(|d| match d {
            crate::parser::meta_ast::MetaDef::Macro(m) => m.imports,
            _ => None,
        })
    }

    #[test]
    fn imports_global_atimport_shape() {
        // @import is the global degenerate case of the import spectrum.
        let c = imports_of(
            "%macro import {\n  %form { @import $path:string }\n  %imports { module: $path, global: true }\n}",
        )
        .expect("%imports parsed");
        assert_eq!(c.module, "path");
        assert!(c.global);
        assert!(c.alias.is_none());
        assert!(c.only.is_empty() && c.hiding.is_empty());
    }

    #[test]
    fn imports_use_namespaced_with_alias_and_only() {
        let c = imports_of(
            "%macro use {\n  %form { @use $path:string }\n  %imports {\n    module: $path\n    as: $alias\n    only: ($a, $b)\n  }\n}",
        )
        .expect("%imports parsed");
        assert_eq!(c.module, "path");
        assert!(!c.global, "@use is namespaced, not global");
        assert_eq!(c.alias.as_deref(), Some("alias"));
        assert_eq!(c.only, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn imports_hiding_list() {
        let c = imports_of(
            "%macro use {\n  %form { @use $path:string }\n  %imports { module: $path; hiding: ($glitch) }\n}",
        )
        .expect("%imports parsed");
        assert_eq!(c.hiding, vec!["glitch".to_string()]);
    }

    #[test]
    fn imports_absent_on_ordinary_macro() {
        // A macro with no %imports clause has imports == None (no false positives).
        let none = imports_of("%macro plain {\n  %form { @plain }\n}");
        assert!(none.is_none());
    }

    // ---- FEAT-118 FUP-053: @use invocation-tail parsing ----

    #[test]
    fn use_tail_alias() {
        let (alias, only, hiding) = super::parse_use_clause_tail(Some("\"std:scene\" as s"));
        assert_eq!(alias.as_deref(), Some("s"));
        assert!(only.is_empty() && hiding.is_empty());
    }

    #[test]
    fn use_tail_only_list() {
        let (alias, only, _) =
            super::parse_use_clause_tail(Some("\"std:scene\" only (camera, light)"));
        assert!(alias.is_none());
        assert_eq!(only, vec!["camera".to_string(), "light".to_string()]);
    }

    #[test]
    fn use_tail_alias_and_hiding() {
        let (alias, only, hiding) =
            super::parse_use_clause_tail(Some("\"std:scene\" as s hiding (glitch)"));
        assert_eq!(alias.as_deref(), Some("s"));
        assert!(only.is_empty());
        assert_eq!(hiding, vec!["glitch".to_string()]);
    }

    #[test]
    fn use_tail_bare_open_import() {
        // Just the module string — open import, no alias/only/hiding.
        let (alias, only, hiding) = super::parse_use_clause_tail(Some("\"std:scene\""));
        assert!(alias.is_none() && only.is_empty() && hiding.is_empty());
    }

    // ---- FEAT-118 M2: MODULE.st manifest parsing ----

    #[test]
    fn manifest_module_name() {
        let f = super::parse("%module scene").expect("parse");
        let m = f.module_manifest.expect("manifest present");
        assert_eq!(m.name.as_deref(), Some("scene"));
    }

    #[test]
    fn manifest_public_list() {
        let f = super::parse("%public (camera, light, fog)").expect("parse");
        let m = f.module_manifest.expect("manifest");
        assert_eq!(
            m.public,
            vec!["camera".to_string(), "light".to_string(), "fog".to_string()]
        );
    }

    #[test]
    fn manifest_reexport() {
        let f = super::parse("%reexport \"std:scene-3d\" (form, surface)").expect("parse");
        let m = f.module_manifest.expect("manifest");
        assert_eq!(m.reexports.len(), 1);
        assert_eq!(m.reexports[0].module, "std:scene-3d");
        assert_eq!(
            m.reexports[0].names,
            vec!["form".to_string(), "surface".to_string()]
        );
    }

    #[test]
    fn manifest_full_module_st() {
        // A complete MODULE.st: name + public + reexport, plus the defs are
        // NOT swept into the manifest (only the three clauses are).
        let src = "%module scene\n%public (camera, light)\n%reexport \"std:scene-3d\" (form)\n";
        let f = super::parse(src).expect("parse");
        let m = f.module_manifest.expect("manifest");
        assert_eq!(m.name.as_deref(), Some("scene"));
        assert_eq!(m.public, vec!["camera".to_string(), "light".to_string()]);
        assert_eq!(m.reexports.len(), 1);
    }

    #[test]
    fn manifest_absent_on_ordinary_file() {
        let f = super::parse("%macro x {\n  %form { @x }\n}").expect("parse");
        assert!(f.module_manifest.is_none());
    }

    // ---- FEAT-118 M3: %using active-extension hook parsing ----

    #[test]
    fn using_hook_claims_and_defaults() {
        let src = "%using {\n  %claims @camera @light @fog\n  %capture_type camera_type { $fov:string }\n  %default fov: 75\n}";
        let f = super::parse(src).expect("parse");
        let hook = f
            .module_manifest
            .and_then(|m| m.using)
            .expect("using hook present");
        assert_eq!(
            hook.claims,
            vec!["camera".to_string(), "light".to_string(), "fog".to_string()]
        );
        assert_eq!(hook.capture_types, vec!["camera_type".to_string()]);
        assert_eq!(hook.defaults, vec!["fov: 75".to_string()]);
        assert!(
            !hook.raw_body.is_empty(),
            "raw body preserved for execution layer"
        );
    }

    #[test]
    fn using_hook_absent_without_clause() {
        let f = super::parse("%module scene").expect("parse");
        assert!(f.module_manifest.unwrap().using.is_none());
    }

    /// Regression: a project-local `%macro` nested INSIDE a stdlib macro body
    /// (here `@media { … }`, which captures `$children*`) must survive
    /// `rematch_with_user_macros`. The augmented re-parse absorbs the user child
    /// into the parent's children block, so the parent SHARES its span with the
    /// stdlib-only parse and the old span-only diff discarded the enriched parent —
    /// silently dropping the nested user directive. The fix replaces the stale
    /// parent match in-place when its nested-match count grew. (everswap-clone
    /// `@layer` inside `@stage` hit this.)
    #[test]
    fn nested_user_macro_inside_stdlib_body_survives_rematch() {
        let src = r#"
%macro widget {
  %scope selector
  %form { @widget(n: $n:number = 1) }
  %binds { widget-prim(&self, n: $n) }
}

.box {
  @media("(min-width: 600px)") {
    @widget(n: 7)
  }
}
"#;
        use crate::parser::ast::NestedScope;
        use crate::syntax::{CapturedValue, FormMatch};

        let mut ast = super::parse(src).expect("parse");
        super::rematch_with_user_macros(&mut ast, src);

        // True if `@widget` appears anywhere reachable from a match: as the match
        // itself, or nested inside its captures (the `@media` children block).
        fn value_has_widget(v: &CapturedValue) -> bool {
            match v {
                CapturedValue::Block(children) => children
                    .iter()
                    .any(|c| c.macro_name == "widget" || c.captures.values().any(value_has_widget)),
                CapturedValue::Named(map) => map.values().any(value_has_widget),
                CapturedValue::Array(items) => items.iter().any(value_has_widget),
                _ => false,
            }
        }
        fn finds_widget(matches: &[FormMatch]) -> bool {
            matches
                .iter()
                .any(|m| m.macro_name == "widget" || m.captures.values().any(value_has_widget))
        }
        fn nested_has_widget(nested: &[NestedScope]) -> bool {
            nested
                .iter()
                .any(|n| finds_widget(&n.matches) || nested_has_widget(&n.nested_scopes))
        }

        let found = finds_widget(&ast.matches)
            || ast
                .scopes
                .iter()
                .any(|s| finds_widget(&s.matches) || nested_has_widget(&s.nested_scopes));

        assert!(
            found,
            "nested project-local @widget must survive rematch inside stdlib @media body"
        );
    }
}
