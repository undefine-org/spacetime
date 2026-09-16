# The eight open bugs, from zero context

Written to support a **design decision per bug**: is this a *syntax* question
(the language should say something different) or a *fix* (the language is right,
the implementation lags)? Each entry gives the mechanism, the evidence, and the
actual choice — not a recommendation dressed as a fact.

Every claim below was re-verified against `HEAD` this session. Where a ticket's
own description turned out to be stale, that is called out.

---

## The shape most of these share

Six of the eight are one sentence: **the compiler was handed something it did
not understand, and said nothing.**

That is worth stating before the individual bugs, because it changes what the
right fix looks like. A parser that *crashes* on bad input is annoying. A parser
that *accepts and discards* bad input is corrosive for a reason that compounds:

- The author gets a green build and a page that silently lacks the behavior
  they wrote.
- The next person to investigate cannot trust absence as evidence. I hit this
  twice in one session (see BUG-351) — I "confirmed" a bug by grepping output
  that was empty for an unrelated reason.

So for this class the question is rarely "how do we make it work". It is
**"what should the compiler SAY"** — and the answer is usually available for
free once one validation site exists.

---

## 1. BUG-334 — `from:` is parsed, accepted, and thrown away

### What an author writes

```spacetime
.list { @on &.click(from: ".item") { $n <- 1; } }
```

Intent: *event delegation*. Attach one listener to `.list`, but only react when
the click actually landed on a `.item` inside it. Standard technique — one
listener instead of N, and it keeps working when rows are added later.

### What happens

`spacetime check` prints `✓`. The page builds. The handler fires on **every**
click anywhere in `.list`, including the container itself. The `from: ".item"`
did nothing whatsoever.

### How I know it did nothing

Not from reading code — by building the page twice, once with the clause and
once without, and diffing the output:

```
diff plain.js delegated.js   →   IDENTICAL
```

Byte-for-byte. The clause is parsed, accepted, and discarded.

### The mechanism

This one is unusual, and it decides the answer: **everything needed already
exists except the connection.**

The primitive declares the parameter (`stdlib/primitives/animation/drivers.st:1971`):

```
%primitive on-mutation-handler(&el, event: ident, target: selector?, actions: expr)
```

The emitted JavaScript has a complete, correct delegation implementation:

```js
const delegateSelector = null;          // <- always null
const handler = function(e) {
  let $ = __stTargetEl;
  if (delegateSelector) { $ = e.target.closest(delegateSelector); ... }
```

The runtime is *finished*. It reads a variable that is hardcoded `null` because
the author's `from:` never reaches it.

The routing mechanism also already exists, as registry data — `param_map`
renames an author-facing param to the primitive's internal name. Exactly one
driver declares one (`stdlib/macros/drivers.st:562`):

```
param_map: "stagger:charStagger,from:staggerFrom"
```

That is `text-change-driver`, where `from:` means something entirely different
(stagger origin). The event drivers — click, submit, focus — declare no
`param_map` at all, so `from:` matches nothing and is dropped.

### Syntax or fix?

**Fix**, and a small one: add `param_map: "from:target"` to the event drivers.
One data line each, no Rust.

But there is a **real syntax question hiding underneath**, and it should be
decided deliberately rather than by default:

> `from:` currently means two different things on two different drivers —
> *delegation target* on click, *stagger origin* on text-change.

Options:

- **(a) Accept the overload.** They never co-occur; each driver documents its
  own params. Cheapest, and `param_map` exists precisely to let a driver name
  its own vocabulary.
- **(b) Rename the delegation one** to something unambiguous — `on:`, `within:`,
  `matching:`, `delegate:`. `matching: ".item"` reads closest to what it does,
  and matches the repo's stated principle that *selectors are matching syntax,
  never addresses*.
- **(c) Rename the stagger one**, since `from:` for "which end does the stagger
  start at" is the weaker claim on the word.

This is a genuine call and I would not make it unilaterally. Note the AGENTS
rule *"the language must be learnable by pattern — knowing one thing should tell
you the next"*: an author who learns `from:` on one driver and guesses wrong on
another is exactly the failure that rule exists to prevent. That argues for (b)
or (c) over (a).

### Verification bar

The emit diff must *stop* being identical (`const delegateSelector = ".item"`).
But that is necessary, not sufficient — this feature's whole job is to make a
click behave differently in a browser, so it needs a `--cdp` gate: click on
`.item` fires the handler, click on the bare container does not.

---

## 2. BUG-331 — `once: true` is accepted and ignored

### What an author writes

```spacetime
.a { @on &.visible(once: true) { opacity: 0 -> 1; } }
```

Intent: fade in when the element scrolls into view, and **stay** faded in. Don't
re-run every time it re-enters the viewport.

### What happens

`✓`. Builds fine. Re-animates on every re-entry.

Same proof as above: with and without `(once: true)` produce byte-identical
output.

### Why this one is NOT the same as BUG-334

> ⚠ **This section was WRONG and is corrected below.** It claimed there is no
> implementation. There is — see the correction. Left visible because the
> mistake is instructive: it is the AUD-011 error (a grep that could not see
> `stdlib/`) committed a second time in the same arc.

~~For `from:`, the implementation existed and the wiring was missing. Here there
is no implementation at all. No `once` parameter on any primitive, and no
one-shot path in the runtime.~~

**Correction.** `once` is fully implemented and working:

- `stdlib/primitives/intersection.st:19` — `once: bool = false`, documented
  *"Disconnect after first intersection"*, with a real `observer.disconnect()`
  at `:73`.
- `@reveal` already exposes it (`stdlib/macros/reveal.st:39`).
- Emit diff proves it drives real guards: `@reveal(once: true)` vs `once: false`
  differ at `if (revealed && true) return;` and `if (true && observer) { ... }`.

∴ `once:` on `@on &.visible` is **exactly** the same defect as `from:` — a
working implementation the driver does not route to. Not a design question.

Worth flagging a trap: `grep -c once dist/spacetime.js` returns **16**, and
`unobserve|disconnect` returns **4**. Reading those counts as "it's implemented"
would be wrong — every hit is runtime-library text (IntersectionObserver's own
API surface), not this feature. Only the A/B diff distinguishes them. This is
the same measurement error that produced two false conclusions elsewhere in this
session.

### Syntax or fix?

**Fix** — the same routing fix as BUG-334, now that the implementation is known
to exist.

Add `once` to the visible driver's declared params so it reaches the
intersection primitive that already honors it. No new runtime code.

The consistency argument is the strong one: `@reveal(once: true)` works today.
An author who learns `once:` there and tries it on `@on &.visible` is currently
punished with silence for a correct guess — the precise failure the
*learnable-by-pattern* rule exists to prevent.

One genuine sub-question: the two existing defaults **disagree**.
`intersection.st` has `once: bool = false`; `@reveal` overrides to `true`. For a
driver the composable default (`false`) is the safer inherit — `@reveal` is
opinionated because it is a finished behavior, not a building block.

Needs a `--cdp` gate: element scrolled in, out, and in again — animation fires
**exactly once**. Headless cannot prove this (no scroll model, BUG-305).

---

## 3. BUG-352 — the root both of the above grow from

### The mechanism

One site, `src/pipeline/drivers.rs:~2473`, binds a driver's named arguments:

```rust
match text_of(a.get("name")) {
    Some(name) => {
        let name = param_map.iter()
            .find(|(from, _)| *from == name)
            .map(|(_, to)| to.clone())
            .unwrap_or(name);          // unknown name passes through unchanged
        args.push(BindArg::Named { ... })
    }
```

`unwrap_or(name)` is the bug. If the registry has no mapping, the author's name
is used as-is; nothing downstream declares it; it evaporates. **Any** misspelled
or invented parameter on **any** of the 28 drivers is silently absorbed. `from:`
and `once:` are just the two that were noticed.

### Syntax or fix?

**Fix**, and it is the highest-value item here, because it converts an unbounded
silent class into a bounded loud one.

The shape matters: validate against the params the primitive *declares* (already
parsed — `typed_primitive_arg` consults them for typing). That keeps driver
variation as **registry data**, so a newly added driver gets validation for free
with no Rust arm per driver — which is the repo's stated elegance bar. And it
*deletes* the `unwrap_or` fall-through rather than adding a special case.

**Expect fallout, and budget for it.** This will light up currently-silent sites
across `projects/` and `demos/`. That is the fix working — every site it finds
is a page whose author wrote a behavior that was never happening. Triage is most
of the work: a typo gets corrected, a param that *should* exist gets a
`param_map` entry or a primitive param.

The deliverable is really the **diagnostic quality**, not the refusal:
`once` is not a parameter of driver `visible` — accepted: duration, delay …

---

## 4. BUG-351 — a page with no markup builds to nothing, and reports success

### What happens

```spacetime
$n number: 0;

body { color: red; }
```

```
$ spacetime build .
✓ Export complete:
  Pages: 0
  Files copied: 0
  Total size: 0 bytes
```

Exit 0. `dist/` created and empty. Add any markup element → `Pages: 1`.

### Why it is worse than it looks

It defeats **verification**, not just authoring. This is how I produced a
vacuous confirmation twice in one session: I built a styles-only repro, grepped
the empty `dist/`, found nothing, and read that as evidence the *feature* was
broken. The absence was my own empty build.

A tool whose failure mode is "silently produce nothing, print ✓" manufactures
false negatives in exactly the situations where someone is trying to prove
something is missing.

### Syntax or fix?

**A judgment call about what `✓` means.** It may be perfectly correct that a page
with no markup emits no page — but then it must *say so*. `Pages: 0` beside a
checkmark is not a statement, it is an omission.

- **(a) Warn**, naming the file: `no page emitted: index.st declares no markup`.
  Cheap, preserves behavior, kills the false-negative class. My default.
- **(b) Refuse** a build where every input produced zero pages. Stronger; would
  break legitimate stylesheet-only or prelude-only projects.
- **(c) Document as intended.** Weakest — the trap stays.

---

## 5. BUG-301 — `check` passes what `build` rejects

### What happens

The same file, the same binary, two verdicts — and **the permissive one is the
gate**:

```
$ spacetime check page.st     ✓
$ spacetime build page.st     error[E0926]: unbound import qualifier `typo` in `@typo/badge`
```

In the failing cases the module's CSS is silently absent — so `check` reports
success on a page that builds to nothing.

### Correction to the ticket

The ticket says `check` prints a clean `✓`. At `HEAD` it now prints a **warning**:

```
warning[W0714]: `@typo/badge` is not a known directive and was ignored
```

That is an improvement but **not a fix**, and the wording is telling: it
announces that the thing was *ignored*. `check` still exits successfully on a
page that `build` refuses with a hard error, and the two tools still disagree
about the same bytes.

### Root cause

`check_file` (`src/main.rs:1703`) never calls `CompileContext::with_imports`, so
`import_scope` stays empty — and the validation pass is gated on it being
non-empty (`src/pipeline/mod.rs:1487`):

```rust
if !context.import_scope.is_empty() {
    let import_diagnostics = check_import_visibility(...);
```

Import validation therefore never runs under `check`. Not a deliberate choice —
a code path that was never wired.

### Syntax or fix?

**Fix.** No language question at all. The design question is only the invariant
you want to commit to, and I'd argue for the strong one:

> `check` and `build` must never disagree about a file. If they can, `check` is
> not a gate — it is a suggestion.

Worth checking whether other validation passes are gated the same way, since the
same wiring omission could hide more than imports.

---

## 6. BUG-326 — the verdict depends on which directory you stand in

### What happens

Same file, same binary, three working directories, two different answers:

```
from repo root      ✓ passed
from the site dir   ✓ passed
from /home/user     ✗ Could not resolve import 'md'
```

The error names the search path, which is the whole story:

```
Searched: [".../scratch/q/site/stdlib/md.st",
           ".../scratch/q/site/stdlib/md/index.st",
           "/home/user/code/ora/stdlib/md.st",      <- cwd-relative
           "/home/user/code/ora/stdlib/md/index.st"]
```

It looks beside the *file*, then beside the **current working directory** — and
never at the toolchain's own stdlib. Standing in the repo root works only
because the repo root happens to contain `stdlib/`. The tool is finding its
standard library by coincidence of location.

### Why it matters beyond the annoyance

Any verdict — CI, editor, a subagent invoked from elsewhere — is only as
reproducible as the shell that produced it. It also silently poisons
investigation: a compile failure that is really "you were in the wrong
directory" reads exactly like a real defect.

### Syntax or fix?

**Fix**, with one design question: *what is the stdlib's identity?*

- **(a) Toolchain-relative** — resolve relative to the binary/installation. A
  standard library belongs to the tool, not the shell. Most predictable; matches
  how other toolchains behave.
- **(b) Project-relative** — resolve from the project root (the directory being
  built), never from cwd. Lets a project vendor its own stdlib.
- **(c) Both, ordered** — project override first, then toolchain fallback. Most
  flexible; the extra rule needs documenting.

Note a concurrent session filed the same finding under a different number
(BUG-350, *"the working directory decides which stdlib loads"*) — reconcile
before working either, or the fix gets written twice.

---

## 7. BUG-350 / BUG-307 — `@editable(bind:)` is not per-instance

The one failing v8 test, confirmed RED at `HEAD`:
`editable_bind_is_per_instance_across_two_compiled_instances`.

`bind:` emits the literal backtick hole **unlowered** under the raw `from_ast`
compile path — the placeholder is written out instead of being replaced with the
per-instance value, so two instances of the same component share one binding
rather than each getting their own.

Related to the class above (a parameter not properly consumed) but a **different
mechanism**: this is *lowering* — a template hole that should have been
substituted and wasn't — not *binding*. It will not be fixed by the validation
site.

**Fix, not syntax.** First step is confirming BUG-307 and BUG-350 are the same
defect (they read identically) and merging them, so it isn't fixed twice.

---

## 8. BUG-342 — a typed declaration with an object literal doesn't parse

### What happens

```spacetime
$u object: { name: "Ada" };
```
```
error[E0946]: directive does not match its declared grammar:
              Expected capture `$value:Expr` but no tokens remaining
```

### Narrowed to exactly one combination

| source | result |
|---|---|
| `$u object: { name: "Ada" };` | **E0946** |
| `$u Foo: { a: 1 };` | **E0946** |
| `$u: { name: "Ada" };` | OK — no type |
| `$u object: [1, 2];` | OK — type, non-brace value |
| `$u object: 3;` | OK |

So it is neither "braces" nor "types" but **the combination**. The type *name*
is irrelevant — a builtin (`object`) and a user type (`Foo`) fail identically,
which rules out any keyword or registry lookup.

### Ruled out — recorded so the ground is not re-walked

- `ExprExtractor` — tracks brace depth correctly; given the tokens it would
  handle `{ name: "Ada" }` fine.
- `TyperefExtractor` — bounded, stops at the colon.
- **The `balanced(';')` theory** — the ticket still reads as though this is the
  lead. It is not: tested with a full rebuild, no effect.
- **The CST is innocent.** I expected the culprit to be `parse_object_or_body`,
  which is a literal stub (*"For now, treat { } as a body"*). But the gate
  `a_typed_declaration_accepts_an_object_literal_value` **passes** on all three
  object shapes.

∴ the value is lost *between a correct CST and the form matcher*. That is the
remaining search space. Next probe: `inspect --layer parse` shows 0 nodes for
the failing file while `--layer registry` shows 8 — find which layer drops it.

(The `parse_object_or_body` stub is worth fixing on its own merits. It is not
this bug.)

### Syntax or fix?

**Fix** — the syntax is unambiguous and three of its four neighbours already
work; the asymmetry is not a design position anyone took.

Blocks the last 2 `migrations_test` failures. Hardest item here, and independent
of everything else — nothing else waits on it.

---

## 9. BUG-343 — scroll reveals flash partially-faded

A scroll-driven element does not hold its first keyframe at progress 0, so a
reveal shows mid-fade before settling. `--cdp` only, 7 failures, unchanged
throughout this arc.

**Fix.** The only item on this list a *visitor to a real page* can see. By user
impact it arguably outranks everything above; it is sequenced last only because
it is browser-investigation work with a different rhythm.

Must be **seen failing** under `--cdp` before the fix and passing after. A
headless run proves nothing about scroll (BUG-305) — and a `needs layout` claim
is *skipped* headlessly while the run still exits 0, which reads exactly like a
pass.

---

## Summary — what actually needs deciding

| bug | nature | your call |
|---|---|---|
| **BUG-352** unknown params absorbed | fix | none — but approve the fallout triage |
| **BUG-334** `from:` dropped | fix + **syntax** | is `from:` overloaded, or rename one? |
| **BUG-331** `once:` ignored | fix | none — ⚠ already implemented, just unrouted |
| **BUG-351** `Pages: 0` + ✓ | **judgment** | warn / refuse / document |
| **BUG-301** check ≠ build | fix | commit to "they must never disagree"? |
| **BUG-326** cwd decides stdlib | fix + **design** | toolchain-relative, project-relative, or both? |
| **BUG-350/307** `@editable(bind:)` | fix | none |
| **BUG-342** object literal | fix | none |
| **BUG-343** scroll flash | fix | take it first? |

Three genuine questions: what an author should CALL the delegation target, what
`✓` promises on an empty build, and where the stdlib lives. The rest is
implementation.

⚠ The fourth ("should `once:` exist?") dissolved: it exists, works, and is
exposed by `@reveal` — I had concluded otherwise from a grep that could not see
`stdlib/`, which is AUD-011's mistake made a second time. Options and evidence:
`PLAN-145-syntax-decisions.md`.
