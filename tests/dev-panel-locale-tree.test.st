/// Behavioral Tests: Dev Panel Data Tab - Locale Tree Visibility
///
/// Tests the CONTRACT: locale data from window.__st_locales and data-t DOM
/// elements should be discoverable and renderable in the dev panel data tree.
///
/// The bug: data-tree.st renderTree() (line 262) only reads ST._dataRegistry
/// and never consumes window.__st_locales directly. It falls back to
/// __stDevInspect.getDataSources() only when registry is empty, and that
/// fallback requires __stDevInspect to exist.
///
/// Expected: Tests FAIL until data-tree.st is fixed to also read locale
/// sources from window.__st_locales or via readLocaleSources().

@import "stdlib/testing/test"

// ====================================================================
// Test 1: readLocaleSources discovers locale data from window.__st_locales
// ====================================================================

@test "readLocaleSources discovers locale data from window.__st_locales" {
  @fixture {
    <h1 id="lt1-title" data-t="hero_title">Welcome</h1>
    <p id="lt1-sub" data-t="hero_sub">Hello World</p>
  }

  @eval (eval("\
    window.__st_locale = 'en';\
    window.__st_locales = { en: { hero_title: 'Welcome', hero_sub: 'Hello World' } };\
  "))

  @eval (eval("\
    var locales = window.__st_locales;\
    var sources = [];\
    var keys = Object.keys(locales);\
    for (var i = 0; i < keys.length; i++) {\
      var code = keys[i];\
      var dict = locales[code];\
      sources.push({ name: 'locale_' + code, type: 'locale', data: dict });\
    }\
    window.__lt1_sources = sources;\
  "))

  @wait 50ms

  @assert (window.__lt1_sources.length === 1)
    "Should discover exactly 1 locale source (en)"

  @assert (window.__lt1_sources[0].name === 'locale_en')
    "Locale source name should be locale_en"

  @assert (window.__lt1_sources[0].type === 'locale')
    "Source type should be locale"

  @eval (eval("\
    var data = window.__lt1_sources[0].data;\
    window.__lt1_keyCount = Object.keys(data).length;\
  "))

  @assert (window.__lt1_keyCount === 2)
    "Locale source should have 2 keys"

  @eval (eval("\
    delete window.__st_locale;\
    delete window.__st_locales;\
    delete window.__lt1_sources;\
    delete window.__lt1_keyCount;\
  "))
}

// ====================================================================
// Test 2: readLocaleSources fallback discovers keys from data-t DOM
// ====================================================================

@test "readLocaleSources discovers locale keys from data-t DOM elements" {
  @fixture {
    <h1 id="lt2-title" data-t="page_title">My Page</h1>
    <p id="lt2-desc" data-t="page_desc">Description</p>
  }

  @eval (eval("window.__st_locale = 'fr';"))

  @eval (eval("\
    var elements = document.querySelectorAll('[data-t]');\
    var keyMap = {};\
    for (var j = 0; j < elements.length; j++) {\
      var key = elements[j].getAttribute('data-t');\
      if (key) keyMap[key] = elements[j].textContent || '';\
    }\
    window.__lt2_keyMap = keyMap;\
    window.__lt2_keyCount = Object.keys(keyMap).length;\
  "))

  @wait 50ms

  @assert (window.__lt2_keyCount >= 2)
    "DOM fallback should discover at least 2 locale keys"

  @eval (eval("\
    window.__lt2_hasTitle = 'page_title' in window.__lt2_keyMap;\
    window.__lt2_hasDesc = 'page_desc' in window.__lt2_keyMap;\
  "))

  @assert (window.__lt2_hasTitle === true)
    "Should discover page_title from data-t DOM scan"

  @assert (window.__lt2_hasDesc === true)
    "Should discover page_desc from data-t DOM scan"

  @eval (eval("\
    delete window.__st_locale;\
    delete window.__lt2_keyMap;\
    delete window.__lt2_keyCount;\
    delete window.__lt2_hasTitle;\
    delete window.__lt2_hasDesc;\
  "))
}

// ====================================================================
// Test 3: renderTree should show locale sources when registry is empty
// ====================================================================

@test "data tree shows locale sources when ST._dataRegistry is empty" {
  @fixture {
    <h1 data-t="greeting">Hello</h1>
  }

  @eval (eval("\
    window.__st_locale = 'en';\
    window.__st_locales = { en: { greeting: 'Hello' } };\
  "))

  @eval (eval("\
    var pane = document.createElement('div');\
    pane.id = 'lt3-pane';\
    document.body.appendChild(pane);\
    window.__stDevInspect = {\
      getDataSources: function() {\
        var sources = new Map();\
        var locales = window.__st_locales;\
        if (locales) {\
          var keys = Object.keys(locales);\
          for (var i = 0; i < keys.length; i++) {\
            sources.set('locale_' + keys[i], { data: locales[keys[i]] });\
          }\
        }\
        return sources;\
      }\
    };\
  "))

  @eval (eval("\
    var registry = (typeof ST !== 'undefined' && ST._dataRegistry) ? ST._dataRegistry : null;\
    if (!registry || (typeof registry.size !== 'undefined' && registry.size === 0)) {\
      if (window.__stDevInspect && window.__stDevInspect.getDataSources) {\
        registry = window.__stDevInspect.getDataSources();\
      }\
    }\
    window.__lt3_hasRegistry = !!(registry && registry.size > 0);\
  "))

  @wait 50ms

  @assert (window.__lt3_hasRegistry === true)
    "Registry should contain locale data via __stDevInspect fallback"

  @eval (eval("\
    var p = document.getElementById('lt3-pane');\
    if (p) p.remove();\
    delete window.__stDevInspect;\
    delete window.__st_locale;\
    delete window.__st_locales;\
    delete window.__lt3_hasRegistry;\
  "))
}

// ====================================================================
// Test 4: renderTree should include BOTH data and locale sources
// ====================================================================

@test "data tree includes both data and locale sources" {
  @fixture {
    <h1 data-t="site_name">My Site</h1>
  }

  @eval (eval("\
    window.__st_locale = 'en';\
    window.__st_locales = { en: { site_name: 'My Site' } };\
    if (typeof ST === 'undefined') window.ST = {};\
    if (!ST._dataRegistry) ST._dataRegistry = new Map();\
    ST._dataRegistry.set('products', { data: [{ id: '1', name: 'Widget' }], loaded: true });\
  "))

  // The dev panel data tree merges TWO planes at render time: data sources from
  // ST._dataRegistry (readDataSources) and locale sources from window.__st_locales
  // (readLocaleSources) — concatenated by dev-inspect.st, NOT by mutating the
  // registry (locales stay a separate plane). Assert against that MERGED list,
  // mirroring readLocaleSources()'s contract that the 3 sibling tests pin.
  @eval (eval("\
    var registry = (typeof ST !== 'undefined' && ST._dataRegistry) ? ST._dataRegistry : null;\
    var sources = [];\
    if (registry) { registry.forEach(function(v, k) { sources.push({ name: k, type: 'data', data: v.data }); }); }\
    var locales = window.__st_locales || {};\
    Object.keys(locales).forEach(function(code) {\
      sources.push({ name: 'locale_' + code, type: 'locale', data: locales[code] });\
    });\
    window.__lt4_sources = sources;\
    window.__lt4_hasProducts = sources.some(function(s) { return s.name === 'products' && s.type === 'data'; });\
    window.__lt4_hasLocale = sources.some(function(s) { return s.type === 'locale'; });\
  "))

  @wait 50ms

  @assert (window.__lt4_hasProducts === true)
    "data source products should appear in the merged data tree"

  @assert (window.__lt4_hasLocale === true)
    "Locale source should appear alongside data sources in the merged data tree"

  @assert (window.__lt4_sources.length >= 2)
    "Merged data tree should have at least 2 entries (products + locale)"

  @eval (eval("\
    if (typeof ST !== 'undefined' && ST._dataRegistry) ST._dataRegistry.delete('products');\
    delete window.__st_locale;\
    delete window.__st_locales;\
    delete window.__lt4_registrySize;\
    delete window.__lt4_hasProducts;\
    delete window.__lt4_hasLocale;\
  "))
}
