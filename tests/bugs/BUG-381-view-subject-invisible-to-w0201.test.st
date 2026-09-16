/// BUG-381 — W0201 false positive: `@view`/`@match` subject invisible to
///           read-usages (trust-erosion, not runtime)
///
/// Symptom: a `@data` source consumed ONLY as the subject of a `@view`
/// (reactive dispatch) or `@match` (mount once) was flagged W0201 "defined but
/// never used" — while the browser mounted and re-mounted the matching arm on
/// every change. The page was always correct; the diagnostic channel cried
/// wolf.
///
/// NOTE on test gating (same shape as BUG-207): W0201 is COMPILE-TIME; the
/// `.test.st` grammar cannot assert on diagnostics. The RED gate is in
/// `src/analysis/tests.rs` (`view_subject_marks_source_used_bug381`,
/// `match_subject_marks_source_used_bug381`,
/// `dotted_view_subject_marks_base_source_used_bug381`).
///
/// This file is the BEHAVIOURAL GUARD: it proves a `@view` subject is a real
/// reactive read — the initial arm mounts, and changing the subject re-mounts
/// the matching arm. It passes before AND after the fix; the defect was in the
/// analysis, never in the runtime.

@import "stdlib/testing/test"
@import "stdlib/enum/dispatch"

@data inline $status381 string : "idle";

@template &idle381() {
  <span class="v381-idle">idle-view</span>
}

@template &thinking381() {
  <span class="v381-thinking">thinking-view</span>
}

@test "@view mounts the arm matching the initial subject value" {
  @fixture {
    <div class="status381a"></div>
  }
  @given .status381a {
    @view $status381 {
      "thinking" => &thinking381();
      _ => &idle381();
    }
  }

  @wait 50ms
  @assert (document.querySelector('.status381a .v381-idle') !== null)
    "the wildcard arm mounts for the initial subject value"
}

@test "@view re-mounts the matching arm when the subject changes" {
  @fixture {
    <div class="status381b"></div>
  }
  @given .status381b {
    @view $status381 {
      "thinking" => &thinking381();
      _ => &idle381();
    }
  }

  @wait 50ms
  @assert (document.querySelector('.status381b .v381-idle') !== null)
    "the wildcard arm mounts first"

  // The subject is a file-scope signal: the global page store is its home, and
  // dispatch-mount resolves through it (FUP-094).
  @eval (window.SpacetimeLocal.status381 = 'thinking')
  @wait 50ms

  @assert (document.querySelector('.status381b .v381-thinking') !== null)
    "changing the subject re-mounts the matching arm"
}

