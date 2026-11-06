/// Vendored dependencies of the `film` module.
///
/// `synth` is FIRST-PARTY (`source: local`): Spacetime's score synthesizer,
/// one plain-JS engine kept as a file so the same bytes run in a browser (the
/// `score-synth` primitive, demand-injected only on pages with a `@sound`) and
/// in `spacetime render`'s V8 (src/render/soundtrack.rs, via
/// `vendor::vendor_blob_source`). `local` means no submodule pin and nothing to
/// bundle: the file IS the artifact.

%vendor STSynth {
  source:  local "synth/synth.js"
  entry:   "synth/synth.js"
  bundle:  direct
  exports: { STSynth }
  out:     "synth/synth.js"
  license: EPL-2.0
}
