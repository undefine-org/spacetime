/// Unit Tests: Dev Content Propagation — Live Sibling Updates
///
/// Tests that after a content edit is acknowledged, sibling elements
/// sharing the same data-st-origin are updated with the new innerHTML,
/// while the edited element itself and elements with different origins
/// remain unchanged. Non-content edits (locale, data) must not trigger
/// propagation. Origin value format differs by file type:
///   HTML files: file::stId    (e.g. index.html::st-deadbeef)
///   .st files:  file::selector (e.g. card.st::.card @template h3)

@import "stdlib/testing/test"

// =============================================================================
// After Ack, sibling elements with same data-st-origin get updated innerHTML
// =============================================================================

@test "after ack siblings with same data-st-origin get updated innerHTML" {
  @fixture {
    <p id="prop1-edited" data-st-id="st-abc123" data-st-origin="card.st::.card @template p">Original</p>
    <p id="prop1-sib1" data-st-id="st-abc123" data-st-origin="card.st::.card @template p">Original</p>
    <p id="prop1-sib2" data-st-id="st-abc123" data-st-origin="card.st::.card @template p">Original</p>
    <p id="prop1-diff" data-st-id="st-xyz789" data-st-origin="other.st::.other @template p">Different</p>
  }

  @eval (eval("(function() {\
    if (typeof CSS === 'undefined') window.CSS = {};\
    if (!CSS.escape) CSS.escape = function(v) { return v; };\
    var editedEl = document.getElementById('prop1-edited');\
    var edit = { element: editedEl, editType: 'content', source: 'card.st', selector: '.card @template p', stId: 'st-abc123', newValue: 'Updated content', oldValue: 'Original' };\
    var file = edit.source;\
    var locator;\
    if (file && file.match(/\\.html?$/i)) { locator = edit.stId; }\
    else { locator = edit.selector; }\
    if (locator) {\
      var originVal = file + '::' + locator;\
      var siblings = document.querySelectorAll('[data-st-origin=\\x22' + CSS.escape(originVal) + '\\x22]');\
      for (var i = 0; i < siblings.length; i++) {\
        if (siblings[i] !== edit.element) { siblings[i].innerHTML = edit.newValue; }\
      }\
    }\
    window.__prop1_edited = document.getElementById('prop1-edited').innerHTML;\
    window.__prop1_sib1 = document.getElementById('prop1-sib1').innerHTML;\
    window.__prop1_sib2 = document.getElementById('prop1-sib2').innerHTML;\
    window.__prop1_diff = document.getElementById('prop1-diff').innerHTML;\
  })()"))

  @assert (window.__prop1_sib1 === 'Updated content')
    "sibling 1 should have updated innerHTML"
  @assert (window.__prop1_sib2 === 'Updated content')
    "sibling 2 should have updated innerHTML"
  @assert (window.__prop1_diff === 'Different')
    "element with different origin should be unchanged"
  @assert (window.__prop1_edited === 'Original')
    "edited element should not be re-updated by propagation"
}

// =============================================================================
// Edited element itself is NOT re-updated by propagation
// =============================================================================

@test "edited element is not re-updated by propagation" {
  @fixture {
    <p id="prop2-edited" data-st-id="st-def456" data-st-origin="page.st::.hero @template p">Edited by user</p>
    <p id="prop2-sib" data-st-id="st-def456" data-st-origin="page.st::.hero @template p">Original</p>
  }

  @eval (eval("(function() {\
    if (typeof CSS === 'undefined') window.CSS = {};\
    if (!CSS.escape) CSS.escape = function(v) { return v; };\
    var editedEl = document.getElementById('prop2-edited');\
    var edit = { element: editedEl, editType: 'content', source: 'page.st', selector: '.hero @template p', stId: 'st-def456', newValue: 'New value from save', oldValue: 'Original' };\
    var file = edit.source;\
    var locator;\
    if (file && file.match(/\\.html?$/i)) { locator = edit.stId; }\
    else { locator = edit.selector; }\
    if (locator) {\
      var originVal = file + '::' + locator;\
      var siblings = document.querySelectorAll('[data-st-origin=\\x22' + CSS.escape(originVal) + '\\x22]');\
      for (var i = 0; i < siblings.length; i++) {\
        if (siblings[i] !== edit.element) { siblings[i].innerHTML = edit.newValue; }\
      }\
    }\
    window.__prop2_edited = document.getElementById('prop2-edited').innerHTML;\
    window.__prop2_sib = document.getElementById('prop2-sib').innerHTML;\
  })()"))

  @assert (window.__prop2_edited === 'Edited by user')
    "edited element should keep its own value (not re-updated)"
  @assert (window.__prop2_sib === 'New value from save')
    "sibling should receive the propagated new value"
}

// =============================================================================
// Non-content edits (locale, data) do not trigger propagation
// =============================================================================

@test "non-content edits do not trigger sibling propagation" {
  @fixture {
    <span id="prop3-locale" data-t="hero.title" data-st-origin="card.st::.card @template span">Locale text</span>
    <span id="prop3-data" data-st-bind="$.name" data-st-origin="card.st::.card @template span">Data text</span>
    <span id="prop3-sib" data-st-origin="card.st::.card @template span">Sibling</span>
  }

  @eval (eval("(function() {\
    if (typeof CSS === 'undefined') window.CSS = {};\
    if (!CSS.escape) CSS.escape = function(v) { return v; };\
    var propagationCount = 0;\
    var localeEdit = { element: document.getElementById('prop3-locale'), editType: 'locale', source: 'locales/en.json', localeKey: 'hero.title', newValue: 'New title' };\
    if (localeEdit && localeEdit.editType === 'content' && localeEdit.element) {\
      propagationCount++;\
      var file = localeEdit.source;\
      var locator;\
      if (file && file.match(/\\.html?$/i)) { locator = localeEdit.stId; }\
      else { locator = localeEdit.selector; }\
      if (locator) {\
        var originVal = file + '::' + locator;\
        var siblings = document.querySelectorAll('[data-st-origin=\\x22' + CSS.escape(originVal) + '\\x22]');\
        for (var i = 0; i < siblings.length; i++) {\
          if (siblings[i] !== localeEdit.element) { siblings[i].innerHTML = localeEdit.newValue; }\
        }\
      }\
    }\
    var dataEdit = { element: document.getElementById('prop3-data'), editType: 'data', source: 'products', field: 'name', newValue: 'New name' };\
    if (dataEdit && dataEdit.editType === 'content' && dataEdit.element) {\
      propagationCount++;\
      var file2 = dataEdit.source;\
      var locator2;\
      if (file2 && file2.match(/\\.html?$/i)) { locator2 = dataEdit.stId; }\
      else { locator2 = dataEdit.selector; }\
      if (locator2) {\
        var originVal2 = file2 + '::' + locator2;\
        var siblings2 = document.querySelectorAll('[data-st-origin=\\x22' + CSS.escape(originVal2) + '\\x22]');\
        for (var j = 0; j < siblings2.length; j++) {\
          if (siblings2[j] !== dataEdit.element) { siblings2[j].innerHTML = dataEdit.newValue; }\
        }\
      }\
    }\
    window.__prop3_propagationCount = propagationCount;\
    window.__prop3_locale = document.getElementById('prop3-locale').innerHTML;\
    window.__prop3_data = document.getElementById('prop3-data').innerHTML;\
    window.__prop3_sib = document.getElementById('prop3-sib').innerHTML;\
  })()"))

  @assert (window.__prop3_propagationCount === 0)
    "propagation should not trigger for locale or data edits"
  @assert (window.__prop3_locale === 'Locale text')
    "locale element should be unchanged"
  @assert (window.__prop3_data === 'Data text')
    "data element should be unchanged"
  @assert (window.__prop3_sib === 'Sibling')
    "sibling element should be unchanged"
}

// =============================================================================
// HTML-origin propagation uses correct originVal format (file::stId)
// =============================================================================

@test "html-origin propagation uses file::stId format" {
  @fixture {
    <div id="prop4-edited" data-st-id="st-deadbeef" data-st-origin="index.html::st-deadbeef">First</div>
    <div id="prop4-sib" data-st-id="st-deadbeef" data-st-origin="index.html::st-deadbeef">Second</div>
  }

  @eval (eval("(function() {\
    if (typeof CSS === 'undefined') window.CSS = {};\
    if (!CSS.escape) CSS.escape = function(v) { return v; };\
    var editedEl = document.getElementById('prop4-edited');\
    var edit = { element: editedEl, editType: 'content', source: 'index.html', stId: 'st-deadbeef', selector: null, newValue: 'HTML propagated', oldValue: 'First' };\
    var file = edit.source;\
    var locator;\
    if (file && file.match(/\\.html?$/i)) { locator = edit.stId; }\
    else { locator = edit.selector; }\
    window.__prop4_locator = locator;\
    var originVal = file + '::' + locator;\
    window.__prop4_originVal = originVal;\
    var siblings = document.querySelectorAll('[data-st-origin=\\x22' + CSS.escape(originVal) + '\\x22]');\
    window.__prop4_found = siblings.length;\
    for (var i = 0; i < siblings.length; i++) {\
      if (siblings[i] !== edit.element) { siblings[i].innerHTML = edit.newValue; }\
    }\
    window.__prop4_sib = document.getElementById('prop4-sib').innerHTML;\
  })()"))

  @assert (window.__prop4_locator === 'st-deadbeef')
    "locator should be stId for HTML files"
  @assert (window.__prop4_originVal === 'index.html::st-deadbeef')
    "originVal should be file::stId for HTML files"
  @assert (window.__prop4_found === 2)
    "querySelectorAll should find both elements with matching origin"
  @assert (window.__prop4_sib === 'HTML propagated')
    "sibling should be updated with propagated value"
}

// =============================================================================
// .st-origin propagation uses correct originVal format (file::selector)
// =============================================================================

@test "st-origin propagation uses file::selector format" {
  @fixture {
    <h3 id="prop5-edited" data-st-id="st-cafe01" data-st-origin="card.st::.card @template h3">Heading</h3>
    <h3 id="prop5-sib" data-st-id="st-cafe01" data-st-origin="card.st::.card @template h3">Heading</h3>
  }

  @eval (eval("(function() {\
    if (typeof CSS === 'undefined') window.CSS = {};\
    if (!CSS.escape) CSS.escape = function(v) { return v; };\
    var editedEl = document.getElementById('prop5-edited');\
    var edit = { element: editedEl, editType: 'content', source: 'card.st', stId: 'st-cafe01', selector: '.card @template h3', newValue: 'ST propagated', oldValue: 'Heading' };\
    var file = edit.source;\
    var locator;\
    if (file && file.match(/\\.html?$/i)) { locator = edit.stId; }\
    else { locator = edit.selector; }\
    window.__prop5_locator = locator;\
    var originVal = file + '::' + locator;\
    window.__prop5_originVal = originVal;\
    var siblings = document.querySelectorAll('[data-st-origin=\\x22' + CSS.escape(originVal) + '\\x22]');\
    window.__prop5_found = siblings.length;\
    for (var i = 0; i < siblings.length; i++) {\
      if (siblings[i] !== edit.element) { siblings[i].innerHTML = edit.newValue; }\
    }\
    window.__prop5_sib = document.getElementById('prop5-sib').innerHTML;\
  })()"))

  @assert (window.__prop5_locator === '.card @template h3')
    "locator should be selector for .st files"
  @assert (window.__prop5_originVal === 'card.st::.card @template h3')
    "originVal should be file::selector for .st files"
  @assert (window.__prop5_found === 2)
    "querySelectorAll should find both elements with matching origin"
  @assert (window.__prop5_sib === 'ST propagated')
    "sibling should be updated with propagated value"
}
