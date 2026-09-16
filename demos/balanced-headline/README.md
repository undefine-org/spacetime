# Balanced Headline — `@balance` (pretext) vs CSS `text-wrap: balance`

A **lean + fusion** showcase for the Spacetime `text` module. Two identical
headlines sit side by side: one balanced by the vendored **pretext** engine via
`@balance`, one by the browser's CSS `text-wrap: balance`. Drag each card's right
edge to compare how the line breaks hold up under resize.

## Run

```sh
cargo run -- serve demos/balanced-headline/
```

## What it shows

| Pattern | Where | What happens |
|---|---|---|
| `@balance` | `.headline--pretext` | pretext binary-searches the narrowest width keeping the same line count → evenly ragged lines |
| `text-wrap: balance` | `.headline--css` | browser best-effort, for comparison |
| reflow-free | both cards | pretext relayouts arithmetically on cached widths — no DOM reflow per resize |
| CJK/RTL-correct | — | pretext uses `Intl.Segmenter`; CSS balance is weak/absent for non-Latin scripts |

## What this page ships

```
core ST runtime + text module (@balance → balance-text primitive)
+ pretext.bundle.js (~44KB, demand-injected because @balance fires)
NO other engines. NO hand-written JavaScript.
```

The pretext engine is shipped **only because** `@balance` is used on this page —
the lean rail (PLAN-024 W2) injects a vendored blob solely when a primitive that
references it fires. A page that imported `stdlib/text` but never called a text
primitive would ship **zero** pretext bytes.

## Architecture

- `index.html` — markup (two headline cards, resize handles via CSS `resize`)
- `index.st` — styling + the `@balance` binding; no custom JavaScript
- pretext is vendored as a git submodule under `stdlib/text/vendor/` and bundled
  into a single owned artifact (see `docs/stdlib/VENDORING.md`)
