/// Balanced Text Primitive (PLAN-024 W5)
///
/// Wraps the vendored pretext engine to produce BALANCED multi-line text: the
/// narrowest width that still yields the same line count as the full container
/// width. This evens out ragged lines (the classic headline-balance problem)
/// WITHOUT DOM reflow — pretext relayouts arithmetically on cached widths.
///
/// Sets the element's `max-inline-size` (px) to the balanced width. Recomputes
/// on resize via ResizeObserver. CSS `text-wrap: balance` is the cheap default;
/// this primitive is the precise, resize-stable, CJK/RTL-correct alternative.
///
/// @yields $balancedWidth - the chosen max-inline-size in px
/// @yields $lines         - resulting line count

%primitive balance-text(&el, lineHeight: number = 1.2) {
  %emit js {
    if (typeof pretext === 'undefined') return;

    const computeFont = (node) => {
      const cs = getComputedStyle(node);
      return `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize}/${cs.lineHeight} ${cs.fontFamily}`;
    };

    const balance = () => {
      // Reset any prior cap so we measure the natural container width.
      el.style.maxInlineSize = '';
      const full = el.clientWidth;
      if (full <= 0) return;

      const text = el.textContent || '';
      const font = computeFont(el);
      const prepared = pretext.prepareWithSegments(text, font);

      const targetLines = pretext.layout(prepared, full, %lineHeight).lineCount;
      // Single line — nothing to balance.
      if (targetLines <= 1) {
        %yield full -> $balancedWidth;
        %yield 1 -> $lines;
        return;
      }

      // Binary-search the narrowest width that still fits in targetLines.
      let lo = 0, hi = full, best = full;
      // ~10 iterations gives sub-pixel precision over any realistic width.
      for (let i = 0; i < 12 && hi - lo > 1; i++) {
        const mid = (lo + hi) / 2;
        const lc = pretext.layout(prepared, mid, %lineHeight).lineCount;
        if (lc <= targetLines) {
          best = mid;
          hi = mid;
        } else {
          lo = mid;
        }
      }

      el.style.maxInlineSize = Math.ceil(best) + 'px';
      %yield Math.ceil(best) -> $balancedWidth;
      %yield targetLines -> $lines;
    };

    balance();

    const resizeObs = new ResizeObserver(() => {
      // Observe the PARENT so our own max-inline-size change doesn't loop.
      balance();
    });
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
    $balancedWidth: number
    $lines: number
  }
}
