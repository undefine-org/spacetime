# Fallback advisor — variant procedure

When the user's intent is ambiguous, do not extrapolate creatively. Recommend
three variants from three different design schools (see
[design-styles.md](design-styles.md)) and let the user pick.

## When to invoke

- User asks for a "hero" / "landing section" / "footer" without specifying style.
- User says "make it nicer" without naming a direction.
- You have two equally defensible options and no rule to break the tie.

If the user has explicitly named a school or style ("minimal please", "this
should feel editorial", "make it kinetic"), skip this; just pick the matching
showcase.

## The procedure

### 1. Identify the scenario

| User asks for                  | Scenario          |
|--------------------------------|-------------------|
| Top section / above-the-fold   | `hero`            |
| Top of page navigation         | `nav`             |
| List of capabilities / pillars | `features-grid`   |
| Closing call to action         | `cta`             |
| Bottom of page                 | `footer`          |
| Image / project list           | `gallery`         |
| Contact / signup               | `form`            |
| Section that animates in       | `reveal-on-scroll` |

## The 8-phase procedure

Translated from the huashu-design doctrine for typed-DSL projects.

### Phase 1 — Deep needs probe (1–3 questions max)

Ask the smallest set of questions that disambiguates. Do not interrogate.

> "Two questions before I sketch:
>  1. Who is the page for — first-time visitor, returning user, or
>     decision-maker scanning?
>  2. Should this section feel **quiet** (the offer is the hero) or **loud**
>     (the experience is the offer)?"

### Phase 2 — Restate the brief

Reflect what you heard back in 100–200 words. Close with: "I prepared 3
directions, each from a different design school. Tell me which feels right."

### Phase 3 — Recommend 3 schools (must come from different rows)

Pick three rows from the [5-school taxonomy](design-styles.md) and name one
representative philosophy per school. Do not recommend three minimalism-leaning
takes; that is not a fallback.

> "Three takes on the hero, each from a different school.
>  **Information-architecture** is data-forward and grid-driven (Pentagram lens).
>  **Motion-poetics** is dynamic and scroll-led (Field.io lens).
>  **Eastern-philosophy** is warm and contemplative (Kenya Hara lens).
>  Which direction?"

### Phase 4 — Show prebuilt showcase gallery

Pull each recommended school's showcase under
`stdlib/showcases/<scenario>/<school>.st` and surface the 1-paragraph
`/// useWhen:` from each file's doc-comment header.

### Phase 5 — Render 3 demos

Render side-by-side. When subagent fan-out is available (see [Parallel
rendering](#parallel-rendering-when-available) below), launch three tasks in
parallel; otherwise fall back to sequential authoring (see [Sequential
fallback](#sequential-fallback)).

### Phase 6 — User picks / mixes / refines

The user replies with one of:
- "School A, ship it."
- "School A's layout but School B's color temperature."
- "None of these — try something else."

Answer "none of these" by re-running Phase 1 with one tighter question.
Do NOT speculate.

### Phase 7 — Generate authoring prompt structure

Translate the picked variant into the project's authoring shape:
- Promote the chosen `.st` into `projects/<name>/sections/`.
- Derive its tokens against the project's `brand-spec.md`.
- Update `projects/<name>/index.st` to import the promoted section.

### Phase 8 — Hand off to junior-designer workflow

Re-enter [junior-designer-workflow.md](junior-designer-workflow.md) at
step 5 (compile early). The 🛑 STOP-AND-WAIT checkpoints from the junior
flow apply to the chosen direction, not the discarded variants.

## The 5-school taxonomy (short table)

| School                   | Visual signature                                |
|--------------------------|-------------------------------------------------|
| information-architecture | Grid-driven, data-forward, restrained           |
| motion-poetics           | Dynamic, immersive, scroll-driven               |
| minimalism               | Order, white space, precision                   |
| experimental             | Avant-garde, generative, visual disruption      |
| eastern-philosophy       | Warm, poetic, contemplative, asymmetric balance |

Full anchors and "use when" guidance: [design-styles.md](design-styles.md).

## Diversity rule

The three schools recommended in Phase 3 MUST come from three different rows
above. Recommending three minimalism-leaning takes defeats the purpose;
the user can't make a real comparison.

When the project's brand-spec.md already constrains direction (e.g. "must
read as quiet, premium, trustworthy"), the diversity rule still applies but
within the *adjacent* schools (e.g. information-architecture + minimalism +
eastern-philosophy — all three "quiet" lineages, but visually distinct).

## Parallel rendering when available

When subagent fan-out is wired (Spell `task` tool or equivalent), prefer
parallel authoring:

```
task: render hero/architecture.st
  -> writes scratch/architecture.st  (information-architecture school)
  -> compiles standalone

task: render hero/motion.st
  -> writes scratch/motion.st  (motion-poetics school)
  -> compiles standalone

task: render hero/eastern.st
  -> writes scratch/eastern.st  (eastern-philosophy school)
  -> compiles standalone
```

The three `scratch/<school>.st` files have no DAG dependency and write to
disjoint paths, so the tasks are safe to dispatch concurrently.

## Sequential fallback

When fan-out is unavailable (single-agent execution), author the three
variants sequentially in the same session:

```bash
mkdir -p projects/<name>/scratch
cp stdlib/showcases/hero/architecture.st projects/<name>/scratch/
cp stdlib/showcases/hero/motion.st       projects/<name>/scratch/
cp stdlib/showcases/hero/eastern.st      projects/<name>/scratch/
cargo run -- check projects/<name>/scratch/
```

Each variant compiles independently, so the three sibling files can render
in one `cargo run -- serve` session.

## Why parallel variants beat sequential iteration

You will ALWAYS be wrong about taste at first. Sequential iteration costs N
rounds of "almost, but". Parallel variants cost one round.

## Why showcases, not LLM creativity

The showcase set is curated and review-passing. LLM-generated layouts in this
context tend to:

- Hallucinate spacing tokens that don't match the project,
- Over-animate (every section "reveals on scroll"),
- Reach for unique-looking layouts that feel familiar elsewhere on the web.

The showcases are intentionally generic — the *brand* is supplied by the
project's tokens, not by the showcase's geometry.
