/// Markdown Render Primitive
///
/// Renders a Markdown string into an element's innerHTML using the vendored
/// snarkdown engine (a ~1KB CommonMark-ish parser). The `snarkdown` global is
/// provided by the vendored bundle (see stdlib/md/MODULE.st `%vendor`), emitted
/// into the page ONLY when a primitive that references it fires — so pages that
/// never render markdown ship none of its bytes.
///
/// The `content` param is the RAW markdown text. For `@doc(src: "…/x.md")` the
/// compiler inlines the file's bytes at build time (see compiler.rs
/// `inject_doc_content`), so the served HTML is produced from the same markdown
/// the docs are written in — the page cannot drift from the docs.
///
/// @param &el      - Element whose innerHTML to populate
/// @param content  - Raw markdown source text

%primitive render-markdown(&el, content: string) {
  %emit js {
    const md = %content;
    if (typeof snarkdown === 'undefined') {
      console.warn('[Spacetime] render-markdown: snarkdown engine not loaded.');
    } else if (%&el) {
      %&el.innerHTML = snarkdown(md);
    }
  }

  %exports {}
}
