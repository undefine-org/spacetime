/// Local Storage Source Primitive
///
/// Returns a binding-protocol object with:
/// - Auto-load from localStorage
/// - Debounced auto-save on set()
/// - Cross-tab sync via storage events
/// - Proper cleanup
///
/// @param key - The localStorage key
/// @param default - Default value if nothing stored
///
/// @yields $binding - The binding-protocol object

%primitive local-storage-source(key: string, default: any?) {
  %emit js {
    (() => {
      const storageKey = %key;
      const defaultValue = %default;

      // Initial load
      let _value;
      try {
        const stored = window.localStorage.getItem(storageKey);
        _value = stored ? JSON.parse(stored) : defaultValue;
      } catch (e) {
        console.warn(`[ST] Failed to load ${storageKey}:`, e);
        _value = defaultValue;
      }

      let _saveTimer = null;
      const _listeners = new Set();

      const binding = {
        __stBinding: true,

        get() { return _value; },

        set(v) {
          if (JSON.stringify(_value) === JSON.stringify(v)) return;
          _value = v;

          // Debounced save (300ms)
          clearTimeout(_saveTimer);
          _saveTimer = setTimeout(() => {
            try {
              window.localStorage.setItem(storageKey, JSON.stringify(v));
            } catch (e) {
              console.warn(`[ST] Failed to save ${storageKey}:`, e);
            }
          }, 300);

          // Update CSS variable
          document.documentElement.style.setProperty(
            `--st-${storageKey}`,
            Array.isArray(v) ? v.length : v
          );

          // Dispatch update event
          document.dispatchEvent(new CustomEvent(`local:${storageKey}:updated`, {
            detail: v,
            bubbles: true
          }));

          // Notify listeners
          _listeners.forEach(fn => fn(v));
        },

        subscribe(fn) {
          _listeners.add(fn);
          return () => _listeners.delete(fn);
        }
      };

      // Cross-tab sync (watchLocalStorage)
      const onStorage = (e) => {
        if (e.key === storageKey && e.newValue !== null) {
          try {
            const newValue = JSON.parse(e.newValue);
            if (JSON.stringify(_value) !== e.newValue) {
              _value = newValue;

              // Update CSS variable
              document.documentElement.style.setProperty(
                `--st-${storageKey}`,
                Array.isArray(newValue) ? newValue.length : newValue
              );

              // Dispatch update event
              document.dispatchEvent(new CustomEvent(`local:${storageKey}:updated`, {
                detail: newValue,
                bubbles: true
              }));

              // Notify listeners
              _listeners.forEach(fn => fn(newValue));
            }
          } catch (e) {
            console.warn(`[ST] Failed to sync ${storageKey}:`, e);
          }
        }
      };
      window.addEventListener('storage', onStorage);

      // Initial CSS variable
      document.documentElement.style.setProperty(
        `--st-${storageKey}`,
        Array.isArray(_value) ? _value.length : _value
      );

      // Return the binding object
      return binding;
    })()
  }

  %exports {
    $binding: any
  }
}
