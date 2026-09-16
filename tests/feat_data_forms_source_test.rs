//! PLAN-135 W4 — `@data forms` : the form-declaration catalog as a static
//! data source. Mirrors `@data dispatch` (FEAT-089): the macros only
//! RECOGNISE the surface; the compiler rewrites a match into `@data inline`
//! whose value is the declaration catalog, so it flows through the identical
//! static-inline path (`@each` SSG-unroll).
//!
//! Rows enumerate every `@form <kind> --name { … }` DECLARATION the build can
//! see — page, `@import`ed files, the `_prelude.st` project overlay, and the
//! stdlib preset library — as
//! `{ name, kind, params, body, doc, source }`.
//! A brand catalog page (demos/brand-catalog) reads this; add a declaration
//! to `_prelude.st` and a card appears next build, zero page edits.
//!
//! Declaration-level `///` doc comments are the row's `doc` — the same
//! contiguous-block rule as %macro docs (FEAT-083), attached at parse time.

use std::process::Command;

fn write_temp(files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "dataforms-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    for (name, source) in files {
        std::fs::write(dir.join(name), source).expect("write source");
    }
    dir
}

/// Compile a page and return (success, combined stdout+stderr, emitted JS text).
fn compile(files: &[(&str, &str)], entry: &str) -> (bool, String, String) {
    let dir = write_temp(files);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(dir.join(entry))
        .output()
        .expect("run spacetime build");
    let js = std::fs::read_to_string(dir.join("spacetime.js")).unwrap_or_default();
    (
        out.status.success(),
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
        js,
    )
}

const PAGE: &str = r#"
/// The brand's primary surface treatment.
@form style --card-surface($pad = 8px) { background: white; padding: $pad; }

/// Brand motion curve.
@form easing --brand-smooth { cubic-bezier(0.4, 0, 0.2, 1) }

@data forms $forms ;

.card { --card-surface; }
"#;

#[test]
fn dataforms_rows_carry_name_kind_params_body_doc_source() {
    let (ok, log, js) = compile(&[("index.st", PAGE)], "index.st");
    assert!(ok, "page compiles: {log}");
    // The declaration rows reached the emitted bundle as data.
    assert!(js.contains("--card-surface"), "style row name: {log}");
    assert!(js.contains("--brand-smooth"), "easing row name: {log}");
    assert!(js.contains("\"style\""), "kind column: {log}");
    assert!(js.contains("\"easing\""), "kind column: {log}");
    // Body text is the declaration's own chunk (param substitution NOT applied).
    assert!(
        js.contains("background: white") || js.contains("background:white"),
        "style body text rides the row: {log}"
    );
    assert!(
        js.contains("cubic-bezier(0.4, 0, 0.2, 1)"),
        "easing body text rides the row: {log}"
    );
    // Params render the author-facing declaration ($pad = 8px).
    assert!(js.contains("pad"), "param name rides the row: {log}");
    // The /// doc block above each declaration is the row's doc.
    assert!(
        js.contains("primary surface treatment"),
        "declaration doc comment rides the row: {log}"
    );
    assert!(js.contains("Brand motion curve"), "second doc rides the row: {log}");
    // Source provenance.
    assert!(js.contains("index.st"), "source file rides the row: {log}");
}

#[test]
fn dataforms_filter_keeps_one_kind() {
    let page = r#"
@form style --card-surface { background: white; }
@form easing --brand-smooth { cubic-bezier(0.4, 0, 0.2, 1) }
@data forms $curves from kind "easing" ;
"#;
    let (ok, log, js) = compile(&[("index.st", page)], "index.st");
    assert!(ok, "page compiles: {log}");
    assert!(js.contains("--brand-smooth"), "easing row survives the filter");
    assert!(
        !js.contains("card-surface"),
        "the style row is filtered OUT of an easing slice"
    );
}

#[test]
fn dataforms_lists_prelude_and_stdlib_declarations() {
    let prelude = r#"
/// Global brand ease.
@form easing --ora-smooth { cubic-bezier(0.25, 0.1, 0.25, 1) }
"#;
    let page = r#"
@data forms $forms from kind "easing" ;
.hero { opacity: 1; }
"#;
    let (ok, log, js) = compile(
        &[("_prelude.st", prelude), ("index.st", page)],
        "index.st",
    );
    assert!(ok, "page compiles: {log}");
    assert!(
        js.contains("--ora-smooth"),
        "a `_prelude.st` declaration is a catalog row: {log}"
    );
    // The stdlib preset library is data too (the 28 shipped curves).
    assert!(
        js.contains("ease-out-expo") || js.contains("--ease-out-expo"),
        "stdlib preset declarations are catalog rows: {log}"
    );
}

#[test]
fn dataforms_rows_unroll_through_each() {
    let page = r#"
@form easing --brand-smooth { cubic-bezier(0.4, 0, 0.2, 1) }

@data forms $forms from kind "easing" ;

<ul>
  @each $forms as $f {
    <li class="row">`$f.name`</li>
  }
</ul>
"#;
    let (ok, log, js) = compile(&[("index.st", page)], "index.st");
    assert!(ok, "page compiles: {log}");
    let html = js.contains("--brand-smooth");
    assert!(
        html,
        "the inline rewrite feeds @each the same as any @data inline: {log}"
    );
}

#[test]
fn dataforms_unknown_filter_key_is_an_empty_slice_like_registry() {
    // Mirrors @data registry/dispatch: the filter retains rows whose field
    // equals the value — an unrecognised key matches nothing. A slice, never
    // a gate: the build stays green. The probe is a MARKUP form so the name
    // appears only in the catalog payload (no easing registration, no style
    // splice) — absence is uncontaminated evidence.
    let page = r#"
@form markup --cat-probe-zz { <span>x</span> }
@data forms $forms from vibe "easing" ;
"#;
    let (ok, log, js) = compile(&[("index.st", page)], "index.st");
    assert!(ok, "unrecognised filter key does not fail the build: {log}");
    assert!(
        !js.contains("--cat-probe-zz"),
        "an unrecognised filter key retains nothing (registry semantics)"
    );
}
