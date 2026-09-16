//! Guard fan-out for the DATA family's optional captures (BUG-264).
//!
//! Every optional directive capture in stdlib/macros/{data-kind,data,score}.st
//! is either guarded by an `optional-clause-error` sibling (this file's neg/pos
//! pairs) or documented as having no plausible near-miss (see the sibling
//! comments in the stdlib files and the REPORT section below).
//!
//! MECHANISM (per file): each error sibling is declared AFTER its productive
//! form, so at equal specificity the productive form is tried first. For a
//! VALID capture the productive form matches CLEANLY and wins immediately (the
//! sibling never fires); for a NEAR-MISS the productive form matches LENIENTLY
//! (drops the clause it could not bind) and the clean-match ranking defers it
//! below the sibling, which then fires E0955 with its data-supplied message.
//! This is the discrimination every test here pins: the near-miss MUST be
//! refused loudly, and the valid + omitted forms MUST stay green.
//!
//! REPORT — optionals with no guard (documented no-plausible-near-miss):
//!   * data-kind.st query/fetch/collection/subscribe-seeded `initial:`,
//!     query `where:/sort:/map:/reduce:` — `balanced(';')` captures any token
//!       run up to the `;`, so there is no value that would drop silently.
//!   * data-kind.st query `dir:` / `limit:` — the property-style BODY parser
//!       binds ANY value as the declared capture type (Ident/Number wrap
//!       without validation), so `dir: 5` / `limit: 6s` are ACCEPTED as
//!       garbage, not silently DROPPED. The sibling mechanism targets drops;
//!       rejecting these needs a compiler strict-reject for Ident/Number
//!       captures, which is beyond stdlib-guard scope (verified empirically:
//!       a candidate sibling does not fire on `limit: 6s`).
//!   * data-kind.st `@data subscribe` / `@data subscribe-seeded` type slot —
//!       the BUG-234 keyword commitment already raises E0945 ("malformed
//!       @data subscribe") loudly; no sibling needed.
//!   * data-kind.st data-signal/data-stream `policy` / `optimistic` — `policy 5`
//!       already fails LOUDLY (E0946, the body grammar refuses the number);
//!       `optimistic { … }` is a `balanced('}')` capture with no silent-drop.
//!   * data-kind.st data-fetch-opts `refresh:` — already guarded by
//!       `data-fetch-opts-invalid-refresh` (landed with BUG-264).
//!
//! DISCRIMINATION (each file): with a sibling removed, its near-miss compiles
//! SILENTLY (verified before the siblings landed: every near-miss below was
//! green); with it, E0955 fires. The pos pairs prove the sibling does not steal
//! valid or omitted optionals.

use std::process::Command;

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn check(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .current_dir(REPO_ROOT)
        .args(["check", entry.to_str().expect("utf-8 path")])
        .output()
        .expect("spacetime check runs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

const TYPE_MSG: &str = "a bare number is not a valid type";

// ─── stdlib/macros/data-kind.st — typeref? slot guards ───────────────────────

#[test]
fn data_inline_type_slot_bare_number_is_refused_not_dropped() {
    // The productive `data-inline` matches `@data inline $x 5 : 5` LENIENTLY
    // (drops the bare number it cannot bind as `$type:typeref?`); the
    // `data-inline-invalid-type` sibling must win and emit E0955.
    let text = check(
        r#"@version 2026-06-09;
@data inline $x 5 : 5;
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "a bare number in the type slot must be refused with E0955; otherwise the \
         lenient productive match silently drops it. Got:\n{text}"
    );
}

#[test]
fn data_inline_type_slot_valid_and_omitted_compile() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $x number : 5;
@data inline $y : 6;
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a real type name or an omitted type must compile clean; the sibling must not \
         steal a valid or omitted type slot. Got:\n{text}"
    );
}

#[test]
fn data_fetch_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data fetch $x 5 : "/api/products";
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "bodyless @data fetch must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_fetch_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data fetch $x Product[] : "/api/products";
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid type name on bodyless @data fetch must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_derive_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $a number : 1;
@data derive $x 5 : $a + 1;
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "data-derive must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_derive_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $a number : 1;
@data derive $x number : $a + 1;
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid type name on data-derive must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_fold_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $a number[] : [];
@data fold $x 5 from $a : (acc, item) -> acc;
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "data-fold must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_fold_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $a number[] : [];
@data fold $x number from $a : (acc, item) -> acc;
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid type name on data-fold must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_query_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $a number[] : [];
@data query $x 5 from $a {
  where: $a > 0;
}
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "data-query must refuse a bare-number type even with a populated body. Got:\n{text}"
    );
}

#[test]
fn data_query_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $a number[] : [];
@data query $x Product[] from $a {
  where: $a > 0;
  dir: asc;
  limit: 6;
}
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid typed data-query must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_fetch_opts_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data fetch $x 5 : "/api/products" {
  refresh: 5s;
}
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "the body @data fetch (data-fetch-opts) must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_fetch_opts_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data fetch $x Product[] : "/api/products" {
  refresh: 5s;
}
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid typed body @data fetch must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_collection_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data collection $x 5 from "/data/products.json" via dev-ws {
  initial: [];
}
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "data-collection must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_collection_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data collection $x Product[] from "/data/products.json" via dev-ws {
  initial: [];
}
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid typed data-collection must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_source_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $url string : "/x";
@data source $x 5 from $url;
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "data-source must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_source_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $url string : "/x";
@data source $x Product[] from $url;
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid typed data-source must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_stream_event_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@data stream $x 5 from "http://sse.invalid";
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "bodyless @data stream (event source) must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_stream_event_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@data stream $x Product[] from "http://sse.invalid";
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid typed bodyless @data stream must compile clean. Got:\n{text}"
    );
}

#[test]
fn data_live_stream_type_slot_bare_number_is_refused_not_dropped() {
    let text = check(
        r#"@version 2026-06-09;
@host $h : ws /api/socket;
@data stream $x 5 from $h {
  receive to R {
    1 => Ok($.body);
  }
}
"#,
    );
    assert!(
        text.contains("E0955") && text.contains(TYPE_MSG),
        "the body @data stream (live-stream) must refuse a bare-number type. Got:\n{text}"
    );
}

#[test]
fn data_live_stream_type_slot_valid_compiles() {
    let text = check(
        r#"@version 2026-06-09;
@host $h : ws /api/socket;
@data stream $x Product[] from $h {
  receive to R {
    1 => Ok($.body);
  }
}
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid typed body @data stream must compile clean. Got:\n{text}"
    );
}

// ─── stdlib/macros/data.st — cycle field? guard ──────────────────────────────

#[test]
fn cycle_field_wrong_kind_is_refused_not_dropped() {
    // `field: $field` (a sigil where a quoted string is expected) is a near-miss
    // for the optional `$field:string?` capture; `cycle-invalid-field` must win.
    let text = check(
        r#"@version 2026-06-09;
@data inline $terms string[] : [];
.hero {
  @cycle(source: terms, interval: 3000ms, field: $field)
}
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("`field:` must be a quoted string"),
        "a non-string cycle field must be refused with E0955. Got:\n{text}"
    );
}

#[test]
fn cycle_field_valid_and_omitted_compile() {
    let text = check(
        r#"@version 2026-06-09;
@data inline $terms string[] : [];
.hero {
  @cycle(source: terms, interval: 3000ms, field: "text");
  @cycle(source: terms, interval: 3000ms);
}
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a quoted field or an omitted field must compile clean; the sibling must not \
         steal a valid cycle. Got:\n{text}"
    );
}

// ─── stdlib/macros/score.st — as &name guard ─────────────────────────────────

#[test]
fn score_as_wrong_sigil_is_refused_not_dropped() {
    // `as $film` (a `$`-sigil where `&film` is expected) fails the optional
    // `$as:score_as_name?` capture; `score-invalid-as` must win instead of the
    // alias vanishing with no diagnostic (the failure mode score.st documents).
    let text = check(
        r#"@version 2026-06-09;
@score &.time(60s) as $film {
  &title at 2s for 3s;
}
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("`as` must name a timeline with `&`"),
        "a wrong-sigil score alias must be refused with E0955. Got:\n{text}"
    );
}

#[test]
fn score_as_valid_and_omitted_compile() {
    let text = check(
        r#"@version 2026-06-09;
@score &.time(60s) as &film {
  &title at 2s for 3s;
}
@score &.time(30s) {
  &title at 2s for 3s;
}
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid `&` alias or an omitted alias must compile clean; the sibling must not \
         steal a valid score. Got:\n{text}"
    );
}
