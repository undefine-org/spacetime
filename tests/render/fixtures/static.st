// tests/render/fixtures/static.st — the DISCRIMINATION control (PLAN-132).
//
// Structurally identical to film.st EXCEPT the clips' windows are never
// consumed: no `@on &.clip` line, so the score publishes a progress signal
// that nothing maps onto style (BUG-258's exact inert state). Rendering this
// MUST produce a frozen video, and the MOTION gate MUST fail on it — the
// discrimination proof that the motion gate is real, not satisfied vacuously
// by a page where nothing works at all.

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

.head { opacity: 0; }
.tail { opacity: 0; }
