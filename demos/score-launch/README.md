# score-launch — one score, every clock, executable

A literate Spacetime page (`index.st.md`) that is two things at once:

1. **A renderable film.** The first thing on the page is a 12-second title
   sequence — four scenes, one `@score` — because the renderer captures the
   viewport from the top. The same fence that plays it in your browser
   produces the mp4:

   ```
   cargo run -- serve demos/score-launch/                                      # the page
   cargo run --features cdp -- render demos/score-launch/ --out scratch/score-launch.mp4   # the film
   ```

2. **A catalog of what a score can say.** Every effect family a video
   catalog ships as a preset is shown as an executable fence: kinetic type
   (§1), transitions — cover/push, blur-dissolve, iris, glitch-cut (§2),
   color and light — color keyframes, hue orbit, neon pulse, shimmer (§3),
   depth — rotate-y flip, dolly (§4), `rate` as the derivative of the window
   map (§5), and the SIP-001 thesis itself: one `--story` fragment under two
   clocks (§6). No JavaScript anywhere: every specimen is clip placements +
   keyframe bodies, including the ones that drive a CUSTOM PROPERTY
   (`--iris`, `--pos`) and let `clip-path`/gradients join the vocabulary.

The page exists to make SIP-001's central claim checkable rather than
persuasive:

> A website and a video differ only in **which signal drives the timeline**.

## Status: an executable specification that moves and renders

- The film's scenes, the cover transition, the color interpolation and the
  stagger spread are gated in a real Chromium:
  `tests/score/score-launch-film.test.st` (trace-sampling gates — a fixed
  `@wait` cannot know a restarted loop's phase).
- The reel, the rate lane and the two-pole composition are gated in
  `tests/score/score-launch.test.st` and `tests/score/two-pole.test.st`
  (BUG-258, BUG-266, BUG-267 — all closed).
- The page renders: 720 frames, 12s @ 60fps, h264, verified by
  `tests/render/render_test.rs` and by smoke-rendering this page.

Getting the catalog to move surfaced three compiler bugs, all fixed and
gated by the files above: an inline-body `@on &.clip { … }` consumer watched
a synthetic `__drive_clip_N` no score publishes (the form-call path was
correct); a `$sel:selector` capture lost its variant in `text_of`, so nested
motion scopes flattened into the root list and same-named properties
collided; and `@score &.loop` inherited `loop-driver`'s `pingpong` default,
palindroming every clip's window on the second half-cycle instead of
restarting the arrangement. A fourth — `stagger: 120ms` in a scope emitted
`delay: 0` — fell with the same wave.

## What it does NOT prove

Section 7 of the page says this in full: the remaining specimens (§2's
dissolve/iris-cut/glitch, §3's hue/neon/shimmer, §4's flip and dolly) are
compile-checked but not individually gated; `&.steps` and `&.playback`
scores are still `planned:`; and `hold(until:)` is deliberately not score
vocabulary.

Every claim in the page is asserted by `tests/score_window_map_test.rs`,
`tests/clip_driver_test.rs`, `tests/score_domain_test.rs`, and the CDP gates
in `tests/score/`.
