#set document(title: "What Inference Is For", author: "Spacetime")
#set page(paper: "a4", margin: (x: 2.4cm, y: 2.5cm), numbering: "1", number-align: center)
#set text(font: ("Libertinus Serif", "Georgia"), size: 11.5pt, lang: "en")
#set par(justify: true, leading: 0.78em)
#show heading: set block(above: 1.7em, below: 0.85em)
#show heading.where(level: 1): set text(size: 15pt, weight: "bold")
#show heading.where(level: 2): set text(size: 12pt, weight: "bold", style: "italic")
#show raw.where(block: false): it => box(fill: luma(240), inset: (x: 3.5pt), outset: (y: 3.5pt), radius: 2.5pt, text(size: 9.8pt, it))
#show raw.where(block: true): it => block(fill: luma(247), inset: 10pt, radius: 3pt, width: 100%, text(size: 9.2pt, it))
#let big(b) = block(width: 100%, above: 1.5em, below: 1.5em, align(center, text(size: 13pt, style: "italic", b)))
#let note(b) = block(inset: (left: 14pt, right: 14pt), above: 1.2em, below: 1.2em, text(size: 10.3pt, style: "italic", fill: luma(60), b))

#v(1.5cm)
#align(center)[
  #text(size: 25pt, weight: "bold")[What Inference Is For]
  #v(0.5em)
  #text(size: 12.5pt, style: "italic")[The vision, and the measurements behind it]
  #v(1.4em)
  #text(size: 10pt, fill: luma(90))[Spacetime · FEAT-109 · #datetime.today().display("[month repr:long] [year]")]
]

#v(1.2cm)

#align(center)[#block(width: 80%)[#text(size: 10.5pt, style: "italic", fill: luma(70))[
  Every number in this document was measured this session, on this corpus, with
  the inferrer that now exists. Where something is a projection rather than a
  measurement, it says so.
]]]

#v(1cm)

= The one-sentence version

The compiler already knows what your values are. It throws that knowledge away
at the moment it captures them, and then four other subsystems each guess it
back, badly, in their own way.

#big[Inference is not a new capability. \ It is refusing to discard one you already have.]

= I. The measurement

I ran the inferrer over every `.st` file in the corpus — 456 files, 8,983
declaration values — with no annotations anywhere. What it can name, today:

#block(inset: (left: 10pt))[
```
  1278   14.2%  length        400px, 200px, 100px
   791    8.8%  color         #6b7280, #f59e0b, #10b981
   778    8.7%  number        1, 0.5, 0
   131    1.5%  easing        linear, ease-out, cubic-bezier(0.34, 1.56, …)
    15    0.2%  duration      600ms, 500ms, 200ms
     6    0.1%  angle         45deg, -135deg, 225deg
  ─────
  2999   33.4%  TYPED WITH ZERO ANNOTATION
```
]

Now the number that matters. How many `@type` annotations exist across
`examples/` and `demos/`?

#big[21.]

Two thousand nine hundred and ninety-nine values whose type is knowable, and
twenty-one places where anyone wrote one down. The type information in this
codebase is almost entirely *latent*: present in the source, derivable by
machine, and used by nothing.

That gap is the feature. Everything below is a consequence of closing it.

== What the other two thirds are

Honesty about the same table. `«unknown»` is 30.6% and `«reference»` 15%, and
those are not failures:

- *references* (`$card.price`, `--ink`) — correctly not literals. Their type is a
  resolution question, not a recognition one.
- *unknown* — mostly JS payloads and compound values my crude line-splitter
  picked up (`translateX(100%)`, `$.closest('product-card').dataset.price`). A
  compound value genuinely is not a scalar; the shorthand story is separate work.
- `«ambiguous: richtext|string|url»` at 18.9% is real and interesting: those are
  bare identifiers, where three shape-scalars fit and no meaning-scalar claims
  them. The inferrer reports the tie rather than picking. See §V.

So: a third is typed today, a third is deferred by design, a third is genuinely
plural or not a value. I would rather show you that table than a headline.

= II. What is being paid for the gap, right now

Four subsystems each answer "what kind of value is this?" independently. All
four are live in the tree today; none delegates to the others.

#block(inset: (left: 10pt))[
```
  src/sync/protocol.rs      classify_value()  — 86 lines, its own
                            hex/rgb/hsl/ms/px/ease-* recognizer, 19 tests
  src/lsp/colors.rs         380 lines, byte-scanning for `#`, `rgb(`,
                            with hand-rolled comment/string skipping
  src/analysis/visual_lint.rs   parse_css_color(), its own colour awareness
  src/color/mod.rs          the real colour PARSER — but not the DETECTOR,
                            so everyone re-derives "is this a colour?" first
```
]

Five columns, no shared row. This is a "one need, one implementation" violation
with four instances, and its cost is not the line count — it is that they
*disagree*, silently, and the disagreements surface as bugs that look unrelated
to each other.

The LSP file even documents its own history in a comment: it used to byte-scan
for `#`, then `rgba(`, then `rgb(`, with a hand-written hex validator that had
to be kept in agreement with the compiler's. `oklch()`, `hsl()`, `color-mix()`
were simply invisible to it until someone noticed.

#note[
  This is the thing worth internalising: nobody chose to write four colour
  detectors. Each was written because, at that point in the code, the type was
  not available — and it was not available because it had been discarded at
  capture. The duplication is a *symptom* of the discard.
]

= III. What it buys, concretely

Four things, in increasing order of how much they change the feel of the
language.

== 1. Deletion

Tier 0 of the plan is pure cleanup with zero behaviour change: one
`infer_value_type`, four call sites, three hand-rolled detectors deleted. The
LSP's 380 lines become a query. The 86-line `classify_value` becomes a
delegation.

Nothing new works. Things simply stop being able to disagree.

== 2. The admin stops guessing

Today the CMS picks its editing control like this — measured, `src/server.rs`:

```rust
fn widget_for(field_name: &str, node: &Value) -> String {
    let lname = field_name.to_ascii_lowercase();
    if lname == "slug" || lname == "id" { return "readonly"; }
    …
```

It looks at the field's *name*. Then it consults `format`, which exists only if
someone wrote an `@type`. So with 21 annotations in the corpus, the overwhelming
majority of values reach the admin as `"string"` and render as a text box.

A colour renders as a text box. A duration renders as a text box. A length
renders as a text box.

With inference, the value's own shape supplies the format, and
`stdlib/scalars/types.st` already maps format → widget as *data*:

```
  %scalar_type color    { %capture "color"    %widget "color" }
  %scalar_type length   { %capture "length"   %widget "range-length" }
  %scalar_type duration { %capture "duration" %widget "range-duration" }
```

So `#6b7280` gets a colour picker, `400px` gets a length slider, `600ms` gets a
duration control — with no annotation, in every one of the 791 + 1278 + 15
places those literals appear. The table is already written. It is starved of
input.

#big[The admin does not need to be taught what a colour is. \ It needs to stop being lied to about what it has.]

== 3. Errors that are currently impossible

This is the one I care most about. Colour algebra exists (`darken`, `mix`,
`alpha`), and its signature is `(color, number) → color`. Nothing enforces it,
because at the call site the argument's type is `Expr` — the opaque box
everything falls into.

```
  darken($brand.radius, 0.12)      // a LENGTH into a colour function
```

Today: no error. It runs, produces garbage, and the failure shows up as a
colour that is subtly wrong on a page, three files away, at runtime.

With inference: a compile error with a span pointing at the argument.

The same mechanism catches interpolation errors — animating between two
endpoints that cannot interpolate is currently a runtime shrug, and becomes a
compile error the moment both endpoints have types.

#note[
  Note what did *not* have to happen for this. No annotations were added, no
  signature language was extended, no new checker was written. The check was
  always expressible; the operand types were being thrown away before it could
  run.
]

== 4. The language stops needing Rust to grow

The acceptance test for the whole approach: a project declares

```
  %scalar_type currency {
    %capture "currency"  %schema "string"
    %format "currency"   %widget "currency"  %zero ""
  }
```

…in its own prelude, and immediately gets parsing, spanned errors, fetch-time
validation, an admin form control, and — once inference is wired — automatic
recognition of every `£12.00` in the source. Zero Rust.

That is what "the metasystem is self-describing" has to mean at the value layer,
and inference is the piece that makes a *literal* participate in it.

= IV. The shape of the thing that was built

One function, opposite polarity to the one that already existed.

```
  refusal      capture_type_accepts()   unknown → ACCEPT   never block a build
  recognition  infer_value_type()       unknown → ABSTAIN  never invent a type
```

Both run the *same grammars* — the `%capture_type` productions the
`%scalar_type` table already names. That shared substrate is what keeps them
honest with each other; it is the "no second parser" rule the original plan set
as its own risk mitigation, and it holds.

The reason two functions are needed rather than one: a validator that does not
know must let the value through, because a false refusal blocks a build the
author cannot fix. An inferrer that does not know must say nothing, because a
false type is worse than no type. One default cannot serve both.

Measured, before the split existed: naively reusing the validator for inference
reported `date` — a scalar row naming no production at all — as the unique type
of `"garbage ~~~ nonsense"`, and returned 17 of 19 sample literals as ambiguous
across up to thirteen scalars.

= V. Where it deliberately refuses

The failure mode of a type system that guesses is not noise — it is silent,
correct-looking wrongness. Two places where abstaining is the whole design.

== A bare number is not a time

`600` is a number. It is *not* a duration, even in a slot where a duration
belongs, because the unit lives at the declaration site rather than in the
spelling: `stagger: 0.05` means seconds, `duration: 600` means milliseconds.

A blanket "a number can be used as a time" once made every staggered animation
in the corpus a thousand times too slow — twenty files, silently, with a fully
green build. Nothing was wrong at any single site.

So the inferrer answers `number`, and a caller that wants a duration must supply
the context. It will not close that gap by guessing.

== Named colours do not infer, and that is a cost I am choosing

`chartreuse` comes back as a *tie* between the shape-scalars, not as a colour.
The 148 CSS named colours are deliberately absent from the grammar, because a
bare identifier in a value position is ambiguous with every other keyword CSS
defines — admitting them would make the grammar claim that `bold` and `hidden`
are colours.

This is a real limitation, it is pinned by a test, and the fix is
property-contextual grammar (knowing that the slot is `color:`), not a hardcoded
list of 148 names.

#note[
  Both refusals share a structure: the missing information is *context* — which
  slot, which declaration — and context is what a later tier supplies. Neither
  is a gap in recognition. Recognition did its job and stopped at its boundary,
  which is exactly what a total, local, one-pass mechanism should do.
]

= VI. Honest status

*Built and proven.* `src/types/value_infer.rs`. 18 gates, written RED before the
module existed, seeded from the probe's measured wrong answers. Each mechanism
was deleted in turn and the suite *seen to fail* — 8, 3, and 3 gates
respectively. Full lib suite 3209 passed / 0 failed, identical to baseline.

*Not wired.* Nothing calls it yet. `convert_property_value` still matches on the
declared type and ends `_ => Expr`, so the evidence is still discarded at
capture. The four shadow classifiers still each do their own thing.

∴ *the vision above is a projection, not a report.* The inferrer answers
correctly on 2,999 corpus values in a test harness. Zero of those answers reach
the admin, the LSP, or the type checker today.

*What is next, in dependency order:*

#block(inset: (left: 10pt))[
```
  Tier 0   four shadow classifiers delegate to one function     pure deletion
  Tier 1   convert_property_value stops discarding evidence     the keystone
  Tier 2   contextual inference from the declared param type    unlocks units
  Tier 3   value-level checking → darken(aLength) is an error   the payoff
```
]

Tier 1 is the keystone: it is the smallest change that turns this from a
function nothing calls into the thing the language runs on. Its acceptance test
is one sentence — *remove the `@type` from a brand seed and the admin still
renders a colour picker* — and when that passes, everything in §III.2 follows
without further work.

= VII. Why this is the right shape

The alternative — annotate everything — was available the whole time. It fails
for a reason worth naming: 21 annotations against 2,999 typeable values is not
laziness, it is a truthful signal about what people will actually write. A type
system that only works when you feed it is a type system that is off by default.

The values were always self-describing. `#6b7280` is a colour in every language
that has ever looked at it. The only thing that made it opaque was a `match`
arm ending in `_ => Expr`.

#big[Stop discarding what the source already says.]
