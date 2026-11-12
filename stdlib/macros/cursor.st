/// Custom Cursor Macro
///
/// Creates a branded custom cursor with smooth lerp-based following,
/// hover state scaling on interactive elements, and click compression.
/// Automatically hides on touch devices.

%macro cursor-macro {

  %form {
    @cursor(
      /// Cursor dot diameter
      size: $size:string = "24px",
      /// Cursor fill color
      color: $color:string = "#ff6b47",
      /// CSS mix-blend-mode
      blend: $blend:string = "difference",
      /// Follow smoothness (0-1, lower = smoother)
      lerp: $lerp:number = 0.15,
      borderWidth: $borderWidth:string = "0px",
      borderColor: $borderColor:string = "currentColor",
      borderRadius: $borderRadius:string = "50%",
      /// Base scale
      scale: $scale:number = 1,
      /// Scale when hovering interactive elements
      hoverScale: $hoverScale:number = 1.5,
      /// Scale on mousedown
      clickScale: $clickScale:number = 0.9,
      /// CSS selector for interactive elements
      hoverElements: $hoverElements:string = "a, button, [data-cursor]"
    )
  }

  %binds {
    cursor-engine(&self, ...) -> { $cursorX, $cursorY, $isHovering, $isClicking }
  }

  %example "basic" {
    body {
      @cursor(size: "24px", color: "var(--accent)", blend: "difference")
    }
  }

  %example "with custom hover targets" {
    .hero {
      @cursor(
        size: "20px",
        color: "#ff6b47",
        hoverScale: 2.0,
        hoverElements: ".card, a, button, [data-cursor]"
      )
    }
  }
}
