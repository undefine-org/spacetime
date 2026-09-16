# 10 · The Belief Pass

```st hidden
@import "./reader.st"
```

> Pass 1 elicits. Pass 2 attacks. **Pass 3 makes the page mean it.**

Runs only on a page that already survived [`05-COPY`](/05-COPY). It is not a
polish pass and not a decoration pass. It is a pass about the difference between
a reader **reading a claim** and a reader **arriving at a conviction**.

---

## 1 · Text and belief are not the same object

A page made of true sentences can still fail, and this is the failure mode the
first two passes cannot see.

**Text** is a proposition the reader must accept on your authority. It arrives
as a request: *believe this*. The reader's only moves are to agree, disagree, or
skim — and skimming is what usually happens, because agreeing with a stranger's
claim costs something and skipping costs nothing.

**Belief** is a conclusion the reader reached themselves. It doesn't arrive as a
proposition at all. It arrives as *recognition* — the feeling of watching
something you already know be laid out accurately by someone else. The reader
supplies the last step, so the conclusion is theirs, and people don't argue with
their own conclusions.

The founder's acceptance test for this project was **"wow, this is EXACTLY what
I need"** (`vb-004`). Read it closely — it is not *"that's a good product"*. It
is a statement about the READER's situation, in the READER's voice. It cannot be
produced by a sentence describing a product. It can only be produced by the
reader recognising their own Tuesday on the page.

**The operative difference:**

*Text says the work is scattered.* → The reader evaluates the claim. Plausible.
Next section.

*Structure scatters something in front of them.* → The reader experiences
scatter, then names it themselves.

Same proposition, different epistemic route. The first asks for agreement. The
second produces conviction and never asks for anything.

### 1.1 · Why this is the LAST pass, not the first

Because the mechanism is amoral. The same technique that makes a true claim
land makes a false one land harder — a fabricated testimonial in a beautiful
card is more dangerous than one in plain text, not less, because the form
suppresses the reader's scepticism precisely when scepticism was the correct
response.

So the belief pass may only run **after** the record is built and the copy is
linted. Applied earlier it is an amplifier pointed at unverified content.

> **Form amplifies. It never validates.**
> The size of a visual claim must be proportional to the strength of its
> evidence — and the strength lives in the record, which does not change because
> a section got prettier.

---

## 2 · What actually converts

The conversion is not "add an image". Most sections that need this pass need
**structure**, not pictures. Six conversions, ordered by how often they apply:

**Sequence → passage of time.** — A list of steps DESCRIBES a process. The same steps on a rail, with timestamps and visible gaps between them, ENACT one. The reader feels duration, which is usually the actual complaint being made.


**Contrast → adjacency.** — "Before vs after" as two paragraphs is a claim about difference. As two columns the reader's eye does the comparison, and the difference becomes an observation rather than an assertion.


**Volume → repetition.** — "A lot of messages arrive" is a quantity claim, and a bare quantity is forgettable and unverifiable. Twelve message shapes stacked on the page is an experience of volume that needs no number and claims none.


**Absence → a hole you can see.** — The hardest and most valuable. Text cannot depict a gap — it can only assert one. Structure can leave a slot visibly unfilled, and an unfilled slot is read as a problem without a sentence saying "this is a problem".


**Abstraction → a specific object.** — "Small maintenance tasks" is a category; a *burst lightbulb* is a thing with a location. Categories are understood; objects are pictured. Only ever use an object the record actually contains.


**State → a thing that changes.** — If a claim is about something being different afterwards, the page can show the change occurring on scroll or hover. NB: this is the most abusable conversion and the one to reach for last — see §4.


---

## 3 · The moodboard, and what it is for

A moodboard in this pass is **not** a style reference. Its job is to answer one
question per section:

> **What does this section want to FEEL like, and what structure produces that
> feeling?**

Built as a small set of named references, each with an explicit *mechanism*
attached — the reason it works, in structural terms you could implement.
A reference without a stated mechanism is decoration, and decoration is how a
brand drifts from a record.

**Rules:**
1. Each entry names a **feeling** and the **structure** producing it. "Feels
   editorial" is not an entry. "Feels editorial *because* a rule and a mono
   kicker force a hierarchy break before every heading" is.
2. The board must include at least one **anti-reference** — something adjacent
   that would be wrong here, with the reason. Knowing the boundary is worth more
   than another example.
3. Every entry must survive the project's existing constraints. A reference
   requiring photography a company doesn't have is a fantasy, not a board.

---

## 4 · The V-gates

Applied to each converted section. Same verdict precedence as
[`05-COPY`](/05-COPY): **CRITICAL FAIL > FAIL > UNEVALUABLE > WARN > PASS**.

### V0 · Provenance is inherited, not reset

*Applies when:* always.

A visual makes a claim. The claim is subject to every rule in
[`02-RECORD`](/02-RECORD) — including I2, which says copy may weaken a
provenance tag but never strengthen it.

**A visual asserts HARDER than the sentence it replaces.** So this gate is
stricter than the text one, not equal to it:

- Anything shaped like a screenshot, a chat log, a receipt, a metric or a
  timeline reads as *evidence*, whatever the caption says. If the underlying
  fact is `founder-reported`, that form is a **CRITICAL FAIL** — it typesets a
  weaker provenance as a stronger one, which is the exact v1 failure in a new
  medium.
- Illustrative structure is legitimate, but must be **visibly diagrammatic**:
  no chrome, no timestamps implying a real session, no fake identifiers.

### V1 · Does the structure carry the meaning?

*Applies when:* the section was converted.

Delete the words and look at what remains. If the shape still communicates the
proposition, the conversion worked. If the shape is a container that happens to
hold text, nothing was converted — it was reformatted. **FAIL.**

### V2 · Does it survive the reader not looking?

*Applies when:* the conversion involves motion, reveal, or scroll behaviour.

Most readers scroll fast. A section whose meaning depends on an animation being
watched has bet the meaning on attention it doesn't have. The static state must
carry the full proposition; motion may only *reinforce*. **FAIL** if the
still frame is incomplete.

Includes the accessibility case: `prefers-reduced-motion` must lose nothing but
motion.

### V3 · Is the intensity proportional to the evidence?

*Applies when:* always.

A section backed by one founder observation may not be the loudest thing on the
page. When visual weight and evidentiary weight disagree, the reader is being
steered by design away from what is actually known. **FAIL.**

Practical form: rank sections by evidence strength, rank them by visual
prominence, and compare the orders. Inversions are the defect.

### V4 · Does it still work at 393px?

*Applies when:* always.

A conversion that only works at desktop width is a desktop decoration. Most SMB
owners read on a phone. Horizontal overflow is an automatic **FAIL**; a
structure that degrades to a plain list on mobile is a **WARN** — the meaning
survives but the conversion didn't.

### V5 · Would a competitor's page look identical with their content in it?

*Applies when:* always.

If yes, the structure is generic and carries no belief. **WARN** — this is a
weak signal on its own, and it is worth recording because a page can pass every
other gate and still be indistinguishable from the category.

---

## 5 · Procedure

1. **Inventory.** List every section across every page. One row: section ·
   proposition it makes · current form · evidence backing it.
2. **Classify.** For each: does the FORM already enact the proposition, or does
   it describe it? Only the describers are candidates.
3. **Prioritise by evidence.** Convert the best-evidenced sections first. This
   inverts the usual instinct — the weakest sections are the most tempting to
   dress up, and dressing them is precisely the sin V0 and V3 exist to catch.
4. **Pick the conversion** from §2. If none fits, the section may not be a
   conversion candidate; leaving it as text is a legitimate outcome.
5. **Implement**, then run V0–V5.
6. **Record** the pass: what converted, what was left, and what the gates
   caught. A conversion rejected for weak evidence is the most useful line in
   that log — it is where the pass earned its keep.

---

## 6 · The failure this pass is designed to prevent

Not ugliness. **Confident emptiness.**

A page can pass Pass 1 and Pass 2 — every line true, every claim sourced — and
still read as a competent brochure for a company the reader has no feeling
about. That page fails on the founder's own criterion while being entirely
honest, which is why the first two passes cannot detect it.

The belief pass exists because a true page that produces no conviction has
wasted the truth it was so careful to establish.

And the inverse discipline is what keeps it safe: **the pass may only make the
reader believe what the record can support.** Anything else is a better-looking
version of the failure the whole method exists to prevent.
