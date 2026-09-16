#set document(title: "What a Value Is", author: "Spacetime")
#set page(
  paper: "a4",
  margin: (x: 2.4cm, y: 2.6cm),
  numbering: "1",
  number-align: center,
)
#set text(font: ("Libertinus Serif", "Georgia"), size: 11.5pt, lang: "en")
#set par(justify: true, leading: 0.78em, first-line-indent: 0em)
#show heading: set block(above: 1.8em, below: 0.9em)
#show heading.where(level: 1): set text(size: 15pt, weight: "bold")
#show heading.where(level: 2): set text(size: 12pt, weight: "bold", style: "italic")
#show raw.where(block: false): it => box(
  fill: luma(240), inset: (x: 3.5pt), outset: (y: 3.5pt), radius: 2.5pt,
  text(size: 9.8pt, it),
)
#show raw.where(block: true): it => block(
  fill: luma(247), inset: 11pt, radius: 3pt, width: 100%,
  text(size: 9.5pt, it),
)
#let aside(b) = block(
  inset: (left: 16pt, right: 16pt), above: 1.4em, below: 1.4em,
  text(size: 10.5pt, style: "italic", fill: luma(60), b),
)
#let big(b) = block(
  width: 100%, above: 1.6em, below: 1.6em, inset: (y: 4pt),
  align(center, text(size: 13pt, style: "italic", b)),
)

#v(2cm)
#align(center)[
  #text(size: 26pt, weight: "bold")[What a Value Is]
  #v(0.6em)
  #text(size: 12.5pt, style: "italic")[
    A small question about CSS that turned out \ to be a large question about types
  ]
  #v(2em)
  #text(size: 10pt, fill: luma(90))[Spacetime · #datetime.today().display("[month repr:long] [year]")]
]

#v(3cm)

#align(center)[#block(width: 78%)[#text(size: 10.5pt, style: "italic", fill: luma(70))[
  This is written for someone who has never seen this codebase. It assumes you
  know what CSS is and roughly what a compiler does, and nothing else. There are
  no ticket numbers in it.
]]]

#pagebreak()

= I. A question that would not go away

Spacetime is a language for building websites. You write one file, and it
produces the HTML, the CSS, the animations, and the client-side behaviour. There
is no JavaScript layer to write by hand — that is the whole premise. If
something needs to move, or fetch, or remember, you say so in Spacetime and the
compiler emits whatever it takes.

Because it produces CSS, it lets you write CSS:

```
card {
  padding: 1.5rem;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0,0,0,.08);
}
```

And because it is its own language, it also lets you write things CSS has never
heard of:

```
card {
  color: --ink;
  width: $sidebar.width;
  opacity: 0 -> 1;
}
```

`--ink` names a design token. `$sidebar.width` reads a value that can change
while the page is running. `0 -> 1` is not a value at all but an animation — a
start and an end, with the arrow meaning *becomes*.

All four of those lines sit in the same block, and the compiler has to
tell them apart. Which means, somewhere in it, there is code that answers:

#big[Is this value CSS, or is it mine?]

The compiler validates CSS. It knows `color: chartreuse` is fine and
`color: chartroose` is a typo, and it will tell you so before you refresh the
page. But it must not point that machinery at `color: --ink`, because a CSS
validator has never heard of `--ink` and will reject it. A compiler that
refuses code its own author wrote correctly is worse than one that says nothing
at all: a missed warning costs you five minutes of debugging, and a false
rejection blocks a build you cannot fix.

So the question gets asked. And once you go looking, you find it has been asked
in an awful lot of places.

== The shape of the mess

Each place answered the question by looking at the characters:

```rust
if value.contains('$')  { /* not CSS */ }
if value.starts_with("--") { /* not CSS */ }
if value.contains("var(") || value.contains("env(") { /* not CSS */ }
```

Which is fine, in the way that a paper cut is fine. It works. It is also a rule
that lives in a dozen places and is written slightly differently in each, so it
drifts, and the drift is invisible until a build fails in a way nobody can
explain. Over a couple of years, the codebase had accumulated *eight* separate
hand-maintained lists of the form "here are the things that are Spacetime and
not CSS" — kept in sync by memory and good intentions.

The obvious fix is the one everybody reaches for. Stop asking about characters.
Give values *types*.

= II. The obvious fix, and where it stops

Say every value has a type. `12px` is a length. `chartreuse` is a colour.
`300ms` is a time. Then instead of scanning for dollar signs you ask the value
what it is, and route accordingly. This is not a clever idea; it is the ordinary
idea, and it is right about nine-tenths of the problem.

Then you get to `--ink` and the ground gives way.

What type is `--ink`? The honest answer is: it depends on somewhere else. It is
a *reference*. Somewhere, possibly in this file, possibly in a shared file,
possibly in a stylesheet this compiler never sees, something declares what
`--ink` is. Until you find that declaration you do not know whether it is a
colour, a length, or a font stack.

And you may never find it. The declaration might live in a file the build does
not include. It might be injected by a theme at runtime. In CSS this is normal
and intentional — the whole point of a custom property is that it can be
supplied later, by someone else, possibly by the user.

So here is the bind:

#aside[
  The compiler cannot say `--ink` is a colour, because it does not know.
  It cannot say `--ink` is *not* a colour, because it might be.
  And it cannot refuse to have an opinion, because it is being asked whether
  to validate the line.
]

The standard escape hatch is to invent a type meaning "I don't know" and place
it at the top of the hierarchy, where everything fits. Call it `Unknown`, and
say every value is an `Unknown`. But top means *this could be anything*, and a
type system will happily reason from that: if `--ink` is anything, and anything
includes colours, then `--ink` in a colour slot is fine — and so is `--ink` in a
length slot, and by the same reasoning, so is a colour in a length slot. Put
"could be anything" into an ordering and it flows through the whole ordering,
carrying nonsense with it.

What is wanted is something admitted everywhere that *proves nothing anywhere*.
That turns out to be a known idea — it is the heart of gradual typing, where a
dynamic value is *consistent with* every type without being a *subtype* of any
— and I will come back to it, because it arrives here from an unexpected
direction.

But first the ground shifts again, and it is worth saying why.

= III. The turn

I spent a while trying to work out what type `--ink` has. That was the wrong
question, and the tell was that it kept not having an answer.

Look at what the compiler actually knows. Not what I wish it knew — what it
has, at the moment it looks at those five characters. It knows this:

```
The text matched this rule:      "--" then an identifier
```

That is the whole of it. Everything else — colour, length, what it resolves to,
whether it resolves at all — is either downstream or unavailable. And yet that
one fact is enough to do the entire job: to validate correctly, to emit
correctly, to reject `--ink` in a slot that genuinely cannot take a reference.
The compiler was never blocked. I was, because I was looking underneath the
grammar for a type, and there is nothing underneath the grammar.

#big[The type of #raw("--ink") *is* "token reference". \ Not "a colour, spelled as a token reference".]

There is no second layer. The syntactic category *is* the semantic one.

This sounds like a distinction without a difference. It is not, and the
difference shows up immediately in what it costs to add something.

== Two worlds

*If types are primary and syntax is how you spell them*, then a new kind of
value needs: a new type, a new variant in the compiler's enumeration of types, a
new branch in every match over that enumeration, a new validation path, and an
update to every list of "things that are not CSS". That is a day's work spread
across a dozen files, and it is exactly where those eight hand-synced lists came
from. They were not sloppiness. They were the shape of the design.

*If syntax is primary*, a new kind of value needs a grammar rule. That is a row
of data. The compiler is not modified at all — it already knows how to match
grammar rules, and it will match this one the moment it is written.

The second world is not merely tidier. It is a different *kind* of system: one
where the interesting knowledge lives in data that a person can read and edit,
rather than in control flow that a person has to trace.

== So what is the checker checking?

If the type is the grammar rule, then "type checking" is asking which rule a
piece of text matches. That is parsing. It runs forward, in one pass, and it
always terminates with an answer.

This is worth dwelling on, because it is a large simplification hiding in a
small sentence. The famous type inference algorithms — the Hindley–Milner family
that ML and Haskell and, in spirit, TypeScript descend from — exist to
reconstruct a type *that nobody wrote down*. You wrote `f x`, and the algorithm
gathers constraints from every use of `f` and `x` across the program, and then
solves them, and the solving can fail in ways that are famously hard to explain.

Spacetime never needs that, because in Spacetime the type is always written
down. It is written in the sigil. `--ink` announces itself. #raw("$sidebar.width")
announces itself. There is nothing to reconstruct.

So we do infer, in the ordinary English sense — the author wrote five characters
and the compiler concluded something — but the mechanism is *recognition*, not
deduction. No constraint store. No solver. No failure mode where the checker
shrugs.

== The one thing that genuinely needs solving

Not everything is local. `--ink` is a token reference here and a colour over
there, and getting from here to there means following the name to its
declaration.

That is a real problem, but notice it is a different *kind* of problem. It is a
lookup — the same shape as resolving an import — not a constraint system. And it
is allowed to fail: the declaration may be in a file we do not have.

Two mechanisms, then, and keeping them apart is most of the clarity:

#aside[
  *Recognition* — text to grammar rule. Total, local, one pass, always answers.

  *Resolution* — name to declaration. Partial, global, may not answer at all.
]

Conflating these is exactly what made `--ink` feel paradoxical. A value whose
declaration cannot be found is still perfectly *recognised*, and almost
everything the compiler does needs only recognition. The paradox was an artifact
of asking one question in place of two.

= IV. A character that lied

Here is a story that is both a good bug and a demonstration of the thesis,
because it is what happens when the recognition step gets one character wrong.

Spacetime has *holes*. A hole is a place in the markup where a value gets
substituted when the page renders:

```
<a href=`$card.url`>`$card.title`</a>
```

Backticks. Same character JavaScript uses for template strings, used here for
something related but different: this is a hole, fill it in. There are about
five thousand of them across the codebase. It is one of the most common
characters in the language.

Now, a compiler's first step is to chop text into tokens — the smallest
meaningful units. `12px` is a number-with-unit. `{` is a brace. `color` is an
identifier. Every sigil in Spacetime gets its own token: `$`, `&`, `@`, `%`,
`#`, and the two-character ones like `->`.

Every sigil except the backtick, which had no rule at all.

So what happened to it? The lexer had a catch-all for unrecognised characters,
and the catch-all did something quietly catastrophic: it labelled the byte as a
*minus sign*. Not as an error — as a minus. There is a comment in the source
that says `/* placeholder */`, which is the sound of someone intending to come
back later.

Downstream, there is a perfectly sensible rule that says: a minus followed
immediately by an identifier is a *vendor prefix*, like `-webkit-transform`, and
should be glued into a single identifier. That rule now applied to backticks.

The result, measured:

```
input:     a `b` c

tokens:    IDENT("a")   IDENT("`b")   MINUS("`")   IDENT("c")
```

Read that middle token again. The opening backtick has been *absorbed into the
word after it*. The closing one is loose, wearing a minus sign's name. There is
no hole here any more. There never was one, as far as anything downstream could
tell.

And things downstream cared. Any code counting braces to find the end of a block
was counting braces inside holes it could not see. There was a known bug filed
about it, with a workaround, and the workaround had a workaround.

== The fix I got wrong first

The obvious reading of that bug: the language has no token for template-literal
syntax, so add one — a token that starts at a backtick and runs to the next
backtick, swallowing everything between, the way a string literal does.

I built exactly that. It passed every test I wrote for it. Then I ran it against
the actual codebase, and:

```
   examples     35 errors  ->  55
   demos        36 errors  ->  82
   test suite   3100 pass  ->  3061 pass, 130 fail
   page tests   744 pass   ->  the runner stopped working entirely
```

Which, once you see it, is funny. A token that runs from one backtick to the
next does not make template literals opaque. It eats every hole in the language,
and everything between any two of them.

The bug report was right about the defect and wrong about the fix, because it
had been written while thinking about JavaScript payloads — where a backtick
really *is* a delimiter — and in Spacetime the same character means something
else and something far more common.

The actual fix is one line, and it is the thesis in miniature: *the backtick is
a sigil, so it lexes like a sigil.* One character. Non-consuming. Exactly like
`$` and `&` and `@`. It does not decide how far a hole reaches; the grammar
decides that, as it already did.

And the second fix is the one that mattered more: unrecognised bytes now get
their own category, and are reported as errors. A placeholder that borrows
another token's identity cannot be contained downstream. It can only be
apologised for, in comments, in the files that inherit the confusion.

#big[A token that lies about what it is \ makes every later question unanswerable.]

Which is the whole argument for taking syntax seriously, delivered by a
counterexample.

== And then the good part

With the backtick lexing as a token, a hole could — for the first time — be
written as a grammar rule:

```
hole = "`" ( binding | name ) "`"
```

Before, it could not be, in a very literal sense: a grammar matches tokens, and
there was no token to match. The hole had to be recognised by scanning
characters in the compiler, because it was inexpressible in the language's own
description of itself.

Now look at what a hole *means*. Every other rule answers *what is this?*
A hole answers *ask me later*. Its content is decided when the page renders.

Which is precisely the "admitted everywhere, proves nothing anywhere" that
Part II went looking for. It did not have to be invented and bolted on as a
special position in a hierarchy. It arrived as a syntactic form, because in this
language deferral *has* a spelling, and the spelling is a backtick.

That is the most satisfying thing I have found in this work, and I want to be
careful not to oversell it — but it is the kind of result that suggests the
framing is load-bearing rather than decorative.

= V. The failure that proved the point

One more story, because it is where the idea nearly broke, and how it broke is
instructive.

Having written grammar rules for all the Spacetime forms, I deleted the
character-scanning check. Tests passed. Then the corpus:

```
demos    36 errors  ->  46
```

Ten new errors, all variations of:

```
error: `1px solid $brand.rule` is not a valid color for `border`
```

`border` is CSS shorthand — a width, a style, and a colour in one line. Here the
colour slot is a live value. This is completely ordinary Spacetime and it had
worked forever.

Why did the old code get it right? By *accident*. `value.contains('$')` does not
care where the dollar sign is. It is a crude test, and its crudeness happened to
cover a case nobody had thought about.

Why did the grammar get it wrong? Because every rule I had written anchored at
the *start* of the value. `--ink` matched. `$brand.rule` matched. `1px solid
$brand.rule` matched none of them, fell through to "this is CSS", and got handed
to a CSS validator that quite reasonably objected.

And this is the bad kind of error — the kind that stops a build with a complaint
the author cannot act on, about code that is correct. Strictly worse than the
missed warning the check existed to prevent.

The fix was not to bring back the character scan. It was to notice that the old
code *knew something* that had never been written down: *a Spacetime value
anywhere inside a CSS value makes the whole thing not-CSS.* Obvious once
stated. Never stated, in eight years, in eight lists.

So it got stated, as a rule, in the same file as the others. Which is the
argument for this whole exercise, arriving from the least glamorous direction
possible: not "the grammar is more elegant", but *the grammar made me say out
loud a rule I did not know I was relying on.*

#aside[
  Every one of those eight deleted lists encoded some knowledge like this. The
  lists were not the problem. The problem was that the knowledge was only ever
  written as code that happened to work, and so it could only be recovered by
  breaking it.
]

= VI. What I am not sure about

Five things I would want to argue through before building further. These are
genuinely open, not rhetorical.

== 1. Should "this fits there" be declared or computed?

Some values are acceptable where others are wanted — a length works where a
length-or-percentage is expected. Call that relation *fits*.

If types are grammar rules, `fits` has a natural definition: rule A fits rule B
when everything A matches, B also matches. Language inclusion. For rules this
small it is computable, so `fits` could be *derived* rather than written down —
no hand-maintained table, which is very much the spirit of everything above.

But derived means *accidental*. Two rules that happen to overlap become related
whether or not anyone meant it, and the relation shifts silently when someone
edits an unrelated rule.

Declared means intentional but fallible. My instinct: declare it, and have the
build *check every declaration against the computed answer*. Then a wrong claim
is impossible and an unintended one never appears. That is better than either
alone — but I have not built it, and "check it against the derivation" is the
kind of sentence that hides a week.

== 2. Is a component a function?

Spacetime has forms — reusable parameterised things you invoke:

```
@fade-in(duration: 400ms, easing: --ease-out)
```

That has a name, typed parameters, and a result. It is a function, and invoking
it is application. Which raises a question with real teeth: *are the ordinary
rules about function types the right rules here?* Function subtyping is the one
place in type theory where the answer is counterintuitive (parameters vary one
way, results the other), and it is exactly the place where getting it wrong
produces a system that seems fine and quietly is not.

And if a form is a function type, what is a macro? A lambda? Is the language's
composition operator function composition?

I do not know whether that is a deep truth or a seductive analogy. It is the
question I most want to spend a day on, and the one I am least willing to guess
at.

== 3. Where does the unit live?

A measured, annoying fact. In one place `stagger: 0.05` means five hundredths of
a *second*. In another, `duration: 600` means six hundred *milliseconds*. Same
bare number, different unit, and the difference is a property of the *place it
is written*, not of the value.

This is the sharpest evidence against a naive reading of everything above. It is
a fact about a value that is genuinely not in the value's syntax.

It is also a trap with a body count. "A number can be used as a time" is locally
reasonable and globally catastrophic: a blanket numeric change once made every
staggered animation a thousand times too slow, across twenty files, silently,
with a green build. Nothing was wrong at any single site.

So the thread: should the *declaration* carry the unit, so the fact comes back
into the syntax where the rest of the type lives? `duration: 600ms` in the
signature rather than a convention in a doc comment. That is a change to the
language, not to the compiler, which is why it needs a conversation and not a
ticket.

== 4. Does this make the language easier or harder to learn?

The governing rule for Spacetime's design is: *knowing one thing should tell you
the next.*

Types-as-syntax is a strong claim in that direction. If every category is a
visible form, reading a value tells you its type with no lookup. `--x` is a
reference, #raw("`x`") is a hole, `$x` is live, `&x` is an identity. Four
sigils, four categories, nothing to memorise.

The risk is the exact inverse. If the type *is* the spelling, then changing a
type means changing the spelling — every retype becomes a rename across the
whole codebase. That is not hypothetical: retiring one sigil in favour of
another recently touched four hundred and two sites.

== 5. Which makes migration load-bearing

If types are syntax, then *changing your mind about a type is a text
transformation across a corpus*, and a language that makes that dangerous will
be a language whose types nobody ever improves.

So the migration story stops being a nicety. It wants to be checkable: given an
old type and a new one, is this change strictly a *widening* — does everything
that was legal stay legal? If yes, it is provably safe and needs no review. If
no, the compiler should be able to hand you the list of sites that will break,
*before* you make the change rather than after.

Under the ordinary reading, that is a nice-to-have. Under this one it is
structural, and it is the piece I would build first after settling question 1.

= VII. Why this might be wrong

The honest failure modes, since a document that only argues one way is
advertising.

*It may not survive contact with resolution.* Everything above is clean because
recognition is clean. The moment you need to know what `--ink` actually resolves
to — for a real diagnostic, a real optimisation — you are back in a global,
partial, failure-prone lookup, and the tidiness of the local story may just be
tidiness about the easy half.

*Grammars are not free.* Twice now I have wanted a rule the grammar could not
express, and had to stop. One of those turned out to be a bug in the grammar
engine; the other might be a real expressiveness limit. A system whose extension
point is "write a rule" is only as good as what a rule can say, and I have
already found the edge of that twice in a week.

*"Syntax is the type" may be true of this language and no other.* CSS values are
unusually syntactic — the surface really does carry most of the meaning. In a
language with real computation, values are produced by expressions and there is
no syntax to look at. I do not think this generalises, and I would be suspicious
of myself if I started thinking it did.

*And the migration risk is real, not theoretical.* Four hundred sites for one
sigil. If types-as-syntax makes types harder to change, it will make them worse
over time, and no amount of elegance at the front pays that back.

= VIII. Where it stands

The character-scanning check is gone. The rules that replaced it live in one
file, as data, where they can be read. The backtick is a token, so a hole can be
described in the language's own terms for the first time. The compiler behaves
identically on the corpus, which is the only evidence that matters — five
thousand pages of unchanged output.

What is not built is everything in Part VI. That is deliberate. Questions 1 and
2 change what gets built, so building first would be guessing with extra steps.

The one thing I would carry out of this if the rest turned out to be wrong is
smaller and more portable than the thesis:

#big[
  A system's important knowledge should be written somewhere \ a person can
  read, rather than encoded in control flow \ that only reveals itself when
  broken.
]

Eight lists said that. So did a minus sign wearing a backtick's clothes. So did
a rule about compound values that nobody had ever written down, and that could
only be recovered by breaking a build.
