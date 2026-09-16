// =============================================================================
// Entry editor drawer — the schema-driven form (FEAT-076 W2).
//
// Opens as a @portal when a row is clicked ($drawerOpen). Renders the active
// entry's fields from its type schema: the server (types.json) assigns each
// field a WIDGET; the admin dispatches per-widget via @match. Nested objects
// (fieldset) + object arrays (list) are flattened into the form list by
// ST.adFormFields (depth-marked group headers), so a flat @each renders an
// arbitrarily-deep form. The admin re-derives nothing client-side.
//
// W2 scope: render every widget kind read-only-ish (inputs reflect values).
// Editing/persistence is wired in W3. The recursive schema IS the stress test.
// =============================================================================

// ── Per-widget field templates (dispatched by @match on $f.widget) ──────────
// Each takes one descriptor $f = {name,label,widget,value,path,depth,enumvals,ref}.

@template &fld-textarea($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <textarea class="ad-input ad-textarea" data-path="`$f.path`" data-widget="textarea">`$f.value`</textarea>
  </div>
}
// Structured rich-text field (PLAN-031 / FEAT-100): mounts an @editable surface
// over the field's document AST ($f.value is the AST OBJECT, not a string — see
// ST.adFormFields richtext keepObject). The surface stores its doc on the host
// and fires a bubbling `input`, so the .ad-form `@on input [data-path]` persist
// wiring carries the AST through ST.adFieldInput → EditJson → server schema
// validation → disk. No admin-specific JS: one template, the generic rail.
// The editable surface attaches as a SCOPE on `.ad-richtext` (NOT inline element
// content). In a `@template` body, behavioral constructs are scope children that the
// factory mounts per-instance (PLAN-039) — `$f.value` resolves against the instance
// param via the surface's instance-scope resolveBoundDoc. Inline-in-element
// `@directive` content does not parse (it is HTML text); the selector-scope form is the
// canonical attach point. (Comments live OUTSIDE the body: a `@template` body is scanned
// as HTML, so a `//` inside it would render as literal text.)
@template &fld-richtext($f) {
  <div class="ad-field ad-field--richtext" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <div class="ad-input ad-richtext" data-path="`$f.path`" data-widget="richtext"></div>
  </div>
  .ad-richtext { @editable(bind: `$f.value`, marks: ["bold","italic","link"], blocks: ["p","h2","blockquote"]); }
}
// Color token: a native swatch + a text input for the hex/value, both bound to
// the same data-path. Used by the Brand pane (PLAN-034 Wave B) for `color` fields.
// Color token (FEAT-107): a unified pill (native swatch + hex text) PLUS an
// expandable perceptual HSV panel — Hue/Saturation/Value/Alpha native range
// sliders. Fully declarative (no pointer-drag). The sliders recompose the hex
// via ST.adColorSlider and write it through the hex text input, so the existing
// brand persist + live-preview path fires unchanged. The disclosure checkbox
// (CSS-only open state) reveals the panel; opening seeds the sliders from the
// current hex (adColorPickerSync, wired in tools.st on `change` of the toggle).
// Dimensioned token slider (FEAT-107): a native range input tunes the NUMBER;
// the unit is preserved + reattached on write-back. A hidden text twin carries
// the composed value (`8px`) + the real data-path so the existing brand/motion
// persist path (which keys on data-path) is unchanged — the slider is a control
// surface over it, not a parallel channel. `&fld-range-length` / `-duration`
// differ only in default bounds (baked into the descriptor by adFormFields).
// Derived token (FEAT-108): READ-ONLY. Shows the resolved color swatch + the
// source formula chip (`scarlet | darken 12%`). Not editable — a derived token
// is computed from literals; you edit the literals, this re-derives. The swatch
// background is the resolved value (a hex/rgba string from the live signal).
@template &fld-derived($f) {
  <div class="ad-field ad-field--derived" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <div class="ad-derived">
      <span class="ad-derived-swatch" style="background: `$f.value`"></span>
      <code class="ad-derived-formula">`$f.formula`</code>
      <span class="ad-derived-val">`$f.value`</span>
    </div>
  </div>
}
@template &fld-readonly($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <input class="ad-input ad-input--readonly" value="`$f.value`" data-path="`$f.path`" data-widget="readonly" readonly>
  </div>
}
@template &fld-media($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <div class="ad-media">
      <span class="ad-media-prev" data-media-prev="`$f.path`"></span>
      <input class="ad-input ad-media-url" value="`$f.value`" data-path="`$f.path`" data-widget="media" placeholder="/assets/&hellip;">
      <button class="ad-btn ad-media-lib" data-media-path="`$f.path`" type="button">Library</button>
    </div>
  </div>
}
@template &fld-select($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <select class="ad-input ad-select" data-path="`$f.path`" data-widget="select"></select>
  </div>
}
@template &fld-chips($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <div class="ad-chips" data-path="`$f.path`" data-widget="chips"></div>
  </div>
}
@template &fld-relation($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <select class="ad-input ad-relation" data-path="`$f.path`" data-widget="relation" data-ref="`$f.ref`"></select>
  </div>
}
@template &fld-relation-multi($f) {
  <div class="ad-field" data-path="`$f.path`">
    <label class="ad-flabel">`$f.label`</label>
    <div class="ad-rel-multi" data-path="`$f.path`" data-widget="relation-multi" data-ref="`$f.ref`"></div>
  </div>
}
// A group header (nested object / list item boundary).
@template &fld-group($f) {
  <div class="ad-group" data-depth="`$f.depth`">
    <span class="ad-group-label">`$f.label`</span>
  </div>
}

// Dispatch: each descriptor carries `tpl` = its widget template name (fld-text,
// fld-readonly, …), assigned by ST.adFormFields from the server widget. The form
// @each invokes it via DYNAMIC DISPATCH &$f.tpl($f) — one level, no intermediate
// @match (FEAT-073 dynamic template-name dispatch resolves &$expr at runtime).

// ── Compute the form when the selection changes ──────────────────────────────
// $activeIndex (row clicked) → resolve the entry + schema → flatten to fields.
.ad-form {
  @on $activeIndex.change {
    $activeEntry <- ST.adActiveEntry(window.SpacetimeLocal.entries, $activeIndex);
    $drawerTitle <- ST.adEntryTitle(window.SpacetimeLocal.entries, $activeIndex, window.SpacetimeLocal.types, window.SpacetimeLocal.activeType);
    $formFields <- ST.adFormFields(ST.adActiveSchema(window.SpacetimeLocal.types, window.SpacetimeLocal.activeType), ST.adActiveEntry(window.SpacetimeLocal.entries, $activeIndex), '');
  }
}

// Render the flattened fields.
.ad-form {
  @each($formFields as $f) {
    &$f.tpl($f);
  }
}

// After the controls are built, hydrate them from their descriptors: fill
// <select> options from the field enum, reflect toggle/chip values. The form
// @each builds control SHELLS; this fills the per-field option lists the
// meta-renderer can't know at template-author time.
.ad-form {
  @on $formFields.change {
    $noop <- ST.adHydrateControls($formFields);
  }
}

// Preload every collection's data (keyed by type) so relation pickers can list
// the target type's entries. Runs once the collection list arrives.
.ad-form {
  @on $collections.change(immediate: true) {
    $noop <- ST.adLoadRelationData($collections, window.SpacetimeLocal.types);
  }
}

// Drawer head reflectors.
.ad-drawer-title { text <- $drawerTitle; }
.ad-drawer-kind  { text <- $activeType; }

// ── The drawer escapes the layout as a @portal modal ─────────────────────────
.ad-drawer { @portal(when: $drawerOpen); }

// Close affordances.
.ad-drawer-x { @on &.click { $drawerOpen <- false; } }
.ad-done     { @on &.click { $drawerOpen <- false; } }

// ── Field editing: persist a change to the active entry by its data-path ──────
// Every widget input carries data-path (relative to the entry) + data-widget.
// On input/change, write the value back to $entries[$activeIndex]<path> + persist
// via dev-ws (optimistic; reverts on reject). Text-ish inputs persist on `input`;
// toggles, selects + relations on `change`. Values are coerced by widget so the
// JSON keeps its declared type (number → Number, toggle → bool) — not a string.
.ad-form {
  @on &.input [data-path] {
    $noop <- ST.adFieldInput($, $activeIndex);
  }
  @on &.change [data-path] {
    $noop <- ST.adFieldInput($, $activeIndex);
  }
}

// ── Media library: a media field's Library button opens the modal FOR that path ─
// $mediaPath records which field the picker is editing; picking persists through
// adEditField and the drawer's value re-renders from the $formFields recompute.
.ad-form {
  @on &.click [data-media-path] {
    $mediaPath <- $.closest('[data-media-path]').dataset.mediaPath;
    $mediaQuery <- "";
    $mediaOpen <- true;
  }
}

// ── Delete the active entry (two-click arm to avoid destructive single click) ─
// Reset the arm whenever the drawer switches entries, so a stale arm can never
// turn a single click on a DIFFERENT entry into an unconfirmed delete (data-loss
// footgun). The button label reflects the armed state so it's visible.
.ad-form {
  @on $activeIndex.change {
    $delArmed <- false;
  }
}
.ad-del {
  @on &.click {
    $delArmed <- ST.adArmDelete($delArmed, $activeIndex);
  }
  .armed: $delArmed;
  text <- ST.adDeleteLabel($delArmed);
}
