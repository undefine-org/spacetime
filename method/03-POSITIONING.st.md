# 03 · Deriving the brief

Record → brief. This stage is **derivation with stated rules**, not judgement
with good taste. An agent running these rules on the same record twice should
land in the same place.

```st hidden
@import "./reader.st"
```

Output: `<project>/method-record/brief.json` + a short prose brief.


## D1 — The frame comes from alternatives, not features

Dunford's causal order ([S15](/08-SOURCES)), and it runs one way only:

```
alternatives ──▶ what we have that they don't ──▶ so-what value
             ──▶ who cares most ──▶ category frame
```

Starting anywhere else produces a brief that describes the product instead of
locating it. Concretely, from `alternatives.json`:

1. **List the real set.** Manual work, chat threads, one person's memory,
   spreadsheets, incumbents, doing nothing. Exclude phantom competitors that
   never appear in a real decision.
2. **For each: what can we do that it can't?** Discard table stakes — anything
   every alternative also has is not a differentiator, however much work it took.
3. **Translate each survivor into consequence.** *So what?* → time, money, risk,
   control, capacity, dignity. Stop when you reach something the buyer would say
   out loud.
4. **Who cares most about that consequence?** → cross-check `segments.json`.
5. **What frame makes the value obvious** without importing false expectations?

**Category choice.** Prefer an existing category the buyer already understands.
Inventing one imposes an education cost the buyer pays before they can evaluate
you ([S15](/08-SOURCES) — Dunford's explicit warning; [S19](/08-SOURCES) —
category-design canon, and note its promotional framing). Invent only when every
existing frame actively lies about what the product is.


## D2 — The axes are read per segment, not per product

From each `segments.json` entry, two independent values
([S14](/08-SOURCES)). Do not collapse them into "cold/warm/hot" — that
shorthand destroys the unaware/problem-aware distinction, which is the most
consequential one on the page.

**Awareness → what the headline must DO:**

Columns: stage · the visitor · headline's job.

- **unaware** — *doesn't see a problem; this is "just how it is"* → make the normal look costly — scene, anomaly, consequence. **Never open with product.**
- **problem-aware** — *feels the pain, no category for it* → name the pain precisely; reveal a class of solution exists
- **solution-aware** — *knows solutions exist, not you* → lead with **mechanism** — the how that others can't claim
- **product-aware** — *knows you, unconvinced* → proof, differentiation, objection, risk reversal
- **most-aware** — *convinced, needs a nudge* → the offer. Extra education is friction

**Sophistication → what the headline may CLAIM:**

Columns: stage · claim-space · move.

- **1** — *fresh* → state the benefit plainly
- **2** — *benefit repeated* → enlarge — more specific, vivid, measurable
- **3** — *claims disbelieved* → **introduce the mechanism**
- **4** — *mechanisms competing* → deepen/demonstrate the mechanism
- **5** — *saturated* → attach to identity — requires a *real* distinction, not lifestyle filler

**The common trap.** Founders and agents both default to stage 1 — plain
benefit — because it feels clean and confident. In a category where every
competitor has said the same words for two years, it reads naive and gets
filed as noise. When `sophistication ≥ 3`, mechanism leads. This single
decision changes the entire page.


## D3 — Onlyness, then attacked

Compress to one falsifiable sentence ([S15](/08-SOURCES) Neumeier):

> Our **[offering]** is the only **[category]** that **[benefit]** for
> **[who]** in **[context]**.

Then delete every word that survives substitution. If a competitor can say the
sentence unchanged, it is not positioning — it is category description.

Record the disqualifiers alongside: *only, given which alternatives, in which
market, as of when.* An unqualified "only" is an unprovable superlative, and it
is the kind of claim that costs trust the moment a reader knows one
counterexample.


## D4 — The promise ladder, evidence-anchored

Attribute → consequence → meaning ([S20](/08-SOURCES) laddering). Ask *"and why
does that matter?"* upward until you reach something a person would say to a
peer.

**Gate:** each rung must cite a record id. A rung with no citation is the point
at which the ladder left the evidence and started composing — cut it there.

> attribute *(mechanism.json)* → consequence *(episodes.json cost)* →
> meaning *(verbatims.json)*

Laddering's known failure is fatigue and confabulation: pushed too far, people
produce plausible terminal values they don't hold. Three rungs with citations
beat five without.


## D5 — Message hierarchy

One promise. Three supports. Proof beside each claim, not in a separate
credibility ghetto.

```json
{
  "one_sentence": "if the visitor remembers one thing",
  "supports": [
    { "claim": "…", "proof": "prf-00X | mech-00X | ep-00X", "objection_answered": "…" }
  ],
  "single_action": "the one thing the page asks for",
  "risk_reducer": "what makes that action safe"
}
```

One action per page. Every additional choice is an exit.


## D6 — Voice, bounded

From `verbatims.json`, not from adjectives:

- **words we use** — harvested verbatims, the buyer's own vocabulary
- **words we refuse** — category jargon the buyer never says
- **claims we will not make** — from `boundaries.json`

Voice is a *vocabulary decision*, derived. "Confident yet approachable" is not a
voice; a list of words the brand says and a list it won't is.

**On archetypes and narrative templates** ([S21](/08-SOURCES),
[S22](/08-SOURCES)): usable as late-stage coherence checks, never as inputs.
Asking "which archetype are we?" before knowing the buyer's belief-state and the
product's proof is choosing a costume before knowing the weather. Jungian
archetype theory has contested empirical status; StoryBrand produces
interchangeable pages when applied as a generator rather than a check.


## The brief's acceptance test

Before it can drive a build:

- ✓ every claim cites ≥1 record id
- ✓ awareness + sophistication set **per segment**, with a reason
- ✓ alternatives include ≥1 non-software entry
- ✓ onlyness sentence survives substitution
- ✓ boundaries name ≥1 real exclusion
- ✓ `gaps.json` open items are listed against the elements they block
- ✓ the single action is one action
