//! The canonical structural-id allocator (PLAN-064 B3.2).
//!
//! ONE definition of "which HtmlExpr children burn a sibling index, and in what
//! order" — consumed by BOTH the Structure IR projection (`html_walk`, which
//! builds navigator nodes) AND the stamped DOM builder (`emit_builder_*_stamped`,
//! which writes `data-st-node` onto guest elements). Sharing this walk is what
//! guarantees a navigator node id and its guest-DOM `data-st-node` are the SAME
//! string — the invariant the whole B3 selection spine rests on.
//!
//! ## The rules (the SINGLE source of truth)
//!
//! Over a sibling list of `HtmlExpr`, a left-to-right index is burned by:
//!   - an `Element` (it becomes `parent.child(idx)`), and
//!   - a BARE `Hole` in text position (a standalone `` `$x` `` that is not a
//!     direct child of an element being absorbed).
//! An index is NOT burned by:
//!   - `Text` / `Raw` runs (no structure), or
//!   - a `Hole` that sits directly inside an element (it is ABSORBED into that
//!     element's bindings, so it never gets its own id).
//!
//! An element's own children are walked with the SAME rules, EXCEPT its direct
//! text-position holes are first filtered out (absorbed), so they never burn a
//! child index. This exactly mirrors the pre-extraction `walk_children` body.

use crate::ir::HtmlExpr;

/// One structural child of a sibling list that owns an id: the sibling `index`
/// it burns and the `HtmlExpr` it points at. Text/Raw and absorbed direct holes
/// never appear here (they burn no index).
pub struct Slot<'a> {
    /// The sibling index this child occupies (0-based, in structural order).
    pub index: usize,
    /// The node at this slot.
    pub expr: &'a HtmlExpr,
}

/// Assign structural sibling indices to a child list per the canonical rules.
/// Returns one [`Slot`] per index-burning child (Element or bare Hole), in order.
/// Text/Raw are skipped. This is the SHARED allocator both the IR walk and the
/// stamped builder use, so their ids can never diverge.
pub fn structural_slots(exprs: &[HtmlExpr]) -> Vec<Slot<'_>> {
    let mut slots = Vec::new();
    let mut idx = 0usize;
    for expr in exprs {
        match expr {
            HtmlExpr::Element { .. } | HtmlExpr::Hole(_) => {
                slots.push(Slot { index: idx, expr });
                idx += 1;
            }
            HtmlExpr::Text(_)
            | HtmlExpr::Raw(_)
            | HtmlExpr::Html(_)
            | HtmlExpr::Markdown(_) => {}
        }
    }
    slots
}

/// The child list to RECURSE into for an element, with its direct text-position
/// holes filtered out (they are absorbed into the element's bindings and must not
/// burn a child index). Returns borrowed refs in source order. Both consumers use
/// this so an element's descendant ids match.
pub fn recurse_children(children: &[HtmlExpr]) -> Vec<&HtmlExpr> {
    children
        .iter()
        .filter(|c| !matches!(c, HtmlExpr::Hole(_)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::JsExpr;

    fn el(tag: &str) -> HtmlExpr {
        HtmlExpr::Element {
            tag: tag.to_string(),
            attrs: vec![],
            children: vec![],
            line: None,
        }
    }
    fn hole(s: &str) -> HtmlExpr {
        HtmlExpr::Hole(JsExpr::Raw(s.to_string()))
    }

    #[test]
    fn text_and_raw_burn_no_index() {
        let kids = vec![
            HtmlExpr::Text("x".into()),
            el("a"),
            HtmlExpr::Raw("<!--c-->".into()),
            el("b"),
        ];
        let slots = structural_slots(&kids);
        assert_eq!(slots.len(), 2, "only the 2 elements burn indices");
        assert_eq!(slots[0].index, 0);
        assert_eq!(
            slots[1].index, 1,
            "the 2nd element is index 1 (text/raw skipped)"
        );
    }

    #[test]
    fn bare_hole_burns_an_index() {
        // `$x`<section></section> -> hole=0, section=1.
        let kids = vec![hole("$x"), el("section")];
        let slots = structural_slots(&kids);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[1].index, 1, "section after a bare hole is index 1");
    }

    #[test]
    fn recurse_filters_direct_holes() {
        // <section>`$x`<p></p></section>: recursing section's children drops the
        // direct hole, so <p> is the FIRST structural child (index 0).
        let children = vec![hole("$x"), el("p")];
        let rec = recurse_children(&children);
        assert_eq!(rec.len(), 1, "direct hole filtered");
        let rec_owned: Vec<HtmlExpr> = rec.into_iter().cloned().collect();
        let slots = structural_slots(&rec_owned);
        assert_eq!(
            slots[0].index, 0,
            "p is index 0 after the direct hole is absorbed"
        );
    }
}
