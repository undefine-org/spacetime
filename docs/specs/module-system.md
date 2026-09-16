# Spacetime module system — design

> Status: **partially implemented** (FEAT-118). 2026-06-14, updated post-FUP-057.
> User-facing guide for the shipped surface: [`docs/language/module-system.md`].
> SHIPPED: FQN-keyed registry · `@import` (global) / `@use` (namespaced) · folder
> = namespace via `MODULE.st %module` · qualified+alias references (`@scene/camera`,
> `@s/camera`) resolving through a per-file ImportScope · `only`/`hiding` filters ·
> `%public`/`%reexport`/`%using` retained from the manifest · `%claims` active
> extension · diagnostics E0924 (ambiguity) / E0926 (unbound qualifier) / E0927
> (visibility). DEFERRED: scoped `%capture_type` sub-grammar activation + `%default`
> value injection (matcher-level); `check`/`build` path unification (FUP-058).
> Companion to `docs/specs/spell-spacetime-unification.md` (§3.6/§3.7 fold the
> Spell/CodePath consequences). This doc owns the *language* design: namespacing,
> aliasing, re-export, the `@use`/`%using` grammar, the resolution algorithm, and
> the `@import → @use` cutover.
>
> Lineage: **Odin** skeleton (folder=package, concatenation, public-by-default,
> collections) · **PureScript** joints (qualified alias, explicit export lists,
> re-export, `hiding`) · **Elixir** soul (`use` runs `__using__` — active
> extension, not passive binding).

---

## 0. Thesis

Spacetime already *is* Odin-shaped without the boundaries: `load_dir_recursive`
concatenates every `.st` in a directory into ONE flat
`HashMap<String, MacroDefAst>` (`src/metasystem/registry.rs`), duplicates are
hard errors, folder location is currently irrelevant to naming. A `MODULE.st`
convention exists (`stdlib/editable/MODULE.st`, `stdlib/dispatch/MODULE.st`) but
is only doc + `@import "./index.st"`. `@import` is a parsed directive that
content-includes into the flat global registry — no namespacing.

∴ a module system is **adding boundaries to existing concatenation**, not a
rewrite. The single design move: **attach namespace to sigil-names**, applied at
two sites (definition registration; reference dispatch), driven by one context
(folder + in-scope `@use` table), declared as a bootstrap `%form` and compiled
to native resolution tables.

**`%form` never changes. The namespace is ambient from folder location and never
appears in a `%form`.**

---

## 1. The five elegance principles (the "why", load-bearing)

- **P1 — Sigil-scoped namespacing (layering).** The namespace attacher keys on
  the *sigil* (`@`, `%`). It touches only the meta/directive vocabulary.
  Everything sigil-less — CSS `.class`/`#id`, HTML `<tag>`, `$runtime-vars` —
  stays global *by construction*, zero special-casing. The sigil IS the
  membership marker for the namespaced layer. (CSS classes are global because
  the DOM is a shared visual namespace; directives are namespaced because they
  are a composable authored vocabulary — the language already separates these by
  sigil; namespacing respects a boundary that already exists.)
- **P2 — Reflective modularity (homoiconicity).** The module system is described
  in the language it governs (a bootstrap `%form`), the way Lisp's `defmacro`
  defines `defmodule`. No privileged Rust module-resolver. Consequence: the
  namespace rules are *data* the LSP and the Spell exporter read directly →
  Spacetime namespacing becomes Spell `::§` CodePath addressing for free.
- **P3 — Co-location of grammar and its extension discipline (locality).** The
  file that defines what `@` means is the file that defines how `@`-names get
  namespaced. The grammar of extension lives with the grammar.
- **P4 — Global is the absence of namespace (parsimony).** A root-collection
  form with no folder has no prefix → bare → global. You never write `%global`;
  globality is the degenerate (empty-namespace) case of the one rule. The
  bootstrap's own `@import`/`%macro`/core directives live at the root → global
  automatically.
- **P5 — Active modules (Elixir).** `@use` is not passive name-binding; it can
  run a module's `%using` hook that *extends the importing scope* — including
  installing scope-local capture-types (sub-grammars). A module is a scoped
  dialect extension, not a bag of macros.

---

## 2. Physical model (Odin)

- **Folder = module = namespace.** All `.st` in a directory concatenate (already
  true). Duplicate local name within a folder = error (already true). The
  module's name defaults to its folder name (override via `%module`).
- **Subdirectories** organize; nesting does **not** imply dependency
  (`scene/distort` needn't import `scene`).
- **Collections = named roots.** Required because stdlib is *embedded in the
  binary* while user modules are on disk. A collection prefix selects the root:
  - `std:`   — embedded stdlib (compiled into the `spacetime` binary)
  - `local:` — project root (cwd-relative)
  - user-defined — `spacetime.kdl`/CLI `--collection name=path`
  ```
  @use "std:scene"          // embedded stdlib
  @use "local:ui/cards"     // project module
  @use "./sibling"          // file-relative (no collection ⇒ relative, like Odin)
  ```
- **FQN is collection-qualified** to avoid cross-collection folder-name clashes:
  the canonical key is `std:scene/camera`. Bare folder names are NOT globally
  unique; the collection disambiguates. `as` aliasing collapses the long key.

---

## 3. Consumer side — `@use` (directive layer, PureScript joints)

```
@use "std:scene"                   // OPEN: scene/* bare-resolvable in this scope
@use "std:scene" as s              // QUALIFIED: @s/camera → scene/camera
@use "std:scene" (camera, light)   // EXPLICIT: only these bare in scope
@use "std:scene" hiding (glitch)   // OPEN minus listed
@use "std:scene" as s (camera)     // qualified + explicit (combine)
```

**Resolution discipline (PureScript):** bare by default; qualify on collision.
Two OPEN `@use`s both exporting `camera` → bare `@camera` is ambiguous → compile
error → author qualifies `@s/camera`. The common case stays ceremony-free; the
LSP (already holding the registry) shows provenance on hover.

**Scope of an `@use`:** file-level by default (the importing file's scope). A
`%using` hook (P5/§5) may widen what it installs, but visibility is the file.

---

## 4. `@import` → `@use` cutover (start here — justified)

`@import "scene"` today ≡ "parse path, merge defs into the flat global registry."
That is exactly **`@use` with everything flattened to global, open, no alias** —
the degenerate point of the `@use` spectrum (P4).

```
@import "scene"               ≡  @use "scene" (open, GLOBAL, unprefixed)
                                  ← the 98 existing @imports keep this meaning
@use    "scene"                  namespaced: scene/* , bare-resolvable, collision-checked
```

**Decision:** introduce `@use` as the namespaced form; **keep `@import` as the
global-merge alias** (literally `@use` with the `global` flag set). Rationale:
- the 98 existing `@import`s keep working unchanged (they still "flood global");
- soft cutover, zero migration; `@import` becomes sugar over `@use`;
- teachable relationship: **"`@import` floods, `@use` scopes."**

This also exercises both auto-extender modes (§6): `@import` drives definition-
side *global* registration (no prefix); `@use` drives both definition prefixing
and reference resolution.

---

## 5. Library-author side — `MODULE.st` becomes the manifest (Elixir `__using__`)

`MODULE.st` exists today as doc + `@import "./index.st"`. Complete it into the
module manifest + active-extension site. **All clauses optional** — implicit
folder-name + public-by-default (Odin floor) until you want control (PureScript
ceiling).

```st
// stdlib/scene/MODULE.st
%module scene                       // OPTIONAL: override implicit folder name (rare)

%exports (camera, light, fog)       // OPTIONAL: explicit API.
                                    //   PRESENT ⇒ only these are importable.
                                    //   ABSENT  ⇒ all public (Odin default).
                                    //   NB: distinct from the existing %exports
                                    //   primitive-clause (exposes $-vars) — this
                                    //   is module-level; see §8 naming note.

%reexport "std:scene-3d" (form, surface)   // OPTIONAL: PureScript façade/bundle

%using($opts) {                     // OPTIONAL: Elixir-style ACTIVE extension.
    %claims @camera @light @fog     //   install namespace-rewrite rules for these
    %capture_type camera_type { ... }   // SCOPED sub-grammar: valid only where
                                        // scene is @use'd (a dialect extension)
    %default fov: 75                 //   scope-local defaults
}
```

The deepest unlock (P5): `@use "std:scene"` can bring in `@camera` **and** the
`camera_params` capture grammar valid only in that scope — the equivalent of
`use Phoenix.Component` activating a whole DSL. Modules are scoped dialects.

---

## 6. The self-hosted namespace auto-extender (the core mechanism)

A bootstrap `%form` that **attaches namespace to sigil-names**, fired at two
sites from one context (folder + in-scope alias table):

```
DEFINITION-side  (short → long, at register_macro):
    %macro camera   defined in collection std:, folder scene/
    → registry key  std:scene/camera
    (author writes BARE; extender prefixes — zero authoring ceremony, P4-friendly)

REFERENCE-side   (bare → correct, at dispatch):
    @camera   in a file that did `@use "std:scene"`
    → resolves to  std:scene/camera
    @s/camera in a file that did `@use "std:scene" as s`
    → alias s expands to std:scene/ , then /camera
```

**Self-hosted means DECLARED in `.st`, COMPILED to native tables — NOT
re-interpreted per token.** This is the identical discipline verse already uses
for every `%form` (written in `.st`, compiled once into `SyntaxRegistry`
matchers). The attacher compiles into:

1. a **prefixed registry key** (string concat at registration — O(1));
2. a **scope-local alias table** (`s → std:scene/`, hashmap — O(1) lookup);
3. a **dispatch pre-filter** (restrict candidates to in-scope namespaces before
   the existing scorer runs).

**Efficiency:** namespace pre-filtering SHRINKS the candidate set the current
scorer iterates → resolution is *cheaper* than today's flat-global dispatch, not
costlier. (The scorer already exists: `build_dispatch_probe`/FEAT-087.)

**Co-location (P3):** the attacher `%form` lives in the same bootstrap `.st` that
defines what `@`/`%` mean. Reading one file shows both "what a directive is" and
"how directives compose into modules."

**Globality (P4):** a definition in a root collection with no folder gets no
prefix → bare key → global. No `%global` keyword; the empty-namespace path is
the same code path.

---

## 7. Resolution algorithm (normative)

Given a sigil-reference `@name` (or `@alias/name`, or `@coll:ns/name`) in file F:

```
1. PARSE the reference into (sigil, segments[], explicit_collection?).
   - segments split on '/'  (the SpacetimeNameLexer path_split axis)
   - a leading "coll:" is an explicit collection
2. If FULLY QUALIFIED (collection + full ns path): look up the exact FQN key. Done.
3. If ALIASED (first segment matches an `as` alias in F's scope):
   expand alias → collection+ns prefix, append remaining segments → FQN. Look up.
4. If BARE:
   a. Collect candidate FQNs = { ns/name : ns ∈ in-scope namespaces of F }
      where in-scope = open `@use`s + explicit-import names + same-folder defs
      + root globals (P4) + bootstrap globals.
   b. 0 candidates → "unknown directive @name" error.
   c. 1 candidate → resolve.
   d. >1 candidate → AMBIGUITY error: "@name exported by {scene, ui}; qualify
      as @s/name or @u/name." (PureScript discipline.)
5. Feed the resolved FQN's %form set to the EXISTING scorer (scope/positional
   dispatch unchanged). Namespace filtering only SHRINKS the input set.
```

Definition registration (mirror):
```
register_macro(def) in (collection C, folder ns):
   key = if C is root && ns empty  →  def.name          (global, P4)
         else                       →  "C:ns/" + def.name
   duplicate key → error (existing behavior, now namespace-scoped)
```

---

## 8. Naming collision with existing `%exports` / `%scope` (must resolve)

Verified in-use today (do not break):
- **`%exports { $x: T }`** — a *primitive* clause exposing runtime `$`-vars (44
  uses, e.g. `stdlib/mobile/primitives/camera.st`). Module-level `%exports
  (names)` (§5) is a DIFFERENT construct at a DIFFERENT site (MODULE.st, name
  list not typed-var block). Disambiguate by **position** (inside `%primitive`
  body vs. in `MODULE.st`) OR rename the module one to **`%public (names)`** to
  avoid overload. **Recommendation: `%public` for the module API list** — clearer
  and collision-free.
- **`%scope file | selector`** — positional dispatch validity (28 uses). The
  module visibility scope (§3) is unrelated; do NOT reuse `%scope`. Module
  visibility is implicit (file-level) — no new keyword needed.

---

## 9. Logical conclusions (design consequences surfaced by this work)

1. **Modules as scoped dialects** (P5/§5): `@use` can scope-extend the *grammar*
   (capture-types), not just vocabulary. Spacetime gains sub-languages on
   demand. Arguably bigger than namespacing itself.
2. **`MODULE.st` completes a started sentence**: the stub convention becomes the
   `%module`/`%public`/`%using` manifest.
3. **Re-export = curated bundles = Spell profile boundaries**: a project module's
   `%reexport`/aliased `@use` gives the Spell exporter natural per-namespace
   profile units. Modularity (verse) = addressability (Spell), one design.
4. **Collections solve the embedded-stdlib problem**: `std:` (in-binary) vs
   `local:` (disk) share one import grammar; the resolver ignores where bytes
   live. Required given embedding, not optional.
5. **The collision rule writes itself** (§7.4d): PureScript ambiguity → qualify;
   LSP shows provenance on hover (registry already loaded).
6. **Authoring gets CHEAPER, not costlier** (§6): definition-side prefixing means
   the author still writes bare `%macro camera` and gets `scene/camera` free.
   Odin's zero-friction floor + PureScript's qualified ceiling, no ceremony tax
   on the common case — the ergonomic win for BOTH meta-compiler dev (one
   bootstrap `%form`) and user (write bare, get namespaced).

---

## 10. The one separator `/` unifies four axes

```
filesystem:   stdlib/scene/camera.st
module FQN:    std:scene/camera
reference:     @scene/camera   (or @s/camera via alias)
Spell CodePath: scene/camera.st::§macro
```

`/` is value-position-only in `.st` today (CSS shorthand `12px/1.5`, grid `1/3`,
aspect `16/9`) — never in sigil/name position, so `@s/camera` is unambiguous by
construction. `::` is reserved for CSS pseudo-elements; `.` collides with
class/field. `/` also reads as path/module to a web audience. See unification
spec §3.6 for the `SpacetimeNameLexer` / `DefaultNameLexer` fold.

---

## 11. Phasing (within FEAT-813 L0/L2; module work is a verse-side track)

- **M0** Collections + FQN keying: `std:`/`local:` roots, collection-qualified
  registry keys, `register_macro` prefixing. No surface change yet (all root/
  global, P4) — pure plumbing, behavior-preserving.
- **M1** `@use` open + `as` alias + bare resolution + ambiguity error
  (§3, §7). `@import` becomes the `global` alias (§4). The auto-extender `%form`
  in the bootstrap (§6).
- **M2** `MODULE.st` manifest: `%module`/`%public`/`%reexport` (§5 minus hook).
- **M3** Elixir soul: `%using` active hook incl. scoped `%capture_type` (P5).
- **M4** LSP provenance-on-hover + Spell exporter consumes namespaces as `::§`
  paths (unification spec §3.7/§4).

M0–M1 are the minimum for namespacing; M3 is the differentiator; M4 is the
Spell payoff.

---

## Appendix — verified anchors (verse)
- `src/metasystem/registry.rs`: `register_macro` (:161), `load_dir_recursive_collecting`
  (:568), `macros: HashMap<String,MacroDefAst>` (:114, flat global today),
  `get_registry_namespace` (:527, codegen-target ns — NOT source ns),
  duplicate-def errors (:77/:80).
- `stdlib/editable/MODULE.st`, `stdlib/dispatch/MODULE.st` — the doc+`@import`
  convention to complete.
- `@import` parse: `src/syntax/cst/parser.rs` (:338) — directive, parse-only.
- existing clauses to not collide with: `%exports` (44, primitive $-vars),
  `%scope` (28, positional dispatch), `%registers` (33), `%order`, `%binds`.
- dispatch scorer to pre-filter (not replace): `build_dispatch_probe` (FEAT-087),
  scope matching `macro_scope_matches` (registry.rs ~:390).
