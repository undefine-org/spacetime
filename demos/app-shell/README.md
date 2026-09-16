# App Shell — `@view` / `@portal` / `@effect` (FEAT-074)

The three Spacetime app-orchestration constructs composed into one declarative,
JavaScript-free admin shell.

| Construct | Here | Lowers to |
|-----------|------|-----------|
| `@view $pane { … }` | rail swaps the center pane (list / graph / settings) | reactive mount: re-mounts the matching `&template()` on `$pane` change |
| `@portal(when: $open)` | the modal escapes the layout grid to `<body>` | scroll-lock (refcounted) · focus-trap + restore · Esc-closes-topmost · `aria-modal` |
| `@effect(on: $rows) { … }` | bumps a live sync counter when the data changes | `ST.watch` + the shared mutation runner (debounce / immediate supported) |

```spacetime
.stage  { @view $pane { "list" => &listPane(); "graph" => &graphPane(); "settings" => &settingsPane(); } }
.modal  { @portal(when: $modalOpen); }
.app    { @effect(on: $rows) { $syncs <- $syncs + 1; } }
```

Build: `cargo run -- build demos/app-shell/` → `dist/`. Serve: `cargo run -- serve demos/app-shell/`.

Verified end-to-end in a real browser (pane swap, modal relocation + scroll-lock +
`aria-modal`, **real `Esc` keypress** closes + restores, effect fires on data change) and
via V8 (`feat074_app_shell_view_portal_effect_compose`).
