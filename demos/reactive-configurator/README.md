# Orbit — Reactive Plan Configurator

A **reactive-bindings showcase** for Spacetime. It demonstrates that
Spacetime's `$` signals work for *userland* state — not just animation
values — with **zero custom JavaScript**.

## Run

```sh
cargo run -- serve demos/reactive-configurator/
```

## What it shows

| Pattern | Where | What happens |
|---|---|---|
| `@data src: inline` | `index.st` | `$seats`, `$annual` — userland state as first-class signals |
| `@on &.click { $x <- … }` | `.seats-inc` etc. | clicks mutate signals via the expression form |
| `.sel { text: $expr }` | display bindings | file-scope reactive bindings repaint on signal change |
| arithmetic in bindings | `.monthly-cost` | `text: $seats * 12` |
| ternary in bindings | `.billing-label` | `text: $annual ? "Annual" : "Monthly"` |
| multi-signal expressions | `.grand-total` | depends on **both** `$seats` and `$annual`; recomputes when either changes |

## Architecture

Thin-shell hybrid: `index.html` provides the static markup + mount points;
`index.st` owns **all** state, interactions, reactive bindings, and styling.
There is no `<script>` of custom JS — the served
`/__spacetime/runtime.js` is generated entirely from `index.st`.

## Verified behaviour

- initial: 3 seats → monthly cost $36, total $36/month
- `+` twice → 5 seats → monthly cost $60
- toggle billing → "Annual" / "/year", total = `5 × 12 × 12 × 0.8 = $576`
- `−` clamps at 1 seat; `+` clamps at 100

See `screenshot.png`.
