/// Meta-Tests: @given Setup Directive
///
/// Tests that @given correctly sets up element state and data bindings.

// =============================================================================
// @given Element State
// =============================================================================

@test "@given sets data-state attribute" {
  @fixture {
    <div data-given-test data-state="initial">Test Element</div>
  }
  @given [data-given-test] { data-state: "modified" }
  @assert (document.querySelector('[data-given-test]').dataset.state === 'modified')
}

@test "@given sets custom data attributes" {
  @fixture {
    <div data-custom-test>Test Element</div>
  }
  @given [data-custom-test] { data-value: "42", data-name: "test" }
  @assert (document.querySelector('[data-custom-test]').dataset.value === '42')
  @assert (document.querySelector('[data-custom-test]').dataset.name === 'test')
}

@test "@given sets textContent" {
  @fixture {
    <div data-text-test>Original</div>
  }
  @given [data-text-test] { textContent: "Modified" }
  @assert (document.querySelector('[data-text-test]').textContent === 'Modified')
}

@test "@given sets innerHTML" {
  @fixture {
    <div data-html-test></div>
  }
  @given [data-html-test] { innerHTML: "<span>Injected</span>" }
  @assert (document.querySelector('[data-html-test] span') !== null)
}

@test "@given sets form value" {
  @fixture {
    <input data-input-test type="text" value="">
  }
  @given [data-input-test] { value: "hello" }
  @assert (document.querySelector('[data-input-test]').value === 'hello')
}

@test "@given sets checkbox checked state" {
  @fixture {
    <input data-checkbox-test type="checkbox">
  }
  @given [data-checkbox-test] { checked: true }
  @assert (document.querySelector('[data-checkbox-test]').checked === true)
}

@test "@given sets disabled state" {
  @fixture {
    <button data-disabled-test>Button</button>
  }
  @given [data-disabled-test] { disabled: true }
  @assert (document.querySelector('[data-disabled-test]').disabled === true)
}

@test "@given sets CSS properties" {
  @fixture {
    <div data-style-test>Styled</div>
  }
  @given [data-style-test] { opacity: "0.5", color: "red" }
  @assert (document.querySelector('[data-style-test]').style.opacity === '0.5')
  @assert (document.querySelector('[data-style-test]').style.color === 'red')
}

// =============================================================================
// @given Context
// =============================================================================

@test "@given stores element in __ctx" {
  @fixture {
    <div data-ctx-test>Context Test</div>
  }
  @given [data-ctx-test] { data-state: "test" }
  @assert (__ctx.element === document.querySelector('[data-ctx-test]'))
}

// W2 REVIEW: `.state_props` splits a body into declarations. A naive split on
// `;`/`,` truncated any value CONTAINING one, and wrote the truncated text to the
// element — a WRONG write, which is worse than a skipped one because the test then
// asserts against silently corrupted content. These lock the quote/bracket-aware
// split.
@test "@given preserves a separator inside a quoted value" {
  @fixture {
    <div data-sep-test>original</div>
  }
  @given [data-sep-test] { textContent: "alpha, beta; gamma" }
  @assert (document.querySelector('[data-sep-test]').textContent === 'alpha, beta; gamma')
    "a comma or semicolon inside a quoted value must survive intact"
}

@test "@given preserves a colon inside a value" {
  @fixture {
    <div data-colon-test>original</div>
  }
  @given [data-colon-test] { textContent: "http://example.com/x" }
  @assert (document.querySelector('[data-colon-test]').textContent === 'http://example.com/x')
    "only the FIRST colon separates a property name from its value"
}

@test "@given still applies every pair of a multi-declaration body" {
  @fixture {
    <div data-multi-test>original</div>
  }
  @given [data-multi-test] { textContent: "hi, there", data-tag: "x;y" }
  @assert (document.querySelector('[data-multi-test]').textContent === 'hi, there')
    "the first pair must apply in full"
  @assert (document.querySelector('[data-multi-test]').dataset.tag === 'x;y')
    "and the SECOND pair must still be found — quote-awareness must not merge the list into one declaration"
}
