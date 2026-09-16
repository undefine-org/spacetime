/// Fixture: imports stdlib/showcases/hero/minimal.st and invokes it.
/// Ensures the showcase compiles when consumed externally.

@import "../../../stdlib/showcases/hero/minimal.st"

.fixture-hero-minimal-host {
    &showcase-hero-minimal("Build less, ship more.", "Spacetime turns CSS into your component model.", "Get started");
}
