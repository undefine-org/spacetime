// tests/render/fixtures/import_only/index.st — entry for the imported-score
// render gate (PLAN-132 W5 FUP). The entry file declares NO score of its own:
// its only timeline lives in the imported `./score_long.st` module (a 4s
// playhead). `spacetime render` must see that imported score and time the
// video to it. Before the merged-AST fix, entry-only totals found nothing and
// this page refused to render — the (b) gate's discrimination proof.

@version 2026-06-09;

@import "./score_long.st";
