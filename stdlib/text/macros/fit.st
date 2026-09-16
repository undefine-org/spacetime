/// Fit Text Macro (PLAN-024 W5)
///
/// Shrink-or-grow text to fit its container at the largest size that stays
/// within `maxLines`, powered by the vendored pretext engine. No overflow, no
/// reflow flash; resize-stable.
///
/// @example single-line shrink-to-fit headline
/// .fit-headline {
///   @fit(min: 16, max: 120)
/// }
///
/// @example allow up to two lines
/// .fit-block {
///   @fit(min: 12, max: 80, maxLines: 2)
/// }

%macro fit {

  %form {
    @fit(
      min: $min:number = 8,
      max: $max:number = 200,
      maxLines: $maxLines:number = 1,
      lineHeight: $lineHeight:number = 1.1
    )
  }

  %binds {
    fit-text(&self, min: $min, max: $max, maxLines: $maxLines, lineHeight: $lineHeight) -> { $fitSize, $lines }
  }
}
