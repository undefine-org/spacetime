use spacetime::{CompileOptions, compile, parse};

fn compile_js(source: &str) -> String {
    let ast = parse(source).expect("source should parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.pipeline_errors.is_empty(),
        "pipeline errors: {:?}",
        compiled.pipeline_errors
    );
    compiled.js
}

fn assert_responsive_directive_compiles_to_guarded_js(name: &str, source: &str) {
    let js = compile_js(source);
    assert!(
        js.contains("matchMedia"),
        "{name} should emit matchMedia guard for nested directive.\nJS:\n{js}"
    );
}

#[test]
fn breakpoint_with_nested_mouse_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@breakpoint + @mouse",
        r#"
.card {
    @breakpoint(md) {
        @mouse tilt(axis: "both") {
            & { rotate-x: 0 -> 8deg; }
        }
    }
}
"#,
    );
}

#[test]
fn media_with_nested_scroll_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@media + @scroll",
        r#"
.hero {
    @media("(min-width: 768px)") {
        @scroll reveal {
            opacity: 0 -> 1;
        }
    }
}
"#,
    );
}

#[test]
fn dark_with_nested_loop_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@dark + @loop",
        r#"
.chip {
    @dark {
        @loop pulse(3s) {
            & { scale: 1 -> 1.05 -> 1; }
        }
    }
}
"#,
    );
}

#[test]
fn light_with_nested_loop_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@light + @loop",
        r#"
.chip {
    @light {
        @loop pulse(3s) {
            & { scale: 1 -> 1.05 -> 1; }
        }
    }
}
"#,
    );
}

#[test]
fn reduced_motion_with_nested_on_visible_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@reduced-motion + @on &.visible",
        r#"
.panel {
    @reduced-motion {
        @on visible reveal(800ms) {
            opacity: 0 -> 1;
        }
    }
}
"#,
    );
}

#[test]
fn container_with_nested_mouse_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@container + @mouse",
        r#"
.card {
    @container("min-width: 400px") {
        @mouse tilt(axis: "x") {
            & { rotate-y: -6deg -> 0 -> 6deg; }
        }
    }
}
"#,
    );
}

#[test]
fn portrait_with_nested_scroll_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@portrait + @scroll",
        r#"
.gallery {
    @portrait {
        @scroll reveal {
            opacity: 0 -> 1;
        }
    }
}
"#,
    );
}

#[test]
fn landscape_with_nested_scroll_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@landscape + @scroll",
        r#"
.gallery {
    @landscape {
        @scroll reveal {
            opacity: 0 -> 1;
        }
    }
}
"#,
    );
}

#[test]
fn print_with_nested_on_visible_emits_guarded_js() {
    assert_responsive_directive_compiles_to_guarded_js(
        "@print + @on &.visible",
        r#"
.content {
    @print {
        @on visible reveal(800ms) {
            opacity: 0 -> 1;
        }
    }
}
"#,
    );
}
