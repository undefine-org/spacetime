//! FEAT-109 W2 — the admin stops guessing.
//!
//! `widget_for` (src/server.rs) picks an editing control from the field's NAME,
//! then from `format` — and `format` exists only when a `@type` declared it. The
//! corpus has 21 `@type` annotations against 2,999 inferable values
//! (tests/corpus_inference_scan.rs), so the overwhelming majority of values
//! reach the admin as `{"type":"string"}` and render as a text box. A colour
//! renders as a text box. A duration renders as a text box.
//!
//! The scalar table already maps format -> widget as DATA
//! (stdlib/scalars/types.st), and `widget_for` already consults it. This wave
//! adds no widget logic; it FEEDS that path from the value's own shape.
//!
//! The contract asserted here: given a sample value, an untyped string field
//! gets the control its literal deserves — and a DECLARED format still wins,
//! because an author who wrote `@type` meant it.

use spacetime::server::widget_for_public as widget_for;
use serde_json::json;

#[test]
fn an_undeclared_colour_gets_a_colour_picker() {
    let node = json!({ "type": "string", "x-st-sample": "#6b7280" });
    assert_eq!(widget_for("brandColour", &node), "color");
}

#[test]
fn an_undeclared_duration_gets_a_duration_control() {
    let node = json!({ "type": "string", "x-st-sample": "600ms" });
    assert_eq!(widget_for("reveal", &node), "range-duration");
}

#[test]
fn an_undeclared_length_gets_a_length_control() {
    let node = json!({ "type": "string", "x-st-sample": "400px" });
    assert_eq!(widget_for("gutter", &node), "range-length");
}

// ---------------------------------------------------------------------------
// Declaration always wins. Inference fills a GAP; it never overrides an author.
// ---------------------------------------------------------------------------

#[test]
fn a_declared_format_beats_the_sample() {
    // The author said this is a date. The sample looking like a colour does not
    // get a vote.
    let node = json!({ "type": "string", "format": "date", "x-st-sample": "#6b7280" });
    assert_eq!(widget_for("published", &node), "date");
}

#[test]
fn a_declared_enum_still_becomes_a_select() {
    let node = json!({
        "type": "string",
        "enum": ["#fff", "#000"],
        "x-st-sample": "#fff"
    });
    assert_eq!(widget_for("shade", &node), "select");
}

#[test]
fn an_identity_field_stays_readonly() {
    let node = json!({ "type": "string", "x-st-sample": "#6b7280" });
    assert_eq!(widget_for("id", &node), "readonly");
    assert_eq!(widget_for("slug", &node), "readonly");
}

// ---------------------------------------------------------------------------
// Abstention. A value inference cannot name must reach the SAME widget it
// reaches today — this keeps the wave a strict addition.
// ---------------------------------------------------------------------------

#[test]
fn a_plain_string_sample_still_gets_a_text_box() {
    let node = json!({ "type": "string", "x-st-sample": "Hello world" });
    assert_eq!(widget_for("title", &node), "text");
}

#[test]
fn a_compound_value_still_gets_a_text_box() {
    let node = json!({ "type": "string", "x-st-sample": "1px solid red" });
    assert_eq!(widget_for("border", &node), "text");
}

#[test]
fn a_reference_still_gets_a_text_box() {
    // A reference names a value supplied elsewhere; there is no literal to edit.
    for r in ["--ink", "var(--ink)", "$brand.ink"] {
        let node = json!({ "type": "string", "x-st-sample": r });
        assert_eq!(widget_for("tint", &node), "text", "{r} got a typed widget");
    }
}

#[test]
fn no_sample_behaves_exactly_as_before() {
    // Every existing caller passes a node with no sample. Those must be
    // untouched — this is the gate that says the wave cannot regress the
    // declared path.
    assert_eq!(widget_for("title", &json!({ "type": "string" })), "text");
    assert_eq!(
        widget_for("description", &json!({ "type": "string" })),
        "textarea"
    );
    assert_eq!(widget_for("image", &json!({ "type": "string" })), "media");
    assert_eq!(widget_for("count", &json!({ "type": "number" })), "number");
    assert_eq!(widget_for("live", &json!({ "type": "boolean" })), "toggle");
}

#[test]
fn a_name_heuristic_still_applies_when_inference_abstains() {
    // `image` is media by NAME. A sample that infers as nothing must not
    // displace that — abstention means "no opinion", not "plain text".
    let node = json!({ "type": "string", "x-st-sample": "garbage ~~~ nonsense" });
    assert_eq!(widget_for("image", &node), "media");
}

#[test]
fn a_colour_sample_beats_a_name_heuristic() {
    // A field called `icon` is media by name, but a value of `#6b7280` is a
    // colour by evidence. Evidence about the actual value beats a guess from
    // the field's name — that is the entire point of the wave.
    let node = json!({ "type": "string", "x-st-sample": "#6b7280" });
    assert_eq!(widget_for("icon", &node), "color");
}
