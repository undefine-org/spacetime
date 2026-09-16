//! Pipeline Module - 5-Layer Compilation Pipeline
//!
//! This module implements the new compilation pipeline with clear separation of concerns:
//!
//! ## Layers
//!
//! 1. **Bootstrap (Layer 0)**: stdlib/*.st → SyntaxRegistry
//!    - Load standard library definitions
//!    - Build form registry for parsing
//!
//! 2. **Parse (Layer 1)**: source + registry → Vec<FormMatch>
//!    - Handled by existing `syntax::parser` module
//!    - Pattern matching against registered forms
//!
//! 3. **Resolve (Layer 2)**: FormMatch → Vec<ResolvedPrimitive>
//!    - Apply %bind clauses from macros
//!    - Determine phase and order
//!    - Create bound argument maps
//!
//! 4. **Sort (Layer 3)**: topological sort by (phase, order)
//!    - Ensure Global phase runs before Selector phase
//!    - Order primitives within each phase
//!
//! 5. **Expand (Layer 4)**: ResolvedPrimitive → Vec<ExpandedPrimitive>
//!    - Evaluate primitive %emit blocks
//!    - Substitute bound arguments
//!    - Generate typed IR (JsFragment/CssFragment with scope isolation)
//!
//! 6. **Emit (Layer 5)**: ExpandedPrimitive → CompiledOutput
//!    - Wrap JS in IIFE/Block scopes for variable isolation
//!    - Add runtime boilerplate
//!    - Format final output
//!
//! ## Usage
//!
//! ```rust,ignore
//! use spacetime::pipeline::{compile, CompileContext};
//! use spacetime::syntax::FormMatch;
//! use spacetime::metasystem::MetaRegistry;
//!
//! let context = CompileContext {
//!     meta_registry: MetaRegistry::new(),
//!     include_runtime: true,
//! };
//!
//! let matches = vec![/* ... */];
//! let output = compile(&matches, &context).expect("compile");
//!
//! println!("JS: {}", output.js);
//! println!("CSS: {}", output.css);
//! ```

pub mod drivers;
pub mod emit;
pub mod evaluate;
pub mod expand;
pub mod qualified_refs;
pub mod resolve;
pub mod scope;
pub mod score;
pub mod sort;
pub mod state_analysis;
pub mod types;

#[cfg(test)]
mod integration_tests;

// Re-export main types
pub use emit::{emit_typed, emit_typed_with_runtime};
pub use expand::{ExpandError, expand_typed};
pub use resolve::{ResolveError, resolve};
pub use sort::{sort, sort_in_place};
pub use types::{
    BoundArgs, CompileError, CompileErrorKind, CssFragment, ElInit, ExpandedPrimitive, JsFragment,
    JsScope, Phase, PipelineOutput, ResolvedPrimitive,
};
// Note: StdlibLoadError and StdlibLoadErrorKind are defined in this module and are public

use std::collections::{HashMap, HashSet};

use crate::diagnostics::{Diagnostic, DiagnosticCode, find_similar_multiple, format_suggestion};
use crate::emit::metasystem_codegen::{
    generate_pattern_match_css, generate_pattern_match_js, generate_state_css, parse_pattern_match,
};
use crate::ir::{CssExpr, JsStmt};
use crate::metasystem::MetaRegistry;
use crate::parser::meta_ast::MetaStateDef;
use crate::syntax::{CapturedValue, FormMatch};

// =============================================================================
// Compilation Context
// =============================================================================

/// Context for pipeline compilation
#[derive(Debug, Clone)]
pub struct CompileContext {
    /// Metasystem registry (primitives, macros, presets)
    pub meta_registry: MetaRegistry,

    /// Whether to include runtime boilerplate
    pub include_runtime: bool,

    /// `check --at-version`: behave as if the project's @version were this
    /// wave date — overrides the @version fact in the retired-syntax
    /// tri-branch (E0911 inert-wave check).
    pub version_override: Option<String>,

    /// FEAT-115 S5c: the parsed World-A scope tree (template Construct scopes +
    /// their nested synthesized/`.sel` scopes). Template-body diagnostics
    /// (E0905 undefined-state, E0917 unused-state) read directive var refs from
    /// HERE — the AST truth — instead of the reify `ComponentBodyDef.directives`
    /// parallel parse. Empty for callers that don't validate template bodies.
    pub scopes: Vec<crate::parser::ScopeBlock>,

    /// FEAT-118 FUP-057: the per-file import environment, derived from the
    /// file's `@use`/`@import` statements. Drives qualified-alias resolution
    /// (`@s/camera`), import validation (unbound alias, `only`/`hiding`
    /// visibility), and `%using` scoping. Empty (`is_empty()`) for files with
    /// no namespaced imports — the common case — so the pipeline skips all
    /// scope work.
    pub import_scope: crate::metasystem::module::ImportScope,

    /// FEAT-120: parse-time body-validation diagnostics produced in `cst_to_stfile`
    /// (`StFile.diagnostics`). These have NO flat-match surface and no source text at
    /// pipeline time, so they are carried in as already-canonical `Diagnostic`s and
    /// drained once into `output.diagnostics`. Replaces the retired `ComponentBodyDef`
    /// capture envelope + its two per-code bridges. Empty for callers that don't parse
    /// template bodies.
    pub body_diagnostics: Vec<crate::diagnostics::Diagnostic>,

    /// Dev-only `(source span, structure id)` addresses for template mounts.
    pub invocation_node_ids: Vec<((usize, usize), String)>,
}

impl CompileContext {
    /// Create a new compile context with the given registry
    pub fn new(meta_registry: MetaRegistry) -> Self {
        Self {
            meta_registry,
            version_override: None,
            include_runtime: true,
            scopes: Vec::new(),
            import_scope: crate::metasystem::module::ImportScope::default(),
            body_diagnostics: Vec::new(),
            invocation_node_ids: Vec::new(),
        }
    }

    /// Create a context without runtime boilerplate
    pub fn with_version_override(mut self, version: Option<String>) -> Self {
        self.version_override = version;
        self
    }

    pub fn without_runtime(mut self) -> Self {
        self.include_runtime = false;
        self
    }

    /// Attach the parsed scope tree so template-body diagnostics can read
    /// directive var refs from World A (FEAT-115 S5c).
    pub fn with_scopes(mut self, scopes: Vec<crate::parser::ScopeBlock>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Attach the per-file import environment (FEAT-118 FUP-057), built from the
    /// file's parsed `@use`/`@import` statements via
    /// [`ImportScope::from_imports`].
    pub fn with_imports(mut self, imports: &[crate::parser::ast::ImportAst]) -> Self {
        self.import_scope = crate::metasystem::module::ImportScope::from_imports(imports);
        self
    }

    /// Attach dev-only template invocation structure ids.
    pub fn with_invocation_node_ids(mut self, ids: Vec<((usize, usize), String)>) -> Self {
        self.invocation_node_ids = ids;
        self
    }

    /// Attach parse-time body-validation diagnostics (FEAT-120) so the pipeline can
    /// drain them into `output.diagnostics` alongside the World-A `check_*` passes.
    pub fn with_body_diagnostics(
        mut self,
        diagnostics: Vec<crate::diagnostics::Diagnostic>,
    ) -> Self {
        self.body_diagnostics = diagnostics;
        self
    }
}

impl Default for CompileContext {
    fn default() -> Self {
        Self::new(MetaRegistry::new())
    }
}

/// A CompileContext whose registry carries one comment-emitting probe
/// primitive per name — the honest replacement for the "Primitive not found"
/// stub the plumbing tests relied on (BUG-268): the pipeline under test sees
/// a REAL primitive, and the emitted marker keeps the name visible to the
/// ordering/content assertions.
#[cfg(test)]
pub(crate) fn probe_context(names: &[&str]) -> CompileContext {
    let mut registry = MetaRegistry::new();
    for name in names {
        let src = format!(
            "%primitive {name} {{\n  %emit js {{ console.log(\"probe: {name}\"); }}\n}}"
        );
        let parsed = crate::parser::parse_for_bootstrap(&src).expect("probe primitive parses");
        for def in parsed.meta_defs {
            if let crate::parser::meta_ast::MetaDef::Primitive(p) = def {
                registry.register_primitive(p).expect("register probe primitive");
            }
        }
    }
    CompileContext::new(registry)
}

// =============================================================================
// Stdlib Load Error
// =============================================================================

use crate::parser::SourceSpan;
use std::path::PathBuf;

/// Error that occurs when loading stdlib files
#[derive(Debug, Clone)]
pub struct StdlibLoadError {
    /// Path to the stdlib file that failed to load
    pub file_path: PathBuf,
    /// Byte offset span where the error occurred (if parse error)
    pub span: Option<SourceSpan>,
    /// Error message
    pub message: String,
    /// Kind of error
    pub kind: StdlibLoadErrorKind,
}

/// Kind of stdlib loading error
#[derive(Debug, Clone)]
pub enum StdlibLoadErrorKind {
    /// File could not be read
    FileNotFound,
    /// File could not be parsed
    ParseError,
    /// Duplicate macro definition
    DuplicateMacro(String),
}

impl std::fmt::Display for StdlibLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file_path.display(), self.message)
    }
}

impl std::error::Error for StdlibLoadError {}

// =============================================================================
// Pipeline Error (unified with CompileError)
// =============================================================================

/// Errors that can occur during pipeline compilation.
///
/// `PipelineError` is now a type alias for the unified `CompileError`.
/// No `From<ResolveError>` or `From<ExpandError>` conversion impls — the type IS the error.
pub type PipelineError = CompileError;

impl From<StdlibLoadError> for CompileError {
    fn from(e: StdlibLoadError) -> Self {
        CompileError::new(
            CompileErrorKind::StdlibLoad(e),
            crate::parser::SourceSpan::default(),
        )
    }
}
// =============================================================================
// Friendly Error Messages
// =============================================================================

/// Valid Spacetime meta directives with descriptions
pub const VALID_DIRECTIVES: &[(&str, &str)] = &[
    // Core definitions
    (
        "primitive",
        "Define a low-level component with JS implementation",
    ),
    ("macro", "Define a reusable template"),
    ("form", "Define the syntax pattern for a macro"),
    ("emit", "Output raw JS/CSS/GLSL code"),
    ("bind", "Bind parameters to primitives"),
    // Data & state
    ("binds", "Declare data bindings"),
    ("derives", "Calculate derived/computed values"),
    ("states", "Define state conditions"),
    ("provides", "Export variables to children"),
    ("exports", "Declare exported values"),
    // Control flow
    ("if", "Conditional logic"),
    ("elif", "Else-if branch"),
    ("else", "Else branch"),
    ("when", "Conditional block based on variable state"),
    ("for", "Loop/iteration"),
    ("on", "Event handler"),
    // Registry
    ("registers", "Register in named registry"),
    ("resolves", "Map symbols to registry locations"),
    ("register", "Registry operation"),
    ("access", "Registry access operation"),
    ("namespace", "Namespace for registry"),
    // Visual & animation
    ("animates", "Specify animatable properties"),
    ("applies", "Apply properties or styles"),
    ("css", "Raw CSS block with interpolation"),
    ("transitions", "CSS transitions"),
    // Other
    ("preset", "Define preset values"),
    ("capture_type", "Define custom capture type"),
    ("cleanup", "Teardown/cleanup code"),
    ("creates", "Declare what the macro creates"),
    ("includes", "Include patterns from other definitions"),
    ("expands", "Expand variables or patterns"),
    ("template", "Reference a template variable"),
    ("method", "Define a method"),
    ("mutate", "Specify DOM mutations"),
    ("trigger", "Trigger a state transition"),
    ("scope", "Define where macro can be used"),
    ("iterates", "Specify iteration mappings"),
];

/// A friendly, human-readable error message
#[derive(Debug, Clone, Default)]
pub struct FriendlyError {
    /// Short summary (replaces the raw parse error)
    pub summary: String,
    /// Detailed explanation of what went wrong
    pub explanation: Option<String>,
    /// "Did you mean...?" suggestion
    pub suggestion: Option<String>,
    /// General help text or documentation reference
    pub help: Option<String>,
}

/// Transform a raw parse error into a friendly, human-readable message
pub fn make_error_friendly(raw_message: &str, source_line: Option<&str>) -> FriendlyError {
    // Remove the file path prefix if present (e.g., "stdlib/runtime/registries.st: ")
    let message = raw_message
        .split(": Parse error")
        .last()
        .unwrap_or(raw_message)
        .trim();

    // Extract the "found X expected Y" pattern
    let (found_char, _expected) = extract_found_expected(message);

    // Try to identify the problematic construct from source line
    if let Some(src) = source_line {
        // Pattern 1: % directive (valid or invalid)
        if let Some(directive) = extract_directive_from_source(src) {
            if is_valid_directive(&directive) {
                // Valid directive but parser errored - wrong context
                return make_wrong_context_directive_error(&directive);
            } else {
                // Invalid/unknown directive
                return make_unknown_directive_error(&directive);
            }
        }

        // Pattern 2: Unexpected $ variable
        if found_char == Some('$') {
            return make_unexpected_variable_error(src);
        }

        // Pattern 3: Unknown % directive at error position (line doesn't start with %)
        if let Some(c) = found_char
            && c.is_alphabetic()
        {
            // Could be an unrecognized keyword after %
            if src.trim().starts_with('%') {
                let trimmed = src.trim().trim_start_matches('%');
                let word: String = trimmed
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !word.is_empty() {
                    if is_valid_directive(&word) {
                        return make_wrong_context_directive_error(&word);
                    } else {
                        return make_unknown_directive_error(&word);
                    }
                }
            }
        }

        // Pattern 4: Unexpected punctuation
        if let Some(c) = found_char
            && !c.is_alphanumeric()
            && c != '$'
            && c != '%'
        {
            return make_unexpected_punctuation_error(c, src);
        }
    }

    // Pattern 5: Extract directive from error message pattern
    // "found 'g' expected 'c', 'r', or 'a'" after %g... suggests unknown directive
    if let Some(c) = found_char
        && c.is_alphabetic()
        && message.contains("expected")
    {
        return FriendlyError {
            summary: format!("Unexpected character '{}' in directive", c),
            explanation: Some(
                "The parser encountered an unexpected character while reading a meta directive"
                    .to_string(),
            ),
            suggestion: find_similar_directive_from_char(c),
            help: Some(
                "Valid directives start with %: %form, %primitive, %emit, %bind, etc.".to_string(),
            ),
        };
    }

    // Fallback: Clean up the raw message
    clean_raw_message(raw_message)
}

/// Extract "found 'X'" and "expected 'Y'" from error message
fn extract_found_expected(message: &str) -> (Option<char>, Option<String>) {
    let found = if let Some(start) = message.find("found '") {
        let after = &message[start + 7..];
        after.chars().next()
    } else {
        None
    };

    let expected = if let Some(start) = message.find("expected ") {
        let after = &message[start + 9..];
        Some(after.to_string())
    } else {
        None
    };

    (found, expected)
}

/// Extract a % directive name from source line
fn extract_directive_from_source(source: &str) -> Option<String> {
    let trimmed = source.trim();
    if let Some(after_percent) = trimmed.strip_prefix('%') {
        let word: String = after_percent
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !word.is_empty() {
            return Some(word);
        }
    }
    None
}

/// Check if a directive name is valid
fn is_valid_directive(name: &str) -> bool {
    VALID_DIRECTIVES.iter().any(|(d, _)| *d == name)
}

/// Create error for unknown directive with suggestions
fn make_unknown_directive_error(directive: &str) -> FriendlyError {
    let suggestion = find_similar_directive(directive);

    FriendlyError {
        summary: format!("Unrecognized meta directive '%{}'", directive),
        explanation: Some(format!(
            "'%{}' is not a valid Spacetime directive",
            directive
        )),
        suggestion,
        help: Some(
            "Common directives: %form, %primitive, %emit, %bind, %when, %for, %on".to_string(),
        ),
    }
}

/// Create error for valid directive in wrong context
fn make_wrong_context_directive_error(directive: &str) -> FriendlyError {
    // Provide context-specific hints based on directive
    let (explanation, suggestion): (String, String) = match directive {
        "when" => (
            "The '%when' directive is only valid inside %macro or %primitive bodies".to_string(),
            "Move %when inside a %macro { ... } or %primitive { ... } block".to_string(),
        ),
        "if" | "elif" | "else" => (
            format!(
                "The '%{}' directive is only valid inside %macro or %emit bodies",
                directive
            ),
            "Move this conditional inside a %macro { ... } or %emit { ... } block".to_string(),
        ),
        "for" => (
            "The '%for' directive is only valid inside %macro bodies".to_string(),
            "Move %for inside a %macro { ... } block".to_string(),
        ),
        "on" => (
            "The '%on' directive is only valid inside %macro or %primitive bodies".to_string(),
            "Move %on inside a %macro { ... } or %primitive { ... } block".to_string(),
        ),
        "emit" => (
            "The '%emit' directive is only valid inside %primitive or %macro bodies".to_string(),
            "Ensure %emit is inside a %primitive { %emit js { ... } } block".to_string(),
        ),
        "bind" | "binds" | "derives" | "provides" => (
            format!(
                "The '%{}' directive is only valid inside %primitive or %macro bodies",
                directive
            ),
            format!("Move %{} inside a definition block", directive),
        ),
        _ => (
            format!(
                "The '%{}' directive is not valid in this context",
                directive
            ),
            "Check that this directive is inside the correct parent block".to_string(),
        ),
    };

    FriendlyError {
        summary: format!("Directive '%{}' not valid in this context", directive),
        explanation: Some(explanation),
        suggestion: Some(suggestion),
        help: Some(
            "Directives must be nested correctly within %primitive or %macro definitions"
                .to_string(),
        ),
    }
}

/// Create error for unexpected $ variable
fn make_unexpected_variable_error(source: &str) -> FriendlyError {
    // Try to extract the variable name
    let var_name = if let Some(start) = source.find('$') {
        let after = &source[start..];
        let name: String = after
            .chars()
            .skip(1)
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();
        if !name.is_empty() {
            format!("${}", name)
        } else {
            "$".to_string()
        }
    } else {
        "$variable".to_string()
    };

    // Check if this is an arrow syntax issue (resolution mapping)
    if source.contains("->") {
        return FriendlyError {
            summary: format!("Arrow syntax '{}' not valid in this context", source.trim()),
            explanation: Some(
                "Resolution mappings with '->' are only valid inside %resolves blocks".to_string(),
            ),
            suggestion: Some(
                "Use '%resolves { $symbol -> ST.registry }' for symbol resolution".to_string(),
            ),
            help: None,
        };
    }

    FriendlyError {
        summary: format!("Unexpected variable '{}' in this context", var_name),
        explanation: Some("Variables with $ prefix are not valid at this position".to_string()),
        suggestion: Some(
            "Check if this should be inside %binds, %provides, %emit, or another block".to_string(),
        ),
        help: None,
    }
}

/// Create error for unexpected punctuation
fn make_unexpected_punctuation_error(punct: char, source: &str) -> FriendlyError {
    let (summary, explanation, suggestion) = match punct {
        '?' => (
            "Unexpected '?' syntax".to_string(),
            "The '?' for optional values is only valid in %form pattern definitions",
            Some("Remove the '?' or use it inside a %form { pattern? } block".to_string()),
        ),
        '{' => (
            "Object literal not allowed here".to_string(),
            "JSON-style object literals like { key: value } are not valid in this context",
            Some(
                "Use %emit js { ... } for JavaScript objects, or use Spacetime syntax".to_string(),
            ),
        ),
        '}' => (
            "Mismatched closing brace '}'".to_string(),
            "Found a closing brace without a matching opening brace",
            Some("Check that all '{' have matching '}' and blocks are properly nested".to_string()),
        ),
        '-' if source.contains("->") => (
            "Arrow syntax '->' not valid here".to_string(),
            "Resolution mappings with '->' are only valid inside %resolves blocks",
            Some("Use '%resolves { $symbol -> ST.registry }' for symbol resolution".to_string()),
        ),
        _ => (
            format!("Unexpected '{}' character", punct),
            "This punctuation is not recognized in Spacetime stdlib syntax",
            None,
        ),
    };

    FriendlyError {
        summary,
        explanation: Some(explanation.to_string()),
        suggestion,
        help: None,
    }
}

/// Find similar directive using Levenshtein distance
fn find_similar_directive(unknown: &str) -> Option<String> {
    let unknown_lower = unknown.to_lowercase();

    // Special case mappings for common mistakes
    let special_suggestions: &[(&str, &[&str])] = &[
        ("get", &["register", "access"]),  // %get -> %register or %access
        ("set", &["register", "binds"]),   // %set -> %register or %binds
        ("use", &["includes", "expands"]), // %use -> %includes or %expands
        ("let", &["binds", "derives"]),    // %let -> %binds or %derives
        ("var", &["binds", "provides"]),   // %var -> %binds or %provides
        ("def", &["primitive", "macro"]),  // %def -> %primitive or %macro
        ("define", &["primitive", "macro"]),
        ("func", &["primitive", "method"]), // %func -> %primitive or %method
        ("function", &["primitive", "method"]),
    ];

    // Check special cases first
    for (pattern, suggestions) in special_suggestions {
        if unknown_lower == *pattern {
            let formatted: Vec<String> = suggestions.iter().map(|s| format!("%{}", s)).collect();
            return Some(format!("Did you mean {}?", formatted.join(" or ")));
        }
    }

    // Use multi-suggestion with smart threshold
    let directive_names: Vec<&str> = VALID_DIRECTIVES.iter().map(|(d, _)| *d).collect();
    let matches = crate::diagnostics::find_similar_multiple(&unknown_lower, &directive_names, 3);
    crate::diagnostics::format_suggestion(&matches)
        .map(|s| s.replacen("did you mean", "Did you mean", 1))
}

/// Try to suggest a directive based on the first character
fn find_similar_directive_from_char(c: char) -> Option<String> {
    let matches: Vec<&str> = VALID_DIRECTIVES
        .iter()
        .filter(|(d, _)| d.starts_with(c.to_ascii_lowercase()))
        .map(|(d, _)| *d)
        .take(3)
        .collect();

    if matches.is_empty() {
        None
    } else {
        Some(format!("Did you mean %{}?", matches.join(", %")))
    }
}

/// Clean up raw message when no specific pattern matches
fn clean_raw_message(raw: &str) -> FriendlyError {
    // Remove byte offset pattern "at 123..456"
    let cleaned = regex::Regex::new(r"at \d+\.\.\d+:?\s*")
        .map(|re| re.replace_all(raw, ""))
        .unwrap_or_else(|_| std::borrow::Cow::Borrowed(raw));

    // Remove file path prefix if present
    let cleaned = if let Some(idx) = cleaned.find(": ") {
        if cleaned[..idx].contains(".st") {
            cleaned[idx + 2..].to_string()
        } else {
            cleaned.to_string()
        }
    } else {
        cleaned.to_string()
    };

    // Capitalize first letter
    let summary = if let Some(first) = cleaned.chars().next() {
        format!("{}{}", first.to_uppercase(), &cleaned[first.len_utf8()..])
    } else {
        cleaned
    };

    FriendlyError {
        summary,
        explanation: None,
        suggestion: None,
        help: Some("Check the Spacetime stdlib syntax for valid constructs".to_string()),
    }
}

// =============================================================================
// Main Pipeline Entry Point
// =============================================================================

// =============================================================================
// Template Reference Validation
// =============================================================================

/// BUG-105: scan a raw source fragment for `@template &name(…)` definitions and
/// add each name to `defined`. Used to admit templates defined inside `@mount`
/// blocks (which compile in a separate sub-program) so a same-block `&name()`
/// invocation is not a false E0402. Anchored on the `@template` keyword followed
/// by `&<ident>` so it does not match invocations (`&name(`) or refs.
fn collect_template_defs(text: &str, defined: &mut std::collections::HashSet<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(rel) = text[i..].find("@template") {
        let mut j = i + rel + "@template".len();
        // Require at least one whitespace after the keyword.
        let ws_start = j;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        // Then a `&` introducing the template name.
        if j > ws_start && j < bytes.len() && bytes[j] == b'&' {
            j += 1;
            let start = j;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-' || bytes[j] == b'_')
            {
                j += 1;
            }
            if j > start
                && let Ok(name) = std::str::from_utf8(&bytes[start..j])
                && !name.is_empty()
            {
                defined.insert(name.to_string());
            }
        }
        i += rel + "@template".len();
    }
}

/// Validate that all template invocations reference defined templates.
///
/// Scans resolved primitives for `register-template` definitions and
/// `invoke-template` invocations. Returns E0402 diagnostics for any
/// invocation that references a template not found in the defined set.
pub fn validate_template_refs(
    primitives: &[ResolvedPrimitive],
    scopes_by_name: &std::collections::HashMap<String, &crate::parser::ScopeBlock>,
) -> Vec<Diagnostic> {
    // Collect all defined template names
    let mut defined: HashSet<String> = primitives
        .iter()
        .filter(|p| p.primitive_name == "register-template")
        .filter_map(|p| match p.args.get("name") {
            // PLAN-150 W9: a `@form shot --name` registers its template under
            // the form name (`--name`) but is invoked as `&name` (collected
            // here without the sigil). Normalise the `--`/`&` prefix so the
            // form-registered template resolves the invocation.
            Some(CapturedValue::Ident(name) | CapturedValue::String(name)) => Some(
                name.trim_start_matches("--")
                    .trim_start_matches('&')
                    .to_string(),
            ),
            _ => None,
        })
        .collect();

    // BUG-105: a `@template &name(…)` defined INSIDE a `@mount { … }` block (a
    // `.test.st` harness) is compiled in a SEPARATE sub-program (test.st's
    // `%$content.js` re-parses + compiles the block), so its `register-template`
    // primitive never reaches this outer `primitives` list — a `&name()` invocation
    // in the same block then false-positives E0402. The block source survives as a
    // raw String/Expr capture (e.g. the `@test` body), so scan every such capture
    // for `@template &name` definitions and admit them to `defined`. (Mirrors the
    // `&name(` invocation scan below; same byte-walk, anchored on the `@template &`
    // def keyword.)
    for prim in primitives {
        for value in prim.args.values() {
            let text = match value {
                CapturedValue::Expr(s) | CapturedValue::String(s) => s.as_str(),
                _ => continue,
            };
            collect_template_defs(text, &mut defined);
        }
    }

    // Check template invocations in selector-scoped primitives
    let mut diagnostics = Vec::new();
    for prim in primitives
        .iter()
        .filter(|p| p.primitive_name == "invoke-template")
    {
        let name = match prim.args.get("name") {
            Some(CapturedValue::Ident(n) | CapturedValue::String(n)) => n,
            _ => continue,
        };

        if !defined.contains(name.as_str()) {
            let mut diag = Diagnostic::error(
                DiagnosticCode::E0402,
                format!("Template '{}' does not exist", name),
            )
            .with_span(prim.span.into());

            let defined_names: Vec<&str> = defined.iter().map(|s| s.as_str()).collect();
            let similar = find_similar_multiple(name, &defined_names, 3);
            if let Some(hint) = format_suggestion(&similar) {
                diag = diag.with_hint(hint);
            }
            diagnostics.push(diag);
        }
    }

    // Also scan register-template bodies for nested template invocations (&name(...)).
    // FEAT-119 (W4): the refs + html are read from the World-A template SCOPE keyed
    // `@template:<name>` (resolved via the prim's `name` arg), NOT the retired
    // ComponentBody capture (whose payload is gone).
    for prim in primitives
        .iter()
        .filter(|p| p.primitive_name == "register-template")
    {
        // Resolve this register-template's World-A scope by its template name.
        let tmpl_scope = prim
            .args
            .get("name")
            .and_then(|v| match v {
                CapturedValue::Ident(n) | CapturedValue::Element(n) => {
                    Some(n.strip_prefix('&').unwrap_or(n).to_string())
                }
                _ => None,
            })
            .and_then(|n| scopes_by_name.get(&n).copied());

        // Structured path: typed nested refs surfaced on the scope (World A).
        if let Some(scope) = tmpl_scope {
            for tref in &scope.refs {
                let name = &tref.template_name;
                // Dynamic dispatch `&$expr(...)`: the name is a runtime value, not a
                // static template name — cannot validate existence at compile time
                // (resolved + warned at runtime). Skip the E0402 check (FEAT-073).
                if name.starts_with('$') {
                    continue;
                }
                if !name.is_empty() && !defined.contains(name) {
                    let defined_names: Vec<&str> = defined.iter().map(|s| s.as_str()).collect();
                    let similar = find_similar_multiple(name, &defined_names, 3);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::E0402,
                        format!("Template '{}' does not exist", name),
                    )
                    .with_span(prim.span.into());
                    if let Some(h) = format_suggestion(&similar) {
                        diag = diag.with_hint(h);
                    }
                    diagnostics.push(diag);
                }
            }
        }
        // Scan all string/expr captures + the scope html for &name( patterns. The
        // scope html is the World-A source; non-body Expr/String captures are scanned
        // verbatim (unchanged).
        let scope_html = tmpl_scope.map(|s| s.html.as_str()).unwrap_or("");
        let texts = prim
            .args
            .values()
            .filter_map(|value| match value {
                CapturedValue::Expr(s) | CapturedValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .chain(std::iter::once(scope_html));
        for body_text in texts {
            // Scan for &name( template invocations
            let bytes = body_text.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'&' {
                    i += 1;
                    let start = i;
                    while i < bytes.len()
                        && (bytes[i].is_ascii_alphanumeric()
                            || bytes[i] == b'-'
                            || bytes[i] == b'_')
                    {
                        i += 1;
                    }
                    if i > start {
                        let name_end = i;
                        // Skip whitespace
                        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                            i += 1;
                        }
                        if i < bytes.len() && bytes[i] == b'(' {
                            let name = std::str::from_utf8(&bytes[start..name_end])
                                .unwrap_or("")
                                .to_string();
                            if !name.is_empty() && !defined.contains(&name) {
                                let defined_names: Vec<&str> =
                                    defined.iter().map(|s| s.as_str()).collect();
                                let similar = find_similar_multiple(&name, &defined_names, 3);
                                let hint = format_suggestion(&similar);
                                let mut diag = Diagnostic::error(
                                    DiagnosticCode::E0402,
                                    format!("Template '{}' does not exist", name),
                                )
                                .with_span(prim.span.into());
                                if let Some(h) = hint {
                                    diag = diag.with_hint(h);
                                }
                                diagnostics.push(diag);
                            }
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }
    }

    diagnostics
}

/// Compile FormMatches through the full pipeline
///
/// This is the main entry point that orchestrates all layers:
/// 1. Resolve: FormMatch → ResolvedPrimitive
/// 2. Sort: Order by (phase, order)
/// 3. Expand: ResolvedPrimitive → ExpandedPrimitive
/// 4. Emit: ExpandedPrimitive → PipelineOutput
///
/// # Example
///
/// ```
/// Generate CSS and JS fragments for `%states` clauses found in evaluated matches.
///
/// For each state with a condition, generates:
/// - CSS: `selector[data-st-state="name"] { prop: val; }` (or pattern-match variant)
/// - JS: `ST.bindState(el, 'condition', 'state_name')` (or `ST.watchTypedUnion` for patterns)
fn generate_state_fragments(
    evaluated: &[evaluate::EvaluatedMatch],
    js_frags: &mut Vec<JsFragment>,
    css_frags: &mut Vec<CssFragment>,
) {
    for ev in evaluated {
        let states_clause = match &ev.states {
            Some(s) => s,
            None => continue,
        };
        let selector = match &ev.form_match.selector {
            Some(s) => s.as_str(),
            None => continue,
        };

        let mut js_stmts = Vec::new();

        for state_def in &states_clause.states {
            let (name, condition, properties) = match state_def {
                MetaStateDef::Named {
                    name,
                    condition,
                    properties,
                } => (name.as_str(), condition.as_deref(), properties),
                MetaStateDef::Variable { .. } => continue,
            };

            if properties.is_empty() && condition.is_none() {
                continue;
            }

            let props: Vec<(String, String)> = properties
                .iter()
                .map(|p| (p.name.clone(), p.value.clone()))
                .collect();

            if let Some(cond) = condition {
                if let Some(pattern) = parse_pattern_match(cond) {
                    // Pattern match state: $var:Variant{bindings}
                    js_stmts.push(JsStmt::Raw(generate_pattern_match_js(&pattern)));
                    if !props.is_empty() {
                        css_frags.push(CssFragment::new(vec![CssExpr::Raw(
                            generate_pattern_match_css(selector, &pattern, &props),
                        )]));
                    }
                } else {
                    // Signal-based state: ST.bindState(el, 'condition', 'name')
                    js_stmts.push(JsStmt::Raw(format!(
                        "ST.bindState(el, '{}', '{}');\n",
                        cond, name
                    )));
                    if !props.is_empty() {
                        css_frags.push(CssFragment::new(vec![CssExpr::Raw(generate_state_css(
                            selector, name, &props,
                        ))]));
                    }
                }
            }
        }

        if !js_stmts.is_empty() {
            js_frags.push(JsFragment {
                stmts: js_stmts,
                cleanup: Vec::new(),
                scope: JsScope::IIFE,
                el_init: Some(ElInit::Selector(selector.to_string())),
                source: SourceSpan::default(),
            });
        }
    }
}

/// Generate JS fragments for `%on $var -> value` transition watchers (PLAN-053).
///
/// For each evaluated `%on` clause, emits a selector-scoped fragment that calls
/// `ST.onTransition(el, var, value, ...)`; on the transition edge it runs the
/// clause's action statements through `ST.runMutations` (the shared `@on` /
/// `@effect` / `@handle` mutation rail). This is how `@drag { on-drop: ... }`
/// fires its drop action when the gesture's `$active` goes true -> false.
///
/// PLAN-077 W4: action statements pass through the variant-construction
/// lowering (`$x <- Connected($send)`); the E0933/E0934 mutation-leg
/// diagnostics are returned for the caller to drain.
fn generate_on_fragments(
    evaluated: &[evaluate::EvaluatedMatch],
    js_frags: &mut Vec<JsFragment>,
    facts: &EnumTypeFacts,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for ev in evaluated {
        if ev.on_clauses.is_empty() {
            continue;
        }
        let selector = match &ev.form_match.selector {
            Some(s) => s.as_str(),
            None => continue,
        };

        let mut js_stmts = Vec::new();
        for on in &ev.on_clauses {
            // The action statements run through ST.runMutations against the
            // element (`el`), so `$` inside an action resolves to the dragged
            // node and page `$signals` resolve via the shared rail.
            let actions: Vec<String> = on
                .actions
                .iter()
                .map(|a| {
                    lower_variant_mutation_stmt(
                        a,
                        facts,
                        ev.form_match.span,
                        None,
                        &mut diagnostics,
                    )
                })
                .collect();
            let actions_json = serde_json::to_string(&actions).unwrap_or_else(|_| "[]".to_string());
            let var_json = serde_json::to_string(&on.var).unwrap_or_else(|_| "\"\"".to_string());
            let value_json =
                serde_json::to_string(&on.value).unwrap_or_else(|_| "\"\"".to_string());
            js_stmts.push(JsStmt::Raw(format!(
                "ST.onTransition(el, {var}, {val}, function() {{ ST.runMutations(el, {{ js_statements: {acts} }}); }});\n",
                var = var_json,
                val = value_json,
                acts = actions_json,
            )));
        }

        if !js_stmts.is_empty() {
            js_frags.push(JsFragment {
                stmts: js_stmts,
                cleanup: Vec::new(),
                scope: JsScope::IIFE,
                el_init: Some(ElInit::Selector(selector.to_string())),
                source: SourceSpan::default(),
            });
        }
    }
    diagnostics
}

/// Map a `%animates` CSS property to a JS transform-token expression template
/// (PLAN-054). `{v}` is replaced with the channel's live value. Returns None for
/// a non-transform property (handled via ST.bindStyle instead).
fn transform_token_template(property: &str) -> Option<&'static str> {
    match property {
        "translate-x" | "translateX" => Some("'translateX(' + v + 'px)'"),
        "translate-y" | "translateY" => Some("'translateY(' + v + 'px)'"),
        "translate-z" | "translateZ" => Some("'translateZ(' + v + 'px)'"),
        "scale" => Some("'scale(' + v + ')'"),
        "scale-x" | "scaleX" => Some("'scaleX(' + v + ')'"),
        "scale-y" | "scaleY" => Some("'scaleY(' + v + ')'"),
        "rotate" => Some("'rotate(' + v + 'deg)'"),
        "rotate-x" | "rotateX" => Some("'rotateX(' + v + 'deg)'"),
        "rotate-y" | "rotateY" => Some("'rotateY(' + v + 'deg)'"),
        _ => None,
    }
}

/// Generate JS fragments for `%animates` + the `%derives` that feed it (PLAN-054).
///
/// For each evaluated match with animate channels, emits a selector-scoped
/// fragment that:
///   1. registers each lowered derive as a reactive computed signal
///      (`ST.derive(el, name, [deps], (v) => compute)`), deps-first, so the
///      animated signals track the gesture live; and
///   2. binds the transform channels with one `ST.bindTransform(el, [...])` so
///      translate-x/y/scale/rotate compose into a single `transform`. A
///      non-transform animated property (e.g. opacity) falls back to
///      `ST.bindStyle`.
/// This is what makes a `@drag` element VISUALLY FOLLOW the cursor.
fn generate_animate_fragments(
    evaluated: &[evaluate::EvaluatedMatch],
    js_frags: &mut Vec<JsFragment>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for ev in evaluated {
        if ev.animates.is_empty() {
            continue;
        }
        let selector = match &ev.form_match.selector {
            Some(s) => s.as_str(),
            None => continue,
        };

        let mut js_stmts = Vec::new();

        // 1. Reactive derives (deps-first order, as collected).
        for d in &ev.derives {
            let deps_json = serde_json::to_string(&d.deps).unwrap_or_else(|_| "[]".to_string());
            js_stmts.push(JsStmt::Raw(format!(
                "ST.derive(el, {name}, {deps}, function(v) {{ return ({compute}); }});\n",
                name = serde_json::to_string(&d.name).unwrap_or_else(|_| "\"\"".to_string()),
                deps = deps_json,
                compute = d.compute_js,
            )));
        }

        // 2. Transform channels composed into one binding; a non-transform but
        //    animatable property binds individually via ST.bindStyle. A property
        //    that is NEITHER a transform channel nor a known animatable CSS
        //    property is a likely typo/unsupported channel — warn (W0713) and skip
        //    it rather than emitting a raw `el.style[<prop>]` write that produces
        //    silently-broken CSS (PLAN-054 audit #6).
        let mut channels: Vec<String> = Vec::new();
        for anim in &ev.animates {
            let sig_json =
                serde_json::to_string(&anim.signal).unwrap_or_else(|_| "\"\"".to_string());
            if let Some(tpl) = transform_token_template(&anim.property) {
                channels.push(format!(
                    "{{ signal: {sig}, fn: function(v) {{ return {tpl}; }} }}",
                    sig = sig_json,
                    tpl = tpl,
                ));
            } else if is_animatable_style_property(&anim.property) {
                // Non-transform but a real animatable CSS property: bind directly.
                js_stmts.push(JsStmt::Raw(format!(
                    "ST.bindStyle(el, {sig}, {prop});\n",
                    sig = sig_json,
                    prop = serde_json::to_string(&anim.property)
                        .unwrap_or_else(|_| "\"\"".to_string()),
                )));
            } else {
                diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::W0713,
                        format!(
                            "%animates property `{}` is not a known transform channel \
                             (translate-x/-y/-z, scale[-x/-y], rotate[-x/-y]) nor a \
                             recognized animatable CSS property; it is ignored",
                            anim.property
                        ),
                    )
                    .with_span(ev.form_match.span.into())
                    .with_hint(
                        "Use a transform channel (e.g. `translate-x: $x`) or an \
                         animatable CSS property (e.g. `opacity`, `width`)."
                            .to_string(),
                    ),
                );
            }
        }
        if !channels.is_empty() {
            js_stmts.push(JsStmt::Raw(format!(
                "ST.bindTransform(el, [{}]);\n",
                channels.join(", ")
            )));
        }

        if !js_stmts.is_empty() {
            js_frags.push(JsFragment {
                stmts: js_stmts,
                cleanup: Vec::new(),
                scope: JsScope::IIFE,
                el_init: Some(ElInit::Selector(selector.to_string())),
                source: SourceSpan::default(),
            });
        }
    }
    diagnostics
}

/// Whether a `%animates` property that is NOT a transform channel is still a
/// recognized animatable CSS property safe to drive via `ST.bindStyle` (a raw
/// `el.style[prop] = <number>` write). Conservative allowlist of numeric/length
/// CSS properties commonly animated; anything else warns (W0713) (PLAN-054).
fn is_animatable_style_property(property: &str) -> bool {
    matches!(
        property,
        "opacity"
            | "width"
            | "height"
            | "top"
            | "left"
            | "right"
            | "bottom"
            | "z-index"
            | "order"
            | "flex-grow"
            | "flex-shrink" // CSS custom properties are author-defined channels — always allowed.
    ) || property.starts_with("--")
}

/// use spacetime::pipeline::{compile, CompileContext};
/// use spacetime::metasystem::MetaRegistry;
///
/// let context = CompileContext::new(MetaRegistry::new());
/// let matches = vec![/* parsed form matches */];
///
/// let output = compile(&matches, &context).unwrap();
/// println!("Generated JS: {}", output.js);
/// ```
pub fn compile(
    matches: &[FormMatch],
    context: &CompileContext,
) -> Result<PipelineOutput, PipelineError> {
    compile_verbose(matches, context).map(|v| v.output)
}

/// Compile with detailed layer outputs for debugging
///
/// Returns intermediate results from each layer for inspection.
pub fn compile_verbose(
    matches: &[FormMatch],
    context: &CompileContext,
) -> Result<VerboseOutput, PipelineError> {
    // PLAN-077 W3+W4: shared enum/sum facts — built ONCE per compile and
    // shared by the W4 mutation lowering (immediately below) and the W3
    // diagnostics (near the end of this fn).
    let enum_facts = EnumTypeFacts::from_matches(matches);

    // Layer -0.5 (PLAN-077 W4): variant-construction lowering on `<-`
    // mutation RHS (`$x <- Connected($send)` → `{type: 'Connected', send:
    // $send}`) — strictly additive, gated on a positive TypeRegistry match.
    // Runs BEFORE evaluate so the lowered statements flow through the normal
    // mutation rail; E0933/E0934 mutation-leg diagnostics ride along.
    let mutation_owned;
    let (mutation_rewritten, mutation_diagnostics) = lower_variant_mutations(matches, &enum_facts);
    let matches: &[FormMatch] = match mutation_rewritten {
        Some(v) => {
            mutation_owned = v;
            &mutation_owned
        }
        None => matches,
    };

    // Layer -1: Canonicalize namespace qualifiers against the per-file import
    // scope (FEAT-118 FUP-057). A qualified call `@s/camera` (alias) or
    // `@scene/camera` (path-suffix) carries a raw qualifier that does NOT match
    // the registry's collection-prefixed key (`std:scene/camera`). Rewriting the
    // qualifier to the resolved namespace's canonical key lets the existing
    // FQN-first resolution in `resolve.rs` hit without any scope awareness of its
    // own. A no-op (clone-free borrow) when the file has no namespaced imports.
    // The ORIGINAL matches keep their raw qualifiers (`s`, `scene`) — the import
    // validation pass (E0926/E0927) reads them to check binding + visibility
    // BEFORE they are rewritten away.
    let raw_matches = matches;
    let canonical_owned;
    let matches: &[FormMatch] = if context.import_scope.is_empty() {
        matches
    } else {
        canonical_owned =
            canonicalize_qualifiers(matches, &context.import_scope, &context.meta_registry);
        &canonical_owned
    };

    // Layer 0: Evaluate — flatten control flow in macro body items
    let mut evaluated = evaluate::evaluate(matches, &context.meta_registry)?;

    // PLAN-124 W3.2: expand `drive(...)` binds (@on's driver-projection slot)
    // into the real driver primitive (registry dispatch table) +
    // apply-animations (resolved motion form). Runs between Evaluate and
    // Resolve: Evaluate flattens the %binds, this rewrites them, Resolve sees
    // only real primitives.
    let drive_diagnostics =
        drivers::expand_drive_binds(&mut evaluated, matches, &context.meta_registry);

    // SIP-001 W4/R1: a `@score` placement must be written in ITS DRIVER'S time
    // domain (`at 2s` under `&.scroll` is meaningless — scroll progress is
    // 0..1). Reads the `domain:` registry column, so a new score-capable driver
    // is covered by declaring one; there is no rule per driver here.
    // Runs alongside the drive expansion because both answer the same question
    // — what does this projection mean — from the same registry.
    let mut score_diagnostics = score::check_score_domains(&evaluated, &context.meta_registry);

    // W4/R3: lower each `score(…)` bind into one `score-window` per clip — a
    // PURE derivation of the score driver's progress signal, never a
    // scheduler. Runs after the domain check so a score whose driver cannot
    // drive one is reported once, not lowered and reported twice.
    score_diagnostics.extend(score::expand_score_binds(
        &mut evaluated,
        matches,
        &context.meta_registry,
    ));

    // A retired-syntax match must not reach Resolve. A PLAN-079 capsule
    // registers one MATCH-ONLY def per `%rewrite` rule, keyed `<id>#<rule>`
    // (`on-cutover#scroll`): it exists to be MATCHED and consumed by the
    // compile-time shim, so it deliberately has no `%emit`/`%binds`. When the
    // shim could not consume it (an imported file, a source-less `from_ast`
    // compile, or a shape the rule can't soundly rewrite), the survivor is
    // exactly what `check_retired_syntax` reports as E0910/E0911 further down.
    //
    // But Resolve runs FIRST, and a def with no `%emit` looks to it like an
    // unknown primitive — so the build died on `E0956: unknown primitive:
    // on-cutover#scroll`, masking the migration diagnostic that actually
    // explains the page. This is the same failure `binds_consumed` already
    // guards for score binds; a retired match earns the same mark, and the
    // tri-branch stays the single owner of what retired syntax reports.
    for ev in &mut evaluated {
        let def_name = ev
            .form_match
            .matched_macro
            .as_deref()
            .unwrap_or(&ev.form_match.macro_name);
        // `Some(rule)` = a rewrite-rule def. An embedded retired macro
        // (`None`) still has real `%binds` and MUST keep expanding — that is
        // the W0715 migration window that lets un-migrated code run.
        if let Some((_, Some(_))) = context.meta_registry.retired_by_macro(def_name) {
            ev.binds_consumed = true;
        }
    }

    // Layer 2: Resolve
    let mut primitives = resolve::resolve_evaluated(&evaluated, &context.meta_registry)?;

    // PLAN-112 W4: only dev-server compiles carry this map. Its ids come from the
    // same bundle walk as `/__spacetime/inspect/structure`; production maps are
    // empty, so build output cannot acquire inspector DOM attributes.
    //
    // W4R REVIEW FINDING (P1): the key is a BYTE SPAN, but `resolve_imports`
    // flat-merges files WITHOUT rebasing spans (src/parser/mod.rs) — provenance
    // lives in a separate `source_file`, which a `ResolvedPrimitive` does not
    // carry. So two invocations in DIFFERENT files can share a span, and taking
    // the first match would stamp one invocation with the OTHER's id: a click
    // would then select, and an edit would rewrite, the wrong source.
    //
    // A stamp is an ADDRESS, so ambiguity must never resolve to a guess. Spans
    // claimed by more than one node are dropped from the map entirely: those
    // invocations simply are not clickable (the tree still edits them), while
    // every stamp that IS emitted is unambiguous.
    let mut ambiguous: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut unique: std::collections::HashMap<(usize, usize), &str> =
        std::collections::HashMap::new();
    for (span, node_id) in &context.invocation_node_ids {
        if unique.insert(*span, node_id.as_str()).is_some() {
            ambiguous.insert(*span);
        }
    }
    for primitive in &mut primitives {
        if primitive.primitive_name == "invoke-template" {
            let span = (primitive.span.start, primitive.span.end);
            if !ambiguous.contains(&span)
                && let Some(node_id) = unique.get(&span)
            {
                primitive.args.insert(
                    "node".to_string(),
                    CapturedValue::String((*node_id).to_string()),
                );
            }
        }
    }

    // FEAT-119 (W3/W4): name→scope map for the World-A factory payload. Built once
    // here and shared by E0402 ref validation and the expand body arm — both read the
    // template payload (refs/html) from the `@template:<name>` Construct scope, not the
    // retired ComponentBody capture.
    let scopes_by_name: std::collections::HashMap<String, &crate::parser::ScopeBlock> = context
        .scopes
        .iter()
        .filter_map(|s| {
            s.selector
                .strip_prefix("@template:")
                .map(|name| (name.to_string(), s))
        })
        .collect();

    // Validate template references (E0402)
    let template_diagnostics = validate_template_refs(&primitives, &scopes_by_name);

    // Layer 3: Sort
    let sorted = sort(primitives);

    // Layer 4: Expand (typed). Reuses `scopes_by_name` built above (World-A payload).
    let expanded = expand_typed(&sorted, &context.meta_registry, &scopes_by_name)?;

    // Collect typed fragments
    let mut js_frags: Vec<_> = expanded
        .iter()
        .filter_map(|e| e.js.as_ref().cloned())
        .collect();
    let mut css_frags: Vec<_> = expanded
        .iter()
        .filter_map(|e| e.css.as_ref().cloned())
        .collect();
    let build_scripts: Vec<String> = expanded
        .iter()
        .flat_map(|e| e.build_scripts.iter().cloned())
        .collect();

    // Generate CSS+JS fragments for %states clauses
    generate_state_fragments(&evaluated, &mut js_frags, &mut css_frags);

    // Generate JS fragments for %on $var -> value transition watchers (PLAN-053)
    let on_mutation_diagnostics = generate_on_fragments(&evaluated, &mut js_frags, &enum_facts);

    // Generate JS fragments for %animates + reactive %derives (PLAN-054).
    // Returns W0713 warnings for unknown animate properties (audit #6).
    let animate_diagnostics = generate_animate_fragments(&evaluated, &mut js_frags);

    // Collect prelude fragments, deduplicated by primitive name.
    // First occurrence wins — subsequent invocations of the same primitive are skipped.
    let mut seen_preludes = std::collections::HashSet::new();
    let mut prelude_js_frags: Vec<JsFragment> = Vec::new();
    let mut prelude_css_frags: Vec<CssFragment> = Vec::new();
    for ep in &expanded {
        if (ep.prelude_js.is_some() || ep.prelude_css.is_some())
            && seen_preludes.insert(ep.primitive_name.clone())
        {
            if let Some(ref pj) = ep.prelude_js {
                prelude_js_frags.push(pj.clone());
            }
            if let Some(ref pc) = ep.prelude_css {
                prelude_css_frags.push(pc.clone());
            }
        }
    }

    // Layer 5: Emit (typed, scope-aware)
    let mut output = if context.include_runtime {
        emit_typed_with_runtime(
            &prelude_js_frags,
            &js_frags,
            &prelude_css_frags,
            &css_frags,
            true,
        )
    } else {
        emit_typed(&prelude_js_frags, &js_frags, &prelude_css_frags, &css_frags)
    };

    // Attach build-time scripts from expanded primitives
    output.build_scripts = build_scripts;

    // Merge template validation diagnostics into output
    output.diagnostics.extend(template_diagnostics);

    // Merge %animates unknown-property warnings (W0713, PLAN-054 audit #6)
    output.diagnostics.extend(animate_diagnostics);

    // FEAT-120: drain parse-time body-validation diagnostics (E0900/E0904/E0906/W0700)
    // produced in `cst_to_stfile`. ONE extend — the canonical channel replacing the
    // retired `ComponentBodyDef` capture envelope and its two per-code bridges.
    output
        .diagnostics
        .extend(context.body_diagnostics.iter().cloned());

    // Retired-syntax check (PLAN-076): any FormMatch claimed by a stdlib
    // %migration that SURVIVED to the pipeline is an error — hint-kind by
    // design; rewrite-kind means the shim couldn't apply it (multi-shape
    // call, imported file, or a source-less `from_ast` compile).
    let retired_syntax_diagnostics = check_retired_syntax(matches, &context);
    output.diagnostics.extend(retired_syntax_diagnostics);

    // Enum/union checks (PLAN-077 W3): E0931 cond-mode totality, E0932
    // dispatch exhaustiveness, E0933 unknown variant, E0934 payload arity.
    // Reads the FormMatch surface (derive-match arms, dispatch @match/@view,
    // state-match forms, @data type declarations, @type sums). Every check
    // skips (never guesses) when the subject's type is not a known sum.
    let enum_union_diagnostics = check_enum_union_diagnostics_with_facts(matches, &enum_facts);
    output.diagnostics.extend(enum_union_diagnostics);
    output.diagnostics.extend(drive_diagnostics);
    output.diagnostics.extend(score_diagnostics);

    // PLAN-077 W4: E0933/E0934 mutation-leg diagnostics from the variant-
    // construction lowering (unknown variant / payload-arity on a `<-` RHS).
    output.diagnostics.extend(mutation_diagnostics);
    output.diagnostics.extend(on_mutation_diagnostics);

    // Check for undefined $var references in template scopes (E0905)
    let undefined_ref_diagnostics = check_undefined_state_refs(matches, &context.scopes);
    output.diagnostics.extend(undefined_ref_diagnostics);

    // BUG-333: an element reference `&$name` with no `&name <sel>;` declaration
    // must be a compile error (E0965), not a dead bundle. Scoped to page
    // css_declarations; a macro's `&$bounds` Element capture (e.g. @drag) lives
    // in `%derives` and never reaches this pass.
    let undeclared_elem_ref_diagnostics = check_undeclared_element_refs(&context.scopes);
    output.diagnostics.extend(undeclared_elem_ref_diagnostics);

    // Check for collection refs outside @each (E0918) and duplicate refs (E0919)
    let ref_diagnostics = check_template_ref_issues(matches, &context.scopes);
    output.diagnostics.extend(ref_diagnostics);

    // Check for <- injection outside template context (E0909)
    let injection_context_diagnostics = check_injection_outside_template(matches);
    output.diagnostics.extend(injection_context_diagnostics);

    // Check for ambiguous namespace dispatch (E0924, FEAT-118 FUP-055)
    let namespace_ambiguity_diagnostics =
        check_namespace_ambiguity(matches, &context.meta_registry);
    output.diagnostics.extend(namespace_ambiguity_diagnostics);

    // Check import binding + visibility (E0926 unbound qualifier, E0927
    // only/hiding violation, FEAT-118 FUP-057). Reads the RAW (pre-canonical)
    // qualifiers against the per-file import scope. No-op when the file has no
    // namespaced imports.
    if !context.import_scope.is_empty() {
        let import_diagnostics =
            check_import_visibility(raw_matches, &context.import_scope, &context.meta_registry);
        output.diagnostics.extend(import_diagnostics);
    }

    // Check for @exports declaring undefined state variables (E0917)
    let export_ref_diagnostics = check_export_refs(matches, &context.scopes);
    output.diagnostics.extend(export_ref_diagnostics);

    // Check for interleaved HTML/CSS in component bodies (W0700)
    // FEAT-120: W0700 (interleaved HTML/CSS) is now emitted by `validate_component_body`
    // at parse time (it has the section-transition count + template name locally) and
    // lands in `StFile.diagnostics`. The pipeline drain is retired.

    // Check for $var shadowing parent scope (W0702)
    let shadowing_diagnostics = check_scope_shadowing(matches, &context.scopes);
    output.diagnostics.extend(shadowing_diagnostics);

    // Check for unused state variables in templates (W0703)
    let unused_state_diagnostics = check_unused_state_vars(matches, &context.scopes);
    output.diagnostics.extend(unused_state_diagnostics);

    // Check for mixing @bind with reactive properties (W0706)
    let mixed_bind_diagnostics = check_mixed_bind_reactive(matches, &context.scopes);
    output.diagnostics.extend(mixed_bind_diagnostics);

    // FEAT-082: collect userland `%emit html` fragments (in expansion order) so
    // the compiler can splice them into the page body.
    output.html = expanded
        .iter()
        .flat_map(|ep| ep.html.iter().cloned())
        .collect();

    Ok(VerboseOutput {
        primitives: sorted.clone(),
        expanded,
        output,
    })
}

/// Retired-syntax tri-branch (PLAN-079 capsules): for every FormMatch
/// claimed by a stdlib `%migration` —
///
/// - wave AT/BEFORE the project's `@version` → E0911 (version inconsistency,
///   the project claims it already crossed that wave);
/// - a rewrite-RULE def survived to the pipeline → E0910 (the compile-time
///   shim should have consumed it: a multi-shape call the rule would
///   silently truncate, an imported file, or a source-less `from_ast`
///   compile);
/// - an embedded retired macro matched and the directive HAS rules that
///   didn't cover this shape → E0910 (no rule covers it; hint attached);
/// - an embedded retired macro matched a HINT-covered directive
///   (`@show`/`@input`) → W0715 and the match COMPILES THROUGH: the
///   embedded macro's `%binds` still expand (the window), with the hint
///   coming from the registry ENTRY, not a Rust match-arm.
///
/// Also owns the @version fact's own integrity (duplicate decls, non-ISO
/// date, multi-file) — E0911, enforced on every compile path (from_file AND
/// from_ast, so `check` and `serve`/`build` agree).

// =============================================================================
// Enum/union diagnostics (PLAN-077 W3)
// =============================================================================

/// A signal's declared sum universe: either a named `@type` sum (resolved via
/// TypeRegistry) or an inline anonymous union's literal variant list.
enum SumUniverse {
    /// Name of a `@type` sum in the TypeRegistry.
    Named(String),
    /// Variants of an inline anonymous union `(A | B | …)` (all nullary).
    Inline(Vec<String>),
}

/// Strip the `$` sigil from a Binding/Ident capture, yielding the signal name.
fn signal_name_of(v: &CapturedValue) -> Option<String> {
    match v {
        CapturedValue::Binding(s) | CapturedValue::Ident(s) => {
            Some(s.strip_prefix('$').unwrap_or(s).to_string())
        }
        _ => None,
    }
}

/// True for the `_` catch-all guard — either the bare `_` string branch or a
/// parenthesized `(_)` (guardText semantics in derive-match's %emit js).
fn cond_guard_is_catch_all(guard: &CapturedValue) -> bool {
    match guard {
        CapturedValue::String(s) => s.trim() == "_",
        CapturedValue::Named(m) => {
            matches!(m.get("expr"), Some(CapturedValue::Expr(e)) if e.trim() == "_")
        }
        _ => false,
    }
}

/// The `guard` capture of one cond-mode arm record.
fn cond_arm_guard(a: &CapturedValue) -> Option<&CapturedValue> {
    match a {
        CapturedValue::Named(m) => m.get("guard"),
        _ => None,
    }
}

/// The cond_block `{ arms: [...] }` array from a data-derive-match capture.
fn cond_arms_of<'a>(fm: &'a FormMatch) -> Option<&'a [CapturedValue]> {
    match fm.captures.get("match") {
        Some(CapturedValue::Named(block)) => match block.get("arms") {
            Some(CapturedValue::Array(arms)) => Some(arms),
            _ => None,
        },
        _ => None,
    }
}

/// Variant list of an inline-union `uitype` capture `{head, tail: [{v}]}`.
fn inline_union_variants(v: &CapturedValue) -> Option<Vec<String>> {
    let CapturedValue::Named(m) = v else {
        return None;
    };
    let CapturedValue::Ident(head) = m.get("head")? else {
        return None;
    };
    let mut out = vec![head.clone()];
    if let Some(CapturedValue::Array(tail)) = m.get("tail") {
        for alt in tail {
            if let CapturedValue::Named(a) = alt
                && let Some(CapturedValue::Ident(v)) = a.get("v")
            {
                out.push(v.clone());
            }
        }
    }
    Some(out)
}

/// Declared sum universe for a signal: its `@data` type capture naming a
/// `@type` sum, or its inline-union literal. None = uncheckable (skip, never
/// guess — Gate-3 criterion a).
fn sum_universe_of(
    signal: &str,
    signal_types: &HashMap<String, String>,
    inline_unions: &HashMap<String, Vec<String>>,
    types: &crate::type_system::TypeRegistry,
) -> Option<SumUniverse> {
    if let Some(variants) = inline_unions.get(signal) {
        return Some(SumUniverse::Inline(variants.clone()));
    }
    let type_name = signal_types.get(signal)?;
    let def = types.get_type(type_name)?;
    if !def.is_sum() {
        return None;
    }
    Some(SumUniverse::Named(type_name.clone()))
}

/// Variant names of a universe (for membership/exhaustiveness checks).
fn universe_variants<'a>(
    universe: &'a SumUniverse,
    types: &'a crate::type_system::TypeRegistry,
) -> Vec<&'a str> {
    match universe {
        SumUniverse::Named(t) => types.variant_names(t),
        SumUniverse::Inline(vs) => vs.iter().map(|s| s.as_str()).collect(),
    }
}

/// Payload arity of one variant of a universe (None = variant unknown —
/// E0933's concern, not E0934's).
fn universe_payload_arity(
    universe: &SumUniverse,
    types: &crate::type_system::TypeRegistry,
    variant: &str,
) -> Option<usize> {
    match universe {
        SumUniverse::Named(t) => types.variant_payload_arity(t, variant),
        // Inline-union variants are nullary by grammar (idents only).
        SumUniverse::Inline(vs) => {
            if vs.iter().any(|v| v == variant) {
                Some(0)
            } else {
                None
            }
        }
    }
}

/// Display name for a universe in diagnostics: `` `@type BadgeState` `` for a
/// named sum, "the inline union `(Blue | Green)`" for an anonymous literal —
/// an inline union has NO `@type` declaration, so naming one would point the
/// author at a construct that doesn't exist (SWARM GATE 3 finding).
fn universe_display(universe: &SumUniverse) -> String {
    match universe {
        SumUniverse::Named(t) => format!("`@type {}`", t),
        SumUniverse::Inline(vs) => format!("the inline union `({})`", vs.join(" | ")),
    }
}

/// The E0933 unknown-variant diagnostic for one usage site. `site` names the
/// construct, e.g. "in `@data derive $x`" / "in `@match $badge`".
fn unknown_variant_diag(
    span: crate::parser::SourceSpan,
    variant: &str,
    universe: &SumUniverse,
    types: &crate::type_system::TypeRegistry,
    site: &str,
) -> Diagnostic {
    let variants = universe_variants(universe, types);
    Diagnostic::error(
        DiagnosticCode::E0933,
        format!(
            "`{}` is not a variant of {} ({})",
            variant,
            universe_display(universe),
            site
        ),
    )
    .with_span(span.into())
    .with_hint(format!("declared variants: {}", variants.join(", ")))
}

/// The E0934 declaration fact for a hint: names the declaring construct (a
/// `@type` for named sums; the inline union itself for anonymous literals —
/// arity is always 0 there). Each leg appends its own action clause.
fn universe_arity_fact(universe: &SumUniverse, variant: &str, arity: usize) -> String {
    match universe {
        SumUniverse::Named(t) => format!(
            "`@type {}` declares `{}` with payload arity {}",
            t, variant, arity
        ),
        SumUniverse::Inline(_) => {
            format!("the inline union declares `{}` with no payload", variant)
        }
    }
}

/// Shared enum/sum facts (PLAN-077 W3 diagnostics + W4 mutation lowering) —
/// built ONCE per compile from the FormMatch surface: the `@type` registry,
/// signal → declared type name (`@data <kind> $name T : …`), and signal →
/// inline-union variant list (`@data derive $x (A | B) : @match { … }`).
struct EnumTypeFacts {
    types: crate::type_system::TypeRegistry,
    signal_types: HashMap<String, String>,
    inline_unions: HashMap<String, Vec<String>>,
}

impl EnumTypeFacts {
    fn from_matches(matches: &[FormMatch]) -> Self {
        const DATA_MACROS: &[&str] = &[
            "data-inline",
            "data-fetch-kind",
            "data-derive",
            "data-fold",
            "data-derive-match",
        ];
        let mut signal_types: HashMap<String, String> = HashMap::new();
        let mut inline_unions: HashMap<String, Vec<String>> = HashMap::new();
        for fm in matches {
            let Some(matched) = fm.matched_macro.as_deref() else {
                continue;
            };
            if !DATA_MACROS.contains(&matched) {
                continue;
            }
            let Some(name) = fm.captures.get("name").and_then(signal_name_of) else {
                continue;
            };
            if let Some(CapturedValue::TypeRef(t)) = fm.captures.get("type") {
                signal_types.insert(name.clone(), t.clone());
            }
            if let Some(uitype) = fm.captures.get("uitype")
                && let Some(variants) = inline_union_variants(uitype)
            {
                inline_unions.insert(name, variants);
            }
        }
        Self {
            types: crate::type_system::TypeRegistry::from_form_matches(matches),
            signal_types,
            inline_unions,
        }
    }

    /// Declared sum universe for a signal (None = uncheckable — skip, never
    /// guess, Gate-3 criterion a).
    fn sum_universe_of(&self, signal: &str) -> Option<SumUniverse> {
        sum_universe_of(signal, &self.signal_types, &self.inline_unions, &self.types)
    }
}

// =============================================================================
// Mutation lowering: `$x <- Variant(args)` (PLAN-077 W4)
// =============================================================================

/// A parsed `<-` statement: target signal (no `$`) + RHS text.
fn parse_mutation_stmt(seg: &str) -> Option<(String, String)> {
    let (left, right) = seg.split_once("<-")?;
    let var = left.trim().trim_start_matches('$');
    if var.is_empty()
        || !var.chars().all(|c| c.is_alphanumeric() || c == '_')
        || var.chars().next().is_some_and(|c| c.is_numeric())
    {
        return None;
    }
    let rhs = right.trim().trim_end_matches(';').trim();
    if rhs.is_empty() {
        return None;
    }
    Some((var.to_string(), rhs.to_string()))
}

/// A capitalized-bareword construction: `Ident` or `Ident(arg1, arg2)` — the
/// WHOLE RHS must be the construction (trailing tokens = not a construction).
/// Returns (ident, args) with args split at top-level commas.
fn parse_variant_construction(rhs: &str) -> Option<(String, Vec<String>)> {
    let ident_end = rhs
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rhs.len());
    let ident = &rhs[..ident_end];
    if ident.is_empty() || !ident.chars().next()?.is_uppercase() {
        return None; // the capitalized-bareword gate — lowercase never lowers
    }
    let rest = rhs[ident_end..].trim();
    if rest.is_empty() {
        return Some((ident.to_string(), vec![]));
    }
    if !(rest.starts_with('(') && rest.ends_with(')')) {
        return None;
    }
    let inner = &rest[1..rest.len() - 1];
    // Split at top-level commas — depth-aware (parens/brackets/braces nest)
    // AND string-aware (a comma inside a quoted literal is NOT a separator;
    // SWARM GATE 4 finding — BalancedExtractor has the same discipline).
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut last = 0usize;
    let mut in_quote: Option<char> = None;
    let mut prev = '\0';
    for (i, c) in inner.char_indices() {
        if let Some(q) = in_quote {
            if c == q && prev != '\\' {
                in_quote = None;
            }
        } else {
            match c {
                '"' | '\'' | '`' => in_quote = Some(c),
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    args.push(inner[last..i].trim().to_string());
                    last = i + 1;
                }
                _ => {}
            }
        }
        prev = c;
    }
    let tail = inner[last..].trim();
    if !tail.is_empty() {
        args.push(tail.to_string());
    }
    Some((ident.to_string(), args))
}

/// Reconstruct the statement text of a DECOMPOSED mutation — `$t <- e` with the
/// `<-` stripped by the capture grammar — from its `target` + `expr` Named keys.
/// The sigil `@on &.<driver> { $x <- Variant; }` body (on_motion_body ->
/// `$mut:mutation`) and the colon form (`@on &.click: $x <- v;`, on_mutation)
/// both carry mutations this way. The structure (both keys present) is the
/// recognition key, never a macro-name or capture-key allowlist.
fn decomposed_mutation_stmt(map: &std::collections::HashMap<String, CapturedValue>) -> Option<String> {
    let target = match map.get("target") {
        Some(CapturedValue::Ident(s))
        | Some(CapturedValue::Binding(s))
        | Some(CapturedValue::String(s)) => s.trim_start_matches('$').to_string(),
        Some(v) => v.to_js(crate::syntax::JsQuoting::Raw),
        None => return None,
    };
    let expr = match map.get("expr") {
        Some(CapturedValue::Expr(e)) => e.clone(),
        Some(v) => v.to_js(crate::syntax::JsQuoting::Raw),
        None => return None,
    };
    if target.is_empty() || expr.is_empty() {
        return None;
    }
    Some(format!("${target} <- {expr}"))
}

/// Lower ONE `<-` statement (PLAN-077 W4): when the RHS is a capitalized
/// bareword construction `Ident`/`Ident(args)` AND the target signal's
/// declared type is a known sum containing `Ident`, rewrite the RHS to the
/// `{type: 'Ident', ...namedFields}` object the union runtime consumes
/// (payload field naming = the W2 convention: bare `$sig` arg → the signal's
/// name; single non-signal → `value`; multi → `argN`). Everything else passes
/// through UNCHANGED — the special case is strictly additive, gated on a
/// positive TypeRegistry match, never a guess.
///
/// Diagnostics (the W3-deferred mutation legs): E0933 when the target IS a
/// known sum but a BARE capitalized `Ident` (no args — unambiguous variant
/// intent, never a JS call) is not a declared variant; E0934 when the
/// construction's arg count ≠ the variant's declared payload arity (the
/// lowering still happens — the error is authoritative).
fn lower_variant_mutation_stmt(
    stmt: &str,
    facts: &EnumTypeFacts,
    span: crate::parser::SourceSpan,
    forbidden_bind: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let Some((var, rhs)) = parse_mutation_stmt(stmt) else {
        return stmt.to_string();
    };
    let Some((ident, args)) = parse_variant_construction(&rhs) else {
        return stmt.to_string();
    };
    let Some(universe) = facts.sum_universe_of(&var) else {
        return stmt.to_string(); // untyped/non-sum target — untouched
    };
    let variants = universe_variants(&universe, &facts.types);
    if !variants.contains(&ident.as_str()) {
        // E0933's mutation leg: BARE `Ident` only — `Ident(...)` could be a
        // legitimate JS constructor/global call (`String($y)`), never flagged.
        if args.is_empty() && !rhs.contains('(') {
            diagnostics.push(unknown_variant_diag(
                span,
                &ident,
                &universe,
                &facts.types,
                &format!("in `${} <- {}`", var, ident),
            ));
        }
        return stmt.to_string();
    }
    if let Some(arity) = universe_payload_arity(&universe, &facts.types, &ident)
        && arity != args.len()
    {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::E0934,
                format!(
                    "`{}` takes {} payload value{} but the constructor has {} (in `${} <- {}`)",
                    ident,
                    arity,
                    if arity == 1 { "" } else { "s" },
                    args.len(),
                    var,
                    rhs
                ),
            )
            .with_span(span.into())
            .with_hint(format!(
                "{} — adjust the construction's arguments",
                universe_arity_fact(&universe, &ident, arity)
            )),
        );
    }
    let mut fields = String::new();
    for (i, arg) in args.iter().enumerate() {
        let bare = arg.strip_prefix('$').filter(|rest| {
            !rest.is_empty()
                && rest.chars().all(|c| c.is_alphanumeric() || c == '_')
                && rest
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
        });
        let field = match bare {
            Some(sig) => sig.to_string(),
            None if args.len() == 1 => "value".to_string(),
            None => format!("arg{}", i),
        };
        if !fields.is_empty() {
            fields.push_str(", ");
        }
        fields.push_str(&format!("{}: {}", field, arg));
    }
    let object = if fields.is_empty() {
        format!("{{type: '{}'}}", ident)
    } else {
        format!("{{type: '{}', {}}}", ident, fields)
    };
    // SWARM GATE 4: in a `@handle` receive arm the body runs under
    // runMutations' BARE-NAME locals substitution, which rewrites every
    // occurrence of the arm's bind name — including object KEYS. If the bind
    // collides with an emitted key (`type`, always present, or a payload
    // field name), the lowered object would be silently corrupted (the
    // discriminator destroyed). Skip the lowering and warn — the author
    // renames the bind or moves the construction into a derive.
    if let Some(bind) = forbidden_bind {
        let collides = bind == "type"
            || args.iter().enumerate().any(|(i, arg)| {
                let bare = arg.strip_prefix('$').filter(|rest| {
                    !rest.is_empty()
                        && rest.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && rest
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphabetic() || c == '_')
                });
                let field = match bare {
                    Some(sig) => sig,
                    None if args.len() == 1 => "value",
                    None => return format!("arg{}", i) == bind,
                };
                field == bind
            });
        if collides {
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::W0716,
                    format!(
                        "`{} <- {}` is not lowered: the receive-arm bind `{}` collides with a key of the emitted union object",
                        var, rhs, bind
                    ),
                )
                .with_span(span.into())
                .with_hint(format!(
                    "runMutations substitutes the bind's bare name everywhere in the arm body, including object keys — rename the `{}` bind (e.g. `{}_payload`) or construct the union in a `@data derive` instead",
                    bind, bind
                )),
            );
            return stmt.to_string();
        }
    }
    // Preserve the statement's leading whitespace shape (`$var <- <object>`).
    let arrow_pos = stmt.find("<-").unwrap();
    format!("{}<- {}", &stmt[..arrow_pos], object)
}

/// Rewrite every mutation-statement TEXT segment (`;`-separated, the rail's
/// own convention — `MutationActions` splits identically) inside a body
/// string. Segments that aren't variant constructions pass through byte-
/// identically (split+join on ';' is lossless for them).
fn lower_mutation_body_text(
    text: &str,
    facts: &EnumTypeFacts,
    span: crate::parser::SourceSpan,
    forbidden_bind: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    if !text.contains("<-") {
        return text.to_string();
    }
    // String-aware: a `;` inside a quoted value is content, not a separator
    // (BUG-225). The `raw` variant keeps segments verbatim so the join below
    // stays byte-lossless for statements that aren't rewritten.
    crate::syntax::events::form_compiler::split_statements_outside_strings_raw(text)
        .into_iter()
        .map(|seg| {
            if seg.contains("<-") {
                lower_variant_mutation_stmt(seg, facts, span, forbidden_bind, diagnostics)
            } else {
                seg.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Recursive capture-tree rewrite: every `Named` map carrying a
/// `js_statements` array (the `mutation_actions` surface: @on / @effect /
/// when-guard) gets each statement lowered; the mutation-text macros
/// (@handle / @data signal / @data stream) additionally get their arm
/// `body`/`stmt` + `optimistic`/`final` clause texts lowered — as String OR
/// Expr captures (BalancedExtractor yields Expr for the clause texts; SWARM
/// GATE 4 finding). A receive arm's `bind` is threaded as the forbidden key
/// for the collision guard.
fn lower_captures_mutations(
    value: &mut CapturedValue,
    is_mutation_text_macro: bool,
    facts: &EnumTypeFacts,
    span: crate::parser::SourceSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        CapturedValue::Named(map) => {
            // A DECOMPOSED MUTATION — `$t <- e` with the `<-` stripped. The sigil
            // `@on &.<driver> { $x <- Variant; }` body (on_motion_body -> `$mut:mutation`)
            // and the colon form (`@on &.click: $x <- v;`, capture type `on_mutation`)
            // both carry it as a Named map keyed `target` + `expr`. Recognised by
            // STRUCTURE (both keys present) — the same key the state_analysis
            // writer-collection walk uses — not by a macro name or capture-key
            // allowlist, so a newly declared driver cannot silently reintroduce the
            // blind spot. Reconstruct the statement, lower it, and re-decompose.
            // `lower_variant_mutation_stmt` only ever rewrites the RHS (the `$var <-`
            // prefix is preserved byte-for-byte), so re-splitting at `<-` is lossless.
            if map.contains_key("target") && map.contains_key("expr") {
                if let Some(stmt) = decomposed_mutation_stmt(map) {
                    let lowered = lower_variant_mutation_stmt(&stmt, facts, span, None, diagnostics);
                    if lowered != stmt {
                        if let Some((_, rhs)) = lowered.split_once("<-") {
                            map.insert(
                                "expr".to_string(),
                                CapturedValue::Expr(rhs.trim().to_string()),
                            );
                        }
                    }
                }
                return;
            }
            // A receive arm's bind (sibling key of body/stmt) — bare-name
            // collision guard input (SWARM GATE 4).
            let arm_bind: Option<String> = if is_mutation_text_macro {
                match map.get("bind") {
                    Some(CapturedValue::Ident(b)) | Some(CapturedValue::Binding(b)) => {
                        Some(b.trim_start_matches('$').to_string())
                    }
                    _ => None,
                }
            } else {
                None
            };
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort(); // deterministic traversal
            for key in keys {
                if key == "js_statements" {
                    if let Some(CapturedValue::Array(stmts)) = map.get_mut(&key) {
                        for stmt in stmts.iter_mut() {
                            if let CapturedValue::String(s) = stmt {
                                *s = lower_variant_mutation_stmt(s, facts, span, None, diagnostics);
                            }
                        }
                    }
                    continue;
                }
                if is_mutation_text_macro
                    && matches!(key.as_str(), "body" | "stmt" | "optimistic" | "final")
                {
                    match map.get_mut(&key) {
                        Some(CapturedValue::String(s)) | Some(CapturedValue::Expr(s)) => {
                            *s = lower_mutation_body_text(
                                s,
                                facts,
                                span,
                                arm_bind.as_deref(),
                                diagnostics,
                            );
                        }
                        _ => {}
                    }
                    continue;
                }
                if let Some(v) = map.get_mut(&key) {
                    lower_captures_mutations(v, is_mutation_text_macro, facts, span, diagnostics);
                }
            }
        }
        CapturedValue::Array(items) => {
            for item in items.iter_mut() {
                lower_captures_mutations(item, is_mutation_text_macro, facts, span, diagnostics);
            }
        }
        _ => {}
    }
}

/// Cheap pre-filter: does any string in the capture tree contain `<-`?
/// Avoids cloning every FormMatch for the (overwhelmingly common) no-mutation
/// case — the transform below clones only candidates (SWARM GATE 4 perf
/// criterion: no new work on compiles with no mutations).
fn captures_have_arrow(v: &CapturedValue) -> bool {
    match v {
        CapturedValue::String(s) | CapturedValue::Expr(s) => s.contains("<-"),
        CapturedValue::Named(m) => {
            // A DECOMPOSED mutation (`$t <- e`) has the arrow STRIPPED — the
            // `<-` is consumed by the capture grammar, so its text carries no
            // arrow. It is still a mutation: the structure (both `target` and
            // `expr` keys present) carries the intent. Same structural key the
            // state_analysis writer-collection walk keys off (BUG-sister fix),
            // so a newly declared driver macro cannot silently reintroduce the
            // blind spot via a capture-name allowlist.
            (m.contains_key("target") && m.contains_key("expr"))
                || m.values().any(captures_have_arrow)
        }
        CapturedValue::Array(items) => items.iter().any(captures_have_arrow),
        _ => false,
    }
}

/// The W4 transform over the whole match slice: returns `Some(rewritten)`
/// only when at least one statement changed (clone-free borrow otherwise —
/// the same shape as `canonicalize_qualifiers`), plus the E0933/E0934
/// mutation-leg diagnostics gathered along the way.
fn lower_variant_mutations(
    matches: &[FormMatch],
    facts: &EnumTypeFacts,
) -> (Option<Vec<FormMatch>>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut owned: Option<Vec<FormMatch>> = None;
    for (i, fm) in matches.iter().enumerate() {
        if !fm.captures.values().any(captures_have_arrow) {
            continue; // no mutation text anywhere — skip the clone entirely
        }
        let is_mutation_text_macro = matches!(
            fm.matched_macro.as_deref(),
            Some("handle") | Some("data-signal") | Some("data-stream")
        );
        let mut trial = fm.clone();
        for (k, v) in trial.captures.iter_mut() {
            // Top-level lifted clause text (e.g. @handle's `optimistic` — an
            // Expr LEAF directly under the captures map, never inside a
            // Named sub-record the walker would key-match).
            if is_mutation_text_macro
                && matches!(k.as_str(), "body" | "stmt" | "optimistic" | "final")
            {
                if let CapturedValue::String(s) | CapturedValue::Expr(s) = v {
                    *s = lower_mutation_body_text(s, facts, fm.span, None, &mut diagnostics);
                }
                continue;
            }
            lower_captures_mutations(v, is_mutation_text_macro, facts, fm.span, &mut diagnostics);
        }
        if trial != *fm {
            owned.get_or_insert_with(|| matches.to_vec())[i] = trial;
        }
    }
    (owned, diagnostics)
}

/// Enum/union checks (PLAN-077 W3):
///   E0931 cond-mode totality — every `@data derive $x T : @match { … }` arm
///         chain must terminate in `_` (and `_` must be LAST — first-match-wins
///         makes later arms unreachable; SWARM GATE 2 deferral).
///   E0932 dispatch exhaustiveness — a `@match $subject` / `@view $subject`
///         whose subject's type is a KNOWN `@type` sum must cover all variants
///         or carry a `_` fallback. Skipped when the subject's type is unknown
///         or not a sum (string subjects NEVER trip this — Gate-3 criterion a).
///   E0933 unknown variant — a dispatch destructure pattern, a cond-mode
///         consequence, or a `@state(when: $x is V)` (state-match form) names a
///         constructor outside the declared sum / inline-union literal.
///   E0934 payload arity — a destructure pattern's binding count (or a
///         cond-mode consequence's payload-arg count) ≠ the variant's declared
///         payload arity.
///
/// Every check fires ONLY on a positive sum-type / inline-union match — an
/// unresolvable type means "skip", never "guess". `<-` mutation RHS variant
/// constructions are W4's lowering site; the E0933 leg for them lands there.
#[cfg_attr(not(test), allow(dead_code))]
fn check_enum_union_diagnostics(matches: &[FormMatch]) -> Vec<Diagnostic> {
    check_enum_union_diagnostics_with_facts(matches, &EnumTypeFacts::from_matches(matches))
}

fn check_enum_union_diagnostics_with_facts(
    matches: &[FormMatch],
    facts: &EnumTypeFacts,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let types = &facts.types;
    let signal_types = &facts.signal_types;
    let inline_unions = &facts.inline_unions;

    for fm in matches {
        // --- E0931/E0933/E0934: cond-mode derive arms ------------------------
        if fm.matched_macro.as_deref() == Some("data-derive-match") {
            let derive_name = fm
                .captures
                .get("name")
                .and_then(signal_name_of)
                .unwrap_or_else(|| "?".to_string());
            if let Some(arms) = cond_arms_of(fm) {
                // E0931: totality — a `_` catch-all must EXIST and be LAST
                // (first-match-wins makes arms after a `_` unreachable — a
                // mid-chain `_` with later arms is the SWARM GATE 2 deferral).
                let catch_positions: Vec<usize> = arms
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| cond_arm_guard(a).is_some_and(cond_guard_is_catch_all))
                    .map(|(i, _)| i)
                    .collect();
                if catch_positions.is_empty() {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0931,
                            format!(
                                "`@data derive ${} : @match` has no `_` catch-all arm",
                                derive_name
                            ),
                        )
                        .with_span(fm.span.into())
                        .with_hint(
                            "guards are arbitrary booleans — the compiler cannot verify coverage, so the chain must always terminate in a value: add a trailing `_ => <Variant>;` arm".to_string(),
                        ),
                    );
                } else if catch_positions.as_slice() != [arms.len() - 1] {
                    // Non-last OR duplicated `_` (SWARM GATE 3 finding: a
                    // second `_` is itself an unreachable arm after the first).
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0931,
                            format!(
                                "`@data derive ${} : @match` has a `_` catch-all in a non-last position",
                                derive_name
                            ),
                        )
                        .with_span(fm.span.into())
                        .with_hint(
                            "first truthy guard wins — every arm after `_` is unreachable; move `_` to the end".to_string(),
                        ),
                    );
                }

                // E0933/E0934: consequences vs the declared universe.
                if let Some(universe) =
                    sum_universe_of(&derive_name, &signal_types, &inline_unions, &types)
                {
                    let variants = universe_variants(&universe, &types);
                    for arm in arms {
                        let CapturedValue::Named(arm_rec) = arm else {
                            continue;
                        };
                        let Some(CapturedValue::Named(cons)) = arm_rec.get("cons") else {
                            continue;
                        };
                        let Some(CapturedValue::Ident(variant)) = cons.get("name") else {
                            continue;
                        };
                        let arg_count = match cons.get("payload") {
                            Some(CapturedValue::Array(payload)) => payload.len(),
                            _ => 0,
                        };
                        if !variants.contains(&variant.as_str()) {
                            diagnostics.push(unknown_variant_diag(
                                fm.span,
                                variant,
                                &universe,
                                &types,
                                &format!("in `@data derive ${}`", derive_name),
                            ));
                            continue; // arity is meaningless for an unknown variant
                        }
                        if let Some(arity) = universe_payload_arity(&universe, &types, variant)
                            && arity != arg_count
                        {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::E0934,
                                    format!(
                                        "`{}` takes {} payload value{} but the constructor has {} (in `@data derive ${}`)",
                                        variant,
                                        arity,
                                        if arity == 1 { "" } else { "s" },
                                        arg_count,
                                        derive_name
                                    ),
                                )
                                .with_span(fm.span.into())
                                .with_hint(format!(
                                    "{} — adjust the constructor's arguments",
                                    universe_arity_fact(&universe, variant, arity)
                                )),
                            );
                        }
                    }
                }
            }
            continue;
        }

        // --- E0932/E0933/E0934: dispatch-mode @match/@view -------------------
        if (fm.macro_name == "match" || fm.macro_name == "view")
            && fm.captures.contains_key("subject")
        {
            let Some(subject) = fm.captures.get("subject").and_then(signal_name_of) else {
                continue;
            };
            let Some(universe) = sum_universe_of(&subject, &signal_types, &inline_unions, &types)
            else {
                continue; // unknown/non-sum subject: uncheckable, never guess
            };
            let variants = universe_variants(&universe, &types);
            let Some(CapturedValue::Array(arms)) = fm.captures.get("arms") else {
                continue;
            };

            let mut covered: HashSet<String> = HashSet::new();
            let mut has_wildcard = false;
            for arm in arms {
                let CapturedValue::Named(arm_rec) = arm else {
                    continue;
                };
                let Some(CapturedValue::Named(pat)) = arm_rec.get("pat") else {
                    continue;
                };
                if pat.contains_key("wild") {
                    has_wildcard = true;
                    continue;
                }
                if let Some(CapturedValue::Named(variant_pat)) = pat.get("variant") {
                    if let Some(CapturedValue::Ident(name)) = variant_pat.get("name") {
                        if !variants.contains(&name.as_str()) {
                            diagnostics.push(unknown_variant_diag(
                                fm.span,
                                name,
                                &universe,
                                &types,
                                &format!("in `@{} ${}`", fm.macro_name, subject),
                            ));
                            continue;
                        }
                        covered.insert(name.as_str().to_string());
                        let binding_count = match variant_pat.get("bindings") {
                            Some(CapturedValue::Array(bs)) => bs.len(),
                            _ => 0,
                        };
                        if let Some(arity) = universe_payload_arity(&universe, &types, name)
                            && arity != binding_count
                        {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::E0934,
                                    format!(
                                        "`{} {{ … }}` binds {} value{} but the variant's payload arity is {} (in `@{} ${}`)",
                                        name,
                                        binding_count,
                                        if binding_count == 1 { "" } else { "s" },
                                        arity,
                                        fm.macro_name,
                                        subject
                                    ),
                                )
                                .with_span(fm.span.into())
                                .with_hint(format!(
                                    "{} — bind exactly that many `$names`, or match the bare tag",
                                    universe_arity_fact(&universe, name, arity)
                                )),
                            );
                        }
                    }
                    continue;
                }
                // Bare payload-less tag routes to `lit` (FUP-079). Count it as
                // coverage when it names a declared variant; a quoted string
                // that names no variant is NOT E0933's concern (it may be a
                // legacy string match the type declaration postdates).
                if let Some(CapturedValue::String(lit)) = pat.get("lit")
                    && variants.contains(&lit.as_str())
                {
                    covered.insert(lit.clone());
                }
            }

            if !has_wildcard {
                let missing: Vec<&str> = variants
                    .iter()
                    .filter(|v| !covered.contains(**v))
                    .copied()
                    .collect();
                if !missing.is_empty() {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0932,
                            format!(
                                "`@{} ${}` is non-exhaustive — no arms for variant{}: {}",
                                fm.macro_name,
                                subject,
                                if missing.len() == 1 { "" } else { "s" },
                                missing.join(", ")
                            ),
                        )
                        .with_span(fm.span.into())
                        .with_hint(
                            "add an arm per missing variant or a `_ =>` fallback (the subject's `@type` is a closed sum, so coverage is decidable)".to_string(),
                        ),
                    );
                }
            }
        }
    }

    // --- E0933/E0934: @state(when: $x is V { $b }) — the state-match form.
    // String-condition `@state(when: "loading")` claims `%macro state` instead
    // and NEVER reaches this leg (Gate-3 criterion a).
    // W6: the `state_when` capture type yields the pattern STRUCTURED —
    // `{when: {signal, variant, bindings?}}` — replacing the old collapsed
    // `signal: Expr` text parse (parse_state_when, retired).
    for fm in matches {
        if fm.matched_macro.as_deref() != Some("state-match") {
            continue;
        }
        let Some(CapturedValue::Named(when)) = fm.captures.get("when") else {
            continue;
        };
        let (Some(CapturedValue::Ident(signal)), Some(CapturedValue::Ident(variant))) =
            (when.get("signal"), when.get("variant"))
        else {
            continue;
        };
        let (signal, variant) = (signal.clone(), variant.clone());
        // The `bindings` key exists only when the author WROTE the
        // `{ $bindings }` destructure group (the capture type's optional
        // group) — presence IS the E0934 gate (a bare `@state(when: $f is
        // Failed)` destructures nothing, so payload arity is not its concern).
        let wrote_braces = when.contains_key("bindings");
        let bindings: Vec<String> = match when.get("bindings") {
            Some(CapturedValue::Array(items)) => items
                .iter()
                .filter_map(|it| match it {
                    CapturedValue::Named(m) => match m.get("b") {
                        // `$b:binding` materializes as Binding("$e") — the
                        // reference extractor (GATE 6 P1: matching only Ident
                        // made binding_count always 0, false-firing E0934 on
                        // every correct-arity destructure).
                        Some(CapturedValue::Binding(b)) => {
                            Some(b.trim_start_matches('$').to_string())
                        }
                        Some(CapturedValue::Ident(b)) => Some(b.clone()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };
        if variant == "_" {
            continue;
        }
        let Some(universe) = sum_universe_of(&signal, &signal_types, &inline_unions, &types) else {
            continue;
        };
        let variants = universe_variants(&universe, &types);
        if !variants.contains(&variant.as_str()) {
            diagnostics.push(unknown_variant_diag(
                fm.span,
                &variant,
                &universe,
                &types,
                &format!("in `@state(when: ${} is {})`", signal, variant),
            ));
            continue;
        }
        if !wrote_braces {
            continue;
        }
        let binding_count = bindings.len();
        if let Some(arity) = universe_payload_arity(&universe, &types, &variant)
            && arity != binding_count
        {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0934,
                    format!(
                        "`{} {{ … }}` binds {} value{} but the variant's payload arity is {} (in `@state(when: ${} is {})`)",
                        variant,
                        binding_count,
                        if binding_count == 1 { "" } else { "s" },
                        arity,
                        signal,
                        variant
                    ),
                )
                .with_span(fm.span.into())
                .with_hint(format!(
                    "{} — bind exactly that many `$names`, or drop the binding group",
                    universe_arity_fact(&universe, &variant, arity)
                )),
            );
        }
    }

    diagnostics
}

/// Waves at or before the project's `@version` are inert (the project
/// already crossed them).
fn check_retired_syntax(matches: &[FormMatch], context: &CompileContext) -> Vec<Diagnostic> {
    let version_fact = crate::migrate::read_syntax_version(matches);
    // `check --at-version` overrides the source's @version fact.
    //
    // HARD CUTOVER: an ABSENT `@version` means CURRENT, not ancient.
    //
    // The migration window (W0715) exists so a project can cross a wave
    // gradually — retired syntax keeps compiling, rewritten in memory, until
    // the project's `@version` reaches the wave date and it becomes E0911.
    // But an absent version compared as "before every wave", so the window
    // stood open FOREVER for any file that never declared one: a file written
    // today in syntax retired months ago compiled silently. That is a
    // permanent amnesty, not a transition, and it is how ~100 sites of retired
    // syntax accumulated unnoticed (AUD-009).
    //
    // Defaulting to the newest wave inverts it: new code is held to the
    // current surface, and a project genuinely mid-migration opts INTO the
    // window by declaring the older `@version` it is migrating FROM — which is
    // exactly what `spacetime migrate` writes and then bumps.
    let version = context
        .version_override
        .clone()
        .or(version_fact.version.clone())
        .or_else(|| {
            context
                .meta_registry
                .newest_migration_wave()
                .map(String::from)
        });
    let mut diagnostics = Vec::new();

    // The fact's own integrity (every compile path — R4: from_ast too, so
    // `check` and `serve`/`build` agree):
    // - several declarations in one file (or a non-ISO date) is an
    //   authoring error;
    if version_fact.decls > 1 {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::E0911,
                format!(
                    "duplicate @version declarations ({}) — one per project, in the root entry file",
                    version_fact.decls
                ),
            )
            .with_hint("keep a single `@version <date>;` at the top of the project's root index.st"),
        );
    }
    if let Some(v) = version.as_deref()
        && !crate::migrate::is_iso_wave_date(v)
    {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::E0911,
                format!(
                    "invalid @version `{v}` — expected an ISO wave date like `@version 2026-06-09;`"
                ),
            )
            .with_hint(
                "keep a single `@version <date>;` at the top of the project's root index.st",
            ),
        );
    }
    // `@version` is a ROOT-ENTRY fact: after the import merge, declarations
    // from MORE THAN ONE file mean it leaked into an import (or the root has
    // several) — an authoring error, not a fact to compile by.
    if version_fact.files > 1 {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::E0911,
                format!(
                    "@version must live in the root entry file only (found in {} files)",
                    version_fact.files
                ),
            )
            .with_hint(
                "keep a single `@version <date>;` at the top of the project's root index.st",
            ),
        );
    }

    // Whether the effective version came from the FILE or from the cutover
    // default changes what a retired-syntax author needs to hear (see below).
    let version_was_declared = version_fact.version.is_some();

    for fm in matches {
        // The owning migration + rule live behind `matched_macro` (the exact
        // def parse selected): `<migration>#<rule>` for a rewrite-rule def,
        // the embedded macro's name for a retired-definition match.
        let def_name = fm.matched_macro.as_deref().unwrap_or(&fm.macro_name);
        let Some((mig_id, rule_id)) = context.meta_registry.retired_by_macro(def_name) else {
            continue;
        };
        let Some(mig) = context.meta_registry.get_migration(mig_id) else {
            continue;
        };
        let directive = format!("@{}", fm.macro_name.trim_start_matches('@'));
        // A wave AT OR BEFORE the project's @version is inert: the project
        // claims it already crossed that wave, so lingering pre-wave syntax
        // is a VERSION INCONSISTENCY (E0911), not a pending migration.
        // Whether the version came from the FILE or from the cutover default
        // changes what the author needs to hear. Declared: "you said you
        // crossed this wave, so this usage is inconsistent." Defaulted: there
        // is no `@version` to be inconsistent WITH — the syntax is simply gone,
        // and telling them `@version was bumped too far` would send them
        // looking for a declaration that does not exist.
        if let Some(v) = version.as_deref()
            && mig.date.as_str() <= v
        {
            if !version_was_declared {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0911,
                        format!(
                            "{} was retired in the {} syntax wave — {}",
                            directive, mig.date, mig.docs
                        ),
                    )
                    .with_span(fm.span.into())
                    .with_hint(format!(
                        "run `spacetime migrate` to rewrite it automatically. (If this project \
                         is still mid-migration, declare the version it is migrating FROM with \
                         `@version <date>;` to reopen the window.) {}",
                        mig.hint_for(fm.macro_name.trim_start_matches('@')).unwrap_or_default()
                    )),
                );
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0911,
                    format!(
                        "@version is {} but this file still uses {} (retired in the {} wave)",
                        v, directive, mig.date
                    ),
                )
                .with_span(fm.span.into())
                .with_hint(format!(
                    "either this usage predates the migration and @version was bumped too far, \
                     or the usage was re-introduced after migrating — {}",
                    mig.docs
                )),
            );
            continue;
        }
        let bare = fm.macro_name.trim_start_matches('@');
        match rule_id {
            // A rewrite-rule def matched but survived to the FINAL AST — the
            // shim should have consumed it. This is the degrade path (an
            // imported file, a source-less compile, or a shape the rule
            // couldn't soundly rewrite) → E0910.
            Some(_) => {
                let hint = mig
                    .hint_for(bare)
                    .map(|text| format!("{} (migration `{}`)", text, mig.id))
                    .unwrap_or_else(|| {
                        format!(
                            "this call can't be auto-migrated (a multi-shape call, an imported \
                             file, or a source-less compile) — migrate it manually or run \
                             `spacetime migrate`. (migration `{}`)",
                            mig.id
                        )
                    });
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0910,
                        format!(
                            "{} was removed in the {} syntax wave — {}",
                            directive, mig.date, mig.docs
                        ),
                    )
                    .with_span(fm.span.into())
                    .with_hint(hint),
                );
            }
            // An embedded retired macro matched.
            None => {
                if !mig.rules_for(bare).is_empty() {
                    // The directive HAS rewrite rules but this call's shape
                    // matched none (e.g. `@bind(when:)` alone) — it can't be
                    // auto-migrated → E0910.
                    let hint = mig
                        .hint_for(bare)
                        .map(|text| format!("{} (migration `{}`)", text, mig.id))
                        .unwrap_or_else(|| {
                            format!(
                                "no rewrite rule covers this {} shape — migrate it manually \
                                 (migration `{}`)",
                                directive, mig.id
                            )
                        });
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0910,
                            format!(
                                "{} was removed in the {} syntax wave — {}",
                                directive, mig.date, mig.docs
                            ),
                        )
                        .with_span(fm.span.into())
                        .with_hint(hint),
                    );
                } else {
                    // Hint-covered directive: the embedded macro's `%binds`
                    // still expand — the old syntax COMPILES through the
                    // window. Warn (W0715) with the migration's %hint.
                    let mut d = Diagnostic::warning(
                        DiagnosticCode::W0715,
                        format!(
                            "{} was retired in the {} syntax wave — compiled via the retired \
                             definition (migration `{}`): {}",
                            directive, mig.date, mig.id, mig.docs
                        ),
                    )
                    .with_span(fm.span.into());
                    if let Some(text) = mig.hint_for(bare) {
                        d = d.with_hint(text.to_string());
                    }
                    diagnostics.push(d);
                }
            }
        }
    }

    diagnostics
}

/// Check for $var references to undefined state variables in template scopes (E0905).
///
/// Walks all template/template-inline form matches and checks that $var references
/// in directives, class toggles, and content injections refer to state variables
/// declared in the same scope or a parent scope (global states).
/// FEAT-115 S5c: extract a `$var` reference from a directive value or statement.
/// Returns the bare name (sigil + any `.field`/`!`/whitespace stripped) of the FIRST
/// `$`-led token, e.g. `$open` from `$open`, `!$open`, `$open | uppercase`.
fn first_signal_ref(s: &str) -> Option<String> {
    let i = s.find('$')?;
    let rest = &s[i + 1..];
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

/// FEAT-115 S5c: collect the state vars REFERENCED by a template scope's directives,
/// reading the World-A scope tree (the AST truth) instead of reify's parallel
/// `ComponentBodyDef.directives` parse. Walks the template Construct scope:
///   - class-toggle / content-binding / self-prop: a nested-scope css_declaration
///     whose value is a `$signal` (`.c--open: $open`, `text <- $open`, `aria-x: $open`)
///   - `@on` action: the `js_statements` of an `on` match (`$open <- !$open`)
/// Each yields `(var_name, hint_kind)` so the caller emits the right E0905 hint.
fn scope_referenced_state_vars(scope: &crate::parser::ScopeBlock) -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = Vec::new();
    // `@on` actions live as `on` matches directly under the scope (or its nested scopes).
    fn walk_on(matches: &[FormMatch], out: &mut Vec<(String, &'static str)>) {
        for m in matches {
            if m.macro_name != "on" {
                continue;
            }
            // Sigil head `@on &.<driver> { … }` — the body is an `on_motion_body`
            // ARRAY of statements; a mutation is a Named map keyed `target` +
            // `expr` (the `<-` stripped). Same structural key as the state_analysis
            // walk. The mutated var is the LHS (`target`); the RHS may also read.
            if let Some(CapturedValue::Array(items)) = m.captures.get("body") {
                for item in items {
                    // A mutation is the `$mut:mutation` capture: `"mut" -> Named{target, expr}`.
                    if let CapturedValue::Named(stmt_item) = item
                        && let Some(CapturedValue::Named(mut_map)) = stmt_item.get("mut")
                        && let Some(stmt_text) = decomposed_mutation_stmt(mut_map)
                    {
                        let lhs = stmt_text.split([':', '<', '=']).next().unwrap_or(&stmt_text);
                        if let Some(v) = first_signal_ref(lhs).or_else(|| first_signal_ref(&stmt_text)) {
                            out.push((v, "event"));
                        }
                    }
                }
            }
            if let Some(CapturedValue::Named(map)) = m.captures.get("body")
                && let Some(CapturedValue::Array(stmts)) = map.get("js_statements")
            {
                for st in stmts {
                    if let CapturedValue::String(s) = st {
                        // The mutated var is the LHS of `<-` / `=`; fall back to first ref.
                        let lhs = s.split([':', '<', '=']).next().unwrap_or(s);
                        if let Some(v) = first_signal_ref(lhs).or_else(|| first_signal_ref(s)) {
                            out.push((v, "event"));
                        }
                    }
                }
            }
        }
    }
    // Reactive css_declarations (class toggle / content binding / self-prop) live on the
    // synthesized + `.sel` nested scopes.
    fn walk_decls(
        nested: &[crate::parser::ast::NestedScope],
        out: &mut Vec<(String, &'static str)>,
    ) {
        for n in nested {
            for cd in &n.css_declarations {
                if cd.value.trim_start().starts_with('$')
                    && let Some(v) = first_signal_ref(&cd.value)
                {
                    let kind = if cd.property.starts_with('.') {
                        "class"
                    } else {
                        "content"
                    };
                    out.push((v, kind));
                }
            }
            walk_on(&n.matches, out);
            walk_decls(&n.nested_scopes, out);
        }
    }
    walk_on(&scope.matches, &mut out);
    walk_decls(&scope.nested_scopes, &mut out);
    out
}

/// FEAT-115 S5c: the state vars DECLARED in a template scope, from its `local-state`
/// matches (World A) — replaces reading reify's `ComponentBodyDef.states`.
fn scope_declared_state_names(scope: &crate::parser::ScopeBlock) -> HashSet<String> {
    fn walk(
        matches: &[FormMatch],
        nested: &[crate::parser::ast::NestedScope],
        out: &mut HashSet<String>,
    ) {
        for m in matches {
            if (m.macro_name == "local-state" || m.macro_name == "local-state-uninitialized")
                && let Some(n) = m.get_ident("name")
            {
                out.insert(n.to_string());
            }
        }
        for n in nested {
            walk(&n.matches, &n.nested_scopes, out);
        }
    }
    let mut out = HashSet::new();
    walk(&scope.matches, &scope.nested_scopes, &mut out);
    out
}

/// FEAT-115 (Q4): the World-A `Construct` scope for a template FormMatch, looked up
/// by its `@template:<name>` synthetic selector. The lint passes read the template's
/// exports/refs/states from HERE (the scope) instead of the retired `ComponentBody`
/// capture fields.
fn template_construct_scope<'a>(
    fm: &FormMatch,
    scopes: &'a [crate::parser::ScopeBlock],
) -> Option<&'a crate::parser::ScopeBlock> {
    let name = fm.get_ident("name")?;
    let want = format!("@template:{}", name);
    scopes.iter().find(|s| {
        matches!(&s.kind, crate::parser::ast::ScopeKind::Construct(_)) && s.selector == want
    })
}

fn check_undefined_state_refs(
    matches: &[FormMatch],
    scopes: &[crate::parser::ScopeBlock],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect global state names from local-state declarations at file level
    let global_state_names: HashSet<String> = matches
        .iter()
        .filter(|fm| {
            (fm.macro_name == "local-state" || fm.macro_name == "local-state-uninitialized")
                && fm.selector.is_none()
        })
        .filter_map(|fm| fm.get_ident("name").map(|s| s.to_string()))
        .collect();

    // FEAT-115 S5c: walk template Construct SCOPES (World A) instead of reify's
    // parallel `ComponentBodyDef.directives`/`states`/`injections` parse. The
    // template `@on`/class-toggle/content-binding/injection directives surface as
    // scope matches + reactive css_declarations; declared states as `local-state`
    // matches. The hint kind drives the same per-directive E0905 message as before.
    for scope in scopes {
        if !matches!(scope.kind, crate::parser::ast::ScopeKind::Construct(_)) {
            continue;
        }
        let fm_span = scope.span;
        let declared_states = scope_declared_state_names(scope);
        for (var, kind) in scope_referenced_state_vars(scope) {
            if declared_states.contains(&var) || global_state_names.contains(&var) {
                continue;
            }
            let hint = match kind {
                "class" => format!("Declare it with: ${} bool: false;", var),
                "event" => format!(
                    "Declare the state variable before using it in event handlers: ${} bool: false;",
                    var
                ),
                _ => format!("Declare it with: ${} string: \"\";", var),
            };
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0905,
                    format!("${} references an undefined state variable", var),
                )
                .with_span(fm_span.into())
                .with_hint(hint),
            );
        }
    }

    diagnostics
}

/// BUG-333: refuse an element reference `&$name` whose name was never declared
/// (`&name <selector>;`). The element-ref macro (`&name <sel>;`) is the only
/// thing that registers a name in the element-reference registry, so a `&$name`
/// read with no matching declaration is an authoring mistake — and one that
/// used to degrade into emitted-but-broken JS (`&ST.resolve(__node,'b')`, a
/// dangling `&`) that failed to parse, killing the whole page behind an E0952
/// that pointed at a byte offset in a generated bundle. Same family as
/// GH-13/GH-22: an unresolvable construct must DIAGNOSE, never ship a dead
/// bundle.
///
/// The registry is page-wide (declarations may live in any scope, including
/// @import-ed files — `ast.scopes` is the import-assembled tree). `&self` has no
/// `$` and is handled elsewhere; a macro's `&$bounds` (an Element capture, e.g.
/// @drag) lives inside `%derives` and never reaches a page css_declaration, so
/// it is not scanned here.
fn check_undeclared_element_refs(scopes: &[crate::parser::ScopeBlock]) -> Vec<Diagnostic> {
    // Pass 1: every declared element reference name, page-wide (`&name <sel>;`
    // is an element-ref FormMatch carrying its name as an `ident` capture).
    fn collect_declared(
        matches: &[FormMatch],
        nested: &[crate::parser::ast::NestedScope],
        out: &mut HashSet<String>,
    ) {
        for m in matches {
            if m.macro_name == "element-ref"
                && let Some(n) = m.get_ident("name")
            {
                out.insert(n.to_string());
            }
        }
        for n in nested {
            collect_declared(&n.matches, &n.nested_scopes, out);
        }
    }
    let mut declared: HashSet<String> = HashSet::new();
    for scope in scopes {
        collect_declared(&scope.matches, &scope.nested_scopes, &mut declared);
    }

    let mut hint_declared = declared.iter().cloned().collect::<Vec<_>>();
    hint_declared.sort();
    let hint_suffix = if hint_declared.is_empty() {
        "no element references are declared in this page".to_string()
    } else {
        format!("declared: {}", hint_declared.join(", "))
    };

    // Pass 2: scan every css_declaration value for `&$name` element reads.
    fn check_value(
        value: &str,
        span: crate::parser::SourceSpan,
        declared: &HashSet<String>,
        hint_suffix: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let bytes = value.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let c = bytes[i] as char;
            // Skip string literals so `&$` inside a quoted string is never read.
            if c == '"' || c == '\'' || c == '`' {
                let quote = c;
                i += 1;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    i += 1;
                    if ch == '\\' && i < bytes.len() {
                        i += 1;
                        continue;
                    }
                    if ch == quote {
                        break;
                    }
                }
                continue;
            }
            if c == '&' && bytes.get(i + 1) == Some(&b'$') {
                let start = i + 2;
                let mut j = start;
                while j < bytes.len()
                    && (bytes[j] as char).is_ascii_alphanumeric()
                    || (j < bytes.len() && bytes[j] == b'_')
                {
                    j += 1;
                }
                if j > start {
                    let name = &value[start..j];
                    if !declared.contains(name) {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::E0965,
                                format!(
                                    "&${name} references an undeclared element reference '{name}'"
                                ),
                            )
                            .with_span(span.into())
                            .with_hint(format!(
                                "Declare it with `&{name} <selector>;` in a scope. {hint_suffix}."
                            )),
                        );
                    }
                    i = j;
                    continue;
                }
            }
            i += 1;
        }
    }

    fn walk(
        nested: &[crate::parser::ast::NestedScope],
        declared: &HashSet<String>,
        hint_suffix: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for n in nested {
            for cd in &n.css_declarations {
                check_value(&cd.value, cd.span, declared, hint_suffix, diagnostics);
            }
            walk(&n.nested_scopes, declared, hint_suffix, diagnostics);
        }
    }

    let mut diagnostics = Vec::new();
    for scope in scopes {
        for cd in &scope.css_declarations {
            check_value(&cd.value, cd.span, &declared, &hint_suffix, &mut diagnostics);
        }
        walk(&scope.nested_scopes, &declared, &hint_suffix, &mut diagnostics);
    }
    diagnostics
}

/// Check for template ref issues: E0918 (collection ref outside @each) and E0919 (duplicate refs).
///
/// Walks all form matches and checks:
/// - E0918: &name[] (is_collection=true) in a NON-TEMPLATE context. Collection refs
///   inside template bodies are always allowed because templates can be invoked from
///   @each in other files. E0918 only fires for collection refs outside template bodies.
/// - E0919: duplicate &name in the same template scope
fn check_template_ref_issues(
    matches: &[FormMatch],
    scopes: &[crate::parser::ScopeBlock],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // FEAT-120: the E0900 body-diagnostic bridge is GONE. Body validation
    // (E0900/E0904/E0906/W0700) runs at parse time and lands in `StFile.diagnostics`,
    // drained once into `output.diagnostics` — no per-code re-derivation from the
    // capture here. This pass now only checks E0918/E0919 (ref issues) below.

    for fm in matches {
        // E0918: check for collection refs in non-template FormMatches
        // Collection refs inside template bodies are always valid — the template
        // can be invoked from @each in another file or context.
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            // FEAT-119 (W4): refs read from the World-A Construct scope (a body-bearing
            // non-template construct, e.g. @editable-*), not the retired capture field.
            if fm.captures.contains_key("body")
                && let Some(scope) = template_construct_scope(fm, scopes)
            {
                for tref in &scope.refs {
                    if tref.is_collection {
                        let ref_name = tref.ref_name.as_deref().unwrap_or("(anonymous)");
                        let diag = Diagnostic::error(
                                DiagnosticCode::E0918,
                                format!(
                                    "&{}[] collection ref used outside @each context",
                                    ref_name
                                ),
                            )
                            .with_span(fm.span.into())
                            .with_hint(
                                "Collection refs (&name[]) are only valid inside @each blocks. Use a singular ref (&name) instead.".to_string(),
                            );
                        diagnostics.push(diag);
                    }
                }
            }
            continue;
        }

        // Template/template-inline: only check for E0919 (duplicate refs). Refs are
        // read from the World-A Construct scope (not the retired capture field).
        let Some(scope) = template_construct_scope(fm, scopes) else {
            continue;
        };

        // E0919: duplicate ref names in same scope
        let mut seen_refs: HashMap<String, bool> = HashMap::new();
        for tref in &scope.refs {
            if let Some(ref_name) = &tref.ref_name {
                if seen_refs.contains_key(ref_name) {
                    let diag = Diagnostic::error(
                        DiagnosticCode::E0919,
                        format!(
                            "Duplicate template instance name &{} in same scope",
                            ref_name
                        ),
                    )
                    .with_span(fm.span.into())
                    .with_hint(format!(
                        "Each template ref must have a unique name. Rename one of the &{} instances.",
                        ref_name
                    ));
                    diagnostics.push(diag);
                } else {
                    seen_refs.insert(ref_name.clone(), true);
                }
            }
        }
    }

    diagnostics
}

/// Rewrite each match's raw namespace qualifier to the canonical key of the
/// namespace it resolves to in `scope` (FEAT-118 FUP-057). `@s/camera` (alias
/// `s`) and `@scene/camera` (path-suffix of an open `std:scene`) both carry a
/// qualifier that does not match the registry's collection-prefixed key; after
/// this pass the qualifier IS that key (`std:scene`), so the FQN-first lookup in
/// `resolve.rs` hits. A qualifier that resolves to nothing is left untouched —
/// the validation pass reports it (E0926); resolution then falls through to the
/// bare path, preserving today's lenient behaviour.
fn canonicalize_qualifiers(
    matches: &[FormMatch],
    scope: &crate::metasystem::module::ImportScope,
    registry: &MetaRegistry,
) -> Vec<FormMatch> {
    matches
        .iter()
        .map(|fm| {
            if !fm.namespace_qualifier.is_empty() {
                // Qualified: resolve the qualifier to its namespace key.
                return match scope.resolve_qualifier(&fm.namespace_qualifier) {
                    Some(ns) => {
                        let mut out = fm.clone();
                        out.namespace_qualifier = vec![ns.key()];
                        out
                    }
                    None => fm.clone(),
                };
            }
            // Bare call: a module `@use`'d into this file whose `%using` hook
            // `%claims` this directive ACTIVELY extends the scope (Elixir
            // `__using__`) — the bare `@camera` binds to the claiming module as
            // if implicitly qualified. This is the scoped namespace-rewrite the
            // `%claims` clause declares, enacted via the same canonical pass.
            if let Some(ns) = claiming_namespace(&fm.macro_name, scope, registry) {
                let mut out = fm.clone();
                out.namespace_qualifier = vec![ns.key()];
                return out;
            }
            fm.clone()
        })
        .collect()
}

/// Find the open `@use`'d namespace whose `%using` hook `%claims` `directive`,
/// if exactly one does (FEAT-118 FUP-056). The claim is what lets a bare call
/// bind to an actively-extending module without an explicit qualifier. Returns
/// `None` when no open namespace claims it, or more than one does (the latter
/// stays a bare call — E0924 ambiguity territory).
fn claiming_namespace<'a>(
    directive: &str,
    scope: &'a crate::metasystem::module::ImportScope,
    registry: &MetaRegistry,
) -> Option<&'a crate::metasystem::module::Namespace> {
    let mut hit = None;
    for ns in &scope.open {
        let claims = registry
            .using_hook(&ns.key())
            .is_some_and(|h| h.claims.iter().any(|c| c == directive));
        if claims {
            if hit.is_some() {
                return None; // two claimers → ambiguous, leave bare.
            }
            hit = Some(ns);
        }
    }
    hit
}

/// Validate import binding + visibility against the per-file import scope
/// (FEAT-118 FUP-057). Two error classes:
///
/// - **E0926 unbound qualifier**: `@q/name` whose qualifier `q` is neither an
///   `as`-alias nor a path-suffix of any `@use`'d namespace — a typo or missing
///   `@use`.
/// - **E0927 visibility**: a name excluded by the importing `@use`'s
///   `only (...)` allow-list or removed by its `hiding (...)` deny-list. Checked
///   for qualified calls (the namespace is known from the qualifier) and for
///   bare calls that resolve into exactly one open namespace.
///
/// Reads RAW qualifiers (before [`canonicalize_qualifiers`] rewrites them).
fn check_import_visibility(
    matches: &[FormMatch],
    scope: &crate::metasystem::module::ImportScope,
    registry: &MetaRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for fm in matches {
        if !fm.namespace_qualifier.is_empty() {
            // Qualified: the qualifier must bind, and the leaf must be visible.
            match scope.resolve_qualifier(&fm.namespace_qualifier) {
                None => {
                    let q = fm.namespace_qualifier.join("/");
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::E0926,
                            format!(
                                "unbound import qualifier `{}` in `@{}/{}`",
                                q, q, fm.macro_name
                            ),
                        )
                        .with_span(fm.span.into())
                        .with_hint(format!(
                            "`{}` is not an alias or imported namespace — add `@use \"…\" as {}` or qualify with an imported module",
                            q, q
                        )),
                    );
                }
                Some(ns) => {
                    if !scope.is_visible(ns, &fm.macro_name) {
                        diagnostics.push(import_invisible_diag(fm, ns));
                    }
                }
            }
            continue;
        }

        // Bare call: only checkable when the name lives in exactly ONE open
        // namespace (otherwise it is global or ambiguous — E0924's concern).
        let open_owners: Vec<&crate::metasystem::module::Namespace> = scope
            .open
            .iter()
            .filter(|ns| {
                let key =
                    crate::metasystem::module::Fqn::new((*ns).clone(), fm.macro_name.clone()).key();
                registry.get_macro(&key).is_some()
            })
            .collect();
        if let [ns] = open_owners.as_slice()
            && !scope.is_visible(ns, &fm.macro_name)
        {
            diagnostics.push(import_invisible_diag(fm, ns));
        }
    }

    diagnostics
}

/// Build the E0927 “name not visible” diagnostic for a reference to `name` in
/// namespace `ns`, distinguishing the `only` vs `hiding` cause for the hint.
fn import_invisible_diag(fm: &FormMatch, ns: &crate::metasystem::module::Namespace) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::E0927,
        format!(
            "`@{}` is not visible — excluded by the `@use \"{}\"` import list",
            fm.macro_name,
            ns.key()
        ),
    )
    .with_span(fm.span.into())
    .with_hint("add it to the `only (…)` list, or remove it from `hiding (…)`".to_string())
}

/// Check for ambiguous NAMESPACE dispatch (E0924, FEAT-118 FUP-055).
///
/// A BARE (unqualified) directive call is ambiguous when two or more imported
/// modules export the same directive name (their macros share a form-directive
/// but live in DISTINCT namespaces). Resolution silently first-matches; the
/// author should qualify (`@alias/name`). Qualified calls (those carrying a
/// namespace_qualifier) are already disambiguated, so they're skipped.
fn check_namespace_ambiguity(matches: &[FormMatch], registry: &MetaRegistry) -> Vec<Diagnostic> {
    use std::collections::BTreeSet;
    let mut diagnostics = Vec::new();

    for fm in matches {
        // A qualified call is unambiguous by construction.
        if !fm.namespace_qualifier.is_empty() {
            continue;
        }
        let candidates = registry.get_all_macros_by_form_directive(&fm.macro_name);
        if candidates.len() < 2 {
            continue;
        }
        // Collect the DISTINCT namespaces the candidates live in. Only a clash
        // ACROSS namespaces is ambiguous (same-namespace overloads are a
        // different, score-based concern handled by E0923).
        let namespaces: BTreeSet<String> = candidates
            .iter()
            .filter_map(|m| m.module.as_ref())
            .filter(|ns| !ns.is_global())
            .map(|ns| ns.path.join("/"))
            .collect();
        if namespaces.len() < 2 {
            continue;
        }
        let ns_list = namespaces.into_iter().collect::<Vec<_>>().join(", ");
        let diag = Diagnostic::warning(
            DiagnosticCode::E0924,
            format!(
                "ambiguous directive `@{}` — exported by modules: {}",
                fm.macro_name, ns_list
            ),
        )
        .with_span(fm.span.into())
        .with_hint(format!(
            "qualify the call to pick a module, e.g. `@{}/{}`",
            ns_list.split(',').next().unwrap_or("module").trim(),
            fm.macro_name
        ));
        diagnostics.push(diag);
    }

    diagnostics
}

/// Check for `<-` content injection syntax used outside a template/component body context (E0909).
///
/// Content injection (`text <- $var`) is only valid inside a @template component body.
/// If it appears at file level (inside a selector scope but not in a template body),
/// emit E0909.
fn check_injection_outside_template(matches: &[FormMatch]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for fm in matches {
        // Only check non-template FormMatches that have a selector (file-level scope blocks)
        if fm.macro_name == "template" || fm.macro_name == "template-inline" {
            continue;
        }
        // `@handle` (and the @data signal/stream it consumes) carry `<-` MUTATIONS
        // in their effect clauses (optimistic/receive/final, send/receive) — these
        // are legitimate reactive-output statements, not content injection. Skip
        // the injection check for them (PLAN-038 W1). The clause bodies are
        // captured raw and lowered by the signal-handler / signal-call primitives.
        // `@test` joins the skip list for the same reason (PLAN-077 W4): its
        // body capture is opaque harness text that legitimately holds `<-`
        // mutations inside @given/@on blocks.
        // `@doc` joins it too (SIP-002): its `content` capture is opaque PROSE
        // (markdown rendered by the md module), so a doc that merely QUOTES a
        // binding — `text <- $count` in a code span — is not an injection. This
        // also removes an asymmetry: `@doc(src: "x.md")` already passed while
        // the same prose inline via `content:` false-errored.
        if fm.macro_name == "handle"
            || fm.macro_name == "data"
            || fm.macro_name == "test"
            || fm.macro_name == "doc"
        {
            continue;
        }

        // Check StyleProperties captures for <- patterns
        for value in fm.captures.values() {
            let has_injection = match value {
                CapturedValue::StyleProperties(props) => {
                    props.iter().any(|(_, v)| v.contains("<-"))
                }
                CapturedValue::String(s) => s.contains(" <- "),
                CapturedValue::Expr(s) => s.contains(" <- "),
                _ => false,
            };
            if has_injection {
                let diag = Diagnostic::error(
                    DiagnosticCode::E0909,
                    "`<-` content injection used outside component body context".to_string(),
                )
                .with_span(fm.span.into())
                .with_hint(
                    "Content injection (`text <- $var`) is only valid inside a @template component body. Move this into a @template definition.".to_string(),
                );
                diagnostics.push(diag);
                break; // One diagnostic per FormMatch
            }
        }
    }

    diagnostics
}

/// Check for @exports declarations that reference undefined state variables (E0917).
///
/// For each template with an @exports block, verifies that every declared $var
/// actually exists in the template's state declarations. This catches typos and
/// stale exports that reference removed state variables.
fn check_export_refs(
    matches: &[FormMatch],
    scopes: &[crate::parser::ScopeBlock],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for fm in matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }

        // World-A: exports + declared states come from the template's Construct scope.
        let Some(scope) = template_construct_scope(fm, scopes) else {
            continue;
        };
        if scope.exports.is_empty() {
            continue;
        }
        let declared_states: HashSet<String> = scope_declared_state_names(scope);

        // E0917: Check each export declaration against state declarations
        for export in &scope.exports {
            if !declared_states.contains(&export.var_name) {
                let template_name = fm.get_ident("name").unwrap_or("(anonymous)");
                let available: Vec<&str> = declared_states.iter().map(|s| s.as_str()).collect();
                let similar = find_similar_multiple(&export.var_name, &available, 3);
                let mut diag = Diagnostic::error(
                    DiagnosticCode::E0917,
                    format!(
                        "@exports declares ${} which is not defined in template &{}",
                        export.var_name, template_name
                    ),
                )
                .with_span(fm.span.into());

                if let Some(hint) = format_suggestion(&similar) {
                    diag = diag.with_hint(hint);
                } else if !available.is_empty() {
                    diag = diag.with_hint(format!(
                        "Available state variables: {}",
                        available
                            .iter()
                            .map(|s| format!("${}", s))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                } else {
                    diag = diag.with_hint(format!(
                        "Declare the state variable first: ${} type: value;",
                        export.var_name
                    ));
                }

                diagnostics.push(diag);
            }
        }
    }

    diagnostics
}

/// Check for $var declarations that shadow a parent scope variable (W0702).
///
/// Walks template scopes and checks if any declared state variable also exists in
/// a parent template's scope. Currently `parent` is always `None` (nested template
/// resolution is deferred), so this will only fire when parent resolution is wired.
/// The check is implemented and ready.
fn check_scope_shadowing(
    matches: &[FormMatch],
    scopes: &[crate::parser::ScopeBlock],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect global state names
    let global_state_names: HashSet<String> = matches
        .iter()
        .filter(|fm| {
            (fm.macro_name == "local-state" || fm.macro_name == "local-state-uninitialized")
                && fm.selector.is_none()
        })
        .filter_map(|fm| fm.get_ident("name").map(|s| s.to_string()))
        .collect();

    // Build a map of template name → declared state var names (World A: from the
    // template's Construct scope, not the retired ComponentBody capture).
    let mut template_states: HashMap<String, HashSet<String>> = HashMap::new();
    let mut template_parents: HashMap<String, Option<String>> = HashMap::new();

    for fm in matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }

        let template_name = fm.get_ident("name").unwrap_or_default().to_string();
        let Some(scope) = template_construct_scope(fm, scopes) else {
            continue;
        };
        template_states.insert(template_name.clone(), scope_declared_state_names(scope));
        // Parent resolution is currently None; ready for when it's wired
        template_parents.insert(template_name, None);
    }

    // Check each template's states against its parent chain
    for fm in matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }

        let template_name = fm.get_ident("name").unwrap_or_default().to_string();
        let Some(scope) = template_construct_scope(fm, scopes) else {
            continue;
        };
        let declared: Vec<String> = scope_declared_state_names(scope).into_iter().collect();

        for var_name in &declared {
            // Check against global scope
            if global_state_names.contains(var_name) {
                let diag = Diagnostic::warning(
                    DiagnosticCode::W0702,
                    format!(
                        "${} in template &{} shadows a global scope variable",
                        var_name, template_name
                    ),
                )
                .with_span(fm.span.into())
                .with_hint(format!(
                    "The global ${} will be hidden inside this template. Consider renaming to avoid confusion.",
                    var_name
                ));
                diagnostics.push(diag);
            }

            // Walk parent chain (when parent resolution is wired)
            let mut current_parent = template_parents.get(&template_name).and_then(|p| p.clone());
            while let Some(ref parent_name) = current_parent {
                if let Some(parent_states) = template_states.get(parent_name)
                    && parent_states.contains(var_name)
                {
                    let diag = Diagnostic::warning(
                        DiagnosticCode::W0702,
                        format!(
                            "${} in template &{} shadows the same variable in parent template &{}",
                            var_name, template_name, parent_name
                        ),
                    )
                    .with_span(fm.span.into())
                    .with_hint(format!(
                        "Consider renaming ${} to avoid confusion with the parent's variable.",
                        var_name
                    ));
                    diagnostics.push(diag);
                }
                current_parent = template_parents.get(parent_name).and_then(|p| p.clone());
            }
        }
    }

    diagnostics
}

/// Check for $var declared but never read in template scope (W0703).
///
/// For each template, collects all declared state variable names and all referenced
/// variable names from directives, injections, and exports. Any declared var not
/// referenced anywhere emits W0703.
fn check_unused_state_vars(
    matches: &[FormMatch],
    scopes: &[crate::parser::ScopeBlock],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for fm in matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }

        let template_name = fm.get_ident("name").unwrap_or("(anonymous)");
        // FEAT-115 (Q4): states, directives, AND exports all come from the template's
        // World-A Construct scope — not the retired ComponentBody capture.
        let Some(scope) = template_construct_scope(fm, scopes) else {
            continue;
        };
        let declared_set = scope_declared_state_names(scope);

        // Skip templates with no state declarations
        if declared_set.is_empty() {
            continue;
        }
        // Deterministic W0703 order (HashSet iteration is unordered).
        let mut declared: Vec<&String> = declared_set.iter().collect();
        declared.sort();

        // Collect all referenced var names: directives (World-A scope) + exports
        // (exported vars count as "read" — exposed for external use).
        let mut referenced: HashSet<String> = scope_referenced_state_vars(scope)
            .into_iter()
            .map(|(v, _)| v)
            .collect();
        for export in &scope.exports {
            referenced.insert(export.var_name.clone());
        }

        // Check for unreferenced declared vars
        for &state_var in &declared {
            if !referenced.contains(state_var) {
                let diag = Diagnostic::warning(
                    DiagnosticCode::W0703,
                    format!(
                        "${} is declared but never read in template &{}",
                        state_var, template_name
                    ),
                )
                .with_span(fm.span.into())
                .with_hint("Remove the unused state variable, or reference it in a directive, injection, or @exports block.".to_string());
                diagnostics.push(diag);
            }
        }
    }

    diagnostics
}

/// Check for mixing @bind with reactive properties in the same template (W0706).
///
/// If a template has both `@bind` FormMatches targeting it AND reactive properties
/// (ClassToggle, ContentBinding) in its component body, emit W0706.
fn check_mixed_bind_reactive(
    matches: &[FormMatch],
    scopes: &[crate::parser::ScopeBlock],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect selectors that have @bind FormMatches
    let bind_selectors: HashSet<String> = matches
        .iter()
        .filter(|fm| fm.macro_name == "bind")
        .filter_map(|fm| fm.selector.clone())
        .collect();

    // Check template bodies for reactive properties
    for fm in matches {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }

        let template_name = fm.get_ident("name").unwrap_or("(anonymous)");
        // FEAT-115 S5c: "has reactive directives" (class toggle / content binding)
        // is read from the World-A scope, not reify's `body.directives`.
        let scope = scopes.iter().find(|s| {
            matches!(&s.kind, crate::parser::ast::ScopeKind::Construct(_))
                && s.selector == format!("@template:{}", template_name)
        });
        let Some(scope) = scope else { continue };
        let has_reactive = scope_referenced_state_vars(scope)
            .iter()
            .any(|(_, kind)| *kind == "class" || *kind == "content");

        if !has_reactive {
            continue;
        }

        // Check if any @bind targets the same selector as this template
        let template_selector = fm.selector.as_deref();

        // Match by selector (if template has one) or check if any @bind exists in same file
        let has_bind = if let Some(sel) = template_selector {
            bind_selectors.contains(sel)
        } else {
            // Template at file level — check if any @bind also at file level
            matches
                .iter()
                .any(|m| m.macro_name == "bind" && m.selector.is_none())
        };

        if has_bind {
            let diag = Diagnostic::warning(
                DiagnosticCode::W0706,
                format!(
                    "Template &{} mixes @bind with reactive properties (.class: $cond, text <- $var)",
                    template_name
                ),
            )
            .with_span(fm.span.into())
            .with_hint(
                "Migrate @bind directives to reactive properties for consistency. @bind is deprecated.".to_string(),
            );
            diagnostics.push(diag);
        }
    }

    diagnostics
}

/// Verbose output with intermediate layer results
#[derive(Debug, Clone)]
pub struct VerboseOutput {
    /// Resolved and sorted primitives
    pub primitives: Vec<ResolvedPrimitive>,

    /// Typed expanded primitives
    pub expanded: Vec<ExpandedPrimitive>,

    /// Final output
    pub output: PipelineOutput,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use crate::syntax::CapturedValue;
    use std::collections::HashMap;

    // PLAN-054 audit #6: %animates property classification.
    #[test]
    fn test_transform_token_template_known_channels() {
        assert!(transform_token_template("translate-x").is_some());
        assert!(transform_token_template("translate-y").is_some());
        assert!(transform_token_template("scale").is_some());
        assert!(transform_token_template("rotate").is_some());
        // Unknown / typo'd transform-ish props are NOT channels.
        assert!(transform_token_template("skew").is_none());
        assert!(transform_token_template("translate").is_none());
        assert!(transform_token_template("wobble").is_none());
    }

    #[test]
    fn test_is_animatable_style_property() {
        // Recognized animatable CSS props -> safe ST.bindStyle fallback.
        assert!(is_animatable_style_property("opacity"));
        assert!(is_animatable_style_property("width"));
        assert!(is_animatable_style_property("--my-channel"));
        // Unknown props -> not animatable (will warn W0713, not emit).
        assert!(!is_animatable_style_property("skew"));
        assert!(!is_animatable_style_property("frobnicate"));
    }

    /// Build a minimal EvaluatedMatch carrying a single %animates channel for the
    /// given property (driven by signal `s`), with no derives.
    fn animate_match(selector: &str, property: &str) -> evaluate::EvaluatedMatch {
        let mut fm = crate::syntax::FormMatch::new("x");
        fm.selector = Some(selector.to_string());
        evaluate::EvaluatedMatch {
            binds_consumed: false,
            form_match: fm,
            bind_decls: Vec::new(),
            emit_blocks: Vec::new(),
            states: None,
            on_clauses: Vec::new(),
            derives: Vec::new(),
            animates: vec![evaluate::EvaluatedAnimate {
                property: property.to_string(),
                signal: "s".to_string(),
            }],
            body_was_evaluated: true,
        }
    }

    #[test]
    fn test_animate_unknown_property_warns_and_skips() {
        // PLAN-054 audit #6: an unknown animate property warns W0713 and emits NO
        // binding (no broken `el.style[skew]` write).
        let evs = vec![animate_match(".box", "skew")];
        let mut frags = Vec::new();
        let diags = generate_animate_fragments(&evs, &mut frags);
        assert_eq!(diags.len(), 1, "unknown animate prop must warn once");
        assert_eq!(diags[0].code, DiagnosticCode::W0713);
        // No fragment emitted (no transform channel, no animatable bindStyle).
        assert!(
            frags.is_empty(),
            "unknown animate prop must emit nothing, got: {:?}",
            frags
        );
    }

    #[test]
    fn test_animate_transform_channel_emits_no_warning() {
        let evs = vec![animate_match(".box", "translate-x")];
        let mut frags = Vec::new();
        let diags = generate_animate_fragments(&evs, &mut frags);
        assert!(diags.is_empty(), "a known transform channel must not warn");
        assert_eq!(frags.len(), 1, "a transform channel must emit a fragment");
        let js: String = frags[0]
            .stmts
            .iter()
            .map(|s| match s {
                JsStmt::Raw(r) => r.clone(),
                _ => String::new(),
            })
            .collect();
        assert!(js.contains("ST.bindTransform"), "got: {}", js);
    }

    #[test]
    fn test_animate_known_style_property_binds_no_warning() {
        let evs = vec![animate_match(".box", "opacity")];
        let mut frags = Vec::new();
        let diags = generate_animate_fragments(&evs, &mut frags);
        assert!(diags.is_empty(), "opacity is animatable, no warning");
        let js: String = frags[0]
            .stmts
            .iter()
            .map(|s| match s {
                JsStmt::Raw(r) => r.clone(),
                _ => String::new(),
            })
            .collect();
        assert!(
            js.contains("ST.bindStyle(el, \"s\", \"opacity\")"),
            "got: {}",
            js
        );
    }

    #[test]
    fn test_compile_empty() {
        let context = CompileContext::default();
        let result = compile(&[], &context);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.js, "");
        assert_eq!(output.css, "");
    }

    #[test]
    fn test_compile_single_form() {
        let context = probe_context(&["data-fetch"]);
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("myData".to_string()),
        );

        let form = FormMatch {
            macro_name: "data-fetch".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = compile(&[form], &context);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should have some JS output
        assert!(!output.js.is_empty());
    }

    #[test]
    fn test_compile_with_runtime() {
        let context = probe_context(&["local-state"]);
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("counter".to_string()),
        );

        let form = FormMatch {
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

        let result = compile(&[form], &context);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should include real ST runtime (from public/runtime/st.js)
        assert!(output.js.contains("Spacetime Core Runtime"));
        assert!(output.js.contains("global.ST = ST"));
    }

    #[test]
    fn test_compile_without_runtime() {
        let context = probe_context(&["local-state"]).without_runtime();
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("counter".to_string()),
        );

        let form = FormMatch {
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

        let result = compile(&[form], &context);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should NOT include runtime (no "Spacetime Core Runtime" header)
        assert!(!output.js.contains("Spacetime Core Runtime"));
    }

    #[test]
    fn test_compile_multiple_forms() {
        let context = probe_context(&["data-fetch", "local-state"]).without_runtime();

        let forms = vec![
            FormMatch {
                macro_name: "data-fetch".to_string(),
                matched_macro: None,
                captures: {
                    let mut map = HashMap::new();
                    map.insert(
                        "name".to_string(),
                        CapturedValue::Ident("users".to_string()),
                    );
                    map
                },
                capture_spans: HashMap::new(),
                selector: None,
                span: SourceSpan::default(),
                source_file: None,
                namespace_qualifier: Vec::new(),
                doc: None,
            },
            FormMatch {
                macro_name: "local-state".to_string(),
                matched_macro: None,
                captures: {
                    let mut map = HashMap::new();
                    map.insert(
                        "name".to_string(),
                        CapturedValue::Ident("count".to_string()),
                    );
                    map
                },
                capture_spans: HashMap::new(),
                selector: None,
                span: SourceSpan::default(),
                source_file: None,
                namespace_qualifier: Vec::new(),
                doc: None,
            },
        ];

        let result = compile(&forms, &context);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Should have output from both forms
        assert!(!output.js.is_empty());
    }

    #[test]
    fn test_compile_verbose() {
        let context = probe_context(&["data-fetch"]).without_runtime();
        let mut captures = HashMap::new();
        captures.insert("name".to_string(), CapturedValue::Ident("test".to_string()));

        let form = FormMatch {
            macro_name: "data-fetch".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = compile_verbose(&[form], &context);
        assert!(result.is_ok());

        let verbose = result.unwrap();
        assert_eq!(verbose.primitives.len(), 1);
        assert_eq!(verbose.expanded.len(), 1);
        assert!(!verbose.output.js.is_empty());
    }

    #[test]
    fn test_phase_ordering() {
        let context = probe_context(&["element-ref", "data-fetch"]).without_runtime();

        // Without a loaded MetaRegistry, fallback gives Phase::Global for all
        // macros. Both resolve to Global with the same order (500).
        let forms = vec![
            FormMatch {
                macro_name: "element-ref".to_string(),
                matched_macro: None,
                captures: HashMap::new(),
                capture_spans: HashMap::new(),
                selector: Some(".button".to_string()),
                span: SourceSpan::default(),
                source_file: None,
                namespace_qualifier: Vec::new(),
                doc: None,
            },
            FormMatch {
                macro_name: "data-fetch".to_string(),
                matched_macro: None,
                captures: HashMap::new(),
                capture_spans: HashMap::new(),
                selector: None,
                span: SourceSpan::default(),
                source_file: None,
                namespace_qualifier: Vec::new(),
                doc: None,
            },
        ];

        let result = compile_verbose(&forms, &context);
        assert!(result.is_ok());

        let verbose = result.unwrap();
        // With empty registry, both fall through to Global phase
        assert_eq!(verbose.primitives[0].phase, Phase::Global);
        assert_eq!(verbose.primitives[1].phase, Phase::Global);
    }

    // =========================================================================
    // Template reference validation (E0402)
    // =========================================================================

    /// Helper: create a register-template ResolvedPrimitive
    fn make_register_template(name: &str) -> ResolvedPrimitive {
        let mut args = HashMap::new();
        args.insert("name".to_string(), CapturedValue::Ident(name.to_string()));
        args.insert("body".to_string(), CapturedValue::ComponentBody);
        ResolvedPrimitive::new("register-template", args, Phase::Global, 0)
    }

    /// Helper: create an invoke-template ResolvedPrimitive
    fn make_invoke_template(name: &str) -> ResolvedPrimitive {
        let mut args = HashMap::new();
        args.insert("name".to_string(), CapturedValue::Ident(name.to_string()));
        ResolvedPrimitive::new("invoke-template", args, Phase::Selector, 200_000)
            .with_span(SourceSpan { start: 10, end: 20 })
    }

    #[test]
    fn test_e0402_valid_template_ref() {
        let primitives = vec![make_register_template("card"), make_invoke_template("card")];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert!(diags.is_empty(), "Expected no diagnostics for valid ref");
    }

    #[test]
    fn test_e0402_missing_template() {
        let primitives = vec![make_invoke_template("nonexistent")];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, crate::diagnostics::DiagnosticCode::E0402);
        assert!(diags[0].message.contains("nonexistent"));
    }

    #[test]
    fn test_e0402_did_you_mean_hint() {
        let primitives = vec![make_register_template("card"), make_invoke_template("crad")];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].hint.as_ref().unwrap().contains("card"),
            "Expected 'did you mean' hint containing 'card', got: {:?}",
            diags[0].hint
        );
    }

    #[test]
    fn test_e0402_multiple_defs_valid() {
        let primitives = vec![
            make_register_template("card"),
            make_register_template("badge"),
            make_invoke_template("card"),
            make_invoke_template("badge"),
        ];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert!(
            diags.is_empty(),
            "Expected no diagnostics when all refs valid"
        );
    }

    /// Helper: a primitive carrying a raw block-source String capture (mimics the
    /// `@test`/`@mount` body that survives as a String at validate time).
    fn make_block_source(body: &str) -> ResolvedPrimitive {
        let mut args = HashMap::new();
        args.insert("body".to_string(), CapturedValue::String(body.to_string()));
        ResolvedPrimitive::new("test", args, Phase::Global, 0)
    }

    #[test]
    fn test_e0402_mount_block_template_def_admitted() {
        // BUG-105: a `@template &card` defined inside a `@mount` block source (which
        // compiles in a separate sub-program, so its register-template is NOT in the
        // outer primitives) must be admitted to `defined` by the source scan — a
        // same-block `&card()` invoke must NOT false-positive E0402.
        let block = "@mount {\n  @template &card($x){<div class=\"card\">x</div>}\n  .host { &card(1); }\n}";
        let primitives = vec![make_block_source(block), make_invoke_template("card")];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert!(
            diags.is_empty(),
            "a @template defined inside a @mount block must be admitted, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_e0402_mount_block_undefined_still_errors() {
        // The fix must NOT suppress a genuinely-undefined ref: `&ghost()` with no
        // matching `@template &ghost` definition anywhere still raises E0402.
        let block = "@mount {\n  @template &card($x){<div>x</div>}\n}";
        let primitives = vec![make_block_source(block), make_invoke_template("ghost")];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert_eq!(diags.len(), 1, "undefined &ghost must still error");
        assert_eq!(diags[0].code, crate::diagnostics::DiagnosticCode::E0402);
    }

    #[test]
    fn test_e0402_two_missing_invocations() {
        let primitives = vec![
            make_invoke_template("missing-one"),
            make_invoke_template("missing-two"),
        ];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert_eq!(diags.len(), 2, "Expected two E0402 diagnostics");
    }

    #[test]
    fn test_e0402_no_templates_no_diagnostics() {
        let primitives = vec![];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_e0402_span_from_invocation() {
        let primitives = vec![make_invoke_template("missing")];
        let diags = validate_template_refs(&primitives, &std::collections::HashMap::new());
        assert_eq!(diags.len(), 1);
        let span = diags[0].span.as_ref().expect("diagnostic should have span");
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
    }

    // =========================================================================
    // E0905: $var reference to undefined state variable
    // =========================================================================

    /// Helper: create a template FormMatch with component body
    // FEAT-119: the factory payload (states/refs) lives on the World-A scope, not the
    // capture (now a validation channel). This helper returns the template FormMatch
    // plus a matching `@template:<name>` ScopeBlock carrying the states/refs, so tests
    // exercise the validators through the real World-A path.
    fn make_template_fm(
        name: &str,
        states: Vec<crate::syntax::ComponentStateDecl>,
        refs: Vec<crate::syntax::TemplateRef>,
    ) -> (FormMatch, crate::parser::ScopeBlock) {
        let mut captures = HashMap::new();
        captures.insert("name".to_string(), CapturedValue::Ident(name.to_string()));
        captures.insert("body".to_string(), CapturedValue::ComponentBody);
        let fm = FormMatch {
            macro_name: "template".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let scope = crate::parser::ScopeBlock {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector: format!("@template:{}", name),
            html: "<div></div>".to_string(),
            states,
            refs,
            ..Default::default()
        };
        (fm, scope)
    }

    #[test]
    fn test_e0905_undefined_class_toggle_var() {
        // FEAT-115 S5c: E0905 now reads the World-A scope tree, so the tests parse
        // real source instead of hand-building reify directives. A body-root class
        // toggle referencing an undeclared `$isActive` must surface E0905.
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\n.d--active: $isActive; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_undefined_state_refs(&f.matches, &f.scopes);
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0905);
        assert!(diags[0].message.contains("isActive"));
    }

    #[test]
    fn test_e0905_defined_var_no_diagnostic() {
        // A class toggle referencing a DECLARED body state must NOT trigger E0905.
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\n$isActive bool: false;\n.d--active: $isActive; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_undefined_state_refs(&f.matches, &f.scopes);
        assert!(
            diags.is_empty(),
            "Defined var should not trigger E0905: {:?}",
            diags
        );
    }

    #[test]
    fn test_e0905_parent_scope_var_allowed() {
        // A template referencing a FILE-LEVEL (global) state var must NOT trigger
        // E0905 — global states are admitted across scopes.
        let f = crate::parse(
            "$globalVar bool: false;\n@template &t() { <div class=\"d\"></div>\n.d--show: $globalVar; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_undefined_state_refs(&f.matches, &f.scopes);
        assert!(
            diags.is_empty(),
            "Parent scope var should not trigger E0905: {:?}",
            diags
        );
    }

    #[test]
    fn test_e0905_undefined_injection_var() {
        // A body-root content injection (`text <- $undeclared`) referencing an
        // undeclared state must surface E0905 (now via the synthesized World-A scope).
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\ntext <- $undeclared; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_undefined_state_refs(&f.matches, &f.scopes);
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0905);
        assert!(diags[0].message.contains("undeclared"));
    }

    #[test]
    fn test_e0965_undeclared_element_ref() {
        // BUG-333: `.card { $x: &$b; }` with no `&b <sel>;` declaration is an
        // E0965 (an undeclared element reference), not a dead bundle.
        let f = crate::parse(
            "@import \"stdlib/syntax/element-ref.st\"\n.card { $x: &$b; }\n<div class=\"card\">h</div>",
        )
        .expect("parse");
        let diags = check_undeclared_element_refs(&f.scopes);
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0965);
        assert!(
            diags[0].message.contains("b"),
            "must name the undeclared ref: {:?}",
            diags[0].message
        );
    }

    #[test]
    fn test_e0965_declared_element_ref_ok() {
        // A declared `&b .some-class;` in the same scope resolves `&$b` — no E0965.
        let f = crate::parse(
            "@import \"stdlib/syntax/element-ref.st\"\n.card { &b .some-class; $x: &$b; }\n<div class=\"card\">h</div>",
        )
        .expect("parse");
        let diags = check_undeclared_element_refs(&f.scopes);
        assert!(diags.is_empty(), "{:?}", diags);
    }

    #[test]
    fn test_e0965_self_and_dotted_rect_not_flagged() {
        // `&self.rect.width` and `&$b.rect.width` (declared `b`) must not trip the
        // diagnostic: `&self` has no `$`, and a declared `&b` resolves `&$b.rect`.
        let f = crate::parse(
            "@import \"stdlib/syntax/element-ref.st\"\n.card { &b .some-class; $x: &self.rect.width; $y: &$b.rect.width; }\n<div class=\"card\">h</div>",
        )
        .expect("parse");
        let diags = check_undeclared_element_refs(&f.scopes);
        assert!(diags.is_empty(), "{:?}", diags);
    }

    #[test]
    fn test_e0905_undefined_event_action_var() {
        // An `@on` action mutating an undeclared state (`$undefined <- !$undefined`)
        // must surface E0905 (var ref read from the `on` match's js_statements).
        let f = crate::parse(
            "@template &t() { <button class=\"b\"></button>\n.b { @on &.click { $undefined <- !$undefined; } } }\n<main></main>",
        )
        .expect("parse");
        let diags = check_undefined_state_refs(&f.matches, &f.scopes);
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::E0905),
            "{:?}",
            diags
        );
    }

    // =========================================================================
    // E0909: <- used outside selector/component context
    // =========================================================================

    #[test]
    fn test_e0909_injection_outside_template() {
        // Simulate a FormMatch at file level with StyleProperties containing <-
        let fm = FormMatch {
            macro_name: "css-property".to_string(),
            matched_macro: None,
            captures: {
                let mut map = HashMap::new();
                map.insert(
                    "body".to_string(),
                    CapturedValue::StyleProperties(vec![(
                        "text".to_string(),
                        "<- $title".to_string(),
                    )]),
                );
                map
            },
            capture_spans: HashMap::new(),
            selector: Some(".header".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let diags = check_injection_outside_template(&[fm]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::E0909);
    }

    #[test]
    fn test_e0909_injection_inside_template_allowed() {
        // Template FormMatches should not trigger E0909
        let (fm, _scope) = make_template_fm(
            "test",
            vec![crate::syntax::ComponentStateDecl {
                var_name: "count".to_string(),
                type_name: "number".to_string(),
                initial: CapturedValue::Number(0.0),
            }],
            vec![],
        );
        let diags = check_injection_outside_template(&[fm]);
        assert!(
            diags.is_empty(),
            "Injection inside template should not trigger E0909"
        );
    }

    // =========================================================================
    // E0918: &name[] collection ref outside @each
    // E0919: duplicate &name in same scope
    // =========================================================================

    #[test]
    fn test_e0918_collection_ref_outside_each() {
        // Non-template FormMatch with collection ref should trigger E0918
        // (collection refs in template bodies are always allowed since
        // the template can be invoked from @each in another file)
        // FEAT-119: a non-template body-bearing construct (e.g. @editable-*) carries
        // its refs on the World-A scope keyed `@template:<name>`. A collection ref there,
        // outside a template, must trigger E0918.
        let mut captures = HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("widget".to_string()),
        );
        captures.insert("body".to_string(), CapturedValue::ComponentBody);
        let fm = FormMatch {
            macro_name: "non-template-scope".to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: Some(".list".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let scope = crate::parser::ScopeBlock {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector: "@template:widget".to_string(),
            refs: vec![crate::syntax::TemplateRef {
                ref_name: Some("cards".to_string()),
                template_name: "counter".to_string(),
                args: vec![],
                is_collection: true,
                arg_names: vec![],
                span: Default::default(),
            }],
            ..Default::default()
        };
        let diags = check_template_ref_issues(&[fm], &[scope]);
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::E0918),
            "Collection ref outside template body should trigger E0918, got: {:?}",
            diags.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_e0918_collection_ref_in_template_allowed() {
        // Collection ref inside a template body is always allowed
        // because the template can be invoked from @each
        let (template_fm, scope) = make_template_fm(
            "test",
            vec![],
            vec![crate::syntax::TemplateRef {
                ref_name: Some("cards".to_string()),
                template_name: "counter".to_string(),
                args: vec![],
                is_collection: true,
                arg_names: vec![],
                span: Default::default(),
            }],
        );
        let diags = check_template_ref_issues(&[template_fm], &[scope]);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::E0918),
            "Collection ref inside template body should not trigger E0918"
        );
    }

    #[test]
    fn test_e0919_duplicate_ref_name() {
        // E0919 reads refs from the World-A Construct scope — parse real source so the
        // scope is populated (two refs with the same name `counter`).
        let f = crate::parse(
            "@template &slot($x) { <div></div> }\n@template &test() { <div></div>\n&counter &slot($x);\n&counter &slot($y); }\n",
        )
        .expect("parse");
        let diags = check_template_ref_issues(&f.matches, &f.scopes);
        assert!(
            diags.iter().any(|d| d.code == DiagnosticCode::E0919),
            "Duplicate ref name should trigger E0919, got: {:?}",
            diags.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_e0919_unique_ref_names_no_diagnostic() {
        let (fm, scope) = make_template_fm(
            "test",
            vec![],
            vec![
                crate::syntax::TemplateRef {
                    ref_name: Some("a".to_string()),
                    template_name: "counter".to_string(),
                    args: vec![],
                    is_collection: false,
                    arg_names: vec![],
                    span: Default::default(),
                },
                crate::syntax::TemplateRef {
                    ref_name: Some("b".to_string()),
                    template_name: "counter".to_string(),
                    args: vec![],
                    is_collection: false,
                    arg_names: vec![],
                    span: Default::default(),
                },
            ],
        );
        let diags = check_template_ref_issues(&[fm], &[scope]);
        assert!(
            !diags.iter().any(|d| d.code == DiagnosticCode::E0919),
            "Unique ref names should not trigger E0919"
        );
    }

    // =========================================================================
    // W0700: Interleaved HTML and CSS sections
    // =========================================================================

    /// Helper: validate a body and return its diagnostics (FEAT-120: W0700 is now
    /// emitted by the validator directly, not a pipeline drain over a transition count).
    fn validate_body(inner: &str, name: &str) -> Vec<Diagnostic> {
        crate::syntax::events::form_compiler::validate_component_body(inner, 0, name)
    }

    #[test]
    fn test_w0700_interleaved_html_css() {
        // HTML → CSS → HTML interleave (two transitions) trips W0700. FEAT-120: the
        // warning is emitted by the validator, which counts the transitions and has the
        // template name locally.
        let diags = validate_body(
            "<div class=\"c\"></div>\n.c { color: red; }\n<span class=\"c\"></span>",
            "test",
        );
        let w0700: Vec<_> = diags
            .iter()
            .filter(|d| d.code == DiagnosticCode::W0700)
            .collect();
        assert_eq!(
            w0700.len(),
            1,
            "interleaved HTML/CSS should trip W0700 once"
        );
        assert!(w0700[0].message.contains("test"));
    }

    #[test]
    fn test_w0700_no_interleaving() {
        // HTML then CSS (one transition) is the normal grouped shape — no W0700.
        let diags = validate_body("<div class=\"c\"></div>\n.c { color: red; }", "test");
        assert!(
            diags.iter().all(|d| d.code != DiagnosticCode::W0700),
            "a single HTML→CSS transition should not trip W0700"
        );
    }

    #[test]
    fn test_w0700_html_only_no_warning() {
        // HTML only (zero transitions) — no W0700.
        let diags = validate_body("<div class=\"c\"></div>", "test");
        assert!(
            diags.iter().all(|d| d.code != DiagnosticCode::W0700),
            "HTML-only template should not trip W0700"
        );
    }

    // =========================================================================
    // W0702: $var shadows parent scope variable
    // =========================================================================

    #[test]
    fn test_w0702_shadows_global_var() {
        // Global state
        let global_fm = FormMatch {
            macro_name: "local-state".to_string(),
            matched_macro: None,
            captures: {
                let mut map = HashMap::new();
                map.insert(
                    "name".to_string(),
                    CapturedValue::Ident("count".to_string()),
                );
                map
            },
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        // Template that declares a $count too (shadows global). Parse real source so
        // the template's states come from its World-A Construct scope.
        let f =
            crate::parse("@template &test() { <div></div>\n$count number: 0; }\n").expect("parse");
        let _ = global_fm;
        let mut matches = f.matches.clone();
        matches.push(FormMatch {
            macro_name: "local-state".to_string(),
            matched_macro: None,
            captures: {
                let mut map = HashMap::new();
                map.insert(
                    "name".to_string(),
                    CapturedValue::Ident("count".to_string()),
                );
                map
            },
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        });
        let diags = check_scope_shadowing(&matches, &f.scopes);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::W0702);
        assert!(diags[0].message.contains("count"));
        assert!(diags[0].message.contains("global"));
    }

    #[test]
    fn test_w0702_no_shadowing() {
        // Global state $x, template declares $y (different names, no shadowing)
        let global_fm = FormMatch {
            macro_name: "local-state".to_string(),
            matched_macro: None,
            captures: {
                let mut map = HashMap::new();
                map.insert(
                    "name".to_string(),
                    CapturedValue::Ident("globalX".to_string()),
                );
                map
            },
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };
        let (template_fm, scope) = make_template_fm(
            "test",
            vec![crate::syntax::ComponentStateDecl {
                var_name: "localY".to_string(),
                type_name: "number".to_string(),
                initial: CapturedValue::Number(0.0),
            }],
            vec![],
        );
        let diags = check_scope_shadowing(&[global_fm, template_fm], &[scope]);
        assert!(diags.is_empty(), "Different names should not trigger W0702");
    }

    // =========================================================================
    // W0703: $var declared but never read
    // =========================================================================

    #[test]
    fn test_w0703_unused_state_var() {
        // FEAT-115 S5c: W0703 reads states+directives from World-A scopes. A declared
        // body state never referenced by any directive must warn.
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\n$count number: 0; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_unused_state_vars(&f.matches, &f.scopes);
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::W0703);
        assert!(diags[0].message.contains("count"));
    }

    #[test]
    fn test_w0703_used_in_class_toggle() {
        // A state referenced by a body-root class toggle must NOT warn.
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\n$isActive bool: false;\n.d--active: $isActive; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_unused_state_vars(&f.matches, &f.scopes);
        assert!(
            diags.is_empty(),
            "Var used in ClassToggle should not trigger W0703: {:?}",
            diags
        );
    }

    #[test]
    fn test_w0703_used_in_injection() {
        // A state referenced by a body-root content injection must NOT warn.
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\n$title string: \"hello\";\ntext <- $title; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_unused_state_vars(&f.matches, &f.scopes);
        assert!(
            diags.is_empty(),
            "Var used in injection should not trigger W0703: {:?}",
            diags
        );
    }

    #[test]
    fn test_w0703_used_in_exports() {
        // A state only referenced via @exports (exposed for external use) counts as
        // read — must NOT warn. exports stay World-A-sourced on body.exports (S2).
        let f = crate::parse(
            "@template &t() { <div class=\"d\"></div>\n$count number: 0;\n@exports { $count: mut } }\n<main></main>",
        )
        .expect("parse");
        let diags = check_unused_state_vars(&f.matches, &f.scopes);
        assert!(
            diags.is_empty(),
            "Var in @exports should not trigger W0703: {:?}",
            diags
        );
    }

    // =========================================================================
    // W0706: Mixing @bind with reactive properties
    // =========================================================================

    #[test]
    fn test_w0706_bind_with_reactive() {
        // FEAT-115 S5c: reactive-ness is read from the World-A scope. Parse a template
        // with a body-root class toggle; inject a file-level `bind` FormMatch (the
        // deprecated @bind has no fleet authoring surface) to trigger the mix warning.
        let f = crate::parse(
            "@template &test() { <div class=\"d\"></div>\n$active bool: false;\n.d--active: $active; }\n<main></main>",
        )
        .expect("parse");
        let mut matches = f.matches.clone();
        matches.push(FormMatch {
            macro_name: "bind".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None, // file level, same as the template
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        });
        let diags = check_mixed_bind_reactive(&matches, &f.scopes);
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::W0706);
        assert!(diags[0].message.contains("test"));
    }

    #[test]
    fn test_w0706_no_bind_no_warning() {
        // A reactive template with NO @bind must NOT warn.
        let f = crate::parse(
            "@template &test() { <div class=\"d\"></div>\n$active bool: false;\n.d--active: $active; }\n<main></main>",
        )
        .expect("parse");
        let diags = check_mixed_bind_reactive(&f.matches, &f.scopes);
        assert!(
            diags.is_empty(),
            "No @bind should not trigger W0706: {:?}",
            diags
        );
    }

    #[test]
    fn test_states_generate_css_and_js() {
        use crate::parser::meta_ast::{
            MacroDefAst, MetaStateDef, MetaStateProperty, MetaStatesClause,
        };

        // Register a test macro with %states
        let mut registry = MetaRegistry::new();
        let macro_def = MacroDefAst {
            name: "test-state".to_string(),
            states: Some(MetaStatesClause {
                states: vec![
                    MetaStateDef::Named {
                        name: "active".to_string(),
                        condition: Some("$isActive".to_string()),
                        properties: vec![
                            MetaStateProperty {
                                name: "opacity".to_string(),
                                value: "1".to_string(),
                            },
                            MetaStateProperty {
                                name: "transform".to_string(),
                                value: "scale(1.1)".to_string(),
                            },
                        ],
                    },
                    MetaStateDef::Named {
                        name: "idle".to_string(),
                        condition: None,
                        properties: vec![MetaStateProperty {
                            name: "opacity".to_string(),
                            value: "0.5".to_string(),
                        }],
                    },
                ],
                span: SourceSpan::default(),
            }),
            ..Default::default()
        };
        registry.register_macro(macro_def).unwrap();

        let context = CompileContext::new(registry).without_runtime();
        let form = FormMatch {
            macro_name: "test-state".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: Some(".widget".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = compile(&[form], &context);
        assert!(result.is_ok(), "compile failed: {:?}", result.err());

        let output = result.unwrap();

        // CSS should contain state selector for 'active' (has condition)
        assert!(
            output.css.contains("data-st-state=\"active\""),
            "CSS should contain state attribute for 'active', got: {}",
            output.css
        );
        assert!(
            output.css.contains("opacity: 1"),
            "CSS should contain state properties, got: {}",
            output.css
        );

        // JS should contain ST.bindState for the conditioned state
        assert!(
            output.js.contains("ST.bindState"),
            "JS should contain ST.bindState, got: {}",
            output.js
        );
        assert!(
            output.js.contains("$isActive"),
            "JS should reference the condition signal, got: {}",
            output.js
        );

        // 'idle' has no condition — should NOT generate JS binding
        // (no condition means it's a default/initial state, no runtime binding needed)
        assert!(
            !output.js.contains("'idle'"),
            "JS should not bind states without conditions, got: {}",
            output.js
        );
    }

    #[test]
    fn test_states_pattern_match_generates_typed_union() {
        use crate::parser::meta_ast::{
            MacroDefAst, MetaStateDef, MetaStateProperty, MetaStatesClause,
        };

        let mut registry = MetaRegistry::new();
        let macro_def = MacroDefAst {
            name: "test-pattern".to_string(),
            states: Some(MetaStatesClause {
                states: vec![MetaStateDef::Named {
                    name: "connected".to_string(),
                    condition: Some("$ws:Connected{send}".to_string()),
                    properties: vec![MetaStateProperty {
                        name: "color".to_string(),
                        value: "green".to_string(),
                    }],
                }],
                span: SourceSpan::default(),
            }),
            ..Default::default()
        };
        registry.register_macro(macro_def).unwrap();

        let context = CompileContext::new(registry).without_runtime();
        let form = FormMatch {
            macro_name: "test-pattern".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: Some(".socket".to_string()),
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        let result = compile(&[form], &context);
        assert!(result.is_ok(), "compile failed: {:?}", result.err());

        let output = result.unwrap();

        // Should generate typed union CSS
        assert!(
            output.css.contains("data-st-ws-type=\"Connected\""),
            "CSS should contain pattern match attribute, got: {}",
            output.css
        );

        // Should generate watchTypedUnion JS
        assert!(
            output.js.contains("ST.watchTypedUnion"),
            "JS should contain watchTypedUnion, got: {}",
            output.js
        );
    }

    #[test]
    fn test_e0924_cross_namespace_ambiguity() {
        // FEAT-118 FUP-055: two imported modules each export `@camera`. A BARE
        // `@camera` is ambiguous (E0924); a qualified `@scene/camera` is not.
        use crate::metasystem::module::Namespace;
        use crate::parser::meta_ast::MetaDef;
        let mut registry = MetaRegistry::new();
        for ns in ["scene", "ui"] {
            let parsed = crate::parser::parse_for_bootstrap(
                "%macro camera {\n  %form { @camera $x:string }\n}",
            )
            .expect("parse");
            for def in parsed.meta_defs {
                if let MetaDef::Macro(mut m) = def {
                    m.module = Some(Namespace::new(None, vec![ns.to_string()]));
                    registry.register_macro(m).expect("register");
                }
            }
        }

        // Bare call → ambiguous.
        let bare = FormMatch::new("camera");
        let diags = check_namespace_ambiguity(&[bare], &registry);
        assert_eq!(diags.len(), 1, "bare @camera across 2 modules is ambiguous");
        assert_eq!(diags[0].code, DiagnosticCode::E0924);

        // Qualified call → no ambiguity.
        let mut qualified = FormMatch::new("camera");
        qualified.namespace_qualifier = vec!["scene".to_string()];
        let diags = check_namespace_ambiguity(&[qualified], &registry);
        assert!(diags.is_empty(), "a qualified @scene/camera is unambiguous");
    }

    #[test]
    fn canonicalize_expands_alias_qualifier_to_namespace_key() {
        // FEAT-118 FUP-057: `@s/camera` where `@use "std:scene" as s` must rewrite
        // the raw qualifier `[s]` to the namespace's canonical key `[std:scene]`
        // so the FQN-first lookup (`std:scene/camera`) hits.
        use crate::metasystem::module::ImportScope;
        use crate::metasystem::module::Namespace;
        use crate::parser::ast::ImportAst;

        let imports = vec![ImportAst {
            path: "std:scene".to_string(),
            namespace: Some(Namespace::from_module_ref("std:scene")),
            alias: Some("s".to_string()),
            only: vec![],
            hiding: vec![],
            span: SourceSpan::default(),
        }];
        let scope = ImportScope::from_imports(&imports);

        let mut fm = FormMatch::new("camera");
        fm.namespace_qualifier = vec!["s".to_string()];
        let rewritten = canonicalize_qualifiers(&[fm], &scope, &MetaRegistry::new());
        assert_eq!(
            rewritten[0].namespace_qualifier,
            vec!["std:scene".to_string()],
            "alias `s` expands to the namespace key `std:scene`"
        );
    }

    #[test]
    fn alias_qualified_call_resolves_through_pipeline() {
        // FEAT-118 FUP-057 end-to-end: a macro keyed `std:scene/camera`, an
        // `@use "std:scene" as s`, and a `@s/camera` call compile cleanly — the
        // canonical pass bridges alias → key so resolution finds the macro.
        use crate::metasystem::module::{ImportScope, Namespace};
        use crate::parser::ast::ImportAst;
        use crate::parser::meta_ast::MetaDef;

        let mut registry = MetaRegistry::new();
        let parsed = crate::parser::parse_for_bootstrap(
            "%macro camera {\n  %form { @camera $fov:string }\n  %bind { $fov: $fov }\n  %primitive log\n}\n%primitive log {\n  %emit_js { console.log($fov); }\n}",
        )
        .expect("parse");
        for def in parsed.meta_defs {
            match def {
                MetaDef::Macro(mut m) => {
                    m.module = Some(Namespace::from_module_ref("std:scene"));
                    registry.register_macro(m).expect("register scene macro");
                }
                MetaDef::Primitive(p) => {
                    registry.register_primitive(p).expect("register primitive");
                }
                _ => {}
            }
        }
        assert!(registry.get_macro("std:scene/camera").is_some());
        // The macro's `%bind { $fov: $fov }` resolves to a primitive named
        // after the macro itself — which does not exist (only `log` does).
        // The "Primitive not found" stub used to paper over that; under
        // E0956 the bind must name something real.
        let probe = crate::parser::parse_for_bootstrap(
            "%primitive camera {\n  %emit_js { console.log(\"probe: camera\"); }\n}",
        )
        .expect("probe parses");
        for def in probe.meta_defs {
            if let MetaDef::Primitive(p) = def {
                registry.register_primitive(p).expect("register camera probe");
            }
        }

        let imports = vec![ImportAst {
            path: "std:scene".to_string(),
            namespace: Some(Namespace::from_module_ref("std:scene")),
            alias: Some("s".to_string()),
            only: vec![],
            hiding: vec![],
            span: SourceSpan::default(),
        }];
        let scope = ImportScope::from_imports(&imports);
        let context = CompileContext {
            meta_registry: registry,
            include_runtime: false,
            scopes: Vec::new(),
            import_scope: scope,
            body_diagnostics: Vec::new(),
            invocation_node_ids: Vec::new(),
            version_override: None,
        };

        let mut fm = FormMatch::new("camera");
        fm.namespace_qualifier = vec!["s".to_string()];
        fm.captures.insert(
            "fov".to_string(),
            crate::syntax::CapturedValue::String("75".to_string()),
        );

        let result = compile(&[fm], &context);
        assert!(
            result.is_ok(),
            "alias-qualified @s/camera should resolve: {:?}",
            result.err()
        );
    }

    /// Build an ImportScope-bearing context with a single namespaced macro
    /// keyed under `std:scene` for the validation tests below.
    fn scene_registry_with_camera() -> MetaRegistry {
        use crate::metasystem::module::Namespace;
        use crate::parser::meta_ast::MetaDef;
        let mut registry = MetaRegistry::new();
        let parsed = crate::parser::parse_for_bootstrap(
            "%macro camera {\n  %form { @camera $fov:string }\n}",
        )
        .expect("parse");
        for def in parsed.meta_defs {
            if let MetaDef::Macro(mut m) = def {
                m.module = Some(Namespace::from_module_ref("std:scene"));
                registry.register_macro(m).expect("register");
            }
        }
        registry
    }

    #[test]
    fn e0926_unbound_qualifier_is_reported() {
        // `@typo/camera` with no `as typo` binding → E0926.
        use crate::metasystem::module::{ImportScope, Namespace};
        use crate::parser::ast::ImportAst;
        let imports = vec![ImportAst {
            path: "std:scene".to_string(),
            namespace: Some(Namespace::from_module_ref("std:scene")),
            alias: Some("s".to_string()),
            only: vec![],
            hiding: vec![],
            span: SourceSpan::default(),
        }];
        let scope = ImportScope::from_imports(&imports);
        let registry = scene_registry_with_camera();

        let mut fm = FormMatch::new("camera");
        fm.namespace_qualifier = vec!["typo".to_string()];
        let diags = check_import_visibility(&[fm], &scope, &registry);
        assert_eq!(diags.len(), 1, "unbound qualifier reported");
        assert_eq!(diags[0].code, DiagnosticCode::E0926);

        // A BOUND alias `s` does not trip E0926.
        let mut ok = FormMatch::new("camera");
        ok.namespace_qualifier = vec!["s".to_string()];
        let diags = check_import_visibility(&[ok], &scope, &registry);
        assert!(diags.is_empty(), "bound alias is fine");
    }

    #[test]
    fn e0927_only_list_excludes_name() {
        // `@use "std:scene" only (light)` → a bare `@camera` is excluded.
        use crate::metasystem::module::{ImportScope, Namespace};
        use crate::parser::ast::ImportAst;
        let imports = vec![ImportAst {
            path: "std:scene".to_string(),
            namespace: Some(Namespace::from_module_ref("std:scene")),
            alias: None,
            only: vec!["light".to_string()],
            hiding: vec![],
            span: SourceSpan::default(),
        }];
        let scope = ImportScope::from_imports(&imports);
        let registry = scene_registry_with_camera();

        // Bare @camera: lives in the open std:scene, excluded by only(light).
        let bare = FormMatch::new("camera");
        let diags = check_import_visibility(&[bare], &scope, &registry);
        assert_eq!(diags.len(), 1, "camera excluded by only(light)");
        assert_eq!(diags[0].code, DiagnosticCode::E0927);
    }

    #[test]
    fn using_claims_rewrites_bare_call_to_claiming_namespace() {
        // FEAT-118 FUP-056: `@use "scene"` of a module whose `%using` hook
        // `%claims @camera` makes a BARE `@camera` bind to `scene/camera` — the
        // active-extension (Elixir __using__) rewrite, enacted in the canonical
        // pass via the registry's `using_hook` reader.
        use crate::metasystem::module::{ImportScope, Namespace};
        use crate::parser::ast::{ImportAst, ModuleManifest, UsingHook};
        use crate::parser::meta_ast::MetaDef;

        let mut registry = MetaRegistry::new();
        // Macro keyed under the bare `scene` namespace (folder-module form).
        let parsed = crate::parser::parse_for_bootstrap(
            "%macro camera {\n  %form { @camera $fov:string }\n}",
        )
        .expect("parse");
        for def in parsed.meta_defs {
            if let MetaDef::Macro(mut m) = def {
                m.module = Some(Namespace::new(None, vec!["scene".to_string()]));
                registry.register_macro(m).expect("register");
            }
        }
        // A manifest under `scene` whose %using hook claims `camera`.
        registry.register_module_manifest(
            "scene",
            ModuleManifest {
                name: Some("scene".to_string()),
                using: Some(UsingHook {
                    claims: vec!["camera".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let imports = vec![ImportAst {
            path: "scene".to_string(),
            namespace: Some(Namespace::new(None, vec!["scene".to_string()])),
            alias: None,
            only: vec![],
            hiding: vec![],
            span: SourceSpan::default(),
        }];
        let scope = ImportScope::from_imports(&imports);

        // Bare @camera → rewritten to qualifier [scene] by the claim.
        let bare = FormMatch::new("camera");
        let rewritten = canonicalize_qualifiers(&[bare], &scope, &registry);
        assert_eq!(
            rewritten[0].namespace_qualifier,
            vec!["scene".to_string()],
            "a claimed bare @camera binds to the claiming namespace"
        );

        // An UNCLAIMED directive stays bare.
        let other = FormMatch::new("light");
        let rewritten = canonicalize_qualifiers(&[other], &scope, &registry);
        assert!(
            rewritten[0].namespace_qualifier.is_empty(),
            "an unclaimed directive is not rewritten"
        );
    }

    #[test]
    fn e0927_hiding_list_excludes_qualified_name() {
        // `@use "std:scene" as s hiding (camera)` → `@s/camera` is hidden.
        use crate::metasystem::module::{ImportScope, Namespace};
        use crate::parser::ast::ImportAst;
        let imports = vec![ImportAst {
            path: "std:scene".to_string(),
            namespace: Some(Namespace::from_module_ref("std:scene")),
            alias: Some("s".to_string()),
            only: vec![],
            hiding: vec!["camera".to_string()],
            span: SourceSpan::default(),
        }];
        let scope = ImportScope::from_imports(&imports);
        let registry = scene_registry_with_camera();

        let mut fm = FormMatch::new("camera");
        fm.namespace_qualifier = vec!["s".to_string()];
        let diags = check_import_visibility(&[fm], &scope, &registry);
        assert_eq!(diags.len(), 1, "camera hidden by hiding(camera)");
        assert_eq!(diags[0].code, DiagnosticCode::E0927);
    }

    // =========================================================================
    // E0931-E0934: enum/union diagnostics (PLAN-077 W3)
    // =========================================================================

    fn enum_diags(src: &str) -> Vec<Diagnostic> {
        let f = crate::parse(src).expect("parse");
        check_enum_union_diagnostics(&f.matches)
    }

    #[test]
    fn test_e0931_missing_catch_all() {
        // No trailing `_` — the guard chain can match nothing and publish nothing.
        let diags = enum_diags(
            "@type BadgeState { Blue | Green }\n@data derive $x BadgeState : @match { ($a) => Blue; }\n.dm { color: red; }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0931);
        assert!(
            diags[0].message.contains("catch-all"),
            "{}",
            diags[0].message
        );
        assert!(diags[0].hint.is_some(), "actionable hint required");
    }

    #[test]
    fn test_e0931_catch_all_not_last() {
        // `_` mid-chain: later arms are unreachable (first-match-wins).
        let diags = enum_diags(
            "@type BadgeState { Blue | Green }\n@data derive $x BadgeState : @match { _ => Green; ($a) => Blue; }\n.dm { color: red; }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0931);
        assert!(
            diags[0].message.contains("non-last"),
            "{}",
            diags[0].message
        );
    }

    #[test]
    fn test_e0932_non_exhaustive_dispatch() {
        // Sum-typed subject, arms cover Blue only, no wildcard -> E0932 naming
        // the missing variants.
        let diags = enum_diags(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n@template &c() { <div class=\"h\"></div>\n@match $badge { Blue => &t(); } }\n<main></main>",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0932);
        assert!(diags[0].message.contains("Orange"), "{}", diags[0].message);
        assert!(diags[0].message.contains("Green"), "{}", diags[0].message);
    }

    #[test]
    fn test_e0933_unknown_variant_cond_and_inline() {
        // Cond-mode consequence outside the named sum, and outside an inline
        // anonymous union's literal — one E0933 each.
        let diags = enum_diags(
            "@type BadgeState { Blue | Green }\n@data derive $x BadgeState : @match { ($a) => Purple; _ => Green; }\n@data derive $y (Blue | Green) : @match { ($a) => Orange; _ => Green; }\n.dm { color: red; }\n",
        );
        let e33: Vec<_> = diags
            .iter()
            .filter(|d| d.code == DiagnosticCode::E0933)
            .collect();
        assert_eq!(e33.len(), 2, "{:?}", diags);
        assert!(e33[0].message.contains("Purple"), "{}", e33[0].message);
        assert!(e33[1].message.contains("Orange"), "{}", e33[1].message);
        assert!(e33[0].hint.is_some(), "declared-variants hint required");
    }

    #[test]
    fn test_e0934_payload_arity_state_match() {
        // Failed(string, number) has arity 2; the @state destructure binds 1.
        let diags = enum_diags(
            "@type FetchState { Idle | Failed(string, number) }\n@data inline $f FetchState : \"Idle\";\n.x { @state(when: $f is Failed { $e }) { color: red; } }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0934);
        assert!(diags[0].message.contains("Failed"), "{}", diags[0].message);
    }

    #[test]
    fn test_e0934_state_match_correct_arity_no_diag() {
        // GATE 6 P1 regression: `$b:binding` captures materialize as
        // CapturedValue::Binding — an Ident-only extraction made
        // binding_count always 0, false-firing E0934 on the documented
        // correct form. Correct arity (1 binding, payload arity 1) must be
        // diagnostic-free.
        let diags = enum_diags(
            "@type FetchState { Idle | Failed(string) }\n@data inline $f FetchState : \"Idle\";\n.x { @state(when: $f is Failed { $e }) { color: red; } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
    }

    #[test]
    fn test_e0931_clean_cond_derive() {
        // Trailing `_`, declared variants, correct arities -> no diagnostics.
        let diags = enum_diags(
            "@type FetchState { Idle | Failed(string) }\n@data derive $f FetchState : @match { ($e != \"\") => Failed($e); _ => Idle; }\n.dm { color: red; }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
    }

    #[test]
    fn test_e0932_string_condition_and_unknown_subject_never_trip() {
        // Gate-3 criterion a: a bare string-condition @state and a @match over
        // a signal with NO declared sum type must produce ZERO enum diagnostics.
        let diags = enum_diags(
            "@data inline $mode : \"idle\";\n@template &c() { <div class=\"h\"></div>\n@match $mode { \"a\" => &t(); } }\n.x { @state(when: \"loading\") { opacity: 0.7; } }\n<main></main>",
        );
        assert!(diags.is_empty(), "{:?}", diags);
    }

    #[test]
    fn test_e0932_wildcard_and_full_coverage_clean() {
        // Wildcard fallback silences E0932; full bare-tag coverage (lit route,
        // FUP-079) is also exhaustive.
        let with_wild = enum_diags(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n@template &c() { <div class=\"h\"></div>\n@match $badge { Blue => &t(); _ => &u(); } }\n<main></main>",
        );
        assert!(with_wild.is_empty(), "{:?}", with_wild);

        let full = enum_diags(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n@template &c() { <div class=\"h\"></div>\n@match $badge { Blue => &t(); Orange => &t(); Green => &t(); } }\n<main></main>",
        );
        assert!(full.is_empty(), "{:?}", full);
    }

    #[test]
    fn test_e0933_product_type_subject_skipped() {
        // A product-typed (non-sum) subject is uncheckable -> skip, never guess.
        let diags = enum_diags(
            "@type Product { price: number; }\n@data inline $p Product : 1;\n@template &c() { <div class=\"h\"></div>\n@match $p { \"a\" => &t(); } }\n<main></main>",
        );
        assert!(diags.is_empty(), "{:?}", diags);
    }

    // --- SWARM GATE 3 coverage findings --------------------------------------

    #[test]
    fn test_e0934_state_match_simple_no_group_is_clean() {
        // GATE 3 P1 regression: a BARE `@state(when: $f is Failed)` (no
        // destructure group authored) on a payload-carrying variant must NOT
        // trip E0934 — nothing is destructured.
        let diags = enum_diags(
            "@type FetchState { Idle | Failed(string) }\n@data inline $f FetchState : \"Idle\";\n.x { @state(when: $f is Failed) { color: red; } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
    }

    #[test]
    fn test_e0933_dispatch_destructure_unknown_variant() {
        // Dispatch-mode destructure leg: `Connected2 { $s }` names no declared
        // variant of the sum-typed subject.
        let diags = enum_diags(
            "@type ConnState { Disconnected | Connected(string) }\n@data inline $ws ConnState : \"Disconnected\";\n@template &c() { <div class=\"h\"></div>\n@match $ws { Disconnected => &t(); Connected2 { $s } => &u($s); } }\n<main></main>",
        );
        // E0933 for the unknown variant; E0932 ALSO fires (Connected2's
        // coverage doesn't count, so Connected is uncovered) — both correct.
        let e33: Vec<_> = diags
            .iter()
            .filter(|d| d.code == DiagnosticCode::E0933)
            .collect();
        assert_eq!(e33.len(), 1, "{:?}", diags);
        assert!(e33[0].message.contains("Connected2"), "{}", e33[0].message);
    }

    #[test]
    fn test_e0934_dispatch_destructure_arity() {
        // Dispatch-mode destructure leg: `Connected { $a, $b }` binds 2 where
        // Connected(string) has payload arity 1. The `_` arm keeps E0932 quiet
        // so ONLY the arity error fires.
        let diags = enum_diags(
            "@type ConnState { Disconnected | Connected(string) }\n@data inline $ws ConnState : \"Disconnected\";\n@template &c() { <div class=\"h\"></div>\n@match $ws { Connected { $a, $b } => &u($a); _ => &t(); } }\n<main></main>",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0934);
        assert!(
            diags[0].message.contains("Connected"),
            "{}",
            diags[0].message
        );
    }

    #[test]
    fn test_e0934_cond_consequence_arity() {
        // Cond-mode consequence leg: `Failed($e, $e)` passes 2 payload args
        // where Failed(string) declares 1.
        let diags = enum_diags(
            "@type FetchState { Idle | Failed(string) }\n@data derive $f FetchState : @match { ($e != \"\") => Failed($e, $e); _ => Idle; }\n.dm { color: red; }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0934);
        assert!(diags[0].message.contains("Failed"), "{}", diags[0].message);
    }

    #[test]
    fn test_e0932_view_non_exhaustive() {
        // The @view spelling of the dispatch block shares the E0932 check.
        let diags = enum_diags(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n@template &c() { <div class=\"h\"></div>\n@view $badge { Blue => &t(); } }\n<main></main>",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0932);
    }

    #[test]
    fn test_e0933_state_match_unknown_variant() {
        // @state(when:) unknown-variant leg (the E0934 test covers the arity
        // branch of the same block).
        let diags = enum_diags(
            "@type FetchState { Idle | Failed(string) }\n@data inline $f FetchState : \"Idle\";\n.x { @state(when: $f is FailedX) { color: red; } }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0933);
        assert!(diags[0].message.contains("FailedX"), "{}", diags[0].message);
    }

    #[test]
    fn test_e0931_duplicate_catch_all() {
        // GATE 3 P3: two `_` arms — the second is unreachable after the first.
        let diags = enum_diags(
            "@type BadgeState { Blue | Green }\n@data derive $x BadgeState : @match { _ => Blue; _ => Green; }\n.dm { color: red; }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0931);
        assert!(
            diags[0].message.contains("non-last"),
            "{}",
            diags[0].message
        );
    }

    #[test]
    fn test_e0933_inline_union_message_names_no_type() {
        // GATE 3 phrasing: an inline-union E0933 must NOT claim a `@type`
        // declaration exists — it names the inline union itself.
        let diags = enum_diags(
            "@data derive $y (Blue | Green) : @match { ($a) => Orange; _ => Green; }\n.dm { color: red; }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0933);
        assert!(
            !diags[0].message.contains("@type"),
            "inline-union message must not name @type: {}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("inline union"),
            "{}",
            diags[0].message
        );
    }

    // =========================================================================
    // W4: mutation lowering `$x <- Variant(args)` (PLAN-077)
    // =========================================================================

    /// Parse a source with one @on body mutation and return the (possibly
    /// rewritten) js_statements + diagnostics from the W4 transform.
    fn lower_on_body(src: &str) -> (Vec<String>, Vec<Diagnostic>) {
        let f = crate::parse(src).expect("parse");
        let facts = EnumTypeFacts::from_matches(&f.matches);
        let (owned, diags) = lower_variant_mutations(&f.matches, &facts);
        let matches = owned.as_deref().unwrap_or(&f.matches);
        let mut stmts = Vec::new();
        for fm in matches {
            if let Some(CapturedValue::Array(items)) = fm.captures.get("body") {
                // The surviving sigil head `@on &.<driver> { … }` captures the
                // body as an `on_motion_body` ARRAY of statements; a mutation is
                // the `$mut:mutation` capture: a statement item is a Named map
                // under a sub-capture key, `"mut" -> Named{target, expr}`.
                // After lowering the map's `expr` holds the lowered RHS, so the
                // helper's reconstruction is the expected statement.
                for item in items {
                    if let CapturedValue::Named(stmt_item) = item
                        && let Some(CapturedValue::Named(mut_map)) = stmt_item.get("mut")
                        && let Some(stmt) = decomposed_mutation_stmt(mut_map)
                    {
                        stmts.push(stmt);
                    }
                }
            } else if let Some(CapturedValue::Named(body)) = fm.captures.get("body")
                && let Some(CapturedValue::Array(items)) = body.get("js_statements")
            {
                // Retired bare-dispatcher shape (kept for robustness).
                for item in items {
                    if let CapturedValue::String(s) = item {
                        stmts.push(s.clone());
                    }
                }
            }
        }
        (stmts, diags)
    }

    #[test]
    fn test_w4_lowers_typed_nullary_variant() {
        let (stmts, diags) = lower_on_body(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n.btn { @on &.click { $badge <- Orange; } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(stmts, vec!["$badge <- {type: 'Orange'}".to_string()]);
    }

    #[test]
    fn test_w4_lowers_through_non_click_driver_submit() {
        // REGRESSION GUARD: lowering must fire through ANY registered event
        // driver, not just click. The fix keys off the DECOMPOSED mutation
        // structure (a Named map with target + expr), never the literal driver
        // name or a macro allowlist — so this can only pass if a driver other
        // than click reaches the same lowering path.
        let (stmts, diags) = lower_on_body(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n.btn { @on &.submit { $badge <- Orange; } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(stmts, vec!["$badge <- {type: 'Orange'}".to_string()]);
    }

    #[test]
    fn test_w4_lowers_payload_construction_named_fields() {
        // Bare $sig args publish under the signal's own name (W2 convention).
        let (stmts, diags) = lower_on_body(
            "@type ConnState { Disconnected | Connected(string, string) }\n@data inline $conn ConnState : \"Disconnected\";\n.btn { @on &.click { $conn <- Connected($send, $received); } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(
            stmts,
            vec!["$conn <- {type: 'Connected', send: $send, received: $received}".to_string()]
        );
    }

    #[test]
    fn test_w4_single_non_signal_arg_is_value_field() {
        let (stmts, diags) = lower_on_body(
            "@type FetchState { Idle | Failed(string) }\n@data inline $fs FetchState : \"Idle\";\n.btn { @on &.click { $fs <- Failed(\"boom\"); } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(
            stmts,
            vec!["$fs <- {type: 'Failed', value: \"boom\"}".to_string()]
        );
    }

    #[test]
    fn test_w4_untyped_capitalized_bareword_unchanged() {
        // NON-REGRESSION GUARANTEE (plan-mandated): `$x <- Foo` where `$x` has
        // NO declared enum type lowers exactly as before — opaque expr, no
        // diagnostic.
        let (stmts, diags) =
            lower_on_body("@data inline $x : 0;\n.btn { @on &.click { $x <- Foo; } }\n");
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(stmts, vec!["$x <- Foo".to_string()]);
    }

    #[test]
    fn test_w4_capitalized_call_on_typed_target_unchanged() {
        // `String($y)`-shaped RHS on a TYPED target: an `Ident(...)` call that
        // isn't a declared variant could be a legit JS constructor — never
        // lowered, never flagged (E0933's mutation leg is bare-ident only).
        let (stmts, diags) = lower_on_body(
            "@type BadgeState { Blue | Orange }\n@data inline $badge BadgeState : \"Blue\";\n.btn { @on &.click { $badge <- String($y); } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(stmts, vec!["$badge <- String($y)".to_string()]);
    }

    #[test]
    fn test_w4_unknown_variant_on_typed_target_is_e0933() {
        let (stmts, diags) = lower_on_body(
            "@type BadgeState { Blue | Orange }\n@data inline $badge BadgeState : \"Blue\";\n.btn { @on &.click { $badge <- Purple; } }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0933);
        assert!(diags[0].message.contains("Purple"), "{}", diags[0].message);
        assert_eq!(
            stmts,
            vec!["$badge <- Purple".to_string()],
            "unchanged on error"
        );
    }

    #[test]
    fn test_w4_payload_arity_mismatch_is_e0934() {
        // Failed(string) arity 1, construction passes 2 — E0934, and the
        // lowering still happens (the error is authoritative).
        let (stmts, diags) = lower_on_body(
            "@type FetchState { Idle | Failed(string) }\n@data inline $fs FetchState : \"Idle\";\n.btn { @on &.click { $fs <- Failed($a, $b); } }\n",
        );
        assert_eq!(diags.len(), 1, "{:?}", diags);
        assert_eq!(diags[0].code, DiagnosticCode::E0934);
        assert_eq!(
            stmts,
            // bare $sig args keep their names even on an arity error
            vec!["$fs <- {type: 'Failed', a: $a, b: $b}".to_string()]
        );
    }

    #[test]
    fn test_w4_lowercase_and_method_rhs_untouched() {
        // Lowercase bareword (never a variant by the capitalized gate) and a
        // method-call RHS both pass through byte-identically.
        let (stmts, diags) = lower_on_body(
            "@type BadgeState { Blue | orange }\n@data inline $badge BadgeState : \"Blue\";\n@data inline $cart : \"[]\";\n.btn { @on &.click { $badge <- orange; $cart <- $cart.concat([1]); } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(
            stmts,
            vec![
                "$badge <- orange".to_string(),
                "$cart <- $cart.concat([1])".to_string()
            ]
        );
    }

    #[test]
    fn test_w4_string_literal_comma_arg_not_split() {
        // SWARM GATE 4 P1: a comma inside a quoted arg literal is NOT an arg
        // separator — `Failed(\"a,b\")` is ONE arg (arity 1 ✓, no phantom E0934).
        let (stmts, diags) = lower_on_body(
            "@type FetchState { Idle | Failed(string) }\n@data inline $fs FetchState : \"Idle\";\n.btn { @on &.click { $fs <- Failed(\"a,b\"); } }\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(
            stmts,
            vec!["$fs <- {type: 'Failed', value: \"a,b\"}".to_string()]
        );
    }

    /// The @handle surface: receive/optimistic/final clause texts (Expr via
    /// BalancedExtractor) must lower too — and an arm bind colliding with an
    /// emitted key skips the lowering with W0716 (SWARM GATE 4 P1s).
    fn lower_handle(src: &str) -> (Vec<FormMatch>, Vec<Diagnostic>) {
        let f = crate::parse(src).expect("parse");
        let facts = EnumTypeFacts::from_matches(&f.matches);
        let (owned, diags) = lower_variant_mutations(&f.matches, &facts);
        (owned.unwrap_or_else(|| f.matches.clone()), diags)
    }

    #[test]
    fn test_w4_handle_optimistic_expr_text_lowers() {
        // optimistic{} is an Expr capture (BalancedExtractor), not String —
        // the walker must handle both.
        let (matches, diags) = lower_handle(
            "@type BadgeState { Blue | Orange | Green }\n@data inline $badge BadgeState : \"Blue\";\n@handle $save {\n  optimistic { $badge <- Orange; }\n  receive { Ok => { $badge <- Green; } }\n}\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        let text = format!(
            "{:?}",
            matches.iter().map(|m| &m.captures).collect::<Vec<_>>()
        );
        assert!(
            text.contains("{type: 'Orange'}"),
            "optimistic clause must be lowered: {}",
            text
        );
        assert!(
            text.contains("{type: 'Green'}"),
            "receive arm body must be lowered: {}",
            text
        );
    }

    #[test]
    fn test_w4_handle_bind_collision_skips_with_w0716() {
        // `Updated(type) => { $m <- Updated(type); }` — the bind `type` collides
        // with the emitted discriminator key: skip the lowering, warn W0716
        // (runMutations' bare-name locals substitution would corrupt it).
        let (matches, diags) = lower_handle(
            "@type SaveResult { Ok | Updated(string) }\n@data inline $m SaveResult : \"Ok\";\n@handle $save {\n  receive { Updated(type) => { $m <- Updated(type); } }\n}\n",
        );
        let w0716: Vec<_> = diags
            .iter()
            .filter(|d| d.code == DiagnosticCode::W0716)
            .collect();
        assert_eq!(w0716.len(), 1, "{:?}", diags);
        let text = format!(
            "{:?}",
            matches.iter().map(|m| &m.captures).collect::<Vec<_>>()
        );
        assert!(
            !text.contains("{type: 'Updated'"),
            "colliding construction must NOT be lowered: {}",
            text
        );
    }

    #[test]
    fn test_w4_handle_non_colliding_bind_lowers() {
        // Positive control: bind `todo` collides with nothing — lowered.
        let (matches, diags) = lower_handle(
            "@type SaveResult { Ok | Updated(string) }\n@data inline $m SaveResult : \"Ok\";\n@handle $save {\n  receive { Updated(payload) => { $m <- Updated(payload); } }\n}\n",
        );
        assert!(diags.is_empty(), "{:?}", diags);
        let text = format!(
            "{:?}",
            matches.iter().map(|m| &m.captures).collect::<Vec<_>>()
        );
        assert!(
            text.contains("{type: 'Updated', value: payload}"),
            "non-colliding bind must lower: {}",
            text
        );
    }
}

#[cfg(test)]
mod inspector_stamp_addressing {
    use super::*;

    /// Build the span→id map the compiler passes, run the same de-duplication the
    /// pipeline applies, and report which spans survive as stampable.
    fn stampable(
        ids: &[((usize, usize), String)],
    ) -> std::collections::HashMap<(usize, usize), String> {
        let mut ambiguous: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();
        let mut unique: std::collections::HashMap<(usize, usize), String> =
            std::collections::HashMap::new();
        for (span, node_id) in ids {
            if unique.insert(*span, node_id.clone()).is_some() {
                ambiguous.insert(*span);
            }
        }
        unique.retain(|span, _| !ambiguous.contains(span));
        unique
    }

    /// W4R REVIEW FINDING (P1): invocation ids are keyed by BYTE SPAN, but
    /// `resolve_imports` flat-merges files without rebasing spans — two
    /// invocations in different files can therefore claim the SAME span. Taking
    /// the first match would stamp one invocation with the other's id, so a click
    /// would select — and an edit would rewrite — the wrong source invocation.
    ///
    /// A stamp is an address: ambiguity must yield NO stamp, never a guess.
    #[test]
    fn a_span_claimed_by_two_nodes_is_never_stamped() {
        let ids = vec![
            ((10, 20), "0.1".to_string()),
            ((10, 20), "0.2.0".to_string()), // same span, different file
            ((30, 40), "0.3".to_string()),
        ];
        let out = stampable(&ids);
        assert!(
            !out.contains_key(&(10, 20)),
            "an ambiguous span must not be stamped at all, got {out:?}"
        );
        assert_eq!(
            out.get(&(30, 40)).map(String::as_str),
            Some("0.3"),
            "an unambiguous span must still be stamped: {out:?}"
        );
    }

    /// The common case must be unaffected: distinct invocations in one file have
    /// distinct spans and all remain clickable.
    #[test]
    fn distinct_spans_all_remain_stampable() {
        let ids = vec![
            ((10, 20), "0.1".to_string()),
            ((21, 30), "0.2".to_string()),
            ((31, 44), "0.3".to_string()),
        ];
        let out = stampable(&ids);
        assert_eq!(out.len(), 3, "every distinct span stays stampable: {out:?}");
    }
}
