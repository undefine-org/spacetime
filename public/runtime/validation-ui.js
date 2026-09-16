/**
 * Spacetime Validation UI
 *
 * Shadow DOM overlay component for displaying compile-time diagnostics.
 * Shows errors as a blocking modal, warnings/infos as a collapsible badge.
 */

(function() {
  'use strict';

  // Get diagnostics from injected global
  const diagnostics = window.__SPACETIME_DIAGNOSTICS__ || [];

  if (diagnostics.length === 0) {
    return; // Nothing to show
  }

  // Categorize diagnostics
  const errors = diagnostics.filter(d => d.severity === 'error');
  const warnings = diagnostics.filter(d => d.severity === 'warning');
  const infos = diagnostics.filter(d => d.severity === 'info');

  // Create the validation overlay component
  class SpacetimeValidation extends HTMLElement {
    constructor() {
      super();
      this.attachShadow({ mode: 'open' });
      this.expanded = false;
    }

    connectedCallback() {
      this.render();
      this.setupEventListeners();
    }

    render() {
      const hasErrors = errors.length > 0;
      const hasWarnings = warnings.length > 0 || infos.length > 0;

      this.shadowRoot.innerHTML = `
        <style>
          :host {
            --st-error: #ef4444;
            --st-error-bg: #fef2f2;
            --st-warning: #f59e0b;
            --st-warning-bg: #fffbeb;
            --st-info: #3b82f6;
            --st-info-bg: #eff6ff;
            --st-text: #1f2937;
            --st-text-muted: #6b7280;
            --st-border: #e5e7eb;
            --st-bg: #ffffff;
            --st-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);

            font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
            font-size: 14px;
            line-height: 1.5;
          }

          * {
            box-sizing: border-box;
          }

          /* Error Modal (Blocking) */
          .error-modal {
            position: fixed;
            inset: 0;
            z-index: 99999;
            display: flex;
            align-items: center;
            justify-content: center;
            background: rgba(0, 0, 0, 0.5);
            backdrop-filter: blur(4px);
          }

          .error-content {
            background: var(--st-bg);
            border-radius: 12px;
            box-shadow: var(--st-shadow);
            max-width: 800px;
            max-height: 80vh;
            width: 90%;
            overflow: hidden;
            display: flex;
            flex-direction: column;
          }

          .error-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 16px 20px;
            border-bottom: 1px solid var(--st-border);
            background: var(--st-error-bg);
          }

          .error-title {
            display: flex;
            align-items: center;
            gap: 10px;
            font-weight: 600;
            color: var(--st-error);
          }

          .error-title svg {
            width: 24px;
            height: 24px;
          }

          .error-close {
            background: none;
            border: none;
            cursor: pointer;
            padding: 4px;
            color: var(--st-text-muted);
            border-radius: 4px;
            display: flex;
            align-items: center;
            justify-content: center;
          }

          .error-close:hover {
            background: rgba(0, 0, 0, 0.1);
            color: var(--st-text);
          }

          .error-close svg {
            width: 20px;
            height: 20px;
          }

          .error-body {
            padding: 20px;
            overflow-y: auto;
          }

          .diagnostic {
            margin-bottom: 16px;
            padding: 12px 16px;
            border-radius: 8px;
            border-left: 4px solid;
          }

          .diagnostic.error {
            background: var(--st-error-bg);
            border-color: var(--st-error);
          }

          .diagnostic.warning {
            background: var(--st-warning-bg);
            border-color: var(--st-warning);
          }

          .diagnostic.info {
            background: var(--st-info-bg);
            border-color: var(--st-info);
          }

          .diagnostic-header {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 6px;
          }

          .diagnostic-code {
            font-weight: 600;
            font-size: 12px;
            padding: 2px 6px;
            border-radius: 4px;
          }

          .diagnostic.error .diagnostic-code {
            background: var(--st-error);
            color: white;
          }

          .diagnostic.warning .diagnostic-code {
            background: var(--st-warning);
            color: white;
          }

          .diagnostic.info .diagnostic-code {
            background: var(--st-info);
            color: white;
          }

          .diagnostic-location {
            font-size: 12px;
            color: var(--st-text-muted);
          }

          .diagnostic-message {
            color: var(--st-text);
            margin-bottom: 6px;
          }

          .diagnostic-hint {
            font-size: 13px;
            color: var(--st-text-muted);
            padding-left: 12px;
            border-left: 2px solid var(--st-border);
          }

          .diagnostic-hint code {
            background: rgba(0, 0, 0, 0.06);
            padding: 1px 5px;
            border-radius: 3px;
            cursor: pointer;
          }

          .diagnostic-hint code:hover {
            background: rgba(0, 0, 0, 0.1);
          }

          /* Error Trace */
          .diagnostic-trace {
            margin-top: 10px;
            font-size: 13px;
          }

          .diagnostic-trace summary {
            cursor: pointer;
            color: var(--st-text-muted);
            user-select: none;
            padding: 4px 0;
          }

          .diagnostic-trace summary:hover {
            color: var(--st-text);
          }

          .trace-frames {
            margin-top: 8px;
            padding-left: 12px;
            border-left: 2px solid var(--st-border);
          }

          .trace-frame {
            margin-bottom: 8px;
            padding: 6px 0;
          }

          .trace-label {
            color: var(--st-text-muted);
            font-style: italic;
          }

          .trace-location {
            display: inline-block;
            margin-left: 8px;
            background: rgba(0, 0, 0, 0.06);
            padding: 2px 6px;
            border-radius: 3px;
            cursor: pointer;
            font-family: inherit;
          }

          .trace-location:hover {
            background: rgba(0, 0, 0, 0.1);
          }

          .trace-source {
            margin: 4px 0 0 0;
            padding: 6px 10px;
            background: rgba(0, 0, 0, 0.04);
            border-radius: 4px;
            overflow-x: auto;
            font-size: 12px;
            color: var(--st-text);
            white-space: pre;
          }

          .diagnostic-location {
            cursor: pointer;
          }

          .diagnostic-location:hover {
            text-decoration: underline;
          }

          /* Warning Badge (Non-blocking) */
          .warning-badge {
            position: fixed;
            bottom: 20px;
            right: 20px;
            z-index: 99998;
          }

          .badge-button {
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 10px 16px;
            background: var(--st-warning);
            color: white;
            border: none;
            border-radius: 8px;
            cursor: pointer;
            font-family: inherit;
            font-size: 14px;
            font-weight: 500;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
            transition: transform 0.15s, box-shadow 0.15s;
          }

          .badge-button:hover {
            transform: translateY(-2px);
            box-shadow: 0 6px 16px rgba(0, 0, 0, 0.2);
          }

          .badge-button svg {
            width: 18px;
            height: 18px;
          }

          .badge-count {
            background: white;
            color: var(--st-warning);
            padding: 2px 8px;
            border-radius: 10px;
            font-size: 12px;
            font-weight: 600;
          }

          .warning-panel {
            position: fixed;
            bottom: 70px;
            right: 20px;
            width: 400px;
            max-height: 60vh;
            background: var(--st-bg);
            border-radius: 12px;
            box-shadow: var(--st-shadow);
            overflow: hidden;
            display: none;
          }

          .warning-panel.expanded {
            display: flex;
            flex-direction: column;
          }

          .panel-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 12px 16px;
            border-bottom: 1px solid var(--st-border);
            background: var(--st-warning-bg);
          }

          .panel-title {
            font-weight: 600;
            color: var(--st-warning);
          }

          .panel-body {
            padding: 12px;
            overflow-y: auto;
          }

          .panel-body .diagnostic {
            margin-bottom: 10px;
            padding: 10px 12px;
          }

          .panel-body .diagnostic:last-child {
            margin-bottom: 0;
          }

          /* Copied toast */
          .toast {
            position: fixed;
            bottom: 100px;
            left: 50%;
            transform: translateX(-50%);
            background: var(--st-text);
            color: white;
            padding: 8px 16px;
            border-radius: 6px;
            font-size: 13px;
            opacity: 0;
            transition: opacity 0.2s;
            pointer-events: none;
          }

          .toast.visible {
            opacity: 1;
          }
        </style>

        ${hasErrors ? this.renderErrorModal() : ''}
        ${hasWarnings && !hasErrors ? this.renderWarningBadge() : ''}
        <div class="toast" id="toast">Copied to clipboard!</div>
      `;
    }

    renderErrorModal() {
      return `
        <div class="error-modal" id="errorModal">
          <div class="error-content">
            <div class="error-header">
              <div class="error-title">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <line x1="12" y1="8" x2="12" y2="12"/>
                  <line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
                <span>${errors.length} Compilation Error${errors.length > 1 ? 's' : ''}</span>
              </div>
              <button class="error-close" id="closeError" title="Dismiss (errors will remain)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18"/>
                  <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
            <div class="error-body">
              ${errors.map(e => this.renderDiagnostic(e, 'error')).join('')}
              ${warnings.map(w => this.renderDiagnostic(w, 'warning')).join('')}
              ${infos.map(i => this.renderDiagnostic(i, 'info')).join('')}
            </div>
          </div>
        </div>
      `;
    }

    renderWarningBadge() {
      const count = warnings.length + infos.length;
      return `
        <div class="warning-badge">
          <button class="badge-button" id="toggleWarnings">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
              <line x1="12" y1="9" x2="12" y2="13"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
            <span>Warnings</span>
            <span class="badge-count">${count}</span>
          </button>
          <div class="warning-panel ${this.expanded ? 'expanded' : ''}" id="warningPanel">
            <div class="panel-header">
              <span class="panel-title">${count} Warning${count > 1 ? 's' : ''}</span>
              <button class="error-close" id="closeWarnings">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18"/>
                  <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
            <div class="panel-body">
              ${warnings.map(w => this.renderDiagnostic(w, 'warning')).join('')}
              ${infos.map(i => this.renderDiagnostic(i, 'info')).join('')}
            </div>
          </div>
        </div>
      `;
    }

    renderDiagnostic(diag, type) {
      const location = diag.line ? `:${diag.line}${diag.column ? ':' + diag.column : ''}` : '';
      const fileName = diag.file_path ? diag.file_path.split('/').pop() : '';
      const fullLocation = fileName ? `${fileName}${location}` : location;
      const copyPath = diag.file_path ? `${diag.file_path}${location}` : '';

      const hintHtml = diag.hint ? this.formatHint(diag.hint) : '';
      const traceHtml = diag.trace ? this.renderTrace(diag.trace) : '';

      return `
        <div class="diagnostic ${type}">
          <div class="diagnostic-header">
            <span class="diagnostic-code">${diag.code}</span>
            ${fullLocation ? `<span class="diagnostic-location" data-copy="${this.escapeAttr(copyPath)}">${fullLocation}</span>` : ''}
          </div>
          <div class="diagnostic-message">${this.escapeHtml(diag.message)}</div>
          ${hintHtml ? `<div class="diagnostic-hint">${hintHtml}</div>` : ''}
          ${traceHtml}
        </div>
      `;
    }

    renderTrace(trace) {
      if (!trace || trace.length === 0) return '';

      return `
        <details class="diagnostic-trace">
          <summary>Error trace</summary>
          <div class="trace-frames">
            ${trace.map(f => this.renderTraceFrame(f)).join('')}
          </div>
        </details>
      `;
    }

    renderTraceFrame(frame) {
      const loc = frame.line ? `:${frame.line}${frame.column ? ':' + frame.column : ''}` : '';
      const display = frame.file_path ? frame.file_path.split('/').slice(-2).join('/') + loc : '';
      const copyPath = frame.file_path ? `${frame.file_path}${loc}` : '';

      return `
        <div class="trace-frame">
          <span class="trace-label">${this.escapeHtml(frame.label)}</span>
          ${display ? `<code class="trace-location" data-copy="${this.escapeAttr(copyPath)}">${display}</code>` : ''}
          ${frame.source_line ? `<pre class="trace-source">${this.escapeHtml(frame.source_line.trim())}</pre>` : ''}
        </div>
      `;
    }

    formatHint(hint) {
      // Convert selector suggestions to clickable code elements
      return this.escapeHtml(hint).replace(
        /'([.#]?[\w-]+(?:__[\w-]+)?)'/g,
        '<code data-copy="$1">$1</code>'
      );
    }

    escapeHtml(text) {
      const div = document.createElement('div');
      div.textContent = text;
      return div.innerHTML;
    }

    escapeAttr(str) {
      return str.replace(/"/g, '&quot;').replace(/'/g, '&#39;');
    }

    setupEventListeners() {
      // Close error modal
      const closeError = this.shadowRoot.getElementById('closeError');
      if (closeError) {
        closeError.addEventListener('click', () => {
          const modal = this.shadowRoot.getElementById('errorModal');
          if (modal) modal.style.display = 'none';
        });
      }

      // Toggle warnings panel
      const toggleWarnings = this.shadowRoot.getElementById('toggleWarnings');
      if (toggleWarnings) {
        toggleWarnings.addEventListener('click', () => {
          this.expanded = !this.expanded;
          const panel = this.shadowRoot.getElementById('warningPanel');
          if (panel) {
            panel.classList.toggle('expanded', this.expanded);
          }
        });
      }

      // Close warnings panel
      const closeWarnings = this.shadowRoot.getElementById('closeWarnings');
      if (closeWarnings) {
        closeWarnings.addEventListener('click', () => {
          this.expanded = false;
          const panel = this.shadowRoot.getElementById('warningPanel');
          if (panel) panel.classList.remove('expanded');
        });
      }

      // Copy on click for code elements in hints
      this.shadowRoot.addEventListener('click', (e) => {
        const code = e.target.closest('code[data-copy]');
        if (code) {
          const text = code.dataset.copy;
          navigator.clipboard.writeText(text).then(() => {
            this.showToast();
          });
        }
      });

      // ESC to close modal
      document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
          const modal = this.shadowRoot.getElementById('errorModal');
          if (modal && modal.style.display !== 'none') {
            modal.style.display = 'none';
          }
        }
      });
    }

    showToast() {
      const toast = this.shadowRoot.getElementById('toast');
      if (toast) {
        toast.classList.add('visible');
        setTimeout(() => toast.classList.remove('visible'), 1500);
      }
    }
  }

  // Register custom element
  customElements.define('spacetime-validation', SpacetimeValidation);

  // Insert into page
  document.body.appendChild(document.createElement('spacetime-validation'));
})();
