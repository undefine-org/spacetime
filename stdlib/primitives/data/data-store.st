/// Data Store Module Primitive (PLAN-133 W3, FUP-175 R2)
///
/// A yield-less, param-free MODULE primitive: installs the shared global data
/// store once per page. `data-source` and `reactive-source` both declare
/// `%uses data-store`; before this module each carried a verbatim copy of the
/// init line, and a page using BOTH emitted it twice (prelude dedup is per
/// primitive name — it cannot catch cross-primitive copies).
%primitive data-store() {
  %emit prelude-js {
    window.__spacetimeData = window.__spacetimeData || {};
  }
}
