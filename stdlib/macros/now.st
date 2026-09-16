/// @now Macro
///
/// Exposes the wall-clock primitive's `$now` signal. The primitive also writes
/// the formatted time directly to the host element's textContent on each tick.

%macro now-macro {
  %form {
    @now(
      every: $every:time = 1s
    ) {
      $_body:component_body
    }
  }

  %binds {
    now(&self, every: $every) -> { $now }
  }
}
