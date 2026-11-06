// tests/render/fixtures/raea_sound.st — notes on the score (PLAN-152): a
// kick pattern placed at 0.5s. The rendered film carries its soundtrack:
// silent before the pattern's `at`, loud in its window.

@version 2026-06-09;
@import "stdlib/film"

<div class="film"><p class="w">beat</p></div>

body { margin: 0; }
.film { position: fixed; inset: 0; background: #fff; }
@form instrument --kick { wave: sine; pitch: 160hz -> 45hz at 25%; gain: 1 -> 0; for: 300ms; }
@form pattern --beat($tempo = 120bpm, $step = 16) { --kick: x . . . ; }
.film {
  @score &.time(1s) {
    &w at 0s for 1s;
    @sound(--beat) at 0.5s for 0.5s;
  }
}
.w { @on &.clip { opacity: 0 -> 1; } }
