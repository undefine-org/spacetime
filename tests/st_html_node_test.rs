use spacetime::compiler::Compiler;
use spacetime::emit::html_reactive::emit_builder;
use spacetime::ir::HtmlExpr;
use spacetime::syntax::SignalScope;

fn compile_edn(source: &str) -> spacetime::compiler::CompiledSpacetime {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("index.edn");
    std::fs::write(&path, source).expect("write EDN page");
    Compiler::from_file(&path, dir.path())
        .unwrap_or_else(|e| panic!("EDN ingress failed: {e}"))
        .without_runtime()
        .compile()
}

#[test]
fn edn_ingress_reads_st_html_and_st_raw_markup_nodes_in_order() {
    let out = compile_edn(
        r#"{:st/html [[:st/html "<span class=\"live\">`$count`</span>"]
                     [:st/raw "<i class=\"raw\">`$count`</i>"]]}
"#,
    );

    assert!(
        out.js.contains("createElement(\"span\")"),
        "reactive node must be built: {}",
        out.js
    );
    assert!(
        out.js.contains("SpacetimeLocal['count']"),
        "reactive hole must bind globally: {}",
        out.js
    );
    assert!(
        out.html.contains("<i class=\"raw\">`$count`</i>"),
        "raw node must stay verbatim: {}",
        out.html
    );
}

#[test]
fn st_html_backtick_hole_binds_while_st_raw_stays_opaque() {
    let reactive = emit_builder(
        &[HtmlExpr::Html("<span>`$count`</span>".into())],
        SignalScope::Element,
    );
    assert!(reactive.contains("createElement(\"span\")"), "{reactive}");
    assert!(reactive.contains("ST.watch(__el, \"count\""), "{reactive}");
    assert!(
        !reactive.contains("innerHTML = \"<span>`$count`</span>\""),
        "{reactive}"
    );

    let raw = emit_builder(
        &[HtmlExpr::Raw("<span>`$count`</span>".into())],
        SignalScope::Element,
    );
    assert!(
        raw.contains("innerHTML = \"<span>`$count`</span>\""),
        "{raw}"
    );
    assert!(!raw.contains("ST.watch(__el, \"count\""), "{raw}");
}

#[test]
fn st_html_bare_dollar_is_literal() {
    let js = emit_builder(
        &[HtmlExpr::Html("<span>$count</span>".into())],
        SignalScope::Element,
    );
    assert!(js.contains("createTextNode(\"$count\")"), "{js}");
    assert!(!js.contains("ST.watch(__el, \"count\""), "{js}");
}

#[test]
fn st_html_elements_remain_available_to_selector_initializers() {
    let js = emit_builder(
        &[HtmlExpr::Html(
            "<button class=\"increment\">add</button>".into(),
        )],
        SignalScope::Element,
    );
    assert!(js.contains("createElement(\"button\")"), "{js}");
    assert!(
        js.contains("setAttribute(\"class\", \"increment\")"),
        "{js}"
    );
}

#[test]
fn st_html_node_round_trips_through_the_edn_document_codec() {
    use spacetime::parser::ast::HtmlInjection;
    let registry = &*spacetime::syntax::STDLIB_REGISTRY;
    let source = r#"{:st/html [[:st/html "<span>`$count`</span>"]
                              [:st/raw "<b>$count</b>"]]}"#;
    let ast = spacetime::edn::to_st_file(source, registry).expect("read nodes");
    assert_eq!(ast.html_blocks[0].injection, HtmlInjection::Reactive);
    assert_eq!(ast.html_blocks[1].injection, HtmlInjection::Raw);

    let document = spacetime::edn::doc::to_document(&ast);
    let wire = spacetime::edn::doc::write_document(&document, registry);
    let reparsed = spacetime::edn::to_st_file(&wire, registry).expect("re-read nodes");
    assert_eq!(reparsed.html_blocks, ast.html_blocks, "wire: {wire}");
}
