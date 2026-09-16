//! File-level EDN ingress (PLAN-148 beyond MCP): `Compiler::from_file` — the
//! choke point serve/check/build all share — accepts an `.edn` page and
//! compiles it through the SAME pipeline as its `.st` twin, and the dev
//! server's proactive compile writes a machine-readable `build-status.json`.
//!
//! The MCP paths have eaten EDN since W5; this test pins the FILE path, which
//! is what makes "write page.edn into a served directory" a working API.

use spacetime::compiler::{CompileCache, Compiler};
use spacetime::server::DevCompileCache;

const ST_SRC: &str = "@data inline $count : 0;\n";
const EDN_SRC: &str = "(data-inline :name $count :value [:st/expr \"0\"])\n";

/// A `sel`-consumed source: before `to_st_file` dual-registered scope
/// members, the EDN page compiled but W0201 flagged `$items` as "defined but
/// never used", because analysis scans `ScopeBlock::matches` and the `each`
/// sat ONLY in the flat match list.
const EDN_EACH: &str = "{:st/forms [(data-inline :name $items :value [:st/expr \"[1, 2]\"])\n                             (sel \".log\" (each :source $items :item $it :key [:st/expr \"null\"] :invocations [:st/array]))]}\n";

fn compile_entry(dir: &std::path::Path, name: &str, src: &str) -> spacetime::compiler::CompiledSpacetime {
    let path = dir.join(name);
    std::fs::write(&path, src).expect("write entry");
    Compiler::from_file(&path, dir)
        .unwrap_or_else(|e| panic!("from_file({name}) failed: {e}"))
        .without_runtime()
        .compile()
}

#[test]
fn from_file_compiles_an_edn_entry_identically_to_st() {
    let dir = tempfile::tempdir().expect("tmpdir");

    let st_out = compile_entry(dir.path(), "index.st", ST_SRC);
    std::fs::remove_file(dir.path().join("index.st")).unwrap();
    let edn_out = compile_entry(dir.path(), "index.edn", EDN_SRC);

    assert_eq!(st_out.js, edn_out.js, "JS differs between .st and .edn file ingress");
    assert_eq!(st_out.css, edn_out.css, "CSS differs between .st and .edn file ingress");
    assert_eq!(st_out.html, edn_out.html, "HTML differs between .st and .edn file ingress");
}

#[test]
fn from_file_reports_edn_read_errors_with_the_file_name() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("index.edn");
    std::fs::write(&path, "(data-inline :name").unwrap();

    let err = match Compiler::from_file(&path, dir.path()) {
        Ok(_) => panic!("broken EDN must be refused"),
        Err(e) => e,
    };
    assert!(err.contains("index.edn"), "error must name the file: {err}");
}

#[test]
fn st_file_that_structurally_resembles_edn_stays_on_the_st_path() {
    // The zero-change guarantee, pinned: ingress is gated by EXTENSION, so a
    // `.st` file starting with `[` and mentioning `:st/` in a declaration must
    // compile as .st — content-detection would route it into the EDN reader
    // and break it.
    let dir = tempfile::tempdir().expect("tmpdir");
    let out = compile_entry(
        dir.path(),
        "page.st",
        "[data-role] { content: \":st/forms\"; }\n",
    );
    assert!(
        out.css.contains("data-role"),
        "the attribute-selector scope must compile as .st: {}",
        out.css
    );
}

#[test]
fn sel_wrapped_forms_lower_to_scope_blocks_so_analysis_sees_them() {
    let reg = &*spacetime::syntax::STDLIB_REGISTRY;
    let edn_ast =
        spacetime::edn::to_st_file(EDN_EACH, reg).unwrap_or_else(|e| panic!("EDN read: {e}"));

    // The dual shape `.st`'s rematch produces (parser/mod.rs: scope members
    // are pushed flat AND cloned into the scope):
    //
    //  - flat in `matches`, selector set — the printer and the event passes
    //    read them there;
    //  - cloned into `ScopeBlock::matches` — the analysis passes (W0201's
    //    @each consumption, read-usages) scan ONLY there. Flat-only was the
    //    bug: the page compiled and every sel-consumed source was falsely
    //    flagged "defined but never used".
    assert!(
        edn_ast
            .matches
            .iter()
            .any(|m| m.macro_name == "each" && m.selector.as_deref() == Some(".log")),
        "the each form must stay in the flat match list, selector set"
    );
    let scope = edn_ast
        .scopes
        .iter()
        .find(|s| s.selector == ".log")
        .expect("a ScopeBlock for .log");
    assert!(
        scope.matches.iter().any(|m| m.macro_name == "each"),
        "the each form must ALSO sit in the scope block, where analysis scans"
    );
}

#[test]
fn raw_css_strings_are_unescaped_on_read() {
    // `:st/css` strings arrive EDN-escaped; the stylesheet must contain the
    // AUTHORED text, not the escapes (a literal `\"` in emitted CSS is an
    // invalid declaration).
    let src = "{:st/css [[:st/raw \"a::before { content: \\\"ok\\\"; }\"]]}\n";
    let ast = spacetime::edn::to_st_file(src, &spacetime::syntax::STDLIB_REGISTRY)
        .unwrap_or_else(|e| panic!("EDN read: {e}"));
    assert_eq!(
        ast.raw_css_blocks[0].source,
        "a::before { content: \"ok\"; }",
        "the escapes must be decoded, not carried into the stylesheet"
    );
}

#[test]
fn proactive_compile_writes_build_status_for_an_edn_site() {
    let dir = tempfile::tempdir().expect("tmpdir");
    std::fs::write(dir.path().join("index.edn"), EDN_SRC).unwrap();

    let compile_cache = CompileCache::new();
    let dev_cache = DevCompileCache::new();
    spacetime::server::compile_default_entry_with_status(
        dir.path(),
        false,
        true,
        &compile_cache,
        &dev_cache,
    );

    let status: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("build-status.json"))
            .expect("build-status.json must exist after a proactive compile"),
    )
    .expect("status is JSON");
    assert_eq!(status["ok"], true, "clean page compiles ok: {status}");
    assert_eq!(status["entry"], "index.edn");
    assert!(status["mtime_ms"].as_u64().unwrap() > 0);
    // The handshake: the status hashes the content it compiled (sha256 hex),
    // so a writer can await the verdict for exactly its own write.
    use sha2::Digest;
    let expect = sha2::Sha256::digest(EDN_SRC.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(status["content_sha256"], expect);
}

#[test]
fn proactive_compile_records_a_broken_edn_site_as_not_ok() {
    let dir = tempfile::tempdir().expect("tmpdir");
    std::fs::write(dir.path().join("index.edn"), "(no-such-form :x 1)").unwrap();

    let compile_cache = CompileCache::new();
    let dev_cache = DevCompileCache::new();
    spacetime::server::compile_default_entry_with_status(
        dir.path(),
        false,
        true,
        &compile_cache,
        &dev_cache,
    );

    let status: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("build-status.json")).expect("status file"),
    )
    .expect("status is JSON");
    assert_eq!(status["ok"], false, "unknown form must fail: {status}");
    assert!(
        !status["diagnostics"].as_array().unwrap().is_empty(),
        "a refusal carries its reason: {status}"
    );
}
