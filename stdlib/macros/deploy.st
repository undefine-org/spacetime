/// @deploy — Spacetime Host account/project config declaration (Wave 3).
///
/// Declares that a site is deployed via Spacetime Host, carrying the
/// project identity the `__host__` status widget (Wave 6) reads to know
/// which project it's showing status for, and which the CLI's `push`/
/// `publish` commands resolve against.
///
///   @deploy {
///     project: "my-blog";
///   }
///
/// A DISPLAY/CONFIG declaration only — like `@cms`, this produces NO
/// runtime JS/CSS (no `%binds`). It registers a pure fact
/// (`deploy(...)`) that:
///   - the compiler reads to know whether to compile the `__host__`
///     widget bundle into this site's build output at all (Wave 6:
///     "the widget's gate is NOT debug-mode, it's simply 'does the
///     compiled site declare `@deploy{}`'" — see
///     docs/architecture/V1_IMPLEMENTATION_PLAN.org §5's revision)
///   - the widget itself reads (via the SAME registry mechanism
///     `@cms`'s admin reads its hints through) to know which project id
///     to show status for / push against, without hand-rolling a
///     separate config-file convention.
///
/// `%scope file` is REQUIRED — confirmed against `MacroScope`'s only two
/// values (`File | Selector`): a top-level account/project config
/// directive is exactly the file-scope case, matching BOTH `@host` and
/// `@cms`'s own scope choice.
///
/// `%order 55` — right after `@cms` (51) and well before `@host` (250):
/// config declarations should resolve early, before anything that might
/// read `$deploy`'s registered facts (the `__host__` widget's own
/// compile-time check, Wave 6).
%macro deploy {
  %scope file
  %order 55

  %form {
    @deploy {
      $config:properties
    }
  }

  %registers deploy() {
    $config
  }
}
