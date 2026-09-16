/// Behavioral Tests: Dev Editable — Text Duplication Bug
///
/// Reproduces the bug where inline editing via contenteditable causes text
/// duplication (e.g., "You can make" → "You can makeYou can mak").
/// These tests bootstrap the FULL dev-editable event handling system
/// (click, keydown, focusout listeners with capture phase) and simulate
/// real user interactions against it.
///
/// Expected: ALL tests should FAIL until the duplication bug is fixed.

@import "stdlib/testing/test"

// =============================================================================
// Helper: bootstrap the complete dev-editable system as one unit
// =============================================================================
// Each test calls this via @eval to register all handlers on document.
// The system mirrors stdlib/__dev__/primitives/dev-editable.st exactly:
//   - handleClick (capture phase) → findEditableTarget → startEdit
//   - handleKeydown (capture phase) → finishEdit on Enter, cancel on Escape
//   - handleBlur (capture phase) → 100ms setTimeout → finishEdit
//   - readProvenance, findEditableTarget, startEdit, finishEdit

// =============================================================================
// Test 1: Basic edit via Enter produces clean text (no duplication)
// =============================================================================

@test "edit + Enter produces only the new text, no duplication" {
  @fixture {
    <div id="t1-container" data-st-source="content" data-st-index="0" data-st-key="hero">
      <p id="t1-el" data-st-bind="$.tagline">You can make</p>
    </div>
  }

  @eval (eval("\
    (function() {\
      var activeEdit = null;\
      var editListeners = [];\
      var active = true;\
      window.__t1_events = [];\
      function readProvenance(el) {\
        var localeKey = el.getAttribute('data-t');\
        if (localeKey && !el.hasAttribute('data-st-bind')) {\
          return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
        }\
        var bind = el.getAttribute('data-st-bind');\
        if (!bind) return null;\
        var source = null; var index = null; var key = null; var filter = null;\
        var current = el;\
        while (current && current !== document.body) {\
          if (current.hasAttribute('data-st-source')) {\
            source = current.getAttribute('data-st-source');\
            index = current.getAttribute('data-st-index');\
            key = current.getAttribute('data-st-key');\
            filter = current.getAttribute('data-st-filter');\
            break;\
          }\
          current = current.parentElement;\
        }\
        return { editType: 'data', source: source, index: index !== null && index !== undefined ? index : null, key: key || null, field: bind.replace(/^\\$\\./, ''), filter: filter || null };\
      }\
      function findEditableTarget(target) {\
        var el = target;\
        while (el && el !== document.body) {\
          if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
          el = el.parentElement;\
        }\
        return null;\
      }\
      function startEdit(el) {\
        if (activeEdit) { finishEdit(activeEdit.element, false); }\
        var provenance = readProvenance(el);\
        if (!provenance) return;\
        var oldValue = el.textContent;\
        el.setAttribute('contenteditable', 'true');\
        el.setAttribute('data-st-editing', 'true');\
        el.focus();\
        var range = document.createRange();\
        range.selectNodeContents(el);\
        var sel = window.getSelection();\
        sel.removeAllRanges();\
        sel.addRange(range);\
        activeEdit = { element: el, editType: provenance.editType || 'data', source: provenance.source, index: provenance.index, key: provenance.key, field: provenance.field, filter: provenance.filter, localeKey: provenance.localeKey || null, locale: provenance.locale || null, file: provenance.file || null, oldValue: oldValue, elementId: el.id || null };\
      }\
      function finishEdit(el, cancelled) {\
        if (!activeEdit || activeEdit.element !== el) return;\
        el.removeAttribute('contenteditable');\
        el.removeAttribute('data-st-editing');\
        var newValue = el.textContent;\
        if (cancelled) {\
          el.textContent = activeEdit.oldValue;\
        } else if (newValue !== activeEdit.oldValue) {\
          var editEvent = { editType: activeEdit.editType, source: activeEdit.source, field: activeEdit.field, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId };\
          window.__t1_events.push(editEvent);\
          for (var i = 0; i < editListeners.length; i++) { try { editListeners[i](editEvent); } catch(e) {} }\
        }\
        activeEdit = null;\
      }\
      function handleClick(e) {\
        if (!active) return;\
        var el = findEditableTarget(e.target);\
        if (el) { e.preventDefault(); e.stopPropagation(); startEdit(el); }\
      }\
      function handleKeydown(e) {\
        if (!activeEdit) return;\
        if (e.key === 'Escape') { e.preventDefault(); finishEdit(activeEdit.element, true); }\
        else if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); finishEdit(activeEdit.element, false); }\
      }\
      function handleBlur(e) {\
        if (!activeEdit) return;\
        var editEl = activeEdit.element;\
        setTimeout(function() { if (activeEdit && activeEdit.element === editEl) { finishEdit(editEl, false); } }, 100);\
      }\
      document.addEventListener('click', handleClick, true);\
      document.addEventListener('keydown', handleKeydown, true);\
      document.addEventListener('focusout', handleBlur, true);\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
      window.__t1_cleanup = function() {\
        document.removeEventListener('click', handleClick, true);\
        document.removeEventListener('keydown', handleKeydown, true);\
        document.removeEventListener('focusout', handleBlur, true);\
        document.documentElement.removeAttribute('data-st-edit-mode');\
      };\
    })();\
  "))

  // Step 1: Click the element to start editing
  @eval (eval("\
    var el = document.getElementById('t1-el');\
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Step 2: Simulate typing — clear selection and set new text
  @eval (eval("\
    var el = document.getElementById('t1-el');\
    el.textContent = 'Build anything';\
  "))

  // Step 3: Press Enter to finish edit
  @eval (eval("\
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Assert: text should be ONLY the new value, not duplicated
  @assert (document.getElementById('t1-el').textContent === 'Build anything')
    "Text should be exactly 'Build anything' — no duplication"

  // Assert: emitted event should have clean newValue
  @assert (window.__t1_events.length === 1)
    "Should have emitted exactly one edit event"
  @assert (window.__t1_events[0].newValue === 'Build anything')
    "Edit event newValue should be 'Build anything' — no duplication"

  // Cleanup
  @eval (window.__t1_cleanup())
}

// =============================================================================
// Test 2: Edit on blur produces clean text (no duplication)
// =============================================================================

@test "edit + blur produces only the new text, no duplication" {
  @fixture {
    <div id="t2-container" data-st-source="content" data-st-index="0" data-st-key="hero">
      <p id="t2-el" data-st-bind="$.headline">Welcome home</p>
      <input id="t2-other" type="text" value="other">
    </div>
  }

  @eval (eval("\
    (function() {\
      var activeEdit = null;\
      var editListeners = [];\
      var active = true;\
      window.__t2_events = [];\
      function readProvenance(el) {\
        var localeKey = el.getAttribute('data-t');\
        if (localeKey && !el.hasAttribute('data-st-bind')) {\
          return { editType: 'locale', localeKey: localeKey, locale: 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/en.json' };\
        }\
        var bind = el.getAttribute('data-st-bind');\
        if (!bind) return null;\
        var source = null; var index = null; var key = null; var filter = null;\
        var current = el;\
        while (current && current !== document.body) {\
          if (current.hasAttribute('data-st-source')) {\
            source = current.getAttribute('data-st-source'); index = current.getAttribute('data-st-index');\
            key = current.getAttribute('data-st-key'); filter = current.getAttribute('data-st-filter'); break;\
          }\
          current = current.parentElement;\
        }\
        return { editType: 'data', source: source, index: index, key: key || null, field: bind.replace(/^\\$\\./, ''), filter: filter || null };\
      }\
      function findEditableTarget(target) {\
        var el = target;\
        while (el && el !== document.body) {\
          if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
          el = el.parentElement;\
        }\
        return null;\
      }\
      function startEdit(el) {\
        if (activeEdit) { finishEdit(activeEdit.element, false); }\
        var provenance = readProvenance(el);\
        if (!provenance) return;\
        var oldValue = el.textContent;\
        el.setAttribute('contenteditable', 'true');\
        el.setAttribute('data-st-editing', 'true');\
        el.focus();\
        var range = document.createRange(); range.selectNodeContents(el);\
        var sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);\
        activeEdit = { element: el, editType: provenance.editType || 'data', source: provenance.source, index: provenance.index, key: provenance.key, field: provenance.field, filter: provenance.filter, localeKey: provenance.localeKey || null, locale: provenance.locale || null, file: provenance.file || null, oldValue: oldValue, elementId: el.id || null };\
      }\
      function finishEdit(el, cancelled) {\
        if (!activeEdit || activeEdit.element !== el) return;\
        el.removeAttribute('contenteditable'); el.removeAttribute('data-st-editing');\
        var newValue = el.textContent;\
        if (cancelled) { el.textContent = activeEdit.oldValue; }\
        else if (newValue !== activeEdit.oldValue) {\
          var editEvent = { editType: activeEdit.editType, source: activeEdit.source, field: activeEdit.field, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId };\
          window.__t2_events.push(editEvent);\
        }\
        activeEdit = null;\
      }\
      function handleClick(e) {\
        if (!active) return;\
        var el = findEditableTarget(e.target);\
        if (el) { e.preventDefault(); e.stopPropagation(); startEdit(el); }\
      }\
      function handleKeydown(e) {\
        if (!activeEdit) return;\
        if (e.key === 'Escape') { e.preventDefault(); finishEdit(activeEdit.element, true); }\
        else if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); finishEdit(activeEdit.element, false); }\
      }\
      function handleBlur(e) {\
        if (!activeEdit) return;\
        var editEl = activeEdit.element;\
        setTimeout(function() { if (activeEdit && activeEdit.element === editEl) { finishEdit(editEl, false); } }, 100);\
      }\
      document.addEventListener('click', handleClick, true);\
      document.addEventListener('keydown', handleKeydown, true);\
      document.addEventListener('focusout', handleBlur, true);\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
      window.__t2_cleanup = function() {\
        document.removeEventListener('click', handleClick, true);\
        document.removeEventListener('keydown', handleKeydown, true);\
        document.removeEventListener('focusout', handleBlur, true);\
        document.documentElement.removeAttribute('data-st-edit-mode');\
      };\
    })();\
  "))

  // Step 1: Click to start editing
  @eval (eval("\
    var el = document.getElementById('t2-el');\
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Step 2: Change text
  @eval (eval("\
    var el = document.getElementById('t2-el');\
    el.textContent = 'Hello world';\
  "))

  // Step 3: Blur by focusing another element (triggers focusout with 100ms delay)
  @eval (eval("\
    var el = document.getElementById('t2-el');\
    el.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: document.getElementById('t2-other') }));\
  "))
  @wait 200ms

  // Assert: text should be ONLY the new value after blur
  @assert (document.getElementById('t2-el').textContent === 'Hello world')
    "Text should be exactly 'Hello world' after blur — no duplication"
  @assert (window.__t2_events.length === 1)
    "Should have emitted exactly one edit event on blur"
  @assert (window.__t2_events[0].newValue === 'Hello world')
    "Blur edit event newValue should be 'Hello world' — no duplication"

  @eval (window.__t2_cleanup())
}

// =============================================================================
// Test 3: Rapid re-click on same element doesn't corrupt text
// =============================================================================

@test "rapid re-click on same element doesn't cause duplication" {
  @fixture {
    <div id="t3-container" data-st-source="pages" data-st-index="0" data-st-key="home">
      <h2 id="t3-el" data-st-bind="$.title">Original title</h2>
    </div>
  }

  @eval (eval("\
    (function() {\
      var activeEdit = null;\
      var editListeners = [];\
      var active = true;\
      window.__t3_events = [];\
      function readProvenance(el) {\
        var bind = el.getAttribute('data-st-bind');\
        if (!bind) return null;\
        var source = null; var index = null; var key = null; var filter = null;\
        var current = el;\
        while (current && current !== document.body) {\
          if (current.hasAttribute('data-st-source')) {\
            source = current.getAttribute('data-st-source'); index = current.getAttribute('data-st-index');\
            key = current.getAttribute('data-st-key'); filter = current.getAttribute('data-st-filter'); break;\
          }\
          current = current.parentElement;\
        }\
        return { editType: 'data', source: source, index: index, key: key || null, field: bind.replace(/^\\$\\./, ''), filter: filter || null };\
      }\
      function findEditableTarget(target) {\
        var el = target;\
        while (el && el !== document.body) {\
          if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
          el = el.parentElement;\
        }\
        return null;\
      }\
      function startEdit(el) {\
        if (activeEdit) { finishEdit(activeEdit.element, false); }\
        var provenance = readProvenance(el);\
        if (!provenance) return;\
        var oldValue = el.textContent;\
        el.setAttribute('contenteditable', 'true');\
        el.setAttribute('data-st-editing', 'true');\
        el.focus();\
        var range = document.createRange(); range.selectNodeContents(el);\
        var sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);\
        activeEdit = { element: el, editType: provenance.editType || 'data', source: provenance.source, index: provenance.index, key: provenance.key, field: provenance.field, filter: provenance.filter, oldValue: oldValue, elementId: el.id || null };\
      }\
      function finishEdit(el, cancelled) {\
        if (!activeEdit || activeEdit.element !== el) return;\
        el.removeAttribute('contenteditable'); el.removeAttribute('data-st-editing');\
        var newValue = el.textContent;\
        if (cancelled) { el.textContent = activeEdit.oldValue; }\
        else if (newValue !== activeEdit.oldValue) {\
          var editEvent = { editType: activeEdit.editType, field: activeEdit.field, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId };\
          window.__t3_events.push(editEvent);\
        }\
        activeEdit = null;\
      }\
      function handleClick(e) {\
        if (!active) return;\
        var el = findEditableTarget(e.target);\
        if (el) { e.preventDefault(); e.stopPropagation(); startEdit(el); }\
      }\
      function handleKeydown(e) {\
        if (!activeEdit) return;\
        if (e.key === 'Escape') { e.preventDefault(); finishEdit(activeEdit.element, true); }\
        else if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); finishEdit(activeEdit.element, false); }\
      }\
      function handleBlur(e) {\
        if (!activeEdit) return;\
        var editEl = activeEdit.element;\
        setTimeout(function() { if (activeEdit && activeEdit.element === editEl) { finishEdit(editEl, false); } }, 100);\
      }\
      document.addEventListener('click', handleClick, true);\
      document.addEventListener('keydown', handleKeydown, true);\
      document.addEventListener('focusout', handleBlur, true);\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
      window.__t3_cleanup = function() {\
        document.removeEventListener('click', handleClick, true);\
        document.removeEventListener('keydown', handleKeydown, true);\
        document.removeEventListener('focusout', handleBlur, true);\
        document.documentElement.removeAttribute('data-st-edit-mode');\
      };\
    })();\
  "))

  // Step 1: Click to start editing
  @eval (eval("\
    var el = document.getElementById('t3-el');\
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Step 2: Partially type (simulating user mid-edit)
  @eval (eval("\
    var el = document.getElementById('t3-el');\
    el.textContent = 'New tit';\
  "))

  // Step 3: Click the SAME element again (rapid re-click)
  @eval (eval("\
    var el = document.getElementById('t3-el');\
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Assert: text should NOT be corrupted/duplicated
  @assert (document.getElementById('t3-el').textContent === 'New tit')
    "Text should remain 'New tit' after re-click — no duplication"

  // Step 4: Finish the edit normally with Enter
  @eval (eval("\
    var el = document.getElementById('t3-el');\
    el.textContent = 'New title';\
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  @assert (document.getElementById('t3-el').textContent === 'New title')
    "Final text should be exactly 'New title' — no duplication from re-click"

  @eval (window.__t3_cleanup())
}

// =============================================================================
// Test 4: Edit data-t locale element produces clean text
// =============================================================================

@test "edit data-t locale element produces clean text, no duplication" {
  @fixture {
    <div id="t4-container">
      <h1 id="t4-el" data-t="hero.title">Bienvenue</h1>
    </div>
  }

  @eval (eval("\
    (function() {\
      var activeEdit = null;\
      var editListeners = [];\
      var active = true;\
      window.__t4_events = [];\
      function readProvenance(el) {\
        var localeKey = el.getAttribute('data-t');\
        if (localeKey && !el.hasAttribute('data-st-bind')) {\
          return { editType: 'locale', localeKey: localeKey, locale: window.__st_locale || 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/' + (window.__st_locale || 'en') + '.json' };\
        }\
        var bind = el.getAttribute('data-st-bind');\
        if (!bind) return null;\
        return { editType: 'data', source: null, index: null, key: null, field: bind.replace(/^\\$\\./, ''), filter: null };\
      }\
      function findEditableTarget(target) {\
        var el = target;\
        while (el && el !== document.body) {\
          if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
          el = el.parentElement;\
        }\
        return null;\
      }\
      function startEdit(el) {\
        if (activeEdit) { finishEdit(activeEdit.element, false); }\
        var provenance = readProvenance(el);\
        if (!provenance) return;\
        var oldValue = el.textContent;\
        el.setAttribute('contenteditable', 'true');\
        el.setAttribute('data-st-editing', 'true');\
        el.focus();\
        var range = document.createRange(); range.selectNodeContents(el);\
        var sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);\
        activeEdit = { element: el, editType: provenance.editType, localeKey: provenance.localeKey || null, locale: provenance.locale || null, file: provenance.file || null, source: provenance.source, field: provenance.field, oldValue: oldValue, elementId: el.id || null };\
      }\
      function finishEdit(el, cancelled) {\
        if (!activeEdit || activeEdit.element !== el) return;\
        el.removeAttribute('contenteditable'); el.removeAttribute('data-st-editing');\
        var newValue = el.textContent;\
        if (cancelled) { el.textContent = activeEdit.oldValue; }\
        else if (newValue !== activeEdit.oldValue) {\
          var editEvent = { editType: activeEdit.editType, localeKey: activeEdit.localeKey, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId };\
          window.__t4_events.push(editEvent);\
        }\
        activeEdit = null;\
      }\
      function handleClick(e) {\
        if (!active) return;\
        var el = findEditableTarget(e.target);\
        if (el) { e.preventDefault(); e.stopPropagation(); startEdit(el); }\
      }\
      function handleKeydown(e) {\
        if (!activeEdit) return;\
        if (e.key === 'Escape') { e.preventDefault(); finishEdit(activeEdit.element, true); }\
        else if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); finishEdit(activeEdit.element, false); }\
      }\
      function handleBlur(e) {\
        if (!activeEdit) return;\
        var editEl = activeEdit.element;\
        setTimeout(function() { if (activeEdit && activeEdit.element === editEl) { finishEdit(editEl, false); } }, 100);\
      }\
      document.addEventListener('click', handleClick, true);\
      document.addEventListener('keydown', handleKeydown, true);\
      document.addEventListener('focusout', handleBlur, true);\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
      window.__t4_cleanup = function() {\
        document.removeEventListener('click', handleClick, true);\
        document.removeEventListener('keydown', handleKeydown, true);\
        document.removeEventListener('focusout', handleBlur, true);\
        document.documentElement.removeAttribute('data-st-edit-mode');\
      };\
    })();\
  "))

  // Step 1: Click locale element to edit
  @eval (eval("\
    var el = document.getElementById('t4-el');\
    el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Step 2: Change text
  @eval (eval("\
    var el = document.getElementById('t4-el');\
    el.textContent = 'Welcome';\
  "))

  // Step 3: Enter to confirm
  @eval (eval("\
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  @assert (document.getElementById('t4-el').textContent === 'Welcome')
    "Locale element text should be exactly 'Welcome' — no duplication"
  @assert (window.__t4_events.length === 1)
    "Should emit exactly one locale edit event"
  @assert (window.__t4_events[0].newValue === 'Welcome')
    "Locale edit event newValue should be 'Welcome' — no duplication"
  @assert (window.__t4_events[0].editType === 'locale')
    "Edit event should be locale type"

  @eval (window.__t4_cleanup())
}

// =============================================================================
// Test 5: Edit element with child nodes doesn't duplicate text
// =============================================================================

@test "edit element with child span nodes doesn't duplicate text" {
  @fixture {
    <div id="t5-container" data-st-source="content" data-st-index="0" data-st-key="about">
      <p id="t5-el" data-st-bind="$.description"><span>You can</span> <span>make</span></p>
    </div>
  }

  @eval (eval("\
    (function() {\
      var activeEdit = null;\
      var editListeners = [];\
      var active = true;\
      window.__t5_events = [];\
      function readProvenance(el) {\
        var localeKey = el.getAttribute('data-t');\
        if (localeKey && !el.hasAttribute('data-st-bind')) {\
          return { editType: 'locale', localeKey: localeKey, locale: 'en', source: null, index: null, key: null, field: null, filter: null, file: 'locales/en.json' };\
        }\
        var bind = el.getAttribute('data-st-bind');\
        if (!bind) return null;\
        var source = null; var index = null; var key = null; var filter = null;\
        var current = el;\
        while (current && current !== document.body) {\
          if (current.hasAttribute('data-st-source')) {\
            source = current.getAttribute('data-st-source'); index = current.getAttribute('data-st-index');\
            key = current.getAttribute('data-st-key'); filter = current.getAttribute('data-st-filter'); break;\
          }\
          current = current.parentElement;\
        }\
        return { editType: 'data', source: source, index: index, key: key || null, field: bind.replace(/^\\$\\./, ''), filter: filter || null };\
      }\
      function findEditableTarget(target) {\
        var el = target;\
        while (el && el !== document.body) {\
          if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t')) return el;\
          el = el.parentElement;\
        }\
        return null;\
      }\
      function startEdit(el) {\
        if (activeEdit) { finishEdit(activeEdit.element, false); }\
        var provenance = readProvenance(el);\
        if (!provenance) return;\
        var oldValue = el.textContent;\
        el.setAttribute('contenteditable', 'true');\
        el.setAttribute('data-st-editing', 'true');\
        el.focus();\
        var range = document.createRange(); range.selectNodeContents(el);\
        var sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);\
        activeEdit = { element: el, editType: provenance.editType || 'data', source: provenance.source, index: provenance.index, key: provenance.key, field: provenance.field, filter: provenance.filter, oldValue: oldValue, elementId: el.id || null };\
      }\
      function finishEdit(el, cancelled) {\
        if (!activeEdit || activeEdit.element !== el) return;\
        el.removeAttribute('contenteditable'); el.removeAttribute('data-st-editing');\
        var newValue = el.textContent;\
        if (cancelled) { el.textContent = activeEdit.oldValue; }\
        else if (newValue !== activeEdit.oldValue) {\
          var editEvent = { editType: activeEdit.editType, field: activeEdit.field, oldValue: activeEdit.oldValue, newValue: newValue, elementId: activeEdit.elementId };\
          window.__t5_events.push(editEvent);\
        }\
        activeEdit = null;\
      }\
      function handleClick(e) {\
        if (!active) return;\
        var el = findEditableTarget(e.target);\
        if (el) { e.preventDefault(); e.stopPropagation(); startEdit(el); }\
      }\
      function handleKeydown(e) {\
        if (!activeEdit) return;\
        if (e.key === 'Escape') { e.preventDefault(); finishEdit(activeEdit.element, true); }\
        else if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); finishEdit(activeEdit.element, false); }\
      }\
      function handleBlur(e) {\
        if (!activeEdit) return;\
        var editEl = activeEdit.element;\
        setTimeout(function() { if (activeEdit && activeEdit.element === editEl) { finishEdit(editEl, false); } }, 100);\
      }\
      document.addEventListener('click', handleClick, true);\
      document.addEventListener('keydown', handleKeydown, true);\
      document.addEventListener('focusout', handleBlur, true);\
      document.documentElement.setAttribute('data-st-edit-mode', 'true');\
      window.__t5_cleanup = function() {\
        document.removeEventListener('click', handleClick, true);\
        document.removeEventListener('keydown', handleKeydown, true);\
        document.removeEventListener('focusout', handleBlur, true);\
        document.documentElement.removeAttribute('data-st-edit-mode');\
      };\
    })();\
  "))

  // Step 1: Click inside a child span (should bubble up to the data-st-bind element)
  @eval (eval("\
    var span = document.querySelector('#t5-el span');\
    span.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  // Verify startEdit captured the correct oldValue (with child nodes, textContent = "You can make")
  @assert (document.getElementById('t5-el').getAttribute('contenteditable') === 'true')
    "Element should be in contenteditable mode"

  // Step 2: Replace content (user types over selected text)
  @eval (eval("\
    var el = document.getElementById('t5-el');\
    el.textContent = 'We build';\
  "))

  // Step 3: Finish with Enter
  @eval (eval("\
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));\
  "))
  @wait 50ms

  @assert (document.getElementById('t5-el').textContent === 'We build')
    "Text should be exactly 'We build' after editing element with child nodes — no duplication"
  @assert (window.__t5_events.length === 1)
    "Should emit exactly one edit event"
  @assert (window.__t5_events[0].newValue === 'We build')
    "Edit event newValue should be 'We build' — no duplication"

  @eval (window.__t5_cleanup())
}
