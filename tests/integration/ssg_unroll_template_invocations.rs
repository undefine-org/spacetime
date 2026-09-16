//! FEAT-154 — SSG-unroll of provably-static @template invocations into served HTML.
//!
//! Mirrors ssg_unroll_each.rs's test shape (compile() -> assert on .html) since a
//! static template invocation is the SAME unroll==hydrate contract one level up
//! (docs/language/data-and-rendering.md §6, extended to invocations).

use spacetime::{compile, compiler::CompileOptions, parse};

#[test]
fn static_invocation_with_literal_args_unrolls_into_html() {
    let src = "@template &card($title) {\n<article class=\"card\"><h3>`$title`</h3></article>\n}\n<main class=\"page\"></main>\n.page {\n&card(\"Hello\");\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.html.contains("<h3"),
        "static invocation must render its heading, html: {}",
        compiled.html
    );
    assert!(
        compiled.html.contains("Hello"),
        "static invocation must render the literal arg value, html: {}",
        compiled.html
    );
    assert!(
        compiled.html.contains("data-st-ssg=\"1\""),
        "unrolled root must carry the hydration-adopt marker, html: {}",
        compiled.html
    );
}

#[test]
fn dynamic_invocation_is_not_unrolled() {
    // A `$`-ref arg is a dynamic signal read — not compile-time-constant. The
    // container stays empty in served HTML (runtime invoke-template fills it).
    let src = "@template &card($title) {\n<article class=\"card\"><h3>`$title`</h3></article>\n}\n@data inline $t : \"Dynamic\";\n<main class=\"page\"></main>\n.page {\n&card($t);\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.html.contains("<h3"),
        "dynamic invocation must not SSG-unroll; html: {}",
        compiled.html
    );
    assert!(
        !compiled.html.contains("data-st-ssg"),
        "no ssg marker for a non-unrolled invocation; html: {}",
        compiled.html
    );
}

#[test]
fn invocation_with_element_param_is_not_unrolled() {
    // v1 scope: an ELEMENT param (`&content`) has no static HTML equivalent —
    // the whole template is disqualified from static rendering.
    let src = "@template &card($title, &content) {\n<article class=\"card\"><h3>`$title`</h3><div>`&content`</div></article>\n}\n<main class=\"page\"></main>\n.page {\n&card(\"Hello\", \"World\");\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.html.contains("<h3"),
        "an element-param template must not SSG-unroll; html: {}",
        compiled.html
    );
}

#[test]
fn body_slot_invocation_is_not_unrolled() {
    // v1 scope: a body-slot invocation (`&card("T") { <p>Body</p> }`) is left
    // dynamic — element-param/slot unrolling is the transitive-nesting follow-up.
    let src = "@template &card($title, &actions) {\n<article class=\"card\"><h3>`$title`</h3><div>`&actions`</div></article>\n}\n<main class=\"page\"></main>\n.page {\n&card(\"Hello\") { <button>Edit</button> }\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.html.contains("<h3"),
        "a body-slot invocation must not SSG-unroll in v1; html: {}",
        compiled.html
    );
}

#[test]
fn optional_param_omitted_renders_empty() {
    let src = "@template &card($title, $subtitle?) {\n<article class=\"card\"><h3>`$title`</h3><p>`$subtitle`</p></article>\n}\n<main class=\"page\"></main>\n.page {\n&card(\"Hello\");\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.html.contains("Hello"),
        "required param must still render, html: {}",
        compiled.html
    );
    // The <p></p> for the omitted optional renders empty, not a literal `` `$subtitle` ``.
    assert!(
        !compiled.html.contains("$subtitle"),
        "an omitted optional param hole must not leak raw syntax, html: {}",
        compiled.html
    );
}

#[test]
fn local_state_initial_seeds_the_unrolled_hole() {
    // A template's own `$count number: 0;` local state renders its declared
    // initial value in a static unroll, matching the runtime factory's own
    // first-paint value (the unroll==hydrate invariant).
    let src = "@template &counter($label) {\n$count number: 0;\n<div class=\"ctr\"><span class=\"l\">`$label`</span><span class=\"v\">`$count`</span></div>\n}\n<main class=\"page\"></main>\n.page {\n&counter(\"Clicks\");\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(compiled.html.contains("Clicks"), "html: {}", compiled.html);
    assert!(
        compiled.html.contains(">0<"),
        "local state initial must seed the unrolled hole as '0', not '0.0' or empty; html: {}",
        compiled.html
    );
}

#[test]
fn required_param_with_missing_arg_is_not_unrolled() {
    // A required param with no corresponding arg is a shape v1 defers to the
    // runtime factory's own defaulting semantics — not unrolled.
    let src = "@template &card($title, $subtitle) {\n<article class=\"card\"><h3>`$title`</h3><p>`$subtitle`</p></article>\n}\n<main class=\"page\"></main>\n.page {\n&card(\"Hello\");\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.html.contains("<h3"),
        "a required-param/arg-count mismatch must not SSG-unroll; html: {}",
        compiled.html
    );
}

#[test]
fn unknown_template_name_is_not_unrolled_and_does_not_panic() {
    let src = "<main class=\"page\"></main>\n.page {\n&doesNotExist(\"x\");\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    assert!(!compiled.html.contains("data-st-ssg"));
}
