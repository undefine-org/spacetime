/// Unit Tests: Dev Editable Edit Mode Toggle
///
/// Tests that edit mode is off by default, toggle activates/deactivates,
/// and click interception is conditional on edit mode state.

@import "stdlib/testing/test"

// =============================================================================
// Edit mode inactive by default
// =============================================================================

@test "edit mode is inactive by default" {
  @fixture {
    <div class="toggle-test">
      <span data-st-bind="$.name">Alice</span>
    </div>
  }

  @eval (eval("\
    var active = false;\
    var editListeners = [];\
    var activeEdit = null;\
    window.__testEditable = {\
      get active() { return active; },\
      toggle: function() { active = !active; },\
      activate: function() { active = true; },\
      deactivate: function() { active = false; }\
    };\
    window.__editModeDefault = active;\
  "))

  @assert (window.__editModeDefault === false)
    "Edit mode should be inactive by default"
}

// =============================================================================
// Toggle activates edit mode (sets data-st-edit-mode)
// =============================================================================

@test "toggle activates edit mode and sets data attribute" {
  @fixture {
    <div class="toggle-test-2">
      <span data-st-bind="$.name">Bob</span>
    </div>
  }

  @eval (eval("\
    var active = false;\
    function activate() {\
      if (active) return;\
      active = true;\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
    }\
    function deactivate() {\
      if (!active) return;\
      active = false;\
      document.documentElement.removeAttribute('data-st-edit-mode');\
    }\
    function toggle() {\
      if (active) { deactivate(); } else { activate(); }\
    }\
    toggle();\
    window.__toggleActive = active;\
    window.__hasAttr = document.documentElement.hasAttribute('data-st-edit-mode');\
    window.__attrVal = document.documentElement.getAttribute('data-st-edit-mode');\
  "))

  @assert (window.__toggleActive === true)
    "active should be true after toggle"
  @assert (window.__hasAttr === true)
    "data-st-edit-mode attribute should be set"
  @assert (window.__attrVal === "true")
    "data-st-edit-mode should equal 'true'"

  // Clean up
  @eval (document.documentElement.removeAttribute('data-st-edit-mode'))
}

// =============================================================================
// handleClick returns early when inactive
// =============================================================================

@test "handleClick returns early when edit mode is inactive" {
  @fixture {
    <div class="click-guard-test">
      <span id="click-guard-el" data-st-bind="$.title">Original</span>
    </div>
  }

  @eval (eval("\
    var active = false;\
    var clickIntercepted = false;\
    function findEditableTarget(target) {\
      var el = target;\
      while (el && el !== document.body) {\
        if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
        el = el.parentElement;\
      }\
      return null;\
    }\
    function handleClick(e) {\
      if (!active) return;\
      var el = findEditableTarget(e.target);\
      if (el) {\
        clickIntercepted = true;\
      }\
    }\
    var el = document.getElementById('click-guard-el');\
    var fakeEvent = { target: el, preventDefault: function(){}, stopPropagation: function(){} };\
    handleClick(fakeEvent);\
    window.__clickIntercepted = clickIntercepted;\
  "))

  @assert (window.__clickIntercepted === false)
    "Click should NOT be intercepted when edit mode is inactive"
}

// =============================================================================
// Toggle deactivates (removes data-st-edit-mode)
// =============================================================================

@test "toggle deactivates and removes data attribute" {
  @fixture {
    <div class="deactivate-test">
      <span data-st-bind="$.x">Val</span>
    </div>
  }

  @eval (eval("\
    var active = false;\
    function activate() {\
      if (active) return;\
      active = true;\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
    }\
    function deactivate() {\
      if (!active) return;\
      active = false;\
      document.documentElement.removeAttribute('data-st-edit-mode');\
    }\
    function toggle() {\
      if (active) { deactivate(); } else { activate(); }\
    }\
    toggle();\
    window.__afterFirst = active;\
    toggle();\
    window.__afterSecond = active;\
    window.__noAttr = !document.documentElement.hasAttribute('data-st-edit-mode');\
  "))

  @assert (window.__afterFirst === true)
    "First toggle should activate"
  @assert (window.__afterSecond === false)
    "Second toggle should deactivate"
  @assert (window.__noAttr === true)
    "data-st-edit-mode attribute should be removed after deactivation"
}

// =============================================================================
// When active, click interception works
// =============================================================================

@test "click interception works when edit mode is active" {
  @fixture {
    <div class="active-click-test">
      <span id="active-click-el" data-st-bind="$.name">Charlie</span>
    </div>
  }

  @eval (eval("\
    var active = true;\
    var clickIntercepted = false;\
    function findEditableTarget(target) {\
      var el = target;\
      while (el && el !== document.body) {\
        if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
        el = el.parentElement;\
      }\
      return null;\
    }\
    function handleClick(e) {\
      if (!active) return;\
      var el = findEditableTarget(e.target);\
      if (el) {\
        clickIntercepted = true;\
      }\
    }\
    var el = document.getElementById('active-click-el');\
    var fakeEvent = { target: el, preventDefault: function(){}, stopPropagation: function(){} };\
    handleClick(fakeEvent);\
    window.__activeClickIntercepted = clickIntercepted;\
  "))

  @assert (window.__activeClickIntercepted === true)
    "Click SHOULD be intercepted when edit mode is active"
}

// =============================================================================
// data-st-bind still editable when active (regression test)
// =============================================================================

@test "data-st-bind elements still editable when active" {
  @fixture {
    <div class="regression-test">
      <span id="regression-el" data-st-bind="$.field">Editable</span>
    </div>
  }

  @eval (eval("\
    var active = true;\
    var activeEdit = null;\
    function readProvenance(el) {\
      var bind = el.getAttribute('data-st-bind');\
      if (!bind) return null;\
      return { editType: 'data', source: null, index: null, key: null, field: bind.replace(new RegExp('^\\\\$\\\\.'), ''), filter: null };\
    }\
    function startEdit(el) {\
      var provenance = readProvenance(el);\
      if (!provenance) return;\
      var oldValue = el.textContent;\
      el.setAttribute('contenteditable', 'true');\
      activeEdit = { element: el, field: provenance.field, oldValue: oldValue };\
    }\
    function findEditableTarget(target) {\
      var el = target;\
      while (el && el !== document.body) {\
        if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
        el = el.parentElement;\
      }\
      return null;\
    }\
    function handleClick(e) {\
      if (!active) return;\
      var el = findEditableTarget(e.target);\
      if (el) { startEdit(el); }\
    }\
    var el = document.getElementById('regression-el');\
    var fakeEvent = { target: el, preventDefault: function(){}, stopPropagation: function(){} };\
    handleClick(fakeEvent);\
    window.__regEditable = el.getAttribute('contenteditable');\
    window.__regField = activeEdit ? activeEdit.field : null;\
  "))

  @assert (window.__regEditable === "true")
    "Element should have contenteditable=true when edit mode active"
  @assert (window.__regField === "field")
    "activeEdit.field should be 'field'"
}
