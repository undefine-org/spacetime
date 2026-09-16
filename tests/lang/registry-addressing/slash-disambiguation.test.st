/// RED matrix — `/` disambiguation (PLAN-117 W1)
///
/// The contract that lets the qualified-reference form `app/$host` ship safely.
/// `/` carries three jobs in a Spacetime file:
///
///   1. registry address   `@b/badge`, `app/$host`   — TIGHT, no trivia
///   2. division           `1 / $cols`               — SPACED
///   3. CSS slash value    `font: 15px/1.6`          — numeric operands
///
/// The disambiguation rule is TIGHT ADJACENCY, already implemented and tested
/// for `@` (src/syntax/cst/parser.rs:636-652 consumes `/IDENT` only when no
/// trivia surrounds the `/`; src/syntax/cst/ast.rs:283: "Trivia ends the name
/// run"). W3 mirrors that loop for a `$`-led leaf.
///
/// Why `/$` is safe where a leading `$` would not be: `@app` is not a valid
/// operand, so `@app/badge` can never be division. `$app` IS a valid operand,
/// so `$app/host` genuinely collides — which is why the qualifier goes in
/// FRONT and the sigil stays glued to its leaf.
///
/// Repo evidence for the rule (counted, not assumed):
///   - real division in expression position: 4 lines, one file
///     (stdlib/mobile/macros/gestures/swipe.st:38-41, all `/ 100`, spaced)
///   - division inside backtick holes: ZERO
///   - CSS slash values: 9 lines, all `font: 15px/1.6` shape
///
/// STATUS: division + CSS rows are GREEN today (non-regression guards).
///         Qualified-reference rows are RED until W3.

@import "stdlib/testing/test"

// Templates are declared at FILE scope (the convention in
// tests/component-model/); a `@template` nested inside `@mount` fails to parse,
// and `@each` with inline markup inside `@mount` likewise — both are
// pre-existing behaviours unrelated to this matrix, confirmed by bisection.
@template &avatar-line() {
    <p class="q-hole">`app/$host`</p>
}

// =============================================================================
// GREEN today — division must survive the new production untouched.
// These are the rows that make `/$` safe to ship: if any of them breaks, the
// disambiguation rule is wrong and W3 must not land.
// =============================================================================

@test "spaced slash between two bindings stays division" {
  @mount {
    <main><p class="div-bb"></p></main>
    $total number: 90;
    $count number: 3;
    .div-bb { text <- $total / $count; }
  }
  @then .div-bb should have_text "30"
}

@test "spaced slash with numeric LHS stays division" {
  // The case named explicitly during design review: `1 / $denominator`.
  // A numeric literal on the left, a binding on the right, spaces around `/`.
  @mount {
    <main><p class="div-nl"></p></main>
    $denominator number: 4;
    .div-nl { text <- 1 / $denominator; }
  }
  @then .div-nl should have_text "0.25"
}

@test "spaced slash with numeric RHS stays division" {
  // Matches the 4 real division sites in the repo (swipe.st: `-$deltaX / 100`).
  @mount {
    <main><p class="div-nr"></p></main>
    $delta number: 50;
    .div-nr { text <- $delta / 100; }
  }
  @then .div-nr should have_text "0.5"
}

@test "spaced slash after a paren group stays division" {
  @mount {
    <main><p class="div-paren"></p></main>
    $w number: 320;
    $gap number: 20;
    $cols number: 3;
    .div-paren { text <- ($w - $gap) / $cols; }
  }
  @then .div-paren should have_text "100"
}

@test "tight slash after a binding stays division" {
  // The closest neighbour to the new form: `$nine/3` is TIGHT, and its left
  // operand is a `$`-ref. It must stay division, because the qualified form
  // puts the sigil on the RIGHT of the slash (`app/$host`), never the left.
  // This row is the precise reason `app/$host` was chosen over `$app/host`.
  @mount {
    <main><p class="div-tight-sig"></p></main>
    $nine number: 9;
    .div-tight-sig { text <- $nine/3; }
  }
  @then .div-tight-sig should have_text "3"
}

@test "tight slash before a binding stays division" {
  // The mirror case, and the one the user named: a tight `/$` where the left
  // operand is a BINDING is unambiguous division — `$total` is an operand, so
  // no qualifier can start there. Only a BARE IDENT head opens the qualified
  // production; see the `1/$denominator` row below for the refusal case.
  @mount {
    <main><p class="div-tight-rhs"></p></main>
    $total number: 12;
    $part number: 4;
    .div-tight-rhs { text <- $total/$part; }
  }
  @then .div-tight-rhs should have_text "3"
}

// =============================================================================
// GREEN today — CSS slash VALUES. `/` in a CSS declaration value is a value
// separator, never an address. 9 real sites in the repo.
// =============================================================================

@test "css font shorthand slash is a value separator" {
  @mount {
    <main><p class="css-font">shorthand</p></main>
    .css-font { font: 15px/1.6 sans-serif; }
  }
  @then .css-font should exist
}

@test "css spaced grid-area slash is a value separator" {
  @mount {
    <main><div class="css-grid">cell</div></main>
    .css-grid { grid-area: 1 / 3; }
  }
  @then .css-grid should exist
}

@test "css tight aspect-ratio slash is a value separator" {
  @mount {
    <main><div class="css-ar">box</div></main>
    .css-ar { aspect-ratio: 16/9; }
  }
  @then .css-ar should exist
}

// =============================================================================
// GREEN today — the `@` half of the pattern already works. These rows are the
// non-regression floor: whatever W3 does to `$`, `@` must be untouched.
// =============================================================================

@test "spaced slash in directive value position is not a qualifier" {
  // Mirrors src/syntax/cst/parser.rs test_spaced_slash_not_a_qualifier:
  // `@grid cols: 1 / 3` resolves the directive name as `grid`, NOT `grid/3`.
  @mount {
    <main><div class="dir-spaced">g</div></main>
    .dir-spaced { grid-template-columns: 1fr / 3fr; }
  }
  @then .dir-spaced should exist
}

// =============================================================================
// RED until W3 — the qualified-reference form.
//
// NB these assert the OBSERVABLE CONSEQUENCE (rendered text), not the parse.
// Verified today: `app/$host` PARSES CLEAN and emits NO binding — a silent
// drop. So each of these runs and fails its claim, which is what a RED test
// must do (see tests/lang/README.md).
// =============================================================================

@test "RED W3 — alias-qualified read resolves to the owner cell" {
  @mount {
    <main><p class="q-alias"></p></main>
    $host string: "https://api.example.com";
    .q-alias { text <- app/$host; }
  }
  @then .q-alias should have_text "https://api.example.com"
}

@test "RED W3 — qualified read inside a markup hole resolves" {
  // The hole is where a qualified ref will be most used (`app/$host` inside
  // markup), and the hole lexer is a SEPARATE path from the value parser — so
  // the production must be proven in both. Templates are declared at file
  // scope (the convention in tests/component-model/) and invoked from a scope.
  @mount {
    <main><div class="q-hole-host"></div></main>
    $host string: "https://api.example.com";
    .q-hole-host { &avatar-line() }
  }
  @then .q-hole should have_text "https://api.example.com"
}

@test "RED W3 — a qualified ref may still descend with a dot" {
  // `/` reaches the entry; `.` descends INTO it. Two separators, two jobs.
  @mount {
    <main><p class="q-descend"></p></main>
    $session string: "ada";
    .q-descend { text <- app/$session.length; }
  }
  @then .q-descend should have_text "3"
}

@test "RED W3 — a bare read in the owning file is unaffected by qualification" {
  // The non-regression that matters most: adding the qualified form must not
  // change what an unqualified `$host` means anywhere.
  @mount {
    <main><p class="q-bare"></p></main>
    $host string: "https://api.example.com";
    .q-bare { text <- $host; }
  }
  @then .q-bare should have_text "https://api.example.com"
}

// =============================================================================
// RED until W3 — the REFUSAL rows.
//
// These are the deliberate cost of `/$`, approved in design review ("forcing a
// space is okay"). The rule the compiler must apply:
//
//   after `/`, a `$`-led leaf opens the QUALIFIED production
//   ONLY when the head before the `/` is a BARE IDENT.
//
// A head that is an operand (`$x`, a number, a paren group) keeps `/` as
// division. The one genuinely ambiguous shape is a TIGHT numeric head
// (`1/$denominator`): a number can never be a qualifier, so this is division —
// but it reads like the new form, so the compiler must REFUSE and name the fix
// rather than silently pick. Refusing beats guessing (docs/testing/FIDELITY_LADDER.md).
//
// NB these cannot be asserted from `.test.st` yet: there is no surface for
// claiming a COMPILE-TIME diagnostic (the `@then` vocabulary is entirely
// runtime/DOM/signal). The claims below assert the OBSERVABLE consequence; the
// diagnostic CODE is asserted in the Rust companion
// (tests/lang/registry_addressing_diagnostics.rs). PLAN-117 W1 decides whether
// to add a `@then compile should error "..."` claim shape and unify the two.
// =============================================================================

@test "RED W3 — tight numeric-head slash before a binding is refused, not guessed" {
  // `1/$denominator` — the compiler must REFUSE with help: "add spaces around
  // `/` to divide". Today it silently computes division; after W3 it errors.
  // Asserted here as: the page does not silently render a division result.
  @mount {
    <main><p class="amb-tight"></p></main>
    $denominator number: 4;
    .amb-tight { text <- 1/$denominator; }
  }
  @then .amb-tight { text != "0.25"; }
}

@test "RED W3 — the spaced form of the same expression is the sanctioned fix" {
  // The other half of the refusal contract: the help text must point at
  // something that WORKS. `1 / $denominator` is GREEN today and must stay so.
  @mount {
    <main><p class="amb-fix"></p></main>
    $denominator number: 4;
    .amb-fix { text <- 1 / $denominator; }
  }
  @then .amb-fix should have_text "0.25"
}

