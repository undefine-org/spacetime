/// Meta-Tests: @assert Custom Assertion Directive
///
/// Tests that @assert works for custom conditions.

// =============================================================================
// Basic Assertions
// =============================================================================

@test "@assert passes for truthy condition" {
  @assert true "True should pass"
  @assert 1 "Non-zero number should pass"
  @assert "string" "Non-empty string should pass"
  @let (obj = Object.create(null))
  @assert obj "Object should pass"
  @let (arr = Array())
  @assert (arr !== undefined) "Array should pass"
}

@test "@assert evaluates expressions" {
  @assert (1 + 1 === 2)
  @let (str = "hello")
  @assert (str.length === 5)
  @let (arr = Array.of(1, 2, 3))
  @assert (arr.includes(2))
}

@test "@assert with comparison operators" {
  @assert (5 > 3)
  @assert (3 < 5)
  @assert (5 >= 5)
  @assert (5 <= 5)
  @assert (5 !== 3)
}

@test "@assert with logical operators" {
  @assert (true && true)
  @assert (true || false)
  @assert (!false)
}

// =============================================================================
// DOM Assertions
// =============================================================================

@test "@assert with DOM queries" {
  @fixture {
    <div data-assert-dom class="target">Content</div>
  }
  // BUG-217: this queried `document.querySelector('div')` — the FIRST div in the
  // document, which is `@fixture`'s own wrapper element, not the div the fixture
  // declares. So `classList.contains('target')` and `textContent === 'Content'`
  // were asserted against the wrapper and failed. The test was wrong, not the
  // framework: query the element the fixture actually declares.
  //
  // (The TODO that stood here proposed a `@assert &([data-assert-dom]).value`
  // sugar. That is a separate feature request, not the reason this failed.)
  @let (el = document.querySelector('[data-assert-dom]'))
  @assert (el !== null)
  @assert (el.classList.contains('target'))
  @assert (el.textContent === 'Content')
}

@test "@assert with querySelectorAll" {
  @fixture {
    <ul data-assert-list>
      <li>One</li>
      <li>Two</li>
      <li>Three</li>
    </ul>
  }
  // TODO: should be:
  // [data-assert-list] li as &items;
  // (or: &items: [data-assert-list] li; (i don't know))
  // @assert &items.length == 3;
  @let (items = document.querySelectorAll('li'))
  @assert (items.length === 3)
}

// =============================================================================
// Function Call Assertions
// =============================================================================

@test "@assert with function results" {
  @let (result = 5 * 2)
  @assert (result === 10)
}

@test "@assert with array methods" {
  @let (arr = Array.of(1, 2, 3, 4, 5))
  @let (filtered = arr.filter(function(x) { return x > 3; }))
  @assert (filtered.length === 2)
  @let (mapped = arr.map(function(x) { return x * 2; }))
  @assert (mapped.includes(6))
}

// =============================================================================
// Object/Array Assertions
// =============================================================================

@test "@assert with object property access" {
  @let (user = { name: 'Alice', age: 30 })
  @assert (user.name === 'Alice')
  @assert (user.age >= 18)
}

@test "@assert with array indexing" {
  @let (items = Array.of('first', 'second', 'third'))
  @assert (items.at(0) === 'first')
  @assert (items.at(-1) === 'third')
}

// =============================================================================
// Type Checking Assertions
// =============================================================================

@test "@assert type checks" {
  @assert (typeof "string" === 'string')
  @assert (typeof 42 === 'number')
  @assert (typeof true === 'boolean')
  @let (obj = { x: 1 })
  @assert (typeof obj === 'object')
  @let (fn = function(x) { return x; })
  @assert (typeof fn === 'function')
  @let (arr = Array.of(1, 2))
  @assert (Array.isArray(arr))
}

@test "@assert instanceof checks" {
  @let (d = new Date())
  @assert (d instanceof Date)
  @let (arr = Array.of(1, 2, 3))
  @assert (arr instanceof Array)
}

// =============================================================================
// Null/Undefined Assertions
// =============================================================================

@test "@assert null checks" {
  @let (nullVal = null)
  @assert (nullVal === null)
}

@test "@assert undefined checks" {
  @let (obj = { existing: true })
  @assert (obj.undefinedProp === undefined)
}
