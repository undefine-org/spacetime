/// Entity World Module Primitive (PLAN-133 single-consumer %uses wave)
///
/// Owns the entity registry (`window.__stWorld.byName`) and the facet
/// resolver (`ST.worldFacet(entity, component, facet)`) — moved verbatim
/// out of entity-scope-impl's prelude. Cross-entity facet paths
/// (`&entity.component.facet`, FEAT-142) resolve here.
%primitive entity-world() {
  %emit prelude-js {
    // General entity registry. A facet path `&entity.component.facet` resolves
    // against `window.__stWorld.byName[entity]`. Installed once (idempotent).
    window.__stWorld = window.__stWorld || { byName: {} };

    // FEAT-142 WAVE C: the facet resolver `ST.worldFacet(entity, component, facet)`.
    // Reads a published facet value off a registered entity. Wave C wires the
    // COMPILE-time routing (a dotted `&e.c.f` capture lowers to this call); the
    // per-frame REACTIVE read is Wave D. Returns null when the entity/component/
    // facet is not (yet) registered, so a resolve is always safe.
    if (typeof window !== 'undefined' && window.ST && !window.ST.worldFacet) {
      window.ST.worldFacet = function(entity, component, facet) {
        var w = window.__stWorld;
        if (!w || !w.byName) return null;
        var e = w.byName[entity];
        if (!e) return null;
        // Bare `&entity` (no component): the entity's default facet, filled in a
        // later wave; for now expose the registered entity object itself.
        if (!component) return e;
        var comp = e.components && e.components[component];
        if (!comp) return null;
        if (!facet) return comp;
        // A component publishes its facets as functions (lazy, camera-dependent)
        // or plain values. Resolve either shape.
        var f = comp[facet];
        return (typeof f === 'function') ? f() : (f === undefined ? null : f);
      };
    }
  }
}
