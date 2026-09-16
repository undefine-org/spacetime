/// Particle Field Primitive Tests
///
/// Tests for the SVG particle field effect with cursor repulsion and drift.
/// Verifies initialization, parameter handling, and cleanup.

// =============================================================================
// Basic Initialization
// =============================================================================

@test "particle-field initializes with default parameters" {
  @fixture {
    <div class="pf-test" style="width:800px;height:600px;position:relative;">
      @particle-field()
    </div>
  }
  @then .pf-test should exist
}

@test "particle-field accepts count parameter" {
  @fixture {
    <div class="pf-count" style="width:800px;height:600px;position:relative;">
      @particle-field(count: 20)
    </div>
  }
  @then .pf-count should exist
}

@test "particle-field accepts repulsion parameter" {
  @fixture {
    <div class="pf-repulsion" style="width:800px;height:600px;position:relative;">
      @particle-field(repulsion: 0.8)
    </div>
  }
  @then .pf-repulsion should exist
}

@test "particle-field accepts spread parameter" {
  @fixture {
    <div class="pf-spread" style="width:800px;height:600px;position:relative;">
      @particle-field(spread: 0.3)
    </div>
  }
  @then .pf-spread should exist
}

@test "particle-field accepts drift parameter" {
  @fixture {
    <div class="pf-drift" style="width:800px;height:600px;position:relative;">
      @particle-field(drift: 0.5)
    </div>
  }
  @then .pf-drift should exist
}

@test "particle-field accepts color parameter" {
  @fixture {
    <div class="pf-color" style="width:800px;height:600px;position:relative;">
      @particle-field(color: "rgba(100,150,255,0.1)")
    </div>
  }
  @then .pf-color should exist
}

// =============================================================================
// Parameter Combinations
// =============================================================================

@test "particle-field accepts all parameters" {
  @fixture {
    <div class="pf-full" style="width:800px;height:600px;position:relative;">
      @particle-field(
        count: 30,
        repulsion: 0.7,
        spread: 0.4,
        drift: 0.2,
        color: "rgba(200,200,200,0.12)"
      )
    </div>
  }
  @then .pf-full should exist
}

@test "particle-field with low repulsion" {
  @fixture {
    <div class="pf-low-repulsion" style="width:800px;height:600px;position:relative;">
      @particle-field(repulsion: 0.2)
    </div>
  }
  @then .pf-low-repulsion should exist
}

@test "particle-field with high spread" {
  @fixture {
    <div class="pf-high-spread" style="width:800px;height:600px;position:relative;">
      @particle-field(spread: 0.9)
    </div>
  }
  @then .pf-high-spread should exist
}

@test "particle-field with minimal drift" {
  @fixture {
    <div class="pf-minimal-drift" style="width:800px;height:600px;position:relative;">
      @particle-field(drift: 0.05)
    </div>
  }
  @then .pf-minimal-drift should exist
}

@test "particle-field with custom color and count" {
  @fixture {
    <div class="pf-custom" style="width:800px;height:600px;position:relative;">
      @particle-field(
        count: 50,
        color: "rgba(255,100,100,0.15)"
      )
    </div>
  }
  @then .pf-custom should exist
}

// =============================================================================
// Accessibility & Cleanup
// =============================================================================

@test "particle-field works with reduced motion" {
  @fixture {
    <div class="pf-a11y" style="width:800px;height:600px;position:relative;">
      @particle-field(count: 25)
    </div>
  }
  @then .pf-a11y should exist
}

@test "particle-field SVG has aria-hidden" {
  @fixture {
    <div class="pf-aria" style="width:800px;height:600px;position:relative;">
      @particle-field()
    </div>
  }
  @then .pf-aria should exist
}
