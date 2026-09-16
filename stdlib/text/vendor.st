/// Vendored dependency declaration for the `text` module (PLAN-024).
///
/// Lives in its own file so the public entry (`index.st`) can pull it into the
/// import graph — the `%vendor` must be registered whenever the module is
/// imported, so demand-driven emission can inject the bundled engine when a
/// text primitive fires. The human-facing manifest is `MODULE.st`.
///
/// `entry` is the author-controlled wrapper (vendor/pretext-entry.ts) that
/// re-exports pretext's public surface onto `globalThis.pretext`; `source` is the
/// pinned git submodule, used to verify the commit. `bun build` flattens the
/// wrapper + submodule graph into the single committed IIFE at `out`.

%vendor pretext {
  source:  submodule "vendor/pretext"
  entry:   "vendor/pretext-entry.ts"
  bundle:  bun
  exports: { prepare, prepareWithSegments, layout, layoutWithLines, walkLineRanges, measureNaturalWidth }
  out:     "vendor/pretext.bundle.js"
  license: MIT
}
