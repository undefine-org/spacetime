/// Content Tree Primitive
///
/// Adds a "Content" tab to the dev panel dashboard showing site structure:
///   - Pages: HTML files with routes
///   - Data Sources: data/*.json files (clickable to open in editor)
///   - Locales: available locale codes
///
/// Fetches from /__spacetime/dev/site.json and auto-refreshes on DataUpdate.
/// Exposes window.__stDevContentTree = { refresh(), selectedSource }

%primitive dev-content-tree() {
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
        tabBtn.setAttribute('data-tab', 'content');
        tabBtn.textContent = 'Content';
        tabBar.appendChild(tabBtn);

        // Add content pane
        var pane = document.createElement('div');
        pane.className = 'st-pane';
        pane.setAttribute('data-pane', 'content');
        pane.innerHTML = '<p class="st-placeholder">Loading site structure...</p>';
        contentArea.appendChild(pane);

        // Wire tab click
        tabBtn.addEventListener('click', function() {
          window.__stDevPanel.switchTab('content');
        });

        // Inject styles into shadow DOM
        var style = document.createElement('style');
        style.textContent = [
          '.st-content-tree { padding: 8px 0; }',
          '.st-ct-section { margin-bottom: 12px; }',
          '.st-ct-section-header { display: flex; align-items: center; gap: 6px; padding: 4px 8px; color: #cdd6f4; font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; }',
          '.st-ct-item { display: flex; align-items: center; gap: 6px; padding: 3px 8px 3px 20px; font-size: 12px; color: #a6adc8; cursor: pointer; border-radius: 3px; transition: background 0.12s; }',
          '.st-ct-item:hover { background: rgba(137,180,250,0.08); }',
          '.st-ct-item.selected { background: rgba(137,180,250,0.15); color: #89b4fa; }',
          '.st-ct-icon { flex-shrink: 0; width: 14px; text-align: center; font-size: 11px; }',
          '.st-ct-empty { color: #585b70; font-size: 11px; padding: 4px 20px; font-style: italic; }'
        ].join('\n');
        shadow.appendChild(style);

        var selectedSource = null;

        function fetchAndRender() {
          fetch('/__spacetime/dev/site.json')
            .then(function(r) { return r.json(); })
            .then(function(data) { renderTree(data); })
            .catch(function() {
              pane.innerHTML = '<p class="st-placeholder">Failed to load site structure</p>';
            });
        }

        function renderTree(data) {
          var html = '<div class="st-content-tree">';

          // Pages
          html += '<div class="st-ct-section"><div class="st-ct-section-header">\u25B8 Pages</div>';
          var pages = data.pages || [];
          if (pages.length > 0) {
            for (var i = 0; i < pages.length; i++) {
              html += '<div class="st-ct-item" data-type="page"><span class="st-ct-icon">\u2630</span>' + escHtml(pages[i]) + '</div>';
            }
          } else {
            html += '<div class="st-ct-empty">No pages found</div>';
          }
          html += '</div>';

          // Data Sources
          html += '<div class="st-ct-section"><div class="st-ct-section-header">\u25B8 Data Sources</div>';
          var sources = data.dataSources || [];
          if (sources.length > 0) {
            for (var j = 0; j < sources.length; j++) {
              var src = sources[j];
              var cls = 'st-ct-item' + (selectedSource === src ? ' selected' : '');
              html += '<div class="' + cls + '" data-type="data" data-source="' + escHtml(src) + '"><span class="st-ct-icon">\u25C9</span>' + escHtml(src) + '</div>';
            }
          } else {
            html += '<div class="st-ct-empty">No data sources</div>';
          }
          html += '</div>';

          // Locales
          html += '<div class="st-ct-section"><div class="st-ct-section-header">\u25B8 Locales</div>';
          var locales = data.locales || [];
          if (locales.length > 0) {
            for (var k = 0; k < locales.length; k++) {
              html += '<div class="st-ct-item" data-type="locale"><span class="st-ct-icon">\u2637</span>' + escHtml(locales[k]) + '</div>';
            }
          } else {
            html += '<div class="st-ct-empty">No locales</div>';
          }
          html += '</div></div>';

          pane.innerHTML = html;

          // Bind click handlers for data source items
          var items = pane.querySelectorAll('.st-ct-item[data-type="data"]');
          for (var m = 0; m < items.length; m++) {
            items[m].addEventListener('click', function(e) {
              var name = e.currentTarget.getAttribute('data-source');
              selectedSource = name;
              if (window.__stDevDataEditor) {
                window.__stDevDataEditor.loadSource(name);
              }
              fetchAndRender();
            });
          }
        }

        function escHtml(str) {
          return String(str || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
        }

        fetchAndRender();

        // Re-fetch on DataUpdate messages from dev-ws
        if (window.__stDevWs && window.__stDevWs.onMessage) {
          window.__stDevWs.onMessage('DataUpdate', function() {
            fetchAndRender();
          });
        }

        window.__stDevContentTree = {
          refresh: fetchAndRender,
          selectSource: function(name) {
            selectedSource = name;
            if (window.__stDevDataEditor) {
              window.__stDevDataEditor.loadSource(name);
            }
            fetchAndRender();
          },
          get selectedSource() { return selectedSource; }
        };
      }
      init();

      %cleanup {
        window.__stDevContentTree = null;
      }
    })();
  }
}
