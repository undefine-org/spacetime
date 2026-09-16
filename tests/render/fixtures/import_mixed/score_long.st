// tests/render/fixtures/import_mixed/score_long.st — imported 4s score module
// (PLAN-132 W5 FUP). Identical to import_only/score_long.st; imported by the
// entry index.st below. The mixed gate's point: an entry-LOCAL 1s score is
// SHORTER than this imported 4s score, so the render must run to 4s — never
// truncate the video to the entry's own (shorter) longest score, which was the
// SILENT failure mode when totals were computed entry-only.
//
// Imported module — no @version here: E0911 allows exactly one per project,
// and it must live in the root entry file.

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --long {
  &a at 0s for 1s;
  &b at 1s for 3s;
}

<div class="impfilm">
  <div class="a">a</div>
  <div class="b">b</div>
</div>

.impfilm {
  @score &.time(4s) {
    --long;
  }
}

.a { opacity: 0; @on &.clip: --clip-rise; }
.b { opacity: 0; @on &.clip: --clip-rise; }
