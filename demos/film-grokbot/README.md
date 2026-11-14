# Grok Bot 1.0, authored in Spacetime

A reconstruction of the [79-second 1.0 launch film](https://x.ai/news/grok-bot-1-0),
1920×1080 at 30 fps. The original copy and sequence are retained so frames can
be compared. UI layout, character anatomy, typography, camera movement, charts,
selection boxes and transitions are authored in Spacetime. Photographic artwork,
the public X VF brand font, and the soundtrack come from the reference;
see [asset provenance](assets/PROVENANCE.org).

```sh
spacetime serve demos/film-grokbot/             # Edit → Film, scrub or change code
spacetime render demos/film-grokbot/ --fps 30 --out scratch/grokbot/film.mp4
spacetime render demos/film-grokbot/ --stills 6,16,26,37,44,56,68,77 --out scratch/grokbot/stills/frame.png
```

The entry composes scenes; it contains no choreography. `score.st` is the edit
decision list on one named `film` transport. Each scene file owns its content
and local choreography. `design/` is the reusable language:

| Layer | Owns | Change here |
|---|---|---|
| Tokens | color and font families | `design/tokens.st` |
| Anatomy | bots, messages, voice panels, pointers, filmstrips | `design/components.st` |
| Surfaces | layout, spacing, silhouette, size | `design/surfaces.st` |
| Motion | arrival, focus, pan, pop, pointer, clip visibility | `design/motion.st` |
| Composition | one scene and its local camera | `scenes/*.st` |
| Time | cut order, starts, durations | `score.st` |

During `serve`, edits to imported design and scene modules invalidate cached
page output before the live reload. The entry file does not need to be touched
to pick up a shared token, anatomy or motion edit.

Use the same forms in another composition:

```st
@import "./design/index.st";

<--scene name="hello" tone="blue">
  <div class="center">
    <--voice>
      <--bubble side="out">Can you join the call?</--bubble>
      <--bubble>Of course.</--bubble>
    </--voice>
  </div>
</--scene>

.hello { @on &.clip { --film-window; .voice { --film-arrive(120px, 16px); } } }
.reel { @score &.time(5s) as &film { &hello at 0s for 5s; } }
```

Scene visibility, camera framing, and component motion have separate elements.
The blur envelope travels with the move and settles to zero. It is an authored
focus treatment, not physically simulated velocity blur. Per-move parameters
let a composition change distance and softness without copying choreography.

Meeting footage uses ordinary image atlases. A score animates `--frame`; CSS
selects a cell with `round()` and `mod()`. The displayed photograph is a pure
function of the playhead, including reverse seeks and still rendering. Column count, row count and cell
aspect ratio are form parameters; a portrait tile crops the image instead of
stretching it.

The executable catalog (`language.st.md`), served at `/language`, also includes a
typed color signal that reskins a voice panel through ordinary `@on` events
and reactive CSS.

The reference soundtrack is an ordinary `@audio` entry on the same `film`
transport. Play, pause, scrub and native export share that score. It remains
a demo asset; the reusable design library has no soundtrack dependency.
Recorded export currently preserves source gain and speed. The original
footage, extraction sheets and render outputs remain in ignored
`scratch/grokbot/`; no standalone application JavaScript is required.

Voice proportions use one typed form parameter, `<--voice scale="1.2">`.
The `--voice-activity` timeline channel grows speaking bars while a diameter
floor preserves round quiet dots. Character front/rear faces turn inside the
outer focus wrapper, so focus blur does not flatten the face's 3D context.

Validation uses the actual Chromium backend. The language motion gates were
first run without their motion declarations: **0 passed, 2 failed, 0 skipped**.
The gates assert element movement, blur settling, atlas addressing, and reverse
seek, rather than emitted strings.


Visual fidelity is a reconstruction, not a certified pixel identity. The cut
and UI beats follow the master, with native-resolution photographic atlases;
some character poses, poster crops and camera curves still differ.
Photographic plates retain some embedded labels and campaign text; those
pixels are not editable typography.
Those plates opt into `captioned="true"` to avoid a second caption layer.
For clean artwork, the poster form defaults to editable `brand` and `line`
parameters rendered as native typography.
The comparison script renders exact
timestamps from both sources so remaining differences stay visible.


Run the browser gates against the served project on port 4549:

```sh
spacetime serve demos/film-grokbot/ -p 4549
spacetime test tests/score/grokbot-film.drive.st --cdp
spacetime test tests/score/grokbot-catalog.drive.st --cdp
spacetime test tests/score/grokbot-language.test.st --cdp
spacetime test tests/score/audio-transport.test.st --cdp
```

Each drive file has one URL: the current CDP runner navigates once per compiled
file. The film gate checks scene windows, photographic movement and aspect,
chart motion and reverse seeking. The catalog gate clicks the real skin controls.
`tests/studio/imported_design_e2e.sh` verifies that changing only a nested
design module updates real browser text and layout on two served routes.
