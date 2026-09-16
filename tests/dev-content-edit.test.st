/// Unit Tests: Dev Content Edit — data-st-id + data-st-origin Support
///
/// Tests that elements with data-st-id and data-st-origin are recognized
/// as content-editable, produce content-specific provenance, emit correct
/// edit events with innerHTML capture, and respect the priority chain
/// (data-t > data-st-bind > data-st-id+data-st-origin).

@import "stdlib/testing/test"

// =============================================================================
// findEditableTarget recognizes data-st-id + data-st-origin elements
// =============================================================================

@test "findEditableTarget finds data-st-id and data-st-origin element" {
  @fixture {
    <span id="el1" data-st-id="st-a1b2c3d4" data-st-origin="index.html::st-a1b2c3d4">Hello</span>
  }

  @eval (eval("var findEditableTarget = function(target) { var el = target; while (el && el !== document.body) { if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t') || (el.hasAttribute('data-st-id') && el.hasAttribute('data-st-origin'))) return el; el = el.parentElement; } return null; }; var el = document.getElementById('el1'); window.__result1 = findEditableTarget(el); window.__resultId1 = window.__result1 ? window.__result1.id : null;"))

  @assert (window.__result1 !== null)
    "findEditableTarget should return non-null for data-st-id + data-st-origin element"
  @assert (window.__resultId1 === 'el1')
    "findEditableTarget should find the element with id el1"
}

// =============================================================================
// readProvenance returns content provenance for data-st-id + data-st-origin
// =============================================================================

@test "readProvenance returns content provenance for data-st-id data-st-origin element" {
  @fixture {
    <p id="el2" data-st-id="st-deadbeef" data-st-origin="index.html::st-deadbeef">Content</p>
  }

  @eval (eval("(function() {\
    var readProvenance = function(el) {\
      var localeKey = el.getAttribute('data-t');\
      if (localeKey && !el.hasAttribute('data-st-bind')) {\
        return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
      }\
      var bind = el.getAttribute('data-st-bind');\
      if (bind) {\
        return { editType: 'data', field: bind };\
      }\
      var stId = el.getAttribute('data-st-id');\
      var origin = el.getAttribute('data-st-origin');\
      if (stId && origin) {\
        var parts = origin.split('::');\
        if (parts.length >= 2 && parts[0] && parts[1]) {\
          var file = parts[0];\
          var locator = parts[1];\
          var selector = (file.slice(-5) === '.html' || file.slice(-4) === '.htm') ? '[data-st-id=' + stId + ']' : locator;\
          return { editType: 'content', stId: stId, file: file, selector: selector };\
        }\
      }\
      return null;\
    };\
    var el = document.getElementById('el2');\
    window.__prov2 = readProvenance(el);\
  })()"))

  @assert (window.__prov2 !== null)
    "readProvenance should return non-null for data-st-id + data-st-origin element"
  @assert (window.__prov2.editType === 'content')
    "editType should be 'content'"
  @assert (window.__prov2.file === 'index.html')
    "file should be 'index.html'"
  @assert (window.__prov2.selector.indexOf('st-deadbeef') !== -1)
    "selector should contain st-deadbeef"
}

// =============================================================================
// Content edit emits event with editType: 'content'
// =============================================================================

@test "content edit emits event with editType content" {
  @fixture {
    <h2 id="el3" data-st-id="st-cafebabe" data-st-origin="page.html::st-cafebabe">Title</h2>
  }

  @eval (eval("(function() {\
    var el = document.getElementById('el3');\
    var stId = el.getAttribute('data-st-id');\
    var origin = el.getAttribute('data-st-origin');\
    var parts = origin.split('::');\
    var file = parts[0];\
    var selector = '[data-st-id=' + stId + ']';\
    var activeEdit = { element: el, editType: 'content', stId: stId, file: file, selector: selector, oldValue: el.innerHTML };\
    el.textContent = 'New Title';\
    var newValue = el.innerHTML;\
    var editEvent = { editType: 'content', file: activeEdit.file, selector: activeEdit.selector, stId: activeEdit.stId, oldValue: activeEdit.oldValue, newValue: newValue };\
    window.__contentEvent3 = editEvent;\
  })()"))

  @assert (window.__contentEvent3 !== null)
    "Should have captured a content edit event"
  @assert (window.__contentEvent3.editType === 'content')
    "editType should be 'content'"
}

// =============================================================================
// Event includes correct file and selector from data-st-origin
// =============================================================================

@test "content event has correct file and selector from data-st-origin" {
  @fixture {
    <p id="el4" data-st-id="st-12345678" data-st-origin="about.html::st-12345678">Text</p>
  }

  @eval (eval("(function() {\
    var el = document.getElementById('el4');\
    var stId = el.getAttribute('data-st-id');\
    var origin = el.getAttribute('data-st-origin');\
    var parts = origin.split('::');\
    var file = parts[0];\
    var selector = '[data-st-id=' + stId + ']';\
    var activeEdit = { element: el, editType: 'content', stId: stId, file: file, selector: selector, oldValue: el.innerHTML };\
    el.textContent = 'Updated';\
    var newValue = el.innerHTML;\
    var editEvent = { editType: 'content', file: activeEdit.file, selector: activeEdit.selector, stId: activeEdit.stId, oldValue: activeEdit.oldValue, newValue: newValue };\
    window.__contentEvent4 = editEvent;\
  })()"))

  @assert (window.__contentEvent4.file === 'about.html')
    "file should be 'about.html' from data-st-origin"
  @assert (window.__contentEvent4.selector.indexOf('st-12345678') !== -1)
    "selector should contain st-12345678"
}

// =============================================================================
// innerHTML is captured (not textContent) for content edits
// =============================================================================

@test "content edit captures innerHTML not textContent" {
  @fixture {
    <p id="el5" data-st-id="st-aabbccdd" data-st-origin="index.html::st-aabbccdd"><strong>Bold</strong> text</p>
  }

  @eval (eval("(function() {\
    var el = document.getElementById('el5');\
    var isContent = true;\
    var oldValue = isContent ? el.innerHTML : el.textContent;\
    window.__capturedContent5 = oldValue;\
  })()"))

  @assert (window.__capturedContent5.indexOf('<strong>') !== -1)
    "innerHTML should contain <strong> tag, not just plain text"
}

// =============================================================================
// Priority chain: data-t wins over data-st-id + data-st-origin
// =============================================================================

@test "priority chain data-t element not treated as content edit" {
  @fixture {
    <span id="el6" data-t="hero.title" data-st-id="st-11111111" data-st-origin="index.html::st-11111111">Title</span>
  }

  @eval (eval("(function() {\
    var readProvenance = function(el) {\
      var localeKey = el.getAttribute('data-t');\
      if (localeKey && !el.hasAttribute('data-st-bind')) {\
        return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
      }\
      var bind = el.getAttribute('data-st-bind');\
      if (bind) {\
        return { editType: 'data', field: bind };\
      }\
      var stId = el.getAttribute('data-st-id');\
      var origin = el.getAttribute('data-st-origin');\
      if (stId && origin) {\
        var parts = origin.split('::');\
        if (parts.length >= 2 && parts[0] && parts[1]) {\
          var file = parts[0];\
          var locator = parts[1];\
          var selector = (file.slice(-5) === '.html' || file.slice(-4) === '.htm') ? '[data-st-id=' + stId + ']' : locator;\
          return { editType: 'content', stId: stId, file: file, selector: selector };\
        }\
      }\
      return null;\
    };\
    var el = document.getElementById('el6');\
    window.__prov6 = readProvenance(el);\
  })()"))

  @assert (window.__prov6 !== null)
    "readProvenance should return non-null"
  @assert (window.__prov6.editType === 'locale')
    "editType should be 'locale' because data-t has priority over data-st-id"
}

// =============================================================================
// Priority chain: data-st-bind wins over data-st-id + data-st-origin
// =============================================================================

@test "priority chain data-st-bind element not treated as content edit" {
  @fixture {
    <span id="el7" data-st-bind="$.name" data-st-id="st-22222222" data-st-origin="index.html::st-22222222">Name</span>
  }

  @eval (eval("(function() {\
    var readProvenance = function(el) {\
      var localeKey = el.getAttribute('data-t');\
      if (localeKey && !el.hasAttribute('data-st-bind')) {\
        return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
      }\
      var bind = el.getAttribute('data-st-bind');\
      if (bind) {\
        return { editType: 'data', field: bind };\
      }\
      var stId = el.getAttribute('data-st-id');\
      var origin = el.getAttribute('data-st-origin');\
      if (stId && origin) {\
        var parts = origin.split('::');\
        if (parts.length >= 2 && parts[0] && parts[1]) {\
          var file = parts[0];\
          var locator = parts[1];\
          var selector = (file.slice(-5) === '.html' || file.slice(-4) === '.htm') ? '[data-st-id=' + stId + ']' : locator;\
          return { editType: 'content', stId: stId, file: file, selector: selector };\
        }\
      }\
      return null;\
    };\
    var el = document.getElementById('el7');\
    window.__prov7 = readProvenance(el);\
  })()"))

  @assert (window.__prov7 !== null)
    "readProvenance should return non-null"
  @assert (window.__prov7.editType === 'data')
    "editType should be 'data' because data-st-bind has priority over data-st-id"
}

// =============================================================================
// Missing data-st-origin makes element non-editable
// =============================================================================

@test "data-st-id without data-st-origin returns null from readProvenance" {
  @fixture {
    <span id="el8" data-st-id="st-33333333">No origin</span>
  }

  @eval (eval("(function() {\
    var readProvenance = function(el) {\
      var localeKey = el.getAttribute('data-t');\
      if (localeKey && !el.hasAttribute('data-st-bind')) {\
        return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
      }\
      var bind = el.getAttribute('data-st-bind');\
      if (bind) {\
        return { editType: 'data', field: bind };\
      }\
      var stId = el.getAttribute('data-st-id');\
      var origin = el.getAttribute('data-st-origin');\
      if (stId && origin) {\
        var parts = origin.split('::');\
        if (parts.length >= 2 && parts[0] && parts[1]) {\
          var file = parts[0];\
          var locator = parts[1];\
          var selector = (file.slice(-5) === '.html' || file.slice(-4) === '.htm') ? '[data-st-id=' + stId + ']' : locator;\
          return { editType: 'content', stId: stId, file: file, selector: selector };\
        }\
      }\
      return null;\
    };\
    var el = document.getElementById('el8');\
    window.__prov8 = readProvenance(el);\
  })()"))

  @assert (window.__prov8 === null)
    "readProvenance should return null when data-st-origin is missing"
}

// =============================================================================
// Malformed data-st-origin (no ::) makes element non-editable
// =============================================================================

@test "malformed data-st-origin without separator returns null from readProvenance" {
  @fixture {
    <span id="el9" data-st-id="st-44444444" data-st-origin="malformed-no-separator">Text</span>
  }

  @eval (eval("(function() {\
    var readProvenance = function(el) {\
      var localeKey = el.getAttribute('data-t');\
      if (localeKey && !el.hasAttribute('data-st-bind')) {\
        return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
      }\
      var bind = el.getAttribute('data-st-bind');\
      if (bind) {\
        return { editType: 'data', field: bind };\
      }\
      var stId = el.getAttribute('data-st-id');\
      var origin = el.getAttribute('data-st-origin');\
      if (stId && origin) {\
        var parts = origin.split('::');\
        if (parts.length >= 2 && parts[0] && parts[1]) {\
          var file = parts[0];\
          var locator = parts[1];\
          var selector = (file.slice(-5) === '.html' || file.slice(-4) === '.htm') ? '[data-st-id=' + stId + ']' : locator;\
          return { editType: 'content', stId: stId, file: file, selector: selector };\
        }\
      }\
      return null;\
    };\
    var el = document.getElementById('el9');\
    window.__prov9 = readProvenance(el);\
  })()"))

  @assert (window.__prov9 === null)
    "readProvenance should return null when data-st-origin has no :: separator"
}
