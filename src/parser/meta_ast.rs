//! Metasystem AST Types
//!
//! AST types for the % metasystem (primitives and macros).

use super::SourceSpan;
use serde::{Deserialize, Serialize};

// =============================================================================
// Primitive Definition
// =============================================================================

/// %primitive name(params) { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveDefAst {
    pub name: String,
    pub params: Vec<PrimitiveParam>,
    pub body: PrimitiveBody,
    /// `%uses a, b` — cross-primitive prelude dependencies (PLAN-133): each
    /// named primitive's prelude is pulled onto any page expanding this one,
    /// emitted once, before this primitive's own prelude.
    #[serde(default)]
    pub uses: Vec<String>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Source file where this primitive is defined (for error traces)
    #[serde(default)]
    pub source_file: Option<String>,
    /// Joined text of the contiguous `///` doc-comment block immediately
    /// preceding the definition (markers + one leading space stripped), or
    /// `None` when undocumented. The single structured description shared by LSP
    /// hover, `--coverage` undocumented reports, and the docs generator
    /// (FEAT-083).
    #[serde(default)]
    pub doc: Option<String>,
}

/// Primitive parameter - either element ref or typed param
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveParam {
    /// &el - element reference parameter
    Element(String),
    /// $name - data/context reference (like $gl for WebGL context)
    Data(String),
    /// $name: type - typed data reference (like $t: number)
    TypedData { name: String, ty: String },
    /// name: type = default
    Typed {
        name: String,
        ty: ParamType,
        default: Option<ParamDefault>,
    },
}

/// Parameter type for primitives
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamType {
    /// Simple type: bool, number, string, object
    Simple(String),
    /// Union type: ("x" | "y" | "both")
    Union(Vec<String>),
    /// Array type: string[]
    Array(String),
    /// Optional type: element?, string?
    Optional(String),
    /// Optional array type: string[]?, number[]?
    OptionalArray(String),
}

/// Default value for primitive parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamDefault {
    String(String),
    Number(f64),
    Bool(bool),
    None,
    EmptyArray,
    EmptyObject,
    /// Array literal with values: ["a", "b"] or [1, 2, 3]
    Array(Vec<ParamDefault>),
    /// CSS length value: 20px, 1rem, 100%, etc.
    Length(f64, String),
}

/// Primitive body containing emit blocks, cleanup, exports, and if blocks
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveBody {
    pub emit_blocks: Vec<EmitBlock>,
    pub cleanup: Option<String>,
    pub exports: Vec<ExportDecl>,
    pub if_blocks: Vec<PrimitiveIfBlock>,
}

/// %if condition { then_body } %else { else_body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveIfBlock {
    pub condition: MetaIfCondition,
    pub then_body: PrimitiveBody,
    pub else_body: Option<PrimitiveBody>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %emit js { content } or %emit glsl { content }
/// Cleanup code is extracted from %cleanup { } blocks within the content by the structured JS parser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitBlock {
    pub lang: EmitLang,
    pub content: String,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Emit language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitLang {
    Js,
    Css,
    Glsl,
    BuildJs,
    /// JS code emitted once per primitive type, before any per-element IIFEs
    PreludeJs,
    /// CSS code emitted once per primitive type
    PreludeCss,
    /// HTML markup emitted into the page body at the directive's position
    /// (FEAT-082). Captures (`%$name`, `%$block.html`/`.source`) are substituted;
    /// the result is spliced into `CompiledSpacetime.html`. Enables userland
    /// markup macros like `@example` (dual-render: live demo + escaped source).
    Html,
}

/// Export declaration: $name: type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDecl {
    /// Variable name (without $)
    pub name: String,
    /// Type annotation (structured)
    pub type_expr: ExportTypeExpr,
    /// Whether type is optional (type?)
    pub optional: bool,
}

/// Type expression for exports - supports function types with `~>`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportTypeExpr {
    /// Simple type: number, string, object, any, void
    Simple(String),
    /// Array type: Node[], MutationRecord[]
    Array(String),
    /// Function type: fn(number, name: string?) ~> number
    Function(FunctionTypeExpr),
    /// Legacy fn type (without full signature): fn or fn(type)
    LegacyFn(Option<String>),
    /// Union type: Disconnected | Connecting | Connected { $send, $received } | Error { $error }
    Union(Vec<UnionVariant>),
}

/// Variant in a union type export
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionVariant {
    /// Variant name (e.g., "Connected", "Error")
    pub name: String,
    /// Optional bindings (e.g., ["$send", "$received"])
    pub bindings: Vec<String>,
}

/// Function type expression: fn(params) ~> return
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionTypeExpr {
    /// Function parameters
    pub params: Vec<FunctionTypeParam>,
    /// Return type
    pub return_type: String,
}

/// Parameter in a function type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionTypeParam {
    /// Optional parameter name (for named parameters)
    pub name: Option<String>,
    /// Parameter type
    pub param_type: String,
    /// Whether the parameter is optional (?)
    pub optional: bool,
}

impl ExportTypeExpr {
    /// Format the type expression for display (e.g., in LSP hover)
    pub fn format(&self) -> String {
        match self {
            ExportTypeExpr::Simple(ty) => ty.clone(),
            ExportTypeExpr::Array(ty) => format!("{}[]", ty),
            ExportTypeExpr::Function(func) => func.format(),
            ExportTypeExpr::LegacyFn(param) => {
                if let Some(p) = param {
                    format!("fn({})", p)
                } else {
                    "fn".to_string()
                }
            }
            ExportTypeExpr::Union(variants) => variants
                .iter()
                .map(|v| v.format())
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }

    /// Returns true if this is a function type
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            ExportTypeExpr::Function(_) | ExportTypeExpr::LegacyFn(_)
        )
    }
}

impl FunctionTypeExpr {
    /// Format the function type for display
    pub fn format(&self) -> String {
        let params: Vec<String> = self.params.iter().map(|p| p.format()).collect();
        format!("fn({}) ~> {}", params.join(", "), self.return_type)
    }
}

impl FunctionTypeParam {
    /// Format the parameter for display
    pub fn format(&self) -> String {
        let mut s = String::new();
        if let Some(ref name) = self.name {
            s.push_str(name);
            s.push_str(": ");
        }
        s.push_str(&self.param_type);
        if self.optional {
            s.push('?');
        }
        s
    }
}

impl UnionVariant {
    /// Format the variant for display
    pub fn format(&self) -> String {
        if self.bindings.is_empty() {
            self.name.clone()
        } else {
            format!("{} {{ {} }}", self.name, self.bindings.join(", "))
        }
    }
}

// =============================================================================
// Macro Definition
// =============================================================================

/// Where a macro is allowed to expand. Only `File` (top-level) and `Selector`
/// (inside a `.sel { }` scope) are enforced — by `macro_scope_matches`. Richer
/// construct-containment ("this macro may only appear inside a `@template`/`@each`
/// body") is expressed as DATA via `%scope within(<construct>)` and checked by
/// `macro_within_matches` (registry.rs), NOT by adding closed enum arms here — a
/// Rust arm the runtime doesn't enforce is rot (PLAN-039 elegance bar). The prior
/// `PropertyValue`/`Operator`/`Expr`/`TimelineBody`/`Template` arms were never
/// enforced (and 4 of 5 never even constructed); removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroScope {
    File,
    Selector,
}

impl MacroScope {
    /// Canonical kebab-case name (`file`, `selector`). Shared by inspect/dispatch
    /// projections so scope strings never drift.
    pub fn as_str(&self) -> &'static str {
        match self {
            MacroScope::File => "file",
            MacroScope::Selector => "selector",
        }
    }
}

/// %macro name { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MacroDefAst {
    pub name: String,
    /// %form { pattern }
    pub form: Option<FormClause>,
    /// %binds { ... }
    pub binds: Vec<BindDecl>,
    /// %derives { ... }
    pub derives: Vec<DeriveDecl>,
    /// %states { ... }
    pub states: Option<MetaStatesClause>,
    /// %registers name { ... }
    pub registers: Option<RegistersClause>,
    /// %imports { module: $path, ... } — a declared compile-time module-load
    /// effect (FEAT-118). `None` for ordinary macros. Enacted by the import
    /// phase; see `docs/specs/declared-effects.md`.
    #[serde(default)]
    pub imports: Option<ImportsClause>,
    /// %resolves { $symbol -> registry }
    pub resolves: Option<ResolvesClause>,
    /// %scope file | selector | property-value | ...
    #[serde(default)]
    pub scopes: Vec<MacroScope>,
    /// %scope within(<construct>) — the macro matches ONLY inside a region whose
    /// owning construct (ScopeKind::Construct(name) / the built-in "selector"/"file")
    /// is one of these names (PLAN-039). Construct names are registry keys (a macro /
    /// capture-type), so this is kinds-as-DATA — NOT a Rust match-arm per kind, and the
    /// successor to the closed `MacroScope` enum. Empty = no within-restriction.
    #[serde(default)]
    pub scope_within: Vec<String>,
    /// %scope element(<tag>) — the selector scope this macro binds to must resolve
    /// to an element whose tag is one of these names (GH-12). Kinds-as-DATA like
    /// `scope_within`: a macro declares the required element, and the compile
    /// pipeline (`validate_scope_element_constraints`) refuses a bound selector
    /// whose implied element cannot be the required tag, and warns when the tag
    /// is unknowable. Empty = no element restriction.
    #[serde(default)]
    pub scope_element: Vec<String>,
    /// %order N — execution priority (lower = earlier)
    #[serde(default)]
    pub order: Option<u32>,
    /// %requires { $gl, $width, ... }
    #[serde(default)]
    pub requires: Vec<String>,
    /// Other body items (when, on, for, if, etc.)
    pub body: Vec<MacroBodyItem>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Source file where this macro is defined (for error traces)
    #[serde(default)]
    pub source_file: Option<String>,
    /// Module namespace this macro belongs to (FEAT-118). `None` = GLOBAL — the
    /// degenerate, ceremony-free default (principle P4). A folder becomes a
    /// namespace only when a `MODULE.st %module` clause or `@use` assigns it
    /// (later milestones); until then every def is global and its registry key
    /// is its bare name.
    #[serde(default)]
    pub module: Option<crate::metasystem::module::Namespace>,
    /// Joined text of the contiguous `///` doc-comment block immediately
    /// preceding the definition (markers + one leading space stripped), or
    /// `None` when undocumented. Shared description surface (FEAT-083).
    #[serde(default)]
    pub doc: Option<String>,
    /// Set when this macro is registered BY a `%migration` (PLAN-079): the
    /// macro is retired syntax owned by that migration's wave. `None` = a
    /// live macro.
    #[serde(default)]
    pub retired: Option<RetiredMacroTag>,
    /// For a `%migration` REWRITE-RULE macro (`<migration-id>#<rule-id>`): the
    /// rule's `%into` template text, whose `$capture` interpolations are the
    /// match captures' CONSUMERS (the rewrite forwards them). `None` for every
    /// live macro and for embedded (non-rule) retired macros. The
    /// capture-consumption lint (E0964) includes this text in a rule macro's
    /// consumption surface so a capture bound in `%match` and re-emitted in
    /// `%into` is not misreported as dropped (I4 / gh-18).
    #[serde(default)]
    pub rewrite_template: Option<String>,
    /// Declared marker `%diagnostic`: this macro's SOLE job is to DETECT an
    /// invalid call shape and emit a diagnostic (`optional-clause-error` /
    /// `drive-as-error`). Its `%form` captures exist to make the invalid shape
    /// match — they are CONSUMED by the diagnostic, never forwarded to a
    /// primitive. The capture-consumption lint (E0964) exempts a
    /// `%diagnostic`-marked macro's captures (I4 / gh-18).
    #[serde(default)]
    pub diagnostic_only: bool,
    /// Declared marker `%pipeline-consumed`: this macro's `%form` captures are
    /// consumed by machinery OUTSIDE the clause text the lint scans — the Rust
    /// compile pipeline (`inject_*_data` rewrites a recognition-only macro) or
    /// a custom capture type whose structured record is decomposed and its
    /// sub-keys forwarded in `%binds` (e.g. `@data signal`'s `receive_block`
    /// -> `arms`/`sum`, `@handle`'s `handle_receive`). The lint cannot see the
    /// consumption by name-reference, so the author declares it. NOT for
    /// genuinely half-implemented macros — those must be `_`-prefixed and
    /// tracked, never hidden behind this marker.
    #[serde(default)]
    pub pipeline_consumed: bool,
    /// Declared `%drops ( a b _ )`: capture names this macro deliberately does
    /// NOT forward — a documented drop (a retired macro whose old `%binds`
    /// already ignored the capture, a rewrite rule whose `%match` accepted
    /// extras that never reached `%into`). The capture-consumption lint
    /// (E0964) treats each name here as CONSUMED-by-declaration, exactly like
    /// a `%diagnostic`/`%pipeline-consumed` marker. This is the author's
    /// ASSERTED drop, distinct from `_`-prefixing the capture — it documents
    /// WHY the capture is dropped. NOT for genuinely half-implemented macros:
    /// a real drop is a deliberate legacy compat tracked in the migration
    /// docs; a half-implemented macro must be `_`-prefixed and filed
    /// (I4 / gh-18).
    #[serde(default)]
    pub drops: Vec<String>,
}

/// %form { @directive(params) { body } }
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FormClause {
    /// The directive name (e.g., "toggle" for @toggle, "on hover" for @on hover)
    pub directive_name: String,
    /// Inline elements before parentheses (captures and literals)
    /// e.g., @type $name:ident { ... } has inline capture $name:ident
    /// e.g., @data $name:ident : $type:typeref { ... } has inline capture and ":" literal
    pub inline_elements: Vec<FormInlineElement>,
    /// Parameters in parentheses
    pub params: Vec<FormParam>,
    /// Inline elements AFTER the parenthesized arg-list and BEFORE the body, e.g.
    /// the `: $returnType:typeref` run in `@fn $n($params): $returnType { $body }`.
    /// Same element vocabulary as `inline_elements`; matched against the main inline
    /// cursor between the ARG_LIST and BODY steps (PLAN-025 / BUG-045).
    #[serde(default)]
    pub post_arg_inline: Vec<FormInlineElement>,
    /// Body capture pattern (if any)
    pub body_capture: Option<String>,
    /// Body params extracted from { prop: $var:type; ... } syntax
    #[serde(default)]
    pub body_params: Vec<FormParam>,
    /// Leading parenthesized GROUPS in the body region, before a greedy body
    /// capture (FEAT-103). Each is a `CapturePatternAst` (the same PEG that powers
    /// `%capture_type`), letting a `%form` body express an optional/required
    /// literal-led head-line — e.g. `( shortcut: $s:string ; )?` — ahead of a
    /// greedy `$body:component_body`. The matcher runs these over the BODY token
    /// prefix, then hands the remaining byte-run to the body capture. Empty for
    /// every form that does not use a body group (the legacy path is untouched).
    #[serde(default)]
    pub body_groups: Vec<CapturePatternAst>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Inline element in a form pattern (before parentheses)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormInlineElement {
    /// An inline capture: $name:type
    Capture(FormCapture, Option<ParamDefault>),
    /// A literal token: ":", "<-", or an identifier
    Literal(String),
    /// Comparison operator followed by a capture: >= $count:number
    Comparison {
        operator: String,
        capture: FormCapture,
    },
    /// An INLINE GROUP of elements with a repetition modifier, e.g.
    /// `( timeout : $timeout:time = 5000 )?`. A genuinely-optional trailing
    /// clause is expressed in `%form` as DATA — an optional inline group the
    /// matcher tries as a unit and skips (applying in-group defaults) when the
    /// whole clause is absent (W3 / PLAN-137, `@wait_until`).
    Group {
        elements: Vec<FormInlineElement>,
        modifier: CaptureModifier,
    },
    /// Keyword block in form body: keyword { captures }modifier
    /// e.g., `uniforms { $uniformBindings:uniform_bindings* }?`
    KeywordBlock {
        keyword: String,
        body_params: Vec<FormParam>,
        modifier: CaptureModifier,
    },
    /// Pseudo-selector in form body: (:name { captures })modifier
    /// e.g., `(:entering { $enterAnim:keyframes })?`
    PseudoSelector {
        name: String,
        body_params: Vec<FormParam>,
        modifier: CaptureModifier,
    },
    /// Pseudo-class in form body: :name { captures }
    /// e.g., `:entering { $enterAnim:keyframes? }`
    PseudoClass {
        name: String,
        body_params: Vec<FormParam>,
    },
}

/// Form parameter: name: $capture:type or name: $a:ident is $b:ident { $c:ident* }
///
/// Supports both simple parameters and complex patterns with multiple elements:
/// - Simple: `when: $condition:string`
/// - Pattern: `when: $signal:ident is $variant:ident { $bindings:ident* }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormParam {
    /// Parameter name as it appears in syntax
    pub name: String,
    /// The inline elements that make up this parameter's value
    /// For simple params, this is a single Capture element
    /// For patterns, this can include Captures and Literals (like "is")
    pub elements: Vec<FormInlineElement>,
    /// Default value (if any) - only valid for simple single-element params
    pub default: Option<ParamDefault>,
}

impl FormParam {
    /// Get the first capture if this is a simple single-capture param
    pub fn capture(&self) -> Option<&FormCapture> {
        if self.elements.len() == 1
            && let FormInlineElement::Capture(ref cap, _) = self.elements[0]
        {
            return Some(cap);
        }
        None
    }

    /// Check if this param is a simple single-capture param
    pub fn is_simple(&self) -> bool {
        self.elements.len() == 1
            && matches!(
                self.elements.first(),
                Some(FormInlineElement::Capture(_, _))
            )
    }
}

/// Form capture: $varname:type? or $source:type as $alias:type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormCapture {
    /// Variable name to bind to
    pub var_name: String,
    /// Capture type
    pub capture_type: CaptureType,
    /// Modifier (optional, zero-or-more)
    pub modifier: CaptureModifier,
    /// Optional alias capture for "as $alias:type" patterns
    /// When present, binds both source and alias in "as $alias:type" patterns
    #[serde(default)]
    pub alias_capture: Option<Box<FormCapture>>,
}

/// Capture types for form patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureType {
    Ident,
    /// An identifier that begins with `--` (a form reference, per SIP-001c).
    /// Distinct from `Ident` because the lexer merges `--rise` into ONE IDENT
    /// token, so only a capture type can demand the sigil (BUG-242).
    DashedIdent,
    /// A bare event name for the legacy `@on` dispatchers — never a
    /// `$`-sigiled binding (the signal-arms surface). Unlike `Ident`, no
    /// sigil-stripping (W3 review P2).
    EventName,
    String,
    Number,
    Bool,
    Time,
    Length,
    Duration,
    Easing,
    Typeref,
    Binding,
    Event,
    Expr,
    Properties,
    Fields,
    Params,
    States,
    Transitions,
    Keyframes,
    Selector,
    Element,
    /// CSS color value: #hex, rgb(), hsl(), oklch(), named colors, etc.
    Color,
    /// Preset capture type (RETIRED — SIP-001c / BUG-263). The `~` preset
    /// prefix is gone: a `~name` in user code errors at parse (the retirement
    /// diagnostic), so this capture type and its `PresetExtractor` cannot match
    /// any parsed input. No stdlib macro declares a `:preset` capture. Kept as
    /// a dormant enum variant so a user-authored `$x:preset` fails as an
    /// unknown/generic capture rather than silently extracting the CSS
    /// sibling-combinator `~`.
    Preset,
    MutationActions,
    /// Template - child element template
    Template,
    /// Parameter list: ($value, &element, $optional?)
    ParamList,
    /// HTML block with interpolation: <div>$value.prop</div>
    HtmlBlock,
    /// Component body: structured parse of HTML + CSS rules + state declarations +
    /// behavioral directives + content injections. Returns Named map with fields:
    /// `html`, `css_rules`, `states`, `exports`, `directives`, `injections`.
    ComponentBody,
    /// JavaScript block: { multi-statement JS code }
    JsBlock,
    /// Template invocation: &template-name($arg1, key: $val)
    TemplateInvocation,
    /// Union type: ("x" | "y" | "both")
    Union(Vec<std::string::String>),
    /// Balanced run: collect tokens until the given delimiter char appears at nesting
    /// depth 0 (respecting () [] {}). The one depth-aware PEG terminal (PLAN-023 W2).
    /// Spelled `$body:balanced(';')` in stdlib %capture_type / %form bodies.
    Balanced(char),
    /// Skip a balanced `{ ... }` block (depth-aware), consuming it and producing an empty
    /// marker. Sibling to Balanced; lets a repeated production step over nested directive
    /// blocks (e.g. `@dark { ... }` inside a properties body) and continue (PLAN-023 W2).
    /// Spelled `$_:skip_block` in stdlib grammar (the capture name is conventionally `_`).
    SkipBlock,
    /// Pattern match: $signal is Variant { $bindings }
    PatternMatch {
        variant: std::string::String,
        bindings: Vec<std::string::String>,
    },
    /// Custom capture type defined via %capture_type
    Custom(std::string::String),
}

/// Capture modifier
///
/// `Counted` carries its permitted lengths as DATA rather than adding one arm per
/// shape: a hex colour is `xdigit{3|4|6|8}`, an ISO date is `digit{4}`, and both
/// are the same mechanism with a different set. The set is a fixed-size inline
/// array so the enum stays `Copy` (it is threaded through the pattern compiler by
/// value); `len` counts the live entries. Eight alternatives is far past anything
/// a real token grammar needs — CSS's longest is four.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureModifier {
    #[default]
    Required,
    Optional,
    ZeroOrMore,
    OneOrMore,
    /// `{n}` or `{a|b|c}` — match EXACTLY one of the listed repetition counts.
    /// Ordered longest-first at match time so `{3|6}` prefers the 6-digit form
    /// (a PEG's ordered choice would otherwise stop at the shorter prefix).
    Counted(CountSet),
}

/// The permitted repetition counts of a `Counted` modifier.
///
/// Fixed-capacity so `CaptureModifier` remains `Copy`. Construct via `new`, read
/// via `counts()`; the invariant is that entries `0..len` are meaningful and
/// sorted DESCENDING, which is what makes longest-match-first free at scan time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountSet {
    counts: [u8; 8],
    len: u8,
}

impl CountSet {
    /// Build from parsed counts. Values are deduplicated and sorted descending;
    /// anything past the 8-entry capacity, and any zero count, is dropped (a
    /// zero-length repetition is not a token shape).
    pub fn new(mut values: Vec<u8>) -> Self {
        values.retain(|n| *n > 0);
        values.sort_unstable_by(|a, b| b.cmp(a));
        values.dedup();
        let mut counts = [0u8; 8];
        let len = values.len().min(8);
        counts[..len].copy_from_slice(&values[..len]);
        Self {
            counts,
            len: len as u8,
        }
    }

    /// The permitted counts, longest first.
    pub fn counts(&self) -> &[u8] {
        &self.counts[..self.len as usize]
    }

    /// True when no count was parsed — the caller should treat the modifier as
    /// malformed rather than as "matches nothing".
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// %binds { primitive(args) -> { $outputs } }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindDecl {
    /// Primitive name being bound
    pub primitive: String,
    /// Arguments to the primitive
    pub args: Vec<BindArg>,
    /// Output variables with optional aliases
    pub outputs: Vec<BindOutput>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Output binding with optional alias
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindOutput {
    /// Original variable name from primitive (e.g., "data")
    pub name: String,
    /// Alias pattern if using "as" (e.g., "${$name}-loading")
    pub alias: Option<String>,
}

/// Bind argument
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindArg {
    /// &element or &self >> selector
    Element {
        name: String,
        child_selector: Option<String>,
    },
    /// name: value
    Named { name: String, value: BindValue },
    /// Positional value
    Positional(BindValue),
}

/// Value in a bind argument
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindValue {
    Variable(String),
    String(String),
    Number(f64),
    Ident(String),
    Array(Vec<String>),
    /// Function call: colorToArray($clearColor)
    FunctionCall {
        name: String,
        args: Vec<BindValue>,
    },
    /// Element reference: &self.parentElement
    ElementRef(String),
}

/// %derives { $name: expression }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeriveDecl {
    /// Variable name (without $)
    pub name: String,
    /// Expression string
    pub expr: String,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %states { state_defs }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaStatesClause {
    pub states: Vec<MetaStateDef>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// State definition in %states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetaStateDef {
    /// Named state with optional condition: name when $condition { properties }
    Named {
        name: String,
        condition: Option<String>,
        properties: Vec<MetaStateProperty>,
    },
    /// Variable reference: $customStates or $customStates? or $condition { properties }
    Variable {
        name: String,
        optional: bool,
        properties: Vec<MetaStateProperty>,
    },
}

/// Property in a meta state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaStateProperty {
    pub name: String,
    pub value: String,
}

/// `%imports { module: $path, as: $alias, only: (...), hiding: (...), global: bool }`
/// — a DECLARED compile-time effect (FEAT-118 / `docs/specs/declared-effects.md`).
///
/// A form carrying this clause declares that matching it LOADS a module and
/// merges/namespaces its definitions. The effect is DATA (this struct), not
/// executable code: the enactor phase reads it and performs a closed set of
/// primitives (resolve → parse → merge → namespace). `@import` is the
/// `global: true` degenerate case (flat-global merge); `@use` is `global:
/// false` (namespaced). Sibling to [`RegistersClause`]/`ResolvesClause`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportsClause {
    /// Capture name holding the module reference (e.g. `path` for `$path`).
    pub module: String,
    /// Capture name for the qualified alias (`as $alias`), if any.
    #[serde(default)]
    pub alias: Option<String>,
    /// Capture names for an explicit import list (`only (a, b)`).
    #[serde(default)]
    pub only: Vec<String>,
    /// Capture names for an open-minus list (`hiding (a, b)`).
    #[serde(default)]
    pub hiding: Vec<String>,
    /// `true` for `@import` (flat-global merge, back-compat); `false` for `@use`
    /// (namespaced). Global is the degenerate point of the import spectrum.
    #[serde(default)]
    pub global: bool,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %registers name { field: $var }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistersClause {
    /// The category name (e.g., "type", "binding")
    pub name: String,
    /// Arguments to the category: e.g., ($name) or ($name, type: $type)
    pub args: Vec<RegistersArg>,
    /// Items in the body
    pub items: Vec<RegisterItem>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Argument in a registers clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegistersArg {
    /// Positional: $name
    Positional(String),
    /// Named: type: $type
    Named { name: String, var: String },
}

/// Item in a registers clause body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegisterItem {
    /// Field: name: $var or name: value
    Field(RegisterField),
    /// Variable reference: $fields
    VarRef(String),
}

/// Field in a registers clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisterField {
    pub name: String,
    pub value: RegisterValue,
}

/// Value in a register field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegisterValue {
    Var(String),
    Array(Vec<RegisterArrayItem>),
    Bool(bool),
    String(String),
    Ident(String),
    /// CSS properties: Vec<(property_name, property_value)>
    Properties(Vec<(std::string::String, std::string::String)>),
}

/// Item in a register array value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegisterArrayItem {
    Var(String),
    String(String),
    Ident(String),
}

/// Items in macro body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MacroBodyItem {
    /// %when $condition { body }
    When(WhenClause),
    /// %on trigger { body }
    On(MetaOnClause),
    /// %for $item in $list { body }
    For(MetaForClause),
    /// %if condition { body } %else { body }
    If(MetaIfClause),
    /// %animates { property: $var }
    Animates(AnimatesClause),
    /// %applies { styles }
    Applies(AppliesClause),
    /// %includes { @patterns }
    Includes(IncludesClause),
    /// %mutate timing { operations }
    Mutate(MetaMutateClause),
    /// %trigger event
    Trigger(String),
    /// %emit js/css { code } - direct code emission in macros
    Emit(EmitBlock),
    /// %binds { primitive calls } - inline binds in conditional bodies
    Binds(Vec<BindDecl>),
}

/// %when $condition { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhenClause {
    pub condition: String,
    pub body: Vec<MacroBodyItem>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %on trigger { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaOnClause {
    pub trigger: MetaOnTrigger,
    pub body: Vec<MetaOnBodyItem>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Trigger for %on
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetaOnTrigger {
    /// $var -> value (value transition)
    VarTransition { var: String, value: String },
    /// $var.event (variable event)
    VarEvent { var: String, event: String },
    /// Plain identifier (event name)
    Event(String),
}

/// Body item in %on
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetaOnBodyItem {
    /// $var <- expr
    Assignment { var: String, expr: String },
    /// A bare action STATEMENT to run when the trigger fires — a signal call
    /// (`$move({...})`), an effect, or a macro-param reference (`$onDrop`) that
    /// expands to the caller's action. Runs through the shared `ST.runMutations`
    /// rail, the same surface `@on { action }` uses (PLAN-053).
    Action(String),
    /// Nested macro body item
    MacroItem(Box<MacroBodyItem>),
    /// Inline emit block
    Emit(EmitBlock),
}

/// %for $item in $list { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaForClause {
    pub variable: String,
    pub source: String,
    pub body: Vec<MacroBodyItem>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %if condition { body } %elif condition { body } %else { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaIfClause {
    pub condition: MetaIfCondition,
    pub then_body: Vec<MacroBodyItem>,
    pub elif_clauses: Vec<MetaElifClause>,
    pub else_body: Option<Vec<MacroBodyItem>>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %elif condition { body }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaElifClause {
    pub condition: MetaIfCondition,
    pub body: Vec<MacroBodyItem>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Condition for %if
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetaIfCondition {
    /// $var (truthy check)
    Truthy(String),
    /// !$var (falsy check)
    Falsy(String),
    /// $var == value
    Equals(String, String),
    /// $var != value
    NotEquals(String, String),
    /// $var < value
    LessThan(String, String),
    /// $var > value
    GreaterThan(String, String),
    /// $var <= value
    LessThanOrEqual(String, String),
    /// $var >= value
    GreaterThanOrEqual(String, String),
    /// $a || $b (logical OR)
    Or(Box<MetaIfCondition>, Box<MetaIfCondition>),
    /// $a && $b (logical AND)
    And(Box<MetaIfCondition>, Box<MetaIfCondition>),
}

/// %animates { property: $var }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimatesClause {
    pub properties: Vec<AnimateProp>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Property in %animates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimateProp {
    pub property: String,
    pub variable: String,
}

/// %applies { styles }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliesClause {
    /// Variable references like $styles
    pub variables: Vec<String>,
    /// Inline properties
    pub properties: Vec<MetaStateProperty>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// %includes { @patterns }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncludesClause {
    pub patterns: Vec<IncludedPattern>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Pattern in %includes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncludedPattern {
    pub name: String,
    pub args: Vec<IncludedPatternArg>,
}

/// Argument in included pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncludedPatternArg {
    pub name: String,
    pub value: String,
}

/// %mutate timing { operations }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaMutateClause {
    pub timing: MutateTiming,
    pub operations: Vec<MutateOperation>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Mutate timing specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateTiming {
    pub kind: String,
    pub state: String,
}

/// Mutate operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateOperation {
    pub name: String,
    pub args: Vec<MutateOpArg>,
}

/// Mutate operation argument
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateOpArg {
    pub name: String,
    pub value: String,
}

// =============================================================================
// Runtime Registry Definition
// =============================================================================

/// Runtime registry definition: %runtime-registry name { targets }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRegistryDef {
    pub name: String,
    pub targets: Vec<RegistryTarget>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Which stdlib file this was loaded from (for incremental cache invalidation)
    #[serde(default)]
    pub source_file: Option<String>,
}

/// Per-target registry definition: %target js { ... }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryTarget {
    pub target_name: String, // "js", "wasm", etc.
    pub namespace: String,   // "ST.functions"
    pub init: String,        // Initialization code
    pub operations: Vec<RegistryOperation>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Registry operation type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegistryOperation {
    Call {
        params: Vec<String>,
        template: String,
    },
    Register {
        params: Vec<String>,
        template: String,
    },
    Access {
        params: Vec<String>,
        template: String,
    },
}

/// Resolves clause for macros: %resolves { $name -> functions }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvesClause {
    pub mappings: Vec<ResolveMapping>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Single resolve mapping: $symbol -> registry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveMapping {
    pub symbol: String,   // "$name"
    pub registry: String, // "functions"
}

// =============================================================================
// Meta Definition Enum
// =============================================================================

/// Preset definition: %preset category &name: value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaPresetDefAst {
    /// Category (e.g., "easing", "duration", "animation")
    pub category: String,
    /// Name with & prefix stripped (e.g., "linear", "fast")
    pub name: String,
    /// Value - either inline or block
    pub value: MetaPresetValue,
    pub span: SourceSpan,
    /// Which stdlib file this was loaded from (for incremental cache invalidation)
    #[serde(default)]
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetaPresetValue {
    /// Inline value like `linear` or `300ms`
    Inline(String),
    /// Block value with property transitions like `{ opacity: 0 -> 1 }`
    Block(String),
}

// =============================================================================
// Capture Type Definition (Meta-Circular Pattern DSL)
// =============================================================================

/// Pattern element in a capture type definition.
///
/// This represents the pattern DSL for defining custom capture types in `.st` files:
/// ```st
/// %capture_type param_list {
///   ( ( $name:binding | &name:element ) "?"? ","? )*
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapturePatternAst {
    /// Capture: $name:type or &name:type
    Capture {
        var_name: String,
        capture_type: CaptureType,
        modifier: CaptureModifier,
    },
    /// Literal: "," or "?"
    Literal(String),
    /// Character class: [abc] or [^abc] (negated)
    CharClass {
        chars: String,
        negated: bool,
        /// Counted repetition (`[0-9a-f]{3|4|6|8}`). `None` = match one character.
        #[serde(default)]
        counts: Option<CountSet>,
    },
    /// Group with modifier: ( pattern )* or ( pattern )+ or ( pattern )?
    Group {
        pattern: Box<CapturePatternAst>,
        modifier: Option<CaptureModifier>,
    },
    /// Sequence: elem1 elem2 elem3
    Sequence(Vec<CapturePatternAst>),
    /// Choice: pattern1 | pattern2
    Choice(Vec<CapturePatternAst>),
}

/// Capture type definition: %capture_type name { pattern }
///
/// Allows defining custom capture types in stdlib `.st` files rather than
/// hardcoding them in Rust. This makes the pattern system fully extensible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureTypeDefAst {
    pub name: String,
    pub pattern: CapturePatternAst,
    #[serde(default)]
    pub span: SourceSpan,
    /// Which stdlib file this was loaded from (for incremental cache invalidation)
    #[serde(default)]
    pub source_file: Option<String>,
}
/// Bundle strategy for a `%vendor` dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VendorBundleStrategy {
    /// Dep is already a single self-contained file — copy + sha-pin, no build.
    Direct,
    /// Multi-file ESM/TS graph — flatten via a one-shot `bun build` into a
    /// single IIFE exposing `exports` on a stable global.
    Bun,
}

/// Vendored dependency definition: `%vendor name { source: …; entry: …; … }`
///
/// Declares a git-submodule-pinned third-party dependency and how to flatten it
/// into a single owned artifact (`out`). See `docs/stdlib/VENDORING.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VendorDefAst {
    pub name: String,
    /// Submodule path, relative to the declaring module dir (`source: submodule "…"`).
    pub source: String,
    /// Bundler entry path within the submodule (`entry: "…"`).
    pub entry: String,
    /// Flatten strategy (`bundle: bun | direct`).
    pub bundle: VendorBundleStrategy,
    /// Symbols hoisted onto the vendored global (`exports: { a, b }`).
    pub exports: Vec<String>,
    /// Generated single-file artifact path, relative to the module dir (`out: "…"`).
    pub out: String,
    /// SPDX license id (`license: MIT`), recorded; must match PROVENANCE.
    pub license: Option<String>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Which stdlib file this was loaded from (for incremental cache invalidation).
    #[serde(default)]
    pub source_file: Option<String>,
}
// =============================================================================

/// Provenance tag on a macro registered BY a migration (PLAN-079): the macro
/// is RETIRED syntax — it still matches (and, when it carries `%binds`, still
/// expands with its original semantics), but the pipeline flags every match
/// against the migration's wave: W0715 while the wave is newer than the
/// project's `@version`, E0911 once inert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetiredMacroTag {
    /// Owning migration id
    pub migration: String,
    /// Wave date (ISO YYYY-MM-DD)
    pub wave: String,
    /// `Some(rule_id)` marks a MATCH-ONLY rule def (synthesized from a
    /// `%rewrite` rule's `%match` — it must never reach Resolve; the shim
    /// consumes its matches). `None` marks an EMBEDDED retired macro (the
    /// verbatim old definition — it may expand normally).
    pub rule: Option<String>,
}

/// One mechanical rewrite rule inside a `%migration` (PLAN-079):
/// `%rewrite <rule-id> { %match { <form pattern> } %into { <template> } }`.
///
/// The `%match` half is EXACTLY the `%form` grammar (same parser path, same
/// capture types); it registers parse-side as a match-only macro (name
/// `<migration-id>#<rule-id>`) so retired call shapes produce a FormMatch.
///
/// The `%into` template is raw source text with backtick-quoted holes
/// (`` `$x` `` — the hole form; a bare `$x` is literal output). Holes splice
/// the RAW SOURCE SLICE of the corresponding `%match` capture (never a typed
/// re-serialization), so formatting/expressions survive verbatim.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MigrationRewriteAst {
    /// Rule id, unique within the migration (e.g. "bind-text") — surfaces in
    /// the pill, `migrate --explain`, and diagnostics.
    pub id: String,
    /// `%match { … }` — the retired call shape. Same grammar as `%form`.
    pub match_form: FormClause,
    /// The %match pattern's raw source text (display: pill expander,
    /// `migrate --explain`) — the structured form can't round-trip losslessly.
    #[serde(default)]
    pub match_source: String,
    /// `%into { … }` template text
    pub template: String,
    /// `%drops ( a b _ )` — extra call args the rule may silently drop
    /// (evidence: they never reached the old `%binds`). Names for named
    /// extras; `_` for positional groups. Without this clause, ANY extra arg
    /// degrades the rewrite (never silently truncate).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drops: Vec<String>,
    #[serde(default)]
    pub span: SourceSpan,
}

/// Manual-migration guidance for one retired directive:
/// `%hint for @show "…"`. Surfaces in W0715 (window open), E0910 (degrade
/// path), E0911 (inert wave), the pill's MANUAL rows, and
/// `migrate --explain`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MigrationHintAst {
    /// The retired directive this hint covers (stored @-trimmed, e.g. "show")
    pub directive: String,
    /// Guidance text
    pub text: String,
    #[serde(default)]
    pub span: SourceSpan,
}

/// A syntax migration (PLAN-076 capsule form, PLAN-079):
///
/// `%migration <id> { %date YYYY-MM-DD; %docs "…";
///   %macro <old> { … }…                — the retired definitions, VERBATIM
///   %rewrite <rule> { %match {…} %into {…} }…   — mechanical rewrites
///   %hint for @directive "…"…          — manual guidance }`
///
/// THE CAPSULE PRINCIPLE: a migration owns EVERYTHING about the syntax it
/// retires — the old grammar AND its semantics (the embedded `%macro`s,
/// registered as retired so old code keeps compiling through the window) —
/// so stdlib proper only ever carries CURRENT syntax. Deleting the wave file
/// deletes the old syntax from existence, in one motion.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MigrationDefAst {
    /// Unique migration id (e.g., "reactive-surface")
    pub id: String,
    /// Wave date, ISO `YYYY-MM-DD`. Lexicographic order == chronological
    /// order; THE wave key (a wave = every migration sharing one date).
    pub date: String,
    /// One-line human description — surfaces in the pill panel, `check`
    /// output, and diagnostics.
    pub docs: String,
    /// The retired macro definitions, embedded verbatim (grammar +
    /// `%binds` semantics). Registered as retired macros.
    #[serde(default)]
    pub macros: Vec<MacroDefAst>,
    /// Mechanical rewrite rules (per-shape)
    #[serde(default)]
    pub rewrites: Vec<MigrationRewriteAst>,
    /// Manual guidance (per directive)
    #[serde(default)]
    pub hints: Vec<MigrationHintAst>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Source file where this migration is defined (for error traces)
    #[serde(default)]
    pub source_file: Option<String>,
    /// Doc comment (///) collected above the definition
    #[serde(default)]
    pub doc: Option<String>,
}

impl MigrationDefAst {
    /// The registry name of a rewrite rule's match-only macro def:
    /// `<migration-id>#<rule-id>`. `#` never appears in authorable macro
    /// names, so rule defs can never collide with real defs.
    pub fn rule_macro_name(migration_id: &str, rule_id: &str) -> String {
        format!("{migration_id}#{rule_id}")
    }

    /// The registry name of an embedded macro's retired registration:
    /// `<migration-id>#@<macro-name>`. Two migrations may embed same-named
    /// macros (e.g. two waves touching @bind) — the migration qualifier
    /// keeps the name-keyed registries collision-free while
    /// `FormMatch.macro_name` still carries the DIRECTIVE for display.
    pub fn embedded_macro_name(migration_id: &str, macro_name: &str) -> String {
        format!("{migration_id}#@{macro_name}")
    }

    /// The macro defs this migration registers: one match-only def per
    /// rewrite rule (tagged retired, `rule: Some(id)`) PLUS every embedded
    /// macro (tagged retired, `rule: None`). Used by ALL registration sites
    /// (meta registry + both parse-side bootstrap paths) — one builder, no
    /// divergent synthetic-def logic.
    ///
    /// ORDER IS LOAD-BEARING: rule defs come FIRST so the parse-side
    /// first-match-wins form matcher tries the specific rule patterns before
    /// an embedded macro's broader form (e.g. `@bind(text: $x)` must hit
    /// rule `bind-text`, not the embedded `bind` macro's all-optional form —
    /// the embedded def is the catch-all for shapes no rule covers).
    pub fn registration_macros(&self) -> Vec<MacroDefAst> {
        let mut out = Vec::with_capacity(self.macros.len() + self.rewrites.len());
        for rule in &self.rewrites {
            out.push(MacroDefAst {
                name: Self::rule_macro_name(&self.id, &rule.id),
                form: Some(rule.match_form.clone()),
                retired: Some(RetiredMacroTag {
                    migration: self.id.clone(),
                    wave: self.date.clone(),
                    rule: Some(rule.id.clone()),
                }),
                span: rule.span,
                source_file: self.source_file.clone(),
                doc: self.doc.clone(),
                // The rule's `%into` template is the match captures' consumer —
                // carry it so the capture-consumption lint (E0964) sees it.
                rewrite_template: (!rule.template.is_empty())
                    .then(|| rule.template.clone()),
                // The rule's `%drops ( … )` are the match captures it
                // deliberately does NOT re-emit in `%into` — carry them so the
                // lint treats each as consumed-by-declaration (I4 / gh-18).
                drops: rule.drops.clone(),
                ..Default::default()
            });
        }
        for mac in &self.macros {
            let mut mac = mac.clone();
            mac.name = Self::embedded_macro_name(&self.id, &mac.name);
            mac.retired = Some(RetiredMacroTag {
                migration: self.id.clone(),
                wave: self.date.clone(),
                rule: None,
            });
            // The migration's file owns the embedded defs (incremental cache
            // / unregister_file key on it).
            if mac.source_file.is_none() {
                mac.source_file = self.source_file.clone();
            }
            out.push(mac);
        }
        out
    }

    /// Directives this migration retires (the embedded macros' form
    /// directive names, @-trimmed, deduped, in declaration order).
    pub fn retired_directives(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for mac in &self.macros {
            if let Some(form) = &mac.form {
                let d = form.directive_name.trim_start_matches('@').to_string();
                if !d.is_empty() && !out.contains(&d) {
                    out.push(d);
                }
            }
        }
        out
    }

    /// The hint covering a directive (@-trimmed), if any.
    pub fn hint_for(&self, directive: &str) -> Option<&str> {
        self.hints
            .iter()
            .find(|h| h.directive == directive)
            .map(|h| h.text.as_str())
    }

    /// Rewrite rules covering a directive (@-trimmed), in declaration order.
    pub fn rules_for(&self, directive: &str) -> Vec<&MigrationRewriteAst> {
        self.rewrites
            .iter()
            .filter(|r| r.match_form.directive_name.trim_start_matches('@') == directive)
            .collect()
    }

    /// True when the migration has at least one mechanical rewrite rule.
    pub fn is_rewrite(&self) -> bool {
        !self.rewrites.is_empty()
    }
}

// =============================================================================

/// The scalar kinds a `%field` may declare on a comment type. Deliberately
/// small: a comment's typed metadata is authored by humans in a `//@` header
/// and by agents over JSON, so every kind must survive both surfaces without
/// ceremony. Anything richer belongs in the comment's TEXT, not its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentFieldKind {
    Str,
    Number,
    Bool,
    Ident,
}

impl CommentFieldKind {
    /// Parse the author-facing spelling (`string`, `number`, `bool`, `ident`).
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "string" => Some(Self::Str),
            "number" => Some(Self::Number),
            "bool" => Some(Self::Bool),
            "ident" => Some(Self::Ident),
            _ => None,
        }
    }

    /// The author-facing spelling, for diagnostics and the types.json roster.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Str => "string",
            Self::Number => "number",
            Self::Bool => "bool",
            Self::Ident => "ident",
        }
    }
}

/// One typed field on a comment type: `%field acceptance string` (required)
/// or `%field priority string?` (optional).
///
/// SIGIL HARMONY: a SPACE introduces the TYPE, exactly as in `$price number`
/// on a param declaration — never `name: type`, which would read as a value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentFieldAst {
    pub name: String,
    pub kind: CommentFieldKind,
    /// `true` when the declaration ended in `?`.
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub span: SourceSpan,
}

/// A comment type (PLAN-123): the DATA behind one kind of `//@` comment.
///
/// `%comment_type agent-task { %label "…" %docs "…" %field acceptance string
///   %color "#e8a13d" %agent_hint "…" }`
///
/// TYPES ARE DATA, NOT RUST: declaring one — in `stdlib/comments/entries/` or
/// in a project's `_prelude.st` — is the ENTIRE act of adding it. It joins the
/// pill's type picker, MCP validation, and the `check` roster with no Rust
/// change, because every consumer reads this registry entry rather than
/// matching on a closed enum. That is the same kinds-as-data rail `%migration`
/// rides, and the reason a project can invent its own vocabulary.
///
/// A comment type has NO compile semantics. It describes comments, which are
/// trivia; nothing here ever reaches emitted output.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CommentTypeDefAst {
    /// Type id as written after `%comment_type` — the token an author types
    /// in `//@<id>(…): …`.
    pub id: String,
    /// Human label for the pill's picker and panel headings.
    pub label: String,
    /// One-line description of what the type MEANS — shown in the picker, the
    /// `check` roster, and the MCP `types` tool.
    pub docs: String,
    /// Typed metadata this type accepts in its `(…)` header.
    #[serde(default)]
    pub fields: Vec<CommentFieldAst>,
    /// Optional accent color for the pill chip.
    #[serde(default)]
    pub color: Option<String>,
    /// Instructions that travel WITH the comment to an agent that picks it up
    /// (inlined into every MCP list row). This is how a type carries its own
    /// follow-up contract instead of the agent inferring one.
    #[serde(default)]
    pub agent_hint: Option<String>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Source file this type was declared in — distinguishes a stdlib default
    /// from a project's `_prelude.st` type in diagnostics and the roster.
    #[serde(default)]
    pub source_file: Option<String>,
    /// Doc comment (`///`) collected above the declaration.
    #[serde(default)]
    pub doc: Option<String>,
    /// Field declarations whose KIND did not parse, as `(field, written-kind)`.
    /// Kept rather than dropped so load-time validation can name the offending
    /// field — a silently-vanished field would make a comment's shape lie.
    #[serde(default)]
    pub unknown_field_kinds: Vec<(String, String)>,
}

impl CommentTypeDefAst {
    /// Look up a declared field by name.
    pub fn field(&self, name: &str) -> Option<&CommentFieldAst> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The fields an author MUST supply in the `(…)` header.
    pub fn required_fields(&self) -> impl Iterator<Item = &CommentFieldAst> {
        self.fields.iter().filter(|f| !f.optional)
    }
}

/// A row in the scalar type table (FEAT-168 / PLAN-122 W4) — the SINGLE SOURCE
/// for what a scalar means to everything downstream of parsing: its JSON
/// schema, its zero value, and which admin control edits it. Declared in
/// `stdlib/scalars/types.st`.
///
/// A scalar is data, not code: adding a new one must require no Rust change,
/// which is the acceptance test for the wave. The `%capture` row joins this
/// table to css-values.st, which says what a scalar's SYNTAX is; this table
/// says what it MEANS.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScalarTypeDefAst {
    /// The scalar's name as written after `%scalar_type` — the token a
    /// consumer (type recognition, schema, zero, widget) looks up.
    pub id: String,
    /// The production in css-values.st that validates this scalar's literals
    /// (e.g. `"color"`). May be empty for base scalars that ARE the JSON
    /// primitives.
    pub capture: String,
    /// The JSON schema this scalar serializes as (`"string"`, `"number"`,
    /// `"boolean"`). REQUIRED.
    pub schema: String,
    /// A format string carried into the admin UI / serialized output. May be
    /// empty (richtext dispatches on a marker, the base scalars have none).
    pub format: String,
    /// The zero value — what an absent/unset scalar defaults to. REQUIRED to
    /// be DECLARED, but legitimately the empty string for most scalars (the
    /// empty string IS the zero), so presence is tracked in
    /// `missing_required_keys`, never by content.
    pub zero: String,
    /// Which admin control edits this scalar (e.g. `"color"`, `"toggle"`).
    /// REQUIRED.
    pub widget: String,
    /// Human documentation of what the scalar is.
    pub docs: String,
    /// Scalars that BEAT this one when a value matches both, most-preferred
    /// first (FEAT-109 `%loses_to`).
    ///
    /// Inference needs this because two scalars matching one value is normal and
    /// often meaningless: `string` accepts any bare identifier, so without a
    /// declared preference every keyword in the language comes back ambiguous
    /// with it. An entry here says "I am the fallback for that" — a judgement
    /// about what the scalars ARE, which is why it is declared rather than
    /// derived from the grammars (their languages overlap; there is no
    /// containment to compute).
    ///
    /// Empty for most rows. A pair not declared here stays
    /// `Inferred::Ambiguous`, which is the answer that never guesses.
    #[serde(default)]
    pub loses_to: Vec<String>,
    /// Required sub-clauses (`%schema`, `%zero`, `%widget`) that were MISSING
    /// from the declaration, kept so load-time validation can name them.
    /// Presence, not content: a declared `%zero ""` is fine, an absent `%zero`
    /// is a malformed row.
    #[serde(default)]
    pub missing_required_keys: Vec<String>,
    /// Sub-clause names inside the block that were NOT understood, kept so
    /// load-time validation can name a typo'd `%widgt` instead of silently
    /// dropping it — the exact failure class this table exists to remove.
    #[serde(default)]
    pub unknown_subclauses: Vec<String>,
    #[serde(default)]
    pub span: SourceSpan,
    /// Source file this scalar was declared in — distinguishes a stdlib default
    /// from a project override in diagnostics.
    #[serde(default)]
    pub source_file: Option<String>,
    /// Doc comment (`///`) collected above the declaration.
    #[serde(default)]
    pub doc: Option<String>,
}

/// Either a primitive, macro, preset, runtime registry, capture type, vendor,
/// migration, comment-type, or scalar-type definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetaDef {
    Primitive(PrimitiveDefAst),
    Macro(MacroDefAst),
    Preset(MetaPresetDefAst),
    RuntimeRegistry(RuntimeRegistryDef),
    CaptureType(CaptureTypeDefAst),
    Vendor(VendorDefAst),
    Migration(MigrationDefAst),
    CommentType(CommentTypeDefAst),
    ScalarType(ScalarTypeDefAst),
}
