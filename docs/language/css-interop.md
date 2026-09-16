# CSS / HTML interop: the host spelling must work or explain itself

**Rule (PLAN-137 W7, GH-20):** when a construct exists in the host language
(CSS or HTML) *and* Spacetime has its own spelling for it, the **host spelling
must either work or explain itself** — vanishing is off the table.

Concretely: a `@font-face { … }` block, a `<title>`, a `<meta>`, a `<link>`, or
an `<html lang>` an author writes must produce the effect they plainly intended,
or fail loudly with a diagnostic that names the Spacetime spelling. It must
never compile clean and silently drop.

## Why

An author reaching for the language they already know is not making a design
choice; they are typing the first thing that works. If Spacetime recognizes the
spelling and then discards it on an internal naming preference, it spends the
author's goodwill defending a preference they were never told about — and, worse,
it turns a missing feature into silent **data loss** (the markup simply never
ships). Recognizing the host spelling and routing it through the Spacetime
mechanism that produces the same output is almost always cheaper than the
goodwill the silent drop costs.

The two directions of this rule, both implemented in W7:

| host spelling | Spacetime spelling | disposition |
|---|---|---|
| `@font-face { … }` (raw CSS at-rule) | `@font("Family") { … }` macro | host spelling **works**: routed into the host CSS at-rule passthrough (`@media`/`@supports`/`@keyframes`), emitted verbatim, converging with the macro on the same `@font-face` rule |
| `<html lang="fr">` wrapper, `<title>`, `<meta>`, `<link>` at `.st` top level | (the page-shell owns `<html>`/`<head>`) | host spelling **works**: hoisted/extracted by `render_page_shell` and fed into the generated shell |

## How the host spelling is honoured — one implementation, no parallel paths

The host spelling must be routed through the *same* machinery that produces the
output, never a second bespoke path:

- `@font-face` joins `HOST_CSS_PASSTHROUGH_AT_RULES` in `src/parser/mod.rs` — the
  **single registry** for "host CSS at-rules Spacetime passes through verbatim".
  `@media`, `@supports`, `@keyframes` were already there (BUG-087); `@font-face`
  is structurally the same (an at-rule with a declaration body). Every consumer
  reads that const — never an inline `"media" | "supports" | …` list that can
  drift. The `@font("Family")` macro remains the name-in-head convenience; both
  spellings land identical `@font-face` rules in the emitted stylesheet.
- `<title>`/`<meta>`/`<link>`/`<base>` and `<html lang>` are hoisted/extracted by
  `src/html/page_shell.rs::extract_head_elements` — the single scanner for
  "document-level metadata authored in a `.st` entry". The shell owns the real
  `<html>`/`<head>`, so the authored tags are lifted into where they actually
  work, and the `<html lang>` value feeds the generated `<html lang>`.

## When vanishing is allowed

Only when the construct is *explicitly* inert by design and the author is told.
A silent drop with a clean `check` is the defect. If a host spelling cannot be
supported yet, it must produce a diagnostic that names the supported spelling —
not a `continue`.

## Index of the involved code

- `src/parser/mod.rs::HOST_CSS_PASSTHROUGH_AT_RULES` — the host-at-rule registry.
- `src/html/page_shell.rs` — `extract_head_elements` / `render_page_shell`: head
  hoisting + `<html lang>` extraction.
- `stdlib/macros/css-at-rules.st` + `stdlib/primitives/font-face.st` — the
  `@font("Family")` spelling, lowering to the same `@font-face` rule.
- Gates: `tests/repro/gh-20-font-face/check-contract.sh` (+ the `--cdp` browser
  half `font-face.cdp.test.st`) and `tests/repro/gh-21-page-head/`.
