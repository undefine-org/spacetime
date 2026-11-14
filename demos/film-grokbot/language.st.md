# A living language for a launch film

The [film](./) is assembled from the same forms shown here. Its
vocabulary is code you can edit in Studio; this document executes those forms.

```st hidden
@import "./design/index.st";
```

## One anatomy, five personalities

Silhouette, color, eyes and motion are independent. The source keeps the form's
name, so changing `--bot` changes its appearances throughout the film.

```st src
<div class="language-cast">
  <--bot kind="blue" label="Research Bot" />
  <--bot kind="orange" label="Deck Bot" />
  <--bot kind="black" label="Grok Bot" />
  <--bot kind="mint" label="Copy Bot" />
  <--bot kind="red" label="Grok Bot heart" />
</div>
```

## A voice panel composes ordinary forms

The panel owns its surface. Bubbles own their content. The scene owns when
those bubbles appear. There is no parallel component or animation API.

```st src
<div class="language-voice">
  <--voice>
    <--bubble side="out">Can you join the call at 10?</--bubble>
    <--bubble>Of course.</--bubble>
  </--voice>
</div>
```

## A skin is typed data

Change one color role and the character, waveform and outgoing message update
together. The component anatomy and its choreography stay independently editable.

```st src
@data inline $languageAccent color : "#168bfa";

<div class="language-swatches">
  <button type="button" class="language-swatch--blue">Research blue</button>
  <button type="button" class="language-swatch--mint">Copy mint</button>
  <button type="button" class="language-swatch--orange">Deck orange</button>
</div>
<div class="language-voice language-live">
  <--voice>
    <--bubble side="out">One color role, every component.</--bubble>
    <--bubble>Same anatomy. Same language.</--bubble>
  </--voice>
</div>
.language-live { --film-blue: $languageAccent; }
.language-swatch--blue { @on &.click { $languageAccent <- "#168bfa"; } }
.language-swatch--mint { @on &.click { $languageAccent <- "#00c8a1"; } }
.language-swatch--orange { @on &.click { $languageAccent <- "#ff7900"; } }
```

## A move includes its lens treatment

Distance and softness are parameters on the same reusable motion form. The
focus envelope settles to zero and is exact when seeking backward. This is
an authored blur treatment; it does not estimate physical velocity.

```st src
<div class="language-demo"><div class="language-arrival"><--bot kind="mint" /></div></div>
.language-demo { @score &.loop(4s) { &language-arrival at 0s for 4s; } }
.language-arrival { @on &.clip { --film-arrive(80px, 12px); } }
```

## Put time in one place

The film's `score.st` owns the cut. Each scene owns its nested choreography.
A scene is a visibility layer, a camera is a framing layer, and a component
has its own element. Each layer can be edited without replacing the others.

The meeting's photographic filmstrips work the same way: `--frame` is a normal
keyframed custom property. CSS addresses its atlas cell with `round()` and
`mod()`. Playback, a reverse seek and an exported still share one clock.

```st hidden
html:root, html:root body { height: auto; overflow: auto; background: white; }
body { max-width: 1000px; margin: 100px auto; padding: 0 40px 100px; font-size: 22px; line-height: 1.6; }
h1, h2 { line-height: 1.15; letter-spacing: -1px; }
h1 { font-size: 65px; }
h2 { margin-top: 80px; font-size: 38px; }
a { color: #0874d5; }
pre { overflow: auto; background: #f5f7f8; padding: 28px; border-radius: 12px; font-size: 15px; }
.language-swatches { display: flex; gap: 12px; margin: 30px 0; }
.language-swatches button { font: inherit; background: #f2f5f7; border: 1px solid #dae0e5; border-radius: 8px; padding: 12px 18px; cursor: pointer; }
.language-swatches button:hover { background: #e5f0f8; }
.language-cast { display: flex; align-items: center; gap: 60px; margin: 50px 0; }
.language-cast .bot { width: 115px; height: 115px; }
.language-voice { height: 320px; position: relative; margin: 50px 0; }
.language-voice .voice { transform: scale(0.64); transform-origin: top left; }
.language-demo { position: relative; width: 100%; height: 250px; background: #e5fff5; border-radius: 20px; overflow: hidden; }
.language-arrival { position: absolute; left: 400px; top: 40px; }
.language-arrival .bot { width: 130px; height: 130px; }
```
