# Spacetime Starter Pack

A tutorial you **run**, not just read — and a literate Spacetime document
(`index.st.md`): one Markdown file whose ` ```st ` fences *are* the program.
The prose is prose; each demo runs where it sits, compiled from the very block
printed above it.

```
cargo run -- serve demos/starter-pack/
```

| chapter | teaches | its live demo |
|---|---|---|
| 1 · Hello | one-file model, sigil rules | the paragraph declared in its own fence |
| 2 · State | `@data inline`, `<-` flow, `@type`, `@data fold` | counter + live task-count badge |
| 3 · Composition | `@template` functions, `@each`, `@view` | card grid, mapped task list, experience switcher |
| 4 · Motion & data | `@on &.visible`, `@scroll scrub`, collections, `@doc` | the reveal card, revealing |

## Why it can't drift

Under SIP-002 the tangler turns prose into `@doc` sections and splices `st`
fences verbatim at their document position. So the code shown to the reader and
the code that runs are **the same source** — if an example breaks, the page
breaks. Fences tagged anything else (` ```spacetime `) are quoted, never run,
which is how the doc shows syntax it doesn't want to execute.

Fences also share one file scope: `$tasks` is declared in chapter 2 and mapped
in chapter 3. The essay is one program, sectioned by prose.

![Starter pack: dark tutorial page titled "Learn Spacetime by running it",
showing a source block followed by the live element that block declared.](screenshot.png)

## Where to go next

- `demos/app-shell/` — `@view` + `@portal` + `@effect` in one app shell
- `demos/reactive-list/` — the `@each` + template loop, minimal
- `demos/spacetime-docs/` — a full docs reader built on `@doc`
- `docs/specs/SIP-002-literate-spacetime-markdown-host.org` — how this file compiles
- `docs/language/` — the language reference
