//! Editable schema extraction + invertibility check (FEAT-097).
//!
//! Given a mark/block declaration's typed params and projection body, derive a
//! [`MarkSchema`] and decide invertibility. The body is parsed with the shared
//! html5ever skeleton parser ([`crate::html::treesink::parse_html_skeleton`]) into
//! [`HtmlExpr`]s; backtick holes in the body are `HtmlExpr::Hole` / `AttrPart::Hole`
//! carrying their source (`$href`, `&sel`).
//!
//! ## The lens
//! A mark is a bidirectional lens over a selection:
//! - **forward** = render (template invocation; the projection body is the view)
//! - **backward** = paste-lift (match pasted HTML against the body to recover the
//!   AST node). Backward only works when the projection is **invertible**.
//!
//! ## Invertibility (statically checked)
//! A mark is invertible iff its body is:
//! 1. a **single root element** (one top-level tag),
//! 2. every attribute value is a **literal OR a single typed-param hole** (no
//!    concatenation, no computed expression — so a pasted attr maps to one param),
//! 3. exactly **one selection hole** (`&sel`) somewhere in the subtree.
//!
//! Non-invertible marks still render (forward) but are excluded from paste-lift
//! (FEAT-101 may add an explicit backward clause for them).

use crate::ir::{AttrPart, HtmlExpr, JsExpr};
use crate::syntax::{TemplateParamDef, TemplateParamKind};

/// Whether a registered editable construct is an inline mark or a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableKind {
    Mark,
    Block,
}

impl EditableKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EditableKind::Mark => "mark",
            EditableKind::Block => "block",
        }
    }
}

/// A single typed attribute the projected element carries, sourced from a free
/// param (e.g. `$href url` → `{ name: "href", type_ref: "url", param: "href" }`).
#[derive(Debug, Clone, PartialEq)]
pub struct AttrSchema {
    /// The attribute name on the projected element (e.g. `href`).
    pub name: String,
    /// The param name that fills it (e.g. `href`); also the AST attr key.
    pub param: String,
    /// The param's declared type, if any (`url`, `string`). Drives validation
    /// (a `url` attr rejects `javascript:` as a *type error*) + inline-edit widget.
    pub type_ref: Option<String>,
}

/// Where the selection content lands in the projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelPosition {
    /// The selection fills the children of the (single) root element.
    Children,
    /// The selection fills the children of a nested descendant (e.g. `<figcaption>`).
    Nested,
}

/// The derived schema for one mark/block.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkSchema {
    /// Registered name (no `&` prefix), e.g. `bold`, `link`, `figure`.
    pub name: String,
    pub kind: EditableKind,
    /// The projected root tag (e.g. `strong`, `a`, `figure`).
    pub tag: String,
    /// Typed attributes carried by the projection (from free params).
    pub attrs: Vec<AttrSchema>,
    /// Whether the projection is invertible (usable for paste-lift).
    pub invertible: bool,
    /// Where the selection lands (only meaningful when invertible).
    pub sel: SelPosition,
    /// Optional keyboard shortcut (`mod+k`), surfaced for the editor keymap.
    pub shortcut: Option<String>,
    /// Editable regions this block declares, in document order (FUP-042). Empty
    /// = flat block (the degenerate single-region case). A multi-region block
    /// (`&columns(&left,&right)`) or a block-region container (`&ul(&items)`)
    /// lists each named sub-space + its kind. Drives the regions lens on BOTH
    /// sides: client project/lift descend these hosts, server validate.rs
    /// validates each region's content by kind.
    pub regions: Vec<RegionDecl>,
}

/// A declared editable region of a block: a named, ordered sub-space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionDecl {
    /// Region name (no `&` prefix), e.g. `left`, `right`, `items`.
    pub name: String,
    /// `RegionKind::Inline` (text+marks) or `RegionKind::Block` (child blocks).
    pub kind: RegionKind,
    /// The projected host tag for this region (e.g. `div`, `ul`). Defaults to `div`.
    pub tag: String,
}

/// The content kind a region holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Holds inline runs (text + marks) — like a flat block's content.
    Inline,
    /// Holds child blocks (recursion) — e.g. an `li` list.
    Block,
}

impl RegionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RegionKind::Inline => "inline",
            RegionKind::Block => "block",
        }
    }
}

/// Why a declaration could not yield an invertible schema. Non-fatal: the mark
/// still renders forward; this only excludes it from paste-lift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// The body had no top-level element (empty or text-only).
    NoRootElement,
    /// The body had more than one top-level element.
    MultipleRoots(usize),
    /// An attribute value mixed literal+hole or had multiple holes (not a clean
    /// single-param mapping). Carries the offending `attr` name.
    ComputedAttr(String),
    /// The selection hole (`&sel`) count was not exactly one.
    SelHoleCount(usize),
    /// A region's hole (`&name`) count was not exactly one (FUP-042). Carries the
    /// region name + the observed count. The invertibility invariant generalizes
    /// "exactly one &sel" to "exactly one hole per declared region".
    RegionHoleCount(String, usize),
}

impl SchemaError {
    pub fn message(&self) -> String {
        match self {
            SchemaError::NoRootElement => {
                "editable projection has no root element (need a single wrapping tag)".into()
            }
            SchemaError::MultipleRoots(n) => {
                format!("editable projection has {n} top-level elements; need exactly one root")
            }
            SchemaError::ComputedAttr(a) => format!(
                "attribute `{a}` is computed (literal+hole or multiple holes); \
                 an invertible mark needs each attr to be a literal or a single typed param"
            ),
            SchemaError::SelHoleCount(n) => {
                format!("editable projection has {n} selection holes (`&sel`); need exactly one")
            }
            SchemaError::RegionHoleCount(name, n) => format!(
                "editable region `&{name}` appears {n} times in the projection; \
                 need exactly one hole per declared region"
            ),
        }
    }
}

/// Extract the schema for one editable mark/block declaration.
///
/// `name` is the registered name without `&`. `params` is the `param_list`
/// capture; `body` is the `component_body` capture (its `.html` is the projection).
/// `shortcut` is the optional `shortcut:` head line value.
///
/// Always returns a [`MarkSchema`] (forward rendering always works). The
/// `invertible` flag + the returned [`SchemaError`]s report whether paste-lift is
/// available and why not.
pub fn extract_editable_schema(
    name: &str,
    kind: EditableKind,
    params: &[TemplateParamDef],
    html: &str,
    shortcut: Option<String>,
) -> (MarkSchema, Vec<SchemaError>) {
    // FEAT-119 (W4): the schema is derived from the body HTML, sourced from the
    // World-A template SCOPE (`@template:<name>`) by the caller — not the retired
    // ComponentBody capture. This fn takes the html string directly.
    let exprs = parse_body_exprs(html);

    // The selection param is the first ELEMENT param named `sel` by convention;
    // every other param is a free (content) param, typed, that fills an attr.
    let free_params: Vec<&TemplateParamDef> = params
        .iter()
        .filter(|p| !(p.kind == TemplateParamKind::Element && p.name == "sel"))
        .collect();

    let mut errors = Vec::new();

    // --- Root element ---
    let roots: Vec<&HtmlExpr> = exprs
        .iter()
        .filter(|e| matches!(e, HtmlExpr::Element { .. }))
        .collect();
    let (tag, root) = match roots.as_slice() {
        [] => {
            errors.push(SchemaError::NoRootElement);
            (String::new(), None)
        }
        [single] => match single {
            HtmlExpr::Element { tag, .. } => (tag.clone(), Some(*single)),
            _ => unreachable!(),
        },
        many => {
            errors.push(SchemaError::MultipleRoots(many.len()));
            // Use the first root for a best-effort tag/attr read.
            match many[0] {
                HtmlExpr::Element { tag, .. } => (tag.clone(), Some(many[0])),
                _ => (String::new(), None),
            }
        }
    };

    // --- Attributes (single-hole / literal only for invertibility) ---
    let mut attrs = Vec::new();
    if let Some(HtmlExpr::Element {
        attrs: root_attrs, ..
    }) = root
    {
        for (attr_name, parts) in root_attrs {
            match classify_attr(parts) {
                AttrKind::SingleHole(param) => {
                    // Map the hole's param (`$href` → `href`) to its declared type.
                    let type_ref = free_params
                        .iter()
                        .find(|p| p.name == param)
                        .and_then(|p| p.type_ref.clone());
                    attrs.push(AttrSchema {
                        name: attr_name.clone(),
                        param,
                        type_ref,
                    });
                }
                AttrKind::Literal => { /* static attr, not a content slot */ }
                AttrKind::Computed => {
                    errors.push(SchemaError::ComputedAttr(attr_name.clone()));
                }
            }
        }
    }

    // --- Regions (FUP-042): non-`sel` element params are named editable regions.
    // A block declaring ≥1 region param is a REGION block; its editable content
    // lives in named sub-spaces rather than a single `&sel`. Each region's kind
    // is BLOCK if the param carried a collection marker (`&items[]`), else inline;
    // its host tag is the element wrapping the region's `&name` hole in the body.
    let region_params: Vec<&TemplateParamDef> = params
        .iter()
        .filter(|p| p.kind == TemplateParamKind::Element && p.name != "sel")
        .collect();

    let regions: Vec<RegionDecl> = region_params
        .iter()
        .map(|p| RegionDecl {
            name: p.name.clone(),
            kind: if p.collection {
                RegionKind::Block
            } else {
                RegionKind::Inline
            },
            tag: region_host_tag(root, &p.name).unwrap_or_else(|| "div".to_string()),
        })
        .collect();

    // --- Selection / region hole accounting ---
    let sel = sel_position(root);
    if regions.is_empty() {
        // Flat block: exactly one `&sel` hole (unchanged invertibility rule).
        let sel_holes = count_sel_holes(&exprs);
        if sel_holes != 1 {
            errors.push(SchemaError::SelHoleCount(sel_holes));
        }
    } else {
        // Region block: NO bare `&sel`, and EACH region hole appears exactly once
        // (the invertibility invariant generalizes "one &sel" → "one hole per
        // region"). A region whose hole is missing or duplicated breaks the lens.
        if count_sel_holes(&exprs) != 0 {
            errors.push(SchemaError::SelHoleCount(count_sel_holes(&exprs)));
        }
        for r in &regions {
            let n = count_named_holes(&exprs, &r.name);
            if n != 1 {
                errors.push(SchemaError::RegionHoleCount(r.name.clone(), n));
            }
        }
    }

    let invertible = errors.is_empty();
    (
        MarkSchema {
            name: name.to_string(),
            kind,
            tag,
            attrs,
            invertible,
            sel,
            shortcut,
            regions,
        },
        errors,
    )
}

/// Count `&name` holes (a region hole) anywhere in the expr tree.
fn count_named_holes(exprs: &[HtmlExpr], region: &str) -> usize {
    let target = format!("&{region}");
    let mut n = 0;
    fn walk(e: &HtmlExpr, target: &str, n: &mut usize) {
        match e {
            HtmlExpr::Hole(JsExpr::Raw(src)) => {
                if src.trim() == target {
                    *n += 1;
                }
            }
            HtmlExpr::Element { children, .. } => {
                for c in children {
                    walk(c, target, n);
                }
            }
            _ => {}
        }
    }
    for e in exprs {
        walk(e, &target, &mut n);
    }
    n
}

/// The tag of the element directly wrapping a region's `&name` hole, i.e. the
/// region's projection host. Walks the body; returns the innermost element whose
/// direct children include the `&name` hole. `None` if the hole sits at the root
/// (then the block tag itself hosts it) or is absent.
fn region_host_tag(root: Option<&HtmlExpr>, region: &str) -> Option<String> {
    let target = format!("&{region}");
    fn walk(e: &HtmlExpr, target: &str) -> Option<String> {
        if let HtmlExpr::Element { tag, children, .. } = e {
            // Direct hole child → this element is the host.
            let here = children
                .iter()
                .any(|c| matches!(c, HtmlExpr::Hole(JsExpr::Raw(src)) if src.trim() == target));
            if here {
                return Some(tag.clone());
            }
            for c in children {
                if let Some(t) = walk(c, target) {
                    return Some(t);
                }
            }
        }
        None
    }
    walk(root?, &target)
}

/// Parse a projection body's HTML into top-level `HtmlExpr`s, recovering backtick
/// holes. The body HTML still contains backtick holes (`` `$href` ``, `` `&sel` ``);
/// we run it through the same normalize→skeleton→parse path the reactive emitter
/// uses, so holes become `HtmlExpr::Hole(JsExpr::Raw(src))` and attr holes become
/// `AttrPart::Hole`.
fn parse_body_exprs(html: &str) -> Vec<HtmlExpr> {
    // `component_html_to_exprs` normalizes backtick holes (`` `$x` ``, `` `&sel` ``)
    // + skeleton-parses. FUP-041: backtick is the one hole form, so no element-param
    // set is needed — `` `&sel` `` is recognized lexically.
    crate::emit::html_reactive::component_html_to_exprs(html)
}

enum AttrKind {
    Literal,
    SingleHole(String),
    Computed,
}

/// Classify an attribute value's parts for invertibility. A clean mapping is one
/// hole alone (`href="`$href`"`) or pure literals. Anything mixed → computed.
fn classify_attr(parts: &[AttrPart]) -> AttrKind {
    let holes: Vec<&JsExpr> = parts
        .iter()
        .filter_map(|p| match p {
            AttrPart::Hole(e) => Some(e),
            _ => None,
        })
        .collect();
    let has_nonempty_lit = parts
        .iter()
        .any(|p| matches!(p, AttrPart::Lit(s) if !s.is_empty()));

    match holes.as_slice() {
        [] => AttrKind::Literal,
        [JsExpr::Raw(src)] if !has_nonempty_lit => {
            // A bare single hole. Strip the leading `$` (value param) for the name.
            let param = src
                .trim()
                .trim_start_matches('$')
                .trim_start_matches('&')
                .to_string();
            AttrKind::SingleHole(param)
        }
        _ => AttrKind::Computed,
    }
}

/// Count `&sel` selection holes across the whole subtree (text-position holes).
fn count_sel_holes(exprs: &[HtmlExpr]) -> usize {
    let mut n = 0;
    for e in exprs {
        count_sel_in(e, &mut n);
    }
    n
}

fn count_sel_in(expr: &HtmlExpr, n: &mut usize) {
    match expr {
        HtmlExpr::Hole(JsExpr::Raw(src)) => {
            let s = src.trim();
            if s == "&sel" || s == "sel" {
                *n += 1;
            }
        }
        HtmlExpr::Element { children, .. } => {
            for c in children {
                count_sel_in(c, n);
            }
        }
        _ => {}
    }
}

/// Determine whether the selection sits directly in the root's children or in a
/// nested descendant. Best-effort; only meaningful for invertible marks.
fn sel_position(root: Option<&HtmlExpr>) -> SelPosition {
    let Some(HtmlExpr::Element { children, .. }) = root else {
        return SelPosition::Children;
    };
    // Direct child sel hole → Children; otherwise it's nested deeper.
    let direct = children.iter().any(|c| {
        matches!(c, HtmlExpr::Hole(JsExpr::Raw(src)) if {
            let s = src.trim();
            s == "&sel" || s == "sel"
        })
    });
    if direct {
        SelPosition::Children
    } else {
        SelPosition::Nested
    }
}

/// Scan a site's FormMatches for `@editable-mark` / `@editable-block` declarations
/// and derive each one's schema. This is the editable analogue of
/// `TypeRegistry::from_form_matches` → types.json: the schema is a projection of
/// the registered constructs, computed on demand from the AST (never a stored
/// allowlist). Returns `(schemas, diagnostics)` where each diagnostic is
/// `(name, SchemaError)` for a non-invertible projection (non-fatal).
pub fn collect_schemas_from_matches(
    matches: &[crate::syntax::FormMatch],
    scopes: &[crate::parser::ast::ScopeBlock],
) -> (Vec<MarkSchema>, Vec<(String, SchemaError)>) {
    // FEAT-119 (W4): body HTML is sourced from the World-A template SCOPE keyed
    // `@template:<name>` (the editable-mark/-block constructs register the SAME
    // Construct identity as @template, FEAT-116), not the retired ComponentBody
    // capture. Build the name→html lookup once.
    let html_by_name: std::collections::HashMap<&str, &str> = scopes
        .iter()
        .filter_map(|s| {
            s.selector
                .strip_prefix("@template:")
                .map(|name| (name, s.html.as_str()))
        })
        .collect();
    let mut schemas = Vec::new();
    let mut diags = Vec::new();
    for fm in matches {
        let kind = match fm.macro_name.as_str() {
            "editable-mark" => EditableKind::Mark,
            "editable-block" => EditableKind::Block,
            _ => continue,
        };
        // The macro %form captures `&$name:ident`, `$params:param_list`,
        // `$body:component_body`. The name is the registered mark name.
        let Some(name) = fm.get_ident("name") else {
            continue;
        };
        let params = fm.get_param_list("params").cloned().unwrap_or_default();
        // World-A html for this construct's scope (empty string if absent).
        let html = html_by_name.get(name).copied().unwrap_or("");
        // FEAT-103: the optional `( shortcut: $s:string ; )?` body-group surfaces
        // `shortcut` as a top-level capture. Absent → None (the group didn't match).
        let shortcut = fm.get_string("shortcut").map(|s| s.to_string());
        let (schema, errs) = extract_editable_schema(name, kind, &params, html, shortcut);
        for e in errs {
            diags.push((name.to_string(), e));
        }
        schemas.push(schema);
    }
    (schemas, diags)
}

/// Serialize the collected schemas into the `editable-schema.json` shape the
/// editor reads: `{ marks: { name: {tag, attrs, sel, invertible} }, blocks: {…} }`.
pub fn schemas_to_json(schemas: &[MarkSchema]) -> serde_json::Value {
    use serde_json::{Map, Value, json};
    let mut marks = Map::new();
    let mut blocks = Map::new();
    for s in schemas {
        let attrs: Map<String, Value> = s
            .attrs
            .iter()
            .map(|a| {
                (
                    a.name.clone(),
                    json!({ "param": a.param, "type": a.type_ref }),
                )
            })
            .collect();
        // Regions (FUP-042): emitted only for region blocks (flat blocks omit the
        // key, so the client/server treat them as the degenerate single-region
        // case). Each entry carries name + kind (+ host tag for projection).
        let regions: Vec<Value> = s
            .regions
            .iter()
            .map(|r| json!({ "name": r.name, "kind": r.kind.as_str(), "tag": r.tag }))
            .collect();
        let mut entry = json!({
            "tag": s.tag,
            "attrs": Value::Object(attrs),
            "sel": match s.sel { SelPosition::Children => "children", SelPosition::Nested => "nested" },
            "invertible": s.invertible,
            "shortcut": s.shortcut,
        });
        if !regions.is_empty() {
            entry["regions"] = Value::Array(regions);
        }
        match s.kind {
            EditableKind::Mark => {
                marks.insert(s.name.clone(), entry);
            }
            EditableKind::Block => {
                blocks.insert(s.name.clone(), entry);
            }
        }
    }
    json!({ "marks": Value::Object(marks), "blocks": Value::Object(blocks) })
}

#[cfg(test)]
mod tests {
    use super::*;

    // FEAT-119 (W4): `extract_editable_schema` now takes the body HTML string directly
    // (sourced from the World-A scope in production). The helper returns the html so
    // `&body("…")` passes as `&str`.
    fn body(html: &str) -> String {
        html.to_string()
    }

    fn sel_param() -> TemplateParamDef {
        TemplateParamDef {
            name: "sel".into(),
            kind: TemplateParamKind::Element,
            optional: false,
            type_ref: None,
            default: None,
            collection: false,
        }
    }

    fn typed(name: &str, ty: &str) -> TemplateParamDef {
        TemplateParamDef {
            name: name.into(),
            kind: TemplateParamKind::Binding,
            optional: false,
            type_ref: Some(ty.into()),
            default: None,
            collection: false,
        }
    }

    /// A named editable region param (`&left`, or `&items[]` when `collection`).
    fn region_param(name: &str, collection: bool) -> TemplateParamDef {
        TemplateParamDef {
            name: name.into(),
            kind: TemplateParamKind::Element,
            optional: false,
            type_ref: None,
            default: None,
            collection,
        }
    }

    #[test]
    fn bold_is_invertible_children() {
        let (schema, errs) = extract_editable_schema(
            "bold",
            EditableKind::Mark,
            &[sel_param()],
            &body("<strong>`&sel`</strong>"),
            None,
        );
        assert!(errs.is_empty(), "errs: {errs:?}");
        assert_eq!(schema.tag, "strong");
        assert!(schema.invertible);
        assert_eq!(schema.sel, SelPosition::Children);
        assert!(schema.attrs.is_empty());
    }

    #[test]
    fn link_carries_typed_href_attr() {
        let (schema, errs) = extract_editable_schema(
            "link",
            EditableKind::Mark,
            &[sel_param(), typed("href", "url")],
            &body("<a href=\"`$href`\">`&sel`</a>"),
            Some("mod+k".into()),
        );
        assert!(errs.is_empty(), "errs: {errs:?}");
        assert_eq!(schema.tag, "a");
        assert!(schema.invertible);
        assert_eq!(schema.attrs.len(), 1);
        assert_eq!(schema.attrs[0].name, "href");
        assert_eq!(schema.attrs[0].param, "href");
        assert_eq!(schema.attrs[0].type_ref.as_deref(), Some("url"));
        assert_eq!(schema.shortcut.as_deref(), Some("mod+k"));
    }

    #[test]
    fn figure_sel_is_nested() {
        let (schema, errs) = extract_editable_schema(
            "figure",
            EditableKind::Block,
            &[sel_param(), typed("src", "url"), typed("alt", "string")],
            &body(
                "<figure><img src=\"`$src`\" alt=\"`$alt`\"><figcaption>`&sel`</figcaption></figure>",
            ),
            None,
        );
        assert!(errs.is_empty(), "errs: {errs:?}");
        assert_eq!(schema.tag, "figure");
        assert!(schema.invertible);
        assert_eq!(schema.sel, SelPosition::Nested);
        // src + alt typed attrs are on the nested <img>, not the root <figure>;
        // root-attr extraction sees none — that is acceptable (the schema records
        // the projection tag; nested-attr mapping is a paste-lift refinement, W3).
        assert_eq!(schema.kind, EditableKind::Block);
    }

    #[test]
    fn multiple_roots_not_invertible() {
        let (schema, errs) = extract_editable_schema(
            "bad",
            EditableKind::Mark,
            &[sel_param()],
            &body("<strong>`&sel`</strong><em>x</em>"),
            None,
        );
        assert!(!schema.invertible);
        assert!(
            errs.iter()
                .any(|e| matches!(e, SchemaError::MultipleRoots(2)))
        );
    }

    #[test]
    fn two_sel_holes_not_invertible() {
        let (schema, errs) = extract_editable_schema(
            "bad",
            EditableKind::Mark,
            &[sel_param()],
            &body("<strong>`&sel``&sel`</strong>"),
            None,
        );
        assert!(!schema.invertible);
        assert!(
            errs.iter()
                .any(|e| matches!(e, SchemaError::SelHoleCount(2)))
        );
    }

    #[test]
    fn computed_attr_not_invertible() {
        // href = literal "/" + hole → computed, not a clean single-param mapping.
        let (schema, errs) = extract_editable_schema(
            "bad",
            EditableKind::Mark,
            &[sel_param(), typed("href", "url")],
            &body("<a href=\"/`$href`\">`&sel`</a>"),
            None,
        );
        assert!(!schema.invertible);
        assert!(
            errs.iter()
                .any(|e| matches!(e, SchemaError::ComputedAttr(a) if a == "href"))
        );
    }

    #[test]
    fn end_to_end_parse_collect_schema() {
        // Parse real `@editable-mark`/`@editable-block` source through the compiler
        // (STDLIB_REGISTRY auto-loads stdlib/macros/editable.st), then derive the
        // schema from the resulting FormMatches — the same path the serve route uses.
        let src = r#"@editable-mark &bold(&sel) { <strong>`&sel`</strong> }
@editable-mark &link(&sel, $href url) { <a href="`$href`">`&sel`</a> }
@editable-block &figure(&sel, $src url, $alt string = "") { <figure><img src="`$src`" alt="`$alt`"><figcaption>`&sel`</figcaption></figure> }
<main class="r"></main>
"#;
        let ast = crate::parser::parse(src).expect("parse editable source");
        let (schemas, diags) = collect_schemas_from_matches(&ast.matches, &ast.scopes);

        // All three registered, none with invertibility diagnostics.
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"bold"), "got {names:?}");
        assert!(names.contains(&"link"), "got {names:?}");
        assert!(names.contains(&"figure"), "got {names:?}");
        assert!(diags.is_empty(), "unexpected schema diags: {diags:?}");

        let link = schemas.iter().find(|s| s.name == "link").unwrap();
        assert_eq!(link.kind, EditableKind::Mark);
        assert_eq!(link.tag, "a");
        assert!(link.invertible);
        assert_eq!(link.attrs.len(), 1);
        assert_eq!(link.attrs[0].name, "href");
        assert_eq!(link.attrs[0].type_ref.as_deref(), Some("url"));

        let figure = schemas.iter().find(|s| s.name == "figure").unwrap();
        assert_eq!(figure.kind, EditableKind::Block);
        assert_eq!(figure.tag, "figure");

        // JSON shape: marks vs blocks split, link carries its typed href.
        let json = schemas_to_json(&schemas);
        assert_eq!(json["marks"]["link"]["tag"], "a");
        assert_eq!(json["marks"]["link"]["attrs"]["href"]["type"], "url");
        assert_eq!(json["marks"]["link"]["invertible"], true);
        assert_eq!(json["blocks"]["figure"]["tag"], "figure");
        assert!(
            json["marks"].get("figure").is_none(),
            "figure is a block, not a mark"
        );
    }

    #[test]
    fn shortcut_head_line_flows_through_form_path() {
        // FEAT-103: the optional `( shortcut: $s:string ; )?` body-group matches via
        // the body-group PEG and surfaces `shortcut` through the %form path (NOT a
        // direct extract_editable_schema call). Proves groups unblock the metadata
        // head-line ahead of the greedy component_body capture.
        let src = r#"@editable-mark &link(&sel, $href url) { shortcut: "mod+k"; <a href="`$href`">`&sel`</a> }
<main class="r"></main>
"#;
        let ast = crate::parser::parse(src).expect("parse");
        let (schemas, _diags) = collect_schemas_from_matches(&ast.matches, &ast.scopes);
        let link = schemas
            .iter()
            .find(|s| s.name == "link")
            .expect("link registered");
        assert_eq!(link.shortcut.as_deref(), Some("mod+k"));
        // The body capture must NOT include the consumed shortcut line: the projection
        // is still a single <a> root with the href hole → invertible.
        assert_eq!(link.tag, "a");
        assert!(
            link.invertible,
            "shortcut head-line must not poison the projection"
        );
        assert_eq!(link.attrs.len(), 1);
        assert_eq!(link.attrs[0].name, "href");
    }

    #[test]
    fn no_shortcut_head_line_yields_none() {
        // The group is OPTIONAL: a mark without a shortcut line still matches and
        // schema.shortcut is None (group consumed nothing).
        let src = r#"@editable-mark &bold(&sel) { <strong>`&sel`</strong> }
<main class="r"></main>
"#;
        let ast = crate::parser::parse(src).expect("parse");
        let (schemas, _diags) = collect_schemas_from_matches(&ast.matches, &ast.scopes);
        let bold = schemas
            .iter()
            .find(|s| s.name == "bold")
            .expect("bold registered");
        assert_eq!(bold.shortcut, None);
        assert_eq!(bold.tag, "strong");
        assert!(bold.invertible);
    }

    #[test]
    fn highlight_showcase_mark_with_typed_color() {
        // FEAT-101 show-piece: a custom mark with a typed free param compiles to an
        // invertible schema entry carrying that attr — no engine change required.
        let src = r#"@editable-mark &highlight(&sel, $color string = "yellow") { <mark class="`$color`">`&sel`</mark> }
<main class="r"></main>
"#;
        let ast = crate::parser::parse(src).expect("parse");
        let (schemas, diags) = collect_schemas_from_matches(&ast.matches, &ast.scopes);
        assert!(diags.is_empty(), "diags: {diags:?}");
        let hl = schemas
            .iter()
            .find(|s| s.name == "highlight")
            .expect("highlight registered");
        assert_eq!(hl.tag, "mark");
        assert!(hl.invertible);
        assert_eq!(hl.attrs.len(), 1);
        assert_eq!(hl.attrs[0].name, "class");
        assert_eq!(hl.attrs[0].param, "color");
        assert_eq!(hl.attrs[0].type_ref.as_deref(), Some("string"));
    }

    #[test]
    fn empty_body_not_invertible() {
        // A truly empty projection has no root element. (A markless text body like
        // `just text` is instead wrapped in a synthetic <span> by the parser, which
        // is a valid single-root projection — so the no-root error is reserved for
        // the genuinely empty case.)
        let (schema, errs) =
            extract_editable_schema("bad", EditableKind::Mark, &[sel_param()], &body(""), None);
        assert!(!schema.invertible);
        assert!(errs.iter().any(|e| matches!(e, SchemaError::NoRootElement)));
    }

    // === FUP-042 R3: region derivation from the projection body ===

    #[test]
    fn end_to_end_region_blocks_through_compiler() {
        // FUP-042 R3: parse the real region showpieces through the compiler
        // (the same path serve uses) and assert the derived region schema. Proves
        // `&columns(&left,&right)` and `&list(&items[])` are AUTHORABLE, not just
        // hand-built in unit tests.
        let src = r#"@editable-block &columns(&left, &right) { <div class="cols"><div class="col">`&left`</div><div class="col">`&right`</div></div> }
@editable-block &list(&items[]) { <ul>`&items`</ul> }
@editable-block &item(&sel) { <li>`&sel`</li> }
<main class="r"></main>
"#;
        let ast = crate::parser::parse(src).expect("parse region source");
        let (schemas, diags) = collect_schemas_from_matches(&ast.matches, &ast.scopes);
        assert!(diags.is_empty(), "unexpected schema diags: {diags:?}");

        let columns = schemas
            .iter()
            .find(|s| s.name == "columns")
            .expect("columns");
        assert!(columns.invertible);
        assert_eq!(columns.regions.len(), 2);
        assert_eq!(columns.regions[0].name, "left");
        assert_eq!(columns.regions[0].kind, RegionKind::Inline);
        assert_eq!(columns.regions[1].name, "right");

        let list = schemas.iter().find(|s| s.name == "list").expect("list");
        assert!(list.invertible);
        assert_eq!(list.regions.len(), 1);
        assert_eq!(list.regions[0].name, "items");
        assert_eq!(list.regions[0].kind, RegionKind::Block);
        assert_eq!(list.regions[0].tag, "ul");

        // JSON: region blocks carry `regions`, flat `&item` does not.
        let json = schemas_to_json(&schemas);
        assert_eq!(json["blocks"]["list"]["regions"][0]["kind"], "block");
        assert_eq!(json["blocks"]["columns"]["regions"][1]["name"], "right");
        assert!(json["blocks"]["item"].get("regions").is_none());
    }

    #[test]
    fn columns_derives_two_inline_regions() {
        // @editable-block &columns(&left, &right) {
        //   <div class="cols"><div class="col">`&left`</div>
        //                      <div class="col">`&right`</div></div> }
        let (schema, errs) = extract_editable_schema(
            "columns",
            EditableKind::Block,
            &[region_param("left", false), region_param("right", false)],
            &body(
                "<div class=\"cols\"><div class=\"col\">`&left`</div>\
                 <div class=\"col\">`&right`</div></div>",
            ),
            None,
        );
        assert!(errs.is_empty(), "unexpected: {errs:?}");
        assert!(schema.invertible);
        assert_eq!(schema.regions.len(), 2);
        assert_eq!(schema.regions[0].name, "left");
        assert_eq!(schema.regions[0].kind, RegionKind::Inline);
        assert_eq!(schema.regions[0].tag, "div"); // host = the wrapping .col div
        assert_eq!(schema.regions[1].name, "right");
        assert_eq!(schema.regions[1].kind, RegionKind::Inline);
    }

    #[test]
    fn ul_collection_param_derives_block_region() {
        // @editable-block &ul(&items[]) { <ul>`&items`</ul> }
        let (schema, errs) = extract_editable_schema(
            "ul",
            EditableKind::Block,
            &[region_param("items", true)],
            &body("<ul>`&items`</ul>"),
            None,
        );
        assert!(errs.is_empty(), "unexpected: {errs:?}");
        assert!(schema.invertible);
        assert_eq!(schema.regions.len(), 1);
        assert_eq!(schema.regions[0].name, "items");
        assert_eq!(schema.regions[0].kind, RegionKind::Block);
        assert_eq!(schema.regions[0].tag, "ul");
    }

    #[test]
    fn region_block_with_missing_hole_not_invertible() {
        // `&right` declared but its hole is absent from the body → not invertible.
        let (schema, errs) = extract_editable_schema(
            "columns",
            EditableKind::Block,
            &[region_param("left", false), region_param("right", false)],
            &body("<div><div>`&left`</div></div>"),
            None,
        );
        assert!(!schema.invertible);
        assert!(
            errs.iter()
                .any(|e| matches!(e, SchemaError::RegionHoleCount(name, 0) if name == "right"))
        );
    }

    #[test]
    fn region_block_with_duplicate_hole_not_invertible() {
        // `&left` appears twice → breaks the one-hole-per-region invariant.
        let (schema, errs) = extract_editable_schema(
            "dup",
            EditableKind::Block,
            &[region_param("left", false)],
            &body("<div><span>`&left`</span><span>`&left`</span></div>"),
            None,
        );
        assert!(!schema.invertible);
        assert!(
            errs.iter()
                .any(|e| matches!(e, SchemaError::RegionHoleCount(name, 2) if name == "left"))
        );
    }

    #[test]
    fn region_block_rejects_stray_sel_hole() {
        // A region block must NOT also carry a bare `&sel`.
        let (schema, errs) = extract_editable_schema(
            "mixed",
            EditableKind::Block,
            &[region_param("left", false)],
            &body("<div><div>`&left`</div><div>`&sel`</div></div>"),
            None,
        );
        assert!(!schema.invertible);
        assert!(
            errs.iter()
                .any(|e| matches!(e, SchemaError::SelHoleCount(1)))
        );
    }

    #[test]
    fn region_schema_emits_regions_json() {
        let (schema, _) = extract_editable_schema(
            "ul",
            EditableKind::Block,
            &[region_param("items", true)],
            &body("<ul>`&items`</ul>"),
            None,
        );
        let json = schemas_to_json(&[schema]);
        let ul = &json["blocks"]["ul"];
        assert_eq!(ul["regions"][0]["name"], "items");
        assert_eq!(ul["regions"][0]["kind"], "block");
        assert_eq!(ul["regions"][0]["tag"], "ul");
    }

    #[test]
    fn flat_block_still_emits_no_regions_json() {
        let (schema, _) = extract_editable_schema(
            "h2",
            EditableKind::Block,
            &[sel_param()],
            &body("<h2>`&sel`</h2>"),
            None,
        );
        let json = schemas_to_json(&[schema]);
        assert!(json["blocks"]["h2"].get("regions").is_none());
    }
}
