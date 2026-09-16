## Kinetic slam

Redoes `caption-kinetic-slam`: each word drops in from oversize with a small
settle (`2.6 -> 0.94 -> 1` — the overshoot is a middle keyframe stop, not a
spring library). Windows overlap by half so the line reads as one gesture.

```st src
<section class="gx g-slam">
  <p class="line"><span class="s1">STOP</span> <span class="s2">MAKING</span> <span class="s3">SENSE</span></p>
</section>

.g-slam .line { font-size: 2.8rem; font-weight: 900; color: #e9eff7; margin: 0; }
.g-slam .line span { display: inline-block; opacity: 0; }

.g-slam {
  @score &.loop(2.8s) {
    &s1 at 0.0s for 0.7s; &s2 at 0.35s for 0.7s; &s3 at 0.7s for 0.7s;
  }
}
.g-slam .s1 { @on &.clip { opacity: 0 -> 1 -> 1; scale: 2.6 -> 0.94 -> 1; } }
.g-slam .s2 { @on &.clip { opacity: 0 -> 1 -> 1; scale: 2.6 -> 0.94 -> 1; } }
.g-slam .s3 { @on &.clip { opacity: 0 -> 1 -> 1; scale: 2.6 -> 0.94 -> 1; } }
```
