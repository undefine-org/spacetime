/// Selection Wrapper Primitive
///
/// Wraps selected text within an element in a styled span.
/// Used for inline text highlighting (handwritten, accent, highlight, quote).
///
/// @param &el - Element containing the selection
/// @param trigger - Event that triggers the wrapping action
/// @param class_name - CSS class to apply to the wrapper span
///
/// @example
/// selection-wrapper(&self, trigger: __wrap_handwritten, class_name: "jal-handwritten")

%primitive selection-wrapper(&el, trigger: ident, class_name: string) {
  %emit js {
    const wrapHandler = () => {
      const selection = window.getSelection();

      // Only proceed if there's a selection with content
      if (!selection || selection.isCollapsed || selection.rangeCount === 0) {
        return;
      }

      const range = selection.getRangeAt(0);

      // Check if selection is within our element
      if (!%&el.contains(range.commonAncestorContainer)) {
        return;
      }

      // Don't wrap if selection is already wrapped in the same class
      const ancestor = range.commonAncestorContainer;
      const parentSpan = ancestor.nodeType === Node.TEXT_NODE
        ? ancestor.parentElement
        : ancestor;

      if (parentSpan && parentSpan.classList && parentSpan.classList.contains(%class_name)) {
        return;
      }

      // Create wrapper span
      const wrapper = document.createElement('span');
      wrapper.className = %class_name;

      try {
        // Wrap the selection
        range.surroundContents(wrapper);

        // Clear the selection
        selection.removeAllRanges();

        // Place cursor after the wrapped content
        const newRange = document.createRange();
        newRange.setStartAfter(wrapper);
        newRange.collapse(true);
        selection.addRange(newRange);
      } catch (e) {
        // If surroundContents fails (e.g., partial element selection),
        // extract contents and insert wrapped version
        const fragment = range.extractContents();
        wrapper.appendChild(fragment);
        range.insertNode(wrapper);

        // Clear selection
        selection.removeAllRanges();
      }
    };

    %&el.addEventListener(%trigger, wrapHandler);

    %cleanup {
      %&el.removeEventListener(%trigger, wrapHandler);
    }
  }

  %exports {}
}
