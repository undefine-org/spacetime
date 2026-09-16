## Side rule

Redoes `lt-side-rule`: a rule draws top-to-bottom, then the copy slides in
beside it. The draw is `scale` on the Y axis — the twin of the clean bar's
X-axis draw, which is the whole point of a pattern language.

```st src
<section class="gx g-srule">
  <div class="lt"><div class="rule"></div><p class="txt">Marked as read — 42 notifications cleared</p></div>
</section>

.g-srule .lt { display: flex; gap: 1.1rem; align-items: stretch; }
.g-srule .rule { width: 4px; background: #8b5cf6; transform-origin: center top; scale: 1 var(--rs, 0); }
.g-srule .txt { margin: 0; font-size: 1.3rem; color: #e9eff7; align-self: center; }

.g-srule { @score &.loop(3s) { &rule at 0.2s for 0.8s; &txt at 0.7s for 0.8s; } }
.g-srule .rule { @on &.clip { --rs: 0 -> 1; } }
.g-srule .txt { @on &.clip { opacity: 0 -> 1; translate: -10px 0px -> 0px 0px; } }
```
