/// Each Iterator Primitive
///
/// Renders a template for each item in a data source.
/// Handles dynamic updates when data changes.
///
/// @param &container - Element to render items into
/// @param source - Data source name or array
/// @param template - Template element ID or selector
/// @param key - Property to use as unique key for diffing
///
/// @yields $items - Array of rendered item elements
/// @yields $update - Function to trigger re-render

%primitive each(
  &container,
  source: string,
  template: string,
  key: string = "id",
  bindings: array = []
) {
  %emit js {
    const containerEl = %&container;
    const sourceName = %source;
    const templateSelector = %template;
    const keyProp = %key;
    const slotBindings = %bindings;

    let items = [];
    // The signature of what is currently RENDERED. A re-render tears the list's
    // DOM down (`innerHTML = ''`) and rebuilds it, which destroys live DOM state
    // the markup cannot re-express: a focused input loses focus and caret, a
    // <select> resets to its first option, an open <details> closes. Polled
    // sources (`@data fetch { refresh }`) re-emit on EVERY tick, almost always
    // with identical data, so an unconditional rebuild meant a 2s poll silently
    // reverted the user's live interaction (BUG-151's class). Compare first and
    // skip the rebuild when nothing changed.
    let renderedSig = null;
    const dataSignature = (data) => {
      try {
        return JSON.stringify(data);
      } catch (e) {
        // Cyclic or non-serializable data cannot be compared cheaply; fall back to
        // always re-rendering, which is the old behaviour (correct, just costlier).
        return null;
      }
    };
    let templateEl = null;

    // Find template element
    const findTemplate = () => {
      // Try IK component system first (templates loaded via IK.init)
      const ikTemplate = window.IK?.templates?.[templateSelector];
      if (ikTemplate?.tagName === 'TEMPLATE') {
        return ikTemplate.content.cloneNode(true).firstElementChild;
      }

      // Try as ID, CSS selector, then data-component attribute
      templateEl = document.getElementById(templateSelector) ||
                   document.querySelector(templateSelector) ||
                   document.querySelector(`template[data-component="${templateSelector}"]`);

      if (templateEl && templateEl.tagName === 'TEMPLATE') {
        return templateEl.content.cloneNode(true).firstElementChild;
      }
      return templateEl ? templateEl.cloneNode(true) : null;
    };

    // Get nested property from object
    const getProperty = (obj, path) => {
      const parts = path.split('.');
      let value = obj;
      for (const part of parts) {
        if (value == null) return '';
        value = value[part];
      }
      return value ?? '';
    };

    // Escape attribute values to prevent XSS
    const escapeAttr = (str) => {
      if (str == null) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#x27;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    };

    // Apply slot bindings from @each declaration
    const applyBindings = (el, item) => {
      for (const binding of slotBindings) {
        const { sel, prop } = binding;
        const value = getProperty(item, prop);

        // Find slot element and replace it with value
        const slot = el.querySelector(`slot[name="${sel}"]`);
        if (slot) {
          // Skip elements being actively edited in dev mode
          if (slot.hasAttribute('data-st-editing')) continue;
          slot.textContent = value;
          if (window.__ST_DEV__) {
            slot.setAttribute('data-st-bind', '$.' + prop);
          }
        } else {
          // Try finding elements with slot attribute (already processed)
          const slotted = el.querySelector(`[slot="${sel}"]`);
          if (slotted) {
            // Skip elements being actively edited in dev mode
            if (slotted.hasAttribute('data-st-editing')) continue;
            slotted.textContent = value;
            if (window.__ST_DEV__) {
              slotted.setAttribute('data-st-bind', '$.' + prop);
            }
          }
        }
      }
    };

    // Interpolate bindings in element
    const interpolate = (el, item, index) => {
      // First apply slot bindings from @each declaration
      applyBindings(el, item);

      // Then replace any remaining $.property references in text
      const walk = (node) => {
        if (node.nodeType === Node.TEXT_NODE) {
          node.textContent = node.textContent.replace(
            /\$\.(\w+(?:\.\w+)*)/g,
            (match, path) => {
              if (window.__ST_DEV__ && node.parentElement) {
                node.parentElement.setAttribute('data-st-bind', match);
              }
              return getProperty(item, path);
            }
          );
        } else if (node.nodeType === Node.ELEMENT_NODE) {
          // Handle attributes
          for (const attr of Array.from(node.attributes)) {
            if (attr.value.includes('$.')) {
              attr.value = attr.value.replace(
                /\$\.(\w+(?:\.\w+)*)/g,
                (match, path) => escapeAttr(getProperty(item, path))
              );
            }
          }
          // Walk children
          for (const child of node.childNodes) {
            walk(child);
          }
        }
      };
      walk(el);

      // Set data-index attribute
      el.dataset.index = index;
      el.dataset.key = item[keyProp] ?? index;
      if (window.__ST_DEV__) {
        el.setAttribute('data-st-source', sourceName);
        el.setAttribute('data-st-index', String(index));
        if (item[keyProp] != null) {
          el.setAttribute('data-st-key', String(item[keyProp]));
        }
      }

      return el;
    };

    // Render all items
    const render = (data) => {
      if (!Array.isArray(data)) {
        console.warn('Each: expected array, got', typeof data);
        return;
      }

      // Nothing changed since the last render → keep the live DOM exactly as it
      // is. `null` means "not comparable", which must always re-render.
      const sig = dataSignature(data);
      if (sig !== null && sig === renderedSig && containerEl.children.length) {
        return;
      }
      renderedSig = sig;

      // Clear existing items
      containerEl.innerHTML = '';
      items = [];

      // Render each item
      data.forEach((item, index) => {
        const baseTemplate = findTemplate();
        if (!baseTemplate) {
          console.error('Each: template not found:', templateSelector);
          return;
        }

        const el = interpolate(baseTemplate.cloneNode(true), item, index);
        containerEl.appendChild(el);
        items.push(el);
      });

      %yield items -> $items;
    };

    // Listen for data source updates (data-registry channel: @data sources).
    document.addEventListener(`data:${sourceName}:loaded`, (e) => {
      render(e.detail);
    });

    // Also listen on the local-signal channel (state mutations + @data publish),
    // so @each over a $state signal re-renders when the signal changes — e.g.
    // `$tasks <- $tasks.concat(...)`. @each previously only heard the data
    // registry, so direct signal mutations never re-rendered the list.
    document.addEventListener(`local:${sourceName}:updated`, (e) => {
      if (e.detail != null) render(e.detail);
    });

    // Handle empty state
    document.addEventListener(`data:${sourceName}:empty`, () => {
      containerEl.innerHTML = '';
      renderedSig = null;
      items = [];
      %yield items -> $items;
    });

    const update = () => {
      const data = window.SpacetimeData?.[sourceName];
      if (data) render(data);
    };

    %yield update -> $update;

    %cleanup {
      // Cleanup handled by event listener removal in parent
    }
  }

  %exports {
    $items: array
    $update: fn() ~> effect
  }
}


/// Keyed Each Primitive
///
/// Optimized each with key-based diffing for efficient updates.
/// Only updates changed items instead of re-rendering all.
///
/// @param &container - Element to render items into
/// @param source - Data source name or array
/// @param template - Template element ID or selector
/// @param key - Property to use as unique key
///
/// @yields $items - Map of key -> element
/// @yields $update - Function to trigger re-render

%primitive keyed-each(
  &container,
  source: string,
  template: string,
  key: string = "id"
) {
  %emit js {
    const containerEl = %&container;
    const sourceName = %source;
    const templateSelector = %template;
    const keyProp = %key;

    const itemMap = new Map(); // key -> { el, data }

    const findTemplate = () => {
      // Try IK component system first (templates loaded via IK.init)
      const ikTemplate = window.IK?.templates?.[templateSelector];
      if (ikTemplate?.tagName === 'TEMPLATE') {
        return ikTemplate.content.cloneNode(true).firstElementChild;
      }

      // Try as ID, CSS selector, then data-component attribute
      const templateEl = document.getElementById(templateSelector) ||
                         document.querySelector(templateSelector) ||
                         document.querySelector(`template[data-component="${templateSelector}"]`);
      if (templateEl?.tagName === 'TEMPLATE') {
        return templateEl.content.cloneNode(true).firstElementChild;
      }
      return templateEl?.cloneNode(true) || null;
    };

    // Escape attribute values to prevent XSS
    const escapeAttr = (str) => {
      if (str == null) return '';
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#x27;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    };

    const interpolate = (el, item, index) => {
      const walk = (node) => {
        if (node.nodeType === Node.TEXT_NODE) {
          node.textContent = node.textContent.replace(
            /\$\.(\w+(?:\.\w+)*)/g,
            (match, path) => {
              if (window.__ST_DEV__ && node.parentElement) {
                node.parentElement.setAttribute('data-st-bind', match);
              }
              let value = item;
              for (const part of path.split('.')) {
                value = value?.[part];
              }
              return value ?? '';
            }
          );
        } else if (node.nodeType === Node.ELEMENT_NODE) {
          for (const attr of Array.from(node.attributes)) {
            if (attr.value.includes('$.')) {
              attr.value = attr.value.replace(
                /\$\.(\w+(?:\.\w+)*)/g,
                (_, path) => {
                  let value = item;
                  for (const part of path.split('.')) {
                    value = value?.[part];
                  }
                  return escapeAttr(value ?? '');
                }
              );
            }
          }
          for (const child of node.childNodes) {
            walk(child);
          }
        }
      };
      walk(el);
      el.dataset.key = item[keyProp] ?? index;
      if (window.__ST_DEV__) {
        el.setAttribute('data-st-source', sourceName);
        el.setAttribute('data-st-index', String(index));
        if (item[keyProp] != null) {
          el.setAttribute('data-st-key', String(item[keyProp]));
        }
      }
      return el;
    };

    const diff = (newData) => {
      if (!Array.isArray(newData)) return;

      const newKeys = new Set(newData.map((item, i) => item[keyProp] ?? i));

      // Remove items no longer in data
      for (const [key, { el }] of itemMap) {
        if (!newKeys.has(key)) {
          el.remove();
          itemMap.delete(key);
        }
      }

      // Add/update items
      let prevEl = null;
      newData.forEach((item, index) => {
        const itemKey = item[keyProp] ?? index;
        const existing = itemMap.get(itemKey);

        if (existing) {
          // Update existing - for now just reorder if needed
          if (prevEl) {
            if (existing.el.previousElementSibling !== prevEl) {
              prevEl.after(existing.el);
            }
          } else if (existing.el !== containerEl.firstElementChild) {
            containerEl.prepend(existing.el);
          }
          existing.data = item;
          prevEl = existing.el;
        } else {
          // Create new
          const baseTemplate = findTemplate();
          if (!baseTemplate) return;

          const el = interpolate(baseTemplate.cloneNode(true), item, index);

          if (prevEl) {
            prevEl.after(el);
          } else {
            containerEl.prepend(el);
          }

          itemMap.set(itemKey, { el, data: item });
          prevEl = el;
        }
      });

      %yield itemMap -> $items;
    };

    document.addEventListener(`data:${sourceName}:loaded`, (e) => {
      diff(e.detail);
    });

    // Local-signal channel (state mutations + @data publish) — keyed each
    // re-renders on direct $signal mutation, not only data-registry loads.
    document.addEventListener(`local:${sourceName}:updated`, (e) => {
      if (e.detail != null) diff(e.detail);
    });

    document.addEventListener(`data:${sourceName}:empty`, () => {
      for (const { el } of itemMap.values()) {
        el.remove();
      }
      itemMap.clear();
      %yield itemMap -> $items;
    });

    const update = () => {
      const data = window.SpacetimeData?.[sourceName];
      if (data) diff(data);
    };

    %yield update -> $update;
  }

  %exports {
    $items: object
    $update: fn() ~> effect
  }
}
