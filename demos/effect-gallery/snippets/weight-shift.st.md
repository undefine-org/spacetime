## Weight shift

Redoes `caption-weight-shift`: emphasis travels down the line as weight and
color together. Same windowing as the karaoke pill — the *only* thing that
changes is which properties the keyframe body interpolates.

```st src
<section class="gx g-wshift">
  <p class="line"><span class="w1">reads</span> <span class="w2">like</span> <span class="w3">a</span> <span class="w4">sentence,</span> <span class="w5">lands</span> <span class="w6">like</span> <span class="w7">a</span> <span class="w8">beat</span></p>
</section>

.g-wshift .line { font-size: 2.2rem; color: #e9eff7; margin: 0; }
.g-wshift .line span { font-weight: 300; color: #4b5568; display: inline-block; }

.g-wshift {
  @score &.loop(4s) {
    &w1 at 0.0s for 0.9s; &w2 at 0.5s for 0.9s; &w3 at 1.0s for 0.9s; &w4 at 1.5s for 0.9s;
    &w5 at 2.0s for 0.9s; &w6 at 2.5s for 0.9s; &w7 at 3.0s for 0.9s; &w8 at 3.5s for 0.9s;
  }
}
.g-wshift .w1 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w2 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w3 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w4 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w5 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w6 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w7 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
.g-wshift .w8 { @on &.clip { font-weight: 300 -> 900 -> 300; color: #4b5568 -> #e9eff7 -> #4b5568; } }
```
