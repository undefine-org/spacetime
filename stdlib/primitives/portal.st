/// Portal Mount Primitive (FEAT-074 W3)
///
/// Relocates an element to document.body when a signal is truthy, escaping the
/// layout tree + stacking context (drawers, modals, preview overlays), and ships
/// the recurring "modal tax" so every modal gets it right for free:
///   - scroll-lock on open (REFCOUNTED via ST.lockScroll — stacked modals unlock
///     only when the last closes)
///   - focus moves into the modal on open; focus restored to the previously
///     focused element on close
///   - Escape closes the TOPMOST modal (ST._portalStack), setting the bound signal
///     false so the close flows through the same reactive path
///   - aria-modal="true" + role="dialog" while open
///
/// When the signal is falsy the element is returned to its original position and
/// all of the above is undone.
///
/// @param &el    - the modal element (relocated to body while open)
/// @param signal - the controlling signal name (without `$`); truthy = open
/// @param closeSignal - optional: the body-scope signal to set false on Esc/close
///                      (defaults to `signal` so `@portal(when: $open)` closes $open)

%primitive portal-mount(
  &el,
  signal: string = "",
  closeSignal: string = ""
) {
  %emit js {
    (() => {
      const node = %&el;
      const sig = %signal;
      const closeSig = %closeSignal || sig;
      if (!node || !sig) return;

      // BUG-155-class: `sig`/`closeSig` may be DOTTED PATHS (`@portal(when:
      // $wrap.isOpen)`) -- resolve/watch the HEAD signal, walk the remaining
      // segments as property access for READS. A write-back through a dotted
      // path (setClose) needs the head object's CURRENT value mutated in place
      // (SpacetimeLocal's Proxy only traps a top-level assignment, not a nested
      // `obj.field = x`), so setClose reads the head, clones+patches the tail
      // field, then reassigns the WHOLE head object to trigger the Proxy.
      const sigHead = sig.split('.')[0];
      const sigTail = sig.split('.').slice(1);
      const closeSigHead = closeSig.split('.')[0];
      const closeSigTail = closeSig.split('.').slice(1);
      const walkTail = (v, tail) => tail.reduce((o, k) => (o == null ? o : o[k]), v);

      // Remember the home so we can restore on close (a placeholder comment node
      // preserves the exact slot even if siblings change).
      const placeholder = document.createComment('portal:' + sig);
      let isOpen = false;
      let restoreFocus = null;
      let trapHandler = null;
      let addedRole = false;
      const stackEntry = { close: () => setClose() };
      const FOCUSABLE = 'a[href],button:not([disabled]),textarea,input:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex="-1"])';

      const setClose = () => {
        if (closeSigTail.length === 0) {
          if (typeof window !== 'undefined' && window.SpacetimeLocal && closeSigHead in window.SpacetimeLocal) {
            window.SpacetimeLocal[closeSigHead] = false;
          } else {
            document.dispatchEvent(new CustomEvent('local:' + closeSigHead + ':updated', { detail: false }));
          }
          return;
        }
        // Dotted close signal: patch the tail field on a shallow clone of the
        // head object, then reassign the whole head through the Proxy so the
        // write is observed (a nested `obj.field = x` bypasses the trap).
        if (typeof window !== 'undefined' && window.SpacetimeLocal && closeSigHead in window.SpacetimeLocal) {
          const head = window.SpacetimeLocal[closeSigHead];
          if (head != null && typeof head === 'object') {
            const clone = Array.isArray(head) ? head.slice() : Object.assign({}, head);
            let cursor = clone;
            for (let i = 0; i < closeSigTail.length - 1; i++) {
              const k = closeSigTail[i];
              cursor[k] = (cursor[k] != null && typeof cursor[k] === 'object') ? Object.assign({}, cursor[k]) : {};
              cursor = cursor[k];
            }
            cursor[closeSigTail[closeSigTail.length - 1]] = false;
            window.SpacetimeLocal[closeSigHead] = clone;
          }
        }
      };

      const readValue = () => {
        if (typeof window !== 'undefined' && window.SpacetimeLocal && sigHead in window.SpacetimeLocal) {
          return walkTail(window.SpacetimeLocal[sigHead], sigTail);
        }
        if (typeof ST !== 'undefined' && ST.get) return walkTail(ST.get(node, sigHead), sigTail);
        return false;
      };

      const open = () => {
        if (isOpen) return;
        isOpen = true;
        // Leave a placeholder in the original slot, relocate to body.
        if (node.parentNode) node.parentNode.insertBefore(placeholder, node);
        document.body.appendChild(node);
        node.setAttribute('aria-modal', 'true');
        addedRole = !node.getAttribute('role');
        if (addedRole) node.setAttribute('role', 'dialog');
        if (typeof ST !== 'undefined' && ST.lockScroll) ST.lockScroll();
        // Focus management: remember the opener, move focus into the modal.
        restoreFocus = (typeof document !== 'undefined') ? document.activeElement : null;
        const first = node.querySelector(FOCUSABLE);
        (first || node).focus && (first || node).focus();
        // Focus-TRAP: Tab past the last focusable wraps to the first; Shift+Tab past the
        // first wraps to the last. Keeps keyboard focus inside the modal (the modal lives
        // at <body> so its background siblings stay tabbable without this).
        trapHandler = (e) => {
          if (e.key !== 'Tab') return;
          const items = Array.from(node.querySelectorAll(FOCUSABLE)).filter(el => el.offsetParent !== null || el === node);
          if (items.length === 0) { e.preventDefault(); node.focus && node.focus(); return; }
          const firstEl = items[0];
          const lastEl = items[items.length - 1];
          const active = document.activeElement;
          if (e.shiftKey && (active === firstEl || !node.contains(active))) {
            e.preventDefault(); lastEl.focus();
          } else if (!e.shiftKey && active === lastEl) {
            e.preventDefault(); firstEl.focus();
          }
        };
        node.addEventListener('keydown', trapHandler);
        if (typeof ST !== 'undefined' && ST.pushPortal) ST.pushPortal(stackEntry);
      };

      const close = () => {
        if (!isOpen) return;
        isOpen = false;
        node.removeAttribute('aria-modal');
        if (addedRole) { node.removeAttribute('role'); addedRole = false; }
        if (trapHandler) { node.removeEventListener('keydown', trapHandler); trapHandler = null; }
        // Return to the original slot.
        if (placeholder.parentNode) {
          placeholder.parentNode.insertBefore(node, placeholder);
          placeholder.remove();
        }
        if (typeof ST !== 'undefined' && ST.unlockScroll) ST.unlockScroll();
        if (typeof ST !== 'undefined' && ST.removePortal) ST.removePortal(stackEntry);
        if (restoreFocus && restoreFocus.focus) restoreFocus.focus();
        restoreFocus = null;
      };

      const apply = () => { readValue() ? open() : close(); };

      const onUpdate = () => apply();
      document.addEventListener('local:' + sigHead + ':updated', onUpdate);
      let unwatch = null;
      if (typeof ST !== 'undefined' && ST.watch) {
        unwatch = ST.watch(node, sigHead, () => apply());
      }
      apply(); // initial state

      if (typeof ST !== 'undefined' && ST.onCleanup) {
        ST.onCleanup(node, () => {
          document.removeEventListener('local:' + sigHead + ':updated', onUpdate);
          if (unwatch) unwatch();
          if (isOpen) close();
        });
      }
    })();
  }
}
