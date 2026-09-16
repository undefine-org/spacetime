/// Animation Properties Primitive
///
/// Adds a "⚡ Animation Properties" section to the Timelines tab,
/// showing editable timeline properties (duration, easing, delay).
/// Edits send EditAst messages via WebSocket for live updating.
///
/// Features:
///   - Lists all registered timelines with their properties
///   - Inline-editable values (duration, easing, delay)
///   - Sends EditAst patches via window.__stDevWs
///   - Auto-refresh via polling (1s interval)
///   - Dark catppuccin theme matching panel

%primitive st-properties() {
  %emit js {
    (function() {
      // Wait for panel to be ready
      var panel = window.__stDevPanel;
      if (!panel) return;

      var timelinesPane = panel.getPane('timelines');
      if (!timelinesPane) return;

      var shadow = panel.shadow;

      // Inject styles into shadow DOM
      var style = document.createElement('style');
      style.textContent = [
        '.st-props { margin-top: 12px; border-top: 1px solid #313244; padding-top: 10px; font-size: 12px; }',
        '.st-props-title { display: flex; align-items: center; gap: 6px; padding: 4px 0 8px; color: #cdd6f4; font-weight: 600; font-size: 12px; user-select: none; -webkit-user-select: none; }',
        '.st-props-title-icon { font-size: 13px; }',
        '.st-props-tl { margin-bottom: 10px; border: 1px solid #313244; border-radius: 6px; overflow: hidden; }',
        '.st-props-tl-header { display: flex; align-items: center; gap: 6px; padding: 6px 8px; background: #181825; cursor: pointer; user-select: none; -webkit-user-select: none; transition: background 0.12s; }',
        '.st-props-tl-header:hover { background: rgba(137,180,250,0.06); }',
        '.st-props-tl-arrow { color: #6c7086; font-size: 10px; width: 14px; text-align: center; transition: transform 0.15s; flex-shrink: 0; }',
        '.st-props-tl-arrow.expanded { transform: rotate(90deg); }',
        '.st-props-tl-name { color: #89b4fa; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }',
        '.st-props-tl-type { font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 3px; text-transform: uppercase; letter-spacing: 0.04em; flex-shrink: 0; }',
        '.st-props-tl-type.scroll { background: rgba(137,180,250,0.15); color: #89b4fa; }',
        '.st-props-tl-type.load { background: rgba(166,227,161,0.15); color: #a6e3a1; }',
        '.st-props-tl-type.click { background: rgba(249,226,175,0.15); color: #f9e2af; }',
        '.st-props-tl-type.hover { background: rgba(203,166,247,0.15); color: #cba6f7; }',
        '.st-props-tl-type.unknown { background: rgba(108,112,134,0.15); color: #6c7086; }',
        '.st-props-tl-body { display: none; padding: 6px 8px; }',
        '.st-props-tl-body.visible { display: block; }',
        '.st-props-row { display: flex; align-items: baseline; gap: 6px; padding: 3px 0; min-height: 22px; }',
        '.st-props-label { color: #a6adc8; flex-shrink: 0; min-width: 64px; font-size: 11px; }',
        '.st-props-sep { color: #45475a; flex-shrink: 0; }',
        '.st-props-val { cursor: text; padding: 1px 6px; border-radius: 3px; border: 1px solid transparent; outline: none; min-width: 40px; transition: border-color 0.15s, background 0.15s; word-break: break-all; font-family: inherit; font-size: 12px; }',
        '.st-props-val:hover { border-color: #313244; }',
        '.st-props-val:focus { border-color: #89b4fa; background: rgba(137,180,250,0.06); }',
        '.st-props-val.editable { color: #a6e3a1; }',
        '.st-props-val.readonly { color: #6c7086; cursor: default; }',
        '.st-props-val.str { color: #f9e2af; }',
        '.st-props-val.num { color: #a6e3a1; }',
        '.st-props-scope { color: #cba6f7; font-size: 11px; font-family: inherit; }',
        '.st-props-empty { color: #585b70; font-size: 12px; text-align: center; padding: 20px 16px; }',
        '.st-props-status { font-size: 10px; padding: 2px 6px; border-radius: 3px; margin-left: auto; transition: opacity 0.3s; }',
        '.st-props-status.ok { color: #a6e3a1; background: rgba(166,227,161,0.1); }',
        '.st-props-status.err { color: #f38ba8; background: rgba(243,139,168,0.1); }',
        '.st-props-range { color: #6c7086; font-size: 11px; }'
      ].join('\n');
      shadow.appendChild(style);

      // Track expanded timelines
      var expandedTl = {};
      var lastJson = '';
      var opCounter = 0;

      // Editable property definitions
      // Properties from the debug runtime timeline objects
      var editableProps = [
        { key: 'duration', label: 'duration', type: 'str', defaultVal: '—' },
        { key: 'easing', label: 'easing', type: 'str', defaultVal: '—' },
        { key: 'delay', label: 'delay', type: 'str', defaultVal: '—' }
      ];

      // Read-only property definitions
      var readonlyProps = [
        { key: 'scope', label: 'scope', format: function(v) { return v || '—'; } },
        { key: 'progress', label: 'progress', format: function(v) { return ((v || 0) * 100).toFixed(1) + '%'; } },
        { key: 'range', label: 'range', format: function(v) { return Array.isArray(v) ? v[0] + ' → ' + v[1] : '—'; } }
      ];

      function esc(str) {
        if (!str) return '';
        return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }

      function typeBadge(type) {
        var t = (type || 'unknown').toLowerCase();
        if (t === 'scroll' || t === 'load' || t === 'click' || t === 'hover') return t;
        return 'unknown';
      }

      function generateOpId() {
        opCounter++;
        return 'prop_' + Date.now() + '_' + opCounter;
      }

      function sendEditAst(tl, property, newValue) {
        var ws = window.__stDevWs;
        if (!ws || !ws.send) return;

        var selector = (tl.scope || '') + ' @' + (tl.type || 'unknown');
        var opId = generateOpId();
        var patch = {};
        patch[property] = newValue;

        var msg = {
          type: 'EditAst',
          file: tl._stFile || '',
          selector: selector.trim(),
          patch: patch,
          op_id: opId
        };

        ws.send(msg);

        // Return opId for status tracking
        return opId;
      }

      function makeEditable(el, tl, propKey) {
        if (el.getAttribute('contenteditable') === 'true') return;
        el.setAttribute('contenteditable', 'true');
        el.setAttribute('spellcheck', 'false');
        var oldText = el.textContent;

        function commit() {
          el.removeAttribute('contenteditable');
          var newText = el.textContent.trim();
          if (newText !== oldText && newText !== '' && newText !== '—') {
            var opId = sendEditAst(tl, propKey, newText);
            // Show brief status
            showStatus(el.parentNode, opId);
          } else {
            el.textContent = oldText;
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

      function showStatus(row, opId) {
        if (!row) return;
        var existing = row.querySelector('.st-props-status');
        if (existing) existing.remove();

        var badge = document.createElement('span');
        badge.className = 'st-props-status ok';
        badge.textContent = 'sent';
        row.appendChild(badge);

        // Listen for ack/reject
        if (window.__stDevWs && window.__stDevWs.onMessage) {
          var handler = function(msg) {
            if (msg.op_id === opId) {
              if (msg.type === 'Ack') {
                badge.className = 'st-props-status ok';
                badge.textContent = '✓';
              } else if (msg.type === 'Reject') {
                badge.className = 'st-props-status err';
                badge.textContent = '✗ ' + (msg.reason || 'failed');
              }
              window.__stDevWs.offMessage && window.__stDevWs.offMessage(handler);
            }
          };
          window.__stDevWs.onMessage(handler);
        }

        // Auto-remove after 3s
        setTimeout(function() {
          badge.style.opacity = '0';
          setTimeout(function() { if (badge.parentNode) badge.remove(); }, 300);
        }, 3000);
      }

      function renderProperties() {
        var inspect = window.__stDevInspect;
        if (!inspect) return;

        var timelines = inspect.getTimelines();
        var json = JSON.stringify(timelines);
        if (json === lastJson) {
          // Still ensure our container exists (timelines-errors may have wiped it)
          var existing = timelinesPane.querySelector('.st-props');
          if (existing) return;
        }
        lastJson = json;

        // Remove old container if present
        var old = timelinesPane.querySelector('.st-props');
        if (old) old.remove();

        // Create properties section
        var container = document.createElement('div');
        container.className = 'st-props';

        // Section title
        var title = document.createElement('div');
        title.className = 'st-props-title';
        title.innerHTML = '<span class="st-props-title-icon">\u26A1</span> Animation Properties';
        container.appendChild(title);

        if (!timelines || timelines.length === 0) {
          var empty = document.createElement('p');
          empty.className = 'st-props-empty';
          empty.textContent = 'No timelines to inspect';
          container.appendChild(empty);
          timelinesPane.appendChild(container);
          return;
        }

        for (var i = 0; i < timelines.length; i++) {
          var tl = timelines[i];
          var tlKey = tl.id || ('tl-' + i);
          var badge = typeBadge(tl.type);

          var tlEl = document.createElement('div');
          tlEl.className = 'st-props-tl';

          // Header
          var header = document.createElement('div');
          header.className = 'st-props-tl-header';
          header.innerHTML = '<span class="st-props-tl-arrow' + (expandedTl[tlKey] ? ' expanded' : '') + '">\u25B6</span>' +
            '<span class="st-props-tl-name">' + esc(tl.id) + '</span>' +
            '<span class="st-props-tl-type ' + badge + '">' + esc(tl.type || 'unknown') + '</span>';

          // Body (collapsible)
          var body = document.createElement('div');
          body.className = 'st-props-tl-body' + (expandedTl[tlKey] ? ' visible' : '');

          // Toggle handler
          (function(k, hd, bd) {
            hd.addEventListener('click', function() {
              expandedTl[k] = !expandedTl[k];
              var arrow = hd.querySelector('.st-props-tl-arrow');
              if (expandedTl[k]) {
                arrow.classList.add('expanded');
                bd.classList.add('visible');
              } else {
                arrow.classList.remove('expanded');
                bd.classList.remove('visible');
              }
            });
          })(tlKey, header, body);

          if (expandedTl[tlKey]) {
            renderTimelineBody(body, tl);
          }

          tlEl.appendChild(header);
          tlEl.appendChild(body);
          container.appendChild(tlEl);
        }

        timelinesPane.appendChild(container);
      }

      function renderTimelineBody(body, tl) {
        body.innerHTML = '';

        // Read-only properties
        for (var r = 0; r < readonlyProps.length; r++) {
          var rp = readonlyProps[r];
          var row = document.createElement('div');
          row.className = 'st-props-row';

          var label = document.createElement('span');
          label.className = 'st-props-label';
          label.textContent = rp.label;

          var sep = document.createElement('span');
          sep.className = 'st-props-sep';
          sep.textContent = ':';

          var val = document.createElement('span');
          if (rp.key === 'scope') {
            val.className = 'st-props-val readonly st-props-scope';
          } else {
            val.className = 'st-props-val readonly';
          }
          val.textContent = rp.format(tl[rp.key]);

          row.appendChild(label);
          row.appendChild(sep);
          row.appendChild(val);
          body.appendChild(row);
        }

        // Editable properties
        for (var e = 0; e < editableProps.length; e++) {
          var ep = editableProps[e];
          var eRow = document.createElement('div');
          eRow.className = 'st-props-row';

          var eLabel = document.createElement('span');
          eLabel.className = 'st-props-label';
          eLabel.textContent = ep.label;

          var eSep = document.createElement('span');
          eSep.className = 'st-props-sep';
          eSep.textContent = ':';

          var eVal = document.createElement('span');
          eVal.className = 'st-props-val editable ' + ep.type;
          eVal.textContent = tl[ep.key] || ep.defaultVal;

          // Click-to-edit
          (function(valEl, timeline, propKey) {
            valEl.addEventListener('click', function(ev) {
              ev.stopPropagation();
              makeEditable(valEl, timeline, propKey);
            });
          })(eVal, tl, ep.key);

          eRow.appendChild(eLabel);
          eRow.appendChild(eSep);
          eRow.appendChild(eVal);
          body.appendChild(eRow);
        }
      }

      // Initial render
      renderProperties();

      // Poll for changes (1s interval)
      var refreshInterval = setInterval(renderProperties, 1000);

      %cleanup {
        clearInterval(refreshInterval);
        if (style.parentNode) style.parentNode.removeChild(style);
        var old = timelinesPane && timelinesPane.querySelector('.st-props');
        if (old) old.remove();
      }
    })();
  }
}
