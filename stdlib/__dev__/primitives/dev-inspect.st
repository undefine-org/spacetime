/// Dev Inspect Primitive
///
/// Reads runtime debug state (timelines, errors, data sources, locale sources)
/// from window.__ST_DEBUG__, ST._dataRegistry, and window.__st_locale,
/// polling every 500ms.
/// Exposes reactive exports for dev panel tabs to consume.
///
/// Handles graceful degradation when debug runtime or data registry
/// haven't loaded yet — keeps polling until they appear.
///
/// @yields $timelines - Array of timeline objects
/// @yields $errors - Array of error objects
/// @yields $timelineCount - Number of active timelines
/// @yields $errorCount - Number of errors
/// @yields $dataSources - Array of data source info objects

%primitive dev-inspect() {
  %emit js {
    (function() {
      var timelines = [];
      var errors = [];
      var timelineCount = 0;
      var errorCount = 0;
      var dataSources = [];

      function readTimelines() {
        var dbg = window.__ST_DEBUG__;
        if (!dbg || typeof dbg.getTimelines !== 'function') return [];
        try {
          var map = dbg.getTimelines();
          if (map && typeof map.values === 'function') {
            return Array.from(map.values());
          }
        } catch (e) {}
        return [];
      }

      function readErrors() {
        var dbg = window.__ST_DEBUG__;
        if (!dbg) return [];
        if (typeof dbg.getErrors === 'function') {
          try { return dbg.getErrors() || []; } catch (e) {}
        }
        if (Array.isArray(dbg.errors)) {
          return dbg.errors;
        }
        return [];
      }

      function readDataSources() {
        var reg = (typeof ST !== 'undefined') ? ST._dataRegistry : null;
        if (!reg || typeof reg.forEach !== 'function') return [];
        var result = [];
        try {
          reg.forEach(function(value, name) {
            result.push({
              name: name,
              type: 'data',
              itemCount: Array.isArray(value.data) ? value.data.length : (value.data ? 1 : 0),
              loaded: !!value.loaded
            });
          });
        } catch (e) {}
        return result;
      }

      function readLocaleSources() {
        var locale = window.__st_locale;
        var results = [];

        // Check window.__st_locales first (future-proofing)
        if (typeof window.__st_locales === 'object' && window.__st_locales !== null) {
          var locales = window.__st_locales;
          var keys = Object.keys(locales);
          for (var i = 0; i < keys.length; i++) {
            var code = keys[i];
            var dict = locales[code];
            results.push({
              name: '@locale "' + code + '"',
              type: 'locale',
              itemCount: (dict && typeof dict === 'object') ? Object.keys(dict).length : 0,
              loading: false,
              error: null,
              data: dict || {},
              active: code === locale
            });
          }
          return results;
        }

        // Fallback: discover from data-t elements in the DOM
        if (!locale) return [];
        var elements = document.querySelectorAll('[data-t]');
        if (elements.length === 0) return [];

        var keyMap = {};
        for (var j = 0; j < elements.length; j++) {
          var el = elements[j];
          var key = el.getAttribute('data-t');
          if (key) keyMap[key] = el.textContent || '';
        }

        results.push({
          name: '@locale "' + locale + '"',
          type: 'locale',
          itemCount: Object.keys(keyMap).length,
          loading: false,
          error: null,
          data: keyMap,
          active: true
        });

        return results;
      }

      function refresh() {
        timelines = readTimelines();
        errors = readErrors();
        dataSources = readDataSources().concat(readLocaleSources());
        timelineCount = timelines.length;
        errorCount = errors.length;

        %yield timelines -> $timelines;
        %yield errors -> $errors;
        %yield timelineCount -> $timelineCount;
        %yield errorCount -> $errorCount;
        %yield dataSources -> $dataSources;
      }

      // Poll every 500ms
      var pollInterval = setInterval(refresh, 500);

      // Initial read
      refresh();

      // Global API for other dev primitives
      window.__stDevInspect = {
        refresh: refresh,
        getTimelines: function() { return timelines; },
        getErrors: function() { return errors; },
        getDataSources: function() { return dataSources; },
        clearErrors: function() {
          var dbg = window.__ST_DEBUG__;
          if (dbg && typeof dbg.clearErrors === 'function') {
            dbg.clearErrors();
            refresh();
          }
        }
      };

      %cleanup {
        clearInterval(pollInterval);
        window.__stDevInspect = null;
      }
    })();
  }

  %exports {
    $timelines: array
    $errors: array
    $timelineCount: number
    $errorCount: number
    $dataSources: array
  }
}
