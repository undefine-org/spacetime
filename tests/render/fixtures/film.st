// tests/render/fixtures/film.st — the render-threshold page (PLAN-132).
//
// ONE score fragment, TWO clocks. The body below is the SAME source the
// browser-test half of the thesis gate compiles: it defines `--story` once,
// splices it into a `.time`-driven score, and consumes each clip's window
// with `@on &.clip` so the arrangement actually MOVES (BUG-258's consumer
// surface). A website plays it with the real wall clock; `spacetime render`
// plays it with Chromium's virtual clock. No page change separates the two.
//
// The score's declared total is 2s — the renderer's timeline length, which
// the frame-count gate asserts (2s @ 10fps -> exactly 20 frames).

@version 2026-06-09;

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --story {
  &head at 0s for 1s;
  &tail at 1s for 1s;
}

<div class="film">
  <div class="head">head</div>
  <div class="tail">tail</div>
</div>

.film {
  @score &.time(2s) {
    --story;
  }
}

.head { opacity: 0; @on &.clip: --clip-rise; }
.tail { opacity: 0; @on &.clip: --clip-rise; }
