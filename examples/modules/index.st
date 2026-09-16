/// FEAT-118 module-system demo — the consumer page.
///
/// Demonstrates the full `@use` surface against the sibling `badge` module:
///
///   @use "./badge.st" as b      — namespaced import, bound to the alias `b`
///   @b/badge("…")               — ALIAS-qualified reference (resolves via the
///                                 per-file ImportScope → the module namespace)
///
/// Other forms the module system supports (see docs/specs/module-system.md):
///   @use "./badge.st"                  — open import; bare `@badge` resolves
///   @use "./badge.st" only (badge)     — allow-list; only `@badge` is visible
///   @use "./badge.st" hiding (other)   — deny-list; everything but `@other`
///   @b/typo  /  @typo/badge            — unbound qualifier → E0926
///   a name excluded by only/hiding     → E0927
///
/// This page compiles cleanly: `@b/badge` resolves through the alias and emits
/// the badge CSS.

@use "./badge.st" as b

<main>
    <span class="badge"></span>
</main>

.badge {
    @b/badge("NEW") {
        color: red;
    }
}
