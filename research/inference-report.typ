#set document(title: "Teaching a Compiler to Read", author: "Spacetime")
#set page(paper: "a4", margin: (x: 2.4cm, y: 2.5cm), numbering: "1", number-align: center)
#set text(font: ("Libertinus Serif", "Georgia"), size: 11.5pt, lang: "en")
#set par(justify: true, leading: 0.78em)
#show heading: set block(above: 1.7em, below: 0.85em)
#show heading.where(level: 1): set text(size: 15pt, weight: "bold")
#show heading.where(level: 2): set text(size: 12pt, weight: "bold", style: "italic")
#show heading.where(level: 3): set text(size: 11.5pt, weight: "bold")
#show raw.where(block: false): it => box(fill: luma(240), inset: (x: 3.5pt), outset: (y: 3.5pt), radius: 2.5pt, text(size: 9.8pt, it))
#show raw.where(block: true): it => block(fill: luma(247), inset: 10pt, radius: 3pt, width: 100%, text(size: 9pt, it))
#let big(b) = block(width: 100%, above: 1.5em, below: 1.5em, align(center, text(size: 13pt, style: "italic", b)))
#let note(b) = block(inset: (left: 14pt, right: 14pt), above: 1.2em, below: 1.2em, text(size: 10.3pt, style: "italic", fill: luma(60), b))

#v(1.8cm)
#align(center)[
  #text(size: 25pt, weight: "bold")[Teaching a Compiler to Read]
  #v(0.5em)
  #text(size: 12.5pt, style: "italic")[How a language got type inference, \ and what went wrong on the way]
  #v(1.6em)
  #text(size: 10pt, fill: luma(90))[Spacetime · #datetime.today().display("[month repr:long] [year]")]
]

#v(1.4cm)
#align(center)[#block(width: 82%)[#text(size: 10.5pt, style: "italic", fill: luma(70))[
  Written for someone who has never seen this codebase. It assumes you know what
  CSS is and roughly what a compiler does. Every number was measured; where
  something is unverified, it says so.
]]]

#v(1cm)

= I. The setting

*Spacetime* is a language for building websites. You write one file and it
produces the HTML, the CSS, the animations and the client-side behaviour — no
hand-written JavaScript. Because it emits CSS, it lets you write CSS:

```
card {
  padding: 1.5rem;
  color: #6b7280;
  transition: 600ms;
}
```

A compiler reading that has, for one moment, complete knowledge. `#6b7280` is a
colour — it could not be anything else. `1.5rem` is a length. `600ms` is a
duration. This is not inference in any clever sense; it is *reading*.

Then it threw all of it away.

The function that captures a value ended like this:

```rust
match capture_type {
    CaptureType::Color => CapturedValue::Color(text),
    CaptureType::Time  => CapturedValue::Time(ms),
    ...
    _ => CapturedValue::Expr(text),      // ← everything else
}
```

`capture_type` is the type someone *declared*. If a form said `:color`, the
value was a colour. Otherwise it became `Expr` — an opaque string. And since
declarations are optional and rare, "otherwise" was almost always.

#big[The compiler knew. It wrote the answer down. \ Then it dropped the paper and walked away.]

== What that cost, measured

Four subsystems each needed to know whether a value was a colour. None could
ask, so each guessed from the string:

#block(inset: (left: 10pt))[
```
  the sync layer        86 lines: hex scan, rgb/hsl prefix test, ms/s
                        suffix test, unit list, ease-* prefix test
  the editor plugin     379 lines of byte scanning
  the contrast linter   its own colour parse
  the colour module     the canonical parser — which both of the
                        above bypassed, because it was the weakest
```
]

Five columns, no shared row. And they disagreed:

#block(inset: (left: 10pt))[
```
  #e8eef7ff   8-digit hex   sync: not a colour   ·   others: a colour
  #abcd       4-digit hex   sync: not a colour   ·   others: a colour
  #zzz        not hex       sync: A COLOUR — it counted characters
                            (length == 4) without ever looking at them
  oklch(...)                sync: invisible; it knew `rgb` and `hsl`
  chartreuse                canonical parser: "cannot resolve"
```
]

Nobody chose to write four colour detectors. Each was written because, at that
point in the code, the type was not available — and it was not available because
it had been discarded at capture. The duplication was a *symptom*.

= II. How big the gap was

Before building anything I measured the whole corpus — 456 files, 8,983
declaration values — asking what a compiler could name with no annotations at
all:

#block(inset: (left: 10pt))[
```
  1278   14.2%   length      400px, 200px, 100px
   791    8.8%   color       #6b7280, #f59e0b, #10b981
   778    8.7%   number      1, 0.5, 0
   131    1.5%   easing      linear, ease-out, cubic-bezier(...)
    15    0.2%   duration    600ms, 500ms, 200ms
     6    0.1%   angle       45deg, -135deg, 225deg
  ─────
  2999   33.4%   knowable with zero annotation
```
]

Then the number that decided the design. How many type annotations exist across
the example and demo sites?

#big[21.]

Two thousand nine hundred and ninety-nine values whose type is knowable; twenty-one
places where anyone wrote one down.

That ratio is not laziness. It is a truthful signal about what people will
actually write, and it rules out the obvious alternative — *annotate
everything*. A type system that only works when you feed it is a type system
that is off by default.

#note[
  The other two thirds are honest too. 15% are *references* (`--ink`,
  `$card.price`) whose type is a lookup, not a reading. 30% are compound values,
  JavaScript payloads and things the grammar does not model. 19% are bare
  identifiers where several shape-types fit and none is more specific. Only the
  first third is a gap; the rest is the system correctly saying "not my
  question".
]

= III. The thing that made it hard

There was already a function that ran the grammars: `capture_type_accepts`. The
obvious move was to reuse it — for each known type, ask "does this value match?"
and see which one answers yes.

Measured, it returns garbage. 17 of 19 sample literals came back matching
several types at once, and the only two *unique* answers were both wrong: the
type `date` was reported as the sole type of `"1px solid red"` and of
`"garbage ~~~ nonsense"`.

The cause is not a bug in any grammar. It is that the existing function answers a
different question, and answers it correctly:

#block(inset: (left: 10pt))[
```
  capture_type_accepts()   "should I REFUSE this value?"
  infer_value_type()       "what IS this value?"
```
]

Every terminal branch of the first defaults to *accept*. An unknown type name is
"not ours to refuse". A reference is accepted by every type deliberately. Those
defaults are right, because a false refusal blocks a build the author cannot fix
while a missed warning costs five minutes.

They are exactly inverted for the second. An inferrer that does not know must say
nothing, because a false type is worse than no type.

#big[Same grammars. Opposite default. \ One function cannot hold both.]

So inference became a second function over the same grammar definitions — which
is what keeps the two honest with each other, rather than a fifth hand-written
detector.

== Where I was wrong twice, and it was useful both times

*Deriving precedence.* When two types both match, one usually should win —
`currentColor` is a colour, not merely a string. I tried to derive that from the
grammars: the type accepting *more* values is less specific, so it loses. Clean,
self-maintaining, no list.

It does not work. Measured: `string` accepts `chartreuse` and refuses `#FF0020`;
`color` accepts `#FF0020` and refuses `chartreuse`. Neither contains the other —
they *overlap*. Inclusion is not expensive here, it is the *wrong relation*: it
answers "incomparable" for exactly the pairs that need an answer. The attempt
took the test suite from 17/18 to 13/18.

What makes `color` beat `string` is not that its language is smaller. It is that
`color` is a claim about *meaning* and `string` is a claim about *shape*, and a
meaning beats a shape. That is a judgement, and a judgement has to be declared.

*A wrong expectation.* I asserted that `chartreuse` infers as a colour. It does
not, deliberately: the 148 named CSS colours are absent from the grammar, because
a bare word in a value position is ambiguous with every other CSS keyword —
admitting them would make the grammar claim that `bold` and `hidden` are colours.
The test was wrong, not the code. It now pins the real contract: the answer may
be plural, but it must never *claim* colour.

= IV. What was built

Five waves. Each had its tests written first, was seen to fail, and had to delete
something.

== Wave 1 — five answers become one

The sync layer's 86-line recognizer, the editor's byte scanner, the linter's
parser and the canonical parser's eleven hand-written colour names: gone. One
grammar decides *what a value is*; one primitive computes *what colour it is*.

Those are genuinely different questions, and separating them mattered. Detection
belongs to the grammar. Conversion belongs to the colour module and deliberately
accepts *more* — contrast maths must work on `chartreuse` even though a bare word
is not inferable as a colour.

#note[
  One planned deletion did not happen, and the reason is worth keeping. I assumed
  the editor plugin held a hand-rolled colour recognizer. Reading it: it finds
  `#`-runs and balanced `name(...)` spans and delegates the verdict, keeping no
  list of function names on purpose. The scan stays — the editor runs on live,
  frequently invalid documents where a full compile may produce nothing, and
  swapping the scan for one would make swatches blink out mid-keystroke. A
  regression dressed as a purification.
]

Along the way the grammar turned out not to know `color-mix()`, `light-dark()` or
`device-cmyk()`. Fixed where such things belong — in the stdlib data file, not in
Rust.

== Wave 2 — the admin stops guessing from field names

Spacetime ships a content admin. It chose its editing controls like this:

```rust
fn widget_for(field_name: &str, node: &Value) -> String {
    let lname = field_name.to_ascii_lowercase();
    if lname == "slug" || lname == "id" { return "readonly"; }
    ...
```

It looked at the field's *name*, then at a format that only exists if someone
wrote an annotation. With 21 of those in the whole corpus, a colour rendered as a
text box.

Now the value gets a vote, and the acceptance test the feature was originally
filed with passes: a colour palette with *no type declaration at all* renders
colour pickers, length sliders and duration controls.

No widget logic was added. The mapping from type to control already existed as
data:

```
%scalar_type color    { %capture "color"    %widget "color" }
%scalar_type duration { %capture "duration" %widget "range-duration" }
```

It was starved of input.

*Order is the design.* Inference runs *after* every declared signal — a
declaration is intent and must never be overridden — and *before* the
name-guesses, because evidence about the actual value beats a guess about its
label. A field called `icon` holding `#6b7280` is a colour, whatever it is
called.

== Wave 3 — where the unit lives

An annoying, load-bearing fact: `stagger: 0.05` means five hundredths of a
*second*, and `duration: 600` means six hundred *milliseconds*. Same bare number,
different unit, and the difference is a property of the *place it is written*.

Reading a literal can never settle that. But the declaration already did:
`$duration:duration` was written before the value was ever read. So inference
gained a second form that takes the slot:

#block(inset: (left: 10pt))[
```
  "600"     in a duration slot  →  duration    the slot supplies the unit
  "600"     with no slot        →  number      nothing said otherwise
  "100%"    in a length slot    →  length      a real tie, broken by context
  "#FF0020" in a duration slot  →  COLOUR      a type error to report, never
                                               a value to reinterpret
```
]

That last line has a body count. A blanket "a number can be used as a time" once
made every staggered animation in this codebase a thousand times too slow —
twenty files, silently, with a fully green build, because nothing was wrong at
any single site. The rule is safe here and was catastrophic there because it is
*ternary*: it depends on the value, the slot, and the fact that a slot was
written down at all. As a global relation it applies everywhere, including where
nobody asked.

=== The wave where testing earned its keep

After eleven passing tests I deleted two mechanisms to check they mattered.

Both deletions left the suite fully green.

Writing the missing tests immediately found a real bug. I had derived
"this slot supplies a unit" as "this type refuses a bare number" — and `color`
refuses `600`, so `600` in a colour slot was being *called a colour*. A slot was
supplying a *kind*, not a unit: precisely the widening the wave exists to forbid.

The fix was to ask a positive question instead: a type is dimensional when its
grammar accepts the same magnitude in several unit spellings (`1px`, `1rem`,
`1s`, `1deg`) and refuses it bare.

#note[
  The second deletion still passes, and I left it that way with a note in the
  source saying so. That check is genuinely redundant — two callees already
  refuse an unknown name. Keeping it is defence in depth; *pretending* it were
  load-bearing would be the lie.
]

== Wave 4 — the payoff, and a bigger finding

The goal: make declared parameter types mean something. A colour handed to a
function expecting a length should be an error, not garbage that surfaces as a
visual bug three files away.

The plan named a specific validator. Before writing code I measured whether it
runs:

#big[Preset definitions produced across the entire corpus: *0*.]

That validator walks a collection that is always empty — for *every* file,
including ones containing the exact syntax it looks for. Its argument-count
check, its unknown-function check and its "did you mean?" suggestions have never
executed on a real file. Filed as a bug, with the reproduction and three
candidate causes to check before touching anything — one of which was "delete the
file".

Adding a type check there would have shipped a gate incapable of failing. So the
wave moved to a surface that does run.

Where it landed, something was already waiting: a diagnostic code named
*"parameter type mismatch"*, a fully-formed error variant carrying which
parameter, what was expected and what was found, complete with rendering and
formatting — and *nothing in the codebase had ever constructed it*. It could not
be constructed, because at the call site the argument was an opaque string with
nothing to compare.

It is connected now. A colour handed to a parameter declared `number` was silent
before; it names the parameter, the expected type and the found type now.

*Five of that wave's eight tests exist to prove it stays quiet*: on references
(whose type lives in another file), on values the grammar does not model, on the
bare-number idiom the whole corpus uses. A validator that cries wolf is worse
than none, because the first thing anyone does is stop reading its output.

#note[
  That wave shipped with a flaw of exactly the kind it had just diagnosed, and
  Wave 6 is the story of finding it.
]

== Wave 5 — paying my own debt

The precedence rule from Part III — declared, not derived — had to live
somewhere, and I had put it in Rust. That is the ninth hand-maintained list in a
project whose recent history is deleting eight of them.

It moved into the table where the types are declared:

```
%scalar_type string {
  %capture  "string"
  %widget   "text"
  %loses_to "color easing length duration time angle percentage number"
}
```

Adding a new type now requires no compiler change to participate. Cost: one
field, one parse branch, four data rows, minus 2,877 characters of Rust.

== Wave 6 — the bug I filed, and the one hiding under it

Wave 4 left a bug on the table: a validator that has never run. Going back for
it turned up three more layers of the same thing, one of them mine.

*Layer one — the reported bug.* The construct that validator was built for is
retired. Every spelling of it now yields nothing: no definition, no diagnostic,
no output. Measured across five variants and a corpus file containing two of
them verbatim. So the answer was not to repair the walk but to delete it — which
was, as filed, one of the three candidate causes.

*Layer two — worse than reported.* The walk matched on one variant of the value
enum. That variant is *never constructed anywhere in the codebase*: four mentions
in the whole tree, every one of them either the declaration or a `match` arm.
Dead twice over. Even with a populated list, not one check could have fired.

*Layer three — mine.* Wave 4's checker had no caller outside its own unit test.
It was correct, it was tested, and it never ran. The exact defect I had just
filed a bug about, reproduced by the fix for it, four days later.

#big[A gate that calls the checker directly cannot tell \ "checks correctly" from "checks correctly and never runs".]

That sentence is the whole lesson, and it is now the reason the replacement gates
assert through the command-line tool's own diagnostic path rather than by calling
the function.

=== Why turning the old checks back on would have been a disaster

The tempting fix is to point the dead validator at live data. Measured against
the corpus — 954 files, 25,160 declaration lines:

#block(inset: (left: 10pt))[
```
  calls whose name the registry knows      :   780
  calls whose name it does NOT know        : 5,915   (132 distinct names)
     var 4187 · clamp 459 · calc 209 · translateY 158 · repeat 94
     linear-gradient 69 · cubic-bezier 66 · minmax 65 · blur 60 ...
```
]

Five thousand nine hundred and fifteen false "unknown function" warnings, on a
corpus that compiles and ships. The argument-count half is no better: 189 of
1,200 calls to *known* names would warn, nearly all of them either JavaScript
living legitimately inside a page, or `oklch(0.9 0 0)` — space-separated CSS
syntax that a comma-counting check reads as one argument instead of three.

And where that Rust table overlapped the standard library, *every single
overlapping entry disagreed with the declaration it duplicates*:

#block(inset: (left: 10pt))[
```
  noise     Rust says 2..2      stdlib declares 0..2
  parallax  Rust says 1..2      stdlib declares 0..2
  random    Rust says 2..2      stdlib declares 0..3
  wave      Rust says 1..4      stdlib declares 0..4
```
]

Five of five, silently wrong — harmless only because nothing read them. The
remaining entries were CSS colour functions the grammar already owns. A second
spelling of a declaration that lives in the standard library is the same
hand-synced list this whole arc has spent its time deleting; the ninth one, and
the one I nearly wrote a validator to defend.

=== Where it actually landed

The surface where a declared parameter and a written argument genuinely meet is
the loop that *already* refuses a misspelled parameter *name*. The name check was
there; the type check was not. They are the same question asked of the same
declaration, so they are now adjacent lines:

#block(inset: (left: 10pt))[
```
error[E173]: argument `duration` of driver `time` is declared `number`,
             but `#FF0020` is a color
  = hint: pass a number value, or change the parameter's declared type
```
]

Net across the compiler: 122 lines added, 963 deleted. Two dead modules gone,
and a check that runs.

=== The mutation that justified the whole exercise

Twelve gates, written to fail first. Disabling the new check fails exactly the
three that assert it fires, and none of the nine that assert its silence — the
right signature.

Then I removed one three-line exemption: the rule that a duration literal in a
numeric time slot is not a mismatch. One gate failed — the one that compiles
*every shipped example and demo* — with sixteen false refusals, most of them in
the language's own tutorial.

That is the stagger incident again, at a different address. A parameter declared
`number` whose authors write `600ms`: both mean the same milliseconds, because
the unit belongs to the declaration site, not to the spelling. Without that
exemption the type checker would have refused the tutorial — and a unit-level
gate would never have noticed.

= V. Numbers

#block(inset: (left: 10pt))[
```
  10 commits

  96 tests, each written before the code it guards:

     value inference          20      contextual inference     13
     capture path             15      admin widgets            12
     argument type checks     12      colour conversion         5
     classifier agreement      7      seed plumbing             5
     editor swatches           7

  unit suite       3203 passed / 0 failed
  integration       217 passed / 0 failed
  example sites      12 files with errors — unchanged
  demo sites          8 files with errors — unchanged
  test fixtures       6 files with errors — unchanged
```
]

The unit count *fell* by eleven, from 3,214. Those eleven tested the deleted
Rust function table — the one whose every overlapping entry contradicted the
standard library. They passed for years. They were testing that a wrong answer
was being given consistently.

The last three lines are the ones that matter. Each was measured twice: once on
this work, once on a pristine checkout of the starting commit in a separate
working tree, by the same command. Identical. Every compiled page produces the
same output it did before. The compiler now knows a great deal more and has not
changed its mind about anything.

#note[
  A second agent was editing the same repository throughout, in files this work
  never touches — and at one point its uncommitted state did not compile. Every
  verification above was therefore run in an isolated working tree containing
  only these changes. It is the difference between "the build is green" and "the
  build is green *because of what I did*".
]

= VI. What is honestly not done

*One more layer of the same defect is still there.* While deleting two dead
validators I found a third uncalled function — in a module Wave 6 does not
otherwise touch. It is recorded, not fixed: bundling an unrelated deletion into
this change would have made it harder to review, which is how dead code gets
added in the first place.

*Three types in the table point at grammars that do not exist.* They are excluded
from inference by a guard that is really a workaround for malformed data. The
guard is load-bearing and tested; the malformed rows remain.

*Three inferable types have nowhere to go.* Angles, percentages and easings are
recognised correctly and then dropped, because the value carrier has no slot for
them. Adding one variant per type is exactly the closed-enum rot this codebase
keeps deleting; the right fix is one general carrier, and it is not written yet.

*Named colours still do not infer*, and the fix is property-context (knowing the
slot is `color:`), never a hardcoded list of 148 names.

*The corpus scan is deliberately crude.* It splits on `:` and picks up
JavaScript payloads. It sizes a population honestly; it is not a parser, and the
30% "unknown" bucket flatters nobody.

= VII. What I would keep

Four things, in order of how portable they are.

*A system's important knowledge should live where a person can read it.* Eight
deleted lists said that. So did a precedence table I wrote in Rust and had to
move six hours later — and a ninth, a Rust copy of standard-library signatures,
whose every overlapping row had drifted into contradicting its original while
eleven tests watched.

*Refusing and recognising are different questions and need opposite defaults.*
Conflating them produced an inferrer that called nonsense a `date`. Once split,
both got simpler.

*A test that has never failed is not yet a test.* Two mechanisms in Wave 3 could
be deleted with the suite still green — and the tests written to close that gap
found a real bug on their first run. The discipline is not "write tests"; it is
*watch them fail before you trust them*.

*And ask the harder version of the question: not "does it pass?" but "could it
fail?"* Three separate layers here were correct, tested, and unreachable. Passing
tests told me nothing about any of them, because a test that calls a function
directly has already assumed the thing most likely to be untrue — that something
else calls it too. I wrote the third layer myself, four days after filing a bug
about the first two. Knowing the failure mode was not enough; only asserting
through the path a user actually travels was.

#v(0.5em)

#big[The values were always self-describing. \ The only thing that made them opaque \ was a line of code that said `_ => Expr`.]
