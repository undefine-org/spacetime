# Authoring catalog

An executable catalog of Spacetime's reusable authoring components. Its live
examples are markup, style and motion definitions in `fixtures.st`. One
`&property-editor` reads their typed parameters and parser-owned default spans
from reflection. Editing a control writes that definition through the shared
guarded rewrite/undo service; it is a real source edit.

```sh
spacetime serve demos/authoring-catalog/ -p 4560
```

The page's component and macro cards come from `@data declarations` and
`@data dispatch`, including the actual components used to render the page.
There is no application JavaScript. UI anatomy, surfaces and behavior live in
`stdlib/authoring`; compact controls now belong to `stdlib/fields`, with the
existing `stdlib/widgets` import retained for compatibility.

Use a disposable copy when experimenting with shared defaults. The source
browser test does so automatically:

```sh
SPACETIME_BIN=./result/bin/spacetime tests/studio/authoring_source_e2e.sh
spacetime test tests/studio/authoring-catalog.drive.st --cdp
```

The source test verifies actual preview changes, quoted source spelling,
dimensioned motion parameters, shared undo and stale-write rejection. The
drive gate expects the catalog at port 4560. Browser behavior is verified with
Chromium; a headless skip does not count as verification.

The light example is a CSS surface illustrating a numeric parameter; the
Studio's 3D inspector uses the same field projection for actual scene parts.
The arrival preview uses the native score clock and an authored blur envelope.

`assets/fonts/Tinos.ttf` is the existing Google Fonts Tinos regular face,
reused from the film assets under the included SIL Open Font License. It is
served locally so the catalog's display typography does not depend on host
fonts. All remaining visuals are native markup and CSS.
