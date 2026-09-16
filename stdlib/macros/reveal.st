/// Kinetic Typography Reveal Macro
///
/// Splits text into characters, words, or lines and animates them
/// with configurable stagger patterns. Supports visibility triggers,
/// custom keyframes, and direction-based reveals.
///
/// @example character reveal
/// .hero-title {
///   @reveal(split: "chars", stagger: 30ms, from: "bottom")
/// }
///
/// @example word reveal with custom keyframes
/// .section-heading {
///   @reveal(split: "words", stagger: 50ms) {
///     opacity: 0 -> 1;
///     translate-y: 1em -> 0;
///   }
/// }
///
/// @example center-out stagger
/// .display-text {
///   @reveal(split: "chars", staggerFrom: "center", stagger: 20ms)
/// }

/// Reveal with custom keyframes body
%macro reveal-with-keyframes {

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
    ) {
      $body:keyframes
    }
  }

  %binds {
    reveal-engine(
      &self,
      split: $split,
      trigger: $trigger,
      stagger: $stagger,
      staggerFrom: $staggerFrom,
      duration: $duration,
      threshold: $threshold,
      easing: $easing,
      from: $from,
      distance: $distance,
      once: $once,
      keyframes: $body
    ) -> { $revealProgress, $isRevealed }
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
    reveal-engine(
      &self,
      split: $split,
      trigger: $trigger,
      stagger: $stagger,
      staggerFrom: $staggerFrom,
      duration: $duration,
      threshold: $threshold,
      easing: $easing,
      from: $from,
      distance: $distance,
      once: $once
    ) -> { $revealProgress, $isRevealed }
  }
}
