/// Spacetime `dispatch` module — live macro-dispatch playground.
///
/// `@dispatch-probe` turns an `<input>` into a live probe: as you type a Spacetime
/// directive call, it asks the dev server's `/__spacetime/dispatch` endpoint to
/// resolve it against every candidate `%macro` form and renders the SCORED table
/// (winner, per-term breakdown, why losers lost). The endpoint runs the compiler's
/// OWN scorer (`build_dispatch_probe`, FEAT-087) — the playground cannot disagree
/// with how Spacetime actually dispatches.
///
/// The only JavaScript lives inside the primitive's `%emit js` (the sanctioned
/// escape hatch, like `render-markdown` / `measure-text`). The endpoint is Rust;
/// the page wiring is `.st`. Dev-server only (the endpoint is a dev route).
///
/// @example
/// @import "stdlib/dispatch"
///
/// <input class="probe" data-probe-out=".probe-table" />
/// <div class="probe-table"></div>
/// .probe { @dispatch-probe(out: ".probe-table") }

@import "./index.st"
