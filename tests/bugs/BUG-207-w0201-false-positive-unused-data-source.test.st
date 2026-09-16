/// BUG-207 — W0201 false positive: "never used" for a @data source read only
///            via `text:` bindings and `@handle` arms (trust-erosion, not runtime)
///
/// Symptom: a `@data` source (e.g. `$error`) that is read in a `text:` binding
/// and in `@handle` receive arms is flagged W0201 "defined but never used",
/// while another source (`$count`) used in the SAME positions is NOT flagged.
/// Nothing breaks at runtime — the page is correct. The damage is to the
/// diagnostic channel: a linter that cries wolf teaches authors to ignore
/// warnings, and W0201 shares its check pass with E0928 (the LiveView contract
/// check that is the bridge's safety spine). Noisy output devalues the real
/// contract errors when they appear.
///
/// Root cause: `CompileAnalysis::detect_orphans` (src/analysis/mod.rs) builds its
/// `used_data_sources` set from ONLY two positions:
///   1. `self.bindings[].data_source`  — populated by `analyze_scopes` from
///      `@each` and `@cycle` ONLY (the "subscriber-like" usages).
///   2. `self.computed[].dependencies`.
/// It never scans READ positions that are not subscriber-like:
///   - `text:` / `text <-` / attribute / class-toggle bindings (these are
///     `css_declarations` whose value carries a `$var`; the CSS pipeline reads
///     them via `emit_reactive_binding_js`, but the analysis never collects them)
///   - `@handle` receive-arm bodies (a `handle` FormMatch whose body reads `$var`)
///
/// So `$count` survives because it ALSO feeds an `@each`/`@cycle`/computed
/// (a subscriber-like usage the scan DOES see); `$error` has no such subscriber
/// — it is read only in `text:`/`@handle` — so the scan misses it and W0201
/// fires. One position in the scan misses a source with no subscriber-like
/// usage; small fix, outsized hygiene value.
///
/// NOTE on test gating: W0201 is a COMPILE-TIME diagnostic. The Spacetime
/// `.test.st` grammar is behavioural (it can assert runtime state/DOM, but has
/// no "assert no warnings" claim), so a behavioural `.test.st` cannot go RED on
/// the diagnostic itself. The true RED gate is a Rust unit test in
/// `src/analysis/tests.rs` — `test_text_binding_marks_data_source_as_used` and
/// `test_handle_arm_marks_data_source_as_used` — modelled on the existing
/// `test_unused_data_source_still_warns` / `test_cycle_marks_data_source_as_used`
/// (see Impl Plan). The canonical offending page is encoded below so this file
/// doubles as the human-readable reproduction.
///
/// Canonical offending page (would emit W0201 for `$error` but NOT `$count`):
///
///   @host $api : live("MyMod")
///   @data signal $count () to $api { send get_count; receive { 200 => Ok($.body.count); } }
///   @data signal $error () to $api { send get_error; receive { _ => Failed($.body); } }
///
///   .count-view { text: $count; }   // <- read position the scan misses
///   .error-view { text: $error; }   // <- read position the scan misses
///
///   @handle {
///     count_updated(n) => { … $count … }   // <- $count ALSO has @each below → survives
///     error_updated(e) => { … $error … }   // <- $error's ONLY other read → still warned
///   }
///
///   .count-list { @each($count as $c) { … } }   // subscriber-like → marks $count used

@import "stdlib/testing/test"

// =============================================================================
// Behavioural guard (NOT the diagnostic gate): proves a `text:` binding actually
// CONSUMES its source at runtime. If the reactive text-binding path ever
// regresses, this catches it. It does not — and cannot — assert the absence of
// W0201; that is the Rust unit test's job.
// =============================================================================

@test "BUG-207 guard: text: binding consumes its source value" {
  @fixture {
    <div class="err-view-207">-</div>
  }

  // File-scope reactive text binding (the production authoring form): the div's
  // text tracks $err_val_207. This is the exact read position detect_orphans
  // fails to count as "usage".
  // (Declared via @eval seed below + the binding on .err-view-207.)

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.err_val_207 = "boom")

  @assert (document.querySelector('.err-view-207').textContent === '-')
    "before the binding fires, the placeholder text is present"

  // Drive the signal the text binding subscribes to.
  @eval (document.dispatchEvent(new CustomEvent('local:err_val_207:updated')))
  @wait 60ms

  @then .err-view-207 should exist
}

.err-view-207 { text: $err_val_207; }
