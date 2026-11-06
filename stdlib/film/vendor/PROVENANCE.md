# Vendored dependency provenance

> The gingerbill contract: each dependency is a liability we are responsible for.
> Pinned · owned-behind-a-primitive · manually bumped. See `docs/stdlib/VENDORING.md`.

## raea — the Render Anything Engine

| field | value |
|-------|-------|
| name | `raea` |
| source | git submodule → https://github.com/shinyobjectz/raea |
| upstream base | `fb3f70844e03cc40185e3a06c394060d26c5cd38` (2026-11-05, canary) |
| pinned commit | the local commit on top of the base (see *Local patches*) |
| license | **FSL-1.1-ALv2** — © 2026 Shane Murphy. Source-available, *not* OSI open source: any use except a *Competing Use* (a commercial product that substitutes for raea or offers substantially similar functionality). Each version becomes Apache-2.0 on the second anniversary of its release (this base: 2028-11-05). Third-party parts keep their own licences (raea's `NOTICE`, `LICENSES/`). |
| artifact | the `raea` binary — `cargo build --release -p raea-cli --no-default-features` inside the submodule (`target/release/raea`); NOT embedded in the Spacetime binary, NOT committed |
| owned behind | `src/render/raea.rs` — the ONLY file in Spacetime that knows raea exists |

### Why we depend on it

Spacetime's thesis is that a website and a video differ only in which signal
drives the timeline (PLAN-132). Until now the only thing that could *draw* the
video was a browser: `spacetime render` drove Chromium over CDP and screenshotted
every frame. raea draws an HTML page without a browser — Blitz/Stylo lay it out,
Vello paints it on the GPU, and frames are encoded in-process — and it accepts
any page that registers a seekable timeline on `window.__timelines`. Spacetime
pages already have exactly one clock (`ST._clock`), so the integration is one
adapter, not a second runtime: `spacetime render --engine raea`.

Rewriting a CSS engine, a GPU 2D renderer and an encoder is exactly the
wheel-reinvention the vendoring rule forbids; we vendor and own raea instead.

### Public surface we use

- `raea render PAGE.html -o OUT.mp4` — an HTML composition to video.
- `raea render PAGE.html -o DIR --at F,F,…` — chosen frames as PNGs.
- The HTML composition contract: a root element with `data-composition-id`,
  `data-width`, `data-height`, `data-fps`, `data-duration`, and a timeline on
  `window.__timelines[id]` answering `totalDuration()` / `totalTime(t)`.

Nothing else. raea's TypeScript/React SDK, 3D, audio, games and agent tooling
are untouched by Spacetime. Blast radius of an upstream change = `src/render/raea.rs`.

### Local patches (on top of the base)

Every patched line carries `[spacetime: local patch, see stdlib/film/vendor/PROVENANCE.md]`.
Each fix was found by rendering a real Spacetime film (`demos/film-showcase/`)
and is a genuine engine gap, not a Spacetime special case:

1. **`runtime/encode/src/render.rs` — HTML stills to a folder.** An HTML page always
   takes the direct GPU path, which wrote only `.mp4`/`.txt`, so the documented
   `raea render PAGE.html -o DIR --at …` could not work ("Is a directory").
   The direct path now writes `frame-NNNNN.png` into a folder.
2. **`sdk/ui/src/dom/element.ts` — `innerHTML` parses markup.** The setter
   stripped tags ("markup is out of scope"). It now parses with raea's own
   composition parser (`html/parse.ts`) and builds real nodes (SVG namespaced),
   so scripts that stamp markup — Spacetime's templates and `@form shot`s — work.
3. **`sdk/ui/src/dom/element.ts` + `nodes.ts` + `media.ts` — `<template>`.**
   `HTMLTemplateElement.content` (a fragment), `DocumentFragment`'s ParentNode
   accessors (`children`, `firstElementChild`, …), and template content carried
   by a deep `cloneNode`.
4. **`vendor/stylo` + `vendor/blitz-paint` — `background-clip: text`.** Stylo
   parsed `text` only under its `gecko` feature; Blitz could not paint it. Now
   parsed in servo mode and painted as source-in: glyphs drawn opaque into an
   isolated layer, the background composited onto them.
5. **`sdk/ui/src/dom/core.ts` — custom properties reach the host.** In HTML
   mode raea dropped every inline custom property before it reached Blitz
   (`htmlValue` returned `undefined` for `--*`), so a stylesheet's `var()` kept
   its fallback forever. A score animating `--reveal` under a
   `mask-image: radial-gradient(… var(--reveal) …)` froze at frame 0. Custom
   properties now pass through as written and Stylo resolves `var()` natively.

6. **`sdk/ui/src/html/page/load.ts` + `time.ts` + `dom/core.ts` — the TS cascade
   runs lazily on inline-style writes.** Any inline-style write marked the page
   dirty and re-ran raea's TypeScript cascade — every stylesheet rule matched
   against the whole document, in QuickJS — on every frame. Its output
   (`_sheet`) only feeds `getComputedStyle` (Stylo paints natively), and an
   inline style cannot change what a rule matches, so style-only mutations now
   mark it stale and it re-runs when computed style is next read. Measured on
   `demos/promo/`: 702 → 44 ms per frame.
7. **`vendor/blitz-dom` (`layout/construct.rs`, `resolve.rs`) — CSS counters**
   (a nested counter keeps its text while construction has a container's
   children out of the tree, and is re-synced with its ancestors marked damaged;
   `systems/page/tests/vars.rs` `nested_counter_follows_its_reset`).
   Blitz drew only the strings of `content`. `counter()` / `counters()` now
   resolve by CSS Lists 3's scoping (`counter-reset` scoped to the element and
   its following siblings, `counter-increment` on the innermost instance), with
   the common counter styles; pseudo-elements that use counters are re-synced
   after every restyle, since their value depends on the whole document.
   (`counter-set` is gecko-only in Stylo and stays unparsed.)

8. **The page→world bridge (PLAN-152): an HTML composition can describe a 3D
   world.** `sdk/ui/src/html/page/load.ts` reports each seek's `[data-3d]` tree
   (`sceneAt`, raea's own TSX `node3d` markup) and `has3d`;
   `engine/page/src/html.rs` keeps it; `runtime/export/src/html3d.rs` reads every
   frame's scene into raea-render's rows (`program`), opens the live engine, and
   draws each frame's page over its 3D (a second, fresh page: a page's clock may
   only run forward), mixing the page's `<audio>`; `--at` writes chosen frames as
   PNGs. `engine/cli/src/cli.rs` sends a page naming `data-3d` there (falling back
   to the direct path when it built none).
9. **`runtime/scene/src/models.rs` — a model whose clip changes.** A row's `anim`
   was the first frame's, so a model that went from "Walking" to "Death" kept
   walking. Distinct clips over the film become a blend (`mix`), each weighted 1
   while it plays, at its own time.
10. **`sdk/ui/src/dom/selectors.ts` — a selector that reads nothing.** A compound
    that consumes no character (CSS nesting's `&`) looped forever, pushing empty
    compounds until memory ran out (10 GB on one page). It is now an invalid
    selector, which callers skip.
11. **`sdk/ui/src/html/page/load.ts` — `data-props` costs no cascade.** A 3D part
    rewritten every frame re-ran the TS cascade (40 s for 15 frames); it changes
    what the world draws, not what a rule matches.

These are offered back upstream in spirit; until upstream lands them they live
in the local commit. Updating: rebase the local commit onto the new base, rebuild,
re-run `tests/render/raea_render_test.rs`, update this file.

### Platform notes

- Linux needs a Vulkan driver. On this workstation Mesa's Intel driver (ANV,
  UHD 630) works; Mesa's NVK (GTX 1660 Ti) does not offer a wgpu feature raea
  requests, so point `VK_ICD_FILENAMES` at the Intel ICD. lavapipe (CPU) also
  works, slowly.
- The binary is located by `$SPACETIME_RAEA`, else
  `stdlib/film/vendor/raea/target/release/raea`, else `raea` on `PATH`.
