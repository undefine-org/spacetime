/// Meta-Tests: @then Assertion Directive
///
/// Tests assertion types to verify they work correctly.
/// Note: Tests that require try/catch for error checking are omitted
/// as raw JS is not valid in test bodies.

// =============================================================================
// Existence Assertions
// =============================================================================

@test "@then exist passes for existing element" {
  @fixture {
    <div data-exist-test>I exist</div>
  }
  @then [data-exist-test] should exist
}

@test "@then not_exist passes for missing element" {
  @then [data-definitely-not-here] should not_exist
}

// =============================================================================
// Visibility Assertions
// =============================================================================

@test "@then be_visible passes for visible element" {
  @fixture {
    <div data-visible-test style="display: block">Visible</div>
  }
  @then [data-visible-test] should be_visible
}

@test "@then be_hidden passes for hidden element" {
  @fixture {
    <div data-hidden-test style="display: none">Hidden</div>
  }
  @then [data-hidden-test] should be_hidden
}

// =============================================================================
// Content Assertions
// =============================================================================

@test "@then have_text passes when text is contained" {
  @fixture {
    <div data-text-test>Hello World</div>
  }
  @then [data-text-test] should have_text "Hello"
  @then [data-text-test] should have_text "World"
}

@test "@then have_exact_text passes for exact match" {
  @fixture {
    <div data-exact-text>Trimmed</div>
  }
  @then [data-exact-text] should have_exact_text "Trimmed"
}

@test "@then have_html passes when HTML is contained" {
  @fixture {
    <div data-html-test><span>Nested</span></div>
  }
  @then [data-html-test] should have_html "<span>Nested</span>"
}

// =============================================================================
// Form Value Assertions
// =============================================================================

@test "@then have_value passes for matching input value" {
  @fixture {
    <input data-value-test type="text" value="test value">
  }
  @then [data-value-test] should have_value "test value"
}

@test "@then be_checked passes for checked checkbox" {
  @fixture {
    <input data-checked-test type="checkbox" checked>
  }
  @then [data-checked-test] should be_checked
}

@test "@then be_unchecked passes for unchecked checkbox" {
  @fixture {
    <input data-unchecked-test type="checkbox">
  }
  @then [data-unchecked-test] should be_unchecked
}

@test "@then be_disabled passes for disabled element" {
  @fixture {
    <button data-disabled-test disabled>Disabled</button>
  }
  @then [data-disabled-test] should be_disabled
}

@test "@then be_enabled passes for enabled element" {
  @fixture {
    <button data-enabled-test>Enabled</button>
  }
  @then [data-enabled-test] should be_enabled
}

// =============================================================================
// Class Assertions
// =============================================================================

@test "@then have_class passes when class exists" {
  @fixture {
    <div data-class-test class="active highlighted">Classy</div>
  }
  @then [data-class-test] should have_class "active"
  @then [data-class-test] should have_class "highlighted"
}

@test "@then not_have_class passes when class missing" {
  @fixture {
    <div data-no-class class="one">No Class</div>
  }
  @then [data-no-class] should not_have_class "two"
}

// =============================================================================
// State and Data Assertions
// =============================================================================

@test "@then have_state passes for matching data-state" {
  @fixture {
    <div data-state-test data-state="active">State</div>
  }
  @then [data-state-test] should have_state "active"
}

@test "@then have_data passes for matching data attribute" {
  @fixture {
    <div data-attr-test data-count="5">Data</div>
  }
  @then [data-attr-test] should have_data "count=5"
}

// =============================================================================
// Attribute Assertions
// =============================================================================

@test "@then have_attr passes for attribute with value" {
  @fixture {
    <input data-attr-value type="text" placeholder="Enter text">
  }
  @then [data-attr-value] should have_attr "placeholder=Enter text"
}

@test "@then have_attr passes for attribute existence" {
  @fixture {
    <input data-attr-exists required>
  }
  @then [data-attr-exists] should have_attr "required"
}

@test "@then not_have_attr passes when attribute missing" {
  @fixture {
    <input data-no-attr type="text">
  }
  @then [data-no-attr] should not_have_attr "disabled"
}

// =============================================================================
// Style Assertions
// =============================================================================

@test "@then have_style passes for matching computed style" {
  @fixture {
    <div data-style-test style="opacity: 0.5">Styled</div>
  }
  @then [data-style-test] should have_style "opacity: 0.5"
}

// =============================================================================
// Collection Assertions
// =============================================================================

@test "@then have_length passes for correct count" {
  @fixture {
    <ul data-list-test>
      <li class="item">One</li>
      <li class="item">Two</li>
      <li class="item">Three</li>
    </ul>
  }
  @then [data-list-test] .item should have_length 3
}

@test "@then contain passes when child exists" {
  @fixture {
    <div data-contain-test>
      <span class="child">Child</span>
    </div>
  }
  @then [data-contain-test] should contain ".child"
}

@test "@then not_contain passes when child missing" {
  @fixture {
    <div data-not-contain>No Child</div>
  }
  @then [data-not-contain] should not_contain ".missing-child"
}

// =============================================================================
// Dimension Assertions
// =============================================================================

@test "@then have_width passes for matching width" {
  @fixture {
    <div data-width-test style="width: 100px; display: inline-block">Width</div>
  }
  @then [data-width-test] should have_width 100
}

@test "@then have_height passes for matching height" {
  @fixture {
    <div data-height-test style="height: 50px">Height</div>
  }
  @then [data-height-test] should have_height 50
}

// =============================================================================
// Empty State Assertions
// =============================================================================

@test "@then be_empty passes for element with no children" {
  @fixture {
    <div data-empty-test></div>
  }
  @then [data-empty-test] should be_empty
}

@test "@then not_be_empty passes for element with children" {
  @fixture {
    <div data-not-empty-test><span>Child</span></div>
  }
  @then [data-not-empty-test] should not_be_empty
}
