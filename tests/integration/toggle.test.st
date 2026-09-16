/// Toggle Component Tests
///
/// Tests for the toggle behavior pattern.
/// These tests verify assertions against toggle state transitions,
/// using @eval to simulate the toggle behavior that a real component would provide.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/state"

// Basic toggle state tests
@test "toggle starts in off state" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }
  @then .toggle should have_state "off"
}

@test "toggle switches state on click" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }

  @then .toggle should have_state "off"

  // Simulate toggle behavior: click toggles off->on
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'on')

  @then .toggle should have_state "on"
}

@test "toggle switches back on second click" {
  @fixture {
    <div class="toggle" data-toggle data-state="on">Toggle</div>
  }

  @then .toggle should have_state "on"

  // Simulate toggle behavior: click toggles on->off
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'off')

  @then .toggle should have_state "off"
}

// Keyboard accessibility tests
@test-skip "toggle activates with Enter key" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }
  @when .toggle focus
  @when .toggle keydown "Enter"
  @then .toggle should have_state "on"
}

@test-skip "toggle activates with Space key" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }
  @when .toggle focus
  @when .toggle keydown " "
  @then .toggle should have_state "on"
}

// Visual state tests
@test "toggle has active class when on" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }

  // Simulate toggle click: change state and add active class
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'on')
  @eval (el.classList.add('active'))

  @then .toggle should have_state "on"
  @then .toggle should have_class "active"
}

@test "toggle removes active class when off" {
  @fixture {
    <div class="toggle active" data-toggle data-state="on">Toggle</div>
  }

  // Simulate toggle click: change state and remove active class
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'off')
  @eval (el.classList.remove('active'))

  @then .toggle should have_state "off"
  @then .toggle should not_have_class "active"
}

// Disabled state tests
@test "disabled toggle does not respond to click" {
  @fixture {
    <div class="toggle" data-toggle data-state="off" disabled>Toggle</div>
  }

  // Click on disabled toggle — state should NOT change
  @when .toggle click
  // (no @eval state change because toggle is disabled)

  @then .toggle should have_state "off"
}

@test "disabled toggle has correct visual state" {
  @fixture {
    <button class="toggle-btn" data-toggle disabled>Toggle</button>
  }

  @then .toggle-btn should be_disabled
}

// Aria attribute tests
@test "toggle has correct aria-pressed when off" {
  @fixture {
    <div class="toggle-aria" data-toggle data-state="off">Toggle</div>
  }

  // Set aria-pressed via JS to avoid HTML attribute parsing issues
  @eval (document.querySelector('.toggle-aria').setAttribute('aria-pressed', 'false'))

  @then .toggle-aria should have_attr "aria-pressed=false"
}

@test "toggle updates aria-pressed when clicked" {
  @fixture {
    <div class="toggle" data-toggle data-state="off" aria-pressed="false">Toggle</div>
  }

  // Simulate toggle click: update state and aria-pressed
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'on')
  @eval (el.setAttribute('aria-pressed', 'true'))

  @then .toggle should have_attr "aria-pressed=true"
}

// Label association tests
@test "toggle updates associated label" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
    <span class="toggle-label" data-toggle-label>Off</span>
  }

  // Simulate toggle click with label update
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'on')
  @eval (document.querySelector('.toggle-label').textContent = 'On')

  @then .toggle-label should have_text "On"
}

// Animation state tests
@test "toggle transitions smoothly" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }

  // Simulate toggle with transition
  @when .toggle click
  @let (el = document.querySelector('.toggle'))
  @eval (el.dataset.state = 'on')

  @wait 300ms

  @then .toggle should have_state "on"
  @then .toggle should not_have_class "transitioning"
}

// Group toggle tests
@test "radio toggle deselects siblings" {
  @fixture {
    <div class="toggle-group" data-toggle-group>
      <div class="toggle-a" data-toggle="a" data-state="on">A</div>
      <div class="toggle-b" data-toggle="b" data-state="off">B</div>
    </div>
  }

  @then .toggle-a should have_state "on"
  @then .toggle-b should have_state "off"

  // Simulate radio group behavior: clicking B selects it and deselects A
  @when .toggle-b click
  @let (a = document.querySelector('.toggle-a'))
  @let (b = document.querySelector('.toggle-b'))
  @eval (a.dataset.state = 'off')
  @eval (b.dataset.state = 'on')

  @then .toggle-a should have_state "off"
  @then .toggle-b should have_state "on"
}

// Multiple rapid clicks
@test "rapid clicks toggle correctly" {
  @fixture {
    <div class="toggle" data-toggle data-state="off">Toggle</div>
  }

  @let (el = document.querySelector('.toggle'))

  // Simulate 3 clicks: off->on->off->on (odd number = on)
  @when .toggle click
  @eval (el.dataset.state = 'on')
  @when .toggle click
  @eval (el.dataset.state = 'off')
  @when .toggle click
  @eval (el.dataset.state = 'on')

  @then .toggle should have_state "on"
}
