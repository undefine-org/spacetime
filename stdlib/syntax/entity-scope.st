/// Entity Scope Registration
///
/// Registers a named entity scope (`&name { @directive... }`) at runtime.
/// Registers the entity in the general `window.__stWorld.byName` registry so
/// cross-entity facet paths (Wave B: `&name.component.facet`) can resolve the
/// entity and its components (FEAT-142).
///
/// Syntax (lowered STRUCTURALLY by the Rust parser, not via a flat %form match):
///   &name { @directive... }
///
/// The per-entity MARKER element (`<st-entity class="st-entity-<name>">`) is
/// injected into the page HTML by the Rust lowering (the compiler owns the
/// structural entity scope, so it owns the structural marker). A CLASS selector
/// is used because a LEADING attribute selector does not emit directive JS in the
/// selector-init path. This primitive only REGISTERS the entity against that
/// marker at runtime; the interior component directives bind to `.st-entity-<name>`.

%primitive entity-scope-impl(
  name: ident
) %uses entity-world {

  %emit js {
    (function() {
      const NAME = '%name';
      const SELECTOR = '.st-entity-' + NAME;
      // The marker is injected into the static HTML by the compiler. Fall back to
      // creating one only if it is somehow absent (defensive), so the registry
      // entry always has a real node.
      let el = document.querySelector(SELECTOR);
      if (!el) {
        el = document.createElement('st-entity');
        el.className = 'st-entity-' + NAME;
        el.setAttribute('data-st-entity', NAME);
        el.style.display = 'none';
        document.body.appendChild(el);
      }
      // Registry init guaranteed by the entity-world module prelude
      // (`%uses entity-world`) — the inline re-init here was the drift shape.
      window.__stWorld.byName[NAME] = {
        name: NAME,
        el: el,
        components: {}
      };
    })();
  }
}
