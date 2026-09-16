# Captions

## Karaoke pill

Redoes the registry's `caption-pill-karaoke`. One word at a time lights up
inside the pill — each word owns a 0.8s window on one loop, and its whole
animation is `opacity: 0.25 -> 1 -> 0.25`.

```st src
<section class="gx g-kara">
  <p class="pill"><span class="w1">THE</span><span class="w2">FUTURE</span><span class="w3">IS</span><span class="w4">COMPILED</span></p>
</section>

.g-kara .pill { display: flex; gap: 0.45em; padding: 0.9rem 1.5rem; border-radius: 999px; background: #121926; color: #e9eff7; font-weight: 800; font-size: 1.5rem; letter-spacing: 0.04em; margin: 0; }
.g-kara .pill span { opacity: 0.25; }

.g-kara {
  @score &.loop(3.2s) {
    &w1 at 0.0s for 0.8s; &w2 at 0.8s for 0.8s;
    &w3 at 1.6s for 0.8s; &w4 at 2.4s for 0.8s;
  }
}
.g-kara .w1 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
.g-kara .w2 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
.g-kara .w3 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
.g-kara .w4 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
```
