//! IR types for code generation
//!
//! These types represent JavaScript, CSS, GLSL, and HTML code in a structured
//! format that enables deferred stringification and introspection.
//!
//! Source spans can be attached to code fragments for source map generation.

use crate::emit::sourcemap::SourceSpan;

/// JavaScript expression AST (for deferred stringification)
#[derive(Debug, Clone, PartialEq)]
pub enum JsExpr {
    /// String/number/bool literal
    Lit(JsLit),
    /// Variable reference: `foo`
    Var(String),
    /// Property access: `obj.prop`
    Prop { obj: Box<JsExpr>, prop: String },
    /// Index access: `arr[idx]`
    Index { arr: Box<JsExpr>, idx: Box<JsExpr> },
    /// Function call: `fn(args)`
    Call {
        callee: Box<JsExpr>,
        args: Vec<JsExpr>,
    },
    /// Method call: `obj.method(args)`
    Method {
        obj: Box<JsExpr>,
        method: String,
        args: Vec<JsExpr>,
    },
    /// Arrow function: `(params) => body`
    Arrow {
        params: Vec<String>,
        body: Box<JsExpr>,
    },
    /// Block: `{ stmts; expr? }`
    Block {
        stmts: Vec<JsStmt>,
        expr: Option<Box<JsExpr>>,
    },
    /// Template literal: `Hello ${name}`
    Template { parts: Vec<TemplatePart> },
    /// Binary operation: `a + b`
    Binary {
        left: Box<JsExpr>,
        op: BinOp,
        right: Box<JsExpr>,
    },
    /// Unary operation: `!a`, `-b`
    Unary { op: UnaryOp, expr: Box<JsExpr> },
    /// Ternary: `cond ? then : else`
    Ternary {
        cond: Box<JsExpr>,
        then_: Box<JsExpr>,
        else_: Box<JsExpr>,
    },
    /// Object literal: `{ key: value }`
    Object(Vec<(String, JsExpr)>),
    /// Array literal: `[items]`
    Array(Vec<JsExpr>),
    /// Raw JS (escape hatch for complex cases)
    Raw(String),
    /// new Constructor(args)
    New {
        callee: Box<JsExpr>,
        args: Vec<JsExpr>,
    },
    /// Spacetime placeholder - resolved during final code generation
    /// Unified marker — replaces legacy Placeholder
    Marker(Marker<JsExpr>),
    /// Composite expression: interleaved raw code, expressions, and markers
    Composite(Vec<JsPart>),
}

/// JavaScript literal values
#[derive(Debug, Clone, PartialEq)]
pub enum JsLit {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Undefined,
}

// ============================================================================
// Unified Marker System
// ============================================================================

/// Generic marker type for Spacetime placeholders.
///
/// Markers capture the syntax of Spacetime placeholders in a target-agnostic way.
/// The generic parameter `E` allows markers to contain expressions of the
/// appropriate type for each target (JsExpr, CssExpr, etc.).
///
/// Semantics are handled by the emitters - the IR just captures the parsed syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum Marker<E> {
    /// `%name` - parameter reference
    /// `in_string` tracks whether this appeared inside a string literal (Some(quote_char))
    Param {
        name: String,
        field: Option<String>,
        in_string: Option<char>,
    },
    /// `%&name` - element reference
    Element(String),
    /// `$name` - signal read
    Signal(String),
    /// `%$var` or `%$var.field` - loop variable reference
    VarRef {
        var: String,
        field: Option<String>,
        in_string: Option<char>,
    },
    /// `%keyword [expr] [-> $target]` - directives like %yield, %emit, %cleanup, %for
    Directive {
        keyword: String,
        expr: Option<Box<E>>,
        target: Option<String>,
    },
}

/// Part of a composite JS expression - either raw code, a structured expression, or a marker.
#[derive(Debug, Clone, PartialEq)]
pub enum JsPart {
    /// Raw JavaScript code fragment
    Raw(String),
    /// Structured JavaScript expression
    Expr(JsExpr),
    /// Spacetime marker to be resolved during emission
    Marker(Marker<JsExpr>),
}

/// Part of a composite CSS expression - either raw code, a structured expression, or a marker.
#[derive(Debug, Clone, PartialEq)]
pub enum CssPart {
    /// Raw CSS code fragment
    Raw(String),
    /// Structured CSS expression
    Expr(CssExpr),
    /// Spacetime marker to be resolved during emission
    Marker(Marker<CssExpr>),
}

/// JavaScript statement
#[derive(Debug, Clone, PartialEq)]
pub enum JsStmt {
    /// Variable declaration: `const/let name = expr`
    Decl {
        kind: DeclKind,
        name: String,
        init: Option<JsExpr>,
    },
    /// Expression statement
    Expr(JsExpr),
    /// If statement
    If {
        cond: JsExpr,
        then_: Vec<JsStmt>,
        else_: Option<Vec<JsStmt>>,
    },
    /// For-of loop
    ForOf {
        var: String,
        iter: JsExpr,
        body: Vec<JsStmt>,
    },
    /// For loop
    For {
        init: Option<Box<JsStmt>>,
        cond: Option<JsExpr>,
        update: Option<JsExpr>,
        body: Vec<JsStmt>,
    },
    /// Return
    Return(Option<JsExpr>),
    /// Break
    Break,
    /// Continue
    Continue,
    /// Raw JS statement (escape hatch)
    Raw(String),
    /// Meta-level for loop: `%for $var in %array { body }`
    ///
    /// This is a compile-time construct that expands to repeated statements
    /// based on a JSON array parameter. The body contains statements with
    /// `%$var.prop` placeholders that get substituted for each array element.
    ForLoopMeta {
        /// Loop variable name (e.g., "item" from `%for $item in ...`)
        var: String,
        /// Array parameter name (e.g., "items" from `%for ... in %items`)
        array: String,
        /// Body statements to repeat for each array element
        body: Vec<JsStmt>,
    },
}

/// Variable declaration kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Const,
    Let,
    Var,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison (loose)
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Comparison (strict)
    EqStrict,
    NeStrict,
    // Logical
    And,
    Or,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    // Assignment
    Assign,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Typeof,
    Void,
}

/// Part of a template literal
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Text(String),
    Expr(JsExpr),
}

/// CSS expression (for structured CSS generation)
#[derive(Debug, Clone, PartialEq)]
pub enum CssExpr {
    /// CSS rule: selector { declarations }
    Rule {
        selector: String,
        declarations: Vec<CssDecl>,
    },
    /// @keyframes name { frames }
    Keyframes {
        name: String,
        frames: Vec<KeyframeBlock>,
    },
    /// Raw CSS (escape hatch)
    Raw(String),
    /// Composite expression: interleaved raw code, expressions, and markers
    Composite(Vec<CssPart>),
}

/// CSS declaration (property: value)
#[derive(Debug, Clone, PartialEq)]
pub struct CssDecl {
    pub property: String,
    pub value: String,
    pub important: bool,
}

/// Keyframe block within @keyframes
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeBlock {
    /// Selector: "0%", "50%", "from", "to"
    pub selector: String,
    pub declarations: Vec<CssDecl>,
}

/// GLSL expression (for WebGL shader generation)
#[derive(Debug, Clone, PartialEq)]
pub enum GlslExpr {
    /// Shader program with vertex and fragment
    Program { vertex: String, fragment: String },
    /// Raw GLSL (most common - shaders are usually written directly)
    Raw(String),
}

/// HTML expression (first-class markup; PLAN-023).
///
/// Built directly by the html5ever TreeSink from a `.st` HTML region. `Hole` carries a
/// Spacetime expression embedded via a backtick hole. In W1 the hole holds
/// `JsExpr::Raw(source)` (the verbatim hole text); the reactive emit phase (W3/W4), which
/// owns the per-element `EmitContext`, rewrites it into the signal-aware accessor form
/// (e.g. `$x` -> `ST.get(el,'x')`). Keeping the variant typed as `JsExpr` now means no enum
/// rework later.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlExpr {
    /// Element: <tag attrs>children</tag>. Attribute values are part-lists so that
    /// `href="`$u`/x"` (literal + hole + literal) round-trips losslessly.
    Element {
        tag: String,
        attrs: Vec<(String, Vec<AttrPart>)>,
        children: Vec<HtmlExpr>,
        /// 1-based line WITHIN the markup block this element was parsed from,
        /// or `None` when the element was synthesized rather than authored.
        ///
        /// Offset by the enclosing `HtmlBlockAst.span`, this yields the exact
        /// place in the `.st` file the element came from — which is what lets a
        /// tool point at SOURCE instead of guessing from rendered content.
        line: Option<u32>,
    },
    /// Text node
    Text(String),
    /// A Spacetime expression hole embedded in HTML (text position).
    Hole(JsExpr),
    /// Raw HTML: trusted and opaque; no hole scanning or signal binding.
    Raw(String),
    /// Reactive HTML supplied as a string. Parsed through `component_html_to_exprs`
    /// when lowered so backtick holes and nested elements behave like authored markup.
    Html(String),
    /// Reactive MARKDOWN supplied as a signal expression. The dual of `Hole`
    /// (reactive text): where a text hole writes `textContent = String(v)`, a
    /// markdown node writes `innerHTML = snarkdown(v)`, re-rendering whenever a
    /// dependency signal changes. The `String` is the Spacetime expression
    /// source (e.g. `$m.text`), transpiled and dep-collected exactly like a text
    /// hole. Renders through the vendored `snarkdown` global (stdlib/md), which
    /// the demand-driven vendor pass injects because this node emits a
    /// `snarkdown(...)` reference. Trusted-HTML sink by construction: the author
    /// asked for markdown rendering, and snarkdown is the sanitizing engine.
    Markdown(String),
}

/// One part of an HTML attribute value: a literal string or an embedded hole.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrPart {
    /// Literal attribute text
    Lit(String),
    /// Embedded Spacetime expression hole (W1: JsExpr::Raw(source))
    Hole(JsExpr),
}

impl AttrPart {
    /// Convenience: a single literal-valued attribute (the common case).
    pub fn lit(s: impl Into<String>) -> Vec<AttrPart> {
        vec![AttrPart::Lit(s.into())]
    }
}

/// Code fragment with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct CodeFragment {
    pub kind: FragmentKind,
    /// Signal dependencies for this fragment
    pub deps: Vec<String>,
    /// Source location for source map generation
    pub source_span: Option<SourceSpan>,
}

impl CodeFragment {
    /// Create a new code fragment
    pub fn new(kind: FragmentKind) -> Self {
        Self {
            kind,
            deps: Vec::new(),
            source_span: None,
        }
    }

    /// Create a new code fragment with dependencies
    pub fn with_deps(kind: FragmentKind, deps: Vec<String>) -> Self {
        Self {
            kind,
            deps,
            source_span: None,
        }
    }

    /// Add a source span to this fragment
    pub fn with_source_span(mut self, span: SourceSpan) -> Self {
        self.source_span = Some(span);
        self
    }
}

/// Kind of code fragment
#[derive(Debug, Clone, PartialEq)]
pub enum FragmentKind {
    Js(Vec<JsStmt>),
    Css(Vec<CssExpr>),
    Glsl(GlslExpr),
    Html(Vec<HtmlExpr>),
}
