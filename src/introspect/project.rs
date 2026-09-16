//! JSON projection for Structure IR consumers.
//!
//! The Structure IR remains the canonical tree. This module owns the stable JSON
//! shape consumed by the MCP workbench and the dev inspector route.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::{NodeKind, bound_params, structure_from_bundle};
use crate::mcp::state::{Bundle, ParamKind, TemplateBundle};

/// Widget selection is data so adding a declared type needs only one table row.
const TYPE_WIDGETS: &[(&str, &str)] = &[
    ("string", "text"),
    ("number", "number"),
    ("bool", "toggle"),
    ("boolean", "toggle"),
    ("color", "color"),
    ("length", "range-length"),
    ("duration", "range-duration"),
];

/// Return the editor widget appropriate for a declared parameter type.
pub fn widget_for_type(type_ref: Option<&str>) -> &'static str {
    type_ref
        .and_then(|type_ref| {
            TYPE_WIDGETS
                .iter()
                .find(|(declared, _)| *declared == type_ref)
                .map(|(_, widget)| *widget)
        })
        .unwrap_or("text")
}

/// Project a compiled bundle's canonical Structure IR into the host JSON shape.
///
/// `file` on a template/extern node names the invocation-site file: its
/// `invoke_span` indexes that file, rather than the invoked template's definition
/// file. `null` means the request's own entry source.
pub fn structure_json(bundle: &Bundle) -> Value {
    let entry = default_entry_template(bundle).unwrap_or("main").to_string();
    structure_json_for_entry(bundle, &entry)
}

/// Choose the template a structure walk should start from when the caller did
/// not name one.
///
/// `main` is a CONVENTION, not a requirement: it is the root only for pages
/// authored as a single entry template (the MCP guest shape, and every fixture
/// this projection was first built against). Real pages overwhelmingly do not
/// have one — they author markup at file scope and use templates as row bodies
/// invoked from `@each`. Treating `main` as mandatory therefore reported "no
/// template named 'main'" on projects whose structure was fully walkable, so the
/// rule is: prefer `main` when present, else the bundle's first template, else
/// nothing (a bundle with NO templates has no template root at all — that is a
/// distinct case the caller must handle, not one to paper over with a name that
/// does not exist).
pub fn default_entry_template(bundle: &Bundle) -> Option<&str> {
    if bundle
        .templates
        .iter()
        .any(|template| template.name == "main")
    {
        return Some("main");
    }
    bundle
        .templates
        .first()
        .map(|template| template.name.as_str())
}

/// Project a compiled bundle's canonical Structure IR from a selected entry template.
pub fn structure_json_for_entry(bundle: &Bundle, entry: &str) -> Value {
    let by_name: HashMap<&str, &TemplateBundle> = bundle
        .templates
        .iter()
        .map(|template| (template.name.as_str(), template))
        .collect();
    let ir = structure_from_bundle(bundle, entry);

    let param_schema = |template: &TemplateBundle| -> Value {
        json!(template
            .params
            .iter()
            .map(|param| {
                json!({
                    "name": param.name,
                    "kind": match param.kind { ParamKind::Binding => "binding", ParamKind::Element => "element" },
                    "type": param.type_ref,
                    "optional": param.optional,
                    "default": param.default,
                    "widget": widget_for_type(param.type_ref.as_deref()),
                })
            })
            .collect::<Vec<Value>>())
    };

    json!(
        ir.iter()
            .map(|node| {
                let span = node
                    .source_span
                    .map(|span| json!({ "start": span.start, "end": span.end }))
                    .unwrap_or(Value::Null);
                match node.kind {
                    NodeKind::Template | NodeKind::Extern => {
                        let template_name = node.template.clone().unwrap_or_default();
                        let (params, bound, param_count, bound_summary) =
                            if let Some(template) = by_name.get(template_name.as_str()) {
                                let paired = bound_params(template, &node.invoke_args);
                                let bound_json = paired
                                    .iter()
                                    .map(|param| {
                                        json!({
                                            "name": param.name,
                                            "kind": param.kind,
                                            "type": param.type_ref,
                                            "value": param.value,
                                            "default": param.default,
                                            "bound": param.bound,
                                        })
                                    })
                                    .collect::<Vec<Value>>();
                                let summary = paired
                                    .iter()
                                    .filter_map(|param| match &param.value {
                                        Some(value) => Some(format!("{}: {}", param.name, value)),
                                        None => Some(param.name.clone()),
                                    })
                                    .collect::<Vec<_>>()
                                    .join("  ·  ");
                                let schema = param_schema(template);
                                let count = schema.as_array().map_or(0, Vec::len);
                                (schema, json!(bound_json), count, summary)
                            } else {
                                (json!([]), json!([]), 0, String::new())
                            };
                        json!({
                            "id": node.id,
                            "label": node.label,
                            "kind": node.kind.as_str(),
                            "depth": node.depth,
                            "template": template_name,
                            "ref_name": node.ref_name,
                            "params": params,
                            "bound_params": bound,
                            "bound_summary": bound_summary,
                            "param_count": param_count,
                            "recursive": node.recursive,
                            "invoke_span": span,
                            "file": node.source_file,
                        })
                    }
                    // NB there is no longer an Element/Hole arm here. It existed
                    // ONLY to force `invoke_span: null`, because the element tree
                    // carried no per-node spans. Now that html5ever's line lands
                    // on every authored element, that arm was byte-identical to
                    // this one — so it is gone rather than kept as a copy that
                    // would silently drift.
                    _ => json!({
                        "id": node.id,
                        "label": node.label,
                        "kind": node.kind.as_str(),
                        "depth": node.depth,
                        "template": Value::Null,
                        "bindings": node.bindings,
                        "params": [],
                        "param_count": 0,
                        "bound_summary": "",
                        "recursive": false,
                        "invoke_span": span,
                    }),
                }
            })
            .collect::<Vec<Value>>()
    )
}

/// Flatten editable bound parameter rows from [`structure_json`].
pub fn param_rows(structure: &Value) -> Value {
    let Some(nodes) = structure.as_array() else {
        return json!([]);
    };
    let mut rows = Vec::new();
    for node in nodes {
        let (Some(start), Some(end)) = (
            node.get("invoke_span")
                .and_then(|span| span.get("start"))
                .and_then(Value::as_u64),
            node.get("invoke_span")
                .and_then(|span| span.get("end"))
                .and_then(Value::as_u64),
        ) else {
            continue;
        };
        let Some(bound) = node.get("bound_params").and_then(Value::as_array) else {
            continue;
        };
        for param in bound {
            let type_ref = param.get("type").and_then(Value::as_str);
            rows.push(json!({
                "node_id": node.get("id").cloned().unwrap_or(Value::Null),
                "param": param.get("name").cloned().unwrap_or(Value::Null),
                "value": param.get("value").cloned().unwrap_or(Value::Null),
                "type": param.get("type").cloned().unwrap_or(Value::Null),
                "bound": param.get("bound").cloned().unwrap_or(Value::Bool(false)),
                "span_start": start,
                "span_end": end,
                "widget": widget_for_type(type_ref),
                "file": node.get("file").cloned().unwrap_or(Value::Null),
            }));
        }
    }
    json!(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn structure_json_preserves_extern_kind() {
        let bundle = crate::mcp::bundle::compile_to_bundle(
            "@template &main() { &outside(); }",
            Path::new("."),
        )
        .expect("compile");
        assert!(
            structure_json(&bundle)
                .as_array()
                .unwrap()
                .iter()
                .any(|node| node["kind"] == "extern")
        );
    }

    #[test]
    fn widget_table_covers_declared_types_and_fallback() {
        for (type_ref, widget) in TYPE_WIDGETS {
            assert_eq!(widget_for_type(Some(type_ref)), *widget);
        }
        assert_eq!(widget_for_type(Some("custom")), "text");
        assert_eq!(widget_for_type(None), "text");
    }

    #[test]
    fn param_rows_include_widget_and_file_and_skip_entry() {
        let bundle = crate::mcp::bundle::compile_to_bundle(
            "@template &child($title color) { <p>`$title`</p> } @template &main() { &child(title: \"red\"); }",
            Path::new("."),
        ).expect("compile");
        let rows = param_rows(&structure_json(&bundle));
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["widget"], "color");
        assert!(rows[0].get("file").is_some());
    }

    /// A MAIN-file invocation in a project that HAS imports must report a `file`
    /// that is either a real, openable path or `null` — never the synthetic
    /// `<bundle>.st` import-resolution anchor. Before this guard, `resolve_imports`
    /// tagged main-file scopes with `<ws_root>/<bundle>.st`, which reached the row
    /// as a plausible-looking path; a span patch addressed to it would target a
    /// file that does not exist (silently, since the value LOOKS like a path).
    #[test]
    fn main_file_rows_never_report_the_synthetic_bundle_path() {
        let root = std::env::temp_dir().join(format!(
            "spacetime-project-synthetic-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create tempdir");
        let parts = root.join("parts.st");
        std::fs::write(
            &parts,
            "@template &child($title string) { <p>`$title`</p> }",
        )
        .expect("write parts");
        // The INVOCATION lives in the main (entry) source, which has no on-disk
        // path here — the compile anchors imports on the synthetic name.
        let main = "@import \"parts.st\"\n@template &main() { &child(title: \"from-main\"); }";
        let bundle = crate::mcp::bundle::compile_to_bundle(main, &root).expect("compile import");
        let rows = param_rows(&structure_json(&bundle));
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["param"] == "title")
            .expect("child row");
        match row["file"].as_str() {
            None => {}
            Some(path) => {
                assert!(
                    !path.contains("<bundle>"),
                    "synthetic import anchor leaked as a row file path: {path}"
                );
                assert!(
                    std::path::Path::new(path).exists(),
                    "row file path must be openable, got: {path}"
                );
            }
        }
        std::fs::remove_dir_all(root).ok();
    }

    /// W1R REVIEW FINDING (P2): the synthetic-anchor guard matched on BASENAME,
    /// so a user's real file legitimately named `<bundle>.st` was collapsed to
    /// `None` — its rows would then be addressed to the ENTRY file and patch the
    /// wrong source. The guard now compares the exact anchor path, so a real file
    /// with that name keeps its own address.
    #[test]
    fn a_real_file_named_like_the_synthetic_anchor_keeps_its_address() {
        let root = std::env::temp_dir().join(format!(
            "spacetime-project-realbundle-{}",
            std::process::id()
        ));
        let nested = root.join("modules");
        std::fs::create_dir_all(&nested).expect("create tempdir");
        // A real, openable file whose NAME collides with the synthetic anchor.
        let collide = nested.join("<bundle>.st");
        std::fs::write(
            &collide,
            "@template &child($title string) { <p>`$title`</p> }\n@template &parent() { &child(title: \"from-collide\"); }",
        )
        .expect("write collide");
        let main = "@import \"modules/<bundle>.st\"\n@template &main() { &parent(); }";
        let bundle = crate::mcp::bundle::compile_to_bundle(main, &root).expect("compile import");
        let rows = param_rows(&structure_json(&bundle));
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["param"] == "title")
            .expect("child row");
        assert_eq!(
            row["file"].as_str(),
            Some(collide.to_str().unwrap()),
            "a real file named like the anchor must keep its own address"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn imported_invocation_rows_address_the_invocation_file() {
        let root =
            std::env::temp_dir().join(format!("spacetime-project-import-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create tempdir");
        let parts = root.join("parts.st");
        std::fs::write(&parts, "@template &child($title string) { <p>`$title`</p> }\n@template &parent() { &child(title: \"from-parts\"); }").expect("write parts");
        let main = "@import \"parts.st\"\n@template &main() { &parent(); }";
        let bundle = crate::mcp::bundle::compile_to_bundle(main, &root).expect("compile import");
        let rows = param_rows(&structure_json(&bundle));
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["param"] == "title")
            .expect("child row");
        assert_eq!(row["file"].as_str(), Some(parts.to_str().unwrap()));
        let source = std::fs::read_to_string(&parts).expect("read parts");
        let start = row["span_start"].as_u64().unwrap() as usize;
        let end = row["span_end"].as_u64().unwrap() as usize;
        assert!(source[start..end].contains("title: \"from-parts\""));
        std::fs::remove_dir_all(root).ok();
    }
}
