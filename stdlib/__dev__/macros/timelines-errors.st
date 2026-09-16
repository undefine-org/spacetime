/// Timelines & Errors Primitive
///
/// Populates the dev-panel's "Timelines" and "Errors" tabs with
/// interactive content. Reads data from window.__stDevInspect
/// (the dev-inspect primitive's global API).
///
/// Features:
///   - Timelines tab: progress bars, type badges, relative timestamps
///   - Errors tab: error list, expandable stack traces, clear button
///   - Reactive polling every 500ms
///   - Dark catppuccin theme matching panel

%primitive timelines-errors() {
  %emit js {
    (function() {
      // Wait for panel and inspect APIs to be ready
      var panel = window.__stDevPanel;
      if (!panel) return;

      var timelinesPane = panel.getPane('timelines');
      var errorsPane = panel.getPane('errors');
      if (!timelinesPane || !errorsPane) return;

      var shadow = panel.shadow;

      // Inject styles into shadow DOM
      var style = document.createElement('style');
      style.textContent = [
        // Timelines styles
        '.st-tl-list { font-size: 12px; }',
        '.st-tl-item { padding: 8px 0; border-bottom: 1px solid #313244; }',
        '.st-tl-item:last-child { border-bottom: none; }',
        '.st-tl-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px; }',
        '.st-tl-name { color: #cdd6f4; font-weight: 600; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 180px; }',
        '.st-tl-meta { display: flex; align-items: center; gap: 6px; }',
        '.st-tl-badge { font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 3px; text-transform: uppercase; letter-spacing: 0.04em; }',
        '.st-tl-badge.scroll { background: rgba(137,180,250,0.15); color: #89b4fa; }',
        '.st-tl-badge.load { background: rgba(166,227,161,0.15); color: #a6e3a1; }',
        '.st-tl-badge.click { background: rgba(249,226,175,0.15); color: #f9e2af; }',
        '.st-tl-badge.hover { background: rgba(203,166,247,0.15); color: #cba6f7; }',
        '.st-tl-badge.unknown { background: rgba(108,112,134,0.15); color: #6c7086; }',
        '.st-tl-progress-track { height: 6px; background: #313244; border-radius: 3px; overflow: hidden; }',
        '.st-tl-progress-fill { height: 100%; background: #89b4fa; border-radius: 3px; transition: width 0.2s ease; min-width: 0; }',
        '.st-tl-footer { display: flex; justify-content: space-between; margin-top: 3px; font-size: 10px; color: #6c7086; }',
        '.st-tl-pct { color: #a6adc8; }',
        '.st-tl-empty { color: #585b70; font-size: 12px; text-align: center; padding: 40px 16px; }',

        // Errors styles
        '.st-err-toolbar { display: flex; justify-content: flex-end; margin-bottom: 8px; }',
        '.st-err-clear { background: #313244; border: 1px solid #45475a; color: #cdd6f4; padding: 3px 10px; border-radius: 4px; cursor: pointer; font-family: inherit; font-size: 11px; transition: background 0.15s; }',
        '.st-err-clear:hover { background: #45475a; }',
        '.st-err-list { font-size: 12px; }',
        '.st-err-item { padding: 8px 0; border-bottom: 1px solid #313244; }',
        '.st-err-item:last-child { border-bottom: none; }',
        '.st-err-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 3px; }',
        '.st-err-type { font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 3px; background: rgba(243,139,168,0.15); color: #f38ba8; text-transform: uppercase; letter-spacing: 0.04em; }',
        '.st-err-time { font-size: 10px; color: #6c7086; }',
        '.st-err-msg { color: #cdd6f4; line-height: 1.4; word-break: break-word; }',
        '.st-err-stack-btn { background: none; border: none; color: #6c7086; cursor: pointer; font-family: inherit; font-size: 10px; padding: 2px 0; margin-top: 4px; transition: color 0.15s; }',
        '.st-err-stack-btn:hover { color: #a6adc8; }',
        '.st-err-stack { display: none; margin-top: 4px; padding: 6px 8px; background: rgba(49,50,68,0.5); border-radius: 4px; font-family: "SF Mono", "Fira Code", monospace; font-size: 10px; color: #6c7086; white-space: pre-wrap; word-break: break-all; line-height: 1.5; overflow-x: auto; }',
        '.st-err-stack.visible { display: block; }',
        '.st-err-empty { color: #585b70; font-size: 12px; text-align: center; padding: 40px 16px; }'
      ].join('\n');
      shadow.appendChild(style);

      // State for change detection
      var lastTlJson = '';
      var lastErrJson = '';

      // Relative time helper
      function timeAgo(ts) {
        if (!ts) return '';
        var diff = Date.now() - ts;
        if (diff < 1000) return 'just now';
        var s = Math.floor(diff / 1000);
        if (s < 60) return s + 's ago';
        var m = Math.floor(s / 60);
        if (m < 60) return m + 'm ago';
        var h = Math.floor(m / 60);
        return h + 'h ago';
      }

      // Escape HTML
      function esc(str) {
        if (!str) return '';
        return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }

      // Badge class for timeline type
      function typeBadgeClass(type) {
        var t = (type || 'unknown').toLowerCase();
        if (t === 'scroll') return 'scroll';
        if (t === 'load') return 'load';
        if (t === 'click') return 'click';
        if (t === 'hover') return 'hover';
        return 'unknown';
      }

      // --- Timelines rendering ---
      function renderTimelines() {
        var inspect = window.__stDevInspect;
        if (!inspect) return;

        var timelines = inspect.getTimelines();
        var json = JSON.stringify(timelines);
        if (json === lastTlJson) return;
        lastTlJson = json;

        timelinesPane.innerHTML = '';

        if (!timelines || timelines.length === 0) {
          var empty = document.createElement('p');
          empty.className = 'st-tl-empty';
          empty.textContent = 'No timelines registered';
          timelinesPane.appendChild(empty);
          return;
        }

        var list = document.createElement('div');
        list.className = 'st-tl-list';

        for (var i = 0; i < timelines.length; i++) {
          var tl = timelines[i];
          var pct = ((tl.progress || 0) * 100).toFixed(1);
          var badgeCls = typeBadgeClass(tl.type);

          var item = document.createElement('div');
          item.className = 'st-tl-item';
          item.innerHTML = '<div class="st-tl-header">' +
            '<span class="st-tl-name">' + esc(tl.id) + '</span>' +
            '<span class="st-tl-meta">' +
              '<span class="st-tl-badge ' + badgeCls + '">' + esc(tl.type || 'unknown') + '</span>' +
            '</span>' +
          '</div>' +
          '<div class="st-tl-progress-track">' +
            '<div class="st-tl-progress-fill" style="width:' + pct + '%"></div>' +
          '</div>' +
          '<div class="st-tl-footer">' +
            '<span class="st-tl-pct">' + pct + '%</span>' +
            '<span>' + timeAgo(tl.lastUpdate || tl.createdAt) + '</span>' +
          '</div>';

          list.appendChild(item);
        }

        timelinesPane.appendChild(list);
      }

      // --- Errors rendering ---
      function renderErrors() {
        var inspect = window.__stDevInspect;
        if (!inspect) return;

        var errors = inspect.getErrors();
        var json = JSON.stringify(errors);
        if (json === lastErrJson) return;
        lastErrJson = json;

        errorsPane.innerHTML = '';

        // Update tab badge
        var errTab = shadow.querySelector('.st-tab[data-tab="errors"]');
        if (errTab) {
          errTab.textContent = errors && errors.length > 0
            ? 'Errors (' + errors.length + ')'
            : 'Errors';
        }

        if (!errors || errors.length === 0) {
          var empty = document.createElement('p');
          empty.className = 'st-err-empty';
          empty.textContent = 'No errors';
          errorsPane.appendChild(empty);
          return;
        }

        // Toolbar with clear button
        var toolbar = document.createElement('div');
        toolbar.className = 'st-err-toolbar';
        var clearBtn = document.createElement('button');
        clearBtn.className = 'st-err-clear';
        clearBtn.textContent = 'Clear';
        clearBtn.addEventListener('click', function() {
          if (window.__stDevInspect && window.__stDevInspect.clearErrors) {
            window.__stDevInspect.clearErrors();
            lastErrJson = '';
            renderErrors();
          }
        });
        toolbar.appendChild(clearBtn);
        errorsPane.appendChild(toolbar);

        var list = document.createElement('div');
        list.className = 'st-err-list';

        // Show newest first
        var reversed = errors.slice().reverse();
        for (var i = 0; i < reversed.length; i++) {
          var err = reversed[i];
          var errType = err.type || 'error';
          var errMsg = '';
          if (err.error && err.error.message) {
            errMsg = err.error.message;
          } else if (typeof err.error === 'string') {
            errMsg = err.error;
          } else if (err.message) {
            errMsg = err.message;
          }

          var stackText = '';
          if (err.error && err.error.stack) {
            stackText = err.error.stack;
          } else if (err.stack) {
            stackText = err.stack;
          }

          var item = document.createElement('div');
          item.className = 'st-err-item';

          var html = '<div class="st-err-head">' +
            '<span class="st-err-type">' + esc(errType) + '</span>' +
            '<span class="st-err-time">' + timeAgo(err.timestamp) + '</span>' +
          '</div>' +
          '<div class="st-err-msg">' + esc(errMsg) + '</div>';

          item.innerHTML = html;

          // Expandable stack trace
          if (stackText) {
            var stackBtn = document.createElement('button');
            stackBtn.className = 'st-err-stack-btn';
            stackBtn.textContent = '\u25B6 Stack trace';

            var stackPre = document.createElement('pre');
            stackPre.className = 'st-err-stack';
            stackPre.textContent = stackText;

            (function(btn, pre) {
              btn.addEventListener('click', function() {
                var isVisible = pre.classList.contains('visible');
                pre.classList.toggle('visible');
                btn.textContent = isVisible ? '\u25B6 Stack trace' : '\u25BC Stack trace';
              });
            })(stackBtn, stackPre);

            item.appendChild(stackBtn);
            item.appendChild(stackPre);
          }

          list.appendChild(item);
        }

        errorsPane.appendChild(list);
      }

      // Initial render
      renderTimelines();
      renderErrors();

      // Poll for updates
      var pollInterval = setInterval(function() {
        var inspect = window.__stDevInspect;
        if (inspect && inspect.refresh) {
          inspect.refresh();
        }
        renderTimelines();
        renderErrors();
      }, 500);

      %cleanup {
        clearInterval(pollInterval);
        if (style.parentNode) style.parentNode.removeChild(style);
        timelinesPane.innerHTML = '<p class="st-placeholder">Timeline inspector coming soon</p>';
        errorsPane.innerHTML = '<p class="st-placeholder">No errors</p>';
        // Reset tab badge
        var errTab = shadow.querySelector('.st-tab[data-tab="errors"]');
        if (errTab) errTab.textContent = 'Errors';
      }
    })();
  }
}
