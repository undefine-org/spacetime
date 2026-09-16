# The Method — from interview to pages

A reusable process for building brand + website copy that is **elicited, never
invented**, and that each reader completes with their own situation.

This folder is project-agnostic. It is the playbook; a project is an instance of
it. Serve it:

```
cargo run -- serve method/
```


## Why this exists

The default failure of an AI-built website is not bad writing. It is **fluent
invention**: an agent reads a technical document, infers what a market must
feel, writes quotes nobody said, answers objections nobody raised, and produces
a page that is internally coherent and externally ungrounded. Every sentence is
plausible. None of it is evidence.

The failure is invisible precisely because the output is good. Nothing in a
polished page announces which sentences came from a human and which the model
supplied to fill a shape.

This method makes that distinction structural rather than a matter of care.


## The pipeline

```
ELICIT  ──▶  RECORD  ──▶  DERIVE  ──▶  BUILD
interview     typed        brief +      pages
(2 passes)    evidence     sitemap      (copy w/ provenance)
              + gaps
```

Each stage may only consume the stage before it. An agent building a page may
not reach past the record to its own imagination. If the record lacks what a
page needs, the answer is **a question to the human**, never a guess.

Columns: stage · document · output.

- **Principles** — [00-PRINCIPLES](/00-PRINCIPLES) → the laws that gate every later stage
- **Elicit** — [01-INTERVIEW](/01-INTERVIEW) → two-pass protocol + question banks
- **Record** — [02-RECORD](/02-RECORD) → typed evidence ledger schema
- **Derive** — [03-POSITIONING](/03-POSITIONING) → brief: frame, axes, segments
- **Derive** — [04-SITEMAP](/04-SITEMAP) → which pages exist, and why
- **Build** — [05-COPY](/05-COPY) → the completion mechanic + the lint
- **Build** — [06-AGENT-NEW](/06-AGENT-NEW) → guidelines: build a new page
- **Build** — [07-AGENT-UPDATE](/07-AGENT-UPDATE) → guidelines: update a page
- **—** — [08-SOURCES](/08-SOURCES) → every claim, with epistemic status
- **Test** — [09-DOGFOOD](/09-DOGFOOD) → the first live firing: what it caught, and the five ways the instrument itself broke
- **Convert** — [10-VISUAL](/10-VISUAL) → the belief pass: make the page assert rather than describe
- **Convert** — [11-DYNAMIC](/11-DYNAMIC) → the dynamic pass: make something *happen* to the reader

The last two stages are deliberately last. [10-VISUAL](/10-VISUAL) amplifies
whatever it is pointed at, and amplification is only safe once the record is
built and the copy is linted — run it early and it is a megaphone aimed at
unverified content. [11-DYNAMIC](/11-DYNAMIC) is stricter still, because
behaviour does not merely assert a claim, it *demonstrates* one: a page that
animates like a working product is claiming to be one, in the register a reader
cannot consciously audit.

Both are optional, and on most pages the honest output of each is a short list
of conversions REFUSED with the evidence each would need.


## The one mechanic worth understanding first

The goal is a reader thinking **"this is EXACTLY what I need"** — where the
*what* differs for every reader, and every reader is nonetheless right.

That is achievable, and it is one lint away from horoscope copy.

A **completion line** has two parts:

- a **frame** — a structure the product genuinely acts on: a relation, an
  artifact, a sequence, a consequence. Concrete, checkable, exclusive.
- a **slot** — the noun the reader fills from their own life. Open.

> The promise that lives in a group chat: `<span>`

The frame (*a promise · living in · a group chat*) is precise and excludes
plenty — businesses that don't run on chat, promises that are already
systematised. The slot is the reader's own mess. Press a button to see three
readers complete the same sentence:

```st hidden
@import "./reader.st"
@data inline $slot : "…the one you're already thinking of.";
```
```st src
<div class="sd">
  <p class="sd-line">The promise that lives in a group chat —
    <span class="sd-fill"></span></p>
  <div class="sd-picks">
    <button class="sd-a">a hotel</button>
    <button class="sd-b">a clinic</button>
    <button class="sd-c">an agency</button>
  </div>
</div>

.sd {
  border: 1px solid #2a2f3a;
  padding: 1.1rem 1.2rem;
  background: #0e1116;
}
.sd-line {
  font-size: 1.05rem;
  line-height: 1.6;
  margin: 0 0 0.9rem;
  color: #e6e9ef;
}
.sd-fill { color: #5eead4; }
.sd-picks { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.sd-picks button {
  font: inherit;
  font-size: 0.85rem;
  padding: 0.35rem 0.75rem;
  background: #1b2029;
  color: #c8cedb;
  border: 1px solid #2a2f3a;
  cursor: pointer;
}
.sd-picks button:hover { background: #242b36; color: #fff; }

.sd-a { @on click { $slot <- "the late checkout a manager approved out loud, to nobody's record."; } }
.sd-b { @on click { $slot <- "the call-back someone promised the patient on Tuesday."; } }
.sd-c { @on click { $slot <- "the invoice approval sitting in one partner's unread tab."; } }

.sd-fill { text <- $slot; }
```

Three readers, three different *exactly what I need*, one true sentence.

**The horoscope version** of that line is *"take control of your business."*
Everyone sees themselves in it because it contains nothing. The difference is
not tone or taste — it is testable, and [05-COPY](/05-COPY) gives the
lint that separates them — provenance first, then structure, then
truth-of-completion.


## What makes this rerunnable

The record is typed data, not prose notes. The brief derives from the record by
stated rules. The sitemap derives from the brief by a stated test. Copy derives
from the record with a provenance tag per line.

So a new project is: run the interview, fill the record, turn the crank. And an
*existing* project modified six months later is: read the record, diff, and
re-elicit only what went stale — which is the mechanism that stops a later agent
from quietly reintroducing invention.
