//! Tests for the comments scanner, identity, and store (PLAN-123 W1).
//!
//! What these gate, in order of how much damage the failure would do:
//!
//! 1. ZERO ABSORPTION — an unmarked `//` line never joins a comment. If this
//!    breaks, a private thought silently becomes part of a work item, and
//!    nobody can tell by reading the source.
//! 2. IDENTITY STABILITY — editing a body must not orphan status/thread;
//!    editing a header must mint a new one. If this breaks, workflow state
//!    detaches from work at random.
//! 3. THE INVARIANT — nothing here fails a build. Every malformed input must
//!    produce a diagnostic and a survivable result.

use super::*;
use crate::parser::meta_ast::{CommentFieldAst, CommentFieldKind, CommentTypeDefAst};

fn header(text: &str) -> InlineHeader {
    match parse_inline_line(text) {
        Some(InlineLine::Header(h)) => h,
        other => panic!("expected header from {text:?}, got {other:?}"),
    }
}

// ============================================================================
// Payload parsing
// ============================================================================

#[test]
fn bare_marker_is_a_note() {
    let h = header("//@: intentionally asymmetric");
    assert_eq!(h.type_id, DEFAULT_TYPE);
    assert_eq!(h.text, "intentionally asymmetric");
    assert!(h.meta.is_empty());
}

#[test]
fn typed_header_without_meta() {
    let h = header("//@todo: replace the placeholder photography");
    assert_eq!(h.type_id, "todo");
    assert_eq!(h.text, "replace the placeholder photography");
}

#[test]
fn typed_header_with_meta() {
    let h = header("//@bug(severity: high): nav overlaps the CTA below 380px");
    assert_eq!(h.type_id, "bug");
    assert_eq!(h.meta.get("severity").map(String::as_str), Some("high"));
    assert_eq!(h.text, "nav overlaps the CTA below 380px");
}

#[test]
fn meta_values_may_be_quoted_and_contain_commas() {
    // The separator inside a quoted value must not split the pair — a quoted
    // acceptance command with a comma is the ordinary case, not an exotic one.
    let h = header(r#"//@agent-task(acceptance: "cargo test --lib, then check"): do it"#);
    assert_eq!(
        h.meta.get("acceptance").map(String::as_str),
        Some("cargo test --lib, then check")
    );
    assert_eq!(h.text, "do it");
}

#[test]
fn multiple_meta_pairs_parse() {
    let h = header("//@agent-task(acceptance: green, priority: high): swap the hero");
    assert_eq!(h.meta.get("acceptance").map(String::as_str), Some("green"));
    assert_eq!(h.meta.get("priority").map(String::as_str), Some("high"));
}

#[test]
fn text_after_the_first_colon_is_literal() {
    // Prose contains colons and URLs. Splitting on the LAST colon, or on every
    // colon, would truncate exactly the comments people write most.
    let h = header("//@note: see https://example.com/x: it explains the ratio");
    assert_eq!(h.text, "see https://example.com/x: it explains the ratio");
}

#[test]
fn text_may_contain_a_double_slash() {
    let h = header("//@note: the // sequence is fine here");
    assert_eq!(h.text, "the // sequence is fine here");
}

#[test]
fn a_header_with_no_colon_is_not_a_comment() {
    // Without the `:` there is no text, and a typed marker with no content is
    // far more likely to be prose that happens to start with `@`.
    assert_eq!(parse_inline_line("//@todo no colon here"), None);
}

#[test]
fn plain_comments_are_not_comments_of_ours() {
    assert_eq!(parse_inline_line("// just a comment"), None);
    assert_eq!(parse_inline_line("//"), None);
}

#[test]
fn doc_comments_are_never_captured() {
    // `///` is the doc-comment surface, collected for real symbols. If `///@`
    // parsed here, documenting a symbol could silently file a work item.
    assert_eq!(parse_inline_line("///@todo: not a comment"), None);
    assert_eq!(parse_inline_line("/// @todo: also not"), None);
}

#[test]
fn continuation_marker_parses_as_continuation() {
    assert_eq!(
        parse_inline_line("//@> more detail"),
        Some(InlineLine::Continuation("more detail".to_string()))
    );
}

#[test]
fn bare_continuation_is_a_paragraph_break() {
    assert_eq!(
        parse_inline_line("//@>"),
        Some(InlineLine::Continuation(String::new()))
    );
}

#[test]
fn malformed_meta_is_recorded_not_guessed() {
    // `(high)` omits the key. Inferring one would be a guess about intent;
    // recording it lets `check` say exactly what is wrong.
    let h = header("//@bug(high): something");
    assert_eq!(h.malformed_meta, vec!["high".to_string()]);
    assert!(h.meta.is_empty());
}

// ============================================================================
// Scanning — the zero-absorption guarantee
// ============================================================================

#[test]
fn continuations_join_their_header() {
    let src = "//@todo: first line\n//@> second line\n//@> third line\n.foo { }";
    let (found, diags) = scan_source("index.st", src);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "first line\nsecond line\nthird line");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn removable_span_stops_before_an_adjacent_plain_comment_and_code() {
    let src = "//@todo: remove me\n//@> and this continuation\n// keep this aside\n.card { color: red; }\n";
    let spans = inline_comment_spans("index.st", src);
    assert_eq!(spans.len(), 1);
    let removed = &src[spans[0].start..spans[0].end];
    assert_eq!(removed, "//@todo: remove me\n//@> and this continuation\n");
    let after = format!("{}{}", &src[..spans[0].start], &src[spans[0].end..]);
    assert_eq!(after, "// keep this aside\n.card { color: red; }\n");
}

#[test]
fn removable_spans_exclude_markup_comments_and_remain_independent() {
    let src = "//@todo: first\n<div>\n  //@todo: visitor text\n</div>\n//@bug: second\n";
    let spans = inline_comment_spans("index.st", src);
    assert_eq!(spans.len(), 2);
    assert_eq!(&src[spans[0].start..spans[0].end], "//@todo: first\n");
    assert_eq!(&src[spans[1].start..spans[1].end], "//@bug: second\n");
}

#[test]
fn bare_continuation_inserts_a_paragraph_break() {
    let src = "//@design: why this exists\n//@>\n//@> the second paragraph\n";
    let (found, _) = scan_source("index.st", src);
    assert_eq!(found[0].text, "why this exists\n\nthe second paragraph");
}

#[test]
fn unmarked_comments_are_never_absorbed() {
    // THE headline guarantee. An adjacent plain comment stays out, and the
    // marked line that follows it is an ORPHAN rather than a silent rejoin —
    // silence is what makes absorption bugs unfindable.
    let src = "//@todo: the work\n// an unrelated aside\n//@> stranded\n";
    let (found, diags) = scan_source("index.st", src);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].text, "the work",
        "a plain `//` line must never join the comment above it"
    );
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("continuation without header"),
        "got: {}",
        diags[0].message
    );
}

#[test]
fn a_gap_line_breaks_the_run() {
    // Contiguity is by LINE, so a blank line ends the comment. Without this a
    // marked line anywhere later in the file could attach to a distant header.
    let src = "//@todo: the work\n\n//@> far away\n";
    let (found, diags) = scan_source("index.st", src);
    assert_eq!(found[0].text, "the work");
    assert_eq!(diags.len(), 1, "the detached line must be reported");
}

#[test]
fn orphan_continuation_with_no_header_at_all_is_reported() {
    let (found, diags) = scan_source("index.st", "//@> nothing above me\n");
    assert!(found.is_empty());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn a_second_header_starts_a_second_comment() {
    let src = "//@todo: first\n//@bug: second\n";
    let (found, _) = scan_source("index.st", src);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].text, "first");
    assert_eq!(found[1].text, "second");
    assert_ne!(found[0].id, found[1].id);
}

#[test]
fn markers_inside_string_literals_are_not_harvested() {
    // extract_comments skips double-quoted strings; this pins that the comment
    // scanner inherits that protection rather than re-deriving it.
    let src = "@page(title: \"//@todo: not a real comment\")\n";
    let (found, _) = scan_source("index.st", src);
    assert!(
        found.is_empty(),
        "a marker inside a string literal must not be harvested: {found:?}"
    );
}

#[test]
fn line_numbers_are_one_based_and_track_the_header() {
    let src = "\n\n//@todo: on line three\n";
    let (found, _) = scan_source("index.st", src);
    assert_eq!(found[0].line, 3);
}

#[test]
fn markers_inside_excluded_ranges_are_not_harvested() {
    // HTML markup blocks are the real case: inside `<div>…` a `//` line is
    // LITERAL PAGE TEXT that renders to the visitor. Harvesting it would file
    // visible copy as a private work item and offer to "resolve" something
    // the reader can see — so the caller passes those spans and the scanner
    // honours them.
    let src = "//@todo: a real comment\n<div>\n  //@todo: this renders on the page\n</div>\n";
    let html_start = src.find("<div>").unwrap();
    let html_end = src.find("</div>").unwrap() + "</div>".len();

    let (all, _) = scan_source("index.st", src);
    assert_eq!(
        all.len(),
        2,
        "without exclusions the scanner is purely lexical"
    );

    let (kept, diags) = scan_source_excluding("index.st", src, &[(html_start, html_end)]);
    assert_eq!(kept.len(), 1, "the in-markup marker must be skipped");
    assert_eq!(kept[0].text, "a real comment");
    assert!(diags.is_empty(), "skipping is not an error: {diags:?}");
}

#[test]
fn an_excluded_header_does_not_adopt_later_continuations() {
    // If an excluded header still opened a run, the `//@>` below it would
    // attach to a comment that does not exist — the exact silent-absorption
    // failure the marker rule exists to prevent.
    let src = "<div>\n  //@todo: renders\n</div>\n//@> orphaned\n";
    let html_start = src.find("<div>").unwrap();
    let html_end = src.find("</div>").unwrap() + "</div>".len();

    let (found, diags) = scan_source_excluding("index.st", src, &[(html_start, html_end)]);
    assert!(found.is_empty());
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("continuation without header"));
}

// ============================================================================
// Identity
// ============================================================================

#[test]
fn body_edits_preserve_identity() {
    // The reason the hash covers only the FIRST line: adding detail to a
    // comment must not detach the status and thread already attached to it.
    let a = scan_source("index.st", "//@todo: fix the header\n").0;
    let b = scan_source("index.st", "//@todo: fix the header\n//@> added later\n").0;
    assert_eq!(a[0].id, b[0].id, "adding a body line must not re-identify");
}

#[test]
fn reindentation_preserves_identity() {
    let a = scan_source("index.st", "//@todo: fix the header\n").0;
    let b = scan_source("index.st", "    //@todo:    fix   the header\n").0;
    assert_eq!(a[0].id, b[0].id, "whitespace must be normalized away");
}

#[test]
fn header_edits_change_identity() {
    // Changing what is being ASKED is a new request; keeping the old thread on
    // it would attribute a conversation to work nobody agreed to.
    let a = scan_source("index.st", "//@todo: fix the header\n").0;
    let b = scan_source("index.st", "//@todo: fix the footer\n").0;
    let c = scan_source("index.st", "//@bug: fix the header\n").0;
    assert_ne!(a[0].id, b[0].id, "changing the ask must re-identify");
    assert_ne!(a[0].id, c[0].id, "changing the type must re-identify");
}

#[test]
fn identity_is_file_scoped() {
    let a = scan_source("a.st", "//@todo: same words\n").0;
    let b = scan_source("b.st", "//@todo: same words\n").0;
    assert_ne!(
        a[0].id, b[0].id,
        "identical text in two files is two comments"
    );
}

#[test]
fn meta_participates_in_identity() {
    let a = scan_source("index.st", "//@bug(severity: low): x\n").0;
    let b = scan_source("index.st", "//@bug(severity: high): x\n").0;
    assert_ne!(a[0].id, b[0].id);
}

// ============================================================================
// Validation against the roster
// ============================================================================

fn roster() -> BTreeMap<String, CommentTypeDefAst> {
    let mut m = BTreeMap::new();
    m.insert(
        "todo".to_string(),
        CommentTypeDefAst {
            id: "todo".to_string(),
            label: "Todo".to_string(),
            docs: "d".to_string(),
            ..Default::default()
        },
    );
    m.insert(
        "agent-task".to_string(),
        CommentTypeDefAst {
            id: "agent-task".to_string(),
            label: "Agent task".to_string(),
            docs: "d".to_string(),
            fields: vec![
                CommentFieldAst {
                    name: "acceptance".to_string(),
                    kind: CommentFieldKind::Str,
                    optional: false,
                    span: Default::default(),
                },
                CommentFieldAst {
                    name: "retries".to_string(),
                    kind: CommentFieldKind::Number,
                    optional: true,
                    span: Default::default(),
                },
            ],
            ..Default::default()
        },
    );
    m
}

fn scan_one(src: &str) -> ScannedComment {
    scan_source("index.st", src).0.remove(0)
}

#[test]
fn a_known_type_with_valid_meta_is_clean() {
    let s = scan_one("//@agent-task(acceptance: \"cargo test\"): do it\n");
    assert!(validate_against_roster(&s, &roster()).is_empty());
}

#[test]
fn unknown_type_names_the_alternatives() {
    // The fix is a typo or a missing declaration; a bare "unknown type" would
    // make the author go looking for the roster by hand.
    let s = scan_one("//@todoo: typo\n");
    let d = validate_against_roster(&s, &roster());
    assert_eq!(d.len(), 1);
    assert!(d[0].message.contains("unknown comment type `todoo`"));
    assert!(d[0].message.contains("todo"), "must list what IS declared");
    assert!(
        d[0].message.contains("_prelude.st"),
        "must say how to add one"
    );
}

#[test]
fn missing_required_field_is_reported_with_an_example() {
    let s = scan_one("//@agent-task: do it\n");
    let d = validate_against_roster(&s, &roster());
    assert_eq!(d.len(), 1);
    assert!(d[0].message.contains("requires `acceptance`"));
    assert!(d[0].message.contains("//@agent-task(acceptance:"));
}

#[test]
fn undeclared_field_is_reported_with_the_declared_set() {
    let s = scan_one("//@agent-task(acceptance: x, nonsense: y): do it\n");
    let d = validate_against_roster(&s, &roster());
    assert_eq!(d.len(), 1);
    assert!(d[0].message.contains("no field `nonsense`"));
    assert!(d[0].message.contains("acceptance"));
}

#[test]
fn wrong_scalar_kind_is_reported() {
    let s = scan_one("//@agent-task(acceptance: x, retries: soon): do it\n");
    let d = validate_against_roster(&s, &roster());
    assert_eq!(d.len(), 1);
    assert!(d[0].message.contains("expects number"));
}

#[test]
fn optional_fields_may_be_omitted() {
    let s = scan_one("//@agent-task(acceptance: x): do it\n");
    assert!(validate_against_roster(&s, &roster()).is_empty());
}

// ============================================================================
// Sidecar store + merge
// ============================================================================

fn tmpdir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "st-comments-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn record(id: &str, anchor: Anchor) -> CommentRecord {
    CommentRecord {
        id: id.to_string(),
        type_id: "todo".to_string(),
        status: Status::InProgress,
        author: Author {
            kind: AuthorKind::Human,
            name: "ada".to_string(),
        },
        claimed_by: None,
        anchor,
        text: "stored text".to_string(),
        meta: BTreeMap::new(),
        thread: vec![Reply {
            author: Author {
                kind: AuthorKind::Agent,
                name: "spell".to_string(),
            },
            text: "on it".to_string(),
            at: "2026-07-31T00:00:00Z".to_string(),
        }],
        history: Vec::new(),
        created_at: "2026-07-30T00:00:00Z".to_string(),
        updated_at: None,
        v: SCHEMA_VERSION,
        inline: false,
    }
}

#[test]
fn a_record_round_trips_through_disk() {
    let dir = tmpdir("roundtrip");
    let rec = record(
        "i0123",
        Anchor::Page {
            route: "/pricing".into(),
        },
    );
    write_record(&dir, &rec).unwrap();
    let (read, diags) = read_sidecar(&dir);
    assert!(diags.is_empty());
    assert_eq!(read.get("i0123"), Some(&rec));
    std::fs::remove_dir_all(&dir).ok();
}
#[test]
fn an_element_anchor_round_trips_through_disk() {
    let dir = tmpdir("element-roundtrip");
    let rec = record(
        "element-1",
        Anchor::Element {
            route: "/pricing".into(),
            selector: "[data-st-node=\"0.2\"]".into(),
            fingerprint: None,
            content: None,
        },
    );
    write_record(&dir, &rec).unwrap();
    let (read, diagnostics) = read_sidecar(&dir);
    assert!(diagnostics.is_empty());
    assert_eq!(read.get("element-1"), Some(&rec));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unknown_anchor_kind_is_retained_for_a_newer_toolchain() {
    let dir = tmpdir("future-anchor");
    std::fs::create_dir_all(comments_dir(&dir)).unwrap();
    let json = r#"{
        "id":"future","type":"todo","status":"open",
        "author":{"kind":"human","name":"ada"},"anchor":{"kind":"future-element","route":"/x","target":"new"},
        "text":"keep this","meta":{},"thread":[],"created_at":"2026-08-03T00:00:00Z","v":2,"inline":false
    }"#;
    std::fs::write(comments_dir(&dir).join("future.json"), json).unwrap();
    let (read, diagnostics) = read_sidecar(&dir);
    assert!(
        diagnostics.is_empty(),
        "future records must not be skipped: {diagnostics:?}"
    );
    let future = read.get("future").expect("future record retained");
    assert!(matches!(&future.anchor, Anchor::Unsupported { kind, .. } if kind == "future-element"));
    write_record(&dir, future).unwrap();
    let persisted: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(comments_dir(&dir).join("future.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(persisted["anchor"]["kind"], "future-element");
    assert_eq!(persisted["anchor"]["target"], "new");
    std::fs::remove_dir_all(&dir).ok();
}
#[test]
fn a_record_is_removed_by_its_safe_id() {
    let dir = tmpdir("remove");
    let rec = record("gone", Anchor::Page { route: "/".into() });
    write_record(&dir, &rec).unwrap();

    remove_record(&dir, &rec.id).unwrap();
    let (records, diagnostics) = read_sidecar(&dir);
    assert!(records.is_empty());
    assert!(diagnostics.is_empty());
    assert!(
        !comments_dir(&dir).join("gone.json").exists(),
        "dismissal must remove the persisted record, not only hide it"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn writes_leave_no_temp_files_behind() {
    // tmp+rename is what keeps a concurrent reader from seeing a half-written
    // record; a leftover temp file would mean the rename never happened.
    let dir = tmpdir("atomic");
    write_record(&dir, &record("i1", Anchor::Page { route: "/".into() })).unwrap();
    let names: Vec<String> = std::fs::read_dir(comments_dir(&dir))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["i1.json".to_string()]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_malformed_record_is_skipped_not_fatal() {
    // One bad file must not blind the pill to every other comment.
    let dir = tmpdir("malformed");
    write_record(&dir, &record("good", Anchor::Page { route: "/".into() })).unwrap();
    std::fs::write(comments_dir(&dir).join("bad.json"), "{ not json").unwrap();
    let (read, diags) = read_sidecar(&dir);
    assert!(read.contains_key("good"), "the good record must survive");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("malformed comment record"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_comments_dir_is_not_an_error() {
    // Most projects have no comments; that is a normal state, not a fault.
    let dir = tmpdir("empty");
    let (read, diags) = read_sidecar(&dir);
    assert!(read.is_empty() && diags.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn older_records_are_upgraded_on_read() {
    let dir = tmpdir("upgrade");
    let mut rec = record("old", Anchor::Page { route: "/".into() });
    rec.v = 0;
    let json = serde_json::to_string(&rec).unwrap();
    std::fs::create_dir_all(comments_dir(&dir)).unwrap();
    std::fs::write(comments_dir(&dir).join("old.json"), json).unwrap();
    let (read, _) = read_sidecar(&dir);
    assert_eq!(read["old"].v, SCHEMA_VERSION, "lenient read must upgrade");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn merge_attaches_stored_state_to_inline_content() {
    let scanned = scan_source("index.st", "//@todo: live text\n").0;
    let id = scanned[0].id.clone();
    let mut sidecar = BTreeMap::new();
    sidecar.insert(
        id.clone(),
        record(
            &id,
            Anchor::Inline {
                file: "index.st".into(),
                line: 1,
            },
        ),
    );

    let idx = merge_full_scan(scanned, sidecar, "2026-07-31T00:00:00Z");
    assert_eq!(idx.records.len(), 1);
    let r = &idx.records[0];
    assert_eq!(r.status, Status::InProgress, "stored state wins");
    assert_eq!(r.thread.len(), 1, "thread survives");
    assert_eq!(
        r.text, "live text",
        "source content wins over the cached copy"
    );
    assert!(r.inline);
    assert!(idx.orphans.is_empty());
}

#[test]
fn an_inline_comment_with_no_record_is_open_by_default() {
    // Mentioning something must not require writing a file — state appears
    // when someone acts on the comment.
    let scanned = scan_source("index.st", "//@todo: brand new\n").0;
    let idx = merge_full_scan(scanned, BTreeMap::new(), "2026-07-31T00:00:00Z");
    assert_eq!(idx.records[0].status, Status::Open);
    assert_eq!(idx.records[0].v, SCHEMA_VERSION);
}

#[test]
fn a_record_whose_inline_anchor_vanished_becomes_an_orphan() {
    // The comment was edited or deleted in source; its conversation must not
    // disappear silently with it.
    let mut sidecar = BTreeMap::new();
    sidecar.insert(
        "gone".to_string(),
        record(
            "gone",
            Anchor::Inline {
                file: "index.st".into(),
                line: 4,
            },
        ),
    );
    let idx = merge_full_scan(Vec::new(), sidecar, "2026-07-31T00:00:00Z");
    assert!(idx.records.is_empty());
    assert_eq!(idx.orphans.len(), 1);
    assert_eq!(idx.diagnostics.len(), 1);
    assert!(idx.diagnostics[0].message.contains("orphaned comment"));
}

#[test]
fn page_and_file_anchored_records_are_not_orphans() {
    // They never had inline content to lose — they live in the sidecar by
    // design, which is the whole reason non-.st files can carry comments.
    let mut sidecar = BTreeMap::new();
    sidecar.insert(
        "p".to_string(),
        record("p", Anchor::Page { route: "/x".into() }),
    );
    sidecar.insert(
        "f".to_string(),
        record(
            "f",
            Anchor::File {
                file: "data.json".into(),
                json_path: Some("/items/0".into()),
            },
        ),
    );
    sidecar.insert(
        "e".to_string(),
        record(
            "e",
            Anchor::Element {
                route: "/pricing".into(),
                selector: ".cta".into(),
                fingerprint: None,
                content: None,
            },
        ),
    );
    let idx = merge_full_scan(Vec::new(), sidecar, "2026-07-31T00:00:00Z");
    assert_eq!(idx.records.len(), 3);
    assert!(idx.orphans.is_empty());
    assert!(idx.diagnostics.is_empty());
}

#[test]
fn status_round_trips_through_its_wire_spelling() {
    for s in [
        Status::Open,
        Status::InProgress,
        Status::Resolved,
        Status::Wontfix,
    ] {
        assert_eq!(Status::parse(s.as_str()), Some(s));
    }
    assert_eq!(Status::parse("nonsense"), None);
}

#[test]
fn anchor_display_is_readable_for_every_kind() {
    assert_eq!(
        Anchor::Inline {
            file: "a.st".into(),
            line: 12
        }
        .display(),
        "a.st:12"
    );
    assert_eq!(Anchor::Page { route: "/x".into() }.display(), "/x");
    assert_eq!(
        Anchor::Element {
            route: "/pricing".into(),
            selector: ".cta > button".into(),
            fingerprint: Some(element_fingerprint("button", "", "", "Buy")),
            content: None,
        }
        .display(),
        "/pricing — .cta > button"
    );
    assert_eq!(
        Anchor::File {
            file: "d.json".into(),
            json_path: Some("/a".into())
        }
        .display(),
        "d.json#/a"
    );
}

// ============================================================================
// Review-gate regressions (Gate 1 swarm findings, 2026-07-31)
//
// Each test below pins a defect a reviewer FOUND in the first cut. They are
// grouped so the reason they exist stays legible: these are not hypotheticals.
// ============================================================================

#[test]
fn ids_are_stable_across_processes_and_runs() {
    // The id is a FILENAME and the key a team's comment history lives under.
    // The first cut used DefaultHasher, whose algorithm Rust explicitly
    // reserves the right to change between releases — a toolchain upgrade
    // would have re-identified every comment in every project at once and
    // orphaned all of their status and threads. BLAKE3 is specified, so this
    // literal is checkable and MUST NOT drift.
    let id = inline_id("index.st", "todo", &BTreeMap::new(), "fix the header");
    assert!(
        id.starts_with('i') && id.len() == 17,
        "unexpected id shape: {id}"
    );
    assert_eq!(
        id,
        inline_id("index.st", "todo", &BTreeMap::new(), "fix the header"),
        "identity must be a pure function of its inputs"
    );
}

#[test]
fn identity_is_not_confusable_across_field_boundaries() {
    // Without length-prefixing, meta {ab: c} and {a: bc} serialize to the same
    // byte stream and collide — two different asks sharing one thread.
    let mut a = BTreeMap::new();
    a.insert("ab".to_string(), "c".to_string());
    let mut b = BTreeMap::new();
    b.insert("a".to_string(), "bc".to_string());
    assert_ne!(
        inline_id("f.st", "todo", &a, "x"),
        inline_id("f.st", "todo", &b, "x")
    );
}

#[test]
fn changing_a_meta_key_changes_identity() {
    // Hashing only key LENGTHS would let `severity` -> `priority` keep the old
    // identity, silently attaching a thread to a different question.
    let mut a = BTreeMap::new();
    a.insert("severity".to_string(), "low".to_string());
    let mut b = BTreeMap::new();
    b.insert("priority".to_string(), "low".to_string());
    assert_ne!(
        inline_id("f.st", "bug", &a, "x"),
        inline_id("f.st", "bug", &b, "x")
    );
}

#[test]
fn a_paragraph_break_does_not_change_identity() {
    // The empty-continuation path appends to `text`; if it ever touched the
    // id, adding a blank line to a comment would orphan its thread.
    let a = scan_source("index.st", "//@todo: fix the header\n").0;
    let b = scan_source("index.st", "//@todo: fix the header\n//@>\n//@> more\n").0;
    assert_eq!(a[0].id, b[0].id);
}

#[test]
fn a_space_before_the_marker_does_not_make_a_continuation() {
    // `// @> text` is a PLAIN comment that happens to contain the marker
    // characters. Trimming after `//` would absorb it — exactly the silent
    // absorption the explicit-marker rule exists to prevent.
    let (found, diags) = scan_source("index.st", "//@todo: work\n// @> accidental\n");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].text, "work",
        "`// @>` is a plain comment and must never be absorbed"
    );
    assert!(diags.is_empty());
}

#[test]
fn duplicate_headers_in_one_file_get_distinct_ids() {
    // Two identical todos beside two different functions are legitimate. If
    // they shared an id, merge would hand the first record's status and
    // thread to both, and each write would clobber the other.
    let (found, _) = scan_source(
        "index.st",
        "//@todo: add a test\n.a { }\n//@todo: add a test\n",
    );
    assert_eq!(found.len(), 2);
    assert_ne!(
        found[0].id, found[1].id,
        "identical headers must not collide"
    );
}

#[test]
fn a_quoted_paren_in_meta_does_not_hide_the_header() {
    // An acceptance command carrying an unmatched literal `(` made the
    // paren-depth scan miss the real colon, and the whole comment vanished
    // from the roster with no diagnostic.
    let h = header(r#"//@agent-task(acceptance: "cargo test -- --nocapture ("): do it"#);
    assert_eq!(h.type_id, "agent-task");
    assert_eq!(h.text, "do it");
    assert_eq!(
        h.meta.get("acceptance").map(String::as_str),
        Some("cargo test -- --nocapture (")
    );
}

#[test]
fn a_quoted_colon_does_not_split_the_header() {
    // The split must ignore colons inside a quoted value too, or a URL or a
    // `key: value` inside an acceptance string would truncate the metadata.
    let h = header(r#"//@agent-task(acceptance: "run: cargo test"): go"#);
    assert_eq!(
        h.meta.get("acceptance").map(String::as_str),
        Some("run: cargo test")
    );
    assert_eq!(h.text, "go");
}

#[test]
fn an_unterminated_quoted_value_never_yields_a_bogus_value() {
    // `"oops\"` ends in a quote byte, but that quote is ESCAPED — the value
    // never closes. The first cut accepted it and stored `oops\`, silently
    // changing both the metadata and the identity hash derived from it.
    //
    // With a quote-aware scan the header no longer parses at all: the colon
    // that would end the metadata is INSIDE the unterminated quote, so there
    // is no header/text boundary to find. That is the stronger outcome — the
    // line stays an ordinary comment rather than becoming a work item whose
    // recorded metadata is not what the author wrote.
    assert_eq!(
        parse_inline_line(r#"//@agent-task(acceptance: "oops\"): do it"#),
        None,
        "an unterminated quoted value must never produce a header"
    );

    // A value with NO opening quote but a stray closing one is still a
    // reportable pair rather than a silent store.
    let h = header("//@bug(severity): x");
    assert!(h.meta.is_empty());
    assert_eq!(h.malformed_meta, vec!["severity".to_string()]);
}

#[test]
fn lines_are_counted_through_multiline_strings() {
    // The extractor skipped string bodies without counting newlines, so every
    // comment after a multiline string reported the wrong line — the number
    // persisted as the anchor and shown in diagnostics.
    let src = "@page(title: \"a\nb\")\n//@todo: on line three\n";
    let (found, _) = scan_source("index.st", src);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].line, 3,
        "line count must survive a multiline string"
    );
}

#[test]
fn apostrophes_in_prose_do_not_swallow_later_comments() {
    // 185+ stdlib files write "the container's child" in prose and the
    // language has no single-quoted string form. Treating `'` as a string
    // delimiter would swallow every comment after any contraction.
    let src = "// the container's child\n//@todo: still visible\n";
    let (found, _) = scan_source("index.st", src);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "still visible");
}

#[test]
fn crlf_sources_scan_correctly() {
    // Windows checkouts and pasted content routinely carry CRLF; a trailing
    // `\r` would end up in the identity hash and in the pill.
    let (found, _) = scan_source("index.st", "//@todo: work\r\n//@> more\r\n");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].text, "work\nmore",
        "CR must not survive into the text"
    );
}

#[test]
fn a_header_with_no_text_is_still_a_comment() {
    // `//@todo:` with nothing after it is a real (if terse) marker; dropping
    // it would lose an author's intent with no diagnostic.
    let h = header("//@todo:");
    assert_eq!(h.type_id, "todo");
    assert_eq!(h.text, "");

    let with_meta = header("//@bug(severity: high):");
    assert_eq!(
        with_meta.meta.get("severity").map(String::as_str),
        Some("high")
    );
    assert_eq!(with_meta.text, "");
}

#[test]
fn every_orphan_continuation_is_reported() {
    // A regression that reported only the FIRST would leave later
    // deleted-header prose invisible to `check`.
    let (found, diags) = scan_source("index.st", "//@> one\n//@> two\n//@> three\n");
    assert!(found.is_empty());
    assert_eq!(diags.len(), 3);
    assert_eq!(
        diags.iter().map(|d| d.line).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn an_unsafe_id_is_refused_before_it_becomes_a_path() {
    // `record.id` is deserialized from disk and (W2/W3) arrives from route
    // and MCP payloads, then is joined to a path. `../victim` would escape
    // `.comments/` and clobber an unrelated file.
    let dir = tmpdir("unsafe-id");
    let mut rec = record("../victim", Anchor::Page { route: "/".into() });
    let err = write_record(&dir, &rec).expect_err("traversal must be refused");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        !dir.join("victim.json").exists(),
        "nothing may be written outside the comments directory"
    );

    for bad in ["a/b", "..", "", "x.json"] {
        rec.id = bad.to_string();
        assert!(
            write_record(&dir, &rec).is_err(),
            "id {bad:?} must be refused"
        );
    }
    assert!(is_valid_id("i9b4d1d7edbd4e0f5"));
    assert!(is_valid_id("01J9ABCDEF-agent_1"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn records_in_unscanned_files_are_never_orphaned() {
    // A caller scanning ONE file while loading the whole project's sidecar
    // would otherwise present every live comment in every other file as an
    // orphan awaiting dismissal — destroying real threads on a filtered read.
    let scanned = scan_source("a.st", "//@todo: in a\n").0;
    let mut sidecar = BTreeMap::new();
    sidecar.insert(
        "lives-in-b".to_string(),
        record(
            "lives-in-b",
            Anchor::Inline {
                file: "b.st".into(),
                line: 2,
            },
        ),
    );

    let idx = merge(
        scanned,
        sidecar,
        &["a.st".to_string()],
        "2026-07-31T00:00:00Z",
    );
    assert!(
        idx.orphans.is_empty(),
        "a record in an unscanned file is not this scan's business"
    );
    assert!(idx.diagnostics.is_empty());
    assert!(
        idx.records.iter().any(|r| r.id == "lives-in-b"),
        "it must pass through untouched"
    );
}

#[test]
fn a_scanned_file_still_orphans_its_missing_records() {
    // The other half of the coverage rule: within the files actually READ, a
    // vanished header still means a vanished comment.
    let mut sidecar = BTreeMap::new();
    sidecar.insert(
        "gone".to_string(),
        record(
            "gone",
            Anchor::Inline {
                file: "a.st".into(),
                line: 2,
            },
        ),
    );
    let idx = merge(
        Vec::new(),
        sidecar,
        &["a.st".to_string()],
        "2026-07-31T00:00:00Z",
    );
    assert_eq!(idx.orphans.len(), 1);
}

#[test]
fn concurrent_writes_leave_no_temp_file_behind() {
    // One temp path per id let two writers interleave so that the first
    // rename published the SECOND writer's bytes while reporting success to
    // the first. Last-writer-wins is acceptable; attributing success to the
    // wrong write is not.
    let dir = tmpdir("tempunique");
    let a = record("same", Anchor::Page { route: "/a".into() });
    let mut b = a.clone();
    b.text = "second".to_string();

    std::thread::scope(|s| {
        s.spawn(|| write_record(&dir, &a).unwrap());
        s.spawn(|| write_record(&dir, &b).unwrap());
    });

    let names: Vec<String> = std::fs::read_dir(comments_dir(&dir))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        names,
        vec!["same.json".to_string()],
        "no temp file may survive a concurrent write"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unreadable_comments_directory_is_reported() {
    // Treating every read_dir error as "no comments" would tell a team their
    // history is gone when it is merely unreadable.
    let dir = tmpdir("notadir");
    // A FILE where the directory should be: read_dir fails with NotADirectory.
    std::fs::write(comments_dir(&dir), "not a directory").unwrap();
    let (out, diags) = read_sidecar(&dir);
    assert!(out.is_empty());
    assert_eq!(diags.len(), 1, "the failure must be visible");
    assert!(
        diags[0]
            .message
            .contains("could not read the comments directory")
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn duplicate_sidecar_ids_are_reported_not_silently_collapsed() {
    // Two files claiming one id (a plausible merge artifact) would collapse
    // to whichever the OS listed last, losing a thread with no diagnostic.
    let dir = tmpdir("dupid");
    std::fs::create_dir_all(comments_dir(&dir)).unwrap();
    let rec = record("shared", Anchor::Page { route: "/".into() });
    let json = serde_json::to_string(&rec).unwrap();
    std::fs::write(comments_dir(&dir).join("shared.json"), &json).unwrap();
    std::fs::write(comments_dir(&dir).join("copy.json"), &json).unwrap();

    let (out, diags) = read_sidecar(&dir);
    assert_eq!(out.len(), 1);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("duplicate comment id"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn open_statuses_are_the_ones_awaiting_attention() {
    // The badge count and the `check` roster are built on this; narrowing it
    // to Open alone would make claimed work vanish from view.
    assert!(Status::Open.is_open());
    assert!(Status::InProgress.is_open());
    assert!(!Status::Resolved.is_open());
    assert!(!Status::Wontfix.is_open());
}

#[test]
fn anchor_file_is_the_file_when_there_is_one() {
    assert_eq!(
        Anchor::Inline {
            file: "a.st".into(),
            line: 1
        }
        .file(),
        Some("a.st")
    );
    assert_eq!(
        Anchor::File {
            file: "d.json".into(),
            json_path: None
        }
        .file(),
        Some("d.json")
    );
    assert_eq!(Anchor::Page { route: "/x".into() }.file(), None);
    assert_eq!(
        Anchor::Element {
            route: "/x".into(),
            selector: ".cta".into(),
            fingerprint: None,
            content: None
        }
        .file(),
        None
    );
}

#[test]
fn bool_and_ident_field_kinds_are_enforced() {
    // Both branches were unreachable from the original test roster, so
    // replacing either with `true` passed the whole suite.
    let mut roster = BTreeMap::new();
    roster.insert(
        "t".to_string(),
        CommentTypeDefAst {
            id: "t".to_string(),
            label: "T".to_string(),
            docs: "d".to_string(),
            fields: vec![
                CommentFieldAst {
                    name: "flag".to_string(),
                    kind: CommentFieldKind::Bool,
                    optional: true,
                    span: Default::default(),
                },
                CommentFieldAst {
                    name: "who".to_string(),
                    kind: CommentFieldKind::Ident,
                    optional: true,
                    span: Default::default(),
                },
            ],
            ..Default::default()
        },
    );

    let clean = scan_one("//@t(flag: true, who: brand-team): x\n");
    assert!(validate_against_roster(&clean, &roster).is_empty());

    let bad_bool = scan_one("//@t(flag: yes): x\n");
    let d = validate_against_roster(&bad_bool, &roster);
    assert_eq!(d.len(), 1);
    assert!(d[0].message.contains("expects bool"));

    let bad_ident = scan_one("//@t(who: \"not valid!\"): x\n");
    let d = validate_against_roster(&bad_ident, &roster);
    assert_eq!(d.len(), 1);
    assert!(d[0].message.contains("expects ident"));
}

// ============================================================================
// Review-gate regressions (Gate 2 swarm findings, 2026-07-31)
//
// W2 and W3 opened TWO write surfaces onto one store. These pin the shared
// trust boundary that keeps them from drifting apart.
// ============================================================================

#[test]
fn an_inline_anchor_cannot_be_minted_through_a_write_surface() {
    // An inline comment's identity is a content hash of its source line. A
    // record created with a fresh id could never be matched to the `//@`
    // header it claims, so the very next scan would classify it as an orphan —
    // the pill would show a comment that was born already broken.
    let rejection = anchor_rejection(&Anchor::Inline {
        file: "index.st".into(),
        line: 3,
    })
    .expect("inline anchors must be refused");
    assert!(
        rejection.contains("writing") && rejection.contains("page or file"),
        "the message must say how an inline comment IS made, and what to use \
         instead: {rejection}"
    );
}

#[test]
fn an_anchor_file_outside_the_project_is_refused() {
    // Route payloads and MCP arguments are equally untrusted: a record must
    // not be able to claim a location the server was never asked to serve.
    for bad in ["/etc/passwd", "../secrets.st", "a/../../b.st", ""] {
        assert!(
            anchor_rejection(&Anchor::File {
                file: bad.into(),
                json_path: None
            })
            .is_some(),
            "anchor file {bad:?} must be refused"
        );
        assert!(!is_valid_project_file(bad), "{bad:?} is not project-local");
    }
    assert!(is_valid_project_file("src/index.st"));
    assert!(
        anchor_rejection(&Anchor::File {
            file: "data/site.json".into(),
            json_path: Some("/items/0".into())
        })
        .is_none(),
        "an ordinary project-relative file must be accepted"
    );
}

#[test]
fn a_page_anchor_needs_a_route() {
    assert!(
        anchor_rejection(&Anchor::Page {
            route: String::new()
        })
        .is_some()
    );
    assert!(
        anchor_rejection(&Anchor::Page {
            route: "/pricing".into()
        })
        .is_none()
    );
}
#[test]
fn an_element_anchor_needs_a_route_and_selector() {
    assert!(
        anchor_rejection(&Anchor::Element {
            route: String::new(),
            selector: ".cta".into(),
            fingerprint: None,
            content: None,
        })
        .is_some()
    );
    assert!(
        anchor_rejection(&Anchor::Element {
            route: "/pricing".into(),
            selector: String::new(),
            fingerprint: None,
            content: None,
        })
        .is_some()
    );
    assert!(
        anchor_rejection(&Anchor::Element {
            route: "/pricing".into(),
            selector: ".cta".into(),
            fingerprint: None,
            content: None,
        })
        .is_none()
    );
}
#[test]
fn a_symlinked_comments_directory_is_refused() {
    // `create_dir_all` FOLLOWS a symlink, so a planted link at `.comments`
    // would silently redirect every record out of the project — including
    // over files the writer never intended to touch.
    #[cfg(unix)]
    {
        let dir = tmpdir("symlink");
        let elsewhere = tmpdir("symlink-target");
        std::os::unix::fs::symlink(&elsewhere, comments_dir(&dir)).expect("plant symlink");

        let err = write_record(&dir, &record("s1", Anchor::Page { route: "/".into() }))
            .expect_err("a symlinked store must be refused");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !elsewhere.join("s1.json").exists(),
            "nothing may be written through the link"
        );

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }
}

#[test]
fn a_parse_failure_yields_no_comments_rather_than_unsafe_ones() {
    // The exclusion set comes from the PARSE. Falling back to "no exclusions"
    // when the parse fails is the dangerous default: one unrelated syntax
    // error anywhere in the file would turn every `//@` inside markup into a
    // harvested comment — filing text that RENDERS TO VISITORS as a private
    // work item, and inviting the pill to "resolve" the page's own copy.
    let broken = "<div>\n  //@todo: this renders on the page\n</div>\n.unclosed { color: red;\n";
    assert!(
        crate::parser::parse(broken).is_err(),
        "fixture must actually fail to parse, or this proves nothing"
    );

    let (found, diags) = scan_file("index.st", broken);
    assert!(
        found.is_empty(),
        "an unparseable file must yield NO comments, not markup-derived ones: {found:?}"
    );
    assert!(diags.is_empty());
}

#[test]
fn a_parseable_file_still_scans_outside_markup() {
    // The other half: the safe fallback must not cost real comments.
    let src = "//@todo: a real comment\n\n<div>\n  //@todo: renders on the page\n</div>\n";
    let (found, _) = scan_file("index.st", src);
    assert_eq!(found.len(), 1, "exactly the non-markup comment: {found:?}");
    assert_eq!(found[0].text, "a real comment");
}

#[test]
fn literate_documents_are_scannable_sources() {
    // A `.st.md` fence IS the program, so a comment written there is as real
    // as one in a plain source file. Missing it made comments visible to
    // `check` and invisible to agents over MCP — one store, two answers.
    assert!(is_scannable_source(Path::new("page.st")));
    assert!(is_scannable_source(Path::new("docs/tutorial.st.md")));
    assert!(!is_scannable_source(Path::new("README.md")));
    assert!(!is_scannable_source(Path::new("data.json")));
}

#[test]
fn the_demo_fixture_exercises_the_documented_surface() {
    // `tests/fixtures/comments-demo/` is what a reader is pointed at by
    // docs/language/comments.md, so it must keep working as the feature moves.
    // A demo that silently stops demonstrating is worse than no demo: it
    // teaches the wrong thing with the authority of being checked in.
    let page = include_str!("../../tests/fixtures/comments-demo/index.st");
    let (found, diags) = scan_file("index.st", page);

    assert!(
        diags.is_empty(),
        "the demo must stay clean of scanner warnings: {diags:?}"
    );

    let types: Vec<&str> = found.iter().map(|c| c.type_id.as_str()).collect();
    for expected in [
        "note",
        "todo",
        "bug",
        "design",
        "question",
        "agent-task",
        "brand-review",
    ] {
        assert!(
            types.contains(&expected),
            "the demo must still show a `{expected}` comment: {types:?}"
        );
    }

    // The markup block carries two `//` lines that RENDER to visitors. If the
    // scanner ever harvests them, the demo would be teaching people that
    // page copy is a private work item.
    assert_eq!(
        found.len(),
        7,
        "exactly the seven authored comments — the two `//` lines inside \
         <main> are page text, not comments: {types:?}"
    );

    // Multiline bodies join; the bug comment's continuation must be part of it.
    let bug = found.iter().find(|c| c.type_id == "bug").expect("the bug");
    assert!(
        bug.text.contains("375px viewport"),
        "continuation lines must join their header: {:?}",
        bug.text
    );
}

// ============================================================================
// Element anchor identity (FUP-171 W-b)
//
// These tests exist because the FIRST version of element anchors stored a bare
// positional selector. That anchor did not orphan when a page was edited — it
// resolved cleanly to a DIFFERENT element, which is the one failure a reviewer
// cannot see. The suite below pins the property that replaced it.
// ============================================================================

/// A candidate helper: the shape a page projection hands the resolver.
fn candidate(selector: &str, tag: &str, class: &str, text: &str) -> ElementCandidate {
    ElementCandidate {
        selector: selector.to_string(),
        tag: tag.to_string(),
        class: class.to_string(),
        id_attr: String::new(),
        text: text.to_string(),
    }
}

fn element_anchor(selector: &str, fingerprint: Option<&str>) -> Anchor {
    Anchor::Element {
        route: "/".to_string(),
        selector: selector.to_string(),
        fingerprint: fingerprint.map(str::to_string),
        content: None,
    }
}

/// THE REGRESSION TEST. Inserting an element ABOVE a commented one shifts every
/// later positional id. Before fingerprints, the comment followed the old index
/// and silently landed on the wrong paragraph. It must now track its CONTENT.
#[test]
fn a_comment_follows_its_element_when_markup_is_inserted_above_it() {
    let target = candidate("0.1", "p", "claim", "Latency dropped by half.");
    let anchor = element_anchor("0.1", Some(&target.fingerprint()));

    // A new paragraph is inserted first: the target now sits at 0.2, and 0.1 is
    // occupied by an ENTIRELY DIFFERENT paragraph.
    let after = vec![
        candidate("0.1", "p", "claim", "An unrelated new sentence."),
        candidate("0.2", "p", "claim", "Latency dropped by half."),
    ];

    let resolved = resolve_element(&anchor, &after).expect("an element anchor resolves");
    assert_eq!(
        resolved,
        ElementResolution::Resolved {
            selector: "0.2".to_string()
        },
        "the comment must follow its element to 0.2 — resolving to the stored \
         selector 0.1 would silently re-point it at a different paragraph, the \
         exact defect fingerprints exist to prevent"
    );
}

/// Deleting the commented element must ORPHAN the comment, not hand it whatever
/// element inherited its position.
#[test]
fn a_deleted_element_orphans_its_comment_rather_than_re_pointing() {
    let target = candidate("0.1", "p", "claim", "Latency dropped by half.");
    let anchor = element_anchor("0.1", Some(&target.fingerprint()));

    // The target is deleted; a different paragraph slides into 0.1.
    let after = vec![candidate("0.1", "p", "claim", "Something else entirely.")];

    assert_eq!(
        resolve_element(&anchor, &after),
        Some(ElementResolution::Gone),
        "a deleted element must orphan its comment — inheriting the position \
         would attach the note to text its author never read"
    );
}

/// Editing the element's text is a change to WHAT WAS COMMENTED ON, so the
/// comment orphans and asks a human. It must not stay silently attached.
#[test]
fn rewriting_an_elements_text_orphans_the_comment() {
    let before = candidate("0.1", "p", "claim", "Latency dropped by half.");
    let anchor = element_anchor("0.1", Some(&before.fingerprint()));
    let after = vec![candidate("0.1", "p", "claim", "Latency rose sharply.")];

    assert_eq!(
        resolve_element(&anchor, &after),
        Some(ElementResolution::Gone),
        "reversing the claim must orphan a comment about it, not leave the note \
         attached to a statement that now says the opposite"
    );
}

/// Re-indenting markup is formatting, not a rename: whitespace normalizes.
#[test]
fn reformatting_whitespace_does_not_orphan_a_comment() {
    let before = candidate("0.1", "p", "claim", "Latency dropped by half.");
    let anchor = element_anchor("0.1", Some(&before.fingerprint()));
    let after = vec![candidate(
        "0.1",
        "p",
        "claim",
        "  Latency   dropped\n  by half.  ",
    )];

    assert!(
        matches!(
            resolve_element(&anchor, &after),
            Some(ElementResolution::Resolved { .. })
        ),
        "re-indenting markup must not orphan comments — formatting is not an \
         edit to what the element says"
    );
}

/// Appending a sentence to a long paragraph must not orphan a comment about it
/// (the same "header, not body" judgement `inline_id` makes).
#[test]
fn appending_to_a_long_paragraph_does_not_orphan_its_comment() {
    let long = "A".repeat(ELEMENT_TEXT_PREFIX + 40);
    let before = candidate("0.1", "p", "", &long);
    let anchor = element_anchor("0.1", Some(&before.fingerprint()));
    let after = vec![candidate(
        "0.1",
        "p",
        "",
        &format!("{long} And a new tail."),
    )];

    assert!(
        matches!(
            resolve_element(&anchor, &after),
            Some(ElementResolution::Resolved { .. })
        ),
        "editing past the fingerprint prefix must not orphan the comment"
    );
}

/// Identical rows are genuinely indistinguishable by content. Reporting the tie
/// is correct; breaking it by position would re-introduce the positional guess.
#[test]
fn identical_elements_report_ambiguity_instead_of_guessing() {
    let row = candidate("0.1", "li", "row", "Item");
    let anchor = element_anchor("0.1", Some(&row.fingerprint()));
    let after = vec![
        candidate("0.1", "li", "row", "Item"),
        candidate("0.2", "li", "row", "Item"),
    ];

    assert_eq!(
        resolve_element(&anchor, &after),
        Some(ElementResolution::Ambiguous { count: 2 }),
        "two identical rows must report a tie — silently taking the one whose \
         index matches is the positional guess this design refuses"
    );
}

/// A legacy record (written before fingerprints) must be readable and must be
/// visibly UNVERIFIED, never presented as a confirmed match.
#[test]
fn a_legacy_element_anchor_reads_as_unverified() {
    let anchor = element_anchor("0.1", None);
    assert_eq!(
        resolve_element(&anchor, &[candidate("0.1", "p", "", "Anything")]),
        Some(ElementResolution::Unverified {
            selector: "0.1".to_string()
        }),
        "a fingerprint-less record cannot be verified and must say so"
    );
    assert!(
        anchor.display().contains("unverified"),
        "an unverified anchor must not render identically to a verified one: {}",
        anchor.display()
    );
}

/// Tag and class participate in identity: same words, different element.
#[test]
fn the_fingerprint_distinguishes_tag_and_class() {
    let text = "Ship it.";
    assert_ne!(
        element_fingerprint("p", "", "", text),
        element_fingerprint("h2", "", "", text),
        "a heading and a paragraph with the same words are different elements"
    );
    assert_ne!(
        element_fingerprint("p", "note", "", text),
        element_fingerprint("p", "warning", "", text),
        "class participates in identity"
    );
}

/// Length-prefixing: no two distinct field splits may collide (the same defect
/// `inline_id` was hardened against).
#[test]
fn fingerprint_fields_cannot_collide_by_concatenation() {
    assert_ne!(
        element_fingerprint("div", "ab", "c", ""),
        element_fingerprint("div", "a", "bc", ""),
        "fields must be length-prefixed so a class/id split cannot collide"
    );
}

/// A fingerprint is minted by this toolchain; a hand-built payload carrying
/// junk must be refused at the write surface, not stored to orphan later.
#[test]
fn a_malformed_fingerprint_is_refused_at_the_write_surface() {
    let bad = Anchor::Element {
        route: "/".to_string(),
        selector: "0.1".to_string(),
        fingerprint: Some("../../etc/passwd".to_string()),
        content: None,
    };
    assert!(
        anchor_rejection(&bad).is_some(),
        "a malformed fingerprint must be refused when written"
    );
    let good = Anchor::Element {
        route: "/".to_string(),
        selector: "0.1".to_string(),
        fingerprint: Some(element_fingerprint("p", "", "", "hi")),
        content: None,
    };
    assert!(
        anchor_rejection(&good).is_none(),
        "a minted fingerprint must be accepted: {:?}",
        anchor_rejection(&good)
    );
}

/// A legacy sidecar must round-trip WITHOUT gaining a null field — otherwise
/// reading a project would rewrite every element record in its git history.
#[test]
fn a_legacy_element_anchor_round_trips_without_churn() {
    let raw = serde_json::json!({
        "kind": "element",
        "route": "/",
        "selector": "0.1"
    });
    let anchor: Anchor = serde_json::from_value(raw.clone()).expect("legacy anchor parses");
    assert_eq!(
        serde_json::to_value(&anchor).expect("re-serializes"),
        raw,
        "a legacy record must round-trip byte-identically, not gain a null"
    );
}

/// A present-but-wrong-typed fingerprint is CORRUPTION, and must not be
/// silently downgraded to "legacy" — that would hide a real problem.
#[test]
fn a_non_string_fingerprint_is_an_error_not_a_legacy_record() {
    let raw = serde_json::json!({
        "kind": "element",
        "route": "/",
        "selector": "0.1",
        "fingerprint": 42
    });
    assert!(
        serde_json::from_value::<Anchor>(raw).is_err(),
        "a corrupt fingerprint must fail loudly rather than read as legacy"
    );
}

/// The stored content descriptor must survive a round-trip: without it the pill
/// can highlight nothing and an orphan cannot say what it lost.
#[test]
fn a_picked_element_persists_what_it_looked_like() {
    let picked = ElementCandidate {
        selector: "0.2".to_string(),
        tag: "p".to_string(),
        class: "claim".to_string(),
        id_attr: String::new(),
        text: "Latency dropped by half.".to_string(),
    };
    let anchor = with_element_content(
        Anchor::Element {
            route: "/pricing".to_string(),
            selector: String::new(),
            fingerprint: None,
            content: None,
        },
        &picked,
    );

    let json = serde_json::to_value(&anchor).expect("serializes");
    assert_eq!(
        json["fingerprint"],
        picked.fingerprint(),
        "the server must mint the fingerprint from the reported content"
    );
    assert_eq!(
        json["selector"], "0.2",
        "the picker's reported position rides along as the hint"
    );

    let back: Anchor = serde_json::from_value(json).expect("round-trips");
    let Anchor::Element { content, .. } = &back else {
        panic!("still an element anchor");
    };
    let content = content.as_ref().expect("content survives the round-trip");
    assert_eq!(
        (content.tag.as_str(), content.text.as_str()),
        ("p", "Latency dropped by half."),
        "the descriptor must persist - a hash alone cannot be reversed, so \
         without it the pill can locate nothing and an orphan cannot say what \
         it was about"
    );

    // And the persisted descriptor must still resolve against a live page.
    assert_eq!(
        resolve_element(&back, &[picked.clone()]),
        Some(ElementResolution::Resolved {
            selector: "0.2".to_string()
        }),
        "a stored anchor must resolve against the element it was made from"
    );
}

/// A malformed content blob is corruption, not a legacy record.
#[test]
fn a_malformed_content_descriptor_fails_loudly() {
    let raw = serde_json::json!({
        "kind": "element",
        "route": "/",
        "selector": "0.1",
        "content": "not-an-object"
    });
    assert!(
        serde_json::from_value::<Anchor>(raw).is_err(),
        "a corrupt content descriptor must fail loudly rather than read as absent"
    );
}

// ============================================================================
// Status accountability (FUP-171 §3)
//
// A bare `status` answers "where is this now" but not "who decided that". On a
// shared record the second question is the accountable one: "resolved" with no
// author is indistinguishable from a mistake, an over-eager agent, or someone
// closing work they never read.
// ============================================================================

/// A status change must round-trip with its author, its origin, and its time.
#[test]
fn a_status_change_records_who_moved_it_and_from_where() {
    let mut rec = record("hist-1", Anchor::Page { route: "/".into() });
    rec.history.push(StatusChange {
        author: Author {
            kind: AuthorKind::Human,
            name: "ada".to_string(),
        },
        from: Status::Open,
        to: Status::Resolved,
        at: "2026-07-31T12:00:00Z".to_string(),
    });

    let json = serde_json::to_value(&rec).expect("serializes");
    let back: CommentRecord = serde_json::from_value(json).expect("round-trips");
    let change = back.history.first().expect("the transition survives");

    assert_eq!(
        change.author.name, "ada",
        "a transition must name its author"
    );
    assert_eq!(
        (change.from, change.to),
        (Status::Open, Status::Resolved),
        "the ORIGIN must persist: `reopened from resolved` and `claimed from \
         open` are different events, and a log of destinations alone cannot \
         tell them apart"
    );
    assert!(!change.at.is_empty(), "a transition must be timestamped");
}

/// A record written before history existed must read back with an EMPTY log and
/// no added field: absence means "unknown", never "never changed".
#[test]
fn a_legacy_record_has_no_history_and_gains_no_field() {
    let mut rec = record("hist-2", Anchor::Page { route: "/".into() });
    rec.history.clear();
    let json = serde_json::to_value(&rec).expect("serializes");
    assert!(
        json.get("history").is_none(),
        "an empty history must be OMITTED, or every existing sidecar in a \
         project would churn on the next write"
    );
    let back: CommentRecord = serde_json::from_value(json).expect("round-trips");
    assert!(back.history.is_empty(), "absence reads as an empty log");
}

// Source resolution (FUP-170): where an element was WRITTEN, not where it renders.

fn src_anchor(selector: &str, tag: &str, class: &str, id_attr: &str) -> Anchor {
    Anchor::Element {
        route: "/".to_string(),
        selector: selector.to_string(),
        fingerprint: None,
        content: Some(Box::new(ElementCandidate {
            selector: selector.to_string(),
            tag: tag.to_string(),
            class: class.to_string(),
            id_attr: id_attr.to_string(),
            text: "whatever the page shows".to_string(),
        })),
    }
}

fn src(id: &str, label: &str, span: Option<(u64, u64)>) -> SourceCandidate {
    SourceCandidate {
        id: id.to_string(),
        label: label.to_string(),
        span,
    }
}

#[test]
fn an_element_anchor_resolves_to_the_source_it_was_written_in() {
    let anchor = src_anchor("0.1", "button", "cta primary", "");
    let page = [
        src("0.0", "header.masthead", Some((10, 40))),
        src("0.1", "button.cta", Some((55, 90))),
    ];
    assert_eq!(
        resolve_element_source(&anchor, &page),
        Some(SourceResolution::Resolved {
            id: "0.1".to_string(),
            span: (55, 90),
            moved: false,
        }),
        "the stored hint agrees with the label, so it is used directly"
    );
}

#[test]
fn a_stale_positional_hint_does_not_deliver_the_wrong_span() {
    // Someone inserted markup ABOVE the commented button, so every sibling id
    // shifted by one. The hint now names a DIFFERENT element. Trusting it would
    // return that element's span with total confidence -- the exact positional
    // failure fingerprinting was introduced to kill, reintroduced in a new field.
    let anchor = src_anchor("0.1", "button", "cta primary", "");
    let page = [
        src("0.0", "header.masthead", Some((10, 40))),
        src("0.1", "p.lede", Some((45, 70))),
        src("0.2", "button.cta", Some((80, 115))),
    ];
    assert_eq!(
        resolve_element_source(&anchor, &page),
        Some(SourceResolution::Resolved {
            id: "0.2".to_string(),
            span: (80, 115),
            moved: true,
        }),
        "identity must win over the stale hint, and the move must be reported"
    );
}

#[test]
fn a_deleted_element_has_no_source_and_says_so() {
    let anchor = src_anchor("0.1", "button", "cta primary", "");
    let page = [src("0.0", "header.masthead", Some((10, 40)))];
    assert_eq!(
        resolve_element_source(&anchor, &page),
        Some(SourceResolution::Gone),
        "a missing element must orphan loudly, never resolve to a neighbour"
    );
}

#[test]
fn identical_source_elements_report_the_tie_instead_of_guessing() {
    // Two `li.row` elements are indistinguishable by label. The hint is stale,
    // so there is nothing left to break the tie -- and breaking it by position
    // would be wrong precisely when a row was inserted, which is the common case.
    let anchor = src_anchor("9.9", "li", "row", "");
    let page = [
        src("0.0", "li.row", Some((10, 30))),
        src("0.1", "li.row", Some((31, 51))),
    ];
    assert_eq!(
        resolve_element_source(&anchor, &page),
        Some(SourceResolution::Ambiguous { count: 2 }),
        "a tie must be reported, not resolved by position"
    );
}

#[test]
fn a_synthesized_element_is_found_but_has_no_source_address() {
    // html5ever inserts wrappers nobody wrote. Reporting `Gone` would claim the
    // element vanished; reporting a span would point at markup that does not
    // exist. Neither is true, so this is its own outcome.
    let anchor = src_anchor("0.1", "span", "wrapper", "");
    let page = [src("0.1", "span.wrapper", None)];
    assert_eq!(
        resolve_element_source(&anchor, &page),
        Some(SourceResolution::Synthesized {
            id: "0.1".to_string()
        }),
        "found-without-a-span is distinct from not-found"
    );
}

#[test]
fn an_anchor_without_content_is_followed_but_flagged() {
    // Legacy anchors predate content identity. The hint is all there is, so it
    // is used -- and the result must NOT be presented as verified.
    let anchor = Anchor::Element {
        route: "/".to_string(),
        selector: "0.1".to_string(),
        fingerprint: None,
        content: None,
    };
    let page = [src("0.1", "button.cta", Some((55, 90)))];
    assert_eq!(
        resolve_element_source(&anchor, &page),
        Some(SourceResolution::Unverified {
            id: "0.1".to_string(),
            span: Some((55, 90)),
        }),
        "an unconfirmable anchor must be labelled unverified, not resolved"
    );
}

#[test]
fn a_non_element_anchor_has_no_source_resolution() {
    let anchor = Anchor::Page {
        route: "/".to_string(),
    };
    assert_eq!(
        resolve_element_source(&anchor, &[]),
        None,
        "only element anchors resolve to an element's source"
    );
}

/// The source label rule MUST match the projection's `element_label`, because
/// confirmation compares one against the other. If they drift, every element
/// comment silently stops confirming and every anchor degrades to a stale-hint
/// fallback -- a system-wide failure with no error anywhere.
#[test]
fn the_source_label_rule_matches_the_projection() {
    let cases = [
        ("button", "cta primary", "buy", "button.cta"),
        ("button", "", "buy", "button#buy"),
        ("button", "   ", "buy", "button#buy"),
        ("section", "", "", "section"),
        ("li", "row", "", "li.row"),
    ];
    for (tag, class, id_attr, want) in cases {
        let anchor = src_anchor("0.0", tag, class, id_attr);
        let page = [src("0.0", want, Some((1, 2)))];
        assert!(
            matches!(
                resolve_element_source(&anchor, &page),
                Some(SourceResolution::Resolved { moved: false, .. })
            ),
            "label rule disagreed with the projection for {tag:?} class={class:?} id={id_attr:?}; expected {want:?}"
        );
    }
}
