//! Form Match - Unified AST Type
//!
//! FormMatch is the universal AST type for all user-facing syntax.
//! All directives are parsed into FormMatch with captures, and macro
//! metadata is derived from %registers, %scope, and %order clauses
//! in the MetaRegistry rather than hardcoded type checks.

use std::collections::HashMap;
use std::path::Path;

use crate::parser::SourceSpan;
use crate::parser::ast::JsonValue;
use crate::parser::meta_ast::BindValue;
use crate::utils::{escape_js_string, parse_time_to_ms};
use serde::{Deserialize, Serialize};

/// A captured value from pattern matching.
///
/// These correspond to the capture types defined in CaptureType (meta_ast.rs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapturedValue {
    /// Identifier: name, count, myVar
    Ident(String),

    /// String literal: "hello", "/api/data"
    String(String),

    /// Number: 100, 3.14, -5
    Number(f64),

    /// Boolean: true, false
    Bool(bool),

    /// Time duration: 500ms, 2s, 100us
    Time(u32),

    /// Length value: 100px, 50%, 2em
    Length(LengthValue),

    /// CSS selector: .class, #id, [attr]
    Selector(String),

    /// Binding reference: $variable, $data.property
    Binding(String),

    /// Element reference: &element
    Element(String),

    /// Arbitrary expression (unparsed)
    Expr(String),

    /// Type reference: Product[], User, string
    TypeRef(String),

    /// Preset reference: ~ease-out, ~fade-in
    Preset(String),

    /// JSON-like value (for defaults and complex values)
    Json(JsonValue),

    /// Nested block of FormMatches
    Block(Vec<FormMatch>),

    /// Array of captured values
    Array(Vec<CapturedValue>),

    /// Named arguments (key-value pairs)
    Named(HashMap<String, CapturedValue>),

    /// Structured property definitions from :properties capture
    Properties(Vec<PropertyDef>),

    /// Structured parameter definitions from :params capture
    Params(Vec<ParamDef>),

    /// Structured keyframe definitions from :keyframes capture
    Keyframes(Vec<KeyframeDef>),

    /// Template parameter list from :param_list capture
    /// (e.g., `$title, &content, $footer?`)
    ParamList(Vec<TemplateParamDef>),

    /// CSS property key-value pairs (e.g., `background: red; color: blue`)
    /// Distinct from `Properties` which stores typed schema field definitions.
    StyleProperties(Vec<(String, String)>),

    /// Pattern match result: `$signal is Variant { $bindings }`
    PatternMatch {
        signal: String,
        variant: String,
        bindings: Vec<String>,
    },

    /// CSS color value: #hex, rgb(), hsl(), oklch(), named colors, etc.
    Color(String),

    /// Component-body marker (FEAT-120). A bare presence-marker: it records that this
    /// match HAS a `component_body` capture (so `expand` lowers the body via the
    /// World-A scope), but carries NO data — the factory payload comes from the
    /// `@template:<name>` scope, and body-validation diagnostics land in
    /// `StFile.diagnostics`. Retired the `ComponentBodyDef` envelope it used to wrap.
    ComponentBody,
}

/// A length value with unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LengthValue {
    pub value: f64,
    pub unit: String,
}

/// A field definition in a type (e.g., `name: string` or `price?: number`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDef {
    pub name: String,
    pub type_ref: String,
    pub optional: bool,
}

/// A function parameter (e.g., `count: number = 0`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub type_ref: String,
    pub default: Option<String>,
}

/// A keyframe definition (e.g., `opacity: 0 -> 1 -> 0.8`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyframeDef {
    pub property: String,
    pub values: Vec<String>,
    /// Nested-scope selector when this entry came from a `sel { prop: a -> b; }`
    /// block inside the keyframes body (BUG-201) — `None` for root-level
    /// property lines. The apply-animations runtime consumes these as
    /// `anims.scopes` (per-selector nested choreography: one driver, children
    /// animate at different rates).
    #[serde(default)]
    pub selector: Option<String>,
}

/// A template parameter definition (e.g., `$title`, `&content`, `$footer?`)
///
/// This represents a parameter in a template macro, where:
/// - Binding parameters (`$name`) pass data values
/// - Element parameters (`&name`) pass HTML element references
/// - Optional parameters have a `?` suffix
///
/// # Pure Spacetime Definition
///
/// In a future version, this could be defined using %capture_type:
///
/// ```st
/// %capture_type param_list {
///   ( ( $name:binding | &name:element ) "?"? ","? )*
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateParamDef {
    pub name: String,
    pub kind: TemplateParamKind,
    pub optional: bool,
    /// Optional type annotation (SPACE form, e.g. `$href url` -> Some("url")).
    /// Drives editable schema emission + inline-edit widget dispatch (FEAT-097).
    /// `None` for untyped params (the legacy `$title`/`&content` shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_ref: Option<String>,
    /// Optional default value source (e.g. `$alt string = ""` -> Some("\"\"")).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Collection marker: `true` when the param carried a trailing `[]`
    /// (`&items[]`). On an element param this marks a BLOCK editable region
    /// (repeating child blocks) vs a bare inline region (FUP-042).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub collection: bool,
}

/// Kind of template parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateParamKind {
    /// Binding parameter: `$name` - passes data values
    Binding,
    /// Element parameter: `&name` - passes HTML element references
    Element,
}

/// A state declaration inside a @template body: `$open bool: false;`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentStateDecl {
    /// Variable name without `$` prefix (e.g., "open")
    pub var_name: String,
    /// Type annotation (e.g., "bool", "number", "string")
    pub type_name: String,
    /// Typed initial value — Bool(false), Number(0.0), String("hello")
    pub initial: CapturedValue,
}

/// An export declaration: `$count` or `$count: mut`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDecl {
    pub var_name: String, // without $
    pub mutable: bool,
}

/// A template ref invocation inside a component body.
/// Represents `&likeCounter &counter("Likes");` or `&cards[] &counter($item);`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateRef {
    /// Optional ref name (e.g., "likeCounter" from `&likeCounter &counter(...)`).
    /// None for anonymous invocations like `&counter("Likes")`.
    pub ref_name: Option<String>,
    /// Template name being invoked (e.g., "counter")
    pub template_name: String,
    /// Arguments passed to the template factory
    pub args: Vec<String>,
    /// Whether this is a collection ref (`&cards[]`) — only valid inside @each
    pub is_collection: bool,
    /// Per-arg parameter NAME for named invocations (BUG-131): `&card(title: "A")`
    /// stores `arg_names[i] = Some("title")` with `args[i] = "\"A\""` (the bare
    /// value). Positional args leave the slot `None`. Parsed ONCE at compile by
    /// `normalize_invocation_args` so both consumers — the runtime factory (binds
    /// `data[name] = value`) and the structure projection (inspector display) —
    /// read the same structured form instead of re-splitting the raw text. When
    /// EVERY slot is `None` (the common positional case) the field is omitted from
    /// the serialized bundle, so existing `refs:` snapshots are unaffected.
    #[serde(default, skip_serializing_if = "all_arg_names_none")]
    pub arg_names: Vec<Option<String>>,
    /// Byte span of the invocation in source (FUP-093/G2): the EditAst write
    /// address for editing THIS invocation's args. `#[serde(skip)]` keeps it out
    /// of the serialized bundle (it is an internal addressing aid, not contract
    /// data) so existing `refs:` snapshots are unaffected. Zero span = unknown.
    #[serde(skip)]
    pub span: SourceSpan,
}

/// serde guard: omit `arg_names` from the bundle when no slot is named (every
/// positional invocation) so existing `refs:` snapshots are byte-identical.
fn all_arg_names_none(v: &[Option<String>]) -> bool {
    v.iter().all(|n| n.is_none())
}

/// Split a raw invocation arg list into (values, names) (BUG-131). Each arg is
/// either positional (`"A"`, `$x`, `42`) — name `None`, value verbatim — or named
/// (`title: "A"`) — name `Some("title")`, value the text after the FIRST top-level
/// `:`. This is the ONE place named args are parsed; both the runtime factory and
/// the structure projection consume the structured result. A `:` inside a quoted
/// string or bracket is NOT a separator. `args` are already comma-split at depth 0
/// by the caller.
pub fn normalize_invocation_args(args: &[String]) -> (Vec<String>, Vec<Option<String>>) {
    let mut values = Vec::with_capacity(args.len());
    let mut names = Vec::with_capacity(args.len());
    for raw in args {
        let (name, value) = split_named_arg(raw);
        names.push(name);
        values.push(value);
    }
    (values, names)
}

/// Split one arg into (optional name, value). A leading `ident :` prefix (the `:`
/// at bracket/quote depth 0) marks a named arg; the name must be a bare identifier
/// (alnum/`_`/`-`). Otherwise the whole arg is the positional value.
fn split_named_arg(raw: &str) -> (Option<String>, String) {
    let s = raw.trim();
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_str: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
        } else {
            match c {
                b'"' | b'\'' => in_str = Some(c),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b':' if depth == 0 => {
                    let name = s[..i].trim();
                    let value = s[i + 1..].trim();
                    if !name.is_empty()
                        && name
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    {
                        return (Some(name.to_string()), value.to_string());
                    }
                    return (None, s.to_string());
                }
                _ => {}
            }
        }
        i += 1;
    }
    (None, s.to_string())
}

// FEAT-120: `ComponentBodyDef` + `ComponentBodyDiag` are DELETED. The body validator
// (`validate_component_body`) now returns canonical `Vec<Diagnostic>` straight into
// `StFile.diagnostics`; the `ComponentBody` capture is a bare presence-marker. There is
// no longer a second diagnostic type or a capture-borne validation channel.

impl LengthValue {
    pub fn new(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value,
            unit: unit.into(),
        }
    }

    pub fn px(value: f64) -> Self {
        Self::new(value, "px")
    }

    pub fn percent(value: f64) -> Self {
        Self::new(value, "%")
    }

    pub fn em(value: f64) -> Self {
        Self::new(value, "em")
    }
}

impl CapturedValue {
    /// Convert a ParamDefault to a CapturedValue
    pub fn from_default(default: &crate::parser::meta_ast::ParamDefault) -> Self {
        use crate::parser::meta_ast::ParamDefault;
        match default {
            ParamDefault::String(s) => CapturedValue::String(s.clone()),
            ParamDefault::Number(n) => CapturedValue::Number(*n),
            ParamDefault::Bool(b) => CapturedValue::Bool(*b),
            ParamDefault::None => CapturedValue::Expr("null".to_string()),
            ParamDefault::EmptyArray => CapturedValue::Array(vec![]),
            ParamDefault::EmptyObject => CapturedValue::Named(HashMap::new()),
            ParamDefault::Array(items) => {
                CapturedValue::Array(items.iter().map(CapturedValue::from_default).collect())
            }
            ParamDefault::Length(val, unit) => {
                CapturedValue::Length(LengthValue::new(*val, unit.clone()))
            }
        }
    }

    /// Extract a string-literal value from this capture, whether it was
    /// captured as a `String` or as a quoted string-literal `Expr`.
    ///
    /// The unified `@data <kind>` surface captures source URLs via `$src:expr`,
    /// so a fetch source like `: "/data/x.json"` arrives as
    /// `Expr("\"/data/x.json\"")` rather than `String("/data/x.json")`. This
    /// helper unifies both so consumers (data-source analysis, etc.) read one
    /// shape. Only the leading quoted token is returned, so trailing tokens the
    /// expression matcher may fold in (e.g. doc comments after the literal) are
    /// ignored.
    pub fn as_string_literal(&self) -> Option<&str> {
        match self {
            CapturedValue::String(s) => Some(s.as_str()),
            CapturedValue::Expr(e) => {
                let s = e.trim_start();
                let quote = s.chars().next().filter(|c| *c == '"' || *c == '\'')?;
                let rest = &s[1..];
                let end = rest.find(quote)?;
                Some(&rest[..end])
            }
            _ => None,
        }
    }

    /// Extract a type name from this capture, whether it was captured as a
    /// `TypeRef` (`Product[]`, `User`) or a bare `Ident` (`string`, `number`).
    ///
    /// Macros vary in how they surface a type position depending on whether the
    /// matcher recognized it as a type reference, so analysis consumers that
    /// want "the type name as written" should read through this rather than
    /// matching variants inline (which historically drifted per call site).
    pub fn as_type_name(&self) -> Option<&str> {
        match self {
            CapturedValue::TypeRef(t) | CapturedValue::Ident(t) => Some(t.as_str()),
            _ => None,
        }
    }

    /// Returns true if the value is body-like (StyleProperties or Block).
    /// Used to identify body arguments in macro calls.
    pub fn is_body_like(&self) -> bool {
        matches!(self, Self::StyleProperties(_) | Self::Block(_))
    }

    /// Extract properties from a body-like value.
    pub fn field_properties(&self) -> Vec<(String, String)> {
        match self {
            CapturedValue::StyleProperties(props) => props.clone(),
            _ => vec![],
        }
    }

    /// Check if the value is truthy (non-empty, non-zero, non-false).
    pub fn is_truthy(&self) -> bool {
        match self {
            CapturedValue::String(s) => !s.is_empty(),
            CapturedValue::Ident(s) => {
                !s.is_empty() && s != "none" && s != "false" && s != "undefined" && s != "null"
            }
            CapturedValue::Number(n) => *n != 0.0,
            CapturedValue::Bool(b) => *b,
            CapturedValue::Time(t) => *t != 0,
            CapturedValue::Length(l) => l.value != 0.0,
            CapturedValue::Selector(s) => !s.is_empty(),
            CapturedValue::Binding(_) => true,
            CapturedValue::Element(_) => true,
            CapturedValue::Expr(e) => {
                !e.is_empty() && e != "undefined" && e != "null" && e != "false"
            }
            CapturedValue::TypeRef(t) => !t.is_empty(),
            CapturedValue::Preset(p) => !p.is_empty(),
            CapturedValue::Color(s) => !s.is_empty(),
            CapturedValue::Json(_) => true,
            CapturedValue::Block(b) => !b.is_empty(),
            CapturedValue::Array(a) => !a.is_empty(),
            CapturedValue::Named(m) => !m.is_empty(),
            CapturedValue::Properties(p) => !p.is_empty(),
            CapturedValue::Params(p) => !p.is_empty(),
            CapturedValue::Keyframes(k) => !k.is_empty(),
            CapturedValue::ParamList(p) => !p.is_empty(),
            CapturedValue::StyleProperties(p) => !p.is_empty(),
            CapturedValue::PatternMatch { .. } => true,
            CapturedValue::ComponentBody => true,
        }
    }

    /// Convert to a display string for substitution contexts.
    pub fn to_string_value(&self) -> String {
        match self {
            CapturedValue::String(s) => s.clone(),
            CapturedValue::Ident(s) => s.clone(),
            CapturedValue::Number(n) => n.to_string(),
            CapturedValue::Bool(b) => b.to_string(),
            CapturedValue::Time(t) => format!("{}ms", t),
            CapturedValue::Length(l) => format!("{}{}", l.value, l.unit),
            CapturedValue::Selector(s) => s.clone(),
            CapturedValue::Binding(s) => s.clone(),
            CapturedValue::Element(s) => s.clone(),
            CapturedValue::Expr(e) => e.clone(),
            CapturedValue::TypeRef(t) => t.clone(),
            CapturedValue::Preset(p) => p.clone(),
            CapturedValue::Color(s) => s.clone(),
            CapturedValue::Json(j) => serde_json::to_string(j).unwrap_or_default(),
            CapturedValue::Block(_) => "[block]".to_string(),
            CapturedValue::Array(items) => {
                let strs: Vec<String> = items.iter().map(|i| i.to_string_value()).collect();
                format!("[{}]", strs.join(", "))
            }
            // A structured capture renders its FIELDS, not the opaque word
            // "[named]". Every capture type that composes (a driver_expr, a
            // score placement, an arm) lands here, so rendering it opaquely
            // made `inspect` blind exactly where a grammar is hardest to get
            // right — you could see THAT a capture matched but never WHAT it
            // held. Sorted so the output is deterministic and diffable.
            CapturedValue::Named(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let fields: Vec<String> = keys
                    .iter()
                    .map(|k| format!("{k}: {}", map[*k].to_string_value()))
                    .collect();
                format!("{{{}}}", fields.join(", "))
            }
            CapturedValue::ComponentBody => "[component-body]".to_string(),
            CapturedValue::Properties(_) => "[properties]".to_string(),
            CapturedValue::Params(_) => "[params]".to_string(),
            CapturedValue::Keyframes(_) => "[keyframes]".to_string(),
            CapturedValue::ParamList(_) => "[param-list]".to_string(),
            CapturedValue::StyleProperties(_) => "[style-properties]".to_string(),
            CapturedValue::PatternMatch {
                signal, variant, ..
            } => {
                format!("{} is {}", signal, variant)
            }
        }
    }

    /// Convert to a numeric value for comparison contexts.
    pub fn to_number_value(&self) -> f64 {
        match self {
            CapturedValue::Number(n) => *n,
            CapturedValue::String(s) => s.parse().unwrap_or(0.0),
            CapturedValue::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            CapturedValue::Ident(s) => s.parse().unwrap_or(0.0),
            CapturedValue::Time(t) => *t as f64,
            CapturedValue::Length(l) => l.value,
            _ => 0.0,
        }
    }
}

/// A generic syntax match - the unified AST type.
///
/// Universal AST type for all user-facing syntax directives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormMatch {
    /// Which %macro matched (e.g., "data-fetch", "local-state", "element-ref")
    pub macro_name: String,

    /// The exact `%macro` definition name that parse selected for this match
    /// (e.g. "data-inline" vs "data-derive"), preserved even when `macro_name`
    /// collapses to the shared `%creates` directive name ("data"). Resolve/evaluate
    /// HONOR this instead of re-guessing the macro from captures — parse already
    /// disambiguated via literal kind-words the capture scorer cannot see.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_macro: Option<String>,

    /// Captured values from the %form pattern
    /// Keys are the capture names (e.g., "name", "type", "value")
    pub captures: HashMap<String, CapturedValue>,

    /// Per-capture source spans for precise error reporting.
    /// Keys match those in `captures`. Enables pointing to specific captures in diagnostics.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub capture_spans: HashMap<String, SourceSpan>,

    /// Scope context (CSS selector) if this match is inside a scope block
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,

    /// Source span for error reporting
    #[serde(default)]
    pub span: SourceSpan,

    /// Source file where this FormMatch is defined (for error traces)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,

    /// FEAT-118 FUP-055: the namespace QUALIFIER on a qualified invocation —
    /// the `scene` (or alias `s`) in `@scene/camera` / `@s/camera`. `None` for
    /// an unqualified call. The matcher records this (rather than discarding the
    /// skipped prefix) so dispatch can resolve it against the importing file's
    /// ImportScope into a fully-qualified namespace, and disambiguate
    /// cross-namespace bare-name collisions. Stored without trailing `/`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub namespace_qualifier: Vec<String>,

    /// Joined text of the contiguous `///` doc-comment block immediately
    /// preceding this match in its source file (PLAN-135 W4) — the same
    /// block rule as %macro docs (FEAT-083, `collect_doc_comment`), attached
    /// at parse time by `fill_match_docs`. `None` when undocumented. The
    /// form-declaration catalog (`@data forms`) serves this as the row's
    /// `doc`; other matches simply never read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}


/// Collect the contiguous `///` doc-comment block for the match whose span
/// starts at `offset` (PLAN-135 W4). The events parser's match spans cover
/// leading AND trailing trivia, so the doc block sits in one of two places:
///
///   1. LEADING — inside this match's own span (blank lines, then the `///`
///      block, then the directive token). The common case.
///   2. SWALLOWED — the PREVIOUS match's span ate this match's leading
///      trivia as trailing whitespace/comments, so the `///` block sits in
///      the bytes directly ABOVE `offset`. Only a block with no blank line
///      between it and the directive qualifies — anything further back
///      belongs to the previous construct, not this one.
///
/// The events-parser twin of the CST `collect_doc_comment`
/// (src/parser/mod.rs): same contiguous-block rule, no sibling walk needed.
/// The FILE-HEADER doc block: the contiguous `///` lines that open a source
/// file, before any declaration.
///
/// A single-pattern module documents ITSELF at the top — every file in
/// `stdlib/showcases/` opens with `/// scenario:` / `/// school:` / … at line 1
/// and declares its `@template` at line 15, separated by an `@import` and a
/// block comment. `doc_comment_before` (correctly) finds no CONTIGUOUS block
/// above the declaration, so the metadata that describes the pattern would be
/// invisible to the catalog without this (PLAN-144 W2).
///
/// Used only as a FALLBACK: a declaration's own doc block always wins, so a
/// multi-declaration file is never mislabelled by its header.
pub fn file_header_doc(source: &str) -> Option<String> {
    let mut lines = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() && lines.is_empty() {
            continue; // leading blank lines before the header
        }
        let Some(rest) = t.strip_prefix("///") else {
            break; // the header ends at the first non-doc line
        };
        if rest.starts_with('/') {
            break; // `////` is a section rule, not a doc line
        }
        lines.push(rest.strip_prefix(' ').unwrap_or(rest).trim_end().to_string());
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

pub fn doc_comment_before(source: &str, offset: usize) -> Option<String> {
    fn strip_doc_line(line: &str) -> Option<String> {
        let rest = line.trim().strip_prefix("///")?;
        if rest.starts_with('/') {
            return None; // `////` is a section rule, not a doc line
        }
        Some(rest.strip_prefix(' ').unwrap_or(rest).trim_end().to_string())
    }

    // 1. LEADING: inside the span, directly above the directive token. The
    // span may open with unrelated trivia — blank lines, a `/* */` file
    // header — so locate the directive line first (the first non-blank,
    // non-comment line), then collect the contiguous `///` block directly
    // above it.
    if let Some(from) = source.get(offset.min(source.len())..) {
        let lines: Vec<&str> = from.lines().collect();
        let mut directive_idx: Option<usize> = None;
        let mut in_block_comment = false;
        for (i, raw) in lines.iter().enumerate() {
            let t = raw.trim();
            if in_block_comment {
                if t.contains("*/") {
                    in_block_comment = false;
                }
                continue;
            }
            if t.is_empty() {
                continue;
            }
            if t.starts_with("/*") {
                if !t.contains("*/") {
                    in_block_comment = true;
                }
                continue;
            }
            if t.starts_with("//") {
                continue;
            }
            directive_idx = Some(i);
            break;
        }
        if let Some(di) = directive_idx {
            let mut block: Vec<String> = Vec::new();
            for raw in lines[..di].iter().rev() {
                let t = raw.trim();
                if t.is_empty() {
                    break; // a blank line between the block and the directive detaches it
                }
                match strip_doc_line(t) {
                    Some(text) => block.push(text),
                    None => break,
                }
            }
            if !block.is_empty() {
                block.reverse();
                return Some(block.join("\n"));
            }
        }
    }

    // 2. SWALLOWED: the block directly above the span, no blank gap.
    let before = source.get(..offset.min(source.len()))?;
    let mut lines: Vec<String> = Vec::new();
    for raw in before.lines().rev() {
        match strip_doc_line(raw) {
            Some(text) => lines.push(text),
            None => break, // a blank or code line: the block (if any) ends
        }
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

impl FormMatch {
    /// Create a new FormMatch with no captures.
    pub fn new(macro_name: impl Into<String>) -> Self {
        Self {
            macro_name: macro_name.into(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    /// Create a FormMatch with captures (without spans).
    pub fn with_captures(
        macro_name: impl Into<String>,
        captures: HashMap<String, CapturedValue>,
    ) -> Self {
        Self {
            macro_name: macro_name.into(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    /// Set the selector context.
    pub fn in_scope(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }

    /// Set the source span.
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = span;
        self
    }

    /// Add a capture (without span).
    pub fn capture(mut self, name: impl Into<String>, value: CapturedValue) -> Self {
        self.captures.insert(name.into(), value);
        self
    }

    /// Add a capture with its source span for precise diagnostics.
    pub fn capture_with_span(
        mut self,
        name: impl Into<String>,
        value: CapturedValue,
        span: SourceSpan,
    ) -> Self {
        let key = name.into();
        self.captures.insert(key.clone(), value);
        self.capture_spans.insert(key, span);
        self
    }

    /// Get the span for a specific capture, if available.
    pub fn get_capture_span(&self, name: &str) -> Option<SourceSpan> {
        self.capture_spans.get(name).copied()
    }

    /// Get a captured value by name.
    pub fn get(&self, name: &str) -> Option<&CapturedValue> {
        self.captures.get(name)
    }

    /// Get a captured identifier.
    pub fn get_ident(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(CapturedValue::Ident(s)) => Some(s),
            _ => None,
        }
    }

    /// Get a captured string.
    pub fn get_string(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(CapturedValue::String(s)) => Some(s),
            _ => None,
        }
    }

    /// Get a captured type name, accepting either a `TypeRef` or a bare `Ident`
    /// in the named position. See [`CapturedValue::as_type_name`].
    pub fn type_name(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|v| v.as_type_name())
    }

    /// Get a captured string literal, accepting either a `String` or a quoted
    /// string-literal `Expr`. See [`CapturedValue::as_string_literal`].
    pub fn string_literal(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|v| v.as_string_literal())
    }

    /// Get a captured number.
    pub fn get_number(&self, name: &str) -> Option<f64> {
        match self.get(name) {
            Some(CapturedValue::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// Get a captured boolean.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.get(name) {
            Some(CapturedValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Get a captured time value (in milliseconds).
    pub fn get_time(&self, name: &str) -> Option<u32> {
        match self.get(name) {
            Some(CapturedValue::Time(t)) => Some(*t),
            _ => None,
        }
    }

    /// Get a captured selector.
    pub fn get_selector(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(CapturedValue::Selector(s)) => Some(s),
            _ => None,
        }
    }

    /// Get a captured binding reference.
    pub fn get_binding(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(CapturedValue::Binding(s)) => Some(s),
            _ => None,
        }
    }

    /// Get a captured expression.
    pub fn get_expr(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(CapturedValue::Expr(s)) => Some(s),
            _ => None,
        }
    }

    /// Get a captured block.
    pub fn get_block(&self, name: &str) -> Option<&[FormMatch]> {
        match self.get(name) {
            Some(CapturedValue::Block(b)) => Some(b),
            _ => None,
        }
    }

    /// Get properties if the capture is a Properties variant.
    pub fn get_properties(&self, name: &str) -> Option<&Vec<PropertyDef>> {
        match self.get(name) {
            Some(CapturedValue::Properties(props)) => Some(props),
            _ => None,
        }
    }

    /// Get params if the capture is a Params variant.
    pub fn get_params(&self, name: &str) -> Option<&Vec<ParamDef>> {
        match self.get(name) {
            Some(CapturedValue::Params(params)) => Some(params),
            _ => None,
        }
    }

    /// Get keyframes if the capture is a Keyframes variant.
    pub fn get_keyframes(&self, name: &str) -> Option<&Vec<KeyframeDef>> {
        match self.get(name) {
            Some(CapturedValue::Keyframes(kf)) => Some(kf),
            _ => None,
        }
    }

    /// Get the template parameter list if the capture is a ParamList variant
    /// (the `param_list` capture type — `@template`/`@editable-mark` params,
    /// carrying name/kind/optional/type_ref/default).
    pub fn get_param_list(&self, name: &str) -> Option<&Vec<TemplateParamDef>> {
        match self.get(name) {
            Some(CapturedValue::ParamList(params)) => Some(params),
            _ => None,
        }
    }

    /// Check if a capture exists and is not None/empty.
    pub fn has(&self, name: &str) -> bool {
        self.captures.contains_key(name)
    }

    /// Check if this match is in a scope context.
    pub fn is_scoped(&self) -> bool {
        self.selector.is_some()
    }

    /// Convert captures to PatternArg vec for macro processing.
    ///
    /// Each (name, CapturedValue) pair maps directly to a PatternArg.
    /// This bridges FormMatch captures into the metasystem expansion API.
    pub fn to_pattern_args(&self) -> Vec<crate::parser::PatternArg> {
        self.captures
            .iter()
            .map(|(name, value)| crate::parser::PatternArg {
                name: name.clone(),
                value: value.clone(),
            })
            .collect()
    }
}

impl Default for FormMatch {
    fn default() -> Self {
        Self::new("")
    }
}

// =============================================================================
// Unified BindValue/CapturedValue → JS Conversion
// =============================================================================

/// Controls how string values are quoted when converting to JavaScript.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsQuoting {
    /// Wrap in single quotes: 'value' — used by compile.rs (declaration graph)
    SingleQuoted,
    /// Raw values, no wrapping — used by expand.rs (primitive templates add their own quotes).
    /// Also enables time-parsing (e.g. "300ms" → 300) and animation keyframe conversion.
    Raw,
    /// Wrap in double quotes: "value" — used by resolve.rs (expression expansion)
    DoubleQuoted,
}

/// Trait for resolving variable names to captured values.
///
/// Implemented by `MacroContext` (expand.rs) and `HashMap<String, CapturedValue>` (resolve.rs).
pub trait VariableResolver {
    fn resolve_variable(&self, name: &str) -> Option<&CapturedValue>;
}

/// Blanket impl for HashMap — used by resolve.rs expression expansion.
impl VariableResolver for HashMap<String, CapturedValue> {
    fn resolve_variable(&self, name: &str) -> Option<&CapturedValue> {
        self.get(name)
    }
}

impl CapturedValue {
    /// Convert this CapturedValue to a JavaScript literal string,
    /// parameterized by quoting mode.
    pub fn to_js(&self, quoting: JsQuoting) -> String {
        // Helper: wrap a string value according to quoting mode
        let quote_str = |s: &str| -> String {
            match quoting {
                JsQuoting::SingleQuoted => format!("'{}'", escape_js_string(s)),
                JsQuoting::DoubleQuoted => {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
                JsQuoting::Raw => s.replace('\'', "\\'"),
            }
        };

        match self {
            CapturedValue::String(s) => {
                // Defensive: strip outer quotes if they weren't stripped earlier in the pipeline
                let s = s.trim();
                let unquoted = if (s.starts_with('"') && s.ends_with('"'))
                    || (s.starts_with('\'') && s.ends_with('\''))
                {
                    &s[1..s.len() - 1]
                } else {
                    s
                };
                // Raw mode: time-parsing for strings like "300ms"
                if quoting == JsQuoting::Raw
                    && let Some(ms) = parse_time_to_ms(unquoted)
                {
                    return ms.to_string();
                }
                quote_str(unquoted)
            }
            CapturedValue::Number(n) => n.to_string(),
            CapturedValue::Bool(b) => b.to_string(),
            CapturedValue::Ident(s) => {
                // DoubleQuoted: strip $ prefix for bindings captured as Ident
                if quoting == JsQuoting::DoubleQuoted {
                    let name = s.strip_prefix('$').unwrap_or(s);
                    return format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""));
                }
                // Raw mode: time-parsing for ident values like "300ms"
                if quoting == JsQuoting::Raw {
                    if let Some(ms) = parse_time_to_ms(s) {
                        return ms.to_string();
                    }
                    return s.clone();
                }
                // SingleQuoted
                format!("'{}'", escape_js_string(s))
            }
            CapturedValue::Time(t) => t.to_string(),
            CapturedValue::Length(l) => match quoting {
                JsQuoting::SingleQuoted => format!("'{}{}'", l.value, l.unit),
                JsQuoting::DoubleQuoted => format!("\"{}{}\"", l.value, l.unit),
                JsQuoting::Raw => format!("{}{}", l.value, l.unit),
            },
            CapturedValue::Selector(s) => quote_str(s),
            CapturedValue::Color(s) => quote_str(s),
            CapturedValue::Binding(s) => {
                // Strip the $ prefix — it's syntax, not part of the name
                let name = s.strip_prefix('$').unwrap_or(s);
                match quoting {
                    JsQuoting::SingleQuoted => format!("'{}'", escape_js_string(name)),
                    JsQuoting::DoubleQuoted => format!("\"{}\"", name),
                    JsQuoting::Raw => name.to_string(),
                }
            }
            CapturedValue::Element(s) => match quoting {
                JsQuoting::DoubleQuoted => s.clone(),
                _ => format!("'{}'", escape_js_string(s)),
            },
            CapturedValue::Expr(e) => {
                // A raw JS expression is spliced VERBATIM in every quoting mode
                // (it is code, not a string value) — callers that need it as a
                // JS string literal quote+escape it themselves. Raw mode also
                // drops a leading `$`.
                if quoting == JsQuoting::Raw {
                    e.trim_start_matches('$').to_string()
                } else {
                    e.clone()
                }
            }
            CapturedValue::TypeRef(s) => quote_str(s),
            CapturedValue::Preset(s) => quote_str(s),
            CapturedValue::Json(j) => match quoting {
                JsQuoting::DoubleQuoted => {
                    serde_json::to_string(j).unwrap_or_else(|_| "null".to_string())
                }
                _ => format!("{:?}", j),
            },
            CapturedValue::Array(items) => {
                let js_items: Vec<String> = items
                    .iter()
                    .map(|item| {
                        if quoting == JsQuoting::Raw {
                            // Raw mode: per-item quoting for arrays
                            match item {
                                CapturedValue::String(s) => format!("'{}'", s.replace('\'', "\\'")),
                                CapturedValue::Ident(s) => format!("'{}'", s.replace('\'', "\\'")),
                                CapturedValue::Number(n) => n.to_string(),
                                CapturedValue::Bool(b) => b.to_string(),
                                _ => "null".to_string(),
                            }
                        } else {
                            item.to_js(quoting)
                        }
                    })
                    .collect();
                format!("[{}]", js_items.join(", "))
            }
            CapturedValue::Named(map) => {
                match quoting {
                    JsQuoting::DoubleQuoted => {
                        // Sort keys for deterministic JS output — HashMap iteration
                        // order is random per instance, which would cause the
                        // _st_init_ content hash (and thus guard name) to vary.
                        let mut pairs: Vec<String> = map
                            .iter()
                            .map(|(k, v)| format!("\"{}\": {}", k, v.to_js(quoting)))
                            .collect();
                        pairs.sort();
                        format!("{{ {} }}", pairs.join(", "))
                    }
                    _ => "null".to_string(),
                }
            }
            CapturedValue::StyleProperties(props) => {
                if quoting == JsQuoting::Raw {
                    // Raw mode: animation detection + keyframe conversion
                    // Delegate to expand.rs helpers via crate-level functions
                    let is_animation = props.iter().any(|(_, v)| v.contains("->"));
                    if is_animation {
                        return crate::metasystem::convert_properties_to_keyframes_js(props);
                    } else if props.len() == 1 {
                        // Single property — extract just the value
                        let (_, val) = &props[0];
                        let trimmed = val.trim();
                        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
                        {
                            return trimmed[1..trimmed.len() - 1].to_string();
                        } else {
                            return val.clone();
                        }
                    }
                    // Fall through to generic object conversion
                }
                if quoting == JsQuoting::DoubleQuoted {
                    let props_str: Vec<String> = props
                        .iter()
                        .map(|(k, v)| {
                            format!(
                                "\"{}\": \"{}\"",
                                k,
                                v.replace('\\', "\\\\").replace('"', "\\\"")
                            )
                        })
                        .collect();
                    return format!("{{ {} }}", props_str.join(", "));
                }
                // SingleQuoted or Raw (multi-property non-animation)
                let js_props: Vec<String> = props
                    .iter()
                    .map(|(k, v)| {
                        let v_trimmed = v.trim();
                        let value_str = if (v_trimmed.starts_with('"') && v_trimmed.ends_with('"'))
                            || (v_trimmed.starts_with('\'') && v_trimmed.ends_with('\''))
                        {
                            v_trimmed.to_string()
                        } else {
                            format!("'{}'", escape_js_string(v))
                        };
                        format!("'{}': {}", k, value_str)
                    })
                    .collect();
                format!("{{{}}}", js_props.join(", "))
            }
            CapturedValue::Block(forms) => {
                // Block contains nested FormMatches — convert to JS array of directive names
                if forms.is_empty() {
                    return "null".to_string();
                }
                // For now, blocks that reach to_js() return null
                // (body expansion via expand_body_value handles them before they reach here)
                "null".to_string()
            }
            CapturedValue::Properties(props) => match quoting {
                JsQuoting::DoubleQuoted => {
                    let props_str: Vec<String> =
                        props.iter().map(|p| format!("\"{}\"", p.name)).collect();
                    format!("[{}]", props_str.join(", "))
                }
                _ => "null".to_string(),
            },
            CapturedValue::Params(params) => match quoting {
                JsQuoting::DoubleQuoted => {
                    let params_str: Vec<String> =
                        params.iter().map(|p| format!("\"{}\"", p.name)).collect();
                    format!("[{}]", params_str.join(", "))
                }
                _ => "null".to_string(),
            },
            CapturedValue::Keyframes(kfs) => match quoting {
                JsQuoting::DoubleQuoted => {
                    let kfs_str: Vec<String> = kfs
                        .iter()
                        .map(|k| format!("\"{}\": {}", k.property, k.values.join(" -> ")))
                        .collect();
                    format!("{{ {} }}", kfs_str.join(", "))
                }
                _ => "null".to_string(),
            },
            CapturedValue::ParamList(params) => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| match quoting {
                        JsQuoting::SingleQuoted => format!("'{}'", escape_js_string(&p.name)),
                        JsQuoting::DoubleQuoted => format!("\"{}\"", p.name),
                        JsQuoting::Raw => format!("'{}'", p.name),
                    })
                    .collect();
                format!("[{}]", params_str.join(", "))
            }
            CapturedValue::PatternMatch {
                signal,
                variant,
                bindings,
            } => match quoting {
                JsQuoting::DoubleQuoted => format!("\"{}:{}\"", signal, variant),
                _ => {
                    if bindings.is_empty() {
                        format!(
                            "'${} is {}'",
                            escape_js_string(signal),
                            escape_js_string(variant)
                        )
                    } else {
                        format!(
                            "'${} is {} {{ {} }}'",
                            escape_js_string(signal),
                            escape_js_string(variant),
                            bindings
                                .iter()
                                .map(|b| format!("${}", b))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                }
            },
            CapturedValue::ComponentBody => {
                // FEAT-120: the capture is a bare marker — the factory body is
                // serialized from the World-A scope (`component_body_to_js_from_scope`),
                // not via this generic value path. Emit an empty marker object.
                "{}".to_string()
            }
        }
    }
}

/// Transpile a Spacetime reactive expression into a JS expression string
/// evaluated against a per-element signal scope.
///
/// `$sig` references become `ST.get(__el, 'sig')` reads (the caller binds `__el`).
/// This is a compile-time transpile: no runtime `eval`/`Function` is used.
///
/// Identifier boundaries are respected so `$count` and `$countTotal` stay
/// distinct, and `$` not followed by an identifier char is left untouched.
pub fn transpile_signal_expr(expr: &str) -> String {
    transpile_signal_expr_with(expr, SignalScope::Element)
}

/// Reader scope for transpiled `$signal` references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalScope {
    /// Per-element signal store: `$sig` -> `ST.get(__el, 'sig')`. Used inside
    /// `@template` bodies where each instance owns its signals.
    Element,
    /// Global signal store: `$sig` -> `SpacetimeLocal['sig']`. Used for
    /// file-scope reactive bindings where signals are page-global.
    Global,
    /// LEXICAL scope resolution from a per-node origin: `$sig` ->
    /// `ST.resolve(__node, 'sig')`. Walks `__node` + ancestors (an instance signal
    /// of an enclosing `@template`) before global page state — the runtime analogue of
    /// compile-time scope-matching. The ONE binding form that is correct for BOTH a
    /// file-scope reactive declaration (no element owns the signal -> resolves to
    /// SpacetimeLocal) and a template-body one (resolves to the instance). The emitter
    /// names the origin `__node` and applies the binding per-matched-node so each
    /// instance reads its own scope.
    Scoped,
}

pub fn signal_read(scope: SignalScope, name: &str) -> String {
    match scope {
        SignalScope::Element => format!("ST.get(__el, '{}')", name),
        SignalScope::Global => format!("SpacetimeLocal['{}']", name),
        SignalScope::Scoped => format!("ST.resolve(__node, '{}')", name),
    }
}

/// Transpile `$signal` references against the given [`SignalScope`].
///
/// String literals are passed through verbatim, and identifier boundaries are
/// respected so `$count` and `$countTotal` stay distinct. This is a
/// compile-time transpile: no runtime `eval`/`Function`.
pub fn transpile_signal_expr_with(expr: &str, scope: SignalScope) -> String {
    let bytes = expr.as_bytes();
    let mut out = String::with_capacity(expr.len() + 16);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        // Skip string literals verbatim so `$` inside strings is untouched.
        if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            out.push(c);
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                out.push(ch);
                i += 1;
                if ch == '\\' && i < bytes.len() {
                    out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
                if ch == quote {
                    break;
                }
            }
            continue;
        }
        // `&$name` — an element reference (BUG-333). `&name <sel>;` registers
        // `name` in the __stRefs registry; this read lowers to `ST.ref("name")`
        // (the registry lookup). Without this the `&` prefix dangled and `$name`
        // became a scoped signal read, emitting `&ST.resolve(...)` — invalid JS
        // that failed to parse and killed the whole bundle.
        if c == '&' && bytes.get(i + 1) == Some(&b'$') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            if j > start {
                let name = &expr[start..j];
                out.push_str(&format!("ST.ref({:?})", name));
                i = j;
                continue;
            }
        }
        if c == '$' {
            // Read the identifier following `$`.
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let name = &expr[start..j];
                out.push_str(&signal_read(scope, name));
                i = j;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Collect the distinct `$signal` names referenced in an expression, in order
/// of first appearance, with the `$` stripped. String literals are skipped.
/// Collect FULL dotted signal reads from an expression: `$base.a.b` yields the
/// pair `("base", ["a", "b"])`. Unlike [`collect_signal_deps`] (which truncates
/// to the head name for dependency tracking), this preserves the access PATH so a
/// typed read can be field-checked against its binding type (FUP-150 / PLAN-119
/// W2). A bare `$base` with no `.` yields `("base", [])`.
///
/// Path segments are ASCII identifier runs (`[A-Za-z0-9_]`) separated by `.`. A
/// numeric index or a bracket access ends the path (arrays are validated at the
/// element type, not per-index). String/backtick literals are skipped so a `.`
/// inside a quoted string is never mistaken for a field access.
pub fn collect_dotted_signal_reads(expr: &str) -> Vec<(String, Vec<String>)> {
    let bytes = expr.as_bytes();
    let mut reads: Vec<(String, Vec<String>)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
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
        if c == '$' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let base = expr[start..j].to_string();
                // Walk `.segment` runs immediately following the base.
                let mut path: Vec<String> = Vec::new();
                let mut k = j;
                while k < bytes.len() && bytes[k] as char == '.' {
                    let seg_start = k + 1;
                    let mut seg_end = seg_start;
                    while seg_end < bytes.len() {
                        let ch = bytes[seg_end] as char;
                        if ch.is_ascii_alphanumeric() || ch == '_' {
                            seg_end += 1;
                        } else {
                            break;
                        }
                    }
                    if seg_end == seg_start {
                        break; // `.` not followed by an identifier (e.g. `.5`, method call spacing)
                    }
                    path.push(expr[seg_start..seg_end].to_string());
                    k = seg_end;
                }
                reads.push((base, path));
                i = k;
                continue;
            }
        }
        i += 1;
    }
    reads
}

/// Split a reactive value on its FILTER PIPE (`expr | filterName`), if it has one.
///
/// Returns `(value_expr, Some(filter))`, or `(whole, None)` when there is no
/// filter. Every reactive emitter MUST route through this rather than calling
/// `split_once('|')` — two of them did, and both emitted invalid JS (BUG-262):
///
/// ```text
///   text <- ($comError || $comLoadError);
///
///   value  -> "($comError "          <- unbalanced
///   filter -> "| $comLoadError)"     <- the closing paren, swallowed into a string
/// ```
///
/// which reached the browser as
///
/// ```js
///   try { __v = ((ST.resolve(__node, 'comError')); } catch (e) { return; }
/// ```
///
/// A SyntaxError anywhere in a bundle means the WHOLE bundle never evaluates,
/// so one `||` in one widget killed every page that served it.
///
/// Three rules, each earned by a case that breaks without it:
/// - `||` is the logical OR operator, NEVER a pipe (the reported bug);
/// - a `|` inside parens/brackets/braces belongs to the sub-expression, so the
///   split only happens at DEPTH ZERO (`f(a | b)` breaks identically otherwise);
/// - a `|` inside a string literal is data (`$x | fmt("a|b")`).
///
/// Bitwise-or (`a | b` as arithmetic) is indistinguishable from a filter pipe by
/// syntax alone and stays read as a pipe — the historical behavior, and the
/// filter path no-ops on an unknown name at runtime.
pub fn split_filter_pipe(src: &str) -> (&str, Option<&str>) {
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        match quote {
            Some(q) => {
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                b'\'' | b'"' | b'`' => quote = Some(c),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b'|' if depth == 0 => {
                    // `||` — logical OR. Skip BOTH characters so the second
                    // pipe is not then read as a split point on its own.
                    if bytes.get(i + 1) == Some(&b'|') {
                        i += 2;
                        continue;
                    }
                    // A pipe immediately after another one was handled above;
                    // this is a genuine filter separator.
                    return (src[..i].trim(), Some(src[i + 1..].trim()));
                }
                _ => {}
            },
        }
        i += 1;
    }
    (src, None)
}

pub fn collect_signal_deps(expr: &str) -> Vec<String> {
    let bytes = expr.as_bytes();
    let mut deps: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
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
        if c == '$' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let name = expr[start..j].to_string();
                if !deps.contains(&name) {
                    deps.push(name);
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    deps
}

/// Serialize a list of component state declarations to the factory `states` JS array.
///
/// FEAT-119 W1: the ONE factory field that is not pure serde. `CapturedValue` derives
/// default (externally-tagged) serde — `Bool(false)` → `{"Bool":false}` — but the
/// runtime reads `state.initial` as a BARE value (`ST.set(el, var_name, initial)`,
/// stdlib/primitives/template.st) and an `Expr` must land as UNQUOTED live JS. So the
/// `initial` projection is a genuine emit transform (Bool/Number bare, String quoted,
/// Expr raw JS), not serde. Decoupled from `ComponentBodyDef` so the World-A emit path
/// (FEAT-119 W3) can call it directly with `scope_states(scope)`.
pub fn state_decls_to_js(states: &[ComponentStateDecl]) -> String {
    let states_js: Vec<String> = states
        .iter()
        .map(|s| {
            let initial_js = match &s.initial {
                CapturedValue::Bool(b) => b.to_string(),
                CapturedValue::Number(n) => n.to_string(),
                CapturedValue::String(s) => {
                    serde_json::to_string(s).unwrap_or_else(|_| "null".to_string())
                }
                CapturedValue::Expr(e) => e.clone(),
                _ => "null".to_string(),
            };
            format!(
                "{{ var_name: {}, type_name: {}, initial: {} }}",
                serde_json::to_string(&s.var_name).unwrap_or_default(),
                serde_json::to_string(&s.type_name).unwrap_or_default(),
                initial_js
            )
        })
        .collect();
    format!("[{}]", states_js.join(", "))
}

// FEAT-119 (W5): the capture-path serializers (`component_body_to_js` /
// `component_body_to_js_with_params`) are DELETED. The factory payload is serialized
// from the World-A scope via `component_body_to_js_from_scope` (proven byte-identical
// to the old capture path by the `feat119_oracle_*` snapshot). The `ComponentBody`
// capture no longer carries a payload to serialize — only the validation channel.

/// Serialize the factory `rawBody` payload from its four constituent parts.
///
/// FEAT-119 (W3): the SINGLE serializer for the runtime template payload, called by
/// BOTH the legacy capture path ([`component_body_to_js_with_params`]) and the
/// World-A emit path ([`component_body_to_js_from_scope`]). html/exports/refs are
/// pure serde; `states` uses [`state_decls_to_js`] (the one non-serde field: bare
/// initials + raw-JS `Expr`); `builder` is HTML→reactive-DOM codegen. Taking parts
/// (not a struct) decouples the serializer from `ComponentBodyDef` so the scope can
/// feed it directly.
pub fn component_payload_to_js(
    html: &str,
    states: &[ComponentStateDecl],
    exports: &[ExportDecl],
    refs: &[TemplateRef],
    element_params: &[String],
) -> String {
    let mut parts = Vec::new();

    // html (pure serde)
    parts.push(format!(
        "html: {}",
        serde_json::to_string(html).unwrap_or_else(|_| "\"\"".to_string())
    ));

    // states: the one non-serde field (bare Bool/Number, quoted String, raw-JS Expr).
    parts.push(format!("states: {}", state_decls_to_js(states)));

    // exports (pure serde)
    let exports_json = serde_json::to_string(exports).unwrap_or_else(|_| "[]".to_string());
    parts.push(format!("exports: {}", exports_json));

    // refs (pure serde). Static `&row(…)` is unified via `template-invoke-bare`;
    // dynamic dispatch `&$w(…)` (FEAT-073) and the collection form flow through here.
    let refs_json = serde_json::to_string(refs).unwrap_or_else(|_| "[]".to_string());
    parts.push(format!("refs: {}", refs_json));

    // Reactive DOM builder (FEAT-077): root-scoped HtmlExpr builder so the factory
    // constructs live DOM with reactive holes instead of string-interpolating `html`.
    // FUP-094: lower with SignalScope::Scoped (not Element) so a hole resolves each
    // `$name` LEXICALLY — the template's own params/locals on the instance root
    // FIRST, then ancestors, then global page state (ST.resolve / ST.watchScoped).
    // This makes an OUTER signal usable inside a template/@each-item hole (e.g. a
    // selection signal driving a per-row highlight) while a param/local still
    // resolves to the instance, because ST.resolve finds the nearest owner first.
    // Strictly more permissive than Element (which was instance-ONLY): a name the
    // instance owns still wins; only a previously-unresolvable outer name now
    // resolves instead of yielding null.
    if !html.trim().is_empty() {
        let exprs = crate::emit::html_reactive::component_html_to_exprs(html);
        let _ = element_params; // backtick `&name` holes are recognized lexically (FUP-041)
        let builder = crate::emit::html_reactive::emit_builder_root_scoped(
            &exprs,
            crate::syntax::SignalScope::Scoped,
        );
        parts.push(format!("builder: {}", builder));
    }

    format!("{{ {} }}", parts.join(", "))
}

/// Serialize the factory `rawBody` payload directly from a World-A `ScopeBlock`.
///
/// FEAT-119 (W3): the EMIT-path entry point. The template `Construct` scope carries
/// the entire payload as fields — `html` (W2), `states`/`refs` (back-filled W3),
/// `exports` (built at scope construction). This reads them and delegates to the
/// shared [`component_payload_to_js`]. No `ComponentBodyDef` copy is consulted: the
/// scope is the single producer. Byte-identical to the capture path (oracle-locked).
pub fn component_body_to_js_from_scope(
    scope: &crate::parser::ast::ScopeBlock,
    element_params: &[String],
) -> String {
    component_payload_to_js(
        &scope.html,
        &scope.states,
        &scope.exports,
        &scope.refs,
        element_params,
    )
}
/// Convert a `BindValue` to a JavaScript string using the given quoting mode and variable resolver.
///
/// Unifies the three separate `bind_value_to_js` functions that existed in compile.rs,
/// expand.rs, and resolve.rs.
pub fn bind_value_to_js(
    resolver: &dyn VariableResolver,
    value: &BindValue,
    quoting: JsQuoting,
) -> String {
    match value {
        BindValue::Variable(name) => {
            if let Some(val) = resolver.resolve_variable(name) {
                val.to_js(quoting)
            } else {
                "null".to_string()
            }
        }
        BindValue::String(s) => match quoting {
            JsQuoting::SingleQuoted => format!("'{}'", s.replace('\'', "\\'")),
            JsQuoting::DoubleQuoted => {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
            JsQuoting::Raw => s.replace('\'', "\\'"),
        },
        BindValue::Number(n) => n.to_string(),
        BindValue::Ident(s) => match quoting {
            JsQuoting::SingleQuoted => format!("'{}'", s),
            JsQuoting::DoubleQuoted => format!("\"{}\"", s),
            JsQuoting::Raw => {
                if let Some(ms) = parse_time_to_ms(s) {
                    ms.to_string()
                } else {
                    s.clone()
                }
            }
        },
        BindValue::Array(items) => {
            let js_items: Vec<String> = items
                .iter()
                .map(|s| match quoting {
                    JsQuoting::SingleQuoted => format!("'{}'", s),
                    JsQuoting::DoubleQuoted => format!("\"{}\"", s),
                    JsQuoting::Raw => format!("'{}'", s),
                })
                .collect();
            format!("[{}]", js_items.join(", "))
        }
        BindValue::FunctionCall { name, args } => {
            let js_args: Vec<String> = args
                .iter()
                .map(|arg| bind_value_to_js(resolver, arg, quoting))
                .collect();
            format!("{}({})", name, js_args.join(", "))
        }
        BindValue::ElementRef(name) => {
            format!("document.querySelector('[data-st-ref=\"{}\"]')", name)
        }
    }
}

/// Try to resolve a BindValue to CSS declaration text.
/// Returns Some(css_text) if the value resolves to StyleProperties,
/// which need to be emitted as `key: value;` declarations in CSS emit blocks.
/// Returns None for other value types (they don't need CSS-specific handling).
pub fn resolve_bind_value_css_override(
    resolver: &dyn VariableResolver,
    value: &BindValue,
) -> Option<String> {
    match value {
        BindValue::Variable(name) => {
            if let Some(val) = resolver.resolve_variable(name) {
                match val {
                    CapturedValue::StyleProperties(props) => {
                        let decls: Vec<String> = props
                            .iter()
                            .map(|(k, v)| format!("{}: {};", k, v))
                            .collect();
                        Some(decls.join("\n    "))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

// =============================================================================
// Template Provenance Injection (ITEM-107-003)
// =============================================================================

/// CSS selector matching text-bearing HTML elements for provenance annotation.
const TEXT_BEARING_SELECTOR: &str = "h1, h2, h3, h4, h5, h6, p, span, li, td, th, a, label, button, figcaption, blockquote, cite, dt, dd, summary";

/// Inject `data-st-id` and `data-st-origin` attributes into text-bearing elements
/// of a template body's HTML string using lol_html.
///
/// This is a compile-time pass that annotates template definitions so that all
/// instances share the same provenance. Only runs in debug builds.
///
/// # Arguments
/// - `html`: Raw HTML content from `ComponentBodyDef.html`
/// - `source_file`: Filename of the .st source (e.g., "card.st")
/// - `scope_selector`: CSS scope selector (e.g., ".card"), empty for top-level
/// - `macro_name`: Macro name (e.g., "template")
/// - `ast_offset`: Byte offset of the template in the .st file
///
/// # Returns
/// Modified HTML with provenance attributes, or original HTML on error.
#[cfg(debug_assertions)]
pub fn inject_template_provenance(
    html: &str,
    source_file: &str,
    scope_selector: &str,
    macro_name: &str,
    ast_offset: usize,
) -> String {
    use lol_html::{HtmlRewriter, Settings, element};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    if html.is_empty() {
        return html.to_string();
    }

    let mut output = Vec::new();
    let mut occurrence_counts: HashMap<String, usize> = HashMap::new();

    let sf = source_file.to_string();
    let ss = scope_selector.to_string();
    let mn = macro_name.to_string();

    let result = {
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!(TEXT_BEARING_SELECTOR, |el| {
                    // Skip elements with existing annotations
                    if el.get_attribute("data-t").is_some()
                        || el.get_attribute("data-st-bind").is_some()
                        || el.get_attribute("data-st-id").is_some()
                    {
                        return Ok(());
                    }

                    let tag = el.tag_name().to_lowercase();

                    // Per-tag-name occurrence index within this template body
                    let idx = {
                        let count = occurrence_counts.entry(tag.clone()).or_insert(0);
                        let i = *count;
                        *count += 1;
                        i
                    };

                    // data-st-id: hash of (source_file, ast_offset, tag_name, occurrence_index)
                    let hash_input = format!("{}:{}:{}:{}", sf, ast_offset, tag, idx);
                    let mut hasher = DefaultHasher::new();
                    hash_input.hash(&mut hasher);
                    let hash_hex = format!("{:016x}", hasher.finish());
                    let st_id = format!("st-{}", &hash_hex[..8]);

                    // data-st-origin: "{source_file}::{scope_selector} §{macro_name} {tag_name}"
                    let origin = format!("{}::{} §{} {}", sf, ss, mn, tag);

                    el.set_attribute("data-st-id", &st_id).ok();
                    el.set_attribute("data-st-origin", &origin).ok();

                    Ok(())
                })],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter.write(html.as_bytes()).and_then(|_| rewriter.end())
    };

    match result {
        Ok(()) => String::from_utf8(output).unwrap_or_else(|_| html.to_string()),
        Err(_) => html.to_string(),
    }
}

/// No-op provenance injection for release builds.
#[cfg(not(debug_assertions))]
pub fn inject_template_provenance(
    html: &str,
    _source_file: &str,
    _scope_selector: &str,
    _macro_name: &str,
    _ast_offset: usize,
) -> String {
    html.to_string()
}

/// Compute a workspace-relative path for data-st-origin emission.
///
/// Prefers `strip_prefix(workspace_root)` — strips the workspace prefix from an
/// absolute source-file path so the origin attribute contains a stable relative path
/// (e.g. `modules/card.st`) that the dev-sync `EditAst` handler can resolve.
///
/// Falls back to `extract_filename` when `strip_prefix` fails (path outside workspace,
/// or workspace_root is relative and source_file is absolute).
fn workspace_relative_path(source_file: &str, workspace_root: &Path) -> String {
    let file_path = Path::new(source_file);

    // Try direct prefix strip first
    if let Ok(remaining) = file_path.strip_prefix(workspace_root)
        && let Some(rel) = remaining.to_str()
        && !rel.is_empty()
    {
        return rel.to_string();
    }

    // Canonicalize tolerance: when workspace_root is relative but source_file is
    // absolute (common in tests), canonicalize both and re-try.
    if let (Ok(canon_file), Ok(canon_root)) =
        (file_path.canonicalize(), workspace_root.canonicalize())
        && let Ok(remaining) = canon_file.strip_prefix(&canon_root)
        && let Some(rel) = remaining.to_str()
        && !rel.is_empty()
    {
        return rel.to_string();
    }

    // Outside the workspace — fall back to bare filename
    extract_filename(source_file).to_string()
}

/// Extract just the filename from a path string (e.g., "/home/user/card.st" → "card.st").
fn extract_filename(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

/// Post-processing pass: inject provenance attributes into all template body HTML.
///
/// Iterates FormMatches for template/template-inline macros and annotates their
/// ComponentBodyDef HTML with `data-st-id` and `data-st-origin` attributes.
/// Only active in debug builds; compiles to a no-op in release.
///
/// # Arguments
/// - `matches`: Mutable slice of FormMatches to process
/// - `default_source_file`: Fallback source file path when `fm.source_file` is None
/// - `workspace_root`: Project workspace root for computing relative paths
#[cfg(debug_assertions)]
pub fn inject_provenance_into_matches(
    matches: &[FormMatch],
    scopes: &mut [crate::parser::ast::ScopeBlock],
    default_source_file: &str,
    workspace_root: &Path,
) {
    // FEAT-119 (W5): template body html lives on the World-A `@template:<name>` scope,
    // not the capture. The provenance HASH must use the FormMatch's metadata
    // (selector + span.start) so it matches what `serve` reproduces at edit time
    // (sync/handlers.rs). So pair each template match with its scope by name and
    // inject into the SCOPE html.
    for fm in matches.iter() {
        if fm.macro_name != "template" && fm.macro_name != "template-inline" {
            continue;
        }
        let Some(name) = fm.get_ident("name") else {
            continue;
        };
        let want = format!("@template:{}", name);
        let source_file_raw = fm.source_file.as_deref().unwrap_or(default_source_file);
        let source_file = workspace_relative_path(source_file_raw, workspace_root);
        let scope_selector = fm.selector.as_deref().unwrap_or("");
        let macro_name = fm.macro_name.clone();
        let ast_offset = fm.span.start;

        for scope in scopes.iter_mut() {
            if scope.selector == want && !scope.html.is_empty() {
                scope.html = inject_template_provenance(
                    &scope.html,
                    &source_file,
                    scope_selector,
                    &macro_name,
                    ast_offset,
                );
            }
        }
    }
}

/// No-op provenance injection for release builds.
#[cfg(not(debug_assertions))]
pub fn inject_provenance_into_matches(
    _matches: &[FormMatch],
    _scopes: &mut [crate::parser::ast::ScopeBlock],
    _default_source_file: &str,
    _workspace_root: &Path,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_dotted_signal_reads_keeps_full_path() {
        // PLAN-119 W2: the extractor preserves the access path (unlike
        // collect_signal_deps, which truncates to the head).
        let reads = collect_dotted_signal_reads("text: $form.errors.email");
        assert_eq!(
            reads,
            vec![(
                "form".to_string(),
                vec!["errors".to_string(), "email".to_string()]
            )]
        );
    }

    #[test]
    fn transpile_element_ref_reads_st_ref() {
        // BUG-333: `&$name` is an element reference, not a scoped signal read.
        // It must lower to `ST.ref("name")` (the __stRefs registry lookup),
        // never `&ST.resolve(...)` (a dangling `&` that fails to parse).
        let out = transpile_signal_expr_with("&$b", SignalScope::Scoped);
        assert_eq!(out, "ST.ref(\"b\")", "element ref lowers to ST.ref: {out}");
        // A plain `$sig` still reads via the scope — untouched.
        let out2 = transpile_signal_expr_with("$count", SignalScope::Scoped);
        assert_eq!(out2, "ST.resolve(__node, 'count')", "signal read unchanged: {out2}");
    }

    #[test]
    fn collect_dotted_signal_reads_bare_and_multi() {
        let reads = collect_dotted_signal_reads("$a + $b.x");
        assert_eq!(
            reads,
            vec![
                ("a".to_string(), vec![]),
                ("b".to_string(), vec!["x".to_string()]),
            ]
        );
    }

    #[test]
    fn collect_dotted_signal_reads_skips_string_literals() {
        // A `.` inside a quoted string must not be read as a field access.
        let reads = collect_dotted_signal_reads("$form.email + \"a.b.c\"");
        assert_eq!(reads, vec![("form".to_string(), vec!["email".to_string()])]);
    }

    #[test]
    fn normalize_invocation_args_splits_named_and_positional() {
        // BUG-131: named args yield Some(name) + bare value; positional yield None.
        let (vals, names) = normalize_invocation_args(&[
            "title: \"Espresso\"".to_string(),
            "price: \"$3.50\"".to_string(),
        ]);
        assert_eq!(
            vals,
            vec!["\"Espresso\"".to_string(), "\"$3.50\"".to_string()]
        );
        assert_eq!(
            names,
            vec![Some("title".to_string()), Some("price".to_string())]
        );

        // Positional: value verbatim, name None.
        let (vals, names) = normalize_invocation_args(&["\"A\"".to_string(), "$item".to_string()]);
        assert_eq!(vals, vec!["\"A\"".to_string(), "$item".to_string()]);
        assert_eq!(names, vec![None, None]);

        // A `:` INSIDE a quoted value is not a separator (URL-safe).
        let (vals, names) = normalize_invocation_args(&["url: \"http://x\"".to_string()]);
        assert_eq!(vals, vec!["\"http://x\"".to_string()]);
        assert_eq!(names, vec![Some("url".to_string())]);

        // A bare positional containing only a quoted `:` stays positional.
        let (vals, names) = normalize_invocation_args(&["\"a: b\"".to_string()]);
        assert_eq!(vals, vec!["\"a: b\"".to_string()]);
        assert_eq!(names, vec![None]);

        // Mixed: positional then named.
        let (vals, names) =
            normalize_invocation_args(&["$x".to_string(), "label: \"Y\"".to_string()]);
        assert_eq!(vals, vec!["$x".to_string(), "\"Y\"".to_string()]);
        assert_eq!(names, vec![None, Some("label".to_string())]);
    }

    #[test]
    fn all_positional_args_omit_arg_names_from_bundle() {
        // serde-skip guard: a purely positional ref serializes WITHOUT an arg_names
        // key (existing snapshots unaffected); a named ref includes it.
        let positional = TemplateRef {
            ref_name: None,
            template_name: "card".to_string(),
            args: vec!["\"A\"".to_string()],
            is_collection: false,
            arg_names: vec![None],
            span: Default::default(),
        };
        let json = serde_json::to_string(&positional).unwrap();
        assert!(
            !json.contains("arg_names"),
            "positional omits arg_names: {json}"
        );

        let named = TemplateRef {
            ref_name: None,
            template_name: "card".to_string(),
            args: vec!["\"A\"".to_string()],
            is_collection: false,
            arg_names: vec![Some("title".to_string())],
            span: Default::default(),
        };
        let json = serde_json::to_string(&named).unwrap();
        assert!(
            json.contains("\"arg_names\""),
            "named includes arg_names: {json}"
        );
        assert!(json.contains("title"), "arg_names carries the name: {json}");
    }

    // === FEAT-119 ORACLE: freeze the exact rawBody JS the factory emits ===
    // Byte-exact contract for the WORLD-A emit path (`component_body_to_js_from_scope`).
    // The snapshot was frozen at W1 against the capture-path serializer and PROVEN
    // byte-identical when the source moved to the scope (W3) — this test now drives the
    // surviving scope path. Covers all four `initial` CapturedValue kinds (the one field
    // that is NOT pure serde: Bool/Number bare, String quoted, Expr raw JS), plus
    // exports, refs, and a hole-bearing html builder.
    #[test]
    fn feat119_oracle_rawbody_js_all_field_kinds() {
        use crate::parser::ast::{ScopeBlock, ScopeKind};
        use crate::syntax::{ComponentStateDecl, ExportDecl, TemplateRef};
        let scope = ScopeBlock {
            kind: ScopeKind::Construct("template".to_string()),
            selector: "@template:counter".to_string(),
            html: "<div class=\"counter\"><span>`$label`</span><b slot=\"v\"></b></div>"
                .to_string(),
            states: vec![
                ComponentStateDecl {
                    var_name: "open".to_string(),
                    type_name: "bool".to_string(),
                    initial: CapturedValue::Bool(false),
                },
                ComponentStateDecl {
                    var_name: "count".to_string(),
                    type_name: "number".to_string(),
                    initial: CapturedValue::Number(3.0),
                },
                ComponentStateDecl {
                    var_name: "title".to_string(),
                    type_name: "string".to_string(),
                    initial: CapturedValue::String("hi".to_string()),
                },
                ComponentStateDecl {
                    var_name: "sum".to_string(),
                    type_name: "number".to_string(),
                    initial: CapturedValue::Expr("1 + 2".to_string()),
                },
            ],
            exports: vec![
                ExportDecl {
                    var_name: "count".to_string(),
                    mutable: true,
                },
                ExportDecl {
                    var_name: "open".to_string(),
                    mutable: false,
                },
            ],
            refs: vec![TemplateRef {
                ref_name: Some("inner".to_string()),
                template_name: "child".to_string(),
                args: vec!["\"x\"".to_string()],
                is_collection: false,
                arg_names: vec![],
                span: Default::default(),
            }],
            ..Default::default()
        };
        let js = component_body_to_js_from_scope(&scope, &[]);
        insta::assert_snapshot!("feat119_rawbody_all_kinds", js);
    }

    #[test]
    fn test_as_string_literal_unifies_string_and_quoted_expr() {
        // Bare String capture (legacy `@data{}` surface).
        assert_eq!(
            CapturedValue::String("/data/x.json".to_string()).as_string_literal(),
            Some("/data/x.json")
        );
        // Quoted Expr capture (`@data fetch ... : "/url"` via `$src:expr`).
        assert_eq!(
            CapturedValue::Expr("\"/data/x.json\"".to_string()).as_string_literal(),
            Some("/data/x.json")
        );
        // Single-quoted Expr.
        assert_eq!(
            CapturedValue::Expr("'/data/y.json'".to_string()).as_string_literal(),
            Some("/data/y.json")
        );
        // Trailing tokens after the literal (e.g. doc comment folded into the
        // expr by the matcher) are ignored — only the leading literal counts.
        assert_eq!(
            CapturedValue::Expr("\"/data/z.json\"\n/// trailing comment".to_string())
                .as_string_literal(),
            Some("/data/z.json")
        );
        // Non-literal captures yield None.
        assert_eq!(
            CapturedValue::Ident("foo".to_string()).as_string_literal(),
            None
        );
        assert_eq!(
            CapturedValue::Expr("foo + bar".to_string()).as_string_literal(),
            None
        );
    }

    #[test]
    fn test_form_match_builder() {
        let fm = FormMatch::new("local-state")
            .capture("name", CapturedValue::Ident("count".to_string()))
            .capture("type", CapturedValue::Ident("number".to_string()))
            .capture("value", CapturedValue::Number(0.0))
            .in_scope(".counter");

        assert_eq!(fm.macro_name, "local-state");
        assert_eq!(fm.get_ident("name"), Some("count"));
        assert_eq!(fm.get_ident("type"), Some("number"));
        assert_eq!(fm.get_number("value"), Some(0.0));
        assert_eq!(fm.selector, Some(".counter".to_string()));
    }

    #[test]
    fn test_captured_value_types() {
        let fm = FormMatch::new("test")
            .capture("ident", CapturedValue::Ident("foo".to_string()))
            .capture("string", CapturedValue::String("bar".to_string()))
            .capture("number", CapturedValue::Number(42.0))
            .capture("bool", CapturedValue::Bool(true))
            .capture("time", CapturedValue::Time(500))
            .capture("selector", CapturedValue::Selector(".class".to_string()))
            .capture("binding", CapturedValue::Binding("$data".to_string()))
            .capture("expr", CapturedValue::Expr("1 + 2".to_string()));

        assert_eq!(fm.get_ident("ident"), Some("foo"));
        assert_eq!(fm.get_string("string"), Some("bar"));
        assert_eq!(fm.get_number("number"), Some(42.0));
        assert_eq!(fm.get_bool("bool"), Some(true));
        assert_eq!(fm.get_time("time"), Some(500));
        assert_eq!(fm.get_selector("selector"), Some(".class"));
        assert_eq!(fm.get_binding("binding"), Some("$data"));
        assert_eq!(fm.get_expr("expr"), Some("1 + 2"));
    }

    #[test]
    fn test_length_value() {
        let px = LengthValue::px(100.0);
        assert_eq!(px.value, 100.0);
        assert_eq!(px.unit, "px");

        let pct = LengthValue::percent(50.0);
        assert_eq!(pct.value, 50.0);
        assert_eq!(pct.unit, "%");

        let em = LengthValue::em(2.0);
        assert_eq!(em.value, 2.0);
        assert_eq!(em.unit, "em");
    }

    #[test]
    fn test_nested_block() {
        let inner = FormMatch::new("bind")
            .capture("text", CapturedValue::Binding("$item.name".to_string()));

        let outer = FormMatch::new("each")
            .capture("items", CapturedValue::Binding("$products".to_string()))
            .capture("content", CapturedValue::Block(vec![inner]));

        let block = outer.get_block("content").unwrap();
        assert_eq!(block.len(), 1);
        assert_eq!(block[0].macro_name, "bind");
    }

    #[test]
    fn test_property_def_creation() {
        let prop = PropertyDef {
            name: "price".to_string(),
            type_ref: "number".to_string(),
            optional: false,
        };
        assert_eq!(prop.name, "price");
        assert!(!prop.optional);
    }

    #[test]
    fn test_property_def_optional() {
        let prop = PropertyDef {
            name: "discount".to_string(),
            type_ref: "number".to_string(),
            optional: true,
        };
        assert!(prop.optional);
    }

    #[test]
    fn test_param_def_with_default() {
        let param = ParamDef {
            name: "count".to_string(),
            type_ref: "number".to_string(),
            default: Some("0".to_string()),
        };
        assert_eq!(param.default, Some("0".to_string()));
    }

    #[test]
    fn test_param_def_without_default() {
        let param = ParamDef {
            name: "name".to_string(),
            type_ref: "string".to_string(),
            default: None,
        };
        assert_eq!(param.default, None);
    }

    #[test]
    fn test_keyframe_def_multi_step() {
        let kf = KeyframeDef {
            selector: None,
            property: "scale".to_string(),
            values: vec![
                "0.9".to_string(),
                "1".to_string(),
                "1.02".to_string(),
                "1".to_string(),
            ],
        };
        assert_eq!(kf.values.len(), 4);
    }

    #[test]
    fn test_is_truthy_expr() {
        // Falsy expressions
        assert!(!CapturedValue::Expr("null".to_string()).is_truthy());
        assert!(!CapturedValue::Expr("undefined".to_string()).is_truthy());
        assert!(!CapturedValue::Expr("false".to_string()).is_truthy());
        assert!(!CapturedValue::Expr("".to_string()).is_truthy());
        // Truthy expressions
        assert!(CapturedValue::Expr("someValue".to_string()).is_truthy());
        assert!(CapturedValue::Expr("$count".to_string()).is_truthy());
        assert!(CapturedValue::Expr("1 + 2".to_string()).is_truthy());
    }

    #[test]
    fn test_is_truthy_ident() {
        // Falsy idents
        assert!(!CapturedValue::Ident("null".to_string()).is_truthy());
        assert!(!CapturedValue::Ident("undefined".to_string()).is_truthy());
        assert!(!CapturedValue::Ident("none".to_string()).is_truthy());
        assert!(!CapturedValue::Ident("false".to_string()).is_truthy());
        assert!(!CapturedValue::Ident("".to_string()).is_truthy());
        // Truthy idents
        assert!(CapturedValue::Ident("validName".to_string()).is_truthy());
        assert!(CapturedValue::Ident("true".to_string()).is_truthy());
    }

    #[test]
    fn test_is_truthy_string() {
        assert!(!CapturedValue::String("".to_string()).is_truthy());
        assert!(CapturedValue::String("hello".to_string()).is_truthy());
        assert!(CapturedValue::String("false".to_string()).is_truthy()); // string "false" is truthy
    }

    #[test]
    fn test_is_truthy_number() {
        assert!(!CapturedValue::Number(0.0).is_truthy());
        assert!(CapturedValue::Number(1.0).is_truthy());
        assert!(CapturedValue::Number(-1.0).is_truthy());
    }

    #[test]
    fn test_is_truthy_bool() {
        assert!(CapturedValue::Bool(true).is_truthy());
        assert!(!CapturedValue::Bool(false).is_truthy());
    }

    #[test]
    fn test_is_truthy_time_and_selector() {
        assert!(!CapturedValue::Time(0).is_truthy());
        assert!(CapturedValue::Time(500).is_truthy());
        assert!(!CapturedValue::Selector("".to_string()).is_truthy());
        assert!(CapturedValue::Selector(".class".to_string()).is_truthy());
    }

    #[test]
    fn test_captured_value_properties() {
        let props = vec![PropertyDef {
            name: "a".to_string(),
            type_ref: "string".to_string(),
            optional: false,
        }];
        let cv = CapturedValue::Properties(props);
        match cv {
            CapturedValue::Properties(p) => assert_eq!(p.len(), 1),
            _ => panic!("Expected Properties"),
        }
    }

    // === CapturedValue::Color tests ===

    #[test]
    fn test_color_value_creation() {
        let color = CapturedValue::Color("#E85D4A".to_string());
        assert!(matches!(color, CapturedValue::Color(ref s) if s == "#E85D4A"));
    }

    #[test]
    fn test_color_to_string_value() {
        // Hex colors
        assert_eq!(
            CapturedValue::Color("#fff".to_string()).to_string_value(),
            "#fff"
        );
        assert_eq!(
            CapturedValue::Color("#E85D4A".to_string()).to_string_value(),
            "#E85D4A"
        );
        assert_eq!(
            CapturedValue::Color("#00ff00cc".to_string()).to_string_value(),
            "#00ff00cc"
        );
        // Named colors
        assert_eq!(
            CapturedValue::Color("cyan".to_string()).to_string_value(),
            "cyan"
        );
        assert_eq!(
            CapturedValue::Color("rebeccapurple".to_string()).to_string_value(),
            "rebeccapurple"
        );
        assert_eq!(
            CapturedValue::Color("transparent".to_string()).to_string_value(),
            "transparent"
        );
        // Function colors
        assert_eq!(
            CapturedValue::Color("rgb(232, 93, 74)".to_string()).to_string_value(),
            "rgb(232, 93, 74)"
        );
        assert_eq!(
            CapturedValue::Color("hsl(120, 100%, 50%)".to_string()).to_string_value(),
            "hsl(120, 100%, 50%)"
        );
        assert_eq!(
            CapturedValue::Color("oklch(0.7 0.15 180)".to_string()).to_string_value(),
            "oklch(0.7 0.15 180)"
        );
        assert_eq!(
            CapturedValue::Color("color-mix(in srgb, red, blue)".to_string()).to_string_value(),
            "color-mix(in srgb, red, blue)"
        );
    }

    #[test]
    fn test_color_is_truthy() {
        assert!(CapturedValue::Color("#E85D4A".to_string()).is_truthy());
        assert!(CapturedValue::Color("cyan".to_string()).is_truthy());
        assert!(CapturedValue::Color("rgb(0, 0, 0)".to_string()).is_truthy());
        assert!(!CapturedValue::Color("".to_string()).is_truthy());
    }

    #[test]
    fn test_color_to_js_single_quoted() {
        let val = CapturedValue::Color("#E85D4A".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'#E85D4A'");
    }

    #[test]
    fn test_color_to_js_double_quoted() {
        let val = CapturedValue::Color("#E85D4A".to_string());
        assert_eq!(val.to_js(JsQuoting::DoubleQuoted), "\"#E85D4A\"");
    }

    #[test]
    fn test_color_to_js_raw() {
        let val = CapturedValue::Color("#E85D4A".to_string());
        // Raw mode should still escape single quotes in the value
        assert_eq!(val.to_js(JsQuoting::Raw), "#E85D4A");
    }

    #[test]
    fn test_color_to_js_named_color() {
        let val = CapturedValue::Color("rebeccapurple".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'rebeccapurple'");
        assert_eq!(val.to_js(JsQuoting::DoubleQuoted), "\"rebeccapurple\"");
    }

    #[test]
    fn test_color_to_js_rgb_function() {
        let val = CapturedValue::Color("rgb(232, 93, 74)".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'rgb(232, 93, 74)'");
        assert_eq!(val.to_js(JsQuoting::DoubleQuoted), "\"rgb(232, 93, 74)\"");
    }

    #[test]
    fn test_color_to_js_hsl_function() {
        let val = CapturedValue::Color("hsl(120, 100%, 50%)".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'hsl(120, 100%, 50%)'");
    }

    #[test]
    fn test_color_to_js_oklch_function() {
        let val = CapturedValue::Color("oklch(0.7 0.15 180)".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'oklch(0.7 0.15 180)'");
    }

    #[test]
    fn test_color_to_js_color_mix() {
        let val = CapturedValue::Color("color-mix(in srgb, red, blue)".to_string());
        assert_eq!(
            val.to_js(JsQuoting::SingleQuoted),
            "'color-mix(in srgb, red, blue)'"
        );
    }

    #[test]
    fn test_color_to_js_rgba() {
        let val = CapturedValue::Color("rgba(0, 0, 0, 0.5)".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'rgba(0, 0, 0, 0.5)'");
    }

    #[test]
    fn test_color_to_js_hsla() {
        let val = CapturedValue::Color("hsla(120 100% 50% / 0.5)".to_string());
        assert_eq!(
            val.to_js(JsQuoting::SingleQuoted),
            "'hsla(120 100% 50% / 0.5)'"
        );
    }

    #[test]
    fn test_color_to_js_oklab() {
        let val = CapturedValue::Color("oklab(50% 40 -20)".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'oklab(50% 40 -20)'");
    }

    #[test]
    fn test_color_to_js_hwb() {
        let val = CapturedValue::Color("hwb(120 20% 30%)".to_string());
        assert_eq!(val.to_js(JsQuoting::SingleQuoted), "'hwb(120 20% 30%)'");
    }

    #[test]
    fn test_color_to_js_display_p3() {
        let val = CapturedValue::Color("color(display-p3 1 0.5 0)".to_string());
        assert_eq!(
            val.to_js(JsQuoting::SingleQuoted),
            "'color(display-p3 1 0.5 0)'"
        );
    }

    #[test]
    fn test_color_to_js_hex_with_alpha() {
        let val = CapturedValue::Color("#RRGGBBAA".to_string());
        assert_eq!(val.to_js(JsQuoting::DoubleQuoted), "\"#RRGGBBAA\"");
    }

    #[test]
    fn test_color_to_js_system_colors() {
        for name in ["Canvas", "CanvasText", "LinkText"] {
            let val = CapturedValue::Color(name.to_string());
            assert_eq!(val.to_js(JsQuoting::SingleQuoted), format!("'{}'", name));
        }
    }

    // =========================================================================
    // Template Provenance Injection tests (ITEM-107-003)
    // =========================================================================

    #[test]
    fn test_inject_provenance_basic() {
        let html = "<h3>Title</h3><p>Body text</p>";
        let result = inject_template_provenance(html, "card.st", ".card", "template", 100);

        // Both elements should get data-st-id and data-st-origin
        assert!(result.contains("data-st-id=\""), "should inject data-st-id");
        assert!(
            result.contains("data-st-origin=\""),
            "should inject data-st-origin"
        );

        // Origin format: "card.st::.card §template {tag}"
        assert!(result.contains("card.st::.card §template h3"), "h3 origin");
        assert!(result.contains("card.st::.card §template p"), "p origin");

        // data-st-id should start with "st-" and have 8 hex chars
        let id_start = result.find("data-st-id=\"").unwrap() + 12;
        let id_end = result[id_start..].find('"').unwrap() + id_start;
        let st_id = &result[id_start..id_end];
        assert!(st_id.starts_with("st-"), "id starts with st-: {}", st_id);
        assert_eq!(st_id.len(), 11, "st- + 8 hex = 11 chars: {}", st_id);
    }

    #[test]
    fn test_inject_provenance_skips_annotated() {
        // Elements with data-t should be skipped
        let html = r#"<h3 data-t="title">Translated</h3><p>Normal</p>"#;
        let result = inject_template_provenance(html, "page.st", ".hero", "template", 50);

        // h3 should NOT get provenance (has data-t)
        assert!(
            !result.contains("§template h3"),
            "h3 with data-t should be skipped"
        );
        // p should get provenance
        assert!(result.contains("§template p"), "p should get provenance");
    }

    #[test]
    fn test_inject_provenance_skips_st_bind() {
        let html = r#"<span data-st-bind="count">0</span><p>Text</p>"#;
        let result = inject_template_provenance(html, "app.st", ".counter", "template", 200);

        assert!(
            !result.contains("§template span"),
            "span with data-st-bind skipped"
        );
        assert!(result.contains("§template p"), "p should get provenance");
    }

    #[test]
    fn test_inject_provenance_skips_existing_st_id() {
        let html = r#"<p data-st-id="st-existing">Already annotated</p><p>New</p>"#;
        let result = inject_template_provenance(html, "test.st", ".box", "template", 10);

        // First p already has data-st-id — should not be modified
        assert!(result.contains("st-existing"), "existing id preserved");
        // Second p should get provenance
        assert!(result.contains("§template p"), "second p gets provenance");
    }

    #[test]
    fn test_inject_provenance_empty_html() {
        let result = inject_template_provenance("", "f.st", ".x", "template", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_inject_provenance_no_text_bearing_tags() {
        let html = "<div><section><img src=\"x.png\"></section></div>";
        let result = inject_template_provenance(html, "f.st", ".x", "template", 0);
        assert!(
            !result.contains("data-st-id"),
            "non-text elements untouched"
        );
    }

    #[test]
    fn test_inject_provenance_occurrence_index() {
        // Two <p> tags should get different data-st-id values
        let html = "<p>First</p><p>Second</p>";
        let result = inject_template_provenance(html, "test.st", ".box", "template", 42);

        // Find both st-ids
        let ids: Vec<&str> = result
            .match_indices("data-st-id=\"st-")
            .map(|(pos, _)| {
                let start = pos + 13; // len("data-st-id=\"")
                let end = start + 11; // len("st-XXXXXXXX")
                &result[start..end]
            })
            .collect();
        assert_eq!(ids.len(), 2, "two elements should have st-ids");
        assert_ne!(ids[0], ids[1], "different occurrence_index → different ids");
    }

    #[test]
    fn test_inject_provenance_top_level_scope() {
        // Empty scope_selector for top-level templates
        let html = "<h1>Hello</h1>";
        let result = inject_template_provenance(html, "main.st", "", "template", 0);
        assert!(
            result.contains("main.st:: §template h1"),
            "empty selector uses :: format"
        );
    }

    // FEAT-119: provenance is injected into the World-A scope html (the emit source),
    // not the capture. This helper builds a template match + its `@template:<name>`
    // scope carrying `html`, runs injection, and returns the resulting scope html.
    #[cfg(debug_assertions)]
    fn provenance_scope_html(
        macro_name: &str,
        selector: Option<&str>,
        span: SourceSpan,
        source_file: Option<&str>,
        html: &str,
        default_source_file: &str,
        workspace_root: &Path,
    ) -> String {
        let mut c = HashMap::new();
        c.insert("name".to_string(), CapturedValue::Ident("t".to_string()));
        c.insert("body".to_string(), CapturedValue::ComponentBody);
        let matches = vec![FormMatch {
            macro_name: macro_name.to_string(),
            matched_macro: None,
            captures: c,
            capture_spans: HashMap::new(),
            selector: selector.map(|s| s.to_string()),
            span,
            source_file: source_file.map(|s| s.to_string()),
            namespace_qualifier: Vec::new(),
            doc: None,
        }];
        let mut scopes = vec![crate::parser::ast::ScopeBlock {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector: "@template:t".to_string(),
            html: html.to_string(),
            ..Default::default()
        }];
        inject_provenance_into_matches(&matches, &mut scopes, default_source_file, workspace_root);
        scopes.into_iter().next().unwrap().html
    }

    #[test]
    fn test_inject_provenance_into_matches_pass() {
        let html = provenance_scope_html(
            "template",
            Some(".card"),
            SourceSpan::new(100, 200),
            Some("/home/user/project/card.st"),
            "<h2>Title</h2>",
            "fallback.st",
            Path::new("/home/user/project"),
        );
        assert!(html.contains("data-st-id"), "html should have data-st-id");
        assert!(
            html.contains("card.st::.card §template h2"),
            "origin uses filename, not full path, got: {}",
            html
        );
    }

    #[test]
    fn test_inject_provenance_skips_non_template() {
        let matches = vec![FormMatch {
            macro_name: "data-fetch".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }];
        // Should be a no-op (no template macro) — a non-template scope's html is untouched.
        let mut scopes = vec![crate::parser::ast::ScopeBlock {
            kind: crate::parser::ast::ScopeKind::Construct("template".to_string()),
            selector: "@template:t".to_string(),
            html: "<h1>x</h1>".to_string(),
            ..Default::default()
        }];
        inject_provenance_into_matches(&matches, &mut scopes, "test.st", Path::new("."));
        assert!(!scopes[0].html.contains("data-st-id"));
    }
    /// BUG-024: Subfolder .st files should emit workspace-relative paths in data-st-origin.
    /// Currently broken — extract_filename strips the directory.
    #[test]
    fn test_inject_provenance_subfolder_relative_path_preserved() {
        let html = provenance_scope_html(
            "template",
            Some(".card"),
            SourceSpan::new(100, 200),
            // Absolute path as set by resolve_imports for a subfolder file
            Some("/home/user/project/modules/card.st"),
            "<h2>Card</h2>",
            "fallback.st",
            Path::new("/home/user/project"),
        );
        assert!(
            html.contains("data-st-origin"),
            "html should have data-st-origin"
        );
        // workspace_relative_path should resolve to modules/card.st
        assert!(
            html.contains("modules/card.st::.card §template h2"),
            "origin should contain subfolder path modules/card.st, got: {}",
            html
        );
    }

    /// BUG-024: When source_file is outside the workspace, fall back to bare filename.
    /// This is the safety net for paths that cannot be made workspace-relative.
    #[test]
    fn test_inject_provenance_outside_workspace_falls_back_to_filename() {
        let html = provenance_scope_html(
            "template",
            Some(".widget"),
            SourceSpan::new(0, 50),
            // Path outside any conceivable workspace — must fall back to basename
            Some("/usr/local/share/spacetime-stdlib/widget.st"),
            "<p>Text</p>",
            "fallback.st",
            Path::new("/workspace"),
        );
        assert!(
            html.contains("widget.st::.widget §template p"),
            "outside-workspace paths should emit bare filename, got: {}",
            html
        );
        // Must NOT contain the original absolute path
        assert!(
            !html.contains("/usr/local"),
            "origin must not leak absolute filesystem paths, got: {}",
            html
        );
    }

    /// BUG-024: When source_file is None, use default_source_file basename.
    #[test]
    fn test_inject_provenance_null_source_file_uses_default() {
        // default_source_file is an absolute path; its basename should be used
        let html = provenance_scope_html(
            "template-inline",
            None,
            SourceSpan::new(500, 550),
            // No source_file — should use default_source_file
            None,
            "<span>Label</span>",
            "/home/user/project/index.st",
            Path::new("/home/user/project"),
        );
        assert!(
            html.contains("index.st:: §template-inline span"),
            "null source_file should fall back to default_source_file basename, got: {}",
            html
        );
    }
}

#[cfg(test)]
mod split_filter_pipe_tests {
    use super::split_filter_pipe;

    /// THE REPORTED BUG (BUG-262). `||` is logical OR, never a filter pipe.
    ///
    /// A naive `split_once('|')` split between the two pipe characters, leaving
    /// the value unbalanced (`($comError `) and the filter carrying the real
    /// closing paren (`| $comLoadError)`). That reached the browser as
    /// `try { __v = ((ST.resolve(...)); }` — a SyntaxError, which means the
    /// WHOLE bundle never evaluates.
    #[test]
    fn logical_or_is_not_a_filter_pipe() {
        let (v, f) = split_filter_pipe("($comError || $comLoadError)");
        assert_eq!(v, "($comError || $comLoadError)");
        assert_eq!(f, None, "`||` must never be read as a filter separator");
    }

    /// A bare `||` outside parens is the same operator.
    #[test]
    fn logical_or_without_parens_is_not_a_pipe() {
        let (v, f) = split_filter_pipe("$a || $b");
        assert_eq!(v, "$a || $b");
        assert_eq!(f, None);
    }

    /// A pipe INSIDE parens belongs to the sub-expression. Without the depth
    /// rule this breaks exactly like the reported bug, just via a different
    /// route — which is why the fix is depth-aware rather than a `||` special
    /// case.
    #[test]
    fn a_pipe_inside_parens_is_not_a_top_level_filter() {
        let (v, f) = split_filter_pipe("fmt($a | inner)");
        assert_eq!(v, "fmt($a | inner)");
        assert_eq!(f, None);
    }

    /// A pipe inside a STRING is data.
    #[test]
    fn a_pipe_inside_a_string_is_data() {
        let (v, f) = split_filter_pipe("$x | fmt(\"a|b\")");
        assert_eq!(v, "$x");
        assert_eq!(f, Some("fmt(\"a|b\")"));
    }

    /// THE POSITIVE — the feature must still work. Without this the three
    /// negatives above are satisfied by a function that never splits anything.
    #[test]
    fn a_real_filter_pipe_still_splits() {
        let (v, f) = split_filter_pipe("$n | currency(\"EUR\")");
        assert_eq!(v, "$n");
        assert_eq!(f, Some("currency(\"EUR\")"));
    }

    /// `||` FOLLOWED BY a real filter: both rules at once.
    #[test]
    fn logical_or_then_a_real_filter() {
        let (v, f) = split_filter_pipe("$a || $b | upper");
        assert_eq!(v, "$a || $b", "the `||` is part of the value");
        assert_eq!(f, Some("upper"), "the single `|` is still the filter");
    }

    /// An escaped quote must not end the string scan early.
    #[test]
    fn an_escaped_quote_does_not_end_the_string() {
        let (v, f) = split_filter_pipe("fmt(\"a\\\"|b\") | upper");
        assert_eq!(v, "fmt(\"a\\\"|b\")");
        assert_eq!(f, Some("upper"));
    }
}

/// Collect reactive-signal names referenced as CSS custom properties:
/// `var(--st-NAME)` → `NAME` (with fallback `var(--st-NAME, X)`). The runtime
/// publishes every signal as `--st-<name>` (reactive-binding.st), so a CSS
/// read of that property IS a use of the signal — the W0201 "defined but never
/// used" false positive for a source consumed only via `var(--st-x)`.
pub fn collect_st_var_deps(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    const MARK: &str = "var(--st-";
    let mut deps: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Advance BYTE-wise but only slice on char boundaries: a value such as
        // `content: "·"` (a multi-byte glyph) used to panic here, taking the
        // whole `check`/`build` down with it.
        if !value.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if value[i..].starts_with(MARK) {
            let start = i + MARK.len();
            let mut j = start;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > start {
                let name = value[start..j].to_string();
                if !deps.contains(&name) {
                    deps.push(name);
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    deps
}

#[cfg(test)]
mod st_var_deps_tests {
    use super::collect_st_var_deps;

    #[test]
    fn finds_signal_reads_with_and_without_fallback() {
        assert_eq!(
            collect_st_var_deps("calc(var(--st-k) * var(--st-total, 0))"),
            vec!["k".to_string(), "total".to_string()]
        );
    }

    /// A multi-byte glyph in a value (`content: "·"`) used to PANIC the scanner
    /// (byte index inside a char), taking `check` and `build` down with it.
    #[test]
    fn survives_multibyte_values() {
        assert!(collect_st_var_deps("\"·\"").is_empty());
        assert_eq!(collect_st_var_deps("éé var(--st-x) ·"), vec!["x".to_string()]);
    }
}

/// Signal names referenced by an `@on`/handler body. The body capture is a
/// `Named` map holding `js_statements` (source lines like `$count <- $count + 1`)
/// and `macro_calls`. Both the LHS write (`$count <-`) and RHS reads are uses
/// of the signal — a handler that WRITES a signal keeps it alive, so "defined
/// but never used" must not fire for it.
pub fn handler_body_signal_deps(
    captures: &std::collections::HashMap<String, CapturedValue>,
) -> Vec<String> {
    let mut deps: Vec<String> = Vec::new();
    let Some(CapturedValue::Named(body)) = captures.get("body") else {
        return deps;
    };
    let Some(CapturedValue::Array(stmts)) = body.get("js_statements") else {
        return deps;
    };
    for st in stmts {
        if let CapturedValue::String(s) = st {
            for d in collect_signal_deps(s) {
                if !deps.contains(&d) {
                    deps.push(d);
                }
            }
        }
    }
    deps
}

/// A scalar capture as a signal NAME: an ident/string verbatim (dashes are
/// legal in signal names — `new-session`), a binding without its `$`. A dotted
/// read (`$chat.status`) reduces to its BASE source — the head signal publishes
/// the update, the tail is property access.
fn capture_signal_name(value: &CapturedValue) -> Option<String> {
    let raw = match value {
        CapturedValue::Ident(s) | CapturedValue::String(s) => s.trim(),
        CapturedValue::Binding(s) => s.trim().trim_start_matches('$'),
        _ => return None,
    };
    let base = raw.split('.').next().unwrap_or(raw);
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

/// Signal names referenced by an `@on` MOTION body — the `on_motion_body`
/// capture shape (stdlib/macros/on.st): an Array of Named entries, each
/// carrying one alternative:
///
///   - `stmt` (on_signal_call `{signal, args}`) — firing `$send($draft)` reads
///     BOTH the fired signal and every signal in its arguments
///   - `mut` (mutation `{target, expr}`) — `$flag <- !$flag` reads the RHS and
///     WRITES the target; a write keeps the signal alive
///   - everything else (`call`, `line`, `slot`, …) — every String/Expr leaf is
///     scanned generically for `$sig` references
///
/// Distinct from [`handler_body_signal_deps`], which reads the `js_statements`
/// map only a `mutation_actions` body carries: an @on body never has that shape
/// at ANALYSIS time (js_statements are synthesized later, in the emit
/// pipeline), so every @on read was invisible to W0201 (BUG-380).
pub fn on_motion_body_signal_deps(
    captures: &std::collections::HashMap<String, CapturedValue>,
) -> Vec<String> {
    fn push(deps: &mut Vec<String>, name: String) {
        if !name.is_empty() && !deps.contains(&name) {
            deps.push(name);
        }
    }
    fn push_expr_deps(deps: &mut Vec<String>, value: &CapturedValue) {
        let text = match value {
            CapturedValue::String(s) | CapturedValue::Expr(s) => Some(s.as_str()),
            _ => None,
        };
        if let Some(t) = text {
            for d in collect_signal_deps(t) {
                push(deps, d);
            }
        }
    }
    // Generic string-leaf walk for body alternatives without a dedicated
    // reader (on_method_call, motion_line, …).
    fn walk_value(deps: &mut Vec<String>, value: &CapturedValue) {
        match value {
            CapturedValue::String(_) | CapturedValue::Expr(_) => push_expr_deps(deps, value),
            CapturedValue::Array(items) => items.iter().for_each(|v| walk_value(deps, v)),
            CapturedValue::Named(map) => map.values().for_each(|v| walk_value(deps, v)),
            _ => {}
        }
    }

    let mut deps: Vec<String> = Vec::new();
    let Some(CapturedValue::Array(entries)) = captures.get("body") else {
        return deps;
    };
    for entry in entries {
        let CapturedValue::Named(map) = entry else { continue };
        if let Some(CapturedValue::Named(stmt)) = map.get("stmt") {
            if let Some(name) = stmt.get("signal").and_then(capture_signal_name) {
                push(&mut deps, name);
            }
            if let Some(CapturedValue::Array(args)) = stmt.get("args") {
                for arg in args {
                    if let CapturedValue::Named(a) = arg
                        && let Some(v) = a.get("value")
                    {
                        push_expr_deps(&mut deps, v);
                    }
                }
            }
        } else if let Some(CapturedValue::Named(mutation)) = map.get("mut") {
            if let Some(name) = mutation.get("target").and_then(capture_signal_name) {
                push(&mut deps, name);
            }
            if let Some(expr) = mutation.get("expr") {
                push_expr_deps(&mut deps, expr);
            }
        } else {
            walk_value(&mut deps, entry);
        }
    }
    deps
}

