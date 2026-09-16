/// Dev Review Tab Primitive
///
/// Renders the dev-panel "Review" tab: a 5-dim audit summary backed by
/// `window.__stDevReview()` (FEAT-015). Polls every 2000ms while the
/// review pane is visible (poll is gated by a MutationObserver on the
/// pane's class list, NOT document.visibilityState).
///
/// Layout:
///   - Top: big score (0..10) with semantic Catppuccin color and a
///     "Refresh" + "Copy report" button row.
///   - 5 dim rows; each shows name, dim score badge, and expands to a
///     findings list when clicked. Findings show severity dot, message,
///     and a clickable selector chip that calls
///     `window.__stDevSelector.highlight(sel)` when present.
///   - Empty state: "All clear" when there are zero findings across
///     every dim.
///   - Walker throw: caught, surfaced as one synthetic error finding so
///     the tab never blanks.

%primitive dev-review-tab() {
  %emit js {
    (function() {
      var panel = window.__stDevPanel;
      if (!panel) return;
      var pane = panel.getPane('review');
      if (!pane) return;
      var shadow = panel.shadow;

      // -------------------------------------------------------------
      // Styles (Catppuccin palette, matching timelines-errors.st)
      // -------------------------------------------------------------
      var style = document.createElement('style');
      style.textContent = [
        '.st-rv { font-size: 12px; color: #cdd6f4; }',
        '.st-rv-top { display: flex; align-items: center; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid #313244; margin-bottom: 8px; }',
        '.st-rv-score { font-size: 36px; font-weight: 700; line-height: 1; font-variant-numeric: tabular-nums; }',
        '.st-rv-score.good { color: #a6e3a1; }',
        '.st-rv-score.mid  { color: #f9e2af; }',
        '.st-rv-score.bad  { color: #f38ba8; }',
        '.st-rv-score.err  { color: #6c7086; }',
        '.st-rv-actions { display: flex; gap: 6px; }',
        '.st-rv-btn { background: #313244; border: 1px solid #45475a; color: #cdd6f4; padding: 4px 10px; border-radius: 4px; cursor: pointer; font: inherit; font-size: 11px; transition: background 0.15s; }',
        '.st-rv-btn:hover { background: #45475a; }',
        '.st-rv-list { list-style: none; padding: 0; margin: 0; }',
        '.st-rv-row { border-bottom: 1px solid #313244; }',
        '.st-rv-row:last-child { border-bottom: none; }',
        '.st-rv-head { display: flex; align-items: center; justify-content: space-between; padding: 8px 0; cursor: pointer; user-select: none; }',
        '.st-rv-head:hover { background: rgba(49,50,68,0.4); }',
        '.st-rv-name { font-weight: 600; text-transform: capitalize; }',
        '.st-rv-meta { display: flex; align-items: center; gap: 8px; }',
        '.st-rv-badge { font-size: 10px; font-weight: 600; padding: 1px 6px; border-radius: 3px; font-variant-numeric: tabular-nums; }',
        '.st-rv-badge.good { background: rgba(166,227,161,0.15); color: #a6e3a1; }',
        '.st-rv-badge.mid  { background: rgba(249,226,175,0.15); color: #f9e2af; }',
        '.st-rv-badge.bad  { background: rgba(243,139,168,0.15); color: #f38ba8; }',
        '.st-rv-caret { color: #6c7086; font-size: 10px; transition: transform 0.15s; }',
        '.st-rv-row.open .st-rv-caret { transform: rotate(90deg); }',
        '.st-rv-findings { display: none; padding: 4px 0 10px 0; }',
        '.st-rv-row.open .st-rv-findings { display: block; }',
        '.st-rv-finding { padding: 6px 8px; margin: 4px 0; background: rgba(49,50,68,0.4); border-radius: 4px; font-size: 11px; line-height: 1.4; }',
        '.st-rv-sev { display: inline-block; width: 6px; height: 6px; border-radius: 50%; margin-right: 6px; vertical-align: middle; }',
        '.st-rv-sev.error { background: #f38ba8; }',
        '.st-rv-sev.warn  { background: #f9e2af; }',
        '.st-rv-sev.info  { background: #89b4fa; }',
        '.st-rv-msg { color: #cdd6f4; word-break: break-word; }',
        '.st-rv-sel { display: inline-block; margin-top: 4px; padding: 1px 6px; border-radius: 3px; background: rgba(137,180,250,0.15); color: #89b4fa; font-family: "SF Mono","Fira Code",monospace; font-size: 10px; cursor: pointer; transition: background 0.15s; }',
        '.st-rv-sel:hover { background: rgba(137,180,250,0.25); }',
        '.st-rv-empty { color: #585b70; text-align: center; padding: 40px 16px; }',
        '.st-rv-empty .st-rv-clear { color: #a6e3a1; font-weight: 600; }',
        '.st-rv-pending { color: #6c7086; text-align: center; padding: 40px 16px; }'
      ].join('\n');
      shadow.appendChild(style);

      // -------------------------------------------------------------
      // Helpers
      // -------------------------------------------------------------
      var DIMS = ['idiomatic', 'animation', 'brand', 'a11y', 'compile', 'hierarchy', 'detail'];
      var openDims = {}; // persist open state across re-renders
      var lastJson = '';

      function esc(s) {
        if (s == null) return '';
        return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }
      function scoreClass(n) {
        if (n == null) return 'err';
        if (n >= 8) return 'good';
        if (n >= 5) return 'mid';
        return 'bad';
      }

      function highlight(sel) {
        if (!sel) return;
        var ds = window.__stDevSelector;
        if (ds && typeof ds.highlight === 'function') {
          try { ds.highlight(sel); } catch (e) {}
        }
      }

      function safeReview() {
        var fn = window.__stDevReview || (window.Spacetime && window.Spacetime.review);
        if (typeof fn !== 'function') return null;
        try {
          return fn();
        } catch (e) {
          return {
            score: 0,
            dims: { idiomatic: 0, animation: 0, brand: 0, a11y: 0, compile: 0, hierarchy: 0, detail: 0 },
            findings: [{
              dim: 'compile',
              severity: 'error',
              message: 'review walker threw: ' + (e && e.message ? e.message : String(e)),
              selector: '',
              value: ''
            }],
            generatedAt: Date.now(),
            __error: true
          };
        }
      }

      // -------------------------------------------------------------
      // Render
      // -------------------------------------------------------------
      function render() {
        var report = safeReview();
        if (!report) {
          // Walker not loaded yet -> placeholder
          if (pane.querySelector('.st-rv-pending')) return;
          pane.innerHTML = '<div class="st-rv-pending">Review pending\u2026</div>';
          lastJson = '';
          return;
        }
        var json = JSON.stringify(report);
        if (json === lastJson) return;
        lastJson = json;

        var totalFindings = (report.findings || []).length;

        // Group findings by dim
        var byDim = {};
        for (var i = 0; i < DIMS.length; i++) byDim[DIMS[i]] = [];
        for (var j = 0; j < (report.findings || []).length; j++) {
          var f = report.findings[j];
          if (f && f.dim && byDim[f.dim]) byDim[f.dim].push(f);
        }

        pane.innerHTML = '';
        var root = document.createElement('div');
        root.className = 'st-rv';

        // Top bar: score + actions
        var top = document.createElement('div');
        top.className = 'st-rv-top';
        var scoreEl = document.createElement('div');
        scoreEl.className = 'st-rv-score ' + scoreClass(report.score);
        scoreEl.textContent = (report.score == null ? '\u2013' : Number(report.score).toFixed(0));
        top.appendChild(scoreEl);

        var actions = document.createElement('div');
        actions.className = 'st-rv-actions';
        var refreshBtn = document.createElement('button');
        refreshBtn.className = 'st-rv-btn';
        refreshBtn.textContent = 'Refresh';
        refreshBtn.addEventListener('click', function() {
          lastJson = '';
          render();
        });
        var copyBtn = document.createElement('button');
        copyBtn.className = 'st-rv-btn';
        copyBtn.textContent = 'Copy report';
        copyBtn.addEventListener('click', function() {
          var text = JSON.stringify(report, null, 2);
          if (navigator.clipboard && navigator.clipboard.writeText) {
            navigator.clipboard.writeText(text).then(function() {
              copyBtn.textContent = 'Copied';
              setTimeout(function() { copyBtn.textContent = 'Copy report'; }, 1200);
            });
          } else {
            copyBtn.textContent = 'Clipboard unavailable';
          }
        });
        actions.appendChild(refreshBtn);
        actions.appendChild(copyBtn);
        top.appendChild(actions);
        root.appendChild(top);

        if (totalFindings === 0 && !report.__error) {
          var allClear = document.createElement('div');
          allClear.className = 'st-rv-empty';
          allClear.innerHTML = '<span class="st-rv-clear">All clear \u2713</span>';
          root.appendChild(allClear);
          pane.appendChild(root);
          return;
        }

        var list = document.createElement('ul');
        list.className = 'st-rv-list';

        for (var k = 0; k < DIMS.length; k++) {
          var dim = DIMS[k];
          var dimScore = (report.dims && report.dims[dim] != null) ? report.dims[dim] : null;
          var findings = byDim[dim];

          var row = document.createElement('li');
          row.className = 'st-rv-row';
          if (openDims[dim]) row.className += ' open';

          var head = document.createElement('div');
          head.className = 'st-rv-head';
          head.dataset.dim = dim;
          head.innerHTML = '<span class="st-rv-name">' + esc(dim) + '</span>' +
            '<span class="st-rv-meta">' +
              '<span class="st-rv-badge ' + scoreClass(dimScore) + '">' +
                (dimScore == null ? '\u2013' : Number(dimScore).toFixed(0)) +
              '</span>' +
              '<span class="st-rv-badge ' + (findings.length === 0 ? 'good' : 'mid') + '">' +
                findings.length + ' finding' + (findings.length === 1 ? '' : 's') +
              '</span>' +
              '<span class="st-rv-caret">\u25B6</span>' +
            '</span>';
          (function(rowEl, dimName) {
            head.addEventListener('click', function() {
              openDims[dimName] = !openDims[dimName];
              if (openDims[dimName]) rowEl.classList.add('open');
              else rowEl.classList.remove('open');
            });
          })(row, dim);
          row.appendChild(head);

          var findingsBox = document.createElement('div');
          findingsBox.className = 'st-rv-findings';
          if (findings.length === 0) {
            var ok = document.createElement('div');
            ok.className = 'st-rv-finding';
            ok.innerHTML = '<span class="st-rv-msg" style="color:#a6e3a1">No issues</span>';
            findingsBox.appendChild(ok);
          } else {
            for (var fi = 0; fi < findings.length; fi++) {
              var fnd = findings[fi];
              var item = document.createElement('div');
              item.className = 'st-rv-finding';
              var sev = (fnd.severity || 'info');
              var html = '<span class="st-rv-sev ' + esc(sev) + '"></span>' +
                '<span class="st-rv-msg">' + esc(fnd.message || '') + '</span>';
              if (fnd.selector) {
                html += '<br><span class="st-rv-sel" data-sel="' + esc(fnd.selector) + '">' + esc(fnd.selector) + '</span>';
              }
              item.innerHTML = html;
              findingsBox.appendChild(item);
            }
          }
          row.appendChild(findingsBox);
          list.appendChild(row);
        }

        // Click delegation for selector chips
        list.addEventListener('click', function(ev) {
          var t = ev.target;
          if (t && t.classList && t.classList.contains('st-rv-sel')) {
            highlight(t.getAttribute('data-sel'));
          }
        });

        root.appendChild(list);
        pane.appendChild(root);
      }

      // -------------------------------------------------------------
      // Visibility-gated polling (MutationObserver, not visibilitychange)
      // -------------------------------------------------------------
      var pollInterval = null;
      function isVisible() {
        return pane.classList && pane.classList.contains('active');
      }
      function startPoll() {
        if (pollInterval) return;
        render();
        pollInterval = setInterval(render, 2000);
      }
      function stopPoll() {
        if (pollInterval) {
          clearInterval(pollInterval);
          pollInterval = null;
        }
      }

      var observer = new MutationObserver(function() {
        if (isVisible()) startPoll();
        else stopPoll();
      });
      observer.observe(pane, { attributes: true, attributeFilter: ['class'] });

      // Initial render so a visible-on-boot pane shows content immediately
      if (isVisible()) startPoll();
      else render(); // populate placeholder once

      %cleanup {
        stopPoll();
        observer.disconnect();
        if (style.parentNode) style.parentNode.removeChild(style);
        pane.innerHTML = '<p class="st-placeholder">Review pending\u2026</p>';
      }
    })();
  }
}
