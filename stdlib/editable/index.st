/// Spacetime `editable` module — public re-exports (PLAN-031 / FEAT-097).
///
/// `@import "stdlib/editable"` brings in the built-in editable marks + blocks
/// (bold/italic/link/h2/p/figure/…). The `@editable-mark` / `@editable-block`
/// DEFINING macros live in `stdlib/macros/editable.st` and load automatically with
/// the rest of the macro set, so a project can declare its OWN marks without this
/// import; importing the module just adds the default vocabulary.

// The editable-surface primitive (the @editable macro's %binds target).
@import "./primitives/editable-surface.st"

// The default mark/block vocabulary.
@import "./builtins.st"

// The edit-mode toggle pill (@edit-toggle) for editable-everything pages.
@import "./edit-toggle.st"
