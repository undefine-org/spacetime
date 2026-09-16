//! FormRegistry tests for the Spacetime LSP.
//!
//! Tests the extraction of directive signatures from macro definitions
//! and the LSP-related functionality like completions and lookups.

use std::path::Path;

use spacetime::lsp::{DirectiveParam, DirectiveSignature, FormRegistry};
use spacetime::metasystem::MetaRegistry;
use spacetime::parser::SourceSpan;
use spacetime::parser::meta_ast::{
    CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, FormParam,
    MacroDefAst, ParamDefault,
};

// =============================================================================
// Helper Functions
// =============================================================================

fn make_form_clause(
    directive_name: &str,
    inline_elements: Vec<FormInlineElement>,
    params: Vec<FormParam>,
    has_body: bool,
) -> FormClause {
    FormClause {
        directive_name: directive_name.to_string(),
        inline_elements,
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

fn make_macro(name: &str, _creates: Option<&str>, form: Option<FormClause>) -> MacroDefAst {
    MacroDefAst {
        name: name.to_string(),
        form,
        binds: vec![],
        derives: vec![],
        states: None,
        registers: None,
        imports: None,
        resolves: None,
        scopes: vec![],
        scope_within: vec![],
        order: None,
        body: vec![],
        requires: vec![],
        span: SourceSpan::default(),
        source_file: None,
        module: None,
        doc: None,
        retired: None,
        ..Default::default()
    }
}

fn make_form_param(
    name: &str,
    var_name: &str,
    capture_type: CaptureType,
    modifier: CaptureModifier,
    default: Option<ParamDefault>,
) -> FormParam {
    FormParam {
        name: name.to_string(),
        elements: vec![FormInlineElement::Capture(
            FormCapture {
                var_name: var_name.to_string(),
                capture_type,
                modifier,
                alias_capture: None,
            },
            None,
        )],
        default,
    }
}

fn make_inline_capture(
    var_name: &str,
    capture_type: CaptureType,
    modifier: CaptureModifier,
    default: Option<ParamDefault>,
) -> FormInlineElement {
    FormInlineElement::Capture(
        FormCapture {
            var_name: var_name.to_string(),
            capture_type,
            modifier,
            alias_capture: None,
        },
        default,
    )
}

// =============================================================================
// Test: Simple Macro Extraction
// =============================================================================

#[test]
fn test_extract_simple_macro_form() {
    // Test a simple macro like @toggle with basic params
    let params = vec![
        make_form_param(
            "initial",
            "initial",
            CaptureType::Ident,
            CaptureModifier::Optional,
            Some(ParamDefault::String("off".to_string())),
        ),
        make_form_param(
            "trigger",
            "trigger",
            CaptureType::Event,
            CaptureModifier::Optional,
            Some(ParamDefault::String("click".to_string())),
        ),
    ];
    let form = make_form_clause("toggle", vec![], params, false);
    let macro_def = make_macro("ToggleMacro", Some("toggle"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry = FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/toggle.st"));

    let directive = form_registry.get_directive("toggle").unwrap();
    assert_eq!(directive.name, "toggle");
    assert_eq!(directive.macro_name, "ToggleMacro");
    assert_eq!(directive.params.len(), 2);

    // Check first param
    assert_eq!(directive.params[0].name, "initial");
    assert_eq!(directive.params[0].capture_type, CaptureType::Ident);
    assert_eq!(directive.params[0].modifier, CaptureModifier::Optional);
    assert!(directive.params[0].default_value.is_some());

    // Check second param
    assert_eq!(directive.params[1].name, "trigger");
    assert_eq!(directive.params[1].capture_type, CaptureType::Event);
}

// =============================================================================
// Test: Params with Types and Defaults
// =============================================================================

#[test]
fn test_extract_params_with_types_and_defaults() {
    // Test @fade-in style params with various types
    let params = vec![
        make_form_param(
            "duration",
            "duration",
            CaptureType::Time,
            CaptureModifier::Required,
            Some(ParamDefault::String("600ms".to_string())),
        ),
        make_form_param(
            "threshold",
            "threshold",
            CaptureType::Number,
            CaptureModifier::Required,
            Some(ParamDefault::Number(0.1)),
        ),
        make_form_param(
            "distance",
            "distance",
            CaptureType::Length,
            CaptureModifier::Required,
            Some(ParamDefault::String("20px".to_string())),
        ),
        make_form_param(
            "easing",
            "easing",
            CaptureType::Easing,
            CaptureModifier::Optional,
            Some(ParamDefault::String("ease-out".to_string())),
        ),
    ];
    let form = make_form_clause("fade-in", vec![], params, true);
    let macro_def = make_macro("FadeInMacro", Some("fade-in"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry =
        FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/fade-in.st"));

    let directive = form_registry.get_directive("fade-in").unwrap();
    assert_eq!(directive.params.len(), 4);

    // Verify types
    assert_eq!(directive.params[0].capture_type, CaptureType::Time);
    assert_eq!(directive.params[1].capture_type, CaptureType::Number);
    assert_eq!(directive.params[2].capture_type, CaptureType::Length);
    assert_eq!(directive.params[3].capture_type, CaptureType::Easing);

    // Verify defaults
    assert_eq!(
        directive.params[0].default_value,
        Some("\"600ms\"".to_string())
    );
    assert_eq!(directive.params[1].default_value, Some("0.1".to_string()));
    assert_eq!(directive.params[3].modifier, CaptureModifier::Optional);

    // Should have body
    assert!(directive.body_type.is_some());
}

// =============================================================================
// Test: Prefix Filtering for Completions
// =============================================================================

#[test]
fn test_directive_completions_prefix_filtering() {
    let mut form_registry = FormRegistry::new();

    // Register multiple directives
    form_registry.register(DirectiveSignature {
        name: "scroll".to_string(),
        macro_name: "ScrollMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: Path::new("/stdlib/scroll.st").to_path_buf(),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    form_registry.register(DirectiveSignature {
        name: "scroll-timeline".to_string(),
        macro_name: "ScrollTimelineMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: Path::new("/stdlib/scroll-timeline.st").to_path_buf(),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    form_registry.register(DirectiveSignature {
        name: "state".to_string(),
        macro_name: "StateMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: Path::new("/stdlib/state.st").to_path_buf(),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    form_registry.register(DirectiveSignature {
        name: "fade-in".to_string(),
        macro_name: "FadeInMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: Path::new("/stdlib/fade-in.st").to_path_buf(),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // Test "scr" prefix - should match scroll and scroll-timeline
    let completions = form_registry.directive_completions("scr");
    assert_eq!(completions.len(), 2);
    let names: Vec<&str> = completions.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"scroll"));
    assert!(names.contains(&"scroll-timeline"));

    // Test "scroll-" prefix - should only match scroll-timeline
    let completions = form_registry.directive_completions("scroll-");
    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].0, "scroll-timeline");

    // Test "st" prefix - should match state
    let completions = form_registry.directive_completions("st");
    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].0, "state");

    // Test case insensitivity
    let completions = form_registry.directive_completions("SCR");
    assert_eq!(completions.len(), 2);

    // Test empty prefix - should return all
    let completions = form_registry.directive_completions("");
    assert_eq!(completions.len(), 4);

    // Test no match
    let completions = form_registry.directive_completions("xyz");
    assert!(completions.is_empty());
}

// =============================================================================
// Test: Inline Elements Extraction
// =============================================================================

#[test]
fn test_extract_inline_elements() {
    // Test @data $name:ident : $type:typeref pattern (inline captures)
    let inline_elements = vec![
        make_inline_capture("name", CaptureType::Ident, CaptureModifier::Required, None),
        FormInlineElement::Literal(":".to_string()),
        make_inline_capture(
            "type",
            CaptureType::Typeref,
            CaptureModifier::Required,
            None,
        ),
    ];
    let params = vec![make_form_param(
        "src",
        "src",
        CaptureType::String,
        CaptureModifier::Optional,
        None,
    )];
    let form = make_form_clause("data", inline_elements, params, true);
    let macro_def = make_macro("DataMacro", Some("data"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry = FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/data.st"));

    let directive = form_registry.get_directive("data").unwrap();

    // Should have 3 params: 2 inline + 1 regular
    assert_eq!(directive.params.len(), 3);

    // First two are inline captures
    assert_eq!(directive.params[0].name, "name");
    assert_eq!(directive.params[0].capture_type, CaptureType::Ident);
    assert_eq!(directive.params[1].name, "type");
    assert_eq!(directive.params[1].capture_type, CaptureType::Typeref);

    // Third is regular param
    assert_eq!(directive.params[2].name, "src");
    assert_eq!(directive.params[2].capture_type, CaptureType::String);
    assert_eq!(directive.params[2].modifier, CaptureModifier::Optional);
}


// =============================================================================
// Test: Macro Without %creates or %form (Not Indexed)
// =============================================================================

#[test]
fn test_helper_macro_not_indexed() {
    // Helper macro with no directive
    let macro_def = make_macro("HelperMacro", None, None);

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry = FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/helper.st"));

    // Should not be in the registry
    assert!(form_registry.get_directive("HelperMacro").is_none());
    assert!(form_registry.is_empty());
}

// =============================================================================
// Test: All Directives Iterator
// =============================================================================

#[test]
fn test_all_directives_iterator() {
    let mut form_registry = FormRegistry::new();

    for name in &["scroll", "fade-in", "toggle", "hover"] {
        form_registry.register(DirectiveSignature {
            name: name.to_string(),
            macro_name: format!("{}Macro", name),
            params: vec![],
            body_type: None,
            definition_span: SourceSpan::default(),
            source_file: Path::new("/test.st").to_path_buf(),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        });
    }

    let all: Vec<&DirectiveSignature> = form_registry.all_directives().collect();
    assert_eq!(all.len(), 4);

    let names: Vec<&str> = form_registry.directive_names().collect();
    assert_eq!(names.len(), 4);
    assert!(names.contains(&"scroll"));
    assert!(names.contains(&"fade-in"));
    assert!(names.contains(&"toggle"));
    assert!(names.contains(&"hover"));
}

// =============================================================================
// Test: Signature Formatting
// =============================================================================

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
        body_type: Some(CaptureType::Keyframes),
        definition_span: SourceSpan::default(),
        source_file: Path::new("/test.st").to_path_buf(),
        documentation: Some("Animate properties".to_string()),
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

// =============================================================================
// Test: Required Params Detection
// =============================================================================

#[test]
fn test_required_params() {
    let sig = DirectiveSignature {
        name: "test".to_string(),
        macro_name: "TestMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "required_no_default".to_string(),
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
                name: "required_with_default".to_string(),
                capture_var: "def".to_string(),
                capture_type: CaptureType::String,
                modifier: CaptureModifier::Required,
                default_value: Some("\"default\"".to_string()),
            },
        ],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: Path::new("/test.st").to_path_buf(),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let required = sig.required_params();
    assert_eq!(required.len(), 1);
    assert_eq!(required[0].name, "required_no_default");

    // has_all_defaults should be false because required_no_default has no default
    assert!(!sig.has_all_defaults());
}

// =============================================================================
// Test: Union Type Formatting
// =============================================================================

#[test]
fn test_union_type_extraction() {
    let union_type = CaptureType::Union(vec!["x".to_string(), "y".to_string(), "both".to_string()]);

    let params = vec![make_form_param(
        "axis",
        "axis",
        union_type,
        CaptureModifier::Optional,
        Some(ParamDefault::String("both".to_string())),
    )];
    let form = make_form_clause("scroll", vec![], params, false);
    let macro_def = make_macro("ScrollMacro", Some("scroll"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry = FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/scroll.st"));

    let directive = form_registry.get_directive("scroll").unwrap();
    assert_eq!(directive.params.len(), 1);

    // Check union type
    match &directive.params[0].capture_type {
        CaptureType::Union(variants) => {
            assert_eq!(variants.len(), 3);
            assert!(variants.contains(&"x".to_string()));
            assert!(variants.contains(&"y".to_string()));
            assert!(variants.contains(&"both".to_string()));
        }
        _ => panic!("Expected Union type"),
    }
}

// =============================================================================
// Test: Source File Path Preserved
// =============================================================================

#[test]
fn test_source_file_preserved() {
    let form = make_form_clause("test", vec![], vec![], false);
    let macro_def = make_macro("TestMacro", Some("test"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let source_path = Path::new("/home/user/project/stdlib/macros/test.st");
    let form_registry = FormRegistry::from_meta_registry(&registry, source_path);

    let directive = form_registry.get_directive("test").unwrap();
    assert_eq!(directive.source_file, source_path);
}

// =============================================================================
// Test: Registry Length and Empty Check
// =============================================================================

#[test]
fn test_registry_len_and_is_empty() {
    let form_registry = FormRegistry::new();
    assert!(form_registry.is_empty());
    assert_eq!(form_registry.len(), 0);

    let mut form_registry = FormRegistry::new();
    form_registry.register(DirectiveSignature {
        name: "test".to_string(),
        macro_name: "TestMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: Path::new("/test.st").to_path_buf(),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    assert!(!form_registry.is_empty());
    assert_eq!(form_registry.len(), 1);
}

// =============================================================================
// Test: DirectiveParam is_required Method
// =============================================================================

#[test]
fn test_directive_param_is_required() {
    let required = DirectiveParam {
        name: "test".to_string(),
        capture_var: "test".to_string(),
        capture_type: CaptureType::String,
        modifier: CaptureModifier::Required,
        default_value: None,
    };
    assert!(required.is_required());

    let optional = DirectiveParam {
        name: "test".to_string(),
        capture_var: "test".to_string(),
        capture_type: CaptureType::String,
        modifier: CaptureModifier::Optional,
        default_value: None,
    };
    assert!(!optional.is_required());

    let with_default = DirectiveParam {
        name: "test".to_string(),
        capture_var: "test".to_string(),
        capture_type: CaptureType::String,
        modifier: CaptureModifier::Required,
        default_value: Some("\"default\"".to_string()),
    };
    assert!(!with_default.is_required());
}

// =============================================================================
// Test: New Template System Types (ParamList, HtmlBlock, TemplateInvocation)
// =============================================================================

#[test]
fn test_param_list_type() {
    // Test @template &name($params:param_list) pattern
    let inline_elements = vec![
        FormInlineElement::Literal("&".to_string()),
        make_inline_capture("name", CaptureType::Ident, CaptureModifier::Required, None),
    ];
    let params = vec![make_form_param(
        "params",
        "params",
        CaptureType::ParamList,
        CaptureModifier::Required,
        None,
    )];
    let form = make_form_clause("template", inline_elements, params, true);
    let macro_def = make_macro("TemplateMacro", Some("template"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry =
        FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/template.st"));

    let directive = form_registry.get_directive("template").unwrap();
    assert_eq!(directive.params.len(), 2);
    assert_eq!(directive.params[0].name, "name");
    assert_eq!(directive.params[0].capture_type, CaptureType::Ident);
    assert_eq!(directive.params[1].name, "params");
    assert_eq!(directive.params[1].capture_type, CaptureType::ParamList);
}

#[test]
fn test_html_block_type() {
    // Test @template body containing html_block
    let params = vec![make_form_param(
        "html",
        "html",
        CaptureType::HtmlBlock,
        CaptureModifier::Required,
        None,
    )];
    let form = make_form_clause("template", vec![], params, false);
    let macro_def = make_macro("TemplateMacro", Some("template"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry =
        FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/template.st"));

    let directive = form_registry.get_directive("template").unwrap();
    assert_eq!(directive.params.len(), 1);
    assert_eq!(directive.params[0].name, "html");
    assert_eq!(directive.params[0].capture_type, CaptureType::HtmlBlock);
}

#[test]
fn test_template_invocation_type() {
    // Test @each body containing template_invocation
    let params = vec![
        make_form_param(
            "source",
            "source",
            CaptureType::Binding,
            CaptureModifier::Required,
            None,
        ),
        make_form_param(
            "item",
            "item",
            CaptureType::Ident,
            CaptureModifier::Required,
            None,
        ),
        make_form_param(
            "invocations",
            "invocations",
            CaptureType::TemplateInvocation,
            CaptureModifier::ZeroOrMore,
            None,
        ),
    ];
    let form = make_form_clause("each", vec![], params, true);
    let macro_def = make_macro("EachMacro", Some("each"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry = FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/each.st"));

    let directive = form_registry.get_directive("each").unwrap();
    assert_eq!(directive.params.len(), 3);
    assert_eq!(directive.params[0].name, "source");
    assert_eq!(directive.params[0].capture_type, CaptureType::Binding);
    assert_eq!(directive.params[1].name, "item");
    assert_eq!(directive.params[1].capture_type, CaptureType::Ident);
    assert_eq!(directive.params[2].name, "invocations");
    assert_eq!(
        directive.params[2].capture_type,
        CaptureType::TemplateInvocation
    );
    assert_eq!(directive.params[2].modifier, CaptureModifier::ZeroOrMore);
}

#[test]
fn test_template_type() {
    // Test the Template capture type
    let params = vec![make_form_param(
        "content",
        "content",
        CaptureType::Template,
        CaptureModifier::Required,
        None,
    )];
    let form = make_form_clause("slot", vec![], params, true);
    let macro_def = make_macro("SlotMacro", Some("slot"), Some(form));

    let mut registry = MetaRegistry::new();
    registry.register_macro(macro_def).unwrap();

    let form_registry = FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/slot.st"));

    let directive = form_registry.get_directive("slot").unwrap();
    assert_eq!(directive.params.len(), 1);
    assert_eq!(directive.params[0].capture_type, CaptureType::Template);
}

// =============================================================================
// Compiler-registry parity (gh-28): the LSP serves the compiler's live registry.
// =============================================================================

#[test]
fn from_compiler_serves_the_live_stdlib_registry() {
    let registry = FormRegistry::from_compiler();
    // The compiler resolves @scroll, @data, @each etc. The old hand-written
    // include_str! snapshot drifted from this set; the live registry must be
    // a superset that includes the core directives the compiler actually emits.
    for expected in ["scroll", "data", "each", "form", "on"] {
        assert!(
            registry.get_directive(expected).is_some(),
            "live registry must resolve @{expected}"
        );
    }
    // Real absolute source paths (go-to-definition opens the actual file).
    if let Some(sig) = registry.get_directive("scroll") {
        assert!(
            sig.source_file.to_string_lossy().contains("stdlib"),
            "source_file should be a real stdlib path, got {}",
            sig.source_file.display()
        );
    }
}

#[test]
fn from_compiler_registry_preserves_project_overlay_macros() {
    // Simulate the compiler loading a project's `_prelude.st` overlay into the
    // MetaRegistry, then check the FormRegistry picks up the project macro.
    use spacetime::parser::meta_ast::{FormCapture, FormClause, FormInlineElement, CaptureType};

    let mut meta = MetaRegistry::new();
    let project_macro = MacroDefAst {
        name: "ProjectCardMacro".to_string(),
        form: Some(FormClause {
            directive_name: "project-card".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "title".to_string(),
                    capture_type: CaptureType::String,
                    modifier: CaptureModifier::Optional,
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
        source_file: Some("/home/user/proj/_prelude.st".to_string()),
        doc: Some("A project-defined directive.".to_string()),
        ..Default::default()
    };
    meta.register_macro(project_macro).unwrap();

    let registry = FormRegistry::from_compiler_registry(&meta);
    let sig = registry.get_directive("project-card").expect("project macro indexed");
    assert_eq!(sig.source_file.to_string_lossy(), "/home/user/proj/_prelude.st");
    assert_eq!(sig.documentation.as_deref(), Some("A project-defined directive."));
}

#[test]
fn live_registry_closes_the_97_token_gap() {
    let registry = FormRegistry::from_compiler();
    let n = registry.directive_names().count();
    assert!(
        n >= 97,
        "the compiler resolves at least 97 leading form tokens; live registry serves {n}"
    );
}
