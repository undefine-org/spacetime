/// Fixture module carrying BOTH a `%`/`@` definition and a `$` cell, so the
/// matrices can prove the load-bearing claim of PLAN-117: ONE alias serves
/// EVERY registry.
///
///   @use "./fixtures/badge.st" as b
///     @b/badge(...)   directive   — works today (FEAT-118)
///     b/$badgeLabel   cell        — RED until W3
///
/// If the alias worked for `@` but not for `$`, the language would fail its own
/// principle: knowing one thing should tell you the next.

%macro badge {
    %form {
        @badge($label:string) {
            $styles:properties
        }
    }
    %binds {
        badge-emit($label, styles: $styles) -> {}
    }
}

%primitive badge-emit {
    %emit css {
        .badge::before { content: "$label"; }
    }
}

$badgeLabel string: "NEW";
