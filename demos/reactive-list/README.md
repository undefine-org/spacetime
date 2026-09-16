# Reactive List — full-Spacetime `@each` showcase

A reactive list rendered **entirely from Spacetime** — no external
web-component `.js` files, no custom JavaScript. An inline `@template`
defines the row markup, `@data` seeds the list, `@each` renders it, and
clicks mutate the `$tasks` signal so the list re-renders reactively.

## Run

```sh
cargo run -- serve demos/reactive-list/
```

## What it shows

| Pattern | Where | What happens |
|---|---|---|
| inline `@template &task-row($task)` | `index.st` | per-item component markup — **no external `.js`** |
| `@data tasks src: inline` | `index.st` | the list as a first-class signal |
| `@each($tasks as $t) { &task-row($t); }` | `.task-list` | reactive list rendering over the signal |
| `@on &.click { $tasks <- … }` | add / remove / clear | mutate the signal → `@each` re-renders |
| `@computed $taskCount` + `.count { text: $taskCount }` | header | derived count, updates live |

## Why this matters

This is the **full-Spacetime** list path: the only file the browser needs
besides the generated runtime is a thin `index.html` shell with mount
points. The row component lives in `index.st` as an inline `@template`,
registered into `Spacetime.templates` at runtime and found by `@each`'s
`each-with-templates` via the same registry — no `<template
data-component>` web-component files required (contrast: `examples/cart-demo`
ships `product-card.js` / `cart-item.js`).

## Verified behaviour

- initial: 3 tasks, count 3
- **Add task** → 4 tasks, count 4
- **remove (×)** first row → 3 tasks, count 3
- **Clear all** → 0 tasks, count 0

All updates are reactive: mutating `$tasks` re-runs `@each` and the
`@computed` count. See `screenshot.png`.
