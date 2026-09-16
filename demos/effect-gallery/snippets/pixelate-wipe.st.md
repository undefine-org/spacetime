## Pixelate wipe

Redoes `grid-pixelate-wipe`: a 5×5 field of tiles materializes in a sweep.
The container owns the window; the nested `.cell` scope declares
`stagger: 300ms`, and every cell's *local* start is spread by its grid index.
The value is a fraction of the window (300ms ≡ 0.3, so the sweep spans ~1.1s
of the 3.6s loop — stagger is window-relative, not absolute). One keyframe
body animates all 25 tiles.

```st src
<section class="gx g-pix">
  <div class="grid">
    <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
    <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
    <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
    <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
    <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
  </div>
</section>

.g-pix .grid { display: grid; grid-template-columns: repeat(5, 44px); gap: 6px; }
.g-pix .cell { width: 44px; height: 44px; border-radius: 6px; background: #5eead4; opacity: 0; }

.g-pix .grid {
  @score &.loop(3.6s) { &grid at 0s for 3.6s; }
  @on &.clip {
    .cell { stagger: 300ms; opacity: 0 -> 1; scale: 0.55 -> 1; }
  }
}
```
