//! Server-side editable AST validation (PLAN-031 / FEAT-099).
//!
//! The structured editor's wire format is the document AST (`{type:"doc",
//! content:[Block]}`), NOT HTML. So the security boundary is AST validation, not
//! HTML sanitization: walk the document, drop any block/mark the schema doesn't
//! describe, drop disallowed attributes, and scheme-check `url`-typed attrs. This
//! is strictly stronger than the legacy `sanitize_richtext` HTML pass — there is
//! no parser to outsmart and every surviving node is typed by the schema.
//!
//! "The schema IS the policy" (mirrors richtext.rs "the list IS the policy"): the
//! allowed shape falls out of the registered `@editable-mark`/`@editable-block`
//! constructs (FEAT-097), never a separate hand-kept allowlist.
//!
//! Fail-closed: a value that is not a recognizable doc AST is replaced with an
//! empty document rather than persisted as-is.

use serde_json::{Map, Value};

/// The validation schema: which blocks/marks exist and, per node, which attrs are
/// allowed and their declared types. Built from `editable-schema.json`
/// (`schemas_to_json`) or directly from `MarkSchema`s.
#[derive(Debug, Clone, Default)]
pub struct ValidationSchema {
    pub blocks: std::collections::HashMap<String, NodeShape>,
    pub marks: std::collections::HashMap<String, NodeShape>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeShape {
    /// attr name -> declared type (e.g. "href" -> Some("url")). Absent attrs are
    /// not allowed on this node.
    pub attrs: std::collections::HashMap<String, Option<String>>,
    /// Editable regions this block declares, in document order (FUP-042). Empty
    /// for a flat block (the degenerate single-region case): its editable content
    /// rides the node's `content` array. A region block validates each region's
    /// content by the region kind instead (`content` is ignored / absent).
    pub regions: Vec<RegionShape>,
}

/// One declared region of a block: a named editable sub-space of a given kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionShape {
    pub name: String,
    /// `"inline"` (holds text+marks) or `"block"` (holds child blocks).
    pub kind: String,
}

impl ValidationSchema {
    /// Build from the `editable-schema.json` JSON shape produced by
    /// `crate::editable::schemas_to_json`: `{ marks:{name:{attrs:{a:{type}}}}, blocks:{…} }`.
    pub fn from_schema_json(json: &Value) -> Self {
        let mut s = ValidationSchema::default();
        for (kind, target) in [("marks", true), ("blocks", false)] {
            if let Some(obj) = json.get(kind).and_then(|v| v.as_object()) {
                for (name, entry) in obj {
                    let mut shape = NodeShape::default();
                    if let Some(attrs) = entry.get("attrs").and_then(|v| v.as_object()) {
                        for (attr_name, attr_def) in attrs {
                            let ty = attr_def
                                .get("type")
                                .and_then(|v| v.as_str())
                                .map(String::from);
                            shape.attrs.insert(attr_name.clone(), ty);
                        }
                    }
                    // Regions (FUP-042): a block may declare named editable
                    // sub-spaces `[{name,kind}]`. Absent/empty = flat block.
                    if let Some(regions) = entry.get("regions").and_then(|v| v.as_array()) {
                        for r in regions {
                            if let (Some(rname), kind) = (
                                r.get("name").and_then(|v| v.as_str()),
                                r.get("kind").and_then(|v| v.as_str()).unwrap_or("inline"),
                            ) {
                                shape.regions.push(RegionShape {
                                    name: rname.to_string(),
                                    kind: kind.to_string(),
                                });
                            }
                        }
                    }
                    if target {
                        s.marks.insert(name.clone(), shape);
                    } else {
                        s.blocks.insert(name.clone(), shape);
                    }
                }
            }
        }
        s
    }
}

/// URL schemes rejected on any `url`-typed attribute. These are the classic
/// script-injection vectors; a `url`-typed attr carrying one is a TYPE ERROR, not
/// a string to scrub (FEAT-099 — replaces richtext.rs scheme-sniffing).
const DANGEROUS_SCHEMES: &[&str] = &["javascript:", "data:", "vbscript:"];

fn is_dangerous_url(s: &str) -> bool {
    // Compare against a scheme prefix on the leading, whitespace/control-stripped,
    // lowercased value (a browser ignores leading control chars before the scheme).
    let cleaned: String = s
        .trim_start()
        .chars()
        .filter(|c| !c.is_control() && *c != '\u{feff}')
        .collect::<String>()
        .to_ascii_lowercase();
    DANGEROUS_SCHEMES
        .iter()
        .any(|scheme| cleaned.starts_with(scheme))
}

/// Validate + sanitize a document AST value against the schema. Returns a clean
/// doc value safe to persist. Fail-closed: a non-doc input yields an empty doc.
pub fn validate_richtext_ast(value: &Value, schema: &ValidationSchema) -> Value {
    let doc = match value.as_object() {
        Some(o) if o.get("type").and_then(|t| t.as_str()) == Some("doc") => o,
        _ => return empty_doc(),
    };
    let content = doc
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| validate_block(b, schema))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // An empty doc must still hold one empty paragraph (the minimal valid doc),
    // matching the runtime `emptyDoc`.
    let content = if content.is_empty() {
        vec![empty_paragraph()]
    } else {
        content
    };
    let mut out = Map::new();
    out.insert("type".into(), Value::String("doc".into()));
    out.insert("content".into(), Value::Array(content));
    Value::Object(out)
}

/// Validate one block. A block whose type is not a registered block is DROPPED
/// (the whole node, since an unknown container cannot be trusted). Its attrs are
/// filtered to the schema; url-typed attrs are scheme-checked.
fn validate_block(block: &Value, schema: &ValidationSchema) -> Option<Value> {
    let obj = block.as_object()?;
    let ty = obj.get("type").and_then(|t| t.as_str())?;
    let shape = schema.blocks.get(ty)?; // unknown block type → drop

    let mut out = Map::new();
    out.insert("type".into(), Value::String(ty.into()));
    if let Some(attrs) = clean_attrs(obj.get("attrs"), shape) {
        out.insert("attrs".into(), attrs);
    }

    // Region block (FUP-042): the schema declares named editable sub-spaces, so
    // the editable content lives under `regions`, not `content`. Validate each
    // DECLARED region's content by its kind; drop any region the schema does not
    // declare (fail-closed — the security boundary cannot trust an undeclared
    // sub-space), and never read a region block's `content`.
    if !shape.regions.is_empty() {
        let src = obj.get("regions").and_then(|r| r.as_object());
        let mut regions_out = Map::new();
        for rs in &shape.regions {
            let region_val = src.and_then(|m| m.get(&rs.name));
            let content = region_val
                .and_then(|rv| rv.as_object())
                .and_then(|rv| rv.get("content").and_then(|c| c.as_array()));
            let clean: Vec<Value> = match content {
                Some(items) if rs.kind == "block" => items
                    .iter()
                    .filter_map(|b| validate_block(b, schema))
                    .collect(),
                Some(items) => items
                    .iter()
                    .filter_map(|i| validate_inline(i, schema))
                    .collect(),
                None => Vec::new(),
            };
            let mut region_obj = Map::new();
            region_obj.insert("kind".into(), Value::String(rs.kind.clone()));
            region_obj.insert("content".into(), Value::Array(clean));
            regions_out.insert(rs.name.clone(), Value::Object(region_obj));
        }
        out.insert("regions".into(), Value::Object(regions_out));
        return Some(Value::Object(out));
    }

    // Flat block (degenerate single-region case): editable content rides `content`.
    let content = obj
        .get("content")
        .and_then(|c| c.as_array())
        .map(|inlines| {
            inlines
                .iter()
                .filter_map(|i| validate_inline(i, schema))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.insert("content".into(), Value::Array(content));
    Some(Value::Object(out))
}

/// Validate one inline node. Only `text` nodes are supported (atoms = W3); a
/// text node's marks are filtered to registered marks with valid attrs.
fn validate_inline(node: &Value, schema: &ValidationSchema) -> Option<Value> {
    let obj = node.as_object()?;
    if obj.get("type").and_then(|t| t.as_str()) != Some("text") {
        return None; // drop non-text inlines (atoms handled in W3)
    }
    let text = obj.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let mut out = Map::new();
    out.insert("type".into(), Value::String("text".into()));
    out.insert("text".into(), Value::String(text.into()));

    if let Some(marks) = obj.get("marks").and_then(|m| m.as_array()) {
        let clean: Vec<Value> = marks
            .iter()
            .filter_map(|m| validate_mark(m, schema))
            .collect();
        if !clean.is_empty() {
            out.insert("marks".into(), Value::Array(clean));
        }
    }
    Some(Value::Object(out))
}

/// Validate one mark. Unknown mark type → DROP the mark (keep the text it wraps).
/// A url-typed attr carrying a dangerous scheme → drop the WHOLE mark (the link is
/// the injection vector; keeping its text but not the href is the safe outcome).
fn validate_mark(mark: &Value, schema: &ValidationSchema) -> Option<Value> {
    let obj = mark.as_object()?;
    let ty = obj.get("type").and_then(|t| t.as_str())?;
    let shape = schema.marks.get(ty)?; // unknown mark → drop

    let mut out = Map::new();
    out.insert("type".into(), Value::String(ty.into()));
    if obj.get("attrs").is_some() {
        match clean_attrs(obj.get("attrs"), shape) {
            Some(attrs) => {
                out.insert("attrs".into(), attrs);
            }
            None => return None, // a dangerous url attr poisoned the mark → drop it
        }
    }
    Some(Value::Object(out))
}

/// Filter an attrs object to the schema's allowed attrs, scheme-checking
/// url-typed values. Returns `None` if a url-typed attr is dangerous (caller
/// decides whether that drops the node). Returns `Some(empty)` when no attrs
/// survive but none were dangerous — caller may then omit the key.
fn clean_attrs(attrs: Option<&Value>, shape: &NodeShape) -> Option<Value> {
    let obj = attrs?.as_object()?;
    let mut out = Map::new();
    for (k, v) in obj {
        let Some(declared_type) = shape.attrs.get(k) else {
            continue; // attr not allowed on this node → drop silently
        };
        if declared_type.as_deref() == Some("url")
            && let Some(s) = v.as_str()
            && is_dangerous_url(s)
        {
            return None; // poison: signal the caller to drop the node
        }
        out.insert(k.clone(), v.clone());
    }
    if out.is_empty() {
        None
    } else {
        Some(Value::Object(out))
    }
}

fn empty_paragraph() -> Value {
    let mut p = Map::new();
    p.insert("type".into(), Value::String("p".into()));
    p.insert("content".into(), Value::Array(vec![]));
    Value::Object(p)
}

fn empty_doc() -> Value {
    let mut out = Map::new();
    out.insert("type".into(), Value::String("doc".into()));
    out.insert("content".into(), Value::Array(vec![empty_paragraph()]));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> ValidationSchema {
        ValidationSchema::from_schema_json(&json!({
            "marks": {
                "bold": { "tag": "strong", "attrs": {} },
                "link": { "tag": "a", "attrs": { "href": { "type": "url" } } }
            },
            "blocks": {
                "p":  { "tag": "p",  "attrs": {} },
                "h2": { "tag": "h2", "attrs": {} }
            }
        }))
    }

    fn doc(content: Value) -> Value {
        json!({ "type": "doc", "content": content })
    }

    // A schema with region blocks (FUP-042): `columns` (two parallel inline
    // regions) and `ul` (one block region holding `li` blocks).
    fn region_schema() -> ValidationSchema {
        ValidationSchema::from_schema_json(&json!({
            "marks": {
                "bold": { "tag": "strong", "attrs": {} }
            },
            "blocks": {
                "p":  { "tag": "p",  "attrs": {} },
                "li": { "tag": "li", "attrs": {} },
                "columns": { "tag": "div", "attrs": {}, "regions": [
                    { "name": "left",  "kind": "inline" },
                    { "name": "right", "kind": "inline" }
                ] },
                "ul": { "tag": "ul", "attrs": {}, "regions": [
                    { "name": "items", "kind": "block" }
                ] }
            }
        }))
    }

    #[test]
    fn valid_doc_passes_through() {
        let d = doc(json!([
            { "type": "h2", "content": [ { "type": "text", "text": "Title" } ] },
            { "type": "p",  "content": [ { "type": "text", "text": "hi", "marks": [ { "type": "bold" } ] } ] }
        ]));
        let out = validate_richtext_ast(&d, &schema());
        assert_eq!(out, d);
    }

    #[test]
    fn unknown_block_is_dropped() {
        let d = doc(json!([
            { "type": "script", "content": [ { "type": "text", "text": "alert(1)" } ] },
            { "type": "p", "content": [ { "type": "text", "text": "ok" } ] }
        ]));
        let out = validate_richtext_ast(&d, &schema());
        let blocks = out["content"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["type"], "p");
    }

    #[test]
    fn unknown_mark_is_dropped_text_kept() {
        let d = doc(json!([
            { "type": "p", "content": [
                { "type": "text", "text": "x", "marks": [ { "type": "blink" }, { "type": "bold" } ] }
            ] }
        ]));
        let out = validate_richtext_ast(&d, &schema());
        let marks = out["content"][0]["content"][0]["marks"].as_array().unwrap();
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0]["type"], "bold");
        assert_eq!(out["content"][0]["content"][0]["text"], "x");
    }

    #[test]
    fn javascript_url_drops_the_link_mark() {
        let d = doc(json!([
            { "type": "p", "content": [
                { "type": "text", "text": "click", "marks": [ { "type": "link", "attrs": { "href": "javascript:alert(1)" } } ] }
            ] }
        ]));
        let out = validate_richtext_ast(&d, &schema());
        // Link mark dropped (poison href), but its TEXT survives.
        let inline = &out["content"][0]["content"][0];
        assert_eq!(inline["text"], "click");
        assert!(inline.get("marks").is_none() || inline["marks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn data_and_vbscript_urls_rejected() {
        for scheme in [
            "data:text/html,<script>",
            "vbscript:msgbox",
            "  JavaScript:alert(1)",
        ] {
            let d = doc(json!([
                { "type": "p", "content": [
                    { "type": "text", "text": "x", "marks": [ { "type": "link", "attrs": { "href": scheme } } ] }
                ] }
            ]));
            let out = validate_richtext_ast(&d, &schema());
            let inline = &out["content"][0]["content"][0];
            assert!(
                inline.get("marks").is_none() || inline["marks"].as_array().unwrap().is_empty(),
                "scheme {scheme} should have dropped the link"
            );
        }
    }

    #[test]
    fn safe_http_and_mailto_urls_kept() {
        for url in [
            "https://example.com",
            "/relative",
            "mailto:a@b.com",
            "#anchor",
        ] {
            let d = doc(json!([
                { "type": "p", "content": [
                    { "type": "text", "text": "x", "marks": [ { "type": "link", "attrs": { "href": url } } ] }
                ] }
            ]));
            let out = validate_richtext_ast(&d, &schema());
            let marks = out["content"][0]["content"][0]["marks"].as_array().unwrap();
            assert_eq!(marks.len(), 1, "url {url} should be kept");
            assert_eq!(marks[0]["attrs"]["href"], url);
        }
    }

    #[test]
    fn disallowed_attr_is_stripped() {
        let d = doc(json!([
            { "type": "p", "attrs": { "onclick": "evil()" }, "content": [ { "type": "text", "text": "x" } ] }
        ]));
        let out = validate_richtext_ast(&d, &schema());
        // onclick is not in p's allowed attrs (none) → no attrs key survives.
        assert!(out["content"][0].get("attrs").is_none());
        assert_eq!(out["content"][0]["content"][0]["text"], "x");
    }

    #[test]
    fn non_doc_input_fails_closed_to_empty_doc() {
        let out = validate_richtext_ast(&json!({ "evil": true }), &schema());
        assert_eq!(out["type"], "doc");
        assert_eq!(out["content"].as_array().unwrap().len(), 1);
        assert_eq!(out["content"][0]["type"], "p");
    }

    #[test]
    fn empty_content_becomes_single_paragraph() {
        let out = validate_richtext_ast(&doc(json!([])), &schema());
        assert_eq!(out["content"].as_array().unwrap().len(), 1);
        assert_eq!(out["content"][0]["type"], "p");
    }

    // === FUP-042 R2: region-block validation (server side of the lockstep) ===

    #[test]
    fn region_block_validates_each_inline_region() {
        let d = doc(json!([
            { "type": "columns", "regions": {
                "left":  { "kind": "inline", "content": [ { "type": "text", "text": "L" } ] },
                "right": { "kind": "inline", "content": [
                    { "type": "text", "text": "R", "marks": [ { "type": "bold" } ] } ] }
            } }
        ]));
        let out = validate_richtext_ast(&d, &region_schema());
        let block = &out["content"][0];
        assert_eq!(block["type"], "columns");
        // regions preserved, content key absent for a region block.
        assert!(block.get("content").is_none());
        assert_eq!(block["regions"]["left"]["content"][0]["text"], "L");
        assert_eq!(block["regions"]["right"]["content"][0]["text"], "R");
        assert_eq!(
            block["regions"]["right"]["content"][0]["marks"][0]["type"],
            "bold"
        );
    }

    #[test]
    fn region_block_validates_block_region_children() {
        let d = doc(json!([
            { "type": "ul", "regions": {
                "items": { "kind": "block", "content": [
                    { "type": "li", "content": [ { "type": "text", "text": "a" } ] },
                    { "type": "li", "content": [ { "type": "text", "text": "b" } ] }
                ] }
            } }
        ]));
        let out = validate_richtext_ast(&d, &region_schema());
        let items = out["content"][0]["regions"]["items"]["content"]
            .as_array()
            .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["type"], "li");
        assert_eq!(items[1]["content"][0]["text"], "b");
    }

    #[test]
    fn region_block_drops_undeclared_region_failclosed() {
        // An `evil` region the schema does not declare is dropped entirely; an
        // unknown block inside a declared block region is dropped too.
        let d = doc(json!([
            { "type": "ul", "regions": {
                "items": { "kind": "block", "content": [
                    { "type": "li", "content": [ { "type": "text", "text": "ok" } ] },
                    { "type": "script", "content": [ { "type": "text", "text": "alert(1)" } ] }
                ] },
                "evil": { "kind": "inline", "content": [ { "type": "text", "text": "x" } ] }
            } }
        ]));
        let out = validate_richtext_ast(&d, &region_schema());
        let regions = out["content"][0]["regions"].as_object().unwrap();
        assert!(regions.contains_key("items"));
        assert!(
            !regions.contains_key("evil"),
            "undeclared region must be dropped"
        );
        let items = regions["items"]["content"].as_array().unwrap();
        assert_eq!(
            items.len(),
            1,
            "unknown <script> block dropped inside region"
        );
        assert_eq!(items[0]["type"], "li");
    }

    #[test]
    fn region_block_with_missing_region_yields_empty_region() {
        // A region the schema declares but the doc omits becomes an empty region
        // (fail-closed: never invents content, never errors).
        let d = doc(json!([
            { "type": "columns", "regions": {
                "left": { "kind": "inline", "content": [ { "type": "text", "text": "L" } ] }
            } }
        ]));
        let out = validate_richtext_ast(&d, &region_schema());
        let regions = &out["content"][0]["regions"];
        assert_eq!(regions["left"]["content"][0]["text"], "L");
        assert_eq!(regions["right"]["content"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn flat_block_unaffected_by_region_schema() {
        // A flat `p` validated under a schema that ALSO has region blocks still
        // takes the flat content path (regions empty for p).
        let d = doc(json!([
            { "type": "p", "content": [ { "type": "text", "text": "hi" } ] }
        ]));
        let out = validate_richtext_ast(&d, &region_schema());
        assert_eq!(out["content"][0]["content"][0]["text"], "hi");
        assert!(out["content"][0].get("regions").is_none());
    }
}
