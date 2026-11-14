@import "stdlib/film";

<div class="film"><div class="dot">Sound follows the score.</div></div>
@form instrument --blip { wave: sine; pitch: 220hz; gain: 0.5 -> 0; for: 200ms; }
.film {
  @score &.time(3s) as &filefilm {
    &dot at 0s for 3s;
    @sound(--blip) at 0.1s;
    @audio(src: "audio/two tones.flac") as &recording at 1s;
  }
}
body { margin: 0; }
.film { position: fixed; inset: 0; background: white; }
.dot { @on &.clip { translate-x: 0px -> 100px; } }
