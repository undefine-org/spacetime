/// Particle Field Macro
///
/// Creates an SVG overlay with particles that drift organically and repel
/// from the cursor. For ambient background effects on dark sections.

%macro particle-field-macro {

  %form {
    @particle-field(
      /// Number of particles
      count: $count:number = 40,
      /// Cursor flee intensity 0-1
      repulsion: $repulsion:number = 0.6,
      /// Inter-particle spacing force 0-1
      spread: $spread:number = 0.5,
      /// Ambient movement energy 0-1
      drift: $drift:number = 0.3,
      /// Base particle color")
      color: $color:string = "rgba(255,255,255,0.08)"
    )
  }

  %binds {
    particle-field(&self, ...) -> { $particleCount, $destroy }
  }

  %example "navy background" {
    .dark-section-bg {
      @particle-field(count: 30, color: "rgba(255,255,255,0.06)")
    }
  }
}
