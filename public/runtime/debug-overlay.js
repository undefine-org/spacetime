/**
 * Spacetime Debug Overlay
 *
 * Visual debugging overlay showing animation progress, properties, and easing curves.
 *
 * Usage:
 *   ST.debug.showOverlay('.hero')  // Show overlay on element
 *   ST.debug.hideOverlay()         // Hide all overlays
 */

(function() {
  'use strict';

  // Guard against multiple initialization
  if (typeof ST === 'undefined') {
    console.error('[ST.debugOverlay] Spacetime runtime not found. Load st.js first.');
    return;
  }

  if (ST.debugOverlay) {
    console.warn('[ST.debugOverlay] Debug overlay already initialized.');
    return;
  }

  // Overlay state
  const activeOverlays = new Map(); // element -> overlay data
  let overlayContainer = null;
  let rafId = null;
  let purityPanel = null;
  let purityRafId = null;

  /**
   * Create the overlay container
   */
  function createContainer() {
    if (overlayContainer) return overlayContainer;

    const container = document.createElement('div');
    container.id = 'st-debug-overlay-container';
    container.style.cssText = `
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      pointer-events: none;
      z-index: 999999;
      font-family: ui-monospace, 'Cascadia Code', 'Courier New', monospace;
    `;

    document.body.appendChild(container);
    overlayContainer = container;
    return container;
  }

  /**
   * Create an overlay panel for an element
   * @param {Element} targetElement
   * @returns {HTMLElement}
   */
  function createOverlayPanel(targetElement) {
    const panel = document.createElement('div');
    panel.className = 'st-debug-panel';
    panel.style.cssText = `
      position: absolute;
      background: rgba(0, 0, 0, 0.9);
      color: #00ff00;
      padding: 12px;
      border-radius: 6px;
      border: 1px solid #00ff00;
      font-size: 11px;
      line-height: 1.4;
      pointer-events: auto;
      box-shadow: 0 4px 16px rgba(0, 255, 0, 0.3);
      min-width: 200px;
      backdrop-filter: blur(4px);
    `;

    // Header
    const header = document.createElement('div');
    header.style.cssText = `
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 8px;
      padding-bottom: 8px;
      border-bottom: 1px solid rgba(0, 255, 0, 0.3);
    `;

    const title = document.createElement('div');
    title.style.cssText = 'font-weight: bold; color: #00ff00;';
    title.textContent = getElementSelector(targetElement);

    const closeBtn = document.createElement('button');
    closeBtn.textContent = '×';
    closeBtn.style.cssText = `
      background: none;
      border: none;
      color: #00ff00;
      font-size: 18px;
      cursor: pointer;
      padding: 0;
      width: 20px;
      height: 20px;
      line-height: 1;
      opacity: 0.7;
    `;
    closeBtn.onmouseenter = () => closeBtn.style.opacity = '1';
    closeBtn.onmouseleave = () => closeBtn.style.opacity = '0.7';
    closeBtn.onclick = () => hideOverlay(targetElement);

    header.appendChild(title);
    header.appendChild(closeBtn);

    // Progress bar
    const progressContainer = document.createElement('div');
    progressContainer.style.cssText = `
      margin-bottom: 8px;
    `;

    const progressLabel = document.createElement('div');
    progressLabel.style.cssText = 'margin-bottom: 4px; opacity: 0.7;';
    progressLabel.textContent = 'Progress:';

    const progressBarBg = document.createElement('div');
    progressBarBg.style.cssText = `
      background: rgba(0, 255, 0, 0.2);
      height: 20px;
      border-radius: 3px;
      overflow: hidden;
      position: relative;
      border: 1px solid rgba(0, 255, 0, 0.3);
    `;

    const progressBar = document.createElement('div');
    progressBar.className = 'st-debug-progress-bar';
    progressBar.style.cssText = `
      background: linear-gradient(90deg, #00ff00, #00cc00);
      height: 100%;
      width: 0%;
      transition: width 0.1s ease-out;
    `;

    const progressText = document.createElement('div');
    progressText.className = 'st-debug-progress-text';
    progressText.style.cssText = `
      position: absolute;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      font-weight: bold;
      color: #000;
      text-shadow: 0 0 2px rgba(255, 255, 255, 0.5);
      font-size: 10px;
    `;
    progressText.textContent = '0%';

    progressBarBg.appendChild(progressBar);
    progressBarBg.appendChild(progressText);
    progressContainer.appendChild(progressLabel);
    progressContainer.appendChild(progressBarBg);

    // Driver info
    const driverInfo = document.createElement('div');
    driverInfo.className = 'st-debug-driver-info';
    driverInfo.style.cssText = `
      margin-bottom: 8px;
      padding: 6px;
      background: rgba(0, 255, 0, 0.1);
      border-radius: 3px;
      font-size: 10px;
    `;

    // Properties display
    const propsContainer = document.createElement('div');
    propsContainer.className = 'st-debug-props';
    propsContainer.style.cssText = `
      font-size: 10px;
      max-height: 200px;
      overflow-y: auto;
    `;

    // Assemble panel
    panel.appendChild(header);
    panel.appendChild(progressContainer);
    panel.appendChild(driverInfo);
    panel.appendChild(propsContainer);

    return panel;
  }

  /**
   * Update overlay panel content
   * @param {Element} targetElement
   * @param {HTMLElement} panel
   */
  function updateOverlayPanel(targetElement, panel) {
    const driverInfo = ST.debug?.getDriverInfo?.(targetElement);
    const animState = ST.debug?.getAnimationState?.(targetElement);

    // Update progress bar
    const progress = driverInfo?.progress ?? 0;
    const progressBar = panel.querySelector('.st-debug-progress-bar');
    const progressText = panel.querySelector('.st-debug-progress-text');

    if (progressBar) {
      progressBar.style.width = `${progress * 100}%`;
    }
    if (progressText) {
      progressText.textContent = `${(progress * 100).toFixed(1)}%`;
      progressText.style.color = progress > 0.5 ? '#000' : '#00ff00';
    }

    // Update driver info
    const driverInfoEl = panel.querySelector('.st-debug-driver-info');
    if (driverInfoEl && driverInfo) {
      driverInfoEl.innerHTML = `
        <div><strong>Driver:</strong> ${driverInfo.type}</div>
        <div><strong>Last Update:</strong> ${new Date(driverInfo.lastUpdate).toLocaleTimeString()}</div>
      `;
    } else if (driverInfoEl) {
      driverInfoEl.innerHTML = `<div style="opacity: 0.5;">No driver active</div>`;
    }

    // Update properties
    const propsEl = panel.querySelector('.st-debug-props');
    if (propsEl) {
      const signals = ST.signals.get(targetElement);
      const computedStyle = window.getComputedStyle(targetElement);

      let propsHtml = '<div style="margin-bottom: 4px; opacity: 0.7;">Properties:</div>';

      // Show animated properties if available
      if (animState && animState.properties) {
        Object.entries(animState.properties).forEach(([prop, value]) => {
          propsHtml += `<div><strong>${prop}:</strong> ${value}</div>`;
        });
      }

      // Show relevant computed styles
      const interestingProps = [
        'opacity', 'transform', 'translateX', 'translateY', 'scale',
        'rotate', 'background', 'color', 'width', 'height'
      ];

      interestingProps.forEach(prop => {
        const value = computedStyle.getPropertyValue(prop) || computedStyle[prop];
        if (value && value !== 'none' && value !== 'auto') {
          propsHtml += `<div><strong>${prop}:</strong> ${value}</div>`;
        }
      });

      // Show signals
      if (signals) {
        propsHtml += '<div style="margin-top: 8px; opacity: 0.7;">Signals:</div>';
        Object.entries(signals).forEach(([name, data]) => {
          if (data && typeof data.v !== 'undefined') {
            const value = typeof data.v === 'object' ? JSON.stringify(data.v) : data.v;
            propsHtml += `<div><strong>$${name}:</strong> ${value}</div>`;
          }
        });
      }

      propsEl.innerHTML = propsHtml;
    }
  }

  /**
   * Position overlay panel near target element
   * @param {Element} targetElement
   * @param {HTMLElement} panel
   */
  function positionOverlay(targetElement, panel) {
    const rect = targetElement.getBoundingClientRect();
    const panelRect = panel.getBoundingClientRect();

    // Position above element by default
    let top = rect.top - panelRect.height - 10;
    let left = rect.left;

    // If not enough space above, position below
    if (top < 10) {
      top = rect.bottom + 10;
    }

    // Ensure panel stays within viewport
    if (left + panelRect.width > window.innerWidth - 10) {
      left = window.innerWidth - panelRect.width - 10;
    }
    if (left < 10) {
      left = 10;
    }

    panel.style.top = `${top}px`;
    panel.style.left = `${left}px`;
  }

  /**
   * Get a simple selector string for an element
   * @param {Element} element
   * @returns {string}
   */
  function getElementSelector(element) {
    if (element.id) return `#${element.id}`;
    if (element.className && typeof element.className === 'string') {
      const classes = element.className.trim().split(/\s+/);
      if (classes.length > 0 && classes[0]) return `.${classes[0]}`;
    }
    return element.tagName.toLowerCase();
  }

  /**
   * Animation loop to update overlays
   */
  function updateLoop() {
    activeOverlays.forEach((data, element) => {
      if (!document.body.contains(element)) {
        // Element removed from DOM
        hideOverlay(element);
        return;
      }

      updateOverlayPanel(element, data.panel);
      positionOverlay(element, data.panel);
    });

    if (activeOverlays.size > 0) {
      rafId = requestAnimationFrame(updateLoop);
    } else {
      rafId = null;
    }
  }

  /**
   * Show overlay for element(s) matching selector
   * @param {string} selector - CSS selector
   * @returns {number} Number of overlays shown
   */
  function showOverlay(selector) {
    const elements = document.querySelectorAll(selector);

    if (elements.length === 0) {
      console.warn(`[ST.debugOverlay] No elements found for selector: ${selector}`);
      return 0;
    }

    const container = createContainer();
    let count = 0;

    elements.forEach(element => {
      // Skip if already has overlay
      if (activeOverlays.has(element)) {
        return;
      }

      const panel = createOverlayPanel(element);
      container.appendChild(panel);

      activeOverlays.set(element, {
        panel,
        selector
      });

      updateOverlayPanel(element, panel);
      positionOverlay(element, panel);
      count++;
    });

    // Start update loop if not running
    if (!rafId && activeOverlays.size > 0) {
      rafId = requestAnimationFrame(updateLoop);
    }

    console.log(`[ST.debugOverlay] Showing ${count} overlay(s) for "${selector}"`);
    return count;
  }

  /**
   * Hide overlay for specific element
   * @param {Element} element
   */
  function hideOverlay(element) {
    const data = activeOverlays.get(element);
    if (!data) return;

    data.panel.remove();
    activeOverlays.delete(element);

    if (activeOverlays.size === 0 && overlayContainer) {
      overlayContainer.remove();
      overlayContainer = null;
    }
  }

  /**
   * Hide all overlays
   */
  function hideAll() {
    activeOverlays.forEach((data, element) => {
      data.panel.remove();
    });

    activeOverlays.clear();

    if (overlayContainer) {
      overlayContainer.remove();
      overlayContainer = null;
    }

    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    console.log('[ST.debugOverlay] All overlays hidden');
  }

  /**
   * Create the purity stats overlay panel
   * @returns {HTMLElement}
   */
  function createPurityPanel() {
    const panel = document.createElement('div');
    panel.id = 'st-purity-overlay';
    panel.style.cssText = `
      position: fixed;
      bottom: 20px;
      right: 20px;
      background: rgba(0, 0, 0, 0.9);
      color: #00ff00;
      padding: 16px;
      border-radius: 8px;
      border: 1px solid #00ff00;
      font-size: 12px;
      font-family: ui-monospace, 'Cascadia Code', 'Courier New', monospace;
      line-height: 1.5;
      pointer-events: auto;
      box-shadow: 0 4px 20px rgba(0, 255, 0, 0.3);
      min-width: 220px;
      backdrop-filter: blur(4px);
      z-index: 999999;
    `;

    // Header
    const header = document.createElement('div');
    header.style.cssText = `
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
      padding-bottom: 8px;
      border-bottom: 1px solid rgba(0, 255, 0, 0.3);
    `;

    const title = document.createElement('div');
    title.style.cssText = 'font-weight: bold; color: #00ff00; font-size: 13px;';
    title.textContent = 'Purity Cache';

    const closeBtn = document.createElement('button');
    closeBtn.textContent = '×';
    closeBtn.style.cssText = `
      background: none;
      border: none;
      color: #00ff00;
      font-size: 20px;
      cursor: pointer;
      padding: 0;
      width: 24px;
      height: 24px;
      line-height: 1;
      opacity: 0.7;
    `;
    closeBtn.onmouseenter = () => closeBtn.style.opacity = '1';
    closeBtn.onmouseleave = () => closeBtn.style.opacity = '0.7';
    closeBtn.onclick = hidePurity;

    header.appendChild(title);
    header.appendChild(closeBtn);

    // Stats container
    const statsContainer = document.createElement('div');
    statsContainer.className = 'st-purity-stats';

    // Hit rate bar
    const hitRateContainer = document.createElement('div');
    hitRateContainer.style.cssText = 'margin-bottom: 12px;';

    const hitRateLabel = document.createElement('div');
    hitRateLabel.style.cssText = 'margin-bottom: 4px; display: flex; justify-content: space-between;';
    hitRateLabel.innerHTML = '<span>Hit Rate</span><span class="st-purity-hit-rate">0%</span>';

    const hitRateBarBg = document.createElement('div');
    hitRateBarBg.style.cssText = `
      background: rgba(255, 0, 0, 0.3);
      height: 8px;
      border-radius: 4px;
      overflow: hidden;
    `;

    const hitRateBar = document.createElement('div');
    hitRateBar.className = 'st-purity-hit-bar';
    hitRateBar.style.cssText = `
      background: linear-gradient(90deg, #00ff00, #00cc00);
      height: 100%;
      width: 0%;
      transition: width 0.2s ease-out;
      border-radius: 4px;
    `;

    hitRateBarBg.appendChild(hitRateBar);
    hitRateContainer.appendChild(hitRateLabel);
    hitRateContainer.appendChild(hitRateBarBg);

    // Stats grid
    const statsGrid = document.createElement('div');
    statsGrid.className = 'st-purity-grid';
    statsGrid.style.cssText = `
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 8px;
      font-size: 11px;
    `;

    statsGrid.innerHTML = `
      <div style="opacity: 0.7;">Hits</div>
      <div class="st-purity-hits" style="text-align: right; font-weight: bold;">0</div>
      <div style="opacity: 0.7;">Misses</div>
      <div class="st-purity-misses" style="text-align: right; font-weight: bold;">0</div>
      <div style="opacity: 0.7;">Invalidations</div>
      <div class="st-purity-invalidations" style="text-align: right; font-weight: bold;">0</div>
      <div style="opacity: 0.7;">Anim Starts</div>
      <div class="st-purity-anim-starts" style="text-align: right; font-weight: bold;">0</div>
      <div style="opacity: 0.7;">Anim Ends</div>
      <div class="st-purity-anim-ends" style="text-align: right; font-weight: bold;">0</div>
    `;

    // Animating elements section
    const animatingSection = document.createElement('div');
    animatingSection.style.cssText = `
      margin-top: 12px;
      padding-top: 8px;
      border-top: 1px solid rgba(0, 255, 0, 0.3);
    `;

    const animatingLabel = document.createElement('div');
    animatingLabel.style.cssText = 'opacity: 0.7; margin-bottom: 4px;';
    animatingLabel.textContent = 'Currently Animating:';

    const animatingList = document.createElement('div');
    animatingList.className = 'st-purity-animating';
    animatingList.style.cssText = `
      font-size: 10px;
      max-height: 60px;
      overflow-y: auto;
      color: #ffcc00;
    `;
    animatingList.textContent = 'none';

    animatingSection.appendChild(animatingLabel);
    animatingSection.appendChild(animatingList);

    // Reset button
    const resetBtn = document.createElement('button');
    resetBtn.textContent = 'Reset Stats';
    resetBtn.style.cssText = `
      margin-top: 12px;
      width: 100%;
      padding: 6px;
      background: rgba(0, 255, 0, 0.2);
      border: 1px solid rgba(0, 255, 0, 0.5);
      color: #00ff00;
      border-radius: 4px;
      cursor: pointer;
      font-family: inherit;
      font-size: 11px;
    `;
    resetBtn.onmouseenter = () => resetBtn.style.background = 'rgba(0, 255, 0, 0.3)';
    resetBtn.onmouseleave = () => resetBtn.style.background = 'rgba(0, 255, 0, 0.2)';
    resetBtn.onclick = () => {
      if (ST._purity) {
        ST._purity.resetStats();
      }
    };

    // Assemble panel
    statsContainer.appendChild(hitRateContainer);
    statsContainer.appendChild(statsGrid);
    statsContainer.appendChild(animatingSection);
    statsContainer.appendChild(resetBtn);

    panel.appendChild(header);
    panel.appendChild(statsContainer);

    return panel;
  }

  /**
   * Update the purity panel with current stats
   */
  function updatePurityPanel() {
    if (!purityPanel || !ST._purity) return;

    const stats = ST._purity.getStats();
    const total = stats.hits + stats.misses;
    const hitRate = total > 0 ? (stats.hits / total * 100) : 0;

    // Update hit rate
    const hitRateEl = purityPanel.querySelector('.st-purity-hit-rate');
    const hitBar = purityPanel.querySelector('.st-purity-hit-bar');
    if (hitRateEl) hitRateEl.textContent = hitRate.toFixed(1) + '%';
    if (hitBar) {
      hitBar.style.width = hitRate + '%';
      // Color based on hit rate
      if (hitRate >= 80) {
        hitBar.style.background = 'linear-gradient(90deg, #00ff00, #00cc00)';
      } else if (hitRate >= 50) {
        hitBar.style.background = 'linear-gradient(90deg, #ffcc00, #ff9900)';
      } else {
        hitBar.style.background = 'linear-gradient(90deg, #ff6600, #ff0000)';
      }
    }

    // Update stats
    const hitsEl = purityPanel.querySelector('.st-purity-hits');
    const missesEl = purityPanel.querySelector('.st-purity-misses');
    const invalsEl = purityPanel.querySelector('.st-purity-invalidations');
    const animStartsEl = purityPanel.querySelector('.st-purity-anim-starts');
    const animEndsEl = purityPanel.querySelector('.st-purity-anim-ends');

    if (hitsEl) hitsEl.textContent = stats.hits.toLocaleString();
    if (missesEl) missesEl.textContent = stats.misses.toLocaleString();
    if (invalsEl) invalsEl.textContent = stats.invalidations.toLocaleString();
    if (animStartsEl) animStartsEl.textContent = stats.animationStarts.toLocaleString();
    if (animEndsEl) animEndsEl.textContent = stats.animationEnds.toLocaleString();

    // Update animating list
    const animatingEl = purityPanel.querySelector('.st-purity-animating');
    if (animatingEl) {
      const animating = [...ST._purity.animating];
      if (animating.length > 0) {
        animatingEl.textContent = animating.join(', ');
        animatingEl.style.color = '#ffcc00';
      } else {
        animatingEl.textContent = 'none';
        animatingEl.style.color = 'rgba(0, 255, 0, 0.5)';
      }
    }
  }

  /**
   * Purity panel update loop
   */
  function purityUpdateLoop() {
    updatePurityPanel();
    purityRafId = requestAnimationFrame(purityUpdateLoop);
  }

  /**
   * Show the purity stats overlay
   */
  function showPurity() {
    if (purityPanel) {
      console.log('[ST.debugOverlay] Purity overlay already visible');
      return purityPanel;
    }

    if (!ST._purity) {
      console.error('[ST.debugOverlay] Purity system not loaded. Include purity.js');
      return null;
    }

    purityPanel = createPurityPanel();
    document.body.appendChild(purityPanel);

    // Start update loop
    purityRafId = requestAnimationFrame(purityUpdateLoop);

    console.log('[ST.debugOverlay] Purity overlay shown');
    return purityPanel;
  }

  /**
   * Hide the purity stats overlay
   */
  function hidePurity() {
    if (!purityPanel) return;

    purityPanel.remove();
    purityPanel = null;

    if (purityRafId) {
      cancelAnimationFrame(purityRafId);
      purityRafId = null;
    }

    console.log('[ST.debugOverlay] Purity overlay hidden');
  }

  // Public API
  const DebugOverlayAPI = {
    show: showOverlay,
    hide: hideOverlay,
    hideAll: hideAll,
    getActiveOverlays: () => new Map(activeOverlays),
    showPurity: showPurity,
    hidePurity: hidePurity
  };

  // Attach to ST namespace
  ST.debugOverlay = DebugOverlayAPI;

  console.log('[ST.debugOverlay] Debug overlay loaded. Call ST.debug.showOverlay(selector) to show.');

})();
