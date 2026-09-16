/// Meta-Tests: @fixture and @cleanup Directives
///
/// Tests that fixtures are created correctly and cleanup runs after tests.

// =============================================================================
// @fixture Creation
// =============================================================================

@test "@fixture creates DOM elements" {
  @fixture {
    <div data-fixture-create>Created by fixture</div>
  }
  @assert (document.querySelector('[data-fixture-create]') !== null)
}

@test "@fixture creates nested elements" {
  @fixture {
    <div data-fixture-parent>
      <div data-fixture-child>
        <span data-fixture-grandchild>Nested</span>
      </div>
    </div>
  }
  @assert (document.querySelector('[data-fixture-parent]') !== null)
  @assert (document.querySelector('[data-fixture-child]') !== null)
  @assert (document.querySelector('[data-fixture-grandchild]') !== null)
}

@test "@fixture creates multiple sibling elements" {
  @fixture {
    <div data-sibling-1>One</div>
    <div data-sibling-2>Two</div>
    <div data-sibling-3>Three</div>
  }
  @assert (document.querySelector('[data-sibling-1]') !== null)
  @assert (document.querySelector('[data-sibling-2]') !== null)
  @assert (document.querySelector('[data-sibling-3]') !== null)
}

@test "@fixture preserves attributes" {
  @fixture {
    <input data-attr-fixture type="email" placeholder="test" required>
  }
  @let (el = document.querySelector('[data-attr-fixture]'))
  @assert (el.type === 'email')
  @assert (el.placeholder === 'test')
  @assert (el.required === true)
}

@test "@fixture preserves inline styles" {
  @fixture {
    <div data-style-fixture style="color: red; opacity: 0.5">Styled</div>
  }
  @let (el = document.querySelector('[data-style-fixture]'))
  @assert (el.style.color === 'red')
  @assert (el.style.opacity === '0.5')
}

@test "@fixture preserves classes" {
  @fixture {
    <div data-class-fixture class="one two three">Classy</div>
  }
  @let (el = document.querySelector('[data-class-fixture]'))
  @assert (el.classList.contains('one'))
  @assert (el.classList.contains('two'))
  @assert (el.classList.contains('three'))
}

// =============================================================================
// @fixture Container
// =============================================================================

@test "@fixture creates container with unique ID" {
  @fixture {
    <div data-container-test>Container Test</div>
  }
  @assert (__ctx.fixture !== null)
  @assert (__ctx.fixture.id.startsWith("__st_test_fixture_"))
}

@test "@fixture container is appended to body" {
  @fixture {
    <div data-body-test>Body Test</div>
  }
  @assert (__ctx.fixture.parentElement === document.body)
}

// =============================================================================
// @cleanup Registration
// =============================================================================

@test "@cleanup registers cleanup function" {
  @fixture {
    <div data-cleanup-test>Cleanup Test</div>
  }
  @cleanup {
    @eval (window.__cleanupRegistered = true)
  }
  @assert (Array.isArray(__ctx.cleanup))
  @assert (__ctx.cleanup.length >= 1)
}

@test "multiple @cleanup functions can be registered" {
  @fixture {
    <div data-multi-cleanup>Multi Cleanup</div>
  }
  @cleanup {
    @eval (window.__cleanup1 = true)
  }
  @cleanup {
    @eval (window.__cleanup2 = true)
  }
  // Fixture auto-registers one cleanup, plus our two
  @assert (__ctx.cleanup.length >= 2)
}

// =============================================================================
// @fixture Auto-Cleanup
// =============================================================================

@test "@fixture registers auto-cleanup" {
  @fixture {
    <div data-auto-cleanup>Auto Cleanup</div>
  }
  // Check that fixture removal is registered
  @eval (window.__hasFixtureCleanup = __ctx.cleanup.length > 0)
  @let (hasFixtureCleanup = window.__hasFixtureCleanup)
  @assert (hasFixtureCleanup)
}

// =============================================================================
// Cleanup Execution Order
// =============================================================================

@test "cleanup functions are called in registration order" {
  @eval (window.__cleanupOrder = [])
  @fixture {
    <div data-order-test>Order Test</div>
  }
  @cleanup {
    @eval (window.__cleanupOrder.push(1))
  }
  @cleanup {
    @eval (window.__cleanupOrder.push(2))
  }
  @cleanup {
    @eval (window.__cleanupOrder.push(3))
  }
  // Run cleanups manually to test order (skip first/fixture cleanup)
  @eval (eval('__ctx.cleanup.slice(1).forEach(function(f){f()})'))
  @assert (window.__cleanupOrder.join(',') === '1,2,3')
}

// =============================================================================
// @fixture with Forms
// =============================================================================

@test "@fixture creates functional forms" {
  @fixture {
    <form data-form-fixture>
      <input type="text" name="username" value="test">
      <input type="password" name="password">
      <button type="submit">Submit</button>
    </form>
  }
  @let (form = document.querySelector('[data-form-fixture]'))
  @assert (form.elements.username.value === 'test')
  @assert (form.querySelector('button[type="submit"]') !== null)
}

// =============================================================================
// @fixture with Lists
// =============================================================================

@test "@fixture creates lists" {
  @fixture {
    <ul data-list-fixture>
      <li>Item 1</li>
      <li>Item 2</li>
      <li>Item 3</li>
    </ul>
  }
  @let (list = document.querySelector('[data-list-fixture]'))
  @assert (list.children.length === 3)
}

// =============================================================================
// @fixture with Tables
// =============================================================================

@test "@fixture creates tables" {
  @fixture {
    <table data-table-fixture>
      <thead>
        <tr><th>Header</th></tr>
      </thead>
      <tbody>
        <tr><td>Cell</td></tr>
      </tbody>
    </table>
  }
  @let (table = document.querySelector('[data-table-fixture]'))
  @assert (table.querySelector('thead') !== null)
  @assert (table.querySelector('tbody') !== null)
}
