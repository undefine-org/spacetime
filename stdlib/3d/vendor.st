/// Vendored dependency declaration for the `3d` module.
///
/// Lives in its own file so the public entry (`index.st`) can pull it into the
/// import graph — the `%vendor` must be registered whenever the module is
/// imported, so demand-driven emission can inject the bundled engine when a 3d
/// primitive fires. The human-facing manifest is `MODULE.st`.
///
/// `entry` is the author-controlled wrapper (vendor/three-entry.ts) that imports
/// three's core + the curated addon set and assigns them onto `globalThis.THREE`
/// (+ `THREE.addons`); `source` is the pinned git submodule, used to verify the
/// commit. `bun build` flattens the wrapper + submodule graph (resolving the bare
/// `three` / `three/addons/*` specifiers via vendor/tsconfig.json `paths`, NO npm)
/// into the single committed IIFE at `out`.

%vendor three {
  source:  submodule "vendor/three"
  entry:   "vendor/three-entry.ts"
  bundle:  bun
  exports: { THREE }
  out:     "vendor/three.bundle.js"
  license: MIT
}
