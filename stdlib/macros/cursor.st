/// Custom Cursor Macro
///
/// Creates a branded custom cursor with smooth lerp-based following,
/// hover state scaling on interactive elements, and click compression.
/// Automatically hides on touch devices.
///
/// @example basic
/// body {
///   @cursor(size: "24px", color: "var(--accent)", blend: "difference")
/// }
///
/// @example with custom hover targets
/// .hero {
///   @cursor(
///     size: "20px",
///     color: "#ff6b47",
///     hoverScale: 2.0,
///     hoverElements: ".card, a, button, [data-cursor]"
///   )
/// }

%macro cursor-macro {

  %form {
    @cursor(
      size: $size:string = "24px",
      color: $color:string = "#ff6b47",
      blend: $blend:string = "difference",
      lerp: $lerp:number = 0.15,
      borderWidth: $borderWidth:string = "0px",
      borderColor: $borderColor:string = "currentColor",
      borderRadius: $borderRadius:string = "50%",
      scale: $scale:number = 1,
      hoverScale: $hoverScale:number = 1.5,
      clickScale: $clickScale:number = 0.9,
      hoverElements: $hoverElements:string = "a, button, [data-cursor]"
    )
  }

  %binds {
    cursor-engine(
      &self,
      size: $size,
      color: $color,
      blend: $blend,
      lerp: $lerp,
      borderWidth: $borderWidth,
      borderColor: $borderColor,
      borderRadius: $borderRadius,
      scale: $scale,
      hoverScale: $hoverScale,
      clickScale: $clickScale,
      hoverElements: $hoverElements
    ) -> { $cursorX, $cursorY, $isHovering, $isClicking }
  }
}
