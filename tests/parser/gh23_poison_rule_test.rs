//! GH-23 regression (W1 / invariant I1): a rule whose key (rightmost) selector
//! matches no markup must be SKIPPED with a diagnostic — never swallowed by its
//! neighbours and never allowed to remove a sibling directive from the build.
//!
//! The reported defect (highest blast radius in the tracker): a single cosmetic
//! line `body:hover .nope { opacity: 1 }` — a DESCENDANT rule whose key `.nope`
//! matches no element — deleted EVERY directive after it in source order. A
//! 12-frame configurator, an acuity chart, a marquee's scroll-velocity and the
//! primary CTA's magnetism vanished from a 2100-line stylesheet while `check`
//! passed with zero diagnostics.
//!
//! Root cause: the events parser's `scope_or_property` only used `at_property`'s
//! two-token lookahead (`IDENT COLON`) to tell a CSS property from a scope block.
//! An element-type selector carrying a pseudo-class (`body:hover`) looks exactly
//! like `IDENT COLON`, so the whole rule AND every sibling after it were parsed
//! as ONE property value — the body parser, by contrast, already guarded with
//! `is_element_scope_block` first. The fix unifies the two paths: an element
//! scope block (`div { }`, `body:hover .nope { }`) is resolved before the
//! property check, exactly as the body parser does.

use spacetime::compiler::Compiler;
use spacetime::diagnostics::DiagnosticCode;
use spacetime::html::{
    parse_html, parse_templates, validate_with_html, HtmlContext,
};
use spacetime::parser::parse;

/// Minimal HTML where `.nope` exists in no markup. `body:hover .nope`'s key
/// selector must therefore be unresolvable.
fn html_without_nope() -> &'static str {
    r#"<div class="wrap"><button class="btn-a">a</button><button class="btn-b">b</button></div>"#
}

fn context(html: &str) -> HtmlContext {
    HtmlContext {
        document: parse_html(html).expect("parse html"),
        templates: parse_templates("").expect("parse empty templates"),
        source_path: "test.html".to_string(),
    }
}

/// Every `@on` directive in the file's scopes, by selector.
fn on_handlers(ast: &spacetime::parser::StFile) -> Vec<&str> {
    ast.scopes
        .iter()
        .filter(|s| s.matches.iter().any(|m| m.macro_name == "on"))
        .map(|s| s.selector.as_str())
        .collect()
}

/// The directive-bearing descendant form must behave IDENTICALLY to the bare
/// form: E0602 fires for the unresolvable rule, and both sibling handlers
/// survive to the parse IR.
#[test]
fn non_matching_descendant_rule_emits_e0602_and_keeps_sibling_handlers() {
    // `.nope` matches no markup. The rule carries a directive, so it cannot be
    // silently skipped as defensive CSS — it must be reported (E0602), and the
    // handlers declared AFTER it must survive.
    let src = r#"
        .wrap { $frame number: 0; }
        .btn-a { @on &.click { $frame <- 1; } }
        body:hover .nope { @on &.click { $frame <- 9; } }
        .btn-b { @on &.click { $frame <- 2; } }
    "#;
    let ast = parse(src).expect("parse");

    // Both sibling handlers survive to the parse IR (the reporter lost `.btn-b`).
    let handlers = on_handlers(&ast);
    assert!(
        handlers.contains(&".btn-a") && handlers.contains(&".btn-b"),
        "both @on handlers must survive to IR; got: {handlers:?}"
    );

    // The descendant rule is reported, not silent.
    let mut diagnostics = spacetime::diagnostics::DiagnosticCollector::new(src);
    validate_with_html(&ast, &context(html_without_nope()), &mut diagnostics, None);
    let e0602: Vec<_> = diagnostics
        .diagnostics()
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0602)
        .collect();
    assert_eq!(
        e0602.len(),
        1,
        "expected exactly one E0602 for the descendant rule; got: {:?}",
        diagnostics
            .diagnostics()
            .iter()
            .map(|d| (d.code, d.message.as_str()))
            .collect::<Vec<_>>()
    );
    assert!(
        e0602[0].message.contains("body:hover .nope"),
        "E0602 must name the descendant selector, got: {}",
        e0602[0].message
    );
}

/// A rule that cannot be scoped must NOT remove a sibling directive from the
/// BUILD. This is the reporter's exact shape: a pure-CSS descendant rule whose
/// key matches nothing, sitting between two handlers.
#[test]
fn scoping_failure_does_not_remove_sibling_directive_from_build() {
    let src = r#"
        .wrap { $frame number: 0; }
        .btn-a { @on &.click { $frame <- 1; } }
        body:hover .nope { opacity: 1 }
        .btn-b { @on &.click { $frame <- 2; } }
    "#;
    let temp = tempfile::tempdir().expect("tempdir");
    let entry = temp.path().join("index.st");
    std::fs::write(&entry, src).expect("write");
    let compiled = Compiler::from_file(&entry, temp.path())
        .expect("compile should succeed")
        .compile();

    let js = &compiled.js;
    // Both handlers are bound. `.btn-b` was the one GH-23 dropped silently.
    assert!(
        js.contains("registerSelectorInit('.btn-a'"),
        ".btn-a handler missing from build"
    );
    assert!(
        js.contains("registerSelectorInit('.btn-b'"),
        ".btn-b handler missing from build — a failed rule removed a sibling directive"
    );
    assert!(
        js.contains("$frame <- 1") && js.contains("$frame <- 2"),
        "both handler bodies must be emitted; got .btn-a only: {}",
        js.contains("$frame <- 2")
    );
}

/// Template-provided selectors must NOT be flagged by the fix: a scope that
/// only ever matches an element a `@template` renders is legitimately dynamic.
/// My change must not start flagging it as E0602.
#[test]
fn template_provided_selector_is_not_a_false_positive() {
    use spacetime::html::inject_template_classes_from_ast;

    // `.tp-card` exists in NO static markup — it is rendered by `&card`.
    let src = r#"
        @template &card {
            <div class="tp-card"><slot></slot></div>
        }
        .tp-card { @on &.click { $t <- 1; } }
    "#;
    let ast = parse(src).expect("parse");
    assert!(
        on_handlers(&ast).contains(&".tp-card"),
        "the template-scoped handler must survive to IR"
    );

    // Register the template's classes as known (the seam production `check`
    // uses), then validate: `.tp-card` must NOT be flagged.
    let mut ctx = context("<div class=\"wrap\"></div>");
    inject_template_classes_from_ast(&mut ctx, &ast, src);

    let mut diagnostics = spacetime::diagnostics::DiagnosticCollector::new(src);
    validate_with_html(&ast, &ctx, &mut diagnostics, None);
    let e0602 = diagnostics
        .diagnostics()
        .iter()
        .filter(|d| d.code == DiagnosticCode::E0602)
        .count();
    assert_eq!(
        e0602, 0,
        "a template-provided selector must not be flagged E0602; got {}",
        e0602
    );
}
