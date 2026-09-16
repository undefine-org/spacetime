/// Data Source Primitive
///
/// Fetches data from URLs, localStorage, or inline definitions.
/// Dispatches events for loading states: loaded, empty, error.
///
/// PLAN-133 W3: the lifecycle (store registration, registry src pinning,
/// {locale} resolution, fetch/localStorage/inline branches, dual-channel
/// publish, error reporting, refresh) lives ONCE in the prelude
/// (`__stDataSource`); each usage emits a single config call. The per-usage
/// code declares no locals — the BUG-274 habitat (a branch-scoped const read
/// from the catch) cannot re-form.
///
/// @param name - Unique identifier for this data source
/// @param src - URL to fetch from, or "localStorage:key", or "inline"
/// @param default - Default value if source is empty
/// @param refresh - Auto-refresh interval in ms (0 = no refresh)
///
/// @yields $data - The loaded data
/// @yields $loading - Whether data is currently loading
/// @yields $error - Error message if load failed
/// @yields $reload - Function to manually reload data

%primitive data-source(
  name: ident,
  src: string,
  default: any?,
  refresh: number = 0
) %uses data-url, data-store, data-channel {
  %emit prelude-js {
    // The whole data-source lifecycle, once per page. Semantics moved here
    // VERBATIM from the per-usage template (W3) — including the load-bearing
    // ordering notes (FUP-047 sync src pinning; the setData → SpacetimeLocal
    // publish ordering that @each depends on).
    window.__stDataSource = window.__stDataSource || function (cfg) {
      const sourceName = cfg.name;
      const srcValue = cfg.src;
      const defaultValue = cfg.default;
      const refreshInterval = cfg.refresh;

      // Register in global data store
      const store = window.__spacetimeData[sourceName] = {
        data: null,
        loading: true,
        error: null
      };

      // Record the source's backing FILE in the data registry SYNCHRONOUSLY at init
      // (before the async fetch resolves) so on-page editing can persist to disk from
      // the very first edit (FUP-047): dev-save's resolveFilePath reads
      // _dataRegistry.get(name).src. Only a real fetch URL is a disk file — 'inline'
      // and 'localStorage:' sources have none (left unset → not EditJson-editable).
      // Done at init, not post-fetch, to avoid a race where a surface persists before
      // the fetch lands. setData() later mutates the same entry in place, so .src
      // survives. ST.afterData ensures/returns the registry entry without waiting.
      if (typeof srcValue === 'string'
          && srcValue !== 'inline'
          && srcValue.indexOf('localStorage:') !== 0
          && ST._dataRegistry) {
        var __resolvedSrc = window.__stResolveUrl(srcValue);
        var __entry = ST._dataRegistry.get(sourceName);
        if (!__entry) {
          __entry = { data: null, loaded: false, listeners: new Set() };
          ST._dataRegistry.set(sourceName, __entry);
        }
        __entry.src = __resolvedSrc;
      }

      const dispatch = (type, detail) => {
        document.dispatchEvent(new CustomEvent(`data:${sourceName}:${type}`, { detail }));
      };

      const load = async () => {
        if (!srcValue) {
          store.loading = false;
          store.error = 'No source URL provided';
          dispatch('error', store.error);
          return;
        }
        store.loading = true;
        store.error = null;
        dispatch('loading', true);

        // Resolve {locale} ONCE per load at function scope: the fetch path AND
        // the catch reporter read the same binding (BUG-274). The chain itself
        // is owned by the `locale` module's prelude (`%uses locale`) — one
        // implementation for every consumer, not a per-template copy.
        const resolvedLocale = window.__stResolveLocale();

        try {
          let result;

          if (srcValue.startsWith('localStorage:')) {
            // Load from localStorage
            const key = srcValue.slice('localStorage:'.length);
            const stored = localStorage.getItem(key);
            if (stored) {
              result = JSON.parse(stored);
            } else if (defaultValue !== undefined) {
              result = defaultValue;
            } else {
              result = null;
            }
          } else if (srcValue === 'inline') {
            // Inline data uses default value
            result = defaultValue;
          } else {
            // Fetch from URL (resolve {locale} placeholder if present)
            if (srcValue.indexOf('{locale}') !== -1 && !resolvedLocale) {
              console.warn('[Spacetime] data-source "' + sourceName + '": URL contains {locale} but no locale could be resolved. Falling back to "en". Ensure @locale runs before @data.');
            }
            const fetchUrl = window.__stResolveUrl(srcValue);
            const response = await fetch(fetchUrl);
            if (!response.ok) {
              throw new Error(response.statusText);
            }
            result = await response.json();
          }

          store.data = result;
          store.loading = false;
          dispatch('data', result);

          // Dual-channel publish (registry + global signal store) — ONE
          // implementation in the data-channel module (`%uses data-channel`).
          // The backing-file `src` was registered synchronously at init
          // (above) and survives __stPublish's in-place setData mutation —
          // nothing to do here for FUP-047 persistence.
          window.__stPublish(sourceName, result);

          if (Array.isArray(result) && result.length === 0) {
            dispatch('empty');
          }
        } catch (err) {
          store.loading = false;
          store.error = err.message;
          dispatch('error', err.message);

          // Resolve the URL for error reporting (same binding as the fetch path)
          const resolvedUrl = window.__stResolveUrl(srcValue);

          // Report to debugger if available (debug mode)
          if (window.__ST_DEBUG__ && window.__ST_DEBUG__.recordError) {
            window.__ST_DEBUG__.recordError('data-source', err, {
              name: sourceName,
              src: resolvedUrl
            });
          }

          // Always log a formatted bug report to console (for copy/paste)
          console.error(
            `%c[Spacetime Runtime Error]%c\n` +
            `Source: data-source\n` +
            `Name: ${sourceName}\n` +
            `URL: ${resolvedUrl}\n` +
            `Error: ${err.message}\n` +
            `\n--- Copy below for bug report ---\n` +
            JSON.stringify({
              type: 'runtime-error',
              source: 'data-source',
              name: sourceName,
              src: resolvedUrl,
              error: err.message,
              stack: err.stack,
              timestamp: new Date().toISOString(),
              url: window.location.href
            }, null, 2),
            'color: #ef4444; font-weight: bold;',
            'color: inherit;'
          );
        }
      };

      store.reload = load;

      // Initial load
      load();

      // Set up refresh interval if specified
      if (refreshInterval > 0) {
        setInterval(load, refreshInterval);
      }

      // Global sources persist for page lifetime (no cleanup needed)
    };
  }

  %emit js {
    window.__stDataSource({ name: %name, src: %src, default: %default, refresh: %refresh });
  }

  %exports {
    $data: any
    $loading: bool
    $error: string?
    $reload: fn() ~> effect
  }
}


/// Persist Data Primitive
///
/// Saves data to localStorage with optional debouncing.
///
/// @param key - localStorage key
/// @param data - Data to persist (reactive)
/// @param debounce - Debounce delay in ms
///
/// @yields $save - Function to manually trigger save
/// @yields $clear - Function to clear stored data

%primitive persist(
  key: string,
  data: any,
  debounce: number = 300
) {
  %emit js {
    const storageKey = %key;
    const debounceMs = %debounce;
    let timeoutId = null;

    const save = () => {
      const value = %data;
      if (value !== undefined && value !== null) {
        localStorage.setItem(storageKey, JSON.stringify(value));
      }
    };

    const debouncedSave = () => {
      if (timeoutId) clearTimeout(timeoutId);
      timeoutId = setTimeout(save, debounceMs);
    };

    const clear = () => {
      localStorage.removeItem(storageKey);
    };

    %yield save -> $save;
    %yield clear -> $clear;

    // Auto-save when data changes (if reactive binding is set up)
    // This would be wired by the macro layer

    %cleanup {
      if (timeoutId) clearTimeout(timeoutId);
    }
  }

  %exports {
    $save: fn() ~> effect
    $clear: fn() ~> effect
  }
}


/// Computed Value Primitive
///
/// Creates a derived/computed value from a data source.
///
/// @param source - Source data to compute from
/// @param compute - Computation expression
/// @param deps - Additional dependencies to watch
///
/// @yields $value - The computed result

%primitive computed(
  source: any,
  compute: expr,
  deps: array = []
) {
  %emit js {
    const computeFn = %compute;
    let value = null;

    const update = () => {
      const src = %source;
      try {
        value = typeof computeFn === 'function' ? computeFn(src) : computeFn;
        %yield value -> $value;
      } catch (err) {
        // Report to debugger if available
        if (window.__ST_DEBUG__ && window.__ST_DEBUG__.recordError) {
          window.__ST_DEBUG__.recordError('computed', err, {});
        }
        console.error('Computed value error:', err);
      }
    };

    // Initial computation
    update();

    // Re-compute when source changes
    // This would be wired by reactive system
  }

  %exports {
    $value: any
  }
}
