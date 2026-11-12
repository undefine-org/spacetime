/// Dispatch Probe Macro (FEAT-091)
///
/// `@dispatch-probe(out: ".selector")` on an `<input>` makes it a live macro-
/// dispatch playground: typing a directive call resolves it against the live
/// registry (via the dev server) and renders the scored candidate table into the
/// `out` element. The winner glows; losers show why they lost.

%macro dispatch-probe {

  %form {
    @dispatch-probe(
      /// selector of the element to render the resolution table into
      out: $out:string,
      /// dispatch endpoint
      endpoint: $endpoint:string? = "/__spacetime/dispatch"
    )
  }

  %binds {
    dispatch-probe(&self, out: $out, endpoint: $endpoint)
  }

  %example "@dispatch-probe" {
    <input class="probe" />
    <div class="probe-table"></div>
    .probe { @dispatch-probe(out: ".probe-table") }
  }
}
