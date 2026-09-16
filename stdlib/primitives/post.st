/// DOM Post-Processing Primitive — a "lens" over any element (PLAN-150 W2).
///
/// A film has a lens: a glow around bright things (bloom), film grain, an edge
/// vignette. On a `@stage` canvas these are real render passes (stdlib/3d's
/// `three-post`); on ordinary DOM they are compositor layers stacked over the
/// element. This primitive injects those layers ONCE and drives them entirely
/// from custom properties on the host (`--st-post-bloom`, `--st-post-grain`,
/// `--st-post-vignette`), so ANIMATING the look is just animating those
/// properties with the ordinary keyframe engine — no new interpolation path.
///
/// Fidelity, stated plainly: DOM bloom is a blurred, screen-blended copy of the
/// element clipped to its own box — it glows, but cannot bleed OUTSIDE the box
/// the way a shader can. It is the same INTENT as the stage pass, and under
/// `spacetime render` both are captured by the same screenshot.
///
/// @param &el       - the element to put the lens over
/// @param styles    - the `@post { … }` block: `bloom`/`grain`/`vignette` keys
///                    with their initial values (a map, e.g. `{bloom: "0.45"}`)
///
/// @yields $ready   - true once the layers are mounted

%primitive post-dom(
  &el,
  styles: any
) {
  %emit js {
    const el = %&el;
    if (!el || el._stPost) { %yield true -> $ready; return; }
    el._stPost = true;

    // The host must be a positioning context and must clip the bloom copy.
    const cs = getComputedStyle(el);
    if (cs.position === 'static' || !cs.position) el.style.position = 'relative';
    if (cs.isolation !== 'isolate') el.style.isolation = 'isolate';

    // Seed the custom properties from the `@post { … }` block so the layers
    // have a value on frame 0. A keyframe body (`@on &.clip { --st-post-bloom:
    // … }`) overrides these live — the layers only ever read the custom
    // property, never a baked value, so animating the look is animating the
    // property with the ordinary keyframe engine.
    // The `properties` capture arrives as an array of `"key: value"` strings
    // (`["bloom: 0.45", "vignette: 0.55"]`); parse it into a lookup.
    const raw = %styles || [];
    const map = {};
    (Array.isArray(raw) ? raw : []).forEach((decl) => {
      const i = String(decl).indexOf(':');
      if (i > 0) map[String(decl).slice(0, i).trim()] = String(decl).slice(i + 1).trim();
    });
    const seed = (key, name) => {
      if (map[key] !== undefined && el.style.getPropertyValue(name) === '') {
        el.style.setProperty(name, map[key]);
      }
    };
    seed('bloom', '--st-post-bloom');
    seed('grain', '--st-post-grain');
    seed('vignette', '--st-post-vignette');

    const layer = (cls, css) => {
      const d = document.createElement('div');
      d.className = cls;
      d.setAttribute('aria-hidden', 'true');
      d.style.cssText = 'position:absolute;inset:0;pointer-events:none;' + css;
      el.appendChild(d);
      return d;
    };

    // BLOOM: a blurred, screen-blended clone of the element's own render.
    // We cannot re-render the element, so we approximate its light with a
    // radial wash whose strength follows --st-post-bloom, plus a blurred
    // box-shadow bloom on the edges. `opacity` carries the animated strength.
    layer('st-post-bloom',
      'z-index:2147483643;mix-blend-mode:screen;' +
      'background:radial-gradient(ellipse 70% 60% at 50% 45%, rgba(255,248,232,0.55), rgba(255,248,232,0) 70%);' +
      'filter:blur(24px);' +
      'opacity:calc(var(--st-post-bloom, 0) * 0.6);');

    // VIGNETTE: a radial darkening toward the edges.
    layer('st-post-vignette',
      'z-index:2147483644;' +
      'background:radial-gradient(ellipse 78% 72% at 50% 50%, transparent 55%, rgba(0,0,0,0.9) 130%);' +
      'opacity:var(--st-post-vignette, 0);');

    // GRAIN: an SVG fractal-noise tile, screen-blended, its position jittered
    // by --st-post-grain-seed so it flickers per frame when driven.
    const grainSvg = "data:image/svg+xml;utf8," + encodeURIComponent(
      "<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'>" +
      "<filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/>" +
      "<feColorMatrix values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0'/></filter>" +
      "<rect width='100%' height='100%' filter=\"url(#n)\"/></svg>");
    layer('st-post-grain',
      'z-index:2147483645;mix-blend-mode:screen;' +
      "background-image:url(\"" + grainSvg + "\");" +
      'background-position:calc(var(--st-post-grain-seed, 0) * 71px) calc(var(--st-post-grain-seed, 0) * 37px);' +
      'opacity:var(--st-post-grain, 0);');

    %yield true -> $ready;
  }
}
