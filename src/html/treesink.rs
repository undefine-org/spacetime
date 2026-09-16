//! html5ever custom TreeSink → `ir::HtmlExpr` (PLAN-023 W1).
//!
//! Spacetime treats valid HTML as valid Spacetime. The W0 CST captures a top-level HTML
//! region as an `HTML_ELEMENT` node and exposes a pure-HTML *skeleton* (backtick holes
//! replaced by sentinel placeholders `\u{E000}<idx>\u{E001}`) plus the ordered hole sources.
//! This module parses that skeleton with html5ever — delegating all HTML5 correctness
//! (nesting, void elements, raw-text, implied tags, recovery) to the spec parser — and
//! converts the result DIRECTLY into `ir::HtmlExpr`, re-injecting holes as `HtmlExpr::Hole`
//! (text position) or `AttrPart::Hole` (attribute position).
//!
//! The sink itself builds a small `Rc<RefCell<Node>>` tree with parent links (enough for
//! html5ever's full tree-mutation surface); the `Node → HtmlExpr` conversion is where holes
//! are recovered. Holes carry `JsExpr::Raw(source)` in W1; the reactive emit phase (W3/W4),
//! which owns the per-element `EmitContext`, rewrites them into signal-aware accessors.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, ParseOpts, QualName, parse_fragment};

use crate::ir::{AttrPart, HtmlExpr, JsExpr};
use crate::syntax::cst::{HOLE_CLOSE, HOLE_OPEN};

/// A node in the intermediate tree the sink builds.
enum NodeData {
    Document,
    Element {
        name: QualName,
        attrs: RefCell<Vec<Attribute>>,
        /// 1-based line WITHIN the markup skeleton where this element's tag was
        /// tokenized. Combined with the enclosing `HtmlBlockAst.span`, this is
        /// what turns a DOM node back into an exact place in the `.st` file.
        line: u64,
    },
    Text(RefCell<String>),
    Comment,
    Pi,
}

struct Node {
    data: NodeData,
    parent: RefCell<Option<Handle>>,
    children: RefCell<Vec<Handle>>,
}

type Handle = Rc<Node>;

impl Node {
    fn new(data: NodeData) -> Handle {
        Rc::new(Node {
            data,
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
        })
    }
}

struct Sink {
    document: Handle,
    quirks: Cell<QuirksMode>,
    /// The line html5ever is currently tokenizing, updated through
    /// `set_current_line`. Read at `create_element` time so every element keeps
    /// the line its OWN tag appeared on — the tokenizer reports the line before
    /// processing the token, so this is correct at the moment of creation and
    /// would be wrong if read any later.
    current_line: Cell<u64>,
}

impl Sink {
    fn new() -> Self {
        Sink {
            document: Node::new(NodeData::Document),
            quirks: Cell::new(QuirksMode::NoQuirks),
            // html5ever reports the first line as 1; start there so an element
            // on line 1 is not mistaken for "unknown".
            current_line: Cell::new(1),
        }
    }

    /// Append a child handle to `parent`, setting the back-pointer.
    fn append_handle(parent: &Handle, child: Handle) {
        *child.parent.borrow_mut() = Some(parent.clone());
        parent.children.borrow_mut().push(child);
    }

    /// Append text to `parent`, merging into a trailing text node if present.
    fn append_text(parent: &Handle, text: &str) {
        if let Some(last) = parent.children.borrow().last()
            && let NodeData::Text(buf) = &last.data
        {
            buf.borrow_mut().push_str(text);
            return;
        }
        let node = Node::new(NodeData::Text(RefCell::new(text.to_string())));
        Self::append_handle(parent, node);
    }

    /// Index of `child` within `parent`'s child list.
    fn child_index(parent: &Handle, child: &Handle) -> Option<usize> {
        parent
            .children
            .borrow()
            .iter()
            .position(|c| Rc::ptr_eq(c, child))
    }
}

impl TreeSink for Sink {
    type Handle = Handle;
    type Output = Handle;
    type ElemName<'a> = &'a QualName;

    fn finish(self) -> Self::Output {
        self.document
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {
        // html5ever recovery is authoritative; non-fatal parse errors are tolerated.
    }

    fn get_document(&self) -> Self::Handle {
        self.document.clone()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        match &target.data {
            NodeData::Element { name, .. } => name,
            _ => panic!("elem_name called on non-element"),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        Node::new(NodeData::Element {
            name,
            attrs: RefCell::new(attrs),
            // The line of THIS element's own tag. html5ever reports a token's
            // line before processing it, so reading here — rather than at append
            // time — is what keeps a parent's line from being overwritten by its
            // children's.
            line: self.current_line.get(),
        })
    }

    /// html5ever's line callback. It fires only when the line CHANGES, so the
    /// cell must persist between calls rather than being reset per token.
    fn set_current_line(&self, line_number: u64) {
        self.current_line.set(line_number);
    }

    fn create_comment(&self, _text: StrTendril) -> Self::Handle {
        Node::new(NodeData::Comment)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        Node::new(NodeData::Pi)
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(node) => Self::append_handle(parent, node),
            NodeOrText::AppendText(text) => Self::append_text(parent, &text),
        }
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let parent = sibling.parent.borrow().clone();
        let Some(parent) = parent else { return };
        let idx = Self::child_index(&parent, sibling).unwrap_or(0);
        match new_node {
            NodeOrText::AppendNode(node) => {
                *node.parent.borrow_mut() = Some(parent.clone());
                parent.children.borrow_mut().insert(idx, node);
            }
            NodeOrText::AppendText(text) => {
                // Merge into preceding text node if present, else insert a new one.
                if idx > 0
                    && let Some(prev) = parent.children.borrow().get(idx - 1)
                    && let NodeData::Text(buf) = &prev.data
                {
                    buf.borrow_mut().push_str(&text);
                    return;
                }
                let node = Node::new(NodeData::Text(RefCell::new(text.to_string())));
                *node.parent.borrow_mut() = Some(parent.clone());
                parent.children.borrow_mut().insert(idx, node);
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if element.parent.borrow().is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        // Fragments have no doctype.
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        // We do not special-case <template>; treat its contents as ordinary children by
        // returning the element itself. (SSG/hydration template semantics live in stdlib.)
        target.clone()
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        Rc::ptr_eq(x, y)
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks.set(mode);
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        if let NodeData::Element {
            attrs: existing, ..
        } = &target.data
        {
            let mut existing = existing.borrow_mut();
            for new_attr in attrs {
                if !existing.iter().any(|a| a.name == new_attr.name) {
                    existing.push(new_attr);
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        let parent = target.parent.borrow().clone();
        if let Some(parent) = parent {
            if let Some(idx) = Self::child_index(&parent, target) {
                parent.children.borrow_mut().remove(idx);
            }
            *target.parent.borrow_mut() = None;
        }
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let moved: Vec<Handle> = node.children.borrow_mut().drain(..).collect();
        for child in moved {
            *child.parent.borrow_mut() = Some(new_parent.clone());
            new_parent.children.borrow_mut().push(child);
        }
    }
}

/// Parse an HTML *skeleton* (holes already replaced by `\u{E000}<idx>\u{E001}` sentinels) in
/// body context, returning top-level `HtmlExpr`s with holes re-injected from `holes`.
pub fn parse_html_skeleton(skeleton: &str, holes: &[String]) -> Vec<HtmlExpr> {
    let sink = Sink::new();
    let context = QualName::new(None, html5ever::ns!(html), html5ever::local_name!("body"));
    let document = parse_fragment(sink, ParseOpts::default(), context, vec![], false)
        .from_utf8()
        .one(skeleton.as_bytes());

    // Fragment parse nests real top-level nodes under a synthesized <html> root element.
    let mut out = Vec::new();
    for child in document.children.borrow().iter() {
        collect_children_as_html(child, holes, &mut out);
    }
    out
}

/// If `node` is the synthesized fragment root (an <html> element), descend into it; else
/// convert `node` itself. This flattens html5ever's hidden root so callers see the real
/// top-level elements.
fn collect_children_as_html(node: &Handle, holes: &[String], out: &mut Vec<HtmlExpr>) {
    if let NodeData::Element { name, attrs, .. } = &node.data
        && name.local.as_ref() == "html"
    {
        // GH-21: an authored top-level `<html lang="…">` wrapper has its attributes MERGED
        // into html5ever's synthesized fragment root (per the HTML spec, an `<html>` start tag
        // in body context folds its attrs into the root). The children are the real content;
        // but the `lang` would otherwise be lost. The shell owns the real `<html>` element, so
        // re-emit a `<html lang="…">` wrapper around the children — `render_page_shell`
        // (extract_head_elements) reads the lang and strips the wrapper. Host spelling works.
        let lang = attrs
            .borrow()
            .iter()
            .find(|a| a.name.local.as_ref() == "lang")
            .map(|a| a.value.clone());
        if let Some(lang) = lang {
            out.push(HtmlExpr::Raw(format!("<html lang=\"{}\">", escape_attr_value(&lang))));
            for child in node.children.borrow().iter() {
                if let Some(expr) = node_to_html(child, holes) {
                    out.push(expr);
                }
            }
            out.push(HtmlExpr::Raw("</html>".to_string()));
            return;
        }
        for child in node.children.borrow().iter() {
            if let Some(expr) = node_to_html(child, holes) {
                out.push(expr);
            }
        }
        return;
    }
    if let Some(expr) = node_to_html(node, holes) {
        out.push(expr);
    }
}

/// Escape an attribute value for round-tripping into a re-emitted tag.
fn escape_attr_value(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

/// Convert one sink `Node` into an `HtmlExpr`, recovering holes from text/attr sentinels.
fn node_to_html(node: &Handle, holes: &[String]) -> Option<HtmlExpr> {
    match &node.data {
        NodeData::Element { name, attrs, line } => {
            let tag = name.local.as_ref().to_string();
            let attr_list = attrs
                .borrow()
                .iter()
                .map(|a| {
                    let key = a.name.local.as_ref().to_string();
                    (key, split_hole_parts(&a.value, holes))
                })
                .collect();
            let mut children = Vec::new();
            for child in node.children.borrow().iter() {
                push_text_or_node(child, holes, &mut children);
            }
            Some(HtmlExpr::Element {
                tag,
                attrs: attr_list,
                children,
                // The authored line, carried out of the sink so downstream
                // tooling can map this element back to source exactly.
                line: u32::try_from(*line).ok(),
            })
        }
        NodeData::Text(buf) => {
            // A bare text node at element level: split into text + hole children.
            let mut parts = Vec::new();
            push_text_segments(&buf.borrow(), holes, &mut parts);
            // node_to_html returns a single expr; if a text node yields multiple parts we
            // can only return one. Callers use push_text_or_node for the multi-part path;
            // here we coalesce trivially (single text or single hole).
            match parts.len() {
                0 => None,
                1 => Some(parts.into_iter().next().unwrap()),
                _ => Some(HtmlExpr::Element {
                    // Should not happen via push_text_or_node; defensive wrap.
                    tag: "span".to_string(),
                    attrs: Vec::new(),
                    children: parts,
                    // SYNTHESIZED, not authored: no line, because claiming one
                    // would point a reader at markup they never wrote.
                    line: None,
                }),
            }
        }
        NodeData::Document | NodeData::Comment | NodeData::Pi => None,
    }
}

/// Append a child node's HtmlExpr(s) to `out`. Text nodes may expand into multiple
/// text/hole parts; element nodes contribute a single subtree.
fn push_text_or_node(node: &Handle, holes: &[String], out: &mut Vec<HtmlExpr>) {
    match &node.data {
        NodeData::Text(buf) => push_text_segments(&buf.borrow(), holes, out),
        _ => {
            if let Some(expr) = node_to_html(node, holes) {
                out.push(expr);
            }
        }
    }
}

/// Split a raw text string containing `\u{E000}<idx>\u{E001}` hole sentinels into an ordered
/// list of `HtmlExpr::Text` and `HtmlExpr::Hole` parts.
fn push_text_segments(text: &str, holes: &[String], out: &mut Vec<HtmlExpr>) {
    for seg in split_sentinels(text) {
        match seg {
            Seg::Text(s) if !s.is_empty() => out.push(HtmlExpr::Text(s)),
            Seg::Text(_) => {}
            Seg::Hole(idx) => {
                let src = holes.get(idx).cloned().unwrap_or_default();
                out.push(HtmlExpr::Hole(JsExpr::Raw(src)));
            }
        }
    }
}

/// Split an attribute value into literal/hole `AttrPart`s.
fn split_hole_parts(value: &str, holes: &[String]) -> Vec<AttrPart> {
    let mut parts = Vec::new();
    for seg in split_sentinels(value) {
        match seg {
            Seg::Text(s) if !s.is_empty() => parts.push(AttrPart::Lit(s)),
            Seg::Text(_) => {}
            Seg::Hole(idx) => {
                let src = holes.get(idx).cloned().unwrap_or_default();
                parts.push(AttrPart::Hole(JsExpr::Raw(src)));
            }
        }
    }
    if parts.is_empty() {
        parts.push(AttrPart::Lit(String::new()));
    }
    parts
}

enum Seg {
    Text(String),
    Hole(usize),
}

// ============================================================================
// Read-only element extraction (FEAT-095)
//
// The same html5ever Sink that powers skeleton→HtmlExpr also serves the
// document-structure extractors (class/id/tag/slot harvesting) that previously
// used lol_html. A pre-order DFS over the parsed Node tree reproduces lol_html's
// document order, with depth + parent index + an attribute map per element.
// ============================================================================

/// One visited element: tag (lowercased), its attributes as an ordered list of
/// (name, value) pairs, its depth (0 = top-level), and the index of its parent
/// element in the visit order (None for top-level). Mirrors the data lol_html's
/// `element!("*")` handler exposed, in the same pre-order document sequence.
pub struct VisitedElement {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub depth: usize,
    pub parent_index: Option<usize>,
}

impl VisitedElement {
    /// First value of an attribute by (case-insensitive) name, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Parse an HTML document/fragment and visit every element in pre-order
/// (document order). Each element is yielded as a [`VisitedElement`]. This is the
/// html5ever-backed replacement for lol_html `element!("*")` extraction walks.
///
/// Holes are irrelevant here (extraction is over real served/authored HTML, not
/// skeletons), so the sentinel machinery is bypassed: attribute values are taken
/// verbatim.
pub fn visit_elements(html: &str) -> Vec<VisitedElement> {
    let sink = Sink::new();
    let context = QualName::new(None, html5ever::ns!(html), html5ever::local_name!("body"));
    let document = parse_fragment(sink, ParseOpts::default(), context, vec![], false)
        .from_utf8()
        .one(html.as_bytes());

    let mut out = Vec::new();
    // Fragment parse nests real top-level nodes under a synthesized <html> root.
    // Descend through it so top-level authored elements report depth 0.
    for child in document.children.borrow().iter() {
        visit_node(child, 0, None, &mut out);
    }
    out
}

fn visit_node(
    node: &Handle,
    depth: usize,
    parent_index: Option<usize>,
    out: &mut Vec<VisitedElement>,
) {
    if let NodeData::Element { name, attrs, .. } = &node.data {
        let tag = name.local.as_ref().to_ascii_lowercase();
        // The fragment root <html> synthesized by html5ever is not a real authored
        // element; flatten through it without emitting or incrementing depth.
        if tag == "html" && parent_index.is_none() {
            for child in node.children.borrow().iter() {
                visit_node(child, depth, None, out);
            }
            return;
        }
        let attr_list: Vec<(String, String)> = attrs
            .borrow()
            .iter()
            .map(|a| (a.name.local.as_ref().to_string(), a.value.to_string()))
            .collect();
        let my_index = out.len();
        out.push(VisitedElement {
            tag,
            attrs: attr_list,
            depth,
            parent_index,
        });
        for child in node.children.borrow().iter() {
            visit_node(child, depth + 1, Some(my_index), out);
        }
    } else {
        // Non-element (text/comment/pi/document): descend without emitting.
        for child in node.children.borrow().iter() {
            visit_node(child, depth, parent_index, out);
        }
    }
}

/// Split a string on `\u{E000}<digits>\u{E001}` hole sentinels.
fn split_sentinels(s: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut buf = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == HOLE_OPEN {
            let mut digits = String::new();
            for d in chars.by_ref() {
                if d == HOLE_CLOSE {
                    break;
                }
                digits.push(d);
            }
            if !buf.is_empty() {
                segs.push(Seg::Text(std::mem::take(&mut buf)));
            }
            if let Ok(idx) = digits.parse::<usize>() {
                segs.push(Seg::Hole(idx));
            }
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        segs.push(Seg::Text(buf));
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_element() {
        let exprs = parse_html_skeleton("<main><h1>Hello</h1></main>", &[]);
        assert_eq!(exprs.len(), 1);
        match &exprs[0] {
            HtmlExpr::Element { tag, children, .. } => {
                assert_eq!(tag, "main");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    HtmlExpr::Element { tag, children, .. } => {
                        assert_eq!(tag, "h1");
                        assert_eq!(children, &vec![HtmlExpr::Text("Hello".into())]);
                    }
                    other => panic!("expected h1 element, got {other:?}"),
                }
            }
            other => panic!("expected main element, got {other:?}"),
        }
    }

    #[test]
    fn void_and_attrs() {
        let exprs = parse_html_skeleton(r#"<img src="a.png" alt="x">"#, &[]);
        assert_eq!(exprs.len(), 1);
        match &exprs[0] {
            HtmlExpr::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                assert_eq!(tag, "img");
                assert!(children.is_empty());
                assert!(attrs.iter().any(|(k, _)| k == "src"));
                assert!(attrs.iter().any(|(k, _)| k == "alt"));
            }
            other => panic!("expected img element, got {other:?}"),
        }
    }

    #[test]
    fn text_hole() {
        // <li>\u{E000}0\u{E001}</li>, hole 0 = "$x"
        let skeleton = format!("<li>{}0{}</li>", HOLE_OPEN, HOLE_CLOSE);
        let exprs = parse_html_skeleton(&skeleton, &["$x".to_string()]);
        match &exprs[0] {
            HtmlExpr::Element { tag, children, .. } => {
                assert_eq!(tag, "li");
                assert_eq!(children.len(), 1);
                assert_eq!(children[0], HtmlExpr::Hole(JsExpr::Raw("$x".into())));
            }
            other => panic!("expected li, got {other:?}"),
        }
    }

    #[test]
    fn text_around_hole() {
        // <p>Hi \u{E000}0\u{E001}!</p>, hole 0 = "$name"
        let skeleton = format!("<p>Hi {}0{}!</p>", HOLE_OPEN, HOLE_CLOSE);
        let exprs = parse_html_skeleton(&skeleton, &["$name".to_string()]);
        match &exprs[0] {
            HtmlExpr::Element { children, .. } => {
                assert_eq!(children.len(), 3);
                assert_eq!(children[0], HtmlExpr::Text("Hi ".into()));
                assert_eq!(children[1], HtmlExpr::Hole(JsExpr::Raw("$name".into())));
                assert_eq!(children[2], HtmlExpr::Text("!".into()));
            }
            other => panic!("expected p, got {other:?}"),
        }
    }

    #[test]
    fn attr_hole() {
        // <a href="\u{E000}0\u{E001}/x">go</a>, hole 0 = "$url"
        let skeleton = format!(r#"<a href="{}0{}/x">go</a>"#, HOLE_OPEN, HOLE_CLOSE);
        let exprs = parse_html_skeleton(&skeleton, &["$url".to_string()]);
        match &exprs[0] {
            HtmlExpr::Element { tag, attrs, .. } => {
                assert_eq!(tag, "a");
                let href = attrs.iter().find(|(k, _)| k == "href").unwrap();
                assert_eq!(
                    href.1,
                    vec![
                        AttrPart::Hole(JsExpr::Raw("$url".into())),
                        AttrPart::Lit("/x".into()),
                    ]
                );
            }
            other => panic!("expected a element, got {other:?}"),
        }
    }

    /// Elements must carry the line their OWN tag was written on.
    ///
    /// This is the difference between pointing a reader at the markup they
    /// wrote and guessing from what got rendered. A parent must not inherit a
    /// child's line, and a sibling further down must not inherit the first's.
    #[test]
    fn elements_carry_their_authored_line() {
        let skeleton = "<main>\n  <h1>Title</h1>\n\n  <p>Body</p>\n</main>";
        let exprs = parse_html_skeleton(skeleton, &[]);

        fn find<'a>(exprs: &'a [HtmlExpr], want: &str) -> Option<&'a HtmlExpr> {
            for e in exprs {
                if let HtmlExpr::Element { tag, children, .. } = e {
                    if tag == want {
                        return Some(e);
                    }
                    if let Some(hit) = find(children, want) {
                        return Some(hit);
                    }
                }
            }
            None
        }
        let line_of = |tag: &str| match find(&exprs, tag) {
            Some(HtmlExpr::Element { line, .. }) => *line,
            _ => panic!("missing <{tag}>"),
        };

        assert_eq!(line_of("main"), Some(1), "<main> is on line 1");
        assert_eq!(
            line_of("h1"),
            Some(2),
            "<h1> is on line 2 — a parent must not swallow its child's line, \
             nor a child its parent's"
        );
        assert_eq!(
            line_of("p"),
            Some(4),
            "<p> is on line 4: the blank line counts, because a reader counts it"
        );
    }
}
