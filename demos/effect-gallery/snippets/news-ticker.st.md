## News ticker

Redoes `news-ticker`: the strip slides left by exactly half its width —
the copy is duplicated, so the loop's end frame is identical to its start
frame and the wrap is invisible. `easing: --linear` keeps the speed constant;
an eased ticker would visibly breathe at every wrap.

```st src
<section class="gx g-tick">
  <div class="rail"><p class="strip">SPACETIME COMPILES TO A FILM ✦ SCORES ARE DATA ✦ NO JAVASCRIPT ✦ SPACETIME COMPILES TO A FILM ✦ SCORES ARE DATA ✦ NO JAVASCRIPT ✦ </p></div>
</section>

.g-tick .rail { overflow: hidden; width: 520px; max-width: 100%; border-block: 1px solid #2a3446; padding: 0.6rem 0; }
.g-tick .strip { margin: 0; white-space: nowrap; font-weight: 700; letter-spacing: 0.08em; color: #e9eff7; display: inline-block; }

.g-tick { @score &.loop(8s) { &strip at 0s for 8s; } }
.g-tick .strip { @on &.clip { translate: 0% 0% -> -50% 0%; easing: --linear; } }
```
