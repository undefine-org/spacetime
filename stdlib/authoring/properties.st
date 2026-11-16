/// A property's provenance, independent of its value control or writer.
@template &origin-trail($p) {
  <div class="au-origin"><span class="au-origin__label">Origin</span><code class="au-origin__value">`$p.origin`</code><span class="au-origin__state">`$p.state`</span></div>
}

/// Legal destinations supplied by the host. Buttons report a scope identity;
/// the template never guesses whether an instance override is representable.
@template &scope-picker($p) {
  <div class="au-scopes" role="group" aria-label="Edit scope"></div>
  .au-scopes {
    @each($p.scopes as $s) {
      <button type="button" class="au-scope" data-scope="`$s.key`" data-selected="`$s.selected`" title="`$s.doc`">`$s.label`</button>
    }
  }
}

/// A shared property editor. `$p` contains label/doc, authored/evaluated values,
/// origin/state, a typed control descriptor, and legal scopes. Controls emit
/// ordinary input/change; a host owns transient preview and guarded commits.
@template &property-editor($p) {
  <section class="au-property" data-property="`$p.key`" data-state="`$p.state`">
    <header class="au-property__head"><span class="au-property__name">`$p.label`</span><code class="au-property__type">`$p.type`</code></header>
    <p class="au-property__doc">`$p.doc`</p>
    <div class="au-property__control"></div>
    <div class="au-property__values"><span>Authored <code class="au-property__authored">`$p.authored`</code></span><span>Now <code class="au-property__evaluated">`$p.evaluated`</code></span></div>
    <p class="au-property__diagnostic" role="status">`$p.diagnostic`</p>
  </section>
  .au-property__control { @each($p.controls as $f) { &$f.tpl($f); } }
  &origin-trail($p);
  &scope-picker($p);
}

@form style --authoring-surface { background: var(--au-panel, #fff); border: 1px solid var(--au-line, #dedfd8); border-radius: 12px; }
@form style --authoring-code { font-family: ui-monospace, "SFMono-Regular", monospace; font-size: 11px; }
.au-property { --authoring-surface; padding: 18px; color: var(--au-ink, #20251f); }
.au-property__head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
.au-property__name { font-weight: 600; font-size: 14px; }
.au-property__type { --authoring-code; color: var(--au-soft, #73796c); }
.au-property__doc { margin: 8px 0 18px; color: var(--au-soft, #73796c); font-size: 12px; line-height: 1.6; }
.au-property__values { display: flex; flex-wrap: wrap; gap: 16px; margin: 16px 0; font-size: 10px; color: var(--au-soft, #73796c); }
.au-property__values code { --authoring-code; color: var(--au-ink, #20251f); margin-left: 6px; }
.au-origin { display: grid; grid-template-columns: auto 1fr auto; gap: 10px; align-items: baseline; font-size: 10px; }
.au-origin__label { color: var(--au-soft, #73796c); }
.au-origin__value { --authoring-code; overflow-wrap: anywhere; }
.au-origin__state { color: var(--au-soft, #73796c); }
.au-scopes { display: flex; gap: 4px; flex-wrap: wrap; margin-top: 14px; }
.au-scope { border: 1px solid var(--au-line, #dedfd8); border-radius: 5px; padding: 5px 9px; background: transparent; color: inherit; font: inherit; font-size: 10px; cursor: pointer; }
.au-scope[data-selected="true"] { background: var(--au-accent, #d8ec9b); border-color: transparent; }
.au-property__diagnostic { margin: 10px 0 0; font-size: 11px; color: #a8442c; }
.au-property__diagnostic:empty { display: none; }
