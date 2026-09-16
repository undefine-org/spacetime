//! Form Registry for LSP Features
//!
//! Indexes `%form` declarations from macros to power LSP features like
//! completions, hover, validation, and go-to-definition.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::metasystem::MetaRegistry;
use crate::parser::SourceSpan;
use crate::parser::meta_ast::{
    CaptureModifier, CaptureType, ExportDecl, FormClause, FormInlineElement, FormParam,
    MacroDefAst, ParamDefault, PrimitiveDefAst,
};

// =============================================================================
// ExportedSignal - A signal exported by a primitive
// =============================================================================

/// A signal exported by a primitive that becomes available in directive bodies.
///
/// For example, the `scroll` primitive exports `$progress`, `$velocity`, etc.
/// These become available when a macro binds to that primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSignal {
    /// Signal name with $ prefix (e.g., "$progress", "$visible")
    pub name: String,

    /// Signal type (e.g., "number", "bool", "string")
    pub signal_type: String,

    /// Whether the signal is optional (type?)
    pub optional: bool,

    /// Description from comment or inferred
    pub description: Option<String>,
}

impl ExportedSignal {
    /// Format the signal for display (e.g., "$progress: number")
    pub fn format(&self) -> String {
        let opt = if self.optional { "?" } else { "" };
        format!("{}: {}{}", self.name, self.signal_type, opt)
    }

    /// Create from a primitive ExportDecl
    pub fn from_export_decl(decl: &ExportDecl) -> Self {
        Self {
            name: format!("${}", decl.name),
            signal_type: decl.type_expr.format(),
            optional: decl.optional,
            description: None, // Comments not yet extracted
        }
    }
}

// =============================================================================
// DirectiveSignature - Describes a directive created by a macro
// =============================================================================

/// Signature of a directive created by a `%macro` with a `%form` clause.
///
/// This represents the "public API" of a macro - how users invoke the directive
/// in their Spacetime code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveSignature {
    /// Directive name (e.g., "scroll", "fade-in", "on hover")
    pub name: String,

    /// The macro that creates this directive
    pub macro_name: String,

    /// Parameters that can be passed to the directive
    pub params: Vec<DirectiveParam>,

    /// The type of body this directive accepts (if any)
    /// For example, `CaptureType::Properties` for property blocks
    pub body_type: Option<CaptureType>,

    /// Source location of the macro definition
    pub definition_span: SourceSpan,

    /// Path to the file containing the macro definition
    pub source_file: PathBuf,

    /// Documentation extracted from comments (if available)
    pub documentation: Option<String>,

    /// Signals exported by bound primitives (available inside directive body)
    #[serde(default)]
    pub exports: Vec<ExportedSignal>,

    /// Names of primitives this directive binds to
    #[serde(default)]
    pub bound_primitives: Vec<String>,

    /// The original FormClause for scoring-based disambiguation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<FormClause>,
}

impl DirectiveSignature {
    /// Returns a formatted signature string for display in hover/completions
    pub fn format_signature(&self) -> String {
        let mut sig = format!("@{}", self.name);

        if !self.params.is_empty() {
            let param_strs: Vec<String> = self.params.iter().map(|p| p.format()).collect();
            sig.push_str(&format!("({})", param_strs.join(", ")));
        }

        if self.body_type.is_some() {
            sig.push_str(" { ... }");
        }

        sig
    }

    /// Returns true if all required parameters have defaults
    pub fn has_all_defaults(&self) -> bool {
        self.params
            .iter()
            .filter(|p| p.modifier == CaptureModifier::Required)
            .all(|p| p.default_value.is_some())
    }

    /// Returns the list of required parameters (no default, not optional)
    pub fn required_params(&self) -> Vec<&DirectiveParam> {
        self.params
            .iter()
            .filter(|p| p.modifier == CaptureModifier::Required && p.default_value.is_none())
            .collect()
    }
}

// =============================================================================
// DirectiveParam - A parameter in a directive signature
// =============================================================================

/// A parameter in a directive's signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveParam {
    /// Parameter name as it appears in the syntax (e.g., "duration", "easing")
    pub name: String,

    /// Variable name that captures this parameter (without $)
    pub capture_var: String,

    /// The type of value this parameter accepts
    pub capture_type: CaptureType,

    /// Whether the parameter is required, optional, or allows multiple values
    pub modifier: CaptureModifier,

    /// Default value if not provided
    pub default_value: Option<String>,
}

impl DirectiveParam {
    /// Format the parameter for display
    pub fn format(&self) -> String {
        let mut s = self.name.clone();
        s.push_str(": ");
        s.push_str(&format_capture_type(&self.capture_type));

        match self.modifier {
            CaptureModifier::Optional => s.push('?'),
            CaptureModifier::ZeroOrMore => s.push('*'),
            CaptureModifier::OneOrMore => s.push('+'),
            CaptureModifier::Counted(counts) => {
                // Render as authored: `{3|4|6|8}` (stored descending, shown ascending
                // so it reads like the grammar the author wrote).
                let mut ns: Vec<u8> = counts.counts().to_vec();
                ns.reverse();
                let list: Vec<String> = ns.iter().map(|n| n.to_string()).collect();
                s.push_str(&format!("{{{}}}", list.join("|")));
            }
            CaptureModifier::Required => {}
        }

        if let Some(ref default) = self.default_value {
            s.push_str(&format!(" = {}", default));
        }

        s
    }

    /// Returns true if this parameter is required (not optional, no default)
    pub fn is_required(&self) -> bool {
        self.modifier == CaptureModifier::Required && self.default_value.is_none()
    }
}

/// Format a CaptureType for display
fn format_capture_type(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Ident => "ident".to_string(),
        CaptureType::DashedIdent => "dashed_ident".to_string(),
        CaptureType::EventName => "event_name".to_string(),
        CaptureType::String => "string".to_string(),
        CaptureType::Number => "number".to_string(),
        CaptureType::Bool => "bool".to_string(),
        CaptureType::Time => "time".to_string(),
        CaptureType::Length => "length".to_string(),
        CaptureType::Duration => "duration".to_string(),
        CaptureType::Easing => "easing".to_string(),
        CaptureType::Typeref => "typeref".to_string(),
        CaptureType::Binding => "binding".to_string(),
        CaptureType::Event => "event".to_string(),
        CaptureType::Expr => "expr".to_string(),
        CaptureType::Properties => "properties".to_string(),
        CaptureType::Fields => "fields".to_string(),
        CaptureType::Params => "params".to_string(),
        CaptureType::States => "states".to_string(),
        CaptureType::Transitions => "transitions".to_string(),
        CaptureType::Keyframes => "keyframes".to_string(),
        CaptureType::Selector => "selector".to_string(),
        CaptureType::Element => "element".to_string(),
        CaptureType::Preset => "preset".to_string(),
        CaptureType::MutationActions => "mutation_actions".to_string(),
        CaptureType::Template => "template".to_string(),
        CaptureType::ParamList => "param_list".to_string(),
        CaptureType::HtmlBlock => "html_block".to_string(),
        CaptureType::JsBlock => "jsblock".to_string(),
        CaptureType::ComponentBody => "component_body".to_string(),
        CaptureType::TemplateInvocation => "template_invocation".to_string(),
        CaptureType::Union(variants) => {
            let variant_strs: Vec<String> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
            format!("({})", variant_strs.join(" | "))
        }
        CaptureType::PatternMatch { variant, bindings } => {
            if bindings.is_empty() {
                format!("pattern (is {})", variant)
            } else {
                format!("pattern (is {} {{ {} }})", variant, bindings.join(", "))
            }
        }
        CaptureType::Custom(name) => name.clone(),
        CaptureType::Balanced(d) => format!("balanced('{}')", d),
        CaptureType::SkipBlock => "skip_block".to_string(),
        CaptureType::Color => "color".to_string(),
    }
}

/// Format a ParamDefault for display
fn format_param_default(default: &ParamDefault) -> String {
    match default {
        ParamDefault::String(s) => format!("\"{}\"", s),
        ParamDefault::Number(n) => n.to_string(),
        ParamDefault::Bool(b) => b.to_string(),
        ParamDefault::None => "none".to_string(),
        ParamDefault::EmptyArray => "[]".to_string(),
        ParamDefault::EmptyObject => "{}".to_string(),
        ParamDefault::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(format_param_default)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ParamDefault::Length(val, unit) => format!("{}{}", val, unit),
    }
}

// =============================================================================
// FormRegistry - Indexes all directive signatures
// =============================================================================

// =============================================================================
// PrimitiveInfo - Information about a primitive for LSP
// =============================================================================

/// Information about a primitive extracted for LSP features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveInfo {
    /// Primitive name (e.g., "scroll", "intersection")
    pub name: String,

    /// Parameters the primitive accepts
    pub params: Vec<PrimitiveParamInfo>,

    /// Signals exported by the primitive
    pub exports: Vec<ExportedSignal>,

    /// Source file path
    pub source_file: PathBuf,

    /// Source span for go-to-definition
    pub definition_span: SourceSpan,
}

/// Information about a primitive parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveParamInfo {
    /// Parameter name
    pub name: String,

    /// Parameter type (e.g., "number", "bool", "element")
    pub param_type: String,

    /// Default value if any
    pub default_value: Option<String>,

    /// Whether this is an element reference (&el)
    pub is_element: bool,
}

impl PrimitiveInfo {
    /// Extract from a PrimitiveDefAst
    pub fn from_primitive_def(def: &PrimitiveDefAst, source_file: &Path) -> Self {
        use crate::parser::meta_ast::{ParamType, PrimitiveParam};

        let params = def
            .params
            .iter()
            .map(|p| match p {
                PrimitiveParam::Element(name) => PrimitiveParamInfo {
                    name: format!("&{}", name),
                    param_type: "element".to_string(),
                    default_value: None,
                    is_element: true,
                },
                PrimitiveParam::Data(name) => PrimitiveParamInfo {
                    name: format!("${}", name),
                    param_type: "any".to_string(),
                    default_value: None,
                    is_element: false,
                },
                PrimitiveParam::TypedData { name, ty } => PrimitiveParamInfo {
                    name: format!("${}", name),
                    param_type: ty.clone(),
                    default_value: None,
                    is_element: false,
                },
                PrimitiveParam::Typed { name, ty, default } => {
                    let type_str = match ty {
                        ParamType::Simple(s) => s.clone(),
                        ParamType::Union(variants) => {
                            format!(
                                "({})",
                                variants
                                    .iter()
                                    .map(|v| format!("\"{}\"", v))
                                    .collect::<Vec<_>>()
                                    .join(" | ")
                            )
                        }
                        ParamType::Array(s) => format!("{}[]", s),
                        ParamType::Optional(s) => format!("{}?", s),
                        ParamType::OptionalArray(s) => format!("{}[]?", s),
                    };
                    PrimitiveParamInfo {
                        name: name.clone(),
                        param_type: type_str,
                        default_value: default.as_ref().map(format_param_default),
                        is_element: false,
                    }
                }
            })
            .collect();

        let exports = def
            .body
            .exports
            .iter()
            .map(ExportedSignal::from_export_decl)
            .collect();

        Self {
            name: def.name.clone(),
            params,
            exports,
            source_file: source_file.to_path_buf(),
            definition_span: def.span,
        }
    }

    /// Format a signature string for display
    pub fn format_signature(&self) -> String {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| {
                if let Some(ref default) = p.default_value {
                    format!("{}: {} = {}", p.name, p.param_type, default)
                } else {
                    format!("{}: {}", p.name, p.param_type)
                }
            })
            .collect();
        format!("{}({})", self.name, params.join(", "))
    }
}

// =============================================================================
// FormRegistry - Indexes all directive signatures and primitives
// =============================================================================

/// Registry of all directive signatures extracted from macros.
///
/// This is the core data structure for LSP features. It indexes all `%form`
/// declarations to enable:
/// - Completions: `directive_completions("scr")` -> ["scroll", "screen"]
/// - Hover: `get_directive("scroll")` -> DirectiveSignature with docs
/// - Validation: Check parameter types and required fields
/// - Go-to-definition: Use `definition_span` and `source_file`
/// - Relationships: Show macro relationships inferred from `%includes`
/// - Primitive exports: Show available signals inside directive bodies
#[derive(Debug, Clone, Default)]
pub struct FormRegistry {
    /// Map from directive name to its signatures (multiple forms per directive)
    directives: HashMap<String, Vec<DirectiveSignature>>,
    /// Map from primitive name to its info
    primitives: HashMap<String, PrimitiveInfo>,
    /// Macro relationships inferred from %includes
    relationships: crate::metasystem::relationships::RelationshipRegistry,
}

impl FormRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry from a MetaRegistry, extracting all directive signatures.
    ///
    /// This processes all macros that have a `%form` clause and indexes them by
    /// their directive name. Multiple forms for the same directive are preserved.
    /// Also extracts primitives and resolves their exports to directive signatures.
    pub fn from_meta_registry(meta: &MetaRegistry, source_file: &Path) -> Self {
        let mut registry = Self::new();

        // First, extract all primitives
        for prim_name in meta.primitive_names() {
            if let Some(prim_def) = meta.get_primitive(prim_name) {
                let prim_info = PrimitiveInfo::from_primitive_def(prim_def, source_file);
                registry
                    .primitives
                    .insert(prim_info.name.clone(), prim_info);
            }
        }

        // Then extract macros with resolved primitive exports
        for macro_name in meta.macro_names() {
            if let Some(macro_def) = meta.get_macro(macro_name)
                && let Some(sig) = Self::extract_signature_with_exports(
                    macro_def,
                    source_file,
                    &registry.primitives,
                )
            {
                registry
                    .directives
                    .entry(sig.name.clone())
                    .or_default()
                    .push(sig);
            }
        }

        registry
    }

    /// Extract a directive signature from a macro definition.
    ///
    /// Returns `None` if the macro doesn't create a directive (no `%creates` or `%form`).
    #[allow(dead_code)] // Used in tests
    fn extract_signature(
        macro_def: &MacroDefAst,
        source_file: &Path,
    ) -> Option<DirectiveSignature> {
        Self::extract_signature_with_exports(macro_def, source_file, &HashMap::new())
    }

    /// Extract a directive signature with resolved primitive exports.
    ///
    /// This version looks up bound primitives and includes their exports.
    fn extract_signature_with_exports(
        macro_def: &MacroDefAst,
        source_file: &Path,
        primitives: &HashMap<String, PrimitiveInfo>,
    ) -> Option<DirectiveSignature> {
        // Get the directive name from %form
        // Strip any leading sigil (@, $, &) since callers look up by bare name
        let directive_name = if let Some(ref form) = macro_def.form {
            form.directive_name
                .trim_start_matches(['@', '$', '&'])
                .to_string()
        } else {
            // No directive created by this macro
            return None;
        };

        // Extract parameters and body type from %form if present
        let (params, body_type) = if let Some(ref form) = macro_def.form {
            (Self::extract_params(form), Self::extract_body_type(form))
        } else {
            (vec![], None)
        };

        // Extract bound primitives and their exports
        let (bound_primitives, exports) =
            Self::extract_bound_primitives_and_exports(macro_def, primitives);

        Some(DirectiveSignature {
            name: directive_name,
            macro_name: macro_def.name.clone(),
            params,
            body_type,
            definition_span: macro_def.span,
            source_file: source_file.to_path_buf(),
            documentation: None, // Populated by extract_signature_with_exports_and_source
            exports,
            bound_primitives,
            form: macro_def.form.clone(),
        })
    }

    /// Extract bound primitive names and their exports from a macro definition.
    fn extract_bound_primitives_and_exports(
        macro_def: &MacroDefAst,
        primitives: &HashMap<String, PrimitiveInfo>,
    ) -> (Vec<String>, Vec<ExportedSignal>) {
        let mut bound_names = Vec::new();
        let mut exports = Vec::new();
        let mut seen_exports = std::collections::HashSet::new();

        // Extract from %binds declarations
        for bind in &macro_def.binds {
            let prim_name = &bind.primitive;
            if !bound_names.contains(prim_name) {
                bound_names.push(prim_name.clone());
            }

            // Look up primitive and extract its exports
            if let Some(prim_info) = primitives.get(prim_name) {
                for export in &prim_info.exports {
                    // Check if this export is captured by the bind output
                    // The bind.outputs specifies which exports to capture
                    let should_include = if bind.outputs.is_empty() {
                        // No explicit outputs = include all
                        true
                    } else {
                        // Check if this export is in the outputs list
                        bind.outputs.iter().any(|out| {
                            // Match by name (without $ prefix on output side)
                            format!("${}", out.name) == export.name
                                || out.name == export.name.trim_start_matches('$')
                        })
                    };

                    if should_include && !seen_exports.contains(&export.name) {
                        // Apply alias if present
                        let mut exported_signal = export.clone();
                        for out in &bind.outputs {
                            if (format!("${}", out.name) == export.name
                                || out.name == export.name.trim_start_matches('$'))
                                && let Some(ref alias) = out.alias
                            {
                                // The alias is a pattern like "${$name}-loading"
                                // For now, just note that an alias exists
                                exported_signal.description =
                                    Some(format!("Aliased as: {}", alias));
                            }
                        }
                        seen_exports.insert(export.name.clone());
                        exports.push(exported_signal);
                    }
                }
            }
        }

        (bound_names, exports)
    }

    /// Extract parameters from a FormClause
    fn extract_params(form: &FormClause) -> Vec<DirectiveParam> {
        let mut params = Vec::new();

        // Extract inline elements that are captures
        for element in &form.inline_elements {
            if let FormInlineElement::Capture(capture, default) = element {
                params.push(DirectiveParam {
                    name: capture.var_name.clone(),
                    capture_var: capture.var_name.clone(),
                    capture_type: capture.capture_type.clone(),
                    modifier: capture.modifier,
                    default_value: default.as_ref().map(format_param_default),
                });
            }
        }

        // Extract regular parameters
        for param in &form.params {
            params.push(Self::form_param_to_directive_param(param));
        }

        params
    }

    /// Convert a FormParam to a DirectiveParam
    fn form_param_to_directive_param(param: &FormParam) -> DirectiveParam {
        // For simple single-capture params, use the capture directly
        // For multi-element params, use the first capture
        if let Some(capture) = param.capture() {
            DirectiveParam {
                name: param.name.clone(),
                capture_var: capture.var_name.clone(),
                capture_type: capture.capture_type.clone(),
                modifier: capture.modifier,
                default_value: param.default.as_ref().map(format_param_default),
            }
        } else {
            // Multi-element param - find first capture in elements
            let (var_name, cap_type, modifier) = param
                .elements
                .iter()
                .filter_map(|e| {
                    if let FormInlineElement::Capture(cap, _) = e {
                        Some((cap.var_name.clone(), cap.capture_type.clone(), cap.modifier))
                    } else {
                        None
                    }
                })
                .next()
                .unwrap_or_else(|| {
                    (
                        "".to_string(),
                        CaptureType::Ident,
                        CaptureModifier::Required,
                    )
                });

            DirectiveParam {
                name: param.name.clone(),
                capture_var: var_name,
                capture_type: cap_type,
                modifier,
                default_value: param.default.as_ref().map(format_param_default),
            }
        }
    }

    /// Extract the body capture type from a FormClause
    fn extract_body_type(form: &FormClause) -> Option<CaptureType> {
        // Check if there's a body capture
        if form.body_capture.is_some() {
            // For now, treat body captures as Properties type
            // In the future, we could parse the body_capture string to determine type
            Some(CaptureType::Properties)
        } else {
            None
        }
    }

    /// Get a directive by name (returns the first form)
    pub fn get_directive(&self, name: &str) -> Option<&DirectiveSignature> {
        self.directives.get(name).and_then(|v| v.first())
    }

    /// Get all forms for a directive name
    pub fn get_all_forms(&self, name: &str) -> &[DirectiveSignature] {
        self.directives
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get a primitive by name
    pub fn get_primitive(&self, name: &str) -> Option<&PrimitiveInfo> {
        self.primitives.get(name)
    }

    /// Get all primitives in the registry
    pub fn all_primitives(&self) -> impl Iterator<Item = &PrimitiveInfo> {
        self.primitives.values()
    }

    /// Get primitive names
    pub fn primitive_names(&self) -> impl Iterator<Item = &str> {
        self.primitives.keys().map(|s| s.as_str())
    }

    /// Get the number of registered primitives
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Register a primitive directly
    pub fn register_primitive(&mut self, info: PrimitiveInfo) {
        self.primitives.insert(info.name.clone(), info);
    }

    /// Get exports for a directive (from its bound primitives)
    pub fn get_directive_exports(&self, directive_name: &str) -> Vec<&ExportedSignal> {
        self.get_directive(directive_name)
            .map(|sig| sig.exports.iter().collect())
            .unwrap_or_default()
    }

    /// Get directive completions matching a prefix
    ///
    /// Returns tuples of (directive_name, signature) for all directives
    /// whose names start with the given prefix.
    pub fn directive_completions(&self, prefix: &str) -> Vec<(String, &DirectiveSignature)> {
        let prefix_lower = prefix.to_lowercase();
        self.directives
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&prefix_lower))
            .filter_map(|(name, sigs)| sigs.first().map(|sig| (name.clone(), sig)))
            .collect()
    }

    /// Get all directives in the registry (flattened across all forms)
    pub fn all_directives(&self) -> impl Iterator<Item = &DirectiveSignature> {
        self.directives.values().flat_map(|v| v.iter())
    }

    /// Get the number of registered directives
    pub fn len(&self) -> usize {
        self.directives.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.directives.is_empty()
    }

    /// Register a directive signature directly (appends as a new form)
    pub fn register(&mut self, signature: DirectiveSignature) {
        self.directives
            .entry(signature.name.clone())
            .or_default()
            .push(signature);
    }

    /// Get all directive names
    pub fn directive_names(&self) -> impl Iterator<Item = &str> {
        self.directives.keys().map(|s| s.as_str())
    }

    /// Extract doc comments (`///`) preceding a span position in source text.
    ///
    /// Looks backward from the span start to find consecutive `///` comment lines.
    /// Returns the combined documentation string with the `///` prefixes stripped.
    fn extract_doc_comments(source: &str, span_start: usize) -> Option<String> {
        if span_start == 0 || source.is_empty() {
            return None;
        }

        // Find the start of the line containing span_start
        let before_span = &source[..span_start];

        // Split into lines, keeping track of positions
        let lines: Vec<&str> = before_span.lines().collect();
        if lines.is_empty() {
            return None;
        }

        // Work backward from the last line before the span to collect /// comments
        let mut doc_lines: Vec<&str> = Vec::new();

        for line in lines.iter().rev() {
            let trimmed = line.trim();

            if trimmed.starts_with("///") {
                // Extract the comment content (strip "///" and leading space)
                let content = trimmed.strip_prefix("///").unwrap_or("");
                let content = content.strip_prefix(' ').unwrap_or(content);
                doc_lines.push(content);
            } else if trimmed.is_empty() {
                // Skip empty lines between comments and definition
                continue;
            } else {
                // Non-comment, non-empty line - stop collecting
                break;
            }
        }

        if doc_lines.is_empty() {
            return None;
        }

        // Reverse to get correct order and join with newlines
        doc_lines.reverse();
        Some(doc_lines.join("\n"))
    }

    /// Index a file's macro and primitive definitions into this registry.
    ///
    /// Parses the given Spacetime source content and extracts:
    /// - Directive signatures from `%macro` definitions with `%form` or `%creates` clauses
    /// - Primitive info from `%primitive` definitions with their exports
    pub fn index_file(&mut self, file: &crate::parser::StFile, source_path: &Path) {
        // Delegate to the version without source (no doc comment extraction)
        self.index_file_with_source(file, source_path, None);
    }

    /// Index a file's macro and primitive definitions with optional source text for doc comments.
    ///
    /// When source is provided, extracts `///` doc comments preceding macro definitions.
    pub fn index_file_with_source(
        &mut self,
        file: &crate::parser::StFile,
        source_path: &Path,
        source: Option<&str>,
    ) {
        // Build a temporary MetaRegistry from the file's meta_defs
        let mut meta = MetaRegistry::new();
        for def in &file.meta_defs {
            let _ = meta.register(def.clone());
        }

        // First, extract primitives
        for prim_name in meta.primitive_names() {
            if let Some(prim_def) = meta.get_primitive(prim_name) {
                let prim_info = PrimitiveInfo::from_primitive_def(prim_def, source_path);
                self.primitives.insert(prim_info.name.clone(), prim_info);
            }
        }

        // Then extract macro signatures with resolved primitive exports
        for macro_name in meta.macro_names() {
            if let Some(macro_def) = meta.get_macro(macro_name)
                && let Some(mut sig) =
                    Self::extract_signature_with_exports(macro_def, source_path, &self.primitives)
            {
                // Extract doc comments if source is available
                if let Some(source_text) = source {
                    sig.documentation =
                        Self::extract_doc_comments(source_text, macro_def.span.start);
                }
                self.directives
                    .entry(sig.name.clone())
                    .or_default()
                    .push(sig);
            }
        }
    }

    /// Create a FormRegistry from the compiler's live MetaRegistry.
    ///
    /// The compiler builds its registry by loading stdlib (from disk via the
    /// mtime-based incremental cache, or the embedded fallback) plus the
    /// project `_prelude.st` overlay. The LSP serves THAT registry — one
    /// source of truth — rather than a hand-maintained `include_str!` list
    /// that drifted from what `spacetime check` actually resolves (gh-28).
    ///
    /// Non-blocking: `cached_stdlib_registry()` reuses the compiler's binary
    /// cache and only re-parses files whose mtime changed, so a fresh editor
    /// startup does not re-parse the full stdlib.
    pub fn from_compiler() -> Self {
        let (meta, _errors) = crate::compiler::cached_stdlib_registry();
        Self::from_compiler_registry(&meta)
    }

    /// Build a FormRegistry from a compiler `MetaRegistry`, using each
    /// definition's OWN canonical source path and captured `///` doc block.
    ///
    /// Unlike the old embedded snapshot (which indexed every stdlib file with
    /// a synthetic name), this preserves real absolute paths so go-to-
    /// definition opens the actual file, and it includes whatever the
    /// MetaRegistry was loaded with — stdlib and any project overlay merged
    /// in by the caller.
    pub fn from_compiler_registry(meta: &MetaRegistry) -> Self {
        let mut registry = Self::new();

        // Primitives first (their exports resolve into directive signatures).
        for prim_name in meta.primitive_names() {
            if let Some(prim_def) = meta.get_primitive(prim_name) {
                let path = prim_def
                    .source_file
                    .as_deref()
                    .map(Path::new)
                    .unwrap_or(Path::new("<unknown>"));
                let prim_info = PrimitiveInfo::from_primitive_def(prim_def, path);
                registry.primitives.insert(prim_info.name.clone(), prim_info);
            }
        }

        // Then macros with resolved primitive exports.
        for (_, macro_def) in meta.iter_macros() {
            let path = macro_def
                .source_file
                .as_deref()
                .map(Path::new)
                .unwrap_or(Path::new("<unknown>"));
            if let Some(mut sig) =
                Self::extract_signature_with_exports(macro_def, path, &registry.primitives)
            {
                // The parser captures the contiguous `///` block on the meta
                // def itself (see test_doc_comment_captured_on_macro), so the
                // LSP gets the same doc comments without re-reading source.
                sig.documentation = macro_def.doc.clone();
                registry
                    .directives
                    .entry(sig.name.clone())
                    .or_default()
                    .push(sig);
            }
        }

        registry
    }

    /// Get the primary relationship for a macro (e.g., "fade-in-up" -> "partial of @fade-in")
    pub fn get_relationship(
        &self,
        macro_name: &str,
    ) -> Option<&crate::metasystem::relationships::MacroRelationship> {
        self.relationships.get_primary_relationship(macro_name)
    }

    /// Check if a macro has relationships (is a partial/wrapper of another macro)
    pub fn has_relationship(&self, macro_name: &str) -> bool {
        self.relationships.has_relationships(macro_name)
    }

    /// Get all relationships for a macro
    pub fn get_all_relationships(
        &self,
        macro_name: &str,
    ) -> Option<&Vec<crate::metasystem::relationships::MacroRelationship>> {
        self.relationships.get_relationships(macro_name)
    }

    /// Get a reference to the relationship registry
    pub fn relationship_registry(&self) -> &crate::metasystem::relationships::RelationshipRegistry {
        &self.relationships
    }

    /// Get a mutable reference to the relationship registry
    pub fn relationship_registry_mut(
        &mut self,
    ) -> &mut crate::metasystem::relationships::RelationshipRegistry {
        &mut self.relationships
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::{FormCapture, MacroDefAst};

    fn make_test_form_clause(
        directive_name: &str,
        params: Vec<FormParam>,
        has_body: bool,
    ) -> FormClause {
        FormClause {
            directive_name: directive_name.to_string(),
            inline_elements: vec![],
            params,
            post_arg_inline: vec![],
            body_capture: if has_body {
                Some("$body".to_string())
            } else {
                None
            },
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        }
    }

    fn make_test_macro(name: &str, form: Option<FormClause>) -> MacroDefAst {
        MacroDefAst {
            retired: None,
            name: name.to_string(),
            form,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        }
    }

    fn make_form_param(name: &str, capture_type: CaptureType) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: name.to_string(),
                    capture_type,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            default: None,
        }
    }

    fn make_optional_param(
        name: &str,
        capture_type: CaptureType,
        default: Option<ParamDefault>,
    ) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: name.to_string(),
                    capture_type,
                    modifier: CaptureModifier::Optional,
                    alias_capture: None,
                },
                None,
            )],
            default,
        }
    }

    #[test]
    fn test_extract_simple_directive() {
        let form = make_test_form_clause("scroll", vec![], false);
        let macro_def = make_test_macro("ScrollMacro", Some(form));

        let sig =
            FormRegistry::extract_signature(&macro_def, Path::new("/test/stdlib.st")).unwrap();

        assert_eq!(sig.name, "scroll");
        assert_eq!(sig.macro_name, "ScrollMacro");
        assert!(sig.params.is_empty());
        assert!(sig.body_type.is_none());
    }

    #[test]
    fn test_extract_directive_with_params() {
        let params = vec![
            make_form_param("duration", CaptureType::Duration),
            make_form_param("easing", CaptureType::Easing),
        ];
        let form = make_test_form_clause("fade-in", params, false);
        let macro_def = make_test_macro("FadeInMacro", Some(form));

        let sig =
            FormRegistry::extract_signature(&macro_def, Path::new("/test/stdlib.st")).unwrap();

        assert_eq!(sig.name, "fade-in");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].name, "duration");
        assert_eq!(sig.params[0].capture_type, CaptureType::Duration);
        assert_eq!(sig.params[1].name, "easing");
        assert_eq!(sig.params[1].capture_type, CaptureType::Easing);
    }

    #[test]
    fn test_extract_directive_with_body() {
        let form = make_test_form_clause("state", vec![], true);
        let macro_def = make_test_macro("StateMacro", Some(form));

        let sig =
            FormRegistry::extract_signature(&macro_def, Path::new("/test/stdlib.st")).unwrap();

        assert_eq!(sig.name, "state");
        assert!(sig.body_type.is_some());
        assert_eq!(sig.body_type, Some(CaptureType::Properties));
    }

    #[test]
    fn test_no_directive_for_macro_without_form() {
        let macro_def = make_test_macro("HelperMacro", None);

        let sig = FormRegistry::extract_signature(&macro_def, Path::new("/test/stdlib.st"));

        assert!(sig.is_none());
    }

    #[test]
    fn test_directive_completions() {
        let mut registry = FormRegistry::new();

        registry.register(DirectiveSignature {
            name: "scroll".to_string(),
            macro_name: "ScrollMacro".to_string(),
            params: vec![],
            body_type: None,
            definition_span: SourceSpan::default(),
            source_file: PathBuf::from("/test/stdlib.st"),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        });

        registry.register(DirectiveSignature {
            name: "screen".to_string(),
            macro_name: "ScreenMacro".to_string(),
            params: vec![],
            body_type: None,
            definition_span: SourceSpan::default(),
            source_file: PathBuf::from("/test/stdlib.st"),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        });

        registry.register(DirectiveSignature {
            name: "fade-in".to_string(),
            macro_name: "FadeInMacro".to_string(),
            params: vec![],
            body_type: None,
            definition_span: SourceSpan::default(),
            source_file: PathBuf::from("/test/stdlib.st"),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        });

        // Test prefix matching
        let completions = registry.directive_completions("scr");
        assert_eq!(completions.len(), 2);

        let names: Vec<&str> = completions.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"scroll"));
        assert!(names.contains(&"screen"));

        // Test case insensitivity
        let completions = registry.directive_completions("SCR");
        assert_eq!(completions.len(), 2);

        // Test no matches
        let completions = registry.directive_completions("xyz");
        assert!(completions.is_empty());

        // Test empty prefix returns all
        let completions = registry.directive_completions("");
        assert_eq!(completions.len(), 3);
    }

    #[test]
    fn test_format_signature() {
        let sig = DirectiveSignature {
            name: "animate".to_string(),
            macro_name: "AnimateMacro".to_string(),
            params: vec![
                DirectiveParam {
                    name: "duration".to_string(),
                    capture_var: "dur".to_string(),
                    capture_type: CaptureType::Duration,
                    modifier: CaptureModifier::Required,
                    default_value: None,
                },
                DirectiveParam {
                    name: "easing".to_string(),
                    capture_var: "ease".to_string(),
                    capture_type: CaptureType::Easing,
                    modifier: CaptureModifier::Optional,
                    default_value: Some("\"ease-out\"".to_string()),
                },
            ],
            body_type: Some(CaptureType::Properties),
            definition_span: SourceSpan::default(),
            source_file: PathBuf::from("/test/stdlib.st"),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        };

        let formatted = sig.format_signature();
        assert!(formatted.contains("@animate"));
        assert!(formatted.contains("duration"));
        assert!(formatted.contains("easing"));
        assert!(formatted.contains("{ ... }"));
    }

    #[test]
    fn test_required_params() {
        let sig = DirectiveSignature {
            name: "test".to_string(),
            macro_name: "TestMacro".to_string(),
            params: vec![
                DirectiveParam {
                    name: "required".to_string(),
                    capture_var: "req".to_string(),
                    capture_type: CaptureType::String,
                    modifier: CaptureModifier::Required,
                    default_value: None,
                },
                DirectiveParam {
                    name: "optional".to_string(),
                    capture_var: "opt".to_string(),
                    capture_type: CaptureType::String,
                    modifier: CaptureModifier::Optional,
                    default_value: None,
                },
                DirectiveParam {
                    name: "with_default".to_string(),
                    capture_var: "def".to_string(),
                    capture_type: CaptureType::String,
                    modifier: CaptureModifier::Required,
                    default_value: Some("\"default\"".to_string()),
                },
            ],
            body_type: None,
            definition_span: SourceSpan::default(),
            source_file: PathBuf::from("/test/stdlib.st"),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        };

        let required = sig.required_params();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].name, "required");
    }

    #[test]
    fn test_param_with_optional_modifier() {
        let params = vec![make_optional_param(
            "easing",
            CaptureType::Easing,
            Some(ParamDefault::String("ease-out".to_string())),
        )];
        let form = make_test_form_clause("animate", params, false);
        let macro_def = make_test_macro("AnimateMacro", Some(form));

        let sig =
            FormRegistry::extract_signature(&macro_def, Path::new("/test/stdlib.st")).unwrap();

        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].modifier, CaptureModifier::Optional);
        assert_eq!(
            sig.params[0].default_value,
            Some("\"ease-out\"".to_string())
        );
    }

    #[test]
    fn test_format_capture_type() {
        assert_eq!(format_capture_type(&CaptureType::Duration), "duration");
        assert_eq!(format_capture_type(&CaptureType::Easing), "easing");
        assert_eq!(format_capture_type(&CaptureType::Properties), "properties");

        let union = CaptureType::Union(vec!["x".to_string(), "y".to_string(), "both".to_string()]);
        assert_eq!(format_capture_type(&union), "(\"x\" | \"y\" | \"both\")");
    }

    #[test]
    fn test_exported_signal_format() {
        let export = ExportedSignal {
            name: "$progress".to_string(),
            signal_type: "number".to_string(),
            optional: false,
            description: None,
        };
        assert_eq!(export.format(), "$progress: number");

        let optional_export = ExportedSignal {
            name: "$data".to_string(),
            signal_type: "object".to_string(),
            optional: true,
            description: Some("Optional data".to_string()),
        };
        assert_eq!(optional_export.format(), "$data: object?");
    }

    #[test]
    fn test_primitive_info_format_signature() {
        let prim = PrimitiveInfo {
            name: "scroll".to_string(),
            params: vec![
                PrimitiveParamInfo {
                    name: "&el".to_string(),
                    param_type: "element".to_string(),
                    default_value: None,
                    is_element: true,
                },
                PrimitiveParamInfo {
                    name: "axis".to_string(),
                    param_type: "(\"x\" | \"y\" | \"both\")".to_string(),
                    default_value: Some("\"y\"".to_string()),
                    is_element: false,
                },
            ],
            exports: vec![ExportedSignal {
                name: "$progress".to_string(),
                signal_type: "number".to_string(),
                optional: false,
                description: None,
            }],
            source_file: PathBuf::from("/test/scroll.st"),
            definition_span: SourceSpan::default(),
        };

        let sig = prim.format_signature();
        assert!(sig.contains("scroll"));
        assert!(sig.contains("&el"));
        assert!(sig.contains("axis"));
    }

    #[test]
    fn test_directive_with_exports() {
        // Create a signature with exports
        let sig = DirectiveSignature {
            name: "fade-in".to_string(),
            macro_name: "FadeInMacro".to_string(),
            params: vec![],
            body_type: None,
            definition_span: SourceSpan::default(),
            source_file: PathBuf::from("/test/fade-in.st"),
            documentation: None,
            exports: vec![ExportedSignal {
                name: "$visible".to_string(),
                signal_type: "bool".to_string(),
                optional: false,
                description: Some("Visibility state".to_string()),
            }],
            bound_primitives: vec!["intersection".to_string()],
            form: None,
        };

        assert_eq!(sig.exports.len(), 1);
        assert_eq!(sig.exports[0].name, "$visible");
        assert_eq!(sig.bound_primitives.len(), 1);
        assert_eq!(sig.bound_primitives[0], "intersection");
    }

    #[test]
    fn test_registry_primitive_methods() {
        let mut registry = FormRegistry::new();

        let prim = PrimitiveInfo {
            name: "scroll".to_string(),
            params: vec![],
            exports: vec![ExportedSignal {
                name: "$progress".to_string(),
                signal_type: "number".to_string(),
                optional: false,
                description: None,
            }],
            source_file: PathBuf::from("/test/scroll.st"),
            definition_span: SourceSpan::default(),
        };

        registry.register_primitive(prim);

        assert_eq!(registry.primitive_count(), 1);
        assert!(registry.get_primitive("scroll").is_some());
        assert!(registry.get_primitive("nonexistent").is_none());

        let names: Vec<_> = registry.primitive_names().collect();
        assert!(names.contains(&"scroll"));
    }

    #[test]
    fn test_extract_doc_comments() {
        // Test basic doc comment extraction
        let source = r#"
/// This is a doc comment
/// with multiple lines
%macro TestMacro {
    %creates @test
}
"#;
        // The macro starts at the position of '%macro'
        let span_start = source.find("%macro").unwrap();
        let docs = FormRegistry::extract_doc_comments(source, span_start);
        assert!(docs.is_some());
        let doc_text = docs.unwrap();
        assert!(doc_text.contains("This is a doc comment"));
        assert!(doc_text.contains("with multiple lines"));

        // Test with no doc comments
        let source_no_docs = r#"
// Regular comment (not doc comment)
%macro TestMacro {
    %creates @test
}
"#;
        let span_start = source_no_docs.find("%macro").unwrap();
        let docs = FormRegistry::extract_doc_comments(source_no_docs, span_start);
        assert!(docs.is_none());

        // Test with empty line between comment and definition
        let source_with_gap = r#"
/// Doc comment with gap

%macro TestMacro {
    %creates @test
}
"#;
        let span_start = source_with_gap.find("%macro").unwrap();
        let docs = FormRegistry::extract_doc_comments(source_with_gap, span_start);
        assert!(docs.is_some());
        assert!(docs.unwrap().contains("Doc comment with gap"));
    }
}
