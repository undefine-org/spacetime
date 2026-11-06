// tests/render/fixtures/raea_counter.st — a CSS counter driven by a score
// (PLAN-151): the promo's frame counter. The score animates `--fr`; the counter
// is reset from it and shown by `::after { content: counter(fr) }`. Unpatched,
// raea's Blitz drew only the strings of `content` — the counter was blank on
// every frame, so the frames could not differ. The counter sits in an inline
// span (the promo's shape): a nested counter's text must follow its value even
// though no box around it is rebuilt.

@version 2026-06-09;

<div class="film"><span class="tm"><span class="num"></span></span></div>

body { margin: 0; }
.film { position: fixed; inset: 0; background: #fff; display: grid; place-content: center; }
.num { font: 700 120px/1 sans-serif; color: #f00; counter-reset: fr calc(round(var(--fr, 0))); }
.num::after { content: counter(fr); }
.film { @score &.time(1s) { &num at 0s for 1s; } }
.num { @on &.clip { --fr: 0 -> 900; } }
