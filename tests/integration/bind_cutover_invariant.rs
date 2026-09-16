//! FEAT-072 repo invariant: the killed reactive macros (@bind / @show / @input /
//! @if-block) must not appear in any tracked `.st` SOURCE. The sole reactive-property
//! surface is the parser-native `:` / `<-` operators. This is a CI-style assertion that
//! the hard cutover stays complete (no regressions reintroducing the deleted directives).
//!
//! Excluded: `stdlib/macros/data.st` (holds the intentional tombstone `%form` decls that
//! keep the directives RECOGNIZED so a use is rejected with E0910), build output (`dist/`,
//! `.cache/`), and comment/doc lines (a `//`- or `///`-prefixed line is documentation).

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Collect `.st` files under `dir`, skipping build/cache dirs.
fn collect_st_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "dist" || name == ".cache" || name == ".git" || name == "node_modules" {
                continue;
            }
            collect_st_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("st") {
            out.push(path);
        }
    }
}

/// A non-comment line that uses one of the killed directives, if any.
fn killed_directive_hit(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("///") {
        return None; // documentation
    }
    for (needle, label) in [("@bind", "@bind"), ("@show", "@show"), ("@input", "@input")] {
        // Word-boundary match: the char after the needle must not continue the
        // directive name (so `@showcases` / `@inputmask` do not false-positive).
        let mut from = 0;
        while let Some(idx) = line[from..].find(needle) {
            let abs = from + idx;
            let after = line[abs + needle.len()..].chars().next();
            let continues =
                matches!(after, Some(c) if c.is_ascii_alphanumeric() || c == '-' || c == '_');
            if !continues {
                return Some(label);
            }
            from = abs + needle.len();
        }
    }
    None
}

#[test]
fn no_killed_reactive_macros_in_tracked_st_sources() {
    let root = repo_root();
    // The migrations entries legitimately REFERENCE the retired directives in
    // their `%match` clauses — they are the migration definitions themselves
    // (PLAN-076; the tombstone forms in stdlib/macros/data.st they replace
    // carried the same carve-out).
    let entries_dir = root.join("stdlib/migrations/entries");

    let mut files = Vec::new();
    // All tracked dirs that hold authored `.st` sources (excludes `.sisyphus`, a historical
    // plan/notepad archive, and `projects/`, a separate gitignored repo).
    for sub in [
        "stdlib",
        "examples",
        "tests",
        "demos",
        "web",
        "experiments",
        "test-trace",
    ] {
        collect_st_files(&root.join(sub), &mut files);
    }

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.starts_with(&entries_dir) {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(file) else {
            continue;
        };
        for (i, line) in contents.lines().enumerate() {
            if let Some(label) = killed_directive_hit(line) {
                violations.push(format!(
                    "{}:{}: {} \u{2014} migrate to the `:` / `<-` surface",
                    file.strip_prefix(&root).unwrap_or(file).display(),
                    i + 1,
                    label
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "FEAT-072 cutover violated \u{2014} killed reactive macros found in tracked .st sources:\n{}",
        violations.join("\n")
    );
}
