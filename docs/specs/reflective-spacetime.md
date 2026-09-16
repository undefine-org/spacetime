# Reflective Spacetime — the self-introspecting, self-improving tower

> Status: **vision** (the "why"). 2026-06-14.
> Grounds the substrate spec `docs/specs/declared-effects.md` (the "build now")
> and the module system `docs/specs/module-system.md` (the first rung).
> Companion: `docs/specs/spell-spacetime-unification.md` (the Spell edge, R10).
>
> This doc names a system that already exists in scattered corners of verse and
> records the discipline that makes it SOUND rather than a foot-gun. It is a map,
> not a mandate: only the bottom rung (R1, the declared-effect substrate) is
> built now. R2–R10 are recorded as *reachable* rungs the substrate unlocks.

---

## 0. Thesis

Spacetime is, latently, a **reflective tower**: a system that (1) reifies its own
structure as data, (2) reads that data from within itself, (3) modifies itself
through *declared effects*, and (4) verifies each modification through the same
machinery it uses for ordinary code — with the agent (Spell/MCP) as the outermost
ring closing a self-improvement loop. Every one of these capabilities already
exists in some corner of the codebase. They have never been named as one system.
Naming them is the unlock: `@import`, `@use`, namespace-attach, `%using`,
metaforms, the dispatch playground, and the Spell/MCP loop stop being seven
features and become seven rungs of one tower.

The leaf that surfaced this — "a `%form` can manipulate the registry" — is one
instance of the general shape. The general shape is the subject of this doc.

---

## 1. The five rows (capabilities) and where verse already reaches

```
reify      structure-as-data    → registry, %form, %capture_type            [HAVE]
reflect    read self in-language → inspect, @dispatch-probe, relationships   [PARTIAL]
re-effect  modify self           → %registers, %resolves, (%imports/M1)      [SEED]
re-verify  close the loop         → @property/@fuzz + the test machinery       [HAVE, unaimed]
re-operate agent drives it        → MCP st_fn_put/st_mount/st_await (regions)    [HAVE]
```

Nothing here is hypothetical infrastructure — it is assembly of parts that exist.

---

## 2. The tower (levels)

```
L4  agent / Spell / MCP        reflective OPERATOR — reads L0–L3, proposes effects   [pages only]
L3  the grammar of the grammar  %capture_type, %form-of-%form (self-describing)        [aspired]
L2  the metasystem              %macro %primitive %form — defines @directives           [HAVE]
L1  effects on L2 at compile     %registers %resolves %imports %installs %using          [SEED]
L0  a .st site                  HTML / CSS / state                                       [HAVE]
ground  the Rust kernel          form-matcher, registry, fs/merge primitives             [HAVE, thick]
```

`standards.org`'s north star — "%form is the unique reference of the syntax that
defines it" — is a claim about a **reflective fixed point** (L3 defining itself).
Verse does not reach it; there is a Rust floor (matcher, embedded-stdlib
`include_str!` table, the `ImportAst` special-case). **A floor is not a failure** —
every reflective tower has a ground (3-Lisp's ground interpreter; Maru/COLA's
~200-line kernel). The discipline is to keep the floor **thin and honest**, not to
eliminate it. `%emit js` is the sanctioned RUNTIME escape hatch; the
effect-enactors (R1) are its COMPILE-TIME twin.

---

## 3. The six soundness properties (the load-bearing discipline)

Recursive self-modification is a graveyard of unhygienic macros, phase cycles,
and "the optimizer rewrote the compiler into garbage." Six properties separate a
sound reflective system from a foot-gun. **These are invariants, not features.**

| # | Property | Have? | Discipline |
|---|---|---|---|
| 1 | **Reified** — structure is data | ✓ registry | never Rust-special-case a construct that could be a form |
| 2 | **Reflective** — data queryable *in-language* | ½ | a SEPARATE reflection vocabulary (`@registry`, mirrors), not meta-methods on base forms (Bracha's Mirrors) |
| 3 | **Declared** — self-mod via *declared effects*, not imperative mutation | seed/M1 | a form DECLARES what it does to the registry (data); a fixed phase decides when/how. Analyzable without running. **The keystone.** |
| 4 | **Stratified** — effects live in well-founded phases | ✗ | metaform@N installs form@N−1; strata declared, tower well-founded, or self-install is nondeterministic |
| 5 | **Grounded** — minimal kernel floor | thick | shrink the Rust floor over time; enactors are the `%emit js`-tier sanctioned primitive set |
| 6 | **Verified** — every self-mod closes through the SAME test/type machinery | ✓ unaimed | the elegance bar's condition 4, promoted to a LOOP INVARIANT. The agent proposes; the KERNEL verifies. |

**Property 3 is the keystone** (the insight that generated this doc): declared
effects = hygienic reflection. The imperative alternative ("a form runs code that
pokes the registry") is Racket/Lisp phase-hell — import order becomes load-bearing
and unanalyzable; you cannot cache, reorder, or know what a file *means* without
*running* it. A design language must never enter there.

**Property 4 is the deep constraint** the substrate spec must invent: the
"ordering rules" for `%imports` are secretly a **phase tower**. (NB: distinct from
the existing runtime `Phase{Global,Selector}` in `pipeline/types.rs` — that is
emission phase. Strata are a metasystem LOAD/EFFECT concept.)

**Property 6 is the safety rail for R7+**: a self-improving system with no oracle
optimizes itself into incoherence. Every self-mod must close through the same
`@test`/`@property`/type machinery ordinary code uses. The agent is never trusted
— only its verified output.

---

## 4. The rungs (breadth: near → far)

**Near — built on what exists (+ M1):**
- **R1 · Declared-effect clause class** (`%imports` → `%installs` → `%reflects`).
  The substrate. Migrates `@import` off its Rust special-case → DELETES code
  (elegance bar ✓). Spec: `declared-effects.md`. **Built in M1.**
- **R2 · Reflection as a directive.** Generalize `@dispatch-probe` (already asks
  the compiler's own scorer live) into `@registry(category:"fn")`,
  `@forms-matching(...)`. Introspection becomes a language primitive: a page can
  render documentation of itself; dev tools become `.st`, not Rust.
- **R3 · Metaforms (`%installs`).** A form whose effect is *registering new
  forms*. The hand-written `morph-to-{color,radius,intensity,falloff,glow,click}`
  family (≈8 near-duplicates in `stdlib/macros/scene/morph-to.st`) collapses to
  ONE `%installs` metaform. **Self-improvement = the system generating its own
  boilerplate.** First rung where Spacetime visibly improves itself.
- **R4 · Provenance everywhere.** (M0 already fixed embedded `source_file`.)
  Generalize: every registry entry, dispatch decision, and emit carries a
  causality trace. `@dispatch-probe` already shows "why losers lost." Provenance
  is the substrate for BOTH debugging and the agent loop (no fix without a why).

**Mid:**
- **R5 · Phase stratification.** Make property 4 explicit. The honest version of
  "forms install forms."
- **R6 · Self-test generation.** Aim `@property`/`@fuzz` at the metasystem: a
  `%form`'s grammar → auto-generated parser fuzz cases → the system tests its own
  grammar. Self-verification as a rung.
- **R7 · The MCP loop for the metasystem.** `st_fn_put`/`st_mount`/`st_await`
  already closes a self-improvement loop for PAGES (compile a function, mount it
  live, await its events). Lift to FORMS: agent reads
  registry (R2) → detects gap/conflict (R4) → proposes a `%form` (R1) → kernel
  compiles + tests (property 6) → applies. Spacetime improving Spacetime, agent
  as the reflection mechanism.

**Far — vision:**
- **R8 · Reflective fixed point.** Thin the Rust floor until `%form` is defined by
  a `%form` (L3 self-hosting). Maru/PyPy lesson: minimal metacircular kernel,
  bootstrap up.
- **R9 · Partial evaluation / self-optimization.** Futamura: specialize a hot
  `%macro` against common args → a faster specialized form the system installs
  itself. The tower collapses its own interpretive overhead.
- **R10 · Mutual reflection, Spell ⟷ Spacetime.** The unification work IS a
  reflective edge: Spacetime emits itself as data → Spell consumes it → Spell
  builds tools FOR Spacetime → those tools are `.st` (Canvas-on-Spacetime) →
  Spacetime renders them → they introspect Spacetime. Productive ouroboros.
  Concretely: `&inspector(registry)` — a visual form-editor, authored in
  Spacetime, rendered by Spacetime, that edits Spacetime.

---

## 5. Why this is unique to Spacetime (not "another reflective Lisp")

Spacetime is a **design/markup** system, so the tower is concrete and VISUAL:
- The metasystem IS the design system; a `%form` family IS a component library.
  Self-improvement = a component library that refactors itself (R3).
- Reflection RENDERS. `inspect registry` → JSON is the dead version. The live
  version is a Spacetime page that reads its own registry and renders it — which
  verse already does in `stdlib/__admin__`, the dispatch playground, the
  `stdlib/__mcp__` picker. The tower's mirror is a webpage.
- The tool that improves the system is built IN the system and rendered BY the
  system. No other reflective language gets this free, because none render. **A
  self-improving design system whose improvement UI is itself a design artifact.**
  This is exactly where the Spell-Canvas-on-Spacetime vision meets the tower:
  `&inspector(registry)`.

---

## 6. Foot-guns (the honest dangers)

1. **Unstratified self-install → nondeterminism.** Mitigation: property 4 is
   non-negotiable — ship `%installs` only with declared strata.
2. **Self-improvement without an oracle → drift.** Mitigation: property 6 as loop
   invariant — kernel verifies, agent only proposes.
3. **Reflection leaking encapsulation** (Bracha, Mirrors). Mitigation: reflection
   is a separate vocabulary, not a method on every form.
4. **Tower performance.** Mitigation: R9 collapses towers; `IncrementalStdlibCache`
   is the seed of self-maintenance.
5. **Over-reflection / reflective astronomy.** Mitigation: the elegance bar's
   "must DELETE a special-case" guards against metaforms that don't earn their
   place (a metaform that doesn't collapse hand-written repetition like R3's
   morph-to family does not ship).

**Challenge to the author (recorded):** this is a LANGUAGE-IDENTITY decision, not
a feature — it changes Spacetime from "a self-describing compiler" to "a
self-improving one," a bigger commitment than the module system. The risk is
SCOPE-GRAVITY: the tower is generative enough that M1 (a module system) could
balloon into "build the reflective kernel." **Resisted by design:** M1 builds only
R1 (the substrate), correctly, because it is load-bearing for everything above and
cheap to get right at the foundation. R2–R10 are named, not built.

---

## 7. Consequence for M1 (no scope expansion, better foundation)

M1 still ships the module system. But its mechanism is now **rung R1**: a
declared-effect clause class (`%imports`), one enactor phase, `@import` migrated
onto it (parity-tested), `@use` as more clauses. The discipline (declared /
stratified / verified) is set ONCE, at the foundation, where it is cheap.
Retrofitting it onto an imperative import path later would be the expensive
mistake. See `declared-effects.md` for the buildable substrate.
