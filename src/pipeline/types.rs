//! Pipeline Types
//!
//! Core types for the 5-layer compilation pipeline.

use std::collections::HashMap;

use crate::ir::{CssExpr, JsStmt};
use crate::parser::SourceSpan;
use crate::syntax::CapturedValue;
use serde::{Deserialize, Serialize};

// =============================================================================
// Phase and Ordering
// =============================================================================

/// Compilation phase - determines primitive execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Global phase - no &element parameter required
    /// These primitives run once per compilation
    Global = 0,

    /// Selector phase - has &element parameter
    /// These primitives run for each matching element
    Selector = 1,
}

impl Phase {
    /// Parse phase from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "global" => Some(Phase::Global),
            "selector" => Some(Phase::Selector),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Global => "global",
            Phase::Selector => "selector",
        }
    }
}

// =============================================================================
// Resolved Primitive
// =============================================================================

/// Bound arguments - parameter name -> captured value
pub type BoundArgs = HashMap<String, CapturedValue>;

/// A resolved primitive call (output of resolve layer)
///
/// This represents a primitive that has been:
/// - Matched via macro expansion
/// - Had its arguments bound via %bind clauses
/// - Been assigned a phase and order for execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedPrimitive {
    /// Name of the primitive to invoke
    pub primitive_name: String,

    /// Bound arguments (parameter name -> value)
    pub args: BoundArgs,

    /// Execution phase
    pub phase: Phase,

    /// Execution order within phase (lower runs first)
    pub order: u32,

    /// CSS selector (for Selector phase primitives)
    pub selector: Option<String>,

    /// CSS styles from macro body capture (for %emit css)
    pub css_styles: Option<String>,

    /// Output signal name remappings from a `%macro`'s `%binds` aliases
    /// (`{ $export as $alias }` -> `export -> alias`). Threaded into
    /// `PrimitiveArgs.outputs` so a `%yield expr -> $export` in the primitive
    /// body writes the ALIAS name (`ST.set(el, "alias", ...)`), which is what the
    /// consuming page reads. Empty for a direct primitive invocation with no
    /// `%binds` aliases (BUG-154).
    pub outputs: HashMap<String, String>,

    /// Source location
    pub span: SourceSpan,
}

impl ResolvedPrimitive {
    /// Create a new resolved primitive
    pub fn new(
        primitive_name: impl Into<String>,
        args: BoundArgs,
        phase: Phase,
        order: u32,
    ) -> Self {
        Self {
            primitive_name: primitive_name.into(),
            args,
            phase,
            order,
            selector: None,
            css_styles: None,
            outputs: HashMap::new(),
            span: SourceSpan::default(),
        }
    }

    /// Set the selector
    pub fn with_selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }

    /// Set CSS styles from macro body capture (for %emit css)
    pub fn with_css_styles(mut self, styles: String) -> Self {
        self.css_styles = Some(styles);
        self
    }

    /// Set the span
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = span;
        self
    }

    /// Set the output signal name remappings (from `%binds` aliases). See the
    /// `outputs` field (BUG-154).
    pub fn with_outputs(mut self, outputs: HashMap<String, String>) -> Self {
        self.outputs = outputs;
        self
    }
}

// =============================================================================
// Typed Fragments (scope-aware, per-language)
// =============================================================================

/// How a JS fragment's code should be scoped during emission.
///
/// Default is `IIFE` — safe by construction. You must explicitly opt into
/// `Inline` to get unsafe (unscoped) behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsScope {
    /// `(function() { el_init; stmts; cleanup; })();`
    /// Variables cannot leak. Cleanup is inlined inside the scope.
    #[default]
    IIFE,
    /// `{ el_init; stmts; cleanup; }`
    /// `const`/`let` safe, `var` leaks. Cleanup is inlined inside the scope.
    Block,
    /// Emitted raw into the script body. Cleanup goes to global handler.
    /// Only for runtime boilerplate.
    Inline,
}

/// How the `el` binding is initialized inside the scope.
#[derive(Debug, Clone, PartialEq)]
pub enum ElInit {
    /// `const el = document.querySelector('selector');`
    Selector(String),
    /// `const el = document.body;`
    Body,
}

/// A JavaScript code fragment — typed, scoped, with lifecycle-aware cleanup.
///
/// The `scope` field determines how the fragment is wrapped during emission.
/// Cleanup statements are always emitted INSIDE the scope (because they
/// reference scoped variables like `mql`, `el`).
#[derive(Debug, Clone)]
pub struct JsFragment {
    /// JS statements (typed IR, not strings)
    pub stmts: Vec<JsStmt>,
    /// Cleanup statements — emitted inside the scope via `ST.onCleanup(el, ...)`
    pub cleanup: Vec<JsStmt>,
    /// Scope wrapping strategy (default: IIFE)
    pub scope: JsScope,
    /// Element binding injected at scope entry
    pub el_init: Option<ElInit>,
    /// Source location
    pub source: SourceSpan,
}

impl JsFragment {
    /// Create a new fragment with statements, defaulting to IIFE scope
    pub fn new(stmts: Vec<JsStmt>) -> Self {
        Self {
            stmts,
            cleanup: Vec::new(),
            scope: JsScope::IIFE,
            el_init: None,
            source: SourceSpan::default(),
        }
    }
}

/// A CSS code fragment — typed, no scoping needed.
///
/// CSS custom properties are global by design, so there's no variable
/// redeclaration concern.
#[derive(Debug, Clone)]
pub struct CssFragment {
    pub exprs: Vec<CssExpr>,
    pub source: SourceSpan,
}

impl CssFragment {
    /// Create a new CSS fragment
    pub fn new(exprs: Vec<CssExpr>) -> Self {
        Self {
            exprs,
            source: SourceSpan::default(),
        }
    }
}

/// Combined output from expanding a single primitive.
#[derive(Debug, Clone)]
pub struct ExpandedPrimitive {
    pub js: Option<JsFragment>,
    pub css: Option<CssFragment>,
    pub exports: Vec<String>,
    /// Build-time scripts from %emit build-js blocks
    pub build_scripts: Vec<String>,
    /// Prelude JS fragment — emitted once per primitive type, before per-element IIFEs
    pub prelude_js: Option<JsFragment>,
    /// Prelude CSS fragment — emitted once per primitive type
    pub prelude_css: Option<CssFragment>,
    /// Primitive name for prelude deduplication
    pub primitive_name: String,
    /// HTML markup from `%emit html` blocks (FEAT-082), captures substituted.
    /// Aggregated into the compiled page body.
    pub html: Vec<String>,
}

// =============================================================================
// Pipeline Output
// =============================================================================

/// Final compilation output (output of emit layer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    /// Compiled JavaScript code
    pub js: String,

    /// Compiled CSS code
    pub css: String,

    /// Build-time scripts from %emit build-js blocks
    pub build_scripts: Vec<String>,

    /// HTML fragments from userland `%emit html` blocks (FEAT-082), in source
    /// order. The compiler splices these into the page body.
    #[serde(default)]
    pub html: Vec<String>,

    /// Diagnostics collected during code generation (warnings, non-fatal errors)
    #[serde(skip)]
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

impl PipelineOutput {
    /// Create a new empty output
    pub fn new() -> Self {
        Self {
            js: String::new(),
            css: String::new(),
            build_scripts: Vec::new(),
            html: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Create output with JS only
    pub fn with_js(js: impl Into<String>) -> Self {
        Self {
            js: js.into(),
            css: String::new(),
            build_scripts: Vec::new(),
            html: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Create output with CSS only
    pub fn with_css(css: impl Into<String>) -> Self {
        Self {
            js: String::new(),
            css: css.into(),
            build_scripts: Vec::new(),
            html: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Create output with both JS and CSS
    pub fn with_code(js: impl Into<String>, css: impl Into<String>) -> Self {
        Self {
            js: js.into(),
            css: css.into(),
            build_scripts: Vec::new(),
            html: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl Default for PipelineOutput {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Unified Compile Error (Phase 1: error type unification)
// =============================================================================

/// Unified error type for all three compilation layers:
/// - Pipeline resolve (was `ResolveError`)
/// - Pipeline expand (was `ExpandError`)
/// - Metasystem macro expansion (was `MacroExpansionError`)
///
/// All callers receive `CompileError`. No `From<MacroExpansionError>` or
/// similar conversion impls — the type IS the error.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub span: crate::parser::SourceSpan,
    // Optional context fields — filled in by the layer that knows them.
    pub macro_name: Option<String>,
    pub selector: Option<String>,
    pub macro_file: Option<String>,
    pub bind_span: Option<crate::parser::SourceSpan>,
    pub bind_primitive: Option<String>,
    pub provided_params: Option<Vec<String>>,
    pub expected_params: Option<Vec<String>>,
    pub primitive_name: Option<String>,
    pub primitive_file: Option<String>,
}

impl CompileError {
    /// Minimal constructor from kind + span.
    pub fn new(kind: CompileErrorKind, span: crate::parser::SourceSpan) -> Self {
        Self {
            kind,
            span,
            macro_name: None,
            selector: None,
            macro_file: None,
            bind_span: None,
            bind_primitive: None,
            provided_params: None,
            expected_params: None,
            primitive_name: None,
            primitive_file: None,
        }
    }

    pub fn with_macro_name(mut self, name: impl Into<String>) -> Self {
        self.macro_name = Some(name.into());
        self
    }
    pub fn with_selector(mut self, sel: impl Into<String>) -> Self {
        self.selector = Some(sel.into());
        self
    }
    pub fn with_macro_file(mut self, file: impl Into<String>) -> Self {
        self.macro_file = Some(file.into());
        self
    }
    pub fn with_bind_span(mut self, span: crate::parser::SourceSpan) -> Self {
        self.bind_span = Some(span);
        self
    }
    pub fn with_bind_primitive(mut self, prim: impl Into<String>) -> Self {
        self.bind_primitive = Some(prim.into());
        self
    }
    pub fn with_provided_params(mut self, params: Vec<String>) -> Self {
        self.provided_params = Some(params);
        self
    }
    pub fn with_expected_params(mut self, params: Vec<String>) -> Self {
        self.expected_params = Some(params);
        self
    }
    pub fn with_primitive_name(mut self, name: impl Into<String>) -> Self {
        self.primitive_name = Some(name.into());
        self
    }
    pub fn with_primitive_file(mut self, file: impl Into<String>) -> Self {
        self.primitive_file = Some(file.into());
        self
    }

    // --- Convenience constructors (mirror MacroExpansionError API) ---

    pub fn unknown_macro(name: String, span: crate::parser::SourceSpan) -> Self {
        Self::new(CompileErrorKind::UnknownMacro(name), span)
    }
    pub fn circular_dependency(cycle: Vec<String>, span: crate::parser::SourceSpan) -> Self {
        Self::new(CompileErrorKind::CircularDependency(cycle), span)
    }
    pub fn unbound_variable(name: String, span: crate::parser::SourceSpan) -> Self {
        Self::new(CompileErrorKind::UnboundVariable(name), span)
    }
    pub fn unsupported_arg_type(
        context: String,
        arg_type: String,
        span: crate::parser::SourceSpan,
    ) -> Self {
        Self::new(
            CompileErrorKind::UnsupportedArgType { context, arg_type },
            span,
        )
    }
    pub fn unresolved_emit_param(
        param_name: String,
        emit_lang: String,
        available_params: Vec<String>,
        span: crate::parser::SourceSpan,
    ) -> Self {
        Self::new(
            CompileErrorKind::UnresolvedEmitParam {
                param_name,
                emit_lang,
                available_params,
            },
            span,
        )
    }
    pub fn max_depth_exceeded(
        max_depth: usize,
        stack: Vec<String>,
        span: crate::parser::SourceSpan,
    ) -> Self {
        Self::new(
            CompileErrorKind::MaxDepthExceeded { max_depth, stack },
            span,
        )
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Clone)]
pub enum CompileErrorKind {
    // ===== Resolve layer (was ResolveErrorKind) =====
    MacroNotFound(String),
    PrimitiveNotFound(String),
    InvalidBind(String),
    /// Missing required parameter (resolve layer — simple string name)
    MissingParameter(String),
    TypeMismatch {
        expected: String,
        got: String,
    },
    MissingRequiredBinding {
        binding: String,
        child_macro: String,
        providers: Vec<String>,
    },

    // ===== Expand layer (was ExpandErrorKind) =====
    TemplateError(String),
    MissingArgument(String),
    TypeError(String),
    UnresolvedParam(String, Vec<String>),
    /// FEAT-119 (W3): a body-bearing construct's factory `body` could not resolve its
    /// World-A `@template:<name>` scope at emit. This is INVARIANT I1 — every
    /// body-bearing construct is named and its scope is built in `cst_to_stfile`, so a
    /// miss is an internal compiler invariant break (never author error). Surfaced as
    /// E0925 by `check` rather than silently emitting an empty body.
    BodyScopeMissing {
        name: String,
        primitive: String,
    },

    // ===== Macro expansion (was MacroExpansionErrorKind) =====
    UnknownMacro(String),
    CircularDependency(Vec<String>),
    /// Missing required parameter (metasystem — carries macro_name + param)
    MacroMissingParameter {
        macro_name: String,
        param: String,
    },
    MacroTypeError {
        param: String,
        expected: String,
        got: String,
    },
    UnboundVariable(String),
    UnknownPrimitive(String),
    UnsupportedArgType {
        context: String,
        arg_type: String,
    },
    UnconsumedPositionalArgs {
        macro_name: String,
        count: usize,
    },
    PatternMismatch {
        macro_name: String,
        param: String,
        expected: String,
        found: String,
    },
    UnresolvedEmitParam {
        param_name: String,
        emit_lang: String,
        available_params: Vec<String>,
    },
    MaxDepthExceeded {
        max_depth: usize,
        stack: Vec<String>,
    },

    // ===== Stdlib loading =====
    StdlibLoad(super::StdlibLoadError),
}

impl std::fmt::Display for CompileErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileErrorKind::MacroNotFound(n) => write!(f, "macro not found: {}", n),
            CompileErrorKind::PrimitiveNotFound(n) => write!(f, "primitive not found: {}", n),
            CompileErrorKind::InvalidBind(m) => write!(f, "invalid bind: {}", m),
            CompileErrorKind::MissingParameter(n) => write!(f, "missing required parameter: {}", n),
            CompileErrorKind::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {}, got {}", expected, got)
            }
            CompileErrorKind::MissingRequiredBinding {
                binding,
                child_macro,
                ..
            } => {
                write!(
                    f,
                    "macro @{} requires binding '{}' from parent scope",
                    child_macro, binding
                )
            }
            CompileErrorKind::TemplateError(m) => write!(f, "template error: {}", m),
            CompileErrorKind::BodyScopeMissing { name, primitive } => write!(
                f,
                "internal: body scope `@template:{}` not found for primitive `{}`",
                name, primitive
            ),
            CompileErrorKind::MissingArgument(a) => write!(f, "missing required argument '{}'", a),
            CompileErrorKind::TypeError(m) => write!(f, "type error: {}", m),
            CompileErrorKind::UnresolvedParam(p, _) => {
                write!(f, "unresolved parameter '%{}' in emit template", p)
            }
            CompileErrorKind::UnknownMacro(n) => write!(f, "unknown macro: %{}", n),
            CompileErrorKind::CircularDependency(cycle) => {
                write!(f, "circular macro dependency: {}", cycle.join(" -> "))
            }
            CompileErrorKind::MacroMissingParameter { macro_name, param } => {
                write!(f, "missing parameter '{}' for macro %{}", param, macro_name)
            }
            CompileErrorKind::MacroTypeError {
                param,
                expected,
                got,
            } => {
                write!(
                    f,
                    "type error for '{}': expected {}, got {}",
                    param, expected, got
                )
            }
            CompileErrorKind::UnboundVariable(n) => write!(f, "unbound variable: ${}", n),
            CompileErrorKind::UnknownPrimitive(n) => write!(f, "unknown primitive: {}", n),
            CompileErrorKind::UnsupportedArgType { context, arg_type } => {
                write!(f, "unsupported argument type '{}' in {}", arg_type, context)
            }
            CompileErrorKind::UnconsumedPositionalArgs { macro_name, count } => {
                write!(
                    f,
                    "macro %{} has {} unconsumed positional argument(s)",
                    macro_name, count
                )
            }
            CompileErrorKind::PatternMismatch {
                macro_name,
                param,
                expected,
                found,
            } => {
                write!(
                    f,
                    "pattern mismatch in %{} for '{}': expected type {}, found {}",
                    macro_name, param, expected, found
                )
            }
            CompileErrorKind::UnresolvedEmitParam {
                param_name,
                emit_lang,
                available_params,
            } => {
                write!(
                    f,
                    "unresolved parameter '%${}' in %emit {} block",
                    param_name, emit_lang
                )?;
                if !available_params.is_empty() {
                    write!(f, " (available: {})", available_params.join(", "))?;
                }
                Ok(())
            }
            CompileErrorKind::MaxDepthExceeded { max_depth, .. } => {
                write!(f, "maximum macro expansion depth ({}) exceeded", max_depth)
            }
            CompileErrorKind::StdlibLoad(e) => write!(f, "stdlib load error: {}", e),
        }
    }
}
