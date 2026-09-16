//! FUP-079: a bare identifier passed to a `:string` macro argument is accepted
//! as a string literal (the unquoted enum-ish surface: `mode: bounce`).
//!
//! Before the fix, a bare ident in a NAMED `:string` slot (`@mm(mode: hello)`)
//! produced NO FormMatch, collapsing the page's runtime JS to 0 bytes — silently
//! (`check` stayed green, the page went blank, no diagnostic). Quoting worked.
//! Now both surfaces (leading-token and named-arg) accept a bare ident through
//! the single `StringExtractor` chokepoint, so the page emits identically.
//!
//! These tests drive the FULL parse → user-macro rematch → emit pipeline (the
//! `StringExtractor` unit tests in src cover the extractor in isolation).

use spacetime::compiler::Compiler;
use spacetime::parser::{parse, rematch_with_user_macros_in};
use spacetime::syntax::CapturedValue;
use std::path::Path;

/// Parse + rematch with the file's own user macros, returning the captures of the
/// first match for `macro_name`.
fn captures_of(
    src: &str,
    macro_name: &str,
) -> Option<std::collections::HashMap<String, CapturedValue>> {
    let mut ast = parse(src).expect("parse");
    if !ast.meta_defs.is_empty() {
        rematch_with_user_macros_in(&mut ast, src, None);
    }
    ast.matches
        .iter()
        .find(|m| m.macro_name == macro_name)
        .map(|m| m.captures.clone())
}

/// Compile a page at repo root so `@import "stdlib"` resolves, returning JS length.
fn js_len(src: &str, stem: &str) -> usize {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = root.join(format!("target/tmp-fup079-{stem}.st"));
    std::fs::create_dir_all(p.parent().unwrap()).ok();
    std::fs::write(&p, src).unwrap();
    let out = Compiler::from_file(&p, root).unwrap().compile();
    std::fs::remove_file(&p).ok();
    assert!(
        out.pipeline_errors.is_empty(),
        "pipeline errors: {:?}",
        out.pipeline_errors
    );
    out.js.len()
}

#[test]
fn named_string_arg_accepts_bare_ident() {
    let src = "%macro mm { %form { @mm(mode: $mode:string) } }\n\
               body { margin: 0; }\n\
               h1 { @mm(mode: hello) }\n";
    let caps = captures_of(src, "mm").expect("@mm(mode: hello) must FormMatch with a bare ident");
    assert_eq!(
        caps.get("mode"),
        Some(&CapturedValue::String("hello".to_string())),
        "bare ident in a named :string arg must capture as String(\"hello\")"
    );
}

#[test]
fn leading_string_arg_accepts_bare_ident() {
    let src = "%macro lt { %form { @lt $mode:string } }\n\
               body { margin: 0; }\n\
               h1 { @lt hello }\n";
    let caps = captures_of(src, "lt").expect("@lt hello (leading token) must FormMatch");
    assert_eq!(
        caps.get("mode"),
        Some(&CapturedValue::String("hello".to_string())),
        "leading-token :string must also accept a bare ident (surface harmony)"
    );
}

#[test]
fn bare_and_quoted_are_equivalent() {
    let bare = "%macro mm { %form { @mm(mode: $mode:string) } }\n\
                body { margin: 0; }\n\
                h1 { @mm(mode: hello) }\n";
    let quoted = "%macro mm { %form { @mm(mode: $mode:string) } }\n\
                  body { margin: 0; }\n\
                  h1 { @mm(mode: \"hello\") }\n";
    assert_eq!(
        captures_of(bare, "mm").unwrap().get("mode"),
        captures_of(quoted, "mm").unwrap().get("mode"),
        "bare and quoted forms must capture the same String value"
    );
}

#[test]
fn bare_ident_table_of_enum_values() {
    // Property-style: a spread of realistic enum-ish values (incl. keyword-spelled
    // and hyphenated) must each capture verbatim as a string.
    let values = ["pingpong", "glass", "aces", "from", "to", "morph", "bottom"];
    for v in values {
        let src = format!(
            "%macro mm {{ %form {{ @mm(mode: $mode:string) }} }}\nh1 {{ @mm(mode: {v}) }}\n"
        );
        let caps = captures_of(&src, "mm").unwrap_or_else(|| panic!("[{v}] must FormMatch"));
        assert_eq!(
            caps.get("mode"),
            Some(&CapturedValue::String(v.to_string())),
            "[{v}] bare enum value must capture verbatim"
        );
    }
}

#[test]
fn expression_head_ident_does_not_match_bare() {
    // A bare ident that heads a larger expression (call / member / index) must NOT
    // be captured piecemeal — the value must stay quoted or use :expr. The @mm
    // match therefore does not fire (no partial capture).
    for bad in ["rgba(0,0,0,0.1)", "a.b", "items[0]"] {
        let src = format!(
            "%macro mm {{ %form {{ @mm(mode: $mode:string) }} }}\nh1 {{ @mm(mode: {bad}) }}\n"
        );
        assert!(
            captures_of(&src, "mm").is_none(),
            "[{bad}] an expression-head ident must NOT match a bare :string slot"
        );
    }
}

#[test]
fn bare_ident_named_arg_emits_nonempty_runtime() {
    // End-to-end emit proof of the reported symptom: the page's runtime JS must be
    // non-empty (was 0 bytes) and byte-equivalent to the quoted form.
    //
    // The macro carries an `%emit` because a `%form` alone declares a SHAPE and
    // binds nothing — `@mm` would then be an unknown primitive (E0956), which is
    // what the leniency removal now says out loud. Since this test asserts on
    // EMITTED bytes, the macro has to actually emit; `%$mode` interpolates the
    // captured string already quoted, so the bare and quoted surfaces must
    // produce byte-identical output. That equality IS the subject of the test.
    let bare = "%macro mm { %form { @mm(mode: $mode:string) } %emit js { const m = %$mode; } }\n\
                body { margin: 0; }\n\
                h1 { @mm(mode: hello) }\n";
    let quoted = "%macro mm { %form { @mm(mode: $mode:string) } %emit js { const m = %$mode; } }\n\
                  body { margin: 0; }\n\
                  h1 { @mm(mode: \"hello\") }\n";
    let bare_len = js_len(bare, "bare");
    let quoted_len = js_len(quoted, "quoted");
    assert!(
        bare_len > 0,
        "bare-ident named :string arg must emit a non-empty runtime (was 0 bytes)"
    );
    assert_eq!(
        bare_len, quoted_len,
        "bare and quoted forms must emit identical runtime JS"
    );
}

#[test]
fn stdlib_loop_mode_accepts_bare_ident() {
    // Real stdlib surface: `@on &.loop(..., mode: $mode:string = "pingpong", ...)`.
    // An unquoted `mode: bounce` must lower correctly (proves the fix reaches the
    // committed stdlib surface, not just synthetic test macros).
    //
    // Written against `@on &.loop` and NOT `@loop`: the latter was retired by the
    // `on-cutover` capsule (stdlib/migrations/entries/2026-07-26-on-cutover.st),
    // which carries `%rewrite loop-positional` / `loop-named` targeting exactly
    // this form. A test pinned to the retired spelling asserts against a surface
    // the migration exists to remove — it would go red again the moment the
    // capsule's wave lands (E0911), for a reason having nothing to do with the
    // bare-ident lowering it means to prove.
    let src = "@import \"stdlib\"\n\
               .spinner {\n\
                 @on &.loop(name: spin, duration: 2s, mode: bounce) { transform: rotate(0deg) -> rotate(360deg); }\n\
               }\n";
    let len = js_len(src, "loop");
    assert!(
        len > 0,
        "stdlib @loop with bare-ident mode must emit a non-empty runtime"
    );
}
