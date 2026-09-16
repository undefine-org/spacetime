/// @when — run state mutations while an element-local signal is truthy (BUG-088)
///
/// Pairs with any primitive that yields an element-scoped signal (notably
/// `@scroll-spy`). When the referenced signal becomes truthy, the guard body
/// runs — the SAME mutation grammar as an `@on` handler.
///
/// @example  nav theming driven by which section is centered
///   .work-section {
///     @scroll-spy
///     @when &self.scrollSpy.isActive { $navTheme <- "dark"; }
///   }
///
/// `&self.scrollSpy.isActive` reads this element's `isActive` signal (the
/// `scrollSpy.` prefix just documents which primitive owns it). The body uses
/// the ordinary `$signal <- value;` mutation surface and is executed via
/// `ST.runMutations`, so it can set any page/element state.

%macro when-guard {
  %order 700

  %form {
    @when &self.$signal:expr {
      $body:mutation_actions
    }
  }

  %binds {
    when-guard(
      &self,
      signal: $signal,
      body: $body
    )
  }
}
