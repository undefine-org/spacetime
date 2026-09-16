# Two cuts of one promo — studio pipeline vs Spacetime

One 30-second storyboard, one sound-design track, two productions:

| | **Studio cut** | **Spacetime cut** |
|---|---|---|
| Source | `demos/promo/studio-cut/` (promo.js + capture tooling; the control group, deliberately not a Spacetime page) | `demos/promo/index.st` |
| Stack | Three.js r184 (WebGL2, `UnrealBloomPass`, custom GLSL), Canvas2D text rasterization, hand-written CDP client, ffmpeg | Spacetime only. `@score &.time(30s)` + 17 `@on &.clip` bodies; `spacetime render`; ffmpeg audio mux |
| Author code | 562 lines JS (+163 lines capture tooling) + 636 KB vendored engine | 513 lines `.st`, **zero** author JavaScript (the served page has one `<script>`: the compiler's runtime) |
| Frame capture | `seek(t)` per frame → `Page.captureScreenshot` → `image2pipe` | `ST._clock.advance(1000/60)` per frame → same CDP screenshot → same ffmpeg pipe |
| Output | 1800 frames · 60 fps · 1920×1080 · 30.000 s · 54 MB (crf 16) | 1800 frames · 60 fps · 1920×1080 · 30.000 s · 5.1 MB (default preset) |
| Wall time | 20.5 min (GPU-accelerated ANGLE/GL, bloom at 1080p) | 14.0 min (`--disable-gpu`, software raster, 40 ms settle per frame) |
| Determinism | By construction (everything is `f(t)`), but the pipeline is bespoke | By construction AND gated: `tests/render/render_test.rs` asserts bit-identical per-frame hashes across runs for any page |
| Behavior gate | none (a script you run) | `tests/score/promo-film.test.st` (3 tests, `--cdp`, seen RED against the pre-fix compiler) |

Outputs: `scratch/promo-studio/out/spacetime-promo-studio.mp4`, `scratch/promo-st/spacetime-promo-st.mp4`, contact sheet `scratch/compare/side-by-side.jpg`.

Storyboard (shared): S1 point of light → "CSS describes space." · S2 perspective grid, time axis sweeps → "Spacetime adds time." · S3 `.st` types itself, described element animates live → "Declare it once." · S4 four drivers, one composition, playheads converge → "Every clock." · S5 filmstrip flies past, frame counter, `spacetime render` → "Render it." · S6 `<script>` shatters → "No JavaScript." · S7 wordmark shimmer, tagline.

## Frame-by-frame verdict

| t | Studio | Spacetime | Winner | Why |
|---|---|---|---|---|
| 0–3 S1 | Point light w/ additive sprite, 2 200 GPU dust motes drifting in 3D, bloom flare on the hit, camera dolly | Radial-gradient orb, 60 stars + 40 dust `<i>` positioned with `mod(sibling-index()·61, 100)%`, `translate` driven by one `--t` | Studio, narrowly | Bloom + real parallax read as "lens". Spacetime's orb is a flat gradient; there is no bloom pass, so the flash is a plain screen-white overlay. Type is identical (same Syne, same focus pull via `blur`+`letter-spacing` keyframes). |
| 3–7.4 S2 | GLSL grid with fog + reveal ring, vertex-displaced ripple on the hit, camera crane from overhead to eye-level | CSS `perspective` + `rotate-x` grid with a `mask-image` reveal driven by `--reveal`; axis is a glowing div sweeping in screen space; ripple is a scaled ellipse | Studio | The crane shot and the 3D ripple are the two things CSS cannot do: no vertex displacement, and `perspective-origin` keyframing is a poor substitute for a camera path. The Spacetime grid still reads correctly as a floor. |
| 7.4–12.8 S3 | Canvas2D token-colored code typed per character, card reveal keyed to the exact typed character | Real DOM `<pre>` with `<b>` token spans; typing is `clip-path: inset(0 calc(100% − (var(--chars) − var(--o)) · 1ch) 0 0)` per line; card reveal keyed by `range:` | **Spacetime** | Real text (subpixel-correct, selectable in the browser), real DOM card — and the animating card IS the `.st` on screen, not a drawing of it. Same 4-panel scale. The caret blink is `sign(sin(var(--chars)·1.3))`. |
| 12.4–18.6 S4 | 4 canvas panels, per-driver playheads, waveform bars, panels converge and scale up, title | Same, in DOM; convergence is four `translate-x` keyframes; waveform bars are `sibling-index()`-sized `<i>`s | Tie | Pixel-equivalent. Spacetime source is shorter and readable; the studio version is a `draw()` callback per panel. |
| 18–23 S5 | 3D filmstrip (24 frames drawn to a 11 520 px canvas texture), second strip in depth, frame counter, riser | 16 `.fr` DOM frames on a `rotate-y` strip in `perspective`; **frame counter is a CSS counter** `counter-reset: fr calc(round(var(--fr)))` | Studio, narrowly | Two layered strips with depth fog vs one strip. The CSS-counter frame number is the cleverest single line in either film, and it renders a genuinely ticking 0000→1800 with no JS. |
| 22.8–27 S6 | `<script>` rasterized to ~4 000 particles from the glyph bitmap, GLSL trajectories (explosion + gravity), additive, chromatic-aberration + slice glitch post | 140 `<i>` particles; trajectory is CSS: `translate: calc(cos(sibling-index()·137.5deg) · … · (1 − exp(−2.2·var(--tau))) …)`; RGB split is two extra `<span>`s in `mix-blend-mode: screen` | Studio | Glyph-shaped burst vs generic burst is the biggest single visual gap. CSS has no way to sample a rasterized glyph into positions. The glitch stutter is comparable. |
| 27–30 S7 | Gradient-clipped text shimmer, bloom halo, rule draws on | Same shimmer via `background-clip: text` + `--pos`; rule draws on via `scale-x`; no halo | Tie | Identical technique. |

**Overall:** the studio cut wins on light — bloom, additive particles, GLSL displacement, a real camera. The Spacetime cut wins on **text, layout and honesty**: every glyph is real DOM type, the code on screen is the code that moves the card, and the whole film is ~500 lines of declarative source that a non-programmer can edit (`at 18s for 5s` is the timeline UI). Given that the Spacetime cut has no bloom, no GPU and no JS, it is startlingly close.

## What Spacetime could not express (filed)

> The full, affirmative design for closing every gap below — with proposed syntax — is **[docs/language/film.st.md](../../docs/language/film.st.md)**; the build order is `!tasks/plans/PLAN-150`. The FUPs below are subsumed by that plan's waves.

> Correction to the first draft of this critique: `stdlib/3d` already ships `@stage`/`@object`/`@particles`/`@post(bloom, rgbShift)`. The real 3D gap is not "no WebGL" — it is that 3D is a separate island driven by `@scroll-3d(drive:)` instead of `@score`/`&.clip` (film.st.md §5).

Each of these is a language/toolchain gap, not a workaround I took:

1. **No post-processing stack** (bloom, chromatic aberration, vignette-as-shader). CSS `filter` has `blur`/`brightness` per element, but no *scene* bloom. The studio cut's "lens" look is entirely this. → **FUP-196** (`@post` for 2D pages, SIP-001b family E).
2. **No camera.** `perspective-origin` + `rotate-x` is a static rig. A `@camera` transport on a `preserve-3d` stage (position/target keyframes) would have carried the S2 crane shot. → **FUP-197**.
3. **Glyph-sampled particles.** `<script>` shattering *into its own shape* needs a rasterized-text sampler. `stdlib/text` (pretext) measures but does not emit positions. → **FUP-198** (`@scatter(from: &el, count: N)` stamping `--x/--y` per child).
4. **Author-controlled keyframe stops.** `a -> b -> c` is evenly spaced. A flash at t=2.2 s inside a 30 s clip needs either a 60-stop chain (I wrote one — it was rejected in review of my own draft and replaced by six 0.5 s hit clips) or `range:` gymnastics. `opacity: 0 -> 0.85 @ 0.25 -> 0` or a `hold` stop would remove an entire category of arithmetic from film authoring. → **FUP-203**.
5. **No audio in `render`.** SIP-001b §8.6 is still open; the film's hits are locked to the wav by hand-placed clip times (`&h1 at 2.08s for 0.5s`). A `@audio` entity with `mark`-derived hit times would make the score the single source of truth. → **FUP-204**.
6. **`render` runs with `--disable-gpu`** (`src/cdp.rs`). Fine for CSS; would gate any future WebGL/`stdlib/3d` film. Note for the render verb, not filed as a bug.

## Compiler bug surfaced (fixed in this commit)

**BUG-382** — the FOUC initial-state CSS flattened every nested-scope keyframe's first stop onto the parent. `.s1 { @on &.clip { .orb { scale: 0.15 -> 1 } } }` emitted `transform: scale(0.15)` on `.s1`; the whole scene rendered at 15 % scale for the entire film because the runtime never rewrites a root transform it does not own. Latent in `demos/score-launch` (its `.sc1` carried `.fh`'s blur), invisible because those values were small. Fix: `keyframes_to_raw_body` in `src/pipeline/expand.rs` regroups by selector. Gates: `test_initial_state_css_keyframes_capture_keeps_nested_scope` (unit) and `promo-film.test.st` test 1 (CDP) — both seen RED before the fix.

## Things Spacetime did better than expected

- **`sibling-index()` + CSS trig is a particle system.** 140 particles, one animated custom property, zero per-element state. Chrome 150 evaluates `cos()/exp()/mod()` in `calc()` inside `translate` correctly under the virtual clock (gated by promo-film test 3).
- **A CSS counter is a frame counter.** `counter-reset: fr calc(round(var(--fr)))` + `content: counter(fr, decimal-leading-zero)` ticks 0000→1800.
- **Typing is `clip-path`.** Per-line `inset()` against a shared `--chars` gives a typewriter with syntax colors and no DOM churn.
- **The score IS the edit.** Every timing change in this film was a number in `at … for …` or `range: a to b`. In the studio cut it was a constant buried in a 560-line `seek()`.
- **Determinism is free.** No `f(t)` discipline needed; the runtime's virtual clock makes any page a film.

## Things that hurt

- **`@wait` timing in CDP tests drifts across mounts in one file** — a fixed wait cannot know the transport's phase after the second `@mount`. Sampled traces (as `score-launch-film.test.st` already does) are the pattern; I lost ~20 min to this.
- **`i[1]` inside `@eval` is parsed as an attribute selector** and fails the whole file with "expected '{' after selector". `.item(1)` works. → **FUP-206**.
- **A `$` in a class name** (`.pr$`) is a parse error with a confusing message; my fault, but the error should say "`$` is a sigil".
- **Preview loop is slow**: 60 frames @ 2 fps took 66 s (40 ms settle + software raster + full compile). A `--from/--to` or `--stills 1.5,9,24` on `render` would cut iteration 10×; I wrote exactly that for the studio capturer in 5 lines. → **FUP-205**.
- **Render is ~6× slower than the studio path per frame** for equivalent content once GPU is off; most of it is the fixed 40 ms settle (72 s of 14 min) and PNG encode of 1080p frames.
