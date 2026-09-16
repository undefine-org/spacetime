## Glitch RGB

Redoes `caption-glitch-rgb`: two tinted ghost copies jitter against the base
glyphs. The jitter is a five-stop keyframe on `translate` + `opacity` — no
random() anywhere, so the glitch is deterministic and testable.

```st src
<section class="gx g-glitch">
  <p class="stack">
    <span class="base">SIGNAL</span>
    <span class="ghost r">SIGNAL</span>
    <span class="ghost c">SIGNAL</span>
  </p>
</section>

.g-glitch .stack { position: relative; font-size: 3.4rem; font-weight: 900; color: #e9eff7; margin: 0; }
.g-glitch .ghost { position: absolute; inset: 0; opacity: 0.75; }
.g-glitch .r { color: #ff3b5c; }
.g-glitch .c { color: #35d6ff; }

.g-glitch {
  @score &.loop(2.4s) { &r at 0s for 2.4s; &c at 0s for 2.4s; }
}
.g-glitch .r { @on &.clip { translate: 0px 0px -> -6px 2px -> 4px -3px -> -3px 1px -> 0px 0px; opacity: 0.75 -> 0.9 -> 0.4 -> 0.85 -> 0.75; } }
.g-glitch .c { @on &.clip { translate: 0px 0px -> 5px -2px -> -5px 3px -> 4px -1px -> 0px 0px; opacity: 0.75 -> 0.4 -> 0.9 -> 0.5 -> 0.75; } }
```
