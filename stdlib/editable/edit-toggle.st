/// Edit-mode toggle (editable-everything, local-dev).
///
/// A floating "Edit / Done" pill that flips `html[data-st-edit-mode]` via the dev
/// edit-mode controller (`window.__stDevEditable.toggle()`, stdlib/__dev__/
/// primitives/dev-editable.st, loaded on every dev page). While edit-mode is on:
///   - every `@richtext` surface on the page becomes editable in place (its gate
///     mirrors the attribute), and
///   - every provenanced text node hover-affords click-to-edit (dev-editable).
/// Persistence is the existing dev-save → EditAst/EditJson path; no new channel.
///
/// The page author writes ZERO JS: `@import "stdlib/editable"` then `@edit-toggle`
/// at file scope. The pill renders only when the dev controller is present (dev
/// mode), so a production build is inert. Local-dev only by design (no auth: the
/// dev server is single-user localhost).
///
/// Self-contained: it owns its `editing` state and mirrors `data-st-edit-mode`;
/// it yields no signal (file-scope, no host element to bind state onto).

%primitive edit-toggle() {
  %emit js {
    (function () {
      if (typeof document === 'undefined') return;
      var editing = false;

      // Pill styles — a single scarlet accent, editorial identity (matches admin).
      var style = document.createElement('style');
      style.setAttribute('data-st-edit-toggle', '');
      style.textContent = [
        '#st-edit-toggle{position:fixed;right:1.25rem;bottom:1.25rem;z-index:2147483000;',
        'font:600 13px/1 system-ui,-apple-system,Segoe UI,sans-serif;letter-spacing:.01em;',
        'display:inline-flex;align-items:center;gap:.5rem;padding:.6rem .95rem;border-radius:999px;',
        'border:1px solid rgba(0,0,0,.08);background:#fff;color:#111;cursor:pointer;',
        'box-shadow:0 2px 6px rgba(0,0,0,.10),0 8px 28px rgba(0,0,0,.10);',
        'transition:transform .12s ease,box-shadow .12s ease,background .12s ease,color .12s ease;}',
        '#st-edit-toggle:hover{transform:translateY(-1px);box-shadow:0 4px 10px rgba(0,0,0,.12),0 12px 36px rgba(0,0,0,.14);}',
        '#st-edit-toggle:active{transform:translateY(0);}',
        '#st-edit-toggle .st-et-dot{width:8px;height:8px;border-radius:999px;background:#d62828;',
        'box-shadow:0 0 0 0 rgba(214,40,40,.45);transition:box-shadow .2s ease;}',
        '#st-edit-toggle[data-on="true"]{background:#d62828;color:#fff;border-color:#d62828;}',
        '#st-edit-toggle[data-on="true"] .st-et-dot{background:#fff;box-shadow:0 0 0 4px rgba(255,255,255,.25);}',
        '@media print{#st-edit-toggle{display:none!important;}}'
      ].join('');
      (document.head || document.documentElement).appendChild(style);

      var btn = document.createElement('button');
      btn.id = 'st-edit-toggle';
      btn.type = 'button';
      btn.setAttribute('aria-pressed', 'false');
      var dot = document.createElement('span');
      dot.className = 'st-et-dot';
      var label = document.createElement('span');
      label.className = 'st-et-label';
      label.textContent = 'Edit';
      btn.appendChild(dot);
      btn.appendChild(label);

      function render() {
        btn.setAttribute('data-on', editing ? 'true' : 'false');
        btn.setAttribute('aria-pressed', editing ? 'true' : 'false');
        label.textContent = editing ? 'Done' : 'Edit';
      }

      btn.addEventListener('click', function () {
        // Prefer the dev controller (owns data-st-edit-mode + provenance editing);
        // fall back to flipping the attribute directly so @richtext still gates.
        if (window.__stDevEditable && window.__stDevEditable.toggle) {
          window.__stDevEditable.toggle();
        } else if (document.documentElement.hasAttribute('data-st-edit-mode')) {
          document.documentElement.removeAttribute('data-st-edit-mode');
        } else {
          document.documentElement.setAttribute('data-st-edit-mode', 'true');
        }
        editing = document.documentElement.hasAttribute('data-st-edit-mode');
        render();
      });

      // Keep the pill in sync if edit-mode is toggled elsewhere.
      if (typeof MutationObserver === 'function') {
        var obs = new MutationObserver(function () {
          var on = document.documentElement.hasAttribute('data-st-edit-mode');
          if (on !== editing) { editing = on; render(); }
        });
        obs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-st-edit-mode'] });
      }

      function mount() { (document.body || document.documentElement).appendChild(btn); render(); }
      if (document.body) { mount(); } else { document.addEventListener('DOMContentLoaded', mount); }
    })();
  }
}

/// File-scope entry: drop `@edit-toggle` on a page to mount the pill.
%macro edit-toggle {
  %scope file
  %order 60

  %form {
    @edit-toggle
  }

  %binds {
    edit-toggle()
  }
}
