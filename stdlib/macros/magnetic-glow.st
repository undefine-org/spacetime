/// Magnetic Glow Macro
///
/// Composes magnetic-effect primitive with CSS box-shadow glow.
/// When the cursor enters the radius, the element glows with increasing
/// intensity as the cursor approaches — the "cursor melts into component" effect.
///
/// @example basic
/// .card {
///   @magnetic-glow(radius: 200, color: "#E85D4A", intensity: 0.6)
/// }
///
/// @example subtle
/// .button {
///   @magnetic-glow(radius: 150, color: "currentColor", intensity: 0.3)
/// }

%macro magnetic-glow-macro {

  %form {
    @magnetic-glow(
      strength: $strength:number = 0.15,
      radius: $radius:number = 200,
      lerp: $lerp:number = 0.1,
      color: $_color:string = "currentColor",
      intensity: $_intensity:number = 0.6
    )
  }

  %binds {
    magnetic-effect(
      &self,
      strength: $strength,
      radius: $radius,
      lerp: $lerp
    ) -> { $magnetX, $magnetY, $distance, $isActive }
  }
}
