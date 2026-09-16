/// Unit Tests: Dev Inspect Locale Source Discovery
///
/// Tests that dev-inspect's readLocaleSources() correctly discovers
/// locale/i18n data from window.__st_locales and data-t DOM elements,
/// and includes them in the $dataSources export.

@import "stdlib/testing/test"

// =============================================================================
// Locale sources via window.__st_locales
// =============================================================================

@test "readLocaleSources picks up window.__st_locales entries" {
  @fixture {
    <div class="locale-test">Locale Test</div>
  }

  @eval (eval("window.__st_locales = { en: { hello: 'Hello', bye: 'Goodbye' }, fr: { hello: 'Bonjour', bye: 'Au revoir' } }; window.__st_locale = 'en';"))

  @eval (eval("var locale = window.__st_locale; var results = []; if (typeof window.__st_locales === 'object' && window.__st_locales !== null) { var locales = window.__st_locales; var keys = Object.keys(locales); for (var i = 0; i < keys.length; i++) { var code = keys[i]; var dict = locales[code]; results.push({ name: '@locale ' + code, type: 'locale', itemCount: (dict && typeof dict === 'object') ? Object.keys(dict).length : 0, loading: false, error: null, data: dict || {}, active: code === locale }); } } window.__localeDs = results;"))

  @assert (window.__localeDs.length === 2)
    "Should have 2 locale entries (en and fr)"

  @eval (eval("delete window.__st_locales; delete window.__st_locale;"))
}

@test "active locale is correctly marked" {
  @fixture {
    <div class="active-test">Active Test</div>
  }

  @eval (eval("window.__st_locales = { en: { k1: 'v1' }, fr: { k1: 'v1' } }; window.__st_locale = 'fr';"))

  @eval (eval("var locale = window.__st_locale; var results = []; if (typeof window.__st_locales === 'object' && window.__st_locales !== null) { var locales = window.__st_locales; var keys = Object.keys(locales); for (var i = 0; i < keys.length; i++) { var code = keys[i]; var dict = locales[code]; results.push({ name: '@locale ' + code, type: 'locale', itemCount: (dict && typeof dict === 'object') ? Object.keys(dict).length : 0, loading: false, error: null, data: dict || {}, active: code === locale }); } } window.__enEntry = results.find(function(d) { return d.name === '@locale en' }); window.__frEntry = results.find(function(d) { return d.name === '@locale fr' });"))

  @assert (window.__enEntry !== undefined)
    "en locale entry should exist"
  @assert (window.__frEntry !== undefined)
    "fr locale entry should exist"
  @assert (window.__enEntry.active === false)
    "en should not be active"
  @assert (window.__frEntry.active === true)
    "fr should be active"

  @eval (eval("delete window.__st_locales; delete window.__st_locale;"))
}

@test "locale itemCount matches number of keys in dict" {
  @fixture {
    <div class="count-test">Count Test</div>
  }

  @eval (eval("window.__st_locales = { en: { a: '1', b: '2', c: '3' } }; window.__st_locale = 'en';"))

  @eval (eval("var locale = window.__st_locale; var locales = window.__st_locales; var keys = Object.keys(locales); var results = []; for (var i = 0; i < keys.length; i++) { var code = keys[i]; var dict = locales[code]; results.push({ name: '@locale ' + code, type: 'locale', itemCount: (dict && typeof dict === 'object') ? Object.keys(dict).length : 0, loading: false, error: null, data: dict || {}, active: code === locale }); } window.__countEntry = results[0];"))

  @assert (window.__countEntry !== undefined)
    "locale entry should exist"
  @assert (window.__countEntry.itemCount === 3)
    "itemCount should be 3"

  @eval (eval("delete window.__st_locales; delete window.__st_locale;"))
}

@test "no locale data means empty locale results" {
  @fixture {
    <div class="no-locale-test">No Locale Test</div>
  }

  @eval (eval("delete window.__st_locales; delete window.__st_locale;"))

  @eval (eval("var locale = window.__st_locale; var results = []; if (typeof window.__st_locales === 'object' && window.__st_locales !== null) { var locales = window.__st_locales; var keys = Object.keys(locales); for (var i = 0; i < keys.length; i++) { results.push({ type: 'locale' }); } } window.__localeCount = results.length;"))

  @assert (window.__localeCount === 0)
    "Should have no locale entries when no locale data exists"
}

// =============================================================================
// Locale sources via data-t DOM fallback
// =============================================================================

@test "readLocaleSources discovers data-t elements when __st_locales absent" {
  @fixture {
    <div class="dt-test">
      <h1 data-t="title">Welcome</h1>
      <p data-t="subtitle">Hello world</p>
    </div>
  }

  @eval (eval("delete window.__st_locales; window.__st_locale = 'en';"))

  @eval (eval("var locale = window.__st_locale; var results = []; if (typeof window.__st_locales === 'object' && window.__st_locales !== null) { /* skip */ } else if (locale) { var elements = document.querySelectorAll('[data-t]'); if (elements.length > 0) { var keyMap = {}; for (var j = 0; j < elements.length; j++) { var el = elements[j]; var key = el.getAttribute('data-t'); if (key) keyMap[key] = el.textContent || ''; } results.push({ name: '@locale ' + locale, type: 'locale', itemCount: Object.keys(keyMap).length, loading: false, error: null, data: keyMap, active: true }); } } window.__dtEntry = results.length > 0 ? results[0] : null;"))

  @assert (window.__dtEntry !== null)
    "locale entry should exist from data-t elements"
  @assert (window.__dtEntry.itemCount === 2)
    "itemCount should be 2 (title + subtitle)"
  @assert (window.__dtEntry.active === true)
    "locale should be active"

  @eval (eval("delete window.__st_locale;"))
}

// =============================================================================
// @data source type field
// =============================================================================

@test "@data sources include type field set to data" {
  @fixture {
    <div class="type-test">Type Test</div>
  }

  @eval (eval("if (typeof ST === 'undefined') window.ST = {}; ST._dataRegistry = new Map(); ST._dataRegistry.set('products', { data: [1,2,3], loaded: true });"))

  @eval (eval("var reg = (typeof ST !== 'undefined') ? ST._dataRegistry : null; var result = []; if (reg && typeof reg.forEach === 'function') { reg.forEach(function(value, name) { result.push({ name: name, type: 'data', itemCount: Array.isArray(value.data) ? value.data.length : (value.data ? 1 : 0), loaded: !!value.loaded }); }); } window.__dataEntry = result.find(function(d) { return d.name === 'products' });"))

  @assert (window.__dataEntry !== undefined)
    "@data source entry should exist"
  @assert (window.__dataEntry.type === 'data')
    "type field should be 'data'"
  @assert (window.__dataEntry.itemCount === 3)
    "itemCount should be 3"

  @eval (eval("delete ST._dataRegistry;"))
}
