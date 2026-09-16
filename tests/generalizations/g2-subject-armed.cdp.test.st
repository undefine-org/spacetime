/// G2 — A TEST WHOSE SUBJECT NEVER INITIALIZED IS AS VACUOUS AS ONE THAT
/// ASSERTS NOTHING.
///
/// BUG-309 gave us the rule "a @test executing zero assertions FAILS". This is
/// its missing sibling. These tests DO execute assertions — against an element
/// that was never armed. The assertion runs; the feature under it was never
/// turned on. A green result means nothing.
///
/// BUG-325: `@fixture` inserts markup but never hands the element to the
/// element-initialization path a real page load runs, so a selector-scoped
/// primitive is never registered with its observer. GH-19 fixed exactly this
/// gap for the HEADLESS backend; the browser path still has it.
///
/// POLARITY: RED until BUG-325 lands. It is written to fail, and it fails for
/// its OWN reason — the first test proves the subject is unarmed, so the second
/// test's failure cannot be misread as a bug in the animation itself.
///
/// Run: spacetime test tests/generalizations/g2-subject-armed.cdp.test.st --cdp
@import "stdlib/testing/test"

.g2curtain {
  @on &.visible(200ms) as $reveal {
    opacity: 0 -> 1;
  }
}

@test "a fixture-mounted element is ARMED — its selector-scoped primitive registered" needs layout {
  // The precondition every viewport-driven browser gate silently assumes.
  // Asserting it FIRST means a failure downstream is attributable: if this
  // fails, nothing below it is evidence about the feature.
  @fixture {
    <div class="g2curtain" style="opacity: 0">x</div>
  }
  @wait 400ms
  @assert (document.querySelector('.g2curtain').style.opacity !== '0')
    "the element was mounted but never initialized — its @on &.visible timeline never ran, so any assertion about that timeline is vacuous (BUG-325)"
}

@test "the reveal reaches its end state once armed" needs layout {
  @fixture {
    <div class="g2curtain2 g2curtain" style="opacity: 0">x</div>
  }
  @wait 400ms
  @assert (document.querySelector('.g2curtain2').style.opacity === '1')
    "a one-shot visible reveal must reach opacity 1 — this is the claim the gate exists to make, and it is only meaningful if the test above passes"
}
