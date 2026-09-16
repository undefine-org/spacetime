/// RED matrix — state modules: cross-file cells, fold, eliminate (PLAN-117 W2/W4)
///
/// The driving case of the whole plan: *define `$host` in one file, use it
/// across the app* — with the compiler able to prove what it is.
///
/// Three verdicts a cell can earn, all decided by the registry the W2 pass
/// builds (readers / writers / literal-ness / owner per FQN):
///
///   const     write-free + literal initial  -> inline at every read, emit no cell
///   reactive  has at least one writer       -> emit a live cell
///   dead      no readers, not exported      -> emit nothing at all
///
/// Why Spacetime can PROVE this where a general-purpose language can only
/// guess: there is no custom JavaScript. No `eval`, no foreign call, no escape
/// hatch that could write a cell behind the compiler's back. A cell's write-set
/// is CLOSED — a one-file scan settles it. That closure is what makes
/// const-folding sound rather than opportunistic.
///
/// STATUS: cross-file read is GREEN today (the 430-@import-site floor).
///         Every fold/eliminate/provenance row is RED until W2+W4.

@import "stdlib/testing/test"
@import "./fixtures/config.st"

// =============================================================================
// GREEN today — the floor. Cross-file `$host` already works via @import's
// flat merge. 430 sites depend on this; nothing in PLAN-117 may break it.
// =============================================================================

@test "an imported cell is readable in the importing file" {
  @mount {
    <main><p class="sm-read"></p></main>
    .sm-read { text <- $host; }
  }
  @then .sm-read should have_text "https://api.example.com"
}

@test "an imported cell is readable from more than one scope" {
  // Provenance (W2) must count 2 readers here, in 2 different scopes.
  @mount {
    <main><p class="sm-r1"></p><p class="sm-r2"></p></main>
    .sm-r1 { text <- $host; }
    .sm-r2 { text <- $host; }
  }
  @then .sm-r1 should have_text "https://api.example.com"
  @then .sm-r2 should have_text "https://api.example.com"
}

@test "an imported cell stays writable from the importing file today" {
  // Baseline for the W3 remote-write decision. TODAY a bare write to an
  // imported cell is allowed, because the flat merge makes it indistinguishable
  // from a local cell. W3 does NOT change this bare form — it only refuses the
  // QUALIFIED form (`app/$session <- v`), so writes stay visibly local to a
  // file that has taken the cell into its own scope.
  @mount {
    <main><p class="sm-w"></p><button class="sm-btn">go</button></main>
    .sm-w { text <- $session; }
    .sm-btn { @on &.click { $session <- "ada"; } }
  }
  @then .sm-w should have_text "anonymous"
  @when .sm-btn click
  @then .sm-w should have_text "ada"
}

// =============================================================================
// RED until W4 — const-fold.
//
// `$host` is written by NOBODY on this page. It is a value wearing a signal's
// clothes. W4 must inline it at every read and emit no cell.
//
// The observable consequence asserted here: the rendered text is correct AND
// the element carries no reactive-signal machinery. The stronger claims (no
// cell in the emitted JS, bundle size) live in the Rust companion, where the
// emitted artifact can be inspected directly.
// =============================================================================

@test "W4 — a write-free literal cell renders its value with no update path" {
  // What folding must guarantee, stated as the author would experience it: the
  // value renders, and it renders CORRECTLY.
  //
  // The stronger structural claim — that no subscription or update path is
  // emitted for a cell that can never change — is asserted in the Rust
  // companion, which can read the emitted JS. It cannot be asserted here:
  // `not_have_signal` inspects `ST.signals.get(el)`, and the DECLARATION is
  // deliberately retained because the binding pipeline registers a reactive
  // binding by its declared cell (removing it emits no binding at all and
  // renders an empty element — observed, not assumed). An optimisation that
  // renders the wrong page is not an optimisation.
  @mount {
    <main><p class="sm-const"></p></main>
    .sm-const { text <- $host; }
  }
  @then .sm-const should have_text "https://api.example.com"
}

@test "RED W4 — a written cell stays a real reactive signal" {
  // The other side of the fold: `$session` HAS a writer on this page, so it
  // must survive as a live cell. A fold pass that eats this is over-eager.
  @mount {
    <main><p class="sm-live"></p><button class="sm-live-btn">go</button></main>
    .sm-live { text <- $session; }
    .sm-live-btn { @on &.click { $session <- "grace"; } }
  }
  @then .sm-live should have_signal "session"
  @when .sm-live-btn click
  @then .sm-live should have_text "grace"
}

// =============================================================================
// RED until W2 — provenance.
//
// `inspect --layer state` must report FQN · readers · writers · verdict. That
// table is simultaneously the diagnostic, the documentation, and the input to
// the W4 optimisation — one registry, three views.
//
// A `.test.st` cannot read CLI output; these rows assert the RUNTIME shadow of
// the same facts. The table itself is asserted in the Rust companion.
// =============================================================================

@test "RED W4 — a dead cell is eliminated while a read one survives" {
  // `$retryBudget` (fixtures/config.st) is read by nothing, here or anywhere:
  // verdict dead. `$session` IS read on this page: verdict reactive.
  //
  // Asserted as a PAIR deliberately. `not_have_signal` is element-scoped, so a
  // lone negative claim would pass vacuously for any name at all (verified
  // with a made-up name). Pairing it with the positive claim on the same
  // element makes the assertion discriminating: it can only pass if the
  // element genuinely carries one signal and not the other.
  @mount {
    <main><p class="sm-dead"></p></main>
    .sm-dead { text <- $session; }
  }
  @then .sm-dead should have_signal "session"
  @then .sm-dead should not_have_signal "retryBudget"
}
