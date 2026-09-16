# Reveal Lines, Reflow-Free — `@reveal split:"lines"` fused onto pretext

The clearest demonstration of the **fusion engine**: an *existing* core macro
gets faster and reflow-free by depending on the opt-in `text` module. `@reveal`
splits a headline into visual lines and animates them; with `stdlib/text`
imported, the line grouping is computed by the vendored **pretext** engine
(pure arithmetic) instead of reading `span.offsetTop` per word (which forces a
synchronous layout each time).

## Run

```sh
cargo run -- serve demos/reveal-lines-reflow-free/
```

## What it shows

| Pattern | Where | What happens |
|---|---|---|
| `@reveal(split: "lines")` | `.kinetic` | core kinetic-typography reveal, grouped into visual lines |
| pretext line grouping | when `stdlib/text` imported | `pretext.layoutWithLines` replaces the `offsetTop` read → no reflow on (re)split |
| progressive enhancement | — | the same macro still works WITHOUT the text module, via the `offsetTop` fallback |

## Why it matters

`stdlib/primitives/reveal.st` originally grouped visual lines by reading each
word's `offsetTop` — a synchronous layout per word, repeated on every resize. The
pretext path computes the same line boundaries from cached glyph widths, so the
reveal re-splits with **zero forced reflow**. The core primitive keeps a graceful
fallback, so it has *no hard dependency* on the vendored engine.

## What this page ships

```
core ST runtime + reveal primitive + text module
+ pretext.bundle.js (~44KB, demand-injected because @reveal references the pretext global)
NO other engines. NO hand-written JavaScript.
```

## Architecture

- `index.html` — one kinetic headline
- `index.st` — styling + the `@reveal` binding; no custom JavaScript
- the pretext branch in `reveal.st` is guarded by `typeof pretext !== 'undefined'`
