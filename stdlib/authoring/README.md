# Authoring components

Import `stdlib/authoring` for reusable property editors, provenance trails,
scope choices and definition cards. The catalog and Studio use these same
templates, styles and typed control projections.

```st
@import "stdlib/authoring";
@data fetch $program : "/__spacetime/reflect.json?entry=index.st";
@data inline $definition string : "--message";
@data derive $properties array : ST.authoring.definitionProperties($program, $definition);

<section class="editor"><div class="properties"></div><p data-authoring-status></p></section>
.properties { @each($properties as $p) { &property-editor($p); } }
.editor { @authoring-edit(entry: "index.st"); }
```

`&property-editor($p)` separates authored/evaluated values, origin, state,
typed controls, legal scopes and diagnostics. `&origin-trail($p)` and
`&scope-picker($p)` can also be composed independently. A host decides the edit
destination; controls do not choose source files or invent overrides.

Compose child templates with body-root references such as
`&origin-trail($p);` after the parent's root markup. Each child owns its markup
and behavior; CSS arranges the resulting children.

`ST.authoring.field(param, overrides)` projects reflected scalar metadata and
choices into the shared compact field contract. `ST.authoring.property` adds
the authoring view. `definitionProperties` targets parser-owned default spans;
defaults without spans expose a diagnostic and cannot be written by the host.
`definitions` and `declarationCards` adapt reflected or compile-time declaration
rows for `&definition-card`.

The edit host listens for ordinary control changes, preserves source quotes,
and uses `/__spacetime/rewrite`. In Studio it delegates to Studio's existing
live-edit/transport handshake. Elsewhere the normal development reload shows
the new source. All persistence uses the shared guarded writer and undo journal.

The views can be used without the source-edit behavior for transient previews.
No request or file write occurs merely because the library is imported.

See `demos/authoring-catalog` for live markup/style/motion cases and the
generated catalog of these components and their behavior macros. Browser
integration gates live in `tests/studio/authoring-catalog.drive.st` and
`tests/studio/authoring_source_e2e.sh`; shared field regressions live under
`stdlib/fields/tests/`.
