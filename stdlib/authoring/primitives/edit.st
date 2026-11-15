/// A source-edit host for property-editor. Fields carry current parser spans;
/// the shared server writer checks expected bytes, parses and journals edits.
/// Controls remain usable without this behavior for transient previews.
%primitive authoring-edit(&host, entry: string = "index.st") {
  %emit js {
    const host = %&host;
    const report = (message, error) => {
      host.dataset.authoringState = error ? 'error' : 'saved';
      const output = host.querySelector('[data-authoring-status]');
      if (output) output.textContent = message;
      host.dispatchEvent(new CustomEvent('authoring:status', { bubbles: true, detail: { message, error: !!error } }));
    };
    const commit = async (event) => {
      const control = event.target;
      const field = control.closest && control.closest('.w-field');
      if (!field || !host.contains(field)) return;
      if (event.type === 'click' && !control.classList.contains('w-choice')) return;
      if (event.type === 'change' && control.classList.contains('w-choice')) return;
      const d = field.dataset;
      if (!d.file || !(Number(d.end) > Number(d.start))) { report('This parameter has no default yet. Open its definition in source.', true); return; }
      let value = control.classList.contains('w-choice') ? control.dataset.word : control.type === 'checkbox' ? String(control.checked) : control.value;
      if (d.quote !== undefined && d.quote !== 'false') value = JSON.stringify(value);
      if (value === d.expect) return;
      report('Saving…');
      try {
        const entry = decodeURIComponent(%entry);
        const transaction = { entry, edits: [{ file: d.file, start: Number(d.start), end: Number(d.end), expect: d.expect, text: value }] };
        // Studio supplies its existing live/reload/transport handshake. The
        // catalog uses the same server writer without importing Studio chrome.
        if (ST.stu && ST.stu.on && ST.stu.send) { await ST.stu.send('/__spacetime/rewrite', transaction, 'Saved to '); return; }
        const response = await fetch('/__spacetime/rewrite', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(transaction)
        });
        const result = await response.json();
        if (!result.ok) throw new Error(result.error || 'The source changed; refresh before editing again.');
        d.expect = value;
        report('Saved shared definition');
      } catch (error) { report(String(error.message || error), true); }
    };
    host.addEventListener('change', commit);
    host.addEventListener('click', commit);
    %cleanup { host.removeEventListener('change', commit); host.removeEventListener('click', commit); }
  }
}
%macro authoring-edit {
  %scope selector
  %form { @authoring-edit(entry: $entry:string = "index.st") }
  %binds { authoring-edit(&self, entry: $entry) }
}
