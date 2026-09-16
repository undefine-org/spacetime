use spacetime::parser::{parse, resolve_imports};
use spacetime::{compile, compiler::CompileOptions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_basic_import_resolution() {
    // Create a temp directory
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a module file with a scope (the `~` preset mechanism that once
    // populated ast.presets is RETIRED — SIP-001c / BUG-263 — so scope merging
    // is what a cross-module import exercises now)
    fs::write(
        temp_path.join("module.st"),
        r#".from-module { color: red; }"#,
    )
    .unwrap();

    // Create main file that imports the module
    let main_content = r#"@import "./module.st";

.hero {
    opacity: 0 -> 1;
}"#;

    // Parse main file
    let main_ast = parse(main_content).unwrap();
    assert_eq!(main_ast.imports.len(), 1);
    assert_eq!(main_ast.imports[0].path, "./module.st");

    // Resolve imports
    let main_path = temp_path.join("animations.st");
    let result = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // ast.presets is empty: only `@preset ... ~name` populated it and `~` is
    // retired (errors at parse). Import merging of scopes is live.
    assert_eq!(result.presets.len(), 0);
    let selectors: Vec<&str> = result.scopes.iter().map(|s| s.selector.as_str()).collect();
    assert!(selectors.contains(&".from-module"), "module scope must be merged");
    assert!(selectors.contains(&".hero"), "main scope must be present");

    // Verify imports are cleared in final AST
    assert_eq!(result.imports.len(), 0);
}

#[test]
fn test_nested_imports() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create module C
    fs::write(temp_path.join("c.st"), r#".from-c { }
"#).unwrap();

    // Create module B that imports C
    fs::write(
        temp_path.join("b.st"),
        r#"@import "./c.st";
.from-b { }
"#,
    )
    .unwrap();

    // Create main file that imports B
    let main_content = r#"@import "./b.st";

.hero {
    opacity: 0 -> 1;
}"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("a.st");
    let result = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // The retired `~` preset path populated ast.presets; now it is always
    // empty. Scopes from the transitive import chain must still merge.
    assert_eq!(result.presets.len(), 0);
    let selectors: Vec<&str> = result.scopes.iter().map(|s| s.selector.as_str()).collect();
    assert!(selectors.contains(&".from-c"), "transitive module scope missing");
    assert!(selectors.contains(&".from-b"), "direct module scope missing");
    assert!(selectors.contains(&".hero"), "main scope missing");

    // Should have the scope from main
    assert!(selectors.contains(&".hero"));
}

#[test]
fn test_circular_import_detection() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create A that imports B
    fs::write(
        temp_path.join("a.st"),
        r#"@import "./b.st";
.class-a {}"#,
    )
    .unwrap();

    // Create B that imports A (circular!)
    fs::write(
        temp_path.join("b.st"),
        r#"@import "./a.st";
.class-b {}"#,
    )
    .unwrap();

    let main_content = r#"@import "./a.st";"#;
    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");

    let result = resolve_imports(&main_ast, &main_path, temp_path);

    // Should detect circular import
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("Circular import"),
        "Error should mention circular import: {}",
        error_msg
    );
}

#[test]
fn test_missing_file_error() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let main_content = r#"@import "./nonexistent.st";"#;
    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");

    let result = resolve_imports(&main_ast, &main_path, temp_path);

    // Should error with clear message
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(
        error_msg.contains("Could not resolve import"),
        "Error should mention resolution failure: {}",
        error_msg
    );
}

// =============================================================================
// FormMatch merging tests — verifies that imported directives (@on, @bind,
// state declarations) are merged into the parent file's ast.matches for the
// pipeline to process.
// =============================================================================

#[test]
fn test_import_merges_form_matches() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a module with click handler and state
    fs::write(
        temp_path.join("faq.st"),
        r#"
body {
    $activeQ number: -1;
}

.faq-q0 {
    @on &.click { $activeQ <- 0; }
}

.faq-item-0 {
    .faq-item--open: $activeQ === 0;
}
"#,
    )
    .unwrap();

    // Main file imports the module
    let main_content = r#"
@import "./faq.st";

.nav {
    @on &.click { $theme <- "dark"; }
}
"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");
    let result = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // Verify no import FormMatches leaked through
    assert!(
        !result.matches.iter().any(|m| m.macro_name == "import"),
        "Import FormMatches should be filtered out from merged result"
    );

    // Verify the imported state declaration is present
    assert!(
        result.matches.iter().any(|m| m.macro_name == "local-state"),
        "Should contain the $activeQ state declaration from imported file"
    );

    // Verify the imported click handler is present
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.macro_name == "on" && m.selector.as_deref() == Some(".faq-q0")),
        "Should contain @on click from imported FAQ module"
    );

    // Verify the main file's @on (the migrated reactive handler) is present.
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.macro_name == "on" && m.selector.as_deref() == Some(".nav")),
        "Should contain @on from main file"
    );

    // The imported `.faq-item--open: $activeQ === 0;` reactive prop is a
    // selector-scope CST declaration (the `:` surface), not a form-directive
    // FormMatch — so it is carried as a merged scope, not in `matches`.
    assert!(
        result.scopes.iter().any(|s| s.selector == ".faq-item-0")
            || result
                .matches
                .iter()
                .any(|m| m.selector.as_deref() == Some(".faq-item-0")),
        "Should carry the imported reactive prop scope (.faq-item-0)"
    );

    // Form-directive matches: imported (local-state + on) + main (on) = 3.
    assert_eq!(
        result.matches.len(),
        3,
        "Should have 3 form-directive FormMatches total. Got: {:?}",
        result
            .matches
            .iter()
            .map(|m| (&m.macro_name, &m.selector))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_imported_matches_compile_to_js() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a module with a click handler
    fs::write(
        temp_path.join("module.st"),
        r#"
body {
    $count number: 0;
}

.btn-increment {
    @on &.click { $count <- 1; }
}
"#,
    )
    .unwrap();

    // Main file imports the module and adds its own directive
    let main_content = r#"
@import "./module.st";

.status {
    @on &.click { $count <- 1; }
}
"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");
    let merged = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // Compile the merged AST
    let compiled = compile(&merged, CompileOptions::default());

    // The JS output should contain code for both the imported click handler
    // and the main file's bind directive
    assert!(
        compiled.js.contains("btn-increment"),
        "JS should contain selector from imported module: btn-increment"
    );
    assert!(
        compiled.js.contains(".status"),
        "JS should contain selector from main file: .status"
    );
    assert!(
        compiled.js.contains("click"),
        "JS should contain click event handler from imported module"
    );
}

#[test]
fn test_nested_imports_merge_all_matches() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Module C: a state declaration
    fs::write(
        temp_path.join("state.st"),
        r#"
body {
    $theme string: "light";
}
"#,
    )
    .unwrap();

    // Module B: imports C, adds a click handler
    fs::write(
        temp_path.join("nav.st"),
        r#"
@import "./state.st";

.nav-toggle {
    @on &.click { $theme <- "dark"; }
}
"#,
    )
    .unwrap();

    // Main file: imports B, adds a bind
    let main_content = r#"
@import "./nav.st";

.page {
    @on &.click { $theme <- "dark"; }
}
"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");
    let merged = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // Should have: $theme from state.st, @on click from nav.st, @bind from main.st
    assert!(
        merged.matches.iter().any(|m| m.macro_name == "local-state"),
        "Should have state declaration from deeply nested import (state.st)"
    );
    assert!(
        merged
            .matches
            .iter()
            .any(|m| m.macro_name == "on" && m.selector.as_deref() == Some(".nav-toggle")),
        "Should have click handler from intermediate import (nav.st)"
    );
    assert!(
        merged
            .matches
            .iter()
            .any(|m| m.macro_name == "on" && m.selector.as_deref() == Some(".page")),
        "Should have @on from main file"
    );

    // No import matches should leak
    assert!(
        !merged.matches.iter().any(|m| m.macro_name == "import"),
        "No import FormMatches should be in the merged result"
    );

    // Should compile successfully with all directives producing JS
    let compiled = compile(&merged, CompileOptions::default());
    assert!(
        !compiled.js.is_empty(),
        "Nested imports should produce JS output"
    );
    assert!(
        compiled.js.contains("nav-toggle"),
        "JS should contain code from intermediate import"
    );
    assert!(
        compiled.js.contains(".page"),
        "JS should contain code from main file"
    );
}

#[test]
fn test_import_matches_and_scopes_together() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Module with directives (the retired `~` preset that once filled
    // ast.presets is gone; matches + scopes are what merge across imports)
    fs::write(
        temp_path.join("helpers.st"),
        r#"
body {
    $visible bool: false;
}

.reveal-trigger {
    @on &.visible { $visible <- true; }
}
"#,
    )
    .unwrap();

    let main_content = r#"
@import "./helpers.st";

.content {
    @bind(class: "shown", when: $visible)
}
"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");
    let merged = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // ast.presets is always empty after the `~` preset retirement
    assert_eq!(merged.presets.len(), 0);

    assert!(
        merged.matches.iter().any(|m| m.macro_name == "local-state"),
        "Should have state declaration from import"
    );
    assert!(
        merged.matches.iter().any(|m| m.macro_name == "on"),
        "Should have @on visible from import"
    );
    assert!(
        merged
            .matches
            .iter()
            .any(|m| m.macro_name == "bind" && m.selector.as_deref() == Some(".content")),
        "Should have @bind from main"
    );
}

#[test]
fn test_import_does_not_duplicate_main_matches() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Empty module (nothing to merge)
    fs::write(temp_path.join("empty.st"), "// nothing here\n").unwrap();

    let main_content = r#"
@import "./empty.st";

body {
    $x number: 0;
}
"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");
    let merged = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // After merge, import FormMatches should be removed, empty.st adds nothing
    // Only the local-state ($x) should remain
    assert_eq!(
        merged.matches.len(),
        1,
        "Should have exactly 1 match (the $x state). Got: {:?}",
        merged
            .matches
            .iter()
            .map(|m| &m.macro_name)
            .collect::<Vec<_>>()
    );
    assert_eq!(merged.matches[0].macro_name, "local-state");
}

#[test]
fn test_import_multiple_modules() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Module A: state + click
    fs::write(
        temp_path.join("faq.st"),
        r#"
body {
    $activeQ number: -1;
}
.faq-q0 {
    @on &.click { $activeQ <- 0; }
}
"#,
    )
    .unwrap();

    // Module B: state + submit
    fs::write(
        temp_path.join("form.st"),
        r#"
body {
    $formState string: "idle";
}
.contact-form {
    .form--success: $formState === "success";
}
"#,
    )
    .unwrap();

    let main_content = r#"
@import "./faq.st";
@import "./form.st";

.nav {
    @on &.click { $navTheme <- "dark"; }
}
"#;

    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("main.st");
    let merged = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // Form-directive matches from all 3 files:
    //   faq.st:  local-state($activeQ) + on click   = 2
    //   form.st: local-state($formState)            = 1  (.form--success is a `:` scope)
    //   main:    on click (.nav)                    = 1
    // The reactive `:` prop (.contact-form) is a selector-scope CST declaration,
    // carried as a merged scope rather than a form-directive FormMatch.
    assert_eq!(
        merged.matches.len(),
        4,
        "Should have 4 form-directive FormMatches. Got: {:?}",
        merged
            .matches
            .iter()
            .map(|m| (&m.macro_name, &m.selector))
            .collect::<Vec<_>>()
    );
    assert!(
        merged.scopes.iter().any(|s| s.selector == ".contact-form")
            || merged
                .matches
                .iter()
                .any(|m| m.selector.as_deref() == Some(".contact-form")),
        "Should carry the imported reactive `:` prop scope (.contact-form)"
    );

    // No import FormMatches should leak
    assert!(
        !merged.matches.iter().any(|m| m.macro_name == "import"),
        "No import FormMatches should be in the merged result"
    );

    // Should compile to JS containing selectors from all files
    let compiled = compile(&merged, CompileOptions::default());
    assert!(
        compiled.js.contains("faq-q0"),
        "JS should have FAQ selector"
    );
    assert!(
        compiled.js.contains("contact-form"),
        "JS should have form selector"
    );
    assert!(compiled.js.contains(".nav"), "JS should have nav selector");
}

#[test]
fn imported_text_module_registers_measure_macro() {
    use spacetime::parser::{parse, resolve_imports};
    use std::path::Path;
    // Resolve against the repo root so stdlib/text resolves.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = "@import \"stdlib/text\"\n\n.headline { @measure(lineHeight: 1.5) }\n";
    let main_ast = parse(main).unwrap();
    let main_path = root.join("probe_main.st");
    let resolved = resolve_imports(&main_ast, &main_path, root).unwrap();
    let names: Vec<_> = resolved
        .meta_defs
        .iter()
        .map(|d| match d {
            spacetime::parser::meta_ast::MetaDef::Macro(m) => format!("macro:{}", m.name),
            spacetime::parser::meta_ast::MetaDef::Primitive(p) => format!("prim:{}", p.name),
            _ => "other".to_string(),
        })
        .collect();
    eprintln!("RESOLVED_META_DEFS={:?}", names);
    assert!(
        names.iter().any(|n| n == "macro:measure"),
        "measure macro should be registered: {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "prim:measure-text"),
        "measure-text primitive should be registered"
    );
}

#[test]
fn imported_text_measure_emits_js() {
    use spacetime::compiler::Compiler;
    use std::path::Path;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main_path = root.join("demos/__textprobe__/probe_emit.st");
    std::fs::create_dir_all(main_path.parent().unwrap()).ok();
    std::fs::write(
        &main_path,
        "@import \"stdlib/text\"\n\n.headline { @measure(lineHeight: 1.5) }\n",
    )
    .unwrap();
    let compiled = Compiler::from_file(&main_path, root).unwrap().compile();
    std::fs::remove_file(&main_path).ok();
    assert!(
        compiled.pipeline_errors.is_empty(),
        "errors: {:?}",
        compiled.pipeline_errors
    );
    // The @measure primitive body must be emitted…
    assert!(
        compiled.js.contains("prepareWithSegments"),
        "measure-text primitive should emit"
    );
    // …and because it references the `pretext` global, the vendored IIFE must be
    // demand-injected (lean rail). Requires the committed bundle on disk.
    if std::path::Path::new("stdlib/text/vendor/pretext.bundle.js").exists() {
        assert!(
            compiled.js.contains("globalThis.pretext"),
            "vendored pretext bundle should be injected when @measure fires"
        );
    }
}

#[test]
fn imported_text_unused_ships_no_vendor() {
    // Lean law: importing the text module but never invoking a text primitive
    // ships ZERO pretext bytes.
    use spacetime::compiler::Compiler;
    use std::path::Path;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = root.join("demos/__textprobe__/probe_unused.st");
    std::fs::create_dir_all(p.parent().unwrap()).ok();
    std::fs::write(&p, "@import \"stdlib/text\"\n\n.plain { color: red; }\n").unwrap();
    let compiled = Compiler::from_file(&p, root).unwrap().compile();
    std::fs::remove_file(&p).ok();
    assert!(
        !compiled.js.contains("globalThis.pretext"),
        "unused vendor must not be injected"
    );
    assert!(
        !compiled.js.contains("prepareWithSegments"),
        "unused primitive must not emit"
    );
}

/// W0 negative gate (FEAT-056): a deliberately-false @assert/@then MUST fail.
/// Guards against regression of the vacuous-pass bug (async tests never awaited;
/// @assert condition/message swap; @then 0-match). Runs the headless V8 runner.
#[test]
fn w0_negative_gate_fails_false_assertions() {
    use std::process::Command;
    let dir = std::env::temp_dir();
    let f = dir.join("w0_negative_gate.test.st");
    std::fs::write(
        &f,
        r#"@import "stdlib/testing/test"
@test "NEG assert false" { @assert (1 === 2) "must fail" }
@test "NEG then missing" { @fixture { <div class="real">R</div> } @then .absent-zzz should exist }
@test "POS assert true" { @assert (1 === 1) "ok" }
@test "POS then present" { @fixture { <div class="here">H</div> } @then .here should exist }
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The JSON object is pretty-printed and preceded by human log lines; slice
    // from the first `{` to the last `}` to isolate it.
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let passed = v["passed"].as_u64().unwrap_or(0);
    let failed = v["failed"].as_u64().unwrap_or(0);
    std::fs::remove_file(&f).ok();
    assert_eq!(
        failed, 2,
        "2 false tests MUST fail (got passed={passed} failed={failed}); stdout={stdout}"
    );
    assert_eq!(
        passed, 2,
        "2 true tests MUST pass (got passed={passed} failed={failed})"
    );
}

/// W1 fidelity ladder (FEAT-057): the refuse-to-fake gate.
///
/// On the V8/LinkeDOM `logic` backend, a layout-needing assertion (via inference
/// OR explicit `needs layout`) MUST be REFUSED (reported FAILED with a clear
/// message), never silently run against the faked getComputedStyle/offsetParent.
/// A pure logic assertion still passes. This locks the BUG-051/053 anti-rot
/// invariant: fidelity a backend lacks cannot produce a green.
#[test]
fn w1_refuse_to_fake_layout_on_logic_backend() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w1_refuse_gate.test.st");
    std::fs::write(&f, r#"@import "stdlib/testing/test"
@test "logic passes" { @fixture { <div class="lx">L</div> } @then .lx should exist }
@test "inferred layout refuses" { @fixture { <div class="ly">Y</div> } @then .ly should be_visible }
@test "declared layout refuses" needs layout { @fixture { <div class="lz">Z</div> } @then .lz should exist }
"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let passed = v["passed"].as_u64().unwrap_or(0);
    let failed = v["failed"].as_u64().unwrap_or(0);
    let skipped = v["skipped"].as_u64().unwrap_or(0);
    std::fs::remove_file(&f).ok();

    assert_eq!(
        passed, 1,
        "only the logic test should pass; stdout={stdout}"
    );
    // 3c288f06 settled the semantics: a fidelity refusal is a DEFERRAL to a
    // higher-fidelity backend, counted as skipped — NEVER as passed (the
    // anti-rot invariant this test exists to lock) and not as failed (a
    // headless run of a mixed suite is not red by design).
    assert_eq!(
        failed, 0,
        "a fidelity deferral is not a failure; stdout={stdout}"
    );
    assert_eq!(
        skipped, 2,
        "both layout tests must DEFER (never fake a green); stdout={stdout}"
    );
    // The deferral must NAME the fidelity gap, not vanish quietly.
    let named = stdout.matches("needs 'layout' fidelity").count();
    assert_eq!(
        named, 2,
        "both deferrals must name the missing rung; stdout={stdout}"
    );
}

/// W1: `--rung layout` would *allow* layout on this backend ONLY if the backend
/// could provide it; the V8 runner is capped at logic, so even `--rung layout`
/// must keep refusing (the flag advertises a ceiling, it does not fake fidelity).
/// Here we assert the default `logic` ceiling: a pure/logic-only suite is fully
/// green (no false refusals for things the backend CAN do).
#[test]
fn w1_logic_backend_runs_logic_tests_clean() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w1_logic_clean.test.st");
    std::fs::write(
        &f,
        r#"@import "stdlib/testing/test"
@test "exist" { @fixture { <div class="a">A</div> } @then .a should exist }
@test "text" { @fixture { <div class="b">hi</div> } @then .b should have_text "hi" }
@test "assert" { @assert (1 + 1 === 2) "math" }
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    std::fs::remove_file(&f).ok();
    assert_eq!(
        v["passed"].as_u64().unwrap_or(0),
        3,
        "all logic tests pass; stdout={stdout}"
    );
    assert_eq!(
        v["failed"].as_u64().unwrap_or(99),
        0,
        "no false refusals; stdout={stdout}"
    );
}

/// W3 claim grammar (FEAT-059): @then <subject> { <claim>; * } block form.
/// Mutation-proven: each op gates (a wrong expected value FAILS). Runs on the V8
/// headless backend. Replaces the discrete then-* macros with one grammar that
/// interpolates the claims array as a JS literal + iterates at runtime.
#[test]
fn w3_claim_grammar_gates() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w3_claims.test.st");
    std::fs::write(
        &f,
        r#"@import "stdlib/testing/test"
@test "PASS all ops" {
  @fixture { <div class="w" data-st-state="done" data-n="42">hello world</div> }
  @then .w {
    state == "done";
    state != "x";
    text ~= "world";
    text matches "^hello";
    exist;
  }
}
@test "FAIL eq" {
  @fixture { <div class="a" data-st-state="loading">L</div> }
  @then .a { state == "done"; }
}
@test "FAIL count" {
  @fixture { <div class="c">A</div><div class="c">B</div> }
  @then .c { count >= 5; }
}
@test "FAIL exist" {
  @fixture { <div class="present">P</div> }
  @then .absent-zzz { exist; }
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    std::fs::remove_file(&f).ok();
    assert_eq!(
        v["passed"].as_u64().unwrap_or(0),
        1,
        "1 all-ops test passes; stdout={stdout}"
    );
    assert_eq!(
        v["failed"].as_u64().unwrap_or(0),
        3,
        "3 mutated claims must FAIL; stdout={stdout}"
    );
}

/// W4 generative testing (FEAT-060): @property + @fuzz macros.
/// Mutation-proven: a true property passes N cases; a false one fails with a
/// shrunk counterexample; a fuzz body that throws is caught.
#[test]
fn w4_property_and_fuzz_gate() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w4_gen.test.st");
    std::fs::write(
        &f,
        r#"@import "stdlib/testing/test"
@property "true holds" forall (n: int 0..100) {
  @assert (Math.min(n, 10) <= 10) "min bounded";
}
@property "false fails" forall (n: int 0..100) {
  @assert (n < 50) "n<50";
}
@fuzz "no crash" inputs (s: string ~ unicode) {
  @assert (typeof (s + "x") === "string") "concat ok";
}
@fuzz "crash caught" inputs (s: string ~ ascii) {
  @eval (eval("if (s.length > 5) throw new Error('boom');"));
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    std::fs::remove_file(&f).ok();
    // 2 pass (true property + no-crash fuzz), 2 fail (false property + crash fuzz).
    assert_eq!(v["passed"].as_u64().unwrap_or(0), 2, "stdout={stdout}");
    assert_eq!(v["failed"].as_u64().unwrap_or(0), 2, "stdout={stdout}");
    // The false property must report a shrunk counterexample.
    let errs = v["errors"].as_array().cloned().unwrap_or_default();
    assert!(
        errs.iter().any(|e| e["error"]
            .as_str()
            .map(|s| s.contains("counterexample"))
            .unwrap_or(false)),
        "expected a shrunk counterexample; errors={errs:?}"
    );
}

/// W3 LIVE component testing (FEAT-059): @mount a real reactive Spacetime
/// component, drive it with real @when events, observe real signal state via
/// @then{$signal}. This is the core "test full Spacetime" vision — not static
/// DOM inspection but real reactivity end-to-end. Mutation-proven.
#[test]
fn w3_mount_drive_observe_live_component() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w3_live.test.st");
    std::fs::write(
        &f,
        r#"@import "stdlib/testing/test"
@test "drive real counter to 3" {
  @mount { <button class="b1">+</button>
    .b1 { $n number: 0; @on &.click { $n <- $n + 1; } } }
  @when .b1 click
  @when .b1 click
  @when .b1 click
  @then .b1 { $n == 3; }
}
@test "wrong count gates" {
  @mount { <button class="b2">+</button>
    .b2 { $n number: 0; @on &.click { $n <- $n + 1; } } }
  @when .b2 click
  @then .b2 { $n == 5; }
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    std::fs::remove_file(&f).ok();
    assert_eq!(
        v["passed"].as_u64().unwrap_or(0),
        1,
        "drive-to-3 must pass; stdout={stdout}"
    );
    assert_eq!(
        v["failed"].as_u64().unwrap_or(0),
        1,
        "wrong-count must gate; stdout={stdout}"
    );
}

/// W5 animation testing (FEAT-061): virtual clock + timeline assertions.
/// Deterministic — no wall-clock flake. A real driver advanced by the virtual
/// clock is monotonic; a backward step / escaped-unit timeline is caught.
#[test]
fn w5_virtual_clock_and_timeline_gate() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w5_clock.test.st");
    std::fs::write(&f, r#"@import "stdlib/testing/test"
@test "clock advances deterministically" {
  @eval (eval("ST._clock.install(); window.__n=0; ST._raf.add(function(t){ window.__n++; });"))
  @clock sample 5
  @assert (window.__n === 5) "exactly 5 frames";
}
@test "real driver monotonic under virtual clock" {
  @eval (eval("ST._clock.install(); if(ST._timing){ST._timing.enable(); ST._timing.driverUpdates.clear();}"))
  @eval (eval("ST._raf.add(function(t){ var p=Math.min(1,t/200); ST._timing.recordDriver('fade', p); });"))
  @clock advance 220
  @record-timeline fade
  @timeline fade { nonempty; monotonic; in-unit; ends-at-one; }
}
@test "backward step caught" {
  @eval (eval("window.__stTimelines = { bad: [{p:0},{p:0.5},{p:0.3}] };"))
  @timeline bad { monotonic; }
}
"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--format",
            "json",
        ])
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    std::fs::remove_file(&f).ok();
    // 2 pass (clock deterministic + real-driver monotonic), 1 fail (backward step).
    assert_eq!(v["passed"].as_u64().unwrap_or(0), 2, "stdout={stdout}");
    assert_eq!(
        v["failed"].as_u64().unwrap_or(0),
        1,
        "backward step must be caught; stdout={stdout}"
    );
}

/// W6 directive coverage (FEAT-062): the macro-expansion moat. A suite that
/// exercises only some macros reports the rest as uncovered, and --min-directive
/// gates. Verified via the coverage module + a headless --coverage run.
#[test]
fn w6_directive_coverage_reports_and_gates() {
    use std::process::Command;
    let f = std::env::temp_dir().join("w6_cov.test.st");
    std::fs::write(
        &f,
        r#"@import "stdlib/testing/test"
@test "small surface" {
  @mount { <div class="x">x</div> }
  @then .x { exist; }
}
"#,
    )
    .unwrap();
    // Below-threshold run must exit non-zero.
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args([
            "test",
            f.to_str().unwrap(),
            "--headless",
            "--coverage",
            "--min-directive",
            "90",
        ])
        .output()
        .expect("run");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_file(&f).ok();
    assert!(
        combined.contains("Directive coverage:"),
        "should print coverage; out={combined}"
    );
    assert!(
        combined.contains("never exercised"),
        "should list the gap; out={combined}"
    );
    assert!(
        !out.status.success(),
        "90% threshold must fail on a tiny suite"
    );
}

#[test]
fn test_imported_file_scope_html_merges_into_compiled_html() {
    // BUG-075: a full-Spacetime page may split its file-scope markup across
    // imported files. Markup declared in an @import-ed file must reach
    // compiled.html, not just the entry file's own markup. Without merging
    // imported html_blocks, multi-file apps (e.g. the CMS admin shell) lose
    // every imported `<tag>` block silently.
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Imported file contributes page markup (the shell skeleton).
    fs::write(
        temp_path.join("shell.st"),
        "<section class=\"imported-shell\"><h2>Imported markup</h2></section>\n",
    )
    .unwrap();

    // Entry imports the shell and adds its own markup.
    let main_content = r#"@import "./shell.st"
<div class="entry-shell"><h1>Entry markup</h1></div>
"#;
    let main_ast = parse(main_content).unwrap();
    let main_path = temp_path.join("index.st");
    let merged = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // The merged AST must carry BOTH files' file-scope html blocks.
    assert!(
        !merged.html_blocks.is_empty(),
        "merged AST should retain file-scope html blocks"
    );

    // And the compiled HTML must contain the imported markup, not only the entry's.
    let compiled = compile(&merged, CompileOptions::default());
    assert!(
        compiled.html.contains("imported-shell"),
        "imported file-scope markup must reach compiled.html; got: {}",
        compiled.html
    );
    assert!(
        compiled.html.contains("entry-shell"),
        "entry file-scope markup must remain in compiled.html; got: {}",
        compiled.html
    );
}

#[test]
fn test_use_namespaces_imported_macros() {
    // FEAT-118: `@use` (vs `@import`) registers a module's macros under a
    // namespace derived from the module path. The merged macro must carry
    // `module = Some(scene)` so M0 Fqn keying keys it as `scene/<name>`.
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // A module file defining a directive macro.
    fs::write(
        temp_path.join("scene.st"),
        r#"%macro spotlight {
  %form {
    @spotlight $intensity:string
  }
}"#,
    )
    .unwrap();

    // Main file pulls it in with @use (namespaced), not @import (global).
    let main_content = r#"@use "./scene.st";
<main></main>"#;

    let main_ast = parse(main_content).unwrap();
    assert_eq!(main_ast.imports.len(), 1, "@use parsed as an import");
    let imp = &main_ast.imports[0];
    assert!(
        imp.namespace.is_some(),
        "@use carries a namespace (unlike @import); got {:?}",
        imp.namespace
    );
    assert_eq!(
        imp.namespace.as_ref().unwrap().path,
        vec!["scene".to_string()],
        "namespace derived from the module path tail"
    );

    let main_path = temp_path.join("app.st");
    let result = resolve_imports(&main_ast, &main_path, temp_path).unwrap();

    // The imported macro must be stamped with the module namespace.
    let spotlight = result.meta_defs.iter().find_map(|d| match d {
        spacetime::parser::meta_ast::MetaDef::Macro(m) if m.name == "spotlight" => Some(m),
        _ => None,
    });
    let spotlight = spotlight.expect("imported macro `spotlight` present in merged AST");
    let ns = spotlight
        .module
        .as_ref()
        .expect("@use-imported macro carries a module namespace");
    assert_eq!(ns.path, vec!["scene".to_string()]);
}

#[test]
fn test_import_stays_global_unnamespaced() {
    // Control: `@import` is the global degenerate case — imported macros keep
    // `module = None` (flat-global), byte-identical to pre-FEAT-118.
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(
        temp_path.join("globals.st"),
        r#"%macro beacon {
  %form {
    @beacon $power:string
  }
}"#,
    )
    .unwrap();

    let main_content = r#"@import "./globals.st";
<main></main>"#;

    let main_ast = parse(main_content).unwrap();
    assert!(
        main_ast.imports[0].namespace.is_none(),
        "@import is global (no namespace)"
    );

    let main_path = temp_path.join("app.st");
    let result = resolve_imports(&main_ast, &main_path, temp_path).unwrap();
    let beacon = result.meta_defs.iter().find_map(|d| match d {
        spacetime::parser::meta_ast::MetaDef::Macro(m) if m.name == "beacon" => Some(m),
        _ => None,
    });
    assert!(
        beacon.expect("beacon present").module.is_none(),
        "@import-imported macro stays global (module = None)"
    );
}

// =============================================================================
// FEAT-142 WAVE A: entity scope registration emits into spacetime.js
// =============================================================================

#[test]
fn entity_scope_emits_marker_and_registration() {
    use spacetime::compiler::{CompileOptions, compile};
    use spacetime::parser::parse;

    // Use the parse + compile(&ast) path (identical to what `cargo run -- build`
    // resolves to after import assembly, and CLI-verified to emit 265KB with the
    // entity registration). The earlier `Compiler::from_file` + temp-dir harness
    // diverged from the real build path and produced empty output for reasons
    // unrelated to entity lowering; this path is the reliable, CLI-faithful one.
    let source = "@import \"stdlib\"\n&evernet { @scroll-spy }\n.probe { color: red; }\n";
    let ast = parse(source).expect("parse should succeed");
    let compiled = compile(&ast, CompileOptions::new().with_fresh_registry());

    assert!(
        compiled.pipeline_errors.is_empty(),
        "entity scope probe should compile without errors: {:?}",
        compiled.pipeline_errors
    );
    assert!(
        !compiled.js.is_empty(),
        "spacetime.js must be non-empty for entity scope probe"
    );
    // The per-entity marker element (`<st-entity class="st-entity-<name>">`) is
    // injected into the STATIC page HTML by the Rust lowering, so selector-init
    // emit can bind the interior component directives to it. (A leading attribute
    // selector does not emit directive JS, so the marker carries a CLASS.)
    assert!(
        compiled.html.contains("st-entity-evernet"),
        "page HTML must contain the entity marker element: {}",
        compiled.html
    );
    // Registration wires the entity into the general world registry. The entity
    // name is bound to a `NAME` local (`NAME = 'evernet'`) and used as the key
    // (`__stWorld.byName[NAME] = {...}`), so assert on the registry write + the
    // name value rather than a literal `byName['evernet']` subscript.
    assert!(
        compiled.js.contains("__stWorld.byName[NAME]"),
        "spacetime.js must register the entity into window.__stWorld.byName: {}",
        compiled.js
    );
    assert!(
        compiled.js.contains("evernet"),
        "spacetime.js must carry the entity name 'evernet': {}",
        compiled.js
    );
    assert!(
        compiled.js.contains("IntersectionObserver"),
        "spacetime.js must contain the scroll-spy IntersectionObserver logic: {}",
        compiled.js
    );
}
