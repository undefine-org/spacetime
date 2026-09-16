---
name: spacetime-design
description: Design, build, debug, and verify Spacetime websites end-to-end. Use this skill for any work in `.st` files or `projects/<name>/` — building from scratch, adding features, debugging compiler/runtime issues, performing review-grade audits, and gating delivery via `cargo run -- doctor`.
---

# Spacetime Design

Procedural skill for the Spacetime DSL: how to think about CSS-native websites, how to debug them when they break, and how to ship them with confidence.

## Status

What works today:
- `Spacetime.review()` runtime API — open devtools in a `cargo run -- serve <project> --debug` session, run `Spacetime.review()` to get the live audit. See [references/review-rubric.md](references/review-rubric.md).
- Dev-panel "Review" tab — visual surface for `Spacetime.review()`: per-dim summary, severity-coded findings, selector chips that highlight host elements, "Copy report" button. Visible under Dashboard → Review.
- `cargo run -- doctor <project>` — runs compile-time `check` (parser + import resolution + validator + `brand-asset-missing` warns), then drives the headless route loop (Chromium via Playwright) per route to evaluate `Spacetime.review()`, capture console errors, and optionally save screenshots. Output: `--format pretty` (per-route summary) or `--format json` (`===JSON===` prefix + serialized `DoctorReport`). Pass/fail = no compile errors AND every route's score ≥ 6 AND zero console errors AND no non-benign skipped routes. Build with `--features browser` to enable headless; without the feature, routes return a benign `"browser feature not enabled"` skip that doesn't gate. Implementation: [[id:FEAT-020-cargo-run-doctor-cli-one-shot-project-au]] (check phase) + [[id:FEAT-033-cargo-run-doctor-headless-route-loop-wit]] (route loop, absorbed FUP-016).
- Showcase tree at `stdlib/showcases/` — 10 of 24 patterns shipped across 5 design schools (information-architecture / motion-poetics / minimalism / experimental / eastern-philosophy). See [references/showcases.md](references/showcases.md).
- This skill's reference files — procedural, browseable.

Planned (not yet runnable):
- (none in this slot — the headless route loop landed via [[id:FEAT-033-cargo-run-doctor-headless-route-loop-wit]] in PLAN-020).
- Dev-panel "Brand" tab — inventory + promote-to-token flow. Tracked by [[id:FEAT-017-dev-panel-brand-tab-css-var-inventory-li]].
- `cargo run -- showcase` CLI — catalog + variant generation. Tracked by [[id:FEAT-019-cargo-run-showcase-cli-gallery-variant-g]].
- Remaining showcase patterns. Tracked by [[id:FUP-013-feat-018-remaining-17-showcases-8-scenar]].
- Structural regex-literal-aware lexer fix for `%emit js` bodies. Tracked by [[id:FUP-017-bug-022-structural-fix-regex-literal-awa]] (BUG-022 closed with loud-failure diagnostic; structural fix tracks here).

Hard Rule #4 ("every delivered website MUST pass `cargo run -- doctor`") is enforced by `cargo run -- doctor`'s check phase + the headless route loop ([[id:FEAT-033-cargo-run-doctor-headless-route-loop-wit]]); the dev-panel Review tab provides the same `Spacetime.review()` audit interactively during authoring.

## Hard rules (these never bend)

1. **No JavaScript outside `%primitive` and `@test` bodies.** Inline event handlers (`onclick=`), hand-rolled `<script>` tags, and ad-hoc DOM manipulation are antipatterns. Express behavior through Spacetime macros (`@on`, `@bind`, `@fade-in`, ...). See [references/anti-patterns.md](references/anti-patterns.md).
2. **No bespoke Rust parsers/fallbacks for stdlib constructs.** If a stdlib pattern won't load, fix the loading mechanism — never reimplement the construct in `src/`. The metasystem is self-describing.
3. **No invented brand colors.** Read the project's `AGENTS.md` and existing `.st`/`.css`. Tokens come from authoritative sources, not from your taste. See [references/project-style-protocol.md](references/project-style-protocol.md).
4. **Every delivered website MUST pass `cargo run -- doctor <project>`.** No exceptions.
5. **No HTML comments, no Unicode/emoji, no `@each` nested in `@template` body.** Parser limitations; respect them.

## Procedures

### Junior-designer workflow (default for new website work)

```
0. Read project's AGENTS.md (if present) and existing index.st.
1. Run project-style protocol; freeze tokens before authoring.
2. Position 4-questions (answer ALL before picking a showcase):
   - narrative-role: which @page role does this section play (hero / data / closer)?
   - viewer-distance: 10cm phone / 1m laptop / 10m presentation?
   - temperature: quiet / professional / loud / warm / playful?
   - capacity-sketch: does the layout fit the route's payload budget at expected viewport sizes?
3. Sketch HTML structure with placeholder copy and stub data via @data.
4. Pick a showcase from stdlib/showcases/ matching scenario × school (see references/showcases.md).
5. Compile early (cargo run -- check), serve early (cargo run -- serve --debug).
   STOP-AND-WAIT: paste compile output + screenshot; wait for user before iterating.
6. Iterate visually. Refine copy and tokens last.
   STOP-AND-WAIT: paste route screenshots; wait for user direction before review pass.
7. Run review/doctor; fix findings; repeat until score >= 8.
```

Full procedure: [references/junior-designer-workflow.md](references/junior-designer-workflow.md).

### Project-style protocol (extracting brand)

7-step extraction with asset-tier inventory; never invent a logo, color, font, or radius before running this:

```
0. Inventory project assets (logo, hero imagery, UI screenshots). Halt if logo missing.
1. Read AGENTS.md, README, /docs/ for explicit declarations.
2. Read the project's existing .st files for @apply / var(--*) usages.
3. grep -rn "var(--" projects/<name>/  ->  inventory of tokens.
4. grep -rn "#[0-9a-fA-F]" projects/<name>/  ->  unbound literals to promote.
5. Freeze tokens.st (or equivalent) before authoring sections.
6. Compose brand-spec.md recording asset paths + token references.
```

Full procedure: [references/project-style-protocol.md](references/project-style-protocol.md).

### Fallback advisor (when ambiguous)

When the user's intent is ambiguous, do NOT extrapolate creatively. Recommend three variants from three different design schools (information-architecture / motion-poetics / minimalism / experimental / eastern-philosophy) and let the user pick.

```
1. Phase 1 — Deep needs probe (1–3 questions max).
2. Phase 2 — Restate the brief; close with "I prepared 3 directions, each from a different school."
3. Phase 3 — Recommend 3 schools (must come from different rows of the 5-school taxonomy).
4. Phase 4 — Show prebuilt showcase gallery for each recommended school.
5. Phase 5 — Render 3 demos (parallel via subagent fan-out where available; sequential fallback otherwise).
6. Phase 6 — User picks / mixes / refines.
7. Phase 7 — Generate authoring prompt + promote chosen variant.
8. Phase 8 — Hand off to junior-designer workflow at compile-early step.
```

5-school taxonomy: see [references/design-styles.md](references/design-styles.md). Diversity rule: three schools recommended MUST come from three different rows.

Full procedure: [references/fallback-advisor.md](references/fallback-advisor.md).

### Pre-delivery doctor (gate before declaring "done")

Before ANY website work is delivered:

```
cargo run -- doctor projects/<name>/ --format pretty
```

Resolve every finding or document an explicit FUP. Pass = score >= 6 on every route AND zero compile-error diagnostics AND zero console errors during navigation.

## 7-dimension review

`Spacetime.review()` (dev runtime) returns a structured audit across:

| Dim        | What it scans                                                                  |
|------------|--------------------------------------------------------------------------------|
| idiomatic  | inline handlers, foreign `<script>` tags, hex in `style=` attrs                |
| animation  | timelines: 0-element, never-started, destroyed-incomplete                      |
| brand      | hex/rgb/hsl literals NOT wrapped in `var(--*)`                                 |
| a11y       | img alt, link/button accessible name, heading order                            |
| compile    | `__ST_DEBUG__.errors` from the dev runtime                                      |
| hierarchy  | heading order (h1→h2→h3) + font-size monotonicity                              |
| detail     | text-wrap on display headings, oklch on `--brand-*` tokens, var() coverage     |

Per-dim score: `clamp(10 - errors*2 - warns, 0, 10)`. Top score = min of dim scores.

Full rubric (when to defer a finding, severity guidance, score thresholds): [references/review-rubric.md](references/review-rubric.md).

## When debugging — pick the right path first

```
runtime symptom (animation, CSS, console)?  ->  references/runtime-debugging.md
compile-time symptom (parse error, macro)?  ->  references/compiler-debugging.md
neither -- agent confused?                  ->  reread project AGENTS.md
```

## References (load on demand)

- [references/runtime-debugging.md](references/runtime-debugging.md) — `Spacetime.diagnose`, `inspectTimeline`, `inspectElement`, `forcePlay`, console patterns.
- [references/compiler-debugging.md](references/compiler-debugging.md) — `cargo run -- inspect <layer>`, `RUST_LOG` modules, parse/expansion/registry/IR/emit pipeline.
- [references/anti-patterns.md](references/anti-patterns.md) — the canonical Spacetime-specific slop list, structural + visual.
- [references/showcases.md](references/showcases.md) — "I need X → file path + macro name."
- [references/review-rubric.md](references/review-rubric.md) — dimension scoring + severity guidance.
- [references/project-style-protocol.md](references/project-style-protocol.md) — brand-asset extraction procedure.
- [references/junior-designer-workflow.md](references/junior-designer-workflow.md) — assumptions+placeholders, compile early, iterate, STOP-AND-WAIT checkpoints.
- [references/fallback-advisor.md](references/fallback-advisor.md) — variant procedure for ambiguous asks.
- [references/design-styles.md](references/design-styles.md) — 5-school taxonomy of design philosophies (information-architecture / motion-poetics / minimalism / experimental / eastern-philosophy).

## File anchors (this skill assumes these locations)

- Runtime audit primitive: `stdlib/__dev__/primitives/dev-review.st` (FEAT-015)
- Showcase tree: `stdlib/showcases/` (FEAT-018)
- Showcase CLI: `src/cli/showcase.rs` (FEAT-019, planned)
- Doctor CLI: `src/cli/doctor.rs` (FEAT-020 DONE; headless loop FEAT-033 in flight)
- Compiler dev runtime handler: `src/server.rs::dev_runtime_js_handler`
- Antipatterns canonical doc: `docs/antipatterns.md`
