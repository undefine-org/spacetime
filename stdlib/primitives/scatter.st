/// Scatter Sample Primitive — stamp per-child positions sampled from a shape.
/// (PLAN-150 W5, film.st.md §6.)
///
/// Samples the rendered glyph coverage of the `from` source and stamps two
/// custom properties (the point's x/y, in px relative to the scatter host) onto
/// each of the host's children. The children are the particles; `@scatter`
/// only ever writes custom properties — motion stays in `@on` and the
/// per-particle geometry stays CSS.
///
/// Sampling is a coverage pass: the source element's text is drawn to an
/// offscreen canvas, and `count` points are chosen by rejection sampling over
/// the alpha channel (denser strokes → more points), so `<script>` scatters
/// into the actual ink of its own letters, not a bounding box. Deterministic:
/// a fixed PRNG seeded from the source text, so a rebuild samples the same
/// points (a film renders bit-identically).
///
/// @param &el     - the scatter host; its children are stamped
/// @param from    - selector of the source to sample (`&tag`, `.headline`)
/// @param count   - number of points / children to stamp
/// @param shape   - the `:shape { --x; --y; }` slot: property names for the point
/// @param rest    - optional `:rest { --rx; --ry; }` slot: a start arrangement
///
/// @yields $ready - true once positions are stamped

%primitive scatter-sample(
  &el,
  from: string = "",
  count: number = 300,
  shape: any,
  rest: any = null
) {
  %emit js {
    const host = %&el;
    if (!host || host._stScattered) { %yield true -> $ready; return; }
    host._stScattered = true;

    // The slot bodies arrive as arrays of `"prop: axis"` strings from the
    // `properties` capture, e.g. `["--x: sample-x", "--y: sample-y"]`. Parse
    // each into { prop, axis } so a slot can name ITS OWN property for each
    // sampled coordinate. `sample-x` -> the point's X, `sample-y` -> its Y.
    const slotMap = (slot) => {
      const raw = Array.isArray(slot) ? slot : [];
      let xProp = null, yProp = null;
      raw.forEach((d) => {
        const s = String(d);
        const c = s.indexOf(':');
        if (c < 0) return;
        let prop = s.slice(0, c).trim();
        const axis = s.slice(c + 1).trim();
        if (prop[0] !== '-') prop = '--' + prop;
        if (/y/i.test(axis)) yProp = prop;
        else if (/x/i.test(axis)) xProp = prop;
      });
      return { xProp, yProp };
    };
    const shapeM = slotMap(%shape);
    const restM = slotMap(%rest);
    const xProp = shapeM.xProp || '--x';
    const yProp = shapeM.yProp || '--y';

    // Deterministic PRNG (mulberry32) seeded from the source's identity, so
    // the same word samples the same points every build.
    const srcSel = %from;
    const src = srcSel ? document.querySelector(srcSel.replace(/^&/, '.')) || document.querySelector(srcSel) : null;
    const seedStr = (src ? (src.textContent || '') : '') + '|' + %count;
    let h = 2166136261 >>> 0;
    for (let i = 0; i < seedStr.length; i++) { h ^= seedStr.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
    const rng = (() => { let a = h >>> 0; return () => { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; })();

    const N = Math.max(1, Math.round(%count));

    // Sample `N` points from the source's glyph coverage. If there is no source
    // (or no measurable text), fall back to a filled rectangle of the host, so
    // @scatter degrades to a generic field rather than stamping nothing.
    const samplePoints = () => {
      const pts = [];
      const hostRect = host.getBoundingClientRect();
      if (src) {
        const r = src.getBoundingClientRect();
        const cssText = getComputedStyle(src).font;
        const W = Math.max(2, Math.ceil(r.width));
        const H = Math.max(2, Math.ceil(r.height));
        // Draw the source text into an offscreen canvas and read its alpha.
        const cv = document.createElement('canvas');
        cv.width = W; cv.height = H;
        const ctx = cv.getContext('2d', { willReadFrequently: true });
        if (ctx) {
          ctx.font = cssText || '700 96px sans-serif';
          ctx.textBaseline = 'top';
          ctx.fillStyle = '#fff';
          ctx.fillText(src.textContent || '', 0, 0);
          let data = null;
          try { data = ctx.getImageData(0, 0, W, H).data; } catch (_e) { data = null; }
          if (data) {
            // Rejection-sample points where alpha is high (the ink).
            let tries = 0;
            const maxTries = N * 200;
            while (pts.length < N && tries < maxTries) {
              tries++;
              const px = Math.floor(rng() * W);
              const py = Math.floor(rng() * H);
              const a = data[(py * W + px) * 4 + 3];
              if (a > 40) {
                // Position relative to the HOST, aligning the source's box.
                const offX = (r.left - hostRect.left) + px;
                const offY = (r.top - hostRect.top) + py;
                pts.push([offX, offY]);
              }
            }
          }
        }
      }
      // Fallback / top-up: fill remaining points across the host box.
      while (pts.length < N) {
        pts.push([rng() * hostRect.width, rng() * hostRect.height]);
      }
      return pts;
    };

    const stamp = () => {
      const pts = samplePoints();
      const kids = Array.from(host.children);
      // Ensure there are `N` children to stamp: if the author under-provided,
      // stamp what exists; @scatter never invents DOM (that stays the author's).
      const n = Math.min(N, kids.length || N);
      for (let i = 0; i < kids.length; i++) {
        const p = pts[i % pts.length];
        kids[i].style.setProperty(xProp, p[0].toFixed(1) + 'px');
        kids[i].style.setProperty(yProp, p[1].toFixed(1) + 'px');
        // The :rest slot defaults to :shape — a start arrangement authors can
        // override in CSS. Stamp the same point under the rest names so a
        // `var(--rx)` reads a sensible default.
        if (restM.xProp) kids[i].style.setProperty(restM.xProp, p[0].toFixed(1) + 'px');
        if (restM.yProp) kids[i].style.setProperty(restM.yProp, p[1].toFixed(1) + 'px');
      }
    };

    // Stamp as soon as the source has a measurable box. Under `spacetime
    // render` the page's rAF is gated by the virtual clock, and a raw
    // requestAnimationFrame(stamp) callback can be dropped by that gate — so the
    // positions never get written and every particle collapses to the host
    // origin (a spiral around 0,0 instead of the word). Stamp SYNCHRONOUSLY
    // when the glyph box is already measurable (it is: opacity:0 does not zero
    // layout), and fall back to rAF only when the box is not yet laid out.
    // Re-stamp on the next frame too, so a live page that lays out slightly
    // later still lands on the glyphs (idempotent — same deterministic points).
    const stampWhenReady = () => {
      const ok = src ? (src.getBoundingClientRect().width > 0) : true;
      if (ok) { stamp(); return true; }
      return false;
    };
    if (!stampWhenReady()) {
      // Not laid out yet: retry on rAF, and once more after, then give up to the
      // host-box fallback so @scatter never leaves the particles unplaced.
      const raf = (typeof requestAnimationFrame !== 'undefined')
        ? requestAnimationFrame : (f) => setTimeout(() => f(0), 16);
      raf(() => { if (!stampWhenReady()) raf(() => { stampWhenReady() || stamp(); }); });
    } else {
      // Already stamped synchronously; stamp once more next frame in case a web
      // font swaps in and shifts the glyph metrics (deterministic, idempotent).
      const raf = (typeof requestAnimationFrame !== 'undefined')
        ? requestAnimationFrame : (f) => setTimeout(() => f(0), 16);
      raf(() => stamp());
    }

    %yield true -> $ready;
  }

  %exports {
    $ready: bool
  }
}
