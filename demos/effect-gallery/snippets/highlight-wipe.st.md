## Highlight wipe

Redoes `caption-highlight`: a marker sweeps behind the phrase. The wipe is a
custom property (`--hw`) feeding `background-size` — the same var()-driven
mechanism as the iris wipe in the score-launch film, here applied to a
longhand so the reactive set clobbers nothing (BUG-279).

```st src
<section class="gx g-hiw">
  <p class="line">Ship <span class="hl">without fear</span>, every Friday.</p>
</section>

.g-hiw .line { font-size: 2rem; font-weight: 700; color: #e9eff7; margin: 0; }
.g-hiw .hl {
  background-image: linear-gradient(#f5c542, #f5c542);
  background-repeat: no-repeat;
  background-size: var(--hw, 0%) 100%;
  color: #10151d; padding: 0 0.15em;
}

.g-hiw { @score &.loop(3s) { &hl at 0.6s for 1.4s; } }
.g-hiw .hl { @on &.clip { --hw: 0% -> 100%; } }
```
