/// Data Tree Primitive
///
/// Populates the dev-panel's "Data" tab with an interactive,
/// expandable tree view of all @data sources and their contents.
/// Reads from ST._dataRegistry and renders into Shadow DOM.
///
/// Features:
///   - Expandable tree: sources → array items → object fields
///   - Click-to-edit inline values
///   - Auto-refresh via polling (1s interval)
///   - Dark catppuccin-inspired theme
///   - Nested object/array support (max 3 levels)

%primitive data-tree() {
  %emit js {
    (function() {
      // Wait for panel to be ready
      var pane = window.__stDevPanel && window.__stDevPanel.getPane('data');
      if (!pane) return;

      var shadow = window.__stDevPanel.shadow;

      // Inject tree styles into shadow DOM
      var style = document.createElement('style');
      style.textContent = [
        '.st-tree { font-size: 12px; line-height: 1.5; }',
        '.st-tree-source { margin-bottom: 8px; }',
        '.st-tree-header { display: flex; align-items: center; gap: 6px; padding: 4px 6px; border-radius: 4px; cursor: pointer; user-select: none; -webkit-user-select: none; transition: background 0.12s; }',
        '.st-tree-header:hover { background: rgba(137,180,250,0.08); }',
        '.st-tree-arrow { color: #6c7086; font-size: 10px; width: 14px; text-align: center; transition: transform 0.15s, color 0.15s; flex-shrink: 0; }',
        '.st-tree-arrow.expanded { transform: rotate(90deg); }',
        '.st-tree-header:hover .st-tree-arrow { color: #cdd6f4; }',
        '.st-tree-source-name { color: #89b4fa; font-weight: 600; }',
        '.st-tree-count { color: #6c7086; font-size: 11px; margin-left: 4px; }',
        '.st-tree-children { display: none; padding-left: 16px; }',
        '.st-tree-children.visible { display: block; }',
        '.st-tree-item { margin: 1px 0; }',
        '.st-tree-item-header { display: flex; align-items: center; gap: 6px; padding: 2px 6px; border-radius: 3px; cursor: pointer; user-select: none; -webkit-user-select: none; transition: background 0.12s; }',
        '.st-tree-item-header:hover { background: rgba(166,173,200,0.06); }',
        '.st-tree-item-index { color: #6c7086; font-size: 11px; min-width: 20px; }',
        '.st-tree-item-preview { color: #a6adc8; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 200px; }',
        '.st-tree-fields { display: none; padding-left: 16px; }',
        '.st-tree-fields.visible { display: block; }',
        '.st-tree-field { display: flex; align-items: baseline; gap: 6px; padding: 1px 6px; border-radius: 3px; min-height: 22px; }',
        '.st-tree-field:hover { background: rgba(166,173,200,0.04); }',
        '.st-tree-field-name { color: #a6adc8; flex-shrink: 0; }',
        '.st-tree-field-sep { color: #45475a; flex-shrink: 0; }',
        '.st-tree-field-value { cursor: text; padding: 0 4px; border-radius: 3px; border: 1px solid transparent; outline: none; min-width: 30px; transition: border-color 0.15s, background 0.15s; word-break: break-all; }',
        '.st-tree-field-value:hover { border-color: #313244; }',
        '.st-tree-field-value:focus, .st-tree-field-value[contenteditable="true"]:focus { border-color: #89b4fa; background: rgba(137,180,250,0.06); }',
        '.st-tree-field-value.str { color: #f9e2af; }',
        '.st-tree-field-value.num { color: #a6e3a1; }',
        '.st-tree-field-value.bool { color: #f38ba8; }',
        '.st-tree-field-value.null { color: #585b70; font-style: italic; }',
        '.st-tree-field-value.obj { color: #6c7086; cursor: pointer; }',
        '.st-tree-nested { padding-left: 16px; display: none; }',
        '.st-tree-nested.visible { display: block; }',
        '.st-tree-empty { color: #585b70; font-size: 12px; text-align: center; padding: 40px 16px; }'
      ].join('\n');
      shadow.appendChild(style);

      // State tracking for expanded nodes
      var expandedSources = {};
      var expandedItems = {};
      var expandedNested = {};
      var lastDataJson = '';

      function getValueType(v) {
        if (v === null || v === undefined) return 'null';
        if (typeof v === 'boolean') return 'bool';
        if (typeof v === 'number') return 'num';
        if (Array.isArray(v)) return 'arr';
        if (typeof v === 'object') return 'obj';
        return 'str';
      }

      function formatValue(v) {
        var t = getValueType(v);
        if (t === 'null') return 'null';
        if (t === 'bool') return v ? 'true' : 'false';
        if (t === 'num') return String(v);
        if (t === 'arr') return '[Array(' + v.length + ')]';
        if (t === 'obj') return '{' + Object.keys(v).length + ' fields}';
        return String(v);
      }

      function getPreview(item) {
        if (item === null || item === undefined) return 'null';
        if (typeof item !== 'object') return String(item);
        var keys = Object.keys(item);
        if (keys.length === 0) return '{}';
        // Show first identifying field
        var idFields = ['name', 'title', 'id', 'label', 'key'];
        for (var i = 0; i < idFields.length; i++) {
          if (item[idFields[i]] !== undefined) {
            return String(item[idFields[i]]);
          }
        }
        return '{' + keys.length + ' fields}';
      }

      function parseValue(str, oldValue) {
        var trimmed = str.trim();
        if (trimmed === 'null') return null;
        if (trimmed === 'true') return true;
        if (trimmed === 'false') return false;
        var num = Number(trimmed);
        if (trimmed !== '' && !isNaN(num)) return num;
        return trimmed;
      }

      function emitEdit(sourceName, index, field, oldValue, newValue) {
        var editEvent = {
          source: sourceName,
          index: index,
          field: field,
          oldValue: oldValue,
          newValue: newValue
        };
        if (window.__stDevSave && window.__stDevSave.save) {
          window.__stDevSave.save(editEvent);
        } else if (window.__stDevWs) {
          window.__stDevWs.send({
            type: 'EditJson',
            file: sourceName,
            path: '[' + index + '].' + field,
            value: newValue
          });
        }
      }

      function makeEditable(el, sourceName, index, field, value) {
        el.setAttribute('contenteditable', 'true');
        el.setAttribute('spellcheck', 'false');
        var oldText = el.textContent;

        function commit() {
          el.removeAttribute('contenteditable');
          var newText = el.textContent;
          if (newText !== oldText) {
            var newVal = parseValue(newText, value);
            emitEdit(sourceName, index, field, value, newVal);
            // Update the type class
            el.className = 'st-tree-field-value ' + getValueType(newVal);
          }
          el.removeEventListener('blur', commit);
          el.removeEventListener('keydown', onKey);
        }

        function onKey(e) {
          if (e.key === 'Enter') {
            e.preventDefault();
            el.blur();
          } else if (e.key === 'Escape') {
            el.textContent = oldText;
            el.blur();
          }
        }

        el.addEventListener('blur', commit);
        el.addEventListener('keydown', onKey);

        // Select all text
        var range = document.createRange();
        range.selectNodeContents(el);
        var sel = shadow.getSelection ? shadow.getSelection() : window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);
      }

      function renderFields(container, obj, sourceName, index, prefix, depth) {
        if (depth > 3) return;
        var keys = Object.keys(obj);
        for (var k = 0; k < keys.length; k++) {
          var field = keys[k];
          var val = obj[field];
          var vType = getValueType(val);
          var fullPath = prefix ? prefix + '.' + field : field;

          var row = document.createElement('div');
          row.className = 'st-tree-field';

          if (vType === 'obj' || vType === 'arr') {
            // Nested expandable
            var nestedKey = sourceName + ':' + index + ':' + fullPath;
            var hdr = document.createElement('div');
            hdr.className = 'st-tree-item-header';
            hdr.innerHTML = '<span class="st-tree-arrow' + (expandedNested[nestedKey] ? ' expanded' : '') + '">\u25B6</span>' +
              '<span class="st-tree-field-name">' + field + '</span>' +
              '<span class="st-tree-field-sep">:</span>' +
              '<span class="st-tree-field-value ' + vType + '">' + formatValue(val) + '</span>';

            var nested = document.createElement('div');
            nested.className = 'st-tree-nested' + (expandedNested[nestedKey] ? ' visible' : '');

            (function(nk, hd, ns, v, vt) {
              hd.addEventListener('click', function() {
                expandedNested[nk] = !expandedNested[nk];
                var arrow = hd.querySelector('.st-tree-arrow');
                if (expandedNested[nk]) {
                  arrow.classList.add('expanded');
                  ns.classList.add('visible');
                } else {
                  arrow.classList.remove('expanded');
                  ns.classList.remove('visible');
                }
              });

              if (expandedNested[nk]) {
                if (vt === 'arr') {
                  for (var ai = 0; ai < v.length; ai++) {
                    if (typeof v[ai] === 'object' && v[ai] !== null) {
                      renderFields(ns, v[ai], sourceName, index, fullPath + '[' + ai + ']', depth + 1);
                    } else {
                      var aRow = document.createElement('div');
                      aRow.className = 'st-tree-field';
                      aRow.innerHTML = '<span class="st-tree-field-name">[' + ai + ']</span>' +
                        '<span class="st-tree-field-sep">:</span>' +
                        '<span class="st-tree-field-value ' + getValueType(v[ai]) + '">' + formatValue(v[ai]) + '</span>';
                      ns.appendChild(aRow);
                    }
                  }
                } else {
                  renderFields(ns, v, sourceName, index, fullPath, depth + 1);
                }
              }
            })(nestedKey, hdr, nested, val, vType);

            container.appendChild(hdr);
            container.appendChild(nested);
          } else {
            // Leaf value — editable
            var nameSpan = document.createElement('span');
            nameSpan.className = 'st-tree-field-name';
            nameSpan.textContent = field;

            var sepSpan = document.createElement('span');
            sepSpan.className = 'st-tree-field-sep';
            sepSpan.textContent = ':';

            var valSpan = document.createElement('span');
            valSpan.className = 'st-tree-field-value ' + vType;
            valSpan.textContent = formatValue(val);

            (function(vs, sn, idx, f, v) {
              vs.addEventListener('click', function(e) {
                e.stopPropagation();
                if (vs.getAttribute('contenteditable') === 'true') return;
                makeEditable(vs, sn, idx, f, v);
              });
            })(valSpan, sourceName, index, fullPath, val);

            row.appendChild(nameSpan);
            row.appendChild(sepSpan);
            row.appendChild(valSpan);
            container.appendChild(row);
          }
        }
      }

      function renderTree() {
        var registry = (typeof ST !== 'undefined' && ST._dataRegistry) ? ST._dataRegistry : null;

        // Merge locale sources from window.__st_locales
        if (typeof window.__st_locales === 'object' && window.__st_locales !== null) {
          if (!registry) registry = new Map();
          var locales = window.__st_locales;
          var localeKeys = Object.keys(locales);
          for (var li = 0; li < localeKeys.length; li++) {
            var code = localeKeys[li];
            var dict = locales[code];
            var localeName = '@locale "' + code + '"';
            if (!registry.has(localeName)) {
              registry.set(localeName, { data: dict || {} });
            }
          }
        }
        if (!registry || registry.size === 0) {
          // Check for dev inspect API fallback
          if (window.__stDevInspect && window.__stDevInspect.getDataSources) {
            registry = window.__stDevInspect.getDataSources();
          }
        }

        // Build a JSON snapshot for change detection
        var snapshot = {};
        if (registry && registry.forEach) {
          registry.forEach(function(entry, name) {
            snapshot[name] = entry.data;
          });
        }
        var snapJson = JSON.stringify(snapshot);
        if (snapJson === lastDataJson) return;
        lastDataJson = snapJson;

        // Clear pane
        pane.innerHTML = '';

        if (!registry || registry.size === 0) {
          var empty = document.createElement('p');
          empty.className = 'st-tree-empty';
          empty.textContent = 'No data sources registered';
          pane.appendChild(empty);
          return;
        }

        var tree = document.createElement('div');
        tree.className = 'st-tree';

        registry.forEach(function(entry, name) {
          var data = entry.data;
          var isArray = Array.isArray(data);
          var count = isArray ? data.length : (data ? 1 : 0);

          var source = document.createElement('div');
          source.className = 'st-tree-source';

          // Source header
          var header = document.createElement('div');
          header.className = 'st-tree-header';
          header.innerHTML = '<span class="st-tree-arrow' + (expandedSources[name] ? ' expanded' : '') + '">\u25B6</span>' +
            '<span class="st-tree-source-name">' + name + '</span>' +
            '<span class="st-tree-count">(' + count + (isArray ? ' items' : '') + ')</span>';

          var children = document.createElement('div');
          children.className = 'st-tree-children' + (expandedSources[name] ? ' visible' : '');

          // Toggle expand
          (function(n, hd, ch) {
            hd.addEventListener('click', function() {
              expandedSources[n] = !expandedSources[n];
              var arrow = hd.querySelector('.st-tree-arrow');
              if (expandedSources[n]) {
                arrow.classList.add('expanded');
                ch.classList.add('visible');
                if (ch.childNodes.length === 0) {
                  renderSourceChildren(ch, n, entry);
                }
              } else {
                arrow.classList.remove('expanded');
                ch.classList.remove('visible');
              }
            });
          })(name, header, children);

          // If already expanded, render children
          if (expandedSources[name]) {
            renderSourceChildren(children, name, entry);
          }

          source.appendChild(header);
          source.appendChild(children);
          tree.appendChild(source);
        });

        pane.appendChild(tree);
      }

      function renderSourceChildren(container, sourceName, entry) {
        container.innerHTML = '';
        var data = entry.data;
        if (!data) {
          var empty = document.createElement('div');
          empty.className = 'st-tree-field';
          empty.innerHTML = '<span class="st-tree-field-value null">No data loaded</span>';
          container.appendChild(empty);
          return;
        }

        if (!Array.isArray(data)) {
          // Single object
          if (typeof data === 'object') {
            renderFields(container, data, sourceName, 0, '', 0);
          } else {
            var leaf = document.createElement('div');
            leaf.className = 'st-tree-field';
            leaf.innerHTML = '<span class="st-tree-field-value ' + getValueType(data) + '">' + formatValue(data) + '</span>';
            container.appendChild(leaf);
          }
          return;
        }

        // Array items
        for (var i = 0; i < data.length; i++) {
          var item = data[i];
          var itemKey = sourceName + ':' + i;

          var itemEl = document.createElement('div');
          itemEl.className = 'st-tree-item';

          var itemHeader = document.createElement('div');
          itemHeader.className = 'st-tree-item-header';
          itemHeader.innerHTML = '<span class="st-tree-arrow' + (expandedItems[itemKey] ? ' expanded' : '') + '">\u25B6</span>' +
            '<span class="st-tree-item-index">[' + i + ']</span>' +
            '<span class="st-tree-item-preview">' + getPreview(item) + '</span>';

          var fields = document.createElement('div');
          fields.className = 'st-tree-fields' + (expandedItems[itemKey] ? ' visible' : '');

          (function(ik, ih, fl, itm, idx) {
            ih.addEventListener('click', function() {
              expandedItems[ik] = !expandedItems[ik];
              var arrow = ih.querySelector('.st-tree-arrow');
              if (expandedItems[ik]) {
                arrow.classList.add('expanded');
                fl.classList.add('visible');
                if (fl.childNodes.length === 0 && typeof itm === 'object' && itm !== null) {
                  renderFields(fl, itm, sourceName, idx, '', 0);
                }
              } else {
                arrow.classList.remove('expanded');
                fl.classList.remove('visible');
              }
            });
          })(itemKey, itemHeader, fields, item, i);

          // If already expanded, render fields
          if (expandedItems[itemKey] && typeof item === 'object' && item !== null) {
            renderFields(fields, item, sourceName, i, '', 0);
          }

          itemEl.appendChild(itemHeader);
          itemEl.appendChild(fields);
          container.appendChild(itemEl);
        }
      }

      // Initial render
      renderTree();

      // Poll for data changes
      var refreshInterval = setInterval(renderTree, 1000);

      %cleanup {
        clearInterval(refreshInterval);
        if (style.parentNode) style.parentNode.removeChild(style);
        pane.innerHTML = '<p class="st-placeholder">Data sources will appear here</p>';
      }
    })();
  }
}
