/// Spacetime `widgets` — one field per param type (PLAN-156).
///
/// A widget is a `@template` that renders one FIELD and reports one `change`.
/// It knows nothing about who uses it: the studio's inspector and the admin's
/// drawer render the same `&w-*`, picked by the field's type. A scalar row's
/// `%widget` names it (`stdlib/scalars/types.st`), and a param with choices takes
/// `&w-choice`.
///
/// A field `$f` is a record:
///
///   label   the param's name          doc     what it is for (its `///`)
///   value   the words in the source   unit    what a number counts (`m`, `°`)
///   min max step                      choices [{ word, doc, on }]
///   written whether the source has it (else the value is the default)
///   file start end expect at before   where its words are (the address a
///                                     write is made against; `at`/`before`
///                                     for a param still to be written)
///
/// The address rides on the field (`.w-field`), so whoever listens for the
/// widget's `change` (or a choice's click) can turn it into a guarded edit.

@import "./primitives/scrub.st"

@template &w-number($f) {
  <label class="w-field" data-kind="number" data-written="`$f.written`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" data-quote="`$f.quote`" data-drive="`$f.drive`"><span class="w-field__label w-field__label--scrub" title="`$f.doc`">`$f.label`</span><input class="w-input w-number" type="number" value="`$f.value`" min="`$f.min`" max="`$f.max`" step="`$f.step`" /><span class="w-field__unit">`$f.unit`</span></label>
}

@template &w-range($f) {
  <label class="w-field" data-kind="range" data-written="`$f.written`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" data-quote="`$f.quote`" data-drive="`$f.drive`"><span class="w-field__label w-field__label--scrub" title="`$f.doc`">`$f.label`</span><input class="w-input w-slider" type="range" value="`$f.value`" min="`$f.min`" max="`$f.max`" step="`$f.step`" /><input class="w-input w-number" type="number" value="`$f.value`" min="`$f.min`" max="`$f.max`" step="`$f.step`" /><span class="w-field__unit">`$f.unit`</span></label>
}

@template &w-color($f) {
  <label class="w-field" data-kind="color" data-written="`$f.written`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" data-quote="`$f.quote`" data-drive="`$f.drive`"><span class="w-field__label" title="`$f.doc`">`$f.label`</span><input class="w-input w-swatch" type="color" value="`$f.hex`" /><input class="w-input w-text" type="text" value="`$f.value`" spellcheck="false" /></label>
}

@template &w-toggle($f) {
  <label class="w-field" data-kind="toggle" data-written="`$f.written`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" data-quote="`$f.quote`" data-drive="`$f.drive`"><span class="w-field__label" title="`$f.doc`">`$f.label`</span><input class="w-input w-check" type="checkbox" checked="`$f.on`" /></label>
}

@template &w-text($f) {
  <label class="w-field" data-kind="text" data-written="`$f.written`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" data-quote="`$f.quote`" data-drive="`$f.drive`"><span class="w-field__label" title="`$f.doc`">`$f.label`</span><input class="w-input w-text" type="text" value="`$f.value`" spellcheck="false" /></label>
}

/// A shorthand's longhands on one row (`pos`: X Y Z), each its own number.
@template &w-vec3($f) {
  <div class="w-field w-group" data-kind="vec3" data-written="`$f.written`"><span class="w-field__label" title="`$f.doc`">`$f.label`</span><span class="w-parts"></span></div>
  .w-parts {
    @each($f.parts as $p) {
      <label class="w-field w-part" data-kind="number" data-written="`$p.written`" data-label="`$p.label`" data-file="`$p.file`" data-start="`$p.start`" data-end="`$p.end`" data-expect="`$p.expect`" data-at="`$p.at`" data-before="`$p.before`" data-quote="false" data-drive="`$p.drive`"><span class="w-field__label w-field__label--scrub" title="`$p.label`">`$p.axis`</span><input class="w-input w-number" type="number" value="`$p.value`" step="`$p.step`" /></label>
    }
  }
}

@template &w-choice($f) {
  <div class="w-field" data-kind="choice" data-written="`$f.written`" data-label="`$f.label`" data-file="`$f.file`" data-start="`$f.start`" data-end="`$f.end`" data-expect="`$f.expect`" data-at="`$f.at`" data-before="`$f.before`" data-quote="`$f.quote`" data-drive="`$f.drive`"><span class="w-field__label" title="`$f.doc`">`$f.label`</span><span class="w-choices"></span></div>
  .w-choices {
    @each($f.choices as $c) {
      <button type="button" class="w-choice" data-on="`$c.on`" data-word="`$c.word`" title="`$c.doc`">`$c.word`</button>
    }
  }
}

// ---- behavior ---------------------------------------------------------------------

.w-field__label--scrub { @scrub; }

// ---- look ---------------------------------------------------------------------------

.w-field {
  display: grid; grid-template-columns: 84px 1fr auto; align-items: center; gap: 6px;
  margin: 0 0 6px; font: 11px/1.4 Inter, -apple-system, sans-serif; color: #1e1e1e;
}
.w-field[data-kind="range"] { grid-template-columns: 84px 1fr 58px auto; }
.w-field[data-kind="color"] { grid-template-columns: 84px 28px 1fr; }
.w-field__label { color: #6b6b6b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
/* a param at its default (a template writes a false flag as no attribute; an
   @each inside one writes it as "false") */
.w-field:not([data-written]) > .w-field__label,
.w-field[data-written="false"] > .w-field__label { color: #a3a3a3; }
/* a param a score drives: its edits key the stop at the playhead */
.w-field[data-drive]:not([data-drive=""]) > .w-field__label::after { content: " ◆"; color: #e8a13d; }
.w-group { grid-template-columns: 84px 1fr; }
.w-parts { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px; }
.w-part { display: grid; grid-template-columns: 12px 1fr; gap: 3px; margin: 0; }
.w-part > .w-field__label { color: #8c8c8c; text-transform: uppercase; font-size: 10px; }
.w-field__unit { color: #8c8c8c; min-width: 0; }
.w-input {
  box-sizing: border-box; width: 100%; min-width: 0; height: 24px; padding: 2px 6px;
  border: 1px solid transparent; border-radius: 4px; background: #f5f5f5; color: inherit; font: inherit;
}
.w-input:hover { border-color: #e6e6e6; }
.w-input:focus { outline: none; border-color: #0d99ff; background: #fff; }
.w-slider { padding: 0; background: transparent; accent-color: #0d99ff; }
.w-swatch { padding: 0; width: 28px; height: 24px; background: transparent; cursor: pointer; }
.w-check { width: 16px; height: 16px; accent-color: #0d99ff; }
.w-choices { display: flex; flex-wrap: wrap; gap: 2px; grid-column: 2 / -1; }
.w-choice {
  height: 22px; padding: 0 8px; border: 1px solid #e6e6e6; border-radius: 4px;
  background: #fff; color: #333; font: inherit; cursor: pointer;
}
.w-choice:hover { background: #f5f5f5; }
.w-choice[data-on="true"] { background: #0d99ff; border-color: #0d99ff; color: #fff; }
