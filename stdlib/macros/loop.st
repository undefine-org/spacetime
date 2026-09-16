/// Loop Timeline Macro
///
/// Creates infinitely looping animations.
/// Uses loop-driver primitive with apply-animations.
///
/// @example positional duration
/// .spinner {
///   @loop spin(1s) {
///     rotate: 0deg -> 360deg
///   }
/// }
///
/// @example named duration with mode
/// .pulse {
///   @loop pulse(duration: 2s, mode: "pingpong") {
///     scale: 1 -> 1.1
///     opacity: 1 -> 0.7
///   }
/// }

/// Sequence Macro
///
/// Creates step-by-step animation sequences.
/// NOTE: Currently simplified - uses time-driver for sequential playback.
/// Full step-by-step triggering requires additional runtime support.
///
/// @example
/// .wizard-steps {
///   @sequence intro(duration: 900ms, stepDelay: 200ms) {
///     .step-1 {
///       opacity: 0 -> 1
///       translate-x: -20px -> 0
///     }
///     .step-2 {
///       opacity: 0 -> 1
///       translate-x: -20px -> 0
///     }
///     .step-3 {
///       opacity: 0 -> 1
///       translate-x: -20px -> 0
///     }
///   }
/// }

%macro sequence {

  %form {
    @sequence $name:ident (
      duration: $duration:time = 1000ms,
      delay: $delay:time = 0ms,
      easing: $easing:easing = ease-out,
      stagger: $stagger:stagger_value = 100,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    time-driver(&self, name: $name, duration: $duration, delay: $delay, iterations: 1, autoplay: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
  }
}


// @after DELETED (PLAN-126 W5): sequencing is `@on $x.done { true => --form; }`
// — arms on the done facet of a published progress signal (D24/D29).


/// Chain Macro
///
/// Chains multiple timelines together with stagger.
/// NOTE: Currently simplified - uses time-driver with stagger.
/// Full sequential chaining requires additional runtime support.
///
/// @example
/// .animated-logo {
///   @chain logo-intro(duration: 1000ms, stagger: 100) {
///     .fade { opacity: 0 -> 1 }
///     .scale { scale: 0.8 -> 1 }
///     .settle { scale: 1 -> 1.02 -> 1 }
///   }
/// }

%macro chain {

  %form {
    @chain $name:ident (
      duration: $duration:time = 1000ms,
      delay: $delay:time = 0ms,
      easing: $easing:easing = ease-out,
      stagger: $stagger:stagger_value = 100,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    time-driver(&self, name: $name, duration: $duration, delay: $delay, iterations: 1, autoplay: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom)
  }
}


/// Mouse Position Macro
///
/// Animation driven by mouse position.
/// NOTE: Currently using mouse-driver with apply-animations.
/// Full expression-based value bindings require additional support.
///
/// @example
/// .tilt-card {
///   @mouse-position tilt {
///     rotate-x: 0 -> 10deg
///     rotate-y: 0 -> 10deg
///   }
/// }

%macro mouse-position {

  %form {
    @mouse-position $name:ident(
      axis: $axis:string = "both",
      invert: $invert:bool = false,
      easing: $easing:easing = linear,
      stagger: $stagger:stagger_value = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    mouse-driver(&self, name: $name, axis: $axis, invert: $invert) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, easing: $easing, stagger: $stagger, staggerFrom: $staggerFrom, interactive: true)
  }
}
