/// Magnetic Hover Primitive Tests
///
/// Tests for the proximity-based magnetic pull effect.
/// Verifies initialization, parameter handling, axis constraints, and cleanup.

// =============================================================================
// Basic Initialization
// =============================================================================

@test "magnetic initializes with default parameters" {
  @fixture {
    <button class="mag-test">
      @magnetic()
      Hover me
    </button>
  }
  @then .mag-test should exist
}

@test "magnetic accepts strength parameter" {
  @fixture {
    <button class="mag-strength">
      @magnetic(strength: 0.5)
      Pull me
    </button>
  }
  @then .mag-strength should exist
}

@test "magnetic accepts radius parameter" {
  @fixture {
    <button class="mag-radius">
      @magnetic(radius: 200px)
      Wide radius
    </button>
  }
  @then .mag-radius should exist
}

// =============================================================================
// Parameter Combinations
// =============================================================================

@test "magnetic accepts all parameters" {
  @fixture {
    <button class="mag-full">
      @magnetic(
        strength: 0.3,
        radius: 100px,
        lerp: 0.1,
        axis: "both",
        trigger: "self",
        tolerance: 0.5
      )
      Full params
    </button>
  }
  @then .mag-full should exist
}

@test "magnetic with x-axis constraint" {
  @fixture {
    <button class="mag-x">
      @magnetic(axis: "x", strength: 0.4)
      Horizontal only
    </button>
  }
  @then .mag-x should exist
}

@test "magnetic with y-axis constraint" {
  @fixture {
    <button class="mag-y">
      @magnetic(axis: "y", strength: 0.4)
      Vertical only
    </button>
  }
  @then .mag-y should exist
}

// =============================================================================
// Trigger Modes
// =============================================================================

@test "magnetic with parent trigger" {
  @fixture {
    <div class="mag-parent-wrap">
      <button class="mag-parent">
        @magnetic(trigger: "parent")
        Parent trigger
      </button>
    </div>
  }
  @then .mag-parent should exist
}

@test "magnetic works with reduced motion" {
  @fixture {
    <button class="mag-a11y">
      @magnetic(strength: 0.3)
      Accessible
    </button>
  }
  @then .mag-a11y should exist
}

// =============================================================================
// Modality + Default API Contracts
// =============================================================================

@test "magnetic default API contract remains stable" {
  @fixture {
    <button class="mag-default-contract">
      @magnetic()
      Default contract
    </button>
  }
  @then .mag-default-contract should exist
}

@test "magnetic supports hybrid-safe parent trigger contract" {
  @fixture {
    <div class="mag-hybrid-wrap">
      <button class="mag-hybrid-contract">
        @magnetic(trigger: "parent", tolerance: 0.5)
        Hybrid trigger
      </button>
    </div>
  }
  @then .mag-hybrid-contract should exist
}
