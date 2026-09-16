/// Transition Edge Primitive — apply a `@form transition` across a score `->`.
/// (PLAN-150 W8, film.st.md §9.)
///
/// A transition placed on a score edge (`&open -> --crossfade -> &grid`) joins
/// two clips: the OUTGOING clip runs the transition's `&from` keyframes as it
/// ends, the INCOMING clip runs `&to` as it begins. Both are driven by the
/// clips' own `@score` windows (the `__clip_<name>` progress signals the score
/// already publishes), so a transition rides the same clock as everything else
/// — it is choreography, not a scheduler.
///
/// The `&from` block plays over the LAST `overlap` fraction of the outgoing
/// clip's window; `&to` over the FIRST `overlap` fraction of the incoming
/// clip's window. `overlap` derives from the transition's duration relative to
/// the joined windows (passed in as a 0..1 fraction), so a longer transition
/// eats more of each clip's edge.
///
/// Keyframes arrive as the same `{properties:[…]}` shape apply-animations
/// consumes; this primitive reuses ST's interpolation so a transition is
/// exactly a pair of clip animations, no new interpolation path.
///
/// @param &el       - the score's scope owner (used only to resolve signals)
/// @param fromSel   - selector of the outgoing clip element
/// @param toSel     - selector of the incoming clip element
/// @param fromSig   - the outgoing clip's window signal name (`__clip_open`)
/// @param toSig     - the incoming clip's window signal name (`__clip_grid`)
/// @param fromKf    - the `&from` keyframes (apply-animations `properties` shape)
/// @param toKf      - the `&to` keyframes
/// @param overlap   - 0..1 fraction of each clip's window the transition covers
///
/// @yields $ready   - true once wired

%primitive transition-edge(
  &el,
  fromSel: string = "",
  toSel: string = "",
  fromSig: string = "",
  toSig: string = "",
  fromKf: any = null,
  toKf: any = null,
  overlap: number = 0.2
) {
  %emit js {
    const root = %&el;
    const fromSel = %fromSel, toSel = %toSel;
    const fromSig = %fromSig, toSig = %toSig;
    const fromKf = %fromKf, toKf = %toKf;
    const overlap = Math.max(0.01, Math.min(1, %overlap));

    if (typeof ST === 'undefined' || !ST.watchScoped) { %yield true -> $ready; return; }

    // Interpolate one keyframe property at progress p (0..1). Mirrors
    // apply-animations' positioned-stop reading: each keyframe has {at, value}.
    const interp = (kfs, p) => {
      if (!kfs || !kfs.length) return null;
      if (p <= kfs[0].at) return kfs[0].value;
      if (p >= kfs[kfs.length - 1].at) return kfs[kfs.length - 1].value;
      for (let i = 0; i < kfs.length - 1; i++) {
        const a = kfs[i], b = kfs[i + 1];
        if (p >= a.at && p <= b.at) {
          const t = (b.at - a.at) > 0 ? (p - a.at) / (b.at - a.at) : 0;
          const av = parseFloat(a.value), bv = parseFloat(b.value);
          if (!isNaN(av) && !isNaN(bv)) {
            const unitM = String(b.value).match(/[a-z%]+$/i);
            return (av + (bv - av) * t).toFixed(3) + (unitM ? unitM[0] : '');
          }
          return a.value;
        }
      }
      return kfs[kfs.length - 1].value;
    };

    const applyKf = (els, props, p) => {
      els.forEach(el => {
        props.forEach(pr => {
          const v = interp(pr.keyframes, p);
          if (v == null) return;
          const norm = pr.property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
          const transformProps = ['translateX','translateY','translateZ','translate','rotate','rotateX','rotateY','rotateZ','scale','scaleX','scaleY','scaleZ','skew','skewX','skewY'];
          if (pr.property.indexOf('--') === 0) { el.style.setProperty(pr.property, v); }
          else if (transformProps.indexOf(norm) !== -1) {
            const cur = el._stTfxT || {}; cur[norm] = v; el._stTfxT = cur;
            el.style.transform = Object.keys(cur).map(k => {
              if (k.indexOf('translate') === 0) return k + '(' + cur[k] + ')';
              if (k.indexOf('rotate') === 0 || k.indexOf('skew') === 0) return k + '(' + cur[k] + ')';
              return k + '(' + cur[k] + ')';
            }).join(' ');
          } else { el.style[norm] = v; }
        });
      });
    };

    // The from/to keyframes arrive as the same `{properties:[{property,
    // keyframes:[{at,value}]}]}` shape apply-animations consumes (the compiler
    // reifies the region's `prop: a -> b;` text into it). Accept that object, or
    // a raw array, or — as a fallback — parse the body text directly.
    const props = (kf) => {
      if (kf && kf.properties) return kf.properties;
      if (Array.isArray(kf)) return kf;
      if (!kf || typeof kf !== 'string') return [];
      const out = [];
      kf.split(';').forEach(line => {
        const c = line.indexOf(':');
        if (c < 0) return;
        const prop = line.slice(0, c).trim();
        if (!prop || prop === 'easing') return;
        const stops = line.slice(c + 1).split('->').map(s => s.trim()).filter(Boolean);
        if (stops.length < 1) return;
        const kfs = stops.map((v, i) => ({ at: stops.length > 1 ? i / (stops.length - 1) : 1, value: v }));
        out.push({ property: prop, keyframes: kfs });
      });
      return out;
    };
    const fromEls = fromSel ? Array.from(document.querySelectorAll(fromSel)) : [];
    const toEls = toSel ? Array.from(document.querySelectorAll(toSel)) : [];
    const fromProps = props(fromKf), toProps = props(toKf);

    if (fromEls.length && fromProps.length) {
      ST.watchScoped(root, fromSig, (wp) => {
        const w = typeof wp === 'number' ? wp : 0;
        if (w < 1 - overlap) return;                       // only the tail
        const p = (w - (1 - overlap)) / overlap;           // 0..1 across the edge
        applyKf(fromEls, fromProps, Math.max(0, Math.min(1, p)));
      });
    }
    if (toEls.length && toProps.length) {
      ST.watchScoped(root, toSig, (wp) => {
        const w = typeof wp === 'number' ? wp : 0;
        if (w > overlap) return;                           // only the head
        const p = w / overlap;                             // 0..1 across the edge
        applyKf(toEls, toProps, Math.max(0, Math.min(1, p)));
      });
    }

    %yield true -> $ready;
  }

  %exports {
    $ready: bool
  }
}
