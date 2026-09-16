# 05 · Copy — the completion mechanic

The target: a reader thinks **"this is EXACTLY what I need"** — where the *what*
differs per reader, and every reader is right.

This document states how that is produced, and the lint that separates it from
its failure twin.


## The mechanic

Comprehension is not decoding; it is **constructing a situation model** — the
reader builds a little world from the sentence, populated from their own
experience ([S06](/08-SOURCES) Zwaan & Radvansky; [S23](/08-SOURCES) Barsalou).
Iser's literary account names the lever: a text's **gaps** are what activate a
reader, and a text that specifies everything produces boredom
([S01](/08-SOURCES), [S02](/08-SOURCES) Ingarden).

Two memory findings explain why reader-supplied detail is stickier than
agent-supplied detail: material a person **generates** is retained better than
material they read ([S24](/08-SOURCES) Slamecka & Graf), and material encoded
**in relation to self** is retained better still ([S08](/08-SOURCES) Rogers et
al.; meta-analysed across 129 studies by [S25](/08-SOURCES) Symons & Johnson —
robust, though smaller against elaboration controls).

∴ A sentence the reader finishes beats a sentence that finishes itself.

**The resolution of the specificity tradeoff** — the crux of the whole request:

> More specific = more vivid, but excludes more readers.
> More abstract = includes more readers, but is forgettable and unfalsifiable.

The resolution is **not** to sit at the midpoint. It is to split the axes:

> **Concrete about STRUCTURE. Open about INSTANCE.**

Draw the *relation* precisely — who owes what to whom, where it lives, what
happens when it slips. Leave the *noun* to the reader.

*Epistemic honesty:* this formula is a defensible synthesis of the sources
above, not a directly validated named law ([S26](/08-SOURCES)). It is stated as
our working rule, and it is testable.


## Anatomy of a completion line

```
The promise that lives in a group chat.
└──┬──┘ └──────────┬──────────┘
   slot            frame
(reader fills)  (product-true, exclusive)
```

**Frame requirements** — all four:

1. names a **structure**: a relation, artifact, sequence, or consequence
2. the product **genuinely acts on** that structure
3. it **excludes** — some plausible readers correctly conclude "not me"
4. it is **imageable** — the reader can see it happening somewhere

**Slot requirements:**

1. the reader can fill it in under a second, from memory
2. **every** legitimate filling is something the product actually handles
3. the filling is *theirs*, not a menu item

Requirement 2 on the slot is the honesty gate. If a reader can fill the slot
with something the product does not handle, the sentence lies — pleasantly, and
at the worst possible moment, since they discover it after committing attention.


## The lint

Run on **every** headline, subhead, section title, and CTA
([S27](/08-SOURCES) Forer/Barnum; [S28](/08-SOURCES) puffery doctrine).

Two things happen before the gates, and both were learned by running this lint
for real against a live page — see [the dogfood log](/09-DOGFOOD).

### Step 1 — Split compound lines

One line, one idea. *"Bookings, approvals, refunds, follow-ups — declared once,
run as conversations…"* is four claims wearing one sentence. **Gate each element
separately**; the line's verdict is the worst of them. One unsupported item
makes the whole line a pleasant overclaim, and a compound line is the easiest
place in the world to hide one.

### Step 2 — Preflight *(decisive, stops the audit)*

Two checks that can end the audit on their own, before any question of quality.

**P1 · Provenance.** A line can be vivid, structural, exclusive, imageable — and
be the worst thing on the page, because it is fabricated testimony. Structural
quality does not redeem invented evidence; it *disguises* it.

- Cites record ids ([I1](/02-RECORD))? No → **FAIL** — not ready.
- `inferred` content presented as `observed`/`stated` ([I2](/02-RECORD))?
  → **CRITICAL FAIL.** Stop. Nothing downstream matters.

**Testimonial grammar is wider than quotation marks.** All of these strengthen
provenance and are equally forbidden on `inferred` content:

- quotation marks around the line
- blockquote or pull-quote styling, quote glyphs
- an attribution: a name, role, company, avatar, or logo
- first-person voice implying a customer speaker
- a label like “what customers tell us” or “we hear this every week”
- a testimonial card layout, even unattributed

A quote-mark scan misses most of these. Check the **rendered page**, not the
source string.

**P2 · Precision.** Every number, duration, count, or superlative in the line
needs a `proof.json` source and a measurement protocol ([R6](/05-COPY)). *"Book
a 20-minute run"* → where does 20 come from? No source → **FAIL** on that
specific, which is fixed by cutting the number, not by softening the sentence.

### Step 3 — The gates, which scope themselves

**There is no role taxonomy.** An earlier draft classified each line (claim,
imperative, doctrine, CTA…) and let the role decide which gates ran — but two
auditors classified the same lines differently and got different verdicts, which
is the same determinism bug as W4 in a new coat ([09-DOGFOOD](/09-DOGFOOD)).

Instead **each gate states its own trigger as a property of the line.** The
question "does this line imply a product behaviour?" is far more objective than
"is this an imperative or a claim?" — and it is the question that actually
matters.

### G0 — Structure or feeling?

> **Applies to:** every line. No exceptions.

Is the line broad because it names a **structure** many situations genuinely
instantiate, or because it names a **feeling** everyone has?

- ✓ **"The promise that lives in a group chat"**  ✗ *"Take control of your business"*
- ✓ **"The step that only works because one person remembers it"**  ✗ *"Peace of mind, finally"*
- ✓ **"Approved out loud, recorded nowhere"**  ✗ *"Run your business with confidence"*
- ✓ **"The follow-up nobody owns"**  ✗ *"Unlock your team's potential"*

Feeling → **FAIL**. This is the Barnum generator: universal affect, recipient
supplies all meaning, nothing said.

### G1 — Commitment

> **Applies when:** the line implies the product *does* something — directly
> (*"chased to done"*), or by implying an outcome the reader would hold you to
> (*"Say it once"* implies saying it twice is unnecessary). If nothing is
> implied about product behaviour, G1 is **n/a** — write `n/a`, not `pass`.

Rewrite as: *Given **[input]**, the product does **[action]**, producing
**[observable output]**, bounded by **[limit]**.*

**Two distinct failures, and they are not the same:**

- **Inexpressible** — you cannot even state the input→output shape without
  inventing a capability → **FAIL**. The line is empty.
- **Expressible but unsupported** — you can state it, but `mechanism.json` does
  not record it → **UNEVALUABLE — blocked on `<gap-id>`**. The line may be
  perfectly good; the record simply cannot yet confirm it.

Collapsing these two is how a missing record gets mistaken for bad copy.

### G2 — Exclusion

> **Applies when:** the line makes a claim about the product or the reader's
> world. Pure instructions (*"Say it once."*) and brand doctrines
> (*"Show, don't claim."*) exclude nobody by construction → **n/a**.

Name a plausible reader for whom this is **false**. If none exists → **FAIL**.

A sentence true of everyone distinguishes nothing. Exclusion is not a cost of
precision; it is the evidence of it — and it is what makes the included reader
trust the page ([P7](/00-PRINCIPLES)).

**The exclusion must be MEANINGFUL, not an edge case.** Any sentence excludes
*somebody* if you reach far enough — "excludes a sole trader with no staff" is
technically true of almost everything and proves nothing. The test:

> Is the excluded reader a **plausible visitor to this page** — someone who could
> land here, read this, and correctly conclude "not me"?

- ✓ *meaningful*: "false for teams already running a ticketing system" — a real,
  populous segment that would otherwise waste a demo call.
- ✗ *edge case*: "false for businesses that don't communicate" — nobody.

Where the excluded group is named in `boundaries.json` or a `segments.json`
precondition, cite the id. An exclusion nobody can point at is a WARN, not a
pass — it usually means the line is broader than it looks.

### G3 — Inversion & substitution *(diagnostic, WARN only)*

> **Applies to:** every line. Never FAILs on its own.

**These two never FAIL a line on their own.** They are diagnostics that say
*"this is category description, not positioning"* — which is a reason to
strengthen the line, not to reject it.

- **Inversion**: does the opposite sound absurd rather than merely wrong?
  *"We keep your promises"* vs *"We break your promises"* — no one would claim
  the inverse, so the line describes the category, not your place in it.
- **Substitution**: can a competitor drop their name in unchanged?

Failing either → **WARN**. The line may ship **only** with a mechanism, a
boundary, or a proof attached that a competitor could *not* copy. Failing both
while also failing G2 → the line is doing no work; rewrite it.

> **Why WARN and not FAIL.** Some true, necessary sentences are category
> descriptions — a page sometimes has to say what it *is* before it says what is
> different. Rejecting those outright pushes an agent toward contrarian phrasing
> for its own sake, which is worse. The category sentence earns its place by
> being *followed* by the differentiating one.
>
> *(Earlier drafts of this document said "fail" in the prose and "WARN" in the
> summary — a contradiction found by the first live run, which made repeated
> audits non-deterministic. WARN is correct.)*

### G4 — Truth of completion *(decisive)*

> **Applies when:** the line has an open slot the reader fills. If every noun is
> supplied by the page, there is nothing to complete → **n/a**. (Do not invent a
> slot to have something to test.)

Enumerate what real readers will fill the slot with — at least five, from
`segments.json`. For each: **does the product actually handle it?**

Any filling it does not handle → **FAIL** or narrow the frame.

This gate is what makes the mechanic honest rather than a horoscope. Barnum copy
permits arbitrary flattering completions; honest copy **constrains admissible
completions to the product's real affordance** ([P6](/00-PRINCIPLES)).

**UNEVALUABLE is a third verdict, and it is not a soft PASS.**

G4 needs `segments.json` (who reads this) and `mechanism.json` (what the product
does). If either is empty, G4 **cannot be run**. The verdict is
`UNEVALUABLE — blocked on <gap-id>`, and it propagates: the line's overall
verdict becomes UNEVALUABLE too, and **the line does not ship as fact**.

This matters because the alternative is worse in both directions. Marking it
FAIL punishes missing evidence rather than exposing it; marking it PASS invents
the completions and launders a guess into a green light. An agent facing an
empty record will do one of those two things unless told to do this instead.

> **The trap that produced this rule.** In the first live run of this lint, the
> auditor reached for the site's own `faq.json` as capability evidence. But the
> FAQ *is v1 copy* — itself unvalidated inference. Using it to validate the hero
> is circular: the page becomes its own proof.
>
> **Rule: copy is never evidence for copy.** G4 may cite only `mechanism.json`,
> `episodes.json`, `proof.json`, or `boundaries.json` — the record, never the
> site.

### Lint summary

```
step 1  SPLIT compound lines → gate each element, worst wins

step 2  PREFLIGHT (stops the audit)
  P1 provenance   cites record ids?            else FAIL
                  inferred shown as observed?  else CRITICAL FAIL — STOP
  P2 precision    every number has a source?   else FAIL (cut the number)

step 3  GATES (each states its own trigger; n/a is a real answer)
  G0  structure not feeling        ∀ lines          else FAIL (Barnum)
  G1  commitment                   if product behaviour implied
                                    └ inexpressible → FAIL (empty)
                                    └ unsupported   → UNEVALUABLE (gap-id)
  G2  excludes a plausible reader  if it claims      else FAIL (universal)
  G3  inversion + substitution     ∀ lines          else WARN (never FAIL alone)
  G4  completions true             if it has a slot
                                    └ no segments/mechanism → UNEVALUABLE (gap-id)
                                    └ completion unhandled  → FAIL (pleasant lie)
```

**Verdicts, in precedence order.** A line takes the *highest-precedence* verdict
any applicable gate returns — not the last one, not a majority:

1. **CRITICAL FAIL** — fabricated provenance. Stop; nothing else matters, and a
   good score elsewhere is not mitigation. It is aggravation.
2. **FAIL** — the line is wrong or empty. Rewrite.
3. **UNEVALUABLE** — the *record* cannot answer yet. Not a copy defect. Ships
   `inferred` or not at all; never a soft pass. Must name its blocking gap-id.
4. **WARN** — ship only with a mechanism, boundary, or proof attached.
5. **PASS** — every applicable gate passed. `n/a` gates do not block a pass.

> **FAIL and UNEVALUABLE are not interchangeable, and the difference is
> actionable.** FAIL means *fix the sentence*. UNEVALUABLE means *fix the
> record* — usually by asking the human a question. Reporting one as the other
> sends an agent to rewrite good copy when it should be running an interview.

**On `n/a`.** Writing `n/a` is a claim that the gate's trigger is absent, and it
is auditable. Writing `pass` for a gate that never ran is not — it manufactures
a green. Prefer an honest `n/a`.


## Try the gates

Two lines, same topic. Same broadness — opposite verdicts.

```st hidden
@import "./reader.st"
@data inline $verdict : "Pick a line.";
@data inline $vclass : "idle";
```

```st src
<div class="lint">
  <div class="lint-row">
    <button class="lint-good">"The promise that lives in a group chat"</button>
    <button class="lint-bad">"Take control of your business"</button>
  </div>
  <pre class="lint-out"></pre>
</div>

.lint { border: 1px solid #2a2f3a; background: #0e1116; padding: 1rem; }
.lint-row { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.85rem; }
.lint-row button {
  font: inherit; font-size: 0.85rem; text-align: left;
  padding: 0.5rem 0.8rem; background: #1b2029; color: #c8cedb;
  border: 1px solid #2a2f3a; cursor: pointer; flex: 1 1 14rem;
}
.lint-row button:hover { background: #242b36; color: #fff; }
.lint-out {
  margin: 0; padding: 0.85rem; background: #070910; color: #9fb0c9;
  font-size: 0.8rem; line-height: 1.7; white-space: pre-wrap; min-height: 7rem;
}

.lint-good {
  @on click {
    $verdict <- "P1 provenance ✓  cites vb-001, ep-002; not styled as testimony\nP2 precision ✓  no unsourced numbers\nG0 structure ✓  a relation: promise · lives in · group chat\nG1 commitment ✓  chat-declared promise → journaled with owner + deadline\nG2 excludes ✓  false for teams already on a ticketing system — a real segment\nG3 diagnostic ✓  no competitor claims this frame\nG4 completions ✓  late checkout · call-back · invoice approval — all in mechanism.json\n\n→ PASS";
  }
}
.lint-bad {
  @on click {
    $verdict <- "P1 provenance ✗  cites nothing — not traceable to the record\nG0 feeling ✗  control is a universal affect, not a structure\nG1 commitment ✗  inexpressible — no input/action/output\nG2 excludes ✗  false for nobody; every owner wants control\nG3 diagnostic WARN  the inverse is absurd → category talk\nG4 completions ✗  reader may fill with payroll, taxes, hiring — not handled\n\n→ FAIL (Barnum)";
  }
}
.lint-out { text <- $verdict; }
```


## Awareness drives the opening move

From `segments.json` ([03-POSITIONING](/03-POSITIONING) D2). The completion
mechanic applies at every stage; **what** the reader completes changes.

- **unaware** — the frame is a *scene*. Reader completes with their own version
  of a normality they had not questioned. Never open with the product.
- **problem-aware** — the frame is the *pain's structure*. Reader completes with
  their instance. **This is where completion lines are strongest.**
- **solution-aware** — the frame is the *mechanism*. Reader completes with the
  step they don't trust today.
- **product-aware** — less completion, more proof. Ambiguity now reads as evasion.
- **most-aware** — none. State the offer.


## Writing rules

**R1 — Verbatim first.** Before composing a line, search `verbatims.json`. A
phrase a real person said outperforms one an agent composed and it is checkable
([S16](/08-SOURCES)). Compose only when nothing fits, and tag it `inferred`.

**R2 — Scene over adjective.** Concrete, imageable language recruits the
reader's own experience ([S06](/08-SOURCES)). *"Approved out loud, at 6pm, to
nobody's record"* beats *"poor accountability"*.

**R3 — No quotation marks around `inferred`.** [I2](/02-RECORD). The single
mechanical rule that prevents fabricated testimony.

**R4 — One idea per line.** A sentence with two frames gives the reader two
slots and they fill neither.

**R5 — Curiosity must be bounded and answered.** A gap works when the reader can
*see* what's missing and the page closes it ([S29](/08-SOURCES) Loewenstein
information-gap). Arbitrary withholding is clickbait.
*(Note: do not invoke the Zeigarnik effect for this — a 2025 meta-analysis finds
no general unfinished-task memory advantage. [S30](/08-SOURCES))*

**R6 — Numbers carry a protocol or don't appear.** Precise figures can raise
credibility ([S31](/08-SOURCES)) but unverifiable precision backfires and
invites scrutiny. No number without a source in `proof.json`.

**R7 — Qualifiers are trust, not weakness.** *"Works where WhatsApp is how
business already gets done"* excludes readers and converts the rest better,
because it proves the page is describing reality ([P7](/00-PRINCIPLES)).

**R8 — Every line carries its record ids.** In a comment beside the source, in
the CMS entry, or in the page's data. A line that cannot cite is not ready
([I1](/02-RECORD)).


## Transportation, and its limit

A reader inside a scene counter-argues less ([S32](/08-SOURCES) Green & Brock).
That is a real effect and a real temptation.

**The limit:** reduced counter-arguing is not truthful persuasion. Immersion
must never be used to slip past qualification — if a reader is wrong for the
product, the page's job is to let them find out *fast*, not to carry them past
the point where they'd have noticed ([P8](/00-PRINCIPLES)).

Applied test: if removing the scene would make a reader realise they're a bad
fit, the scene is doing dishonest work.
