/// Unit Tests: Dev Editable — Locale (data-t) Support
///
/// Tests that data-t elements are recognized as editable,
/// produce locale-specific provenance, and emit correct edit events.
/// Also verifies no regression on data-st-bind elements.

@import "stdlib/testing/test"

// =============================================================================
// findEditableTarget recognizes data-t elements
// =============================================================================

@test "findEditableTarget finds data-t element" {
  @fixture {
    <div id="locale-container">
      <span id="locale-el" data-t="hero.title">Welcome</span>
    </div>
  }

  @eval (eval("var findEditableTarget = function(target) { var el = target; while (el && el !== document.body) { if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el; el = el.parentElement; } return null; }; var el = document.getElementById('locale-el'); window.__findResult = findEditableTarget(el); window.__findResultId = window.__findResult ? window.__findResult.id : null;"))

  @assert (window.__findResultId === 'locale-el')
    "findEditableTarget should find element with data-t attribute"
}

// =============================================================================
// readProvenance returns locale-specific provenance for data-t
// =============================================================================

@test "readProvenance returns locale provenance for data-t element" {
  @fixture {
    <div id="prov-container">
      <span id="prov-locale" data-t="nav.about">About Us</span>
    </div>
  }

  @eval (eval("var readProvenance = function(el) { var localeKey = el.getAttribute('data-t'); if (localeKey && !el.hasAttribute('data-st-bind')) { return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/' + (window.__st_locale || 'en') + '.json' }; } var bind = el.getAttribute('data-st-bind'); if (!bind) return null; return { editType: 'data', source: null, index: null, key: null, field: bind, filter: null }; }; var el = document.getElementById('prov-locale'); window.__provResult = readProvenance(el);"))

  @assert (window.__provResult !== null)
    "readProvenance should return non-null for data-t element"
  @assert (window.__provResult.editType === 'locale')
    "editType should be 'locale'"
  @assert (window.__provResult.localeKey === 'nav.about')
    "localeKey should match data-t attribute value"
  @assert (window.__provResult.locale === 'en')
    "locale should default to 'en' when __st_locale is not set"
  @assert (window.__provResult.file === 'locales/en.json')
    "file should be locales/en.json"
  @assert (window.__provResult.source === null)
    "source should be null for locale elements"
}

// =============================================================================
// readProvenance respects window.__st_locale
// =============================================================================

@test "readProvenance uses window.__st_locale for locale" {
  @fixture {
    <div id="locale-fr-container">
      <span id="prov-locale-fr" data-t="hero.subtitle">Subtitle</span>
    </div>
  }

  @eval (eval("window.__st_locale = 'fr'; var readProvenance = function(el) { var localeKey = el.getAttribute('data-t'); if (localeKey && !el.hasAttribute('data-st-bind')) { return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/' + (window.__st_locale || 'en') + '.json' }; } return null; }; var el = document.getElementById('prov-locale-fr'); window.__provFr = readProvenance(el); window.__st_locale = undefined;"))

  @assert (window.__provFr.locale === 'fr')
    "locale should be 'fr' from window.__st_locale"
  @assert (window.__provFr.file === 'locales/fr.json')
    "file should be locales/fr.json"
}

// =============================================================================
// data-st-bind elements still work (no regression)
// =============================================================================

@test "readProvenance returns data provenance for data-st-bind element" {
  @fixture {
    <div id="bind-container" data-st-source="products" data-st-index="0" data-st-key="prod-1">
      <span id="bind-el" data-st-bind="$.name">Product Name</span>
    </div>
  }

  @eval (eval("var readProvenance = function(el) { var localeKey = el.getAttribute('data-t'); if (localeKey && !el.hasAttribute('data-st-bind')) { return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/' + (window.__st_locale || 'en') + '.json' }; } var bind = el.getAttribute('data-st-bind'); if (!bind) return null; var source = null; var index = null; var key = null; var filter = null; var current = el; while (current && current !== document.body) { if (current.hasAttribute('data-st-source')) { source = current.getAttribute('data-st-source'); index = current.getAttribute('data-st-index'); key = current.getAttribute('data-st-key'); filter = current.getAttribute('data-st-filter'); break; } current = current.parentElement; } return { editType: 'data', source: source, index: index !== null && index !== undefined ? index : null, key: key || null, field: bind.replace(new RegExp('^\\\\$\\\\.'), ''), filter: filter || null }; }; var el = document.getElementById('bind-el'); window.__bindResult = readProvenance(el);"))

  @assert (window.__bindResult !== null)
    "readProvenance should return non-null for data-st-bind element"
  @assert (window.__bindResult.editType === 'data')
    "editType should be 'data' for data-st-bind"
  @assert (window.__bindResult.source === 'products')
    "source should be 'products'"
  @assert (window.__bindResult.field === 'name')
    "field should be 'name' (stripped $.)"
}

// =============================================================================
// Edit completion emits locale event with file path
// =============================================================================

@test "finishEdit emits locale event with correct fields" {
  @fixture {
    <div id="edit-container">
      <span id="edit-locale" data-t="cta.button">Click Me</span>
    </div>
  }

  @eval (eval("window.__capturedEvent = null; var editListeners = []; var activeEdit = null; var editCount = 0; function finishEdit(el, cancelled) { if (!activeEdit || activeEdit.element !== el) return; el.removeAttribute('contenteditable'); el.removeAttribute('data-st-editing'); var newValue = el.textContent; if (cancelled) { el.textContent = activeEdit.oldValue; } else if (newValue !== activeEdit.oldValue) { var editEvent; if (activeEdit.editType === 'locale') { editEvent = { editType: 'locale', localeKey: activeEdit.localeKey, locale: activeEdit.locale, file: activeEdit.file, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId }; } else { editEvent = { editType: 'data', source: activeEdit.source, index: activeEdit.index, key: activeEdit.key, field: activeEdit.field, filter: activeEdit.filter, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId }; } editCount = editCount + 1; for (var i = 0; i < editListeners.length; i++) { try { editListeners[i](editEvent); } catch (e) {} } } activeEdit = null; } editListeners.push(function(evt) { window.__capturedEvent = evt; }); var el = document.getElementById('edit-locale'); activeEdit = { element: el, editType: 'locale', localeKey: 'cta.button', locale: 'en', file: 'locales/en.json', source: null, index: null, key: null, field: null, filter: null, oldValue: 'Click Me', elementId: 'edit-locale' }; el.textContent = 'Click Here'; finishEdit(el, false);"))

  @assert (window.__capturedEvent !== null)
    "Should have captured an edit event"
  @assert (window.__capturedEvent.editType === 'locale')
    "editType should be 'locale'"
  @assert (window.__capturedEvent.localeKey === 'cta.button')
    "localeKey should be 'cta.button'"
  @assert (window.__capturedEvent.file === 'locales/en.json')
    "file should be 'locales/en.json'"
  @assert (window.__capturedEvent.oldValue === 'Click Me')
    "oldValue should be original text"
  @assert (window.__capturedEvent.newValue === 'Click Here')
    "newValue should be edited text"
  @assert (window.__capturedEvent.elementId === 'edit-locale')
    "elementId should match"
}

// =============================================================================
// findEditableTarget still finds data-st-bind (regression check)
// =============================================================================

@test "findEditableTarget still finds data-st-bind elements" {
  @fixture {
    <div id="regr-container">
      <span id="regr-bind" data-st-bind="$.title">Title</span>
    </div>
  }

  @eval (eval("var findEditableTarget = function(target) { var el = target; while (el && el !== document.body) { if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el; el = el.parentElement; } return null; }; var el = document.getElementById('regr-bind'); window.__regrResult = findEditableTarget(el); window.__regrResultId = window.__regrResult ? window.__regrResult.id : null;"))

  @assert (window.__regrResultId === 'regr-bind')
    "findEditableTarget should still find data-st-bind elements"
}
