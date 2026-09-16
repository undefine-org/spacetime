## Mask reveal

Redoes `lt-mask-reveal`: a cover sweeps in, and the text appears *while fully
covered*, so the cover's exit is the reveal. Both elements read the same
window; the text's keyframes simply stay at 0 until the cover is closed.

```st src
<section class="gx g-mrev">
  <p class="wrap"><span class="txt">Revealed by the cover</span><span class="cover"></span></p>
</section>

.g-mrev .wrap { position: relative; display: inline-block; overflow: hidden; font-size: 2rem; font-weight: 800; color: #e9eff7; margin: 0; }
.g-mrev .cover { position: absolute; inset: 0; background: #5eead4; }
.g-mrev .txt { opacity: 0; }

.g-mrev { @score &.loop(3s) { &cover at 0.3s for 1.6s; &txt at 0.3s for 1.6s; } }
.g-mrev .cover { @on &.clip { translate: -101% 0% -> 0% 0% -> 0% 0% -> 101% 0%; } }
.g-mrev .txt { @on &.clip { opacity: 0 -> 0 -> 1 -> 1; } }
```
