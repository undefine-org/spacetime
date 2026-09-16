# Spacetime Docs — a documentation reader, written in Spacetime

A **`.st`-only** page (no `index.html`, no hand-written JavaScript, nothing from
npm) that **reads the project's own testing docs**. Every long-form section is a
Markdown file under [`docs/testing/`](../../docs/testing/), rendered to HTML at
build time by Spacetime's own [`md` module](../../stdlib/md/) (`@doc(src: …)` →
vendored snarkdown). The page cannot drift from the docs, because it **is** the
docs.

```sh
cargo run -- serve demos/spacetime-docs/
```

![Full-page screenshot of the Spacetime docs reader: a dark, glassy two-column
layout. A sticky left sidebar lists section links (Overview, Core grammar,
Fidelity ladder, Generative, Animation, Automation, CDP backend). The right
column opens with a balanced hero headline "Test it. Script it. Automate it. One
vocabulary." over a quiet WebGL glow, with a gradient "This page is alive" button
and a live click counter. Below, glass doc cards render the actual testing
Markdown — headings, prose, and syntax-styled fenced code blocks — straight from
docs/testing/*.md.](screenshot.png)

## What it demonstrates (all live, zero authored JS)

| part | primitives | the point |
|------|-----------|-----------|
| **Sidebar** | plain `<a href="#…">` anchors | section nav with zero JavaScript |
| **Hero** | `@balance` (text module) + `@scene` (WebGL) + `@on &.click` + `` `$clicks` `` | the headline is balanced by the text engine; one signal proves the page is alive |
| **Doc sections** | `@doc(src: "docs/testing/*.md")` (the `md` module) | each card renders a **canonical Markdown file** to HTML at build time — not a hand-copied duplicate |
| **Reveals** | `@scroll` | each doc card staggers in as you scroll |

## The idea: docs as a Spacetime program

The north star mirrors the test infrastructure: **one source describes behavior;
the runtime makes it real.** Here the *world* is "the testing docs." The page
holds no prose of its own — it points `@doc` at the real Markdown:

```spacetime
@import "stdlib/md";

<section class="doc-card">
  <div class="doc__body" data-doc="guide"></div>
</section>

[data-doc="guide"] { @doc(src: "docs/testing/GUIDE.md") }
```

At build time the compiler reads `GUIDE.md`, inlines its bytes, and the vendored
snarkdown engine turns them into `<h1>`/`<h2>`/`<pre class="code">`/`<code>` in
the browser. Edit the Markdown, rebuild — the reader updates. The docs are the
feature.

## How it's wired

- **`md` module** — [`stdlib/md/`](../../stdlib/md/): a `render-markdown` primitive
  (`%emit js` calling `snarkdown`) behind the `@doc` macro.
- **Vendored engine** — [`snarkdown`](../../stdlib/md/vendor/) pinned as a git
  submodule, bundled to a ~2KB IIFE, demand-injected only when a page uses `@doc`
  (same rail as `stdlib/text`'s pretext).
- **Build-time inlining** — `compiler.rs::inject_doc_content` reads the `src` file
  and moves its bytes into the render call; a missing file renders a visible error,
  never a silent blank.

No custom JavaScript. No `index.html`. The whole reader is one `.st` file plus
the Markdown it documents.

## The macro reference (`/reference`)

[`reference.st`](reference.st) is a **living map of Spacetime's dispatch machine**
— served at `/reference` (a `.st`-only multi-page project, FEAT-093).

![Macro reference: a dark page titled "Every directive. Every form. Live." with a
search box reading "266 macros", a row of kind-filter chips (all/data/events/
scene/animation/motion/text/test), and a grid of cards. Each card shows a
directive like @after with a MACRO badge and its form signature rendered as
colour-coded chips — purple directive, teal $bindings, dim :types, blue params —
plus a one-line doc.](reference-screenshot.png)

The cards ARE the compiler's own `%macro` forms: `@data dispatch $macros` projects
every dispatchable macro as `{ directive, signature, tokens, doc, kind, … }`, and
`@each` renders one card each. The signature is rendered token-by-token as colored
chips by **dispatch role** (directive / binding / type / literal / param / delim).
Add a `%macro` to the stdlib and a card appears next build.

Live, zero authored JS:
- **Search** — `@on &.input { $q <- $.value }` drives a `$q` signal; an
  `each-inline-filtered` `when` clause (FEAT-094) hides non-matching cards.
- **Kind chips** — `@on &.click { $kind <- $.dataset.kind }`.

### Watch dispatch resolve (the live probe)

![The live dispatch probe: an input containing `@data fetch $api : "/users";` and
below it a scored candidate table. The winning row "✓ +13 data-fetch-kind" glows
teal with its per-term breakdown (literal fetch +3, $name:binding +2, …). Losing
rows show red "literal `dispatch` -100" / orange "missing" reasons explaining why
they lost.](reference-probe.png)

Type a directive call into the probe and watch Spacetime resolve it **keystroke by
keystroke**: the winning `%macro` glows, losers show *why* they lost (`literal-miss
-100`, `missing`, `type-mismatch`). It's a thin view over the compiler's OWN
scorer — the `/__spacetime/dispatch` endpoint (FEAT-091) runs the same
`build_dispatch_probe` that `cargo run -- inspect --layer dispatch --probe "<call>"`
prints, so the playground can never disagree with how dispatch actually works.

- **`dispatch` module** — [`stdlib/dispatch/`](../../stdlib/dispatch/): a
  `@dispatch-probe` macro over a `%emit js` primitive (the only JS) that posts the
  input to the endpoint and renders the table.
- **Substrate** — `inspect --layer dispatch` + `explain_form_match` (FEAT-087) +
  the `E0923` ambiguous-dispatch warning (FEAT-088).

## Modules — `@use` and namespaces (FEAT-118)

The reader imports the `text` and `md` modules with `@import` (the global flood).
The newer `@use` form *scopes* an import under a namespace instead — so two
modules can each define `@badge` without colliding, and a page can pull in exactly
the directives it wants under the name it wants:

```spacetime
@use "std:scene" as s            // namespaced import, bound to alias `s`
.hero { @s/camera(fov: "75") }   // alias-qualified reference

@use "std:scene" only (camera)   // PureScript-style import-list filter
```

References resolve through a per-file *import scope*: a bare `@camera`, a
path-qualified `@scene/camera`, or an alias-qualified `@s/camera` all reach the
module's `@camera`. Typos (`@scn/camera`) error with `E0926`; names excluded by
`only`/`hiding` error with `E0927`; a bare name exported by two open modules warns
`E0924`. A library folder names itself with a `MODULE.st` (`%module`, `%public`,
`%reexport`, and the Elixir-style active `%using %claims` hook).

Runnable example: [`examples/modules/`](../../examples/modules/) — `cargo run --
build examples/modules/index.st -o /tmp/out`. Full guide:
[`docs/language/module-system.md`](../../docs/language/module-system.md).
