/// Intersection Observer Primitive
///
/// Wraps the IntersectionObserver API to detect when elements
/// enter or exit the viewport (or a specified root element).

%primitive intersection(
  /// The element to observe
  &el,
  /// Visibility threshold (0-1)
  threshold: number = 0.5,
  /// Disconnect after first intersection
  once: bool = false,
  /// Root element for intersection
  root: element? = none,
  /// Root margin (CSS margin syntax)
  margin: string = "0px"
) {
  %emit js {
    // A SENSOR that also feeds timelines: `@on &.visible` publishes `$ratio`
    // as its progress, so the ratio must be sampled CONTINUOUSLY. An
    // IntersectionObserver only calls back when a threshold is CROSSED, so a
    // single-threshold observer freezes progress at whatever value the one
    // crossing carried (BUG-323: fully-visible element stuck at 0.605).
    // The ladder samples every 1% so the ratio tracks the element.
    const __stThreshold = %threshold;
    if (typeof __stThreshold !== 'number' || !(__stThreshold >= 0 && __stThreshold <= 1)) {
      // Loud, never silent: an out-of-range threshold is an authoring error
      // (commonly a positional arg landing in the wrong slot — BUG-322).
      throw new Error(
        'intersection: threshold must be a number between 0 and 1, got ' +
        JSON.stringify(__stThreshold) +
        ' — did you mean a NAMED argument, e.g. `@on &.visible(duration: 300ms)`?'
      );
    }
    const __stLadder = [];
    for (let __i = 0; __i <= 100; __i++) __stLadder.push(__i / 100);
    if (__stLadder.indexOf(__stThreshold) === -1) {
      __stLadder.push(__stThreshold);
      __stLadder.sort((a, b) => a - b);
    }

    const options = {
      threshold: __stLadder,
      rootMargin: '%margin'
    };

    // Only set root if explicitly provided (null = viewport)
    if (%root !== null) {
      options.root = %root;
    }

    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        // `$visible` keeps its authored meaning — "at least `threshold` of me
        // is on screen" — rather than the ladder's raw `isIntersecting`,
        // which would flip true at the first pixel and silently change
        // `once:` and every boolean consumer.
        const __stRatio = entry.intersectionRatio;
        const __stVisible = __stRatio >= __stThreshold;
        %yield __stVisible -> $visible;
        %yield __stRatio -> $ratio;
        %yield entry.boundingClientRect -> $rect;
        %yield entry.rootBounds -> $rootBounds;
        %yield entry.time -> $time;

        // Auto-disconnect if once mode and now visible
        if (%once && __stVisible) {
          observer.disconnect();
        }
      }
    }, options);

    observer.observe(%&el);

    %cleanup {
      observer.disconnect();
    }
  }

  %exports {
    /// Whether element is currently visible
    $visible: bool        // true when element intersects root
    /// Intersection ratio (0-1)
    $ratio: number        // 0-1 intersection ratio
    /// Bounding client rect of observed element
    $rect: DOMRect        // bounding rect of observed element
    $rootBounds: DOMRect? // bounding rect of root
    $time: number         // timestamp of intersection change
  }
}
