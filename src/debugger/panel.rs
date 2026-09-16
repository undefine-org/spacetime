//! Debug Panel HTML/CSS/JS Generation
//!
//! Generates the visual debug panel UI that displays:
//! - Timeline progress bars
//! - Signal values with change highlighting
//! - State machine states
//! - Event history log

use super::DebuggerConfig;

/// Generate the debug panel HTML
///
/// Returns a complete HTML document containing:
/// - Panel container with tabs
/// - Timelines tab: progress bars for each timeline
/// - Signals tab: name/value pairs with change highlighting
/// - States tab: current state for each state machine
/// - History tab: scrollable log of changes
/// - CSS for styling
/// - JS for interactivity and updates
///
/// # Arguments
///
/// * `config` - The debugger configuration
///
/// # Returns
///
/// An HTML string containing the complete debug panel
pub fn generate_debug_panel(config: &DebuggerConfig) -> String {
    let position_css = config.panel_position.css_position();

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Spacetime Debugger</title>
    <style>
{css}
    </style>
</head>
<body>
    <div class="st-debug-panel" style="{position_css}">
        <div class="st-debug-header">
            <div class="st-debug-title">
                <span class="st-debug-logo">ST</span>
                Spacetime Debugger
            </div>
            <div class="st-debug-controls">
                <button class="st-debug-btn st-debug-btn-minimize" title="Minimize">_</button>
                <button class="st-debug-btn st-debug-btn-close" title="Close">x</button>
            </div>
        </div>

        <div class="st-debug-tabs">
            <button class="st-debug-tab st-debug-tab-active" data-tab="timelines">Timelines</button>
            <button class="st-debug-tab" data-tab="signals">Signals</button>
            <button class="st-debug-tab" data-tab="states">States</button>
            <button class="st-debug-tab" data-tab="errors">Errors <span class="st-error-badge" id="st-error-badge"></span></button>
            <button class="st-debug-tab" data-tab="history">History</button>
        </div>

        <div class="st-debug-content">
            <div class="st-debug-pane st-debug-pane-active" data-pane="timelines">
                <div class="st-debug-empty" id="st-timelines-empty">No timelines registered</div>
                <div class="st-debug-list" id="st-timelines-list"></div>
            </div>

            <div class="st-debug-pane" data-pane="signals">
                <div class="st-debug-empty" id="st-signals-empty">No signals tracked</div>
                <div class="st-debug-list" id="st-signals-list"></div>
            </div>

            <div class="st-debug-pane" data-pane="states">
                <div class="st-debug-empty" id="st-states-empty">No state machines</div>
                <div class="st-debug-list" id="st-states-list"></div>
            </div>

            <div class="st-debug-pane" data-pane="errors">
                <div class="st-debug-error-controls">
                    <button class="st-debug-btn" id="st-clear-errors">Clear</button>
                    <span class="st-debug-error-count" id="st-error-count">0 errors</span>
                </div>
                <div class="st-debug-empty" id="st-errors-empty">No errors</div>
                <div class="st-debug-error-list" id="st-errors-list"></div>
            </div>

            <div class="st-debug-pane" data-pane="history">
                <div class="st-debug-history-controls">
                    <button class="st-debug-btn" id="st-clear-history">Clear</button>
                    <button class="st-debug-btn" id="st-export-history">Export</button>
                    <span class="st-debug-history-count" id="st-history-count">0 events</span>
                </div>
                <div class="st-debug-empty" id="st-history-empty">No events recorded</div>
                <div class="st-debug-history-list" id="st-history-list"></div>
            </div>
        </div>

        <div class="st-debug-footer">
            <span class="st-debug-status" id="st-debug-status">Connected</span>
            <span class="st-debug-fps" id="st-debug-fps">-- fps</span>
        </div>
    </div>

    <script>
{js}
    </script>
</body>
</html>
"##,
        css = generate_panel_css(),
        js = generate_panel_js(),
        position_css = position_css,
    )
}

/// Generate the CSS for the debug panel
fn generate_panel_css() -> String {
    r##"
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
            background: transparent;
        }

        .st-debug-panel {
            position: fixed;
            width: 380px;
            max-height: 500px;
            background: #1a1a2e;
            border: 1px solid #3a3a5e;
            border-radius: 8px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            display: flex;
            flex-direction: column;
            font-size: 12px;
            color: #e0e0e0;
            z-index: 999999;
            overflow: hidden;
        }

        .st-debug-panel.minimized {
            max-height: 36px;
        }

        .st-debug-panel.minimized .st-debug-tabs,
        .st-debug-panel.minimized .st-debug-content,
        .st-debug-panel.minimized .st-debug-footer {
            display: none;
        }

        .st-debug-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px 12px;
            background: #252542;
            border-bottom: 1px solid #3a3a5e;
            cursor: move;
        }

        .st-debug-title {
            display: flex;
            align-items: center;
            gap: 8px;
            font-weight: 600;
            color: #fff;
        }

        .st-debug-logo {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            width: 20px;
            height: 20px;
            background: linear-gradient(135deg, #6366f1, #8b5cf6);
            border-radius: 4px;
            font-size: 10px;
            font-weight: 700;
        }

        .st-debug-controls {
            display: flex;
            gap: 4px;
        }

        .st-debug-btn {
            padding: 4px 8px;
            background: #3a3a5e;
            border: none;
            border-radius: 4px;
            color: #a0a0c0;
            cursor: pointer;
            font-size: 11px;
            transition: background 0.2s, color 0.2s;
        }

        .st-debug-btn:hover {
            background: #4a4a7e;
            color: #fff;
        }

        .st-debug-tabs {
            display: flex;
            background: #1e1e36;
            border-bottom: 1px solid #3a3a5e;
        }

        .st-debug-tab {
            flex: 1;
            padding: 8px 12px;
            background: transparent;
            border: none;
            border-bottom: 2px solid transparent;
            color: #8080a0;
            cursor: pointer;
            font-size: 11px;
            font-weight: 500;
            transition: all 0.2s;
        }

        .st-debug-tab:hover {
            background: rgba(99, 102, 241, 0.1);
            color: #a0a0c0;
        }

        .st-debug-tab-active {
            color: #6366f1;
            border-bottom-color: #6366f1;
            background: rgba(99, 102, 241, 0.1);
        }

        .st-debug-tab.st-tab-flash {
            animation: tab-flash 1s ease-out;
        }

        @keyframes tab-flash {
            0%, 20%, 40%, 60%, 80% { background: rgba(239, 68, 68, 0.4); color: #fff; }
            10%, 30%, 50%, 70%, 90%, 100% { background: transparent; }
        }

        .st-debug-content {
            flex: 1;
            min-height: 200px;
            position: relative;
            overflow: hidden;
        }

        .st-debug-pane {
            display: none;
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            overflow-y: auto;
            padding: 8px;
        }

        .st-debug-pane-active {
            display: block;
        }

        .st-debug-empty {
            text-align: center;
            color: #6060a0;
            padding: 24px;
            font-style: italic;
        }

        .st-debug-list {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        /* Timeline items */
        .st-timeline-item {
            background: #252542;
            border-radius: 6px;
            padding: 10px 12px;
        }

        .st-timeline-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 8px;
        }

        .st-timeline-name {
            font-weight: 600;
            color: #fff;
        }

        .st-timeline-type {
            font-size: 10px;
            color: #6366f1;
            background: rgba(99, 102, 241, 0.2);
            padding: 2px 6px;
            border-radius: 4px;
        }

        .st-timeline-progress-bar {
            height: 6px;
            background: #3a3a5e;
            border-radius: 3px;
            overflow: hidden;
        }

        .st-timeline-progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #6366f1, #8b5cf6);
            border-radius: 3px;
            transition: width 0.1s ease-out;
        }

        .st-timeline-value {
            text-align: right;
            font-size: 10px;
            color: #8080a0;
            margin-top: 4px;
        }

        /* Signal items */
        .st-signal-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            background: #252542;
            border-radius: 6px;
            padding: 8px 12px;
        }

        .st-signal-item.changed {
            animation: signal-flash 0.5s ease-out;
        }

        @keyframes signal-flash {
            0% { background: rgba(99, 102, 241, 0.4); }
            100% { background: #252542; }
        }

        .st-signal-name {
            color: #a0a0c0;
        }

        .st-signal-element {
            font-size: 10px;
            color: #6060a0;
        }

        .st-signal-value {
            font-family: monospace;
            color: #4ade80;
            background: rgba(74, 222, 128, 0.1);
            padding: 2px 8px;
            border-radius: 4px;
        }

        /* State items */
        .st-state-item {
            background: #252542;
            border-radius: 6px;
            padding: 10px 12px;
        }

        .st-state-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 8px;
        }

        .st-state-element {
            font-weight: 600;
            color: #fff;
        }

        .st-state-current {
            font-size: 11px;
            color: #22d3ee;
            background: rgba(34, 211, 238, 0.2);
            padding: 4px 10px;
            border-radius: 4px;
            font-weight: 600;
        }

        .st-state-transition {
            font-size: 10px;
            color: #6060a0;
        }

        .st-state-transition-arrow {
            color: #8b5cf6;
            margin: 0 4px;
        }

        /* Error items */
        .st-error-badge {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            min-width: 16px;
            height: 16px;
            font-size: 10px;
            font-weight: 600;
            background: #ef4444;
            color: #fff;
            border-radius: 8px;
            padding: 0 4px;
            margin-left: 4px;
        }

        .st-error-badge:empty {
            display: none;
        }

        .st-debug-error-controls {
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 8px 0;
            border-bottom: 1px solid #3a3a5e;
            margin-bottom: 8px;
        }

        .st-debug-error-count {
            margin-left: auto;
            color: #ef4444;
            font-size: 10px;
            font-weight: 500;
        }

        .st-debug-error-list {
            display: flex;
            flex-direction: column;
            gap: 8px;
            max-height: 280px;
            overflow-y: auto;
        }

        .st-error-item {
            background: #252542;
            border-radius: 6px;
            padding: 10px 12px;
            border-left: 3px solid #ef4444;
        }

        .st-error-header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            margin-bottom: 6px;
        }

        .st-error-source {
            font-size: 10px;
            color: #ef4444;
            background: rgba(239, 68, 68, 0.2);
            padding: 2px 6px;
            border-radius: 4px;
            font-weight: 500;
        }

        .st-error-time {
            font-size: 10px;
            color: #6060a0;
            font-family: monospace;
        }

        .st-error-message {
            font-weight: 600;
            color: #fff;
            margin-bottom: 6px;
            word-break: break-word;
        }

        .st-error-context {
            font-size: 11px;
            color: #a0a0c0;
            margin-bottom: 6px;
        }

        .st-error-context-item {
            display: flex;
            gap: 4px;
        }

        .st-error-context-key {
            color: #8080a0;
        }

        .st-error-context-value {
            color: #22d3ee;
            font-family: monospace;
        }

        .st-error-stack-toggle {
            font-size: 10px;
            color: #6366f1;
            background: transparent;
            border: none;
            cursor: pointer;
            padding: 4px 0;
        }

        .st-error-stack-toggle:hover {
            text-decoration: underline;
        }

        .st-error-stack {
            display: none;
            font-size: 10px;
            font-family: monospace;
            color: #8080a0;
            background: rgba(0, 0, 0, 0.3);
            padding: 8px;
            border-radius: 4px;
            margin-top: 6px;
            white-space: pre-wrap;
            word-break: break-word;
            max-height: 120px;
            overflow-y: auto;
        }

        .st-error-stack.expanded {
            display: block;
        }

        /* History items */
        .st-debug-history-controls {
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 8px 0;
            border-bottom: 1px solid #3a3a5e;
            margin-bottom: 8px;
        }

        .st-debug-history-count {
            margin-left: auto;
            color: #6060a0;
            font-size: 10px;
        }

        .st-debug-history-list {
            display: flex;
            flex-direction: column;
            gap: 4px;
            max-height: 280px;
            overflow-y: auto;
        }

        .st-history-item {
            display: flex;
            align-items: flex-start;
            gap: 8px;
            padding: 6px 8px;
            background: #252542;
            border-radius: 4px;
            font-size: 11px;
        }

        .st-history-time {
            color: #6060a0;
            font-family: monospace;
            white-space: nowrap;
        }

        .st-history-type {
            color: #8b5cf6;
            font-weight: 500;
            white-space: nowrap;
        }

        .st-history-data {
            color: #a0a0c0;
            flex: 1;
            word-break: break-word;
        }

        /* Footer */
        .st-debug-footer {
            display: flex;
            justify-content: space-between;
            padding: 6px 12px;
            background: #252542;
            border-top: 1px solid #3a3a5e;
            font-size: 10px;
            color: #6060a0;
        }

        .st-debug-status {
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .st-debug-status::before {
            content: '';
            width: 6px;
            height: 6px;
            background: #4ade80;
            border-radius: 50%;
        }

        .st-debug-status.disconnected::before {
            background: #f87171;
        }

        /* Scrollbar styling */
        .st-debug-pane::-webkit-scrollbar,
        .st-debug-history-list::-webkit-scrollbar {
            width: 6px;
        }

        .st-debug-pane::-webkit-scrollbar-track,
        .st-debug-history-list::-webkit-scrollbar-track {
            background: #1a1a2e;
        }

        .st-debug-pane::-webkit-scrollbar-thumb,
        .st-debug-history-list::-webkit-scrollbar-thumb {
            background: #3a3a5e;
            border-radius: 3px;
        }

        .st-debug-pane::-webkit-scrollbar-thumb:hover,
        .st-debug-history-list::-webkit-scrollbar-thumb:hover {
            background: #4a4a7e;
        }
"##
    .to_string()
}

/// Generate the JavaScript for the debug panel
fn generate_panel_js() -> String {
    r##"
(function() {
    'use strict';

    // Panel state
    const panel = {
        minimized: false,
        activeTab: 'timelines',
        updateInterval: null,
        frameCount: 0,
        lastFpsUpdate: Date.now(),
        // Change detection caches
        lastTimelinesJson: '',
        lastSignalsJson: '',
        lastStatesJson: '',
        lastErrorsLength: 0,
        lastHistoryLength: 0,
    };

    // DOM references
    const elements = {
        panel: document.querySelector('.st-debug-panel'),
        tabs: document.querySelectorAll('.st-debug-tab'),
        panes: document.querySelectorAll('.st-debug-pane'),
        minimizeBtn: document.querySelector('.st-debug-btn-minimize'),
        closeBtn: document.querySelector('.st-debug-btn-close'),
        timelinesEmpty: document.getElementById('st-timelines-empty'),
        timelinesList: document.getElementById('st-timelines-list'),
        signalsEmpty: document.getElementById('st-signals-empty'),
        signalsList: document.getElementById('st-signals-list'),
        statesEmpty: document.getElementById('st-states-empty'),
        statesList: document.getElementById('st-states-list'),
        errorsEmpty: document.getElementById('st-errors-empty'),
        errorsList: document.getElementById('st-errors-list'),
        errorCount: document.getElementById('st-error-count'),
        errorBadge: document.getElementById('st-error-badge'),
        clearErrorsBtn: document.getElementById('st-clear-errors'),
        historyEmpty: document.getElementById('st-history-empty'),
        historyList: document.getElementById('st-history-list'),
        historyCount: document.getElementById('st-history-count'),
        clearHistoryBtn: document.getElementById('st-clear-history'),
        exportHistoryBtn: document.getElementById('st-export-history'),
        status: document.getElementById('st-debug-status'),
        fps: document.getElementById('st-debug-fps'),
    };

    // Initialize panel
    function init() {
        // Tab switching
        elements.tabs.forEach(tab => {
            tab.addEventListener('click', () => switchTab(tab.dataset.tab));
        });

        // Minimize/close
        elements.minimizeBtn.addEventListener('click', toggleMinimize);
        elements.closeBtn.addEventListener('click', closePanel);

        // History controls
        elements.clearHistoryBtn.addEventListener('click', clearHistory);
        elements.exportHistoryBtn.addEventListener('click', exportHistory);

        // Error controls
        if (elements.clearErrorsBtn) {
            elements.clearErrorsBtn.addEventListener('click', clearErrors);
        }

        // Make header draggable
        makeDraggable();

        // Connect to parent window's debug runtime
        connectToRuntime();

        // Start update loop
        startUpdateLoop();
    }

    // Switch active tab
    function switchTab(tabName) {
        panel.activeTab = tabName;

        elements.tabs.forEach(tab => {
            tab.classList.toggle('st-debug-tab-active', tab.dataset.tab === tabName);
        });

        elements.panes.forEach(pane => {
            pane.classList.toggle('st-debug-pane-active', pane.dataset.pane === tabName);
        });
    }

    // Toggle minimized state
    function toggleMinimize() {
        panel.minimized = !panel.minimized;
        elements.panel.classList.toggle('minimized', panel.minimized);
        elements.minimizeBtn.textContent = panel.minimized ? '+' : '_';
    }

    // Close panel
    function closePanel() {
        if (panel.updateInterval) {
            clearInterval(panel.updateInterval);
        }
        elements.panel.style.display = 'none';
    }

    // Make panel draggable
    function makeDraggable() {
        const header = elements.panel.querySelector('.st-debug-header');
        let isDragging = false;
        let startX, startY, startLeft, startTop;

        header.addEventListener('mousedown', (e) => {
            if (e.target.tagName === 'BUTTON') return;
            isDragging = true;
            startX = e.clientX;
            startY = e.clientY;
            const rect = elements.panel.getBoundingClientRect();
            startLeft = rect.left;
            startTop = rect.top;
            header.style.cursor = 'grabbing';
        });

        document.addEventListener('mousemove', (e) => {
            if (!isDragging) return;
            const dx = e.clientX - startX;
            const dy = e.clientY - startY;
            elements.panel.style.left = (startLeft + dx) + 'px';
            elements.panel.style.top = (startTop + dy) + 'px';
            elements.panel.style.right = 'auto';
            elements.panel.style.bottom = 'auto';
        });

        document.addEventListener('mouseup', () => {
            if (isDragging) {
                isDragging = false;
                header.style.cursor = 'move';
            }
        });
    }

    // Connect to debug runtime
    function connectToRuntime() {
        // Try to access parent window's debug runtime (for iframe usage)
        if (window.parent && window.parent.__ST_DEBUG__) {
            window.__ST_DEBUG__ = window.parent.__ST_DEBUG__;
        }

        // If no debug runtime found, show disconnected status
        if (!window.__ST_DEBUG__) {
            elements.status.textContent = 'Disconnected';
            elements.status.classList.add('disconnected');
            return;
        }

        // Register as the panel
        window.__ST_DEBUG__.panel = {
            onDebugEvent: handleDebugEvent,
        };

        elements.status.textContent = 'Connected';
        elements.status.classList.remove('disconnected');
    }

    // Handle debug events from runtime
    function handleDebugEvent(type, data) {
        // Update relevant UI based on event type
        switch (type) {
            case 'timeline.register':
            case 'timeline.progress':
                updateTimelinesView();
                break;
            case 'signal.change':
                updateSignalsView();
                highlightSignal(data.element, data.name);
                break;
            case 'state.register':
            case 'state.transition':
                updateStatesView();
                break;
            case 'error':
                updateErrorsView();
                // Flash the errors tab to draw attention
                const errorsTab = document.querySelector('[data-tab="errors"]');
                if (errorsTab) {
                    errorsTab.classList.add('st-tab-flash');
                    setTimeout(() => errorsTab.classList.remove('st-tab-flash'), 1000);
                }
                break;
            case 'errors.clear':
                updateErrorsView();
                break;
            case 'history.clear':
                updateHistoryView();
                break;
        }
    }

    // Start update loop
    function startUpdateLoop() {
        // Initial update
        updateAllViews();

        // Update loop - 500ms is sufficient for debugging (was 100ms)
        panel.updateInterval = setInterval(() => {
            panel.frameCount++;
            const now = Date.now();
            if (now - panel.lastFpsUpdate >= 1000) {
                const fps = Math.round(panel.frameCount * 1000 / (now - panel.lastFpsUpdate));
                elements.fps.textContent = fps + ' fps';
                panel.frameCount = 0;
                panel.lastFpsUpdate = now;
            }

            // Skip updates when minimized
            if (panel.minimized) return;

            // Periodic view updates (with change detection)
            updateAllViews();
        }, 500);
    }

    // Update all views
    function updateAllViews() {
        updateTimelinesView();
        updateSignalsView();
        updateStatesView();
        updateErrorsView();
        updateHistoryView();
    }

    // Update timelines view (with change detection)
    function updateTimelinesView() {
        if (!window.__ST_DEBUG__) return;

        const timelines = window.__ST_DEBUG__.getTimelines();

        // Change detection: skip DOM update if data unchanged
        const json = JSON.stringify(timelines);
        if (json === panel.lastTimelinesJson) return;
        panel.lastTimelinesJson = json;

        if (timelines.length === 0) {
            elements.timelinesEmpty.style.display = 'block';
            elements.timelinesList.innerHTML = '';
            return;
        }

        elements.timelinesEmpty.style.display = 'none';
        elements.timelinesList.innerHTML = timelines.map(t => `
            <div class="st-timeline-item">
                <div class="st-timeline-header">
                    <span class="st-timeline-name">${escapeHtml(t.id)}</span>
                    <span class="st-timeline-type">${escapeHtml(t.type)}</span>
                </div>
                <div class="st-timeline-progress-bar">
                    <div class="st-timeline-progress-fill" style="width: ${(t.progress * 100).toFixed(1)}%"></div>
                </div>
                <div class="st-timeline-value">${(t.progress * 100).toFixed(1)}%</div>
            </div>
        `).join('');
    }

    // Update signals view (with change detection)
    function updateSignalsView() {
        if (!window.__ST_DEBUG__) return;

        const signals = window.__ST_DEBUG__.getSignals();

        // Change detection: skip DOM update if data unchanged
        const json = JSON.stringify(signals);
        if (json === panel.lastSignalsJson) return;
        panel.lastSignalsJson = json;

        if (signals.length === 0) {
            elements.signalsEmpty.style.display = 'block';
            elements.signalsList.innerHTML = '';
            return;
        }

        elements.signalsEmpty.style.display = 'none';
        elements.signalsList.innerHTML = signals.map(s => `
            <div class="st-signal-item" data-signal="${escapeHtml(s.element)}:${escapeHtml(s.name)}">
                <div>
                    <div class="st-signal-name">${escapeHtml(s.name)}</div>
                    <div class="st-signal-element">${escapeHtml(s.element)}</div>
                </div>
                <div class="st-signal-value">${formatValue(s.value)}</div>
            </div>
        `).join('');
    }

    // Update states view (with change detection)
    function updateStatesView() {
        if (!window.__ST_DEBUG__) return;

        const stateMachines = window.__ST_DEBUG__.getStateMachines();

        // Change detection: skip DOM update if data unchanged
        const json = JSON.stringify(stateMachines);
        if (json === panel.lastStatesJson) return;
        panel.lastStatesJson = json;

        if (stateMachines.length === 0) {
            elements.statesEmpty.style.display = 'block';
            elements.statesList.innerHTML = '';
            return;
        }

        elements.statesEmpty.style.display = 'none';
        elements.statesList.innerHTML = stateMachines.map(sm => {
            const lastTransition = sm.lastTransition
                ? `<div class="st-state-transition">${escapeHtml(sm.lastTransition.from)}<span class="st-state-transition-arrow">-></span>${escapeHtml(sm.lastTransition.to)} (${escapeHtml(sm.lastTransition.event)})</div>`
                : '';
            return `
                <div class="st-state-item">
                    <div class="st-state-header">
                        <span class="st-state-element">${escapeHtml(sm.element)}</span>
                        <span class="st-state-current">${escapeHtml(sm.current)}</span>
                    </div>
                    ${lastTransition}
                </div>
            `;
        }).join('');
    }

    // Update errors view (with change detection)
    function updateErrorsView() {
        if (!window.__ST_DEBUG__) return;

        const errors = window.__ST_DEBUG__.getErrors();

        // Update badge and count
        if (elements.errorBadge) {
            elements.errorBadge.textContent = errors.length > 0 ? errors.length : '';
        }
        if (elements.errorCount) {
            elements.errorCount.textContent = errors.length + ' error' + (errors.length !== 1 ? 's' : '');
        }

        // Change detection
        if (errors.length === panel.lastErrorsLength) return;
        panel.lastErrorsLength = errors.length;

        if (errors.length === 0) {
            elements.errorsEmpty.style.display = 'block';
            elements.errorsList.innerHTML = '';
            return;
        }

        elements.errorsEmpty.style.display = 'none';

        // Show newest first
        const reversed = errors.slice().reverse();
        elements.errorsList.innerHTML = reversed.map(err => {
            const contextHtml = err.context && Object.keys(err.context).length > 0
                ? `<div class="st-error-context">${Object.entries(err.context).map(([k, v]) =>
                    `<div class="st-error-context-item"><span class="st-error-context-key">${escapeHtml(k)}:</span> <span class="st-error-context-value">${escapeHtml(String(v))}</span></div>`
                  ).join('')}</div>`
                : '';

            const stackHtml = err.stack
                ? `<button class="st-error-stack-toggle" onclick="this.nextElementSibling.classList.toggle('expanded')">Show stack trace</button>
                   <pre class="st-error-stack">${escapeHtml(err.stack)}</pre>`
                : '';

            return `
                <div class="st-error-item" data-error-id="${escapeHtml(err.id)}">
                    <div class="st-error-header">
                        <span class="st-error-source">${escapeHtml(err.source)}</span>
                        <span class="st-error-time">${formatTime(err.timestamp)}</span>
                    </div>
                    <div class="st-error-message">${escapeHtml(err.message)}</div>
                    ${contextHtml}
                    ${stackHtml}
                </div>
            `;
        }).join('');
    }

    // Clear errors
    function clearErrors() {
        if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.clearErrors();
        }
        updateErrorsView();
    }

    // Update history view (with change detection)
    function updateHistoryView() {
        if (!window.__ST_DEBUG__) return;

        const history = window.__ST_DEBUG__.getHistory(100);
        elements.historyCount.textContent = history.length + ' events';

        // Change detection: skip DOM update if length unchanged
        // (history only grows, so length check is sufficient)
        if (history.length === panel.lastHistoryLength) return;
        panel.lastHistoryLength = history.length;

        if (history.length === 0) {
            elements.historyEmpty.style.display = 'block';
            elements.historyList.innerHTML = '';
            return;
        }

        elements.historyEmpty.style.display = 'none';

        // Reverse to show newest first (use reverse() in-place on slice result)
        const reversed = history.slice().reverse();
        elements.historyList.innerHTML = reversed.map(h => `
            <div class="st-history-item">
                <span class="st-history-time">${formatTime(h.timestamp)}</span>
                <span class="st-history-type">${escapeHtml(h.type)}</span>
                <span class="st-history-data">${formatHistoryData(h.data)}</span>
            </div>
        `).join('');
    }

    // Highlight a changed signal (without forced reflow)
    function highlightSignal(element, name) {
        const selector = `[data-signal="${element}:${name}"]`;
        const el = elements.signalsList.querySelector(selector);
        if (el) {
            el.classList.remove('changed');
            // Use rAF to trigger animation restart without blocking main thread
            requestAnimationFrame(() => el.classList.add('changed'));
        }
    }

    // Clear history
    function clearHistory() {
        if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.clearHistory();
        }
        updateHistoryView();
    }

    // Export history
    function exportHistory() {
        if (!window.__ST_DEBUG__) return;

        const json = window.__ST_DEBUG__.exportJSON();
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `spacetime-debug-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
    }

    // Format value for display
    function formatValue(value) {
        if (value === null) return 'null';
        if (value === undefined) return 'undefined';
        if (typeof value === 'string') return '"' + escapeHtml(value) + '"';
        if (typeof value === 'number') return value.toFixed(2);
        if (typeof value === 'boolean') return value ? 'true' : 'false';
        if (typeof value === 'object') return JSON.stringify(value);
        return String(value);
    }

    // Format timestamp for display
    function formatTime(timestamp) {
        const date = new Date(timestamp);
        const h = date.getHours().toString().padStart(2, '0');
        const m = date.getMinutes().toString().padStart(2, '0');
        const s = date.getSeconds().toString().padStart(2, '0');
        const ms = date.getMilliseconds().toString().padStart(3, '0');
        return `${h}:${m}:${s}.${ms}`;
    }

    // Format history data for display
    function formatHistoryData(data) {
        if (!data) return '';
        const parts = [];
        for (const [key, value] of Object.entries(data)) {
            parts.push(`${key}=${formatValue(value)}`);
        }
        return escapeHtml(parts.join(' '));
    }

    // Escape HTML
    function escapeHtml(text) {
        if (typeof text !== 'string') return String(text);
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
"##
    .to_string()
}

/// Generate an embeddable panel overlay (for injection into pages)
///
/// This generates a JS snippet that creates and injects the debug panel
/// directly into an existing page. The panel starts collapsed showing stats,
/// and expands to show full tabbed interface when clicked.
pub fn generate_panel_overlay(config: &DebuggerConfig) -> String {
    let position_css = config.panel_position.css_position();

    format!(
        r##"// Spacetime Debug Panel Overlay
(function() {{
    'use strict';

    // Don't inject twice
    if (document.getElementById('st-debug-panel-overlay')) return;

    // Create panel container
    const container = document.createElement('div');
    container.id = 'st-debug-panel-overlay';

    // Inject styles
    const style = document.createElement('style');
    style.textContent = `{css}`;
    document.head.appendChild(style);

    // Create panel HTML
    container.innerHTML = `
        <div class="st-debug-panel st-collapsed" style="{position_css}">
            <div class="st-debug-header">
                <div class="st-debug-title">
                    <span class="st-debug-logo">ST</span>
                    <span class="st-debug-title-text">Debugger</span>
                </div>
                <div class="st-debug-controls">
                    <button class="st-debug-btn st-debug-menu-btn" title="Commands">⋮</button>
                    <button class="st-debug-btn st-debug-collapse-btn" title="Collapse">_</button>
                </div>
            </div>

            <!-- Collapsed: Mini stats view -->
            <div class="st-debug-mini-content">
                <div class="st-debug-mini-stat" id="st-mini-timelines">0 timelines</div>
                <div class="st-debug-mini-stat" id="st-mini-signals">0 signals</div>
                <div class="st-debug-mini-stat" id="st-mini-states">0 states</div>
            </div>

            <!-- Expanded: Full panel view -->
            <div class="st-debug-expanded-content">
                <div class="st-debug-tabs">
                    <button class="st-debug-tab st-debug-tab-active" data-tab="timelines">Timelines</button>
                    <button class="st-debug-tab" data-tab="signals">Signals</button>
                    <button class="st-debug-tab" data-tab="states">States</button>
                    <button class="st-debug-tab" data-tab="errors">Errors <span class="st-error-badge" id="st-error-badge"></span></button>
                    <button class="st-debug-tab" data-tab="history">History</button>
                </div>

                <div class="st-debug-content">
                    <div class="st-debug-pane st-debug-pane-active" data-pane="timelines">
                        <div class="st-debug-view-options">
                            <label class="st-debug-option">
                                <input type="checkbox" id="st-opt-hide-inactive"> Hide inactive
                            </label>
                            <label class="st-debug-option">
                                <input type="checkbox" id="st-opt-active-top" checked> Active on top
                            </label>
                        </div>
                        <div class="st-debug-empty" id="st-timelines-empty">No timelines registered</div>
                        <div class="st-debug-list" id="st-timelines-list"></div>
                    </div>

                    <div class="st-debug-pane" data-pane="signals">
                        <div class="st-debug-empty" id="st-signals-empty">No signals tracked</div>
                        <div class="st-debug-list" id="st-signals-list"></div>
                    </div>

                    <div class="st-debug-pane" data-pane="states">
                        <div class="st-debug-empty" id="st-states-empty">No state machines</div>
                        <div class="st-debug-list" id="st-states-list"></div>
                    </div>

                    <div class="st-debug-pane" data-pane="errors">
                        <div class="st-debug-error-controls">
                            <button class="st-debug-btn" id="st-clear-errors">Clear</button>
                            <span class="st-debug-error-count" id="st-error-count">0 errors</span>
                        </div>
                        <div class="st-debug-empty" id="st-errors-empty">No errors</div>
                        <div class="st-debug-error-list" id="st-errors-list"></div>
                    </div>

                    <div class="st-debug-pane" data-pane="history">
                        <div class="st-debug-history-controls">
                            <button class="st-debug-btn" id="st-clear-history">Clear</button>
                            <button class="st-debug-btn" id="st-export-history">Export</button>
                            <span class="st-debug-history-count" id="st-history-count">0 events</span>
                        </div>
                        <div class="st-debug-empty" id="st-history-empty">No events recorded</div>
                        <div class="st-debug-history-list" id="st-history-list"></div>
                    </div>
                </div>

                <div class="st-debug-footer">
                    <span class="st-debug-status" id="st-debug-status">Connected</span>
                    <span class="st-debug-fps" id="st-debug-fps">-- fps</span>
                </div>
            </div>

            <!-- Commands drop-up menu -->
            <div class="st-debug-menu" id="st-debug-menu">
                <button class="st-debug-menu-item" data-cmd="timelines">List Timelines</button>
                <button class="st-debug-menu-item" data-cmd="signals">List Signals</button>
                <button class="st-debug-menu-item" data-cmd="states">List States</button>
                <div class="st-debug-menu-divider"></div>
                <button class="st-debug-menu-item" data-cmd="logging">Toggle Logging</button>
                <div class="st-debug-menu-divider"></div>
                <button class="st-debug-menu-item" data-cmd="export">Export JSON</button>
                <button class="st-debug-menu-item" data-cmd="clear">Clear History</button>
            </div>
        </div>
    `;

    document.body.appendChild(container);

    {js}
}})();
"##,
        css = generate_overlay_css(),
        position_css = position_css,
        js = generate_overlay_js(),
    )
}

/// Generate comprehensive CSS for the overlay panel (collapsed + expanded states)
fn generate_overlay_css() -> String {
    r##"
/* Base panel styles */
.st-debug-panel {
    position: fixed;
    width: 380px;
    max-height: 500px;
    background: rgba(26, 26, 46, 0.97);
    border: 1px solid #3a3a5e;
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace;
    font-size: 12px;
    color: #e0e0e0;
    z-index: 999999;
    backdrop-filter: blur(12px);
    display: flex;
    flex-direction: column;
    overflow: visible;
    transition: width 0.2s ease, max-height 0.2s ease;
}

/* Collapsed state */
.st-debug-panel.st-collapsed {
    width: 180px;
    max-height: none;
}
.st-debug-panel.st-collapsed .st-debug-expanded-content { display: none; }
.st-debug-panel.st-collapsed .st-debug-collapse-btn { display: none; }

/* Expanded state */
.st-debug-panel:not(.st-collapsed) .st-debug-mini-content { display: none; }

/* Header */
.st-debug-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: rgba(37, 37, 66, 0.9);
    border-bottom: 1px solid #3a3a5e;
    cursor: pointer;
    user-select: none;
}
.st-debug-panel.st-collapsed .st-debug-header {
    border-bottom: none;
    border-radius: 8px;
}
.st-debug-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: #fff;
}
.st-debug-logo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: linear-gradient(135deg, #6366f1, #8b5cf6);
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
}
.st-debug-controls {
    display: flex;
    gap: 4px;
}

/* Buttons */
.st-debug-btn {
    padding: 4px 8px;
    background: #3a3a5e;
    border: none;
    border-radius: 4px;
    color: #a0a0c0;
    cursor: pointer;
    font-size: 11px;
    transition: background 0.15s, color 0.15s;
}
.st-debug-btn:hover { background: #4a4a7e; color: #fff; }

/* Mini content (collapsed) */
.st-debug-mini-content {
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.st-debug-mini-stat {
    color: #8080a0;
    font-size: 11px;
}

/* Expanded content */
.st-debug-expanded-content {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    max-height: 420px;
    overflow: hidden;
}

/* Tabs */
.st-debug-tabs {
    display: flex;
    background: #1e1e36;
    border-bottom: 1px solid #3a3a5e;
}
.st-debug-tab {
    flex: 1;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #8080a0;
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.15s;
}
.st-debug-tab:hover {
    background: rgba(99, 102, 241, 0.1);
    color: #a0a0c0;
}
.st-debug-tab-active {
    color: #6366f1;
    border-bottom-color: #6366f1;
    background: rgba(99, 102, 241, 0.1);
}

/* Content panes */
.st-debug-content {
    flex: 1;
    min-height: 200px;
    position: relative;
    overflow: hidden;
}
.st-debug-pane {
    display: none;
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    overflow-y: auto;
    padding: 8px;
}
.st-debug-pane-active { display: block; }

/* View options */
.st-debug-view-options {
    display: flex;
    gap: 12px;
    padding: 6px 8px;
    margin-bottom: 8px;
    background: rgba(37, 37, 66, 0.6);
    border-radius: 4px;
    border-bottom: 1px solid #3a3a5e;
}
.st-debug-option {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: #8080a0;
    cursor: pointer;
    user-select: none;
}
.st-debug-option input[type="checkbox"] {
    width: 12px;
    height: 12px;
    accent-color: #6366f1;
    cursor: pointer;
}
.st-debug-option:hover {
    color: #a0a0c0;
}

.st-debug-empty {
    text-align: center;
    color: #6060a0;
    padding: 24px;
    font-style: italic;
}
.st-debug-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

/* Timeline items */
.st-timeline-item {
    background: #252542;
    border-radius: 6px;
    padding: 10px 12px;
    transition: opacity 0.2s, transform 0.2s;
}
.st-timeline-item.st-timeline-inactive {
    opacity: 0.5;
    transform: scale(0.95);
    padding: 6px 10px;
}
.st-timeline-item.st-timeline-inactive .st-timeline-header {
    margin-bottom: 4px;
}
.st-timeline-item.st-timeline-inactive .st-timeline-progress-bar {
    height: 4px;
}
.st-timeline-item.st-timeline-active {
    border-left: 3px solid #6366f1;
}
.st-timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}
.st-timeline-name {
    font-weight: 600;
    color: #fff;
    font-size: 11px;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.st-timeline-type {
    font-size: 10px;
    color: #6366f1;
    background: rgba(99, 102, 241, 0.2);
    padding: 2px 6px;
    border-radius: 4px;
}
.st-timeline-progress-bar {
    height: 6px;
    background: #3a3a5e;
    border-radius: 3px;
    overflow: hidden;
}
.st-timeline-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #6366f1, #8b5cf6);
    border-radius: 3px;
    transition: width 0.1s ease-out;
}
.st-timeline-value {
    text-align: right;
    font-size: 10px;
    color: #8080a0;
    margin-top: 4px;
}

/* Signal items */
.st-signal-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #252542;
    border-radius: 6px;
    padding: 8px 12px;
}
.st-signal-item.changed {
    animation: signal-flash 0.5s ease-out;
}
@keyframes signal-flash {
    0% { background: rgba(99, 102, 241, 0.4); }
    100% { background: #252542; }
}
.st-signal-name { color: #a0a0c0; }
.st-signal-element {
    font-size: 10px;
    color: #6060a0;
}
.st-signal-value {
    font-family: monospace;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.1);
    padding: 2px 8px;
    border-radius: 4px;
}

/* State items */
.st-state-item {
    background: #252542;
    border-radius: 6px;
    padding: 10px 12px;
}
.st-state-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}
.st-state-element {
    font-weight: 600;
    color: #fff;
}
.st-state-current {
    font-size: 11px;
    color: #22d3ee;
    background: rgba(34, 211, 238, 0.2);
    padding: 4px 10px;
    border-radius: 4px;
    font-weight: 600;
}
.st-state-transition {
    font-size: 10px;
    color: #6060a0;
}
.st-state-transition-arrow {
    color: #8b5cf6;
    margin: 0 4px;
}

/* Error items */
.st-error-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 16px;
    height: 16px;
    font-size: 10px;
    font-weight: 600;
    background: #ef4444;
    color: #fff;
    border-radius: 8px;
    padding: 0 4px;
    margin-left: 4px;
}
.st-error-badge:empty { display: none; }
.st-debug-error-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    border-bottom: 1px solid #3a3a5e;
    margin-bottom: 8px;
}
.st-debug-error-count {
    margin-left: auto;
    color: #ef4444;
    font-size: 10px;
    font-weight: 500;
}
.st-debug-error-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
}
.st-error-item {
    background: #252542;
    border-radius: 6px;
    padding: 10px 12px;
    border-left: 3px solid #ef4444;
}
.st-error-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 6px;
}
.st-error-source {
    font-size: 10px;
    color: #ef4444;
    background: rgba(239, 68, 68, 0.2);
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 500;
}
.st-error-time {
    font-size: 10px;
    color: #6060a0;
    font-family: monospace;
}
.st-error-message {
    font-weight: 600;
    color: #fff;
    margin-bottom: 6px;
    word-break: break-word;
}
.st-error-context {
    font-size: 11px;
    color: #a0a0c0;
    margin-bottom: 6px;
}
.st-error-context-item {
    display: flex;
    gap: 4px;
}
.st-error-context-key { color: #8080a0; }
.st-error-context-value {
    color: #22d3ee;
    font-family: monospace;
}
.st-error-stack-toggle {
    font-size: 10px;
    color: #6366f1;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px 0;
}
.st-error-stack-toggle:hover { text-decoration: underline; }
.st-error-stack {
    display: none;
    font-size: 10px;
    font-family: monospace;
    color: #8080a0;
    background: rgba(0, 0, 0, 0.3);
    padding: 8px;
    border-radius: 4px;
    margin-top: 6px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 100px;
    overflow-y: auto;
}
.st-error-stack.expanded { display: block; }

.st-debug-tab.st-tab-flash {
    animation: tab-flash 1s ease-out;
}
@keyframes tab-flash {
    0%, 20%, 40%, 60%, 80% { background: rgba(239, 68, 68, 0.4); color: #fff; }
    10%, 30%, 50%, 70%, 90%, 100% { background: transparent; }
}

/* History */
.st-debug-history-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    border-bottom: 1px solid #3a3a5e;
    margin-bottom: 8px;
}
.st-debug-history-count {
    margin-left: auto;
    color: #6060a0;
    font-size: 10px;
}
.st-debug-history-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 240px;
    overflow-y: auto;
}
.st-history-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 8px;
    background: #252542;
    border-radius: 4px;
    font-size: 11px;
}
.st-history-time {
    color: #6060a0;
    font-family: monospace;
    white-space: nowrap;
}
.st-history-type {
    color: #8b5cf6;
    font-weight: 500;
    white-space: nowrap;
}
.st-history-data {
    color: #a0a0c0;
    flex: 1;
    word-break: break-word;
}

/* Footer */
.st-debug-footer {
    display: flex;
    justify-content: space-between;
    padding: 6px 12px;
    background: rgba(37, 37, 66, 0.8);
    border-top: 1px solid #3a3a5e;
    font-size: 10px;
    color: #6060a0;
}
.st-debug-status {
    display: flex;
    align-items: center;
    gap: 6px;
}
.st-debug-status::before {
    content: '';
    width: 6px;
    height: 6px;
    background: #4ade80;
    border-radius: 50%;
}

/* Drop-up menu */
.st-debug-menu {
    position: absolute;
    bottom: 100%;
    right: 8px;
    margin-bottom: 4px;
    background: rgba(37, 37, 66, 0.98);
    border: 1px solid #3a3a5e;
    border-radius: 6px;
    box-shadow: 0 -4px 16px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    min-width: 160px;
    display: none;
    backdrop-filter: blur(8px);
}
.st-debug-menu.st-menu-open { display: block; }
.st-debug-menu-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: #c0c0d0;
    font-size: 11px;
    text-align: left;
    cursor: pointer;
    transition: background 0.1s;
}
.st-debug-menu-item:hover {
    background: rgba(99, 102, 241, 0.2);
    color: #fff;
}
.st-debug-menu-divider {
    height: 1px;
    background: #3a3a5e;
    margin: 4px 0;
}

/* Scrollbar */
.st-debug-pane::-webkit-scrollbar,
.st-debug-history-list::-webkit-scrollbar {
    width: 6px;
}
.st-debug-pane::-webkit-scrollbar-track,
.st-debug-history-list::-webkit-scrollbar-track {
    background: #1a1a2e;
}
.st-debug-pane::-webkit-scrollbar-thumb,
.st-debug-history-list::-webkit-scrollbar-thumb {
    background: #3a3a5e;
    border-radius: 3px;
}
.st-debug-pane::-webkit-scrollbar-thumb:hover,
.st-debug-history-list::-webkit-scrollbar-thumb:hover {
    background: #4a4a7e;
}
"##
    .to_string()
}

/// Generate JavaScript for the overlay panel (expand/collapse, tabs, menu, updates)
fn generate_overlay_js() -> String {
    r##"
    // Storage key
    const STORAGE_KEY = 'st-debugger-state';

    // Load persisted state
    function loadState() {
        try {
            const saved = localStorage.getItem(STORAGE_KEY);
            if (saved) return JSON.parse(saved);
        } catch (e) {}
        return null;
    }

    // Save state to localStorage
    function saveState() {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify({
                expanded: state.expanded,
                activeTab: state.activeTab,
                hideInactive: state.hideInactive,
                activeOnTop: state.activeOnTop,
            }));
        } catch (e) {}
    }

    const savedState = loadState();

    // Panel state
    const state = {
        expanded: savedState?.expanded ?? false,
        activeTab: savedState?.activeTab ?? 'timelines',
        hideInactive: savedState?.hideInactive ?? false,
        activeOnTop: savedState?.activeOnTop ?? true,
        menuOpen: false,
        loggingEnabled: false,
        loggingUnsubscribe: null,
        frameCount: 0,
        lastFpsUpdate: Date.now(),
        // Change detection caches
        lastTimelinesJson: '',
        lastSignalsJson: '',
        lastStatesJson: '',
        lastErrorsLength: 0,
        lastHistoryLength: 0,
        lastMiniStats: '',
    };

    // DOM refs
    const panel = container.querySelector('.st-debug-panel');
    const header = panel.querySelector('.st-debug-header');
    const menuBtn = panel.querySelector('.st-debug-menu-btn');
    const collapseBtn = panel.querySelector('.st-debug-collapse-btn');
    const menu = panel.querySelector('.st-debug-menu');
    const tabs = panel.querySelectorAll('.st-debug-tab');
    const panes = panel.querySelectorAll('.st-debug-pane');

    // Elements for content
    const els = {
        miniTimelines: document.getElementById('st-mini-timelines'),
        miniSignals: document.getElementById('st-mini-signals'),
        miniStates: document.getElementById('st-mini-states'),
        timelinesEmpty: document.getElementById('st-timelines-empty'),
        timelinesList: document.getElementById('st-timelines-list'),
        signalsEmpty: document.getElementById('st-signals-empty'),
        signalsList: document.getElementById('st-signals-list'),
        statesEmpty: document.getElementById('st-states-empty'),
        statesList: document.getElementById('st-states-list'),
        errorsEmpty: document.getElementById('st-errors-empty'),
        errorsList: document.getElementById('st-errors-list'),
        errorCount: document.getElementById('st-error-count'),
        errorBadge: document.getElementById('st-error-badge'),
        clearErrors: document.getElementById('st-clear-errors'),
        historyEmpty: document.getElementById('st-history-empty'),
        historyList: document.getElementById('st-history-list'),
        historyCount: document.getElementById('st-history-count'),
        clearHistory: document.getElementById('st-clear-history'),
        exportHistory: document.getElementById('st-export-history'),
        status: document.getElementById('st-debug-status'),
        fps: document.getElementById('st-debug-fps'),
        optHideInactive: document.getElementById('st-opt-hide-inactive'),
        optActiveTop: document.getElementById('st-opt-active-top'),
    };

    // Initialize view option checkboxes
    if (els.optHideInactive) {
        els.optHideInactive.checked = state.hideInactive;
        els.optHideInactive.addEventListener('change', () => {
            state.hideInactive = els.optHideInactive.checked;
            saveState();
            updateTimelinesView();
        });
    }
    if (els.optActiveTop) {
        els.optActiveTop.checked = state.activeOnTop;
        els.optActiveTop.addEventListener('change', () => {
            state.activeOnTop = els.optActiveTop.checked;
            saveState();
            updateTimelinesView();
        });
    }

    // Toggle expanded/collapsed
    function toggleExpand() {
        state.expanded = !state.expanded;
        panel.classList.toggle('st-collapsed', !state.expanded);
        if (state.expanded) updateAllViews();
        saveState();
    }

    // Header click to toggle (but not on buttons)
    header.addEventListener('click', (e) => {
        if (e.target.tagName !== 'BUTTON') {
            toggleExpand();
        }
    });

    // Collapse button
    collapseBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        state.expanded = false;
        panel.classList.add('st-collapsed');
        saveState();
    });

    // Menu toggle
    menuBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        state.menuOpen = !state.menuOpen;
        menu.classList.toggle('st-menu-open', state.menuOpen);
    });

    // Close menu when clicking outside
    document.addEventListener('click', (e) => {
        if (!menu.contains(e.target) && e.target !== menuBtn) {
            state.menuOpen = false;
            menu.classList.remove('st-menu-open');
        }
    });

    // Menu commands
    menu.addEventListener('click', (e) => {
        const cmd = e.target.dataset.cmd;
        if (!cmd) return;

        state.menuOpen = false;
        menu.classList.remove('st-menu-open');

        if (!window.__ST_DEBUG__) {
            console.warn('[Spacetime] Debug runtime not available');
            return;
        }

        switch (cmd) {
            case 'timelines':
                console.log('%c[Spacetime] Timelines:', 'color: #6366f1; font-weight: bold');
                console.table(window.__ST_DEBUG__.getTimelines());
                break;
            case 'signals':
                console.log('%c[Spacetime] Signals:', 'color: #6366f1; font-weight: bold');
                console.table(window.__ST_DEBUG__.getSignals());
                break;
            case 'states':
                console.log('%c[Spacetime] State Machines:', 'color: #6366f1; font-weight: bold');
                console.table(window.__ST_DEBUG__.getStateMachines());
                break;
            case 'logging':
                if (state.loggingEnabled && state.loggingUnsubscribe) {
                    state.loggingUnsubscribe();
                    state.loggingUnsubscribe = null;
                    state.loggingEnabled = false;
                    console.log('%c[Spacetime] Logging disabled', 'color: #f87171');
                } else {
                    state.loggingUnsubscribe = window.__ST_DEBUG__.subscribe((type, data) => {
                        const color = type.includes('timeline') ? '#6366f1' :
                                      type.includes('signal') ? '#4ade80' :
                                      type.includes('state') ? '#22d3ee' : '#8b5cf6';
                        console.log(`%c[ST:${type}]`, `color: ${color}`, data);
                    });
                    state.loggingEnabled = true;
                    console.log('%c[Spacetime] Logging enabled - events will be logged to console', 'color: #4ade80');
                }
                break;
            case 'export':
                const json = window.__ST_DEBUG__.exportJSON();
                const blob = new Blob([json], { type: 'application/json' });
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = `spacetime-debug-${Date.now()}.json`;
                a.click();
                URL.revokeObjectURL(url);
                console.log('%c[Spacetime] Exported debug snapshot', 'color: #4ade80');
                break;
            case 'clear':
                window.__ST_DEBUG__.clearHistory();
                updateHistoryView();
                console.log('%c[Spacetime] History cleared', 'color: #f87171');
                break;
        }
    });

    // Tab switching
    tabs.forEach(tab => {
        tab.addEventListener('click', () => {
            const tabName = tab.dataset.tab;
            state.activeTab = tabName;

            tabs.forEach(t => t.classList.toggle('st-debug-tab-active', t.dataset.tab === tabName));
            panes.forEach(p => p.classList.toggle('st-debug-pane-active', p.dataset.pane === tabName));
            saveState();
        });
    });

    // Apply initial state from localStorage
    if (state.expanded) {
        panel.classList.remove('st-collapsed');
    }
    if (state.activeTab !== 'timelines') {
        tabs.forEach(t => t.classList.toggle('st-debug-tab-active', t.dataset.tab === state.activeTab));
        panes.forEach(p => p.classList.toggle('st-debug-pane-active', p.dataset.pane === state.activeTab));
    }

    // History controls
    els.clearHistory?.addEventListener('click', () => {
        if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.clearHistory();
            updateHistoryView();
        }
    });

    els.exportHistory?.addEventListener('click', () => {
        if (!window.__ST_DEBUG__) return;
        const json = window.__ST_DEBUG__.exportJSON();
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `spacetime-debug-${Date.now()}.json`;
        a.click();
        URL.revokeObjectURL(url);
    });

    // Error controls
    els.clearErrors?.addEventListener('click', () => {
        if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.clearErrors();
            updateErrorsView();
        }
    });

    // Update functions (with change detection)
    function updateMiniStats() {
        if (!window.__ST_DEBUG__) return;
        const tl = window.__ST_DEBUG__.getTimelines().length;
        const sig = window.__ST_DEBUG__.getSignals().length;
        const st = window.__ST_DEBUG__.getStateMachines().length;
        const key = `${tl}:${sig}:${st}`;
        if (key === state.lastMiniStats) return;
        state.lastMiniStats = key;
        els.miniTimelines.textContent = tl + ' timelines';
        els.miniSignals.textContent = sig + ' signals';
        els.miniStates.textContent = st + ' states';
    }

    function updateTimelinesView() {
        if (!window.__ST_DEBUG__) return;
        let timelines = window.__ST_DEBUG__.getTimelines();

        // Change detection: skip DOM update if data unchanged
        const json = JSON.stringify(timelines);
        if (json === state.lastTimelinesJson) return;
        state.lastTimelinesJson = json;

        // Determine active status (progress > 0 and < 1)
        timelines = timelines.map(t => ({
            ...t,
            isActive: t.progress > 0 && t.progress < 1
        }));

        // Filter if hiding inactive
        if (state.hideInactive) {
            timelines = timelines.filter(t => t.isActive);
        }

        // Sort active on top if enabled
        if (state.activeOnTop) {
            timelines.sort((a, b) => {
                if (a.isActive && !b.isActive) return -1;
                if (!a.isActive && b.isActive) return 1;
                return 0;
            });
        }

        if (timelines.length === 0) {
            els.timelinesEmpty.style.display = 'block';
            els.timelinesEmpty.textContent = state.hideInactive ? 'No active timelines' : 'No timelines registered';
            els.timelinesList.innerHTML = '';
            return;
        }

        els.timelinesEmpty.style.display = 'none';
        els.timelinesList.innerHTML = timelines.map(t => {
            const activeClass = t.isActive ? 'st-timeline-active' : 'st-timeline-inactive';
            return `
            <div class="st-timeline-item ${activeClass}">
                <div class="st-timeline-header">
                    <span class="st-timeline-name" title="${escapeHtml(t.id)}">${escapeHtml(t.id)}</span>
                    <span class="st-timeline-type">${escapeHtml(t.type)}</span>
                </div>
                <div class="st-timeline-progress-bar">
                    <div class="st-timeline-progress-fill" style="width: ${(t.progress * 100).toFixed(1)}%"></div>
                </div>
                <div class="st-timeline-value">${(t.progress * 100).toFixed(1)}%</div>
            </div>
        `}).join('');
    }

    function updateSignalsView() {
        if (!window.__ST_DEBUG__) return;
        const signals = window.__ST_DEBUG__.getSignals();

        // Change detection
        const json = JSON.stringify(signals);
        if (json === state.lastSignalsJson) return;
        state.lastSignalsJson = json;

        if (signals.length === 0) {
            els.signalsEmpty.style.display = 'block';
            els.signalsList.innerHTML = '';
            return;
        }

        els.signalsEmpty.style.display = 'none';
        els.signalsList.innerHTML = signals.map(s => `
            <div class="st-signal-item" data-signal="${escapeHtml(s.element)}:${escapeHtml(s.name)}">
                <div>
                    <div class="st-signal-name">${escapeHtml(s.name)}</div>
                    <div class="st-signal-element">${escapeHtml(s.element)}</div>
                </div>
                <div class="st-signal-value">${formatValue(s.value)}</div>
            </div>
        `).join('');
    }

    function updateStatesView() {
        if (!window.__ST_DEBUG__) return;
        const stateMachines = window.__ST_DEBUG__.getStateMachines();

        // Change detection
        const json = JSON.stringify(stateMachines);
        if (json === state.lastStatesJson) return;
        state.lastStatesJson = json;

        if (stateMachines.length === 0) {
            els.statesEmpty.style.display = 'block';
            els.statesList.innerHTML = '';
            return;
        }

        els.statesEmpty.style.display = 'none';
        els.statesList.innerHTML = stateMachines.map(sm => {
            const lastTransition = sm.lastTransition
                ? `<div class="st-state-transition">${escapeHtml(sm.lastTransition.from)}<span class="st-state-transition-arrow">→</span>${escapeHtml(sm.lastTransition.to)} (${escapeHtml(sm.lastTransition.event)})</div>`
                : '';
            return `
                <div class="st-state-item">
                    <div class="st-state-header">
                        <span class="st-state-element">${escapeHtml(sm.element)}</span>
                        <span class="st-state-current">${escapeHtml(sm.current)}</span>
                    </div>
                    ${lastTransition}
                </div>
            `;
        }).join('');
    }

    function updateErrorsView() {
        if (!window.__ST_DEBUG__) return;
        const errors = window.__ST_DEBUG__.getErrors();

        // Update badge and count
        if (els.errorBadge) {
            els.errorBadge.textContent = errors.length > 0 ? errors.length : '';
        }
        if (els.errorCount) {
            els.errorCount.textContent = errors.length + ' error' + (errors.length !== 1 ? 's' : '');
        }

        // Change detection
        if (errors.length === state.lastErrorsLength) return;
        state.lastErrorsLength = errors.length;

        if (errors.length === 0) {
            els.errorsEmpty.style.display = 'block';
            els.errorsList.innerHTML = '';
            return;
        }

        els.errorsEmpty.style.display = 'none';
        const reversed = errors.slice().reverse();
        els.errorsList.innerHTML = reversed.map(err => {
            const contextHtml = err.context && Object.keys(err.context).length > 0
                ? `<div class="st-error-context">${Object.entries(err.context).map(([k, v]) =>
                    `<div class="st-error-context-item"><span class="st-error-context-key">${escapeHtml(k)}:</span> <span class="st-error-context-value">${escapeHtml(String(v))}</span></div>`
                  ).join('')}</div>`
                : '';

            const stackHtml = err.stack
                ? `<button class="st-error-stack-toggle" onclick="this.nextElementSibling.classList.toggle('expanded')">Show stack trace</button>
                   <pre class="st-error-stack">${escapeHtml(err.stack)}</pre>`
                : '';

            return `
                <div class="st-error-item">
                    <div class="st-error-header">
                        <span class="st-error-source">${escapeHtml(err.source)}</span>
                        <span class="st-error-time">${formatTime(err.timestamp)}</span>
                    </div>
                    <div class="st-error-message">${escapeHtml(err.message)}</div>
                    ${contextHtml}
                    ${stackHtml}
                </div>
            `;
        }).join('');
    }

    function updateHistoryView() {
        if (!window.__ST_DEBUG__) return;
        const history = window.__ST_DEBUG__.getHistory(100);
        els.historyCount.textContent = history.length + ' events';

        // Change detection (history only grows)
        if (history.length === state.lastHistoryLength) return;
        state.lastHistoryLength = history.length;

        if (history.length === 0) {
            els.historyEmpty.style.display = 'block';
            els.historyList.innerHTML = '';
            return;
        }

        els.historyEmpty.style.display = 'none';
        const reversed = history.slice().reverse();
        els.historyList.innerHTML = reversed.map(h => `
            <div class="st-history-item">
                <span class="st-history-time">${formatTime(h.timestamp)}</span>
                <span class="st-history-type">${escapeHtml(h.type)}</span>
                <span class="st-history-data">${formatHistoryData(h.data)}</span>
            </div>
        `).join('');
    }

    function updateAllViews() {
        updateTimelinesView();
        updateSignalsView();
        updateStatesView();
        updateErrorsView();
        updateHistoryView();
    }

    // Helpers
    function escapeHtml(text) {
        if (typeof text !== 'string') return String(text);
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    function formatValue(value) {
        if (value === null) return 'null';
        if (value === undefined) return 'undefined';
        if (typeof value === 'string') return '"' + escapeHtml(value) + '"';
        if (typeof value === 'number') return value.toFixed(2);
        if (typeof value === 'boolean') return value ? 'true' : 'false';
        if (typeof value === 'object') return JSON.stringify(value);
        return String(value);
    }

    function formatTime(timestamp) {
        const date = new Date(timestamp);
        const h = date.getHours().toString().padStart(2, '0');
        const m = date.getMinutes().toString().padStart(2, '0');
        const s = date.getSeconds().toString().padStart(2, '0');
        return `${h}:${m}:${s}`;
    }

    function formatHistoryData(data) {
        if (!data) return '';
        const parts = [];
        for (const [key, value] of Object.entries(data)) {
            parts.push(`${key}=${formatValue(value)}`);
        }
        return escapeHtml(parts.join(' '));
    }

    // Update loop - 500ms is sufficient for debugging (was 100ms)
    setInterval(() => {
        // FPS counter
        state.frameCount++;
        const now = Date.now();
        if (now - state.lastFpsUpdate >= 1000) {
            const fps = Math.round(state.frameCount * 1000 / (now - state.lastFpsUpdate));
            if (els.fps) els.fps.textContent = fps + ' fps';
            state.frameCount = 0;
            state.lastFpsUpdate = now;
        }

        // Update mini stats (with change detection) when collapsed
        if (!state.expanded) {
            updateMiniStats();
            return;
        }

        // Update expanded views (with change detection)
        updateAllViews();
    }, 500);

    // Check connection
    if (window.__ST_DEBUG__) {
        if (els.status) els.status.textContent = 'Connected';
    } else {
        if (els.status) {
            els.status.textContent = 'Disconnected';
            els.status.classList.add('disconnected');
        }
    }

    console.log('[Spacetime] Debug panel overlay injected');
"##.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debugger::PanelPosition;

    #[test]
    fn test_generate_debug_panel() {
        let config = DebuggerConfig::default();
        let html = generate_debug_panel(&config);

        // Check HTML structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Spacetime Debugger"));
        assert!(html.contains("st-debug-panel"));

        // Check tabs
        assert!(html.contains("data-tab=\"timelines\""));
        assert!(html.contains("data-tab=\"signals\""));
        assert!(html.contains("data-tab=\"states\""));
        assert!(html.contains("data-tab=\"errors\""));
        assert!(html.contains("data-tab=\"history\""));

        // Check position
        assert!(html.contains("bottom: 16px; right: 16px;"));
    }

    #[test]
    fn test_generate_debug_panel_custom_position() {
        let config = DebuggerConfig::new().position(PanelPosition::TopLeft);
        let html = generate_debug_panel(&config);

        assert!(html.contains("top: 16px; left: 16px;"));
    }

    #[test]
    fn test_generate_panel_overlay() {
        let config = DebuggerConfig::default();
        let js = generate_panel_overlay(&config);

        // Check it's self-contained JS
        assert!(js.contains("(function()"));
        assert!(js.contains("st-debug-panel-overlay"));
        assert!(js.contains("window.__ST_DEBUG__"));
    }
}
