# Junior-designer workflow

The default mental model for new website work. Optimized for early visual
feedback and fast iteration.

## Why "junior designer"

The disposition is intentionally junior: assume nothing about the brand,
explicitly placeholder the copy, and show progress to the user as soon as
the page compiles. Senior moves (custom motion, bespoke layouts, signature
type) come **after** the user signs off on direction.

## The loop

```
0. Read project AGENTS.md and existing index.st.
1. Run the project-style protocol; freeze tokens before authoring.
2. Position 4-questions (answer ALL before picking a showcase):
   - narrative-role: which @page role does this section play (hero / data / closer)?
   - viewer-distance: 10cm phone / 1m laptop / 10m presentation?
   - temperature: quiet / professional / loud / warm / playful?
   - capacity-sketch: does the layout fit the route's payload budget?
3. Sketch HTML structure with placeholder copy.
4. Stub data via @data { src: inline; value: [...] }.
5. Pick a showcase per section (see references/showcases.md).
6. Compile early: cargo run -- check projects/<name>/index.st.
7. Serve early: cargo run -- serve projects/<name>/ --debug.
   🛑 STOP-AND-WAIT: paste compile output + first-paint screenshot; wait for user direction before iterating.
8. Iterate visually. Refine copy and tokens last.
   🛑 STOP-AND-WAIT: paste route screenshots; wait for user before review pass.
9. Run review/doctor. Score >= 8 before declaring "ready for review".

## Placeholders are explicit

Never silently invent copy. Use bracketed placeholders that survive
review:

```st
&showcase-hero-minimal(
  "[Headline TBD]",
  "[One-sentence subhead — ask user]",
  "[Primary CTA label]"
);
```

When the user asks "where did this copy come from?" the answer is "I asked,
and the placeholder is still here." That is much better than "I made it up."

## Compile early, often, loud

After every edit:

```bash
cargo run -- check projects/<name>/index.st
```

A compile failure is a problem you can fix in 30 seconds. A compile failure
discovered after 20 minutes of more edits is a problem you spend 20 minutes
bisecting.

## Serve early

Before any styling work feels "done":

```bash
cargo run -- serve projects/<name>/
```

Open the dev panel, look at the page in actual pixels. AI tools talk
themselves into believing the markup is correct without ever rendering it.
Render it.

## Refining

Order of operations once the page renders:

1. Layout (boxes in the right places).
2. Type scale (sizes, weights, line-heights).
3. Color and contrast.
4. Motion (animations, transitions).
5. Copy.

Beginners reverse 1 and 5. Don't.

## Definition of done

- `cargo run -- check` passes.
- `Spacetime.review()` returns `score >= 8` on every route.
- `a11y` dim is **clean** (no `error`-severity findings, period).
- `cargo run -- doctor projects/<name>/` exits 0.
- Screenshots attached to the work item.
- Copy is signed off.
