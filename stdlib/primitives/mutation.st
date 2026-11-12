/// Mutation Observer Primitive
///
/// Wraps the MutationObserver API to detect DOM changes.
/// Useful for reacting to dynamic content changes.

%primitive mutation(
  /// The element to observe
  &el,
  /// Observe child list changes
  children: bool = true,
  /// Observe attribute changes
  attributes: bool = false,
  /// Observe text content changes
  characterData: bool = false,
  /// Observe entire subtree
  subtree: bool = false,
  /// List of attributes to observe (when attributes=true)
  attributeFilter: string[]? = none,
  attributeOldValue: bool = false,
  characterDataOldValue: bool = false
) {
  %emit js {
    const observer = new MutationObserver((mutations) => {
      const added = [];
      const removed = [];
      const changed = [];

      for (const mutation of mutations) {
        switch (mutation.type) {
          case 'childList':
            added.push(...Array.from(mutation.addedNodes));
            removed.push(...Array.from(mutation.removedNodes));
            break;
          case 'attributes':
            changed.push({
              type: 'attribute',
              target: mutation.target,
              attributeName: mutation.attributeName,
              oldValue: mutation.oldValue,
              newValue: mutation.target.getAttribute(mutation.attributeName)
            });
            break;
          case 'characterData':
            changed.push({
              type: 'characterData',
              target: mutation.target,
              oldValue: mutation.oldValue,
              newValue: mutation.target.textContent
            });
            break;
        }
      }

      %yield added -> $added;
      %yield removed -> $removed;
      %yield changed -> $changed;
      %yield mutations -> $mutations;
      %yield added.length > 0 || removed.length > 0 || changed.length > 0 -> $hasMutations;
    });

    const config = {
      childList: %children,
      attributes: %attributes,
      characterData: %characterData,
      subtree: %subtree,
      attributeOldValue: %attributeOldValue,
      characterDataOldValue: %characterDataOldValue
    };

    if (%attributeFilter !== null) {
      config.attributeFilter = %attributeFilter;
    }

    observer.observe(%&el, config);

    %cleanup {
      observer.disconnect();
    }
  }

  %exports {
    /// Nodes added in this mutation
    $added: Node[]                // newly added nodes
    /// Nodes removed in this mutation
    $removed: Node[]              // removed nodes
    /// Attribute/characterData changes
    $changed: MutationChange[]    // attribute/text changes
    /// Raw mutation records
    $mutations: MutationRecord[]  // raw mutation records
    $hasMutations: bool           // true when any mutation occurred
  }
}
