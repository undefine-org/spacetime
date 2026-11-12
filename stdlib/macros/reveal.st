/// Kinetic Typography Reveal Macro
///
/// Splits text into characters, words, or lines and animates them
/// with configurable stagger patterns. Supports visibility triggers,
/// custom keyframes, and direction-based reveals.
///
/// Reveal with custom keyframes body
%macro reveal-with-keyframes {

  %form {
    @reveal(
      /// Split mode: "chars", "words", or "lines"
      split: $split:string = "chars",
      /// Animation trigger: "visible", "load", or "scroll"
      trigger: $trigger:ident = visible,
      /// Delay between each unit (ms)
      stagger: $stagger:stagger_value = 30,
      /// Stagger pattern: "first", "last", "center", "edges", "random"
      staggerFrom: $staggerFrom:string = "first",
      /// Animation duration per unit (ms)
      duration: $duration:number = 600,
      /// IntersectionObserver threshold for visible trigger
      threshold: $threshold:number = 0.2,
      /// CSS easing function name
      easing: $easing:easing = ease-out,
      /// Reveal direction: "bottom", "top", "left", "right", "none"
      from: $from:string = "bottom",
      /// Travel distance from starting position
      distance: $distance:length = 100%,
      /// Whether to animate only once
      once: $once:bool = true
    ) {
      $body:keyframes
    }
  }

  %binds {
    reveal-engine(&self, keyframes: $body, ...) -> { $revealProgress, $isRevealed }
  }

  %example "word reveal with custom keyframes" {
    .section-heading {
      @reveal(split: "words", stagger: 50ms) {
        opacity: 0 -> 1;
        translate-y: 1em -> 0;
      }
    }
  }
}


/// Reveal without keyframes (uses built-in from/distance)
%macro reveal-simple {

  %form {
    @reveal(
      split: $split:string = "chars",
      trigger: $trigger:ident = visible,
      stagger: $stagger:stagger_value = 30,
      staggerFrom: $staggerFrom:string = "first",
      duration: $duration:number = 600,
      threshold: $threshold:number = 0.2,
      easing: $easing:easing = ease-out,
      from: $from:string = "bottom",
      distance: $distance:length = 100%,
      once: $once:bool = true
    )
  }

  %binds {
    reveal-engine(&self, ...) -> { $revealProgress, $isRevealed }
  }

  %example "character reveal" {
    .hero-title {
      @reveal(split: "chars", stagger: 30ms, from: "bottom")
    }
    <h1 class="hero-title">Hello world</h1>
  }

  %example "center-out stagger" {
    .display-text {
      @reveal(split: "chars", staggerFrom: "center", stagger: 20ms)
    }
    <p class="display-text">Spacetime</p>
  }
}
