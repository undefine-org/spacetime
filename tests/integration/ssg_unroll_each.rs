//! PLAN-023 W4 — static @each SSG unroll into served HTML.

use spacetime::{compile, compiler::CompileOptions, parse};

#[test]
fn static_each_unrolls_rows_into_container() {
    let src = "@data inline $items : [\"Alpha\", \"Beta\", \"Gamma\"];\n<ul class=\"list\"></ul>\n.list {\n@each($items as $i) {\n<li>`$i`</li>\n}\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    eprintln!("HTML:\n{}", compiled.html);
    assert!(
        compiled.html.contains("<li>Alpha</li>"),
        "html: {}",
        compiled.html
    );
    assert!(compiled.html.contains("<li>Beta</li>"));
    assert!(compiled.html.contains("<li>Gamma</li>"));
}

#[test]
fn dynamic_each_is_not_unrolled() {
    // A fetch source is not compile-time-constant: the container stays empty in
    // served HTML (runtime hydrate fills it).
    let src = "@data fetch $items : \"/api/items\";\n<ul class=\"list\"></ul>\n.list {\n@each($items as $i) {\n<li>`$i`</li>\n}\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.html.contains("<li>"),
        "dynamic source must not SSG-unroll; html: {}",
        compiled.html
    );
}

#[test]
fn nested_each_unrolls_inner_rows_into_static_html() {
    // BUG-081: an @each whose inline body contains a nested @each over a field of
    // the outer item must SSG-unroll the INNER rows too (so no-JS / SEO sees them),
    // matching the runtime hydrate.
    let src = "@data inline $rows : [{\"name\":\"a\",\"tags\":[{\"t\":\"x\"},{\"t\":\"y\"}]},{\"name\":\"b\",\"tags\":[{\"t\":\"z\"}]}];\n\
<div class=\"grid\"></div>\n\
.grid {\n\
@each($rows as $r) {\n\
<article class=\"card\"><code class=\"name\">`$r.name`</code><span class=\"tags\">@each($r.tags as $t) { <em class=\"tag\">`$t.t`</em> }</span></article>\n\
}\n\
}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    // Outer rows.
    assert!(
        compiled.html.contains(">a</code>") || compiled.html.contains("a</code>"),
        "outer rows: {}",
        compiled.html
    );
    // Inner rows with concrete field text.
    let tags = compiled.html.matches("class=\"tag\"").count();
    assert_eq!(
        tags, 3,
        "expected 3 inner <em.tag> rows, html: {}",
        compiled.html
    );
    assert!(
        compiled.html.contains(">x</em>")
            && compiled.html.contains(">y</em>")
            && compiled.html.contains(">z</em>"),
        "inner field holes must be filled, html: {}",
        compiled.html
    );
}
