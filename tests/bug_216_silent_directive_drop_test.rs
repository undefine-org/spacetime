//! BUG-216 — a directive that cannot be parsed must never compile to silence.
//!
//! The reported symptom was an assertion that PASSES while asserting nothing:
//!
//! ```st
//! @assert (false) ("expected 3, got " + rows.length)   // test PASSED
//! ```
//!
//! The bug filed for it guessed the mechanism as "an unmatched directive is
//! dropped, and every caller discards the MatchDiagnostic". That was wrong —
//! measured with a probe, this source produces ZERO diagnostics and the `@assert`
//! DOES match. The real chain has two links, and each is fixed and covered here:
//!
//! 1. The CST's post-arglist loop consumed a SECOND parenthesized group token by
//!    token, so `("m " + a.b)` was eaten as far as the `.` — which the loop treats
//!    as the start of a nested `.class` scope — and broke mid-expression, leaving
//!    `b)` to parse as a new statement ("expected '{' after selector"). Any message
//!    containing a member access or call hit this; `("literal")` did not, which is
//!    why the shape looked arbitrary.
//!
//! 2. That parse error was then SWALLOWED: the opaque-block recompile (FUP-069)
//!    bailed with `let Ok(ast) = parse(..) else { return String::new() }`, so an
//!    unparseable `@test` body compiled to an EMPTY string. The test still
//!    registered, ran nothing, and reported PASS.
//!
//! The second is the more dangerous half and is why this was #A: a body that
//! fails to parse is indistinguishable from a body that legitimately emits
//! nothing, so the failure is invisible to a green suite.

use spacetime::compiler::Compiler;

fn compile_js(src: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, src).expect("write");
    Compiler::from_file(&entry, dir.path())
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .compile()
        .js
}

/// Link 1: the parser must accept a second parenthesized group containing a
/// member access, a call, or an index — the shapes an informative failure
/// message is actually made of.
#[test]
fn second_paren_group_with_member_access_parses() {
    let cases = [
        r#"@assert (false) ("m " + a.b)"#,
        r#"@assert (false) ("m " + rows.length)"#,
        r#"@assert (false) ("m " + f(1))"#,
        r#"@assert (false) ("m " + xs[0])"#,
        r#"@assert (false) ("m " + a.b.c.d)"#,
        // Regressions: the forms that already worked must keep working.
        r#"@assert (false) "plain""#,
        r#"@assert (false) ("literal")"#,
        r#"@assert (false)"#,
    ];
    for case in cases {
        // Parse the directive as a BARE FRAGMENT, exactly as the opaque-block
        // recompile does. Wrapping it in `@test { … }` would hide the defect: the
        // outer program parses fine either way (the braces are well-formed), and
        // only the fragment pass sees the broken expression. An earlier version of
        // this test wrapped the case and PASSED against the unfixed parser.
        assert!(
            spacetime::parser::parse(case).is_ok(),
            "must parse as a bare body fragment, but did not:\n{case}"
        );
    }
}

/// Link 2 — the one that made the bug invisible. An unparseable body must NOT
/// yield an empty string. It emits a throw naming BUG-216, so the failure
/// surfaces the moment the body runs instead of silently doing nothing.
#[test]
fn an_unparseable_directive_body_emits_a_throw_not_silence() {
    // Outer parse SUCCEEDS (the @test directive and its braces are well-formed);
    // only the BODY fragment is unparseable, which is precisely the shape the
    // opaque-block recompile handles — and previously swallowed.
    let src = "@import \"stdlib/testing/test\";\n@test \"t\" {\n  @eval (1) x.y)\n}\n";
    assert!(
        spacetime::parser::parse(src).is_ok(),
        "precondition: the OUTER program must parse, so the failure is isolated to \
         the body fragment the sub-program recompiles"
    );
    let js = compile_js(src);
    assert!(
        js.contains("BUG-216"),
        "a body that fails to parse must emit a diagnostic throw, not compile to \
         nothing — silence here is what let an empty test report PASS"
    );
    assert!(
        js.contains("did not run"),
        "the thrown message must say the body's contents did not run, since that \
         is the consequence an author needs to understand"
    );
}

/// The end-to-end property the bug is really about: an assertion that is present
/// in the source must be present in the output. A `(false)` assertion that
/// compiles to nothing is a test that lies.
#[test]
fn an_assertion_with_an_expression_message_reaches_the_bundle() {
    let src = "@import \"stdlib/testing/test\";\n@test \"t\" {\n  @fixture { <div class=\"x\"></div> }\n  @assert (false) (\"m \" + [1,2].length)\n}\n";
    let js = compile_js(src);
    assert!(
        js.contains("Assertion failed"),
        "the assertion's emit must reach the bundle — it previously vanished \
         entirely, and the test containing it PASSED"
    );
    // The fixture is in the same body; if the body were dropped it would go too.
    assert!(
        js.contains("__ctx"),
        "the surrounding test body must survive — the whole body was dropped, not \
         just the assertion"
    );
}

/// W1.1 REVIEW FINDING — the first fix consumed too much.
///
/// The original patch called `parse_inline_expression()` for the trailing paren
/// group. That function consumes up to a STATEMENT boundary (`;` `{` `}`), not to
/// the group's matching `)`, so after the group closed it kept going and ate the
/// NEXT statement. `@assert (c) ("m " + a.b)` followed by `.after { … }` silently
/// lost the scope — a worse failure than the drop it was fixing, because a valid
/// sibling statement disappears from the page with no error at all.
///
/// The group is now consumed as exactly one balanced `( … )`.
#[test]
fn a_trailing_paren_group_does_not_swallow_the_next_statement() {
    let src = "@assert (false) (\"m \" + a.b)\n.after { color: red; }\n";
    let file = spacetime::parser::parse(src).expect("must parse");
    assert_eq!(
        file.scopes.len(),
        1,
        "the sibling `.after` scope must survive — consuming past the group's \
         matching `)` silently deletes whatever follows the directive"
    );

    // Same shape with a following directive rather than a scope.
    let src2 = "@assert (false) (\"m \" + a.b)\n@assert (true) \"ok\"\n";
    assert!(
        spacetime::parser::parse(src2).is_ok(),
        "a following directive must survive too"
    );
}

/// An UNCLOSED group must fail loudly at EOF, not consume the rest of the file.
/// Silently treating it as closed is how a typo becomes a missing page section.
#[test]
fn an_unclosed_paren_group_errors_instead_of_eating_the_file() {
    let src = "@assert (false) (\"m \" + a.b\n.after { color: red; }\n";
    let err = spacetime::parser::parse(src)
        .err()
        .expect("an unclosed group must be an error, not a silent consume-to-EOF");
    let text = format!("{err:?}");
    assert!(
        text.contains("unclosed"),
        "the error must name the unclosed paren so the author can find it, got: {text}"
    );
}

/// A `@behavior` state-machine body must not be shredded into keyframes.
///
/// The keyframe fallback splits a property at `input.find(':')` — the FIRST
/// colon anywhere in the text. For
///
/// ```st
/// @behavior .t machine(initial: "a") { a -> b on click; }
/// ```
///
/// that colon is the one inside `machine(initial: …)`, so the "property name"
/// became `@behavior .t machine(initial` and the remainder `"a") { a -> b on
/// click; }` was scanned as keyframe data, producing nonsense keyframes
/// `{ at: 0, value: '"a") { a' }` and `{ at: 1, value: 'b on click; }`.
///
/// The body then failed to reparse and was dropped, so `transition_count via
/// claim` in tests/lang/assertions/then-group.test.st ran ZERO assertions.
///
/// Root cause of the misrouting: a captured body is sent to keyframe conversion
/// merely because it contains `->`. In a state machine `a -> b` is a STATE
/// TRANSITION, not an animation keyframe — the same arrow, two grammars.
#[test]
fn a_state_machine_body_is_not_parsed_as_keyframes() {
    let src = "<button class=\"t\">+</button>\n\n\
               .t { @behavior machine(initial: \"a\") { a -> b on click; } }\n";
    let js = compile_js(src);
    assert!(
        !js.contains("machine(initial"),
        "`machine(initial` appearing as an emitted PROPERTY NAME means the body \
         was split at the colon inside `machine(…)` and shredded into keyframes:\n{js}"
    );
}

/// Over-widening guard: a genuine keyframe body must still convert.
///
/// "Do not treat this as keyframes" is trivially satisfiable by never treating
/// anything as keyframes, which would silently kill every animation.
#[test]
fn a_real_keyframe_body_still_converts() {
    let src = "<div class=\"x\">hi</div>\n\n\
               .x { @on &.click { opacity: 0 -> 1; } }\n";
    let js = compile_js(src);
    assert!(
        js.contains("opacity"),
        "a real `opacity: 0 -> 1` keyframe must still reach the bundle:\n{js}"
    );
}


/// BUG-356. A bodyless directive followed by a newline-led `:root` or `*` rule.
///
/// `inline_args` breaks out of a directive's inline run when the next line
/// begins a new statement — it had guards for `.class`, `#id`, `<el`, a bare
/// element selector (`h1 { … }`), `&ref`, and `ident:` declarations. `:root`
/// and `*` were missing from that list, so a terminator-less directive
/// swallowed the selector as an argument and its `{ … }` as the directive's
/// body.
///
/// The author-visible result was E0946 blamed on the DIRECTIVE, up to ~94 lines
/// above the rule that actually caused it — which is why the report read as
/// "the error points at the wrong line". `demos/editable-blog/index.st` failed
/// to build for exactly this reason: a bare `@edit-toggle` on line 44 followed
/// (after a comment block) by `:root { … }`.
///
/// The guarded selectors were already a list; these two are the members that
/// were missing from it, not a new mechanism.
#[test]
fn a_bodyless_directive_does_not_swallow_a_following_root_or_star_rule() {
    for selector in [":root", "*"] {
        let src = format!(
            "@import \"stdlib/editable\"\n\n\
             <main><article class=\"post-body\">hi</article></main>\n\n\
             @edit-toggle\n\n\
             {selector} {{ color: #16161a; }}\n"
        );
        // Driven through the real binary: this defect lives in the parse of a
        // WHOLE file (a directive's inline run reaching across a newline), so
        // the instrument has to be the same one an author uses.
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scratch")
            .join(format!(
                "bug356-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let entry = dir.join("index.st");
        std::fs::write(&entry, &src).expect("write");
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_spacetime"))
            .arg("check")
            .arg(&entry)
            .output()
            .expect("spacetime binary should run");
        let _ = std::fs::remove_dir_all(&dir);
        let mut text = String::from_utf8_lossy(&out.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        assert!(
            !text.contains("E0946"),
            "a bodyless directive followed by `{selector} {{ … }}` must not \
             report E0946 against the directive — the rule begins a NEW \
             statement:\n{text}"
        );
    }
}

/// Over-widening companion. Admitting `:` and `*` as statement starters must not
/// make them stop an inline run MID-DIRECTIVE, where both are ordinary syntax:
/// `*` multiplies and `:` introduces a value. Only a NEWLINE-led occurrence that
/// actually reaches a `{` begins a new rule.
///
/// `@data derive $area : $w * 3;` puts BOTH on the directive's inline run — the
/// value colon and a multiplication — so a guard that fired on the token rather
/// than on "newline-led AND reaches a brace" truncates the expression here.
#[test]
fn colon_and_star_still_work_inside_a_directive() {
    let js = compile_js(
        "<main><div class=\"box\">x</div></main>\n\n\
         $w number: 4;\n\
         @data derive $area : $w * 3;\n",
    );
    assert!(
        js.contains("* 3") || js.contains("*3"),
        "`$w * 3` in a directive's value is multiplication on the inline run \
         and must survive the newline-led statement guard"
    );
}
