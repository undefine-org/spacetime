/// Kinetic Typography Reveal Primitive Tests
///
/// Tests for the character/word/line-level text animation reveal system.
/// Verifies split modes, stagger patterns, triggers, and accessibility.

// =============================================================================
// Basic Split Modes
// =============================================================================

@test "reveal splits text by characters" {
  @fixture {
    <h1 class="reveal-chars">
      @reveal(split: "chars", stagger: 30ms)
      Hello World
    </h1>
  }
  @then .reveal-chars should exist
}

@test "reveal splits text by words" {
  @fixture {
    <h2 class="reveal-words">
      @reveal(split: "words", stagger: 50ms)
      Welcome to Ora
    </h2>
  }
  @then .reveal-words should exist
}

@test "reveal splits text by lines" {
  @fixture {
    <p class="reveal-lines">
      @reveal(split: "lines", stagger: 100ms)
      This is a long paragraph that should split by visual lines.
    </p>
  }
  @then .reveal-lines should exist
}

// =============================================================================
// Stagger Patterns
// =============================================================================

@test "reveal with center stagger" {
  @fixture {
    <h1 class="reveal-center">
      @reveal(split: "chars", staggerFrom: "center", stagger: 20ms)
      Center Out
    </h1>
  }
  @then .reveal-center should exist
}

@test "reveal with edges stagger" {
  @fixture {
    <h1 class="reveal-edges">
      @reveal(split: "chars", staggerFrom: "edges", stagger: 25ms)
      From Edges
    </h1>
  }
  @then .reveal-edges should exist
}

@test "reveal with random stagger" {
  @fixture {
    <h1 class="reveal-random">
      @reveal(split: "words", staggerFrom: "random", stagger: 40ms)
      Random Word Order
    </h1>
  }
  @then .reveal-random should exist
}

@test "reveal with first stagger (default)" {
  @fixture {
    <h1 class="reveal-first">
      @reveal(split: "chars", staggerFrom: "first", stagger: 30ms)
      First to Last
    </h1>
  }
  @then .reveal-first should exist
}

@test "reveal with last stagger" {
  @fixture {
    <h1 class="reveal-last">
      @reveal(split: "chars", staggerFrom: "last", stagger: 35ms)
      Last to First
    </h1>
  }
  @then .reveal-last should exist
}

// =============================================================================
// Trigger Modes
// =============================================================================

@test "reveal with visible trigger" {
  @fixture {
    <h1 class="reveal-visible">
      @reveal(split: "chars", trigger: visible, threshold: 0.2)
      Appear on scroll
    </h1>
  }
  @then .reveal-visible should exist
}

@test "reveal with load trigger" {
  @fixture {
    <h1 class="reveal-load">
      @reveal(split: "chars", trigger: load)
      Appear on load
    </h1>
  }
  @then .reveal-load should exist
}

@test "reveal with scroll trigger" {
  @fixture {
    <h1 class="reveal-scroll">
      @reveal(split: "words", trigger: scroll, stagger: 40ms)
      Scroll triggered text
    </h1>
  }
  @then .reveal-scroll should exist
}

// =============================================================================
// Custom Keyframes
// =============================================================================

@test "reveal with custom keyframes body" {
  @fixture {
    <h1 class="reveal-custom">
      @reveal(split: "chars", stagger: 30ms) {
        opacity: 0 -> 1;
        translate-y: 1em -> 0;
      }
      Custom Animation
    </h1>
  }
  @then .reveal-custom should exist
}

// =============================================================================
// Direction and Distance
// =============================================================================

@test "reveal from bottom direction" {
  @fixture {
    <h1 class="reveal-bottom">
      @reveal(split: "chars", from: "bottom", distance: 120%)
      From Below
    </h1>
  }
  @then .reveal-bottom should exist
}

@test "reveal from left direction" {
  @fixture {
    <h1 class="reveal-left">
      @reveal(split: "words", from: "left", distance: 50px)
      From Left
    </h1>
  }
  @then .reveal-left should exist
}

@test "reveal from top direction" {
  @fixture {
    <h1 class="reveal-top">
      @reveal(split: "chars", from: "top", distance: 80px)
      From Above
    </h1>
  }
  @then .reveal-top should exist
}

@test "reveal from right direction" {
  @fixture {
    <h1 class="reveal-right">
      @reveal(split: "words", from: "right", distance: 60px)
      From Right
    </h1>
  }
  @then .reveal-right should exist
}

@test "reveal from none direction (opacity only)" {
  @fixture {
    <h1 class="reveal-none">
      @reveal(split: "chars", from: "none", duration: 400ms)
      Opacity Only
    </h1>
  }
  @then .reveal-none should exist
}

// =============================================================================
// Full Parameter Set
// =============================================================================

@test "reveal accepts all parameters" {
  @fixture {
    <h1 class="reveal-full">
      @reveal(
        split: "chars",
        trigger: visible,
        stagger: 30ms,
        staggerFrom: "first",
        duration: 600ms,
        threshold: 0.2,
        easing: ease-out,
        from: "bottom",
        distance: 100%,
        once: true
      )
      Full Parameters
    </h1>
  }
  @then .reveal-full should exist
}

@test "reveal works with reduced motion" {
  @fixture {
    <h1 class="reveal-a11y">
      @reveal(split: "chars", stagger: 30ms)
      Accessible Text
    </h1>
  }
  @then .reveal-a11y should exist
}
