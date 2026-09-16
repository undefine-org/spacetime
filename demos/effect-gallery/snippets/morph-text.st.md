## Morph text

Redoes `morph-text`: two words dissolve through each other — blur out,
letter-space, blur back in as the other word. Both windows cover the same 4s with
keyframes in opposite phase; the ends equal the starts, so the loop is
seamless.

```st src
<section class="gx g-morph">
  <p class="stack"><span class="a">chaos</span><span class="b">clarity</span></p>
</section>

.g-morph .stack { position: relative; font-size: 3rem; font-weight: 800; color: #e9eff7; margin: 0; }
.g-morph .b { position: absolute; left: 0; top: 0; color: #5eead4; opacity: 0; }

.g-morph { @score &.loop(4s) { &a at 0s for 4s; &b at 0s for 4s; } }
.g-morph .a { @on &.clip { opacity: 1 -> 0 -> 0 -> 1; blur: 0px -> 10px -> 10px -> 0px; letter-spacing: 0em -> 0.3em -> 0.3em -> 0em; } }
.g-morph .b { @on &.clip { opacity: 0 -> 1 -> 1 -> 0; blur: 10px -> 0px -> 0px -> 10px; letter-spacing: 0.3em -> 0em -> 0em -> 0.3em; } }
```
