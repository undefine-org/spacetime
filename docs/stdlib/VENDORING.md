# Stdlib Vendoring & Module Architecture

> Spec for PLAN-024. Status: W0 (contract). The implementation in W1+ is
> verified against this document.

## Philosophy

gingerbill's four distinctions (*Package Managers are Evil*, 2025):

| concept | what it is | Spacetime's stance |
|---------|-----------|--------------------|
| package | a unit of code | ✓ the **module** (a folder under `stdlib/`) |
| repository | where packages are discovered | ✓ git (submodules) |
| build system | turns source → artifact | ✓ the compiler itself (`cargo run -- …`) |
| **package manager** | **automates dependency resolution** | ✗ **none. by design.** |

The *package manager* is the evil — the automation of dependency hell
(transitive resolution, lockfiles, "its dependencies' dependencies"). Spacetime
already lives the Go ideal: a batteries-included stdlib compiled into the binary,
no resolver, no lockfile, no transitive graph.

We do **not** add a package manager. We add a **disciplined vendoring unit**:

- **pin** — a git submodule at a specific commit.
- **provenance** — `PROVENANCE.md` records source · commit · license · *why we depend*.
- **own** — every dependency is wrapped behind one of *our* primitives. The rest
  of Spacetime depends on `@measure`, never on the vendored library's API. Blast
  radius of an upstream change = one primitive file.
- **manual bump** — updating means moving the submodule SHA by hand and
  re-bundling. Friction is a feature: you are responsible for the liability.

## Module = vertical vendoring slice

A module is a self-contained folder under `stdlib/`. The shape generalizes
`stdlib/mobile/`:

```
stdlib/<module>/
  MODULE.st            manifest: identity, public surface, %vendor blocks
  index.st             @import re-exports — what `@import "stdlib/<module>"` brings in
  macros/              @-surface (user-facing, no JS)
  primitives/          %primitive %emit js — the ONLY hand-authored JS
  vendor/
    <dep>/             git submodule, pinned @ commit
    <dep>.bundle.js    GENERATED single-file artifact (the owned, shipped blob)
    PROVENANCE.md      source repo · commit SHA · license · why-we-depend
  tests/               *.test.st — run via the playwright_st harness (real Chromium)
```

Rules:

1. **A module is a vertical slice.** It owns its primitives, macros, vendored
   engine, and tests. One module = one mental affordance.
2. **`tests/` subfolder.** All `*.test.st` for a module live in `<module>/tests/`,
   never loose. (The stdlib loader already skips `*.test.st` everywhere; this is
   an organizational convention, enforced by review.)
3. **Opt-in.** A module is brought in by `@import "stdlib/<module>"`. Importing a
   module does **not** ship its engine onto pages that never invoke it (see *Lean law*).
4. **JS only in two places:** `%primitive %emit js`, and owned `vendor/*.bundle.js`
   blobs. Nowhere else. (See `docs/antipatterns.md`.)

## The `%vendor` meta-construct

Declares a vendored dependency and how to flatten it into a single owned artifact.
Lives in a module's `MODULE.st` (or a primitive file).

```
%vendor <name> {
  source:  submodule "<path-relative-to-module>"   // a pinned git submodule
  entry:   "<path-within-submodule>"                // bundler root (e.g. src/layout.ts)
  bundle:  bun | direct                             // how to flatten (see below)
  exports: { name1, name2, … }                      // symbols hoisted onto the global
  out:     "<path-relative-to-module>"              // generated single-file artifact
  license: <SPDX>                                   // recorded; must match PROVENANCE
}
```

Field semantics:

- **`source: submodule "…"`** — the pin. A git submodule at a fixed commit. This is
  the provenance anchor. (Only `submodule` is supported; no npm, no URL fetch.)
- **`entry`** — the module graph root the bundler starts from.
- **`bundle`** — exactly two strategies, no more:
  - `direct` — the dep is already a single self-contained file → copy + sha-pin. No build.
  - `bun` — multi-file ESM/TS graph → one-shot `bun build` flattening into a single
    IIFE that assigns `exports` onto a stable global. We shell out to `bun`; we do
    not author a bundler.
- **`exports`** — the symbols the IIFE exposes. `%emit js` in a primitive references
  them through the vendored global; nothing else may.
- **`out`** — the generated artifact, committed in-tree. This is the **owned blob**
  that actually ships. It is hot-reload-tracked (see below).

The bundler is **one shared routine** invoked from two triggers (DRY): `build.rs`
and the `vendor build` CLI. They never diverge.

## Three disjoint "build" verbs

These must never bleed into one another.

| verb | compiles | vendoring? |
|------|----------|-----------|
| `cargo build` | the Rust **binary** | `build.rs` LAZY rebundle → embedded stdlib |
| `cargo run -- build <site>` | a **website** | **NONE** — consumes the registry as-is |
| `cargo run -- vendor build <module>` | a **stdlib asset** | regenerates the blob → hot-reload |

### `cargo build` — lazy embed (build.rs)

On binary compile, `build.rs` ensures `vendor/<dep>.bundle.js` is fresh, then the
embedded-stdlib mechanism bakes it in. **Lazy**: rebundling is *skipped* when

```
bundle artifact exists  AND  its recorded SHA matches the submodule's HEAD commit
```

…or when `bun` is absent but a bundle is already present. This keeps `cargo build`
working for contributors who are not touching the vendored dep and may not have
`bun` installed. Rebundle fires only when the pin moved or the artifact is missing.

| bundle present | SHA matches | bun present | action |
|:---:|:---:|:---:|---|
| ✓ | ✓ | — | **skip** (fresh) |
| ✓ | ✗ | ✓ | rebundle (pin moved) |
| ✓ | ✗ | ✗ | **skip + warn** (stale but buildable) |
| ✗ | — | ✓ | rebundle (missing) |
| ✗ | — | ✗ | **hard error** (cannot build, no bun, no artifact) |

### `cargo run -- vendor build <module>` — a hot-reload concern

This is **stdlib subsystem tooling, not site-build**. It regenerates
`vendor/<dep>.bundle.js` on the filesystem. A running `serve` loop notices the
blob's mtime change through the stdlib incremental cache and re-emits the owning
primitive — **no binary recompile, no site rebuild**.

```
shell A:  cargo run -- serve demos/text-engine/          (running)
shell B:  cargo run -- vendor build text                 (rebundles → vendor/pretext.bundle.js)
          → incremental_cache sees the blob change
          → re-emits measure-text primitive's embedded blob
          → browser hot-reloads
```

It also records the bundle SHA + refreshes `PROVENANCE.md`. `cargo run -- check`
re-verifies the recorded SHA against the artifact and reports drift (a human must
re-review).

## Lean law

> Importing a module ≠ shipping its engine. Only invoked capabilities emit.

A page emits a vendored blob's IIFE **only when a primitive referencing the
`%vendor` actually fired** during compilation. A page that `@import`s `stdlib/text`
but never calls `@measure`/`@balance`/… ships **zero** pretext bytes.

This is *demand-driven emission*. The vendored IIFE is concatenated into the page
runtime blob, deduped, once, iff its capability is in the page's referenced set.

> Scope note (PLAN-024): the lean rail is implemented for **vendored blobs + the
> `text` module** only. The 9 always-on core runtime modules
> (`format_js_with_runtime`) are a separate, later cleanup.

## What stays out (anti-goals)

- ✗ No npm / lockfile / `node_modules` in the pipeline.
- ✗ No runtime CDN fetch. The blob is in-tree and embedded.
- ✗ No Rust reimplementation of a vendored construct (e.g. text segmentation).
  Correctness (CJK/RTL) lives in the vendored library; the metasystem stays
  self-describing.
- ✗ No vendoring logic in the `cargo run -- build <site>` path.
