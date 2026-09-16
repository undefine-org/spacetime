/// Easing Module Primitive (PLAN-133 single-consumer %uses wave)
///
/// Owns the easing-function table behind `window.__stGetEasing(name)` —
/// extracted from animate-property's prelude under the "every shared prelude
/// is a module" rule: the next animation primitive needing easings declares
/// `%uses easing` instead of copying the table.
%primitive easing() {
  %emit prelude-js {
    window.__stGetEasing = window.__stGetEasing || ((name) => {
      const easings = {
        linear: t => t,
        easeOutQuad: t => t * (2 - t),
        easeInOutQuad: t => t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t,
        easeOutCubic: t => (--t) * t * t + 1,
      };
      return easings[name] || easings.linear;
    });
  }
}
