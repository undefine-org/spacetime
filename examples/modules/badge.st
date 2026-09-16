/// `badge` module — a tiny namespaced module for the FEAT-118 module-system
/// demo. It defines ONE public directive, `@badge`, that renders a CSS
/// `::before` label on the host element.
///
/// Imported with `@use "./badge.st"` (namespaced), its `@badge` keys under the
/// module's namespace rather than flooding the global scope — so two modules can
/// each define a `@badge` without colliding. Referenced bare (`@badge`), via a
/// path qualifier (`@badge/badge`), or via an alias (`@b/badge` after
/// `@use "./badge.st" as b`).

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
