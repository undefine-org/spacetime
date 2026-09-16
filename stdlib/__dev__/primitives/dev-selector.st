/// Dev Selector Primitive
///
/// Element picker/inspector tool for the dev panel, similar to Chrome/Firefox
/// DevTools' "inspect element" mode. When activated, hovering over page elements
/// shows a semi-transparent blue overlay with a tooltip showing the element's
/// tag, class, and Spacetime annotations (locale key, animation name, data binding).
/// Clicking locks the selection.
///
/// Activation: window.__stDevSelector.activate() or Ctrl+Shift+C
/// Deactivation: window.__stDevSelector.deactivate() or Escape key
///
/// @yields $active - Whether selector mode is active
/// @yields $selectedEl - Currently selected DOM element (or null)
/// @yields $selectedInfo - Object with tag, classes, and Spacetime annotations

%primitive dev-selector() {
  %emit js {
    (function() {
      var active = false;
      var selectedEl = null;
      var selectedInfo = null;
      var overlayEl = null;
      var tooltipEl = null;
      var hoveredEl = null;

      // --- Create overlay element ---
      function ensureOverlay() {
        if (overlayEl) return;
        overlayEl = document.createElement('div');
        overlayEl.id = '__st-dev-selector-overlay';
        overlayEl.style.cssText = 'position:fixed;pointer-events:none;z-index:99998;background:rgba(137,180,250,0.15);border:2px solid rgba(137,180,250,0.6);border-radius:2px;display:none;transition:top 0.05s ease,left 0.05s ease,width 0.05s ease,height 0.05s ease;';
        document.body.appendChild(overlayEl);
      }

      // --- Create tooltip element ---
      function ensureTooltip() {
        if (tooltipEl) return;
        tooltipEl = document.createElement('div');
        tooltipEl.id = '__st-dev-selector-tooltip';
        tooltipEl.style.cssText = 'position:fixed;pointer-events:none;z-index:99999;background:rgba(30,30,46,0.92);color:#cdd6f4;font-family:monospace;font-size:11px;padding:3px 8px;border-radius:4px;white-space:nowrap;display:none;max-width:400px;overflow:hidden;text-overflow:ellipsis;box-shadow:0 2px 8px rgba(0,0,0,0.3);';
        document.body.appendChild(tooltipEl);
      }

      // --- Spacetime info discovery for an element ---
      function getSpacetimeInfo(el) {
        if (!el || !el.tagName) return null;

        var tag = el.tagName.toLowerCase();
        var classNameStr = typeof el.className === 'string' ? el.className : (el.className && el.className.baseVal ? el.className.baseVal : '');
        var classes = classNameStr ? classNameStr.split(' ').filter(function(c) { return c.length > 0; }) : [];

        var info = {
          tag: tag,
          classes: classes,
          localeKey: null,
          dataBinding: null,
          dataSource: null,
          timeline: null,
          stateInfo: null
        };

        // Check data-t attribute (locale key)
        var dataT = el.getAttribute('data-t');
        if (dataT) {
          info.localeKey = dataT;
        }

        // Check data-st-bind (data binding)
        var bind = el.getAttribute('data-st-bind');
        if (bind) {
          info.dataBinding = bind;
        }

        // Check data-st-source (@each source)
        var source = el.getAttribute('data-st-source');
        if (source) {
          info.dataSource = source;
        }

        // Walk ancestors for inherited bindings
        if (!info.dataBinding && !info.dataSource) {
          var current = el.parentElement;
          while (current && current !== document.body) {
            if (current.hasAttribute('data-st-source')) {
              info.dataSource = current.getAttribute('data-st-source') + ' (inherited)';
              break;
            }
            if (current.hasAttribute('data-st-bind')) {
              info.dataBinding = current.getAttribute('data-st-bind') + ' (inherited)';
              break;
            }
            current = current.parentElement;
          }
        }

        // Query __ST_DEBUG__ for timeline info
        var dbg = window.__ST_DEBUG__;
        if (dbg && typeof dbg.getTimelines === 'function') {
          try {
            var timelines = dbg.getTimelines();
            if (Array.isArray(timelines)) {
              for (var i = 0; i < timelines.length; i++) {
                var tl = timelines[i];
                if (tl.scope) {
                  // Check if element matches timeline scope selector
                  try {
                    if (el.matches && el.matches(tl.scope)) {
                      info.timeline = tl.id + ' (' + tl.type + ', ' + Math.round(tl.progress * 100) + '%)';
                      break;
                    }
                  } catch (e) {
                    // Invalid selector, skip
                  }
                }
              }
            }
          } catch (e) {}
        }

        // Query __ST_DEBUG__ for state machine info
        if (dbg && typeof dbg.getStateMachines === 'function') {
          try {
            var sms = dbg.getStateMachines();
            if (Array.isArray(sms)) {
              var elId = el.id ? '#' + el.id : null;
              var elClass = classNameStr ? el.tagName.toLowerCase() + '.' + classes.slice(0, 2).join('.') : null;
              for (var j = 0; j < sms.length; j++) {
                var sm = sms[j];
                if ((elId && sm.element === elId) || (elClass && sm.element === elClass)) {
                  info.stateInfo = 'state: ' + sm.current;
                  break;
                }
              }
            }
          } catch (e) {}
        }

        return info;
      }

      // --- Format tooltip text from info ---
      function formatTooltip(info) {
        if (!info) return '';
        var parts = [];

        // Tag + classes
        var label = info.tag;
        if (info.classes.length > 0) {
          label = label + '.' + info.classes.slice(0, 3).join('.');
        }
        parts.push(label);

        // Spacetime annotations
        if (info.localeKey) {
          parts.push('t=' + info.localeKey);
        }
        if (info.dataBinding) {
          parts.push('bind=' + info.dataBinding);
        }
        if (info.dataSource) {
          parts.push('src=' + info.dataSource);
        }
        if (info.timeline) {
          parts.push('tl=' + info.timeline);
        }
        if (info.stateInfo) {
          parts.push(info.stateInfo);
        }

        return parts.join(' | ');
      }

      // --- Position overlay over element ---
      function positionOverlay(el) {
        if (!el || !overlayEl || !tooltipEl) return;

        var rect = el.getBoundingClientRect();
        overlayEl.style.top = rect.top + 'px';
        overlayEl.style.left = rect.left + 'px';
        overlayEl.style.width = rect.width + 'px';
        overlayEl.style.height = rect.height + 'px';
        overlayEl.style.display = 'block';

        // Position tooltip above the overlay, or below if no room
        var info = getSpacetimeInfo(el);
        var text = formatTooltip(info);
        tooltipEl.textContent = text;
        tooltipEl.style.display = 'block';

        var tooltipTop = rect.top - 28;
        if (tooltipTop < 4) {
          tooltipTop = rect.bottom + 4;
        }
        var tooltipLeft = rect.left;
        if (tooltipLeft < 4) {
          tooltipLeft = 4;
        }
        // Prevent tooltip from overflowing right edge
        var maxLeft = window.innerWidth - 410;
        if (tooltipLeft > maxLeft && maxLeft > 4) {
          tooltipLeft = maxLeft;
        }
        tooltipEl.style.top = tooltipTop + 'px';
        tooltipEl.style.left = tooltipLeft + 'px';
      }

      // --- Hide overlay ---
      function hideOverlay() {
        if (overlayEl) overlayEl.style.display = 'none';
        if (tooltipEl) tooltipEl.style.display = 'none';
      }

      // --- Is element part of the selector UI? ---
      function isSelectorElement(el) {
        if (!el) return false;
        if (el === overlayEl || el === tooltipEl) return true;
        if (el.id === '__st-dev-selector-overlay') return true;
        if (el.id === '__st-dev-selector-tooltip') return true;
        if (el.id === '__spacetime-dev-panel') return true;
        if (el.id === '__spacetime-dev-indicator') return true;
        // Check if inside the dev panel shadow host
        var parent = el.parentElement;
        while (parent) {
          if (parent.id === '__spacetime-dev-panel') return true;
          if (parent.id === '__st-dev-selector-overlay') return true;
          if (parent.id === '__st-dev-selector-tooltip') return true;
          parent = parent.parentElement;
        }
        return false;
      }

      // --- Event handlers ---
      function handleMouseMove(e) {
        if (!active) return;
        var target = document.elementFromPoint(e.clientX, e.clientY);
        if (!target || isSelectorElement(target)) return;
        hoveredEl = target;
        positionOverlay(target);
      }

      function handleClick(e) {
        if (!active) return;
        var target = document.elementFromPoint(e.clientX, e.clientY);
        if (!target || isSelectorElement(target)) return;

        e.preventDefault();
        e.stopPropagation();

        // Lock selection
        selectedEl = target;
        selectedInfo = getSpacetimeInfo(target);

        %yield selectedEl -> $selectedEl;
        %yield selectedInfo -> $selectedInfo;

        // Keep overlay visible on selected element
        positionOverlay(target);

        // Send InspectElement via WS for server-side context
        if (window.__stDevWs && window.__stDevWs.send) {
          var selector = '';
          if (selectedInfo) {
            if (selectedInfo.classes && selectedInfo.classes.length > 0) {
              selector = '.' + selectedInfo.classes.join('.');
            } else if (target.id) {
              selector = '#' + target.id;
            } else {
              selector = selectedInfo.tag || target.tagName.toLowerCase();
            }
          }
          if (selector) {
            window.__stDevWs.send({
              type: 'InspectElement',
              selector: selector,
              file_hint: null
            });
          }
        }
      }

      function handleKeydown(e) {
        // Escape deactivates selector mode
        if (e.key === 'Escape' && active) {
          e.preventDefault();
          e.stopPropagation();
          deactivate();
          return;
        }

        // Ctrl+Shift+C toggles selector mode
        if (e.key === 'C' && e.ctrlKey && e.shiftKey && !e.altKey && !e.metaKey) {
          e.preventDefault();
          e.stopPropagation();
          toggle();
        }
      }

      // --- Activation/deactivation ---
      function activate() {
        if (active) return;
        active = true;
        ensureOverlay();
        ensureTooltip();

        document.addEventListener('mousemove', handleMouseMove, true);
        document.addEventListener('click', handleClick, true);

        // Change cursor to crosshair
        document.documentElement.style.cursor = 'crosshair';

        %yield active -> $active;
      }

      function deactivate() {
        if (!active) return;
        active = false;
        hoveredEl = null;

        document.removeEventListener('mousemove', handleMouseMove, true);
        document.removeEventListener('click', handleClick, true);

        hideOverlay();

        // Restore cursor
        document.documentElement.style.cursor = '';

        %yield active -> $active;
      }

      function toggle() {
        if (active) {
          deactivate();
        } else {
          activate();
        }
      }

      function getSelectedInfo() {
        return {
          element: selectedEl,
          info: selectedInfo
        };
      }

      // --- Global keyboard listener (always active for Ctrl+Shift+C) ---
      document.addEventListener('keydown', handleKeydown, true);

      // --- Global API ---
      window.__stDevSelector = {
        activate: activate,
        deactivate: deactivate,
        toggle: toggle,
        getSelectedInfo: getSelectedInfo,
        getSpacetimeInfo: getSpacetimeInfo
      };

      // Initial yields
      %yield active -> $active;
      %yield selectedEl -> $selectedEl;
      %yield selectedInfo -> $selectedInfo;

      %cleanup {
        deactivate();
        document.removeEventListener('keydown', handleKeydown, true);
        if (overlayEl && overlayEl.parentNode) {
          overlayEl.parentNode.removeChild(overlayEl);
          overlayEl = null;
        }
        if (tooltipEl && tooltipEl.parentNode) {
          tooltipEl.parentNode.removeChild(tooltipEl);
          tooltipEl = null;
        }
        window.__stDevSelector = null;
      }
    })();
  }

  %exports {
    $active: bool
    $selectedEl: object
    $selectedInfo: object
  }
}
