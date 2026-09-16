# Lower thirds

## Clean bar

Redoes `lt-clean-bar`: the bar draws on first, the label follows. Two
windows, two elements, one loop — the bar's draw is `scale` on the X axis
with `transform-origin` pinning it to the left.

```st src
<section class="gx g-cbar">
  <div class="lt"><div class="bar"></div><p class="txt">Quarterly revenue, up 18%</p></div>
</section>

.g-cbar .lt { display: flex; align-items: center; gap: 1.1rem; }
.g-cbar .bar { width: 180px; height: 6px; background: #5eead4; transform-origin: left center; scale: var(--bs, 0) 1; }
.g-cbar .txt { font-size: 1.4rem; font-weight: 700; color: #e9eff7; margin: 0; }

.g-cbar { @score &.loop(3s) { &bar at 0.2s for 0.9s; &txt at 0.8s for 0.8s; } }
.g-cbar .bar { @on &.clip { --bs: 0 -> 1; opacity: 0 -> 1; } }
.g-cbar .txt { @on &.clip { opacity: 0 -> 1; translate: -14px 0px -> 0px 0px; } }
```
