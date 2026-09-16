use spacetime::{
    compiler::{CompileOptions, compile},
    parser::parse,
};

fn main() {
    let input = r#"
.item-list {
    @state(when: "loading") {
        opacity: 0.5;
        min-height: 200px;
    }
}
"#;

    let ast = parse(input).expect("Failed to parse");

    // Debug: show macro calls in the AST
    println!("=== Macro Calls in Scopes ===");
    for scope in &ast.scopes {
        println!("Scope: {}", scope.selector);
        for mc in &scope.macro_calls {
            println!("  @{} args: {:?}", mc.name, mc.args);
            if let Some(body) = &mc.body {
                println!("  body properties: {:?}", body.properties);
            }
        }
    }

    let result = compile(&ast, CompileOptions::default());

    println!("\n=== CSS output ===");
    println!("{}", result.css);
}
