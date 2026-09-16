/// Spacetime Dev Tools
/// Loaded only in debug mode via /__spacetime/dev/ routes.
/// This file is compiled separately from user code.

%primitive dev-flag() {
  %emit js {
    window.__ST_DEV__ = true;
  }
}

@import "./primitives/dev-hooks.st"

@import "./primitives/dev-ws.st"
@import "./primitives/dev-panel.st"
@import "./primitives/dev-inspect.st"
@import "./primitives/dev-editable.st"
@import "./primitives/dev-save.st"
@import "./primitives/dev-selector.st"
@import "./primitives/dev-collection-crud.st"
@import "./primitives/dev-media-picker.st"
@import "./primitives/dev-content-tree.st"
@import "./primitives/dev-data-editor.st"
@import "./primitives/dev-review.st"
@import "./primitives/dev-review-tab.st"

@import "./macros/data-tree.st"
@import "./macros/timelines-errors.st"
@import "./macros/panel.st"
@import "./macros/editor.st"
@import "./macros/st-properties.st"
@import "./macros/rich-editor.st"
@import "./macros/admin-panel.st"

@dev-editor

@dev-panel

@rich-editor

@admin-panel

%primitive dev-indicator() {
  %emit js {
    (function() {
      var indicator = document.createElement('div');
      indicator.id = '__spacetime-dev-indicator';
      indicator.textContent = 'ST Dev';
      indicator.style.cssText = 'position:fixed;bottom:8px;right:8px;background:#1e1e2e;color:#cdd6f4;padding:4px 12px;border-radius:6px;font-family:monospace;font-size:12px;z-index:99999;opacity:0.7;pointer-events:none;';
      if (document.body) {
        document.body.appendChild(indicator);
      } else {
        document.addEventListener('DOMContentLoaded', function() {
          document.body.appendChild(indicator);
        });
      }
    })();
  }
}
