use spacetime::compiler::CompileOptions;
use spacetime::{compile, parser::parse};

/// WAVE A (FEAT-142): `&name { @directive... }` must lower to a live entity scope
/// that registers the entity and emits its nested directives. This test guards the
/// dead-letter regression where `&evernet { @scroll-spy }` compiled to 0 bytes.
#[test]
fn entity_scope_registers_and_emits_nested_directive() {
    let source = r#"@import "stdlib";

&evernet {
    @scroll-spy
}
"#;
    let ast = parse(source).expect("parse should succeed");
    let compiled = compile(&ast, CompileOptions::default());

    assert!(
        compiled.pipeline_errors.is_empty(),
        "compilation produced errors: {:?}",
        compiled.pipeline_errors
    );

    assert!(
        !compiled.js.is_empty(),
        "entity scope must emit non-zero JS; got empty output"
    );

    assert!(
        compiled.js.contains("window.__stWorld") || compiled.js.contains("__stWorld"),
        "entity registration into window.__stWorld must be emitted; got:\n{}",
        compiled.js
    );

    assert!(
        compiled.js.contains("evernet"),
        "entity name 'evernet' must appear in emitted JS; got:\n{}",
        compiled.js
    );

    assert!(
        compiled.js.contains("IntersectionObserver"),
        "nested @scroll-spy directive must emit its JS; got:\n{}",
        compiled.js
    );
}
