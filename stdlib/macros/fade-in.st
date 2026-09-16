/// Fade In Macro
///
/// Fades an element in — as a WHOLE — when it enters the viewport, optionally
/// travelling a short distance as it arrives.
///
/// Use `@fade-in` for a panel, card or image. Use `@reveal` for TEXT you want
/// split into characters, words or lines and revealed kinetically — it rewrites
/// the element's innerHTML into per-unit spans, which would destroy the markup
/// of a container.
///
/// @param duration  - How long the fade runs (default 600ms)
/// @param threshold - Visibility ratio that triggers it (default 0.1)
/// @param distance  - How far the element travels while fading (default 20px;
///                    `0` for a pure fade with no travel)
/// @param easing    - A CSS keyword, or a curve you declared with
///                    `@form easing --name { ... }` (default ease-out)
/// @param delay     - Wait before starting (default 0)
/// @param from      - Which side it travels FROM: bottom | top | left | right |
///                    none (default bottom, i.e. it rises into place)
/// @param once      - Fade only the first time it becomes visible (default true)
///
/// Honours `prefers-reduced-motion`: the element ARRIVES at its final state
/// without animating, rather than not arriving at all.
///
/// @example
/// .hero {
///   @fade-in(duration: 800ms, threshold: 0.2)
/// }
///
/// @example a declared easing curve, and travel from the left
/// @form easing --friendly { spring(300, 22, 1) }
/// .panel {
///   @fade-in(duration: 500ms, easing: --friendly, from: left, distance: 40px)
/// }
///
/// @example custom keyframes — the body REPLACES the built-in fade, so describe
/// the whole animation (an author writing keyframes is not decorating ours)
/// .card {
///   @fade-in(duration: 600ms) {
///     opacity: 0 -> 1;
///     scale: 0.95 -> 1;
///   }
/// }

%macro fade-in {

  %form {
    @fade-in(
      duration: $duration:time = 600ms,
      threshold: $threshold:number = 0.1,
      distance: $distance:length = 20px,
      easing: $easing:easing = ease-out,
      delay: $delay:time = 0,
      from: $from:ident = bottom,
      once: $once:bool = true
    ) {
      $customKeyframes:keyframes?
    }
  }

  // BUG-364: this used to bind `intersection` and animate in a `%when $visible`
  // body, which was empty with a TODO — "the intersection observer triggers but
  // no animation plays". That design could never work: `%when` is evaluated at
  // COMPILE time against captured macro bindings (stdlib/macros/each.st,
  // BUG-362), so it cannot see a runtime signal.
  //
  // `@reveal` is the model — a primitive that owns its observer and drives a
  // real WAAPI animation. `fade-in-engine` is that primitive for the
  // whole-element case.
  %binds {
    fade-in-engine(
      &self,
      duration: $duration,
      delay: $delay,
      distance: $distance,
      easing: $easing,
      threshold: $threshold,
      direction: $from,
      once: $once,
      keyframes: $customKeyframes
    )
  }
}


/// Fade In Up Macro
///
/// Convenience variant that fades in from below.

%macro fade-in-up {

  %form {
    @fade-in-up(
      $_duration:time = 600ms,
      distance: $_distance:length = 30px,
      easing: $_easing:easing = ease-out
    )
  }

  %includes {
    @fade-in(
      duration: $_duration,
      distance: $_distance,
      easing: $_easing
    )
  }
}


/// Fade In Down Macro
///
/// Convenience variant that fades in from above.

%macro fade-in-down {

  %form {
    @fade-in-down(
      $duration:time = 600ms,
      distance: $distance:length = 30px,
      easing: $easing:easing = ease-out
    )
  }

  // BUG-364 — see the note on `fade-in` above. `direction: "top"` is what makes
  // this the DOWN variant: the element starts above its place and settles into
  // it, the mirror of `fade-in-up`.
  %binds {
    fade-in-engine(
      &self,
      duration: $duration,
      distance: $distance,
      direction: "top",
      easing: $easing
    )
  }
}


/// Staggered Fade In Macro
///
/// Fades in children with staggered timing.
///
/// @example
/// .grid {
///   @fade-in-stagger(stagger: 100ms) {
///     > .item
///   }
/// }

%macro fade-in-stagger {

  %form {
    @fade-in-stagger(
      $duration:time = 600ms,
      stagger: $_stagger:stagger_value = 50ms,
      threshold: $threshold:number = 0.1,
      distance: $distance:length = 20px
    ) {
      $_childSelector:selector
    }
  }

  // BUG-364: the stagger variant fades CHILDREN in sequence, which is the shape
  // `reveal-engine` already implements (it staggers per unit). Binding it with
  // `split: "none"` is NOT available today, so this variant stays honest rather
  // than pretending: it fades the container as a whole, and the per-child
  // stagger is filed as follow-up work rather than silently doing nothing.
  //
  // `$_childSelector` and `$stagger` are captured and currently unconsumed —
  // the `$_` prefix marks that deliberately, so the gap is visible in the source
  // instead of hiding behind an empty body.
  %binds {
    fade-in-engine(
      &self,
      duration: $duration,
      distance: $distance,
      threshold: $threshold
    )
  }
}
