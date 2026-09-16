/// Editable-everything blog demo (PLAN-031 / editable-everything).
///
/// A published blog that READS as a blog and EDITS in place. The author writes
/// only Spacetime — no custom JS, no editing wiring:
///   - `@richtext(bind: …)` renders the post body as final HTML and upgrades to a
///     structured rich-text editor when edit-mode is on (gate: edit-mode);
///   - `@edit-toggle` mounts the floating Edit/Done pill that flips edit-mode;
///   - plain bound text (`$post.title`) is click-to-edit via the dev provenance
///     path while edit-mode is on.
/// Run: cargo run -- serve projects/editable-blog/ -p 4455
/// Click "Edit" (bottom-right) → the title and the article body become editable
/// in place; the structured surface keeps headings/bold/italic/links/quotes.

@import "stdlib/editable"

// ── The post: fetched from a backing file so on-page edits PERSIST (FUP-047).
// The structured body is a rich-text document AST. Editing the @richtext surface
// in edit-mode writes back to content/post.json via the dev-save → EditJson rail.
@data fetch $post : "/content/post.json";

// ── The page: ordinary Spacetime markup. The data bindings carry provenance; the
//    @richtext block carries the structured surface. No editing code here. ──────
<main class="blog">
  <article class="post">
    <header class="post-head">
      <p class="post-kicker">Field notes</p>
      <h1 class="post-title" data-st-bind="$post.title">`$post.title`</h1>
      <p class="post-dateline">`$post.dateline`</p>
    </header>
    <img class="post-cover" data-st-bind="$post.cover" src="`$post.cover`" alt="Cover image">
    <div class="post-body"></div>
  </article>
  <footer class="blog-foot">
    <span class="dot"></span> Local preview &middot; click <strong>Edit</strong> to write in place
  </footer>
</main>

// Mount the structured body surface (edit-mode-gated) onto the body container.
.post-body {
  @richtext(bind: $post.body, marks: ["bold","italic","link"], blocks: ["p","h2","blockquote"]);
}

// Mount the Edit/Done pill.
@edit-toggle

// ─────────────────────────────────────────────────────────────────────────────
// Styling — an editorial blog. Serif body, generous measure, a single scarlet
// accent (matches the admin identity). The editor states reuse the SAME type;
// only subtle affordances appear in edit-mode.
// ─────────────────────────────────────────────────────────────────────────────
:root {
  --ink: #16161a;
  --muted: #6b6b76;
  --accent: #d62828;
  --rule: rgba(0,0,0,0.08);
  --paper: #fbfaf8;
}

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--paper);
  color: var(--ink);
  font-family: Georgia, "Iowan Old Style", "Times New Roman", serif;
  -webkit-font-smoothing: antialiased;
}

.blog {
  max-width: 42rem;
  margin: 0 auto;
  padding: 5rem 1.5rem 7rem;
}

.post-head { margin-bottom: 1.75rem; }

.post-cover {
  display: block;
  width: 100%;
  height: auto;
  border-radius: 10px;
  margin-bottom: 2.5rem;
}

.post-kicker {
  margin: 0 0 1rem;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--accent);
}

.post-title {
  margin: 0 0 0.75rem;
  font-size: 2.6rem;
  line-height: 1.08;
  letter-spacing: -0.018em;
  font-weight: 700;
}

.post-dateline {
  margin: 0;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  font-size: 0.85rem;
  color: var(--muted);
}

.post-body {
  font-size: 1.18rem;
  line-height: 1.72;
  border-top: 1px solid var(--rule);
  padding-top: 2rem;
}

.post-body p { margin: 0 0 1.4rem; }

.post-body h2 {
  margin: 2.4rem 0 1rem;
  font-size: 1.5rem;
  line-height: 1.2;
  letter-spacing: -0.01em;
}

.post-body blockquote {
  margin: 2rem 0;
  padding: 0.4rem 0 0.4rem 1.4rem;
  border-left: 3px solid var(--accent);
  font-style: italic;
  color: #33333a;
}

.post-body a {
  color: var(--accent);
  text-decoration: none;
  border-bottom: 1px solid rgba(214,40,40,0.35);
}

.post-body a:hover { border-bottom-color: var(--accent); }

.post-body strong { font-weight: 700; }

// In edit-mode the surface gets a soft frame + a writing affordance, so it reads
// as a document until you choose to edit, then clearly invites typing.
html[data-st-edit-mode] .post-body[contenteditable="true"] {
  outline: none;
  background: #fff;
  border-radius: 10px;
  padding: 1.6rem 1.4rem 0.4rem;
  margin: 0 -1.4rem;
  box-shadow: 0 0 0 1px var(--rule), 0 10px 30px rgba(0,0,0,0.06);
  transition: box-shadow 0.15s ease;
}

html[data-st-edit-mode] .post-title[contenteditable="true"] {
  outline: 2px solid rgba(214,40,40,0.5);
  outline-offset: 4px;
  border-radius: 4px;
}

.blog-foot {
  margin-top: 4rem;
  padding-top: 1.25rem;
  border-top: 1px solid var(--rule);
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  font-size: 0.82rem;
  color: var(--muted);
}

.blog-foot .dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: var(--accent);
  margin-right: 0.45rem;
  vertical-align: middle;
}
