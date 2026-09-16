/// Comments element picker (PLAN-123 / FUP-171 W-d).
///
/// Click an element on the page to anchor a comment to it. The listener EXISTS
/// ONLY while picking — that is both the safety boundary (ordinary page clicks
/// are untouched when the composer is closed) and cheaper than an always-on
/// listener that branches per click. Same shape as the inspector's
/// `inspect-pick`, deliberately: two pickers with two behaviours would be two
/// answers to one need.
///
/// ## What it reports, and what it does NOT
///
/// It reports what it SEES — tag, class, id, text — and lets the SERVER mint
/// the content fingerprint that becomes the anchor's identity. It does not hash
/// anything here. A second BLAKE3 implementation in JavaScript would mean two
/// definitions of one rule, and the moment they disagreed by a byte, every
/// anchor written by one surface would be invisible to the other: a silent,
/// project-wide orphaning with no error anywhere.
///
/// The positional `data-st-node` path rides along as a HINT only. Spacetime's
/// structural ids shift when markup is inserted above the target, so an anchor
/// keyed on position resolves cleanly to the WRONG element after an edit —
/// which is exactly why identity is content, not place.
///
/// @param &el     - the widget host (scope root for signal resolution)
/// @param picking - name of the boolean signal that gates pick mode
/// @param onpick  - name of the object signal the picked content is written to
%primitive comments-pick(&el, picking: string = "", onpick: string = "") {
  %emit js {
    (() => {
      const host = %&el;
      const sig = %picking;
      const out = %onpick;
      if (!sig || !out) { return; }
      const sigHead = sig.split('.')[0];
      let listening = false;

      // The element's own text, with nested markup flattened. `innerText` would
      // reflect CSS visibility and force layout on every pick; `textContent` is
      // what the source says, which is what the fingerprint hashes.
      const textOf = (el) => (el.textContent || '').replace(/\s+/g, ' ').trim();

      const pick = (event) => {
        const target = event.target;
        if (!target || !target.closest) { return; }
        // Never pick the comments widget itself — clicking "pick" would
        // otherwise anchor a comment to the pick button.
        if (target.closest('.st-comments-widget')) { return; }

        // A pick is a SELECTION gesture, not a page interaction. The listener
        // runs in the capture phase, so suppressing here stops the page's own
        // handlers AND the default action: without it, picking a link navigates
        // away and destroys the selection just made, and picking a button fires
        // whatever it does — including destructive actions — merely because
        // someone wanted to comment on it.
        event.preventDefault();
        event.stopPropagation();

        const stamped = target.closest('[data-st-node]');
        // `ST.set` on the widget host, the same transition `insSelectNode` uses
        // for the inspector: the picked value belongs to the widget's own scope.
        ST.set(host, out.split('.')[0], {
          selector: (stamped && stamped.dataset.stNode) || '',
          tag: (target.tagName || '').toLowerCase(),
          class: target.getAttribute('class') || '',
          id_attr: target.getAttribute('id') || '',
          text: textOf(target),
          route: window.location.pathname
        });
        // One pick per arming: leaving pick mode on would swallow the user's
        // next ordinary click on the page.
        ST.set(host, sigHead, false);
      };

      const sync = () => {
        const enabled = !!ST.resolve(host, sigHead);
        if (enabled && !listening) {
          document.addEventListener('click', pick, true);
          listening = true;
        }
        if (!enabled && listening) {
          document.removeEventListener('click', pick, true);
          listening = false;
        }
      };

      let stopWatching = null;
      if (ST.watchScoped) { stopWatching = ST.watchScoped(host, sigHead, sync); }
      sync();

      %cleanup {
        if (stopWatching) { stopWatching(); }
        if (listening) { document.removeEventListener('click', pick, true); }
      }
    })();
  }
}

%macro comments-pick {
  %form { @comments-pick(on: $picking:binding, to: $picked:binding) }
  %binds { comments-pick(&self, picking: $picking, onpick: $picked) }
}
