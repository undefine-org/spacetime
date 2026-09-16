/// Scroll Spy Macro
///
/// Tracks whether this section is the one currently at the viewport center.
/// Use with @when or reactive props (.cls: $active) to react to section activation.
///
/// @example nav theming
/// .work-section {
///   @scroll-spy
///   @when &self.scrollSpy.isActive {
///     .nav { background: navy; }
///   }
/// }

%macro scroll-spy-macro {

  %form {
    @scroll-spy
  }

  %binds {
    scroll-spy(&self) -> { $isActive, $progress, $direction }
  }
}
