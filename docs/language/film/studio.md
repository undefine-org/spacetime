# Film studio

*Working doc for PLAN-155. Being built. Companion to `../film.st.md` and
`scenes-and-sound.md`.*

The film studio is the PLAN-154 studio's Film mode: a timeline, keyframes, sound
lanes, an inspector and the code, all drawn from the film's own source, where
every gesture rewrites one word of that source.

## A score is a transport

A score's time driver is a position, not a stopwatch:

- `pause` holds the frame; `play` goes on from it (a finished score plays again
  from the top).
- `seek(ms)` shows that frame at once, forward or back, paused or playing.
- `loop(in, out)` plays a section over and over; `loop()` plays through.

`@score &.time(24s) as &film { … }` names the timeline `film`. Tools hold it as
`ST._transports.film` (`{play, pause, seek, loop, position, playing,
duration}`); the score element carries `film__playing`. The score's `@sound` is
silent while paused and follows every seek. A model's clip plays on from the
frame a seek lands on (a render never seeks, so a rendered clip is exact).

Gate: `tests/score/transport.test.st`.
