# Spacetime Testing DSL

A declarative testing framework built into Spacetime using the metasystem. Write tests that read like specifications using `@test`, `@given`, `@when`, `@then` syntax.

## Quick Start

```spacetime
// toggle.test.st

@test "toggle switches state on click" {
  @fixture {
    <button data-toggle data-state="off">Toggle Me</button>
  }

  @given [data-toggle] { data-state: "off" }
  @when [data-toggle] click
  @then [data-toggle] should have_state "on"
}
```

## Core Directives

### @test - Define a Test

```spacetime
@test "descriptive test name" {
  // test body with @given, @when, @then
}
```

#### Variants

```spacetime
// Skip a test temporarily
@test.skip "not yet implemented" {
  @then .feature should exist
}

// Run only this test (useful during development)
@test.only "focus on this test" {
  @then .box should be_visible
}
```

### @given - Set Initial State

Set up element properties, styles, or data before testing.

```spacetime
// Set data attributes
@given [data-toggle] { data-state: "off" }

// Set CSS properties
@given .box { opacity: 0; transform: translateY(20px) }

// Set form values
@given .input { value: "hello" }

// Set multiple properties
@given .modal {
  display: none
  data-open: "false"
}

// Set data bindings
@given $count = 0
@given $items = ["a", "b", "c"]
```

### @when - Perform Actions

Trigger user interactions on elements.

```spacetime
// Mouse events
@when .button click
@when .item dblclick
@when .card hover
@when .area mouseleave

// Keyboard events
@when .input keydown "Enter"
@when .field keyup "Escape"
@when .search press "Tab"

// Form events
@when .text-input input "hello world"
@when .select change "option2"
@when .checkbox check
@when .toggle uncheck
@when .field focus
@when .input blur
@when .form submit
@when .form reset

// Scroll
@when .container scroll 100

// Drag and drop
@when .draggable drag
@when .dropzone drop
```

### @then - Assert Conditions

Verify element state after actions.

#### Existence

```spacetime
@then .element should exist
@then .removed should not_exist
```

#### Visibility

```spacetime
@then .modal should be_visible
@then .hidden should be_hidden
```

#### Content

```spacetime
@then .title should have_text "Welcome"
@then .heading should have_exact_text "Hello World"
@then .container should have_html "<span>content</span>"
```

#### Form State

```spacetime
@then .input should have_value "submitted"
@then .checkbox should be_checked
@then .toggle should be_unchecked
@then .submit should be_disabled
@then .field should be_enabled
```

#### Focus

```spacetime
@then .input should be_focused
@then .button should not_be_focused
```

#### Classes

```spacetime
@then .card should have_class "active"
@then .item should not_have_class "selected"
```

#### Data Attributes & State

```spacetime
@then [data-toggle] should have_state "on"
@then .item should have_data "id=123"
```

#### Attributes

```spacetime
@then .link should have_attr "href=/home"
@then .image should have_attr "alt"
@then .button should not_have_attr "disabled"
```

#### Styles

```spacetime
@then .box should have_style "opacity: 1"
@then .panel should have_style "display: flex"
```

#### Collections

```spacetime
@then .items should have_length 5
@then .list should contain ".item"
@then .container should not_contain ".error"
```

#### Dimensions

```spacetime
@then .box should have_width 200
@then .panel should have_height 100
```

#### Empty State

```spacetime
@then .list should be_empty
@then .container should not_be_empty
```

## Waiting & Async

### @wait - Fixed Delay

```spacetime
@when .button click
@wait 500ms
@then .result should be_visible
```

### @wait_until - Condition-Based

```spacetime
@when .load-button click
@wait_until document.querySelector('.loaded') timeout: 5s
@then .content should be_visible
```

### @wait_for - Element Appearance

```spacetime
@when .trigger click
@wait_for .async-content timeout: 10s
@then .async-content should have_text "Loaded"
```

## Fixtures

Create temporary DOM structures for isolated testing.

```spacetime
@test "modal opens and closes" {
  @fixture {
    <div class="modal-container">
      <button class="open-btn">Open</button>
      <div class="modal" data-state="closed">
        <button class="close-btn">Close</button>
        <div class="content">Modal Content</div>
      </div>
    </div>
  }

  @given .modal { data-state: "closed" }
  @when .open-btn click
  @then .modal should have_state "open"

  @when .close-btn click
  @then .modal should have_state "closed"
}
```

Fixtures are automatically cleaned up after each test.

## Custom Assertions

Use `@assert` for conditions not covered by `@then`:

```spacetime
@test "custom validation" {
  @when .submit click
  @assert document.querySelectorAll('.error').length === 0 "no errors should appear"
  @assert $formValid === true "form should be valid"
}
```

## Mocking

Mock functions and APIs for isolated testing:

```spacetime
@test "handles API response" {
  @mock fetch returns { ok: true, json: () => ({ users: [] }) }

  @when .load-users click
  @wait_for .users-loaded timeout: 3s
  @then .user-list should be_empty
}

@test "handles API error" {
  @mock fetch returns { ok: false, status: 500 }

  @when .load-users click
  @wait_for .error-message timeout: 3s
  @then .error-message should have_text "Failed to load"
}
```

## Snapshots

Capture DOM snapshots for visual regression testing:

```spacetime
@test "component renders correctly" {
  @given .card { data-variant: "primary" }
  @snapshot "primary card initial"

  @when .card hover
  @snapshot "primary card hovered"

  @when .card click
  @snapshot "primary card active" of .card
}
```

## Cleanup

Register cleanup code for test teardown:

```spacetime
@test "with custom cleanup" {
  @fixture {
    <div id="test-root"></div>
  }

  @cleanup {
    localStorage.removeItem('test-key');
    sessionStorage.clear();
  }

  // ... test body
}
```

## Complete Example: Testing a Toggle Component

```spacetime
// File: components/toggle.test.st

// Test suite for the toggle component
.test-container {
  @test "toggle starts in off state" {
    @fixture {
      <button class="toggle" data-state="off">
        <span class="label">Off</span>
      </button>
    }

    @then .toggle should have_state "off"
    @then .label should have_text "Off"
  }

  @test "toggle switches to on when clicked" {
    @fixture {
      <button class="toggle" data-state="off">
        <span class="label">Off</span>
      </button>
    }

    @given .toggle { data-state: "off" }
    @when .toggle click
    @then .toggle should have_state "on"
  }

  @test "toggle switches back to off on second click" {
    @fixture {
      <button class="toggle" data-state="off"></button>
    }

    @when .toggle click
    @then .toggle should have_state "on"

    @when .toggle click
    @then .toggle should have_state "off"
  }

  @test "toggle is keyboard accessible" {
    @fixture {
      <button class="toggle" data-state="off"></button>
    }

    @when .toggle focus
    @then .toggle should be_focused

    @when .toggle keydown "Enter"
    @then .toggle should have_state "on"

    @when .toggle keydown " "
    @then .toggle should have_state "off"
  }

  @test.skip "toggle animates transition" {
    // Animation testing not yet implemented
    @then .toggle should have_class "transitioning"
  }
}
```

## Complete Example: Testing a Form

```spacetime
// File: components/login-form.test.st

.test-container {
  @test "form validates required fields" {
    @fixture {
      <form class="login-form">
        <input class="email" type="email" required>
        <input class="password" type="password" required>
        <button class="submit" type="submit">Login</button>
        <div class="error" style="display: none"></div>
      </form>
    }

    @when .submit click
    @then .email should have_class "invalid"
    @then .error should be_visible
  }

  @test "form submits with valid data" {
    @fixture {
      <form class="login-form">
        <input class="email" type="email" required>
        <input class="password" type="password" required>
        <button class="submit" type="submit">Login</button>
      </form>
    }

    @mock fetch returns { ok: true, json: () => ({ token: "abc123" }) }

    @when .email input "user@example.com"
    @when .password input "secretpass"
    @when .submit click

    @wait_for .success-message timeout: 3s
    @then .success-message should have_text "Welcome"
  }

  @test "form shows error on failed login" {
    @fixture {
      <form class="login-form">
        <input class="email" type="email">
        <input class="password" type="password">
        <button class="submit">Login</button>
        <div class="error"></div>
      </form>
    }

    @mock fetch returns { ok: false, status: 401 }

    @when .email input "wrong@example.com"
    @when .password input "wrongpass"
    @when .submit click

    @wait 500ms
    @then .error should have_text "Invalid credentials"
  }
}
```

## Running Tests

### In the Browser

Tests run automatically when the page loads. Results are displayed in the test reporter UI and logged to the console.

### Programmatically

```javascript
// Run all tests
await window.__spacetime_run_tests();

// Run filtered tests
await window.__spacetime_run_tests("toggle");

// Access results
console.log(window.__spacetime_test_results);
```

### Test Events

The test runner emits events you can listen to:

```javascript
document.addEventListener('test.start', (e) => {
  console.log(`Running ${e.detail.total} tests`);
});

document.addEventListener('test.pass', (e) => {
  console.log(`✓ ${e.detail.name}`);
});

document.addEventListener('test.fail', (e) => {
  console.log(`✗ ${e.detail.name}: ${e.detail.error}`);
});

document.addEventListener('test.complete', (e) => {
  const { passed, failed, skipped } = e.detail;
  console.log(`Done: ${passed} passed, ${failed} failed, ${skipped} skipped`);
});
```

## Best Practices

1. **One assertion focus per test** - Test one behavior at a time
2. **Descriptive test names** - Names should describe the expected behavior
3. **Use fixtures** - Isolate tests with their own DOM
4. **Clean up side effects** - Use `@cleanup` for localStorage, timers, etc.
5. **Mock external dependencies** - Use `@mock` for APIs and external services
6. **Wait appropriately** - Use `@wait_for` over fixed `@wait` when possible
7. **Test user interactions** - Focus on what users do, not implementation details

## Assertion Reference

| Assertion | Description | Example |
|-----------|-------------|---------|
| `exist` | Element exists in DOM | `should exist` |
| `not_exist` | Element doesn't exist | `should not_exist` |
| `be_visible` | Element is visible | `should be_visible` |
| `be_hidden` | Element is hidden | `should be_hidden` |
| `have_text` | Contains text | `should have_text "Hello"` |
| `have_exact_text` | Exact text match | `should have_exact_text "Hello"` |
| `have_value` | Form input value | `should have_value "test"` |
| `be_checked` | Checkbox is checked | `should be_checked` |
| `be_unchecked` | Checkbox is unchecked | `should be_unchecked` |
| `be_disabled` | Element is disabled | `should be_disabled` |
| `be_enabled` | Element is enabled | `should be_enabled` |
| `be_focused` | Element has focus | `should be_focused` |
| `have_class` | Has CSS class | `should have_class "active"` |
| `not_have_class` | Doesn't have class | `should not_have_class "hidden"` |
| `have_state` | Has data-state value | `should have_state "open"` |
| `have_data` | Has data attribute | `should have_data "id=123"` |
| `have_attr` | Has attribute | `should have_attr "href=/home"` |
| `have_style` | Has computed style | `should have_style "opacity: 1"` |
| `have_length` | Collection count | `should have_length 5` |
| `contain` | Contains child element | `should contain ".item"` |
| `be_empty` | Has no children | `should be_empty` |
| `have_width` | Element width in px | `should have_width 200` |
| `have_height` | Element height in px | `should have_height 100` |

## Action Reference

| Action | Description | Example |
|--------|-------------|---------|
| `click` | Mouse click | `@when .btn click` |
| `dblclick` | Double click | `@when .item dblclick` |
| `hover` | Mouse enter | `@when .card hover` |
| `mouseleave` | Mouse leave | `@when .card mouseleave` |
| `focus` | Focus element | `@when .input focus` |
| `blur` | Blur element | `@when .input blur` |
| `input` | Input text | `@when .field input "text"` |
| `change` | Change value | `@when .select change "opt"` |
| `keydown` | Key press | `@when .input keydown "Enter"` |
| `keyup` | Key release | `@when .input keyup "Escape"` |
| `check` | Check checkbox | `@when .cb check` |
| `uncheck` | Uncheck checkbox | `@when .cb uncheck` |
| `scroll` | Scroll element | `@when .list scroll 100` |
| `submit` | Submit form | `@when .form submit` |
| `drag` | Start drag | `@when .item drag` |
| `drop` | Drop | `@when .zone drop` |
