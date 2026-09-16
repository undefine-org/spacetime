# Learn Spacetime by running it

This page is a **literate Spacetime document** — one `index.st.md` file. The
prose you are reading is markdown; the code blocks below are not illustrations
of Spacetime, they *are* this page's program. Every demo runs where it sits,
compiled from the very block printed above it.

Serve it yourself:

```
cargo run -- serve demos/starter-pack/
```


## 1 · Hello, Spacetime

A Spacetime page is **one file**. Markup, style, state, and behavior live
together; the compiler splits them into served HTML, CSS, and a JS runtime for
you. No `index.html` required, no build config, no npm.

```st src
<p class="hello">Hello, Spacetime — this paragraph was declared in a fence.</p>

.hello {
  padding: 0.9rem 1.1rem;
  border-left: 2px solid #5eead4;
  background: #10131a;
}
```

Three things to notice:

- **Tag literals are first-class.** `<p>…</p>` at file scope IS page markup —
  not a string, not JSX.
- **Selectors are scopes.** `.hello { … }` holds CSS *and* (as you'll see
  next) state and behavior for the elements it matches.
- **The dev server live-reloads.** Edit this file, save, the page updates.

### The one-sigil rule

Spacetime is strict about sigils — each has exactly one meaning, everywhere:

- `` ` `` (backtick) is THE hole: it interpolates a value into markup.
- `$name` introduces a **value** (a signal, a param).
- `&name` introduces an **element** (a template, an element param).
- `@word` is a **directive** (the author-facing behavior surface).

In this document that rule has a pleasant consequence: markdown's backtick
stays markdown's. Prose is inert — a code span here never becomes a hole. To
show a live value inside a sentence you drop into a fence and bind an element,
which you'll see in the next chapter.

---

## 2 · State: signals in, reactivity out

State is declared where it belongs — on a scope. `@data inline` gives the page
a signal; any element scope can own its own.

```st src
@data inline $count : 0;
```

Events write signals, and bindings read them. `<-` means *reactive flow into*:
the event writes, the binding renders. That is the entire loop — no hooks, no
setState, no subscriptions.

```st src
<div class="counter">
  <button class="minus" type="button">−</button>
  <span class="count-out">0</span>
  <button class="plus" type="button">+</button>
</div>

.plus     { @on &.click { $count <- $count + 1; } }
.minus    { @on &.click { $count <- $count - 1; } }
.count-out { text <- $count; }
```

Press the buttons — the number above is this page's own state.

> **Why a binding and not a hole?** `text <- $count` renders correctly on first
> paint. A hole in static shell markup has no value until the runtime hydrates,
> so a readout would flash empty. Bindings are the first-paint-correct choice.

### Typed collections

Data gets a schema. A `@type` powers validation *and* the generated admin UI
(with `@cms` display hints, when the data is server-backed):

```st src
@type Task {
  id: string;
  label: string;
}

@data inline $tasks Task[] : [
  { "id": "t1", "label": "Read chapter one" },
  { "id": "t2", "label": "Click the counter" },
  { "id": "t3", "label": "Swap the experience below" }
];
```

### Derived state

`@data fold` computes a signal from another — here, a live count of the
collection above:

```st src
@data fold $taskCount number from $tasks : acc + 1 ;
```

```st
<p class="badge">tasks in the collection: <span class="badge-n"></span></p>

.badge-n { text <- $taskCount; }
```


## 3 · Composition: templates are functions

`@template` defines a component **as a function**: parameters in, markup out.
This is how Spacetime experiences compose.

```st src
@template &concept-card($title, $body) {
  <article class="ccard">
    <h4 class="ccard__title">`$title`</h4>
    <p class="ccard__body">`$body`</p>
  </article>
}
```

A selector scope *calls* it. Literal arguments render at build time — the
served HTML contains real markup, so crawlers and no-JS readers see content:

```st src
<div class="cards"></div>

.cards {
  &concept-card("Declared", "Say it once, in plain terms.");
  &concept-card("Reactive", "Signals flow; the DOM follows.");
  &concept-card("Composed", "Functions all the way down.");
}
```

### Mapping data onto a template

`@each` applies the function over a collection — the same `$tasks` declared in
chapter 2, because **every fence in this document shares one file scope**. The
essay is one program, sectioned by prose.

```st src
@template &task-row($t) {
  <li class="trow" data-id="`$t.id`">
    <span class="trow__dot"></span>
    <span>`$t.label`</span>
  </li>
}
```

```st src
<ul class="task-list"></ul>

.task-list {
  @each($tasks as $t) { &task-row($t); }
}
```

Static data unrolls into the served HTML and the runtime takes over the same
DOM — *unroll == hydrate*, one artifact rather than a server copy and a client
copy that can disagree.

### Swapping whole experiences: `@view`

`@view` mounts the template matching a signal, and re-mounts when it changes.
This is page-level composition: one signal chooses which experience runs.

```st src
@data inline $pane string: "cards";

@template &cardsPane() {
  <p class="pane-note">Experience A — three cards from one template function.</p>
}

@template &listPane() {
  <p class="pane-note">Experience B — a list mapped from typed data.</p>
}
```

```st src
<div class="switch">
  <button class="to-cards" type="button">cards</button>
  <button class="to-list" type="button">list</button>
</div>
<div class="stage"></div>

.stage    { @view $pane { "cards" => &cardsPane(); "list" => &listPane(); } }
.to-cards { @on &.click { $pane <- "cards"; } }
.to-list  { @on &.click { $pane <- "list"; } }
```

Press the buttons: one signal, two mounted experiences. `@match` is its
render-once sibling — dispatch without re-mounting.

---

## 4 · Motion, data, and the rest of the surface

### Reveal on visibility

`@on &.visible` publishes a named 0→1 progress variable (`--st-<name>`) that CSS
consumes — animation as *data*, not imperative code:

```st src
<div class="reveal-demo">This card revealed with the pattern described here.</div>

.reveal-demo {
  padding: 1rem 1.2rem;
  border: 1px solid #232833;
  border-radius: 8px;
  background: #10131a;
  opacity: var(--st-demo-reveal, 0);
  transform: translateY(calc((1 - var(--st-demo-reveal, 0)) * 20px));
}

.reveal-demo { @on &.visible(650ms) as $demo-reveal { opacity: 0 -> 1; } }
```

### Scroll-driven (scrubbed) animation

`@on &.scroll` with `scrub: true` ties the variable to scroll progress — the reader
owns the timeline. Windowed `clamp()` slices sequence sub-animations:

```spacetime
.track   { @on &.scroll(start: 0, end: 1, scrub: true) as $story { --p: 0 -> 1; } }
.step--2 { opacity: clamp(0, (var(--st-story, 0) - 0.3) / 0.1, 1); }
```

That block is tagged `spacetime`, not `st` — so it is **quoted, never run**.
Both spellings matter in a literate document: `st` fences are the program,
every other language is an illustration.

### Server-backed collections

Swap `@data inline` for a file-backed collection and the content becomes
editable in the generated admin (serve with `--debug`, open
`/__spacetime/admin/`):

```spacetime
@data collection $faqs Faq[] from "/data/faq.json" via dev-ws {
  initial: [];
}
@cms(Faq) { title: q; subtitle: a; }
```

### Docs as a feature — and this page as the proof

`@doc` renders a markdown file at build time. A literate `.st.md` is that idea
turned inside out: instead of a `.st` page pulling in documentation, a markdown
document hosts the program. The tangler turns prose into `@doc` sections and
splices `st` fences verbatim at their document position — which is why every
demo above appears exactly where its explanation does.

Knuth's literate programming needed two outputs: *tangle* (a program) and
*weave* (a document). Spacetime already guarantees that the served page and the
running program are one artifact, so **weave == tangle**: this document is the
running program. Its demos cannot drift from its prose, because they are the
same source.

---

## Where to go next

- `demos/app-shell/` — `@view` + `@portal` + `@effect` composed into an app shell
- `demos/reactive-list/` — the `@each` + template loop, minimal
- `demos/spacetime-docs/` — a full docs reader built on `@doc`
- `docs/testing/GUIDE.md` — the test grammar (`@test` / `@mount` / `@when` / `@then`)
- `docs/specs/SIP-002-literate-spacetime-markdown-host.org` — how this file compiles
- `docs/language/` — the deep language reference

### The rules that keep you honest

- **No custom JavaScript.** If a behavior needs it, the feature belongs in the
  toolchain, not your page.
- **One sigil, one meaning.** Never overload the backtick, `$`, `&`, or `@`.
- **Unroll == hydrate.** Static data renders into served HTML; the runtime
  converges on the same DOM.

```st hidden
body {
  background: #0d0f12;
  color: #e9ecf1;
  font-family: "Inter", system-ui, sans-serif;
  line-height: 1.65;
  margin: 0;
}

main, .lit-prose, .lit-src, .counter, .cards, .task-list, .switch, .stage,
.badge, .reveal-demo, .hello {
  max-width: 46rem;
  margin-inline: auto;
}

.lit-prose { padding-inline: 1.2rem; }

.lit-prose h1 {
  font-size: clamp(2rem, 5vw, 3.2rem);
  font-weight: 800;
  letter-spacing: -0.03em;
  line-height: 1.05;
  margin: 2.5rem 0 1rem;
}

.lit-prose h2 {
  font-size: 1.5rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: #5eead4;
  margin: 2.6rem 0 0.8rem;
}

.lit-prose h3 {
  font-size: 1.05rem;
  font-weight: 700;
  margin: 1.8rem 0 0.6rem;
}

.lit-prose p { color: #aab2c0; margin: 0 0 0.9rem; }
.lit-prose strong { color: #e9ecf1; }
.lit-prose ul { color: #aab2c0; margin: 0 0 1rem 1.2rem; }
.lit-prose li { margin-bottom: 0.4rem; }
.lit-prose hr { border: none; border-top: 1px solid #232833; margin: 2.5rem 0; }

.lit-prose blockquote {
  border-left: 2px solid #5eead4;
  margin: 0 0 1rem;
  padding: 0.2rem 0 0.2rem 1rem;
  color: #8b93a3;
}

.lit-prose code {
  font-family: "SF Mono", ui-monospace, monospace;
  font-size: 0.85em;
  background: #161a21;
  border: 1px solid #232833;
  border-radius: 4px;
  padding: 0.1em 0.35em;
  color: #d7dce4;
}

.lit-prose pre {
  background: #10131a;
  border: 1px solid #232833;
  border-radius: 8px;
  padding: 1rem 1.2rem;
  overflow-x: auto;
}

.lit-prose pre code { background: none; border: none; padding: 0; }

.lit-src {
  background: #0b0e13;
  border: 1px solid #232833;
  border-left: 2px solid #5eead4;
  border-radius: 8px;
  padding: 1rem 1.2rem;
  overflow-x: auto;
  font-family: "SF Mono", ui-monospace, monospace;
  font-size: 0.8rem;
  line-height: 1.6;
  color: #d7dce4;
  margin: 1rem auto 0.9rem;
}

.counter { display: flex; align-items: center; gap: 1rem; margin-block: 1rem; }

.counter button {
  width: 2.6rem;
  height: 2.6rem;
  font-size: 1.3rem;
  background: #161a21;
  color: #e9ecf1;
  border: 1px solid #2c3340;
  border-radius: 8px;
  cursor: pointer;
}

.counter button:hover { border-color: #5eead4; }

.count-out {
  font-family: "SF Mono", ui-monospace, monospace;
  font-size: 1.6rem;
  min-width: 2.5ch;
  text-align: center;
}

.badge { font-family: "SF Mono", ui-monospace, monospace; font-size: 0.85rem; color: #8b93a3; }
.badge-n { color: #5eead4; }

.cards { display: flex; flex-direction: column; gap: 0.6rem; margin-block: 1rem; }

.ccard { background: #161a21; border: 1px solid #232833; border-radius: 8px; padding: 0.8rem 1rem; }
.ccard__title { font-size: 0.95rem; font-weight: 700; color: #5eead4; margin: 0 0 0.2rem; }
.ccard__body { font-size: 0.85rem; color: #8b93a3; margin: 0; }

.task-list { list-style: none; padding: 0; display: flex; flex-direction: column; gap: 0.5rem; margin-block: 1rem; }

.trow {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  background: #161a21;
  border: 1px solid #232833;
  border-radius: 8px;
  padding: 0.65rem 0.9rem;
  font-size: 0.9rem;
}

.trow__dot { width: 8px; height: 8px; border-radius: 50%; background: #5eead4; flex-shrink: 0; }

.switch { display: flex; gap: 0.6rem; margin-block: 1rem 0.8rem; }

.switch button {
  font-family: "SF Mono", ui-monospace, monospace;
  font-size: 0.75rem;
  letter-spacing: 0.08em;
  padding: 0.5rem 1rem;
  background: #161a21;
  color: #e9ecf1;
  border: 1px solid #2c3340;
  border-radius: 999px;
  cursor: pointer;
}

.switch button:hover { border-color: #5eead4; }

.pane-note { color: #8b93a3; font-size: 0.9rem; }

@media (prefers-reduced-motion: reduce) {
  .reveal-demo { opacity: 1; transform: none; }
}
```
