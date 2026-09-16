//! Tests for compile-time analysis.

use super::*;
use crate::diagnostics::Severity;
use crate::parser::{SourceSpan, StateAst, StateMachineAst, TransitionAst};

// Helper to create type FormMatch with fields as Array of Named
fn make_type_form_match(name: &str, fields: Vec<(&str, bool)>) -> FormMatch {
    let field_values: Vec<CapturedValue> = fields
        .into_iter()
        .map(|(field_name, optional)| {
            let mut map = HashMap::new();
            map.insert(
                "name".to_string(),
                CapturedValue::Ident(field_name.to_string()),
            );
            map.insert("optional".to_string(), CapturedValue::Bool(optional));
            CapturedValue::Named(map)
        })
        .collect();

    FormMatch::new("type")
        .capture("name", CapturedValue::Ident(name.to_string()))
        .capture("fields", CapturedValue::Array(field_values))
}

// Helper to create data FormMatch.
//
// `src` is captured as a quoted `Expr` to mirror what the real `@data <kind>`
// macro (`$src:expr`) produces — NOT a bare `String`. A prior version of this
// helper used `String`, which baked in a shape the parser never emits and let a
// capture-shape regression ship green (BUG-admin-data-source-expr-src).
fn make_data_form_match(name: &str, type_name: &str, src: Option<&str>) -> FormMatch {
    let mut fm = FormMatch::new("data")
        .capture("name", CapturedValue::Ident(name.to_string()))
        .capture("type", CapturedValue::TypeRef(type_name.to_string()));

    if let Some(s) = src {
        fm = fm.capture("src", CapturedValue::Expr(format!("\"{s}\"")));
    }
    fm
}

// Helper to create computed FormMatch
fn make_computed_form_match(
    name: &str,
    type_name: &str,
    from: &str,
    has_where: bool,
    has_sort: bool,
    has_limit: bool,
) -> FormMatch {
    let mut fm = FormMatch::new("computed")
        .capture("name", CapturedValue::Ident(name.to_string()))
        .capture("type", CapturedValue::TypeRef(type_name.to_string()))
        .capture("from", CapturedValue::Ident(from.to_string()));

    if has_where {
        fm = fm.capture("where", CapturedValue::Expr("$.active == true".to_string()));
    }
    if has_sort {
        fm = fm.capture("sort", CapturedValue::Expr("$.price".to_string()));
    }
    if has_limit {
        fm = fm.capture("limit", CapturedValue::Number(10.0));
    }
    fm
}

// Helper to create function FormMatch
fn make_fn_form_match(name: &str, params: Vec<(&str, &str)>, return_type: &str) -> FormMatch {
    let param_values: Vec<CapturedValue> = params
        .into_iter()
        .map(|(pname, ptype)| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), CapturedValue::Ident(pname.to_string()));
            map.insert(
                "type".to_string(),
                CapturedValue::TypeRef(ptype.to_string()),
            );
            CapturedValue::Named(map)
        })
        .collect();

    FormMatch::new("fn")
        .capture("name", CapturedValue::Ident(name.to_string()))
        .capture("params", CapturedValue::Array(param_values))
        .capture(
            "returnType",
            CapturedValue::TypeRef(return_type.to_string()),
        )
}

#[test]
fn test_type_analysis() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![
        make_type_form_match(
            "User",
            vec![("id", false), ("name", false), ("email", true)],
        ),
        make_type_form_match("Post", vec![("title", false), ("body", false)]),
    ];

    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_types(&matches, &registry);

    assert_eq!(analysis.types.len(), 2);

    let user_analysis = analysis.types.iter().find(|t| t.name == "User").unwrap();
    assert_eq!(user_analysis.total_fields, 3);
    assert_eq!(user_analysis.optional_fields, 1);

    let post_analysis = analysis.types.iter().find(|t| t.name == "Post").unwrap();
    assert_eq!(post_analysis.total_fields, 2);
    assert_eq!(post_analysis.optional_fields, 0);
}

#[test]
fn test_data_source_analysis() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![
        make_data_form_match("users", "User[]", Some("/data/users.json")),
        FormMatch::new("data")
            .capture("name", CapturedValue::Ident("cart".to_string()))
            .capture("type", CapturedValue::TypeRef("CartItem[]".to_string()))
            .capture("localStorage", CapturedValue::String("cart".to_string())),
    ];

    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    assert_eq!(analysis.data_sources.len(), 2);

    let users_analysis = analysis
        .data_sources
        .iter()
        .find(|d| d.name == "users")
        .unwrap();
    assert_eq!(users_analysis.type_name, "User[]");
    assert!(users_analysis.is_array);
    assert!(matches!(users_analysis.source, DataSource::File(_)));

    let cart_analysis = analysis
        .data_sources
        .iter()
        .find(|d| d.name == "cart")
        .unwrap();
    assert!(matches!(cart_analysis.source, DataSource::LocalStorage));
}

/// W0205: a `@data` form whose `src` capture is PRESENT but does not resolve to
/// a usable source (the capture-shape-drift fingerprint) must warn rather than
/// silently degrade to `Runtime`. Regression guard for
/// BUG-admin-data-source-expr-src.
#[test]
fn test_w0205_unresolved_data_source_warns() {
    let mut analysis = CompileAnalysis::new();
    // `src` present but a non-literal expression — cannot resolve to a URL.
    let matches = vec![
        FormMatch::new("data")
            .capture("name", CapturedValue::Binding("$broken".to_string()))
            .capture("type", CapturedValue::TypeRef("Item[]".to_string()))
            .capture("src", CapturedValue::Expr("someVar + other".to_string())),
    ];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    assert!(
        matches!(analysis.data_sources[0].source, DataSource::Runtime),
        "unresolved src degrades to Runtime"
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::W0205),
        "unresolved @data src must emit W0205; got {:?}",
        analysis.diagnostics
    );
}

/// A `@data` form with NO `src` capture at all is a legitimate runtime source
/// (e.g. populated imperatively) — it must NOT trigger the W0205 drift warning.
#[test]
fn test_w0205_absent_src_is_silent() {
    let mut analysis = CompileAnalysis::new();
    let matches = vec![make_data_form_match("runtimeOnly", "Item[]", None)];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    assert!(matches!(
        analysis.data_sources[0].source,
        DataSource::Runtime
    ));
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::W0205),
        "absent src must not warn; got {:?}",
        analysis.diagnostics
    );
}

/// A valid quoted-Expr `src` (the real `@data fetch` shape) resolves to a File
/// source and emits no drift warning.
#[test]
fn test_w0205_valid_expr_src_resolves_clean() {
    let mut analysis = CompileAnalysis::new();
    let matches = vec![make_data_form_match(
        "talents",
        "Talent[]",
        Some("/data/talent.json"),
    )];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    assert!(
        matches!(&analysis.data_sources[0].source, DataSource::File(p) if p == "/data/talent.json"),
        "quoted-Expr src resolves to File"
    );
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::W0205),
        "valid src must not warn"
    );
}

#[test]
fn test_computed_analysis() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![make_computed_form_match(
        "activeUsers",
        "User[]",
        "users",
        false,
        false,
        false,
    )];

    analysis.analyze_computed(&matches);

    assert_eq!(analysis.computed.len(), 1);

    let active_users = &analysis.computed[0];
    assert_eq!(active_users.name, "activeUsers");
    assert_eq!(active_users.type_name, "User[]");
    assert_eq!(active_users.dependencies, vec!["users"]);
    assert!(!active_users.has_where);
    assert!(!active_users.has_sort);
    assert!(!active_users.has_limit);
    assert!(!active_users.has_reduce);
}

#[test]
fn test_computed_analysis_with_operations() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![make_computed_form_match(
        "topProducts",
        "Product[]",
        "products",
        true,
        true,
        true,
    )];

    analysis.analyze_computed(&matches);

    assert_eq!(analysis.computed.len(), 1);

    let top_products = &analysis.computed[0];
    assert!(top_products.has_where);
    assert!(top_products.has_sort);
    assert!(top_products.has_limit);
    assert!(!top_products.has_reduce);
}

#[test]
fn test_function_analysis() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![make_fn_form_match(
        "getUser",
        vec![("id", "string")],
        "User",
    )];

    analysis.analyze_functions(&matches);

    assert_eq!(analysis.functions.len(), 1);

    let get_user = &analysis.functions[0];
    assert_eq!(get_user.name, "getUser");
    assert_eq!(get_user.signature, "getUser(id: string): User");
}

#[test]
fn test_template_collection() {
    let mut analysis = CompileAnalysis::new();

    // Create an @each FormMatch with source and template
    let mut each_match = FormMatch::new("each");
    each_match = each_match
        .capture("source", CapturedValue::Binding("$users".to_string()))
        .capture("template", CapturedValue::Ident("user-card".to_string()));

    // Add slot bindings to the invocations
    let invocations = vec![
        {
            let mut slot1 = HashMap::new();
            slot1.insert("slot".to_string(), CapturedValue::Ident("name".to_string()));
            slot1.insert(
                "expr".to_string(),
                CapturedValue::Expr("$.name".to_string()),
            );
            CapturedValue::Named(slot1)
        },
        {
            let mut slot2 = HashMap::new();
            slot2.insert(
                "slot".to_string(),
                CapturedValue::Ident("email".to_string()),
            );
            slot2.insert(
                "expr".to_string(),
                CapturedValue::Expr("$.email".to_string()),
            );
            CapturedValue::Named(slot2)
        },
    ];
    each_match = each_match.capture("invocations", CapturedValue::Array(invocations));

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".user-list".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![each_match],
        span: SourceSpan::default(),
        source_file: None,
        states: Vec::new(),
        html: String::new(),
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_scopes(&scopes);

    assert_eq!(analysis.templates.len(), 1);
    assert_eq!(analysis.bindings.len(), 1);

    let template = &analysis.templates[0];
    assert_eq!(template.name, "user-card");
    assert_eq!(template.slots, vec!["name", "email"]);
    assert!(template.used);

    let binding = &analysis.bindings[0];
    assert_eq!(binding.selector, ".user-list");
    assert_eq!(binding.data_source, "users");
}

#[test]
fn test_state_machine_analysis() {
    let mut analysis = CompileAnalysis::new();

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".editor".to_string(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        behavior: crate::parser::BehaviorBlock {
            state_machine: Some(StateMachineAst {
                initial: "viewing".to_string(),
                inline_states: vec![],
                arrow_transitions: vec![],
            }),
            states: vec![
                StateAst {
                    when: "viewing".to_string(),
                    properties: vec![],
                },
                StateAst {
                    when: "editing".to_string(),
                    properties: vec![],
                },
            ],
            transitions: vec![
                TransitionAst {
                    from: "viewing".to_string(),
                    to: "editing".to_string(),
                    on: "dblclick".to_string(),
                    run: None,
                    debounce_ms: None,
                },
                TransitionAst {
                    from: "editing".to_string(),
                    to: "viewing".to_string(),
                    on: "blur".to_string(),
                    run: None,
                    debounce_ms: None,
                },
            ],
            async_transitions: vec![],
            mutates: vec![],
        },
        matches: vec![],
        span: SourceSpan::default(),
        source_file: None,
        html: String::new(),
        states: vec![],
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_scopes(&scopes);

    assert_eq!(analysis.state_machines.len(), 1);

    let sm = &analysis.state_machines[0];
    assert_eq!(sm.selector, ".editor");
    assert_eq!(sm.initial, "viewing");
    assert_eq!(sm.states.len(), 2);
    assert_eq!(sm.transitions.len(), 2);

    // Both states should have incoming and outgoing transitions
    for state in &sm.states {
        assert!(state.has_incoming);
        assert!(state.has_outgoing);
    }
}

#[test]
fn test_orphan_detection_unused_data() {
    let mut analysis = CompileAnalysis::new();

    // Define data sources using FormMatch
    let matches = vec![
        make_data_form_match("users", "User[]", None),
        make_data_form_match("posts", "Post[]", None),
    ];

    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    // Create a binding that only uses "users"
    analysis.bindings.push(BindingAnalysis {
        selector: ".user-list".to_string(),
        data_source: "users".to_string(),
        instance_count: None,
    });

    analysis.detect_orphans();

    // Should have one warning for unused "posts"
    let warnings: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Warning))
        .collect();

    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, DiagnosticCode::W0201);
    assert!(warnings[0].message.contains("posts"));
}

#[test]
fn test_orphan_detection_unreachable_state() {
    let mut analysis = CompileAnalysis::new();

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".widget".to_string(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        behavior: crate::parser::BehaviorBlock {
            state_machine: Some(StateMachineAst {
                initial: "idle".to_string(),
                inline_states: vec![],
                arrow_transitions: vec![],
            }),
            states: vec![
                StateAst {
                    when: "idle".to_string(),
                    properties: vec![],
                },
                StateAst {
                    when: "active".to_string(),
                    properties: vec![],
                },
                StateAst {
                    when: "error".to_string(),
                    properties: vec![],
                },
            ],
            transitions: vec![TransitionAst {
                from: "idle".to_string(),
                to: "active".to_string(),
                on: "click".to_string(),
                run: None,
                debounce_ms: None,
            }],
            async_transitions: vec![],
            mutates: vec![],
        },
        matches: vec![],
        span: SourceSpan::default(),
        source_file: None,
        html: String::new(),
        states: vec![],
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_scopes(&scopes);
    analysis.detect_orphans();

    // Should have warnings for states with no incoming/outgoing transitions
    let warnings: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Warning))
        .collect();

    // "error" has no incoming, "active" has no outgoing
    assert!(warnings.len() >= 2);
}

#[test]
fn test_report_generation() {
    let mut analysis = CompileAnalysis::new();

    // Add some test data
    analysis.types.push(TypeAnalysis {
        name: "User".to_string(),
        total_fields: 3,
        optional_fields: 1,
        used: true,
    });

    analysis.data_sources.push(DataAnalysis {
        name: "users".to_string(),
        type_name: "User".to_string(),
        is_array: true,
        source: DataSource::File("/data/users.json".to_string()),
        item_count: Some(5),
        validated: true,
        used: true,
    });

    let report = analysis.generate_report("test-project");

    assert!(report.contains("SPACETIME COMPILE REPORT: test-project"));
    assert!(report.contains("Types:"));
    assert!(report.contains("User (3 fields, 1 optional)"));
    assert!(report.contains("Data Sources:"));
    assert!(report.contains("users: User[]"));
    assert!(report.contains("5 items"));
    assert!(report.contains("WARNINGS: 0 | ERRORS: 0"));
    assert!(report.contains("BUILD SUCCEEDED"));
}

#[test]
fn test_validate_computed_dependencies() {
    let mut analysis = CompileAnalysis::new();

    analysis.data_sources.push(DataAnalysis {
        name: "users".to_string(),
        type_name: "User".to_string(),
        is_array: true,
        source: DataSource::Runtime,
        item_count: None,
        validated: false,
        used: true,
    });

    analysis.computed.push(ComputedAnalysis {
        name: "activeUsers".to_string(),
        type_name: "User[]".to_string(),
        dependencies: vec!["users".to_string(), "nonexistent".to_string()],
        has_where: true,
        has_sort: false,
        has_limit: false,
        has_reduce: false,
    });

    analysis.validate_computed_dependencies();

    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0405);
    assert!(errors[0].message.contains("nonexistent"));
}

#[test]
fn test_validate_binding_sources() {
    let mut analysis = CompileAnalysis::new();

    analysis.data_sources.push(DataAnalysis {
        name: "users".to_string(),
        type_name: "User".to_string(),
        is_array: true,
        source: DataSource::Runtime,
        item_count: None,
        validated: false,
        used: true,
    });

    analysis.bindings.push(BindingAnalysis {
        selector: ".user-list".to_string(),
        data_source: "users".to_string(),
        instance_count: None,
    });

    analysis.bindings.push(BindingAnalysis {
        selector: ".post-list".to_string(),
        data_source: "posts".to_string(), // Doesn't exist
        instance_count: None,
    });

    analysis.validate_binding_sources();

    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, DiagnosticCode::E0405);
    assert!(errors[0].message.contains("posts"));
}

#[test]
fn test_cycle_marks_data_source_as_used() {
    let mut analysis = CompileAnalysis::new();

    // Define data source
    let matches = vec![make_data_form_match("heroTerms", "HeroTerm[]", None)];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    // Create a scope with @cycle consuming heroTerms
    let cycle_fm =
        FormMatch::new("cycle").capture("source", CapturedValue::Ident("heroTerms".to_string()));

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".hero__flap".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![cycle_fm],
        span: SourceSpan::default(),
        source_file: None,
        states: Vec::new(),
        html: String::new(),
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_scopes(&scopes);
    analysis.detect_orphans();

    // heroTerms should NOT trigger W0201 (it's used by @cycle)
    let w0201: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .collect();
    assert!(
        w0201.is_empty(),
        "Expected no W0201 when @cycle consumes the data source, got: {:?}",
        w0201
    );
}

#[test]
fn test_cycle_marks_type_as_used() {
    let mut analysis = CompileAnalysis::new();

    // Define type and data source
    let matches = vec![
        make_type_form_match("HeroTerm", vec![("text", false)]),
        make_data_form_match("heroTerms", "HeroTerm[]", None),
    ];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_types(&matches, &registry);
    analysis.analyze_data_sources(&matches, &registry);

    // Create a scope with @cycle consuming heroTerms
    let cycle_fm =
        FormMatch::new("cycle").capture("source", CapturedValue::Ident("heroTerms".to_string()));

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".hero__flap".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![cycle_fm],
        span: SourceSpan::default(),
        source_file: None,
        states: Vec::new(),
        html: String::new(),
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_scopes(&scopes);
    analysis.detect_orphans();

    // HeroTerm should NOT trigger W0203 (it's used by heroTerms which is used by @cycle)
    let w0203: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0203)
        .collect();
    assert!(
        w0203.is_empty(),
        "Expected no W0203 when type is used by @cycle-consumed data source, got: {:?}",
        w0203
    );
}

#[test]
fn test_unused_data_source_still_warns() {
    let mut analysis = CompileAnalysis::new();

    // Define data source with neither @each nor @cycle
    let matches = vec![make_data_form_match("orphanData", "Orphan[]", None)];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    // Empty scopes — no @each, no @cycle
    analysis.analyze_scopes(&[]);
    analysis.detect_orphans();

    // orphanData should trigger W0201
    let w0201: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .collect();
    assert_eq!(w0201.len(), 1);
    assert!(w0201[0].message.contains("orphanData"));
}

// =============================================================================
// BUG-207 — W0201 false positive: a @data source read only via `text:` bindings
// and `@handle` arms is flagged "never used", while a source that ALSO feeds an
// @each/@cycle/computed (a subscriber-like usage) is not. detect_orphans only
// counts subscriber-like positions; text/handle READ positions are missed.
// These two tests are RED until the usage scan is extended (see Impl Plan).
// =============================================================================

#[test]
fn test_text_binding_marks_data_source_as_used() {
    let mut analysis = CompileAnalysis::new();

    // A @data source consumed ONLY by a `text:` binding (a css_declaration whose
    // value is `$error`). No @each, no @cycle, no computed dependency.
    let matches = vec![make_data_form_match("error", "string", None)];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".error-view".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![crate::parser::CssDeclaration {
            property: "text".to_string(),
            value: "$error".to_string(),
            is_injection: false,
            span: SourceSpan::default(),
        }],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![],
        span: SourceSpan::default(),
        source_file: None,
        exports: vec![],
        refs: vec![],
        states: vec![],
        html: String::new(),
    }];

    analysis.analyze_scopes(&scopes);
    analysis.analyze_read_usages(&scopes, &matches);
    analysis.detect_orphans();

    // `error` IS used (its value flows into .error-view's text). W0201 must NOT fire.
    let w0201: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .collect();
    assert!(
        w0201.is_empty(),
        "a @data source read via `text: $error` must be marked used (BUG-207), got: {:?}",
        w0201
    );
}

#[test]
fn test_handle_arm_marks_data_source_as_used() {
    let mut analysis = CompileAnalysis::new();

    // A @data signal consumed by a `@handle $error { ... }` directive. The handle's
    // `signal` binding names the consumed source; its arm bodies may read more $vars.
    // In the real pipeline `@handle` is a flat directive in `ast.matches`; mirror
    // that here so `analyze_read_usages` sees it via the `matches` arg.
    let handle_fm =
        FormMatch::new("handle").capture("signal", CapturedValue::Binding("$error".to_string()));
    let matches = vec![
        make_data_form_match("error", "string", None),
        handle_fm.clone(),
    ];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: ".error-panel".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![handle_fm],
        span: SourceSpan::default(),
        source_file: None,
        exports: vec![],
        refs: vec![],
        states: vec![],
        html: String::new(),
    }];

    analysis.analyze_scopes(&scopes);
    analysis.analyze_read_usages(&scopes, &matches);
    analysis.detect_orphans();

    let w0201: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .collect();
    assert!(
        w0201.is_empty(),
        "a @data source consumed by `@handle $error` must be marked used (BUG-207), got: {:?}",
        w0201
    );
}

// FEAT-088: analyze_dispatch emits E0923 for an ambiguous call, nothing otherwise.
#[test]
fn analyze_dispatch_emits_e0923_on_tie() {
    use crate::metasystem::MetaRegistry;
    use crate::parser::meta_ast::{
        CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, MacroDefAst,
    };

    fn widget_macro(name: &str, two_caps: bool) -> MacroDefAst {
        let mut inline = vec![FormInlineElement::Capture(
            FormCapture {
                var_name: "x".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
                alias_capture: None,
            },
            None,
        )];
        if two_caps {
            inline.push(FormInlineElement::Capture(
                FormCapture {
                    var_name: "y".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            ));
        }
        MacroDefAst {
            retired: None,
            name: name.to_string(),
            form: Some(FormClause {
                directive_name: "widget".to_string(),
                inline_elements: inline,
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
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

    let call = {
        let mut caps = HashMap::new();
        caps.insert("x".to_string(), CapturedValue::Ident("hi".to_string()));
        FormMatch {
            macro_name: "widget".to_string(),
            matched_macro: None,
            captures: caps,
            ..Default::default()
        }
    };

    // Ambiguous pair → E0923 fires once.
    let mut reg = MetaRegistry::new();
    reg.register_macro(widget_macro("widget-a", false)).unwrap();
    reg.register_macro(widget_macro("widget-b", false)).unwrap();
    let mut analysis = CompileAnalysis::new();
    analysis.analyze_dispatch(std::slice::from_ref(&call), &reg);
    let hits: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0923)
        .collect();
    assert_eq!(hits.len(), 1, "ambiguous call must emit one E0923");
    assert!(hits[0].message.contains("widget-a") && hits[0].message.contains("widget-b"));

    // Disambiguated pair → no E0923.
    let mut reg2 = MetaRegistry::new();
    reg2.register_macro(widget_macro("widget-a", false))
        .unwrap();
    reg2.register_macro(widget_macro("widget-b", true)).unwrap();
    let mut analysis2 = CompileAnalysis::new();
    analysis2.analyze_dispatch(std::slice::from_ref(&call), &reg2);
    assert!(
        !analysis2
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::E0923),
        "disambiguated dispatch must not warn"
    );
}

/// BUG-126: a macro whose outputs come solely from `%binds { prim(..) -> { $a } }`
/// (no `%registers binding($x)`) must have its yields registered as referenceable
/// signals — read from the registry, NOT a hardcoded primitive list. Before the
/// fix, `register_macro_yield_signals` did not exist and such yields raised E0405.
#[test]
fn test_bug126_binds_yields_register_as_signals() {
    use crate::metasystem::MetaRegistry;
    use crate::parser::meta_ast::{
        BindArg, BindDecl, BindOutput, BindValue, CaptureModifier, CaptureType, FormCapture,
        FormClause, FormInlineElement, MacroDefAst,
    };

    // A macro `@peers(room: ...) as $name` whose ONLY outputs are %binds yields:
    //   presence(&self, room) -> { $users, $myId, $set }
    // plus an aliased yield `${$name}_count` to exercise capture interpolation.
    let macro_def = MacroDefAst {
        retired: None,
        name: "peers-test".to_string(),
        form: Some(FormClause {
            directive_name: "peers".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "name".to_string(),
                    capture_type: CaptureType::Binding,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        }),
        binds: vec![BindDecl {
            primitive: "presence".to_string(),
            args: vec![BindArg::Positional(BindValue::Variable("self".to_string()))],
            outputs: vec![
                BindOutput {
                    name: "$users".to_string(),
                    alias: None,
                },
                BindOutput {
                    name: "$set".to_string(),
                    alias: None,
                },
                BindOutput {
                    name: "count".to_string(),
                    alias: Some("${$name}_count".to_string()),
                },
            ],
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

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    // The invocation: `@peers ... as $peers` (capture `name` = $peers).
    let call =
        FormMatch::new("peers").capture("name", CapturedValue::Binding("$peers".to_string()));

    let mut analysis = CompileAnalysis::new();
    analysis.register_macro_yield_signals(std::slice::from_ref(&call), &registry);

    let known = analysis.reactive_source_names();
    assert!(
        known.contains("users"),
        "literal yield $users must register (got {:?})",
        known
    );
    assert!(known.contains("set"), "literal yield $set must register");
    assert!(
        known.contains("peers_count"),
        "aliased yield ${{$name}}_count must resolve to peers_count (got {:?})",
        known
    );

    // GREEN: a binding that references the now-known yield does NOT error.
    analysis.bindings.push(crate::analysis::BindingAnalysis {
        selector: ".stage".to_string(),
        data_source: "users".to_string(),
        instance_count: None,
    });
    // RED control: a genuinely-absent binding still errors.
    analysis.bindings.push(crate::analysis::BindingAnalysis {
        selector: ".stage".to_string(),
        data_source: "absent".to_string(),
        instance_count: None,
    });
    analysis.validate_binding_sources();
    let errs: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0405)
        .collect();
    assert_eq!(
        errs.len(),
        1,
        "only the absent binding errors, not the yield"
    );
    assert!(errs[0].message.contains("absent"));
}

/// BUG-155-adjacent: `validate_binding_sources` must resolve a DOTTED-PATH
/// binding source (`@each($wrap.items as $x)` -> `data_source: "wrap.items"`)
/// against its HEAD segment, not the literal dotted string. Before the fix, a
/// dotted-path @each ALWAYS raised a false-positive E0405 ("references
/// 'wrap.items', which does not exist") even when `$wrap` was a perfectly
/// valid, known reactive source — the runtime resolves the SAME shape
/// correctly (see BUG-155's `ST.resolvePath` fix in each-with-templates); the
/// static analysis must agree with the runtime about what a valid source is.
#[test]
fn test_bug155_dotted_path_binding_source_resolves_via_head() {
    let mut analysis = CompileAnalysis::new();

    // $wrap is a known reactive source (e.g. a @data inline local).
    analysis.locals.push(crate::analysis::LocalAnalysis {
        name: "wrap".to_string(),
        type_name: "object".to_string(),
        is_array: false,
    });

    // GREEN: a dotted-path binding into a KNOWN head must NOT error.
    analysis.bindings.push(crate::analysis::BindingAnalysis {
        selector: ".dotted".to_string(),
        data_source: "wrap.items".to_string(),
        instance_count: None,
    });
    // RED control: a dotted path into an UNKNOWN head must still error (the fix
    // must not blanket-suppress E0405 for every dotted string).
    analysis.bindings.push(crate::analysis::BindingAnalysis {
        selector: ".dotted-bad".to_string(),
        data_source: "absent.items".to_string(),
        instance_count: None,
    });

    analysis.validate_binding_sources();
    let errs: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0405)
        .collect();
    assert_eq!(
        errs.len(),
        1,
        "only the absent-head dotted binding errors, not the known-head one (got {:?})",
        errs
    );
    assert!(errs[0].message.contains("absent.items"));
}

// =============================================================================
// PLAN-119 W2 (FUP-150) — field-check typed dotted reads (E0401)
// =============================================================================

// Build a scope whose single css declaration carries `value` (the read expr).
fn scope_with_read(selector: &str, value: &str) -> ScopeBlock {
    ScopeBlock {
        selector: selector.to_string(),
        css_declarations: vec![crate::parser::CssDeclaration {
            property: "text".to_string(),
            value: value.to_string(),
            is_injection: false,
            span: SourceSpan::default(),
        }],
        ..Default::default()
    }
}

// A FormEnvelope-shaped registry: FormEnvelope { errors: FormErrors, valid: bool },
// FormErrors { email: string }.
fn form_envelope_registry() -> crate::type_system::TypeRegistry {
    use crate::parser::{TypeDef, TypeExpr, TypeField};
    let types = vec![
        TypeDef {
            name: "FormErrors".to_string(),
            fields: vec![TypeField {
                name: "email".to_string(),
                optional: false,
                type_expr: TypeExpr::Primitive("string".to_string()),
            }],
            variants: vec![],
            span: SourceSpan::default(),
        },
        TypeDef {
            name: "FormEnvelope".to_string(),
            fields: vec![
                TypeField {
                    name: "errors".to_string(),
                    optional: false,
                    type_expr: TypeExpr::Reference("FormErrors".to_string()),
                },
                TypeField {
                    name: "valid".to_string(),
                    optional: false,
                    type_expr: TypeExpr::Primitive("boolean".to_string()),
                },
            ],
            variants: vec![],
            span: SourceSpan::default(),
        },
    ];
    crate::type_system::TypeRegistry::from_types(&types).unwrap()
}

#[test]
fn test_validate_typed_reads_flags_bad_field() {
    // A typed subscribe `$form FormEnvelope` + a read of a NESTED bad field.
    let mut analysis = CompileAnalysis::new();
    let registry = form_envelope_registry();
    analysis.analyze_data_sources(
        &[make_data_form_match("form", "FormEnvelope", None)],
        &registry,
    );
    let scopes = vec![
        scope_with_read(".ok", "$form.errors.email"),
        scope_with_read(".bad", "$form.errors.emial"),
    ];
    analysis.validate_typed_reads(&registry, &scopes);

    let errs: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0401)
        .collect();
    assert_eq!(
        errs.len(),
        1,
        "exactly the typo'd nested read errors, not the good one (got {:?})",
        errs
    );
    assert!(errs[0].message.contains("emial"));
}

#[test]
fn test_validate_typed_reads_untyped_base_is_silent() {
    // Regression: an UNTYPED data source (`type_name == "any"`, pre-FUP-144
    // default) must NOT be field-checked — the pass is purely additive.
    let mut analysis = CompileAnalysis::new();
    let registry = form_envelope_registry();
    // No `type` capture → type_name defaults to "any".
    analysis.analyze_data_sources(
        &[FormMatch::new("data").capture("name", CapturedValue::Ident("form".to_string()))],
        &registry,
    );
    let scopes = vec![scope_with_read(".bad", "$form.anything.at.all")];
    analysis.validate_typed_reads(&registry, &scopes);

    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|d| d.code != DiagnosticCode::E0401),
        "untyped base must not be field-checked: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_validate_typed_reads_bare_base_is_valid() {
    // A bare `$form` read (no dotted path) is trivially valid — nothing to walk.
    let mut analysis = CompileAnalysis::new();
    let registry = form_envelope_registry();
    analysis.analyze_data_sources(
        &[make_data_form_match("form", "FormEnvelope", None)],
        &registry,
    );
    let scopes = vec![scope_with_read(".ok", "$form")];
    analysis.validate_typed_reads(&registry, &scopes);
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|d| d.code != DiagnosticCode::E0401),
        "bare base read must be valid: {:?}",
        analysis.diagnostics
    );
}

// =============================================================================
// GH-26 — E0408 (signal used but not defined) edge tests
// =============================================================================

/// Runs the production analysis path on `src` and returns the diagnostics.
fn analyze_src(src: &str) -> Vec<crate::diagnostics::Diagnostic> {
    let ast = crate::parser::parse(src).expect("parse");
    let mut analysis = CompileAnalysis::new();
    // Mirror check_file's ordering: register %binds yields BEFORE the E0408 pass
    // so a macro/template-published signal is a known source.
    let (registry, _) = crate::compiler::cached_stdlib_registry();
    analysis.register_macro_yield_signals(&ast.matches, &registry);
    analysis.analyze_locals(&ast.matches);
    analysis.analyze_data_sources(&ast.matches, &crate::type_system::TypeRegistry::new());
    analysis.analyze_computed(&ast.matches);
    analysis.analyze_signals_from_ast(&ast);
    analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0408)
        .cloned()
        .collect()
}

fn has_e0408(src: &str, name: &str) -> bool {
    analyze_src(src)
        .iter()
        .any(|d| d.code == DiagnosticCode::E0408 && d.message.contains(name))
}

#[test]
fn e0408_fires_for_genuinely_undefined_signal() {
    // The GH-26 shape: a binding read of a never-declared, never-written signal.
    assert!(
        has_e0408(
            "@data inline $present : \"ok\";\n.probe { text <- $definitelyMissing; }",
            "definitelyMissing"
        ),
        "an undefined signal read in a binding must raise E0408"
    );
}

#[test]
fn e0408_does_not_fire_for_parent_scope_signal() {
    // A primitive yield in an enclosing scope is visible to a nested scope.
    assert!(
        !has_e0408(
            "@import \"stdlib\"\n.card { @scroll fade(start: 0, end: 1) { opacity: 0 -> 1; } .child { opacity: $progress; } }",
            "progress"
        ),
        "a signal provided by a parent scope must not be flagged undefined"
    );
}

#[test]
fn e0408_does_not_fire_for_template_published_signal() {
    // A template's %binds yield ($factory) is registered before the E0408 pass.
    assert!(
        !has_e0408(
            "@import \"stdlib\"\n@template &probe($n) { <div class=\"p\"></div> }\n.card { text <- $factory; }",
            "factory"
        ),
        "a signal published by a template must not be flagged undefined"
    );
}

#[test]
fn e0408_does_not_fire_for_local_state_signal() {
    // `@state $x` / `$x string: v` declares a signal — reads are valid.
    assert!(
        !has_e0408(
            "$label string: \"hello\";\n.text-display { text <- $label; }",
            "label"
        ),
        "a locally-declared state signal must not be flagged undefined"
    );
}

// =============================================================================
// Analysis-coverage class test (PLAN-137 W2): every `pub fn analyze_*` in
// src/analysis has a production caller reachable from check_file. This is the
// class fix behind GH-26/#25 — a correct diagnostic with no consumer is the
// silent-acceptance failure mode. New analyses must be wired into check_file
// (directly or transitively) or this test fails.
// =============================================================================

#[test]
fn every_pub_analyze_fn_has_a_production_caller_reachable_from_check_file() {
    let root = env!("CARGO_MANIFEST_DIR");
    let analysis_dir = std::path::Path::new(root).join("src/analysis");
    let main_path = std::path::Path::new(root).join("src/main.rs");

    // Collect every `pub fn analyze_*` name across src/analysis/.
    let mut analyze_fns: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in std::fs::read_dir(&analysis_dir).unwrap() {
        let path = entry.unwrap().path();
        let is_tests = path.file_name().and_then(|n| n.to_str()) == Some("tests.rs");
        if path.extension().map(|e| e == "rs").unwrap_or(false) && !is_tests {
            let src = std::fs::read_to_string(&path).unwrap();
            // `pub fn analyze_foo` — match any `pub fn analyze_<ident>`.
            let mut rest = src.as_str();
            while let Some(idx) = rest.find("pub fn analyze_") {
                let start = idx + "pub fn analyze_".len();
                let name: String = rest[start..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    analyze_fns.insert(name);
                }
                rest = &rest[start..];
            }
        }
    }
    assert!(!analyze_fns.is_empty(), "no analyze_* fns discovered");

    // Direct callers in main.rs: `analysis.analyze_x(` or bare `analyze_x(`.
    let main_src = std::fs::read_to_string(&main_path).unwrap();
    let mut directly_called: std::collections::HashSet<String> = std::collections::HashSet::new();
    for name in &analyze_fns {
        let needle_dot = format!("analyze_{}(", name);
        let needle_bare = format!("analyze_{}(", name);
        if main_src.contains(&needle_dot) || main_src.contains(&needle_bare) {
            directly_called.insert(name.clone());
        }
    }
    assert!(
        !directly_called.is_empty(),
        "no analyze_* fn is called from main.rs at all — check_file wires nothing"
    );

    // Intra-analysis call graph: does any analyze_* body call another?
    // (e.g. analyze_timelines -> analyze_timelines_with_config,
    //       analyze_signals_with_diagnostics -> analyze_signals)
    let mut callee_of: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    // Record calls between analyze_* fns anywhere in this file. For each
    // `analyze_<name>` occurrence, the enclosing fn is the nearest `fn ` BEFORE
    // it in the FULL file text (absolute offset — a shrinking window would miss
    // declarations that precede the scan position).
    for entry in std::fs::read_dir(&analysis_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            let src = std::fs::read_to_string(&path).unwrap();
            let mut search_from = 0usize;
            while let Some(idx) = src[search_from..].find("analyze_") {
                let abs = search_from + idx;
                let start = abs + "analyze_".len();
                let name: String = src[start..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && analyze_fns.contains(&name) {
                    let before = &src[..abs];
                    if let Some(pos) = before.rfind("fn ") {
                        let caller: String = before[pos + 3..]
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect();
                        let caller_norm: &str = caller.strip_prefix("analyze_").unwrap_or(&caller);
                        if analyze_fns.contains(caller_norm) {
                            callee_of
                                .entry(caller_norm.to_string())
                                .or_default()
                                .push(name.clone());
                        }
                    }
                }
                search_from = abs + name.len() + 1;
            }
        }
    }

    // Reachability: start from main.rs direct callers, follow intra-analysis edges.
    let mut reachable = directly_called.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for caller in reachable.clone() {
            if let Some(callees) = callee_of.get(&caller) {
                for c in callees {
                    if reachable.insert(c.clone()) {
                        changed = true;
                    }
                }
            }
        }
    }

    let unreachable: Vec<&String> = analyze_fns
        .iter()
        .filter(|n| !reachable.contains(*n))
        .collect();
    assert!(
        unreachable.is_empty(),
        "pub fn analyze_* with no production caller reachable from check_file: {:?} \
         (GH-26/#25 class defect: a diagnostic with no consumer). Wire it into \
         check_file or this test fails.",
        unreachable
    );
}

#[test]
fn e0408_does_not_fire_for_imported_known_source() {
    // A signal defined by an imported module surfaces as a known reactive
    // source (a data source / %binds yield) — `@data inline` in an imported
    // module lands in data_sources, which the E0408 pass excludes.
    assert!(
        !has_e0408(
            "@import \"stdlib\"\n@data inline $greeting : \"hi\";\n.card { text <- $greeting; }",
            "greeting"
        ),
        "a signal provided by an imported module must not be flagged undefined"
    );
}

// =============================================================================
// BUG-380 / BUG-381 — W0201 false positives: `@on` motion-body consumption and
// `@view`/`@match` subjects are invisible to `analyze_read_usages`.
//
// These parse REAL source (and one EDN document through the real ingress) and
// run the production check chain, so the captures under test are the shapes the
// parser/reader ACTUALLY produce — never a hand-baked map (the
// BUG-admin-data-source-expr-src lesson). RED until analyze_read_usages learns
// the on_motion_body entry shapes and the view/match `subject` binding.
// =============================================================================

/// The W0201 messages the production check chain emits for an already-lifted
/// StFile. Mirrors check_file's ordering: data sources first, then scopes, then
/// read-usages, then orphan detection.
fn w0201_messages(ast: &crate::parser::StFile) -> Vec<String> {
    let mut analysis = CompileAnalysis::new();
    let registry = crate::type_system::TypeRegistry::new();
    analysis.analyze_data_sources(&ast.matches, &registry);
    analysis.analyze_scopes(&ast.scopes);
    analysis.analyze_read_usages(&ast.scopes, &ast.matches);
    analysis.detect_orphans();
    analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .map(|d| d.message.clone())
        .collect()
}

fn w0201_from_st(src: &str) -> Vec<String> {
    let ast = crate::parser::parse(src).expect("parse");
    w0201_messages(&ast)
}

fn w0201_from_edn(src: &str) -> Vec<String> {
    let ast = crate::edn::to_st_file(src, &crate::syntax::STDLIB_REGISTRY).expect("edn ingress");
    w0201_messages(&ast)
}

fn assert_no_w0201_for(messages: &[String], name: &str, ctx: &str) {
    assert!(
        !messages.iter().any(|m| m.contains(&format!("'{name}'"))),
        "{ctx}: `{name}` is consumed and must not warn W0201, got: {messages:?}"
    );
}

#[test]
fn on_signal_call_marks_fired_signal_and_args_used_bug380() {
    // The composer shape: `$send($draft);` reads BOTH the fired signal and its
    // argument. Both were flagged before the fix.
    let msgs = w0201_from_st(
        "@data inline $draft : \"\";\n\
         @data inline $send : null;\n\
         .composer { @on &.click { $send($draft); } }",
    );
    assert_no_w0201_for(&msgs, "send", "@on signal call");
    assert_no_w0201_for(&msgs, "draft", "@on signal call");
}

#[test]
fn on_mutation_marks_target_and_expr_used_bug380() {
    // `$flag <- !$flag` — a handler WRITES the signal; a write keeps it alive.
    // This is the case handler_body_signal_deps' own doc claims to cover, and
    // does not: @on's body is on_motion_body, which never carries js_statements
    // at analysis time.
    let msgs = w0201_from_st(
        "@data inline $flag : false;\n\
         .box { @on &.click { $flag <- !$flag; } }",
    );
    assert_no_w0201_for(&msgs, "flag", "@on mutation");
}

#[test]
fn view_subject_marks_source_used_bug381() {
    let msgs = w0201_from_st(
        "@data inline $status : \"idle\";\n\
         @template &bubble($m) { <div class='bubble'>`$m`</div> }\n\
         .status { @view $status { _ => &bubble(); } }",
    );
    assert_no_w0201_for(&msgs, "status", "@view subject");
}

#[test]
fn match_subject_marks_source_used_bug381() {
    // @match is the same dispatch with reactive: false — the subject is read
    // once at mount instead of on every change, but it IS read.
    let msgs = w0201_from_st(
        "@data inline $mode : \"list\";\n\
         @template &panel($m) { <div class='panel'>`$m`</div> }\n\
         .mode { @match $mode { _ => &panel(); } }",
    );
    assert_no_w0201_for(&msgs, "mode", "@match subject");
}

#[test]
fn dotted_view_subject_marks_base_source_used_bug381() {
    // `dispatch-mount` documents dotted subjects (`$chat.status`); the read is
    // of the BASE source.
    let msgs = w0201_from_st(
        "@data inline $chat : null;\n\
         @template &bubble($m) { <div class='bubble'>`$m`</div> }\n\
         .status { @view $chat.status { _ => &bubble(); } }",
    );
    assert_no_w0201_for(&msgs, "chat", "dotted @view subject");
}

#[test]
fn edn_page_on_and_view_consumption_marks_sources_used_bug380_381() {
    // The exact shape spell's interface emits for its seam page — the document
    // whose W0201s forced the downstream `@exempt_sources` list. One document
    // exercises BOTH fixes through the EDN ingress: the fired signal + its arg
    // (BUG-380) and the view subject (BUG-381).
    let msgs = w0201_from_edn(
        "{:st/forms [(data-inline :name $draft :type string :value [:st/expr \"\\\"\\\"\"])\n\
         \x20(data-inline :name $send :type signal :value [:st/expr \"null\"])\n\
         \x20(data-inline :name $status :type string :value [:st/expr \"\\\"idle\\\"\"])\n\
         \x20(template :name bubble :params [:st/paramlist [\"m\" :binding false nil nil false]] :body [:st/component-body])\n\
         \x20(sel \".composer\" (on-driver-body :driver [:st/named {\"member\" click \"subject\" \"\"}] :body [:st/array [:st/named {\"stmt\" [:st/named {\"args\" [:st/array [:st/named {\"value\" [:st/expr \"$draft\"]}]] \"signal\" send}]}]]))\n\
         \x20(sel \".status\" (view :subject $status :arms [:st/array [:st/named {\"inv\" [:st/named {\"args\" [:st/array] \"name\" bubble}] \"pat\" [:st/named {\"wild\" _}]}]]))]\n\
         \x20:st/constructs [{:kind \"template\" :selector \"@template:bubble\" :html \"<div class='bubble'>`$m`</div>\" :decls {}}]}",
    );
    assert_no_w0201_for(&msgs, "draft", "EDN @on signal call");
    assert_no_w0201_for(&msgs, "send", "EDN @on signal call");
    assert_no_w0201_for(&msgs, "status", "EDN @view subject");
}

#[test]
fn genuinely_unused_source_still_warns_via_parse() {
    // The negative the fix must not lose: a source nothing consumes still warns.
    // (Companion to the hand-constructed test_unused_data_source_still_warns —
    // this one proves the parse-path chain, not just the data structure.)
    let msgs = w0201_from_st("@data inline $unused : \"x\";");
    assert!(
        msgs.iter().any(|m| m.contains("'unused'")),
        "a genuinely unused source must keep warning, got: {msgs:?}"
    );
}
#[test]
fn test_template_html_hole_marks_data_source_as_used() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![make_data_form_match("partial", "String", None)];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    // A template whose body reads `$partial` — and nothing else touches it.
    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: "@template:streaming".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![],
        span: SourceSpan::default(),
        source_file: None,
        states: Vec::new(),
        html: r#"<article class="bubble"><p class="bubble__text">$partial</p></article>"#
            .to_string(),
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_read_usages(&scopes, &[]);
    analysis.detect_orphans();

    let w0201: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .collect();
    assert!(
        w0201.is_empty(),
        "a $name hole in a template body is a runtime read — no W0201 expected, got: {:?}",
        w0201
    );
}

#[test]
fn test_template_html_escaped_dollar_is_not_a_read() {
    let mut analysis = CompileAnalysis::new();

    let matches = vec![make_data_form_match("now", "String", None)];
    let registry = crate::type_system::TypeRegistry::from_types(&[]).unwrap();
    analysis.analyze_data_sources(&matches, &registry);

    // `$$now` renders the literal text `$now` (BUG-112) — it is NOT a read.
    let scopes = vec![ScopeBlock {
        kind: Default::default(),
        selector: "@template:doc".to_string(),
        behavior: crate::parser::BehaviorBlock::default(),
        css_declarations: vec![],
        form_refs: Vec::new(),
        nested_scopes: vec![],
        matches: vec![],
        span: SourceSpan::default(),
        source_file: None,
        states: Vec::new(),
        html: r#"<code>.clock { text <- $$now; }</code>"#.to_string(),
        exports: vec![],
        refs: vec![],
    }];

    analysis.analyze_read_usages(&scopes, &[]);
    analysis.detect_orphans();

    let w0201: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::W0201)
        .collect();
    assert_eq!(
        w0201.len(),
        1,
        "$$ is the literal-dollar escape — the source stays unused: {:?}",
        w0201
    );
}
