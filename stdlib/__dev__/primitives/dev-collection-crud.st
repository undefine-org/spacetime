%primitive dev-collection-crud() {
  %emit js {
    (function() {
      if (!window.__ST_DEV__) return;

      var style = document.createElement('style');
      style.setAttribute('data-st-dev', 'collection-crud');
      style.textContent = [
        '[data-st-source] { position: relative; }',
        '.st-crud-add { display: flex; align-items: center; justify-content: center; gap: 6px; padding: 8px; margin: 4px 0; border: 2px dashed #313244; border-radius: 6px; color: #6c7086; font-size: 12px; cursor: pointer; transition: border-color 0.15s, color 0.15s, background 0.15s; font-family: system-ui, sans-serif; }',
        '.st-crud-add:hover { border-color: #89b4fa; color: #89b4fa; background: rgba(137,180,250,0.05); }',
        '.st-crud-item-overlay { position: absolute; top: 4px; right: 4px; display: none; gap: 2px; z-index: 9998; }',
        '[data-st-source] > [data-st-index]:hover .st-crud-item-overlay { display: flex; }',
        '.st-crud-btn { background: rgba(30,30,46,0.9); border: 1px solid #313244; color: #a6adc8; width: 22px; height: 22px; border-radius: 3px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 11px; transition: background 0.12s, color 0.12s; backdrop-filter: blur(4px); }',
        '.st-crud-btn:hover { background: rgba(30,30,46,1); color: #cdd6f4; }',
        '.st-crud-btn.danger:hover { color: #f38ba8; border-color: rgba(243,139,168,0.3); }',
      ].join('\n');
      if (document.head) document.head.appendChild(style);

      // Resolve file path from source name
      function resolveFilePath(sourceName) {
        if (typeof ST !== 'undefined' && ST._dataRegistry) {
          var entry = ST._dataRegistry.get(sourceName);
          if (entry && entry.src) {
            var src = entry.src;
            if (src.charAt(0) === '/') src = src.substring(1);
            return src;
          }
        }
        return 'data/' + sourceName + '.json';
      }

      function setupContainers() {
        var containers = document.querySelectorAll('[data-st-source]');
        for (var i = 0; i < containers.length; i++) {
          setupContainer(containers[i]);
        }
      }

      function setupContainer(container) {
        // Skip if already set up
        if (container.__stCrudSetup) return;
        container.__stCrudSetup = true;

        var sourceName = container.getAttribute('data-st-source');
        var filePath = resolveFilePath(sourceName);

        // Ensure container is position:relative for overlays
        if (getComputedStyle(container).position === 'static') {
          container.style.position = 'relative';
        }

        // Add "+" button after container
        var addBtn = document.createElement('div');
        addBtn.className = 'st-crud-add';
        addBtn.innerHTML = '<span>+</span> Add item';
        addBtn.addEventListener('click', function() {
          if (window.__stDevSave && window.__stDevSave.saveArrayInsert) {
            // Default item — try to get schema, fallback to empty object
            window.__stDevSave.saveArrayInsert(filePath, '', {}, null);
          }
        });
        container.parentNode.insertBefore(addBtn, container.nextSibling);

        // Add overlays to each child with data-st-index
        setupChildOverlays(container, filePath);

        // Re-setup on mutations (items added/removed by @each re-render)
        var obs = new MutationObserver(function() {
          setupChildOverlays(container, filePath);
        });
        obs.observe(container, { childList: true });
      }

      function setupChildOverlays(container, filePath) {
        var children = container.querySelectorAll('[data-st-index]');
        var totalItems = children.length;
        for (var j = 0; j < children.length; j++) {
          var child = children[j];
          if (child.__stCrudOverlay) continue;
          child.style.position = 'relative';

          var overlay = document.createElement('div');
          overlay.className = 'st-crud-item-overlay';

          var idx = parseInt(child.getAttribute('data-st-index'), 10);

          // Move up button
          if (idx > 0) {
            var upBtn = document.createElement('button');
            upBtn.className = 'st-crud-btn';
            upBtn.textContent = '↑';
            upBtn.title = 'Move up';
            (function(i) {
              upBtn.addEventListener('click', function(e) {
                e.stopPropagation();
                if (window.__stDevSave) window.__stDevSave.saveArrayReorder(filePath, '', i, i - 1);
              });
            })(idx);
            overlay.appendChild(upBtn);
          }

          // Move down button
          if (idx < totalItems - 1) {
            var downBtn = document.createElement('button');
            downBtn.className = 'st-crud-btn';
            downBtn.textContent = '↓';
            downBtn.title = 'Move down';
            (function(i) {
              downBtn.addEventListener('click', function(e) {
                e.stopPropagation();
                if (window.__stDevSave) window.__stDevSave.saveArrayReorder(filePath, '', i, i + 1);
              });
            })(idx);
            overlay.appendChild(downBtn);
          }

          // Remove button
          var removeBtn = document.createElement('button');
          removeBtn.className = 'st-crud-btn danger';
          removeBtn.textContent = '✕';
          removeBtn.title = 'Remove item';
          (function(i) {
            removeBtn.addEventListener('click', function(e) {
              e.stopPropagation();
              if (window.__stDevSave) window.__stDevSave.saveArrayDelete(filePath, '', i);
            });
          })(idx);
          overlay.appendChild(removeBtn);

          child.appendChild(overlay);
          child.__stCrudOverlay = overlay;
        }
      }

      // Initial setup
      if (document.readyState === 'complete' || document.readyState === 'interactive') {
        setTimeout(setupContainers, 500);
      } else {
        document.addEventListener('DOMContentLoaded', function() {
          setTimeout(setupContainers, 500);
        });
      }

      // Re-scan on DataUpdate
      if (window.__stDevWs && window.__stDevWs.onMessage) {
        window.__stDevWs.onMessage('DataUpdate', function() {
          // Wait a tick for @each to re-render
          setTimeout(function() {
            var containers = document.querySelectorAll('[data-st-source]');
            for (var i = 0; i < containers.length; i++) {
              containers[i].__stCrudSetup = false;
              // Remove old overlays
              var overlays = containers[i].querySelectorAll('.st-crud-item-overlay');
              for (var j = 0; j < overlays.length; j++) overlays[j].remove();
              var children = containers[i].querySelectorAll('[data-st-index]');
              for (var k = 0; k < children.length; k++) children[k].__stCrudOverlay = null;
            }
            // Remove old add buttons to prevent duplication
            var addBtns = document.querySelectorAll('.st-crud-add');
            for (var b = 0; b < addBtns.length; b++) addBtns[b].remove();
            setupContainers();
          }, 100);
        });
      }

      %cleanup {
        if (style && style.parentNode) style.parentNode.removeChild(style);
      }
    })();
  }
}
