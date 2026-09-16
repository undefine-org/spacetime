use spacetime::{CompileOptions, compile, parse};

fn main() {
    let input = r#"
@type Product {
    name: string;
    price: number;
}

@fn formatPrice(price: number): string {
    return '$' + price.toFixed(2);
}

@data products: Product[] {
    src: "/api/products";
}

.product-list {
    @each(products) {
        template: "product-card";
        [slot="name"]: $.name;
        [slot="price"]: formatPrice($.price);
    }
}
"#;

    let ast = parse(input).expect("Failed to parse");

    // Debug: print scope info
    println!("=== Scopes ===");
    for scope in &ast.scopes {
        println!("Scope: {}", scope.selector);
        let each_blocks_count = scope
            .matches
            .iter()
            .filter(|m| m.macro_name == "each" || m.macro_name == "each-block")
            .count();
        println!("  each_blocks: {}", each_blocks_count);
        println!("  macro_calls: {}", scope.macro_calls.len());
        for mc in &scope.macro_calls {
            println!(
                "    macro: {} {:?}",
                mc.name,
                mc.body.as_ref().map(|b| &b.properties)
            );
        }
    }

    let compiled = compile(&ast, CompileOptions::default());
    println!("\n=== Generated JS ===");
    println!("{}", compiled.js);
    println!("\n=== Generated CSS ===");
    println!("{}", compiled.css);
}
