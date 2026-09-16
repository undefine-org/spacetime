/// Mutation Observer Primitive
///
/// Wraps the MutationObserver API to detect DOM changes.
/// Useful for reacting to dynamic content changes.
///
/// @param &el - The element to observe
/// @param children - Observe child list changes, default true
/// @param attributes - Observe attribute changes, default false
/// @param characterData - Observe text content changes, default false
/// @param subtree - Observe entire subtree, default false
/// @param attributeFilter - List of attributes to observe (when attributes=true)
///
/// @yields $added - Nodes added in this mutation
/// @yields $removed - Nodes removed in this mutation
/// @yields $changed - Attribute/characterData changes
/// @yields $mutations - Raw mutation records

%primitive mutation(
  &el,
  children: bool = true,
  attributes: bool = false,
  characterData: bool = false,
  subtree: bool = false,
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
    $added: Node[]                // newly added nodes
    $removed: Node[]              // removed nodes
    $changed: MutationChange[]    // attribute/text changes
    $mutations: MutationRecord[]  // raw mutation records
    $hasMutations: bool           // true when any mutation occurred
  }
}
