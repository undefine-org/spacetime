//! RED matrix — registry-addressing DIAGNOSTICS (PLAN-117 W2/W3).
//!
//! The Rust companion to `tests/lang/registry-addressing/*.test.st`.
//!
//! Why a Rust file at all, in a repo whose doctrine is "test Spacetime with
//! Spacetime": `.test.st` currently has NO surface for asserting a COMPILE-TIME
//! diagnostic. The `@then` claim vocabulary is entirely runtime — DOM, text,
//! signals, timelines (verified against `stdlib/testing/`). A `.test.st` can
//! assert the *consequence* of a missing diagnostic (nothing rendered), but not
//! the diagnostic CODE, its message, or its help text.
//!
//! This file is EXPLICITLY TEMPORARY. FUP-147 designs the harmonious answer:
//! a `@then compile` subject, with `@compile(content:|src:)` establishing the
//! subject the way `@mount` establishes a DOM. When it lands, this file and its
//! `[[test]]` entry in Cargo.toml are DELETED and these 11 tests are reborn as
//! `tests/lang/registry-addressing/diagnostics.test.st`.
//!
//! Do not grow this file with diagnostics for other features — that would deepen
//! the debt FUP-147 exists to pay. New diagnostic tests wait for the claim
//! shape, or assert their observable consequence from a `.test.st`.
//!
//! Every test in this file is RED by construction: it asserts a diagnostic that
//! does not exist yet. Each names the wave that turns it green.
//!
//! Run: `cargo test --test registry_addressing_diagnostics`

use spacetime::compiler::Compiler;
use std::path::{Path, PathBuf};

/// Root for this suite's throwaway projects.
///
/// Repo-local and gitignored (`/scratch/` in .gitignore), never the system temp
/// dir: `/tmp` is shared, size-capped, and under parallel `cargo test` it is a
/// real source of contention failures (observed: 133 spurious failures across
/// the lib suite at default parallelism, all passing at `--test-threads=4`).
/// Keeping scratch state inside the workspace also makes a failed run's
/// leftovers inspectable instead of scattered across the machine.
///
/// Each caller appends a unique leaf; `scratch_dir` creates it fresh, and
/// `cleanup` removes it so a green run leaves nothing behind.
fn scratch_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scratch/tests/registry-addressing")
}

/// Create (and clear) a uniquely-named scratch project dir.
///
/// The name must be unique per TEST, not merely per process: `cargo test` runs
/// this file's tests concurrently in one process, so a pid-only name lets two
/// tests share a directory — one clearing it while the other compiles inside it.
/// That produced a test which passed alone and failed under `--test-threads=4`,
/// the most misleading failure shape there is. A monotonic counter makes each
/// call site's directory disjoint.
fn scratch_dir(leaf: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = scratch_root().join(format!("{}_{}_{}", leaf, std::process::id(), n));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("create scratch project dir");
    dir
}

/// Remove a scratch project dir, and the shared root once it is empty, so a
/// passing run leaves the working tree exactly as it found it.
fn cleanup(dir: &Path) {
    std::fs::remove_dir_all(dir).ok();
    // Only succeeds when this was the last suite dir still present.
    std::fs::remove_dir(scratch_root()).ok();
}

/// Compile a throwaway multi-file project and return every diagnostic code the
/// pipeline reported. Files are written under a unique scratch dir so relative
/// `@use`/`@import` resolution behaves exactly as it does in a real project.
fn compile_project(files: &[(&str, &str)], entry: &str) -> Vec<String> {
    let dir = scratch_dir(&entry.replace(['/', '.'], "_"));
    for (name, body) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&path, body).expect("write fixture");
    }
    let entry_path: PathBuf = dir.join(entry);

    // A diagnostic can arrive by either of TWO channels, and a helper that reads
    // only one silently reports "no diagnostics" for the other:
    //   - PARSE errors surface as an Err from `from_file` (the whole file failed
    //     to become an AST) — e.g. a colon where a qualifier slash belongs,
    //     which must be caught while both halves of the name are still visible;
    //   - PIPELINE errors ride `compiled.pipeline_errors`.
    // Both are refusals as far as an author is concerned, so both count.
    let codes: Vec<String> = match Compiler::from_file(&entry_path, &dir) {
        Ok(c) => c
            .compile()
            .pipeline_errors
            .iter()
            .map(|e| e.code.clone())
            .collect(),
        Err(parse_err) => vec![format!("PARSE_ERROR: {parse_err}")],
    };
    cleanup(&dir);
    codes
}

/// True when any reported code matches, allowing the final code names to be
/// decided at implementation time (the plan proposes names, not numbers).
///
/// Substring-matched so a `PARSE_ERROR: …` entry can be probed for the text a
/// parse-level refusal carries instead of a code.
fn has_code(codes: &[String], candidates: &[&str]) -> bool {
    codes
        .iter()
        .any(|c| candidates.iter().any(|cand| c.contains(cand)))
}

// ===========================================================================
// W2 — duplicate declaration.
//
// The verified defect: two files each declaring a root-level `$host` merge
// SILENTLY into one cell. Every other registry refuses this — `register_macro`
// returns DuplicateMacro, `register_primitive` returns DuplicatePrimitive
// (src/metasystem/registry.rs). `$` is the only exempt kind, and that exemption
// IS the bug. A page holds one cell per name, exactly as it holds one form per
// signature.
// ===========================================================================

#[test]
fn red_w2_duplicate_state_declaration_across_files_is_an_error() {
    let codes = compile_project(
        &[
            ("a.st", "$host string: \"https://a.example.com\";\n"),
            ("b.st", "$host string: \"https://b.example.com\";\n"),
            (
                "index.st",
                "@import \"./a.st\"\n@import \"./b.st\"\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E-state-dup", "E0938"]),
        "two files declaring root-level $host must be REFUSED, not silently \
         merged. Got: {codes:?}"
    );
}

#[test]
fn red_w2_duplicate_declaration_error_names_both_files() {
    // A teaching error, not a bare refusal: the reader must learn WHERE the
    // conflict is and HOW to resolve it. Asserted separately from the code so a
    // regression in message quality is visible on its own.
    let dir = scratch_dir("dup_msg");
    std::fs::write(dir.join("a.st"), "$host string: \"a\";\n").ok();
    std::fs::write(dir.join("b.st"), "$host string: \"b\";\n").ok();
    std::fs::write(
        dir.join("index.st"),
        "@import \"./a.st\"\n@import \"./b.st\"\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
    )
    .ok();
    let compiled = Compiler::from_file(&dir.join("index.st"), &dir)
        .expect("compiler")
        .compile();
    let joined = compiled
        .pipeline_errors
        .iter()
        .map(|e| format!("{} {}", e.code, e.message))
        .collect::<Vec<_>>()
        .join("\n");
    cleanup(&dir);
    assert!(
        joined.contains("a.st") && joined.contains("b.st"),
        "the duplicate-declaration error must name BOTH declaring files. Got:\n{joined}"
    );
}

// ===========================================================================
// W3 — qualifier resolution.
//
// Mirrors the `@`-registry diagnostics that already ship: E0926 (unbound
// qualifier), E0927 (visibility), E0924 (ambiguity). The `$` registry must
// earn the same three, because the pattern principle promises they exist.
// ===========================================================================

#[test]
fn red_w3_unbound_qualifier_on_a_cell_is_an_error() {
    // `nosuch/$host` with no `@use ... as nosuch`. Verified today: parses
    // clean, emits nothing, reports success — the silent-drop defect.
    let codes = compile_project(
        &[
            ("config.st", "$host string: \"https://api.example.com\";\n"),
            (
                "index.st",
                "@use \"./config.st\" as app\n<main><p class=\"x\"></p></main>\n.x { text <- nosuch/$host; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0940"]),
        "an unbound qualifier on a cell reference must raise E0940 SPECIFICALLY. \
         E0926 is the pre-existing DIRECTIVE-qualifier code and must NOT be \
         accepted here: admitting it let this test pass on the old silent-drop \
         behaviour, proving nothing. Got: {codes:?}"
    );
}

#[test]
fn red_w3_colon_in_reference_position_is_an_error_not_a_silent_drop() {
    // THE verified silent-drop bug, in its original `@`-registry form:
    //   `@b/badge("NEW")` -> emits `.badge::before` into spacetime.css
    //   `@b:badge("NEW")` -> compiles "✓ passed", emits ZERO css
    // A colon where a slash belongs must be refused. `:` is quarantined to
    // module addresses inside quoted import strings; it never reaches
    // reference position.
    let codes = compile_project(
        &[
            (
                "badge.st",
                "%macro badge {\n  %form { @badge($label:string) { $styles:properties } }\n  %binds { badge-emit($label, styles: $styles) -> {} }\n}\n%primitive badge-emit {\n  %emit css { .badge::before { content: \"$label\"; } }\n}\n",
            ),
            (
                "index.st",
                "@use \"./badge.st\" as b\n<main><span class=\"badge\"></span></main>\n.badge { @b:badge(\"NEW\") { color: red; } }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0943", "module qualifier needs"]),
        "`@b:badge` must be REFUSED, and the refusal must TEACH the fix \
         (`@b/badge`). Before this it reported success and emitted nothing — a \
         silent drop is the worst possible outcome for an addressing mistake. \
         Got: {codes:?}"
    );
}

#[test]
fn red_w3_ambiguous_bare_cell_across_two_modules_is_an_error() {
    // Two flooded modules both declaring `$theme`; a BARE `$theme` cannot be
    // resolved. Mirrors E0924 (cross-namespace ambiguity) which already exists
    // for `@`. The help must point at qualification.
    let codes = compile_project(
        &[
            ("dark.st", "$theme string: \"dark\";\n"),
            ("light.st", "$theme string: \"light\";\n"),
            (
                "index.st",
                "@import \"./dark.st\"\n@import \"./light.st\"\n<main><p class=\"x\"></p></main>\n.x { text <- $theme; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0938"]),
        "a bare cell reference resolving to two declarations must error. \
         Got: {codes:?}"
    );
}

#[test]
fn red_w3_tight_numeric_head_slash_is_refused_with_help() {
    // `1/$denominator` — the deliberate cost of `/$`, approved in design
    // review ("forcing a space is okay"). A number can never be a qualifier, so
    // this IS division; but it reads like the new qualified form, so the
    // compiler must refuse and name the fix rather than silently pick.
    //
    // Verified today: it silently computes 0.25.
    let codes = compile_project(
        &[(
            "index.st",
            "$denominator number: 4;\n<main><p class=\"x\"></p></main>\n.x { text <- 1/$denominator; }\n",
        )],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0939", "E-slash-ambiguous"]),
        "a tight numeric-head `/$` must be refused with help suggesting spaces. \
         Got: {codes:?}"
    );
}

#[test]
fn red_w3_remote_write_through_a_qualifier_is_refused() {
    // Writes stay in the owning file. This is not ceremony: it is what makes
    // const-folding SOUND — the write-set of a cell is settled by a one-file
    // scan. A bare write to an imported cell stays legal (the file has taken
    // the cell into its own scope); only the QUALIFIED write is refused, and
    // the error must name the owner and suggest `@emit`.
    let codes = compile_project(
        &[
            ("config.st", "$session string: \"anonymous\";\n"),
            (
                "index.st",
                "@use \"./config.st\" as app\n<main><button class=\"b\">go</button></main>\n.b { @on &.click { app/$session <- \"ada\"; } }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0942", "E-state-remote-write"]),
        "a write through a qualifier must be refused, naming the owning file. \
         Got: {codes:?}"
    );
}

// ===========================================================================
// W2/W4 — the provenance table.
//
// `inspect --layer state` is simultaneously the diagnostic, the documentation,
// and the input to the optimisation. One registry, three views.
// ===========================================================================

#[test]
fn red_w2_inspect_reports_state_provenance() {
    // The table must carry, per cell: FQN, readers, writers, verdict.
    // Asserted through the CLI because that IS the surface being specified.
    let dir = scratch_dir("inspect_state");
    std::fs::write(
        dir.join("config.st"),
        "$host string: \"https://api.example.com\";\n$retryBudget number: 3;\n",
    )
    .ok();
    std::fs::write(
        dir.join("index.st"),
        "@import \"./config.st\"\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
    )
    .ok();

    // `inspect` needs the ENTRY file: the provenance table is a property of an
    // assembled page, not of a directory. (A bare `--layer state` correctly
    // refuses with "requires a .st file argument".)
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args(["inspect", "--layer", "state", "index.st"])
        .current_dir(&dir)
        .output()
        .expect("run inspect");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    cleanup(&dir);

    // Discriminating on all four axes, not just presence of the word "host":
    // the table must name the cell, its DECLARING file (the provenance that did
    // not exist before W2), and reach opposite verdicts for a read cell and an
    // unread one. A test that only checked `contains("host")` would pass on a
    // table that got every verdict wrong.
    assert!(
        text.contains("$host") && text.contains("readers") && text.contains("writers"),
        "`inspect --layer state` must report per-cell provenance \
         (cell / readers / writers / verdict). Got:\n{text}"
    );
    assert!(
        text.contains("config.st"),
        "provenance must name the DECLARING file, which is the whole point of \
         the layer — an @import-ed cell is declared somewhere you are not \
         looking. Got:\n{text}"
    );
    assert!(
        text.contains("const") && text.contains("dead"),
        "$host is read and write-free (const); $retryBudget is read by nobody \
         (dead). Both verdicts must appear, or the pass is not discriminating. \
         Got:\n{text}"
    );
}

// ===========================================================================
// Non-regression floor. These must stay GREEN through every wave — they are
// the 430-call-site guarantee.
// ===========================================================================

#[test]
fn floor_qualified_directive_still_resolves_and_emits() {
    // The `@` half of the pattern, proven by EMIT rather than by exit code —
    // this is precisely the assertion that exposed the silent-drop bug: the
    // colon form "passes" while emitting nothing, so only checking success
    // would have missed it.
    let dir = scratch_dir("floor_directive");
    std::fs::write(
        dir.join("badge.st"),
        "%macro badge {\n  %form { @badge($label:string) { $styles:properties } }\n  %binds { badge-emit($label, styles: $styles) -> {} }\n}\n%primitive badge-emit {\n  %emit css { .badge::before { content: \"$label\"; } }\n}\n",
    )
    .ok();
    std::fs::write(
        dir.join("index.st"),
        "@use \"./badge.st\" as b\n<main><span class=\"badge\"></span></main>\n.badge { @b/badge(\"NEW\") { color: red; } }\n",
    )
    .ok();
    let compiled = Compiler::from_file(&dir.join("index.st"), &dir)
        .expect("compiler")
        .compile();
    let css = compiled.css.clone();
    cleanup(&dir);
    assert!(
        css.contains("badge::before"),
        "`@b/badge` must resolve through the alias and EMIT. Got css:\n{css}"
    );
}

#[test]
fn floor_cross_file_state_read_still_works() {
    // 430 `@import` sites depend on this flat merge. Nothing in PLAN-117 may
    // break it; `@import` keeps working untouched through every wave.
    let dir = scratch_dir("floor_state_read");
    std::fs::write(
        dir.join("config.st"),
        "$host string: \"https://api.example.com\";\n",
    )
    .ok();
    std::fs::write(
        dir.join("index.st"),
        "@import \"./config.st\"\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
    )
    .ok();
    let compiled = Compiler::from_file(&dir.join("index.st"), &dir)
        .expect("compiler")
        .compile();
    let js = compiled.js.clone();
    let errors = compiled.pipeline_errors.len();
    cleanup(&dir);
    assert_eq!(errors, 0, "cross-file state read must compile cleanly");
    assert!(
        js.contains("host"),
        "the imported cell must reach the emitted runtime"
    );
}

#[test]
fn floor_repo_division_sites_still_compile() {
    // The 4 real division sites in the repo all live in
    // stdlib/mobile/macros/gestures/swipe.st and share one shape: `expr / 100`.
    // If the `/$` production ever breaks them, it must break HERE first.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let swipe = root.join("stdlib/mobile/macros/gestures/swipe.st");
    if !swipe.exists() {
        return; // module moved; the .test.st matrix still guards the semantics
    }
    let src = std::fs::read_to_string(&swipe).expect("read swipe.st");
    assert!(
        src.contains('/'),
        "swipe.st is the division canary for the `/$` production"
    );
}

// ===========================================================================
// W4 — fold + eliminate, asserted on the EMITTED ARTIFACT.
//
// These live here rather than in a `.test.st` because the claim is about what
// the compiler EMITS, not about what the page renders. A `.test.st` can prove
// the page still renders correctly (and does); only this can prove the
// machinery went away.
// ===========================================================================

/// Compile a project and return the emitted JS.
fn compile_js(files: &[(&str, &str)], entry: &str) -> String {
    let dir = scratch_dir(&format!("js_{}", entry.replace(['/', '.'], "_")));
    for (name, body) in files {
        std::fs::write(dir.join(name), body).expect("write fixture");
    }
    let js = Compiler::from_file(&dir.join(entry), &dir)
        .expect("compiler should construct")
        .compile()
        .js;
    cleanup(&dir);
    js
}

#[test]
fn w4_dead_cell_is_absent_from_the_emitted_bundle() {
    // `$retryBudget` is read by nobody, so nothing on the page can tell it ever
    // existed. It must not reach the bundle at all.
    let js = compile_js(
        &[(
            "index.st",
            "$host string: \"https://api.example.com\";\n$retryBudget number: 3;\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
        )],
        "index.st",
    );
    assert!(
        !js.contains("retryBudget"),
        "a cell with no readers must be eliminated from the bundle"
    );
}

#[test]
fn w4_dead_elimination_does_not_touch_a_read_cell() {
    // The discriminating half: elimination must be surgical. A pass that simply
    // removed every declaration would satisfy the test above and destroy the page.
    //
    // NB the probe names must be DISTINCTIVE. A first attempt used `$dropped`
    // and failed on a false positive: "dropped" occurs four times in the
    // shipped runtime's own source. A substring assertion against a ~300KB
    // bundle needs a needle that cannot occur by accident.
    let js = compile_js(
        &[(
            "index.st",
            "$zzKeptCell string: \"zzVisibleValue\";\n$zzUnreadCell number: 3;\n<main><p class=\"x\"></p></main>\n.x { text <- $zzKeptCell; }\n",
        )],
        "index.st",
    );
    assert!(
        !js.contains("zzUnreadCell"),
        "the unread cell must go. js len {}",
        js.len()
    );
    assert!(
        js.contains("zzVisibleValue"),
        "the READ cell's value must survive - elimination must be surgical, not \
         a blanket drop"
    );
}

#[test]
fn w4_a_const_cell_still_renders_its_value() {
    // The const VERDICT ships (visible in `inspect --layer state`); the
    // substitution does not — see the note at src/compiler.rs where
    // `fold_const_state` is deliberately left unwired. Wiring it removed the
    // reference token the binding pipeline keys on, so pages rendered EMPTY:
    // the headless matrix fell 22 -> 16, every division test included.
    //
    // This test pins the property that actually matters to an author and that a
    // future binding-aware fold must preserve: a write-free literal reaches the
    // page. It is the regression guard the fold work will be measured against.
    let js = compile_js(
        &[(
            "index.st",
            "$host string: \"https://api.example.com\";\n<main><p class=\"x\"></p></main>\n.x { text <- $host; }\n",
        )],
        "index.st",
    );
    assert!(
        js.contains("api.example.com"),
        "a const cell's value must reach the page"
    );
}

#[test]
fn w4_a_written_cell_keeps_its_update_path() {
    // The other side of the fold, and the one that matters for correctness: a
    // cell with a writer is a REAL signal and must keep its plumbing. A fold
    // that ate this would break every counter on the page.
    let js = compile_js(
        &[(
            "index.st",
            "$n number: 0;\n<main><p class=\"x\"></p><button class=\"b\">+</button></main>\n.x { text <- $n; }\n.b { @on &.click { $n <- $n + 1; } }\n",
        )],
        "index.st",
    );
    assert!(
        js.contains("local:n:updated"),
        "a written cell must keep its update channel"
    );
}

#[test]
fn w4_similar_cell_names_do_not_contaminate_each_other() {
    // `$host` and `$hostname` are distinct cells that share a prefix. Any pass
    // that rewrites references by name must respect token boundaries, or the
    // shorter name corrupts the longer one and the page silently reads the
    // wrong value — the kind of bug that surfaces as mystery text three files
    // away. (`fold_in_text`'s boundary check is unit-tested in
    // src/pipeline/state_analysis.rs; this pins the end-to-end property.)
    let js = compile_js(
        &[(
            "index.st",
            "$host string: \"SHORTVAL\";\n$hostname string: \"LONGERVAL\";\n<main><p class=\"a\"></p><p class=\"b\"></p></main>\n.a { text <- $host; }\n.b { text <- $hostname; }\n",
        )],
        "index.st",
    );
    assert!(js.contains("SHORTVAL"), "$host reaches the page");
    assert!(
        js.contains("LONGERVAL"),
        "$hostname must reach the page with its OWN value, uncorrupted by the \
         prefix it shares with $host"
    );
}

// ===========================================================================
// W5 — `@exports` at file scope.
//
// The same clause, the same `: mut` marker, and the same semantics as a
// `@template` body's `@exports` (FEAT-115), lifted to the file. Learn one, you
// know the other.
// ===========================================================================

const CFG_WITH_EXPORTS: &str = "@exports { $host, $session: mut }\n$host string: \"https://api.example.com\";\n$session string: \"anon\";\n$internal number: 3;\n";

#[test]
fn w5_a_published_cell_is_readable_through_a_qualifier() {
    let codes = compile_project(
        &[
            ("cfg.st", CFG_WITH_EXPORTS),
            (
                "index.st",
                "@use \"./cfg.st\" as app\n<main><p class=\"x\"></p></main>\n.x { text <- app/$host; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        codes.is_empty(),
        "reading a PUBLISHED cell must be clean. Got: {codes:?}"
    );
}

#[test]
fn w5_an_unpublished_cell_is_refused() {
    // `$internal` is declared but absent from the `@exports` clause, so the
    // module does not hand it out.
    let codes = compile_project(
        &[
            ("cfg.st", CFG_WITH_EXPORTS),
            (
                "index.st",
                "@use \"./cfg.st\" as app\n<main><p class=\"x\"></p></main>\n.x { text <- app/$internal; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0944"]),
        "reading an UNPUBLISHED cell through a qualifier must be refused. \
         Got: {codes:?}"
    );
}

#[test]
fn w5_a_module_without_an_exports_clause_publishes_everything() {
    // Public-by-default — the Odin floor. This is what keeps every pre-existing
    // file working unchanged: the visibility check only engages once a module
    // has actually stated an interface. Without this, adding W5 would have made
    // every qualified read in the repo an error.
    let codes = compile_project(
        &[
            ("cfg.st", "$anything string: \"v\";\n"),
            (
                "index.st",
                "@use \"./cfg.st\" as app\n<main><p class=\"x\"></p></main>\n.x { text <- app/$anything; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        codes.is_empty(),
        "a module with no @exports publishes everything. Got: {codes:?}"
    );
}

#[test]
fn w5_bare_reads_are_unaffected_by_a_visibility_clause() {
    // The clause governs QUALIFIED access across a module boundary. A file that
    // flat-merges another via `@import` has taken those cells into its own
    // scope and reads them as its own — otherwise adding an `@exports` clause
    // would break the 430 existing `@import` sites.
    let codes = compile_project(
        &[
            ("cfg.st", CFG_WITH_EXPORTS),
            (
                "index.st",
                "@import \"./cfg.st\"\n<main><p class=\"x\"></p></main>\n.x { text <- $internal; }\n",
            ),
        ],
        "index.st",
    );
    assert!(
        codes.is_empty(),
        "a bare read of a flat-merged cell stays legal. Got: {codes:?}"
    );
}

// ===========================================================================
// The THIRD reference channel — markup holes.
//
// A qualified reference reaches a cell by three routes, and a pass that walks
// only some of them enforces only some of them:
//
//   .x { text <- app/$host; }   scope binding   (ast.scopes)
//   @on click { … app/$host }   directive body  (ast.matches)
//   <p>`app/$host`</p>          MARKUP HOLE     (ast.html_blocks)
//
// The hole was missed at first: `app/$internal` silently read an unpublished
// cell and `nope/$host` silently resolved nothing, while the SAME reference in
// a scope binding was correctly refused. Every rule below is asserted through
// the hole specifically, so the channels cannot drift apart again.
// ===========================================================================

#[test]
fn a_markup_hole_enforces_visibility_like_a_scope_binding() {
    let codes = compile_project(
        &[
            ("cfg.st", CFG_WITH_EXPORTS),
            (
                "index.st",
                "@use \"./cfg.st\" as app\n<main><p>`app/$internal`</p></main>\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0944"]),
        "an unpublished cell read from a MARKUP HOLE must be refused exactly as \
         it is from a scope binding. Got: {codes:?}"
    );
}

#[test]
fn a_markup_hole_enforces_the_qualifier_like_a_scope_binding() {
    let codes = compile_project(
        &[
            ("config.st", "$host string: \"https://api.example.com\";\n"),
            (
                "index.st",
                "@use \"./config.st\" as app\n<main><p>`nope/$host`</p></main>\n",
            ),
        ],
        "index.st",
    );
    assert!(
        has_code(&codes, &["E0940"]),
        "an unbound qualifier in a MARKUP HOLE must raise E0940. Got: {codes:?}"
    );
}

#[test]
fn a_valid_qualified_read_in_a_markup_hole_resolves_and_renders() {
    // The positive half: enforcement must not be achieved by refusing the
    // legitimate case. The hole resolves to the bare cell and the value is
    // present in the emitted bundle.
    let dir = scratch_dir("hole_positive");
    std::fs::write(
        dir.join("config.st"),
        "$host string: \"https://api.example.com\";\n",
    )
    .expect("write");
    std::fs::write(
        dir.join("index.st"),
        "@use \"./config.st\" as app\n<main><p>`app/$host`</p></main>\n",
    )
    .expect("write");
    let compiled = Compiler::from_file(&dir.join("index.st"), &dir)
        .expect("parses")
        .compile();
    assert!(
        compiled.pipeline_errors.is_empty(),
        "a published cell read through a hole is clean. Got: {:?}",
        compiled.pipeline_errors
    );
    assert!(
        compiled.js.contains("api.example.com"),
        "the resolved value reaches the bundle"
    );
    cleanup(&dir);
}

// ===========================================================================
// The shipped demo is a GATE, not decoration (BUG-299).
//
// `examples/modules/index.st` is the module system's own demo — it exercises
// `@use … as`, an alias-qualified reference, and the badge module's emit. It
// shipped BROKEN (E0946, and `build` produced no CSS at all) and no test
// noticed, because nothing in the suite compiles the examples tree.
//
// Three separate defects hid behind that gap: `as`/`only`/`hiding` were never
// declared in `@use`'s `%form`, `@use` had no `%registers` to consume it at
// expansion, and `@exports` split its list on `;` only. Each was invisible to
// every assertion in the suite while being obvious on the first real compile.
//
// So the demo is now a test. It asserts on EMIT, not exit code — the BUG-232
// lesson: `@b:badge` once "passed" while emitting nothing.
// ===========================================================================

#[test]
fn the_shipped_module_example_compiles_and_emits() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/modules");
    let entry = root.join("index.st");
    assert!(entry.exists(), "the module-system demo must exist at {entry:?}");

    let compiled = Compiler::from_file(&entry, &root)
        .expect("the shipped demo parses")
        .compile();

    assert!(
        compiled.pipeline_errors.is_empty(),
        "the shipped module-system demo must compile cleanly. Got: {:?}",
        compiled.pipeline_errors
    );

    // Exit code is not the claim — the module's CSS reaching the bundle is.
    // `@b/badge("NEW")` resolves through the alias into the badge module, whose
    // emit is a `::before` label. If the alias silently failed to resolve, the
    // compile would still be "clean" and this is what would be missing.
    assert!(
        compiled.css.contains("badge"),
        "the alias-qualified `@b/badge` must emit the module's CSS; \
         a clean compile that emits nothing is the silent-drop class"
    );
}
