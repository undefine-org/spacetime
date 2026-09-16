/// GH-26: E0408 ("signal used but not defined") is implemented in
/// src/analysis/signals.rs but never invoked by the production check_file path.
/// Unknown reactive signals therefore emit runtime lookups and render EMPTY
/// instead of failing compilation.
///
/// Transition contract for this gate:
///   * TODAY (bug live): this file COMPILES clean (zero diagnostics) and
///     `.probe { text <- $definitelyMissing }` renders EMPTY. The assertion
///     `__probeText !== ""` therefore FAILS → the gate is RED, naming the
///     missing behavior ("undefined signal must render non-empty text OR
///     produce a compile diagnostic"). check --verbose currently emits no
///     E0408 — only the unrelated E0923 ambiguity + W0201 (see evidence.txt).
///   * WHEN E0408 IS WIRED (fix lands): this file FAILS TO COMPILE with the
///     E0408 diagnostic on `$definitelyMissing`. A `.test.st` that fails to
///     compile is reported as a FAILED file — which is the INTENDED GREEN
///     signal here: the missing-signal diagnostic is now firing, exactly the
///     desired end-state. So "RED today, GREEN (fails-to-compile w/ E0408)
///     when fixed" is the honest reading; the assertion below only exists for
///     the interim where the build still compiles.
///
/// shape mirrors index.st: `@data inline $present : "ok"` + a binding that
/// reads the never-defined `$definitelyMissing`.

@import "stdlib/testing/test"

@data inline $present : "ok";

@test "undefined signal $definitelyMissing must render non-empty or be rejected" {
  @mount {
    <div class="probe"></div>
    .probe { text <- $definitelyMissing; }
  }
  @flush
  @wait 50ms
  @eval (window.__probe = document.querySelector('.probe'))
  @eval (window.__probeText = window.__probe ? window.__probe.textContent : 'NO-PROBE')
  // RED today: __probeText is "" (silent empty render, no diagnostic).
  // Goes GREEN-by-compile-failure once E0408 is wired (see header contract).
  @assert (window.__probeText !== '')
    "undefined signal $definitelyMissing must render non-empty text OR produce a compile diagnostic (today it silently renders empty)"
}
