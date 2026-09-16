/// Vendored dependency declaration for the `md` module.
///
/// Lives in its own file so the public entry (`index.st`) can pull it into the
/// import graph — the `%vendor` must be registered whenever the module is
/// imported, so demand-driven emission can inject the bundled engine when a
/// markdown primitive fires. The human-facing manifest is `MODULE.st`.
///
/// `entry` is the author-controlled wrapper (vendor/snarkdown-entry.ts) that
/// re-exports snarkdown's default `parse` onto `globalThis.snarkdown`; `source`
/// is the pinned git submodule, used to verify the commit. `bun build` flattens
/// the wrapper + submodule into the single committed IIFE at `out`.

%vendor snarkdown {
  source:  submodule "vendor/snarkdown"
  entry:   "vendor/snarkdown-entry.ts"
  bundle:  bun
  exports: { snarkdown }
  out:     "vendor/snarkdown.bundle.js"
  license: MIT
}
