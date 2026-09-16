/// Meta-Tests: @when Action Directive
///
/// Tests that @when correctly dispatches events and performs actions.

// =============================================================================
// Click Actions
// =============================================================================

@test "@when click dispatches click event" {
  @fixture {
    <button data-click-test>Click Me</button>
  }
  @eval (window.__clickFired = false)
  @eval (document.querySelector('[data-click-test]').addEventListener('click', () => { window.__clickFired = true; }))
  @when [data-click-test] click
  @assert (window.__clickFired === true)
}

@test "@when dblclick dispatches dblclick event" {
  @fixture {
    <div data-dblclick-test>Double Click</div>
  }
  @eval (window.__dblclickFired = false)
  @eval (document.querySelector('[data-dblclick-test]').addEventListener('dblclick', () => { window.__dblclickFired = true; }))
  @when [data-dblclick-test] dblclick
  @assert (window.__dblclickFired === true)
}

// =============================================================================
// Input Actions
// =============================================================================

@test "@when input sets value and dispatches event" {
  @fixture {
    <input data-input-action type="text" value="">
  }
  @eval (window.__inputFired = false)
  @let (el = document.querySelector('[data-input-action]'))
  @eval (el.addEventListener('input', () => { window.__inputFired = true; }))
  @when [data-input-action] input "hello world"
  @assert (el.value === 'hello world')
  @assert (window.__inputFired === true)
}

@test "@when change sets value and dispatches event" {
  @fixture {
    <select data-change-action>
      <option value="a">A</option>
      <option value="b">B</option>
    </select>
  }
  @eval (window.__changeFired = false)
  @let (el = document.querySelector('[data-change-action]'))
  @eval (el.addEventListener('change', () => { window.__changeFired = true; }))
  @when [data-change-action] change "b"
  @assert (el.value === 'b')
  @assert (window.__changeFired === true)
}

// =============================================================================
// Focus Actions
// =============================================================================

@test "@when focus focuses element" {
  @fixture {
    <input data-focus-test type="text">
  }
  @let (el = document.querySelector('[data-focus-test]'))
  @when [data-focus-test] focus
  @assert (document.activeElement === el)
}

@test "@when blur blurs element" {
  @fixture {
    <input data-blur-test type="text">
  }
  @let (el = document.querySelector('[data-blur-test]'))
  @eval (el.focus())
  @assert (document.activeElement === el)
  @when [data-blur-test] blur
  @assert (document.activeElement !== el)
}

// =============================================================================
// Mouse Actions
// =============================================================================

@test "@when hover dispatches mouseenter event" {
  @fixture {
    <div data-hover-test>Hover Me</div>
  }
  @eval (window.__hoverFired = false)
  @eval (document.querySelector('[data-hover-test]').addEventListener('mouseenter', () => { window.__hoverFired = true; }))
  @when [data-hover-test] hover
  @assert (window.__hoverFired === true)
}

@test "@when mouseleave dispatches mouseleave event" {
  @fixture {
    <div data-leave-test>Leave Me</div>
  }
  @eval (window.__leaveFired = false)
  @eval (document.querySelector('[data-leave-test]').addEventListener('mouseleave', () => { window.__leaveFired = true; }))
  @when [data-leave-test] mouseleave
  @assert (window.__leaveFired === true)
}

// =============================================================================
// Keyboard Actions
// =============================================================================

@test "@when keydown dispatches keydown event with key" {
  @fixture {
    <input data-keydown-test type="text">
  }
  @eval (window.__keydownKey = null)
  @eval (document.querySelector('[data-keydown-test]').addEventListener('keydown', (e) => { window.__keydownKey = e.key; }))
  @when [data-keydown-test] keydown "Enter"
  @assert (window.__keydownKey === 'Enter')
}

@test "@when press is alias for keydown" {
  @fixture {
    <input data-press-test type="text">
  }
  @eval (window.__pressKey = null)
  @eval (document.querySelector('[data-press-test]').addEventListener('keydown', (e) => { window.__pressKey = e.key; }))
  @when [data-press-test] press "Escape"
  @assert (window.__pressKey === 'Escape')
}

@test "@when keyup dispatches keyup event" {
  @fixture {
    <input data-keyup-test type="text">
  }
  @eval (window.__keyupKey = null)
  @eval (document.querySelector('[data-keyup-test]').addEventListener('keyup', (e) => { window.__keyupKey = e.key; }))
  @when [data-keyup-test] keyup "Tab"
  @assert (window.__keyupKey === 'Tab')
}

// =============================================================================
// Form Actions
// =============================================================================

@test "@when submit dispatches submit event" {
  @fixture {
    <form data-submit-test><button type="submit">Submit</button></form>
  }
  @eval (window.__submitFired = false)
  @eval (document.querySelector('[data-submit-test]').addEventListener('submit', (e) => { e.preventDefault(); window.__submitFired = true; }))
  @when [data-submit-test] submit
  @assert (window.__submitFired === true)
}

@test "@when check sets checked and fires change" {
  @fixture {
    <input data-check-test type="checkbox">
  }
  @eval (window.__checkChangeFired = false)
  @let (el = document.querySelector('[data-check-test]'))
  @eval (el.addEventListener('change', () => { window.__checkChangeFired = true; }))
  @when [data-check-test] check
  @assert (el.checked === true)
  @assert (window.__checkChangeFired === true)
}

@test "@when uncheck sets unchecked and fires change" {
  @fixture {
    <input data-uncheck-test type="checkbox" checked>
  }
  @eval (window.__uncheckChangeFired = false)
  @let (el = document.querySelector('[data-uncheck-test]'))
  @eval (el.addEventListener('change', () => { window.__uncheckChangeFired = true; }))
  @when [data-uncheck-test] uncheck
  @assert (el.checked === false)
  @assert (window.__uncheckChangeFired === true)
}

// =============================================================================
// @when Context
// =============================================================================

@test "@when stores target element in __ctx" {
  @fixture {
    <button data-ctx-when>Button</button>
  }
  @when [data-ctx-when] click
  @assert (__ctx.targetElement === document.querySelector('[data-ctx-when]'))
}

@test "@when stores last action in __ctx" {
  @fixture {
    <button data-action-when>Button</button>
  }
  @when [data-action-when] click
  @assert (__ctx.lastAction === 'click')
}

// =============================================================================
// @when Error Handling
// =============================================================================

// Note: @when error tests require try/catch which isn't valid in Spacetime test bodies.

// =============================================================================
// Generic Event Dispatch
// =============================================================================

@test "@when dispatches custom events" {
  @fixture {
    <div data-custom-event>Custom</div>
  }
  @eval (window.__customEventFired = false)
  @eval (document.querySelector('[data-custom-event]').addEventListener('my-custom-event', () => { window.__customEventFired = true; }))
  @when [data-custom-event] my-custom-event
  @assert (window.__customEventFired === true)
}
