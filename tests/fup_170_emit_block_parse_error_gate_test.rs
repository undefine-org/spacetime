//! FEAT-170 / BUG-256 — a primitive whose `%emit js` block fails to PARSE must
//! FAIL the build, never silently emit nothing.
//!
//! # The bug
//!
//! `%primitive` bodies are compiled in `metasystem_codegen`: an `%emit` block
//! that fails to parse records an ERROR-severity E0800 diagnostic on the
//! generated IR and contributes ZERO statements. The expand layer then DROPPED
//! `ir.diagnostics` (a `trace!` only), so the primitive emitted nothing and
//! every downstream layer reported success: the macro registered, the bind
//! resolved, `check` printed green. A page shipped with the primitive silently
//! absent — the banned silent-drop class living inside the compiler's own
//! plumbing (BUG-256's whole session was spent chasing the wrong layer because
//! every instrumented layer said OK).
//!
//! # The fix
//!
//! `expand` promotes a JS-EMIT PARSE ERROR diagnostic out of the IR into a
//! `CompileError` — the exact error channel every other directive parse error
//! uses — which `pipeline::compile` turns into a counted E0812 error. Both
//! `check` and `build` (single-file and directory/export) now fail.
//!
//! # What is asserted here
//!
//! The positive AND the negative. A regression test that only asserts the good
//! case would have passed throughout the entire lifetime of this bug.

use std::process::Command;

/// Compile a source string through the real `check` path and report whether it
/// was accepted, plus the combined output.
fn check_source(source: &str) -> (bool, String) {
    run_cli(source, &["check"])
}

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!(
        "fup170-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacetime"));
    cmd.arg(args[0]).arg(&file).args(&args[1..]);
    let out = cmd.output().expect("run spacetime CLI");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), combined)
}

/// A `%primitive` whose `%emit js` block is bound to a user macro and
/// instantiated on a real page (with markup, so it is a page and the pipeline
/// actually expands it — a markup-less `.st` is not a page and `check` reports
/// green on it regardless, the BUG-256 methodology trap).
fn broken_emit_source(js_line: &str) -> String {
    format!(
        r#"@version 2026-06-09;

%primitive broken-emit() {{
  %emit js {{
    {js_line}
  }}
}}

%macro spike {{
  %form {{
    @spike
  }}
  %binds {{
    broken-emit()
  }}
}}

<div class="stage"><div class="title">T</div></div>

.stage {{
  @spike
}}
"#
    )
}

/// RED gate: a malformed emit block must make `check` FAIL, and the diagnostic
/// must name the primitive and the parse reason.
#[test]
fn malformed_emit_block_fails_check_naming_primitive_and_reason() {
    // `var x = ;` is a JS syntax error the emit parser rejects (TS1109).
    let source = broken_emit_source("var x = ;");
    let (ok, output) = check_source(&source);

    assert!(
        !ok,
        "FEAT-170: a broken emit block must FAIL check, but it exited 0.\n\
         A green exit here is exactly the BUG-256 silent drop — the build \
         reported success while the primitive emitted nothing.\nOutput:\n{output}"
    );
    assert!(
        output.contains("broken-emit"),
        "FEAT-170: the failure must name the offending primitive, got:\n{output}"
    );
    assert!(
        output.contains("JS emit parse error"),
        "FEAT-170: the failure must carry the parse reason, got:\n{output}"
    );
    assert!(
        output.contains("error(s),"),
        "FEAT-170: the summary must COUNT the error (never '0 error(s)' with a \
         green check), got:\n{output}"
    );
}

/// NEGATIVE gate: the process/result status must be nonzero — asserting only on
/// stderr text is weak (the dropped diagnostic could still be printed while the
/// process succeeds). This asserts the actual gate, not its rendering.
#[test]
fn dropped_emit_block_never_accompanies_success_status() {
    let source = broken_emit_source("var x = ;");
    let (ok, output) = run_cli(&source, &["build"]);
    assert!(
        !ok,
        "FEAT-170: a broken emit block must fail `build`, but it exited 0.\n\
         The emitted bundle was therefore missing the primitive while the build \
         reported success — the banned class.\nOutput:\n{output}"
    );
    assert!(
        output.contains("broken-emit") && output.contains("JS emit parse error"),
        "FEAT-170: `build` must name the primitive and the parse reason, got:\n{output}"
    );
}

/// Positive gate: a WELL-FORMED emit block must still compile, AND must actually
/// EMIT — a positive-only suite that rejects everything would pass, so exit-0
/// alone is not enough. Build a site directory and prove the emitted statement
/// reaches the shipped bundle.
#[test]
fn wellformed_emit_block_still_compiles_and_emits() {
    let source = broken_emit_source("var x = 42;");
    let (ok, output) = check_source(&source);
    assert!(
        ok,
        "FEAT-170: a WELL-FORMED emit block must still compile. Output:\n{output}"
    );
    assert!(
        !output.contains("E0812"),
        "FEAT-170: a well-formed emit block must NOT be rejected. Output:\n{output}"
    );

    // Exiting 0 is not enough: a silently-dropped directive also exits 0. Build
    // a SITE DIRECTORY and read the emitted bundle — proving the statement is
    // really in the shipped JS, not merely "didn't error".
    let dir = std::env::temp_dir().join(format!(
        "fup170-good-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create site dir");
    std::fs::write(dir.join("index.st"), &source).expect("write source");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacetime"));
    cmd.arg("build").arg(&dir);
    let out = cmd.output().expect("run spacetime build");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "FEAT-170: well-formed site `build` must succeed. Output:\n{combined}"
    );
    let bundle = std::fs::read_to_string(dir.join("dist").join("spacetime.js"))
        .expect("site build must produce dist/spacetime.js");
    assert!(
        bundle.contains("var x = 42"),
        "FEAT-170: the well-formed emit block must actually EMIT its statement \
         into the bundle, not merely fail to error. Bundle:\n{bundle}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
