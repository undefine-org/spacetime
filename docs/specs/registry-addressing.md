# Registry addressing — one reference shape for every sigil-space

**Status:** IMPLEMENTED (W1-W5). See `!tasks/plans/PLAN-117` for the wave log.
**Related:** `docs/specs/module-system.md` (FEAT-118), `docs/language/symbol-system.md`

---

## 1. The principle

> The language must be learnable by pattern — knowing one thing should tell you
> the next. *(AGENTS.md, Decisionmaking)*

Spacetime has four author-level registries, each selected by a sigil:

| sigil | registry | what a name in it denotes |
|---|---|---|
| `@` | directives | an invocation |
| `%` | macros / primitives / capture types | a grammar definition |
| `$` | cells | a reactive data location |
| `&` | places | an element, template, or entity on the page |

Today `@b/badge` resolves through a module alias and `$b/host` does not. That
asymmetry is not a missing feature; it is the language failing its own
principle. This spec closes it with **one reference shape across all four**.

## 2. The four axes

| axis | meaning | phase | example |
|---|---|---|---|
| **sigil** | *which registry* | selects | `@` `%` `$` `&` |
| **`/`** | address *within* a registry | compile-time | `@3d/stage`, `app/$host` |
| **`:`** | collection prefix — *only inside an import string* | compile-time | `@use "std:scene"` |
| **`.`** | descent *into* the resolved thing | runtime | `&hero.rect.top`, `$user.id` |

`/` and `.` are not rivals. **`/` gets you *to* an entry; `.` goes *into* it.**
Each keeps exactly one meaning.

## 3. The reference forms

```st
@3d/stage          %3d/env-form        app/$host          app/&rail
@stage             %env-form           $host              &rail        /* bare */
```

### 3.1 Why the sigil sits differently for `@`/`%` than for `$`/`&`

`@scene/camera` puts the sigil at the head; `app/$host` puts it on the leaf.
This is not an inconsistency — it follows from what each sigil marks:

- **`@` and `%` type an INVOCATION.** The sigil marks the *act*; the path says
  *of what*. `@scene/camera` is ONE registry name, stored slash-joined
  (`std:scene/camera`), and `@camera` is the same atom's short spelling.
- **`$` and `&` type a REFERENT.** The sigil marks the *thing*. `$host` is an
  atom — the sigil is part of the name — so qualification attaches *outside*
  the atom rather than stretching a path under the sigil.

∴ invocations lead their path; referents hug their leaf.

### 3.2 Why `app/$host` and not `$app/host`

`$app` is a **valid operand** — it reads a cell. So `$app/host` is genuinely
ambiguous with division and no lexing rule can settle it without whitespace
significance.

`@app` is **not** an operand; it can only begin a reference. That is the entire
reason `@scene/camera` has been safe since FEAT-118.

Putting the qualifier in front restores the same property: **after `/`, a `$`
never begins a divisor**, so `/$` is unambiguous wherever the head is a bare
identifier.

### 3.3 Why not `app.$host`

`.` in expression position already heads live bare-identifier member access.
Verified in-repo:

```st
@data fold $cartTotal number from $cart : acc + (item.price * item.qty) ;
```

`acc` and `item` occupy exactly the head space a dotted qualifier would need.
`/` in that position has no such occupant.

## 4. The `:` quarantine

`:` is the most overloaded token in the language. Counted across the repo:

| role | evidence |
|---|---|
| CSS declaration | ubiquitous |
| CSS pseudo (`:hover`, `::before`) | 181 lines |
| value-introducer (`$open bool: false`) | 39 state-init lines + all `@data` |
| **meta capture type (`$preset:string`)** | **618 lines** |
| macro named argument (`@env(preset: studio)`) | pervasive |
| ternary (`a ? b : c`) | live |

Collection prefixes coexist with all six only because they appear **inside a
quoted import string** — a sealed lexical zone where nothing else parses.
Verified: exactly 2 occurrences of a `coll:` prefix repo-wide, both inside doc
comments; zero in live code.

> **Normative.** `:` appears only inside a module address, and a module address
> appears only inside a quoted import string. `:` never enters reference
> position.

∴ `local:config/$session` is **not** a reference. Write:

```st
@use "local:config" as app          /* the one place `local:` appears */
.hero { text <- app/$host; }        /* reference: alias-qualified */
```

This costs nothing — `as` collapses the long key, which `module-system.md` §2
already teaches as the recommended style.

### 4.1 A colon in reference position is an error

Verified defect (differential build):

```
@b/badge("NEW")   →  emits `.badge::before` into spacetime.css
@b:badge("NEW")   →  reports "✓ passed", emits ZERO css
```

A silent drop is the worst outcome for an addressing mistake. `E-qualifier-colon`
refuses it and names the fix.

## 5. Lexing: tight adjacency

The rule already exists and ships for `@` (`src/syntax/cst/parser.rs:636-652`,
mirrored in `ast.rs:283`): consume `/IDENT` segments **only when no trivia
surrounds the `/`**. A spaced `/` ends the name run.

W3 mirrors that loop for a `$`-led leaf. The full rule:

> After `/`, a `$`-led leaf opens the **qualified** production **only when the
> head before the `/` is a bare identifier**. A head that is an operand (`$x`, a
> number, a paren group) keeps `/` as division.

Evidence that this is safe, counted rather than assumed:

| shape | occurrences | verdict |
|---|---|---|
| real division in expression position | 4 (one file, all `/ 100`) | unaffected — spaced, numeric RHS |
| division inside backtick holes | 0 | unaffected |
| CSS slash values | 9 (all `font: 15px/1.6`) | unaffected — numeric head |

### 5.1 The one refusal

`1/$denominator` — a **tight numeric head** before a `$` leaf. A number can
never be a qualifier, so this *is* division; but it reads exactly like the
qualified form. The compiler **refuses** (`E-slash-ambiguous`) and names the
fix rather than silently picking:

```
error[E-slash-ambiguous]: `1/$denominator` is ambiguous
  = a tight `/$` reads as a registry address, but `1` cannot be a qualifier
  help: add spaces to divide — `1 / $denominator`
```

Refusing beats guessing (`docs/testing/FIDELITY_LADDER.md`). Forcing a space in
this one shape is the accepted cost of the form.

## 6. Resolution

The machinery exists and is **already sigil-agnostic**. The sigil is consumed at
stage 1 and never participates in resolution:

1. **Parse** — the qualifier is glued into the reference head by the
   tight-adjacency loop, and carried in `FormMatch.namespace_qualifier`
   (`src/syntax/events/form_compiler.rs:379-398`).
2. **Canonicalise** — `ImportScope::resolve_qualifier` rewrites an alias (or a
   literal namespace-path suffix) to the namespace key
   (`src/pipeline/mod.rs:2878-2896`, `src/metasystem/module.rs:340`).
3. **Visibility** — `ImportScope::is_visible` applies `only` / `hiding`.
4. **Lookup** — FQN-first (`Fqn::key`), falling back to the bare name; a global
   definition keys to its bare name byte-for-byte, so existing files are
   unaffected.

W3 adds **one production**. Stages 2–4 are reused verbatim — which is also why a
*fifth* registry would cost zero grammar.

## 7. Cells are addresses; grammar is dialect

`@use` and `@import` both bring a module's **grammar** into a file, and that
flooding is correct, not a compromise:

```st
%form { @env(preset: $preset:string = "studio") }
```

That `string` is a capture type resolved from the registry while the parser is
still deciding what these bytes *mean*. There is no head to qualify — no
reference site at all. Importing such a module changes **what this file's
language is**. That is a dialect being spoken, and a dialect cannot be
namespaced.

Cells are the opposite: `$host` is a lookup by name, and names are addresses.
Two files silently merging their `$host` into one cell is not composition — it
is the compiler guessing where it should refuse.

> `@use` borrows a dialect (grammar floods). `$` binds by name (addresses
> resolve, and collide loudly).

## 7b. Publishing cells: `@exports` at file scope

A file declares which page-global cells it hands out with the **same clause, the
same `: mut` marker, and the same parser** a `@template` body already uses
(FEAT-115) — lifted to file scope. Knowing one tells you the other.

```st
/* config.st */
@exports { $host, $session: mut }

$host string: "https://api.example.com";   /* published, read-only */
$session string: "anon";                    /* published, writable by its owner */
$retryBudget number: 3;                     /* file-private */
```

```st
@use "./config.st" as app
.hero { text <- app/$host; }        /* ✓ published */
.hero { text <- app/$retryBudget; } /* ✗ E0944 — not published */
```

Rules:

- **Public by default** (the Odin floor). A file with no `@exports` publishes
  everything, so the check only engages once a module states an interface — which
  is what keeps all 430 existing `@import` sites working untouched.
- **The clause governs QUALIFIED access.** A file that flat-merges another with
  `@import` has taken those cells into its own scope and reads them bare, as its
  own.
- **Reads cross the boundary; writes do not** (E0942). Not ceremony: it is what
  keeps a cell's write-set settleable by a one-file scan, which is what makes
  const-folding sound.
- Implemented as a **stdlib `%macro exports`** (`stdlib/macros/presets.st`), not
  a Rust special case — the metasystem describes its own surface.

## 8. Diagnostics

| code | trigger | help |
|---|---|---|
| `E0938` | two files declare root-level `$host` | names both files; suggests rename |
| `E0939` | `1/$denominator` (tight numeric head) | "add spaces to divide" |
| `E0940` | `app/$host` with no `as app` | lists qualifiers in scope (E0926 mirror) |
| `E0941` | `app/$nope` — module has no such cell | lists the page-global cells |
| `E0942` | `app/$host <- v` (write through a qualifier) | names the owner; suggests `@emit` |
| `E0943` | `@b:badge` — colon where a slash belongs | "write `@b/badge`" (fixes BUG-232) |
| `E0944` | reading a cell absent from the owner's `@exports` | lists the published cells |

Existing `@`-registry diagnostics they mirror: E0924 (ambiguity), E0926
(unbound qualifier), E0927 (visibility).

## 9. What this buys

### 9.1 Introspection — one string, three surfaces

```
author writes      app/$host
registry key       local:config/host
inspect            spacetime inspect '$local:config/host'
                   spacetime inspect '$local:config/*'     → every cell in the module
                   spacetime inspect '@3d/*'               → every directive in the dialect
Spell CodePath     config.st::$host
```

No translation layer between what you write, what the compiler stores, and what
the tooling queries.

### 9.2 Optimisation — proven, not inferred

Because **there is no custom JavaScript** — no `eval`, no foreign call, no
escape hatch — a cell's write-set is **closed**. Write-freedom is *proven* by a
one-file scan, not guessed at. No `const` keyword, no `: mut` ceremony for the
common case:

```
$ spacetime inspect --layer state

local:config/$host      string   "https://api.example.com"
    readers  profile.st (2) · settings.st (1) · nav.st (1)     writers ∅
    verdict  const — folded at 4 sites, cell not emitted
local:config/$session   string   "anonymous"
    readers  profile.st (3)     writers  auth.st:41 (@on &.submit)
    verdict  reactive — cell emitted
local:config/$retryBudget  number  3
    readers  ∅     writers ∅
    verdict  dead — eliminated
```

One registry, three views: the diagnostic, the documentation, and the input to
the optimisation pass.

This is an optimisation a general-purpose language *cannot* make soundly. It is
available here only because the no-custom-JS rule closed the escape hatch.

**What ships today.** Dead-cell elimination is wired: a cell no one reads never
reaches the bundle. Const *folding* — substituting a write-free literal at its
read sites — is implemented and unit-tested but deliberately **not** wired: the
emitted binding is registered from the reference TOKEN, so replacing `$host`
with its literal removes what the binding pipeline keys on and the element
renders empty (measured: the headless matrix fell 22→16, taking every division
test with it). The `const` verdict still ships in `inspect --layer state`, and
the pass is ready for a binding-aware rework. An optimisation that renders the
wrong page is not an optimisation.

**Every reader channel counts.** A cell is read by a scope binding, a handler
body, *or a file-scope markup hole* (`` <li>`$x`</li> ``, FEAT-078). Holes live
on `html_blocks` — not in `matches` or `scopes` — and a pass that walked only
those two judged a hole-only cell dead and deleted a working page. A reader the
analysis cannot see is the one way elimination can go wrong.

## 10. Elegance-bar check

1. **Small extension of existing grammar** — one production, mirroring a loop
   twenty lines away in the same file. ✓
2. **Variants as data** — every registry resolves through the same
   `resolve_qualifier` / `Fqn` path; a fifth registry costs zero grammar. ✓
3. **Deletes a special case** — removes `$`'s exemption from
   duplicate-definition checking (E0938) and closes two silent-drop paths: the
   unresolvable qualified reference (E0940) and `@b:badge` (E0943 / BUG-232). ✓

   NB an earlier revision of this spec proposed deleting `%public` as a parallel
   export mechanism. It is **not** one, and it is enforced
   (`registry.rs::is_public`). The two clauses are different LAYERS, and the
   sigil says which: `%public` in a folder's `MODULE.st` governs the `@`/`%`
   VOCABULARY a module hands out; `@exports` in a file governs the `$` CELLS it
   publishes. Collapsing them would be the overload, not the cleanup.
4. **Tested by the machinery it generalises** — `.test.st` on the same headless
   runner as every other language feature. ✓

## 11. Conformance tests

| file | proves |
|---|---|
| `tests/lang/registry-addressing/slash-disambiguation.test.st` | the `/` token map: division, CSS values, qualification, the refusal |
| `tests/lang/registry-addressing/qualified-reference.test.st` | one alias serves every registry |
| `tests/lang/registry-addressing/state-modules.test.st` | cross-file cells, fold, eliminate |
| `tests/lang/registry_addressing_diagnostics.rs` | diagnostic codes and message quality |

## 12. Out of scope

- **`app/&rail`** (entity qualification) — the pattern permits it; no use case
  in hand. The grammar reserves the shape; resolution is unimplemented.
- **Remote writes** — parsed and refused, so the diagnostic can teach. A
  `: mut` capability ladder is a later decision.
- **`@import` deprecation** — `@import` keeps working untouched. 430 sites.
- **Selector-block state** — plural by construction (a selector matches 0..N
  elements), therefore never statically addressable. A documented boundary,
  not a gap: params in, events out.
