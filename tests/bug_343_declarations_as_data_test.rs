//! PLAN-144 W2 — declarations are data, for every entity (BUG-343).
//!
//! `@data forms` enumerates `@form` declarations, and a pattern library made
//! of `@template`s could not list its own patterns. Not by design: the gate is
//! `is_form_declaring_macro`, which asks the registry one question — does this
//! macro `%registers form`? — and `@template` registers `template`. The string
//! `"form"` is hardcoded in 8 sites against a `%registers` rail that already
//! carries 7 entities (29 driver, 16 binding, 7 form, 4 host, 2 template,
//! 2 type).
//!
//! So the fix GENERALIZES rather than adding a sibling surface: one
//! `@data declarations $x from <entity>` over the existing rail, with
//! `@data forms` as sugar for `entity form`. The `/// key: value` doc fields
//! every showcase pattern already carries become a `fields` map on the row.
//!
//! The load-bearing gate here is `forms_rows_are_unchanged`: generalizing a
//! predicate used by score, drivers and the form compiler could quietly widen
//! what any of them accept, and the brand catalog's 46 rows / 12-34 origin
//! split is the sharpest available proof that it did not.

use std::path::PathBuf;
use std::process::Command;

fn build_page(name: &str, source: &str) -> (bool, String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("scratch").join(format!("w2-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("index.st"), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        .current_dir(&root)
        .output()
        .expect("run build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(dir.join("dist/index.html")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success() && !log.contains("Export failed"), log, html)
}

/// Enumerate templates — the thing a pattern library is made of.
#[test]
fn templates_are_enumerable_as_declarations() {
    let (ok, log, html) = build_page(
        "templates",
        "@import \"../../stdlib/showcases/hero/minimal.st\"\n\
         @import \"../../stdlib/showcases/hero/eastern.st\"\n\n\
         @data declarations $t from template ;\n\n\
         <main class=\"page\"><ul class=\"list\"></ul></main>\n\
         .page { display: block; }\n\
         .list { @each($t as $x) { <li class=\"row\">`$x.name`</li> } }\n",
    );
    assert!(ok, "`@data declarations … from template` must compile:\n{log}");
    for name in ["showcase-hero-minimal", "showcase-hero-eastern"] {
        assert!(
            html.contains(name),
            "template `{name}` must appear as a declaration row — a pattern \
             library that cannot list its own patterns is the whole bug:\n{html}"
        );
    }
}

/// The doc-comment fields every showcase pattern already carries become data,
/// so a gallery can group and filter without re-parsing anything by hand.
#[test]
fn doc_comment_fields_reach_the_row() {
    let (ok, log, html) = build_page(
        "fields",
        "@import \"../../stdlib/showcases/hero/minimal.st\"\n\n\
         @data declarations $t from template ;\n\n\
         <main class=\"page\"><ul class=\"list\"></ul></main>\n\
         .page { display: block; }\n\
         .list { @each($t as $x) { <li class=\"row\">`$x.fields.school`|`$x.fields.scenario`</li> } }\n",
    );
    assert!(ok, "fields must compile:\n{log}");
    assert!(
        html.contains("minimalism") && html.contains("hero"),
        "`/// school:` and `/// scenario:` must reach `$x.fields` — all 11 \
         showcase patterns already carry 6 consistent keys, so this needs no \
         `.st` edits to light up:\n{html}"
    );
}

/// THE REGRESSION GUARD. `@data forms` becomes sugar for `entity form`, and
/// the predicate it shares with score/drivers/form-compiler is generalized.
/// The brand catalog is the sharpest proof the rows did not shift: 46 forms,
/// split 12 project / 34 stdlib.
#[test]
fn forms_rows_are_unchanged() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(root.join("demos/brand-catalog"))
        .current_dir(&root)
        .output()
        .expect("build catalog");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success() && !log.contains("Export failed"),
        "the brand catalog must still build:\n{log}"
    );
    let html = std::fs::read_to_string(root.join("demos/brand-catalog/dist/index.html"))
        .expect("catalog html");
    // Measure the BROWSE RAIL specifically, not the whole document: the
    // per-kind stages re-render the same declarations (92 `.fcard`s in total),
    // and one stray `>stdlib<` lives in the page chrome. Scoping to the rail
    // means this asserts the row SET, which is the claim — not an incidental
    // document-wide substring count.
    let rail = {
        let start = html
            .find("class=\"cards cards--all\"")
            .expect("the browse rail must exist");
        &html[start..]
    };
    let rail_end = rail.find("</section>").unwrap_or(rail.len());
    let rail = &rail[..rail_end];

    let cards = rail.matches("class=\"fcard\"").count();
    assert_eq!(
        cards, 46,
        "the catalog's form rows must be UNCHANGED by generalizing the \
         predicate. `is_form_declaring_macro` is shared with score, drivers and \
         the form compiler, so a careless widening would show up here first."
    );
    // The browse rail carries one origin label per row; the split is the
    // sharpest single number proving the row SET did not shift.
    let project = rail.matches(">project<").count();
    let stdlib = rail.matches(">stdlib<").count();
    assert_eq!(
        (project, stdlib),
        (12, 34),
        "the origin split must be unchanged (12 project / 34 stdlib) — \
         verified live in the browser before this change"
    );
}

/// An unknown entity must REFUSE, not return an empty list. A silent empty is
/// the banned failure class (BUG-229/243 lineage) and would read exactly like
/// "this project has no templates".
#[test]
fn an_unknown_entity_is_refused() {
    let (ok, log, _) = build_page(
        "unknown-entity",
        "@data declarations $x from wibble ;\n\n\
         <main class=\"page\">y</main>\n\
         .page { display: block; }\n",
    );
    assert!(
        !ok,
        "an unknown entity must refuse — returning an empty list would be \
         indistinguishable from a project that genuinely has none:\n{log}"
    );
    assert!(
        log.contains("wibble"),
        "the diagnostic must name the unknown entity:\n{log}"
    );
}

/// Every showcase pattern must yield a non-empty `school`. This is the guard
/// for the accepted risk in choosing doc-comment fields over a declared
/// metadata block: a typo'd key (`schoool:`) lands in the map unnoticed and
/// the gallery's rail silently empties.
#[test]
fn every_showcase_pattern_declares_its_school() {
    let (ok, log, html) = build_page(
        "all-schools",
        "@import \"../../stdlib/showcases/hero/minimal.st\"\n\
         @import \"../../stdlib/showcases/hero/editorial.st\"\n\
         @import \"../../stdlib/showcases/hero/maximal.st\"\n\
         @import \"../../stdlib/showcases/hero/architecture.st\"\n\
         @import \"../../stdlib/showcases/hero/motion.st\"\n\
         @import \"../../stdlib/showcases/hero/eastern.st\"\n\
         @import \"../../stdlib/showcases/nav/minimal.st\"\n\
         @import \"../../stdlib/showcases/cta/minimal.st\"\n\
         @import \"../../stdlib/showcases/footer/minimal.st\"\n\
         @import \"../../stdlib/showcases/features-grid/minimal.st\"\n\n\
         @data declarations $t from template ;\n\n\
         <main class=\"page\"><ul class=\"list\"></ul></main>\n\
         .page { display: block; }\n\
         .list { @each($t as $x) { <li class=\"row\">[`$x.fields.school`]</li> } }\n",
    );
    assert!(ok, "build:\n{log}");
    // Count the rows FIRST. Without this the `[]` check passes trivially when
    // no rows exist at all — which is exactly how it read before the fix, and
    // a gate that passes on an empty page is not a gate.
    let rows = html.matches("class=\"row\"").count();
    assert_eq!(
        rows, 10,
        "all ten imported patterns must produce a row — otherwise the \
         empty-school check below is vacuous:\n{html}"
    );
    let empty = html.matches("[]").count();
    assert_eq!(
        empty, 0,
        "every showcase template must carry a school — an empty one means a \
         mistyped `/// school:` key, which is exactly the silent failure the \
         doc-field approach risks:\n{html}"
    );
}
