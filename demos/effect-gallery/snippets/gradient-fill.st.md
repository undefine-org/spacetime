## Gradient fill

Redoes `caption-gradient-fill`: light pours through the glyphs. The gradient
is clipped to the text; a custom property slides `background-position` across
the oversized gradient. Longhand `background-image` again — the shorthand
would reset the clip (BUG-279).

```st src
<section class="gx g-gfill">
  <p class="word">GRADIENT</p>
</section>

.g-gfill .word {
  font-size: 3.4rem; font-weight: 900; letter-spacing: 0.02em; margin: 0;
  background-image: linear-gradient(100deg, #3a4356 40%, #5eead4 50%, #3a4356 60%);
  background-size: 300% 100%;
  background-position: var(--gp, 100%) 0;
  -webkit-background-clip: text; background-clip: text;
  color: transparent;
}

.g-gfill { @score &.loop(3s) { &word at 0s for 3s; } }
.g-gfill .word { @on &.clip { --gp: 100% -> 0%; } }
```
