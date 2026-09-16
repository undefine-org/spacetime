# The effect gallery

HyperFrames, redone in Spacetime — one literate snippet per registry block.

```sh
cargo run -- serve demos/effect-gallery/
```

## What this is

The [HyperFrames registry](../../../scratch/hyperframes/registry/registry.json)
catalogs 117 React/Remotion motion pieces (captions, lower thirds, wipes,
texture tricks; 25 more are code-snippet syntax themes). Each snippet under
`snippets/` redoes one of those blocks in the Spacetime idiom: **one `@score`
placement plus one keyframe body**, no JavaScript, no library, no build glue.

Every snippet is a self-contained `.st.md` file: the prose, the full source
(shown on the page via a ` ```st src ` fence), and the live result all live in
one file. `index.st.md` only `@import`s the snippets — edit one and rebuild,
the gallery cannot drift. This is the same composition pattern as
`projects/backdesk-new/process.st.md`, with the prose living IN the snippets.

For the long-form version — a 12s title sequence rendered to mp4 with
`cargo run --features cdp -- render` — see [../score-launch](../score-launch/).

## Adding a snippet

1. `snippets/<name>.st.md` with a `## Title`, one or two sentences naming the
   registry block redone, and one ` ```st src ` fence.
2. Scope every selector under a unique `.g-<name>` class; use literal colors
   (snippets must be copy-pasteable, no host dependency).
3. `@import "./snippets/<name>.st.md"` in `index.st.md`, in gallery order
   (imports render in import order, after the host's own prose).

## Contracts that will bite you (learned the hard way, now gated)

- **Placement names are addresses.** A consumer `.foo` finds its window by
  NAME: the placement `&foo` and the consumer's LAST selector class must match
  (`selector_element` rule). `.g-kara .w1` consuming `&w1` is fine — the last
  compound is the element. A mismatched name compiles clean and never moves.
- **Container-level stagger**: put the `@score` and `@on &.clip` on the
  container and the motion scope nested (`.cell { stagger: 45ms; … }`).
  Per-element clips give every element stagger index 0.
- **Gradients with reactive values**: write `background-image:` (longhand).
  The `background:` shorthand lands via `setProperty` and resets static
  longhands like `background-clip: text` (BUG-279).
- **Linear loops** (tickers, marquees): `easing: --linear` in the keyframe
  body, and make the last frame identical to the first (duplicate the copy,
  travel exactly half).

## Gates

`tests/gallery/effect-gallery.test.st` (run with `--cdp`) mirrors four fences
verbatim — karaoke windows, pixelate stagger, ticker travel, morph phases —
and asserts they MOVE, with traces. Behavior-first per BUG-252: a green
string in the bundle is not a working effect.
