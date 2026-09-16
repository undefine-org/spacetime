/// Balanced Text Macro (PLAN-024 W5)
///
/// Even out a multi-line headline so its ragged lines are visually balanced,
/// powered by the vendored pretext engine. Resize-stable and reflow-free, and
/// correct for CJK/RTL where CSS `text-wrap: balance` is weak or absent.
///
/// @example
/// .hero-title {
///   @balance
/// }
///
/// @example tune line-height used for the line-count target
/// .hero-title {
///   @balance(lineHeight: 1.1)
/// }

%macro balance {

  %form {
    @balance(
      lineHeight: $lineHeight:number = 1.2
    )
  }

  %binds {
    balance-text(&self, lineHeight: $lineHeight) -> { $balancedWidth, $lines }
  }
}
