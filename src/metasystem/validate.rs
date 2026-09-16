//! Validation for primitives and macros.
//!
//! Checks semantic correctness of % definitions and validates
//! macro calls against %form patterns.

#[cfg(any(feature = "lsp", feature = "wasm"))]
use crate::lsp::form_registry::FormRegistry;
use crate::parser::meta_ast::*;
use std::collections::HashSet;

use super::registry::MetaRegistry;
use crate::emit::emit_tokenizer::{EmitSegment, tokenize_emit};
use crate::parser::SourceSpan;

/// Built-in types that don't need @type definitions.
/// These are hardcoded in the compiler.
const BUILTIN_TYPES: &[&str] = &[
    // Primitives
    "number", "string", "bool", "effect", "void", "any",
    // Containers (these can be generic but we check the base name)
    "array", "map", "optional", // DOM types (simple)
    "element", "object",
];

/// Validation error with source location
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum ValidationErrorKind {
    /// Unknown primitive referenced in %binds
    UnknownPrimitive(String),
    /// Unknown macro referenced in %includes
    UnknownMacro(String),
    /// Missing required parameter
    MissingParameter { primitive: String, param: String },
    /// Parameter type mismatch
    ParameterTypeMismatch {
        param: String,
        expected: String,
        got: String,
    },
    /// Invalid emit language
    InvalidEmitLang(String),
    /// Unbound variable in emit block
    UnboundEmitVariable(String),
    /// Invalid yield target
    InvalidYieldTarget(String),
    /// Invalid form pattern
    InvalidFormPattern(String),
    /// Unknown capture type
    UnknownCaptureType(String),
    /// Circular macro dependency
    CircularMacroDependency(Vec<String>),
    /// Unbound variable in macro body
    UnboundMacroVariable(String),
    /// Invalid binds declaration
    InvalidBindsDecl(String),
    /// Export type mismatch
    ExportTypeMismatch {
        name: String,
        declared: String,
        actual: String,
    },
    /// Duplicate export
    DuplicateExport(String),
    /// Invalid derives expression
    InvalidDerivesExpr(String),
    /// Missing form clause for directive
    MissingFormForDirective(String),
    /// Invalid trigger reference
    InvalidTrigger(String),
    /// Macro body item in invalid context
    InvalidBodyContext(String),
    /// Unknown directive (no matching %form or %creates)
    UnknownDirective(String),
    /// Migration `%date` is not a valid ISO `YYYY-MM-DD` (PLAN-076)
    InvalidMigrationDate(String),
    /// Migration failed structural validation (PLAN-076)
    InvalidMigration(String),
    /// Missing required argument in macro call
    MissingRequiredArg { directive: String, param: String },
    /// Unexpected argument in macro call
    UnexpectedArg { directive: String, arg: String },
    /// Argument type mismatch in macro call
    ArgTypeMismatch {
        directive: String,
        param: String,
        expected: String,
        got: String,
    },
    /// E0196: Yield to undeclared export
    YieldToUndeclaredExport(String),
    /// E0197: Export never yielded
    ExportNeverYielded(String),
    /// E0198: Legacy function type (bare fn without signature)
    LegacyFunctionType(String),
    /// E0199: Unknown type reference in export
    UnknownTypeReference(String),
    /// Optional inline capture without explicit default value
    OptionalCaptureWithoutDefault {
        macro_name: String,
        capture_name: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ValidationErrorKind::InvalidMigrationDate(msg)
            | ValidationErrorKind::InvalidMigration(msg) => {
                write!(f, "{}", msg)
            }
            ValidationErrorKind::UnknownPrimitive(name) => {
                write!(f, "unknown primitive: {}", name)
            }
            ValidationErrorKind::UnknownMacro(name) => {
                write!(f, "unknown macro: %{}", name)
            }
            ValidationErrorKind::MissingParameter { primitive, param } => {
                write!(
                    f,
                    "missing required parameter '{}' for primitive {}",
                    param, primitive
                )
            }
            ValidationErrorKind::ParameterTypeMismatch {
                param,
                expected,
                got,
            } => {
                write!(
                    f,
                    "type mismatch for '{}': expected {}, got {}",
                    param, expected, got
                )
            }
            ValidationErrorKind::InvalidEmitLang(lang) => {
                write!(
                    f,
                    "invalid emit language: {} (expected 'js' or 'glsl')",
                    lang
                )
            }
            ValidationErrorKind::UnboundEmitVariable(name) => {
                write!(f, "unbound variable in emit block: ${}", name)
            }
            ValidationErrorKind::InvalidYieldTarget(target) => {
                write!(f, "invalid yield target: {}", target)
            }
            ValidationErrorKind::InvalidFormPattern(msg) => {
                write!(f, "invalid form pattern: {}", msg)
            }
            ValidationErrorKind::UnknownCaptureType(ty) => {
                write!(f, "unknown capture type: {}", ty)
            }
            ValidationErrorKind::CircularMacroDependency(cycle) => {
                write!(f, "circular macro dependency: {}", cycle.join(" -> "))
            }
            ValidationErrorKind::UnboundMacroVariable(name) => {
                write!(f, "unbound variable in macro body: ${}", name)
            }
            ValidationErrorKind::InvalidBindsDecl(msg) => {
                write!(f, "invalid binds declaration: {}", msg)
            }
            ValidationErrorKind::ExportTypeMismatch {
                name,
                declared,
                actual,
            } => {
                write!(
                    f,
                    "export ${} declared as {} but yields {}",
                    name, declared, actual
                )
            }
            ValidationErrorKind::DuplicateExport(name) => {
                write!(f, "duplicate export: ${}", name)
            }
            ValidationErrorKind::InvalidDerivesExpr(msg) => {
                write!(f, "invalid derives expression: {}", msg)
            }
            ValidationErrorKind::MissingFormForDirective(name) => {
                write!(f, "@{} needs a %form clause to define its syntax", name)
            }
            ValidationErrorKind::InvalidTrigger(name) => {
                write!(f, "invalid trigger: {}", name)
            }
            ValidationErrorKind::InvalidBodyContext(msg) => {
                write!(f, "invalid macro body context: {}", msg)
            }
            ValidationErrorKind::UnknownDirective(name) => {
                write!(f, "unknown directive @{} - no macro defines this", name)
            }
            ValidationErrorKind::MissingRequiredArg { directive, param } => {
                write!(f, "@{} requires argument '{}'", directive, param)
            }
            ValidationErrorKind::UnexpectedArg { directive, arg } => {
                write!(f, "@{} does not accept argument '{}'", directive, arg)
            }
            ValidationErrorKind::ArgTypeMismatch {
                directive,
                param,
                expected,
                got,
            } => {
                write!(
                    f,
                    "@{} argument '{}' expects {}, got {}",
                    directive, param, expected, got
                )
            }
            ValidationErrorKind::YieldToUndeclaredExport(name) => {
                write!(
                    f,
                    "yield to undeclared export ${} - add it to %exports block",
                    name
                )
            }
            ValidationErrorKind::ExportNeverYielded(name) => {
                write!(f, "export ${} is declared but never yielded", name)
            }
            ValidationErrorKind::LegacyFunctionType(name) => {
                write!(
                    f,
                    "export ${} uses bare 'fn' - use 'fn() ~> effect' or 'fn(params) ~> ReturnType'",
                    name
                )
            }
            ValidationErrorKind::UnknownTypeReference(ty) => {
                write!(
                    f,
                    "unknown type '{}' in export - use built-in types or define with @type",
                    ty
                )
            }
            ValidationErrorKind::OptionalCaptureWithoutDefault {
                macro_name,
                capture_name,
            } => {
                write!(
                    f,
                    "optional inline capture '${}' in macro '{}' must have an explicit default (e.g. ${}:type? = value)",
                    capture_name, macro_name, capture_name
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl ValidationError {
    pub fn unknown_primitive(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::UnknownPrimitive(name),
            span,
        }
    }

    pub fn unknown_macro(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::UnknownMacro(name),
            span,
        }
    }

    pub fn unbound_emit_variable(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::UnboundEmitVariable(name),
            span,
        }
    }

    pub fn unbound_macro_variable(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::UnboundMacroVariable(name),
            span,
        }
    }

    pub fn duplicate_export(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::DuplicateExport(name),
            span,
        }
    }

    pub fn circular_dependency(cycle: Vec<String>, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::CircularMacroDependency(cycle),
            span,
        }
    }

    pub fn yield_to_undeclared_export(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::YieldToUndeclaredExport(name),
            span,
        }
    }

    pub fn export_never_yielded(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::ExportNeverYielded(name),
            span,
        }
    }

    pub fn legacy_function_type(name: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::LegacyFunctionType(name),
            span,
        }
    }

    pub fn unknown_type_reference(ty: String, span: SourceSpan) -> Self {
        Self {
            kind: ValidationErrorKind::UnknownTypeReference(ty),
            span,
        }
    }
}

/// Validate a primitive definition
pub fn validate_primitive(
    primitive: &PrimitiveDefAst,
    _registry: &MetaRegistry,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Collect parameter names for variable checking
    let mut param_names: HashSet<String> = HashSet::new();
    for param in &primitive.params {
        match param {
            PrimitiveParam::Element(name) => {
                param_names.insert(name.clone());
            }
            PrimitiveParam::Data(name) => {
                // Data parameters like $gl are also valid references
                param_names.insert(name.clone());
            }
            PrimitiveParam::TypedData { name, .. } => {
                // Typed data parameters like $t: number
                param_names.insert(name.clone());
            }
            PrimitiveParam::Typed { name, .. } => {
                param_names.insert(name.clone());
            }
        }
    }

    // Collect declared export names
    let mut export_names: HashSet<String> = HashSet::new();
    for export in &primitive.body.exports {
        // Check for duplicate exports
        if export_names.contains(&export.name) {
            errors.push(ValidationError::duplicate_export(
                export.name.clone(),
                primitive.span,
            ));
        }
        export_names.insert(export.name.clone());

        // E0198: Check for legacy function types (bare fn)
        if let ExportTypeExpr::LegacyFn(_) = &export.type_expr {
            errors.push(ValidationError::legacy_function_type(
                export.name.clone(),
                primitive.span,
            ));
        }

        // E0199: Check that type references are valid
        validate_export_type(
            &export.type_expr,
            &export.name,
            &primitive.span,
            &mut errors,
        );
    }

    // Collect yield targets from emit blocks
    let mut yield_targets: HashSet<String> = HashSet::new();
    for emit in &primitive.body.emit_blocks {
        // Find variable references like %param, %&el, $var
        validate_emit_content(&emit.content, &param_names, &emit.span, &mut errors);

        // Collect yield targets using the tokenizer
        collect_yield_targets(&emit.content, &mut yield_targets);
    }

    // E0196: Check that all yield targets are declared in exports
    for target in &yield_targets {
        if !export_names.contains(target) {
            errors.push(ValidationError::yield_to_undeclared_export(
                target.clone(),
                primitive.span,
            ));
        }
    }

    // E0197: Check that all exports are yielded (except optional ones)
    for export in &primitive.body.exports {
        // Skip optional exports - they don't need to be yielded
        if export.optional {
            continue;
        }
        // Skip function exports - they're typically set once, not yielded dynamically
        if matches!(
            &export.type_expr,
            ExportTypeExpr::Function(_) | ExportTypeExpr::LegacyFn(_)
        ) {
            continue;
        }
        if !yield_targets.contains(&export.name) {
            errors.push(ValidationError::export_never_yielded(
                export.name.clone(),
                primitive.span,
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Collect yield targets from emit content
fn collect_yield_targets(content: &str, targets: &mut HashSet<String>) {
    let segments = tokenize_emit(content);
    for segment in segments {
        if let EmitSegment::Yield { signal, .. } = segment
            && let Some(name) = signal
        {
            targets.insert(name);
        }
        // Note: signal_param (dynamic signal names like $%name) are not tracked
        // since they're resolved at expansion time
    }
}

/// Validate type references in an export type expression
fn validate_export_type(
    type_expr: &ExportTypeExpr,
    export_name: &str,
    span: &SourceSpan,
    errors: &mut Vec<ValidationError>,
) {
    match type_expr {
        ExportTypeExpr::Simple(type_name) => {
            // Extract base type name (handle generics like array<T>)
            let base_type = extract_base_type(type_name);
            if !is_builtin_type(&base_type) {
                errors.push(ValidationError::unknown_type_reference(
                    type_name.clone(),
                    *span,
                ));
            }
        }
        ExportTypeExpr::Array(element_type) => {
            // Check the element type
            let base_type = extract_base_type(element_type);
            if !is_builtin_type(&base_type) {
                errors.push(ValidationError::unknown_type_reference(
                    element_type.clone(),
                    *span,
                ));
            }
        }
        ExportTypeExpr::Function(func_type) => {
            // Check return type
            let return_base = extract_base_type(&func_type.return_type);
            if !is_builtin_type(&return_base) {
                errors.push(ValidationError::unknown_type_reference(
                    func_type.return_type.clone(),
                    *span,
                ));
            }
            // Check parameter types
            for param in &func_type.params {
                let param_base = extract_base_type(&param.param_type);
                if !is_builtin_type(&param_base) {
                    errors.push(ValidationError::unknown_type_reference(
                        param.param_type.clone(),
                        *span,
                    ));
                }
            }
        }
        ExportTypeExpr::LegacyFn(_) => {
            // Already handled by E0198 check above
        }
        ExportTypeExpr::Union(_variants) => {
            // Union type variants are valid - they represent state names, not type references
            // No type validation needed as variant names are user-defined identifiers
        }
    }
    // Suppress unused variable warning
    let _ = export_name;
}

/// Extract base type name from a potentially generic type (e.g., "array<T>" -> "array")
fn extract_base_type(type_name: &str) -> String {
    if let Some(pos) = type_name.find('<') {
        type_name[..pos].to_string()
    } else {
        type_name.to_string()
    }
}

/// Check if a type name is a built-in type
fn is_builtin_type(type_name: &str) -> bool {
    BUILTIN_TYPES.contains(&type_name)
}

/// Validate an emit block's content for variable references
fn validate_emit_content(
    content: &str,
    params: &HashSet<String>,
    span: &SourceSpan,
    errors: &mut Vec<ValidationError>,
) {
    // Find %param and %&el references
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            // Check for %&el (element reference) or %param
            let mut name = String::new();
            let is_element = chars.peek() == Some(&'&');
            if is_element {
                chars.next();
            }

            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    name.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            if !name.is_empty() && !is_special_directive(&name) {
                // Check if this parameter exists
                if !params.contains(&name) {
                    errors.push(ValidationError::unbound_emit_variable(name, *span));
                }
            }
        }
    }
}

/// Check if a name is a special directive (yield, etc.)
fn is_special_directive(name: &str) -> bool {
    matches!(name, "yield" | "cleanup" | "export" | "emit")
}
/// Validate a %migration capsule (PLAN-079):
/// 1. %date is ISO YYYY-MM-DD (string compare == chronological compare).
/// 2. At least one embedded %macro (the retired definition), each with a
///    %form naming a directive; embedded names unique per capsule (2b).
/// 3. %rewrite rules: unique ids, directive ∈ embedded, non-empty %into,
///    template holes ⊆ %match captures, no self-cycle.
/// 4. %hint directives ∈ embedded.
/// 5. Coverage: every embedded directive has at least one %rewrite rule OR
///    a %hint — a retirement must tell users what to do.
/// 6. No shadowing: a live (non-retired) macro may not claim a directive a
///    registered migration owns, and vice versa.
/// Cross-migration chains (older templates may only reference NEWER waves'
/// directives; same-wave same-shape overlaps rejected) are checked post-load
/// by `validate_migration_chains`.
pub fn validate_migration(
    def: &MigrationDefAst,
    registry: &MetaRegistry,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // 1. %date shape: YYYY-MM-DD, month 01-12, day 01-31. Kept as a string
    // (lexicographic == chronological); validity checked by shape, not a
    // calendar crate.
    let date_ok = crate::migrate::is_iso_wave_date(&def.date);
    if !date_ok {
        errors.push(ValidationError {
            kind: ValidationErrorKind::InvalidMigrationDate(format!(
                "migration `{}`: %date `{}` is not ISO YYYY-MM-DD",
                def.id, def.date
            )),
            span: def.span,
        });
    }

    // 2. The capsule must embed ≥1 retired macro, each with a %form naming a
    // directive (PLAN-079: the migration owns the old grammar).
    if def.macros.is_empty() {
        errors.push(ValidationError {
            kind: ValidationErrorKind::InvalidMigration(format!(
                "migration `{}`: at least one embedded %macro (the retired definition) is required",
                def.id
            )),
            span: def.span,
        });
    }
    // 2b. Embedded macro names must be unique within the capsule — the
    // registration key is `<mig-id>#@<name>`, so a duplicate would fail
    // registration mid-loop, leaving the registry half-populated.
    {
        let mut seen = std::collections::HashSet::new();
        for mac in &def.macros {
            if !seen.insert(mac.name.clone()) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidMigration(format!(
                        "migration `{}`: duplicate embedded %macro `{}` — one retired \
                         definition per name (merge the forms into one %macro)",
                        def.id, mac.name
                    )),
                    span: mac.span,
                });
            }
        }
    }
    let embedded = def.retired_directives();
    for mac in &def.macros {
        let directive = mac
            .form
            .as_ref()
            .map(|f| f.directive_name.trim_start_matches('@'))
            .unwrap_or("");
        if directive.is_empty() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}`: embedded %macro `{}` needs a %form naming a directive (e.g. @bind)",
                    def.id, mac.name
                )),
                span: mac.span,
            });
        }
    }

    // 3. Rewrite rules.
    let mut rule_ids: HashSet<&str> = HashSet::new();
    for rule in &def.rewrites {
        if rule.id.is_empty() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}`: a %rewrite rule needs an id (`%rewrite bind-text {{ … }}`)",
                    def.id
                )),
                span: rule.span,
            });
        } else if !rule_ids.insert(rule.id.as_str()) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}`: duplicate %rewrite rule id `{}`",
                    def.id, rule.id
                )),
                span: rule.span,
            });
        }
        let directive = rule.match_form.directive_name.trim_start_matches('@');
        if directive.is_empty() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}` rule `{}`: %match must name a directive (e.g. @bind)",
                    def.id, rule.id
                )),
                span: rule.span,
            });
        } else if !embedded.iter().any(|d| d == directive) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}` rule `{}`: %match directive @{} is not retired by any embedded %macro",
                    def.id, rule.id, directive
                )),
                span: rule.span,
            });
        }
        if rule.template.trim().is_empty() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}` rule `{}`: %into template is empty",
                    def.id, rule.id
                )),
                span: rule.span,
            });
        }
        // Holes ⊆ the rule's %match captures.
        let captures = collect_form_capture_names(&rule.match_form);
        let spliced: HashSet<String> = extract_rewrite_holes(&rule.template)
            .into_iter()
            .collect();
        // `%drops` may name an EXTRA arg (one the old shape accepted but this
        // rule never binds) or a %match capture the rule deliberately does NOT
        // splice into %into — a DECLARED drop (I4 / gh-18: the lint reads it
        // as consumption-by-declaration). The one thing it must never name is a
        // capture the rule BOTH splices and drops: that is consumed AND dropped,
        // a confused rule. `_` (positional groups) is exempt.
        for drop in &rule.drops {
            if drop != "_" && captures.contains(drop) && spliced.contains(drop) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidMigration(format!(
                        "migration `{}` rule `{}`: %drops names `{}`, which is also spliced by %into — a capture is consumed or dropped, never both",
                        def.id, rule.id, drop
                    )),
                    span: rule.span,
                });
            }
        }
        for hole in extract_rewrite_holes(&rule.template) {
            if !captures.contains(&hole) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidMigration(format!(
                        "migration `{}` rule `{}`: template hole `${}` is not a %match capture                          (captures: {})",
                        def.id,
                        rule.id,
                        hole,
                        captures.iter().cloned().collect::<Vec<_>>().join(", ")
                    )),
                    span: rule.span,
                });
            } else if unsafe_optional_hole(&rule.match_form, &hole) {
                // An OPTIONAL capture (`$x:t?`) with NO default produces NO
                // value when absent — the template then renders the hole
                // VERBATIM into migrated source, where it silently compiles
                // through as junk (measured: `@loop x(600ms)` → `mode:
                // `$mode``). The behavior-exact fill is the retired macro's
                // default — declare it on the %match capture.
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidMigration(format!(
                        "migration `{}` rule `{}`: template splices optional capture `${}?` which has no default — it renders an unresolved hole when absent; give it the retired macro's default",
                        def.id, rule.id, hole
                    )),
                    span: rule.span,
                });
            }
        }
        // Self-cycle: the template must not reference its OWN directive —
        // UNLESS the template continues the directive with a non-ident token
        // (a partial retirement: `@on hover …` → `@on &.hover …`). The output
        // can never re-match the %match's literal-led form, so no cycle is
        // possible; the shim also applies rules ONCE to matches collected
        // upfront, never re-scanning rewritten output.
        if !directive.is_empty()
            && template_references_directive(&rule.template, directive)
            && !template_continues_with_non_ident(&rule.template, directive)
        {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}` rule `{}`: template references its own directive @{} (cycle)",
                    def.id, rule.id, directive
                )),
                span: rule.span,
            });
        }
    }

    // 4. Hints name an embedded directive.
    for hint in &def.hints {
        if !embedded.iter().any(|d| d == &hint.directive) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}`: %hint for @{} names no embedded %macro's directive",
                    def.id, hint.directive
                )),
                span: hint.span,
            });
        }
    }

    // 5. Coverage: every embedded directive has ≥1 rule OR a hint.
    for directive in &embedded {
        if def.rules_for(directive).is_empty() && def.hint_for(directive).is_none() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}`: retired @{} has neither a %rewrite rule nor a %hint —                      a retirement must tell users what to do",
                    def.id, directive
                )),
                span: def.span,
            });
        }
    }

    // 6. Shadow guard: a live macro must not already claim an embedded
    // directive (a migration is only valid for RETIRED syntax) — unless the
    // migration is a PARTIAL retirement: some of the directive's forms
    // retire while the directive itself lives on with new shapes. Marked by
    // a same-directive rule template with a non-ident continuation (see the
    // self-cycle check); anything else is a genuine shadow.
    let partial: std::collections::HashSet<&str> = def
        .rewrites
        .iter()
        .filter(|r| {
            template_references_directive(
                &r.template,
                r.match_form.directive_name.trim_start_matches('@'),
            ) && template_continues_with_non_ident(
                &r.template,
                r.match_form.directive_name.trim_start_matches('@'),
            )
        })
        .map(|r| r.match_form.directive_name.trim_start_matches('@'))
        .collect();
    for directive in &embedded {
        if partial.contains(directive.as_str()) {
            continue;
        }
        if let Some(existing) = registry.get_macro_by_form_directive(directive)
            && existing.retired.is_none()
        {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidMigration(format!(
                    "migration `{}`: directive @{} is still owned by live macro `{}` —                      a migration is only valid for RETIRED syntax",
                    def.id, directive, existing.name
                )),
                span: def.span,
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// True when the template references `@directive` as a whole word
/// (substring match whose next char is not an identifier char). Multi-word
/// directives (`"on hover"` for `@on hover`) match verbatim.
/// True when the template's directive reference is followed by a token the
/// %match's first inline literal can never begin with (`&`, `$`) — the
/// continuation can never re-match, so a same-directive template cannot
/// cycle. `(`/`{`/ident continuations CAN re-match (`@bind(text: …)` rewrites
/// to the same shape — a real cycle) and stay rejected. This is the marker
/// of a PARTIAL retirement: the directive lives on with a new shape while
/// some of its forms retire.
fn template_continues_with_non_ident(template: &str, directive: &str) -> bool {
    let needle = format!("@{}", directive.trim_start_matches('@'));
    let mut from = 0;
    let mut found = false;
    while let Some(idx) = template[from..].find(&needle) {
        let after = from + idx + needle.len();
        let next = template[after..].chars().find(|c| !c.is_whitespace());
        match next {
            // EVERY reference must continue with `&`/`$` (can't re-match a
            // literal-led %match). Exempting on the FIRST reference while a
            // LATER one still re-matches would admit a real self-cycle.
            Some(c) if matches!(c, '&' | '$') => found = true,
            _ => return false,
        }
        from = after;
    }
    found
}

fn template_references_directive(template: &str, directive: &str) -> bool {
    let needle = format!("@{}", directive.trim_start_matches('@'));
    let mut from = 0;
    while let Some(idx) = template[from..].find(&needle) {
        let after = from + idx + needle.len();
        let boundary = template
            .as_bytes()
            .get(after)
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_'));
        if boundary {
            return true;
        }
        from += idx + 1;
    }
    false
}

/// Post-load migration validation (PLAN-076/PLAN-079) — ORDER-INDEPENDENT.
/// Runs after every stdlib migration is registered (per-registration load
/// order cannot decide cross-migration facts):
///
/// 1. FORWARD-ONLY CHAINS: if migration A's rewrite template references a
///    directive retired by migration B, B's wave must be STRICTLY NEWER than
///    A's — chains point forward in time (A>B>C), so the apply loop
///    terminates.
/// 2. INTRA-WAVE OVERLAP: two rewrite RULES in the SAME wave matching the
///    same directive with the SAME param-name set would race over the same
///    source spans — a load error. (Same wave + same directive + DIFFERENT
///    param sets is the normal case: e.g. one wave retiring `@bind(text:)`
///    AND `@bind(attr:)` — disjoint shapes, both applied by reverse-order
///    splicing.)
pub fn validate_migration_chains(registry: &MetaRegistry) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let all: Vec<&MigrationDefAst> = registry.all_migrations().collect();

    for mig in &all {
        for rule in &mig.rewrites {
            for other in &all {
                if other.id == mig.id {
                    continue;
                }
                for other_directive in other.retired_directives() {
                    if template_references_directive(&rule.template, &other_directive)
                        && other.date.as_str() <= mig.date.as_str()
                    {
                        errors.push(ValidationError {
                            kind: ValidationErrorKind::InvalidMigration(format!(
                                "migration `{}` rule `{}` ({}): template references @{} owned by older-or-equal                                  migration `{}` ({}) — chains must point FORWARD in time",
                                mig.id, rule.id, mig.date, other_directive, other.id, other.date
                            )),
                            span: rule.span,
                        });
                    }
                }
            }
        }
    }

    // Intra-wave overlap: (date, directive, sorted param names) must be
    // unique across ALL rules of ALL migrations in the wave. INLINE LITERALS
    // discriminate too (`@on hover …` vs `@on visible …` — the event word
    // IS the shape; the on-cutover's per-event rules).
    let mut seen: std::collections::HashMap<
        (String, String, Vec<String>, Vec<String>),
        (String, String),
    > = std::collections::HashMap::new();
    for mig in &all {
        for rule in &mig.rewrites {
            let mut params: Vec<String> = rule
                .match_form
                .params
                .iter()
                .map(|p| p.name.clone())
                .collect();
            params.sort();
            let literals: Vec<String> = rule
                .match_form
                .inline_elements
                .iter()
                .filter_map(|e| match e {
                    crate::parser::meta_ast::FormInlineElement::Literal(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            // Order MATTERS: `@on hover visible` and `@on visible hover` are
            // disjoint match shapes — sorting would conflate them into one
            // overlap key and reject valid same-wave rules.
            let key = (
                mig.date.clone(),
                rule.match_form
                    .directive_name
                    .trim_start_matches('@')
                    .to_string(),
                params,
                literals,
            );
            if let Some((first_mig, first_rule)) =
                seen.insert(key, (mig.id.clone(), rule.id.clone()))
            {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidMigration(format!(
                        "rules `{}`/`{}` and `{}`/`{}` share wave {} and match the same @{} shape —                          same-wave rules must have DISJOINT %match shapes",
                        first_mig,
                        first_rule,
                        mig.id,
                        rule.id,
                        mig.date,
                        rule.match_form.directive_name.trim_start_matches('@')
                    )),
                    span: rule.span,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Capture names a `%match` form binds: `$var` names from inline captures
/// (including `Comparison` captures and `as $alias` alias captures, both
/// recursed), named params (recursing into block/pseudo body_params),
/// post-arg inline captures, and the body capture
/// (`"$body:component_body"` -> `"body"`).
///
/// True when `hole` names a %match capture that is OPTIONAL (`?`) with NO
/// default (element-level or enclosing-param-level): splicing it renders the
/// literal hole text when absent. Required captures always bind (or the rule
/// doesn't match); `*`/`+` bind arrays; defaults always produce a value.
fn unsafe_optional_hole(form: &FormClause, hole: &str) -> bool {
    fn check(cap: &FormCapture, has_default: bool, hole: &str) -> bool {
        let bare = cap.var_name.trim_start_matches(['$', '&', '{', ' ']);
        if bare == hole {
            return matches!(cap.modifier, CaptureModifier::Optional) && !has_default;
        }
        cap.alias_capture
            .as_ref()
            .is_some_and(|a| check(a, has_default, hole))
    }
    fn walk(elements: &[FormInlineElement], param_default: bool, hole: &str) -> bool {
        elements.iter().any(|el| match el {
            FormInlineElement::Capture(cap, el_default) => {
                check(cap, param_default || el_default.is_some(), hole)
            }
            FormInlineElement::Comparison { capture, .. } => check(capture, false, hole),
            FormInlineElement::KeywordBlock { body_params, .. }
            | FormInlineElement::PseudoSelector { body_params, .. }
            | FormInlineElement::PseudoClass { body_params, .. } => body_params
                .iter()
                .any(|p| walk(&p.elements, p.default.is_some(), hole)),
            _ => false,
        })
    }
    walk(&form.inline_elements, false, hole)
        || form
            .params
            .iter()
            .any(|p| walk(&p.elements, p.default.is_some(), hole))
        || form
            .body_params
            .iter()
            .any(|p| walk(&p.elements, p.default.is_some(), hole))
}

/// LIMITATION (documented, P3): `body_groups` (FEAT-103 leading body-group
/// PEG patterns) bind names too, but extracting them needs the
/// CapturePatternAst walker; retirement-shaped forms don't use body groups.
fn collect_form_capture_names(form: &FormClause) -> HashSet<String> {
    fn insert_capture(cap: &FormCapture, out: &mut HashSet<String>) {
        // var_name keeps its sigil for BODY captures (`$body`) but not inline
        // ones (`scope`) — holes are written bare, so compare bare.
        out.insert(
            cap.var_name
                .trim_start_matches(['$', '&', '{', ' '])
                .to_string(),
        );
        if let Some(alias) = &cap.alias_capture {
            insert_capture(alias, out);
        }
    }
    fn from_elements(elements: &[FormInlineElement], out: &mut HashSet<String>) {
        for element in elements {
            match element {
                FormInlineElement::Capture(cap, _) => insert_capture(cap, out),
                FormInlineElement::Comparison { capture, .. } => insert_capture(capture, out),
                FormInlineElement::KeywordBlock { body_params, .. }
                | FormInlineElement::PseudoSelector { body_params, .. }
                | FormInlineElement::PseudoClass { body_params, .. } => {
                    for param in body_params {
                        from_elements(&param.elements, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = HashSet::new();
    from_elements(&form.inline_elements, &mut out);
    from_elements(&form.post_arg_inline, &mut out);
    for param in &form.params {
        from_elements(&param.elements, &mut out);
    }
    for param in &form.body_params {
        from_elements(&param.elements, &mut out);
    }
    if let Some(body_capture) = &form.body_capture {
        // A %match body capture carries its block verbatim — brace, newlines,
        // whitespace (`"{\n  $body:keyframes\n}"`). Strip the brace and the
        // sigil so the hole check compares NAMES.
        let name = body_capture
            .trim()
            .trim_start_matches('{')
            .trim()
            .trim_start_matches('$')
            .split(':')
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() {
            out.insert(name.to_string());
        }
    }
    out
}

/// Hole names referenced by a `%rewrite` template: `` `$name` `` — the
/// backtick-quoted hole form. A bare `$name` in the template is LITERAL
/// output text and is NOT collected.
fn extract_rewrite_holes(template: &str) -> Vec<String> {
    let mut holes = Vec::new();
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
            let start = i + 2;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            if end > start && end < bytes.len() && bytes[end] == b'`' {
                holes.push(template[start..end].to_string());
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    holes
}

/// Validate a macro definition
pub fn validate_macro(
    macro_def: &MacroDefAst,
    registry: &MetaRegistry,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Collect bound variables from %binds and %form
    let mut bound_vars: HashSet<String> = HashSet::new();

    // Variables from form captures
    if let Some(form) = &macro_def.form {
        for param in &form.params {
            // For multi-element params, collect all captures
            for element in &param.elements {
                if let FormInlineElement::Capture(capture, _) = element {
                    bound_vars.insert(capture.var_name.clone());
                }
            }
        }
        if let Some(body_capture) = &form.body_capture {
            bound_vars.insert(body_capture.clone());
        }

        // Check: optional inline captures must have explicit defaults
        for element in &form.inline_elements {
            if let FormInlineElement::Capture(cap, default) = element
                && cap.modifier == CaptureModifier::Optional
                && default.is_none()
            {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::OptionalCaptureWithoutDefault {
                        macro_name: macro_def.name.clone(),
                        capture_name: cap.var_name.clone(),
                    },
                    span: form.span,
                });
            }
        }
        for param in &form.params {
            if param.default.is_some() {
                continue;
            } // param-level default covers all captures
            for element in &param.elements {
                if let FormInlineElement::Capture(cap, default) = element
                    && cap.modifier == CaptureModifier::Optional
                    && default.is_none()
                {
                    errors.push(ValidationError {
                        kind: ValidationErrorKind::OptionalCaptureWithoutDefault {
                            macro_name: macro_def.name.clone(),
                            capture_name: cap.var_name.clone(),
                        },
                        span: form.span,
                    });
                }
            }
        }
    }

    // Variables from binds outputs
    for bind in &macro_def.binds {
        // Check that the primitive exists
        if !registry.has_primitive(&bind.primitive) {
            errors.push(ValidationError::unknown_primitive(
                bind.primitive.clone(),
                bind.span,
            ));
        }

        for output in &bind.outputs {
            bound_vars.insert(output.name.clone());
        }
    }

    // Variables from derives
    for derive in &macro_def.derives {
        bound_vars.insert(derive.name.clone());
    }

    // Check for circular dependencies via %includes
    let mut include_stack = vec![macro_def.name.clone()];
    check_circular_includes(macro_def, registry, &mut include_stack, &mut errors);

    // Validate macro body items
    validate_macro_body(
        &macro_def.body,
        &bound_vars,
        registry,
        &macro_def.span,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check for circular %includes dependencies
fn check_circular_includes(
    macro_def: &MacroDefAst,
    registry: &MetaRegistry,
    stack: &mut Vec<String>,
    errors: &mut Vec<ValidationError>,
) {
    for item in &macro_def.body {
        if let MacroBodyItem::Includes(includes) = item {
            for pattern in &includes.patterns {
                if stack.contains(&pattern.name) {
                    let mut cycle = stack.clone();
                    cycle.push(pattern.name.clone());
                    errors.push(ValidationError::circular_dependency(cycle, includes.span));
                    continue;
                }

                if let Some(included_macro) = registry.get_macro(&pattern.name) {
                    stack.push(pattern.name.clone());
                    check_circular_includes(included_macro, registry, stack, errors);
                    stack.pop();
                }
            }
        }
    }
}

/// Validate macro body items
fn validate_macro_body(
    body: &[MacroBodyItem],
    bound_vars: &HashSet<String>,
    registry: &MetaRegistry,
    span: &SourceSpan,
    errors: &mut Vec<ValidationError>,
) {
    for item in body {
        match item {
            MacroBodyItem::When(when) => {
                // Check condition variable is bound
                if !bound_vars.contains(&when.condition) {
                    errors.push(ValidationError::unbound_macro_variable(
                        when.condition.clone(),
                        when.span,
                    ));
                }
                validate_macro_body(&when.body, bound_vars, registry, span, errors);
            }
            MacroBodyItem::On(on_clause) => {
                validate_on_trigger(&on_clause.trigger, bound_vars, &on_clause.span, errors);
            }
            MacroBodyItem::For(for_clause) => {
                // Check source variable is bound
                let source_var = &for_clause.source;
                if !bound_vars.contains(source_var) {
                    errors.push(ValidationError::unbound_macro_variable(
                        source_var.clone(),
                        for_clause.span,
                    ));
                }

                // Add loop variable to bound vars for body
                let mut inner_vars = bound_vars.clone();
                inner_vars.insert(for_clause.variable.clone());
                validate_macro_body(&for_clause.body, &inner_vars, registry, span, errors);
            }
            MacroBodyItem::If(if_clause) => {
                validate_if_condition(&if_clause.condition, bound_vars, &if_clause.span, errors);
                validate_macro_body(&if_clause.then_body, bound_vars, registry, span, errors);
                for elif_clause in &if_clause.elif_clauses {
                    validate_if_condition(
                        &elif_clause.condition,
                        bound_vars,
                        &elif_clause.span,
                        errors,
                    );
                    validate_macro_body(&elif_clause.body, bound_vars, registry, span, errors);
                }
                if let Some(else_body) = &if_clause.else_body {
                    validate_macro_body(else_body, bound_vars, registry, span, errors);
                }
            }
            MacroBodyItem::Includes(includes) => {
                for pattern in &includes.patterns {
                    if !registry.has_macro(&pattern.name) {
                        errors.push(ValidationError::unknown_macro(
                            pattern.name.clone(),
                            includes.span,
                        ));
                    }
                }
            }
            MacroBodyItem::Animates(animates) => {
                for prop in &animates.properties {
                    if !bound_vars.contains(&prop.variable) {
                        errors.push(ValidationError::unbound_macro_variable(
                            prop.variable.clone(),
                            animates.span,
                        ));
                    }
                }
            }
            MacroBodyItem::Applies(applies) => {
                for var in &applies.variables {
                    if !bound_vars.contains(var) {
                        errors.push(ValidationError::unbound_macro_variable(
                            var.clone(),
                            applies.span,
                        ));
                    }
                }
            }
            MacroBodyItem::Mutate(_) | MacroBodyItem::Trigger(_) | MacroBodyItem::Binds(_) => {
                // These are validated separately or don't need variable checks here
            }
            MacroBodyItem::Emit(emit_block) => {
                // Validate that parameter references in emit content exist
                // For now, basic validation - could parse %$var patterns and check them
                let _ = emit_block; // Used in runtime expansion
            }
        }
    }
}

/// Validate an %on trigger
fn validate_on_trigger(
    trigger: &MetaOnTrigger,
    bound_vars: &HashSet<String>,
    span: &SourceSpan,
    errors: &mut Vec<ValidationError>,
) {
    match trigger {
        MetaOnTrigger::VarTransition { var, .. } | MetaOnTrigger::VarEvent { var, .. } => {
            if !bound_vars.contains(var) {
                errors.push(ValidationError::unbound_macro_variable(var.clone(), *span));
            }
        }
        MetaOnTrigger::Event(_) => {
            // Event names don't need to be bound
        }
    }
}

/// Validate an %if condition
fn validate_if_condition(
    condition: &MetaIfCondition,
    bound_vars: &HashSet<String>,
    span: &SourceSpan,
    errors: &mut Vec<ValidationError>,
) {
    match condition {
        MetaIfCondition::Truthy(v) | MetaIfCondition::Falsy(v) => {
            if !bound_vars.contains(v) {
                errors.push(ValidationError::unbound_macro_variable(v.clone(), *span));
            }
        }
        MetaIfCondition::Equals(v, _)
        | MetaIfCondition::NotEquals(v, _)
        | MetaIfCondition::LessThan(v, _)
        | MetaIfCondition::GreaterThan(v, _)
        | MetaIfCondition::LessThanOrEqual(v, _)
        | MetaIfCondition::GreaterThanOrEqual(v, _) => {
            if !bound_vars.contains(v) {
                errors.push(ValidationError::unbound_macro_variable(v.clone(), *span));
            }
        }
        MetaIfCondition::Or(left, right) | MetaIfCondition::And(left, right) => {
            validate_if_condition(left, bound_vars, span, errors);
            validate_if_condition(right, bound_vars, span, errors);
        }
    }
}

// =============================================================================
// FormMatch Validation (requires lsp or wasm feature)
// =============================================================================

#[cfg(any(feature = "lsp", feature = "wasm"))]
#[allow(dead_code)]
/// Format a CaptureType for error messages.
fn format_capture_type(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Ident => "identifier".to_string(),
        CaptureType::DashedIdent => "dashed identifier".to_string(),
        CaptureType::EventName => "dashed identifier".to_string(),
        CaptureType::String => "string".to_string(),
        CaptureType::Number => "number".to_string(),
        CaptureType::Bool => "boolean".to_string(),
        CaptureType::Time => "time".to_string(),
        CaptureType::Length => "length".to_string(),
        CaptureType::Duration => "duration".to_string(),
        CaptureType::Easing => "easing".to_string(),
        CaptureType::Typeref => "type reference".to_string(),
        CaptureType::Binding => "binding".to_string(),
        CaptureType::Event => "event".to_string(),
        CaptureType::Expr => "expression".to_string(),
        CaptureType::Properties => "properties block".to_string(),
        CaptureType::Fields => "fields block".to_string(),
        CaptureType::Params => "parameters".to_string(),
        CaptureType::States => "states block".to_string(),
        CaptureType::Transitions => "transitions block".to_string(),
        CaptureType::Keyframes => "keyframes block".to_string(),
        CaptureType::Selector => "selector".to_string(),
        CaptureType::Element => "element".to_string(),
        CaptureType::Preset => "preset".to_string(),
        CaptureType::MutationActions => "mutation actions".to_string(),
        CaptureType::Template => "template".to_string(),
        CaptureType::ParamList => "parameter list".to_string(),
        CaptureType::HtmlBlock => "HTML block".to_string(),
        CaptureType::JsBlock => "JavaScript block".to_string(),
        CaptureType::ComponentBody => "component body".to_string(),
        CaptureType::TemplateInvocation => "template invocation".to_string(),
        CaptureType::Union(variants) => {
            let quoted: Vec<String> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
            format!("one of {}", quoted.join(", "))
        }
        CaptureType::PatternMatch { variant, bindings } => {
            if bindings.is_empty() {
                format!("pattern match (is {})", variant)
            } else {
                format!(
                    "pattern match (is {} {{ {} }})",
                    variant,
                    bindings.join(", ")
                )
            }
        }
        CaptureType::Custom(name) => format!("custom type '{}'", name),
        CaptureType::Balanced(d) => format!("balanced run until '{}'", d),
        CaptureType::SkipBlock => "skipped block".to_string(),
        CaptureType::Color => "color".to_string(),
    }
}

#[cfg(any(feature = "lsp", feature = "wasm"))]
/// Validate all FormMatches in scopes.
///
/// FormMatches are already validated by the form matching process, but we still
/// need to check that the directive name is recognized by the registry for LSP
/// diagnostics (e.g., unknown directives).
pub fn validate_scope_form_matches(
    matches: &[crate::syntax::FormMatch],
    registry: &FormRegistry,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for fm in matches {
        // FormMatches were already matched by the event parser, so we know
        // the syntax is valid. We only need to check that the directive is
        // recognized by the form registry (for LSP "unknown directive" warnings).
        if registry.get_directive(&fm.macro_name).is_none() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::UnknownDirective(fm.macro_name.clone()),
                span: fm.span,
            });
        }
    }

    errors
}
#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> MetaRegistry {
        MetaRegistry::new()
    }

    #[test]
    fn test_validate_primitive_duplicate_export() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![
                    ExportDecl {
                        name: "x".to_string(),
                        type_expr: ExportTypeExpr::Simple("number".to_string()),
                        optional: false,
                    },
                    ExportDecl {
                        name: "x".to_string(),
                        type_expr: ExportTypeExpr::Simple("number".to_string()),
                        optional: false,
                    },
                ],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::DuplicateExport(_)
        ));
    }

    #[test]
    fn test_validate_macro_unknown_primitive_in_binds() {
        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: None,
            binds: vec![BindDecl {
                primitive: "nonexistent".to_string(),
                args: vec![],
                outputs: vec![],
                span: SourceSpan::default(),
            }],
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
        };

        let result = validate_macro(&macro_def, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::UnknownPrimitive(_)
        ));
    }

    #[test]
    fn test_validate_macro_unbound_variable() {
        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: None,
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
            body: vec![MacroBodyItem::When(WhenClause {
                condition: "unbound".to_string(),
                body: vec![],
                span: SourceSpan::default(),
            })],
            requires: vec![],
            span: SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        let result = validate_macro(&macro_def, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0].kind,
            ValidationErrorKind::UnboundMacroVariable(_)
        ));
    }

    // =========================================================================
    // E0196: Yield to undeclared export
    // =========================================================================

    #[test]
    fn test_e0196_yield_to_undeclared_export() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "%yield 42 -> $undeclared;".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "x".to_string(),
                    type_expr: ExportTypeExpr::Simple("number".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, ValidationErrorKind::YieldToUndeclaredExport(name) if name == "undeclared")));
    }

    #[test]
    fn test_e0196_yield_to_declared_export_ok() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "%yield 42 -> $x;".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "x".to_string(),
                    type_expr: ExportTypeExpr::Simple("number".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_ok());
    }

    // =========================================================================
    // E0197: Export never yielded
    // =========================================================================

    #[test]
    fn test_e0197_export_never_yielded() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "console.log('no yields');".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "orphan".to_string(),
                    type_expr: ExportTypeExpr::Simple("string".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(
            |e| matches!(&e.kind, ValidationErrorKind::ExportNeverYielded(name) if name == "orphan")
        ));
    }

    #[test]
    fn test_e0197_optional_export_not_required_to_yield() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "console.log('no yields');".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "maybeValue".to_string(),
                    type_expr: ExportTypeExpr::Simple("string".to_string()),
                    optional: true,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_ok());
    }

    #[test]
    fn test_e0197_function_export_not_required_to_yield() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "const start = () => {};".to_string(),
                    span: SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "start".to_string(),
                    type_expr: ExportTypeExpr::Function(FunctionTypeExpr {
                        params: vec![],
                        return_type: "effect".to_string(),
                    }),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_ok());
    }

    // =========================================================================
    // E0198: Legacy function type (bare fn)
    // =========================================================================

    #[test]
    fn test_e0198_legacy_function_type() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "start".to_string(),
                    type_expr: ExportTypeExpr::LegacyFn(None),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(
            |e| matches!(&e.kind, ValidationErrorKind::LegacyFunctionType(name) if name == "start")
        ));
    }

    // =========================================================================
    // E0199: Unknown type reference
    // =========================================================================

    #[test]
    fn test_e0199_unknown_type_reference() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "data".to_string(),
                    type_expr: ExportTypeExpr::Simple("FooBar".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(
            |e| matches!(&e.kind, ValidationErrorKind::UnknownTypeReference(ty) if ty == "FooBar")
        ));
    }

    #[test]
    fn test_e0199_builtin_types_ok() {
        for builtin in &[
            "number", "string", "bool", "effect", "void", "any", "array", "map", "optional",
            "element", "object",
        ] {
            let primitive = PrimitiveDefAst {
                name: "test".to_string(),
                params: vec![],
                body: PrimitiveBody {
                    emit_blocks: vec![EmitBlock {
                        lang: EmitLang::Js,
                        content: "%yield value -> $x;".to_string(),
                        span: SourceSpan::default(),
                    }],
                    cleanup: None,
                    exports: vec![ExportDecl {
                        name: "x".to_string(),
                        type_expr: ExportTypeExpr::Simple(builtin.to_string()),
                        optional: false,
                    }],
                    if_blocks: vec![],
                },
                uses: vec![],
                span: SourceSpan::default(),
                source_file: None,
                doc: None,
            };

            let result = validate_primitive(&primitive, &empty_registry());
            assert!(
                result.is_ok(),
                "Built-in type '{}' should be valid",
                builtin
            );
        }
    }

    #[test]
    fn test_e0199_unknown_type_in_function_return() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "getValue".to_string(),
                    type_expr: ExportTypeExpr::Function(FunctionTypeExpr {
                        params: vec![],
                        return_type: "UnknownType".to_string(),
                    }),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, ValidationErrorKind::UnknownTypeReference(ty) if ty == "UnknownType")));
    }

    #[test]
    fn test_helper_extract_base_type() {
        assert_eq!(extract_base_type("array<number>"), "array");
        assert_eq!(extract_base_type("map<string, number>"), "map");
        assert_eq!(extract_base_type("number"), "number");
        assert_eq!(extract_base_type("optional<string>"), "optional");
    }
}
