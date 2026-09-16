/// Dispatch Probe Primitive (FEAT-091)
///
/// Turns `&el` (an `<input>`) into a live macro-dispatch probe. On each input
/// (debounced), it GETs `/__spacetime/dispatch?call=<value>` — the dev server runs
/// the compiler's OWN scorer (build_dispatch_probe) — and renders the scored
/// candidate table into the element matched by `out`. The winning row glows; each
/// rejected row shows WHY it lost (literal-miss / missing / type-mismatch).
///
/// No bespoke scoring in JS: the playground is a thin view over the same Rust
/// resolution `inspect --layer dispatch --probe` prints.
///
/// @param &el - the <input> the user types a call into
/// @param out - selector of the element to render the resolution table into
/// @param endpoint - dispatch endpoint (default the dev route)

%primitive dispatch-probe(
  &el,
  out: string,
  endpoint: string = "/__spacetime/dispatch"
) {
  %emit js {
    (() => {
      const inputEl = %&el;
      const outSel = %out;
      const endpoint = %endpoint;
      if (!inputEl) return;

      const esc = (s) => String(s == null ? '' : s)
        .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

      const render = (data, outEl) => {
        if (!outEl) return;
        if (!data || data.error || !data.directive) {
          outEl.innerHTML = '<p class="probe__err">' +
            esc((data && data.error) ? data.error : 'type a directive call, e.g. @data fetch $api : "/u";') +
            '</p>';
          return;
        }
        const rows = (data.candidates || []).map((c) => {
          const cls = c.winner ? 'probe__row probe__row--win'
            : (c.total == null ? 'probe__row probe__row--out' : 'probe__row');
          const score = c.winner ? '\u2713 +' + c.total
            : (c.total == null ? '\u2205' : '+' + c.total);
          const terms = (c.terms || []).map((t) => {
            const tc = t.verdict === 'matched' ? 'probe__term'
              : 'probe__term probe__term--' + t.verdict;
            const pts = (t.points >= 0 ? '+' : '') + t.points;
            return '<span class="' + tc + '">' + esc(t.element_label) + ' ' + pts + '</span>';
          }).join('');
          return '<div class="' + cls + '">' +
            '<span class="probe__score">' + esc(score) + '</span>' +
            '<span class="probe__macro">' + esc(c.macro_name) + '</span>' +
            '<span class="probe__terms">' + terms + '</span>' +
            '</div>';
        }).join('');
        const banner = data.ambiguous
          ? '<p class="probe__amb">\u26a0 ambiguous \u2014 top two candidates tie</p>'
          : (data.margin != null ? '<p class="probe__margin">won by +' + esc(data.margin) + '</p>' : '');
        outEl.innerHTML = '<div class="probe__dir">' + esc(data.directive) + '</div>' + rows + banner;
      };

      const probe = () => {
        const outEl = document.querySelector(outSel);
        const call = inputEl.value || '';
        fetch(endpoint + '?call=' + encodeURIComponent(call))
          .then((r) => r.json())
          .then((data) => render(data, outEl))
          .catch(() => {
            if (outEl) outEl.innerHTML = '<p class="probe__err">live probe needs the dev server</p>';
          });
      };

      let timer = null;
      const onInput = () => {
        clearTimeout(timer);
        timer = setTimeout(probe, 140);
      };
      inputEl.addEventListener('input', onInput);
      // Initial paint (e.g. a seeded example value).
      probe();

      %cleanup {
        inputEl.removeEventListener('input', onInput);
        clearTimeout(timer);
      }
    })()
  }

  %exports {}
}
