# 06 · Guidelines — building a new page

**Read this before writing any markup.** These are operating instructions for an
agent, not background reading.

```st hidden
@import "./reader.st"
```


## The prime directive

> **You may not invent evidence.**
>
> Every specific on the page traces to [the record](/02-RECORD). If the record
> lacks it: ask, mark it `inferred`, or cut the section. Never fill the shape
> with a plausible sentence.

You will feel a strong pull to fill gaps — a section shape suggests its content,
and you can produce something fluent instantly. **That pull is the failure
mode this method exists to stop.** The output of that pull is indistinguishable
from real material, to you and to the human reviewing it.


## Gate 0 — Do you have a record?

```
record exists?  ──no──▶  STOP. Run the interview (01-INTERVIEW). Do not build.
      │
     yes
      ▼
brief passes acceptance test (03-POSITIONING)?  ──no──▶  derive it first
      ▼
this page is in sitemap.json with split_test_passed?  ──no──▶  it doesn't exist
      ▼
                    build
```

Building without a record produces exactly what v1 produced: a coherent site
grounded in nothing, whose fabrications are invisible because they're
well-written.


## Step 1 — Load the page's context

From `sitemap.json`, this page's entry. From the record, only the ids it cites.

Establish before writing:

- **awareness** → what the headline must *do* ([03](/03-POSITIONING) D2)
- **sophistication** → what it may *claim*
- **the one promise** → shared across all pages
- **the single action**
- **boundaries** that apply here
- **open gaps** blocking elements on this page


## Step 2 — Spine as questions

Each section answers **one** visitor question. A section that cannot name its
question is cut.

Default spine — **reorder by awareness, don't apply blindly**:

Each entry: **section** — *visitor question* → source.

- **hero** — *what is this, and what do I get?* → brief promise + verbatims
- **problem** — *do you understand my actual mess?* → episodes, verbatims
- **mechanism** — *how does it work, in my terms?* → mechanism.json
- **demonstration** — *show me, don't tell me* → mechanism + product itself
- **proof** — *why should I trust you?* → proof.json — **or cut**
- **boundary** — *is this for me?* → boundaries.json
- **objections** — *yes but —* → gaps + Pass 2
- **action** — *what's the smallest safe next step?* → brief action + risk reducer

**Awareness reorders this:**

- *unaware* → open on scene. Product appears late.
- *problem-aware* → open on the pain's structure. **Completion lines strongest here.**
- *solution-aware* → **mechanism moves up, near the top.**
- *product-aware* → proof and objections move up; agitation gets cut.
- *most-aware* → hero + action. Everything else is friction.

**Cut rules.** `proof.json` empty → cut the proof section; do not replace it
with adjectives. No publishable episode → the problem section is written as
assertion in brand voice, never as quotes ([I2](/02-RECORD)).


## Step 3 — Write, verbatim-first

For each line:

1. **Search `verbatims.json` first.** Real phrase > composed phrase ([R1](/05-COPY)).
2. Nothing fits → compose, tag `inferred`, cite what it derives from.
3. **Run the lint** ([05-COPY](/05-COPY)) on every headline, subhead, section
   title, and CTA: split compound lines → preflight (provenance + precision,
   either can stop the audit) → the gates, each of which states its own trigger.
   A gate whose trigger is absent is `n/a`. A gate the record cannot answer is
   `UNEVALUABLE — blocked on <gap-id>` — which means *fix the record*, not
   *rewrite the line*. Never a soft pass.
4. Record the ids beside the line.

**Provenance in the source.** Every copy block carries its citation:

```json
{
  "id": "hero-1",
  "text": "The promise that lives in a group chat.",
  "tag": "inferred",
  "derives_from": ["vb-001", "ep-002"],
  "gates": "G0✓ G1✓ G2✓ G3✓ G4✓"
}
```

Cheap to write, and it makes the next agent's job auditable instead of
archaeological.


## Step 4 — Hard prohibitions

Mechanical rules. No judgement calls.

- ✗ **quotes around an `inferred` line** — fabricated testimony ([I2](/02-RECORD))
- ✗ **invented customer names, logos, counts** — fabricated proof
- ✗ **statistics without `proof.json`** — unverifiable precision ([R6](/05-COPY))
- ✗ **"trusted by hundreds" without a denominator** — fake social proof ([P9](/00-PRINCIPLES))
- ✗ **countdowns / fake urgency** — manufactured scarcity
- ✗ **feelings as headlines** — Barnum ([G0](/05-COPY))
- ✗ **an objection nobody raised** — invented FAQ — cut instead
- ✗ **a persona nobody described** — invented segment
- ✗ **category jargon the buyer never says** — [D6](/03-POSITIONING)
- ✗ **burying a precondition** — [R7](/05-COPY)


## Step 5 — Build it

Spacetime rules ([AGENTS.md](/00-PRINCIPLES)):

- **No custom JavaScript.** Every behaviour is a directive. A missing feature is
  a language design conversation, not a `<script>` tag.
- **Copy is data.** Repeating content → `data/*.json` behind `@type` + `@cms`.
  Copy edits never touch modules.
- One `.st`/`.st.md` per route at project root → clean paths.
- Shared theme/components in `modules/`; skins are `@view` switchers, never
  duplicate files ([04](/04-SITEMAP)).
- **Motion must argue.** Animation that demonstrates a claim earns its place;
  decoration does not. Never hide copy behind motion — reduced-motion and mobile
  get the full content.


## Step 6 — Verify like an adversary

Not "does it compile."

```
cargo run -- check <project>/
cargo run -- serve <project>/
```

Then, in a real browser, desktop **and** mobile:

- ✓ no horizontal overflow at 393px
- ✓ reveals settle; reduced-motion shows full content
- ✓ every interactive element works
- ✓ the CTA is reachable without hunting

Then the **content** audit — the part that actually matters:

- ✓ every line cites record ids
- ✓ no `inferred` line is in quotation marks
- ✓ every headline classified by role and passed its applicable gates
- ✓ no line is UNEVALUABLE-but-shipped-as-fact
- ✓ G4 run against ≥5 real completions per completion line
- ✓ boundaries stated, not buried
- ✓ one action, one promise
- ✓ open gaps: elements marked or cut, never quietly filled


## Step 6b — Optional: the belief pass

Once the page passes Step 6, it is *true*. That is not the same as *convincing*.

A page can be entirely sourced and still read as a competent brochure the reader
feels nothing about — which fails the acceptance criterion while being perfectly
honest. If that describes what you just built, run
[10-VISUAL](/10-VISUAL): convert the sections whose FORM merely describes their
proposition into structures that ENACT it.

Two hard conditions on running it:

- **Never before Step 6.** The pass amplifies whatever it is aimed at; an
  unlinted page gets its unsourced claims amplified too.
- **V0 is stricter than the text gates.** Anything shaped like a screenshot, a
  chat log, a receipt, a metric or a timeline reads as EVIDENCE regardless of
  its caption. Giving that form to a `founder-reported` or `inferred` fact is a
  CRITICAL FAIL — it is the fabricated-testimonial failure in a new medium.

If the record is thin, the correct output of this pass is a short list of
conversions you REFUSED and the evidence each would need. That list is worth
more than the conversions you shipped.


## Step 6c — Optional: the dynamic pass

Strictly after 6b. [11-DYNAMIC](/11-DYNAMIC) asks whether anything *happens* to
the reader, rather than sitting there being true.

The order matters: 6b decides what each section's form should SAY, and this pass
decides whether any of it should say it over time. Reversed, you animate a
layout you are about to replace.

One rule dominates the others (D0): anything that behaves like a working
product — a status changing itself, a counter rising, a message getting
answered — CLAIMS that this happens. A fabricated demo is the strongest lie a
website can tell, so unless the record holds it as observed fact, it is a
CRITICAL FAIL.

Before using any Spacetime feature here, read §3.6 of that document: several
animation features named in this repository's `docs/` are aspirational and fail
SILENTLY. And verify against a production build, never `serve` (§3.7).


## Step 7 — Report honestly

State plainly:

- what you built
- **what you inferred** and from which entries
- **what you could not source** and what you did about it
- which gaps still block which elements
- what you did *not* verify

Confident-but-unchecked reads as a lie when it breaks. A report that names its
own soft spots is worth more than one that sounds finished.


## The self-check

Before yielding, one question:

> **If the human asked "where did this sentence come from?" — for every sentence
> on the page — could I answer?**

Anything that answers *"it seemed right"* is the thing this method exists to
catch. Go back and fix it.
