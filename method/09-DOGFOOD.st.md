# 09 · Dogfood — the first live firing

```st hidden
@import "./reader.st"
```

The lint was run, adversarially, against a real page: `backdesk.org` v1. This
document records what it caught **and where the instrument itself broke**,
because a method that hides its own failures is asking to be trusted rather than
checked — which is the thing it exists to prevent ([P8](/00-PRINCIPLES)).

## Setup

13 lines: 3 pain quotes, 3 pillar titles, 4 ledger claims, hero + subhead + CTA.
Auditor: an adversarial reviewer instructed that confirmation was worthless and
that finding faults in the *method* mattered more than finding faults in the
copy.

Evidence available: 4 verbatims, 1 boundary, 12 gaps, and five empty record
files — because Backdesk has no users and no interview has been run.

## Result on the copy

**No clean PASS in 13 lines.** Not because the writing is bad — much of it is
good — but because it is *unfounded*, and the lint is designed to notice exactly
that difference.

**Three CRITICAL FAILs**, all in `pains.json`:

> `"Did anyone answer the guest who asked to move their booking?"`
> `"Who approved this refund?"`
> `"The follow-up just… didn't happen."`

These score well on G0–G3. They are structural, imageable, exclusive, vivid. And
they are **fabricated testimony**: an agent's inference about SMB pain, typeset
inside quotation marks, which is the visual grammar of speech. Nobody said them.

That combination is the whole finding. **Structural quality did not redeem the
fabrication — it disguised it.** The three worst lines on the page were among
the best-scoring under the original gate order, which is why G0.5 now runs first
and can end an audit on its own.

**One FAIL** on the hero subhead — a compound line listing *bookings, approvals,
refunds, follow-ups*, where at least one element has no support in the record.
Pleasant overbreadth: every reader finds their item in the list, and the list
promises more than the record can carry.

**Everything else: UNEVALUABLE**, blocked on `gap-005` (no episode) and
`gap-007` (no mechanism). Which is the correct answer, and one the lint could
not previously express.

## What broke in the instrument

Five defects. All are now fixed in [05-COPY](/05-COPY); they are recorded here
because *how* a rule fails is more instructive than the rule.

### W1 — The gates assumed every line was a claim

*"Say it once."* failed G1 and G2 — no input→output commitment, excludes nobody.
But it is an **imperative section title**, and imperatives have neither. The lint
was manufacturing false failures for a whole class of legitimate lines, and an
agent following it literally would have rewritten good copy into bad.

→ **Fixed:** a role classifier runs first (claim · completion · imperative ·
doctrine · CTA · compound), and each role declares which gates apply.

### W2 — G4 had no way to say "I can't tell"

G4 requires five reader completions from `segments.json`. `segments.json` is
empty. The gate was unrunnable, and both available verdicts were wrong: FAIL
punishes missing evidence, PASS invents the completions.

Worse, the auditor — reasonably, under pressure to produce a verdict — reached
for the site's own `faq.json` as capability evidence. **But the FAQ is v1 copy,
itself unvalidated inference.** The page would have become its own proof.

→ **Fixed:** `UNEVALUABLE — blocked on <gap-id>` is a first-class verdict that
propagates to the line. Plus an explicit rule: **copy is never evidence for
copy**; G4 may cite only the record.

### W3 — Provenance was an appendix

I1/I2 are the method's load-bearing invariants, and the lint didn't check them.
The three fabricated quotes sailed through four of five gates.

→ **Fixed:** G0.5 runs before everything and is decisive. Its definition of
testimonial grammar now covers blockquote styling, attributions, avatars,
first-person voice, and testimonial card layouts — **not just quotation marks**,
since a quote-mark scan misses most of the ways a page implies a speaker.

### W4 — G3 contradicted itself

The prose said inversion/substitution failures **fail** a line; the summary block
said **WARN**. Two audits of the same line could reach different verdicts
depending on which half was read. A lint that is not deterministic is not a lint.

→ **Fixed:** WARN, explicitly, with the reasoning stated — some true, necessary
sentences *are* category descriptions, and rejecting them outright pushes an
agent toward contrarian phrasing for its own sake.

### W5 — G2 couldn't tell exclusion from an edge case

"Excludes a sole trader with no staff" is technically true of nearly every
sentence ever written. G2 was passable by reaching far enough.

→ **Fixed:** the excluded reader must be a **plausible visitor to this page** —
someone who could land here and correctly conclude "not me" — and cited from
`boundaries.json` or a segment precondition where one exists.

## Round 2 — the repairs, re-attacked

The five fixes were re-run against the same 13 lines by a second adversarial
auditor, briefed to break them. **They partially held, and one of them was
actively harmful.**

### The repair that made things worse

W1's fix was a **six-way role classifier** — classify each line (claim,
completion, imperative, doctrine, CTA, compound), and let the role decide which
gates run.

It reintroduced the exact bug W4 had just eliminated. Three lines classified
differently by two reasonable auditors, and the classification *changed the
verdict*:

- *"Ask why. Get a receipt."* — imperative, or claim? (It is grammatically the
  first and functionally the second.)
- *"No black boxes"* — doctrine, or product claim?
- *"Book a 20-minute run"* — CTA, or a claim containing an unproven "20-minute"?

A lint whose result depends on a taxonomy call is not deterministic. I had fixed
non-determinism in G3 and then rebuilt it one section earlier, in a bigger form:
6 roles × 6 gates × 4 verdicts, needing a lookup table to run.

→ **Fixed properly:** the taxonomy is **deleted**. Each gate now states its own
trigger as a property of the line — *"does this imply a product behaviour?"* is
far more objective than *"is this an imperative?"*, and it is the question that
actually matters. `n/a` became a real, auditable answer.

### The distinction that was still collapsed

G1 lumped together two failures that demand opposite responses:

- **inexpressible** — you cannot state input→output without inventing a
  capability. The *sentence* is empty → FAIL.
- **expressible but unsupported** — you can state it; the record just doesn't
  confirm it → UNEVALUABLE.

Reporting the second as the first sends an agent to rewrite perfectly good copy
when it should be running an interview. **FAIL means fix the sentence.
UNEVALUABLE means fix the record.**

### The other two

- **No verdict precedence.** With gates returning FAIL, UNEVALUABLE and WARN at
  once, the overall verdict was undefined — six lines collided. → Explicit
  precedence: CRITICAL FAIL > FAIL > UNEVALUABLE > WARN > PASS.
- **R6 was never wired in.** The rule that numbers need a source existed in the
  writing rules and no gate enforced it, so *"Book a 20-minute run"* sailed
  through. → Now **P2**, in the preflight, where it stops the audit.

### Still open

Honest remainder, not yet fixed:

- **How an UNEVALUABLE line ships.** It may ship `inferred` — but nothing says
  how that must look on the rendered page, if anything.
- **A misleading verbatim.** R1 says prefer a real phrase. Nothing says what to
  do when the real phrase is *true as speech but false as a claim*.
- **Speaker identity.** G0.5/P1 lists testimonial grammar, but not how to treat
  a first-person line that is genuinely the *founder's* voice rather than an
  implied customer's.

## What this says about the method

**The diagnosis was right and the apparatus was half-built.** It found the exact
defect it was designed to find — fabricated testimony wearing good structure —
and it found it in the strongest-looking lines on the page, which is the case
that matters, because those are the ones nobody questions.

But it could not yet express *"I don't know"*, it mis-graded four legitimate
lines, and it contradicted itself once. Every one of those is a defect that only
appears under real use. None would have surfaced from re-reading the document.

Round 2 taught the sharper lesson, and it is about *how to fix a lint* rather
than about copy:

> **A repair that adds a taxonomy is usually a repair that adds ambiguity.**

W1's problem was real — imperatives were failing gates that shouldn't apply to
them. But the instinct to fix it by *classifying lines* put a subjective
judgement upstream of every objective one, and made the whole lint depend on it.
Moving the trigger into each gate solved the same problem with strictly less
machinery, and removed a lookup table.

That generalises: when a rule mis-fires on a class of inputs, prefer **narrowing
the rule's own trigger** over **classifying the inputs**. The first is local and
testable; the second is global and contestable.

∴ the honest status: **v0.3, tested twice, never used in anger.** Both firings
were against a project with an almost-empty record — which exercised provenance
and UNEVALUABLE thoroughly and left the two hardest gates barely tested. G2's
meaningfulness test and G4's truth-of-completion have still never run in the
condition they were designed for: a page with real evidence behind it.

Three things remain open (above), and they are open on purpose rather than
unnoticed. The next firing — after a real interview, against a page built from a
real record — is the one that tests what these two could not.
