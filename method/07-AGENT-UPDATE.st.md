# 07 · Guidelines — updating an existing page

Updating is **more dangerous than building**, and it is where this method most
often gets quietly destroyed.

```st hidden
@import "./reader.st"
```

A new page starts from an empty record and obvious ignorance. An update starts
from a page that already reads well — so an agent "improving" the copy has no
felt signal that it is replacing elicited evidence with invention. The page gets
smoother and less true, and nothing announces it.

> **The rule that prevents it:**
> **The record is the source of truth. The page is its output.**
> If a change isn't in the record, it isn't an improvement — it's a regression
> with better rhythm.


## Gate 0 — Classify the request

- **fix typo/layout/bug, copy unchanged** → **§A Mechanical**
- **reword existing copy, same claim** → **§B Rewrite**
- **new claim, section, or page** → **§C Extension** — needs record support
- **"make it punchier / more compelling"** → **§D Vague** — resolve before touching
- **"make it more visual"** → **§F Belief pass** — see below
- **"make it more dynamic / add motion"** → **§G Dynamic pass** — see below
- **market/product/audience changed** → **§E Re-elicit**

Misclassifying **C as B** is the most common and most damaging error: a "reword"
that smuggles in a new claim, unsourced, past every check.

The second most damaging is treating **§F as §A** — "just a visual change,
nothing textual" — because a visual change is a CLAIM change. A structure that
looks like a screenshot or a receipt asserts more than the sentence it replaced,
so a "purely cosmetic" edit can introduce an unsourced claim without touching a
single word.


## §A — Mechanical

Layout, spacing, bug, accessibility. **Copy bytes unchanged.**

- ✓ verify no copy changed: `git diff` on `data/*.json` should be empty
- ✓ re-verify browser at desktop + 393px


## §B — Rewrite (same claim, new words)

1. **Find the record ids** the line cites. No citation → treat as **§C**, and
   file a `gaps.json` entry: this page predates provenance.
2. **Re-derive from the same ids.** The new wording must be entailed by the same
   evidence.
3. **Re-run the gates** ([05-COPY](/05-COPY)). Passing before does not mean
   passing after — G2 (exclusion) and G4 (truth of completion) are the ones a
   "punchier" rewrite silently breaks, because punchier usually means *broader*.
4. **Prefer verbatims.** A rewrite is a good moment to replace a composed line
   with a real phrase from `verbatims.json`.
5. **Tag cannot strengthen.** `inferred` stays `inferred` no matter how confident
   the new phrasing sounds ([I2](/02-RECORD)).

> **Watch:** rewrites drift toward the generic because generic sentences sound
> smooth. If your new line no longer excludes anyone, you have improved the prose
> and destroyed the positioning.


## §C — Extension (new claim / section / page)

**Stop. Check the record.**

```
does the record support this claim?
  │
  ├─ yes ──▶ build it (06-AGENT-NEW steps 3–7)
  │
  ├─ partially ──▶ build it tagged `inferred`, cite the partial support,
  │                file a gaps.json entry
  │
  └─ no ──▶ DO NOT WRITE IT.
            file gaps.json · ask the human · leave the section out
```

New **page**? It must pass the split test in [04-SITEMAP](/04-SITEMAP) with ≥2
axes, and be added to `sitemap.json` before it is built. A page invented because
the nav looked thin is a page nobody needed.


## §D — Vague ("make it punchier")

Do not act on this directly. It is the single most reliable way to convert an
evidence-grounded page into a generic one, because "punchier" and "broader" feel
identical from inside the sentence.

Resolve it into one of:

- **failing a specific gate?** → name which. G2 and G4 are usual suspects.
- **wrong awareness stage?** → the headline may be doing the wrong *job*
  ([03](/03-POSITIONING) D2) — e.g. leading with benefit at sophistication 4.
- **buried verbatim?** → a real phrase exists in the record and isn't on the page.
- **too many ideas per line?** → [R4](/05-COPY).
- **abstraction where a scene belongs?** → [R2](/05-COPY).

Then execute as §B. If none of these fit, the request is a taste disagreement —
surface it to the human with the tradeoff named, rather than guessing.


## §F — Belief pass ("make it more visual / more dynamic")

Run [10-VISUAL](/10-VISUAL). Two things to establish BEFORE starting, because
both change what the request even means:

**1 · Has the page passed the copy lint?** If not, this is not a §F request —
it is §D wearing better clothes. Amplifying unlinted copy is the one sequencing
error the whole method is arranged to prevent.

**2 · Is the request about form, or about a missing claim?** "More dynamic"
sometimes means *the page is boring*, and sometimes means *the page is not
saying enough*. The second is §C and needs record support; no amount of
structure fixes a page that has nothing to assert.

**The §F-specific prohibition.** An update is more dangerous than a build here
than anywhere else in this document, because visual work feels cosmetic and is
not. On an existing page you will be tempted to give an EXISTING sentence a
stronger form — a testimonial-shaped card, a receipt, a metric, a timeline.
Check the provenance of the sentence FIRST. Under I2 a form may weaken a claim's
apparent strength but never raise it, and form raises it far more sharply than
wording does.

**Report the refusals.** The output of a §F pass on a thin record is mostly a
list of conversions you declined and the evidence each needs. That list is the
deliverable, not a consolation prize.


## §G — Dynamic pass ("add motion / make it feel alive")

Run [11-DYNAMIC](/11-DYNAMIC). Everything in §F applies, plus three things
specific to updating an existing page:

**1 · Motion is subtractive on a live page.** On a new build you are adding
emphasis to a blank field. On a page already in use you are *removing* emphasis
from everything you did not animate. Before adding, name what will become
relatively quieter, and confirm you want that.

**2 · Check what is already moving.** Count the existing behaviours first. The
most common outcome of an honest §G pass on a page that has had two of them is
**deleting** a behaviour, not adding one — D5 counts what moves at once and asks
whether that number describes the product.

**3 · The features may not exist.** Several animation constructs named in this
repository's `docs/` are aspirational and fail SILENTLY — `@fade-in` binds an
observer and animates nothing; `@bind` is retired; `&hero.rect.top` yields
nothing. §3.6 of that document is the verified list. And a dev-server browser
check can show a false negative (§3.7): verify on a production build.

The D0 prohibition is absolute and worth restating, because on an update it is
easy to reach for: **do not animate a section into looking like a working
product** unless the record holds that behaviour as observed fact. Motion is
read as demonstration.


## §E — Re-elicit (something changed in the world)

Product shipped a capability · a market opened or closed · real users appeared ·
a competitor changed the claim-space.

**Do not patch the page. Re-run the affected slice of the interview**
([01-INTERVIEW](/01-INTERVIEW)), then re-derive.

Targeted re-elicitation:

Each entry: **what changed** — *re-run* → then re-derive.

- **new capability** — *§1.5 mechanism, §1.6 boundaries* → mechanism, claims, FAQ
- **first real users** — *§1.2 events, §1.4 verbatims* → **proof.json — likely a new page** ([04](/04-SITEMAP))
- **new competitor** — *§1.3 alternatives, §2.4 sophistication* → frame, onlyness, possibly headline strategy
- **new market** — *§1.6 boundaries, §2.4 awareness* → segments, sitemap, qualifiers
- **a claim proved wrong** — *§2.3 falsify* → remove claim + everything downstream of it

**Supersede, never overwrite.** New entries set `supersedes`; old entries stay
([02-RECORD](/02-RECORD)). The history of a corrected belief is often more useful
than the correction — it tells the next agent what looked true and why.

**The high-value transition:** a product's *first real users* changes what the
site may claim. `proof.json` goes from empty to populated; pages that led with
mechanism-because-we-had-nothing-else can now lead with evidence; a proof page
may now pass the split test. This is the most valuable update a site ever
receives — treat it as a re-derivation, not an insertion.


## Drift audit — run periodically, unprompted

Independent of any request:

- ✓ every shipped line still cites live record ids
- ✓ no line's tag has strengthened since last audit
- ✓ no `inferred` line has acquired quotation marks
- ✓ every headline still passes its applicable gates (roles re-checked)
- ✓ boundaries still accurate — has the product outgrown a stated limit?
- ✓ `gaps.json` open items: still blocking, or answerable now?
- ✓ pages still pass the split test, or should two have merged?
- ✓ the one promise is still one promise across all pages

Drift is not decay of the code. It is **accumulated small inventions**, each
individually defensible, that together turn an evidence-grounded site back into
a plausible one. Nothing flags it. Only the audit finds it.


## Report

- what changed, by classification (A–G)
- record ids touched; entries added or superseded
- gates re-run, with results
- **what you were asked for and did not do, and why** ← the most important line

That last item is where an agent earns trust. "I did not add the testimonials
section because `proof.json` is empty and inventing customers would violate the
brand's own thesis" is a better outcome than a page with three plausible
customers on it.
