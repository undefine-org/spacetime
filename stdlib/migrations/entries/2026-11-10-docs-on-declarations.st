/// A meta wave (PLAN-156): it rewrites declarations — `%macro`, `%primitive` —
/// not directive calls, with a built-in transform. Run on the stdlib with
/// `spacetime migrate stdlib --meta`; a project's own macros take it with
/// `spacetime migrate <project>`.
%migration docs-on-declarations {
  %date 2026-11-10
  %scope meta
  %docs "PLAN-156: a /// @param or @yields line moves onto the param or export it describes; a doc that only restated a default is dropped."
  %transform docs-onto-declarations
}
