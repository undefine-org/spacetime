// tests/render/fixtures/import_only/score_long.st — the IMPORTED-module half of
// the imported-score render gate (PLAN-132 W5 FUP, reviewer 68.3).
//
// The renderer's duration is the LONGEST declared @score total. This module
// carries its own markup AND its own `@score` block: its 4s playhead is the
// only timeline on the page. Before the merged-AST fix, render re-parsed just
// the entry file, so a score living in an imported module was invisible — loud
// "no score" when it is the only playhead. This module is imported by
// index.st (import_only) and by import_mixed/index.st.
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
