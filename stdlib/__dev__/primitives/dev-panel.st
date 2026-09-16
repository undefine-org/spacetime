/// Dev Panel Primitive
///
/// Renders a right-docked collapsible sidebar for dev tools.
/// Provides the interactive shell: header, mode toggle, tab bar, content area, footer.
/// Actual tab content is composed by macros in later tasks.
///
/// Uses Shadow DOM for style isolation from user's site.
///
/// Two modes:
///   - Inspector: element inspection (empty state until element selected)
///   - Dashboard: existing Data/Timelines/Errors tabs
///
/// Keyboard shortcuts:
///   - Ctrl+Shift+C: toggle element inspector (handled by dev-selector, panel switches mode)
///   - Ctrl+\: toggle sidebar expanded/collapsed
///   - Escape: deselect inspected element
///
/// Sidebar state persists in sessionStorage:
///   - st-devpanel-visible: expanded (true) vs collapsed (false)
///   - st-devpanel-mode: inspector | dashboard
///
/// Responsive behavior (CSS media queries, no JS):
///   - >= 768px: normal sidebar (320px, max 40vw)
///   - < 768px: auto-collapses to 32px strip
///   - < 480px: full-width overlay when expanded
///
/// @yields $activeTab - Currently selected dashboard tab name
/// @yields $minimized - Whether sidebar is collapsed (compat with old API)

%primitive dev-panel() {
  %emit js {
    (function() {
      // Create Shadow DOM host for style isolation
      var host = document.createElement('div');
      host.id = '__spacetime-dev-panel';
      var shadow = host.attachShadow({ mode: 'open' });

      // CSS styles (dark catppuccin-inspired theme)
      var style = document.createElement('style');
      style.textContent = [
        ':host { all: initial; font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace; font-size: 13px; line-height: 1.4; }',
        '*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }',

        '.st-sidebar { display: flex; height: 100%; width: 320px; }',

        '.st-sidebar-strip { width: 32px; flex-shrink: 0; background: #181825; border-right: 1px solid #313244; display: flex; flex-direction: column; align-items: center; gap: 8px; padding-top: 12px; cursor: pointer; transition: background 0.15s; user-select: none; -webkit-user-select: none; }',
        '.st-sidebar-strip:hover { background: rgba(137,180,250,0.06); }',
        '.st-strip-chevron { color: #6c7086; font-size: 14px; transition: color 0.15s; }',
        '.st-sidebar-strip:hover .st-strip-chevron { color: #cdd6f4; }',
        '.st-strip-label { writing-mode: vertical-rl; text-orientation: mixed; color: #45475a; font-size: 9px; letter-spacing: 3px; text-transform: uppercase; }',

        '.st-sidebar-main { flex: 1; display: flex; flex-direction: column; min-width: 0; background: #1e1e2e; overflow: hidden; }',

        '.st-sidebar-header { display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: #181825; border-bottom: 1px solid #313244; flex-shrink: 0; }',
        '.st-sidebar-title { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; color: #cdd6f4; letter-spacing: 0.02em; }',
        '.st-sidebar-title-icon { font-size: 14px; }',
        '.st-sidebar-controls { display: flex; gap: 4px; }',
        '.st-sidebar-controls button { background: none; border: 1px solid #313244; color: #6c7086; width: 22px; height: 22px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 14px; line-height: 1; padding: 0; transition: background 0.15s, color 0.15s, border-color 0.15s; }',
        '.st-sidebar-controls button:hover { background: #313244; color: #cdd6f4; border-color: #45475a; }',
        '.st-sidebar-controls .st-panel-edit-mode.active { background: rgba(137,180,250,0.2); color: #89b4fa; border-color: rgba(137,180,250,0.3); }',

        '.st-mode-toggle { display: flex; padding: 6px 10px; gap: 4px; background: #181825; border-bottom: 1px solid #313244; flex-shrink: 0; }',
        '.st-mode-btn { flex: 1; background: none; border: 1px solid transparent; color: #6c7086; padding: 5px 8px; border-radius: 4px; cursor: pointer; font-family: inherit; font-size: 11px; font-weight: 500; text-align: center; transition: background 0.15s, color 0.15s, border-color 0.15s; }',
        '.st-mode-btn:hover { color: #a6adc8; background: rgba(137,180,250,0.06); }',
        '.st-mode-btn.active { color: #89b4fa; background: rgba(137,180,250,0.1); border-color: rgba(137,180,250,0.2); }',

        '.st-sidebar-body { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: hidden; }',

        '.st-inspector-pane { display: none; flex: 1; flex-direction: column; overflow-y: auto; min-height: 0; }',
        '.st-inspector-pane.active { display: flex; }',
        '.st-inspector-pane::-webkit-scrollbar { width: 6px; }',
        '.st-inspector-pane::-webkit-scrollbar-track { background: transparent; }',
        '.st-inspector-pane::-webkit-scrollbar-thumb { background: #45475a; border-radius: 3px; }',

        '.st-inspector-empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px; padding: 40px 24px; }',
        '.st-inspector-empty-icon { color: #45475a; opacity: 0.7; }',
        '.st-inspector-empty-text { color: #a6adc8; font-size: 13px; text-align: center; line-height: 1.5; }',
        '.st-inspector-empty-hint { color: #585b70; font-size: 11px; text-align: center; line-height: 1.5; }',
        'kbd { display: inline-block; background: #313244; color: #bac2de; padding: 1px 5px; border-radius: 3px; font-family: inherit; font-size: 10px; border: 1px solid #45475a; }',

        '.st-inspector-selected { display: none; flex-direction: column; padding: 12px; gap: 8px; }',
        '.st-inspector-selected.visible { display: flex; }',
        '.st-inspector-sel-header { display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; background: #181825; border-radius: 4px; border: 1px solid #313244; }',
        '.st-inspector-sel-tag { color: #89b4fa; font-weight: 600; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }',
        '.st-inspector-sel-deselect { background: none; border: none; color: #6c7086; cursor: pointer; font-size: 16px; padding: 0 4px; line-height: 1; transition: color 0.15s; }',
        '.st-inspector-sel-deselect:hover { color: #f38ba8; }',
        '.st-inspector-sel-info { display: flex; flex-direction: column; gap: 4px; }',
        '.st-inspector-sel-row { display: flex; gap: 8px; padding: 3px 8px; font-size: 11px; border-radius: 3px; }',
        '.st-inspector-sel-row:hover { background: rgba(137,180,250,0.04); }',
        '.st-inspector-sel-label { color: #6c7086; flex-shrink: 0; min-width: 54px; }',
        '.st-inspector-sel-value { color: #a6adc8; word-break: break-all; }',
        '.st-inspector-sel-none { color: #585b70; font-size: 11px; padding: 3px 8px; font-style: italic; }',

        '.st-dashboard-pane { display: none; flex: 1; flex-direction: column; min-height: 0; }',
        '.st-dashboard-pane.active { display: flex; }',

        '.st-panel-tabs { display: flex; gap: 4px; padding: 6px 10px; background: #1e1e2e; border-bottom: 1px solid #313244; flex-shrink: 0; }',
        '.st-tab { background: none; border: 1px solid transparent; color: #6c7086; padding: 4px 10px; border-radius: 4px; cursor: pointer; font-family: inherit; font-size: 11px; font-weight: 500; transition: background 0.15s, color 0.15s, border-color 0.15s; }',
        '.st-tab:hover { color: #a6adc8; background: rgba(137,180,250,0.06); }',
        '.st-tab.active { color: #89b4fa; background: rgba(137,180,250,0.1); border-color: rgba(137,180,250,0.2); }',

        '.st-panel-content { flex: 1; overflow-y: auto; min-height: 0; }',
        '.st-panel-content::-webkit-scrollbar { width: 6px; }',
        '.st-panel-content::-webkit-scrollbar-track { background: transparent; }',
        '.st-panel-content::-webkit-scrollbar-thumb { background: #45475a; border-radius: 3px; }',
        '.st-panel-content::-webkit-scrollbar-thumb:hover { background: #585b70; }',
        '.st-pane { display: none; padding: 12px; min-height: 200px; }',
        '.st-pane.active { display: block; }',
        '.st-placeholder { color: #585b70; font-size: 12px; text-align: center; padding: 40px 16px; }',

        '.st-sidebar-footer { display: flex; align-items: center; gap: 6px; padding: 6px 12px; background: #181825; border-top: 1px solid #313244; font-size: 11px; color: #6c7086; flex-shrink: 0; }',
        '.st-status-dot { width: 6px; height: 6px; border-radius: 50%; background: #f38ba8; flex-shrink: 0; transition: background 0.3s; }',
        '.st-status-dot.connected { background: #a6e3a1; }',
        '.st-status-text { white-space: nowrap; }',

        '.st-selector-pill { display: none; padding: 8px 10px; background: #181825; border-bottom: 1px solid #313244; flex-shrink: 0; }',
        '.st-selector-pill.visible { display: block; }',
        '.st-selector-pill-selector { font-family: inherit; font-size: 12px; color: #89b4fa; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }',
        '.st-selector-pill-source { font-size: 10px; color: #585b70; margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }',

        ':host { width: 320px; max-width: 40vw; transition: transform 0.3s cubic-bezier(0.4,0,0.2,1), width 0.3s cubic-bezier(0.4,0,0.2,1); }',
        ':host(.st-collapsed) { transform: translateX(calc(100% - 32px)); }',
        ':host(.st-expanded) { transform: translateX(0); }',
        '@media (max-width: 767px) { :host { transform: translateX(calc(100% - 32px)); } }',
        '@media (max-width: 479px) { :host, :host(.st-expanded) { width: 100vw; max-width: 100vw; } }',
        '.st-prop-tree { flex: 1; overflow-y: auto; }',
        '.st-prop-section { border-bottom: 1px solid #313244; }',
        '.st-prop-section-header { display: flex; align-items: center; gap: 6px; padding: 6px 10px; cursor: pointer; user-select: none; color: #cdd6f4; font-size: 11px; font-weight: 600; }',
        '.st-prop-section-header:hover { background: #1e1e2e; }',
        '.st-prop-section-chevron { font-size: 8px; transition: transform 0.2s; width: 10px; }',
        '.st-prop-section.collapsed .st-prop-section-chevron { transform: rotate(0deg); }',
        '.st-prop-section:not(.collapsed) .st-prop-section-chevron { transform: rotate(90deg); }',
        '.st-prop-section-count { color: #585b70; font-weight: 400; }',
        '.st-prop-section.collapsed .st-prop-section-body { display: none; }',
        '.st-prop-row { display: flex; align-items: center; padding: 3px 10px 3px 20px; font-size: 11px; gap: 6px; }',
        '.st-prop-row:hover { background: #1e1e2e; }',
        '.st-prop-name { color: #89b4fa; flex: 0 0 auto; max-width: 40%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }',
        '.st-prop-value { color: #cdd6f4; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }',
        '.st-prop-color-swatch { display: inline-block; width: 10px; height: 10px; border-radius: 2px; border: 1px solid #45475a; vertical-align: middle; margin-right: 4px; }',
        '.st-prop-source { color: #585b70; font-size: 9px; flex: 0 0 auto; }',
        '.st-prop-edit { color: #585b70; cursor: pointer; font-size: 10px; flex: 0 0 auto; visibility: hidden; }',
        '.st-prop-row:hover .st-prop-edit { visibility: visible; }',
        '.st-prop-edit:hover { color: #89b4fa; }',
        '.st-prop-edit-input { background: #1e1e2e; border: 1px solid #89b4fa; color: #cdd6f4; font-size: 11px; padding: 1px 4px; width: 100%; outline: none; font-family: inherit; }',
        '.st-state-display { padding: 6px 10px; }',
        '.st-state-current { font-size: 11px; color: #cdd6f4; margin-bottom: 6px; }',
        '.st-state-pills { display: flex; flex-wrap: wrap; gap: 4px; }',
        '.st-state-pill { padding: 2px 8px; border-radius: 10px; font-size: 10px; background: #313244; color: #a6adc8; }',
        '.st-state-pill.active { background: #89b4fa; color: #1e1e2e; font-weight: 600; }'
      ].join('\n');
      shadow.appendChild(style);

      // Sidebar HTML structure
      var sidebar = document.createElement('div');
      sidebar.className = 'st-sidebar';
      sidebar.innerHTML =
        '<div class="st-sidebar-strip">' +
          '<span class="st-strip-chevron">\u00BB</span>' +
          '<span class="st-strip-label">DEV</span>' +
        '</div>' +
        '<div class="st-sidebar-main">' +
          '<div class="st-sidebar-header">' +
            '<span class="st-sidebar-title"><span class="st-sidebar-title-icon">\u26A1</span> Spacetime</span>' +
            '<div class="st-sidebar-controls">' +
              '<button class="st-panel-edit-mode" title="Edit Mode"><svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 4.1 12 6"/><path d="m5.1 8-2.9-.8"/><path d="m6 12-1.9 2"/><path d="M7.2 2.2 8 5.1"/><path d="M9.037 9.69a.498.498 0 0 1 .653-.653l11 4.5a.5.5 0 0 1-.074.949l-4.349 1.041a1 1 0 0 0-.74.739l-1.04 4.35a.5.5 0 0 1-.95.074z"/></svg></button>' +
            '</div>' +
          '</div>' +
          '<div class="st-mode-toggle">' +
            '<button class="st-mode-btn active" data-mode="inspector">Inspector</button>' +
            '<button class="st-mode-btn" data-mode="dashboard">Dashboard</button>' +
          '</div>' +
          '<div class="st-sidebar-body">' +
            '<div class="st-inspector-pane active">' +
              '<div class="st-selector-pill">' +
                '<div class="st-selector-pill-selector"></div>' +
                '<div class="st-selector-pill-source"></div>' +
              '</div>' +
              '<div class="st-inspector-empty">' +
                '<svg class="st-inspector-empty-icon" xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="22" y1="12" x2="18" y2="12"/><line x1="6" y1="12" x2="2" y2="12"/><line x1="12" y1="6" x2="12" y2="2"/><line x1="12" y1="22" x2="12" y2="18"/></svg>' +
                '<div class="st-inspector-empty-text">Click an element to inspect</div>' +
                '<div class="st-inspector-empty-hint">or press <kbd>Ctrl+Shift+C</kbd> to enter inspect mode</div>' +
              '</div>' +
              '<div class="st-inspector-selected"></div>' +
              '<div class="st-prop-tree"></div>' +
            '</div>' +
            '<div class="st-dashboard-pane">' +
              '<div class="st-panel-tabs">' +
                '<button class="st-tab active" data-tab="data">Data</button>' +
                '<button class="st-tab" data-tab="timelines">Timelines</button>' +
                '<button class="st-tab" data-tab="errors">Errors</button>' +
                '<button class="st-tab" data-tab="review">Review</button>' +
              '</div>' +
              '<div class="st-panel-content">' +
                '<div class="st-pane active" data-pane="data">' +
                  '<p class="st-placeholder">Data sources will appear here</p>' +
                '</div>' +
                '<div class="st-pane" data-pane="timelines">' +
                  '<p class="st-placeholder">Timeline inspector coming soon</p>' +
                '</div>' +
                '<div class="st-pane" data-pane="errors">' +
                  '<p class="st-placeholder">No errors</p>' +
                '</div>' +
                '<div class="st-pane" data-pane="review">' +
                  '<p class="st-placeholder">Review pending\u2026</p>' +
                '</div>' +
              '</div>' +
            '</div>' +
          '</div>' +
          '<div class="st-sidebar-footer">' +
            '<span class="st-status-dot"></span>' +
            '<span class="st-status-text">Connecting\u2026</span>' +
          '</div>' +
        '</div>';
      shadow.appendChild(sidebar);

      // State
      var activeTab = 'data';
      var mode = 'inspector';
      var collapsed = false;
      var currentSelectedEl = null;
      var currentSelectedInfo = null;

      // Restore state from sessionStorage
      try {
        var savedMode = sessionStorage.getItem('st-devpanel-mode');
        if (savedMode === 'inspector' || savedMode === 'dashboard') {
          mode = savedMode;
        }
        if (sessionStorage.getItem('st-devpanel-visible') === 'false') {
          collapsed = true;
        }
      } catch (e) {}

      // Element references
      var strip = sidebar.querySelector('.st-sidebar-strip');
      var chevron = sidebar.querySelector('.st-strip-chevron');
      var modeButtons = sidebar.querySelectorAll('.st-mode-btn');
      var inspectorPane = sidebar.querySelector('.st-inspector-pane');
      var dashboardPane = sidebar.querySelector('.st-dashboard-pane');
      var inspectorEmpty = sidebar.querySelector('.st-inspector-empty');
      var inspectorSelected = sidebar.querySelector('.st-inspector-selected');
      var tabs = sidebar.querySelectorAll('.st-tab');
      var panes = sidebar.querySelectorAll('.st-pane');
      var editModeBtn = sidebar.querySelector('.st-panel-edit-mode');
      var selectorPill = sidebar.querySelector('.st-selector-pill');
      var pillSelectorEl = sidebar.querySelector('.st-selector-pill-selector');
      var pillSourceEl = sidebar.querySelector('.st-selector-pill-source');
      var propTreeEl = sidebar.querySelector('.st-prop-tree');

      // Host positioning - right-docked sidebar
      // Width, max-width, and transition are in :host CSS rules for responsive overrides
      host.style.cssText = 'position:fixed;top:0;right:0;height:100vh;z-index:99998;';

      // Apply initial collapsed state via CSS class (not inline transform)
      if (collapsed) {
        host.classList.add('st-collapsed');
        chevron.textContent = '\u00AB';
      }

      // --- Collapse/Expand toggle ---
      function toggleSidebar() {
        collapsed = !collapsed;
        if (collapsed) {
          host.classList.add('st-collapsed');
          host.classList.remove('st-expanded');
          chevron.textContent = '\u00AB';
        } else {
          host.classList.remove('st-collapsed');
          host.classList.add('st-expanded');
          chevron.textContent = '\u00BB';
        }
        try {
          sessionStorage.setItem('st-devpanel-visible', collapsed ? 'false' : 'true');
        } catch (e) {}
        %yield collapsed -> $minimized;
      }

      strip.addEventListener('click', toggleSidebar);

      // --- Mode switching ---
      function switchMode(modeName) {
        mode = modeName;
        for (var i = 0; i < modeButtons.length; i++) {
          if (modeButtons[i].getAttribute('data-mode') === modeName) {
            modeButtons[i].classList.add('active');
          } else {
            modeButtons[i].classList.remove('active');
          }
        }
        if (modeName === 'inspector') {
          inspectorPane.classList.add('active');
          dashboardPane.classList.remove('active');
        } else {
          dashboardPane.classList.add('active');
          inspectorPane.classList.remove('active');
        }
        try {
          sessionStorage.setItem('st-devpanel-mode', modeName);
        } catch (e) {}
      }

      for (var m = 0; m < modeButtons.length; m++) {
        modeButtons[m].addEventListener('click', function(e) {
          var modeName = e.currentTarget.getAttribute('data-mode');
          if (modeName) switchMode(modeName);
        });
      }

      // Apply initial mode
      switchMode(mode);

      // --- Tab switching (Dashboard mode) ---
      function switchTab(tabName) {
        activeTab = tabName;
        for (var i = 0; i < tabs.length; i++) {
          if (tabs[i].getAttribute('data-tab') === tabName) {
            tabs[i].classList.add('active');
          } else {
            tabs[i].classList.remove('active');
          }
        }
        for (var j = 0; j < panes.length; j++) {
          if (panes[j].getAttribute('data-pane') === tabName) {
            panes[j].classList.add('active');
          } else {
            panes[j].classList.remove('active');
          }
        }
        %yield activeTab -> $activeTab;
      }

      for (var t = 0; t < tabs.length; t++) {
        tabs[t].addEventListener('click', function(e) {
          var tab = e.currentTarget.getAttribute('data-tab');
          if (tab) switchTab(tab);
        });
      }

      // --- Edit mode toggle ---
      editModeBtn.addEventListener('click', function() {
        var editable = window.__stDevEditable;
        if (editable) {
          editable.toggle();
          if (editable.active) {
            editModeBtn.classList.add('active');
          } else {
            editModeBtn.classList.remove('active');
          }
        }
      });

      // --- Inspector selection display ---
      function escHtml(str) {
        if (!str) return '';
        return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
      }

      function clearSelection() {
        currentSelectedEl = null;
        currentSelectedInfo = null;
        inspectorEmpty.style.display = '';
        inspectorSelected.style.display = 'none';
        inspectorSelected.classList.remove('visible');
        inspectorSelected.innerHTML = '';
        if (selectorPill) selectorPill.classList.remove('visible');
        if (propTreeEl) propTreeEl.innerHTML = '';
        lastElementContext = null;
      }

      function showSelection(info) {
        if (!info || !info.info) {
          clearSelection();
          return;
        }
        var si = info.info;
        currentSelectedEl = info.element;
        currentSelectedInfo = si;

        inspectorEmpty.style.display = 'none';
        inspectorSelected.style.display = 'flex';
        inspectorSelected.classList.add('visible');

        // Build element tag display
        var tag = si.tag || 'unknown';
        if (si.classes && si.classes.length > 0) {
          tag += '.' + si.classes.slice(0, 3).join('.');
        }

        // Update selector pill
        if (selectorPill) {
          var pillSelector = si.tag || 'unknown';
          if (si.classes && si.classes.length > 0) {
            pillSelector = '.' + si.classes.join('.');
          }
          pillSelectorEl.textContent = pillSelector;
          pillSourceEl.textContent = '';
          selectorPill.classList.add('visible');
        }

        var html = '<div class="st-inspector-sel-header">' +
          '<span class="st-inspector-sel-tag">' + escHtml(tag) + '</span>' +
          '<button class="st-inspector-sel-deselect" title="Deselect (Esc)">\u00D7</button>' +
        '</div>' +
        '<div class="st-inspector-sel-info">';

        var hasInfo = false;
        if (si.localeKey) {
          html += '<div class="st-inspector-sel-row"><span class="st-inspector-sel-label">Locale</span><span class="st-inspector-sel-value">' + escHtml(si.localeKey) + '</span></div>';
          hasInfo = true;
        }
        if (si.dataBinding) {
          html += '<div class="st-inspector-sel-row"><span class="st-inspector-sel-label">Binding</span><span class="st-inspector-sel-value">' + escHtml(si.dataBinding) + '</span></div>';
          hasInfo = true;
        }
        if (si.dataSource) {
          html += '<div class="st-inspector-sel-row"><span class="st-inspector-sel-label">Source</span><span class="st-inspector-sel-value">' + escHtml(si.dataSource) + '</span></div>';
          hasInfo = true;
        }
        if (si.timeline) {
          html += '<div class="st-inspector-sel-row"><span class="st-inspector-sel-label">Timeline</span><span class="st-inspector-sel-value">' + escHtml(si.timeline) + '</span></div>';
          hasInfo = true;
        }
        if (si.stateInfo) {
          html += '<div class="st-inspector-sel-row"><span class="st-inspector-sel-label">State</span><span class="st-inspector-sel-value">' + escHtml(si.stateInfo) + '</span></div>';
          hasInfo = true;
        }

        if (!hasInfo) {
          html += '<p class="st-inspector-sel-none">No Spacetime bindings on this element</p>';
        }

        html += '</div>';
        inspectorSelected.innerHTML = html;

        // Bind deselect button
        var deselectBtn = inspectorSelected.querySelector('.st-inspector-sel-deselect');
        if (deselectBtn) {
          deselectBtn.addEventListener('click', function() {
            var selector = window.__stDevSelector;
            if (selector) selector.deactivate();
            clearSelection();
          });
        }
      }

      // --- Connection status + Inspector polling ---
      function updateStatus() {
        var dot = sidebar.querySelector('.st-status-dot');
        var text = sidebar.querySelector('.st-status-text');
        var ws = window.__stDevWs;
        if (ws && ws.connected) {
          dot.classList.add('connected');
          text.textContent = 'Connected';
        } else {
          dot.classList.remove('connected');
          text.textContent = 'Disconnected';
        }
      }

      function updateInspector() {
        if (mode !== 'inspector') return;
        var selector = window.__stDevSelector;
        if (!selector) return;
        var info = selector.getSelectedInfo();
        if (info && info.element) {
          if (info.element !== currentSelectedEl) {
            showSelection(info);
          }
        } else if (currentSelectedEl) {
          clearSelection();
        }
      }

      var pollInterval = setInterval(function() {
        updateStatus();
        updateInspector();
      }, 500);
      updateStatus();

      // --- Keyboard shortcuts ---
      function handlePanelKeydown(e) {
        // Ctrl+\ -> toggle sidebar expanded/collapsed
        if (e.ctrlKey && e.key === '\\' && !e.shiftKey && !e.altKey && !e.metaKey) {
          e.preventDefault();
          toggleSidebar();
          return;
        }

        // Ctrl+Shift+C -> switch to inspector mode + expand if collapsed
        // (dev-selector.st handles the actual selector activation/deactivation)
        if (e.ctrlKey && e.shiftKey && e.key === 'C' && !e.altKey && !e.metaKey) {
          switchMode('inspector');
          if (collapsed) toggleSidebar();
          return;
        }

        // Escape -> deselect element in inspector mode
        if (e.key === 'Escape' && !e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) {
          if (mode === 'inspector' && currentSelectedEl) {
            clearSelection();
          }
          return;
        }
      }

      document.addEventListener('keydown', handlePanelKeydown, true);

      // --- Property tree rendering ---
      var collapsedSections = {};
      var lastElementContext = null;

      function isColorValue(prop) {
        return prop && prop.value && prop.value.parsed && prop.value.parsed.type === 'Color';
      }

      function getColorString(prop) {
        if (isColorValue(prop) && prop.value.parsed.value) return prop.value.parsed.value;
        // Fallback: check if raw value looks like a color
        var raw = prop.value ? prop.value.raw : '';
        if (/^#[0-9a-fA-F]{3,8}$/.test(raw)) return raw;
        if (/^(rgb|hsl)a?\(/.test(raw)) return raw;
        return null;
      }

      function formatSource(prop) {
        if (!prop.source_file) return '';
        var s = prop.source_file;
        // Strip path to just filename
        var slash = s.lastIndexOf('/');
        if (slash !== -1) s = s.substring(slash + 1);
        if (prop.source_line) s += ':' + prop.source_line;
        return s;
      }

      function sendEditAst(file, selector, property, value) {
        if (window.__stDevWs && window.__stDevWs.send) {
          window.__stDevWs.send({
            type: 'EditAst',
            file: file || '',
            selector: selector || '',
            patch: { [property]: value },
            op_id: 'edit_' + Date.now()
          });
        }
      }

      function makePropertyEditable(valueSpan, prop, msgSelector) {
        if (valueSpan._editing) return;
        valueSpan._editing = true;
        var oldText = prop.value ? prop.value.raw : '';
        var input = document.createElement('input');
        input.type = 'text';
        input.className = 'st-prop-edit-input';
        input.value = oldText;
        valueSpan.textContent = '';
        valueSpan.appendChild(input);
        input.focus();
        input.select();

        function commit() {
          var newVal = input.value.trim();
          valueSpan._editing = false;
          if (newVal && newVal !== oldText) {
            sendEditAst(prop.source_file, msgSelector, prop.name, newVal);
            valueSpan.textContent = newVal;
          } else {
            valueSpan.textContent = oldText;
          }
        }

        input.addEventListener('blur', commit);
        input.addEventListener('keydown', function(ev) {
          if (ev.key === 'Enter') { ev.preventDefault(); input.blur(); }
          else if (ev.key === 'Escape') { input.value = oldText; input.blur(); }
        });
      }

      function renderPropertyRow(prop, msgSelector) {
        var row = document.createElement('div');
        row.className = 'st-prop-row';

        var nameEl = document.createElement('span');
        nameEl.className = 'st-prop-name';
        nameEl.textContent = prop.name || '';
        row.appendChild(nameEl);

        var valueEl = document.createElement('span');
        valueEl.className = 'st-prop-value';
        var colorStr = getColorString(prop);
        if (colorStr) {
          var swatch = document.createElement('span');
          swatch.className = 'st-prop-color-swatch';
          swatch.style.background = colorStr;
          valueEl.appendChild(swatch);
        }
        var rawVal = prop.value ? prop.value.raw : '';
        valueEl.appendChild(document.createTextNode(rawVal));
        row.appendChild(valueEl);

        var src = formatSource(prop);
        if (src) {
          var srcEl = document.createElement('span');
          srcEl.className = 'st-prop-source';
          srcEl.textContent = src;
          row.appendChild(srcEl);
        }

        if (prop.editable) {
          var editBtn = document.createElement('span');
          editBtn.className = 'st-prop-edit';
          editBtn.textContent = '\u270E';
          editBtn.title = 'Edit';
          (function(ve, p, s) {
            editBtn.addEventListener('click', function(ev) {
              ev.stopPropagation();
              makePropertyEditable(ve, p, s);
            });
          })(valueEl, prop, msgSelector);
          row.appendChild(editBtn);
        }

        return row;
      }

      function renderSection(section, msgSelector) {
        var sectionEl = document.createElement('div');
        sectionEl.className = 'st-prop-section';
        var sKey = section.label || 'unknown';
        if (collapsedSections[sKey]) sectionEl.classList.add('collapsed');

        var header = document.createElement('div');
        header.className = 'st-prop-section-header';

        var chevronEl = document.createElement('span');
        chevronEl.className = 'st-prop-section-chevron';
        chevronEl.textContent = '\u25B6';
        header.appendChild(chevronEl);

        var labelEl = document.createElement('span');
        labelEl.className = 'st-prop-section-label';
        labelEl.textContent = sKey;
        header.appendChild(labelEl);

        var count = section.properties ? section.properties.length : 0;
        var countEl = document.createElement('span');
        countEl.className = 'st-prop-section-count';
        countEl.textContent = '(' + count + ')';
        header.appendChild(countEl);

        sectionEl.appendChild(header);

        var body = document.createElement('div');
        body.className = 'st-prop-section-body';
        if (section.properties) {
          for (var i = 0; i < section.properties.length; i++) {
            body.appendChild(renderPropertyRow(section.properties[i], msgSelector));
          }
        }
        sectionEl.appendChild(body);

        // Toggle collapse
        (function(el, key) {
          header.addEventListener('click', function() {
            collapsedSections[key] = !collapsedSections[key];
            if (collapsedSections[key]) {
              el.classList.add('collapsed');
            } else {
              el.classList.remove('collapsed');
            }
          });
        })(sectionEl, sKey);

        return sectionEl;
      }

      function renderStateMachine(sm) {
        var container = document.createElement('div');
        container.className = 'st-prop-section';
        var sKey = 'State Machine';
        if (collapsedSections[sKey]) container.classList.add('collapsed');

        var header = document.createElement('div');
        header.className = 'st-prop-section-header';
        var chevronEl = document.createElement('span');
        chevronEl.className = 'st-prop-section-chevron';
        chevronEl.textContent = '\u25B6';
        header.appendChild(chevronEl);
        var labelEl = document.createElement('span');
        labelEl.className = 'st-prop-section-label';
        labelEl.textContent = sKey;
        header.appendChild(labelEl);
        container.appendChild(header);

        var body = document.createElement('div');
        body.className = 'st-prop-section-body';

        var display = document.createElement('div');
        display.className = 'st-state-display';

        var currentDiv = document.createElement('div');
        currentDiv.className = 'st-state-current';
        currentDiv.innerHTML = 'Current: <strong>' + escHtml(sm.current_state || 'unknown') + '</strong>';
        display.appendChild(currentDiv);

        if (sm.states && sm.states.length > 0) {
          var pillsDiv = document.createElement('div');
          pillsDiv.className = 'st-state-pills';
          for (var i = 0; i < sm.states.length; i++) {
            var pill = document.createElement('span');
            pill.className = 'st-state-pill';
            if (sm.states[i] === sm.current_state) pill.classList.add('active');
            pill.textContent = sm.states[i];
            pillsDiv.appendChild(pill);
          }
          display.appendChild(pillsDiv);
        }

        body.appendChild(display);
        container.appendChild(body);

        // Toggle collapse
        (function(el, key) {
          header.addEventListener('click', function() {
            collapsedSections[key] = !collapsedSections[key];
            if (collapsedSections[key]) {
              el.classList.add('collapsed');
            } else {
              el.classList.remove('collapsed');
            }
          });
        })(container, sKey);

        return container;
      }

      function renderPropertyTree(msg) {
        if (!propTreeEl) return;
        lastElementContext = msg;
        propTreeEl.innerHTML = '';

        if (msg.not_found) return;

        var msgSelector = msg.selector || '';

        // Render sections (skip empty)
        if (msg.sections && msg.sections.length > 0) {
          for (var i = 0; i < msg.sections.length; i++) {
            var sec = msg.sections[i];
            if (!sec.properties || sec.properties.length === 0) continue;
            propTreeEl.appendChild(renderSection(sec, msgSelector));
          }
        }

        // Render state machine if present
        if (msg.state_machine && msg.state_machine.current_state) {
          propTreeEl.appendChild(renderStateMachine(msg.state_machine));
        }

        // Render data bindings as a section
        if (msg.data_bindings && msg.data_bindings.length > 0) {
          var dbSection = { label: 'Data Bindings', properties: [] };
          for (var j = 0; j < msg.data_bindings.length; j++) {
            var db = msg.data_bindings[j];
            dbSection.properties.push({
              name: db.name,
              value: { raw: db.data_type || 'unknown' },
              editable: false
            });
          }
          propTreeEl.appendChild(renderSection(dbSection, msgSelector));
        }
      }

      // --- WebSocket ElementContext listener (Task 11+14) ---
      var wsUnsubscribe = null;
      if (window.__stDevWs && window.__stDevWs.onMessage) {
        wsUnsubscribe = window.__stDevWs.onMessage('ElementContext', function(msg) {
          if (msg && selectorPill) {
            if (msg.selector) {
              pillSelectorEl.textContent = msg.selector;
              var srcText = '';
              if (msg.source_files && msg.source_files.length > 0) {
                srcText = msg.source_files.map(function(sf) {
                  return sf.file + (sf.line ? ':' + sf.line : '');
                }).join(', ');
              }
              pillSourceEl.textContent = srcText;
              selectorPill.classList.add('visible');
            }

            // Render property tree
            renderPropertyTree(msg);

            // Auto-switch to inspector mode
            if (mode !== 'inspector') {
              switchMode('inspector');
            }
            // Expand sidebar if collapsed
            if (collapsed) {
              toggleSidebar();
            }
          }
        });
      }

      // --- Expose panel API for tab content primitives ---
      window.__stDevPanel = {
        shadow: shadow,
        getPane: function(name) {
          return sidebar.querySelector('.st-pane[data-pane="' + name + '"]');
        },
        get activeTab() {
          return activeTab;
        },
        switchTab: switchTab,
        get host() {
          return host;
        },
        get mode() {
          return mode;
        },
        switchMode: switchMode
      };

      // Append to body
      if (document.body) {
        document.body.appendChild(host);
      } else {
        document.addEventListener('DOMContentLoaded', function() {
          document.body.appendChild(host);
        });
      }

      // Initial yields
      %yield activeTab -> $activeTab;
      %yield collapsed -> $minimized;

      %cleanup {
        clearInterval(pollInterval);
        document.removeEventListener('keydown', handlePanelKeydown, true);
        if (wsUnsubscribe) {
          wsUnsubscribe();
        }
        if (host.parentNode) host.parentNode.removeChild(host);
        window.__stDevPanel = null;
      }
    })();
  }

  %exports {
    $activeTab: string
    $minimized: bool
  }
}
