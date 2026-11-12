/// A meta wave (PLAN-156): it rewrites declarations — `%macro`, `%primitive` —
/// not directive calls, with a built-in transform. Run on the stdlib with
/// `spacetime migrate stdlib --meta`; a project's own macros take it with
/// `spacetime migrate <project>`.
%migration examples-compile {
  %date 2026-11-10
  %scope meta
  %docs "PLAN-156: /// @example blocks become %example clauses, which spacetime check --examples compiles."
  %transform examples-to-clauses
}
