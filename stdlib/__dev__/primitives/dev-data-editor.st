/// Data Editor Primitive
///
/// Adds an "Editor" tab to the dev panel dashboard with a schema-driven
/// form generator for JSON data sources.
///
/// - Fetches type schemas from /__spacetime/dev/types.json
/// - When a data source is selected (via content tree), fetches its JSON data
/// - Generates form fields based on JSON Schema (string, number, boolean,
///   uri, array, object)
/// - Debounced saves via window.__stDevWs.send() EditJson messages
/// - Array CRUD via window.__stDevSave.saveArrayInsert/Delete
/// - Visual feedback: green flash on success, red on reject
///
/// Exposes window.__stDevDataEditor = { loadSource(name) }

%primitive dev-data-editor() {
  %emit js {
    (function() {
      function init() {
        if (!window.__stDevPanel) { setTimeout(init, 200); return; }
        var shadow = window.__stDevPanel.shadow;
        var tabBar = shadow.querySelector('.st-panel-tabs');
        var contentArea = shadow.querySelector('.st-panel-content');
        if (!tabBar || !contentArea) { setTimeout(init, 200); return; }

        // Add tab button
        var tabBtn = document.createElement('button');
        tabBtn.className = 'st-tab';
        tabBtn.setAttribute('data-tab', 'editor');
        tabBtn.textContent = 'Editor';
        tabBar.appendChild(tabBtn);

        // Add editor pane
        var pane = document.createElement('div');
        pane.className = 'st-pane';
        pane.setAttribute('data-pane', 'editor');
        pane.innerHTML = '<p class="st-placeholder">Select a data source from the Content tab</p>';
        contentArea.appendChild(pane);

        tabBtn.addEventListener('click', function() {
          window.__stDevPanel.switchTab('editor');
        });

        // Inject styles
        var style = document.createElement('style');
        style.textContent = [
          '.st-editor-form { padding: 8px; }',
          '.st-editor-header { display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; margin-bottom: 8px; background: #181825; border-radius: 4px; }',
          '.st-editor-title { font-size: 12px; font-weight: 600; color: #cdd6f4; }',
          '.st-editor-status { font-size: 10px; color: #585b70; }',
          '.st-field { margin-bottom: 8px; padding: 6px 8px; border-radius: 4px; background: rgba(49,50,68,0.3); }',
          '.st-field-label { display: block; font-size: 11px; color: #a6adc8; margin-bottom: 3px; font-weight: 500; }',
          '.st-field-input { width: 100%; background: #181825; border: 1px solid #313244; color: #cdd6f4; padding: 4px 8px; border-radius: 3px; font-size: 12px; font-family: inherit; outline: none; transition: border-color 0.15s; }',
          '.st-field-input:focus { border-color: #89b4fa; }',
          '.st-field-input[type="checkbox"] { width: auto; margin-right: 6px; }',
          '.st-field-input.st-url-input { color: #89b4fa; }',
          '.st-field-input.st-save-ok { border-color: #a6e3a1; background: rgba(166,227,161,0.05); }',
          '.st-field-input.st-save-err { border-color: #f38ba8; background: rgba(243,139,168,0.05); }',
          '.st-array-section { margin: 8px 0; padding: 6px; border: 1px solid #313244; border-radius: 4px; }',
          '.st-array-header { display: flex; align-items: center; justify-content: space-between; padding: 4px 6px; cursor: pointer; }',
          '.st-array-title { font-size: 11px; color: #cdd6f4; font-weight: 600; }',
          '.st-array-count { font-size: 10px; color: #585b70; }',
          '.st-array-actions { display: flex; gap: 4px; }',
          '.st-array-btn { background: none; border: 1px solid #313244; color: #a6adc8; padding: 2px 8px; border-radius: 3px; font-size: 11px; cursor: pointer; transition: background 0.12s, color 0.12s; }',
          '.st-array-btn:hover { background: #313244; color: #cdd6f4; }',
          '.st-array-btn.st-btn-add { border-color: rgba(166,227,161,0.3); color: #a6e3a1; }',
          '.st-array-btn.st-btn-add:hover { background: rgba(166,227,161,0.1); }',
          '.st-array-btn.st-btn-remove { border-color: rgba(243,139,168,0.3); color: #f38ba8; }',
          '.st-array-btn.st-btn-remove:hover { background: rgba(243,139,168,0.1); }',
          '.st-array-items { padding-top: 6px; }',
          '.st-array-item { padding: 6px; margin-bottom: 4px; background: rgba(24,24,37,0.5); border-radius: 3px; position: relative; }',
          '.st-array-item-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px; }',
          '.st-array-item-idx { font-size: 10px; color: #585b70; }',
          '.st-object-section { margin: 4px 0; padding: 4px 6px; border-left: 2px solid #313244; }',
          '.st-object-toggle { cursor: pointer; font-size: 11px; color: #a6adc8; margin-bottom: 4px; }',
          '.st-load-more { display: block; width: 100%; background: none; border: 1px dashed #313244; color: #585b70; padding: 6px; border-radius: 3px; font-size: 11px; cursor: pointer; margin-top: 4px; text-align: center; }',
          '.st-load-more:hover { border-color: #89b4fa; color: #89b4fa; }',
          '.st-raw-editor { width: 100%; min-height: 200px; background: #181825; border: 1px solid #313244; color: #cdd6f4; padding: 8px; border-radius: 4px; font-size: 12px; font-family: "SF Mono", "Fira Code", monospace; resize: vertical; outline: none; }',
          '.st-raw-editor:focus { border-color: #89b4fa; }',
          '.st-add-field-btn { background: none; border: 1px dashed #313244; color: #585b70; padding: 3px 8px; border-radius: 3px; font-size: 11px; cursor: pointer; margin-top: 4px; }',
          '.st-add-field-btn:hover { border-color: #89b4fa; color: #89b4fa; }'
        ].join('\n');
        shadow.appendChild(style);

        var typeSchemas = null;
        var dataSourceMap = null;
        var currentSource = null;
        var currentData = null;
        var currentFile = null;
        var debounceTimers = {};
        var arrayVisibleCount = {}; // track "load more" state per array path

        // Fetch type schemas and data source mapping on init
        fetch('/__spacetime/dev/types.json')
          .then(function(r) { return r.json(); })
          .then(function(d) { typeSchemas = d.types || {}; })
          .catch(function() { typeSchemas = {}; });

        fetch('/__spacetime/dev/site.json')
          .then(function(r) { return r.json(); })
          .then(function(d) { dataSourceMap = d.dataSourceMap || {}; })
          .catch(function() { dataSourceMap = {}; });

        function loadSource(name) {
          currentSource = name;
          currentFile = name;
          pane.innerHTML = '<p class="st-placeholder">Loading ' + escHtml(name) + '...</p>';

          fetch('/' + name)
            .then(function(r) { return r.json(); })
            .then(function(data) {
              currentData = data;
              renderEditor();
            })
            .catch(function() {
              pane.innerHTML = '<p class="st-placeholder">Failed to load data source: ' + escHtml(name) + '</p>';
            });

          // Switch to editor tab
          window.__stDevPanel.switchTab('editor');
        }

        function renderEditor() {
          if (!currentSource || currentData === null) return;

          // Resolve type name from dataSourceMap, then look up schema by type name
          var sourceInfo = dataSourceMap ? dataSourceMap[currentSource] : null;
          var typeName = sourceInfo ? sourceInfo.type : null;
          var schema = typeName && typeSchemas ? typeSchemas[typeName] : null;
          var html = '<div class="st-editor-form">';
          html += '<div class="st-editor-header"><span class="st-editor-title">' + escHtml(currentSource) + '</span>';
          html += '<span class="st-editor-status">' + (schema ? 'Schema-driven' : 'Raw JSON') + '</span></div>';

          if (Array.isArray(currentData)) {
            // Array at root level
            html += renderArrayField('', currentData, schema && schema.items ? schema.items : null);
          } else if (schema && schema.properties) {
            // Object with known schema
            html += renderObjectFields('', currentData, schema);
          } else {
            // No schema — raw JSON editor
            html += '<textarea class="st-raw-editor" data-path="">' + escHtml(JSON.stringify(currentData, null, 2)) + '</textarea>';
          }

          html += '</div>';
          pane.innerHTML = html;
          bindEditorEvents();
        }

        function renderObjectFields(basePath, data, schema) {
          var html = '';
          var props = schema && schema.properties ? schema.properties : {};
          var keys = Object.keys(props);

          // Also include keys from data not in schema
          if (data && typeof data === 'object' && !Array.isArray(data)) {
            var dataKeys = Object.keys(data);
            for (var i = 0; i < dataKeys.length; i++) {
              if (keys.indexOf(dataKeys[i]) === -1) keys.push(dataKeys[i]);
            }
          }

          for (var k = 0; k < keys.length; k++) {
            var key = keys[k];
            var fieldPath = basePath ? basePath + '.' + key : key;
            var fieldSchema = props[key] || {};
            var value = data ? data[key] : undefined;
            html += renderField(fieldPath, key, value, fieldSchema, schema);
          }

          // Optional fields not yet set
          if (schema && schema.properties) {
            var required = schema.required || [];
            var propKeys = Object.keys(schema.properties);
            for (var r = 0; r < propKeys.length; r++) {
              var pk = propKeys[r];
              if (data && data[pk] !== undefined) continue;
              if (required.indexOf(pk) !== -1) continue;
              // Already rendered above
            }
          }

          return html;
        }

        function renderField(path, label, value, fieldSchema, parentSchema) {
          var type = fieldSchema.type || inferType(value);
          var format = fieldSchema.format || '';

          if (type === 'array') {
            return renderArrayField(path, value || [], fieldSchema.items || null);
          }

          if (type === 'object') {
            var collapsed = true;
            return '<div class="st-object-section" data-path="' + escHtml(path) + '">' +
              '<div class="st-object-toggle" data-collapsed="true">\u25B8 ' + escHtml(label) + '</div>' +
              '<div class="st-object-body" style="display:none;">' +
              renderObjectFields(path, value || {}, fieldSchema) +
              '</div></div>';
          }

          var html = '<div class="st-field">';
          html += '<label class="st-field-label">' + escHtml(label) + '</label>';

          if (type === 'boolean') {
            html += '<input type="checkbox" class="st-field-input" data-path="' + escHtml(path) + '" data-type="boolean"' + (value ? ' checked' : '') + '>';
          } else if (type === 'number' || type === 'integer') {
            html += '<input type="number" class="st-field-input" data-path="' + escHtml(path) + '" data-type="number" value="' + escHtml(String(value != null ? value : '')) + '">';
          } else if (format === 'uri' || format === 'url') {
            html += '<input type="url" class="st-field-input st-url-input" data-path="' + escHtml(path) + '" data-type="string" value="' + escHtml(String(value || '')) + '" placeholder="https://">';
          } else {
            html += '<input type="text" class="st-field-input" data-path="' + escHtml(path) + '" data-type="string" value="' + escHtml(String(value != null ? value : '')) + '">';
          }

          // "Add field" for optional fields with no value
          if (value === undefined && parentSchema && parentSchema.properties && parentSchema.properties[label]) {
            html += '<button class="st-add-field-btn" data-path="' + escHtml(path) + '">+ Add ' + escHtml(label) + '</button>';
          }

          html += '</div>';
          return html;
        }

        function renderArrayField(path, arr, itemSchema) {
          var arrLen = arr ? arr.length : 0;
          var visibleKey = path || '__root';
          var visible = arrayVisibleCount[visibleKey] || 50;
          var showCount = Math.min(arrLen, visible);

          var html = '<div class="st-array-section" data-array-path="' + escHtml(path) + '">';
          html += '<div class="st-array-header">';
          html += '<span class="st-array-title">' + escHtml(path || 'Items') + '</span>';
          html += '<span class="st-array-count">' + arrLen + ' items</span>';
          html += '<div class="st-array-actions">';
          html += '<button class="st-array-btn st-btn-add" data-action="add" data-array-path="' + escHtml(path) + '">+ Add</button>';
          html += '</div></div>';
          html += '<div class="st-array-items">';

          for (var i = 0; i < showCount; i++) {
            html += '<div class="st-array-item" data-index="' + i + '">';
            html += '<div class="st-array-item-header">';
            html += '<span class="st-array-item-idx">#' + i + '</span>';
            html += '<button class="st-array-btn st-btn-remove" data-action="remove" data-array-path="' + escHtml(path) + '" data-index="' + i + '">\u00D7</button>';
            html += '</div>';

            var itemPath = path ? path + '[' + i + ']' : '[' + i + ']';
            var item = arr[i];

            if (item && typeof item === 'object' && !Array.isArray(item)) {
              html += renderObjectFields(itemPath, item, itemSchema || {});
            } else if (Array.isArray(item)) {
              html += renderArrayField(itemPath, item, itemSchema && itemSchema.items ? itemSchema.items : null);
            } else {
              html += '<input type="text" class="st-field-input" data-path="' + escHtml(itemPath) + '" data-type="' + inferType(item) + '" value="' + escHtml(String(item != null ? item : '')) + '">';
            }

            html += '</div>';
          }

          html += '</div>';

          if (arrLen > visible) {
            html += '<button class="st-load-more" data-array-key="' + escHtml(visibleKey) + '">Load more (' + (arrLen - visible) + ' remaining)</button>';
          }

          html += '</div>';
          return html;
        }

        function bindEditorEvents() {
          // Field change handlers (debounced)
          var inputs = pane.querySelectorAll('.st-field-input');
          for (var i = 0; i < inputs.length; i++) {
            var input = inputs[i];
            var evtName = input.type === 'checkbox' ? 'change' : 'input';
            input.addEventListener(evtName, (function(el) {
              return function() { debouncedSave(el); };
            })(input));
          }

          // Raw JSON editor
          var rawEditor = pane.querySelector('.st-raw-editor');
          if (rawEditor) {
            rawEditor.addEventListener('input', function() {
              debouncedSaveRaw(rawEditor);
            });
          }

          // Object section toggles
          var toggles = pane.querySelectorAll('.st-object-toggle');
          for (var t = 0; t < toggles.length; t++) {
            toggles[t].addEventListener('click', function(e) {
              var toggle = e.currentTarget;
              var body = toggle.nextElementSibling;
              var collapsed = toggle.getAttribute('data-collapsed') === 'true';
              toggle.setAttribute('data-collapsed', collapsed ? 'false' : 'true');
              toggle.textContent = (collapsed ? '\u25BE ' : '\u25B8 ') + toggle.textContent.substring(2);
              body.style.display = collapsed ? '' : 'none';
            });
          }

          // Array add buttons
          var addBtns = pane.querySelectorAll('[data-action="add"]');
          for (var a = 0; a < addBtns.length; a++) {
            addBtns[a].addEventListener('click', function(e) {
              var arrayPath = e.currentTarget.getAttribute('data-array-path');
              handleArrayAdd(arrayPath);
            });
          }

          // Array remove buttons
          var removeBtns = pane.querySelectorAll('[data-action="remove"]');
          for (var r = 0; r < removeBtns.length; r++) {
            removeBtns[r].addEventListener('click', function(e) {
              var arrayPath = e.currentTarget.getAttribute('data-array-path');
              var index = parseInt(e.currentTarget.getAttribute('data-index'), 10);
              handleArrayRemove(arrayPath, index);
            });
          }

          // Load more buttons
          var loadMoreBtns = pane.querySelectorAll('.st-load-more');
          for (var l = 0; l < loadMoreBtns.length; l++) {
            loadMoreBtns[l].addEventListener('click', function(e) {
              var key = e.currentTarget.getAttribute('data-array-key');
              arrayVisibleCount[key] = (arrayVisibleCount[key] || 50) + 50;
              renderEditor();
            });
          }
        }

        function debouncedSave(inputEl) {
          var path = inputEl.getAttribute('data-path');
          var type = inputEl.getAttribute('data-type');
          if (debounceTimers[path]) clearTimeout(debounceTimers[path]);
          debounceTimers[path] = setTimeout(function() {
            var val;
            if (type === 'boolean') {
              val = inputEl.checked;
            } else if (type === 'number') {
              val = inputEl.value === '' ? null : Number(inputEl.value);
            } else {
              val = inputEl.value;
            }
            sendEdit(path, val, inputEl);
          }, 300);
        }

        function debouncedSaveRaw(textarea) {
          if (debounceTimers['__raw']) clearTimeout(debounceTimers['__raw']);
          debounceTimers['__raw'] = setTimeout(function() {
            try {
              var parsed = JSON.parse(textarea.value);
              sendEdit('', parsed, textarea);
            } catch (e) {
              flashField(textarea, false);
            }
          }, 300);
        }

        function sendEdit(path, value, inputEl) {
          if (!window.__stDevWs || !currentFile) return;
          var opId = 'op_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
          var msg = {
            type: 'EditJson',
            file: currentFile,
            path: path,
            value: value,
            op_id: opId
          };
          window.__stDevWs.send(msg);

          if (window.__stDevWs.onAck) {
            window.__stDevWs.onAck(opId, function() {
              flashField(inputEl, true);
              // Update local data so re-render is correct
              setNestedValue(currentData, path, value);
            });
          }
          if (window.__stDevWs.onReject) {
            window.__stDevWs.onReject(opId, function() {
              flashField(inputEl, false);
            });
          }
        }

        function handleArrayAdd(arrayPath) {
          if (!window.__stDevSave || !currentFile) return;
          var arr = getNestedValue(currentData, arrayPath);
          var newItem = guessDefaultItem(arrayPath);
          var index = arr ? arr.length : 0;
          window.__stDevSave.saveArrayInsert(currentFile, arrayPath, newItem, index);
          // Optimistic update
          if (arr) {
            arr.push(newItem);
          }
          renderEditor();
        }

        function handleArrayRemove(arrayPath, index) {
          if (!window.__stDevSave || !currentFile) return;
          window.__stDevSave.saveArrayDelete(currentFile, arrayPath, index);
          // Optimistic update
          var arr = getNestedValue(currentData, arrayPath);
          if (arr) {
            arr.splice(index, 1);
          }
          renderEditor();
        }

        function guessDefaultItem(arrayPath) {
          // Try to infer from schema (resolve type via dataSourceMap)
          var si = dataSourceMap ? dataSourceMap[currentSource] : null;
          var tn = si ? si.type : null;
          if (typeSchemas && tn && typeSchemas[tn]) {
            var schema = typeSchemas[tn];
            var itemSchema = resolveArrayItemSchema(schema, arrayPath);
            if (itemSchema) {
              if (itemSchema.type === 'object' && itemSchema.properties) {
                var obj = {};
                var keys = Object.keys(itemSchema.properties);
                for (var i = 0; i < keys.length; i++) {
                  var ps = itemSchema.properties[keys[i]];
                  obj[keys[i]] = ps.type === 'number' ? 0 : ps.type === 'boolean' ? false : '';
                }
                return obj;
              }
              if (itemSchema.type === 'number') return 0;
              if (itemSchema.type === 'boolean') return false;
            }
          }
          return '';
        }

        function resolveArrayItemSchema(schema, path) {
          if (!path) return schema.items || null;
          var parts = path.replace(/\[\d+\]/g, '.items').split('.');
          var cur = schema;
          for (var i = 0; i < parts.length; i++) {
            if (!cur) return null;
            if (parts[i] === 'items') {
              cur = cur.items;
            } else if (cur.properties) {
              cur = cur.properties[parts[i]];
            } else {
              return null;
            }
          }
          return cur && cur.items ? cur.items : null;
        }

        function flashField(el, success) {
          if (!el) return;
          var cls = success ? 'st-save-ok' : 'st-save-err';
          el.classList.add(cls);
          setTimeout(function() { el.classList.remove(cls); }, 800);
        }

        function getNestedValue(obj, path) {
          if (!path) return obj;
          var parts = parsePath(path);
          var cur = obj;
          for (var i = 0; i < parts.length; i++) {
            if (cur == null) return undefined;
            cur = cur[parts[i]];
          }
          return cur;
        }

        function setNestedValue(obj, path, value) {
          if (!path) {
            // Root replacement — update currentData reference
            currentData = value;
            return;
          }
          var parts = parsePath(path);
          var cur = obj;
          for (var i = 0; i < parts.length - 1; i++) {
            if (cur == null) return;
            cur = cur[parts[i]];
          }
          if (cur != null) {
            cur[parts[parts.length - 1]] = value;
          }
        }

        function parsePath(path) {
          // Convert "foo.bar[0].baz" to ["foo", "bar", 0, "baz"]
          var result = [];
          var segments = path.split('.');
          for (var i = 0; i < segments.length; i++) {
            var seg = segments[i];
            var bracketMatch = seg.match(/^([^\[]*)\[(\d+)\]$/);
            if (bracketMatch) {
              if (bracketMatch[1]) result.push(bracketMatch[1]);
              result.push(parseInt(bracketMatch[2], 10));
            } else if (seg) {
              result.push(seg);
            }
          }
          return result;
        }

        function inferType(value) {
          if (value === null || value === undefined) return 'string';
          if (typeof value === 'boolean') return 'boolean';
          if (typeof value === 'number') return 'number';
          if (Array.isArray(value)) return 'array';
          if (typeof value === 'object') return 'object';
          return 'string';
        }

        function escHtml(str) {
          return String(str || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
        }

        // Listen for DataUpdate to re-fetch current source
        if (window.__stDevWs && window.__stDevWs.onMessage) {
          window.__stDevWs.onMessage('DataUpdate', function(msg) {
            if (currentSource && msg && msg.file === currentFile) {
              loadSource(currentSource);
            }
          });
        }

        window.__stDevDataEditor = {
          loadSource: loadSource
        };
      }
      init();

      %cleanup {
        window.__stDevDataEditor = null;
      }
    })();
  }
}
