/// Browser EventSource rail. The vendor name IS the referenced global token
/// (inject_vendor_preludes word-boundary-matches the name against site JS),
/// so the shim is demand-injected only on pages that use @data stream.
%vendor SpacetimeEventSource {
  source: submodule "vendor"
  entry: "vendor/event-source.js"
  bundle: direct
  exports: { SpacetimeEventSource }
  out: "vendor/event-source.bundle.js"
  license: MIT
}
