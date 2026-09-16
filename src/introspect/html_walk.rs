//! Element + hole projection (PLAN-064 B2a).
//!
//! Walks the parsed `HtmlExpr` tree of a template body into [`StructureNode`]s.
//! This is what kills the "1-node hero" bug: a `<section><h1><p><button>` body
//! projects as its real element tree, each element carrying the reactive
//! `bindings` (holes) it reads — the layer tree Webflow shows, PLUS the binding
//! edges Webflow can't.
//!
//! Generative nodes (`@each`/`@match`) and template-ref nodes are layered on by
//! the bundle walk (B2b); this module owns the pure element/hole projection so it
//! is unit-testable against a raw HTML string with no bundle machinery.

use super::{IdPath, NodeKind, Span, StructureNode};
use crate::emit::html_reactive::component_html_to_exprs;
use crate::ir::{AttrPart, HtmlExpr, JsExpr};

/// Project a template body's clean HTML into a flat, depth-tagged node list.
///
/// `html` is the `ScopeBlock::html` (the CST `reconstruct_template_body_html`
/// output). `root` is the id path of the enclosing node (a template root); the
/// top-level elements become its children. `base_depth` is that root's depth.
///
/// Text/Raw nodes are NOT emitted as their own rows (they carry no structure);
/// a `Hole` in text position IS emitted (it is a binding site the inspector can
/// address). Holes are also surfaced as an element's `bindings` so the common
/// case (an element with `` `$title` `` text) reads as one element row with a
/// binding chip, not a separate hole row — but a bare top-level hole still gets
/// its own node so nothing is lost.
pub(crate) fn structure_from_html(
    html: &str,
    root: &IdPath,
    base_depth: usize,
    out: &mut Vec<StructureNode>,
) {
    let exprs = component_html_to_exprs(html);
    let lines = LineOffsets::new(html);
    walk_children(&exprs, root, base_depth, out, &lines);
}

/// Byte offset of the start of each 1-based line in a markup body.
///
/// html5ever reports an element's position as a LINE; a `SourceSpan` is byte
/// offsets. This is the bridge, built once per body rather than per element so
/// projecting a large page stays linear instead of quadratic.
pub(crate) struct LineOffsets {
    starts: Vec<usize>,
    len: usize,
}

impl LineOffsets {
    pub(crate) fn new(text: &str) -> Self {
        let mut starts = vec![0usize];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(index + 1);
            }
        }
        Self {
            starts,
            len: text.len(),
        }
    }

    /// The span covering 1-based `line`, or `None` when the line is out of
    /// range — a line past the end means the element was SYNTHESIZED, and
    /// inventing a span for it would point a reader at markup nobody wrote.
    fn span_of_line(&self, line: u32) -> Option<Span> {
        let index = usize::try_from(line).ok()?.checked_sub(1)?;
        let start = *self.starts.get(index)?;
        let end = self.starts.get(index + 1).map_or(self.len, |next| *next);
        Some(Span { start, end })
    }
}

/// Walk a sibling list of `HtmlExpr`, emitting a node per structural child.
fn walk_children(
    exprs: &[HtmlExpr],
    parent: &IdPath,
    depth: usize,
    out: &mut Vec<StructureNode>,
    lines: &LineOffsets,
) {
    // Structural sibling indices come from the SHARED allocator (html_ids), so the
    // IR node ids and the stamped guest-DOM `data-st-node` ids are computed by ONE
    // walk and can never diverge (the B3 selection-spine invariant).
    for slot in super::html_ids::structural_slots(exprs) {
        let id = parent.child(slot.index);
        match slot.expr {
            HtmlExpr::Element {
                tag,
                attrs,
                children,
                line,
            } => {
                let label = element_label(tag, attrs);
                // An element's bindings = the holes in its OWN attrs + its direct
                // text-hole children (not descendants — those get their own rows).
                let mut bindings = attr_bindings(attrs);
                for c in children {
                    if let HtmlExpr::Hole(js) = c {
                        for b in hole_bindings(js) {
                            if !bindings.contains(&b) {
                                bindings.push(b);
                            }
                        }
                    }
                }
                out.push(StructureNode {
                    id: id.render(),
                    kind: NodeKind::Element,
                    label,
                    depth,
                    // EXACT source mapping: html5ever reports the line each
                    // element's own tag was tokenized on, and `LineOffsets`
                    // turns that into byte offsets in this body. `None` only
                    // when the element was SYNTHESIZED rather than authored —
                    // an invented span would point a reader at markup they
                    // never wrote, which is worse than admitting we do not know.
                    source_span: line.and_then(|l| lines.span_of_line(l)),
                    source_file: None,
                    bindings,
                    data_source: None,
                    loop_var: None,
                    template: None,
                    ref_name: None,
                    invoke_args: Vec::new(),
                    recursive: false,
                });
                // Recurse into element children (they become this element's rows)
                // — but SKIP direct text-position holes (absorbed into `bindings`
                // above): `recurse_children` applies the SAME filter the stamped
                // builder uses, so descendant ids match.
                let structural_children: Vec<HtmlExpr> =
                    super::html_ids::recurse_children(children)
                        .into_iter()
                        .cloned()
                        .collect();
                walk_children(&structural_children, &id, depth + 1, out, lines);
            }
            HtmlExpr::Hole(js) => {
                // A BARE hole in text position (not absorbed into an element's
                // bindings above) — e.g. a template body that is just `` `$x` ``.
                // Emit it so nothing is lost. (Its sibling index came from the
                // shared allocator above.)
                let names = hole_bindings(js);
                let label = match names.first() {
                    Some(n) if names.len() == 1 => format!("`${n}`"),
                    Some(n) => format!("`${n} …`"),
                    None => "`…`".to_string(),
                };
                out.push(StructureNode {
                    id: id.render(),
                    kind: NodeKind::Hole,
                    label,
                    depth,
                    source_span: None,
                    source_file: None,
                    bindings: names,
                    data_source: None,
                    loop_var: None,
                    template: None,
                    ref_name: None,
                    invoke_args: Vec::new(),
                    recursive: false,
                });
            }
            // `structural_slots` only yields Element/Hole, so Text/Raw/Html/
            // Markdown never reach here; the arm exists for match exhaustiveness.
            HtmlExpr::Text(_)
            | HtmlExpr::Raw(_)
            | HtmlExpr::Html(_)
            | HtmlExpr::Markdown(_) => {}
        }
    }
}

/// A readable element label: `tag` plus its first class (`section.hero`) or id
/// (`div#main`), mirroring a devtools/Webflow layer label.
fn element_label(tag: &str, attrs: &[(String, Vec<AttrPart>)]) -> String {
    // Prefer `.class` (first token of a literal class attr); fall back to `#id`.
    if let Some(class) = literal_attr(attrs, "class")
        && let Some(first) = class.split_whitespace().next()
        && !first.is_empty()
    {
        return format!("{tag}.{first}");
    }
    if let Some(id) = literal_attr(attrs, "id")
        && !id.trim().is_empty()
    {
        return format!("{tag}#{}", id.trim());
    }
    tag.to_string()
}

/// The purely-literal value of an attribute (`None` if it has any hole part).
fn literal_attr(attrs: &[(String, Vec<AttrPart>)], name: &str) -> Option<String> {
    let (_, parts) = attrs.iter().find(|(k, _)| k.eq_ignore_ascii_case(name))?;
    let mut s = String::new();
    for p in parts {
        match p {
            AttrPart::Lit(l) => s.push_str(l),
            // A hole in the attr means it is not a stable literal label source.
            AttrPart::Hole(_) => return None,
        }
    }
    Some(s)
}

/// The binding names read by an element's attribute holes (e.g. `href="`$u`"`).
fn attr_bindings(attrs: &[(String, Vec<AttrPart>)]) -> Vec<String> {
    let mut out = Vec::new();
    for (_, parts) in attrs {
        for p in parts {
            if let AttrPart::Hole(js) = p {
                for b in hole_bindings(js) {
                    if !out.contains(&b) {
                        out.push(b);
                    }
                }
            }
        }
    }
    out
}

/// Extract EVERY signal a hole reads, as base binding names (no `$`). A hole is
/// `JsExpr::Raw("$title")` / `Raw("$item.title")` / a compound `Raw("$a + $b")`;
/// delegates to the SAME `collect_signal_deps` the reactive emit uses, so the
/// inspector's binding chips match the runtime's subscription edges EXACTLY (a
/// compound expr reads MULTIPLE signals — all are surfaced, not just the first).
/// A non-signal expression (a literal, a bare call) yields an empty vec.
fn hole_bindings(js: &JsExpr) -> Vec<String> {
    let src = match js {
        JsExpr::Raw(s) => s.clone(),
        JsExpr::Var(v) => v.clone(),
        _ => return Vec::new(),
    };
    crate::syntax::collect_signal_deps(&src)
}

/// Build a span from a `(start, end)` pair (degenerate spans → None).
pub(super) fn span_of(start: usize, end: usize) -> Option<Span> {
    Span::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::introspect::IdPath;

    fn walk(html: &str) -> Vec<StructureNode> {
        let mut out = Vec::new();
        structure_from_html(html, &IdPath::root(), 0, &mut out);
        out
    }

    #[test]
    fn hero_projects_element_tree_not_one_node() {
        // THE bug B2a kills: a multi-element body must project its real tree, not
        // collapse to a single node.
        let nodes = walk(
            "<section class=\"hero\"><h1>`$title`</h1><p>`$subtitle`</p><button>Go</button></section>",
        );
        // section, h1, p, button = 4 element nodes.
        let els: Vec<_> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Element)
            .collect();
        assert_eq!(
            els.len(),
            4,
            "hero must project 4 element nodes: {nodes:#?}"
        );
        assert_eq!(els[0].label, "section.hero", "root labelled tag.class");
        assert_eq!(els[0].depth, 0);
        assert_eq!(els[1].label, "h1");
        assert_eq!(els[1].depth, 1, "h1 nests under section");
    }

    #[test]
    fn element_carries_its_text_hole_as_binding() {
        let nodes = walk("<h1>`$title`</h1>");
        let h1 = &nodes[0];
        assert_eq!(h1.kind, NodeKind::Element);
        assert_eq!(h1.bindings, vec!["title".to_string()], "h1 reads $title");
        // The text hole is absorbed into the element's bindings, NOT a separate row.
        assert_eq!(
            nodes.len(),
            1,
            "one element row, not element + hole: {nodes:#?}"
        );
    }

    #[test]
    fn attr_hole_is_a_binding() {
        let nodes = walk("<a href=\"`$url`\">link</a>");
        assert_eq!(
            nodes[0].bindings,
            vec!["url".to_string()],
            "attr hole $url is a binding"
        );
        assert_eq!(nodes[0].label, "a");
    }

    #[test]
    fn dotted_hole_binding_is_base_name() {
        let nodes = walk("<span>`$item.title`</span>");
        assert_eq!(
            nodes[0].bindings,
            vec!["item".to_string()],
            "dotted hole dep is base 'item'"
        );
    }

    #[test]
    fn compound_hole_surfaces_every_signal() {
        // Reviewer B2-IR P2: a compound hole (`$a + $b`) reads MULTIPLE signals;
        // the binding chips must match the runtime's subscription edges (both a
        // AND b), not just the leading var. Delegates to collect_signal_deps.
        let nodes = walk("<span>`$a + $b`</span>");
        assert!(
            nodes[0].bindings.contains(&"a".to_string())
                && nodes[0].bindings.contains(&"b".to_string()),
            "compound hole surfaces BOTH signals: {:?}",
            nodes[0].bindings
        );
    }

    #[test]
    fn ids_are_stable_dotted_paths() {
        let nodes = walk("<div class=\"a\"><span>x</span><span>y</span></div>");
        assert_eq!(nodes[0].id, "0", "root div id");
        assert_eq!(nodes[1].id, "0.0", "first span");
        assert_eq!(nodes[2].id, "0.1", "second span");
    }

    #[test]
    fn label_prefers_class_then_id() {
        assert_eq!(walk("<div id=\"main\">x</div>")[0].label, "div#main");
        assert_eq!(
            walk("<div class=\"c\" id=\"main\">x</div>")[0].label,
            "div.c"
        );
    }

    /// EXACT source mapping, end to end: every authored element resolves to the
    /// byte range of the line it was written on.
    ///
    /// Locating by CONTENT can only ever say "something that looks like this is
    /// here now". A span says "you wrote this, there" — it survives duplicate
    /// text, it survives an element being emptied, and it points at source
    /// rather than at rendered output.
    #[test]
    fn every_authored_element_maps_to_its_exact_source_bytes() {
        let html = "<main>\n  <h1>Title</h1>\n\n  <p>Body</p>\n</main>";
        let nodes = walk(html);

        let span_text = |label: &str| {
            let node = nodes
                .iter()
                .find(|n| n.label.contains(label))
                .unwrap_or_else(|| {
                    panic!(
                        "no node labelled {label}: {:?}",
                        nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
                    )
                });
            let span = node
                .source_span
                .as_ref()
                .unwrap_or_else(|| panic!("{label} has no source span"));
            &html[span.start..span.end]
        };

        assert!(
            span_text("main").starts_with("<main>"),
            "main must map to its own line, got {:?}",
            span_text("main")
        );
        assert!(
            span_text("h1").contains("<h1>Title</h1>"),
            "h1 must map to line 2, got {:?}",
            span_text("h1")
        );
        // The blank line matters: an off-by-one here would point a reader at
        // whitespace and read as "the tool is broken".
        assert!(
            span_text("p").contains("<p>Body</p>"),
            "p must map to line 4 across the blank line, got {:?}",
            span_text("p")
        );
    }

    /// Two elements with IDENTICAL content must map to DIFFERENT places.
    ///
    /// This is precisely what content matching cannot do — it reports the pair
    /// as indistinguishable. A span tells them apart because they were written
    /// in different places, which is the whole reason to want exact mapping.
    #[test]
    fn identical_elements_still_map_to_distinct_source_spans() {
        let html = "<ul>\n  <li>Item</li>\n  <li>Item</li>\n</ul>";
        let nodes = walk(html);
        let spans: Vec<_> = nodes
            .iter()
            .filter(|n| n.label.contains("li"))
            .filter_map(|n| n.source_span.as_ref())
            .map(|s| (s.start, s.end))
            .collect();

        assert_eq!(spans.len(), 2, "both list items must be projected");
        assert_ne!(
            spans[0], spans[1],
            "identical rows must map to different source spans — this is the \
             case content matching provably cannot resolve"
        );
    }
}
