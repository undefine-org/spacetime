/// Behavior gate: the LAUNCH DEMO's headline reel actually moves (BUG-258 closed)
///
/// `demos/score-launch/index.st.md` is a literate page written while BUG-258 was
/// open, and it carried an honesty note that "nothing on this page moves yet".
/// BUG-258 is closed: clip-consumer (4/4) proves a score's window DRIVES an
/// element's style via `@on &.clip`. This file is the demo's half of that proof —
/// it runs the demo's OWN reel (the exact markup and scores from its section 1
/// fence) and asserts the three headline cards move on their assigned windows.
///
/// Why @mount and not @drive: the demo's built page references `/spacetime.js`
/// absolutely, so it only runs under the dev server (no file:// self-contained
/// build exists for a literate page). A committed gate cannot depend on a live
/// server, so the ACTUAL demo content — markup, `@score` body, `&.clip`
/// consumers, `--clip-rise` form — is mounted verbatim and driven in the warm
/// Chromium. Same program, real page semantics.
///
/// NEGATIVE-FIRST, exactly like clip-consumer: a clip whose window has not
/// opened must be STILL. The reel is a 9s loop; `title` owns 0s..3s, `body`
/// 3s..6s, `cta` 6s..9s. The windows DO NOT OVERLAP, so "still before its
/// window" is observable and nothing else on the page could have moved a card.
///
/// THE PAIRING IS THE TEST (the exact shape of a past failure): the score's
/// `&title` must match the consumer element's class-derived name. `@on &.clip`
/// on `.title` watches `__clip_title`; the score publishes `__clip_title`. If a
/// compound selector (`.card.title`) ever hosted the consumer, selector_element
/// would return None and nothing would join — a green build where nothing moves.
///
/// SCOPE OF THIS FILE: the reel (§1) and the rate lane (§5), mounted
/// verbatim. The demo's section-0 FILM, section-2 cover transition and
/// section-3 color interpolation are gated in `score-launch-film.test.st`
/// (split: the CDP runner budgets 30s per FILE and these cycles are long).
/// The demo's section-6 two-pole composition (a `&.clip` score nested inside
/// a parent score) is gated separately in `two-pole.test.st` — it depends on
/// two compiler fixes that this file's failures surfaced during development:
/// a nested `&.clip` score watching its parent window on its own node instead
/// of with `watchScoped` (BUG-266), and a bare `for 50%` under a `.time`
/// clock leaking the raw fraction against `total` (BUG-267). Both are fixed
/// and proved by the two-pole gate.
///
/// NB `--cdp` (real Chromium): "did this element move at time T" is a
/// paint-adjacent, timing-dependent claim; `--headless` refuses by design.

@import "stdlib/testing/test"

// --- The demo's section-1 program, verbatim --------------------------------
@form motion --clip-rise { opacity: 0 -> 1; }

.reel {
  @score &.loop(9s) as &reel {
    &title at 0s for 3s;
    &body  at 3s for 3s;
    &cta   at 6s for 3s;
  }
}

.card {
  position: absolute;
  left: 1.2rem;
  right: 1.2rem;
  top: 45%;
  padding: 0.9rem 1.1rem;
  border-left: 2px solid #5eead4;
  background: #10131a;
  color: #e5e7eb;
  font: 500 1.05rem/1.4 ui-sans-serif, system-ui, sans-serif;
  opacity: 0;
}

.title { @on &.clip: --clip-rise; }
.body  { @on &.clip: --clip-rise; }
.cta   { @on &.clip: --clip-rise; }

// --- The demo's section-2 rate lane, verbatim -------------------------------
@form motion --dot-move { translate-x: 0 -> 180px; }

.rates {
  @score &.loop(4s) {
    &normal at 0s for 4s;
    &fast   at 0s for 4s rate 2;
    &slow   at 0s for 4s rate 0.5;
    &back   at 0s for 4s rate -1;
  }
}

.dot {
  translate-x: 0;
}

.normal { @on &.clip: --dot-move; }
.fast   { @on &.clip: --dot-move; }
.slow   { @on &.clip: --dot-move; }
.back   { @on &.clip: --dot-move; }

// =============================================================================
// 1. THE NEGATIVE — the `cta` card owns 6s..9s; before 6s it must be STILL. A
//    nonzero value here means the card is driven by the WRONG signal (e.g. the
//    reel's raw progress instead of its own window) or by no window at all.
// =============================================================================

@test "score-launch: a clip outside its window has not moved" needs timing {
  @mount {
    <div class="reel">
      <div class="card title">A score is an arrangement</div>
      <div class="card body">Three clips, one playhead</div>
      <div class="card cta">Nothing here is JavaScript</div>
    </div>
  }

  @wait 300ms

  @eval (window.__sl1 = parseFloat(getComputedStyle(document.querySelector('.cta')).opacity))
  @assert (window.__sl1 === 0)
    "at 300ms the `cta` card's window (6s..9s) has not opened, so it must be untouched — a nonzero value means it is being driven by the reel's raw progress instead of its own window"
}

// =============================================================================
// 2. …and a clip INSIDE its window must move. Without this the negative above
//    is satisfied by a page where nothing works at all.
// =============================================================================

@test "score-launch: a clip inside its window has moved" needs timing {
  @mount {
    <div class="reel">
      <div class="card title">A score is an arrangement</div>
      <div class="card body">Three clips, one playhead</div>
      <div class="card cta">Nothing here is JavaScript</div>
    </div>
  }

  @wait 500ms

  @eval (window.__sl2 = parseFloat(getComputedStyle(document.querySelector('.title')).opacity))
  @assert (window.__sl2 > 0)
    "at 500ms `title` is ~1/6 through its 0s..3s window, so it must have moved — 0 here is BUG-258 exactly: the window is published and no `&.clip` consumer reads it"
}

// =============================================================================
// 3. THE WINDOW IS A MAP, NOT A SWITCH. `title` must be DONE before `cta`
//    starts. Distinguishes a real window map from "every clip plays the reel".
// =============================================================================

@test "score-launch: an earlier clip completes before a later one begins" needs timing {
  @mount {
    <div class="reel">
      <div class="card title">A score is an arrangement</div>
      <div class="card body">Three clips, one playhead</div>
      <div class="card cta">Nothing here is JavaScript</div>
    </div>
  }

  @wait 3400ms

  @eval (window.__sl3a = parseFloat(getComputedStyle(document.querySelector('.title')).opacity))
  @eval (window.__sl3b = parseFloat(getComputedStyle(document.querySelector('.cta')).opacity))

  @assert (window.__sl3a > 0.9)
    "at 3400ms `title`'s window (0s..3s) has closed, so it must be at its end value — below 1 means the clip is tracking the reel's raw progress (0.38 at this instant) rather than its own window"
  @assert (window.__sl3b === 0)
    "at 3400ms `cta`'s window (6s..9s) has not opened — nonzero means windows are not being honored at all"
}

// =============================================================================
// 4. THE JOIN. By 6.5s the LAST card (`cta`) must have moved. If clip windows
//    were wired by counter rather than by element (the naming defect in the
//    task brief), this is what catches it.
// =============================================================================

@test "score-launch: the later clip moves when its own window opens" needs timing {
  @mount {
    <div class="reel">
      <div class="card title">A score is an arrangement</div>
      <div class="card body">Three clips, one playhead</div>
      <div class="card cta">Nothing here is JavaScript</div>
    </div>
  }

  // A fixed @wait cannot know the driver's startTime: mount + warm-Chromium
  // overhead lands anywhere in the 9s cycle, and a restarted loop has dead
  // zones. So SAMPLE the trace and assert over it — phase-independent, with
  // the ordering claim (cta still before its window, moving inside it) kept.
  @eval (window.__sl4s = [], window.__sl4t = setInterval(function () {
    window.__sl4s.push({
      cta: parseFloat(getComputedStyle(document.querySelector('.cta')).opacity),
      title: parseFloat(getComputedStyle(document.querySelector('.title')).opacity)
    });
  }, 200))

  @wait 10000ms

  @eval (clearInterval(window.__sl4t))
  @eval (window.__sl4still = window.__sl4s.filter(function (s) { return s.title > 0.9 && s.cta === 0; }).length)
  @eval (window.__sl4moved = window.__sl4s.filter(function (s) { return s.cta > 0.2; }).length)

  @assert (window.__sl4still > 0)
    "no sample ever saw `cta` still while `title` was done — if `cta` starts moving the instant the loop starts, its window (6s..9s) is not being honored"
  @assert (window.__sl4moved > 0)
    "no sample ever saw `cta` move — the clip join is broken: each clip is addressed by something other than its own element name"
}

// =============================================================================
// 5. THE RATE LANE (section 2): `rate` is the derivative of the window map, so
//    every dot moves but at its own speed. `.normal` (rate 1) and `.back`
//    (rate -1, reverse) both translate within the first second of the 4s loop.
//    Proves the demo's rate consumers (`@on &.clip: --dot-move`) are real.
// =============================================================================

@test "score-launch: the rate lane moves its dots" needs timing {
  @mount {
    <div class="rates">
      <div class="lane"><span class="dot normal">rate 1 — normal</span></div>
      <div class="lane"><span class="dot fast">rate 2 — finishes early, holds</span></div>
      <div class="lane"><span class="dot slow">rate 0.5 — never finishes</span></div>
      <div class="lane"><span class="dot back">rate -1 — runs backward</span></div>
    </div>
  }

  @wait 1000ms

  @eval (window.__sl5a = parseFloat(getComputedStyle(document.querySelector('.dot.normal')).getPropertyValue('--st-__clip_normal')))
  @eval (window.__sl5b = parseFloat(getComputedStyle(document.querySelector('.dot.back')).getPropertyValue('--st-__clip_back')))

  @assert (window.__sl5a > 0.1)
    "at 1000ms the `normal` dot (rate 1, window 0s..4s) has advanced — a near-zero value means its `@on &.clip` consumer is not reading the rate lane's window"
  @assert (window.__sl5b > 0.1)
    "at 1000ms the `back` dot (rate -1) has advanced — a reverse rate still moves the clip, and 0 here means `rate` is not reaching the consumer"
}
