# Effects

## Shimmer sweep

Redoes `shimmer-sweep`: a light beam crosses the card while it "loads". The
beam is an overlay div with a gradient *longhand*; the custom property
`--sh` carries it across. This is the film's wordmark trick, applied to a
surface instead of glyphs.

```st src
<section class="gx g-shim">
  <div class="card">
    <p class="t">SUMMER COLLECTION</p>
    <p class="s">Loading the lookbook…</p>
    <div class="beam"></div>
  </div>
</section>

.g-shim .card { position: relative; overflow: hidden; padding: 1.6rem 2rem; border-radius: 14px; background: #141b28; border: 1px solid #232d3f; min-width: 340px; }
.g-shim .t { margin: 0 0 0.3rem; font-weight: 800; color: #e9eff7; letter-spacing: 0.06em; }
.g-shim .s { margin: 0; color: #6b7a90; font-size: 0.95rem; }
.g-shim .beam { position: absolute; inset: 0; background-image: linear-gradient(105deg, transparent 35%, rgba(255,255,255,0.16) 50%, transparent 65%); translate: var(--sh, -100%) 0%; }

.g-shim { @score &.loop(2.6s) { &beam at 0s for 2.6s; } }
.g-shim .beam { @on &.clip { --sh: -100% -> 100%; } }
```
