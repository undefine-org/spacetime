/// Field control macros — the `@`-surface for the composite widgets.
///
/// A `%primitive` is not addressable from a selector scope on its own; a
/// `%macro` is what binds it to `&self` and gives it a `@directive` spelling
/// (the `stdlib/text` `@measure` pattern). Without these bridges the widget
/// templates' `@field-*-control` attachments resolve to nothing, and the
/// composite widgets render but never respond — which is exactly what happened
/// before this file existed: sliders moved and no value was ever composed.
///
/// Simple widgets (text/number/toggle) need no control: the browser's own input
/// IS the control, and the consumer's delegated `@on input` reads it directly.
/// Only the composite widgets — whose visible control is not the element that
/// carries the value — need logic to recompose one from the other.

/// Range: the slider tunes a NUMBER, the twin carries `number + unit`.
/// Attach on the field root; it wires the slider to its twin + readout.
%macro field-range-control {
  %form {
    @field-range-control()
  }
  %binds {
    field-range-control(&self)
  }
}

/// Color: four HSV channel sliders recompose a hex into the text input, which
/// is the single element the consumer's persist rail reads.
%macro field-color-control {
  %form {
    @field-color-control()
  }
  %binds {
    field-color-control(&self)
  }
}

/// Toggle: `checked` is DOM state, not a renderable attribute, so the shared
/// checkbox must be hydrated from its descriptor after mount.
%macro field-toggle-control {
  %form {
    @field-toggle-control()
  }
  %binds {
    field-toggle-control(&self)
  }
}
