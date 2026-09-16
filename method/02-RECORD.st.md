# 02 · The record

The interview's output. **Typed data, not prose notes** — because every later
stage is a function of it, and because six months from now an agent must be able
to diff it rather than re-imagine the market.

```st hidden
@import "./reader.st"
```

Lives at `<project>/method-record/*.json`. Git-tracked. It is the source of
truth for every sentence on the site.


## The universal envelope

Every entry, in every file, carries provenance ([P1](/00-PRINCIPLES)):

```json
{
  "id": "ev-004",
  "tag": "observed | stated | inferred",
  "text": "…",
  "source": "who or what this came from",
  "date": "2026-07-25",
  "confidence": "high | medium | low",
  "supersedes": null
}
```

- `tag` — the provenance class. **The load-bearing field.** Nothing downstream
  may promote a tag; a page may only demote it.
- `source` — for `stated`, the person. For `observed`, the occasion or artifact.
  For `inferred`, **the agent and the entry ids it was derived from**.
- `confidence` — the human's certainty, not the agent's.
- `supersedes` — set when a later answer corrects an earlier one. The old entry
  is **kept**. A record that overwrites its own history cannot be audited, and
  the correction is often more informative than the correction's content.


## The files

### `verbatims.json` — the language asset

Exact phrases, unedited. Highest-value output of Pass 1
([P3](/00-PRINCIPLES), [S16](/08-SOURCES)).

```json
{
  "id": "vb-001",
  "tag": "stated",
  "text": "the organisational hell of WhatsApp groups",
  "context": "describing what managers currently live in",
  "usable_as": ["headline", "section-title", "agitation"],
  "date": "2026-07-25"
}
```

Rule: **never tidy a verbatim.** The awkwardness is the fingerprint. If it needs
smoothing for use, the smoothed version is a *new* `inferred` entry that cites
the original.

### `episodes.json` — events

One real occasion each. The antidote to introspection ([P2](/00-PRINCIPLES)).

```json
{
  "id": "ep-002",
  "tag": "observed",
  "what_happened": "…",
  "before": "what was happening just prior",
  "after": "what was done immediately after",
  "cost": "hours / money / trust / apology",
  "patch": "what someone did by hand",
  "who": "role, not name, unless publishable",
  "date_of_event": "…",
  "publishable": false
}
```

`publishable` gates whether this can appear on the site. An episode the human
told in confidence is still valid evidence for *deriving* copy while being
unusable *as* copy.

### `alternatives.json` — the competitive set

Includes non-software. Doing nothing is an entry ([S15](/08-SOURCES)).

```json
{
  "id": "alt-003",
  "tag": "stated",
  "alternative": "a WhatsApp group + one manager who remembers",
  "kind": "manual | incumbent-tool | internal-person | spreadsheet | do-nothing",
  "why_chosen": "…",
  "why_fails": "…",
  "what_we_do_instead": "…"
}
```

### `segments.json` — situations, not industries

Per [P7](/00-PRINCIPLES). The `situation` is primary; `industry_skin` is
vocabulary.

```json
{
  "id": "seg-001",
  "situation": "coordination happens in a chat thread nobody owns",
  "trigger": "the observable event that starts the search",
  "who_cares_most": "role + why acute for them",
  "industry_skin": ["hotel", "clinic", "agency"],
  "preconditions": ["channel penetration", "team size", "regulatory"],
  "awareness": "unaware | problem-aware | solution-aware | product-aware | most-aware",
  "sophistication": 1,
  "evidence": ["ep-002", "vb-001"]
}
```

`awareness` and `sophistication` are per-segment, **not** per-product — the same
product meets different segments at different stages, and this is the field that
drives headline strategy in [05-COPY](/05-COPY).

### `mechanism.json` — how it actually works

The ordered sequence. Required at sophistication ≥ 3.

```json
{
  "id": "mech-001",
  "step": 2,
  "trigger": "…",
  "actor": "system | human",
  "action": "…",
  "produces": "the observable artifact",
  "waits_for": "…",
  "refuses_to": "…"
}
```

`produces` and `refuses_to` are the highest-value fields: they are what make
claims checkable and what let a page demonstrate rather than assert
([P8](/00-PRINCIPLES)).

### `boundaries.json` — where it does not fit

```json
{
  "id": "bnd-001",
  "tag": "stated",
  "boundary": "markets without high WhatsApp penetration",
  "kind": "market-precondition | wrong-fit | not-yet | never",
  "consequence": "…",
  "say_publicly": true
}
```

`kind: "not-yet"` vs `"never"` matters — one is a roadmap, the other is
positioning. Conflating them produces either overpromising or needless
self-exclusion.

### `proof.json` — the persuasion inventory

Per [P9](/00-PRINCIPLES). Entries are assets that exist, or the file is short.

```json
{
  "id": "prf-001",
  "kind": "authority | social-proof | scarcity | reciprocity | demonstration",
  "asset": "what actually exists",
  "checkable_by": "how a skeptic could verify it",
  "limits": "where this authority ends",
  "publishable": true
}
```

An empty `proof.json` is a legitimate state for a young product. It is
information: it tells the sitemap that the page must lead with **mechanism and
demonstration** rather than credibility borrowed from customers it does not have.

### `gaps.json` — the honesty ledger

Per [P5](/00-PRINCIPLES). What we *don't* know, recorded rather than filled.

```json
{
  "id": "gap-001",
  "question": "what does a manager do in the first 5 min of the day?",
  "why_it_matters": "determines whether the hero opens on morning triage",
  "blocking": ["hero-headline"],
  "status": "open | asked | answered | accepted-risk"
}
```

Anything `blocking` a page element and still `open` means that element **cannot
ship as fact**. It ships as `inferred`, or it is cut.


## Two invariants

**I1 — Traceability.** Every line of shipped copy carries the record ids it
derives from. A line that cannot cite is not ready. This is what makes
"show, don't claim" true of the *construction*, not merely the subject
([P8](/00-PRINCIPLES)).

**I2 — Monotonic provenance.** Copy may weaken a tag (an `observed` fact stated
as a general assertion) but may never strengthen one. An `inferred` claim can
never appear in quotation marks, be attributed to a customer, or be typeset as
testimony ([P1](/00-PRINCIPLES)).

I2 is the single rule that would have prevented v1's fabricated pain quotes, and
it is mechanically checkable.


## The record as an interface

Downstream stages consume it strictly:

```
verbatims  ──▶ headline candidates, section titles, agitation
episodes   ──▶ demonstration, problem section, proof-by-scene
alternatives ▶ positioning frame, comparison, "unlike X" clauses
segments   ──▶ SITEMAP (one page per situation that passes the test)
mechanism  ──▶ how-it-works, the demo, sophistication-3+ leads
boundaries ──▶ qualifiers, FAQ, honest exclusions
proof      ──▶ trust section — or its principled absence
gaps       ──▶ questions to the human; blocked elements
```

If a stage wants something the record doesn't have, that is a `gaps.json` entry
and a question — never an invention.
