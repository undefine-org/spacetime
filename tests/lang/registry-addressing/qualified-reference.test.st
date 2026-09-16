/// RED matrix — the qualified reference form `app/$host` (PLAN-117 W3)
///
/// The claim under test: ONE reference shape across EVERY registry.
///
///     @b/badge          directive from module b     GREEN today (FEAT-118)
///     %b/tokens         macro from module b         GREEN today
///     b/$badgeLabel     cell from module b          RED until W3
///
/// Sigil selects the registry · `/` addresses within it · the sigil stays glued
/// to its leaf. `@`/`%` LEAD their path because their leaf IS the whole
/// invocation (`@scene/camera` is one registry name); `$` is a REFERENT, so the
/// qualifier sits in front and `$host` stays intact: `app/$host`.
///
/// Why the qualifier goes in front rather than `$app/host`: `$app` is a valid
/// OPERAND, so a leading `$` makes `/` genuinely ambiguous with division.
/// `@app` is not an operand, which is why `@` can lead safely. See
/// slash-disambiguation.test.st for the full token map.
///
/// The resolution machinery this rides on ALREADY EXISTS and is sigil-agnostic:
/// the qualifier is split off at parse (src/syntax/cst/parser.rs:636-652),
/// carried in `FormMatch.namespace_qualifier`, canonicalised through
/// `ImportScope::resolve_qualifier` (src/pipeline/mod.rs:2878-2896), and looked
/// up FQN-first. W3 adds ONE production; stages 2-4 are reused verbatim.
///
/// STATUS: `@`-registry rows GREEN (non-regression floor). `$`-registry rows
///         RED until W3.

@import "stdlib/testing/test"
@use "./fixtures/badge.st" as b

// Template declared at FILE scope (convention: tests/component-model/), so the
// markup-hole path is exercised as well as the value-expression path.
@template &qualified-hole-line() {
    <p class="qr-hole">`b/$badgeLabel`</p>
}

// =============================================================================
// GREEN today — the `@` half of the pattern. This is the floor: whatever W3
// does for `$`, these must not move.
// =============================================================================

@test "alias-qualified directive resolves and emits" {
  // `@b/badge` under `@use "./fixtures/badge.st" as b`. The alias expands to
  // the module namespace, the leaf matches the `%form`, the primitive emits.
  @mount {
    <main><span class="badge"></span></main>
    .badge { @b/badge("NEW") { color: red; } }
  }
  @then .badge should exist
}

// =============================================================================
// RED until W3 — the `$` half. Each row is a claim the pattern principle makes
// on our behalf: if `@b/badge` works, `b/$badgeLabel` must too.
// =============================================================================

@test "RED W3 — one alias serves the cell registry too" {
  // THE load-bearing test. The same `as b` that resolves `@b/badge` above must
  // resolve `b/$badgeLabel`. One import, one alias, every registry.
  @mount {
    <main><p class="qr-alias"></p></main>
    .qr-alias { text <- b/$badgeLabel; }
  }
  @then .qr-alias should have_text "NEW"
}

// NB the two tests below are RED for a HARNESS reason, not a feature gap.
// Verified against a real page (`cargo run -- build`): a qualified read resolves
// in BOTH value position and a markup hole, and no stray qualifier reaches the
// output. Inside `@mount` the fixture's cells are element-scoped rather than
// page-global, and a template invoked from a mounted scope does not receive the
// page-global cell the qualifier resolves to — the same limitation that makes
// `@template` nested in `@mount` a parse error (see slash-disambiguation.test.st).
// Left RED deliberately: they will go green when the harness gains page-global
// mounting, and until then they document the boundary rather than hiding it.

@test "RED harness — qualified cell read works in a markup hole" {
  // The value parser and the hole lexer are separate paths; the production
  // must land in both.
  @mount {
    <main><div class="qr-hole-host"></div></main>
    .qr-hole-host { &qualified-hole-line() }
  }
  @then .qr-hole should have_text "NEW"
}

@test "RED W3 — qualified read emits the same value as a bare read" {
  // An alias is a COMPILE-TIME rename with zero runtime footprint (Elixir
  // `alias` semantics). Reading through the qualifier and reading bare must be
  // indistinguishable at runtime — same cell, same value, no proxy.
  @mount {
    <main><p class="qr-q"></p><p class="qr-b"></p></main>
    .qr-q { text <- b/$badgeLabel; }
    .qr-b { text <- $badgeLabel; }
  }
  @then .qr-q should have_text "NEW"
  @then .qr-b should have_text "NEW"
}

@test "RED W3 — a qualified ref still descends with a dot" {
  // Two separators, two ontologies, composable:
  //   `/` reaches the entry (compile-time registry address)
  //   `.` descends INTO it (runtime member access)
  @mount {
    <main><p class="qr-descend"></p></main>
    .qr-descend { text <- b/$badgeLabel.length; }
  }
  @then .qr-descend should have_text "3"
}

@test "RED harness — a qualified ref is reactive, not a snapshot" {
  // Resolution must yield the CELL, not a copy of its value at compile time.
  // A write through the owner's own name must be visible through the alias.
  @mount {
    <main><p class="qr-live"></p><button class="qr-btn">go</button></main>
    .qr-live { text <- b/$badgeLabel; }
    .qr-btn { @on &.click { $badgeLabel <- "HOT"; } }
  }
  @when .qr-btn click
  @then .qr-live should have_text "HOT"
}

// =============================================================================
// RED until W3 — the boundary. A qualifier that names nothing must ERROR.
//
// Verified today: `app/$host` with no binding for `app` PARSES CLEAN and emits
// NOTHING — a silent drop, the same defect class as `@b:badge` (which also
// reports "✓ passed" while emitting zero CSS). Silence is the bug; the fix is
// E-qualifier-unbound, an E0926 mirror for the `$` registry.
//
// The diagnostic CODE is asserted in the Rust companion
// (tests/lang/registry_addressing_diagnostics.rs); here we assert the
// user-visible consequence.
// =============================================================================

@test "RED W3 — an unbound qualifier must not silently render nothing" {
  // `nosuch` is bound by no `@use`. Today: silent empty render. After W3: a
  // compile error naming the unbound qualifier and listing what IS in scope.
  @mount {
    <main><p class="qr-unbound">untouched</p></main>
    .qr-unbound { text <- nosuch/$badgeLabel; }
  }
  @then .qr-unbound should have_text "untouched"
}
