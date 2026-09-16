//! Bundle-level Structure IR producer (PLAN-064 B2b).
//!
//! Combines the element/hole projection (B2a, [`super::html_walk`]) with the
//! template-composition graph (the `&card($item)` refs) into ONE tree, rooted at
//! the bundle's entry template (`&main`). This is the entry point both live-coding
//! hosts call: the MCP workbench navigator and the dev-ws `InspectStructure`.
//!
//! For each template node it emits:
//!   1. the template node itself (name + bound params),
//!   2. its ELEMENT tree (from `body.html` via the html walk) as children,
//!   3. its composed sub-templates (`body.refs`) — recursing into in-bundle
//!      templates (cycle-guarded), or a leaf `extern` node for a ref to a
//!      template outside this bundle (a stdlib component).
//!
//! An `@each`-driven `&item($x)` invocation is flattened into a template's `refs`
//! by the compile pipeline (a selector-scoped `@each { &item($x); }` lands as a
//! nested-scope ref). It therefore appears as a `template` child here. Labelling
//! that child as an explicit `@each` ITERATOR node (with its driving signal) needs
//! the bundle to carry the each-scope metadata — tracked as a follow-up; the
//! composition + element tree (the layer-tree parity win) is complete here.

use std::collections::HashMap;

use super::html_walk::structure_from_html;
use super::{IdPath, NodeKind, StructureNode};
use crate::mcp::state::{Bundle, ParamKind, TemplateBundle};

/// Produce the full Structure IR for a bundle, rooted at `entry` (usually
/// `"main"`). Returns a flat, depth-tagged node list (the host renders by indent).
/// An unknown `entry` yields an empty list.
pub fn structure_from_bundle(bundle: &Bundle, entry: &str) -> Vec<StructureNode> {
    structure_from_templates(&bundle.templates, entry)
}

/// Produce structure from already-collected templates. Compiler-side dev stamping
/// reuses this exact walk, keeping invocation stamps and inspect-route ids aligned.
pub fn structure_from_templates(templates: &[TemplateBundle], entry: &str) -> Vec<StructureNode> {
    let by_name: HashMap<&str, &TemplateBundle> =
        templates.iter().map(|t| (t.name.as_str(), t)).collect();
    let mut out = Vec::new();
    let Some(root) = by_name.get(entry) else {
        return out;
    };
    let mut path = Vec::new();
    // Seed the entry template at path `[0]` (renders "0") so its own id never
    // collides with its first child `[0.0]`; `IdPath::root()` (`[]`) is a
    // parent-only seed and is never emitted.
    walk_template(
        root,
        None,
        &IdPath::root().child(0),
        0,
        &by_name,
        &mut path,
        &mut out,
    );
    out
}

/// Pair a template's declared params with the raw arg sources an invocation
/// passed, yielding `(param_name, kind, type, default, bound_value, is_bound)`
/// rows. A host uses this to show a template node's BOUND values (what the call
/// set) alongside its schema. A named arg (`title: "A"`) is normalized to its
/// bare value; a positional arg pairs by index. This is host-display logic over
/// the IR, allocating no ids — the IR remains the single id authority.
pub fn bound_params(tpl: &TemplateBundle, invoke_args: &[String]) -> Vec<BoundParam> {
    tpl.params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let raw = invoke_args.get(i);
            // Named-arg invocations store `name: value`; strip a leading `<param>:`
            // that matches THIS param so the value is not double-printed.
            let value = raw.map(|raw| {
                let trimmed = raw.trim_start();
                if let Some(rest) = trimmed.strip_prefix(&p.name) {
                    let rest = rest.trim_start();
                    if let Some(after) = rest.strip_prefix(':') {
                        return after.trim().to_string();
                    }
                }
                raw.clone()
            });
            BoundParam {
                name: p.name.clone(),
                kind: match p.kind {
                    crate::mcp::state::ParamKind::Binding => "binding",
                    crate::mcp::state::ParamKind::Element => "element",
                },
                type_ref: p.type_ref.clone(),
                default: p.default.clone(),
                value,
                bound: raw.is_some(),
            }
        })
        .collect()
}

/// One paired param row (schema + bound value) for a template invocation node.
pub struct BoundParam {
    pub name: String,
    pub kind: &'static str,
    pub type_ref: Option<String>,
    pub default: Option<String>,
    pub value: Option<String>,
    pub bound: bool,
}

/// The invocation context that produced a template node: the `&child(args)` ref
/// (its ref name, arg sources, and source span). `None` for the ENTRY template
/// (nobody invokes it — it is the mount root).
struct Invoke<'a> {
    ref_name: Option<&'a str>,
    args: &'a [String],
    span: (usize, usize),
    source_file: Option<&'a str>,
}

/// Walk one template: emit its node, its element tree, then its ref children.
#[allow(clippy::too_many_arguments)]
fn walk_template(
    tpl: &TemplateBundle,
    invoke: Option<&Invoke<'_>>,
    id: &IdPath,
    depth: usize,
    by_name: &HashMap<&str, &TemplateBundle>,
    path: &mut Vec<String>,
    out: &mut Vec<StructureNode>,
) {
    let on_path = path.contains(&tpl.name);
    let ref_name = invoke.and_then(|i| i.ref_name);
    let label = ref_name.unwrap_or(tpl.name.as_str()).to_string();
    // A template node's write address is its INVOCATION span (`&child(…)`), so a
    // host can route an arg edit to it (B4). The entry template has no invocation.
    let source_span = invoke.and_then(|i| super::html_walk::span_of(i.span.0, i.span.1));
    out.push(StructureNode {
        id: id.render(),
        kind: NodeKind::Template,
        label,
        depth,
        source_span,
        source_file: invoke.and_then(|i| i.source_file).map(str::to_string),
        bindings: Vec::new(),
        data_source: None,
        loop_var: None,
        template: Some(tpl.name.clone()),
        ref_name: ref_name.map(|s| s.to_string()),
        invoke_args: invoke.map(|i| i.args.to_vec()).unwrap_or_default(),
        recursive: on_path,
    });

    // Cycle guard: don't descend a template already on this path (recursive
    // component). Its node is still emitted (above) so the recursion is visible.
    if on_path {
        return;
    }
    path.push(tpl.name.clone());

    // Track the next sibling index under THIS template node: element subtree first,
    // then ref children, so ids stay a stable position path.
    let mut child_idx = 0usize;

    // 1. The ELEMENT tree (B2a). Project the template's clean body HTML as children
    //    of this template node. Each top-level element becomes `id.child(child_idx)`.
    let before = out.len();
    structure_from_html(&tpl.body.html, id, depth + 1, out);
    // Advance the sibling index past however many TOP-LEVEL element rows the html
    // walk added (depth == depth+1 rows whose id path is exactly one segment
    // longer than `id`). The walk assigns them `id.child(0..)`, so count them to
    // continue ref numbering without collision.
    let top_level_added = out[before..]
        .iter()
        .filter(|n| n.depth == depth + 1)
        .count();
    child_idx += top_level_added;

    // 2. Ref children (composed sub-templates). Each `&child(...)` in this
    //    template's body becomes a child node; recurse into in-bundle templates.
    for r in &tpl.body.refs {
        let child_id = id.child(child_idx);
        child_idx += 1;
        let invoke = Invoke {
            ref_name: r.ref_name.as_deref(),
            args: &r.args,
            span: (r.span.start, r.span.end),
            source_file: tpl.source_file.as_deref(),
        };
        if let Some(child) = by_name.get(r.template_name.as_str()) {
            walk_template(
                child,
                Some(&invoke),
                &child_id,
                depth + 1,
                by_name,
                path,
                out,
            );
        } else {
            // A ref to a template OUTSIDE this bundle (a stdlib component): a leaf
            // `extern` node so the structure stays complete.
            out.push(StructureNode {
                id: child_id.render(),
                kind: NodeKind::Extern,
                label: r
                    .ref_name
                    .clone()
                    .unwrap_or_else(|| r.template_name.clone()),
                depth: depth + 1,
                source_span: super::html_walk::span_of(r.span.start, r.span.end),
                source_file: tpl.source_file.clone(),
                bindings: Vec::new(),
                data_source: None,
                loop_var: None,
                template: Some(r.template_name.clone()),
                ref_name: r.ref_name.clone(),
                invoke_args: r.args.clone(),
                recursive: false,
            });
        }
    }

    path.pop();
}

/// The parameter names a template declares (used by hosts to pair bound values).
/// Kept here so a consumer can label a template node's arity without re-reading
/// the bundle param specs.
pub fn template_param_names(tpl: &TemplateBundle) -> Vec<String> {
    tpl.params
        .iter()
        .map(|p| match p.kind {
            ParamKind::Binding | ParamKind::Element => p.name.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::bundle::compile_to_bundle;
    use std::path::Path;

    fn bundle(src: &str) -> Bundle {
        compile_to_bundle(src, Path::new(".")).expect("compile")
    }

    #[test]
    fn single_template_projects_its_element_tree() {
        // THE B2 win: a single-template hero must project its element tree, not one
        // node. Before B2, stage_structure walked refs only → 1 node.
        let b = bundle(
            "@template &main() { <section class=\"hero\"><h1>`$t`</h1><p>`$s`</p></section> }",
        );
        let nodes = structure_from_bundle(&b, "main");
        // main (template) + section + h1 + p = 4 nodes.
        let kinds: Vec<_> = nodes.iter().map(|n| (n.kind, n.label.as_str())).collect();
        assert!(
            nodes.len() >= 4,
            "expected template + element tree, got: {kinds:?}"
        );
        assert_eq!(nodes[0].kind, NodeKind::Template);
        assert_eq!(nodes[0].label, "main");
        assert!(
            nodes
                .iter()
                .any(|n| n.kind == NodeKind::Element && n.label == "section.hero"),
            "section element projected: {kinds:?}"
        );
        assert!(
            nodes
                .iter()
                .any(|n| n.kind == NodeKind::Element && n.label == "h1"),
            "h1 element projected: {kinds:?}"
        );
    }

    #[test]
    fn composed_template_is_walked_into() {
        let b = bundle(
            "@template &item($it) { <li>`$it`</li> }\n\
             @template &main() {\n  <ul class=\"list\"></ul>\n  .list { @each($items as $x) { &item($x); } }\n}",
        );
        let nodes = structure_from_bundle(&b, "main");
        // main → ul (element) → item (template child, @each-driven) → li (element).
        assert_eq!(nodes[0].label, "main");
        assert!(
            nodes
                .iter()
                .any(|n| n.kind == NodeKind::Template && n.label == "item"),
            "composed &item appears as a template node: {:?}",
            nodes
                .iter()
                .map(|n| (n.kind.as_str(), n.label.as_str()))
                .collect::<Vec<_>>()
        );
        assert!(
            nodes
                .iter()
                .any(|n| n.kind == NodeKind::Element && n.label == "li"),
            "item's <li> element is projected (walked into): {:?}",
            nodes
                .iter()
                .map(|n| (n.kind.as_str(), n.label.as_str()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn unknown_entry_is_empty() {
        let b = bundle("@template &main() { <div>x</div> }");
        assert!(structure_from_bundle(&b, "nope").is_empty());
    }

    #[test]
    fn ids_are_unique_across_the_tree() {
        let b = bundle(
            "@template &item($it) { <li>`$it`</li> }\n\
             @template &main() {\n  <ul class=\"list\"><li>static</li></ul>\n  .list { @each($xs as $x) { &item($x); } }\n}",
        );
        let nodes = structure_from_bundle(&b, "main");
        let ids: Vec<_> = nodes.iter().map(|n| n.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "all node ids unique: {ids:?}");
    }
}
