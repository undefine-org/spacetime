/// Dev Save Primitive
///
/// Orchestrates the save flow for dev tools inline editing.
/// Listens for edit events from dev-editable, sends EditJson/EditAst messages
/// via the dev-ws WebSocket, handles Ack/Reject responses, and
/// triggers @each re-rendering on DataUpdate via ST.setData().
///
/// Handles JSON edits (EditJson) and content edits (EditAst).
///
/// @yields $saving - Whether any edits are currently in-flight
/// @yields $lastSaveError - Last error message from a rejected edit
/// @yields $savedCount - Total number of successfully saved edits

%primitive dev-save() {
  %emit js {
    (function() {
      var saving = false;
      var lastSaveError = '';
      var savedCount = 0;

      // In-flight edits keyed by op_id: {element, oldValue, field, source}
      var pendingEdits = {};
      var pendingCount = 0;

      // Unsubscribe handles for cleanup
      var unsubDataUpdate = null;
      var unsubEditable = null;

      // Inject toast styles for save error notifications
      var toastStyleEl = document.createElement('style');
      toastStyleEl.setAttribute('data-st-dev', 'save-toast');
      toastStyleEl.textContent = [
        '@keyframes st-dev-toast-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }',
        '@keyframes st-dev-toast-out { from { opacity: 1; } to { opacity: 0; } }',
        '#st-dev-toast { position: fixed; bottom: 1.5rem; right: 1.5rem; z-index: 99999; display: flex; flex-direction: column; gap: 0.5rem; pointer-events: none; }',
        '.st-dev-toast-item { padding: 0.55rem 1rem; border-radius: 6px; font-family: system-ui, sans-serif; font-size: 13px; line-height: 1.4; max-width: 380px; word-break: break-word; background: rgba(17,17,27,0.92); color: #f38ba8; border: 1px solid rgba(243,139,168,0.35); box-shadow: 0 4px 16px rgba(0,0,0,0.4); animation: st-dev-toast-in 0.18s ease; backdrop-filter: blur(4px); pointer-events: auto; }',
        '.st-dev-toast-item.st-dev-toast-dismissing { animation: st-dev-toast-out 0.3s ease forwards; }'
      ].join('\n');
      if (document.head) {
        document.head.appendChild(toastStyleEl);
      } else {
        document.addEventListener('DOMContentLoaded', function() {
          document.head.appendChild(toastStyleEl);
        });
      }

      function yieldState() {
        %yield saving -> $saving;
        %yield lastSaveError -> $lastSaveError;
        %yield savedCount -> $savedCount;
      }

      function generateOpId() {
        return 'op_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
      }

      // Sanitizer mirrors __stSanitizeLocaleHtml in stdlib/primitives/i18n.st
      function __stSanitizeContentHtml(html) {
        // Strip script/style/iframe/object/embed/form tags and contents
        html = html.replace(new RegExp('<(script|style|iframe|object|embed|form)[^>]*>[\\s\\S]*?<\\/\\1>', 'gi'), '');
        // Strip self-closing dangerous tags
        html = html.replace(new RegExp('<(script|style|iframe|object|embed|form|input|textarea|select)[^>]*\\/?>', 'gi'), '');
        // Strip on* event handler attributes
        html = html.replace(new RegExp('\\s+on\\w+\\s*=\\s*(?:"[^"]*"|' + "'[^']*'" + '|[^\\s>]+)', 'gi'), '');
        // Strip javascript: protocol from href/src/action
        html = html.replace(new RegExp('(href|src|action)\\s*=\\s*"javascript:[^"]*"', 'gi'), '$1=""');
        html = html.replace(new RegExp("(href|src|action)\\s*=\\s*'javascript:[^']*'", 'gi'), "$1=''");
        return html;
      }


      // Resolve file path from source name.
      // Check ST._dataRegistry for the real src path, fallback to "data/<source>.json"
      function resolveFilePath(source) {
        if (typeof ST !== 'undefined' && ST._dataRegistry) {
          var entry = ST._dataRegistry.get(source);
          if (entry && entry.src && typeof entry.src === 'string') {
            // Strip leading slash for relative path
            var src = entry.src;
            if (src.charAt(0) === '/') src = src.substring(1);
            return src;
          }
        }
        return 'data/' + source + '.json';
      }

      // Visual feedback: flash an element's background color briefly
      function flashElement(el, color, duration) {
        if (!el || !el.style) return;
        var orig = el.style.transition;
        var origBg = el.style.backgroundColor;
        el.style.transition = 'background-color 0.15s ease';
        el.style.backgroundColor = color;
        setTimeout(function() {
          el.style.backgroundColor = origBg || '';
          setTimeout(function() {
            el.style.transition = orig || '';
          }, 200);
        }, duration || 500);
      }


      // Show a transient error toast. Replaces any existing toast to prevent pile-up.
      function showErrorToast(reason) {
        var container = document.getElementById('st-dev-toast');
        if (!container) {
          container = document.createElement('div');
          container.id = 'st-dev-toast';
          document.body ? document.body.appendChild(container) : document.addEventListener('DOMContentLoaded', function() { document.body.appendChild(container); });
        }
        // Remove existing items to prevent pile-up on rapid failures
        while (container.firstChild) {
          container.removeChild(container.firstChild);
        }
        var item = document.createElement('div');
        item.className = 'st-dev-toast-item';
        item.textContent = reason || 'Edit rejected';
        container.appendChild(item);
        // Auto-dismiss after 5 seconds with fade-out
        setTimeout(function() {
          item.classList.add('st-dev-toast-dismissing');
          setTimeout(function() {
            if (item.parentNode) item.parentNode.removeChild(item);
          }, 320);
        }, 5000);
      }
      // Process an edit event from dev-editable
      function saveEdit(editEvent) {
        var ws = window.__stDevWs;
        if (!ws || !ws.connected) {
          lastSaveError = 'WebSocket not connected';
          yieldState();
          return;
        }

        var opId = generateOpId();
        var filePath, jsonPath, value, element, oldValue;

        if (editEvent.editType === 'content') {
          // Content edit: use editEvent.file, selector, innerHTML values
          filePath = editEvent.file;
          element = editEvent.elementId ? document.getElementById(editEvent.elementId) : null;
          oldValue = editEvent.oldValue;
          var sanitizedContent = __stSanitizeContentHtml(editEvent.newValue || '');
        } else if (editEvent.editType === 'locale') {
          // Locale edit: use editEvent.file, localeKey, newValue
          filePath = editEvent.file;
          jsonPath = editEvent.localeKey;
          value = editEvent.newValue;
          element = editEvent.elementId ? document.getElementById(editEvent.elementId) : null;
          oldValue = editEvent.oldValue;
        } else {
          // Data edit. Two path shapes flow through here uniformly:
          //   - COLLECTION (admin/each): {source, index, field} → '[i].field'
          //   - SINGLE/EXPLICIT (on-page @richtext): {source, path} → path verbatim
          //     (e.g. '.body' for $post.body, '[2].body' for $posts[2].body). The
          //     surface computes the path from its bind ($x.f / $x[i].f) and passes
          //     it explicitly, so the page case needs no index/field convention.
          var source = editEvent.source;
          var field = editEvent.field;
          value = editEvent.newValue !== undefined ? editEvent.newValue : editEvent.value;
          element = editEvent.elementId ? document.getElementById(editEvent.elementId) : editEvent.element || null;
          oldValue = editEvent.oldValue;
          filePath = editEvent.file || resolveFilePath(source);
          if (editEvent.path != null) {
            jsonPath = editEvent.path;
          } else {
            jsonPath = '[' + editEvent.index + '].' + field;
          }
        }

        // Track this in-flight edit
        pendingEdits[opId] = {
          element: element,
          oldValue: oldValue,
          field: editEvent.field || editEvent.localeKey,
          source: editEvent.source || editEvent.file,
          editType: editEvent.editType,
          selector: editEvent.selector,
          stId: editEvent.stId,
          newValue: sanitizedContent !== undefined ? sanitizedContent : editEvent.newValue
        };
        pendingCount++;
        saving = true;
        lastSaveError = '';
        yieldState();

        // Send message: EditAst for content edits, EditJson for data/locale
        if (editEvent.editType === 'content') {
          var patch = { '__content': sanitizedContent };
          if (editEvent.stId) patch.__st_id = editEvent.stId;
          ws.send({
            type: 'EditAst',
            file: filePath,
            selector: editEvent.selector,
            patch: patch,
            op_id: opId
          });
        } else {
          ws.send({
            type: 'EditJson',
            file: filePath,
            path: jsonPath,
            value: value,
            op_id: opId
          });
        }

        // Register ack/reject callbacks
        ws.onAck(opId, function() {
          var edit = pendingEdits[opId];
          delete pendingEdits[opId];
          pendingCount--;
          savedCount++;
          saving = pendingCount > 0;

          // Green flash on success
          if (edit && edit.element) {
            flashElement(edit.element, 'rgba(166,227,161,0.3)', 500);
          }

          // Sibling propagation for content edits
          if (edit && edit.editType === 'content' && edit.element) {
            var file = edit.source;
            var locator;
            if (file && file.match(/\.html?$/i)) {
              locator = edit.stId;
            } else {
              locator = edit.selector;
            }
            if (locator) {
              var originVal = file + '::' + locator;
              var siblings = document.querySelectorAll(
                '[data-st-origin="' + CSS.escape(originVal) + '"]'
              );
              for (var i = 0; i < siblings.length; i++) {
                if (siblings[i] !== edit.element) {
                  siblings[i].innerHTML = edit.newValue;
                }
              }
            }
          }

          yieldState();
        });

        ws.onReject(opId, function(msg) {
          var edit = pendingEdits[opId];
          delete pendingEdits[opId];
          pendingCount--;
          saving = pendingCount > 0;
          lastSaveError = (msg && msg.reason) || 'Edit rejected';
          showErrorToast(lastSaveError);

          // Red flash on failure and revert
          if (edit && edit.element) {
            flashElement(edit.element, 'rgba(243,139,168,0.3)', 500);
            // Revert the DOM to original value
            if (edit.oldValue !== undefined) {
              if (edit.editType === 'content') {
                edit.element.innerHTML = String(edit.oldValue);
              } else {
                edit.element.textContent = String(edit.oldValue);
              }
            }
          }

          yieldState();
        });
      }

      // Listen for DataUpdate messages to trigger @each re-rendering
      function handleDataUpdate(msg) {
        if (!msg || !msg.source || !msg.data) return;

        // Extract source name from file path: "data/products.json" -> "products"
        var sourceName = msg.source;
        // Strip directory prefix
        var slashIdx = sourceName.lastIndexOf('/');
        if (slashIdx !== -1) sourceName = sourceName.substring(slashIdx + 1);
        // Strip .json extension
        if (sourceName.indexOf('.json') === sourceName.length - 5) {
          sourceName = sourceName.substring(0, sourceName.length - 5);
        }

        // Update via ST.setData which triggers @each re-rendering
        if (typeof ST !== 'undefined' && ST.setData) {
          ST.setData(sourceName, msg.data);
        }
      }

      // Subscribe to DataUpdate messages from dev-ws
      if (window.__stDevWs) {
        unsubDataUpdate = window.__stDevWs.onMessage('DataUpdate', handleDataUpdate);
      }

      // Subscribe to edit events from dev-editable
      function connectEditable() {
        if (window.__stDevEditable && window.__stDevEditable.onEdit) {
          unsubEditable = window.__stDevEditable.onEdit(function(editEvent) {
            saveEdit(editEvent);
          });
        }
      }

      // dev-editable may not be ready yet; try now and retry on interval
      connectEditable();
      var editableCheckInterval = null;
      if (!unsubEditable) {
        editableCheckInterval = setInterval(function() {
          connectEditable();
          if (unsubEditable) {
            clearInterval(editableCheckInterval);
            editableCheckInterval = null;
          }
        }, 200);
      }

      // Expose global API for other dev primitives
      window.__stDevSave = {
        save: function(editEvent) {
          saveEdit(editEvent);
        },
        get pending() {
          return pendingCount;
        },
        saveArrayInsert: function(file, path, item, index) {
          var ws = window.__stDevWs;
          if (!ws || !ws.connected) return;
          var opId = generateOpId();
          var op = index !== undefined && index !== null
            ? { op: 'Insert', item: item, index: index }
            : { op: 'Insert', item: item, index: null };
          pendingEdits[opId] = { source: file, editType: 'arrayInsert' };
          pendingCount++;
          saving = true;
          yieldState();
          ws.send({ type: 'EditJsonArray', file: file, path: path || '', op: op, op_id: opId });
          ws.onAck(opId, function() {
            delete pendingEdits[opId];
            pendingCount--;
            savedCount++;
            saving = pendingCount > 0;
            yieldState();
          });
          ws.onReject(opId, function(reason) {
            delete pendingEdits[opId];
            pendingCount--;
            saving = pendingCount > 0;
            lastSaveError = reason;
            showErrorToast(reason);
            yieldState();
          });
        },
        saveArrayDelete: function(file, path, index) {
          var ws = window.__stDevWs;
          if (!ws || !ws.connected) return;
          var opId = generateOpId();
          pendingEdits[opId] = { source: file, editType: 'arrayDelete' };
          pendingCount++;
          saving = true;
          yieldState();
          ws.send({ type: 'EditJsonArray', file: file, path: path || '', op: { op: 'Delete', index: index }, op_id: opId });
          ws.onAck(opId, function() {
            delete pendingEdits[opId];
            pendingCount--;
            savedCount++;
            saving = pendingCount > 0;
            yieldState();
          });
          ws.onReject(opId, function(reason) {
            delete pendingEdits[opId];
            pendingCount--;
            saving = pendingCount > 0;
            lastSaveError = reason;
            showErrorToast(reason);
            yieldState();
          });
        },
        saveArrayReorder: function(file, path, fromIndex, toIndex) {
          var ws = window.__stDevWs;
          if (!ws || !ws.connected) return;
          var opId = generateOpId();
          pendingEdits[opId] = { source: file, editType: 'arrayReorder' };
          pendingCount++;
          saving = true;
          yieldState();
          ws.send({ type: 'EditJsonArray', file: file, path: path || '', op: { op: 'Reorder', from_index: fromIndex, to_index: toIndex }, op_id: opId });
          ws.onAck(opId, function() {
            delete pendingEdits[opId];
            pendingCount--;
            savedCount++;
            saving = pendingCount > 0;
            yieldState();
          });
          ws.onReject(opId, function(reason) {
            delete pendingEdits[opId];
            pendingCount--;
            saving = pendingCount > 0;
            lastSaveError = reason;
            showErrorToast(reason);
            yieldState();
          });
        }
      };

      // Initial yield
      yieldState();

      %cleanup {
        if (unsubDataUpdate) unsubDataUpdate();
        if (unsubEditable) unsubEditable();
        if (editableCheckInterval) clearInterval(editableCheckInterval);
        pendingEdits = {};
        pendingCount = 0;
        window.__stDevSave = null;
        if (toastStyleEl && toastStyleEl.parentNode) toastStyleEl.parentNode.removeChild(toastStyleEl);
        var toastContainer = document.getElementById('st-dev-toast');
        if (toastContainer && toastContainer.parentNode) toastContainer.parentNode.removeChild(toastContainer);
      }
    })();
  }

  %exports {
    $saving: bool
    $lastSaveError: string
    $savedCount: number
  }
}
