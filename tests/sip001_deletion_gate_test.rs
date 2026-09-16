//! SIP-001 P23 deletion gate.
//!
//! A deletion gate keeps the family's elegance claim falsifiable: an implementation
//! that quietly restores an old surface must turn CI red, not merely leave prose
//! behind that says the surface disappeared. These are structural claims, so this
//! gate inspects declarations and parser paths rather than emitted output.
//!
//! Out of scope: multicam's state-machine workaround and copy-paste style mixins
//! have no unique syntactic identity, so grep cannot distinguish them from a
//! legitimate state machine or repeated style. `trigger: "..."` is likewise
//! ambiguous with event-driver DOM event names. The clip-degrade claim names no
//! shipped construct (the gate records the absence of its canonical identifier,
//! but cannot prove the absence of an unnamed special case).

use std::fs;
use std::path::{Path, PathBuf};

const RETIRED_HEADS: &[&str] = &[
    "scroll", "load", "hover", "click", "mouse", "pointer", "time", "loop",
];
const EXPECTED_EASING_FORMS: usize = 28;

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn st_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("read source directory") {
            let entry = entry.expect("read source entry");
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "st") {
                files.push(path);
            }
        }
    }
    files
}

fn non_capsule_stdlib_files() -> Vec<PathBuf> {
    st_files(&repo_path("stdlib"))
        .into_iter()
        .filter(|path| !path.starts_with(repo_path("stdlib/migrations/entries")))
        .collect()
}

fn source_without_st_comments(path: &Path) -> String {
    fs::read_to_string(path)
        .expect("read Spacetime source")
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn retired_head_registrations(source: &str) -> Vec<String> {
    let mut registrations = Vec::new();
    let mut macro_body = String::new();
    let mut in_macro = false;

    for line in source.lines() {
        if line.trim_start().starts_with("%macro ") {
            if in_macro {
                registrations.extend(retired_heads_in_macro(&macro_body));
                macro_body.clear();
            }
            in_macro = true;
        }
        if in_macro {
            macro_body.push_str(line);
            macro_body.push('\n');
        }
    }
    if in_macro {
        registrations.extend(retired_heads_in_macro(&macro_body));
    }
    registrations
}

fn directive_heads_in_macro<'a>(macro_body: &str, heads: &'a [&str]) -> Vec<&'a str> {
    heads
        .iter()
        .copied()
        .filter(|head| {
            macro_body.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with(&format!("@{head} ")) || line.starts_with(&format!("@{head}("))
            })
        })
        .collect()
}

fn retired_heads_in_macro(macro_body: &str) -> Vec<String> {
    directive_heads_in_macro(macro_body, RETIRED_HEADS)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn all_rust_source() -> String {
    let mut source = String::new();
    let mut dirs = vec![repo_path("src")];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("read Rust source directory") {
            let entry = entry.expect("read Rust source entry");
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                source.push_str(&fs::read_to_string(path).expect("read Rust source"));
                source.push('\n');
            }
        }
    }
    source
}

#[test]
fn retired_macro_heads_exist_only_in_the_migration_capsule() {
    let registrations: Vec<_> = non_capsule_stdlib_files()
        .into_iter()
        .flat_map(|path| {
            retired_head_registrations(&source_without_st_comments(&path))
                .into_iter()
                .map(move |head| format!("{}: @{head}", path.display()))
        })
        .collect();

    assert!(
        registrations.is_empty(),
        "retired heads must be registered only inside stdlib/migrations/entries/ \\
         (%migration is the deliberate compatibility capsule):\n{}",
        registrations.join("\n")
    );
}

#[test]
fn retired_head_matcher_rejects_a_non_capsule_control() {
    let control = "%macro forbidden {\n  %form {\n    @scroll old { $body:keyframes }\n  }\n}";
    assert_eq!(retired_head_registrations(control), ["scroll"]);
}
#[test]
fn after_head_and_on_event_dispatcher_exist_only_in_the_migration_capsule() {
    let violations: Vec<_> = non_capsule_stdlib_files()
        .into_iter()
        .flat_map(|path| {
            let source = source_without_st_comments(&path);
            let mut found: Vec<String> = directive_heads_in_macro(&source, &["after"])
                .into_iter()
                .map(|head| format!("{}: @{head}", path.display()))
                .collect();
            if source.contains("%macro on-event-dispatcher") {
                found.push(format!("{}: %macro on-event-dispatcher", path.display()));
            }
            found
        })
        .collect();
    assert!(
        violations.is_empty(),
        "@after and the old @on event dispatcher are compatibility-only shapes:\n{}",
        violations.join("\n")
    );
}
#[test]
fn effect_macro_and_runner_primitive_are_deleted() {
    assert!(
        !repo_path("stdlib/macros/effect.st").exists(),
        "@effect must migrate to @on $signal; its macro remains live"
    );
    assert!(
        !repo_path("stdlib/primitives/effect.st").exists(),
        "effect-runner is the retired @effect runtime path and must be deleted with the macro"
    );
}

#[test]
fn st_timelines_string_registry_is_gone() {
    let mentions: Vec<_> = non_capsule_stdlib_files()
        .into_iter()
        .filter_map(|path| {
            let source = source_without_st_comments(&path);
            source
                .contains("ST.timelines")
                .then(|| path.display().to_string())
        })
        .collect();
    assert!(
        mentions.is_empty(),
        "ST.timelines survived in:\n{}",
        mentions.join("\n")
    );
}

#[test]
fn twenty_eight_easing_curves_are_forms_not_entities() {
    let forms = non_capsule_stdlib_files()
        .into_iter()
        .map(|path| source_without_st_comments(&path))
        .flat_map(|source| source.lines().map(str::to_owned).collect::<Vec<_>>())
        .filter(|line| line.trim_start().starts_with("@form easing --ease-"))
        .count();
    assert_eq!(
        forms, EXPECTED_EASING_FORMS,
        "SIP-001 promises exactly 28 `@form easing --ease-*` declarations; \\
         a different count means curves remain entities or were silently lost"
    );
}

#[test]
fn preset_ref_prefix_path_is_deleted() {
    let source = all_rust_source();
    assert!(
        !source.contains("PRESET_REF") && !source.contains("parse_preset_ref"),
        "the `~` preset prefix path remains in Rust syntax code"
    );
}

#[test]
fn pose_track_object_index_is_deleted() {
    let uses: Vec<_> = non_capsule_stdlib_files()
        .into_iter()
        .filter_map(|path| {
            let source = source_without_st_comments(&path);
            source
                .contains("objectIndex")
                .then(|| path.display().to_string())
        })
        .collect();
    assert!(
        uses.is_empty(),
        "@pose-track objectIndex is a migration-only legacy shape, but remains in:\n{}",
        uses.join("\n")
    );
}

#[test]
fn clip_degrade_has_no_shipped_named_special_case() {
    let source = format!(
        "{}\n{}",
        all_rust_source(),
        non_capsule_stdlib_files()
            .into_iter()
            .map(|path| source_without_st_comments(&path))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        !source.contains("clip-degrade") && !source.contains("clip_degrade"),
        "a named clip-degrade special case exists; it must be replaced by driver fallback"
    );
}

#[test]
fn freeze_ramp_and_reverse_have_no_dedicated_macro_heads() {
    let dedicated: Vec<_> = non_capsule_stdlib_files()
        .into_iter()
        .filter_map(|path| {
            let source = source_without_st_comments(&path);
            ["freeze", "ramp", "reverse"]
                .iter()
                .find(|name| source.contains(&format!("%macro {name}")))
                .map(|name| format!("{}: %{name}", path.display()))
        })
        .collect();
    assert!(
        dedicated.is_empty(),
        "rate concepts still have dedicated macros:\n{}",
        dedicated.join("\n")
    );
}
