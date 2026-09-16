# Vendored dependency provenance

> The gingerbill contract: each dependency is a liability we are responsible for.
> Pinned · owned-behind-a-primitive · manually bumped. See `docs/stdlib/VENDORING.md`.

## pretext

| field | value |
|-------|-------|
| name | `@chenglou/pretext` |
| source | git submodule → https://github.com/chenglou/pretext |
| pinned commit | `796b4691ca782ec44df9eb5d470abeca4d25732f` |
| version | v0.0.7 (+3 commits) |
| license | MIT — © 2026 Pretext contributors |
| entry wrapper | `pretext-entry.ts` (re-exports onto `globalThis.pretext`) |
| bundled artifact | `pretext.bundle.js` (generated, ~43KB IIFE) |
| bundler | `bun build --format=iife --minify` |

### Why we depend on it

Accurate text measurement (line count, height, balanced line breaks, fit, flow)
**without** triggering DOM reflow. `getBoundingClientRect`/`offsetHeight` force a
synchronous layout — pretext measures via Canvas `measureText` + the browser font
engine as ground truth, and computes line breaks arithmetically. This is the only
correct path for CJK/RTL/grapheme-aware layout; we do NOT reimplement it in Rust.

### Public surface we use

`prepare`, `prepareWithSegments`, `layout`, `layoutWithLines`, `walkLineRanges`, `measureNaturalWidth`
(see `MODULE.st` `%vendor exports`). Keep `pretext-entry.ts`'s import list in sync.

### Updating (manual — friction is a feature)

```sh
git -C stdlib/text/vendor/pretext fetch origin
git -C stdlib/text/vendor/pretext checkout <new-commit>
cargo run -- vendor build text        # rebundle from the new pin
# update this file's pinned-commit/version, re-run the text module tests, commit
```

### Runtime requirements

`Intl.Segmenter` + Canvas 2D `measureText`. Present in browsers; absent in the V8
headless test runtime — text-module tests therefore run via the **playwright_st**
harness (real Chromium), not the headless V8 path.
