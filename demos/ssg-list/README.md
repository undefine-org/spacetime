# SSG List — first-class HTML + inline `@each` + static unroll

A full-Spacetime page (no `index.html`, no `.js`) that demonstrates PLAN-023:

- **First-class HTML** — markup is written inline with the `<` sigil and `` `…` ``
  holes; no web-component files.
- **`@data inline`** — a static, compile-time-constant source.
- **`@data derive`** — a live count derived from the source.
- **Inline `@each`** — the list body is raw `<li>` markup, not a template invocation.
- **SSG unroll == hydrate** — because the source is *static*, the rows are unrolled
  into the **served HTML** at compile time (real `<li>` for crawlers / no-JS), and the
  runtime hydration produces the **identical** DOM (no flash, no duplication).

## Run

```sh
cargo run -- serve demos/ssg-list/
```

Then **view source** at `http://127.0.0.1:3333/` — you will see real list markup:

```html
<ul class="basket"><li class="basket__item">Apple</li>…</ul>
```

not an empty container filled by JavaScript. Disable JS and the list is still there.

See `docs/language/data-and-rendering.md` for the full reference.
