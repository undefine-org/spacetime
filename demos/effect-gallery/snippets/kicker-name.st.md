## Kicker + name

Redoes `lt-kicker-name`: the kicker rises into its mask, the name follows a
beat later. The mask is just `overflow: hidden` on the wrapper; the rise is
`translate` measured in percent of the element itself.

```st src
<section class="gx g-kick">
  <div class="lt">
    <p class="mask"><span class="kck">FIELD REPORT</span></p>
    <p class="mask"><span class="nme">Amara Okafor — Lagos Bureau</span></p>
  </div>
</section>

.g-kick .lt { display: flex; flex-direction: column; gap: 0.35rem; }
.g-kick .mask { overflow: hidden; margin: 0; }
.g-kick .mask span { display: inline-block; }
.g-kick .kck { font-size: 0.85rem; letter-spacing: 0.22em; font-weight: 700; color: #5eead4; }
.g-kick .nme { font-size: 1.6rem; font-weight: 800; color: #e9eff7; }

.g-kick { @score &.loop(3.4s) { &kck at 0.2s for 0.9s; &nme at 0.6s for 1.0s; } }
.g-kick .kck { @on &.clip { translate: 0px 110% -> 0px 0%; opacity: 0 -> 1; } }
.g-kick .nme { @on &.clip { translate: 0px 110% -> 0px 0%; opacity: 0 -> 1; } }
```
