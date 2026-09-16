/// Find the element a comment is about (PLAN-123 / FUP-171 W-e).
///
/// A comment anchored to an element is useless if you cannot see WHICH element.
/// This resolves an element anchor against the live page and outlines the match,
/// scrolling it into view.
///
/// ## Resolution is by CONTENT, never by position
///
/// It re-derives the match the same way the server does: compare the anchor's
/// stored content descriptor against what is on the page NOW. The stored
/// `data-st-node` selector is used only as a FAST PATH, and any candidate it
/// finds is CONFIRMED against content before being accepted — because a
/// positional id shifts when markup is inserted above the target, so trusting
/// it would highlight the wrong element and quietly teach the reader that the
/// comment was about text its author never saw.
///
/// Four outcomes, mirroring `comments::resolve_element` so both sides of the
/// wire describe the same reality:
///   found      — exactly one content match (highlight it)
///   gone       — nothing matches (say so; the comment is an orphan)
///   ambiguous  — several identical candidates (say so; never guess)
///   unverified — a legacy record with no content descriptor (say so)
///
/// @param &el     - the widget host (scope root)
/// @param request - name of the object signal carrying { selector, tag, class, id_attr, text }
/// @param outcome - name of the string signal the outcome is written to
%primitive comments-locate(&el, request: string = "", outcome: string = "") {
  %emit js {
    (() => {
      const host = %&el;
      const reqName = %request;
      const outName = %outcome;
      if (!reqName || !outName) { return; }
      const reqHead = reqName.split('.')[0];
      const outHead = outName.split('.')[0];
      const MARK = 'st-comments-located';

      const norm = (s) => (s || '').replace(/\s+/g, ' ').trim();
      // MUST match `comments::ELEMENT_TEXT_PREFIX`. The comparison is a prefix
      // so appending to a long paragraph does not lose its comment.
      const PREFIX = 120;

      const sameContent = (el, want) =>
        (el.tagName || '').toLowerCase() === norm(want.tag).toLowerCase() &&
        norm(el.getAttribute('class') || '') === norm(want.class || '') &&
        norm(el.getAttribute('id') || '') === norm(want.id_attr || '') &&
        norm(el.textContent).slice(0, PREFIX) === norm(want.text).slice(0, PREFIX);

      const clear = () => {
        document.querySelectorAll('.' + MARK).forEach((el) => el.classList.remove(MARK));
      };

      const locate = (want) => {
        clear();
        if (!want || !want.tag) {
          // No content descriptor: a record written before identity existed.
          ST.set(host, outHead, 'unverified');
          return;
        }
        // Fast path: the stored position, CONFIRMED by content. An unconfirmed
        // positional hit is exactly the silent mis-point this design refuses.
        if (want.selector) {
          const at = document.querySelector('[data-st-node="' + want.selector + '"]');
          if (at && sameContent(at, want)) {
            at.classList.add(MARK);
            at.scrollIntoView({ block: 'center', behavior: 'smooth' });
            ST.set(host, outHead, 'found');
            return;
          }
        }
        // Content scan. Never falls back to position: a comment that cannot be
        // confirmed is reported, not guessed at.
        const hits = [];
        const all = document.querySelectorAll(norm(want.tag) || '*');
        for (let i = 0; i < all.length; i++) {
          const el = all[i];
          if (el.closest('.st-comments-widget')) { continue; }
          if (sameContent(el, want)) { hits.push(el); }
          if (hits.length > 1) { break; }
        }
        if (hits.length === 1) {
          hits[0].classList.add(MARK);
          hits[0].scrollIntoView({ block: 'center', behavior: 'smooth' });
          ST.set(host, outHead, 'found');
        } else if (hits.length === 0) {
          ST.set(host, outHead, 'gone');
        } else {
          ST.set(host, outHead, 'ambiguous');
        }
      };

      let last = null;
      const sync = () => {
        const want = ST.resolve(host, reqHead);
        if (want === last) { return; }
        last = want;
        if (want) { locate(want); } else { clear(); }
      };

      let stopWatching = null;
      if (ST.watchScoped) { stopWatching = ST.watchScoped(host, reqHead, sync); }
      sync();

      %cleanup {
        if (stopWatching) { stopWatching(); }
        clear();
      }
    })();
  }
}

%macro comments-locate {
  %form { @comments-locate(for: $request:binding, to: $outcome:binding) }
  %binds { comments-locate(&self, request: $request, outcome: $outcome) }
}
