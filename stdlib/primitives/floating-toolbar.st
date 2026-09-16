/// Floating Toolbar Primitive
///
/// Creates and manages a floating WYSIWYG toolbar that appears near text selections.
/// Positions above or below the selection and dispatches events when buttons are clicked.
///
/// @param &el - Element that the toolbar is associated with
/// @param position - Toolbar position: "above", "below", or "auto"
/// @param buttons - Array of button identifiers (e.g., ["handwritten", "accent", "highlight", "quote"])
///
/// @example
/// floating-toolbar(&self, position: "above", buttons: ["handwritten", "accent", "highlight", "quote"])

%primitive floating-toolbar(&el, position: string = "above", buttons: array) {
  %emit js {
    let toolbar = null;
    let isToolbarVisible = false;

    // Create toolbar DOM structure
    const createToolbar = () => {
      const tb = document.createElement('div');
      tb.className = 'st-edit-toolbar';
      tb.style.cssText = `
        position: absolute;
        display: none;
        background: white;
        border: 1px solid #ccc;
        border-radius: 4px;
        padding: 4px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.15);
        z-index: 10000;
        gap: 2px;
      `;

      // Button configurations
      const buttonConfigs = {
        handwritten: { label: 'Handwritten', icon: '✍️' },
        accent: { label: 'Accent', icon: '💡' },
        highlight: { label: 'Highlight', icon: '🖍️' },
        quote: { label: 'Quote', icon: '💬' },
        clear: { label: 'Clear', icon: '✖️' }
      };

      // Create buttons from the provided array
      %for $btn in %buttons {
        const action = %$btn;
        const config = buttonConfigs[action] || { label: action, icon: '•' };

        const button = document.createElement('button');
        button.type = 'button';
        button.setAttribute('data-action', action);
        button.setAttribute('aria-label', config.label);
        button.textContent = config.icon;
        button.style.cssText = `
          background: transparent;
          border: 1px solid transparent;
          border-radius: 3px;
          padding: 4px 8px;
          cursor: pointer;
          font-size: 16px;
          transition: background 0.2s;
        `;

        button.addEventListener('mouseenter', () => {
          button.style.background = '#f0f0f0';
        });

        button.addEventListener('mouseleave', () => {
          button.style.background = 'transparent';
        });

        button.addEventListener('mousedown', (e) => {
          e.preventDefault(); // Prevent losing focus/selection
        });

        button.addEventListener('click', (e) => {
          e.preventDefault();
          e.stopPropagation();

          // Dispatch custom event for this action
          const eventName = action === 'clear'
            ? '__unwrap_all'  // Clear dispatches unwrap for all highlights
            : '__wrap_' + action;

          %&el.dispatchEvent(new CustomEvent(eventName, { bubbles: false }));

          // Hide toolbar after action
          hideToolbar();
        });

        tb.appendChild(button);
      }

      document.body.appendChild(tb);
      return tb;
    };

    // Position toolbar relative to selection
    const positionToolbar = () => {
      const selection = window.getSelection();
      if (!selection || selection.rangeCount === 0 || !toolbar) {
        return;
      }

      const range = selection.getRangeAt(0);
      const rect = range.getBoundingClientRect();

      const toolbarRect = toolbar.getBoundingClientRect();
      const scrollX = window.pageXOffset || document.documentElement.scrollLeft;
      const scrollY = window.pageYOffset || document.documentElement.scrollTop;

      let top, left;

      // Calculate horizontal center
      left = rect.left + scrollX + (rect.width / 2) - (toolbarRect.width / 2);

      // Ensure toolbar doesn't go off-screen horizontally
      const maxLeft = window.innerWidth + scrollX - toolbarRect.width - 10;
      const minLeft = scrollX + 10;
      left = Math.max(minLeft, Math.min(left, maxLeft));

      // Calculate vertical position based on preference
      const positionPref = %position;
      const spaceAbove = rect.top;
      const spaceBelow = window.innerHeight - rect.bottom;

      if (positionPref === 'above') {
        top = rect.top + scrollY - toolbarRect.height - 8;
      } else if (positionPref === 'below') {
        top = rect.bottom + scrollY + 8;
      } else {
        // 'auto': choose based on available space
        if (spaceAbove > toolbarRect.height + 8 || spaceAbove > spaceBelow) {
          top = rect.top + scrollY - toolbarRect.height - 8;
        } else {
          top = rect.bottom + scrollY + 8;
        }
      }

      toolbar.style.left = left + 'px';
      toolbar.style.top = top + 'px';
    };

    // Show toolbar
    const showToolbar = () => {
      if (!toolbar) {
        toolbar = createToolbar();
      }

      toolbar.style.display = 'flex';
      positionToolbar();
      isToolbarVisible = true;
    };

    // Hide toolbar
    const hideToolbar = () => {
      if (toolbar) {
        toolbar.style.display = 'none';
        isToolbarVisible = false;
      }
    };

    // Handle selection changes
    const handleSelectionChange = () => {
      const selection = window.getSelection();

      // Show toolbar only if:
      // 1. There's a non-collapsed selection
      // 2. Selection is within our element
      // 3. Element is in editing state (has contenteditable="true")
      if (selection &&
          !selection.isCollapsed &&
          selection.rangeCount > 0 &&
          %&el.contentEditable === 'true') {

        const range = selection.getRangeAt(0);

        if (%&el.contains(range.commonAncestorContainer)) {
          // Small delay to ensure selection is stable
          setTimeout(showToolbar, 50);
        } else {
          hideToolbar();
        }
      } else {
        hideToolbar();
      }
    };

    // Listen for selection changes
    document.addEventListener('selectionchange', handleSelectionChange);

    // Update position on scroll/resize
    const handlePositionUpdate = () => {
      if (isToolbarVisible) {
        positionToolbar();
      }
    };

    window.addEventListener('scroll', handlePositionUpdate, true);
    window.addEventListener('resize', handlePositionUpdate);

    // Hide toolbar when element exits editing state
    const handleStateExit = (e) => {
      if (e.detail.from === 'editing') {
        hideToolbar();
      }
    };

    %&el.addEventListener('state.exit', handleStateExit);

    %cleanup {
      document.removeEventListener('selectionchange', handleSelectionChange);
      window.removeEventListener('scroll', handlePositionUpdate, true);
      window.removeEventListener('resize', handlePositionUpdate);
      %&el.removeEventListener('state.exit', handleStateExit);

      if (toolbar && toolbar.parentNode) {
        toolbar.parentNode.removeChild(toolbar);
      }
    }
  }

  %exports {}
}
