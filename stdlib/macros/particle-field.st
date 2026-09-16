/// Particle Field Macro
///
/// Creates an SVG overlay with particles that drift organically and repel
/// from the cursor. For ambient background effects on dark sections.
///
/// @example navy background
/// .dark-section-bg {
///   @particle-field(count: 30, color: "rgba(255,255,255,0.06)")
/// }

%macro particle-field-macro {

  %form {
    @particle-field(
      count: $count:number = 40,
      repulsion: $repulsion:number = 0.6,
      spread: $spread:number = 0.5,
      drift: $drift:number = 0.3,
      color: $color:string = "rgba(255,255,255,0.08)"
    )
  }

  %binds {
    particle-field(
      &self,
      count: $count,
      repulsion: $repulsion,
      spread: $spread,
      drift: $drift,
      color: $color
    ) -> { $particleCount, $destroy }
  }
}
