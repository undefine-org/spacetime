# Vendored dependency provenance

> The gingerbill contract: each dependency is a liability we are responsible for.
> Pinned · owned-behind-a-primitive · manually bumped. See `docs/stdlib/VENDORING.md`.

## three.js

| field | value |
|-------|-------|
| name | `three` |
| source | git submodule → https://github.com/mrdoob/three.js |
| pinned commit | `281815cbebe8c3bd48cde7f2cca6d2fced26141e` (r0.184.0) |
| license | MIT — © 2010-2026 three.js authors |
| entry wrapper | `three-entry.ts` (imports core + addons → `globalThis.THREE` / `THREE.addons`) |
| resolver config | `tsconfig.json` (`paths`: `three` → `build/three.module.js`, `three/addons/*` → `examples/jsm/*`) |
| bundled artifact | `three.bundle.js` (generated, ~0.83 MB minified IIFE) |
| bundler | `bun build --format=iife --global-name=three --minify` |

### Why we depend on it

three.js is the de-facto WebGL renderer / scene-graph for the web. It powers the
ambitious, award-winning 3D experiences Spacetime sites want to replicate
declaratively — studio-lit PBR objects, refractive glass (transmission), GPU
particle fields, scroll-driven camera work, GLTF model loading, and a real
post-processing pipeline (bloom, chromatic aberration). Hand-rolling any of that
in Rust/raw-WebGL is exactly the wheel-reinvention the project's AGENTS charter
forbids: we vendor and OWN the engine behind our own `@stage` / `@object` /
`@light` surface instead. three.js is also the low-level escape hatch (its
`RawShaderMaterial` + `EffectComposer` cover bespoke GLSL), so there is no second
"raw WebGL" module to maintain — one engine, one declarative surface.

### Public surface we use

Exposed by `three-entry.ts` on `globalThis`:

- `THREE` — the full three.js core namespace (`Scene`, `PerspectiveCamera`,
  `WebGLRenderer`, geometries, `MeshPhysicalMaterial`, lights, `InstancedMesh`,
  `Clock`, math types, …).
- `THREE.addons` — the curated addon set:
  `OrbitControls`, `GLTFLoader`, `DRACOLoader`, `EffectComposer`, `RenderPass`,
  `UnrealBloomPass`, `ShaderPass`, `OutputPass`, `RGBShiftShader`,
  `RoomEnvironment`.

Nothing in Spacetime references three.js except the `stdlib/3d` primitives; site
code only ever sees the `@`-macros, never `THREE`. Blast radius of an upstream
change = the `stdlib/3d/primitives/` files.

### Updating (manual — friction is a feature)

```sh
git -C stdlib/3d/vendor/three fetch origin
git -C stdlib/3d/vendor/three checkout <new-commit>
cargo run -- vendor build 3d          # rebundle from the new pin
# update this file's pinned-commit, re-run the 3d module tests, commit the bundle
```

### Runtime requirements

A WebGL2-capable `<canvas>` (three falls back to WebGL1). The render loop, resize,
DPR, context-loss recovery, and tone-mapping are owned by the `three-stage`
primitive. WebGL is a real-GPU/browser capability: the engine therefore CANNOT be
unit-tested under the headless V8 runtime (no WebGL context) — structural truths
(scene graph built, object/light/particle counts, material params) test on the V8
`logic` rung, while actual rendered pixels / FPS / interaction test on the `--cdp`
real-Chromium `paint` rung. See `docs/testing/FIDELITY_LADDER.md`.
