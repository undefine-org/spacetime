## Layered vignette

Redoes `title-scrim-sweep`: a title plate needs a **scrim** to stay readable
over artwork, and a sheen sweeping behind the text to keep it alive. That is two
background layers on one element — the scrim in front, the moving sheen behind
it:

    background: <scrim gradient>, <sheen gradient>;

The scrim is a fixed dark wash (literal `rgba`, so it stays a constant veil); the
sheen is built from `$brand`, which makes the whole declaration reactive. This is
the case [FUP-184] adds: a comma-separated layer LIST is one value naming two
layers, and the routing grammar has to see it as an image before it can send it
to `background-image` instead of the destructive `background` shorthand.

The per-layer longhands are what make it work — `background-size` gives the
scrim a fixed `100% 100%` and the sheen an oversized `260%` to travel across,
one entry per layer, in the same order. Before [FUP-184] the reactive shorthand
reset both entries to `auto` the moment the signal rendered, so the sheen had no
over-width gradient left to move through and the sweep died silently.

Note the commas *inside* each `linear-gradient(…)`: they belong to the gradient,
not to the layer list. Nothing here has to escape them — `image_core` consumes
balanced parens, so an inner comma is eaten long before the list separator is
considered.

```st src
<section class="gx g-layer">
  <p class="plate">SPACETIME</p>
</section>

@data inline $brand : { "ink": "#0d1117", "accent": "#5eead4" };

.g-layer .plate {
  margin: 0; padding: 1.4rem 2.2rem;
  font-size: 3.2rem; font-weight: 900; letter-spacing: 0.04em;
  color: #e9eff7; border-radius: 14px;

  background: linear-gradient(180deg, rgba(13,17,23,0.86) 0%, rgba(13,17,23,0.55) 100%), linear-gradient(100deg, $brand.ink 40%, $brand.accent 50%, $brand.ink 60%);
  background-size: 100% 100%, 260% 100%;
  background-position: 0 0, var(--sweep, 100%) 0;
  background-repeat: no-repeat, no-repeat;
}

.g-layer { @score &.loop(3.2s) { &plate at 0s for 3.2s; } }
.g-layer .plate { @on &.clip { --sweep: 100% -> 0%; } }
```
