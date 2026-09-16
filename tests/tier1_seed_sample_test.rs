//! FEAT-109 W2, end to end — a seed's own values reach the admin's widget
//! decision.
//!
//! `tier1_admin_widget_test.rs` proves the DECISION (given a sample, the right
//! control). This proves the PLUMBING: that a literal written in a `@data
//! inline` seed becomes the `x-st-sample` the decision reads. Both are needed —
//! a correct decision fed by nothing is a feature that does not exist, which is
//! exactly the state the whole FEAT was filed about.

use spacetime::server::{attach_field_samples_public as attach, widget_for_public as widget_for};

/// Parse a site source into the form matches the admin handler works from.
fn matches_of(src: &str) -> Vec<spacetime::syntax::FormMatch> {
    let ast = spacetime::parse(src).expect("site parses");
    ast.matches
}

/// A schema shaped like `generate_json_schema` produces for a record type whose
/// fields are all bare strings — i.e. what an UNANNOTATED type looks like.
fn bare_string_schema(fields: &[&str]) -> serde_json::Value {
    let props: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .map(|f| (f.to_string(), serde_json::json!({ "type": "string" })))
        .collect();
    serde_json::json!({ "type": "object", "properties": props })
}

fn seed() -> String {
    [
        "@data inline $brand : {",
        "  \"ink\": \"#e8eef7\",",
        "  \"surface\": \"#0b0e14\",",
        "  \"radius\": \"8px\",",
        "  \"reveal\": \"600ms\",",
        "  \"title\": \"Backdesk\"",
        "};",
        "",
        "card {",
        "  color: --ink;",
        "}",
    ]
    .join("\n")
}

#[test]
fn seed_literals_become_samples() {
    let mut schema = bare_string_schema(&["ink", "surface", "radius", "reveal", "title"]);
    attach(&mut schema, &matches_of(&seed()));

    let props = schema["properties"].as_object().expect("properties");
    assert_eq!(props["ink"]["x-st-sample"], "#e8eef7");
    assert_eq!(props["radius"]["x-st-sample"], "8px");
    assert_eq!(props["reveal"]["x-st-sample"], "600ms");
}

/// THE ACCEPTANCE TEST, as filed with the original FEAT: a brand seed with no
/// `@type` still renders typed controls.
#[test]
fn an_untyped_brand_seed_still_gets_typed_controls() {
    let mut schema = bare_string_schema(&["ink", "surface", "radius", "reveal", "title"]);
    attach(&mut schema, &matches_of(&seed()));
    let props = schema["properties"].as_object().expect("properties");

    assert_eq!(widget_for("ink", &props["ink"]), "color");
    assert_eq!(widget_for("surface", &props["surface"]), "color");
    assert_eq!(widget_for("radius", &props["radius"]), "range-length");
    assert_eq!(widget_for("reveal", &props["reveal"]), "range-duration");

    // Prose stays prose. Inference abstains and the name heuristics decide, as
    // they always did.
    assert_eq!(widget_for("title", &props["title"]), "text");
}

#[test]
fn a_field_with_no_seed_value_is_untouched() {
    let mut schema = bare_string_schema(&["ink", "unseeded"]);
    attach(&mut schema, &matches_of(&seed()));
    let props = schema["properties"].as_object().expect("properties");

    assert!(
        props["unseeded"].get("x-st-sample").is_none(),
        "a field absent from the seed must get no sample"
    );
    assert_eq!(widget_for("unseeded", &props["unseeded"]), "text");
}

#[test]
fn a_site_with_no_inline_data_is_a_no_op() {
    let mut schema = bare_string_schema(&["ink"]);
    let before = schema.clone();
    attach(&mut schema, &matches_of("card { color: #fff; }"));
    assert_eq!(schema, before, "no seed data must leave the schema alone");
}

#[test]
fn nested_records_are_walked() {
    let src = [
        "@data inline $site : {",
        "  \"theme\": { \"accent\": \"#5eead4\", \"gutter\": \"24px\" },",
        "  \"posts\": [ { \"tint\": \"#f59e0b\" } ]",
        "};",
    ]
    .join("\n");
    let mut schema = bare_string_schema(&["accent", "gutter", "tint"]);
    attach(&mut schema, &matches_of(&src));
    let props = schema["properties"].as_object().expect("properties");

    assert_eq!(widget_for("accent", &props["accent"]), "color");
    assert_eq!(widget_for("gutter", &props["gutter"]), "range-length");
    assert_eq!(
        widget_for("tint", &props["tint"]),
        "color",
        "a value inside an array of records must still be found"
    );
}
