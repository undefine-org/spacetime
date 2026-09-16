/// Magnetic Hover Macro
///
/// Applies proximity-based magnetic pull to an element.
/// When the mouse enters the configured radius, the element
/// is smoothly drawn toward the cursor position.
///
/// @example button
/// .cta-button {
///   @magnetic(strength: 0.3, radius: 100px)
/// }
///
/// @example horizontal only
/// .nav-link {
///   @magnetic(strength: 0.2, axis: "x", radius: 80px)
/// }

%macro magnetic-macro {

  %form {
    @magnetic(
      strength: $strength:number = 0.3,
      radius: $radius:number = 100,
      lerp: $lerp:number = 0.1,
      axis: $axis:string = "both",
      trigger: $trigger:string = "self",
      tolerance: $tolerance:number = 0.5
    )
  }

  %binds {
    magnetic-effect(
      &self,
      strength: $strength,
      radius: $radius,
      lerp: $lerp,
      axis: $axis,
      trigger: $trigger,
      tolerance: $tolerance
    ) -> { $magnetX, $magnetY, $distance, $isActive }
  }
}
