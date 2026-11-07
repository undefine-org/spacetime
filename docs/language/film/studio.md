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

## The film's shape: `score.json`

`GET /__spacetime/studio/score.json?entry=index.st` (dev server) is what the
studio draws. Every word in it carries `{file, start, end, text}`:

- `scores[]`: name, host, total, `entries[]` — a clip's `subject`, `at`, `for`
  (and `at_ms`, `for_ms`), a sound's `form`, `at`, `for`, `notes`;
- `clips{subject}`: the clip's `@on &.clip` body as `channels[]` — `scope`
  (`&robot`, or none for the element itself), `prop`, `stops[]` (`value`, `at`,
  `easing`);
- `forms{--name}`: instruments (props with stops) and patterns (rows with cells);
- `parts[]`: the stage and its parts with every param, written (with its span)
  or at its default;
- `programs[]`: the synth program of each score.

It is read from the parser's own matches: the metasystem keeps the span of
every capture, nested, under its grammar's path, so nothing re-reads the
language.

## Edits are words

`POST /__spacetime/studio/apply` takes PLAN-154's ops and three more:

| op | does | guarded by |
|---|---|---|
| `span {file, start, end, expect, value}` | replaces one word `score.json` handed out | the bytes still read `expect` |
| `insert {file, at, before, text}` | adds a param or a stop | the bytes just ahead read `before` |
| `source {file, base, text}` | saves a whole file (the code pane) | `base` is the hash of the file it was edited from |

A gesture is one of these: drag a clip and its `at 0.4s` becomes `at 1s`; click
a drum cell and its `.` becomes `x`. An edit on a stale address is refused,
never guessed.
