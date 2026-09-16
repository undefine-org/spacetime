//! PLAN-150 — THE FILM DOC GATE.
//!
//! Two duties, both about the golden reference `docs/language/film.st.md` and
//! its per-wave working docs under `docs/language/film/`:
//!
//!   1. LANDED BUILDS GREEN. Every `docs/language/film/wN-*.st.md` uses only
//!      syntax that has shipped up to its wave, so it MUST compile. A working
//!      doc that stops building is a regression in a landed wave.
//!
//!   2. UNLANDED FAILS LOUD. Each film word whose implementation is still ahead
//!      of us has a one-line canary here, asserted to fail with a SPECIFIC
//!      diagnostic code. This is the BUG-252 discipline applied to a spec: a
//!      feature that is not built must not compile to nothing on a green build.
//!      When a wave lands, its canary moves from `MUST_FAIL` into that wave's
//!      working doc as a green fence.
//!
//! Run: `cargo test --test film_doc_gate`. No `cdp`, no server — pure compile.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_spacetime")
}

/// Build a `.st` / `.st.md` source string in a fresh temp dir; return
/// (success, combined stdout+stderr).
fn build_source(src: &str, ext: &str) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!(
        "film-gate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join(format!("index.{ext}"));
    std::fs::write(&file, src).expect("write source");
    let out = Command::new(bin())
        .arg("check")
        .arg(&file)
        .output()
        .expect("run spacetime check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), text)
}

// ===========================================================================
// Duty 1: every landed working doc builds green.
// ===========================================================================

fn working_docs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/language/film");
    let mut docs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("docs/language/film exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".st.md"))
        .collect();
    docs.sort();
    docs
}

#[test]
fn every_working_doc_builds_green() {
    let docs = working_docs();
    assert!(
        !docs.is_empty(),
        "expected at least the W0 working doc under docs/language/film/"
    );
    for doc in docs {
        let src = std::fs::read_to_string(&doc).expect("read working doc");
        let (ok, log) = build_source(&src, "st.md");
        assert!(
            ok,
            "working doc {} must BUILD (it uses only landed syntax):\n{log}",
            doc.display()
        );
    }
}

// ===========================================================================
// Duty 2: every unlanded film word fails loud with its own code.
//
// (canary source, expected diagnostic code, the wave that turns it green)
// ===========================================================================

const MUST_FAIL: &[(&str, &str, &str)] = &[
    // W1 (positioned stops, per-step easing, --steps), W2 (@post DOM lens),
    // W3 (camera form kind) and W5 (@scatter) have LANDED — their canaries are
    // green fences in the wN working docs, enforced by
    // `every_working_doc_builds_green`.
];

#[test]
fn every_unlanded_word_fails_loud() {
    for (src, code, wave) in MUST_FAIL {
        let (ok, log) = build_source(src, "st");
        assert!(
            !ok,
            "[{wave}] this canary must FAIL until its wave lands, but it built green:\n{src}\n{log}"
        );
        assert!(
            log.contains(code),
            "[{wave}] expected diagnostic {code}; got:\n{log}"
        );
    }
}

/// The reserved-directive path specifically: `@scatter` must name its wave, not
/// fall through to W0714 "unknown directive, ignored" (which is a warning — a
/// green build). This pins the W0 mechanism itself.
#[test]
fn reserved_film_directive_is_an_error_not_a_warning() {
    // Every film directive has LANDED. The one remaining loud-error case is a
    // misplaced one: `@audio` is a SCORE ENTRY (W7), meaningless as a standalone
    // directive. Written outside a `@score` body it must be a HARD error that
    // steers the author to the score — never the generic W0714 "unknown
    // directive, ignored" warning (the silent-drop class this gate closes).
    let (ok, log) = build_source(".b { @audio(src: \"a.wav\") as &bed at 0s; }", "st");
    assert!(!ok, "@audio outside a score must be a hard error, not a warning:\n{log}");
    assert!(
        log.contains("E0970") && log.contains("score"),
        "the error must steer the author to the score:\n{log}"
    );
    assert!(
        !log.contains("W0714"),
        "@audio must NOT fall through to the generic unknown-directive warning:\n{log}"
    );
}

// ===========================================================================
// Duty 3: known-silent gaps are ACKNOWLEDGED. These still compile green today
// (value-level words the later wave enforces). The test asserts they are STILL
// green — so when a wave makes one loud, this test fails and forces the canary
// to move to MUST_FAIL. It is a tripwire against silent scope drift, not an
// endorsement.
// ===========================================================================

const KNOWN_SILENT: &[(&str, &str)] = &[
    // Both former silent gaps have LANDED and are behaviour-gated:
    //   W6 `random()` (056b9cc2) → tests/score/random.test.st + noise.test.st
    //   W10 `@reveal(split: outline)` (this wave) → tests/score/outline.test.st
    // Each still builds green (it always did), so it is no longer a *silent
    // gap* — it is a working feature fenced by its working doc. The list is now
    // empty; if a future silent-accept gap appears, add its canary here.
];

#[allow(dead_code)]
const KNOWN_SILENT_EMPTY_OK: () = ();

#[test]
fn known_silent_gaps_are_still_silent() {
    for (src, wave) in KNOWN_SILENT {
        let (ok, _log) = build_source(src, "st");
        assert!(
            ok,
            "[{wave}] this gap is documented as still-silent in docs/language/film/w0-doc-gate.st.md; \
             it now FAILS to build, which means its wave landed — move it from KNOWN_SILENT to MUST_FAIL \
             (with its diagnostic code) and add a green fence to the wave's working doc."
        );
    }
}
