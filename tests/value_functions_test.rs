//! Integration tests for value functions (wave, random, noise, parallax)
//!
//! Parse tests are active. Codegen tests checking specific function definitions
//! are ignored pending stdlib %emit pipeline integration for value function
//! primitives (stdlib/primitives/animation/functions.st).

use spacetime::{compile, compiler::CompileOptions, parse};

// =============================================================================
// PARSE VERIFICATION TESTS (active)
// =============================================================================

#[test]
fn test_wave_function_parses() {
    let input = r#"
.element {
    @loop wobble(2s) {
        & {
            rotate: 0deg -> wave(sin, 1, 5deg);
        }
    }
}
"#;
    parse(input).expect("@loop with wave() should parse");
}

#[test]
fn test_random_function_parses() {
    let input = r#"
.element {
    @scroll reveal {
        & {
            rotate: 0deg -> random(-5deg, 5deg);
        }
    }
}
"#;
    parse(input).expect("@scroll with random() should parse");
}

#[test]
fn test_noise_function_parses() {
    let input = r#"
.element {
    @scroll reveal {
        & {
            translateY: 0px -> noise(0.5, 15px);
        }
    }
}
"#;
    parse(input).expect("@scroll with noise() should parse");
}

#[test]
fn test_parallax_function_parses() {
    let input = r#"
.element {
    @mouse parallax {
        & {
            translateY: 0px -> parallax(100px);
        }
    }
}
"#;
    parse(input).expect("@mouse with parallax() should parse");
}

#[test]
fn test_multiple_function_types_parse() {
    let input = r#"
.element {
    @loop complex(3s) {
        & {
            translateX: 0px -> wave(sin, 1, 10px);
            translateY: 0px -> noise(0.5, 15px);
            rotate: 0deg -> random(-5deg, 5deg);
        }
    }
}
"#;
    parse(input).expect("@loop with multiple value functions should parse");
}

#[test]
fn test_wave_with_all_parameters_compiles() {
    let input = r#"
.element {
    @loop wobble(2s) {
        & {
            rotate: 0deg -> wave(cos, 2, 10deg, 0.25);
        }
    }
}
"#;
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());

    // @loop currently produces JS with stWave function markers
    assert!(compiled.js.contains("stWave"));
    assert!(compiled.js.contains("cos") || compiled.js.contains("10deg"));
}

#[test]
fn test_nested_function_calls_parse() {
    let input = r#"
.element {
    @loop complex(2s) {
        & {
            translateY: 0px -> wave(sin, 1, parallax(20px));
        }
    }
}
"#;
    parse(input).expect("@loop with nested function calls should parse");
}

#[test]
fn test_runtime_function_evaluation_logic() {
    let input = r#"
.element {
    @loop test(1s) {
        & {
            opacity: 0 -> wave(sin, 1, 10px);
        }
    }
}
"#;
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());

    // @loop runtime includes resolveValue infrastructure for __st_fn_ markers
    assert!(compiled.js.contains("resolveValue"));
    assert!(compiled.js.contains("__st_fn_"));
    assert!(compiled.js.contains("evaluateRuntimeFunction"));
    assert!(compiled.js.contains("elementIndex"));
    assert!(compiled.js.contains("elementRect"));
}

// =============================================================================
// CODEGEN TESTS (ignored — stdlib %emit pipeline not yet wired for function primitives)
// =============================================================================

#[test]
#[ignore = "INIT-038: stdlib %emit pipeline does not yet emit function definitions — needs primitive expansion for wave/random/noise/parallax"]
fn test_wave_function_codegen() {
    let input = r#"
.element {
    @loop wobble(2s) {
        & {
            rotate: 0deg -> wave(sin, 1, 5deg);
        }
    }
}
"#;
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.js.contains("function stWave"),
        "JS should contain wave function definition from primitive %emit block"
    );
}

#[test]
#[ignore = "INIT-038: stdlib %emit pipeline does not yet emit function definitions — needs primitive expansion for wave/random/noise/parallax"]
fn test_multiple_function_codegen() {
    let input = r#"
.element {
    @loop complex(3s) {
        & {
            translateX: 0px -> wave(sin, 1, 10px);
            translateY: 0px -> noise(0.5, 15px);
            rotate: 0deg -> random(-5deg, 5deg);
        }
    }
}
"#;
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(compiled.js.contains("function stWave"));
    assert!(compiled.js.contains("function stRandom"));
    assert!(compiled.js.contains("function stNoise"));
}
