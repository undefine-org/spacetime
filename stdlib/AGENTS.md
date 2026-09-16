# stdlib — organization charter

Rules for working inside `stdlib/`. `README.md` describes *what* the library is;
this file governs *how* it is organized and extended. Deeper `AGENTS.md` override.

## Philosophy (gingerbill, lived)

A *package manager* — the automation of transitive dependency hell — is the evil.
Spacetime has **no package manager** and never will. Instead, the Go ideal:

| concept | here |
|---|---|
| package | a **module** (a folder under `stdlib/`) |
| repository | git (submodules for vendored deps) |
| build system | the compiler (`cargo run -- …`) |
| package manager | **none — by design** |

The stdlib is batteries-included and vendored into the binary
(`src/stdlib_embedded.rs`). Every dependency is a liability **we own**: pinned,
provenance-recorded, wrapped behind our own primitives, manually bumped. Friction
on updates is a feature.

## Symbol system (who writes what)

| Symbol | Domain | Phase | Who |
|---|---|---|---|
| `$` | reactive data | runtime | users |
| `&` | DOM elements | runtime | users |
| `@` | directives | runtime | users |
| `%` | macros / primitives / vendor | compile | **library authors only** |

Users write `@ $ &`. You, editing stdlib, write `%`.

## Where JavaScript is allowed

JS exists in exactly **two** places. Nowhere else.

1. `%primitive … %emit js { … }` — the only hand-authored JS.
2. Owned `vendor/*.bundle.js` blobs — third-party engines, vendored and bundled.

A `@macro` composes primitives; it contains **no JS**. If you reach for JS in a
macro, you need a primitive. See `docs/antipatterns.md`.

## Module = vertical slice (the modular layout)

`stdlib/text/` is the reference shape. A module owns its primitives, macros,
vendored engine, and tests as one self-contained folder:

```
stdlib/<module>/
  MODULE.st            manifest: identity, public surface, %vendor (human-facing)
  index.st             public re-exports — what `@import "stdlib/<module>"` brings in
  vendor.st            %vendor declarations (imported by index.st so they register)
  macros/              @-surface (no JS)
  primitives/          %primitive %emit js
  vendor/
    <dep>/             git submodule, pinned @ commit
    <dep>.bundle.js    GENERATED, committed — the owned, shipped artifact
    <dep>-entry.ts     author-controlled bundler entry (re-exports onto a global)
    PROVENANCE.md      source · commit · license · why-we-depend
  tests/               *.test.st (real-DOM via the playwright_st harness)
```

Rules:

- **Opt-in.** A module is pulled in by `@import "stdlib/<module>"`. The resolver
  accepts both `stdlib/<module>.st` (flat) and `stdlib/<module>/index.st` (folder).
- **`%vendor` must be in the import graph.** Put it in `vendor.st` (or a file
  `index.st` imports), not only in `MODULE.st` — the declaration must register
  whenever the module is imported so demand-driven emission can inject the blob.
- **Tests live in `<module>/tests/`**, never loose. Name them `*.test.st`.
- **`@import` statements: prefer a trailing `;`.** A bodyless `@import "x"` with no
  `;` directly followed by a `.selector { … }` block parses cleanly now (the
  newline guard), but `;` is unambiguous and the safer convention.

> The legacy flat dirs (`primitives/`, `macros/`, `capture-types/`, …) predate the
> modular layout and remain the implicit `core` everyone gets. New cohesive
> features land as modules; do not grow the flat dirs with module-shaped work.

## Vendoring discipline (`%vendor`)

```st
%vendor <name> {
  source:  submodule "vendor/<dep>"   // a pinned git submodule (the provenance anchor)
  entry:   "vendor/<dep>-entry.ts"    // bundler root, MODULE-dir-relative
  bundle:  bun | direct               // bun: flatten an ESM/TS graph; direct: already 1 file
  exports: { a, b, … }                // symbols the bundle exposes on globalThis.<name>
  out:     "vendor/<dep>.bundle.js"   // generated, committed artifact
  license: <SPDX>
}
```

Workflow (maintainer-time only — users need no toolchain):

```sh
git submodule add <repo> stdlib/<module>/vendor/<dep>   # pin
cargo run -- vendor build <module>                      # regenerate the bundle (hot-reloadable)
# update PROVENANCE.md, re-run the module's tests, commit the bundle
```

- `cargo build` lazily regenerates the bundle from the pinned submodule
  (`build.rs`), skipping when the artifact matches the commit or `bun` is absent.
- `cargo run -- vendor build <module>` is a **stdlib hot-reload** concern — NOT a
  site build. It rewrites the on-disk blob; a running `serve` re-emits.
- Never re-source a `%vendor` from npm/CDN. Submodule pin only.
- Wrap the dep behind a primitive. The rest of Spacetime depends on your `@macro`,
  never on the vendored API — so an upstream change has a one-file blast radius.

Full spec: `docs/stdlib/VENDORING.md`.

## Lean law

> Importing a module ≠ shipping its engine. Only invoked capabilities emit.

A vendored blob's IIFE is injected into a page **only when** a primitive that
references its global actually fires. A page that `@import`s a module but never
calls one of its primitives ships **zero** of that engine's bytes. Don't defeat
this — keep engine usage behind primitives, never inline-load a vendored global.

## Tests

- Pure-compile behavior (parse, expand, emit content) → Rust tests
  (`tests/*.rs`, `cargo test`).
- Anything needing real DOM / Canvas / `Intl.Segmenter` → `<module>/tests/*.test.st`
  via the playwright_st harness (real Chromium).
- A vendored engine that needs browser APIs **cannot** be unit-tested under the
  headless V8 runtime — do not add a Rust fallback to make it testable there.
  Correctness (CJK/RTL/grapheme handling) lives in the vendored engine.

## Retiring syntax = write a `%migration` capsule (PLAN-076/079)

When the language retires an authoring surface, the retirement is a `%migration`
entry in `stdlib/migrations/entries/<date>-<id>.st` — stdlib DATA, never a Rust
match-arm (`check_bind_deprecation`-style special cases are deleted and stay
deleted). The entry is a self-contained **capsule**: it embeds the retired
`%macro` definitions VERBATIM (old grammar + `%binds` semantics, so the old
syntax keeps COMPILING through the window), one `%rewrite <id> { %match {…}
%into {…} }` rule per mechanically rewritable shape (backtick holes), and one
`%hint for @dir "…"` per directive needing human judgment. One migration may
retire SEVERAL macros — a wave that removes a whole surface is ONE capsule.
`%date` is the wave key. Load-time validation covers dates, embedded-name and
rule-id uniqueness, coverage (every retired directive has a rule OR a hint),
hole names, live-macro collisions, chain direction (incl. same-capsule hops),
and same-wave overlap. Scaffold new entries with `spacetime migrate --scaffold
<id>`; inspect live ones with `spacetime migrate <proj> --explain <id>`.
Full guide: `docs/language/migrations.md`.

## Don'ts

- ✗ No package manager, lockfile, `node_modules`, or runtime CDN fetch.
- ✗ No JS outside `%emit js` and owned `vendor/*.bundle.js`.
- ✗ No Rust reimplementation of a construct that belongs in stdlib/vendored code
  (the metasystem is self-describing — fix loading, don't bypass it).
- ✗ No vendoring logic in the `cargo run -- build <site>` path.
- ✗ No growing the legacy flat dirs with new module-shaped features.
