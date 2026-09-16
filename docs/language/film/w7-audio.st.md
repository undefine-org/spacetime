# W7 — audio is on the score

*Working doc for PLAN-150 W7. Builds and runs today (browser-play half). The
"audio is on the score" of `../film.st.md` §8.*

```st hidden
@import "stdlib/md"
```

## The word

Sound is a score entity: a source, a name, and a start position. The score is
the ONE transport — `&bed` is an `<audio>` element the score seeks and plays; it
never runs its own clock.

```st src
.stage {
  @score &.time(30s) as &film {
    @audio(src: "assets/promo.wav") as &bed at 0s;
    &open at 0s for 3.6s;
  }
}
```

`@audio(src:) as &name at <t>;` is a score entry, alongside a clip line, a
`mark`, and a `gap`. Written outside a `@score` body it is a hard error — sound
has no meaning without a transport.

## Keyframing the sound

`gain` and `rate` are keyframed on the named entity in an ordinary `@on &.clip`
block — the audio is registered under its `&name` (the W4 stage-object rail
reused for a media entity), so `@on` addresses it exactly like any element:

```st src
.stage2 {
  @score &.time(30s) as &film2 {
    @audio(src: "assets/bed.wav") as &bed2 at 0s;
    &intro at 0s for 30s;
  }
}
.bed2 { @on &.clip { gain: 0 -> 1; } }
```

## How it lowers

`src/pipeline/score.rs::audio_entries` detects each `@audio` entry and emits a
`score-audio` bind (`stdlib/primitives/score-audio.st`). The primitive creates a
hidden `<audio>` element and watches the score's own progress signal: as the
score advances, the audio SEEKS to `progress · total − at` and plays while its
window is live. Under the browser's `.time` transport it plays in real time and
seeks on scrub; the score, not the audio, owns the clock.

## Not yet (later in W7's arc)

- The `render` MUX: under `spacetime render`, muxing the track at its `at` offset
  with its gain curve applied by ffmpeg (today the audio is browser-play only;
  the render still needs the mux step).
- `mark &hit at 2.2s;` as an audio-relative position, and `pan`.
- A conformance test asserting the two backends agree on every mark to a frame.
