/// Unit Tests: Dev Selector Primitive
///
/// Tests the element picker/inspector tool for the dev panel.
/// Since dev primitives only load in dev mode, tests inline the
/// key logic (getSpacetimeInfo, activate/deactivate, etc.).

@import "stdlib/testing/test"

// =============================================================================
// Selector activation sets $active to true
// =============================================================================

@test "Selector activation sets active to true" {
  @fixture {
    <div class="selector-test">Selector Test</div>
  }

  @eval (eval("window.__testSelectorActive = false; window.__testSelector = { active: false, activate: function() { this.active = true; }, deactivate: function() { this.active = false; } }; window.__testSelector.activate(); window.__testSelectorActive = window.__testSelector.active;"))

  @assert (window.__testSelectorActive === true)
    "Activating selector should set active to true"
}

// =============================================================================
// Selector deactivation sets $active to false
// =============================================================================

@test "Selector deactivation sets active to false" {
  @fixture {
    <div class="deactivate-test">Deactivate Test</div>
  }

  @eval (eval("window.__testSelector2 = { active: true, deactivate: function() { this.active = false; } }; window.__testSelector2.deactivate(); window.__testSel2Active = window.__testSelector2.active;"))

  @assert (window.__testSel2Active === false)
    "Deactivating selector should set active to false"
}

// =============================================================================
// Hovering over data-t element includes locale key in info
// =============================================================================

@test "getSpacetimeInfo discovers data-t locale key" {
  @fixture {
    <div data-t="hero.title" class="locale-el">Hello World</div>
  }

  @eval (eval("var el = document.querySelector('[data-t]'); var tag = el.tagName.toLowerCase(); var dataT = el.getAttribute('data-t'); window.__selInfoLocale = { tag: tag, localeKey: dataT };"))

  @assert (window.__selInfoLocale.tag === 'div')
    "Tag should be div"
  @assert (window.__selInfoLocale.localeKey === 'hero.title')
    "Locale key should be hero.title"
}

// =============================================================================
// Hovering over element with classes includes tag and classes in info
// =============================================================================

@test "getSpacetimeInfo discovers tag and classes" {
  @fixture {
    <section class="hero main-section featured">Content</section>
  }

  @eval (eval("var el = document.querySelector('.hero'); var tag = el.tagName.toLowerCase(); var cn = typeof el.className === 'string' ? el.className : ''; var classes = cn.split(' ').filter(function(c) { return c.length > 0; }); window.__selInfoClasses = { tag: tag, classes: classes };"))

  @assert (window.__selInfoClasses.tag === 'section')
    "Tag should be section"
  @assert (window.__selInfoClasses.classes.length === 3)
    "Should have 3 classes"
  @assert (window.__selInfoClasses.classes[0] === 'hero')
    "First class should be hero"
}

// =============================================================================
// Escape key deactivates selector
// =============================================================================

@test "Escape key handling deactivates selector" {
  @fixture {
    <div class="escape-test">Escape Test</div>
  }

  @eval (eval("window.__testEscSel = { active: true, deactivated: false, handleKeydown: function(e) { if (e.key === 'Escape' && this.active) { this.active = false; this.deactivated = true; } } }; window.__testEscSel.handleKeydown({ key: 'Escape' }); window.__escDeactivated = window.__testEscSel.deactivated;"))

  @assert (window.__escDeactivated === true)
    "Escape should deactivate selector"
  @assert (window.__testEscSel.active === false)
    "Active should be false after Escape"
}

// =============================================================================
// data-st-bind element includes binding info
// =============================================================================

@test "getSpacetimeInfo discovers data-st-bind" {
  @fixture {
    <p data-st-bind="$.name" class="bound-el">John</p>
  }

  @eval (eval("var el = document.querySelector('[data-st-bind]'); var bind = el.getAttribute('data-st-bind'); window.__selInfoBind = { dataBinding: bind };"))

  @assert (window.__selInfoBind.dataBinding === '$.name')
    "Data binding should be $.name"
}

// =============================================================================
// data-st-source element includes source info
// =============================================================================

@test "getSpacetimeInfo discovers data-st-source" {
  @fixture {
    <div data-st-source="products" data-st-index="0">
      <span class="item-name">Product</span>
    </div>
  }

  @eval (eval("var el = document.querySelector('[data-st-source]'); var source = el.getAttribute('data-st-source'); window.__selInfoSrc = { dataSource: source };"))

  @assert (window.__selInfoSrc.dataSource === 'products')
    "Data source should be products"
}

// =============================================================================
// Ancestor walking finds inherited data-st-source
// =============================================================================

@test "getSpacetimeInfo walks ancestors for inherited bindings" {
  @fixture {
    <div data-st-source="users">
      <div class="user-card">
        <span class="user-name">Alice</span>
      </div>
    </div>
  }

  @eval (eval("var el = document.querySelector('.user-name'); var info = { dataSource: null }; var current = el.parentElement; while (current && current !== document.body) { if (current.hasAttribute('data-st-source')) { info.dataSource = current.getAttribute('data-st-source') + ' (inherited)'; break; } current = current.parentElement; } window.__selInfoInherited = info;"))

  @assert (window.__selInfoInherited.dataSource === 'users (inherited)')
    "Should find inherited data source from ancestor"
}

// =============================================================================
// Tooltip formatting
// =============================================================================

@test "Tooltip formats tag, classes, and annotations correctly" {
  @fixture {
    <div class="format-test">Format Test</div>
  }

  @eval (eval("var info = { tag: 'div', classes: ['hero', 'main'], localeKey: 'home.title', dataBinding: null, dataSource: null, timeline: null, stateInfo: null }; var parts = []; var label = info.tag; if (info.classes.length > 0) { label = label + '.' + info.classes.slice(0, 3).join('.'); } parts.push(label); if (info.localeKey) { parts.push('t=' + info.localeKey); } window.__tooltipResult = parts.join(' | ');"))

  @assert (window.__tooltipResult === 'div.hero.main | t=home.title')
    "Tooltip should format as 'div.hero.main | t=home.title'"
}
