// tests/render/fixtures/raea_mask_var.st — a mask that reads an animated custom
// property (PLAN-151). The score grows `--reveal`, the mask's radial stop reads
// it through var(): the red disc must grow. Unpatched, raea's Stylo gave
// `mask-*` no repaint damage, so the mask froze at its first value.

@version 2026-06-09;

<div class="film"><div class="disc"></div></div>

body { margin: 0; }
.film { position: fixed; inset: 0; background: #fff; }
.disc {
  position: absolute; inset: 0; background: #f00;
  mask-image: radial-gradient(circle at 50% 50%, #000 calc(var(--reveal, 0) * 0.6%), transparent calc(var(--reveal, 0) * 0.6% + 1%));
}
.film { @score &.time(1s) { &disc at 0s for 1s; } }
.disc { @on &.clip { --reveal: 0 -> 100; } }
