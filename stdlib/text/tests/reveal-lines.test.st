/// Text Module Tests: reveal split:"lines" fused onto pretext
///
/// NOTE: these .test.st assertions do not yet GATE — blocked on BUG-051 (see
/// PLAN-026). Verified meanwhile by tests/text_module_test.rs at the emit level.
///
/// When stdlib/text is imported, the core @reveal primitive's "lines" mode uses
/// the vendored pretext engine to group visual lines arithmetically (no DOM
/// reflow) instead of the offsetTop fallback. Runs in real Chromium.

@import "stdlib/testing/test"
@import "stdlib/text"

// =============================================================================
// Line splitting produces per-line reveal units
// =============================================================================

@test "reveal lines splits wrapped text into multiple line spans" {
  @fixture {
    <div class="rl-multi" style="width: 120px; font: 16px/1.5 sans-serif">
      The quick brown fox jumps over the lazy dog
      @reveal(split: "lines", trigger: "load", stagger: 0, duration: 1)
    </div>
  }

  @let (el = document.querySelector('.rl-multi'))
  @eval (eval("window.__lineUnits = el.querySelectorAll('.st-reveal-line').length"))
  @assert (__lineUnits >= 2)
    "wrapped text should yield multiple .st-reveal-line units"
}

@test "reveal lines preserves full text across line spans" {
  @fixture {
    <div class="rl-text" style="width: 120px; font: 16px/1.5 sans-serif">
      alpha beta gamma delta epsilon zeta eta theta
      @reveal(split: "lines", trigger: "load", stagger: 0, duration: 1)
    </div>
  }

  @let (el = document.querySelector('.rl-text'))
  @eval (eval("window.__joined = Array.from(el.querySelectorAll('.st-reveal-line')).map(function(s){return s.textContent}).join(' ').replace(/\\s+/g,' ').trim()"))
  @assert (__joined.indexOf('alpha') === 0 && __joined.indexOf('theta') > 0)
    "every word should survive the line split, in order"
}

@test "reveal lines keeps an accessible full-text copy" {
  @fixture {
    <div class="rl-a11y" style="width: 120px; font: 16px/1.5 sans-serif">
      accessible reveal copy that wraps across lines
      @reveal(split: "lines", trigger: "load", stagger: 0, duration: 1)
    </div>
  }

  @let (el = document.querySelector('.rl-a11y'))
  @eval (eval("window.__sr = el.querySelector('.sr-only') ? el.querySelector('.sr-only').textContent : ''"))
  @assert (__sr.indexOf('accessible reveal copy') === 0)
    "an sr-only span should carry the full original text"
}
