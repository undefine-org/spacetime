/// Inspector pick-mode bridge (PLAN-112 W4).
///
/// The document click listener exists ONLY while `$insPicking` is true. That is
/// both the safety boundary (ordinary page clicks stay untouched) and cheaper
/// than keeping an always-on listener that branches per click.
%primitive inspect-pick(&el, picking: string = "") {
  %emit js {
    (() => {
      const host = %&el;
      const sig = %picking;
      if (sig) {
        const sigHead = sig.split('.')[0];
        let listening = false;
        const pick = (event) => {
          const target = event.target;
          const inDock = target && target.closest && target.closest('.st-inspector-widget');
          const stamped = target && target.closest && target.closest('[data-st-node]');
          // A template-less page has no invocation to pick — its editable surface
          // is the `@data` record behind an `@each` row. Try that BEFORE giving
          // up, so pick works on the 12-of-23 projects that declare no template.
          // Invocation stamps win when both are present: a template invocation is
          // the more specific thing the user pointed at.
          if (!inDock && !stamped) {
            const addr = ST.insDataAddress && ST.insDataAddress(target);
            if (addr) {
              event.preventDefault();
              event.stopPropagation();
              ST.insSelectDataNode(host, addr);
              return;
            }
          }
          if (!inDock && stamped) {
            // A pick is a SELECTION gesture, not a page interaction. The listener
            // runs in the capture phase, so suppressing here stops the page's own
            // handlers AND the default action: without this, picking a link
            // navigates away (destroying the selection the user just made) and
            // picking a button fires whatever it does — including destructive
            // actions — merely because someone wanted to inspect it.
            event.preventDefault();
            event.stopPropagation();
            ST.insSelectNode(host, stamped.dataset.stNode || '', ST.resolve(host, 'insParams'));
          }
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
      }
    })();
  }
}

%macro inspector-pick {
  %form { @inspector-pick(on: $picking:binding) }
  %binds { inspect-pick(&self, picking: $picking) }
}
