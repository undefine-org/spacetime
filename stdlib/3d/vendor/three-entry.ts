// Vendored-entry wrapper for three.js (mirrors stdlib/text/vendor/pretext-entry.ts
// and stdlib/md/vendor/snarkdown-entry.ts).
//
// bun's IIFE format drops ES exports. This wrapper imports three's core surface
// plus the addon modules we depend on (controls, loaders, postprocessing,
// environments, shaders), and assigns them onto stable globals our `%emit js`
// primitives reference:
//
//   globalThis.THREE         — the full three.js core namespace (r0.184.0)
//   globalThis.THREE.addons  — the curated addon set (see %vendor `exports`)
//
// The bare `three` / `three/addons/*` specifiers below are resolved by bun via
// vendor/tsconfig.json `paths` (three → build/three.module.js; three/addons/* →
// examples/jsm/*) — NO npm, NO node_modules. bun discovers that tsconfig by
// walking up from THIS file, so the bundle resolves regardless of the cwd the
// bundler is invoked from (build.rs runs from the repo root).
//
// Keep the import list in sync with the `exports:` field of the `%vendor three`
// declaration in vendor.st / MODULE.st.
import * as THREE from "three";

import { OrbitControls } from "three/addons/controls/OrbitControls.js";

import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
// NOTE: DRACOLoader is intentionally NOT bundled. Its source uses `import.meta.url`
// (to locate the decoder worker), which bun's IIFE format leaves intact — and the
// dev server injects the runtime as a CLASSIC script, where `import.meta` is a hard
// SyntaxError that halts the whole runtime. GLTFLoader handles uncompressed .glb/.gltf
// (the common case) without DRACO. Compressed-mesh support returns once the runtime is
// emitted as a module (tracked separately).

import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import { ShaderPass } from "three/addons/postprocessing/ShaderPass.js";
import { OutputPass } from "three/addons/postprocessing/OutputPass.js";
import { RGBShiftShader } from "three/addons/shaders/RGBShiftShader.js";

import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";

// Expose the core namespace on a stable global, then attach the curated addons
// under THREE.addons so a primitive reaches everything through one entry point.
(globalThis as any).THREE = THREE;
(globalThis as any).THREE.addons = {
  OrbitControls,
  GLTFLoader,
  EffectComposer,
  RenderPass,
  UnrealBloomPass,
  ShaderPass,
  OutputPass,
  RGBShiftShader,
  RoomEnvironment,
};
