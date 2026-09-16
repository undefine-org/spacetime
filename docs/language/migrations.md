# Syntax Migrations (`%migration`)

Spacetime evolves its authoring surface aggressively. A **syntax migration**
is how a retirement is *declared*: as stdlib **data**, not as a Rust
match-arm in the compiler. One `%migration` entry is a self-contained
**capsule**: it embeds the retired `%macro` definitions **verbatim** — the
old grammar *and* its `%binds` semantics — plus the rewrite rules and hints
that carry users across. Stdlib proper only ever carries the *new* syntax;
the capsule *is* the old syntax's only home. Delete the entry and the old
syntax ceases to exist.

```
old syntax keeps compiling (the embedded retired definition + a warning)
        │
        ▼
the pill shows pending waves (dev dock, next to the host pill)
        │
        ▼
Apply (pill or `spacetime migrate`) → files rewritten on disk
        │
        ▼
@version bumped → the wave is inert for this project forever
```

## The capsule

Migrations live in `stdlib/migrations/entries/<date>-<id>.st`:

```st
%migration reactive-surface {
  %date 2026-06-09
  %docs "FEAT-072: the paren-macro reactive surface (@bind/@show/@input) became `:` / `<-`."

  // The retired definition, VERBATIM (one %macro per retired directive).
  // Its %binds still route to their primitives, so the old syntax keeps
  // COMPILING through the window — not just parsing.
  %macro bind {
    %form { @bind(text: $text:expr? = undefined, /* … */) }
    %if $text {
      %binds { bind-text(&self, value: $text) }
    }
  }

  // Automatic rewrite rule: old shape → new syntax.
  %rewrite bind-text {
    %match {
      @bind(text: $x:expr)
    }
    %into {
      text <- `$x`;
    }
  }

  // …and where no mechanical rewrite is sound, guidance for the manual path.
  %hint for @show "Replace with a reactive class toggle — e.g. `.hidden: !$cond;`."
}
```

- **`%migration <id>`** — unique across all entries (duplicate = stdlib load
  error). One migration may retire **several** macros — a wave that removes
  a whole surface is ONE capsule, not seven fragments.
- **`%date YYYY-MM-DD`** — REQUIRED. The **wave key**: a wave is every
  migration sharing one date. Waves apply oldest-first. Entry filenames must
  start with the wave date (`2026-06-09-*.st`) so sorted load order matches
  chronological order.
- **`%docs "…"`** — one human line, shown in the pill, `check` output, and
  diagnostics.
- **`%macro <old>`** (≥1) — the retired definition, embedded verbatim. It
  registers as a *retired-tagged* macro (`<migration>#@<name>`), so retired
  source still parses into a `FormMatch` AND its `%binds` still expand —
  the old semantics remain executable exactly as they were.
- **`%rewrite <rule-id> { %match {…} %into {…} }`** — an automatic rule.
  `%match` is EXACTLY the [`%form` grammar](form-syntax.md) (same parser,
  same capture types); it registers as a match-only def (`<migration>#<rule-id>`)
  that outranks the embedded macro's broader form for its specific shape.
  `%into` is a source-text template: holes are **backtick-quoted** captures
  (`` `$x` `` — the [hole form](data-and-rendering.md); a bare `$x` is
  literal output). Holes splice the capture's text: string-literal captures
  splice unquoted (`@bind(attr: "src")` → `src <- …`); everything else
  splices its raw source slice, so formatting and expressions survive
  verbatim.
- **`%hint for @directive "…"`** — manual-migration guidance for one
  retired directive this capsule owns. A directive needs **either** a rule
  **or** a hint (coverage is validated at load — a silent retirement is a
  load error).

## Three compile-time behaviors (the tri-branch)

| the call… | compile-time | apply |
|---|---|---|
| matches a **rewrite rule** | **compat shim**: rewritten in memory, compiles, one W0715 warning | pill / `spacetime migrate` persists it |
| matches the embedded macro, **no rule covers its shape** (multi-shape calls) | hard error **E0910** with the entry's `%hint` | **manual** rows (no Apply) |
| matches the embedded macro, **hint-covered** directive (`@show`, `@input`) | **compiles through the window** — the retired `%binds` expand, one W0715 with the `%hint` | **manual** rows (no Apply) |

Soundness rule: a `%rewrite` only fires when the call's args are a
**subset** of the rule's `%match` params. A call carrying more (e.g. a
multi-shape `@bind(text:, class:)` against the single-shape rule) would
silently lose the rest — it degrades to E0910 with a "split or migrate
manually" note instead. The arg scanner is string- and comment-aware
(`//…`, `/* … */`), so it never fails open into a data-losing rewrite.

## `@version` — the project's syntax fact

```st
@version 2026-06-09;
```

Lives once in the root entry file (like `@deploy{}`). Means: *"this
project's source is written against the syntax as of this wave date."*

- waves **≤ `@version`** are **inert**: no shim, no pending. Lingering
  pre-wave syntax is then a **version inconsistency** (E0911), not a
  pending migration.
- waves **> `@version`** participate normally.
- No `@version` = date-zero: every wave participates (the bootstrap case).

Written by the toolchain, not by hand: the first persisted apply (pill or
CLI) **inserts** it after the leading comments; later applies **update** it
in place. The in-memory shim never writes it.

## The pill

Every dev-server page carries the **dev dock** (bottom-right): the
migrations pill sits left of the host pill, same glass chrome.

- **Hidden** when the project has zero pending entries.
- Click → panel: one row per pending wave (date, counts), an **Apply**
  button on waves with automatic rows, amber **manual** chips on hint-kind
  rows.
- Each entry expands (**old → new**): its rewrite rules (old shape in red,
  new template in green) and its hints — the same data `migrate --explain`
  prints.
- Apply → the server rewrites the files with the same engine, bumps
  `@version`, the file watcher reloads the page, and the pill disappears.

## The CLI

```sh
spacetime migrate <project-dir> [--wave YYYY-MM-DD] [--dry-run]
spacetime migrate <project-dir> --explain <migration-id>
spacetime migrate --scaffold <new-migration-id>
spacetime check <paths> [--at-version YYYY-MM-DD]
```

- `--dry-run` prints per-hunk previews and the would-be `@version` change;
  writes nothing.
- Waves apply oldest-first: `--wave` refuses to skip an older pending wave.
- After a successful apply the rerun is a no-op ("the project is current") —
  idempotence is structural (`@version` makes the wave inert).
- Only files under the project dir are touched; never `stdlib/`.
- **`--explain <id>`** prints the capsule's full contract — wave, docs,
  embedded retired macros, every rule (old → new), every hint — and counts
  the sites it still claims in your project.
- **`--scaffold <id>`** writes a new capsule skeleton at
  `stdlib/migrations/entries/<today>-<id>.st` (stdlib-authoring tool).
- **`check --at-version <date>`** behaves as if your `@version` were that
  date: retired syntax from waves at/before it reports as E0911 (*"this is
  what bumping would break"*); later waves report as pending.
- `spacetime check` prints a "N syntax migrations pending" note per file.

## Authoring a retirement (stdlib contributors)

1. Delete the old macro/form from its stdlib home.
2. `spacetime migrate --scaffold <id>`, then fill in the capsule: the
   retired `%macro` **verbatim** from where it lived (recover it from git:
   `git show <commit>^:<file>`), one `%rewrite` rule per mechanically
   rewritable shape, one `%hint` per directive that needs human judgment.
3. Registration is validated at load: ISO date; ≥1 embedded macro (unique
   names); unique rule ids; rule/hint directives ∈ embedded macros;
   template holes ⊆ `%match` captures; no self-cycles; forward-only chains
   (a rule may reference a **newer** wave's directive for A→B→C migrations,
   including same-capsule rule→rule hops); disjoint rule shapes within a
   wave; no collision with a live macro's directive.
4. Run `cargo test --lib` — migration tests ride the same machinery.

**Never** write a Rust check for a retired syntax. If the capsule grammar
can't express a retirement, extend the migration system (see
`!tasks/plans/PLAN-079-*`) rather than hardcoding a special case.

## Trade-offs (accepted, by design)

- Comments **inside** a matched span are consumed by the rewrite (the
  directive's own span is replaced). Surrounding trivia is untouched.
- The shim is **root-entry-only** for the in-memory compile path: a
  rule-covered shape in an `@import`-ed file errors (E0910) until
  `spacetime migrate` rewrites it on disk (the CLI/route scan all project
  files). Hint-covered directives (`@show`/`@input`) compile through
  everywhere — the embedded `%binds` expand regardless of file.
- A `%rewrite` template that produces non-parsing output is caught at the
  re-parse step and reported as a stdlib authoring error (E0912), never a
  silent bad splice.
- Two waves that both touch the same-named directive register distinct
  defs (`<mig-a>#@bind`, `<mig-b>#@bind`); the OLDER wave's def matches
  first. Re-retirement ergonomics (a wave retiring what an earlier wave's
  rewrite produced) are follow-up territory.
