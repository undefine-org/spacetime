# Fit to Box — `@fit` shrink-to-fit

A **lean + fusion** showcase for the Spacetime `text` module. Each box has a
draggable right edge; the headline inside chooses the largest font-size that fits
one line at the current width — powered by the vendored **pretext** engine via
`@fit`. No overflow flash, no DOM reflow.

## Run

```sh
cargo run -- serve demos/fit-to-box/
```

## What it shows

| Pattern | Where | What happens |
|---|---|---|
| `@fit(min, max, maxLines)` | `.fit-line` | binary-searches the largest font-size fitting `maxLines` at the box width |
| no overflow | both boxes | candidate sizes are measured arithmetically before applying — text never spills then snaps back |
| reflow-free | resize | pretext measures on cached widths; no synchronous layout per drag |

## What this page ships

```
core ST runtime + text module (@fit → fit-text primitive)
+ pretext.bundle.js (~44KB, demand-injected because @fit fires)
NO other engines. NO hand-written JavaScript.
```

A sibling page importing `stdlib/text` but never calling a text primitive ships
**zero** pretext bytes — the lean rail (PLAN-024 W2) only injects a vendored blob
when a primitive that references it fires.

## Architecture

- `index.html` — two resizable boxes
- `index.st` — styling + the `@fit` binding; no custom JavaScript
