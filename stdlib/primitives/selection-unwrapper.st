/// Selection Unwrapper Primitive
///
/// Removes highlight wrapper spans from selected text, preserving the text content.
/// Used to clear formatting/highlighting from inline text.
///
/// @param &el - Element containing the selection
/// @param trigger - Event that triggers the unwrapping action
/// @param class_name - CSS class of wrapper spans to remove
///
/// @example
/// selection-unwrapper(&self, trigger: __unwrap_handwritten, class_name: "jal-handwritten")

%primitive selection-unwrapper(&el, trigger: ident, class_name: string) {
  %emit js {
    const unwrapHandler = () => {
      const selection = window.getSelection();

      // If there's a selection, unwrap within the selection
      if (selection && !selection.isCollapsed && selection.rangeCount > 0) {
        const range = selection.getRangeAt(0);

        // Check if selection is within our element
        if (!%&el.contains(range.commonAncestorContainer)) {
          return;
        }

        // Find all wrapper spans within the selection
        const container = range.commonAncestorContainer;
        const wrappers = [];

        if (container.nodeType === Node.ELEMENT_NODE) {
          // If the container itself is a wrapper span, include it
          if (container.classList && container.classList.contains(%class_name)) {
            wrappers.push(container);
          }
          // Find all descendant wrapper spans
          wrappers.push(...container.querySelectorAll('span.' + %class_name));
        } else if (container.nodeType === Node.TEXT_NODE) {
          // Check if parent is a wrapper span
          const parent = container.parentElement;
          if (parent && parent.classList && parent.classList.contains(%class_name)) {
            wrappers.push(parent);
          }
        }

        // Unwrap each wrapper span
        wrappers.forEach(wrapper => {
          const parent = wrapper.parentNode;
          while (wrapper.firstChild) {
            parent.insertBefore(wrapper.firstChild, wrapper);
          }
          parent.removeChild(wrapper);
          // Normalize to merge adjacent text nodes
          parent.normalize();
        });

      } else {
        // No selection: unwrap all wrapper spans in the entire element
        const wrappers = %&el.querySelectorAll('span.' + %class_name);

        wrappers.forEach(wrapper => {
          const parent = wrapper.parentNode;
          while (wrapper.firstChild) {
            parent.insertBefore(wrapper.firstChild, wrapper);
          }
          parent.removeChild(wrapper);
          // Normalize to merge adjacent text nodes
          parent.normalize();
        });
      }

      // Clear selection after unwrapping
      if (selection) {
        selection.removeAllRanges();
      }
    };

    %&el.addEventListener(%trigger, unwrapHandler);

    %cleanup {
      %&el.removeEventListener(%trigger, unwrapHandler);
    }
  }

  %exports {}
}
