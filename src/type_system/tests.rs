//! Tests for the type system.

use super::*;
use crate::diagnostics::SourceSpan as DiagSourceSpan;
use crate::parser::{SourceSpan, TypeField};

// Helper function to create a simple type definition
fn create_type(name: &str, fields: Vec<(&str, TypeExpr, bool)>) -> TypeDef {
    TypeDef {
        name: name.to_string(),
        fields: fields
            .into_iter()
            .map(|(name, type_expr, optional)| TypeField {
                name: name.to_string(),
                optional,
                type_expr,
            })
            .collect(),
        variants: Vec::new(),
        span: SourceSpan::default(),
    }
}

#[test]
fn test_type_registry_basic() {
    let types = vec![create_type(
        "Person",
        vec![
            ("name", TypeExpr::Primitive("string".to_string()), false),
            ("age", TypeExpr::Primitive("number".to_string()), false),
            ("email", TypeExpr::Primitive("string".to_string()), true),
        ],
    )];

    let registry = TypeRegistry::from_types(&types).unwrap();
    assert!(registry.get_type("Person").is_some());
    assert!(registry.get_type("Unknown").is_none());
}

#[test]
fn test_duplicate_type_names() {
    let types = vec![
        create_type(
            "Person",
            vec![("name", TypeExpr::Primitive("string".to_string()), false)],
        ),
        create_type(
            "Person",
            vec![("age", TypeExpr::Primitive("number".to_string()), false)],
        ),
    ];

    let result = TypeRegistry::from_types(&types);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0101);
}

#[test]
fn test_unknown_type_reference() {
    let types = vec![create_type(
        "Post",
        vec![
            ("title", TypeExpr::Primitive("string".to_string()), false),
            ("author", TypeExpr::Reference("Person".to_string()), false), // Person doesn't exist
        ],
    )];

    let result = TypeRegistry::from_types(&types);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0102);
    assert!(
        errors[0]
            .message
            .contains("Unknown type reference 'Person'")
    );
}

#[test]
fn test_circular_type_dependency() {
    let types = vec![
        create_type(
            "A",
            vec![("b", TypeExpr::Reference("B".to_string()), false)],
        ),
        create_type(
            "B",
            vec![("a", TypeExpr::Reference("A".to_string()), false)],
        ),
    ];

    let result = TypeRegistry::from_types(&types);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.code == DiagnosticCode::E0101));
}

#[test]
fn test_circular_dependency_with_optional() {
    // Optional fields don't prevent circular dependencies in the current implementation,
    // but we document it as a solution
    let types = vec![
        create_type(
            "A",
            vec![("b", TypeExpr::Reference("B".to_string()), true)], // optional
        ),
        create_type(
            "B",
            vec![("a", TypeExpr::Reference("A".to_string()), false)],
        ),
    ];

    // This will still detect a cycle, but the hint suggests using optional fields
    let result = TypeRegistry::from_types(&types);
    assert!(result.is_err());
}

#[test]
fn test_valid_type_reference() {
    let types = vec![
        create_type(
            "Author",
            vec![
                ("name", TypeExpr::Primitive("string".to_string()), false),
                ("email", TypeExpr::Primitive("string".to_string()), false),
            ],
        ),
        create_type(
            "Post",
            vec![
                ("title", TypeExpr::Primitive("string".to_string()), false),
                ("author", TypeExpr::Reference("Author".to_string()), false),
            ],
        ),
    ];

    let registry = TypeRegistry::from_types(&types).unwrap();
    assert!(registry.get_type("Author").is_some());
    assert!(registry.get_type("Post").is_some());
}

#[test]
fn test_resolve_primitive_type() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let expr = TypeExpr::Primitive("string".to_string());
    let resolved = registry.resolve_type_expr(&expr).unwrap();
    assert_eq!(resolved, ResolvedType::Primitive("string".to_string()));
}

#[test]
fn test_resolve_array_type() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let expr = TypeExpr::Array(Box::new(TypeExpr::Primitive("string".to_string())));
    let resolved = registry.resolve_type_expr(&expr).unwrap();
    match resolved {
        ResolvedType::Array(inner) => {
            assert_eq!(*inner, ResolvedType::Primitive("string".to_string()));
        }
        _ => panic!("Expected array type"),
    }
}

#[test]
fn test_resolve_object_type() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let expr = TypeExpr::Object(vec![
        TypeField {
            name: "name".to_string(),
            optional: false,
            type_expr: TypeExpr::Primitive("string".to_string()),
        },
        TypeField {
            name: "age".to_string(),
            optional: true,
            type_expr: TypeExpr::Primitive("number".to_string()),
        },
    ]);
    let resolved = registry.resolve_type_expr(&expr).unwrap();
    match resolved {
        ResolvedType::Object(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "name");
            assert!(!fields[0].optional);
            assert_eq!(fields[1].name, "age");
            assert!(fields[1].optional);
        }
        _ => panic!("Expected object type"),
    }
}

#[test]
fn test_resolve_union_type() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let expr = TypeExpr::Union(vec!["S".to_string(), "M".to_string(), "L".to_string()]);
    let resolved = registry.resolve_type_expr(&expr).unwrap();
    match resolved {
        ResolvedType::Union(variants) => {
            assert_eq!(variants.len(), 3);
            assert_eq!(variants, vec!["S", "M", "L"]);
        }
        _ => panic!("Expected union type"),
    }
}

#[test]
fn test_resolve_type_reference() {
    let types = vec![create_type(
        "Person",
        vec![
            ("name", TypeExpr::Primitive("string".to_string()), false),
            ("age", TypeExpr::Primitive("number".to_string()), false),
        ],
    )];

    let registry = TypeRegistry::from_types(&types).unwrap();
    let expr = TypeExpr::Reference("Person".to_string());
    let resolved = registry.resolve_type_expr(&expr).unwrap();
    match resolved {
        ResolvedType::Object(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "name");
            assert_eq!(fields[1].name, "age");
        }
        _ => panic!("Expected object type"),
    }
}

#[test]
fn test_validate_property_path_simple() {
    let types = vec![create_type(
        "Person",
        vec![
            ("name", TypeExpr::Primitive("string".to_string()), false),
            ("age", TypeExpr::Primitive("number".to_string()), false),
        ],
    )];

    let registry = TypeRegistry::from_types(&types).unwrap();
    let type_expr = TypeExpr::Reference("Person".to_string());

    // Valid path
    let result = registry.validate_property_path(
        &type_expr,
        &["name".to_string()],
        DiagSourceSpan::default(),
    );
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        ResolvedType::Primitive("string".to_string())
    );

    // Invalid path
    let result = registry.validate_property_path(
        &type_expr,
        &["email".to_string()],
        DiagSourceSpan::default(),
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, DiagnosticCode::E0401);
    assert!(err.message.contains("Property 'email' does not exist"));
}

#[test]
fn test_validate_property_path_nested() {
    let types = vec![
        create_type(
            "Address",
            vec![
                ("street", TypeExpr::Primitive("string".to_string()), false),
                ("city", TypeExpr::Primitive("string".to_string()), false),
            ],
        ),
        create_type(
            "Person",
            vec![
                ("name", TypeExpr::Primitive("string".to_string()), false),
                ("address", TypeExpr::Reference("Address".to_string()), false),
            ],
        ),
    ];

    let registry = TypeRegistry::from_types(&types).unwrap();
    let type_expr = TypeExpr::Reference("Person".to_string());

    // Valid nested path
    let result = registry.validate_property_path(
        &type_expr,
        &["address".to_string(), "city".to_string()],
        DiagSourceSpan::default(),
    );
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        ResolvedType::Primitive("string".to_string())
    );

    // Invalid nested path
    let result = registry.validate_property_path(
        &type_expr,
        &["address".to_string(), "country".to_string()],
        DiagSourceSpan::default(),
    );
    assert!(result.is_err());
}

#[test]
fn test_validate_property_path_on_array() {
    let types = vec![
        create_type(
            "Item",
            vec![
                ("id", TypeExpr::Primitive("string".to_string()), false),
                ("name", TypeExpr::Primitive("string".to_string()), false),
            ],
        ),
        create_type(
            "Collection",
            vec![(
                "items",
                TypeExpr::Array(Box::new(TypeExpr::Reference("Item".to_string()))),
                false,
            )],
        ),
    ];

    let registry = TypeRegistry::from_types(&types).unwrap();
    let type_expr = TypeExpr::Reference("Collection".to_string());

    // Valid path into array items
    let result = registry.validate_property_path(
        &type_expr,
        &["items".to_string(), "name".to_string()],
        DiagSourceSpan::default(),
    );
    assert!(result.is_ok());
}

#[test]
fn test_filter_compatibility_currency() {
    let number_type = ResolvedType::Primitive("number".to_string());
    let string_type = ResolvedType::Primitive("string".to_string());

    assert!(number_type.is_compatible_with_filter("currency"));
    assert!(!string_type.is_compatible_with_filter("currency"));
}

#[test]
fn test_filter_compatibility_string_filters() {
    let string_type = ResolvedType::Primitive("string".to_string());
    let number_type = ResolvedType::Primitive("number".to_string());

    assert!(string_type.is_compatible_with_filter("uppercase"));
    assert!(string_type.is_compatible_with_filter("lowercase"));
    assert!(string_type.is_compatible_with_filter("capitalize"));
    assert!(string_type.is_compatible_with_filter("truncate"));

    assert!(!number_type.is_compatible_with_filter("uppercase"));
}

#[test]
fn test_filter_compatibility_array_filters() {
    let array_type = ResolvedType::Array(Box::new(ResolvedType::Primitive("string".to_string())));
    let string_type = ResolvedType::Primitive("string".to_string());

    assert!(array_type.is_compatible_with_filter("count"));
    assert!(!string_type.is_compatible_with_filter("count"));
}

#[test]
fn test_filter_compatibility_universal_filters() {
    let number_type = ResolvedType::Primitive("number".to_string());
    let string_type = ResolvedType::Primitive("string".to_string());

    // default and json work on any type
    assert!(number_type.is_compatible_with_filter("default"));
    assert!(string_type.is_compatible_with_filter("default"));
    assert!(number_type.is_compatible_with_filter("json"));
    assert!(string_type.is_compatible_with_filter("json"));
}

#[test]
fn test_json_schema_generation_primitive() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let schema = generate_json_schema(&TypeExpr::Primitive("string".to_string()), &registry);
    assert_eq!(schema, serde_json::json!({ "type": "string" }));

    let schema = generate_json_schema(&TypeExpr::Primitive("number".to_string()), &registry);
    assert_eq!(schema, serde_json::json!({ "type": "number" }));

    let schema = generate_json_schema(&TypeExpr::Primitive("boolean".to_string()), &registry);
    assert_eq!(schema, serde_json::json!({ "type": "boolean" }));

    let schema = generate_json_schema(&TypeExpr::Primitive("url".to_string()), &registry);
    assert_eq!(
        schema,
        serde_json::json!({ "type": "string", "format": "uri" })
    );
}

#[test]
fn test_json_schema_generation_array() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let schema = generate_json_schema(
        &TypeExpr::Array(Box::new(TypeExpr::Primitive("string".to_string()))),
        &registry,
    );
    assert_eq!(
        schema,
        serde_json::json!({
            "type": "array",
            "items": { "type": "string" }
        })
    );
}

#[test]
fn test_json_schema_generation_object() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let schema = generate_json_schema(
        &TypeExpr::Object(vec![
            TypeField {
                name: "name".to_string(),
                optional: false,
                type_expr: TypeExpr::Primitive("string".to_string()),
            },
            TypeField {
                name: "age".to_string(),
                optional: true,
                type_expr: TypeExpr::Primitive("number".to_string()),
            },
        ]),
        &registry,
    );

    let expected = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "number" }
        },
        "required": ["name"]
    });

    assert_eq!(schema, expected);
}

#[test]
fn test_json_schema_generation_union() {
    let registry = TypeRegistry::from_types(&[]).unwrap();
    let schema = generate_json_schema(
        &TypeExpr::Union(vec!["S".to_string(), "M".to_string(), "L".to_string()]),
        &registry,
    );

    assert_eq!(
        schema,
        serde_json::json!({
            "type": "string",
            "enum": ["S", "M", "L"]
        })
    );
}

#[test]
fn test_json_schema_generation_reference() {
    let types = vec![create_type(
        "Person",
        vec![
            ("name", TypeExpr::Primitive("string".to_string()), false),
            ("age", TypeExpr::Primitive("number".to_string()), false),
        ],
    )];

    let registry = TypeRegistry::from_types(&types).unwrap();
    let schema = generate_json_schema(&TypeExpr::Reference("Person".to_string()), &registry);

    let expected = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "number" }
        },
        "required": ["name", "age"]
    });

    assert_eq!(schema, expected);
}

#[test]
fn test_validate_json_type_mismatch() {
    let schema = serde_json::json!({ "type": "string" });
    let data = serde_json::json!(42);

    let errors = validate_json_against_schema(&data, &schema, "value");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0502);
    assert!(errors[0].message.contains("Type mismatch"));
}

#[test]
fn test_validate_json_missing_required_field() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "number" }
        },
        "required": ["name", "age"]
    });

    let data = serde_json::json!({ "name": "Alice" });

    let errors = validate_json_against_schema(&data, &schema, "");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0501);
    assert!(errors[0].message.contains("Missing required field 'age'"));
}

#[test]
fn test_validate_json_invalid_enum() {
    let schema = serde_json::json!({
        "type": "string",
        "enum": ["S", "M", "L"]
    });

    let data = serde_json::json!("XL");

    let errors = validate_json_against_schema(&data, &schema, "size");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0503);
    assert!(errors[0].message.contains("Invalid enum value"));
}

#[test]
fn test_validate_json_nested_object() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "person": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }
        },
        "required": ["person"]
    });

    let data = serde_json::json!({ "person": { "age": 30 } });

    let errors = validate_json_against_schema(&data, &schema, "");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Missing required field 'name'"));
}

#[test]
fn test_validate_json_valid_data() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "number" }
        },
        "required": ["name", "age"]
    });

    let data = serde_json::json!({ "name": "Alice", "age": 30 });

    let errors = validate_json_against_schema(&data, &schema, "");
    assert_eq!(errors.len(), 0);
}

// =============================================================================
// FormMatch Integration Tests
// =============================================================================

#[test]
fn test_from_form_matches_creates_registry_with_correct_types() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let fields = vec![
        PropertyDef {
            name: "id".to_string(),
            type_ref: "string".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "price".to_string(),
            type_ref: "number".to_string(),
            optional: false,
        },
    ];

    let matches = vec![
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("Product".to_string()))
            .capture("fields", CapturedValue::Properties(fields)),
    ];

    let registry = TypeRegistry::from_form_matches(&matches);

    let product_type = registry.get_type("Product");
    assert!(product_type.is_some());

    let product = product_type.unwrap();
    assert_eq!(product.fields.len(), 2);
    assert_eq!(product.fields[0].name, "id");
    assert_eq!(product.fields[1].name, "price");
}

#[test]
fn test_from_form_matches_empty_matches() {
    use crate::syntax::FormMatch;

    let matches: Vec<FormMatch> = vec![];
    let registry = TypeRegistry::from_form_matches(&matches);

    assert_eq!(registry.type_names().len(), 0);
}

#[test]
fn test_from_form_matches_type_without_fields() {
    use crate::syntax::{CapturedValue, FormMatch};

    let matches =
        vec![FormMatch::new("type").capture("name", CapturedValue::Ident("EmptyType".to_string()))];

    let registry = TypeRegistry::from_form_matches(&matches);

    let empty_type = registry.get_type("EmptyType");
    assert!(empty_type.is_some());
    assert_eq!(empty_type.unwrap().fields.len(), 0);
}
#[test]
fn test_sum_type_parses_variants_end_to_end() {
    // PLAN-038 W0 keystone: a tagged-sum `@type` body parses into TypeDef.variants
    // through the REAL parser + stdlib `variant_list` capture grammar (no Rust
    // discriminator). A product body with no `|` stays a product (variants empty).
    let src = r#"@type AddResult {
  Created(Todo)
  | Partial(Report)
  | Invalid(Errors)
  | Failed
}

@type Product {
  id: string;
  price: number;
}
"#;
    let ast = crate::parser::parse(src).expect("parse sum + product @type");
    let registry = TypeRegistry::from_form_matches(&ast.matches);

    // The SUM type: four variants, marked a sum.
    let add = registry
        .get_type("AddResult")
        .expect("AddResult registered");
    assert!(add.is_sum(), "AddResult must be a sum type");
    assert_eq!(
        add.variants
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Created", "Partial", "Invalid", "Failed"],
        "variant names + order preserved",
    );
    assert_eq!(
        add.variants[0].payload,
        vec!["Todo".to_string()],
        "Created carries Todo"
    );
    assert!(add.variants[3].payload.is_empty(), "Failed is payload-less");
    assert!(add.fields.is_empty(), "a sum has no product fields");

    // The PRODUCT type: unaffected — no variants, two fields.
    let product = registry.get_type("Product").expect("Product registered");
    assert!(!product.is_sum(), "Product is a product type, not a sum");
    assert_eq!(product.fields.len(), 2);
    assert!(product.variants.is_empty());
}
#[test]
fn test_sum_type_multi_and_array_payloads() {
    // Multi-arg variant payloads (`Pair(A, B)`) and array payload types (`Many(Todo[])`)
    // both survive the variant grammar as ordered raw type-ref strings.
    let src = r#"@type Outcome {
  Pair(Todo, Report)
  | Many(Todo[])
  | None
}
"#;
    let ast = crate::parser::parse(src).expect("parse multi/array payloads");
    let registry = TypeRegistry::from_form_matches(&ast.matches);
    let t = registry.get_type("Outcome").expect("Outcome registered");
    assert!(t.is_sum());
    assert_eq!(
        t.variants[0].payload,
        vec!["Todo".to_string(), "Report".to_string()]
    );
    assert_eq!(t.variants[1].payload, vec!["Todo[]".to_string()]);
    assert!(t.variants[2].payload.is_empty());
}

#[test]
fn test_single_field_product_is_not_a_sum() {
    // A one-field product (`{ id: string }`) has zero `|` bars, so the sum form's
    // `variant_list` (which requires ≥1 bar) cannot match — it stays a product.
    let src = "@type Solo {\n  id: string;\n}\n";
    let ast = crate::parser::parse(src).expect("parse single-field product");
    let registry = TypeRegistry::from_form_matches(&ast.matches);
    let t = registry.get_type("Solo").expect("Solo registered");
    assert!(!t.is_sum(), "single-field product must NOT be a sum");
    assert_eq!(t.fields.len(), 1);
}

#[test]
fn test_inline_sum_type_one_line() {
    // The whole sum on one line (`{ A(T) | B(U) | C }`) parses identically to the
    // multi-line form — the bar, not the newline, separates variants.
    let src = "@type R { Ok(Todo) | Err(Errors) | Pending }\n";
    let ast = crate::parser::parse(src).expect("parse inline sum");
    let registry = TypeRegistry::from_form_matches(&ast.matches);
    let t = registry.get_type("R").expect("R registered");
    assert!(t.is_sum());
    assert_eq!(
        t.variants
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Ok", "Err", "Pending"],
    );
    assert_eq!(t.variants[0].payload, vec!["Todo".to_string()]);
    assert!(t.variants[2].payload.is_empty());
}
#[test]
fn test_variant_names_accessor() {
    // PLAN-077 W0: variant_names returns the declared variant set in order for a
    // registered sum, and an empty Vec for BOTH a missing type name AND a product
    // (non-sum) type — the two "not an enum" cases are indistinguishable by design.
    let src = r#"@type AddResult {
  Created(Todo)
  | Invalid(Errors)
  | Failed
}

@type Product {
  id: string;
  price: number;
}
"#;
    let ast = crate::parser::parse(src).expect("parse sum + product @type");
    let registry = TypeRegistry::from_form_matches(&ast.matches);

    // Hit: a sum returns its variants in declared order.
    assert_eq!(
        registry.variant_names("AddResult"),
        vec!["Created", "Invalid", "Failed"],
        "sum returns its variant set in order"
    );

    // Miss: a product type (exists, but not a sum) returns empty, not an error.
    assert!(
        registry.variant_names("Product").is_empty(),
        "a product (non-sum) type returns an empty variant set"
    );

    // Miss: an unknown type name returns empty, not a panic.
    assert!(
        registry.variant_names("Nonexistent").is_empty(),
        "an unknown type name returns an empty variant set"
    );
}

#[test]
fn test_variant_payload_arity_accessor() {
    // PLAN-077 W0: variant_payload_arity distinguishes four cases — payload-less
    // variant (Some(0)), payload-carrying variant (Some(n)), unknown variant on a
    // known sum (None), and unknown / non-sum type (None).
    let src = r#"@type Outcome {
  Pair(Todo, Report)
  | Many(Todo[])
  | None
}

@type Product {
  id: string;
}
"#;
    let ast = crate::parser::parse(src).expect("parse outcome + product");
    let registry = TypeRegistry::from_form_matches(&ast.matches);

    // Hit: payload-carrying variants report their arity.
    assert_eq!(
        registry.variant_payload_arity("Outcome", "Pair"),
        Some(2),
        "Pair(A, B) has arity 2"
    );
    assert_eq!(
        registry.variant_payload_arity("Outcome", "Many"),
        Some(1),
        "Many(Todo[]) has arity 1"
    );

    // Hit: payload-less variant EXISTS with arity 0 — distinct from "unknown variant".
    assert_eq!(
        registry.variant_payload_arity("Outcome", "None"),
        Some(0),
        "a payload-less variant has arity 0, not None"
    );

    // Miss: a variant name that isn't declared on this sum → None.
    assert_eq!(
        registry.variant_payload_arity("Outcome", "Unknown"),
        None,
        "an undeclared variant on a known sum returns None"
    );

    // Miss: a product (non-sum) type has no variant set at all → None.
    assert_eq!(
        registry.variant_payload_arity("Product", "None"),
        None,
        "a product type has no variants to look up"
    );

    // Miss: an unknown type name → None (no panic).
    assert_eq!(
        registry.variant_payload_arity("Nonexistent", "None"),
        None,
        "an unknown type name returns None"
    );
}
#[test]
fn test_from_form_matches_type_with_optional_fields() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let fields = vec![
        PropertyDef {
            name: "id".to_string(),
            type_ref: "string".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "discount".to_string(),
            type_ref: "number".to_string(),
            optional: true,
        },
    ];

    let matches = vec![
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("Product".to_string()))
            .capture("fields", CapturedValue::Properties(fields)),
    ];

    let registry = TypeRegistry::from_form_matches(&matches);

    let product_type = registry.get_type("Product").unwrap();
    assert!(!product_type.fields[0].optional);
    assert!(product_type.fields[1].optional);
}

#[test]
fn test_from_form_matches_mixed_matches_only_types_extracted() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let type_fields = vec![PropertyDef {
        name: "name".to_string(),
        type_ref: "string".to_string(),
        optional: false,
    }];

    let matches = vec![
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("User".to_string()))
            .capture("fields", CapturedValue::Properties(type_fields)),
        FormMatch::new("data").capture("name", CapturedValue::Ident("users".to_string())),
        FormMatch::new("fn").capture("name", CapturedValue::Ident("greet".to_string())),
    ];

    let registry = TypeRegistry::from_form_matches(&matches);

    // Only the type should be registered
    assert_eq!(registry.type_names().len(), 1);
    assert!(registry.get_type("User").is_some());
}

#[test]
fn test_from_form_matches_type_with_complex_type_refs() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let fields = vec![
        PropertyDef {
            name: "tags".to_string(),
            type_ref: "string[]".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "status".to_string(),
            type_ref: "\"draft\" | \"published\"".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "author".to_string(),
            type_ref: "User".to_string(),
            optional: false,
        },
    ];

    let matches = vec![
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("Post".to_string()))
            .capture("fields", CapturedValue::Properties(fields)),
    ];

    let registry = TypeRegistry::from_form_matches(&matches);

    let post_type = registry.get_type("Post").unwrap();
    assert_eq!(post_type.fields.len(), 3);

    // Check array type parsing
    match &post_type.fields[0].type_expr {
        TypeExpr::Array(inner) => match **inner {
            TypeExpr::Primitive(ref s) => assert_eq!(s, "string"),
            _ => panic!("Expected primitive type in array"),
        },
        _ => panic!("Expected array type for tags"),
    }

    // Check union type parsing
    match &post_type.fields[1].type_expr {
        TypeExpr::Union(variants) => {
            assert_eq!(variants.len(), 2);
            assert!(variants.contains(&"draft".to_string()));
            assert!(variants.contains(&"published".to_string()));
        }
        _ => panic!("Expected union type for status"),
    }

    // Check reference type parsing
    match &post_type.fields[2].type_expr {
        TypeExpr::Reference(name) => assert_eq!(name, "User"),
        _ => panic!("Expected reference type for author"),
    }
}

#[test]
fn test_from_form_matches_missing_name_capture() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let fields = vec![PropertyDef {
        name: "id".to_string(),
        type_ref: "string".to_string(),
        optional: false,
    }];

    // Type FormMatch without a name capture
    let matches = vec![FormMatch::new("type").capture("fields", CapturedValue::Properties(fields))];

    let registry = TypeRegistry::from_form_matches(&matches);

    // Should not register any types since name is missing
    assert_eq!(registry.type_names().len(), 0);
}

#[test]
fn test_from_form_matches_multiple_types() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let user_fields = vec![PropertyDef {
        name: "name".to_string(),
        type_ref: "string".to_string(),
        optional: false,
    }];

    let product_fields = vec![
        PropertyDef {
            name: "title".to_string(),
            type_ref: "string".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "price".to_string(),
            type_ref: "number".to_string(),
            optional: false,
        },
    ];

    let matches = vec![
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("User".to_string()))
            .capture("fields", CapturedValue::Properties(user_fields)),
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("Product".to_string()))
            .capture("fields", CapturedValue::Properties(product_fields)),
    ];

    let registry = TypeRegistry::from_form_matches(&matches);

    assert_eq!(registry.type_names().len(), 2);
    assert!(registry.get_type("User").is_some());
    assert!(registry.get_type("Product").is_some());

    assert_eq!(registry.get_type("User").unwrap().fields.len(), 1);
    assert_eq!(registry.get_type("Product").unwrap().fields.len(), 2);
}

#[test]
fn test_parse_type_ref_primitives() {
    use crate::syntax::{CapturedValue, FormMatch, PropertyDef};

    let fields = vec![
        PropertyDef {
            name: "str".to_string(),
            type_ref: "string".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "num".to_string(),
            type_ref: "number".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "bool".to_string(),
            type_ref: "boolean".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "link".to_string(),
            type_ref: "url".to_string(),
            optional: false,
        },
        PropertyDef {
            name: "bg".to_string(),
            type_ref: "color".to_string(),
            optional: false,
        },
    ];

    let matches = vec![
        FormMatch::new("type")
            .capture("name", CapturedValue::Ident("Test".to_string()))
            .capture("fields", CapturedValue::Properties(fields)),
    ];

    let registry = TypeRegistry::from_form_matches(&matches);
    let test_type = registry.get_type("Test").unwrap();

    assert!(matches!(&test_type.fields[0].type_expr, TypeExpr::Primitive(s) if s == "string"));
    assert!(matches!(&test_type.fields[1].type_expr, TypeExpr::Primitive(s) if s == "number"));
    assert!(matches!(&test_type.fields[2].type_expr, TypeExpr::Primitive(s) if s == "boolean"));
    assert!(matches!(&test_type.fields[3].type_expr, TypeExpr::Primitive(s) if s == "url"));
    assert!(matches!(&test_type.fields[4].type_expr, TypeExpr::Primitive(s) if s == "color"));
}

// =============================================================================
// @host transport binding (PLAN-038 W1, pillar 1)
// =============================================================================

#[test]
fn test_host_http_with_headers_parses() {
    let src = r#"@host $api : http("https://api.example.com") { headers: { authorization: $token }; }
"#;
    let ast = crate::parser::parse(src).expect("parse @host http+headers");
    let hosts: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "host")
        .collect();
    assert_eq!(hosts.len(), 1, "one @host match");
    let h = hosts[0];
    assert_eq!(h.get_binding("name"), Some("$api"), "host name captured");
    assert_eq!(
        h.get_expr("url").map(|s| s.trim()),
        Some("\"https://api.example.com\""),
        "url captured"
    );
    assert!(h.get("headers").is_some(), "headers object captured");
}

#[test]
fn test_host_http_bodyless_parses() {
    let src = "@host $api : http(\"https://api.example.com\");\n";
    let ast = crate::parser::parse(src).expect("parse bodyless @host http");
    let hosts: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "host")
        .collect();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].get_binding("name"), Some("$api"));
    assert!(
        hosts[0].get("headers").is_none(),
        "no headers in bodyless form"
    );
}

#[test]
fn test_host_ws_parses() {
    let src = "@host $mcp : ws($hostSocket);\n";
    let ast = crate::parser::parse(src).expect("parse @host ws");
    let hosts: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.macro_name == "host")
        .collect();
    assert_eq!(hosts.len(), 1, "one @host ws match");
    assert_eq!(hosts[0].get_binding("name"), Some("$mcp"));
    assert_eq!(
        hosts[0].get_expr("socket").map(|s| s.trim()),
        Some("$hostSocket"),
        "ws socket captured"
    );
}

// =============================================================================
// @data signal / @data stream — reactive output (PLAN-038 W1, pillar 2)
// =============================================================================

#[test]
fn test_data_signal_parses_and_matches() {
    let src = r#"@type Todo { id: string; title: string }
@type Errors { message: string }
@host $api : http("https://api.example.com")
@data signal $add($text string) to $api {
  send POST "/api/todos" { title: $text }
  receive to AddResult {
    200 => Created($.body as Todo);
    422 => Invalid($.body as Errors);
    _ => Failed($.statusText);
  }
  policy latest
}
<div class="x">app</div>
"#;
    let ast = crate::parser::parse(src).expect("parse @data signal");
    // The signal macro creates a `binding` (mirrors @data fetch) — assert it matched
    // with the signal source_kind and host captured (not silently skipped).
    let sig = ast
        .matches
        .iter()
        .find(|m| m.get_binding("name") == Some("$add"));
    assert!(
        sig.is_some(),
        "the @data signal must produce a match for $add"
    );
    let sig = sig.unwrap();
    // The match's macro_name is the directive creates-name (`data`); the signal
    // %registers a `binding` fact + binds the `signal-call` primitive.
    assert_eq!(
        sig.macro_name, "data",
        "@data signal creates a data directive match"
    );
    assert_eq!(sig.get_binding("host"), Some("$api"), "host captured");
}

#[test]
fn test_data_signal_bodyless_send_only_parses() {
    // A signal whose body is just send+receive (no policy/optimistic) parses.
    let src = r#"@type T { id: string }
@host $api : http("u")
@data signal $go($x string) to $api {
  send POST "/go" { x: $x }
  receive to R {
    200 => Ok($.body as T);
    _ => Err($.statusText);
  }
}
<div class="a">a</div>
"#;
    let ast = crate::parser::parse(src).expect("parse send+receive only");
    assert!(
        ast.matches
            .iter()
            .any(|m| m.get_binding("name") == Some("$go")),
        "send+receive-only signal matches"
    );
}

#[test]
fn test_data_stream_ws_parses() {
    // @data stream over a ws host with emit send + multi-arm receive parses + matches.
    let src = r#"@type Preview { html: string }
@host $mcp : ws($sock)
@data stream $watch($id string) to $mcp {
  send emit "subscribe" { id: $id }
  receive to Event {
    "previewed" => Previewed($.payload as Preview);
    "chosen" => Chosen($.payload as Preview);
  }
  policy queue
}
<div class="x">a</div>
"#;
    let ast = crate::parser::parse(src).expect("parse @data stream");
    let s = ast
        .matches
        .iter()
        .find(|m| m.get_binding("name") == Some("$watch"));
    assert!(s.is_some(), "@data stream produces a match for $watch");
    assert_eq!(
        s.unwrap().get_binding("host"),
        Some("$mcp"),
        "ws host captured"
    );
}

// =============================================================================
// @handle consumer directive (PLAN-038 W1, pillar 4)
// =============================================================================

#[test]
fn test_handle_parses_all_clauses() {
    let src = r#"@type Todo { id: string }
@host $api : http("u")
@data signal $add($text string) to $api {
  send POST "/x" { title: $text }
  receive to R { 200 => Created($.body as Todo); _ => Failed($.statusText); }
}
.quick-add {
  @handle $add {
    optimistic { $x <- 1; }
    receive {
      Created(todo) => { $items <- todo; }
      _ => { $err <- 1; }
    }
    final { $draft <- 0; }
  }
}
<div class="quick-add">a</div>
"#;
    let ast = crate::parser::parse(src).expect("parse @handle");
    let h = ast.matches.iter().find(|m| m.macro_name == "handle");
    assert!(h.is_some(), "@handle produces a match");
    assert_eq!(
        h.unwrap().get_binding("signal"),
        Some("$add"),
        "consumed signal captured"
    );
}

#[test]
fn test_handle_bare_parses() {
    // A bare `@handle $sig {}` (no clauses — adopts def behavior) parses.
    let src = r#"@type T { id: string }
@host $api : http("u")
@data signal $go($x string) to $api {
  send POST "/g" { x: $x }
  receive to R { 200 => Ok($.body as T); _ => Err($.statusText); }
}
.box { @handle $go { } }
<div class="box">a</div>
"#;
    let ast = crate::parser::parse(src).expect("parse bare @handle");
    assert!(
        ast.matches
            .iter()
            .any(|m| m.macro_name == "handle" && m.get_binding("signal") == Some("$go")),
        "bare @handle matches"
    );
}

// =============================================================================
// Scoped sum from receive arms (PLAN-038 FUP-081 #5)
// =============================================================================

#[test]
fn test_receive_block_registers_scoped_sum() {
    // A `@data signal` whose `receive to AddResult { … }` block defines a named
    // sum from its arms: ctor = variant name, `as <Type>` = carried payload.
    let src = r#"@type Todo { id: string }
@type Errors { message: string }
@host $api : http("u")
@data signal $add($text string) to $api {
  send POST "/api/todos" { title: $text }
  receive to AddResult {
    200 => Created($.body as Todo);
    422 => Invalid($.body as Errors);
    _ => Failed($.statusText);
  }
}
<div class="x">a</div>
"#;
    let ast = crate::parser::parse(src).expect("parse");
    let reg = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
    let sum = reg
        .get_type("AddResult")
        .expect("AddResult registered from receive arms");
    assert!(sum.is_sum(), "AddResult must be a sum type");
    let names: Vec<&str> = sum.variants.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["Created", "Invalid", "Failed"],
        "variant names from arm ctors"
    );
    // Carried types from `as <Type>`; Failed has no `as` → empty payload.
    let created = sum.variants.iter().find(|v| v.name == "Created").unwrap();
    assert_eq!(
        created.payload,
        vec!["Todo".to_string()],
        "Created carries Todo"
    );
    let invalid = sum.variants.iter().find(|v| v.name == "Invalid").unwrap();
    assert_eq!(
        invalid.payload,
        vec!["Errors".to_string()],
        "Invalid carries Errors"
    );
    let failed = sum.variants.iter().find(|v| v.name == "Failed").unwrap();
    assert!(
        failed.payload.is_empty(),
        "Failed (no `as`) is payload-less"
    );
}

#[test]
fn test_explicit_type_not_clobbered_by_receive_sum() {
    // An explicit `@type AddResult { … }` WINS over a same-named receive block.
    let src = r#"@type AddResult { code: int }
@host $api : http("u")
@data signal $add($text string) to $api {
  send POST "/t" { x: $text }
  receive to AddResult {
    200 => Created($.body as Todo);
    _ => Failed($.statusText);
  }
}
<div class="x">a</div>
"#;
    let ast = crate::parser::parse(src).expect("parse");
    let reg = crate::type_system::TypeRegistry::from_form_matches(&ast.matches);
    let t = reg.get_type("AddResult").expect("AddResult present");
    // The explicit product @type wins: it has a `code` field and is NOT a sum.
    assert!(
        !t.is_sum(),
        "explicit @type AddResult (product) must not be clobbered by the receive sum"
    );
    assert!(
        t.fields.iter().any(|f| f.name == "code"),
        "explicit field preserved"
    );
}

// =============================================================================
// receive envelope alias (PLAN-038 FUP-081 #1) — §S1 POSITION DECIDES ROLE
// =============================================================================

fn parse_signal_receive(recv: &str) -> crate::syntax::FormMatch {
    let src = format!(
        "@host $api : http(\"u\")\n@data signal $a($t string) to $api {{\n  send POST \"/t\" {{ x: $t }}\n  {}\n}}\n<div class=\"x\">a</div>\n",
        recv
    );
    let ast = crate::parser::parse(&src).expect("parse signal");
    ast.matches
        .into_iter()
        .find(|m| m.macro_name == "data")
        .expect("data match")
}

#[test]
fn test_receive_to_sum_no_alias() {
    // The critical case: `to` must bind the sum, NOT be captured as the alias.
    let m = parse_signal_receive(
        "receive to AddResult { 200 => Ok($.body as T); _ => No($.statusText); }",
    );
    assert_eq!(m.get_ident("sum"), Some("AddResult"), "sum captured");
    assert_eq!(
        m.get_ident("alias"),
        None,
        "`to` must NOT be captured as alias"
    );
}

#[test]
fn test_receive_alias_to_sum() {
    let m = parse_signal_receive(
        "receive Response to AddResult { 200 => Ok($.body as T); _ => No($.statusText); }",
    );
    assert_eq!(m.get_ident("alias"), Some("Response"), "alias left of `to`");
    assert_eq!(m.get_ident("sum"), Some("AddResult"), "sum right of `to`");
}

#[test]
fn test_receive_alias_only() {
    let m =
        parse_signal_receive("receive Response { 200 => Ok($.body as T); _ => No($.statusText); }");
    assert_eq!(
        m.get_ident("alias"),
        Some("Response"),
        "lone name before the brace is the envelope alias"
    );
    assert_eq!(m.get_ident("sum"), None, "no sum without `to`");
}

#[test]
fn test_receive_no_head() {
    let m = parse_signal_receive("receive { 200 => Ok($.body as T); _ => No($.statusText); }");
    assert_eq!(m.get_ident("alias"), None, "no alias");
    assert_eq!(m.get_ident("sum"), None, "no sum");
}

// =============================================================================
// PLAN-119 W1 — typed live-data substrate: zero_value + check_read_path
// =============================================================================

#[test]
fn test_zero_value_scalars() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.zero_value(&TypeExpr::Primitive("number".to_string())),
        serde_json::json!(0)
    );
    assert_eq!(
        registry.zero_value(&TypeExpr::Primitive("string".to_string())),
        serde_json::json!("")
    );
    assert_eq!(
        registry.zero_value(&TypeExpr::Primitive("boolean".to_string())),
        serde_json::json!(false)
    );
    // String-backed domain scalars zero to "".
    assert_eq!(
        registry.zero_value(&TypeExpr::Primitive("color".to_string())),
        serde_json::json!("")
    );
}

#[test]
fn test_zero_value_array_is_empty() {
    let registry = TypeRegistry::new();
    let ty = TypeExpr::Array(Box::new(TypeExpr::Primitive("string".to_string())));
    assert_eq!(registry.zero_value(&ty), serde_json::json!([]));
}

#[test]
fn test_zero_value_record_and_nested() {
    // FormEnvelope-shaped: a nested record. This is the case a hand-written seed
    // cannot express (BUG-228) but zero_value builds structurally.
    let types = vec![
        create_type(
            "FormValues",
            vec![
                ("email", TypeExpr::Primitive("string".to_string()), false),
                ("agree", TypeExpr::Primitive("boolean".to_string()), false),
            ],
        ),
        create_type(
            "FormEnvelope",
            vec![
                (
                    "values",
                    TypeExpr::Reference("FormValues".to_string()),
                    false,
                ),
                ("valid", TypeExpr::Primitive("boolean".to_string()), false),
                ("count", TypeExpr::Primitive("number".to_string()), false),
            ],
        ),
    ];
    let registry = TypeRegistry::from_types(&types).unwrap();
    let zero = registry.zero_value(&TypeExpr::Reference("FormEnvelope".to_string()));
    assert_eq!(
        zero,
        serde_json::json!({
            "values": { "email": "", "agree": false },
            "valid": false,
            "count": 0
        })
    );
    // The serialized form IS the JS literal a seed emits — no parser round-trip.
    // (Assert via round-trip, not a literal string: serde_json key order is not
    // a contract, but the serialized value must parse back to the same shape.)
    let serialized = zero.to_string();
    assert!(serialized.starts_with('{') && serialized.ends_with('}'));
    let reparsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(reparsed, zero);
}

#[test]
fn test_zero_value_optional_fields_absent() {
    let types = vec![create_type(
        "Partial",
        vec![
            ("required", TypeExpr::Primitive("string".to_string()), false),
            ("maybe", TypeExpr::Primitive("number".to_string()), true),
        ],
    )];
    let registry = TypeRegistry::from_types(&types).unwrap();
    let zero = registry.zero_value(&TypeExpr::Reference("Partial".to_string()));
    // Optional field is absent from the zero instance (defaults to undefined).
    assert_eq!(zero, serde_json::json!({ "required": "" }));
}

#[test]
fn test_zero_value_cycle_is_total() {
    // A self-referential type must not recurse forever; the cyclic edge is null.
    // (Constructed directly — from_types rejects cycles, but zero_value must be
    // total even if handed one.)
    let mut registry = TypeRegistry::new();
    registry.register_type(
        "Node".to_string(),
        vec![
            TypeField {
                name: "label".to_string(),
                optional: false,
                type_expr: TypeExpr::Primitive("string".to_string()),
            },
            TypeField {
                name: "next".to_string(),
                optional: false,
                type_expr: TypeExpr::Reference("Node".to_string()),
            },
        ],
    );
    let zero = registry.zero_value(&TypeExpr::Reference("Node".to_string()));
    assert_eq!(zero, serde_json::json!({ "label": "", "next": null }));
}

#[test]
fn test_zero_value_unknown_reference_is_null() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.zero_value(&TypeExpr::Reference("Nope".to_string())),
        serde_json::Value::Null
    );
}

#[test]
fn test_check_read_path_delegates() {
    // check_read_path is the production entry to validate_property_path: GREEN on
    // a real field, RED (E0401 + did-you-mean) on a typo.
    let types = vec![create_type(
        "Envelope",
        vec![
            ("email", TypeExpr::Primitive("string".to_string()), false),
            ("valid", TypeExpr::Primitive("boolean".to_string()), false),
        ],
    )];
    let registry = TypeRegistry::from_types(&types).unwrap();
    let ty = TypeExpr::Reference("Envelope".to_string());

    assert!(
        registry
            .check_read_path(&ty, &["email".to_string()], DiagSourceSpan::default())
            .is_ok()
    );

    let err = registry
        .check_read_path(&ty, &["emial".to_string()], DiagSourceSpan::default())
        .unwrap_err();
    assert_eq!(err.code, DiagnosticCode::E0401);
    assert!(
        err.message.contains("emial"),
        "message names the bad field: {}",
        err.message
    );
    // The did-you-mean hint fires for a near-miss.
    let has_hint = err.message.contains("email") || err.notes.iter().any(|n| n.contains("email"));
    assert!(has_hint, "expected a 'did you mean email' hint: {:?}", err);
}
