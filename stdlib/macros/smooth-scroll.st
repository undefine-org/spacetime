/// Smooth Scroll Macro
///
/// Applies Lenis-style smooth scrolling to an element or the window.
/// Intercepts wheel and keyboard events, applying lerp-based interpolation
/// for a buttery-smooth scroll experience.
///
/// @example
/// body {
///   @smooth-scroll(lerp: 0.08)
/// }
///
/// @example horizontal
/// .horizontal-container {
///   @smooth-scroll(orientation: "horizontal", lerp: 0.1)
/// }

%macro smooth-scroll-macro {

  %form {
    @smooth-scroll(
      lerp: $lerp:number = 0.1,
      duration: $duration:number = 1.2,
      smoothWheel: $smoothWheel:bool = true,
      smoothTouch: $smoothTouch:bool = false,
      infinite: $infinite:bool = false,
      orientation: $orientation:string = "vertical"
    )
  }

  %binds {
    smooth-scroll(
      &self,
      lerp: $lerp,
      duration: $duration,
      smoothWheel: $smoothWheel,
      smoothTouch: $smoothTouch,
      infinite: $infinite,
      orientation: $orientation
    ) -> { $scrollY, $velocity, $progress, $isScrolling, $direction }
  }
}
