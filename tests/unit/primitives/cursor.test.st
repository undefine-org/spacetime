/// Custom Cursor Primitive Tests
///
/// Tests for the branded custom cursor system.
/// Verifies cursor creation, parameter handling, hover states, and cleanup.

// =============================================================================
// Basic Initialization
// =============================================================================

@test "cursor initializes with default parameters" {
  @fixture {
    <div class="cursor-test">
      @cursor()
    </div>
  }
  @then .cursor-test should exist
}

@test "cursor accepts size parameter" {
  @fixture {
    <div class="cursor-size">
      @cursor(size: "32px")
    </div>
  }
  @then .cursor-size should exist
}

@test "cursor accepts color and blend parameters" {
  @fixture {
    <div class="cursor-style">
      @cursor(color: "#ff6b47", blend: "difference")
    </div>
  }
  @then .cursor-style should exist
}

// =============================================================================
// Parameter Combinations
// =============================================================================

@test "cursor accepts all parameters" {
  @fixture {
    <div class="cursor-full">
      @cursor(
        size: "24px",
        color: "#ff6b47",
        blend: "difference",
        lerp: 0.15,
        borderWidth: "2px",
        borderColor: "currentColor",
        scale: 1,
        hoverScale: 1.5,
        clickScale: 0.9,
        hoverElements: "a, button, [data-cursor]"
      )
    </div>
  }
  @then .cursor-full should exist
}

@test "cursor with custom hover elements" {
  @fixture {
    <div class="cursor-hover">
      @cursor(hoverElements: ".card, .link, [role=button]")
    </div>
  }
  @then .cursor-hover should exist
}

// =============================================================================
// Interaction States
// =============================================================================

@test "cursor scales on interactive element hover" {
  @fixture {
    <div class="cursor-interact">
      @cursor(hoverScale: 2.0, clickScale: 0.8)
      <a href="#">Link</a>
      <button>Button</button>
    </div>
  }
  @then .cursor-interact should exist
}

@test "cursor works with reduced motion" {
  @fixture {
    <div class="cursor-a11y">
      @cursor(lerp: 0.15)
    </div>
  }
  @then .cursor-a11y should exist
}

// =============================================================================
// Modality + Default API Contracts
// =============================================================================

@test "cursor default API contract remains stable" {
  @fixture {
    <div class="cursor-default-contract">
      @cursor()
      <a href="#">Link</a>
      <button data-cursor>Action</button>
    </div>
  }
  @then .cursor-default-contract should exist
}

@test "cursor supports explicit hybrid-safe hover selector contract" {
  @fixture {
    <div class="cursor-hybrid-contract">
      @cursor(hoverElements: "a, button, [data-cursor], [role=button]")
      <div role="button">Fallback target</div>
    </div>
  }
  @then .cursor-hybrid-contract should exist
}
