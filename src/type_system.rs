//! Type system for Spacetime data binding.
//!
//! Provides:
//! - Type registry building from `@type` definitions
//! - Type reference resolution
//! - Circular dependency detection
//! - JSON Schema generation
//! - Property path validation

use crate::diagnostics::{Diagnostic, DiagnosticCode, SourceSpan};
use crate::parser::{TypeDef, TypeExpr, TypeField};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

// =============================================================================
// Type Registry
// =============================================================================

/// Registry of all defined types in a Spacetime file.
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    types: HashMap<String, TypeDef>,
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create a new empty TypeRegistry
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    /// Register a product type with fields.
    pub fn register_type(&mut self, name: String, fields: Vec<TypeField>) {
        use crate::parser::SourceSpan;
        self.register_type_def(TypeDef {
            name,
            fields,
            variants: Vec::new(),
            span: SourceSpan::default(),
        });
    }

    /// Register a fully-formed TypeDef (product OR sum). PLAN-038 W0.
    pub fn register_type_def(&mut self, def: TypeDef) {
        self.types.insert(def.name.clone(), def);
    }

    /// Create a TypeRegistry from FormMatch instances
    pub fn from_form_matches(matches: &[crate::syntax::FormMatch]) -> Self {
        use crate::parser::SourceSpan;
        let mut registry = Self::new();
        for fm in matches.iter().filter(|m| m.macro_name == "type") {
            if let Some(name) = fm.get_ident("name") {
                // SUM body (PLAN-038 W0): the stdlib `variant_list` capture reifies
                // into the `variants` capture as an Array of Named `{ name, payload }`
                // records. A non-empty variants capture ⟺ a tagged-sum @type; it is
                // mutually exclusive with product `fields`.
                let variants = extract_variants(fm);
                let fields = if variants.is_empty() {
                    // Product body: typed fields from the `fields` capture.
                    fm.get_properties("fields")
                        .map(|props| {
                            props
                                .iter()
                                .map(|p| TypeField {
                                    name: p.name.clone(),
                                    optional: p.optional,
                                    type_expr: parse_type_ref_to_expr(&p.type_ref),
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                registry.register_type_def(TypeDef {
                    name: name.to_string(),
                    fields,
                    variants,
                    span: SourceSpan::default(),
                });
            }
        }

        // SCOPED SUM (PLAN-038 FUP-081 #5): a `@data signal`/`@data stream` whose
        // `receive to <Sum> { Ctor(… as T); … }` block NAMES a response sum defines
        // that sum from its arms — the arms ARE the variant definition (one
        // type-creation mechanism, two scopes §S1). This lets reference.st / the MCP
        // registry introspect the response interface. An explicit `@type <Sum>`
        // WINS (registered above); we never clobber it.
        for fm in matches.iter().filter(|m| m.macro_name == "data") {
            let Some(sum_name) = fm.get_ident("sum") else {
                continue;
            };
            if sum_name.is_empty() || registry.get_type(sum_name).is_some() {
                continue; // no sum named, or an explicit @type already owns it
            }
            if let Some(def) = sum_from_receive_arms(sum_name, fm.get("arms")) {
                registry.register_type_def(def);
            }
        }
        registry
    }

    /// Build a type registry from the AST.
    ///
    /// Validates:
    /// - No duplicate type names
    /// - All type references exist
    /// - No circular dependencies
    pub fn from_types(types: &[TypeDef]) -> Result<Self, Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        let mut type_map = HashMap::new();

        // First pass: collect all type names and check for duplicates
        for type_def in types {
            if type_map.contains_key(&type_def.name) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0101,
                        format!("Duplicate type definition '{}'", type_def.name),
                    )
                    .with_span(type_def.span.into()),
                );
            } else {
                type_map.insert(type_def.name.clone(), type_def.clone());
            }
        }

        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        let registry = TypeRegistry { types: type_map };

        // Second pass: validate all type references and check for circular dependencies
        for type_def in types {
            registry.validate_type_references(type_def, &mut diagnostics);
            registry.check_circular_dependency(&type_def.name, &mut diagnostics);
        }

        if !diagnostics.is_empty() {
            Err(diagnostics)
        } else {
            Ok(registry)
        }
    }

    /// Get a type definition by name.
    pub fn get_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Get all type names.
    pub fn type_names(&self) -> Vec<&str> {
        self.types.keys().map(|s| s.as_str()).collect()
    }

    /// Iterate over all registered types.
    pub fn iter_types(&self) -> impl Iterator<Item = (&String, &TypeDef)> {
        self.types.iter()
    }

    /// Variant names of a registered `@type` sum (e.g. `["Blue", "Orange", ...]`).
    ///
    /// Read-only query used by the enum/union validation passes (PLAN-077):
    /// dispatch-mode `@match $subject` / `@view $subject` / `@state(when: $x is V)`
    /// exhaustiveness (E0932) and unknown-variant checks (E0933) read the declared
    /// variant set from here. Returns an empty Vec for a missing type name OR a
    /// product (non-sum) type — the caller treats "no variant set" as "not an
    /// enum", never an error.
    pub fn variant_names(&self, type_name: &str) -> Vec<&str> {
        self.types
            .get(type_name)
            .map(|def| def.variants.iter().map(|v| v.name.as_str()).collect())
            .unwrap_or_default()
    }

    /// Payload arity of a single variant of a registered `@type` sum.
    ///
    /// `Created(Todo)` → `Some(1)`; `Pair(A, B)` → `Some(2)`; payload-less `Failed`
    /// → `Some(0)` (the variant EXISTS, just carries no payload). Returns `None`
    /// for a missing type name, a product type, OR a variant name that isn't
    /// declared on the (existing) sum — the caller distinguishes "not a sum /
    /// unknown variant" (None) from "payload-less variant" (Some(0)). Used by the
    /// payload-binding-mismatch check (E0934, PLAN-077) and the mutation
    /// lowering's variant-construction arity.
    pub fn variant_payload_arity(&self, type_name: &str, variant: &str) -> Option<usize> {
        self.types
            .get(type_name)?
            .variants
            .iter()
            .find(|v| v.name == variant)
            .map(|v| v.payload.len())
    }

    /// Resolve a type expression to a concrete type.
    pub fn resolve_type_expr(&self, expr: &TypeExpr) -> Result<ResolvedType, Diagnostic> {
        match expr {
            TypeExpr::Primitive(name) => Ok(ResolvedType::Primitive(name.clone())),
            TypeExpr::Array(inner) => {
                let resolved_inner = self.resolve_type_expr(inner)?;
                Ok(ResolvedType::Array(Box::new(resolved_inner)))
            }
            TypeExpr::Object(fields) => {
                let mut resolved_fields = Vec::new();
                for field in fields {
                    let resolved_type = self.resolve_type_expr(&field.type_expr)?;
                    resolved_fields.push(ResolvedTypeField {
                        name: field.name.clone(),
                        optional: field.optional,
                        type_expr: resolved_type,
                    });
                }
                Ok(ResolvedType::Object(resolved_fields))
            }
            TypeExpr::Union(variants) => Ok(ResolvedType::Union(variants.clone())),
            // A relation `id(T)` resolves to a plain string id at runtime.
            TypeExpr::Apply { .. } => Ok(ResolvedType::Primitive("string".to_string())),
            TypeExpr::Reference(name) => {
                // Resolve the reference
                if let Some(type_def) = self.get_type(name) {
                    // Create an object type from the type definition
                    let mut resolved_fields = Vec::new();
                    for field in &type_def.fields {
                        let resolved_type = self.resolve_type_expr(&field.type_expr)?;
                        resolved_fields.push(ResolvedTypeField {
                            name: field.name.clone(),
                            optional: field.optional,
                            type_expr: resolved_type,
                        });
                    }
                    Ok(ResolvedType::Object(resolved_fields))
                } else {
                    Err(Diagnostic::error(
                        DiagnosticCode::E0102,
                        format!("Unknown type reference '{}'", name),
                    ))
                }
            }
        }
    }

    /// Validate that all type references in a type definition exist.
    fn validate_type_references(&self, type_def: &TypeDef, diagnostics: &mut Vec<Diagnostic>) {
        for field in &type_def.fields {
            self.validate_type_expr(&field.type_expr, type_def.span.into(), diagnostics);
        }
    }

    /// Validate a type expression recursively.
    fn validate_type_expr(
        &self,
        expr: &TypeExpr,
        span: SourceSpan,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match expr {
            TypeExpr::Reference(name) => {
                if !self.types.contains_key(name) {
                    let hint = crate::diagnostics::find_similar(name, &self.type_names(), 2)
                        .map(|s| format!("Did you mean '{}'?", s));

                    let mut diag = Diagnostic::error(
                        DiagnosticCode::E0102,
                        format!("Unknown type reference '{}'", name),
                    )
                    .with_span(span);

                    if let Some(h) = hint {
                        diag = diag.with_hint(h);
                    }

                    diagnostics.push(diag);
                }
            }
            TypeExpr::Array(inner) => {
                self.validate_type_expr(inner, span, diagnostics);
            }
            TypeExpr::Object(fields) => {
                for field in fields {
                    self.validate_type_expr(&field.type_expr, span, diagnostics);
                }
            }
            // A relation `id(T)`: validate that the target type `T` exists, the
            // same way a bare reference is validated (an `id` of a non-existent
            // type is a real authoring error).
            TypeExpr::Apply { ctor, args } if ctor == "id" => {
                if let Some(arg) = args.first() {
                    self.validate_type_expr(arg, span, diagnostics);
                }
            }
            TypeExpr::Apply { .. } => {
                // Unknown constructor: validate its arguments structurally.
                if let TypeExpr::Apply { args, .. } = expr {
                    for a in args {
                        self.validate_type_expr(a, span, diagnostics);
                    }
                }
            }
            TypeExpr::Primitive(_) | TypeExpr::Union(_) => {
                // Nothing to validate
            }
        }
    }

    /// Check for circular type dependencies.
    fn check_circular_dependency(&self, type_name: &str, diagnostics: &mut Vec<Diagnostic>) {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if let Some(cycle) = self.find_cycle(type_name, &mut visited, &mut path) {
            let cycle_str = cycle.join(" -> ");
            if let Some(type_def) = self.types.get(type_name) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0101,
                        "Circular type dependency detected".to_string(),
                    )
                    .with_span(type_def.span.into())
                    .with_note(format!("Dependency cycle: {}", cycle_str))
                    .with_hint(
                        "Break the cycle by making one reference optional (?) or using string IDs"
                            .to_string(),
                    ),
                );
            }
        }
    }

    /// Find a cycle in type dependencies using DFS.
    fn find_cycle(
        &self,
        type_name: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if path.contains(&type_name.to_string()) {
            // Found a cycle
            let cycle_start = path.iter().position(|s| s == type_name).unwrap();
            let mut cycle = path[cycle_start..].to_vec();
            cycle.push(type_name.to_string());
            return Some(cycle);
        }

        if visited.contains(type_name) {
            return None;
        }

        visited.insert(type_name.to_string());
        path.push(type_name.to_string());

        if let Some(type_def) = self.types.get(type_name) {
            for field in &type_def.fields {
                if let Some(cycle) = self.find_cycle_in_expr(&field.type_expr, visited, path) {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        None
    }

    /// Find a cycle in a type expression.
    fn find_cycle_in_expr(
        &self,
        expr: &TypeExpr,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        match expr {
            TypeExpr::Reference(name) => self.find_cycle(name, visited, path),
            TypeExpr::Array(inner) => self.find_cycle_in_expr(inner, visited, path),
            TypeExpr::Object(fields) => {
                for field in fields {
                    if let Some(cycle) = self.find_cycle_in_expr(&field.type_expr, visited, path) {
                        return Some(cycle);
                    }
                }
                None
            }
            // `id(T)` is a REFERENCE by id, not an embedding, so it never forms a
            // structural cycle — `id(Self)` (self-relations) is legal precisely
            // because the id is a pointer, not nested data.
            TypeExpr::Apply { .. } => None,
            TypeExpr::Primitive(_) | TypeExpr::Union(_) => None,
        }
    }

    /// Validate a property path against a type.
    ///
    /// Returns the resolved type of the property, or an error if the path is invalid.
    pub fn validate_property_path(
        &self,
        type_expr: &TypeExpr,
        path: &[String],
        span: SourceSpan,
    ) -> Result<ResolvedType, Diagnostic> {
        if path.is_empty() {
            return self.resolve_type_expr(type_expr);
        }

        let resolved = self.resolve_type_expr(type_expr)?;
        self.validate_path_on_resolved(&resolved, path, span, type_expr)
    }

    /// Validate a property path on a resolved type.
    fn validate_path_on_resolved(
        &self,
        resolved: &ResolvedType,
        path: &[String],
        span: SourceSpan,
        original_type: &TypeExpr,
    ) -> Result<ResolvedType, Diagnostic> {
        if path.is_empty() {
            return Ok(resolved.clone());
        }

        let field_name = &path[0];
        let remaining_path = &path[1..];

        match resolved {
            ResolvedType::Object(fields) => {
                // Find the field
                if let Some(field) = fields.iter().find(|f| &f.name == field_name) {
                    // Recursively validate the remaining path
                    self.validate_path_on_resolved(
                        &field.type_expr,
                        remaining_path,
                        span,
                        original_type,
                    )
                } else {
                    // Field not found
                    let available: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
                    let hint = crate::diagnostics::find_similar(field_name, &available, 2)
                        .map(|s| format!("Did you mean '{}'?", s));

                    let type_name = self.get_type_name(original_type);
                    let mut diag = Diagnostic::error(
                        DiagnosticCode::E0401,
                        format!(
                            "Property '{}' does not exist on type '{}'",
                            field_name, type_name
                        ),
                    )
                    .with_span(span)
                    .with_note(format!("Available properties: {}", available.join(", ")));

                    if let Some(h) = hint {
                        diag = diag.with_hint(h);
                    }

                    Err(diag)
                }
            }
            ResolvedType::Array(inner) => {
                // For arrays, validate the path on the inner type
                self.validate_path_on_resolved(inner, path, span, original_type)
            }
            _ => {
                let type_name = self.get_type_name(original_type);
                Err(Diagnostic::error(
                    DiagnosticCode::E0401,
                    format!(
                        "Cannot access property '{}' on primitive type '{}'",
                        field_name, type_name
                    ),
                )
                .with_span(span))
            }
        }
    }

    /// Get a human-readable type name from a TypeExpr.
    fn get_type_name(&self, expr: &TypeExpr) -> String {
        match expr {
            TypeExpr::Primitive(name) => name.clone(),
            TypeExpr::Array(inner) => format!("{}[]", self.get_type_name(inner)),
            TypeExpr::Object(_) => "object".to_string(),
            TypeExpr::Union(variants) => variants.join(" | "),
            TypeExpr::Reference(name) => name.clone(),
            TypeExpr::Apply { ctor, args } => {
                let inner: Vec<String> = args.iter().map(|a| self.get_type_name(a)).collect();
                format!("{}({})", ctor, inner.join(", "))
            }
        }
    }

    // =========================================================================
    // Typed live-data substrate (PLAN-119 W1)
    //
    // Two entry points that make a registered binding TYPE do work:
    //   - `zero_value`      — materialize a type's zero instance as a concrete
    //                         JSON value (FUP-149: type-derived subscribe seed).
    //   - `check_read_path` — the single production entry to the existing
    //                         `validate_property_path` (FUP-150: field-name
    //                         checking on typed reads). Thin wrapper so callers
    //                         don't reach into the recursive validator directly.
    // =========================================================================

    /// Materialize the ZERO VALUE of a type as a concrete `serde_json::Value`.
    ///
    /// number→0, string→"", boolean→false, record→object of field zero-values,
    /// array→[], union→first-variant-string (documented choice: a tagged sum's
    /// zero is its first declared variant), unknown/opaque→null.
    ///
    /// The result serializes DIRECTLY to a JS literal (`value.to_string()`), so a
    /// type-derived seed is emitted as serialized JSON — it never round-trips
    /// through the seed-expr parser, side-stepping the nested-object / bare-key
    /// balanced-capture limitation (BUG-228 / PLAN-120) by construction.
    ///
    /// `visited` guards self-referential / mutually-recursive `@type`s: a
    /// reference already being expanded yields `null` (its zero is "absent")
    /// rather than recursing forever.
    pub fn zero_value(&self, expr: &TypeExpr) -> serde_json::Value {
        // Fetch the scalar table once and thread it down: `zero_value_guarded`
        // recurses (records, arrays, references), and each recursive call
        // materializing a primitive would otherwise re-clone the whole stdlib
        // registry. The public entry keeps its original signature.
        let (scalar_registry, _) = crate::compiler::cached_stdlib_registry();
        self.zero_value_guarded(expr, &mut Vec::new(), &scalar_registry)
    }

    fn zero_value_guarded(
        &self,
        expr: &TypeExpr,
        visited: &mut Vec<String>,
        scalar_registry: &crate::MetaRegistry,
    ) -> serde_json::Value {
        match expr {
            TypeExpr::Primitive(name) => match scalar_registry.get_scalar_type(name) {
                // The `%zero` column (a STRING in the table) converted per `%schema`.
                Some(row) => scalar_zero_value(row),
                // A primitive no `%scalar_type` row declares (unit tests
                // constructing types directly, or a registry that failed to
                // load) degrades to the pre-table default: the empty string.
                None => serde_json::json!(""),
            },
            TypeExpr::Array(_) => serde_json::json!([]),
            TypeExpr::Object(fields) => {
                let mut obj = serde_json::Map::new();
                for field in fields {
                    // Optional fields are ABSENT from the zero instance (they
                    // default to undefined, matching their optionality).
                    if field.optional {
                        continue;
                    }
                    obj.insert(
                        field.name.clone(),
                        self.zero_value_guarded(&field.type_expr, visited, scalar_registry),
                    );
                }
                serde_json::Value::Object(obj)
            }
            // A tagged sum's zero is its first declared variant name (a string);
            // an empty union degrades to null.
            TypeExpr::Union(variants) => match variants.first() {
                Some(v) => serde_json::json!(v),
                None => serde_json::Value::Null,
            },
            // `id(T)` relations serialize as a bare id string at runtime; their
            // zero is the empty id.
            TypeExpr::Apply { .. } => serde_json::json!(""),
            TypeExpr::Reference(name) => {
                if visited.iter().any(|n| n == name) {
                    // Cycle: this type is already being expanded up-stack.
                    return serde_json::Value::Null;
                }
                match self.get_type(name) {
                    Some(type_def) => {
                        visited.push(name.clone());
                        let mut obj = serde_json::Map::new();
                        for field in &type_def.fields {
                            if field.optional {
                                continue;
                            }
                            obj.insert(
                                field.name.clone(),
                                self.zero_value_guarded(&field.type_expr, visited, scalar_registry),
                            );
                        }
                        visited.pop();
                        serde_json::Value::Object(obj)
                    }
                    // Unknown reference: null (a separate check, E0102, reports
                    // the dangling reference; zero_value stays total).
                    None => serde_json::Value::Null,
                }
            }
        }
    }

    /// The single PRODUCTION entry point for field-name checking a dotted read
    /// against a binding's registered type (FUP-150). Thin wrapper over the
    /// recursive `validate_property_path`: an empty path is trivially valid; a
    /// path segment naming a field the type does not have yields E0401 with a
    /// `find_similar` "did you mean" hint.
    ///
    /// Untyped call sites simply don't call this (purely additive) — there is no
    /// type to check against, so today's behavior is preserved.
    pub fn check_read_path(
        &self,
        binding_type: &TypeExpr,
        path: &[String],
        span: SourceSpan,
    ) -> Result<ResolvedType, Diagnostic> {
        self.validate_property_path(binding_type, path, span)
    }
}

// =============================================================================
// Resolved Type
// =============================================================================

/// A fully resolved type (all references expanded).
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    Primitive(String),
    Array(Box<ResolvedType>),
    Object(Vec<ResolvedTypeField>),
    Union(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTypeField {
    pub name: String,
    pub optional: bool,
    pub type_expr: ResolvedType,
}

impl ResolvedType {
    /// Check if this type is compatible with a filter.
    ///
    /// Returns true if the type can be used with the given filter.
    pub fn is_compatible_with_filter(&self, filter_name: &str) -> bool {
        match filter_name {
            // Filters that work on any type
            "default" | "json" => true,

            // Number filters
            "currency" => self.is_number(),

            // String filters
            "uppercase" | "lowercase" | "capitalize" | "truncate" => self.is_string(),

            // Date filters
            "date" => self.is_string(), // Assuming dates are stored as strings

            // Array filters
            "count" => self.is_array(),

            // Conditional
            "if" => true, // Works on any type

            // Pluralize works on numbers
            "pluralize" => self.is_number(),

            _ => false,
        }
    }

    /// Check if this is a number type.
    fn is_number(&self) -> bool {
        matches!(self, ResolvedType::Primitive(name) if name == "number")
    }

    /// Check if this is a string type.
    fn is_string(&self) -> bool {
        matches!(
            self,
            ResolvedType::Primitive(name) if name == "string" || name == "url" || name == "color"
        )
    }

    /// Check if this is an array type.
    fn is_array(&self) -> bool {
        matches!(self, ResolvedType::Array(_))
    }

    /// Get the expected type name for error messages.
    pub fn expected_type_for_filter(filter_name: &str) -> &'static str {
        match filter_name {
            "currency" | "pluralize" => "number",
            "uppercase" | "lowercase" | "capitalize" | "truncate" | "date" => "string",
            "count" => "array",
            _ => "any",
        }
    }
}

// =============================================================================
// JSON Schema Generation
// =============================================================================

/// Generate a JSON Schema from a type expression.
pub fn generate_json_schema(type_expr: &TypeExpr, registry: &TypeRegistry) -> serde_json::Value {
    match type_expr {
        // `richtext`: a string carrying constrained HTML; flag it so the admin
        // renders the block editor and the server sanitizes on persist (FUP-029).
        TypeExpr::Primitive(name) if name == "richtext" => serde_json::json!({
            "type": "string",
            "x-st-richtext": true,
        }),
        TypeExpr::Primitive(name) => primitive_to_schema(name),
        TypeExpr::Array(inner) => {
            serde_json::json!({
                "type": "array",
                "items": generate_json_schema(inner, registry)
            })
        }
        TypeExpr::Object(fields) => {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for field in fields {
                properties.insert(
                    field.name.clone(),
                    generate_json_schema(&field.type_expr, registry),
                );
                if !field.optional {
                    required.push(field.name.clone());
                }
            }

            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required,
            })
        }
        TypeExpr::Union(variants) => {
            serde_json::json!({
                "type": "string",
                "enum": variants,
            })
        }
        // CMS relation: `id(Agent)` serializes to a bare id STRING, annotated
        // with `x-st-ref` naming the target type. The admin reads x-st-ref to
        // render a relation picker instead of a text input (FUP-028).
        TypeExpr::Apply { ctor, args } if ctor == "id" => {
            let target = match args.first() {
                Some(TypeExpr::Reference(name)) => name.clone(),
                Some(other) => registry.get_type_name(other),
                None => String::new(),
            };
            if target.is_empty() {
                // Degenerate `id()` with no target: degrade to a plain string
                // rather than emit an empty x-st-ref (which would render a
                // relation picker bound to nothing).
                serde_json::json!({ "type": "string" })
            } else {
                serde_json::json!({
                    "type": "string",
                    "x-st-ref": target,
                })
            }
        }
        // Unknown constructor: fall back to a plain string (forward-compatible).
        TypeExpr::Apply { .. } => serde_json::json!({ "type": "string" }),
        TypeExpr::Reference(name) => {
            // Resolve the reference and generate schema for it
            if let Some(type_def) = registry.get_type(name) {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for field in &type_def.fields {
                    properties.insert(
                        field.name.clone(),
                        generate_json_schema(&field.type_expr, registry),
                    );
                    if !field.optional {
                        required.push(field.name.clone());
                    }
                }

                serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required,
                })
            } else {
                // Fallback for unknown reference
                serde_json::json!({})
            }
        }
    }
}

/// Convert a primitive type name to JSON Schema.
fn primitive_to_schema(name: &str) -> serde_json::Value {
    // The scalar table (stdlib/scalars/types.st) is the single source: the JSON
    // `type` comes from `%schema`, the `format` (when non-empty) from `%format`.
    // A name no `%scalar_type` row declares (e.g. a unit test constructing a
    // `Primitive` directly, or a registry that failed to load) degrades to the
    // pre-table default `{ "type": "string" }` rather than panicking.
    let (scalar_registry, _) = crate::compiler::cached_stdlib_registry();
    match scalar_registry.get_scalar_type(name) {
        Some(row) => {
            let mut obj = serde_json::json!({ "type": row.schema });
            if !row.format.is_empty() {
                obj["format"] = serde_json::json!(row.format);
            }
            obj
        }
        None => serde_json::json!({ "type": "string" }), // Default to string
    }
}

/// Convert a `%scalar_type` row's `%zero` column (a STRING in the table) into a
/// concrete JSON zero, shaped by the row's `%schema`. schema `number` -> JSON
/// number (`"0"` -> `0`), `boolean` -> JSON bool (`"false"` -> `false`), and
/// every other schema is a string-backed scalar whose declared `%zero` IS the
/// JSON string value (almost always the empty string). A number/boolean row
/// whose `%zero` fails to parse degrades to `0` / `false`.
fn scalar_zero_value(row: &crate::parser::meta_ast::ScalarTypeDefAst) -> serde_json::Value {
    match row.schema.as_str() {
        "number" => match row.zero.parse::<i64>() {
            Ok(i) => serde_json::json!(i),
            Err(_) => match row.zero.parse::<f64>() {
                Ok(f) => serde_json::json!(f),
                Err(_) => serde_json::json!(0),
            },
        },
        "boolean" => serde_json::json!(row.zero == "true"),
        _ => serde_json::json!(row.zero.as_str()),
    }
}

/// Extract tagged-sum variants from a `@type` FormMatch's `variants` capture
/// (PLAN-038 W0). Returns an empty Vec for a product type (no `variants` capture).
///
/// The stdlib `variant_list` grammar (`$head:variant_def $tail:variant_alt+`)
/// reifies to `Named{ head: <variant>, tail: Array[Named{ v: <variant> }] }`,
/// where each `<variant>` is `Named{ name: Ident, payload?: Array[Named{ ty }] }`.
/// This walks that shape into ordered `VariantDef`s (head first, then each tail).
/// Pure restructure — no parsing (reifier-honesty rule).
fn extract_variants(fm: &crate::syntax::FormMatch) -> Vec<crate::parser::VariantDef> {
    use crate::syntax::CapturedValue;

    let Some(CapturedValue::Named(top)) = fm.get("variants") else {
        return Vec::new();
    };

    let mut variants = Vec::new();

    // The leading variant.
    if let Some(head) = top.get("head")
        && let Some(v) = variant_from_value(head)
    {
        variants.push(v);
    }

    // Each `| variant` tail unit wraps its variant under the `v` key.
    if let Some(CapturedValue::Array(tails)) = top.get("tail") {
        for tail in tails {
            if let CapturedValue::Named(unit) = tail
                && let Some(inner) = unit.get("v")
                && let Some(v) = variant_from_value(inner)
            {
                variants.push(v);
            }
        }
    }

    variants
}

/// Build a sum `TypeDef` from a signal-def `receive` block's arms (PLAN-038
/// FUP-081 #5). Each arm's consequence is a variant constructor
/// `Created($.body as Todo)`: the `ctor` is the variant NAME, and the carried
/// `as <Type>` ascription (parsed from the payload Expr) is its single payload
/// type. An arm with no `as` (e.g. `Failed($.statusText)`) yields a payload-less
/// variant. A bare-expr consequence (no `variant`) contributes no named variant.
/// Returns None when the sum name is empty or no named variant is gathered.
fn sum_from_receive_arms(
    sum_name: &str,
    arms: Option<&crate::syntax::CapturedValue>,
) -> Option<crate::parser::TypeDef> {
    use crate::parser::SourceSpan;
    use crate::syntax::CapturedValue;

    if sum_name.is_empty() {
        return None;
    }
    let CapturedValue::Array(items) = arms? else {
        return None;
    };

    let mut variants = Vec::new();
    for arm in items {
        let CapturedValue::Named(rec) = arm else {
            continue;
        };
        // The consequence: cons.variant.{ctor, payload}. A bare-expr cons has no
        // `variant` key — skip (no named variant to register).
        let Some(CapturedValue::Named(cons)) = rec.get("cons") else {
            continue;
        };
        let Some(CapturedValue::Named(variant)) = cons.get("variant") else {
            continue;
        };
        let name = match variant.get("ctor") {
            Some(CapturedValue::Ident(s)) => s.clone(),
            _ => continue,
        };
        // The carried type = the trailing `as <Type>` of the payload expr.
        let payload = match variant.get("payload") {
            Some(CapturedValue::Expr(e)) | Some(CapturedValue::String(e)) => {
                payload_type_from_as(e).into_iter().collect()
            }
            _ => Vec::new(),
        };
        variants.push(crate::parser::VariantDef { name, payload });
    }

    if variants.is_empty() {
        return None;
    }
    Some(crate::parser::TypeDef {
        name: sum_name.to_string(),
        fields: Vec::new(),
        variants,
        span: SourceSpan::default(),
    })
}

/// Extract the carried type from a payload expression's trailing `as <Type>`
/// ascription. `"$.body as Todo"` → `Some("Todo")`; `"normalize($.x) as Result"`
/// → `Some("Result")`; `"$.statusText"` (no `as`) → None. The type token is the
/// identifier run after the LAST ` as `.
fn payload_type_from_as(expr: &str) -> Option<String> {
    let lower_marker = " as ";
    let idx = expr.rfind(lower_marker)?;
    let after = expr[idx + lower_marker.len()..].trim();
    let ty: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.' || *c == '[' || *c == ']')
        .collect();
    let ty = ty.trim_end_matches(['[', ']']).to_string();
    if ty.is_empty() { None } else { Some(ty) }
}

/// Convert one captured `variant_def` record (`Named{ name, payload? }`) into a
/// `VariantDef`. The payload is an Array of `Named{ ty: TypeRef }` records; each
/// ty's text becomes one raw payload type string. A payload-less tag yields `[]`.
fn variant_from_value(value: &crate::syntax::CapturedValue) -> Option<crate::parser::VariantDef> {
    use crate::syntax::CapturedValue;

    let CapturedValue::Named(rec) = value else {
        return None;
    };
    let name = match rec.get("name") {
        Some(CapturedValue::Ident(s)) => s.clone(),
        _ => return None,
    };

    let mut payload = Vec::new();
    if let Some(CapturedValue::Array(items)) = rec.get("payload") {
        for item in items {
            if let CapturedValue::Named(tyrec) = item
                && let Some(text) = captured_type_text(tyrec.get("ty"))
            {
                payload.push(text);
            }
        }
    }

    Some(crate::parser::VariantDef { name, payload })
}

/// Text of a captured payload type (a `typeref` leaf yields `TypeRef`; tolerate
/// `Ident`/`Expr` too). Trimmed; empty → None.
fn captured_type_text(v: Option<&crate::syntax::CapturedValue>) -> Option<String> {
    use crate::syntax::CapturedValue;
    let text = match v {
        Some(CapturedValue::TypeRef(s))
        | Some(CapturedValue::Ident(s))
        | Some(CapturedValue::Expr(s)) => s.trim().to_string(),
        _ => return None,
    };
    if text.is_empty() { None } else { Some(text) }
}

/// Parse a type_ref string into a TypeExpr
///
/// This is a simple parser that handles:
/// - Primitive types (string, number, boolean, etc.)
/// - Array types (Type[])
/// - Union types ("a" | "b" | "c")
/// - Type references (User, Product, etc.)
///
/// For now, this is a basic implementation that handles common cases.
/// More complex parsing can be added as needed.
pub(crate) fn parse_type_ref_to_expr(type_ref: &str) -> TypeExpr {
    let trimmed = type_ref.trim();

    // Check for array type
    if let Some(inner) = trimmed.strip_suffix("[]") {
        return TypeExpr::Array(Box::new(parse_type_ref_to_expr(inner)));
    }

    // Check for union type (simple check for | character)
    if trimmed.contains('|') {
        let variants: Vec<String> = trimmed
            .split('|')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();
        return TypeExpr::Union(variants);
    }

    // Parameterized application: `ctor(arg, ...)` e.g. the CMS relation `id(Agent)`.
    // Parens are the codebase convention for type arguments (cf. capture maps).
    if let Some(open) = trimmed.find('(')
        && trimmed.ends_with(')')
        && open > 0
    {
        let ctor = trimmed[..open].trim().to_string();
        let inner = &trimmed[open + 1..trimmed.len() - 1];
        let args: Vec<TypeExpr> = inner
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(parse_type_ref_to_expr)
            .collect();
        return TypeExpr::Apply { ctor, args };
    }

    // Check for primitive types: a name is a scalar IFF a `%scalar_type` row in
    // stdlib/scalars/types.st declares it (FEAT-168). `color`/`length`/`duration`
    // are domain scalars that serialize as strings but carry a `format`; the base
    // scalars live in the table too, so the recognition list has a single source
    // rather than a Rust match that must be re-synced by hand.
    let (scalar_registry, _) = crate::compiler::cached_stdlib_registry();
    if scalar_registry.get_scalar_type(trimmed).is_some() {
        TypeExpr::Primitive(trimmed.to_string())
    } else {
        // Otherwise, treat as a reference to another type
        TypeExpr::Reference(trimmed.to_string())
    }
}

/// Validate JSON data against a JSON Schema.
///
/// Returns a list of validation errors.
pub fn validate_json_against_schema(
    data: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Simple schema validation (can be extended with a full JSON Schema validator)
    if let Some(schema_type) = schema.get("type").and_then(|v| v.as_str()) {
        let data_type = match data {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        };

        if schema_type != data_type {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::E0502,
                format!(
                    "Type mismatch at {}: expected {}, found {}",
                    path, schema_type, data_type
                ),
            ));
            return diagnostics;
        }

        // Validate object properties
        if schema_type == "object"
            && let (Some(data_obj), Some(properties)) = (
                data.as_object(),
                schema.get("properties").and_then(|v| v.as_object()),
            )
        {
            // Check required fields
            if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
                for req in required {
                    if let Some(field_name) = req.as_str()
                        && !data_obj.contains_key(field_name)
                    {
                        diagnostics.push(Diagnostic::error(
                            DiagnosticCode::E0501,
                            format!("Missing required field '{}' at {}", field_name, path),
                        ));
                    }
                }
            }

            // Validate each property
            for (key, value) in data_obj {
                if let Some(prop_schema) = properties.get(key) {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    diagnostics.extend(validate_json_against_schema(value, prop_schema, &new_path));
                }
            }
        }

        // Validate array items
        if schema_type == "array"
            && let (Some(data_arr), Some(items_schema)) = (data.as_array(), schema.get("items"))
        {
            for (i, item) in data_arr.iter().enumerate() {
                let new_path = format!("{}[{}]", path, i);
                diagnostics.extend(validate_json_against_schema(item, items_schema, &new_path));
            }
        }

        // Validate enum
        if let Some(enum_values) = schema.get("enum").and_then(|v| v.as_array())
            && !enum_values.contains(data)
        {
            let valid_values: Vec<_> = enum_values.iter().filter_map(|v| v.as_str()).collect();
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::E0503,
                format!(
                    "Invalid enum value at {}: expected one of [{}], found {:?}",
                    path,
                    valid_values.join(", "),
                    data
                ),
            ));
        }
    }

    diagnostics
}
