/// Gate: a `@form style` body may splice another form (BUG-327)
///
/// A style form whose body spliced another form emitted NO CSS AT ALL — not
/// the inner form's declarations, and not even its own sibling declarations.
/// The whole rule vanished, `check` reported `✓ All 1 file(s) passed`, and
/// `build` succeeded. That is the project's banned bug class (BUG-229 /
/// BUG-243 lineage: silent acceptance, silent drop) inside the form system
/// itself.
///
/// Composition is what makes design "schools" swappable: a pattern splices a
/// ROLE, a school binds that role to its own chunk, and switching school is a
/// block of rebindings instead of an edit to every splice in every pattern.
///
///   .hero__title { --role-title; }                  // pattern names a ROLE
///   @form style --role-title { --sc-min-title; }    // school binds it
///
/// These claims are `needs layout`: they read COMPUTED style, so they need a
/// real CSSOM cascade. `--headless` (LinkeDOM) cannot apply a class rule to an
/// element, so even the one-level baseline fails there — which would read as
/// this bug rather than as the backend's ceiling.

@import "stdlib/testing/test"

@form style --leaf {
  font-size: 44px;
  color: #0f172a;
}

/// The composed form: splices `--leaf`, and carries a declaration of its own.
/// BOTH must survive.
@form style --mid {
  --leaf;
  font-weight: 700;
}

/// Depth 2 — composition must not be a one-level special case.
@form style --outer {
  --mid;
  letter-spacing: -0.02em;
}

.direct { --leaf; }
.nested { --mid; }
.deep   { --outer; }

// =============================================================================
// 1. THE BASELINE. One-level splice already worked; if this ever breaks, the
//    failures below mean something different and this file says so first.
// =============================================================================

@test "form splice: a direct splice expands (baseline)" needs layout {
  @fixture { <div class="direct">d</div> }
  @wait 50ms

  @eval (window.__d = getComputedStyle(document.querySelector('.direct')).fontSize)
  @assert (window.__d === '44px')
    "a one-level `--leaf;` splice must expand — if THIS fails, the form system is broken more fundamentally than BUG-327"
}

// =============================================================================
// 2. THE BUG. The inner form's declarations must reach the consumer.
// =============================================================================

@test "form splice: a form body's splice reaches the consumer" needs layout {
  @fixture { <div class="nested">n</div> }
  @wait 50ms

  @eval (window.__n = getComputedStyle(document.querySelector('.nested')).fontSize)
  @assert (window.__n === '44px')
    "`--mid` splices `--leaf`, so `.nested` must get `font-size: 44px` — BUG-327 emitted NOTHING for this rule, silently, with `check` reporting success"
}

// =============================================================================
// 3. THE SIBLING. `--mid`'s OWN declaration must survive too. This is what
//    proves the bug was "the whole rule is voided" rather than merely "the
//    inner splice does not expand" — and it is the assertion that catches a
//    fix which drops the splice instead of expanding it.
// =============================================================================

@test "form splice: a composed form keeps its own declarations" needs layout {
  @fixture { <div class="nested">n</div> }
  @wait 50ms

  @eval (window.__nw = getComputedStyle(document.querySelector('.nested')).fontWeight)
  @assert (window.__nw === '700')
    "`--mid` declares `font-weight: 700` beside its splice — a fix that expands the splice but drops the sibling declaration has replaced one silent loss with another"
}

// =============================================================================
// 4. DEPTH. Composition at two levels, so a fix cannot be a single-hop
//    special case.
// =============================================================================

@test "form splice: composition nests more than one level deep" needs layout {
  @fixture { <div class="deep">x</div> }
  @wait 50ms

  @eval (window.__x1 = getComputedStyle(document.querySelector('.deep')).fontSize)
  @eval (window.__x2 = getComputedStyle(document.querySelector('.deep')).letterSpacing)
  @assert (window.__x1 === '44px')
    "`--outer` splices `--mid` which splices `--leaf`, so the leaf's declaration must reach two levels up"
  @assert (window.__x2 !== 'normal')
    "`--outer`'s own declaration must survive its splice, exactly as `--mid`'s does"
}

// =============================================================================
// 5. THE CYCLE. `--cyc-a` splices `--cyc-b` which splices `--cyc-a`: there is no
//    fixed point, so expansion must REFUSE by name rather than loop forever or
//    emit the silently empty rule the unexpanded case used to produce.
//
//    This claim is about a COMPILE diagnostic, so it is pinned in Rust
//    (tests/bug_327_form_composition_test.rs) where the compiler's exit status
//    and message are observable. A `.test.st` cannot host it: a file that fails
//    to compile cannot also make an assertion about why.
// =============================================================================
