/// Built-in editable marks + blocks (PLAN-031 / FEAT-097).
///
/// The default vocabulary every `@editable` surface gets: the common inline
/// marks (bold/italic/underline/code/link) and block containers
/// (paragraph/headings/blockquote/figure). These are ORDINARY `@editable-mark`
/// / `@editable-block` declarations expressed in the open model — there is no
/// privileged built-in set baked into Rust. An author adds, removes, or overrides
/// any of these by declaring their own; the schema + registries fall out of
/// whatever is registered (AGENTS self-describing rule).
///
/// Import with `@import "stdlib/editable"` (the editable module entry re-exports
/// this), then list the ones you want in an `@editable` surface:
///   @editable(bind: $body) { marks: &bold &italic &link; blocks: &h2 &p; }

// --- inline marks ---

@editable-mark &bold(&sel)      { <strong>`&sel`</strong> }
@editable-mark &italic(&sel)    { <em>`&sel`</em> }
@editable-mark &underline(&sel) { <u>`&sel`</u> }
@editable-mark &strike(&sel)    { <s>`&sel`</s> }
@editable-mark &code(&sel)      { <code>`&sel`</code> }

/// A link carries a typed `href` (url): the editor offers an inline href editor,
/// and the `url` type makes `javascript:` a validation error server-side (FEAT-099)
/// rather than a scheme-sniff. Single root + single typed-hole attr + one `&sel`
/// ⇒ invertible ⇒ pasted `<a href>` lifts back to a link mark.
@editable-mark &link(&sel, $href url) { <a href="`$href`">`&sel`</a> }

// --- block containers ---

@editable-block &p(&sel)          { <p>`&sel`</p> }
@editable-block &h2(&sel)         { <h2>`&sel`</h2> }
@editable-block &h3(&sel)         { <h3>`&sel`</h3> }
@editable-block &blockquote(&sel) { <blockquote>`&sel`</blockquote> }

/// A figure nests the selection in a `<figcaption>` and carries typed image
/// attrs. The selection sits in a NESTED region (SelPosition::Nested) — still a
/// single root + single `&sel`, so it renders forward; nested-attr paste-lift is
/// refined in W3 (FEAT-101).
@editable-block &figure(&sel, $src url, $alt string = "") {
  <figure>
    <img src="`$src`" alt="`$alt`">
    <figcaption>`&sel`</figcaption>
  </figure>
}

/// A code block: the selection projects verbatim inside `<pre>`. Single root +
/// single `&sel` → invertible (pasted `<pre>` lifts back to a code block).
@editable-block &pre(&sel) { <pre>`&sel`</pre> }

// ── advanced show-pieces (prove the open model, FEAT-101) ───────────────────

/// A highlight mark with a typed `color` free param — demonstrates that ANY
/// inline span with a typed attribute is a mark with zero engine changes. The
/// `color` becomes a persisted, schema-validated attr; the projection binds it
/// inline. Pasted `<mark>` lifts back to a highlight.
@editable-mark &highlight(&sel, $color string = "yellow") {
  <mark class="`$color`">`&sel`</mark>
}

/// A keyboard-key mark — shows a purely-structural mark (no free params) is just
/// a one-line declaration; the tag (`kbd`) is its own inverse for paste-lift.
@editable-mark &kbd(&sel) { <kbd>`&sel`</kbd> }

// ── regions show-pieces (FUP-042) ───────────────────────────────────────────
//
// Regions make the document model RECURSIVE. A region is a named editable
// sub-space; lists (block region holding child blocks) and columns (parallel
// inline regions) fall out of ONE primitive. A non-`&sel` element param is a
// region; a trailing `[]` (`&items[]`) marks a BLOCK region (repeating child
// blocks) vs a bare inline region. Each region hole must appear exactly once —
// the invertibility invariant generalizes "one &sel" → "one hole per region".

/// A list item: one inline region, the degenerate single-region container that a
/// `&list` block region holds. (A flat `&li` would also work; naming the region
/// `sel` keeps it the familiar single-selection shape.)
@editable-block &item(&sel) { <li>`&sel`</li> }

/// An unordered list: ONE block region `&items[]` holding `&item` blocks. The
/// `[]` marks the region as block-kind; `<ul>` is its projection host, and pasted
/// `<ul><li>…` lifts back through the region.
@editable-block &list(&items[]) { <ul>`&items`</ul> }

/// Two-column layout: TWO parallel inline regions `&left` / `&right`, each its
/// own editable sub-space (the property a flat block could never express — the
/// old one-`&sel` rule forbade >1 selection hole). Each `.col` div hosts a
/// region; project emits a `data-st-region` host per side, lift recovers both.
@editable-block &columns(&left, &right) {
  <div class="st-columns">
    <div class="col">`&left`</div>
    <div class="col">`&right`</div>
  </div>
}
