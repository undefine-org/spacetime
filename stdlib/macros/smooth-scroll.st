/// Smooth Scroll Macro
///
/// Applies Lenis-style smooth scrolling to an element or the window.
/// Intercepts wheel and keyboard events, applying lerp-based interpolation
/// for a buttery-smooth scroll experience.

%macro smooth-scroll-macro {

  %form {
    @smooth-scroll(
      /// Interpolation factor (0-1, lower = smoother)
      lerp: $lerp:number = 0.1,
      /// Scroll duration in seconds
      duration: $duration:number = 1.2,
      /// Whether to smooth mouse wheel scrolling
      smoothWheel: $smoothWheel:bool = true,
      /// Whether to smooth touch scrolling (false preserves native)
      smoothTouch: $smoothTouch:bool = false,
      /// Enable infinite scroll wrapping
      infinite: $infinite:bool = false,
      /// Scroll axis: "vertical" or "horizontal"
      orientation: $orientation:string = "vertical"
    )
  }

  %binds {
    smooth-scroll(&self, ...) -> { $scrollY, $velocity, $progress, $isScrolling, $direction }
  }

  %example "@smooth-scroll-macro" {
    body {
      @smooth-scroll(lerp: 0.08)
    }
  }

  %example "horizontal" {
    .horizontal-container {
      @smooth-scroll(orientation: "horizontal", lerp: 0.1)
    }
  }
}
