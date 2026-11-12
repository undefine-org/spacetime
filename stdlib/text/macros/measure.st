/// Text Measurement Macro (PLAN-024 W3)
///
/// Reflow-free reactive text metrics for an element, powered by the vendored
/// pretext engine. Exposes `$lines`, `$height`, and `$widest` for use in
/// bindings, conditions, and downstream layout macros (@balance, @fit, …).

%macro measure {

  %form {
    @measure(
      lineHeight: $lineHeight:number = 1.5
    )
  }

  %binds {
    measure-text(&self, lineHeight: $lineHeight) -> { $lines, $height, $widest }
  }

  %example "@measure" {
    .headline {
      @measure -> { $lines, $height };
      /* now $lines / $height are reactive metrics of this element's text */
    }
  }

  %example "custom line height" {
    .body-copy {
      @measure(lineHeight: 1.6) -> { $lines };
    }
  }
}
