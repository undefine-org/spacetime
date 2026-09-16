/// Scroll Registry Module Primitive (PLAN-133 single-consumer %uses wave)
///
/// Owns the rAF-throttled scroll-handler fan-out: one document scroll
/// listener, `window.__stScrollHandlers` as the handler list, dead handlers
/// (returning false) spliced on each frame. scroll-driver declares
/// `%uses scroll-registry`; the next scroll-linked primitive does the same
/// instead of installing a second listener.
%primitive scroll-registry() {
  %emit prelude-js {
    window.__stScrollHandlers = window.__stScrollHandlers || [];
    if (!window.__stScrollListenerInstalled) {
      window.__stScrollListenerInstalled = true;
      window.addEventListener('scroll', () => {
        if (window.__stScrollRaf) cancelAnimationFrame(window.__stScrollRaf);
        window.__stScrollRaf = requestAnimationFrame(() => {
          window.__stScrollRaf = null;
          for (let i = window.__stScrollHandlers.length - 1; i >= 0; i--) {
            if (!window.__stScrollHandlers[i]()) {
              window.__stScrollHandlers.splice(i, 1);
            }
          }
        });
      }, { passive: true });
    }
  }
}
