/// Spacetime `enum` module — public surface (PLAN-077).
///
/// First-class tagged-union/enum types: `@type Name { A | B }` declarations
/// (declared in stdlib/macros/type-data.st — NOT owned here, see MODULE.st),
/// value-producing `@match` derives, variant-pattern dispatch, and generalized
/// state→CSS reflection. See MODULE.st for the design rationale.
///
/// REGISTRATION: this module is registry-scanned (it is in `STDLIB_DIRS`,
/// `src/metasystem/registry.rs` — the same always-on mechanism as the legacy
/// flat dirs and `stdlib/migrations/entries`), so its capture types resolve for
/// registry-scanned forms (e.g. `@match`/`@view` in this module's
/// dispatch.st reference `arm`). The @imports below are for module documentation and
/// explicit `@import "stdlib/enum"` consumers; they are parse-only no-ops for
/// the registry scanner (which registers every `.st` file's meta-defs directly).

// -----------------------------------------------------------------------------
// CAPTURE TYPES (the @match/@type grammar — kinds-as-data)
// -----------------------------------------------------------------------------

@import "./capture-types/variant-pattern.st"  // variant_pattern, variant_binding
@import "./capture-types/match-arms.st"       // match_pat, arm, match_value_arm, match_block
@import "./capture-types/cond-arms.st"        // cond_guard, cond_arm, cond_block, variant_construct, vc_arg
@import "./capture-types/inline-union.st"     // inline_variant_type, inline_variant_alt

@import "./derive.st"               // @data derive … : @match (cond-mode) + derive-match primitive

// -----------------------------------------------------------------------------
// DISPATCH / STATE (land in W5, W6)
// -----------------------------------------------------------------------------

// @import "./dispatch.st"  // W5: unified @match/@view (supersedes the retired view macro)
// @import "./state.st"     // W6: generalized state→CSS reflection writer
