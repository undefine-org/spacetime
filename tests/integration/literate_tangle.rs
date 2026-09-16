//! SIP-002 — literate Spacetime (`.st.md`): tangle → compile → served page.
//!
//! Unit coverage for the transform itself lives in `src/literate.rs`. These
//! tests prove the END-TO-END rail: a real `.st.md` on disk goes through
//! `Compiler::from_file` (the single central read that every entry point uses)
//! and produces served HTML in DOCUMENT ORDER with the fence bodies live.

use spacetime::compiler::Compiler;
use spacetime::literate;
use std::path::{Path, PathBuf};

/// Write a `.st.md` into a unique temp dir and return `(dir, entry_path)`.
fn write_literate(tag: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("st_lit_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let entry = dir.join("index.st.md");
    std::fs::write(&entry, body).expect("write entry");
    (dir, entry)
}

fn compile_literate(dir: &Path, entry: &Path) -> spacetime::CompiledSpacetime {
    Compiler::from_file(entry, Path::new("."))
        .expect("compiler from_file on a .st.md")
        .with_site_dir(Some(dir.to_path_buf()))
        .fresh_registry()
        .compile()
}

#[test]
fn literate_document_compiles_through_from_file() {
    // The whole point: a markdown file IS a compilable Spacetime entry.
    let doc = format!(
        "# Title\n\nProse before the code.\n\n{}\n{}\n{}\n",
        "```st", "<div class=\"demo\">live</div>", "```"
    );
    let (dir, entry) = write_literate("basic", &doc);
    let compiled = compile_literate(&dir, &entry);

    assert!(
        compiled.html.contains("class=\"demo\""),
        "fence markup must reach served HTML: {}",
        compiled.html
    );
    assert!(
        compiled.html.contains("lit-prose"),
        "prose must become a mount section: {}",
        compiled.html
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn served_html_preserves_document_order() {
    // prose → demo → prose, exactly as authored. This is what makes a fence's
    // live result appear where it sits in the essay.
    let doc = format!(
        "first para\n\n{}\n{}\n{}\n\nsecond para\n",
        "```st", "<div class=\"demo\">live</div>", "```"
    );
    let (dir, entry) = write_literate("order", &doc);
    let compiled = compile_literate(&dir, &entry);

    // Segment ids are namespaced per file (seed-n) so merged literate files
    // cannot collide — assert on the marker, not a specific number.
    let seg0 = compiled
        .html
        .find("data-lit-seg=\"")
        .expect("first prose section");
    let demo = compiled.html.find("class=\"demo\"").expect("demo markup");
    let seg1 = compiled.html[demo..]
        .find("data-lit-seg=\"")
        .map(|i| demo + i)
        .expect("second prose section");
    assert!(
        seg0 < demo && demo < seg1,
        "document order lost; html:\n{}",
        compiled.html
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prose_renders_as_markdown_not_literal_escapes() {
    // Regression for the bug SIP-002's first fixture caught: prose reached
    // snarkdown with literal backslash-n. The emitted JS must carry newline
    // ESCAPES (`\n`), never an escaped backslash.
    let doc = "# Heading\n\nSome **bold** prose.\n";
    let (dir, entry) = write_literate("escapes", doc);
    let compiled = compile_literate(&dir, &entry);

    assert!(
        compiled.js.contains("snarkdown"),
        "md engine must be demand-injected: {}",
        &compiled.js[..compiled.js.len().min(200)]
    );
    assert!(
        compiled.js.contains("# Heading\\n\\nSome **bold** prose."),
        "prose must carry real newline escapes"
    );
    assert!(
        !compiled.js.contains("# Heading\\\\n"),
        "literal backslash-n leaked into prose"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signals_declared_in_one_fence_are_live_in_a_later_one() {
    // Fences share ONE file scope — the essay is one program, sectioned by prose.
    let doc = format!(
        "intro\n\n{}\n{}\n{}\n\nmiddle prose\n\n{}\n{}\n{}\n",
        "```st hidden",
        "@data inline $n : 0;",
        "```",
        "```st",
        "<button class=\"plus\">+</button>\n<span class=\"out\"></span>\n\n.plus { @on &.click { $n <- $n + 1; } }\n.out { text <- $n; }",
        "```"
    );
    let (dir, entry) = write_literate("scope", &doc);
    let compiled = compile_literate(&dir, &entry);

    assert!(
        compiled.html.contains("class=\"plus\"") && compiled.html.contains("class=\"out\""),
        "later fence markup missing: {}",
        compiled.html
    );
    assert!(
        compiled.js.contains("plus"),
        "click handler from the later fence must be wired"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn src_flag_shows_source_and_still_runs_it() {
    let doc = format!(
        "{}\n{}\n{}\n",
        "```st src", "<div class=\"demo\">live</div>", "```"
    );
    let (dir, entry) = write_literate("srcflag", &doc);
    let compiled = compile_literate(&dir, &entry);

    let shown = compiled
        .html
        .find("class=\"lit-src\"")
        .expect("verbatim source block");
    let live = compiled
        .html
        .rfind("class=\"demo\"")
        .expect("live markup after it");
    assert!(
        shown < live,
        "source must precede the result: {}",
        compiled.html
    );
    // The shown source is ESCAPED DATA — angle brackets are entities, so the
    // browser displays the code instead of mounting a second live copy.
    assert!(
        compiled.html.contains("&lt;div class=") && compiled.html.contains("&gt;live&lt;/div&gt;"),
        "shown source must be escaped data, not parsed markup: {}",
        compiled.html
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn non_st_fences_are_quoted_never_executed() {
    // A ```spacetime fence documents code WITHOUT running it — the existing
    // convention across docs/ must keep working inside a literate document.
    let doc = format!(
        "{}\n{}\n{}\n",
        "```spacetime", "<div class=\"ghost\">quoted</div>", "```"
    );
    let (dir, entry) = write_literate("inert", &doc);
    let compiled = compile_literate(&dir, &entry);

    // The prose is rendered at BUILD time now, so the quoted fence lands in
    // the HTML as ESCAPED text inside `<pre><code>` (`&lt;div class="ghost"&gt;`)
    // — the bare attribute text is present, a LIVE `<div class="ghost">`
    // element is not. Probe for the element, not the substring.
    assert!(
        !compiled.html.contains("<div class=\"ghost\""),
        "quoted markup must NOT be spliced as code: {}",
        compiled.html
    );
    assert!(
        compiled.html.contains("&lt;div class=\"ghost\"&gt;quoted&lt;/div&gt;"),
        "quoted markup must survive as escaped prose content: {}",
        compiled.html
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plain_st_files_are_untouched_by_the_literate_rail() {
    // Additive by construction: `.st` is not literate, so its bytes pass
    // straight through `read_source`.
    let dir = std::env::temp_dir().join(format!("st_lit_plain_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let entry = dir.join("index.st");
    std::fs::write(&entry, "<div class=\"plain\">x</div>\n").expect("write");

    assert!(!literate::is_literate(&entry));
    let (content, map) = literate::read_source(&entry).expect("read");
    assert_eq!(content, "<div class=\"plain\">x</div>\n");
    assert!(map.is_none(), "a plain .st file has no line map");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_error_in_a_fence_reports_the_authored_st_md_line() {
    // SIP-002 W4: the parser sees TANGLED text. Without remapping, an error
    // inside a fence is reported at the tangled line number and the snippet
    // shows synthesized wiring the author never wrote — actively misleading.
    // `Compiler::from_file` must render the diagnostic against the ORIGINAL.
    let doc = format!(
        "# Title\n\nprose\n\nmore prose\n\n{}\n<div class=\"ok\"></div>\n.broken {{ color: ; @@@ }}\n{}\n",
        "```st", "```"
    );
    //                                    ^ the bad token is on AUTHORED line 9
    let (dir, entry) = write_literate("w4diag", &doc);

    let err = Compiler::from_file(&entry, Path::new("."))
        .err()
        .expect("a malformed fence must fail to compile");

    assert!(
        err.contains("index.st.md:9"),
        "diagnostic must point at the authored line 9; got:\n{err}"
    );
    // The rendered snippet must be the AUTHOR's document, fence markers and
    // all — not the synthesized @doc wiring.
    assert!(
        !err.contains("data-lit-seg"),
        "diagnostic leaked tangled wiring to the reader:\n{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_error_in_a_plain_st_file_is_unaffected_by_the_remap() {
    let dir = std::env::temp_dir().join(format!("st_lit_plainerr_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let entry = dir.join("index.st");
    // Bad token on line 2.
    std::fs::write(
        &entry,
        "<div class=\"ok\"></div>\n.broken { color: ; @@@ }\n",
    )
    .expect("write");

    let err = Compiler::from_file(&entry, Path::new("."))
        .err()
        .expect("malformed .st must fail");
    assert!(
        err.contains("index.st:2"),
        "plain .st diagnostics must keep their own line numbers; got:\n{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_literate_document_can_import_a_st_module() {
    // Chrome/layout belongs in a `.st` module, imported by the essay — but
    // `resolve_imports` reads bytes on its own path, so it needed the same
    // tangle-aware read. (It also means a `.st` page can import a `.st.md`.)
    let dir = std::env::temp_dir().join(format!("st_lit_import_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("chrome.st"),
        "<aside class=\"side\">nav</aside>\n.side { position: fixed; }\n",
    )
    .expect("write module");
    let doc = format!(
        "{}\n@import \"./chrome.st\"\n{}\n\nprose after the import.\n",
        "```st hidden", "```"
    );
    let entry = dir.join("index.st.md");
    std::fs::write(&entry, doc).expect("write entry");

    let compiled = compile_literate(&dir, &entry);
    assert!(
        compiled.html.contains("class=\"side\""),
        "imported module markup missing from a literate page: {}",
        compiled.html
    );
    assert!(
        compiled.css.contains("position: fixed"),
        "imported module styles missing: {}",
        compiled.css
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_plain_st_page_can_import_a_literate_document() {
    // The inverse direction: `.st` imports `.st.md`. Without a tangle-aware
    // read the importer would parse markdown as Spacetime and silently
    // contribute nothing — no error, just a missing section.
    let dir = std::env::temp_dir().join(format!("st_lit_revimport_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let lit = format!(
        "# Doc\n\nprose\n\n{}\n<div class=\"from-literate\">x</div>\n{}\n",
        "```st", "```"
    );
    std::fs::write(dir.join("part.st.md"), lit).expect("write literate part");
    let entry = dir.join("index.st");
    std::fs::write(
        &entry,
        "@import \"./part.st.md\"\n<main class=\"host\"></main>\n",
    )
    .expect("write entry");

    let compiled = compile_literate(&dir, &entry);
    assert!(
        compiled.html.contains("class=\"from-literate\""),
        "literate import contributed nothing: {}",
        compiled.html
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn imported_literate_files_keep_their_own_prose() {
    // BUG: two literate files merged into one page both number their prose
    // segments from 0, so every `[data-lit-seg="0"] { @doc(content: …) }`
    // rule collides and the last import's prose renders in EVERY prose
    // section (host's included). Segment ids must be namespaced per file.
    let dir = std::env::temp_dir().join(format!("st_lit_ns_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("snips")).expect("temp dir");
    std::fs::write(
        dir.join("snips").join("a.st.md"),
        "## Alpha\n\nAlpha's own prose.\n\n```st\n<div class=\"da\">A</div>\n```\n",
    )
    .expect("write a");
    std::fs::write(
        dir.join("snips").join("b.st.md"),
        "## Beta\n\nBeta's own prose.\n\n```st\n<div class=\"db\">B</div>\n```\n",
    )
    .expect("write b");
    let entry = dir.join("index.st.md");
    std::fs::write(
        &entry,
        "```st hidden\n@import \"./snips/a.st.md\"\n@import \"./snips/b.st.md\"\n```\n\n# Host title\n\nThe host's own prose.\n",
    )
    .expect("write entry");

    let compiled = compile_literate(&dir, &entry);

    // Three distinct prose segments must exist, each carrying ITS OWN markdown
    // to the runtime (the content travels inside the served page's doc rules).
    for prose in ["Alpha's own prose", "Beta's own prose", "host's own prose"] {
        assert!(
            compiled.html.contains(prose) || compiled.js.contains(prose),
            "prose segment lost or overwritten by another file's: {prose}"
        );
    }
    // And the segment ids must not collide across files: at most one element
    // may carry any single data-lit-seg value.
    let mut ids: Vec<String> = Vec::new();
    let mut rest = compiled.html.as_str();
    while let Some(i) = rest.find("data-lit-seg=\"") {
        let tail = &rest[i + 14..];
        let end = tail.find('"').expect("closing quote");
        ids.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    let mut dedup = ids.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(
        ids.len(),
        dedup.len(),
        "data-lit-seg ids collide across merged literate files: {ids:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
