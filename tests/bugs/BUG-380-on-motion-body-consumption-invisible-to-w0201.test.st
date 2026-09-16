/// BUG-380 — W0201 false positive: `@on` motion-body consumption invisible to
///           read-usages (trust-erosion, not runtime)
///
/// Symptom: a `@data` source consumed ONLY inside an `@on` handler body — as a
/// mutation target (`$flag <- !$flag`), as a fired signal (`$send($draft)`), or
/// as a signal-call argument — was flagged W0201 "defined but never used" by
/// `check --deny-warnings`. The page was always correct; the diagnostic channel
/// cried wolf.
///
/// NOTE on test gating (same shape as BUG-207): W0201 is a COMPILE-TIME
/// diagnostic. The `.test.st` grammar is behavioural — it cannot assert "no
/// warning was emitted". The RED gate is in `src/analysis/tests.rs`
/// (`on_signal_call_marks_fired_signal_and_args_used_bug380`,
///  `on_mutation_marks_target_and_expr_used_bug380`,
///  `edn_page_on_and_view_consumption_marks_sources_used_bug380_381`).
///
/// This file is the BEHAVIOURAL GUARD: it proves the consumption the analysis
/// now counts is real at runtime — a click runs the handler and the write
/// reaches a `text:` binding. It passes before AND after the fix; that is what
/// "the compiler was wrong, not the page" means.
///
/// The fired-signal channel (`$send($draft)`) is not asserted here: in real
/// pages the callee is a `data-signal` bound to a LiveView host, which a
/// headless world has no server for. The Rust tests carry that channel.

@import "stdlib/testing/test"

@data inline $flag380 string : "off";

@test "@on mutation writes its signal and the page store follows" {
  @fixture {
    <button class="flag380-btn">toggle</button>
  }
  @given .flag380-btn {
    @on &.click { $flag380 <- "on"; }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.flag380 = "off")

  @when .flag380-btn click
  @wait 50ms

  @assert (window.SpacetimeLocal.flag380 === "on")
    "a click runs the handler and the mutation reaches the file-scope signal"
}

