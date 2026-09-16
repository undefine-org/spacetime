# Literate Spacetime — Markdown as host (`.st.md`)

A `.st.md` file is a **Markdown document whose ` ```st ` fences ARE the
program**. Prose is prose; code runs where it sits. The compiler *tangles* the
document into ordinary `.st` source before parsing — prose becomes `@doc`
sections, fences splice through verbatim, and **document order is preserved**.

Use it when a page's prose and its behavior belong together: tutorials,
design-system guidelines, essays with live demos, runnable specs.

Spec: [`docs/specs/SIP-002-literate-spacetime-markdown-host.org`](../specs/SIP-002-literate-spacetime-markdown-host.org).
Worked examples: [`demos/starter-pack/index.st.md`](../../demos/starter-pack/index.st.md),
`projects/backdesk-new/brand.st.md`.

## Hello, literate

```markdown
# The counter

Press the button; the number after it is live.

`````````st hidden
@data inline $n : 0;
`````````

`````````st src
<button class="plus">+</button>
<span class="out"></span>

.plus { @on &.click { $n <- $n + 1; } }
.out  { text <- $n; }
`````````

That is the whole reactive loop.
```

Serve it like any entry: `cargo run -- serve mysite/` (an `index.st.md` is
found at `/`, and `essay.st.md` is served at `/essay`).

## Fence flags

The fence info string selects behavior. Flags are data, not syntax:

| info string | tangled (runs)? | source shown? | use |
|---|---|---|---|
| `st` | yes | no | code whose *effect* is the point |
| `st src` | yes | **yes**, above the result | teaching: show it AND run it |
| `st hidden` | yes | no | plumbing (imports, theme CSS) |
| anything else (`markdown`, `spacetime`, bare) | **no** | yes | quoting code you do *not* want to run |

That last row matters: a bare ` ``` ` or ` ```spacetime ` fence is **inert
prose**, rendered as a normal code block. It is how you document syntax without
executing it.

## The rules

**Fences share one file scope.** A signal declared in fence 1 is live in fence
9 — the document is one program, sectioned by prose. Anything legal at `.st`
file scope is legal in a fence: `@import`, `@type`, `@data`, `@template`, tag
literals, selector scopes, `@test`.

**Prose is inert.** Markdown's backtick is a code span; Spacetime's backtick is
a hole. They would collide, so prose is never hole-interpolated — one sigil,
one meaning. To show a live value in running text, drop into a fence and bind
an element:

```spacetime
<span class="n-out"></span>
.n-out { text <- $n; }
```

This is also *first-paint-correct*: a `text <- $sig` binding renders on the
server pass, where a hole in static shell markup would flash empty.

**The `stdlib/md` import is automatic.** A document with prose gets
`@import "stdlib/md"` synthesized (skipped if you already import it, or if the
document is all code). Zero boilerplate.

**Prose is rendered at build time.** `build`/`serve` run the vendored markdown
engine once, in the compiler's V8, and emit the prose as static HTML inside
`<section class="lit-prose">` — the exported page carries its text for
crawlers and no-JS readers, and there is no in-browser markdown pass to flash.
The same engine renders in-browser for a `@doc(content:)` that is not static.

## Diagnostics point at your file

The parser sees tangled text, so offsets are remapped back before rendering: an
error inside a fence reports **your** `.st.md` line and column, with your own
document in the snippet. A fence body splices verbatim, so the column is exact.

## Structuring a page: modules and ordering

A literate document should hold **prose and its own subject matter**. Page
furniture (sidebars, mastheads, shared layout CSS) belongs in a `.st` module
the document imports — and imports work in both directions: a `.st.md` can
import a `.st`, and a `.st` page can import a `.st.md`.

```
```st hidden
@import "./modules/reader-chrome.st"
```
```

Two ordering rules follow from how markup is assembled. Both are silent when
broken — the page compiles and every element exists, just in the wrong place:

1. **Imported element literals are appended AFTER the importing file's own
   markup.** So *in-flow* chrome — a hero, a masthead, a footer — must live in
   the document itself, or it lands at the bottom of the page. Only *off-flow*
   markup (a `position: fixed` sidebar, a portal) is safe to put in a module.
   Styles have no such constraint: put all of them in the module.
2. **A chrome fence must come before the opening prose.** Prose segments are
   emitted in document order like everything else, so a fence placed after the
   first paragraph renders its markup after that paragraph.

Layout note: a literate document's prose sections are emitted at **file
scope**, so there is no wrapper element to make a CSS-grid child of. A
two-column reader is therefore built with a fixed sidebar plus a matching
content inset (`margin-left`) on the content blocks, rather than
`grid-template-columns`.

## Authoring notes

- A `---` rule must sit **inside** a prose block, not alone between two fences.
  Isolated, it becomes its own prose segment and the vendored markdown renderer
  (snarkdown) emits a literal `---` — it needs surrounding block context.
- Snarkdown has **no table support**. Write tables as lists, or write a
  `<table>` in a fence wrapped in `<div class="lit-prose">` so it inherits the
  prose styling.
- Snarkdown emits a paragraph break as `<br />`, never `<p>`. Style
  `.lit-prose br { display: block; content: ""; margin-bottom: … }` for
  paragraph rhythm (`projects/lith-management/modules/globals.st`).
- Pages in a sub-folder (`missions/index.st.md`) belong to the nearest
  ancestor holding `_prelude.st`: every entry point (`serve`, `build`, `check`
  on a file or folder, `render`, the test runner) resolves the project that
  way, so brand forms declared there splice identically everywhere.
- Editor LSP features (hover, completion) do **not** work inside fences yet —
  tracked as FUP-140. Diagnostics do work.

## Literate mode vs `@doc(src:)`

Both render markdown; they answer different questions.

- **`.st.md`** — the page **owns** its prose. Prose and behavior are one file
  and cannot drift.
- **`@doc(src: "x.md")`** — the page **references** a document that belongs
  elsewhere (a canonical spec, a doc read standalone, a file cited by other
  work). Inlining a copy would guarantee the drift `@doc(src:)` prevents.

They compose: `projects/backdesk-new/process.st.md` is literate prose whose
chapters are `@doc(src:)` calls inside fences, because those chapters are cited
artifacts in their own right. `demos/spacetime-docs/` stays a plain `.st`
reader for exactly this reason — it renders `docs/testing/*.md`, which the
testing docs own.

## Why this shape

Knuth's literate programming needed two outputs: *tangle* → a compilable
program, *weave* → a printable document. Spacetime already guarantees that the
served page and the running program are one artifact (**unroll == hydrate**),
so **weave == tangle**: the woven document *is* the running program. Its demos
cannot drift from its prose, because they are the same source.
