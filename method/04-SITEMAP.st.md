# 04 · Sitemap

Which pages exist, and why each earns its URL. Derived from `segments.json` by a
stated test — not from a conventional site outline.

```st hidden
@import "./reader.st"
```


## The split test

A segment earns its own page **only if ≥2 of these differ materially** from the
segment already served:

1. **awareness stage** — the headline must do a different job
2. **sophistication stage** — the claim may take a different form
3. **the alternatives set** — they'd use something else if you vanished
4. **the mechanism shown** — a genuinely different path through the product
5. **the boundary/preconditions** — different qualification
6. **the search vocabulary** — they arrive with different words in their head

**Only 1 differs** → same page. Use a *switcher* (a tab, a toggle, a
reader-selected skin) rather than a URL. Two pages that differ in vocabulary
alone are a maintenance liability: they drift, and the drift is invisible.

**Only vocabulary differs (6 alone)** → a **routing page**: it exists to catch
the reader's search words and hand them to the real page fast. Its job is
recognition and transfer, not persuasion. Keep it short, and make the handoff
obvious.

> **The trap this prevents:** generating one page per industry because industries
> are easy to enumerate. Industry is a *skin* over a situation
> ([P7](/00-PRINCIPLES)). Two industries in the same situation want the same
> page in different clothes — that's a switcher. One industry spanning two
> situations wants two pages.


## Page types

Columns: type · exists when · job.

- **Home** — *always* → route + carry the strongest single frame
- **Situation** — *a segment passes the split test* → convert one situation completely
- **Skin / routing** — *vocabulary differs, situation doesn't* → recognition → transfer
- **Mechanism** — *sophistication ≥ 3 in any segment* → show how, in depth, once
- **Proof** — *`proof.json` is non-trivial* → evidence, gathered
- **Boundary** — *preconditions exclude meaningfully* → honest qualification — who this is *not* for
- **Method / process** — *the build itself demonstrates the thesis* → show the work ([P8](/00-PRINCIPLES))

The **Boundary** page is unusual and disproportionately valuable for young
products: a page that says plainly who should not buy is the cheapest
credibility available, and it is the one page a fabricating competitor cannot
copy.


## Home's job in a multipage site

Home stops being a full pitch and becomes a **fork**. Its risk is inversion:
built for everyone, it lands on no one — the exact Barnum failure at
architecture scale ([05-COPY](/05-COPY) G0/G2).

Rules:

1. **One frame at the top.** The strongest completion line across all segments —
   not an average of them. Averaging is how you get "streamline your operations."
2. **The fork is visible early** and uses the reader's *own* words for the
   branches.
3. **The branch label is a situation**, not a department: *"coordination lives in
   a chat thread"*, not *"For Operations Teams"*.
4. **Home still converts** for a reader who never forks. Fork ≠ abdicate.


## Cross-page invariants

- **One promise, many instantiations.** Every page instantiates the *same*
  onlyness sentence. Pages that promise different things are different products,
  and the reader who visits two will feel the seam.
- **One action, everywhere.** Same CTA verb, same risk reducer.
- **Mechanism is stated once and linked**, not re-explained per page — divergent
  re-explanations are how a site starts lying to itself.
- **Boundaries appear on every page they apply to**, not quarantined on the
  boundary page.
- **Every page cites the record.** A page with no `segments.json` entry does not
  exist ([I1](/02-RECORD)).


## Output shape

`<project>/method-record/sitemap.json`:

```json
{
  "pages": [
    {
      "path": "/",
      "type": "home",
      "serves": ["seg-001", "seg-002"],
      "frame": "the strongest completion line",
      "forks_to": ["/coordination", "/handovers"],
      "action": "book a 20-minute run"
    },
    {
      "path": "/coordination",
      "type": "situation",
      "serves": ["seg-001"],
      "split_test_passed": ["awareness", "alternatives", "mechanism"],
      "awareness": "problem-aware",
      "sophistication": 3,
      "skins": ["hotel", "clinic"],
      "evidence": ["ep-002", "vb-001", "mech-001"]
    }
  ]
}
```

`split_test_passed` is mandatory on every non-home page and must list ≥2 axes.
A page that cannot fill it is a switcher on another page, not a URL.


## Building it in Spacetime

Multipage is native — one `.st` / `.st.md` per route at project root, served at
its clean path (`about.st.md` → `/about`). No router, no config.

- shared theme/tokens → `modules/theme.st`, imported by every page
- shared components → `modules/*.st`
- per-page copy → `data/<page>.json`, typed with `@type` + `@cms` so copy is
  editable without touching code
- **skins** (same situation, different vocabulary) → a `@view` switcher or a
  reactive class on one page, **not** a second file

The last point matters: the moment a skin becomes a file, it drifts from its
sibling, and nobody notices until the two pages contradict each other in front
of a customer.
