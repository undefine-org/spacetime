fn main() {
    let src = "@template &ta($n) { <div class=\"ta\">A</div> }\n@template &tb($n) { <div class=\"tb\">B</div> }\n@template &host($kind) {\n  <div class=\"h\"></div>\n  @match $kind { \"a\" => &ta($kind); \"b\" => &tb($kind); }\n}\n.host { &host(\"b\"); }\n";
    let ast = spacetime::parse(src).expect("parse");
    for m in &ast.matches {
        println!(
            "match: macro={} matched={:?} captures={:?}",
            m.macro_name,
            m.matched_macro,
            m.captures.keys().collect::<Vec<_>>()
        );
    }
    let c = spacetime::Compiler::from_ast(&ast)
        .without_runtime()
        .compile();
    println!(
        "errors: {:?}",
        c.pipeline_errors
            .iter()
            .map(|e| e.code.clone())
            .collect::<Vec<_>>()
    );
    let js = &c.js;
    for (i, line) in js.lines().enumerate() {
        if line.contains("arms") && line.contains("pat") {
            println!("JS:{}: {}", i, &line[..line.len().min(300)]);
        }
    }
    println!(
        "has match-render: {}",
        js.contains("match-render") || js.contains("armPattern")
    );
}
