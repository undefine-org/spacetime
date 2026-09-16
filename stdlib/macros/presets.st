/// Preset system — RETIRED (SIP-001c / BUG-263).
///
/// The `~` preset mechanism (`%preset`/`@preset` declarations, `@apply`, and
/// `~name` references) is deleted. It never resolved at runtime: a `~name` in
/// an easing/duration slot was emitted verbatim and the runtime easing map fell
/// back to the default (the arc's silent-acceptance class). Named chunks of
/// language are now `@form` declarations; easings in particular are
/// `@form easing --name` (see the curve library below). The `~` sigil survives
/// only as the CSS general-sibling combinator.


/// Standard Easing Curves — FORMS, not entities (SIP-001c / BUG-263)
///
/// SIP-001 promises 28 named easing curves. They are `@form easing --name`
/// DECLARATIONS (the `form-easing` macro, form.st): each registers into the
/// form registry keyed by `--name`, and the drive expansion strips the `--`
/// sigil at the emission boundary (D12, SIP-001-W3 report) so the runtime
/// easing map (keyed BARE) resolves the name. This replaces the retired
/// the retired `%preset` easing-entity rows and deletes the `~` preset
/// token kind from the parser (BUG-263). The PUBLIC NAMES are unchanged
/// (`--ease-out-expo` is what `~ease-out-expo` was), so a migrated page
/// compiles identically.

@form easing --ease-in { cubic-bezier(0.42, 0, 1, 1) }
@form easing --ease-out { cubic-bezier(0, 0, 0.58, 1) }
@form easing --ease-in-out { cubic-bezier(0.42, 0, 0.58, 1) }

@form easing --ease-in-quad { cubic-bezier(0.55, 0.085, 0.68, 0.53) }
@form easing --ease-out-quad { cubic-bezier(0.25, 0.46, 0.45, 0.94) }
@form easing --ease-in-out-quad { cubic-bezier(0.455, 0.03, 0.515, 0.955) }

@form easing --ease-in-cubic { cubic-bezier(0.55, 0.055, 0.675, 0.19) }
@form easing --ease-out-cubic { cubic-bezier(0.215, 0.61, 0.355, 1) }
@form easing --ease-in-out-cubic { cubic-bezier(0.645, 0.045, 0.355, 1) }

@form easing --ease-in-quart { cubic-bezier(0.895, 0.03, 0.685, 0.22) }
@form easing --ease-out-quart { cubic-bezier(0.165, 0.84, 0.44, 1) }
@form easing --ease-in-out-quart { cubic-bezier(0.77, 0, 0.175, 1) }

@form easing --ease-in-quint { cubic-bezier(0.755, 0.05, 0.855, 0.06) }
@form easing --ease-out-quint { cubic-bezier(0.23, 1, 0.32, 1) }
@form easing --ease-in-out-quint { cubic-bezier(0.86, 0, 0.07, 1) }

@form easing --ease-in-sine { cubic-bezier(0.12, 0, 0.39, 0) }
@form easing --ease-out-sine { cubic-bezier(0.61, 1, 0.88, 1) }
@form easing --ease-in-out-sine { cubic-bezier(0.37, 0, 0.63, 1) }

@form easing --ease-in-expo { cubic-bezier(0.95, 0.05, 0.795, 0.035) }
@form easing --ease-out-expo { cubic-bezier(0.19, 1, 0.22, 1) }
@form easing --ease-in-out-expo { cubic-bezier(1, 0, 0, 1) }

@form easing --ease-in-circ { cubic-bezier(0.55, 0, 1, 0.45) }
@form easing --ease-out-circ { cubic-bezier(0, 0.55, 0.45, 1) }
@form easing --ease-in-out-circ { cubic-bezier(0.85, 0, 0.15, 1) }

@form easing --ease-in-back { cubic-bezier(0.6, -0.28, 0.735, 0.045) }
@form easing --ease-out-back { cubic-bezier(0.175, 0.885, 0.32, 1.275) }
@form easing --ease-in-out-back { cubic-bezier(0.68, -0.55, 0.265, 1.55) }

// The classic CSS keyword curve, the elastic family (not a single cubic-bezier
// — a spring body is the honest data), and the author-defined springs.
@form easing --linear { cubic-bezier(0, 0, 1, 1) }
@form easing --ease { cubic-bezier(0.25, 0.1, 0.25, 1) }
@form easing --ease-out-elastic { spring(400, 18, 1) }

@form easing --spring-gentle { spring(200, 20, 1) }
@form easing --spring-default { spring(300, 15, 1) }
@form easing --spring-bouncy { spring(400, 10, 1) }
@form easing --spring-stiff { spring(500, 25, 1) }


/// Import Macro
///
/// Imports presets and macros from other files.
///
/// @example
/// @import "stdlib/presets.st"
/// @import "./my-presets.st"

%macro import {

  %form {
    @import $path:string
  }

  // FEAT-118: @import DECLARES its compile-time effect (load + flat-global merge)
  // instead of being a hardcoded Rust special-case. `global: true` is the
  // degenerate point of the import spectrum ("@import floods, @use scopes").
  // The parser recognises any directive whose macro carries a global %imports
  // clause as an import (SyntaxRegistry::is_global_import).
  %imports {
    module: $path
    global: true
  }

  %registers import {
    path: $path
  }
}

/// Imports a module's directives under its NAMESPACE (FEAT-118).
///
/// Where `@import` floods definitions into the global namespace, `@use` scopes
/// them: the loaded module's macros register under a namespace derived from the
/// module path (e.g. `@use "std:scene"` → the module's `@camera` keys as
/// `scene/camera`). Two modules may therefore define the same `@directive`
/// without colliding. `global: false` is what distinguishes a scoped `@use`
/// from a flat `@import` — the namespaced point of the import spectrum.
///
/// @example
/// @use "std:scene"
/// @use "./components/cards.st"
/// @use "./components/cards.st" as c
/// @use "./components/cards.st" only (card, deck)
/// @use "./components/cards.st" hiding (internal)

/// The `@use` invocation TAIL, declared as grammar (BUG-296).
///
/// `as` / `only` / `hiding` were always real — `parse_use_clause_tail`
/// (parser/mod.rs) reads them into the per-file `ImportScope`, and the module
/// docs teach all three. But they were never DECLARED in the `%form`: they
/// survived only because a trailing token that no capture claimed used to be
/// silently ignored. BUG-264 correctly closed that leniency for body-less
/// forms, and `@use "…" as b` immediately became E0946 — the clause had no
/// grammar to match, so it read as junk after the directive.
///
/// The fix is to say what was always true. Declaring the tail keeps ONE source
/// of authority (the form registry) and makes the leniency-independent: `@use`
/// no longer relies on a token being dropped. `on_as` (`stdlib/macros/on.st`)
/// is the same shape — knowing one tells you the other.
%capture_type use_alias { "as" $name:ident }

/// The visibility filter — an allow-list or a deny-list, never both.
/// A union, so `only (…)` and `hiding (…)` are one optional slot: they occupy
/// the same position and are mutually exclusive by construction.
%capture_type use_filter {
  ( "only" "(" ( $only:ident ","? )* ")"
  | "hiding" "(" ( $hiding:ident ","? )* ")" )
}

%macro use {

  %form {
    @use $path:string $_alias:use_alias? $_filter:use_filter?
  }

  %imports {
    module: $path
    global: false
  }

  // Symmetric with `@import` above, and load-bearing for the same reason
  // (BUG-296). A directive that declares only `%imports` is consumed by the
  // PARSER (which lifts it into `file.imports` and `continue`s) but has nothing
  // to hand the expansion layer. That was invisible while an unresolved
  // primitive degraded to a `// Primitive not found` comment; BUG-268 correctly
  // made it a hard E0956, and `@use` — alone among the imports — had no
  // `%registers` to make it a consumed record. `@import` never showed the bug
  // because it always had one.
  %registers import {
    path: $path
  }
}

/// Declares which page-global cells this FILE publishes (PLAN-117 W5).
///
/// The same clause, the same `: mut` marker, and the same parser as a
/// `@template` body's `@exports` (FEAT-115) — lifted to file scope. Knowing one
/// tells you the other.
///
/// A file with NO `@exports` publishes everything (public-by-default — the Odin
/// floor), so every existing file keeps working unchanged. Once a file states an
/// interface, reading an unlisted cell through a module qualifier is E0944.
///
/// Reads may cross a module boundary; WRITES stay in the owning file (E0942).
/// That is not ceremony — it is what keeps a cell's write-set settleable by a
/// one-file scan, which is what makes const-folding sound.
///
/// This replaces the spec-reserved `%public (…)` manifest clause, which was
/// parsed but never enforced and had zero uses: one export mechanism, not two.
///
/// @example
/// @exports { $host, $session: mut }
%macro exports {

  %form {
    @exports { $body:balanced('}')? }
  }

  // The body is harvested by the parser into `StFile.file_exports` via the
  // canonical `parse_exports_block` (the single source of export value-semantics
  // and E0904 diagnostics). No %binds: this declares an INTERFACE, it does not
  // emit runtime behaviour.
  %registers exports {
    body: $body
  }
}

