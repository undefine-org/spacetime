/// Text Measurement Primitive (PLAN-024 W3)
///
/// Wraps the vendored pretext engine to measure an element's text WITHOUT
/// triggering DOM reflow. Yields reactive line count, total height, and widest
/// line width, recomputed on resize via ResizeObserver.
///
/// The `pretext` global is provided by the vendored bundle (see
/// stdlib/text/MODULE.st `%vendor`), which is emitted into the page ONLY when a
/// primitive that references it fires — so pages that never measure text ship
/// none of pretext's bytes.
///
/// @yields $lines  - number of visual lines at the element's current width
/// @yields $height - total laid-out text height in px
/// @yields $widest - widest single-line natural width in px

%primitive measure-text(&el, lineHeight: number = 1.5) {
  %emit js {
    const computeFont = (node) => {
      const cs = getComputedStyle(node);
      // Canvas/pretext font shorthand: style weight size/line family
      return `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize}/${cs.lineHeight} ${cs.fontFamily}`;
    };

    const measure = () => {
      if (typeof pretext === 'undefined') return;
      const text = %&el.textContent || '';
      const font = computeFont(%&el);
      const width = %&el.clientWidth || 0;
      if (width <= 0) return;

      const prepared = pretext.prepareWithSegments(text, font);
      const result = pretext.layout(prepared, width, %lineHeight);

      %yield result.lineCount -> $lines;
      %yield result.height -> $height;
      %yield pretext.measureNaturalWidth(prepared) -> $widest;
    };

    measure();

    const resizeObs = new ResizeObserver(measure);
    resizeObs.observe(%&el);

    %cleanup {
      resizeObs.disconnect();
    }
  }

  %exports {
    $lines: number
    $height: number
    $widest: number
  }
}
