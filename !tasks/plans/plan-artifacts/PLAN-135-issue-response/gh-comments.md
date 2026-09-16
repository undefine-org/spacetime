# GitHub issue responses — drafted 2026-08-06

Every comment below is verdict-first, then the mental model, then what happens next.
Repros live in `tests/repro/gh-<N>-*/`. Verdicts came from running them against the
freshest buildable main (HEAD `f13a9783` does not currently compile — concurrent WIP).

Post with: `gh issue comment <N> --body-file <section>`; close where marked **CLOSE**.

---

## #6 — one-shot timelines don't hold end state — **CLOSE**

Verified fixed. Repro: `tests/repro/gh-6-hold-end-state/` — a one-shot `@on visible`
timeline, advanced past its duration with `@clock`, keeps both `opacity: 1` and
`translateY(-100px)`.

The mental model worth carrying: a driver is just a function from *something* to a
progress number between 0 and 1. Scroll position, visibility, elapsed time — all of
them do the same job. "Holding the end state" isn't a separate feature; it's the answer
to "what does the element look like at p = 1", and the keyframe applier now answers that
unconditionally instead of unwinding.

One caveat I could not test at the `logic` rung: if the driver *re-fires* from p = 0
(element leaves and re-enters the viewport), you'll see the revert again. That's a
different question — re-entry policy, not fill mode. If your original case was a curtain
that flickers on re-scroll, reopen with that detail and we'll treat it as its own issue.

---

## #8 — directive inside HTML markup is dropped and leaks — **stays open, wave W4**

Still reproduces exactly as you described. `inspect --layer ir` gives 0 primitives, 0 js,
0 css; `check` is silent; inside `@fixture` the directive's *source text* shows up as page
content. Repro: `tests/repro/gh-8-directive-in-html/`.

The model: a `.st` file has two different kinds of region, and they're not
interchangeable. Markup describes what the page **is**. A selector scope describes what
elements **do**:

```spacetime
<div class="hero">…</div>        // markup — literally what ships

.hero { @scroll fade(…) { … } }  // scope — behavior attached to matching elements
```

Directives bind through selectors, never through nesting, because a directive must be
able to attach to elements that don't exist yet (a row rendered later by `@each`, a
fragment spliced in at runtime). Nesting can only describe what's already there.

So the shape you wrote is unsupported by design — but "unsupported" must never mean
"silently ignored". The fix is a diagnostic that names the shape and shows you the
selector-scope rewrite. Making the nested form actually bind stays out of scope.

---

## #9 — animate-color calls nonexistent ST.lerpColor — **CLOSE**

Fixed in `6de67b4e`. `animate-color` now dispatches the way the keyframe path already
did: `ST.interpolateColor` for OKLCH, `ST.interpolateColorRgb` for sRGB, plus a guard so
an unparseable endpoint skips the tick instead of assigning `null`.

Your find is a good illustration of a structural gap: a `%primitive` emits JavaScript as
a *string*, so nothing type-checks the call against what the runtime actually exports. A
ghost function can sit in stdlib for months and only fail on the first tick of the first
page that uses it. The commit's gate is deliberately blunt about that — it asserts that
`ST.lerpColor` does **not** exist, so a primitive naming it again is a failing test rather
than a silent TypeError.

The deeper fix (interpolation chosen by *type* from the `%scalar_type` table, so a new
animatable type doesn't mean a new primitive) is FUP-160 and now has the scalar table it
needed.

---

## #10 — second template invocation doesn't re-render @each — **CLOSE**

No longer reproduces. Repro: `tests/repro/gh-10-template-each-twice/` — a template with
`@each` invoked twice on one page renders 3 rows in *both* instances.

Worth knowing why this class of bug happens at all: a template invocation is not a copy
of markup, it's an instance with its own scope. Anything cached "per template" instead of
"per instance" produces exactly your symptom — the second one thinks the work is already
done. If you hit something similar again, the diagnostic question is always: *what key is
this cache using, and does that key distinguish the two things I can see on screen?*

Note the related build-time case is still open as #11 (static unrolling at compile time
has its own evaluation path).

---

## #11 — SSG @each unroll doesn't apply filter pipes — **stays open, wave W7**

Still open, and deliberately so. Here's the tension worth understanding.

Spacetime renders a static list twice over: once at build time (so the HTML in
view-source is real content, good for SEO and no-JS) and once at runtime (so the same
list can react to data changes). Filters like `| uppercase` are defined in **stdlib**, as
Spacetime — not in Rust. The runtime can apply them because the runtime evaluates
Spacetime. The build-time unroller currently can't, because it has its own small
interpolation path.

There are two ways out, and only one is allowed: re-implement the filters in Rust (fast,
and wrong — the project rule is that Rust never re-implements a construct stdlib
defines), or teach the build-time path to evaluate stdlib filters properly. We're doing
the second. That's why this sat rather than got a quick patch.

---

## #12 — @stage on a non-canvas element fails only at runtime — **stays open, design settled, wave W6**

Confirmed, and the fix is now decided: `@stage` will declare the element it requires.

Macros already declare *where* they may be attached, as data:

```spacetime
%macro stage {
  %scope selector          // today: "attach me to a selector scope"
  …
}
```

`%scope` already supports `file`, `selector`, `property-value`, and
`within(<construct>)`. We're adding an element constraint —
`%scope element(canvas)` — so the requirement "this needs a `<canvas>`" is stated once,
in the macro, as data, and the compiler checks it at bind time. `@stage` on a `<div>`
becomes a compile error, and three.js's try/catch plus the sibling `console.warn` stop
being the only thing standing between you and a blank page.

The general principle: if a constraint is knowable at compile time, it belongs in the
macro's declaration, not in a runtime guard. And new constraint kinds should be new
*data*, not new Rust match arms.

---

## #13 — 3D macros without the import are silently dropped — **CLOSE**

Fixed by `98f92c0c` (unknown primitive is now a build failure). Your exact repro:

```
error[E0956]: unknown primitive: stage
  hint: Check the %binds/driver that names it
```

`check` and `build` both exit 1.

The thing that made this bug possible: expansion had a catch-all arm that emitted a
comment (`// Primitive not found: stage`) instead of failing. A catch-all that "handles"
the unknown case by writing a note to nobody is how a missing import turns into a blank
page. The general rule now is that the default arm fails.

---

## #14 — documented @object bareword form silently defaults — **stays open, wave W3**

Still reproduces. `@object torusknot(size: 1.2, …)` compiles clean and emits
`geomFor("icosahedron")` — the default. The nested-body form falls back to
`switch ("standard")` the same way. Repro: `tests/repro/gh-14-object-bareword/`.

Two separate lessons in one issue.

The first is about grammars: a `%form` either matches your call or it doesn't. When it
doesn't, every capture keeps its default, and if nothing complains you get a *plausible*
result rather than an error — the worst possible failure, because it looks like it
worked. Fixing that is one invariant (see #37) applied everywhere.

The second is about docs: the examples in `object.st`'s doc comment were written by hand
and never compiled, so they drifted from the grammar underneath them. Spacetime has an
answer for this — literate `.st.md` files and `@doc(src:)`, where the example *is* the
program. Doc examples for stdlib macros should live somewhere that fails the build when
they lie. That's part of the same wave.

---

## #15 — named %form args must appear in declaration order — **CLOSE**

No longer reproduces. In-order, out-of-order, and fully scrambled all bind correctly now:
`@object(metalness: 0.9, shape: "sphere", detail: 6, size: 1.4, roughness: 0.15)` emits
every value from the source, and the `glass-hero` demo you flagged now emits `const s =
1.7` matching what it says. Repro: `tests/repro/gh-15-named-arg-order/`.

You were right that this was the important kind of bug: a named-argument syntax with a
positional binder underneath isn't a small inconsistency, it's a lie about the language.

Note the evidence here is IR-level, not behavioral — 3D values aren't observable at the
`logic` rung. If you want to be certain for your own case, `inspect --layer ir` and read
the constants.

---

## #16 — bare-tag selector after a semicolon-less @import is swallowed — **CLOSE**

Fixed. `@import "stdlib"` with no trailing `;` followed immediately by `canvas.hero { … }`
now emits identical CSS to the version with the semicolon. Repro:
`tests/repro/gh-16-bare-tag-import/` (four variants, including the class-selector control).

Your root-cause read was exactly right: the newline guard enumerated the tokens that can
start the next construct and forgot bare identifiers. Whenever you write a "can I stop
here?" check in a parser, the honest way to write it is to enumerate what may *follow*,
and that list has to be complete — a partial list means constructs get eaten, not
rejected.

---

## #17 — headless CSS assertion dies with a generic TypeError — **stays open, wave W5**

Still exactly as filed. The stub is still `getComputedStyle = (el) => el.style || {}`, so
a stylesheet-derived read fails with `Cannot read properties of undefined (reading
'trim')` — no rung named, no `--cdp` hint. Repro: `tests/repro/gh-17-headless-cssom/`.

This one matters more than its symptom. Spacetime's testing story rests on a fidelity
ladder: `--headless` gives you logic, `--cdp` gives you layout, timing and paint, and a
backend that *can't* provide a rung is supposed to **refuse** rather than fake a pass.
The refusal is the feature — it's what stops a green suite from certifying something that
never ran.

A stub that returns `{}` breaks that contract quietly. The fix is to make CSSOM reads
refuse the same way timing assertions already do (`needs timing` → named refusal → "run
with `--cdp`"). Same pattern, one more rung.

---

## #18 — @reveal's custom keyframes body is dropped — **stays open, wave W4**

Still reproduces: the body compiles clean, zero diagnostics, and the property in it never
appears in the emitted JS. Repro: `tests/repro/gh-18-reveal-keyframes/`.

The anatomy is worth learning because it generalizes. A macro has two halves:

```spacetime
%form  { @reveal(…) { $body:keyframes } }   // what the author may write
%binds { reveal-engine(&self, …) -> { } }   // what gets handed to the primitive
```

`%form` *captured* `$body`. `%binds` never passed it on. Parsing succeeded, so nothing
complained — but a capture with no consumer is a promise the compiler doesn't keep.

The structural fix isn't "wire this one up", it's a check that every `%form` capture is
either consumed in `%binds`/`%emit` or explicitly marked as ignorable. That turns a whole
genus of silent half-features into build errors, and it's cheap because the information
is already in the registry.

---

## #19 — selector-scoped primitives never initialize under --headless — **CLOSE**

No longer reproduces. A `@reveal` attached through a top-level selector rule now
initializes headless through *both* paths — explicit `ST.initElement` + flush, and the
automatic MutationObserver — and produces its split nodes. Repro:
`tests/repro/gh-19-headless-selector-init/`.

Keep the instinct that made you file it, though. "The test passed but I can't prove
anything ran" is the single most valuable suspicion a test author can have. The project
rule that came out of this era says it plainly: if a feature's job is to move something,
some gate has to assert it moved.

---

## #20 — raw CSS @font-face is silently dropped — **stays open, and it's becoming a feature, wave W7**

Confirmed: the block vanishes with no diagnostic, no CSS, nothing in `--layer emit`.
Repro: `tests/repro/gh-20-font-face/`.

The original plan was "warn and point at `@font(...)`". That's been overruled, and I
think correctly: **support what people reach for first.** `@font-face` is what every CSS
author on earth types. A language that recognizes it, understands it, and then refuses it
on a naming technicality is spending the author's goodwill to defend an internal
preference.

So the resolution is: raw `@font-face` becomes supported syntax, handled through the same
machinery `@font(...)` uses. Until that lands, the interim behavior is still a
diagnostic — silence is never the answer.

Generalize it as a design rule: when a construct exists in the host language (CSS, HTML)
and Spacetime has its own spelling, the host spelling should either work or explain
itself. Vanishing is the one option that's off the table.

---

## #21 — no way to set title/meta/lang from a .st-only page — **stays open, planned as PLAN-035, wave W7**

Confirmed: there's no `@title`, `@meta`, or `@head` anywhere in stdlib, and
`render_page_shell` hard-codes `<html lang="en">` and `<title>Spacetime</title>`.

This is tracked as **PLAN-035** ("`.st`-only page shell cannot set lang/title/meta/fonts")
— that's the plan to read; nothing is filed under the name "@seo". Its acceptance
criteria are already the right ones: a `.st` entry sets `<title>` and `<html lang>`,
registers remote fonts without authored HTML, and a real project builds with **no**
`index.html` and identical head output.

Your second observation is the sharper one and shouldn't get lost: an authored
`index.html` silently wins over `index.st`'s own body markup. An escape hatch that
quietly overrides the primary path converts "missing feature" into "data loss". Whatever
`@head` ends up looking like, that override needs to become explicit or diagnosed.

---

## #22 — @use leaks a "Primitive not found: use" comment — **CLOSE**

Fixed. A page using `@use` builds with zero occurrences of `Primitive not found`, and the
module's CSS emits normally. Repro: `tests/repro/gh-22-use-comment-leak/`.

Same root cause as #13, which is why one change closed both: the expansion layer had a
catch-all arm that wrote a comment instead of failing, so a compile-time-only directive
falling into the primitive path left a note in the bundle rather than an error on your
terminal.

One incidental find while reproducing: the literal call in your issue (`@b/badge("NEW")`
with no body) now fails with `E0946: Directive does not match its declared grammar:
Expected body block { ... }`. That's the grammar being strict, unrelated to the leak — but
if that shape *should* be legal, it's worth a separate issue.

---

## #23 — a non-matching descendant rule drops every macro after it — **stays open, TOP PRIORITY, wave W1**

Reproduced end to end, and it is as bad as you documented. In the minimal case the
handler before the poison rule emits and the one after it does not: parse layer shows
`.btn-b` with `directives: 0`, the built JS contains `frame <- 1` and not `frame <- 2`,
and the behavioral gate fails with `$frame == "2" (got 0)`. `check` passes throughout.
Repro: `tests/repro/gh-23-poison-descendant/`.

This is first in the queue. Not because it's the most interesting — because its blast
radius is "everything below this line in the file", and your 2100-line stylesheet losing
20 directives (a configurator, an acuity chart, a marquee, the primary CTA's magnetism) is
the worst possible manifestation: every gate green, two features gone.

The structural read: rule scoping resolves selectors against markup, and a *failed*
resolution currently **aborts the pass** rather than skipping the rule. Skipping is always
correct; aborting is never correct. There's precedent for the diagnostic too — `E0602`
already exists for the bare-selector case, and the descendant case should behave the
same way and *continue*.

Your two secondary notes are real and will be filed separately so they don't get lost:
`@cursor` parking at the top-left until the first pointermove, and `check` not counting
`$signal <- …` writes or `var(--st-*)` reads as uses of a signal.

---

## #24 — @reveal(split:"lines") re-splits its own generated DOM — **stays open, wave W5**

Still present in the emitted code: `splitText` re-snapshots `Array.from(el.childNodes)`
on every call, and the ResizeObserver calls it again. Repro:
`tests/repro/gh-24-reveal-resplit/`.

Note what the repro *couldn't* do — there's no `ResizeObserver` at the `logic` rung, so
the re-split path isn't exercisable headless at all. That's recorded in the test rather
than papered over, and it's precisely why this fix needs a `--cdp` gate that resizes a
real element and counts the reveal units.

The lesson is one of the most transferable in front-end work: **a DOM transform must be
idempotent, or it must read from a snapshot it owns.** The moment a transform's input is
its own past output, calling it twice is a different program than calling it once. Adding
observers (which fire in an order you don't control — your note about ResizeObserver
beating IntersectionObserver is exactly right) turns "twice" from a hypothetical into a
guarantee.

---

## #25 — failed matches for known directives are discarded — **CLOSE**

Fixed. All three of your shapes now produce errors:

```
error[E0946] index.st:3:9  @scroll — Expected capture $name:Ident
error[E0946] index.st:4:9  @load   — Expected capture $name:Ident
error[E0946] index.st:5:9  @each   — Expected $source:Binding, got "COLON"
```

Repro: `tests/repro/gh-25-failed-matches/`.

Your diagnosis was precisely right, down to the line numbers: the matcher computed a good
`MatchDiagnostic`, and the caller destructured it into `_diags`. That's the shape of bug
worth remembering — the error path *existed*, was *correct*, and had no consumer. When
you're auditing a diagnostic system, "who reads this channel?" catches more than "is this
error implemented?".

This is also the invariant we're now extending to captures that consume only part of
their input (#37, #35, #36) — same channel, more coverage.

---

## #26 — undefined-signal analysis is never invoked — **stays open, wave W2**

Confirmed with your repro: `check --verbose` emits no E0408, and the build emits
`ST.resolve(__node, 'definitelyMissing')`. Repro: `tests/repro/gh-26-e0408-unwired/`.

The mental model for why this is a *big* deal despite being a tiny fix: Spacetime's
reactive lookups resolve by name at runtime, walking up from the node. That's what makes
signals composable — a template doesn't need to know where its data comes from. The
tradeoff is that a typo can't fail at runtime either; it just resolves to nothing and
renders empty. The compiler is the only place that can catch it, and the analysis for it
has existed, correct and unreachable, this whole time.

Fix is: call it from `check_file`. It ships with #27 and #34 as one "make `check` honest"
wave, because all three are the same failure — diagnostics that exist and don't reach you.

---

## #27 — canonical @data inline reports a false E0923 ambiguity — **stays open, wave W2**

Still fires, with the range anchors as broken as you described (the second diagnostic
points past the end of the first line). Repro: `tests/repro/gh-27-false-ambiguity/`.
`541f625f` fixed the `@form` union-literal family; its own message notes the `@data`
bare-literal family as residual, and that's what you're seeing.

The structural point: literal-aware parsing already *decided* — it selected `data-inline`
and stored that in `FormMatch.matched_macro`. The ambiguity lint then ignores that result
and re-scores from captures alone, where `inline` and `derive` look identical.

Two passes deriving the same fact independently will always eventually disagree, and the
one that runs later gets to be wrong in public. The fix isn't better scoring — it's the
lint reading the decision that was already made.

---

## #28 — LSP uses a drifted static registry — **stays open, wave W8 (root of the LSP cluster)**

Untouched, and this is the one to fix first — #29, #30, #31 and much of #32 and #33 are
downstream of it.

The compiler builds its registry by *loading* stdlib and the project overlay, so it always
knows exactly what macros exist, including project-defined ones. The LSP maintains a
hand-written `include_str!` list, skips files that fail to parse, and serves every editor
feature from that snapshot. Your measurement — 97 leading form tokens in the compiler
versus what the LSP advertises — is the whole issue in one number.

Two sources of truth for the same fact don't drift *if* someone maintains them; the
problem is that nothing fails when they diverge, so the drift is invisible until a user
hits it. The fix is not better syncing, it's deletion: the LSP consumes the compiler's
registry, project roots included, and the static list goes away.

That's also why the per-provider issues will get much smaller: a completion generator
that reads real `%form` grammar doesn't need to guess.

---

## #29 — completion snippets flatten %form grammar into invalid syntax — **stays open, wave W8**

Untouched. And it's the highest-harm item in the LSP cluster, for a reason worth naming:
the editor is *generating* the exact malformed shapes that used to compile to nothing.
Your issue and #25 were the same story from both ends — the tool wrote `@scroll(name: ${1})`,
the compiler shrugged, the author got a page that does nothing.

Half of that is now fixed: those shapes produce `E0946` (see #25). The other half is
this. A `%form` is a grammar — positional captures, sigils, literal words, a typed body —
and a snippet builder that models every form as "named parameters in parens" isn't
approximating the grammar, it's discarding it.

Once the LSP reads the real registry (#28), snippets should be generated *from* the form
grammar, so a snippet that doesn't compile becomes structurally impossible rather than
merely unfortunate.

---

## #30 — go-to-definition returns null for stdlib, macros, variables, templates — **stays open, wave W8**

Untouched. The two causes you found are both about identity: embedded registry entries
carry fake relative paths like `timeline` instead of a real file URI, and definitions are
stored without the span they came from.

The generalizable bit: an index is only as useful as what it records at *write* time. You
cannot recover a definition's location later if the thing that knew it threw it away —
which is also why #31's references land on whole directive blocks. Both fall out of the
same repair: store canonical absolute path + exact capture span when the definition is
registered, and make sure `register_definition` actually has a production caller.

---

## #31 — references duplicate whole directive blocks, can't include declarations — **stays open, wave W8**

Untouched. Your root-cause list is complete and I'd only underline two of them.

`UsageVisitor` walks `file.matches` *and* the scoped matches, but the former already
contains the latter — so every usage is found twice. Whenever a result set is exactly
duplicated, suspect a container being traversed through two paths rather than a bug in the
matching itself.

And `@each` stores `FormMatch.span` (the whole block) instead of the capture's own span,
which is why your references cover the entire directive. Same root as #30: the span you
record is the span you can serve.

---

## #32 — LSP diagnostics diverge from check; preset scan warns inside comments — **partially fixed, rest is wave W8**

Partially addressed. The `~preset` machinery was retired in the SIP-001c work
(`783fc242`, `e111f172`) and the LSP's `~`-preset debris was deleted with it, so the E163
false positive on a comment mentioning `~not-a-real-preset` should be gone.

The larger half stands: LSP diagnostics only cover parse failures plus a raw text scan,
while `check` runs registry semantics, dispatch checks, and pipeline analysis. So the
editor stays quiet about an unknown signal (#26 will make `check` loud about it) or an
unresolved project macro (#28 is why it can't see them at all).

The target state is "one diagnostic engine, two front-ends" — the LSP should be publishing
the compiler's diagnostics, not deriving its own approximation of them. That's the only
version of this that doesn't drift again in six months.

---

## #33 — color and semantic-token scanners ignore lexical context — **stays open, wave W8**

Untouched. Two providers, one mistake: both scan raw text where a lexer's answer was
already available. Hence colour pickers on `#abc` inside a comment and inside arbitrary
strings, and a multi-line block-comment token that gets built and then dropped because the
emitter only handles single-line spans.

The rule of thumb: if the CST knows something, a regex over the same bytes is not a
shortcut, it's a second, worse parser. Drive both providers from lexer/CST context —
colours only in CSS value positions, comments excluded — and split multi-line comment
tokens per line, since that's what the LSP protocol requires.

---

## #34 — check hides warnings unless --verbose, still prints "All files passed" — **stays open, wave W2**

Confirmed. `@data inline $unused : 1;` → plain `check` prints `✓ All 1 file(s) passed`
with exit 0 and no warning text; `--verbose` reveals `W0201 defined but never used`.
Repro: `tests/repro/gh-34-hidden-warnings/`.

Your suggested behaviour is what we're implementing: render warnings by default, add
`--quiet` to suppress, add `--deny-warnings` for CI, keep exit 0 for warning-only runs
unless deny mode is on.

The reason this is grouped with #26 and #27 rather than treated as cosmetic: a summary
line is a contract. "All files passed" over a file with warnings teaches the reader that
both the summary and the warnings are noise — and once the tool has taught that, every
future diagnostic you add inherits the discount.

---

## #35 — object-typed local-state initializers vanish — **stays open, wave W3**

Confirmed. `$user object: { name: "Ada" };` compiles clean, the parse layer captures only
the scalar `$flag`, `"Ada"` appears nowhere in the build, and the binding still emits
`ST.resolve(__node, 'user')`. Repro: `tests/repro/gh-35-object-local-state/`.

Your fix direction is right and worth restating as a principle: don't add a
local-state-specific parser in Rust. The stdlib form already says `$value:expr`, so the
question isn't "how do we parse objects here", it's "why does the expression capture stop
early and why is stopping early acceptable?"

That second half is the real bug and it's shared with #37 and #36: a capture that consumes
part of its input and leaves the rest is treated as success. When the fix lands — a
bounded region must be fully consumed or produce a diagnostic — this case either parses
properly or fails loudly, and either is fine. Silence isn't.

---

## #36 — optional trailing literal clauses are still required — **stays open, worse than filed, wave W3**

Confirmed, and the repro found more than the issue claims: **no** `@wait_until` form
works. Full form, bare form, trivial form, parenthesized form — `inspect --layer emit`
finds zero `__timeout`/`pollInterval` in all of them, and `@wait_until false timeout: 500ms`
doesn't time out at all; it falls straight through to the next assertion. The shipped
`stdlib/testing/meta/wait-timing.test.st` fails 9/15 headless. Repro:
`tests/repro/gh-36-wait-until-optional/`.

So this is being upgraded: it's not only a grammar-optionality bug, it's a broken window
in the test library itself, and anything depending on `@wait_until` is currently
asserting nothing.

The grammar lesson is still the one you identified, and it's a good one: **"has a
default" and "may be absent" are different axes.** A capture's default says what value to
use when the clause is present but unspecified; it says nothing about whether the literal
`timeout:` may be omitted. Splitting into sibling forms doesn't rescue it either, because
greedy `expr`/`selector` captures let the shorter form swallow the longer one's keyword —
which is the same "consume everything you can, never check what's left" behaviour behind
#37 and #35.

---

## #37 — bare @data signal params parse as an empty list and corrupt the wire payload — **stays open, wave W3**

Confirmed precisely. `@data signal $cursor(x number, y number)` compiles with zero
diagnostics and emits `callParams = [] || []`, where the canonical `$add($text string)`
emits a populated array. At runtime the body can't bind `$x`/`$y`, throws, gets caught,
and falls back to raw arguments — so the wire payload changes shape with nothing anywhere
saying so. Repro: `tests/repro/gh-37-bare-signal-params/`.

This is the cleanest statement of the invariant the next wave is built around, so I want
to name it precisely with your example. `param_list` is a *repetition*: "zero or more of
`$binding type`". Given `x number, y number`, it matches zero items — which is a legal
outcome for a repetition — and the matcher accepts the match while non-empty input
remains unconsumed.

**Matching is not the same as matching everything.** A bounded region (the parentheses
here) must be fully consumed, or the match is a failure with a diagnostic showing the
expected shape (`$x number`). We are explicitly *not* broadening the grammar to accept
bare names, and the runtime fallback stays defensive but stops being load-bearing —
defensive runtime code is a seatbelt, never a substitute for the compiler telling you
you're wrong.

Your issue, #35 and #36 are the same defect wearing three costumes, and they'll be fixed
by one rule.
