# Reflect and rewrite

Every tool that reads or writes a Spacetime page does it the same two ways
(PLAN-156). The studio, the admin, MCP agents and the migrator all use them,
and none of them has a private parser or a private writer.

## Reflect: the program as tools see it

```
GET /__spacetime/reflect.json?entry=index.st        (dev server)
st_reflect { path, entry }                          (MCP)
crate::reflect::page(site_dir, entry)               (Rust)
```

A page's files (the entry, its `@import`s, the project's `_prelude.st`) are
parsed against the stdlib registry, the project overlay and the page's own
`%macro`s. Every form match is returned like this:

```json
{
  "id": "0:2817",
  "directive": "gltf", "macro": "gltf",
  "file": 0, "start": 2817, "end": 2893, "selector": "canvas.world", "parent": "0:2311",
  "doc": "the hero of the film",
  "params": [
    { "name": "scale", "type": "number", "default": "0", "optional": true,
      "written": { "file": 0, "start": 2871, "end": 2875, "text": "0.42" }, "value": 0.42 },
    { "name": "spinY", "type": "number", "default": "0", "optional": true,
      "written": null, "value": 0 }
  ],
  "entries": [ { "kind": "part", "key": "robot", "fields": { "kind": "gltf" } } ],
  "spans": { "named": { … }, "src": { … } },
  "children": []
}
```

- **params** are the ones the macro's `%form` *declares*, with their capture
  type and default. A param the author wrote has its words and byte span in
  `written`; a param left out has `written: null` and its default.
- **entries** are the macro's `%registers` clause, resolved for this call:
  `$var` fields become the words this match captured. A tool asks for « every
  `clip` » or « every `part` » by entry kind. It never names a macro, so
  renaming one breaks no tool.
- **spans** are every capture span the parse kept, nested paths included
  (`entries.3.at`): the addresses for edits below the param level.
- `files[i].hash` is the FNV-1a hash of the file's text. A transaction can pin
  a file to it.

Nothing in `src/reflect` knows a macro by name. What a construct *is* gets
declared next to it in the stdlib.

## Rewrite: the one writer

```
POST /__spacetime/rewrite        { entry, edits: [...], base?: { path: hash } }
POST /__spacetime/rewrite/undo
st_rewrite { path, entry, edits } · st_undo { path }      (MCP)
crate::rewrite::{apply, commit, undo, page}               (Rust)
```

An edit reads « the bytes `start..end` of `file`, which read `expect`, now
read `text` ». A transaction's edits apply together or not at all:

| refused when | error |
|---|---|
| a span no longer reads `expect`, or a file's hash isn't its `base` | `STALE: …` |
| two edits overlap, or a span leaves the file | span error |
| a file isn't one of the page's | unknown file |
| a file that parsed would stop parsing | nothing is written |

Files are written through a temp file and a rename. Each commit is journaled
(its files' before and after texts), and `undo` puts the last one back if
nobody has changed those files since. In the studio, **Ctrl/Cmd+Z** is that
undo, so it reverts the last edit of *any* writer, an agent's included.

The studio's gesture ops (`/__spacetime/studio/apply`) and `spacetime migrate`
write through the same functions.
