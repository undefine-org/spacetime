//! Structure IR — the shared live-coding introspection substrate (PLAN-064 B2).
//!
//! ONE producer, consumed by BOTH live-coding hosts (the MCP workbench navigator
//! and the dev-ws host's InspectStructure). It replaces the two DIVERGENT
//! introspection stacks that existed before: the MCP `stage_structure_from_bundle`
//! (which walked template-refs ONLY, so a whole `<section><h1><p><button>` hero
//! projected as ONE node) and the dev-ws `handle_inspect_element` (a CSS-selector
//! shaped, rendered-DOM view whose dispatch was dead).
//!
//! ## Why the CST, not the DOM
//!
//! Webflow's layer tree can only show what HTML/CSS expresses: a static box tree.
//! Its `@each` equivalent is ONE opaque "Collection List" node, because HTML has no
//! concept of "a repeated region bound to data". Spacetime's CST DOES — so we walk
//! the CST, which is strictly richer than the DOM, and render node KINDS the DOM
//! can't name:
//!   - `element` / `hole`  — the static box tree + its reactive bindings
//!   - `each`              — an ITERATOR node: the source signal + loop var + the
//!                           body RULE walked once (not N flattened runtime rows)
//!   - `match` / `arm`     — the dispatch structure (every arm), which the DOM
//!                           throws away at runtime (only the taken arm exists)
//!   - `template`          — a composed sub-template you walk INTO (recursive)
//!
//! ## The node
//!
//! Every [`StructureNode`] carries a stable `id` (a path address like `"0.1.2"`,
//! its position in the tree — survives a re-walk), a `source_span` (the byte range
//! in the `.st` source — the EditAst WRITE address), and its reactive `bindings`
//! (the holes/params it reads). The id you INSPECT is the id you SELECT (B3) is the
//! id you EDIT (B4) — one address space.

use serde::{Deserialize, Serialize};

/// The kind of a structure node. CST-rich: strictly a superset of the DOM's
/// vocabulary. Serialized lowercase (`element`, `hole`, …) for the host JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// An HTML element literal: `<section class="hero">…`.
    Element,
    /// A reactive hole embedded in markup: `` `$title` `` (a binding site).
    Hole,
    /// An `@each($src as $item)` iterator: source signal + loop var + body rule.
    Each,
    /// An `@match $subject` render-dispatch switch.
    Match,
    /// One arm of an `@match` (`variant => &tpl(…)`).
    Arm,
    /// A composed sub-template invocation (`&card($item)`) — walked into.
    Template,
    /// A template invocation whose target is NOT in this bundle (a stdlib
    /// component); a leaf, so the tree stays complete.
    Extern,
}

impl NodeKind {
    /// The lowercase tag used in the host JSON + `data-kind` attributes.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Element => "element",
            NodeKind::Hole => "hole",
            NodeKind::Each => "each",
            NodeKind::Match => "match",
            NodeKind::Arm => "arm",
            NodeKind::Template => "template",
            NodeKind::Extern => "extern",
        }
    }
}

/// One node of the Structure IR. A flat depth-tagged projection (the host list
/// bindings render by indent) with a stable `id` path so selection is addressable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructureNode {
    /// Stable path id (`"0"`, `"0.1"`, `"0.1.2"`) — position in the tree. Survives
    /// a re-walk of the same source; distinct from `label` (which repeats). This is
    /// the SELECTION address (B3) and the key `EditAst` will re-key onto (B4).
    pub id: String,
    /// The node kind (element / hole / each / match / arm / template / extern).
    pub kind: NodeKind,
    /// Display label: `"section.hero"`, `` "`$title`" ``, `"@each $items"`,
    /// `"loading →"`, a template name.
    pub label: String,
    /// Indent depth (0 = the template root). Drives the navigator's `--mcp-depth`.
    pub depth: usize,
    /// The byte span of this node's source construct — the EditAst WRITE address.
    /// `None` when the node has no addressable source (e.g. a synthesized root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_span: Option<Span>,
    /// The source file containing this node's invocation span. `None` denotes
    /// the request entry source; element-only nodes have no file address.
    pub source_file: Option<String>,
    /// The reactive bindings this node reads (hole/param names, without `$`). An
    /// element with `` `$title` `` text carries `["title"]`; the reactive edges.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<String>,
    /// For `each`/`match`: the driving signal (`$items` -> `"items"`). The
    /// generative node's data source. Empty for static nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    /// For `each`: the loop variable (`$item` -> `"item"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_var: Option<String>,
    /// For `template`/`extern`: the invoked template's name (distinct from `label`,
    /// which prefers a `ref_name`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// For `template`/`extern`: the invocation's ref name (`&hero &card(…)` -> `hero`),
    /// or `None` for an anonymous invocation. Distinct from `template` (the target).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_name: Option<String>,
    /// For `template`/`extern`: the RAW positional/named arg source strings this
    /// invocation passed (`&card("A", title: "B")` -> `["\"A\"", "title: \"B\""]`).
    /// A host pairs these with the target template's param schema to show bound
    /// values; empty for a template DEFINITION node (the entry) or a no-arg call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invoke_args: Vec<String>,
    /// `true` when this template node is a RECURSIVE re-entry (already on the walk
    /// path) — a cycle sentinel that is not descended into.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recursive: bool,
}

/// A byte span in the `.st` source (the EditAst write address).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// A span is meaningful only when `end > start`; a zero/degenerate span is None.
    pub fn new(start: usize, end: usize) -> Option<Self> {
        if end > start {
            Some(Span { start, end })
        } else {
            None
        }
    }
}

/// A stable path-id builder. Ids are dotted position paths (`"0.1.2"`) so a node's
/// address is its structural location — stable across a re-walk of the same source.
pub(crate) struct IdPath(Vec<usize>);

impl IdPath {
    fn root() -> Self {
        IdPath(Vec::new())
    }
    /// The current path rendered as a dotted string (`"0.1.2"`). An empty path is
    /// only ever a PARENT-of-children (the entry seed), never emitted as a node's
    /// id, so it renders to the empty string; every real node has ≥1 segment.
    fn render(&self) -> String {
        self.0
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }
    fn child(&self, index: usize) -> Self {
        let mut v = self.0.clone();
        v.push(index);
        IdPath(v)
    }
}

pub mod bundle_walk;
pub mod html_ids;
pub mod html_walk;
pub mod project;

pub use bundle_walk::{BoundParam, bound_params, structure_from_bundle};
pub(crate) use html_walk::structure_from_html;
pub use project::{param_rows, structure_json, widget_for_type};

/// Project a template body's clean HTML into a flat element/hole node list,
/// depth-relative to 0 (the body root). A thin convenience over
/// [`structure_from_html`] for a consumer that wants a template's element subtree
/// on its own — e.g. the MCP host injecting element rows beneath a template node.
/// The returned nodes carry depth `0..` (the caller offsets by the template's
/// depth) and dotted-path ids local to this body (the caller re-keys to its own
/// id scheme if needed).
pub fn structure_from_html_nodes(html: &str) -> Vec<StructureNode> {
    let mut out = Vec::new();
    structure_from_html(html, &IdPath::root(), 0, &mut out);
    out
}
