# demos/3d — declarative 3D with `stdlib/3d` (vendored three.js)

Four self-contained showcases of the `stdlib/3d` module: ambitious, award-style
3D built **entirely** from Spacetime's `@`-surface (`@stage` / `@object` /
`@light` / `@env` / `@orbit` / `@parallax` / `@post` / `@particles` / `@gltf` /
`@scroll-3d`). No hand-written JavaScript, no three.js API in site code — the
vendored engine lives behind the macros (see `stdlib/3d/`).

```sh
cargo run -- serve demos/3d/
# then open:
#   /glass-hero/      /particle-morph/      /gltf-studio/      /scroll-tunnel/
```

## The showcases

| demo | what it proves | key surface |
|---|---|---|
| **glass-hero/** | a thin-film **iridescent** faceted gem, studio-lit, drag-orbit + mouse parallax | `@stage` `@env` `@light` `@object(material:physical, iridescence)` `@orbit` `@parallax` |
| **particle-morph/** | **30,000** GPU particles that **morph** sphere→knot→galaxy→grid on scroll, additive + bloom | `@particles` `@scroll-3d(drive:morph)` `@post(bloom)` |
| **gltf-studio/** | a **real glTF PBR model** (the canonical Damaged Helmet) under three-point studio lighting, drag-to-inspect | `@gltf(src:)` `@env` `@light` `@orbit` |
| **scroll-tunnel/** | a **scroll-driven camera flythrough** of a glowing neon knot with **chromatic aberration** + bloom | `@scroll-3d(drive:dollyZ)` `@post(bloom, rgbShift)` |

## Notes on the material choices (the honest version)

- **Iridescence over transmissive "glass".** three.js `transmission` is
  *screen-space refraction*: it bends whatever the renderer drew **behind** the
  object. On a dark minimalist hero there's little behind it to bend, so a
  transmissive object collapses to an opaque pearl-white blob (the bright
  environment reflection dominates). Thin-film **iridescence** carries its colour
  on the surface itself, so it reads beautifully at every rotation angle on a dark
  background. (See the `stdlib/3d` follow-ups for the full analysis.)
- **No `@post` on transmissive scenes.** three's transmission render pass and the
  `EffectComposer` (`@post`) don't compose — bloom flattens transmission to a
  milky blob. `@post` is great on emissive/standard scenes (see particle-morph and
  scroll-tunnel); it's simply omitted where transmission is in play.

## The model asset

`gltf-studio/models/helmet/` is the three.js canonical **DamagedHelmet** glTF
(MIT / CC-BY, from the three.js examples). It's a multi-file glTF (`.gltf` + `.bin`
+ JPG textures); the dev server serves `.gltf`/`.bin`/`.glb` as of PLAN-050.

## Validation

- **Gating (offline):** Rust emit tests in `src/compiler.rs` (`three_*`) assert
  the macros resolve and emit the right three.js calls + demand-inject the engine
  bundle (and that an unused import ships **zero** engine bytes — the lean law).
- **Paint rung:** `stdlib/3d/tests/*.test.st` assert the live scene graph in a real
  browser (`--cdp`); they refuse on headless V8 (no WebGL) and currently await the
  CDP harness fix (BUG-099).
- **End-to-end:** these four demos were rendered + screenshot-reviewed in a real
  Chromium during development.
