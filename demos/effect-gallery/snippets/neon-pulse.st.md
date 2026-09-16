## Neon pulse

Redoes `caption-neon-glow`: the sign flickers and breathes. `brightness` and
`opacity` are numbers, so the flicker is a five-stop keyframe — irregular
because the stops are irregular, deterministic because they are data.

```st src
<section class="gx g-neon">
  <p class="sign">OPEN 24/7</p>
</section>

.g-neon .sign { font-size: 3rem; font-weight: 900; color: #5eead4; text-shadow: 0 0 18px rgba(94,234,212,0.55); margin: 0; }

.g-neon { @score &.loop(2.6s) { &sign at 0s for 2.6s; } }
.g-neon .sign { @on &.clip { brightness: 1 -> 2.1 -> 1 -> 2.1 -> 1; opacity: 1 -> 0.55 -> 1 -> 0.7 -> 1; } }
```
