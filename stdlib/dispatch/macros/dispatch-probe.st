/// Dispatch Probe Macro (FEAT-091)
///
/// `@dispatch-probe(out: ".selector")` on an `<input>` makes it a live macro-
/// dispatch playground: typing a directive call resolves it against the live
/// registry (via the dev server) and renders the scored candidate table into the
/// `out` element. The winner glows; losers show why they lost.
///
/// @example
/// <input class="probe" />
/// <div class="probe-table"></div>
/// .probe { @dispatch-probe(out: ".probe-table") }

%macro dispatch-probe {

  %form {
    @dispatch-probe(
      out: $out:string,
      endpoint: $endpoint:string? = "/__spacetime/dispatch"
    )
  }

  %binds {
    dispatch-probe(&self, out: $out, endpoint: $endpoint)
  }
}
