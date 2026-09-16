/// Dev Editable Primitive
///
/// Makes provenance-annotated DOM elements click-to-edit using
/// contenteditable. Elements with `data-st-bind`, `data-st-source`,
/// or `data-t` (i18n locale) attributes get hover affordances and inline editing.
///
/// Does NOT handle saving — only captures edits and emits events
/// via `window.__stDevEditable` API. The dev-save primitive
/// subscribes to these events to persist changes.
///
/// @yields $activeEdit - Current edit in progress or null
/// @yields $editCount - Number of completed edits this session

%primitive dev-editable() %uses locale {
  %emit js {
    (function() {
      var activeEdit = null;
      var editCount = 0;
      var editListeners = [];
      var styleEl = null;
      var observer = null;
      var active = false;

      // --- Inject hover/edit affordance styles into document head ---
      styleEl = document.createElement('style');
      styleEl.setAttribute('data-st-dev', 'editable');
      styleEl.textContent = [
        '@keyframes st-dev-saving { 0%, 100% { outline-color: rgba(137,180,250,0.4); } 50% { outline-color: rgba(137,180,250,1); } }',
        'html[data-st-edit-mode] [data-st-bind], html[data-st-edit-mode] [data-t], html[data-st-edit-mode] [data-st-id][data-st-origin] { position: relative; }',
        'html[data-st-edit-mode] [data-st-bind]:hover, html[data-st-edit-mode] [data-t]:hover, html[data-st-edit-mode] [data-st-id][data-st-origin]:hover { outline: 1px dashed rgba(137,180,250,0.5); outline-offset: 2px; cursor: text; }',
        'html[data-st-edit-mode] [data-st-bind]:hover::after, html[data-st-edit-mode] [data-t]:hover::after, html[data-st-edit-mode] [data-st-id][data-st-origin]:hover::after { content: "Click to edit"; position: absolute; top: -1.6em; right: 0; font-size: 10px; padding: 1px 5px; background: rgba(30,30,46,0.85); color: #cdd6f4; border-radius: 3px; pointer-events: none; opacity: 0; transition: opacity 0.15s ease; white-space: nowrap; z-index: 9999; font-family: system-ui, sans-serif; }',
        'html[data-st-edit-mode] [data-st-bind]:hover:not([contenteditable="true"]):not([data-st-saving])::after, html[data-st-edit-mode] [data-t]:hover:not([contenteditable="true"]):not([data-st-saving])::after, html[data-st-edit-mode] [data-st-id][data-st-origin]:hover:not([contenteditable="true"]):not([data-st-saving])::after { opacity: 1; }',
        '[data-st-bind][contenteditable="true"], [data-t][contenteditable="true"], [data-st-id][data-st-origin][contenteditable="true"] { outline: 2px solid rgba(137,180,250,0.8); outline-offset: 2px; background: rgba(137,180,250,0.05); cursor: text; min-height: 1em; max-height: 12em; overflow-y: auto; }',
        '[data-st-bind][contenteditable="true"]:focus, [data-t][contenteditable="true"]:focus, [data-st-id][data-st-origin][contenteditable="true"]:focus { outline: 2px solid #89b4fa; outline-offset: 2px; box-shadow: 0 0 0 4px rgba(137,180,250,0.15); }',
        'html[data-st-edit-mode] [data-st-bind][contenteditable="true"]:empty::before, html[data-st-edit-mode] [data-t][contenteditable="true"]:empty::before, html[data-st-edit-mode] [data-st-id][data-st-origin][contenteditable="true"]:empty::before { content: "Type something…"; color: rgba(205,214,244,0.35); pointer-events: none; font-style: italic; }',
        '[data-st-bind][data-st-saving], [data-t][data-st-saving], [data-st-id][data-st-origin][data-st-saving] { outline: 2px solid rgba(137,180,250,0.6); outline-offset: 2px; animation: st-dev-saving 1s ease-in-out infinite; pointer-events: none; }'
      ].join('\n');
      if (document.head) {
        document.head.appendChild(styleEl);
      } else {
        document.addEventListener('DOMContentLoaded', function() {
          document.head.appendChild(styleEl);
        });
      }

      // --- Utility: read provenance from element or ancestors ---
      function readProvenance(el) {
        // Check for locale element first
        var localeKey = el.getAttribute('data-t');
        if (localeKey && !el.hasAttribute('data-st-bind')) {
          return {
            editType: 'locale',
            localeKey: localeKey,
            // Full chain via the locale module (`%uses locale`): runtime
            // override → document lang → 'en' (the local copy had dropped the
            // document-lang step — FUP-175 R3 drift).
            locale: window.__stResolveLocale(),
            source: null,
            index: null,
            key: null,
            field: null,
            filter: null,
            file: 'locales/' + window.__stResolveLocale() + '.json'
          };
        }

        var bind = el.getAttribute('data-st-bind');
        if (!bind) {
          // Third branch: content element (data-st-id + data-st-origin)
          var stId = el.getAttribute('data-st-id');
          var origin = el.getAttribute('data-st-origin');
          if (stId && origin) {
            var sepIdx = origin.indexOf('::');
            if (sepIdx === -1) return null; // malformed → not editable
            var file = origin.substring(0, sepIdx);
            var locator = origin.substring(sepIdx + 2);
            if (!file || !locator) return null; // empty parts → not editable

            // Reconstruct selector based on file type
            var selector;
            if (file.match(/\.html?$/i)) {
              selector = '[data-st-id="' + stId + '"]';
            } else {
              selector = locator; // .st files: locator IS the selector
            }

            return {
              editType: 'content',
              stId: stId,
              file: file,
              selector: selector,
              source: null, index: null, key: null, field: null, filter: null
            };
          }

          return null;
        }

        // Walk up to find source container
        var source = null;
        var index = null;
        var key = null;
        var filter = null;
        var current = el;
        while (current && current !== document.body) {
          if (current.hasAttribute('data-st-source')) {
            source = current.getAttribute('data-st-source');
            index = current.getAttribute('data-st-index');
            key = current.getAttribute('data-st-key');
            filter = current.getAttribute('data-st-filter');
            break;
          }
          current = current.parentElement;
        }

        // Two binding shapes resolve to a persist target (mirrors FUP-047):
        //   COLLECTION (an @each row): a `[data-st-source]` ancestor supplied `source`
        //     + `index`; the bind is a per-row field (`$.title` → field 'title'). dev-save
        //     builds '[index].field'.
        //   SINGLE OBJECT (a plain bind outside any @each, e.g. `$post.title`): there is
        //     NO source ancestor, so the BIND ITSELF carries the whole path. Parse it into
        //     {source, path}: first segment after `$` is the data-source name, the rest is
        //     the JSON path ('.title', '[2].body', …). dev-save persists {source, path}
        //     directly (the file resolves via _dataRegistry.get(source).src). This is what
        //     makes the WHOLE published page editable, not just @each rows (W3).
        if (source === null) {
          var raw = bind.replace(/^\$/, '');           // 'post.title' / 'posts[2].title'
          var m = raw.match(/^([^.\[]+)(.*)$/);          // source = first segment
          if (m && m[1] && m[2]) {
            return {
              editType: 'data',
              source: m[1],
              index: null,
              key: null,
              field: null,
              path: m[2],                               // '.title' / '[2].title'
              filter: null
            };
          }
        }

        return {
          editType: 'data',
          source: source,
          index: index !== null && index !== undefined ? index : null,
          key: key || null,
          field: bind.replace(/^\$\./, ''),
          path: null,
          filter: filter || null
        };
      }

      // --- Find closest editable element ---
      function findEditableTarget(target) {
        var el = target;
        while (el && el !== document.body) {
          if (el.hasAttribute('data-st-bind') || el.hasAttribute('data-t') || (el.hasAttribute('data-st-id') && el.hasAttribute('data-st-origin'))) return el;
          el = el.parentElement;
        }
        return null;
      }

      // --- Start editing ---
      function startEdit(el) {
        if (activeEdit) {
          // Finish any active edit first
          finishEdit(activeEdit.element, false);
        }

        var provenance = readProvenance(el);
        if (!provenance) return;

        var isContent = provenance.editType === 'content';
        var oldValue = isContent ? el.innerHTML : el.textContent;
        if (!isContent) {
          // Normalize to single text node BEFORE selection to prevent
          // cross-node duplication when element has child spans or
          // stale text nodes from a previous re-render.
          el.textContent = oldValue;
        }
        el.setAttribute('contenteditable', 'true');
        el.setAttribute('data-st-editing', 'true');
        el.focus();

        // Select all text for easy replacement
        var range = document.createRange();
        range.selectNodeContents(el);
        var sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);

        activeEdit = {
          element: el,
          editType: provenance.editType || 'data',
          source: provenance.source,
          index: provenance.index,
          key: provenance.key,
          field: provenance.field,
          path: provenance.path || null,
          filter: provenance.filter,
          localeKey: provenance.localeKey || null,
          locale: provenance.locale || null,
          file: provenance.file || null,
          selector: provenance.selector || null,
          stId: provenance.stId || null,
          oldValue: oldValue,
          elementId: el.id || null
        };

        %yield activeEdit -> $activeEdit;
      }

      // --- Finish editing ---
      function finishEdit(el, cancelled) {
        if (!activeEdit || activeEdit.element !== el) {
          if (activeEdit && !activeEdit.element.isConnected && activeEdit.elementId) {
            var replacement = document.getElementById(activeEdit.elementId);
            if (replacement) {
              el = replacement;
              activeEdit.element = replacement;
            } else {
              activeEdit = null;
              %yield activeEdit -> $activeEdit;
              return;
            }
          } else {
            return;
          }
        }

        el.removeAttribute('contenteditable');
        el.removeAttribute('data-st-editing');
        var newValue;
        var isContent = activeEdit.editType === 'content';
        if (!el.isConnected) {
          if (activeEdit.elementId) {
            var fresh = document.getElementById(activeEdit.elementId);
            if (fresh) {
              newValue = isContent ? fresh.innerHTML : fresh.textContent;
            } else {
              activeEdit = null;
              %yield activeEdit -> $activeEdit;
              return;
            }
          } else {
            activeEdit = null;
            %yield activeEdit -> $activeEdit;
            return;
          }
        } else {
          newValue = isContent ? el.innerHTML : el.textContent;
        }

        if (cancelled) {
          // Restore original value
          if (isContent) {
            el.innerHTML = activeEdit.oldValue;
          } else {
            el.textContent = activeEdit.oldValue;
          }
        } else if (newValue !== activeEdit.oldValue) {
          // Emit edit event
          var editEvent;
          if (activeEdit.editType === 'locale') {
            editEvent = {
              editType: 'locale',
              localeKey: activeEdit.localeKey,
              locale: activeEdit.locale,
              file: activeEdit.file,
              oldValue: activeEdit.oldValue,
              newValue: newValue,
              elementId: activeEdit.elementId
            };
          } else if (activeEdit.editType === 'content') {
            editEvent = {
              editType: 'content',
              file: activeEdit.file,
              selector: activeEdit.selector,
              stId: activeEdit.stId,
              oldValue: activeEdit.oldValue,
              newValue: newValue,
              elementId: activeEdit.elementId
            };
          } else {
            editEvent = {
              editType: 'data',
              source: activeEdit.source,
              index: activeEdit.index,
              key: activeEdit.key,
              field: activeEdit.field,
              path: activeEdit.path || null,   // single-object persist path (W3); null → collection
              filter: activeEdit.filter,
              oldValue: activeEdit.oldValue,
              newValue: newValue,
              elementId: activeEdit.elementId
            };
          }

          editCount = editCount + 1;
          %yield editCount -> $editCount;

          // Notify listeners
          for (var i = 0; i < editListeners.length; i++) {
            try {
              editListeners[i](editEvent);
            } catch (e) {
              // Swallow listener errors
            }
          }
        }

        activeEdit = null;
        %yield activeEdit -> $activeEdit;
      }

      // --- Event delegation on document.body ---
      function handleClick(e) {
        if (!active) return;
        var el = findEditableTarget(e.target);
        if (el) {
          e.preventDefault();
          e.stopPropagation();
          startEdit(el);
        }
      }

      function handleKeydown(e) {
        if (!activeEdit) return;

        if (e.key === 'Escape') {
          e.preventDefault();
          finishEdit(activeEdit.element, true);
        } else if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          finishEdit(activeEdit.element, false);
        }
      }

      function handleBlur(e) {
        if (!activeEdit) return;
        var editEl = activeEdit.element;
        setTimeout(function() {
          if (!activeEdit) return;
          var currentEl = activeEdit.element;
          // If the observer replaced the element (recovery), don't auto-finish —
          // the user should continue editing the replacement.
          if (currentEl !== editEl && currentEl.isConnected) return;
          if (currentEl === editEl || !editEl.isConnected) {
            finishEdit(currentEl, false);
          }
        }, 100);
      }

      // Use capture phase for click to intercept before other handlers
      document.addEventListener('click', handleClick, true);
      document.addEventListener('keydown', handleKeydown, true);
      document.addEventListener('focusout', handleBlur, true);

      // --- MutationObserver: detect new data-st-bind elements ---
      // (No per-element setup needed since we use event delegation,
      //  but observer is here if future features need it)
      observer = new MutationObserver(function(mutations) {
        if (!activeEdit) return;
        var editEl = activeEdit.element;
        if (editEl.isConnected) return;
        if (activeEdit.elementId) {
          var replacement = document.getElementById(activeEdit.elementId);
          if (replacement) {
            var isContent = activeEdit.editType === 'content';
            replacement.setAttribute('contenteditable', 'true');
            replacement.setAttribute('data-st-editing', 'true');
            // Preserve the user's in-progress edit on the replacement.
            // Use innerHTML for content edits, textContent for text edits.
            if (isContent) {
              replacement.innerHTML = editEl.innerHTML;
            } else {
              replacement.textContent = editEl.textContent;
            }
            replacement.focus();
            activeEdit.element = replacement;
            return;
          }
        }
        activeEdit = null;
        %yield activeEdit -> $activeEdit;
      });
      observer.observe(document.body || document.documentElement, {
        childList: true,
        subtree: true
      });

      // --- Edit mode activation ---
      function activate() {
        if (active) return;
        active = true;
        document.documentElement.setAttribute('data-st-edit-mode', 'true');
        %yield active -> $editModeActive;
      }

      function deactivate() {
        if (!active) return;
        active = false;
        document.documentElement.removeAttribute('data-st-edit-mode');
        // If there's an active edit, finish it
        if (activeEdit) {
          finishEdit(activeEdit.element, false);
        }
        %yield active -> $editModeActive;
      }

      function toggle() {
        if (active) { deactivate(); } else { activate(); }
      }

      // --- Global API ---
      window.__stDevEditable = {
        onEdit: function(callback) {
          editListeners.push(callback);
          // Return unsubscribe function
          return function() {
            var idx = editListeners.indexOf(callback);
            if (idx !== -1) editListeners.splice(idx, 1);
          };
        },

        getActiveEdit: function() {
          return activeEdit;
        },

        cancelEdit: function() {
          if (activeEdit) {
            finishEdit(activeEdit.element, true);
          }
        },

        // Mark an element as saving (pulsing border animation)
        markSaving: function(el) {
          if (el && el.setAttribute) {
            el.setAttribute('data-st-saving', 'true');
          }
        },

        // Clear saving state from an element
        clearSaving: function(el) {
          if (el && el.removeAttribute) {
            el.removeAttribute('data-st-saving');
          }
        },

        activate: activate,
        deactivate: deactivate,
        toggle: toggle,
        get active() { return active; }
      };

      // Initial yields
      %yield activeEdit -> $activeEdit;
      %yield editCount -> $editCount;
      %yield active -> $editModeActive;

      %cleanup {
        document.removeEventListener('click', handleClick, true);
        document.removeEventListener('keydown', handleKeydown, true);
        document.removeEventListener('focusout', handleBlur, true);
        if (observer) {
          observer.disconnect();
          observer = null;
        }
        document.documentElement.removeAttribute('data-st-edit-mode');
        if (styleEl && styleEl.parentNode) {
          styleEl.parentNode.removeChild(styleEl);
        }
        if (activeEdit) {
          activeEdit.element.removeAttribute('contenteditable');
          activeEdit.element.removeAttribute('data-st-editing');
          activeEdit = null;
        }
        window.__stDevEditable = null;
      }
    })();
  }

  %exports {
    $activeEdit: object
    $editCount: number
    $editModeActive: bool
  }
}
