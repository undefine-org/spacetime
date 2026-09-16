# Spacetime Modules — `@use`, namespaces, and aliasing

> Status: **implemented** (FEAT-118). This guide covers the user-facing module
> surface that ships today. The full design rationale (the five elegance
> principles, the resolution algorithm, the `@import → @use` cutover) lives in
> [`docs/specs/module-system.md`](../specs/module-system.md).

Spacetime concatenates every `.st` file it loads into one registry. **Modules add
boundaries to that concatenation** so two files can each define `@badge` without
colliding, and a page can pull in exactly the directives it wants under the name
it wants. There is no new syntax to learn for the *definitions* — a folder is a
module, and the namespace is ambient from where a file lives. You only write
module syntax at the **import site**.

The whole system rests on one rule: **the sigil is the membership marker.**
`@`-directives and `%`-definitions get namespaced; sigil-less names — CSS
`.class` / `#id`, HTML `<tag>`, `$runtime-vars` — stay global by construction,
because the DOM is a shared visual namespace. You never write `%global`; a file
with no module is simply the empty-namespace (global) case.

---

## `@import` vs `@use` — flood vs scope

There are two points on the import spectrum:

```st
@import "stdlib/text"     // FLOOD: merge every definition into the global scope
@use    "std:scene"       // SCOPE: bring the module in under its namespace
```

- **`@import`** is the historical, unscoped include. Everything the target defines
  becomes globally visible, exactly as if you'd pasted it in. Use it for the
  stdlib essentials a page always wants.
- **`@use`** scopes the import: the module's directives key under the module's
  namespace, reachable by qualifier or alias, and subject to `only`/`hiding`
  filters. Use it when you want boundaries — multiple modules, or a controlled
  surface.

> Mnemonic: **`@import` floods, `@use` scopes.** `@import` is the degenerate
> `@use` (open, global, unprefixed).

---

## Referencing a `@use`'d module

Given a module imported with `@use`, its directives are reachable three ways:

```st
@use "std:scene"               // open import

.hero  { @camera(fov: "75") }       // BARE — resolves if unambiguous
.hero  { @scene/camera(fov: "75") } // PATH-QUALIFIED — names the namespace
```

```st
@use "std:scene" as s          // aliased import

.hero  { @s/camera(fov: "75") }     // ALIAS-QUALIFIED — `s` → std:scene
```

- **Bare** (`@camera`) resolves when the name is unambiguous across what's in
  scope. If two open modules both export `@camera`, the bare call is **ambiguous**
  and the compiler asks you to qualify it (warning `E0924`).
- **Path-qualified** (`@scene/camera`) names the module by a path-suffix of its
  namespace.
- **Alias-qualified** (`@s/camera`) names the module by the alias bound with `as`.

A qualifier that names nothing in scope — a typo, or a missing `@use` — is an
**error** (`E0926`):

```st
@use "std:scene" as s
.hero { @scn/camera(fov: "75") }   // ✗ E0926: unbound qualifier `scn`
```

---

## Controlling the surface — `only` and `hiding`

`@use` accepts PureScript-style import-list filters, in any order after the
module ref (and after `as`, if present):

```st
@use "std:scene" only (camera, light)   // ONLY these two are visible
@use "std:scene" hiding (fog)            // everything EXCEPT fog
@use "std:scene" as s only (camera)      // combine with an alias
```

Referencing a name the list excludes is an **error** (`E0927`):

```st
@use "std:scene" only (light)
.hero { @camera(fov: "75") }   // ✗ E0927: @camera not visible (only allows: light)
```

- **`only (a, b)`** — an allow-list. Only the listed names are importable.
- **`hiding (a, b)`** — a deny-list. Everything except the listed names.
- Neither present — open import (everything the module exports).

---

## Library authors — `MODULE.st`

A folder becomes a named module with a `MODULE.st` manifest:

```st
%module scene            // the namespace this folder's defs register under
%public (camera, light)  // OPTIONAL export allow-list (absent = all public)
%reexport "std:effects" (bloom)   // OPTIONAL façade: re-expose another module's names
```

- **`%module <name>`** binds every `.st` in the folder to the `<name>` namespace.
- **`%public (…)`** is the export allow-list. Present ⇒ only those names are
  importable (Odin's public-by-default floor still applies when absent).
- **`%reexport "ref" (names)`** re-exposes another module's names through this one
  — a façade.

### Active modules — `%using` (Elixir `__using__`)

A module can do more than expose names: it can **actively extend** the importing
scope. The `%using` hook declares what `@use`-ing the module installs:

```st
%using {
  %claims @camera @light     // bare @camera/@light bind to THIS module on @use
  %capture_type camera_type { … }   // a scoped sub-grammar (advanced)
  %default fov: 75                   // scope-local defaults (advanced)
}
```

`%claims` is the active part that ships today: when a page `@use`s a module whose
`%using` hook `%claims @camera`, a **bare** `@camera` in that page binds to the
claiming module as if implicitly qualified — the module reaches into the page's
vocabulary, rather than waiting to be addressed. (Scoped `%capture_type` grammars
and `%default` values are parsed and retained; their activation is a deeper
matcher-level feature.)

---

## The `/` separator unifies four axes

One glyph, `/`, means "namespace path" everywhere:

| axis | form |
|------|------|
| filesystem | `stdlib/scene/camera.st` |
| fully-qualified name | `std:scene/camera` |
| reference | `@scene/camera` · `%b/tokens` · `app/$host` |
| Spell CodePath | `scene/camera.st::§macro` |

This is why a Spacetime namespace maps cleanly onto a Spell `::§` address — the
addressing scheme is shared by construction.

---

## Referencing a `$` cell — `app/$host`

A module's **cells** are addressed by the same `/`, with one difference you can
read off the sigil:

```st
@use "./config.st" as app

<p>the API lives at `app/$host`</p>
```

| you write | because |
|-----------|---------|
| `@b/badge`, `%b/tokens` | the sigil types an **invocation** — it leads the path, and the whole `b/badge` is one registry name |
| `app/$host` | the sigil types a **referent** — it stays glued to its leaf, so the qualifier goes in front |

**Invocations lead; referents hug.** That is the whole rule, and it is why the
qualifier moves rather than the sigil.

### Why not `$app/host`

Because `$app` is a valid operand and `@app` is not. `@scene/camera` has been
unambiguous since FEAT-118 precisely because nothing can divide by `@app` —
but `$app / host` is real arithmetic. Putting the qualifier in front restores
the property: **after a `/`, a `$` never begins a divisor.**

So division is untouched, and needs no spaces to stay division:

```st
button {
  @on &.click { $half <- $total/$count; }   // division — head `$total` is an operand
}

<p>`app/$host`</p>                        // qualified ref — head `app` is a bare name
```

The one genuinely ambiguous shape — a tight numeric head, `1/$denominator` —
is **refused** (`E0939`) rather than guessed. Add spaces: `1 / $denominator`.

### A qualifier is a rename, not an indirection

`app/$host` compiles to *exactly* the same JS as the bare `$host` in the file
that declares it — byte-identical. There is no wrapper, no lookup, no cost.
The alias is resolved and discarded at compile time.

### Publishing — `@exports`

A file is public by default. Add an `@exports` clause and it becomes a
contract: anything unlisted is then private (`E0944`).

```st
// config.st
$host string: "https://api.example.com";
$sessionId string: "";

@exports { $host }        // $sessionId is now private to this file
```

Cells are read-only through a qualifier — `app/$host <- "x"` is refused
(`E0942`). A cell is written where it is declared, so its write-set stays
readable in one file. (Mutable exports use `@exports { $count: mut }`.)

> `%public` in a `MODULE.st` is a **different** control and still applies: `%`
> marks which *grammar* a module hands out, `@exports` marks which *cells* a
> file publishes. One sigil, one meaning.

### Inspecting

```sh
cargo run -- inspect --layer state index.st
```


```
$host  string  = "https://api.example.com"
    declared config.st
    readers  app.st (1)          writers ∅
    verdict  const — write-free literal; foldable at every read
```

Because Spacetime pages carry no custom JavaScript, a cell's write-set is
**closed** — so `verdict` is proven by a whole-page scan, not guessed. A cell
with no readers is `dead` and is dropped from the bundle.

---

## Diagnostics quick reference

| code | meaning | severity |
|------|---------|----------|
| `E0924` | bare directive ambiguous across two open modules — qualify it | warning |
| `E0926` | qualifier names nothing in scope (typo / missing `@use`) | error |
| `E0927` | name excluded by the import's `only`/`hiding` list | error |
| `E0938` | the same `$cell` is declared twice on one page — rename one | error |
| `E0939` | ambiguous tight `1/$x` — add spaces to divide | error |
| `E0940` | qualifier in `app/$host` names nothing in scope | error |
| `E0941` | that module declares no such cell | error |
| `E0942` | cannot write through a qualifier — write it where it is declared | error |
| `E0943` | a `:` where a `/` belongs (`@b:badge`) — use `@b/badge` | error |
| `E0944` | the cell is not in that file's `@exports` | error |

### Upgrading an existing page

Nothing to migrate — these codes are all **new refusals of things that
previously had no meaning**, so existing code keeps compiling. Verified across
the 26 examples, 25 demos, 345 stdlib files and 23 private projects (267 `.st`
files): **zero** occurrences.

Two shapes change behaviour, both from *silence* to *an error*:

| was | now |
|-----|-----|
| `@b:badge(…)` reported success and emitted **nothing** | `E0943`, with the `@b/badge` fix in the message |
| two files each declaring `$theme` merged silently | `E0938`, naming both files |

If you hit either, the message names the fix. There is no flag to restore the
old behaviour: both old paths silently produced a page the author did not
write.

---

## A complete example

See [`examples/modules/`](../../examples/modules/): a `badge` module and a page
that imports it `as b` and calls `@b/badge`. Build it with:

```sh
cargo run -- build examples/modules/index.st -o scratch/modules-out
```

```st
// examples/modules/badge.st
%macro badge {
  %form { @badge($label:string) { $styles:properties } }
  %binds { badge-emit($label, styles: $styles) -> {} }
}
%primitive badge-emit {
  %emit css { .badge::before { content: "$label"; } }
}
```

```st
// examples/modules/index.st
@use "./badge.st" as b

<main><span class="badge"></span></main>

.badge {
  @b/badge("NEW") { color: red; }
}
```

The alias-qualified `@b/badge` resolves through the per-file import scope to the
module's `@badge` and emits the badge CSS.
