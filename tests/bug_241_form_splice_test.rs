//! BUG-241 — statement-position form splices (`--name;`) dispatch, and an
//! unknown name is a HARD error (E0947), never another silent drop.
//!
//! Before the `parse_body_item` dispatch arm, a leading-dash IDENT fell into
//! the CSS-property fallthrough, whose no-colon arm just BUMPED the ident:
//! `--card-surface;` produced no declaration, no CST node, and no diagnostic.
//!
//! The arm RECOGNIZES the splice (FORM_REF → `ScopeBlock::form_refs`), and
//! `validate_form_refs` checks every splice against the forms declared by
//! macros registering into category `form` (registry DATA — stdlib form.st or
//! a same-file `%macro` with `%registers form(...)`). EXPANSION of a validated
//! splice (splicing the declared body into the scope) is W3 scope and
//! deliberately NOT asserted here.

use std::process::Command;

fn write_temp(source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "bug241-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");
    file
}

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let file = write_temp(source);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacetime"));
    cmd.arg(args[0]).arg(&file).args(&args[1..]);
    let out = cmd.output().expect("run spacetime CLI");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    (out.status.success(), combined)
}

/// THE BUG INVERSION: an unknown statement-position `--name;` is a hard
/// E0947 error at the splice's own span — today it is silently dropped.
#[test]
fn unknown_form_is_a_hard_error() {
    let src = ".card {\n  --card-suface;\n  color: white;\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "an unknown form splice must FAIL check:\n{out}");
    assert!(out.contains("E0947"), "the error must be E0947:\n{out}");
    assert!(
        out.contains("--card-suface"),
        "the error must name the unknown form:\n{out}"
    );
    assert!(
        out.contains("@form"),
        "the hint must point at the declaration surface:\n{out}"
    );
}

/// A CSS custom property (`--brand: red;`) is NOT a form splice — the `:`
/// guard keeps it on the property path, and it must still be kept.
#[test]
fn custom_properties_are_unaffected() {
    let src = ".a { --brand: red; color: white; }\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(passed, "a custom property must still compile:\n{out}");

    // Observable effect, asserted on the AST rather than inspect output
    // (inspect wraps labels in ANSI color codes, which a byte-contains
    // assertion cannot see through).
    let ast = spacetime::parser::parse(src).expect("parse");
    let scope = ast
        .scopes
        .iter()
        .find(|s| s.selector == ".a")
        .expect("scope");
    assert_eq!(
        scope.css_declarations.len(),
        2,
        "the custom property AND the plain property must both be kept"
    );
    assert!(
        scope
            .css_declarations
            .iter()
            .any(|d| d.property == "--brand"),
        "the custom property keeps its dashed name, with sigil:\n{:?}",
        scope.css_declarations
    );
    assert!(
        scope.form_refs.is_empty(),
        "a custom property is NOT a form splice:\n{:?}",
        scope.form_refs
    );
}

/// A declared form splices cleanly — file-level AND scope-level declaration.
#[test]
fn a_declared_form_splices() {
    for (pos, src) in [
        (
            "file",
            "@form style --card-surface { background: #121722; }\n\n.card {\n  --card-surface;\n  color: white;\n}\n",
        ),
        (
            "scope",
            ".host {\n  @form style --card-surface { background: #121722; }\n  .card {\n    --card-surface;\n  }\n}\n",
        ),
    ] {
        let (passed, out) = run_cli(src, &["check"]);
        assert!(
            passed,
            "a form declared at {pos} level must splice without error:\n{out}"
        );
    }
}

/// The splice is CARRIED, not dropped: it reaches `ScopeBlock::form_refs`\n/// with its sigil and its arguments. This is the observable that replaces\n/// "no diagnostic" — a recognized ref is inert until W3, never invisible.
#[test]
fn the_splice_is_carried_with_its_args() {
    let src = "@form motion --rise($distance = 24px) { opacity: 0 -> 1; }\n\n.card {\n  --rise(12px);\n}\n";
    let file = write_temp(src);
    let content = std::fs::read_to_string(&file).unwrap();
    let ast = spacetime::parser::parse(&content).expect("parse");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());

    let scope = ast
        .scopes
        .iter()
        .find(|s| s.selector == ".card")
        .expect("the .card scope exists");
    assert_eq!(scope.form_refs.len(), 1, "one splice, carried");
    let form_ref = &scope.form_refs[0];
    assert_eq!(form_ref.name, "--rise", "the name keeps its sigil");
    assert_eq!(
        form_ref.args.as_deref(),
        Some("12px"),
        "the call-site args are carried"
    );
}

/// A macro that MERELY EXISTS is not enough — the arm consults the REGISTRY.
/// A user `%macro` whose `%form` mentions a dashed ident but which declares
/// NO `%registers form(...)` must not legitimize the splice.
#[test]
fn macro_presence_without_registration_still_errors() {
    let src = r#"
%macro fake-form {
  %order 700
  %form { @fake $name:dashed_ident { $body:properties } }
  %binds { noop(&self, decl: $name) }
}

@fake --ghost { color: red; }

.card {
  --ghost;
}
"#;
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        !passed,
        "a form-looking macro with no %registers form(...) must NOT legitimize \
         the splice — validation consults the registry, not macro presence:\n{out}"
    );
    assert!(
        out.contains("E0947"),
        "still the unknown-form error:\n{out}"
    );
}

/// Combinator regressions (BUG-243's arm-order lesson): the new IDENT-arm
/// branch must not disturb `>`, `+`, `~` combinators or `&` blocks.
#[test]
fn combinators_still_dispatch_first() {
    let src =
        ".a {\n  > .b { color: red; }\n  ~ .c { color: blue; }\n  + .d { color: green; }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(passed, "combinators must keep dispatching:\n{out}");
    let ast = spacetime::parser::parse(src).expect("parse");
    let scope = ast
        .scopes
        .iter()
        .find(|s| s.selector == ".a")
        .expect("scope");
    assert_eq!(
        scope.nested_scopes.len(),
        3,
        "all three combinator blocks must nest:\n{:?}",
        scope
            .nested_scopes
            .iter()
            .map(|n| &n.selector)
            .collect::<Vec<_>>()
    );
}

// --- W2 review findings (2 reviewers, both "incorrect") ----------------------

/// FILE-ROOT splice: previously hit error_recover, which check COUNTED but
/// never rendered — a file with a root-level splice failed with no diagnostic
/// at all. Now recognized and validated like any other.
#[test]
fn root_level_splice_is_a_rendered_error() {
    let src = "--ghost;\n\n.a { color: red; }\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "an unknown root-level splice must fail:\n{out}");
    assert!(
        out.contains("E0947") && out.contains("--ghost"),
        "the failure must be RENDERED (named, coded) — the silent-count \
         failure this replaces gave the author nothing:\n{out}"
    );
}

/// DIRECTIVE-BODY splice: `@form motion --reveal { --fade-in; … }` is
/// SIP-001c's composition case. The parser recognized the FORM_REF but the
/// carry only walked scope bodies, so a typo'd nested splice was accepted
/// with no validation. The sweep closes it.
#[test]
fn directive_body_splice_is_validated() {
    let src = "@form motion --reveal {\n  --fade-in;\n  opacity: 0 -> 1;\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        !passed,
        "an unknown form inside a directive body must fail:\n{out}"
    );
    assert!(out.contains("E0947"), "must be E0947:\n{out}");

    // …and the same body with the form DECLARED composes clean.
    let ok = "@form motion --fade-in { opacity: 0 -> 1; }\n\n@form motion --reveal {\n  --fade-in;\n  opacity: 0 -> 1;\n}\n";
    let (passed, out) = run_cli(ok, &["check"]);
    assert!(passed, "a declared nested splice must compose:\n{out}");
}

/// TRAILING JUNK: `@form motion --x junk { … }` previously MATCHED — the
/// inline grammar consumed `motion` and `--x` and IGNORED `junk`, registering
/// a form the author never declared. A form-declaring macro's inline grammar
/// is a contract (scoped: stdlib grammars like `@preset`/`@loop` rely on the
/// historical leniency, whose general tightening is its own measured wave).
#[test]
fn trailing_junk_after_a_valid_declaration_is_rejected() {
    let src = "@form motion --x junk { opacity: 0 -> 1; }\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "trailing tokens must be rejected:\n{out}");
    assert!(
        out.contains("E0946"),
        "the declared grammar must be enforced (E0946):\n{out}"
    );
}

/// COMMENT-SEPARATED custom property: `--brand /* note */ : red;` — comments
/// are trivia. Before the arm this was ALREADY dropped silently (the
/// property lookahead skipped only WHITESPACE); the arm made it loud; now it
/// simply WORKS.
#[test]
fn comment_separated_custom_property_is_a_property() {
    let src = ".a { --brand /* note */ : red; color: white; }\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        passed,
        "a comment-separated custom property must compile:\n{out}"
    );
    let ast = spacetime::parser::parse(src).expect("parse");
    let scope = ast
        .scopes
        .iter()
        .find(|s| s.selector == ".a")
        .expect("scope");
    assert!(
        scope
            .css_declarations
            .iter()
            .any(|d| d.property == "--brand"),
        "the custom property must be kept, sigil intact:\n{:?}",
        scope.css_declarations
    );
    assert!(
        scope.form_refs.is_empty(),
        "it is not a splice:\n{:?}",
        scope.form_refs
    );
}

/// IMPORTED-FILE splice: merge_ast discarded the imported file's diagnostics,
/// so an unknown splice in an imported file vanished whenever the merged AST
/// had no user macros to trigger a rematch. Diagnostics now travel with the
/// file.
#[test]
fn an_unknown_splice_in_an_imported_file_errors() {
    let dir = std::env::temp_dir().join(format!(
        "bug241-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("other.st"), ".b {\n  --ghost;\n}\n").unwrap();
    let main = dir.join("index.st");
    std::fs::write(&main, "@import \"./other.st\"\n\n.a { color: red; }\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&main)
        .output()
        .expect("run spacetime check");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        !out.status.success(),
        "an unknown splice in an IMPORTED file must fail the importing check:\n{combined}"
    );
    assert!(
        combined.contains("E0947"),
        "the imported file's diagnostic must travel through the merge:\n{combined}"
    );
}
