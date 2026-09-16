/// Unit Tests: Timeline Assertions
///
/// Timeline assertions consume named progress signals from the asserted element.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/timeline"

// =============================================================================
// Timeline Progress Assertions
// =============================================================================

@test "timeline progress >= threshold passes when above" {
  @fixture { <div class="animated" data-timeline-test>Animated</div> }

  @let (el = document.querySelector('.animated'))
  @eval (ST.set(el, 'test-intro', 0.7))

  @then .animated should have_timeline_progress "test-intro" >= 0.5
}

@test "timeline progress >= threshold fails when below" {
  @fixture { <div class="animated-fail" data-progress-fail>Animated</div> }

  @let (el = document.querySelector('.animated-fail'))
  @eval (ST.set(el, 'test-low', 0.3))

  @try { @then .animated-fail should have_timeline_progress "test-low" >= 0.5 }
  @assert ($error !== null)
    "Should throw when progress below threshold"
}

@test "timeline progress <= threshold passes when below" {
  @fixture { <div class="animated-lte" data-lte-test>Animated</div> }

  @let (el = document.querySelector('.animated-lte'))
  @eval (ST.set(el, 'test-lte', 0.3))

  @then .animated-lte should have_timeline_progress "test-lte" <= 0.5
}

@test "timeline progress == value with tolerance" {
  @fixture { <div class="animated-eq" data-eq-test>Animated</div> }

  @let (el = document.querySelector('.animated-eq'))
  @eval (ST.set(el, 'test-eq', 0.505))

  @then .animated-eq should have_timeline_progress "test-eq" == 0.5
}

// =============================================================================
// Timeline State Assertions
// =============================================================================

@test "timeline complete passes at progress one" {
  @fixture { <div class="complete-test">Complete</div> }

  @let (el = document.querySelector('.complete-test'))
  @eval (ST.set(el, 'complete-intro', 1))

  @then .complete-test should have_timeline_complete "complete-intro"
}

@test "timeline complete fails below progress one" {
  @fixture { <div class="incomplete-test">Incomplete</div> }

  @let (el = document.querySelector('.incomplete-test'))
  @eval (ST.set(el, 'incomplete-intro', 0.5))

  @try { @then .incomplete-test should have_timeline_complete "incomplete-intro" }
  @assert ($error !== null)
    "Should throw when timeline progress is incomplete"
}

@test "timeline running passes for intermediate progress" {
  @fixture { <div class="running-test">Running</div> }

  @let (el = document.querySelector('.running-test'))
  @eval (ST.set(el, 'running-anim', 0.5))

  @then .running-test should have_timeline_running "running-anim"
}

@test "has timeline passes for a published progress signal" {
  @fixture { <div class="has-tl-test">Has Timeline</div> }

  @let (el = document.querySelector('.has-tl-test'))
  @eval (ST.set(el, 'exists-timeline', 0))

  @then .has-tl-test should have_timeline "exists-timeline"
}

@test "has timeline fails when progress signal is absent" {
  @fixture { <div class="no-tl-test">No Timeline</div> }

  @try { @then .no-tl-test should have_timeline "nonexistent-timeline-xyz" }
  @assert ($error !== null)
    "Should throw when timeline signal is absent"
}

// =============================================================================
// Wait for Timeline
// =============================================================================

@test "wait_for_timeline resolves immediately when body signal is complete" {
  @fixture { <div class="wait-complete">Wait Complete</div> }

  @eval (ST.set(document.body, 'already-done', 1))

  @let (start = Date.now())
  @wait_for_timeline "already-done" timeout 1s
  @let (elapsed = Date.now() - start)

  @assert (elapsed < 100)
    "Should resolve immediately when already complete"
}

@test "wait_for_timeline waits for body signal completion" {
  @fixture { <div class="wait-async">Wait Async</div> }

  @eval (ST.set(document.body, 'async-done', 0))
  @eval (eval("setTimeout(function() { ST.set(document.body, 'async-done', 1) }, 100)"))

  @let (start = Date.now())
  @wait_for_timeline "async-done" timeout 2s
  @let (elapsed = Date.now() - start)

  @assert (elapsed >= 90 && elapsed < 500)
    "Should wait for completion (not instant, not timeout)"
}

@test "wait_for_timeline waits for descendant-owned completion" {
  @fixture { <div class="wait-child-owner">Wait Child Owner</div> }

  @let (el = document.querySelector('.wait-child-owner'))
  @eval (ST.set(el, 'child-async-done', 0))
  @eval (eval("setTimeout(function() { ST.set(document.querySelector('.wait-child-owner'), 'child-async-done', 1) }, 100)"))

  @let (start = Date.now())
  @wait_for_timeline "child-async-done" timeout 2s
  @let (elapsed = Date.now() - start)

  @assert (elapsed >= 90 && elapsed < 500)
    "Should wait for a descendant-owned timeline to complete"
}

@test "wait_for_timeline prunes detached timeline publishers" {
  @fixture { <div class="stale-timeline-owner"></div><div class="live-timeline-owner"></div> }

  @let (stale = document.querySelector('.stale-timeline-owner'))
  @let (live = document.querySelector('.live-timeline-owner'))
  @eval (ST.set(stale, 'reused-timeline-name', 1))
  @eval (stale.remove())
  @eval (ST.set(live, 'reused-timeline-name', 0))
  @eval (eval("setTimeout(function() { ST.set(document.querySelector('.live-timeline-owner'), 'reused-timeline-name', 1) }, 100)"))

  @wait_for_timeline "reused-timeline-name" timeout 2s
  @assert (ST._published.get('reused-timeline-name').has(stale) === false)
    "Should remove detached timeline publishers while finding a live owner"
}

// =============================================================================
// Animation State
// =============================================================================

@test "be_animating detects active animations" {
  @fixture { <div class="animating-test">Animating</div> }

  @let (el = document.querySelector('.animating-test'))
  @eval (el.animate([__obj("opacity", 0), __obj("opacity", 1)], __obj("duration", 1000, "iterations", Infinity)))
  @wait 50ms

  @then .animating-test should be_animating
}

@test "not_be_animating passes for static element" {
  @fixture { <div class="static-test">Static Element</div> }

  @then .static-test should not_be_animating
}

// =============================================================================
// Timing Instrumentation
// =============================================================================

@test "enable_timing and capture_timing_report work" {
  @fixture { <div class="timing-test">Timing Test</div> }

  @enable_timing
  @eval (ST._timing.recordRAF(0))
  @eval (ST._timing.recordRAF(16.67))
  @eval (ST._timing.recordRAF(33.34))
  @capture_timing_report

  @assert (__ctx.timingReport !== undefined)
    "Timing report should be captured"
  @assert (__ctx.timingReport.sampleCount === 2)
    "Should have 2 RAF delta samples"
}
