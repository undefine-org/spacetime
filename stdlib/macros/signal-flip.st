/// Animate positional changes caused by a subscribed signal write.
///
/// Place `@flip` on the element whose server-driven position changes:
///
/// .card {
///   @flip $card(duration: 240ms)
/// }
///
/// The signal must be supplied by `@data subscribe`. Its assign diff writes
/// through the established local channel; @flip only measures and animates the
/// resulting layout change.
%macro signal-flip {
  %scope selector

  %form {
    @flip $signal:binding(
      duration: $duration:time = 300ms,
      easing: $easing:easing = ease-out
    )
  }

  %binds {
    signal-flip(&self, signal: $signal, duration: $duration, easing: $easing)
  }
}
