/// A meta wave (PLAN-156): it rewrites declarations — `%macro`, `%primitive` —
/// not directive calls, with a built-in transform. Run on the stdlib with
/// `spacetime migrate stdlib --meta`; a project's own macros take it with
/// `spacetime migrate <project>`.
%migration params-once {
  %date 2026-11-10
  %scope meta
  %docs "PLAN-156: %binds args that only forward a form param under its own name become one ...; the form is the one signature."
  %transform forward-binds
}
