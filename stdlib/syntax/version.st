/// @version — the project's syntax-version fact (PLAN-076).
///
///   @version 2026-06-09;
///
/// Declares "this project's source is written against the Spacetime syntax
/// as of this migration-wave date". The migrations system
/// (`stdlib/migrations/`, docs/language/migrations.md) reads it to scope
/// WHICH waves participate when compiling this project:
///
///   - waves with `%date` ≤ `@version` are INERT — the project has already
///     crossed them; their retired directives are not matched, shimmed, or
///     counted as pending.
///   - waves with `%date` > `@version` participate normally.
///   - an absent `@version` means date-zero: every wave participates (the
///     bootstrap case for pre-migrations projects).
///
/// Written by the toolchain, not by hand: the FIRST persisted migration
/// apply (the migrations pill or `spacetime migrate`) INSERTS it at the top
/// of the root entry file; later applies UPDATE the date in place.
///
/// Like `@deploy`, this produces NO runtime JS/CSS (no `%binds`) — it
/// registers a pure fact (`version(...)`) that the compiler reads from the
/// parsed matches (`migrate::read_syntax_version`). One per project, in the
/// root entry file; duplicates are an E0911 authoring error.
///
/// `%order 54` — right before `@deploy` (55): the syntax version is the most
/// fundamental compile-time fact a file can carry.
%macro version {
  %scope file
  %order 54

  %form {
    @version $date:expr ;
  }

  %registers version() {
    $date
  }
}
