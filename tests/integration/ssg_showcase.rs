//! PLAN-023 W6 — SSG showcase demo + BUG-043 regression.

use spacetime::{compile, compiler::CompileOptions, parse};

fn html_of(src: &str) -> String {
    compile(&parse(src).expect("parse"), CompileOptions::default()).html
}

/// The demos/ssg-list showcase unrolls its static list into served HTML.
#[test]
fn ssg_showcase_demo_unrolls() {
    let src = std::fs::read_to_string("demos/ssg-list/index.st").expect("read demo");
    let html = html_of(&src);
    for fruit in ["Apple", "Banana", "Cherry", "Date"] {
        assert!(
            html.contains(&format!(">{}</li>", fruit)),
            "served HTML must contain unrolled <li>{}</li>; got:\n{}",
            fruit,
            html
        );
    }
}

/// BUG-043 regression: a backtick hole in a top-level HTML block must NOT drop a
/// subsequent scope's `@each` match (which previously left the list un-rendered).
#[test]
fn hole_in_html_block_does_not_drop_each() {
    let src = "@data inline $fruits : [\"Apple\", \"Banana\"];\n\
        @data derive $count number : $fruits.length;\n\
        <main><span>`$count`</span><ul class=\"basket\"></ul></main>\n\
        .basket {\n  @each($fruits as $fruit) {\n    <li>`$fruit`</li>\n  }\n}\n";
    let html = html_of(src);
    assert!(
        html.contains(">Apple</li>") && html.contains(">Banana</li>"),
        "@each must still render despite a hole in the HTML block; got:\n{}",
        html
    );
}

/// BUG-043 P1 (W6 review): a top-level VOID (<img>/<br>) or self-closing element
/// before a scope must not over-skip and drop the scope's @each (scan_html_end must
/// return at a depth-0 void/self-close open tag, matching scan_html_region).
#[test]
fn void_element_does_not_drop_each() {
    for head in ["<img src=\"l.png\">", "<br>", "<custom-x/>"] {
        let src = format!(
            "@data inline $f : [\"Apple\"];\n{}\n<ul class=\"basket\"></ul>\n.basket {{ @each($f as $x) {{ <li>`$x`</li> }} }}\n",
            head
        );
        let html = html_of(&src);
        assert!(
            html.contains(">Apple</li>"),
            "head {} dropped @each; got:\n{}",
            head,
            html
        );
    }
}
