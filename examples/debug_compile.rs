// Simple test to see what gets compiled
use spacetime::{
    compiler::{CompileOptions, compile},
    parser::meta_ast,
    parser::parse,
};

fn main() {
    let stdlib = include_str!("../stdlib/testing/test.st");
    let test = r#"
.test-suite {
    @test "should fail for nonexistent" {
        @then .nonexistent should exist
    }
}
"#;

    let source = format!("{}\n\n{}", stdlib, test);

    let ast = match parse(&source) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    };

    // Debug: Show macro definitions
    println!("=== Macro Definitions ===");
    for meta_def in &ast.meta_defs {
        if let meta_ast::MetaDef::Macro(m) = meta_def {
            println!("  Macro: %{}", m.name);
            println!("    Creates: {:?}", m.creates);
            println!("    Body items: {}", m.body.len());
            for item in &m.body {
                println!("      {:?}", std::mem::discriminant(item));
            }
        }
    }

    // Debug: Show macro calls in scopes
    println!("\n=== Macro Calls in Scopes ===");
    for scope in &ast.scopes {
        println!("  Scope: {}", scope.selector);
        println!("    Macro calls: {}", scope.macro_calls.len());
        for mc in &scope.macro_calls {
            println!("      @{} with {} args", mc.name, mc.args.len());
            for (i, arg) in mc.args.iter().enumerate() {
                println!("        arg[{}]: {:?}", i, arg);
            }
        }
    }

    let compiled = compile(&ast, CompileOptions::default());

    println!("\n=== Generated JS (last 500 chars) ===");
    let js_len = compiled.js.len();
    if js_len > 500 {
        println!("{}", &compiled.js[js_len - 500..]);
    } else {
        println!("{}", compiled.js);
    }

    // Check for test registration
    if compiled.js.contains("__spacetime_register_test") {
        println!("\n✓ Contains test registration");
    } else {
        println!("\n✗ Missing test registration!");
    }

    if compiled.js.contains("@then") || compiled.js.contains("__thenEl") {
        println!("✓ Contains assertion code");
    } else {
        println!("✗ Missing assertion code!");
    }
}
