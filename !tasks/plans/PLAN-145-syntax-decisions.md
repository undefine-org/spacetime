# The four decisions, as concrete syntax

Each option below is written the way an author would actually type it, with what
the compiler would do in response. Evidence for every claim was gathered this
session; where it **overturned my previous analysis**, that is marked ⚠.

---

## ⚠ Two of the four decisions dissolved under evidence

Before the menus, the corrections — because they change what you are choosing
between.

**Decision 2 was wrong.** I told you `once:` "has no implementation anywhere"
and framed it as *refuse vs. build it*. That was false. `once` is **fully
implemented and working**:

- `stdlib/primitives/intersection.st:19` — `once: bool = false`, documented
  *"Disconnect after first intersection"*, with a real `observer.disconnect()`
  at `:73`.
- `@reveal` exposes it (`stdlib/macros/reveal.st:39`, `once: $once:bool = true`).
- Proven by emit diff — `@reveal(once: true)` vs `once: false` produce
  genuinely different guards:
  ```js
  if (revealed && true)  return;      // once: true
  if (revealed && false) return;      // once: false
  if (true && observer)  { ... }      // the disconnect path
  ```

So `once:` on `@on &.visible` is **not** a design question. It is the *same
routing defect as `from:`* — a working implementation the driver doesn't reach.
This collapses decision 2 into decision 1.

My error came from grepping `src/**/*.rs` and the driver registry, not
`stdlib/primitives/`. That is **exactly** the AUD-011 mistake, twice in one arc.

**Decision 1 is narrower than I implied.** I presented `from:` as "overloaded
between two drivers, pick a rename". But `target:` — the name I was treating as
an internal detail — appears **54 times** as a declared param and is written by
authors **zero** times. There is no author-facing collision to resolve; there is
an internal vocabulary that was never surfaced.

---

## Decision 1 — how an author names the delegation target

### Evidence

What authors actually write inside `@on(...)` today:

| param | uses |
|---|---|
| `name:` | 214 |
| `duration:` | 20 |
| `immediate:` | 5 |
| `threshold:`, `distance:`, `debounce:` | 1 each |

`from:` is written by an author **zero** times in `@on`. Its only registry use
is `text-change-driver`'s `param_map: "stagger:charStagger,from:staggerFrom"`,
where it means *stagger origin*. So `from:` for delegation is **not** an
established spelling being broken — it is a proposal from a bug report.

Meanwhile `target: selector` is declared 54× internally, including on the very
primitive that implements delegation, and is never typed by an author.

### Option A — surface the internal name

```spacetime
.list { @on &.click(target: ".item") { $n <- 1; } }
```

`param_map` is unnecessary; the author's word already equals the primitive's.
One less indirection.

✓ Zero new vocabulary — 54 existing declarations already use it.
✗ `target` is ambiguous in DOM English: `e.target` is *what was clicked*, but
  here it means *what to filter for*. Those coincide, which mitigates it.

### Option B — `matching:`

```spacetime
.list { @on &.click(matching: ".item") { $n <- 1; } }
```

Reads as what it does: react when the click matches `.item`. Aligns with the
repo principle *selectors are matching syntax, never addresses*, and with the
runtime, which literally calls `e.target.closest(sel)`.

✓ Unambiguous; a reader who has never seen it guesses right.
✗ One new author-facing word; needs `param_map: "matching:target"`.

### Option C — `within:`

```spacetime
.list { @on &.click(within: ".item") { $n <- 1; } }
```

✗ **Actively wrong.** `within` implies a container to scope to; the selector
  identifies the *innermost* element. Listed only to reject it explicitly.

### Option D — `from:` as originally proposed

```spacetime
.list { @on &.click(from: ".item") { $n <- 1; } }
```

✓ Matches the ticket.
✗ Collides with `from:` = stagger origin on `text-change`, and with `from:` as
  a *direction* in `@reveal` (`from: $from:string = "bottom"`). Three meanings
  for one word.
✗ Directly violates *"the language must be learnable by pattern — knowing one
  thing should tell you the next."*

> **My read: A or B, not D.** A is cheapest and already true internally; B reads
> best to someone who has never seen it. The `from:` spelling in the ticket is
> the one option I would actively argue against, because `@reveal(from: "bottom")`
> already teaches an author that `from:` means *direction*.

---

## Decision 2 — `once:` (now a routing question, not a design one)

Since `once` already works on `@reveal`, the only question is whether the
`visible` driver exposes it — and consistency says yes.

### Option A — route it, one data line

```spacetime
.a { @on &.visible(once: true) { opacity: 0 -> 1; } }
```

Add `once` to the visible driver's declared params. The intersection primitive
already implements the disconnect; nothing new is written.

✓ Makes `@on &.visible` and `@reveal` agree — an author who learned `once:` on
  one is right about the other. Exactly the learnability rule.
✓ No new concept, no new runtime code.

### Option B — refuse it, point at `@reveal`

```
error: `once` is not a parameter of driver `visible`
  hint: `@reveal(once: true)` provides one-shot reveals
```

✓ Honest and cheap.
✗ Hard to justify *now*: the parameter exists, the implementation exists, the
  driver is the only thing not passing it along. Refusing would be arbitrary.

### What defaults?

Note the two existing defaults **disagree**: `intersection.st` has
`once: bool = false`; `@reveal` overrides to `once = true`. For a driver, the
composable default (`false`) is the safer inherit — a reveal is opinionated
because a reveal is a finished behavior.

> **My read: A.** This stopped being a judgment call the moment the emit diff
> showed `once` driving a real `observer.disconnect()`.

---

## Decision 3 — what `✓` promises on a zero-page build

Today:

```
$ spacetime build .          # index.st has signals + styles, no markup
✓ Export complete:
  Pages: 0
```

### Option A — warn, keep exit 0

```
⚠ no page emitted: index.st declares no markup
✓ Export complete:
  Pages: 0
```

✓ Kills the false-negative trap (this is what fooled me twice).
✓ Breaks nothing — prelude-only and stylesheet-only projects still build.
✗ A warning people learn to scroll past.

### Option B — refuse when *every* input emitted zero pages

```
✗ Export produced no pages.
  index.st declares no markup — a page needs at least one element.
  (If this is a stylesheet-only module, it belongs in an @import, not a route.)
```

✓ Strongest guarantee: a green build always means something was produced.
✗ Breaks a legitimate layout: a project of pure `@import` modules with one entry
  page. Needs an opt-out, which is new surface.

### Option C — refuse per-file, not per-build

Any file *treated as a route* that emits nothing is an error; files reached only
by `@import` are exempt.

✓ Targets exactly the broken case — "this was supposed to be a page and isn't."
✓ No opt-out flag needed; the distinction is already known (route vs import).
✗ Needs certainty that route/import classification is reliable at that point.

> **My read: C if the classification is available, else A.** B's opt-out is new
> surface for a case C handles structurally. Worth 20 minutes checking C's
> feasibility before settling for A.

---

## Decision 4 — where the stdlib lives

### Evidence

`src/lsp/workspace/import_resolver.rs:210-218` builds the search roots:

```rust
if let Some(ref stdlib_path) = self.config.stdlib_path { roots.push(...) }
if let Ok(cwd) = std::env::current_dir() { roots.push(cwd.join("stdlib")) }
```

`config.stdlib_path`, then **`$PWD/stdlib`**. No toolchain-relative root at all.
Standing in the repo root works only because the repo root happens to contain
`stdlib/`. Confirmed live: the same file gives ✓ from the repo root and ✗ from
`/home/user`, with the error naming `/home/user/code/ora/stdlib/md.st` — a path
assembled from wherever the shell happened to be.

There is a real reason for the file-relative root, recorded in a comment right
above: *"a file on disk is the more specific answer and can always be followed;
the embedded name is the fallback."* That is BUG-340's filesystem-first
behavior, deliberate and worth preserving.

### Option A — toolchain-relative, with project override

Resolution order:
1. `<project root>/stdlib/` — a project may vendor its own
2. the toolchain's own stdlib (binary-relative, or embedded)
3. **never** `$PWD`

```spacetime
@import "stdlib/md"    # same answer from any directory
```

✓ Reproducible: same file + same binary = same verdict, always.
✓ Keeps vendoring, keeps filesystem-first within the project.
✗ "Toolchain's own" must be well-defined for `cargo run`, an installed binary,
  and the embedded fallback. This is the actual work.

### Option B — project-relative only

Resolve from the project root; drop `$PWD` and the toolchain root.

✓ Simplest rule, one sentence to document.
✗ Every project must vendor a stdlib, or the embedded fallback carries
  everything alone — which is what BUG-340 moved *away* from.

### Option C — keep `$PWD`, document it

✗ Listed for completeness. It makes any CI or editor verdict a function of the
  shell, and makes "you were in the wrong directory" look identical to a real
  compile error.

### Option D — explicit, no implicit root

```spacetime
@stdlib "../../stdlib"     # declared once per project
@import "stdlib/md"
```

✓ Zero magic.
✗ Boilerplate in every project for something that should have one obvious
  answer. Contradicts "learnable by pattern."

> **My read: A.** It is the only option where a verdict is reproducible, and it
> preserves both the vendoring escape hatch and BUG-340's filesystem-first
> intent. Reconcile with the concurrently-filed duplicate (BUG-350, *"the working
> directory decides which stdlib loads"*) before implementing, or it gets written
> twice.

---

## Summary

| # | decision | options | my read |
|---|---|---|---|
| 1 | delegation param name | `target:` · `matching:` · ~~`within:`~~ · ~~`from:`~~ | **A or B** — reject `from:`, it already means direction |
| 2 | `once:` on `visible` | route it · refuse it | **route** — ⚠ it is already implemented and working |
| 3 | zero-page build | warn · refuse-all · refuse-routes | **C**, fall back to **A** |
| 4 | stdlib location | toolchain+project · project-only · ~~cwd~~ · explicit | **A** |

Decision 2 is no longer really a decision. Decision 1 is the one where your
taste matters most — everything else has a defensible default.

The correction worth carrying forward: I have now twice concluded "not
implemented" from a grep that could not see `stdlib/`. In a two-language system
the only sound evidence is **building a page and diffing the emit**. Both
mistakes were caught by doing that.
