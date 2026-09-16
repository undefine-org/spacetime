# Namespaced Signals — grouped state under one object signal

Demonstrates **namespaced (dot-notation) signal field bindings** in
Spacetime: a single `$pan` object signal groups related fields
(`deltaX` / `deltaY` / `scale`), and file-scope bindings read individual
fields with `$pan.deltaX`. No custom JavaScript.

## Run

```sh
cargo run -- serve demos/namespaced-signals/
```

## What it shows

| Pattern | Where | What happens |
|---|---|---|
| `@data pan : any` (object) | `index.st` | one signal grouping related fields |
| `.val-x { text: $pan.deltaX }` | readout cells | namespaced (dotted) field binding |
| `@on &.click { $pan <- { …new } }` | pad buttons | replace the object → every field binding re-renders |

## Why this matters

Grouping related state under one object signal (gestures, pointer state,
transforms) keeps logically-connected values together. Each binding
`$pan.deltaX` reads the field off the root object stored in
`SpacetimeLocal['pan']` and subscribes to `local:pan:updated`, so any
mutation of the object reactively repaints all its field bindings — the
bridge from reactive bindings into the animation/gesture sphere.

## Verified behaviour (live, in-browser)

- init: `$pan` = `{deltaX:0, deltaY:0, scale:100}` — fields render 0 / 0 / 100
- → ×2 / ↑ / +zoom → deltaX 20, deltaY −10, scale 110
- reset → 0, 0, 100

All field bindings update reactively from the single `$pan` object.

### Note: the fix that made this work

This demo surfaced a real bug: `@data` sources populated only the
`@each`/`ST.afterData` data registry, never `SpacetimeLocal`, so
file-scope `$signal` bindings over a `@data` source (scalar *or* object)
never initialized — they only worked for `$state`/`@on`-mutated or
`@computed` signals. Fixed in `stdlib/primitives/data/source.st`: a
loaded `@data` source now also writes `SpacetimeLocal[name]` and
dispatches `local:<name>:updated`, unifying the two signal channels. See
`screenshot.png`.
