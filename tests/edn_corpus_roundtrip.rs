//! **The thesis test** (PLAN-148 W2, SPEC §2 obligation P2).
//!
//! The EDN design rests on one claim: *Spacetime's grammar is data*. If true, any
//! form can be printed from its `%form` pattern plus its captures, with no
//! per-construct code. This test attacks that claim with the entire real corpus.
//!
//! ```text
//! ∀ f ∈ stdlib/**/*.st ∪ demos/**/*.st :
//!     parse(f).matches  ≡  parse(print(parse(f).matches)).matches
//! ```
//!
//! Equivalence is SEMANTIC (class A), not textual: macro identity per match must
//! survive. Comments and formatting are explicitly not preserved.
//!
//! # Reading a failure
//!
//! This test is a MEASURING INSTRUMENT, not a pass/fail gate on day one. It
//! prints a ranked defect list — every macro that cannot be pattern-printed, and
//! every printed fragment that fails to re-parse. Per SPEC §8.2 those are verse
//! defects with names, not a fudge factor to tune away.
//!
//! `EDN_CORPUS_STRICT=1` turns the report into a hard failure. That is the state
//! we are driving toward; until the defect list is empty the report is the
//! deliverable.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use spacetime::edn::print_forms;
use spacetime::syntax::STDLIB_REGISTRY;

fn collect_st_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // `vendor/` holds third-party sources that are not ours to print,
            // and `.cache/` is build output.
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "vendor" || name == ".cache" || name == "node_modules" {
                continue;
            }
            collect_st_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("st") {
            out.push(path);
        }
    }
}

#[derive(Default)]
struct Report {
    files_total: usize,
    files_parsed: usize,
    forms_total: usize,
    forms_printed: usize,
    /// macro name → count of print failures.
    print_failures: BTreeMap<String, usize>,
    /// macro name → one example message (first seen).
    print_examples: BTreeMap<String, String>,
    /// files whose printed output failed to re-parse.
    reparse_failures: Vec<(PathBuf, String)>,
    /// files where the match count changed across the round trip.
    count_drift: Vec<(PathBuf, usize, usize)>,
    /// files where a macro identity changed across the round trip.
    identity_drift: Vec<(PathBuf, String, String)>,
}

#[test]
fn corpus_roundtrip_report() {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut files = Vec::new();
    collect_st_files(&Path::new(root).join("stdlib"), &mut files);
    collect_st_files(&Path::new(root).join("demos"), &mut files);
    files.sort();

    assert!(
        !files.is_empty(),
        "corpus is empty — the test would trivially pass and prove nothing"
    );

    let mut r = Report {
        files_total: files.len(),
        ..Default::default()
    };

    for path in &files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };

        // A file the CURRENT parser cannot read is not this test's business.
        let Ok(ast) = spacetime::parse(&source) else {
            continue;
        };
        r.files_parsed += 1;
        if ast.matches.is_empty() {
            continue;
        }
        r.forms_total += ast.matches.len();

        // Per-form pass FIRST, purely to attribute failures to a macro name — one
        // bad form must not hide the rest of the file in the defect list.
        let mut file_ok = true;
        for m in &ast.matches {
            match spacetime::edn::print_form_in(m, &STDLIB_REGISTRY, Some(&ast)) {
                Ok(_) => r.forms_printed += 1,
                Err(e) => {
                    file_ok = false;
                    *r.print_failures.entry(e.macro_name.clone()).or_default() += 1;
                    r.print_examples
                        .entry(e.macro_name.clone())
                        .or_insert_with(|| e.message.clone());
                }
            }
        }
        if !file_ok {
            continue;
        }

        // Whole-file round trip through the REAL entry point.
        //
        // This must be `print_forms`, not a join of `print_form` results: a
        // selector-scoped form only means what it means INSIDE its scope block,
        // and `print_forms` is what reconstructs those blocks. Joining the
        // per-form strings drops the scopes and manufactures drift that the
        // shipped API does not have (measured: `confirm.st` reported 6 → 3 under
        // the join, and round-trips 6 → 6 through `print_forms`).
        // `print_file`, not `print_forms`: a `@template` body lives in a sibling
        // `@template:<name>` scope, not in the match, so only the whole-document
        // entry point can print it (W4).
        let printed = match spacetime::edn::print_file(&ast, &STDLIB_REGISTRY) {
            Ok(t) => t,
            Err(e) => {
                *r.print_failures.entry(e.macro_name.clone()).or_default() += 1;
                r.print_examples
                    .entry(e.macro_name.clone())
                    .or_insert_with(|| e.message.clone());
                continue;
            }
        };
        match spacetime::parse(&printed) {
            Ok(reparsed) => {
                if reparsed.matches.len() != ast.matches.len() {
                    r.count_drift
                        .push((path.clone(), ast.matches.len(), reparsed.matches.len()));
                    continue;
                }
                for (a, b) in ast.matches.iter().zip(reparsed.matches.iter()) {
                    let an = a.matched_macro.as_deref().unwrap_or(&a.macro_name);
                    let bn = b.matched_macro.as_deref().unwrap_or(&b.macro_name);
                    if an != bn {
                        r.identity_drift
                            .push((path.clone(), an.to_string(), bn.to_string()));
                        break;
                    }
                }
            }
            Err(e) => {
                r.reparse_failures
                    .push((path.clone(), format!("{e:?}").chars().take(160).collect()));
            }
        }
    }

    // ── the report ───────────────────────────────────────────────────────────
    let pct = if r.forms_total == 0 {
        0.0
    } else {
        100.0 * r.forms_printed as f64 / r.forms_total as f64
    };

    println!("\n════ EDN corpus round-trip (PLAN-148 W2 thesis test) ════");
    println!("registry forms          : {}", STDLIB_REGISTRY.form_count());
    println!("files scanned / parsed  : {} / {}", r.files_total, r.files_parsed);
    println!(
        "forms printed           : {} / {}  ({pct:.1}%)",
        r.forms_printed, r.forms_total
    );
    println!("distinct failing macros : {}", r.print_failures.len());
    println!("re-parse failures       : {}", r.reparse_failures.len());
    println!("match-count drift       : {}", r.count_drift.len());
    println!("macro-identity drift    : {}", r.identity_drift.len());

    if !r.print_failures.is_empty() {
        println!("\n── print failures by macro (the verse defect list) ──");
        let mut ranked: Vec<_> = r.print_failures.iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (name, count) in ranked.iter().take(40) {
            let ex = r.print_examples.get(*name).map(String::as_str).unwrap_or("");
            println!("  {count:>5}  {name:<38} {}", ex.chars().take(90).collect::<String>());
        }
        if ranked.len() > 40 {
            println!("  … and {} more", ranked.len() - 40);
        }
    }

    if !r.reparse_failures.is_empty() {
        println!("\n── printed text that did not re-parse ──");
        for (p, e) in r.reparse_failures.iter().take(15) {
            println!("  {}\n      {e}", p.display());
        }
        if r.reparse_failures.len() > 15 {
            println!("  … and {} more", r.reparse_failures.len() - 15);
        }
    }

    if !r.count_drift.is_empty() {
        println!("\n── match-count drift ──");
        for (p, before, after) in r.count_drift.iter().take(15) {
            println!("  {} : {before} → {after}", p.display());
        }
        if r.count_drift.len() > 15 {
            println!("  … and {} more", r.count_drift.len() - 15);
        }
    }

    if !r.identity_drift.is_empty() {
        println!("\n── macro-identity drift ──");
        for (p, a, b) in r.identity_drift.iter().take(15) {
            println!("  {} : {a} → {b}", p.display());
        }
        if r.identity_drift.len() > 15 {
            println!("  … and {} more", r.identity_drift.len() - 15);
        }
    }
    println!("═════════════════════════════════════════════════════════\n");

    // Guard against the report silently measuring nothing.
    assert!(
        r.forms_total > 0,
        "no forms were parsed from the corpus — the instrument is broken, not the thesis"
    );

    // ── the EDN ingress gate (BUG-370) ─────────────────────────────────────
    //
    // The measurement above is `.st → EDN → .st`. It never feeds EDN back
    // through the READER, so the printer half and the reader half were each
    // tested against `.st` and never against EACH OTHER — the one direction a
    // GENERATOR uses. That gap hid a total failure: `spacetime edn F` produced
    // output `spacetime st` rejected, for every form whose pattern references a
    // `%capture_type` with inner captures (`:arms` is not a capture of this
    // form). 43 of 120 files round-tripped.
    //
    // So this gate asserts the generator's direction directly.
    {
        let mut ingress_ok = 0usize;
        let mut ingress_fail: Vec<String> = Vec::new();

        for path in &files {
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            let Ok(parsed) = spacetime::parse(&src) else {
                continue;
            };
            let doc = spacetime::edn::to_document(&parsed);
            let text = spacetime::edn::write_document(&doc, &STDLIB_REGISTRY);

            match spacetime::edn::to_st_file(&text, &STDLIB_REGISTRY) {
                Ok(_) => ingress_ok += 1,
                Err(e) => ingress_fail.push(format!("{}: {e}", path.display())),
            }
        }

        println!("── EDN ingress ────────────────────────────────────");
        println!("  documents read back : {ingress_ok}");
        println!("  rejected            : {}", ingress_fail.len());
        for f in ingress_fail.iter().take(5) {
            println!("    {}", &f[..f.len().min(140)]);
        }

        // Ratcheted, like the other budgets: the remaining rejections are
        // `[:st/block …]` bodies (FUP-188, structured Expr), a KNOWN and filed
        // limitation rather than a defect in the ingress path.
        const KNOWN_INGRESS_GAPS: usize = 8;
        assert!(
            ingress_fail.len() <= KNOWN_INGRESS_GAPS,
            "EDN the printer emits must be readable by the reader: {} rejected \
             (budget {KNOWN_INGRESS_GAPS}, FUP-188 opaque bodies). Lower the \
             budget when you fix cases; never raise it.",
            ingress_fail.len()
        );
    }

    // ── the standing gates ───────────────────────────────────────────────────
    //
    // These are ALWAYS on, not behind an env var: they are clean today, and a
    // regression in any of them means the thesis has sprung a leak. A gate that
    // only runs when someone remembers to set a variable is not a gate.

    assert!(
        r.print_failures.is_empty(),
        "every registered form must print from its %form pattern — that IS the \
         grammar-as-data thesis. Failing macros: {:?}",
        r.print_failures
    );

    assert!(
        r.identity_drift.is_empty(),
        "a form must round-trip to ITSELF; identity drift means the printed text \
         matched a different macro: {:?}",
        r.identity_drift
    );

    // Re-parse failures are tracked against a known, filed defect rather than a
    // bare number, so a NEW one fails immediately instead of hiding in a budget.
    const KNOWN_REPARSE_DEFECTS: usize = 2; // BUG-368 (states capture truncates)
    assert!(
        r.reparse_failures.len() <= KNOWN_REPARSE_DEFECTS,
        "new re-parse failure(s) beyond the {KNOWN_REPARSE_DEFECTS} filed under \
         BUG-368: {:?}",
        r.reparse_failures
    );

    // Match-count drift is still being driven down; the ratchet stops it going
    // back up while the remaining cases are worked through.
    //
    // 19 -> 15: printing now resolves FLATTENED captures (a `%form` names
    // `$send`/`$receive`, the match stores `send_clause`/`receive_block`'s own
    // captures at the top level). `@data signal` had been printing as
    // `@data signal $inc() to $counter;` — body gone, which is every signal in
    // the corpus plus four `@on`-bearing showcases.
    const DRIFT_RATCHET: usize = 15;
    assert!(
        r.count_drift.len() <= DRIFT_RATCHET,
        "match-count drift regressed past the ratchet ({} > {DRIFT_RATCHET}). \
         Lower the ratchet when you fix cases; never raise it.",
        r.count_drift.len()
    );

    // Full strictness, for the run that closes out the remaining work.
    if std::env::var("EDN_CORPUS_STRICT").is_ok() {
        assert!(
            r.reparse_failures.is_empty() && r.count_drift.is_empty(),
            "strict mode: the corpus round trip is not yet clean (see report above)"
        );
    }
}
