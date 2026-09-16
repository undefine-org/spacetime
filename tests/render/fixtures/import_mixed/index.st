// tests/render/fixtures/import_mixed/index.st — the NEGATIVE gate for the
// imported-score render fix (PLAN-132 W5 FUP). The page has an entry-LOCAL 1s
// score AND imports a 4s score from `./score_long.st`. The render duration is
// the LONGEST total across the whole page = 4s. Entry-only totals would see
// only the local 1s and SILENTLY truncate the video to 1s — this gate proves
// the fix does not.

@version 2026-06-09;

@import "./score_long.st";

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --local {
  &x at 0s for 1s;
}

<div class="localfilm">
  <div class="x">x</div>
</div>

.localfilm {
  @score &.time(1s) {
    --local;
  }
}

.x { opacity: 0; @on &.clip: --clip-rise; }
