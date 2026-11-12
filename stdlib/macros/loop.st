/// Loop Timeline
///
/// Infinitely looping animations. `@loop` retired in the 2026-07-26 wave
/// (SIP-001); a loop is now the `@on` head with the `.loop` driver:
///
///   .spinner { @on &.loop(duration: 1s) { rotate: 0deg -> 360deg; } }
///   .pulse { @on &.loop(duration: 2s, mode: "pingpong") { scale: 1 -> 1.1; opacity: 1 -> 0.7; } }

/// Sequence Macro
///
/// Creates step-by-step animation sequences.
/// NOTE: Currently simplified - uses time-driver for sequential playback.
/// Full step-by-step triggering requires additional runtime support.

%macro sequence {

  %form {
    @sequence $name:ident (
      /// Animation duration in ms
      duration: $duration:time = 1000ms,
      /// Delay before starting in ms
      delay: $delay:time = 0ms,
      /// function name
      easing: $easing:easing = ease-out,
      /// Delay between elements (ms)
      stagger: $stagger:stagger_value = 100,
      /// Stagger origin ("first", "last", "center")
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    time-driver(&self, name: $name, duration: $duration, delay: $delay, iterations: 1, autoplay: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, ...)
  }

  %example "@sequence" {
    .wizard-steps {
      @sequence intro(duration: 900ms, stagger: 200ms) {
        .step {
          opacity: 0 -> 1;
          translate-x: -20px -> 0;
        }
      }
    }
    <ol class="wizard-steps">
      <li class="step">Account</li>
      <li class="step">Profile</li>
      <li class="step">Done</li>
    </ol>
  }
}


// @after DELETED (PLAN-126 W5): sequencing is `@on $x.done { true => --form; }`
// — arms on the done facet of a published progress signal (D24/D29).


/// Chain Macro
///
/// Chains multiple timelines together with stagger.
/// NOTE: Currently simplified - uses time-driver with stagger.
/// Full sequential chaining requires additional runtime support.

%macro chain {

  %form {
    @chain $name:ident (
      /// Animation duration in ms
      duration: $duration:time = 1000ms,
      /// Delay before starting in ms
      delay: $delay:time = 0ms,
      /// function name
      easing: $easing:easing = ease-out,
      /// Delay between elements (ms)
      stagger: $stagger:stagger_value = 100,
      /// Stagger origin ("first", "last", "center")
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    time-driver(&self, name: $name, duration: $duration, delay: $delay, iterations: 1, autoplay: true) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, ...)
  }

  %example "@chain" {
    .animated-logo {
      @chain logo-intro(duration: 1000ms, stagger: 100) {
        .fade { opacity: 0 -> 1 }
        .scale { scale: 0.8 -> 1 }
        .settle { scale: 1 -> 1.02 -> 1 }
      }
    }
  }
}


/// Mouse Position Macro
///
/// Animation driven by mouse position.
/// NOTE: Currently using mouse-driver with apply-animations.
/// Full expression-based value bindings require additional support.

%macro mouse-position {

  %form {
    @mouse-position $name:ident(
      /// "x", "y", or "both"
      axis: $axis:string = "both",
      /// Invert progress direction
      invert: $invert:bool = false,
      /// function name
      easing: $easing:easing = linear,
      /// Delay between elements (ms)
      stagger: $stagger:stagger_value = 0,
      /// Stagger origin ("first", "last", "center")
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }

  %binds {
    mouse-driver(&self, name: $name, axis: $axis, invert: $invert) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body, interactive: true, ...)
  }

  %example "@mouse-position" {
    .tilt-card {
      @mouse-position tilt {
        rotate-x: 0 -> 10deg
        rotate-y: 0 -> 10deg
      }
    }
  }
}
