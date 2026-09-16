/* ============================================================================
   field.st — FORMS `@field` sugar, SPIKE 4 (FUP-142, Tier A).

   Local, page-scoped prototype (lives in the spike fixture, NOT stdlib yet —
   promote after the design settles). Encodes the SPIKE-3 stale-read fix as a
   single call so no page can hand-write the cross-field bug:

     .signup .field-email input { @field($values, email, on: $validate) }

   expands (via the field-input primitive) to exactly the proven pattern:
     - reactive `input.value` <- $values.email
     - on input: write the LOCAL client-authoritative values signal
                 ($values.email <- el.value), then fire $validate($values)

   The inputs write $values (client-authoritative); the server form is consulted
   ONLY for errors via a sibling `@field-error`. That's why the concurrent
   fast-typing bug is impossible BY CONSTRUCTION.
   ============================================================================ */

%primitive field-input(
  &el,
  values: string = "",
  field: string = "",
  fire: string = ""
) {
  %emit js {
    (() => {
      const node = %&el;
      const valuesSig = %values;   // e.g. "values"  (local values signal name)
      const key = %field;          // e.g. "email"
      const fireName = %fire;      // e.g. "validate" (signal-call to invoke)
      if (!node || !valuesSig || !key) return;

      // The values signal is FILE-scoped (a `@data inline`), so it lives in the
      // shared `window.SpacetimeLocal` Proxy — NOT element-scoped `ST.set`. All
      // fields read/write the SAME object here; that shared identity is what
      // makes cross-field merges consistent (element-scoped writes would give
      // each input its own private copy → the sibling key is lost).
      const readValues = () => {
        const v = window.SpacetimeLocal ? window.SpacetimeLocal[valuesSig] : null;
        return v && typeof v === 'object' ? v : {};
      };

      // Reactive value: reflect $values[key] into the input, but never fight the
      // user mid-keystroke (only write when the value actually diverges).
      const sync = () => {
        const next = readValues()[key] ?? '';
        if (node.value !== next && document.activeElement !== node) node.value = next;
      };
      sync();
      document.addEventListener('local:' + valuesSig + ':updated', sync);
      document.addEventListener('local:updated', sync);

      // Input: merge THIS field's key into the shared values object, then fire
      // the validate signal-call by name. Reading the shared CURRENT object (not
      // a per-element copy, not a stale server echo) keeps sibling keys intact
      // even when two fields update in the same tick.
      node.addEventListener('input', () => {
        const next = Object.assign({}, readValues(), { [key]: node.value });
        if (window.SpacetimeLocal) window.SpacetimeLocal[valuesSig] = next;
        if (fireName) {
          const fire = (ST._signalCalls || {})[fireName];
          if (typeof fire === 'function') fire(next);
        }
      });
    })();
  }

  %exports {}
}

%macro field {
  %scope selector
  %order 700

  %form {
    @field($values:binding, $field:ident, on: $fire:binding)
  }

  %binds {
    field-input(&self, values: $values, field: $field, fire: $fire)
  }
}


%primitive field-error-text(
  &el,
  form: string = "",
  field: string = ""
) {
  %emit js {
    (() => {
      const node = %&el;
      const formSig = %form;   // the SERVER-authoritative form signal name
      const key = %field;
      if (!node || !formSig || !key) return;
      const render = () => {
        const f = window.SpacetimeLocal ? window.SpacetimeLocal[formSig] : null;
        const errs = f && typeof f === 'object' ? f.errors : null;
        const msg = errs && typeof errs === 'object' ? (errs[key] ?? '') : '';
        node.textContent = msg == null ? '' : String(msg);
      };
      render();
      document.addEventListener('local:' + formSig + ':updated', render);
      document.addEventListener('local:updated', render);
    })();
  }

  %exports {}
}

%macro field-error {
  %scope selector
  %order 700

  %form {
    @field-error($form:binding, $field:ident)
  }

  %binds {
    field-error-text(&self, form: $form, field: $field)
  }
}
