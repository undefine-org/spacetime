/// Fit Text Primitive (PLAN-024 W5)
///
/// Shrink-to-fit: choose the largest font-size (within [min, max]) at which the
/// text fits the element's width in at most `maxLines` lines, using the vendored
/// pretext engine. Pure arithmetic relayout per candidate size — no DOM reflow,
/// no overflow flash. Recomputes on resize.
///
/// pretext widths scale linearly with font-size for a given font, so we measure
/// once at a reference size and scale, then verify the chosen size's line count.
///
/// @yields $fitSize - the chosen font-size in px
/// @yields $lines   - resulting line count at that size

%primitive fit-text(
  &el,
  min: number = 8,
  max: number = 200,
  maxLines: number = 1,
  lineHeight: number = 1.1
) {
  %emit js {
    if (typeof pretext === 'undefined') return;

    const familyOf = (node) => {
      const cs = getComputedStyle(node);
      return { style: cs.fontStyle, weight: cs.fontWeight, family: cs.fontFamily };
    };

    const fit = () => {
      const width = el.clientWidth;
      if (width <= 0) return;
      const text = el.textContent || '';
      const f = familyOf(el);

      // Binary-search the largest size in [min, max] that lays out within maxLines.
      let lo = %min, hi = %max, best = %min;
      for (let i = 0; i < 14 && hi - lo > 0.5; i++) {
        const mid = (lo + hi) / 2;
        const font = `${f.style} ${f.weight} ${mid}px/${%lineHeight} ${f.family}`;
        const prepared = pretext.prepare(text, font);
        const lc = pretext.layout(prepared, width, %lineHeight).lineCount;
        const natural = pretext.measureNaturalWidth(prepared);
        // Fits if it stays within the line budget AND no single line overflows.
        if (lc <= %maxLines && natural <= width) {
          best = mid;
          lo = mid;
        } else {
          hi = mid;
        }
      }

      const size = Math.floor(best * 100) / 100;
      el.style.fontSize = size + 'px';

      const finalFont = `${f.style} ${f.weight} ${size}px/${%lineHeight} ${f.family}`;
      const finalLines = pretext.layout(pretext.prepare(text, finalFont), width, %lineHeight).lineCount;

      %yield size -> $fitSize;
      %yield finalLines -> $lines;
    };

    fit();

    const resizeObs = new ResizeObserver(() => fit());
    if (el.parentElement) {
      resizeObs.observe(el.parentElement);
    } else {
      resizeObs.observe(el);
    }

    %cleanup {
      resizeObs.disconnect();
    }
  }

  %exports {
    $fitSize: number
    $lines: number
  }
}
