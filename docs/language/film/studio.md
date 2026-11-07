# Film studio

*Working doc for PLAN-155. Ships today (dev server). Companion to `../film.st.md` and
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

## Using it

`spacetime serve demos/film-robot-day/`, open the page, press the dock's
**Edit** chip. On a page with a `@score` the studio adds Film mode:

| | does |
|---|---|
| Space / K / L / J | play-pause / pause / play / back a second |
| ← → (Shift: a second) | a frame |
| Home / End | start / end |
| I / O, Esc | loop in / out, clear |
| ruler | scrub |
| clip bar / its right edge | drag: its `at` / its `for` (snaps to 0.1 s; Shift: free) |
| diamond | drag: the stop's `at p%` |
| drum cell | click: `.` ↔ `x` |
| Inspector → Film | the selection's words, editable |
| Explorer → Scene | the stage's parts and every param |
| Code | the file, saved whole when you leave it |

Every change is a source edit; the page reloads and the film comes back at the
same frame, paused or playing, looped, with the same selection (about 5 s on a
release build for Robot Day). Gate: `tests/studio/film_studio_e2e.sh`.
