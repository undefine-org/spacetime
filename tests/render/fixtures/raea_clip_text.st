// tests/render/fixtures/raea_clip_text.st — `background-clip: text` (PLAN-151).
// A red background clipped to its glyphs, the text itself transparent: only the
// letters are red. Unpatched, raea's Stylo dropped `text` (gecko-only) and the
// background filled the whole box — a red slab.

@version 2026-06-09;

<div class="film"><h1 class="word">IIII</h1></div>

body { margin: 0; }
.film { position: fixed; inset: 0; background: #fff; display: grid; place-content: center; }
.word {
  margin: 0; padding: 0 40px; font: 900 200px/1 sans-serif;
  background: #f00; background-clip: text; -webkit-background-clip: text; color: transparent;
  opacity: 0;
}
.film { @score &.time(1s) { &word at 0s for 1s; } }
.word { @on &.clip { opacity: 0 -> 1; } }
