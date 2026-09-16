//! What would the compiler know, if it stopped throwing the evidence away?
//!
//! Walks every `.st` in the corpus, pulls the value side of each `prop: value;`
//! declaration, and runs the FEAT-109 inferrer over it. Reports the
//! distribution. This is a MEASUREMENT, not a gate — it exists to size the
//! feature honestly before anything is wired to it.
//!
//! Run: cargo test --test corpus_inference_scan -- --nocapture

use spacetime::types::value_infer::{Inferred, infer_value_type};
use std::collections::BTreeMap;

fn st_files(root: &str, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, "dist" | "node_modules" | "vendor" | ".git") {
                continue;
            }
            st_files(p.to_str().unwrap_or(""), out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("st") {
            out.push(p);
        }
    }
}

/// Crude but honest: the value side of `ident: ... ;` on one line. Good enough
/// to size a population; deliberately not a parser.
fn declaration_values(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with('@') || t.starts_with('%') {
            continue;
        }
        let Some(colon) = t.find(':') else { continue };
        let (lhs, rhs) = t.split_at(colon);
        if lhs.is_empty() || !lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            continue;
        }
        let val = rhs[1..].trim().trim_end_matches(';').trim();
        if !val.is_empty() && !val.contains('{') {
            out.push(val.to_string());
        }
    }
    out
}

#[test]
fn what_the_compiler_would_know() {
    let mut files = Vec::new();
    for root in ["examples", "demos", "stdlib", "tests/fixtures"] {
        st_files(root, &mut files);
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    let mut examples: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        for v in declaration_values(&src) {
            total += 1;
            let key = match infer_value_type(&v) {
                Inferred::Scalar(s) => s,
                Inferred::Reference => "«reference»".to_string(),
                Inferred::Ambiguous(set) => format!("«ambiguous: {}»", set.join("|")),
                Inferred::Unknown => "«unknown»".to_string(),
            };
            *counts.entry(key.clone()).or_default() += 1;
            let e = examples.entry(key).or_default();
            if e.len() < 3 && !e.contains(&v) {
                e.push(v.clone());
            }
        }
    }

    println!("\n=== {} .st files, {total} declaration values ===\n", files.len());
    let mut rows: Vec<_> = counts.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1));
    let mut typed = 0usize;
    for (k, n) in &rows {
        let pct = (**n as f64) * 100.0 / (total.max(1) as f64);
        if !k.starts_with('«') {
            typed += **n;
        }
        println!(
            "  {:>6}  {:>5.1}%  {:<28} e.g. {:?}",
            n,
            pct,
            k,
            examples.get(*k).map(|v| v.join(", ")).unwrap_or_default()
        );
    }
    println!(
        "\n  TYPED WITH ZERO ANNOTATION: {typed} / {total}  ({:.1}%)",
        (typed as f64) * 100.0 / (total.max(1) as f64)
    );
    assert!(total > 0, "scanned nothing");
}
