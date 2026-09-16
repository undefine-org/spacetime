#set document(title: "Syntax Inference", author: "Spacetime architecture note")
#set page(paper: "a4", margin: (x: 2.2cm, y: 2.4cm), numbering: "1", number-align: center)
#set text(font: ("Libertinus Serif", "Georgia"), size: 11pt, lang: "en")
#set par(justify: true, leading: 0.72em)
#show heading: set block(above: 1.5em, below: 0.8em)
#show heading.where(level: 1): set text(size: 17pt, weight: "bold")
#show heading.where(level: 2): set text(size: 13pt, weight: "bold")
#show heading.where(level: 3): set text(size: 11.5pt, weight: "bold", style: "italic")
#show raw.where(block: false): it => box(fill: luma(243), inset: (x: 3pt), outset: (y: 3pt), radius: 2pt, text(size: 9.5pt, it))
#show raw.where(block: true): it => block(fill: luma(246), inset: 10pt, radius: 3pt, width: 100%, text(size: 9pt, it))
#let note(t, b) = block(fill: luma(250), stroke: (left: 2pt + luma(160)), inset: (x: 12pt, y: 10pt), radius: 2pt, width: 100%, above: 1.2em, below: 1.2em)[#text(weight: "bold")[#t] \ #b]
#let thread(n, t, b) = block(fill: rgb("#fdfaf3"), stroke: (left: 2.5pt + rgb("#c9a227")), inset: (x: 12pt, y: 10pt), radius: 2pt, width: 100%, above: 1.3em, below: 1.3em)[#text(weight: "bold", size: 10pt)[Thread #n — #t] \ #v(0.2em) #b]

#align(center)[
  #text(size: 22pt, weight: "bold")[Syntax Inference]
  #v(0.3em)
  #text(size: 11pt, style: "italic")[Why Spacetime's type system is not a type system, \ and what it becomes if we stop pretending it is]
  #v(0.2em)
  #text(size: 9pt)[Spacetime architecture note · #datetime.today().display()]
]

#v(1.0em)

#note[Read this after][
  `value-typing-paradox` (6pp) and `relations-from-zero` (5pp). Those two
  established the machinery: tagged value nodes, and the `≤`/`~` relations. This
  one asks whether the machinery is aimed at the right target. It concludes that
  it is not — not quite — and that the correction makes the system *smaller*.
]

= Part I — The forest

== A confession about the previous two documents

Both earlier notes reason as though Spacetime has a type system with an awkward
syntax attached. Values have types; types have relations; syntax is how you spell
a value. That framing produced real work — the escape list became a grammar, the
lexer stopped lying about the hole sigil — so it was not wasted. But it has the
dependency backwards, and the backwardness is starting to cost.

In Spacetime, *syntax is not the spelling of a type. Syntax is the type.*

Consider what the compiler actually knows about `--ink`. It does not know it is a
colour. It knows it matched a production:

```
%capture_type token_ref { "--" $name:ident }
```

That is the entire content of the judgement. "Colour" arrives later, if at all,
from the declaration site — and often never arrives, because `--ink` is a
reference and the referent may be in another file. Yet the value is perfectly
well-typed for every purpose the compiler has: it can be validated, emitted,
lowered, and refused in the wrong slot.

The type of `--ink` *is* `token_ref`. Not "a colour, spelled as a token
reference". The syntactic category is the semantic one, and there is no second
layer underneath.

== Three systems, and why none of them is the type system

The audit found three candidate type systems, none authoritative:

#block(inset: (left: 8pt))[
  `%scalar_type` — 14 rows. Carries *meaning*: a length is a distance, a time is
  a duration, a colour has a widget.

  `%capture_type` — 82 productions. Carries *shape*: what token sequence
  constitutes one.

  lightningcss — carries *property validity*: whether this value is legal for
  `border-radius`.
]

The instinct is to unify them: one table, three columns. That instinct has been
correct twice in this arc and is wrong here.

They are not three views of one thing. `%capture_type` is *constitutive* — a
value IS its production, and nothing else determines what it is. `%scalar_type`
is *attributive* — it hangs facts onto a category that already exists. And
lightningcss is *external* — it encodes a foreign specification we do not own
and must not fork.

Only the first is a type system. The second is metadata. The third is a
consultant.

#note[The practical consequence][
  Stop trying to give `%scalar_type` rows a subtyping relation. Give the
  *productions* one. `%scalar_type` then inherits it for free, because a scalar
  type names a production. This is the one design correction in this document,
  and it deletes rather than adds — see Part IV.
]

== Why this is not pedantry

If types are primary and syntax is spelling, then a new syntactic form needs a
new type, a new Rust enum arm, and a new validation path. That is the world this
arc has been fighting, and it is where all eight deleted hand-synced lists came
from.

If syntax is primary, a new syntactic form needs a *production*. That is a
stdlib row. The Rust side never learns about it.

The same claim, in the language of the elegance bar: variants must be data. But
the deeper reason they can be data is that in this language *the variants are
grammatical*, and grammar is already data. We were not choosing an
implementation strategy. We were noticing the shape of the thing.

= Part II — What "inference" means here

== Inference without unification

Hindley–Milner infers types by *unification*: collect constraints from usage,
solve them, propagate. It was rejected earlier in this arc for a good local
reason — CSS is not a lambda calculus, and there are no type variables to solve
for.

The deeper reason to reject it is now visible. HM infers a type that was never
written. Spacetime never needs to: the type *is* written, in the syntax, at every
site. There is nothing to solve for.

What Spacetime needs is not inference-as-solving. It is *inference-as-parsing*:
given these bytes, which production do they instantiate? That question has an
answer by direct matching, and the answer is complete.

#note[Why the name still fits][
  We infer, in the ordinary sense — the author wrote `--ink` and the compiler
  concluded `token_ref`. But the mechanism is recognition, not deduction. It runs
  forward, in one pass, with no constraint store and no failure mode where the
  solver gives up. That is a considerably *smaller* thing than HM, which is the
  point.
]

== The one thing that genuinely needs inferring

Not everything is determined locally. `--ink` is a `token_ref` locally and a
colour globally, and the global fact requires following the reference to its
declaration.

This is the real inference problem, and it is a *reachability* question, not a
unification one: what does this name resolve to, and what did that site declare?
It is the same shape as import resolution, which this codebase already does
(eight ordered steps, sibling-first as of `b31fc39d`).

So: two mechanisms, cleanly separated.

#block(inset: (left: 8pt))[
  *Recognition* — bytes to production. Total, local, one pass, no failure mode.

  *Resolution* — name to declaration. Partial, global, may not terminate at an
  answer, and must degrade gracefully when it does not.
]

Conflating them is what makes value typing feel paradoxical. A value that cannot
be resolved is still perfectly recognised, and most of the compiler only needs
recognition.

= Part III — Thinking threads

These are the open questions I would want to argue about before building. They
are threads, not proposals: each is a place where the "syntax is the type"
reading suggests something the previous documents did not.

#thread(1)[Does a production subsume another production?][
  `≤` was posed as a relation on *scalar types* — `length ≤ length_percentage`.
  Under this reading it is a relation on *productions*, and productions already
  have a natural one: a grammar A is below B when everything A matches, B
  matches.

  That is *language inclusion*, and for the small grammars here it is decidable.
  Which raises the sharp question: should `≤` be *declared* (a `%widens_to` row,
  as filed) or *derived* (computed from the productions)?

  Derived is seductive — no hand-maintained edges, no risk of a wrong claim,
  exactly the "no hand-synced lists" doctrine. But it makes every widening
  accidental: two grammars that happen to overlap become related whether or not
  anyone meant it, and the relation changes silently when a production is edited.

  Declared is honest about intent and can be *checked against* derivation: assert
  every declared edge is a real inclusion. Wrong claims become impossible; unmeant
  ones stay impossible. I think that is the answer, and it is strictly better than
  either alone — but it is worth arguing.
]

#thread(2)[Is a hole a production or a mode?][
  As of this week the backtick lexes as a sigil, so a hole *can* be expressed
  grammatically:

  ```
  %capture_type hole { "`" $inner:expr "`" }
  ```

  That would let the last remaining lexical escape in `validate_declaration_value`
  become a grammar query, closing the arc that started with eight lists.

  But look at what a hole *does*. It is not a kind of value — it is a value of
  *unknown kind, deferred to runtime*. Every other production answers "what is
  this?"; a hole answers "ask later".

  So a hole may be the one place where the `~` relation earns its keep: it is
  consistent with everything and a subtype of nothing. That is precisely the
  Unknown of gradual typing, and it arrives not as a lattice position but as a
  *syntactic form*. Which is a rather beautiful outcome if it holds — gradual
  typing falling out of the grammar rather than being bolted onto it.

  Worth testing against real pages before believing.
]

#thread(3)[What is a directive, typologically?][
  `@fade-in(easing: --ease-out-expo)` — the compiler checks this against a `%form`.
  Forms are kinded (style, easing, score, motion, markup, value) and take
  parameters. A form is therefore a *function type*, and form matching is
  *application*.

  That is the one place where real type-theoretic machinery might pay, because
  functions are where subtyping gets interesting (contravariance in parameters,
  covariance in results). The current mechanism is a scoring heuristic, and FUP-190
  proposes replacing it with a join.

  The thread: if a form is a function type, is `%macro` a *lambda*? And if so,
  what is `%includes` — composition? The language may already have a calculus in
  it, discovered rather than designed. I do not know whether that is a deep truth
  or a false analogy, and it is the single most interesting question here.
]

#thread(4)[Where does the unit live?][
  Measured, and one of the sharpest facts in this arc: `stagger: 0.05` is seconds,
  `duration: number = 600` is milliseconds. The unit is a property of the
  *declaration site*, never of the spelling.

  Under "syntax is the type" this is uncomfortable, because it is a fact that is
  not in the syntax. It is the clearest evidence that recognition alone is
  insufficient and resolution is genuinely needed.

  But note what it is NOT: it is not a subtyping question. `number ≤ time` is
  locally reasonable and globally catastrophic — a blanket numeric migration once
  made staggers 1000× too slow across twenty files, silently, with a green build.
  The unit is a ternary relation keyed by site, or it is not a relation at all.

  Thread: should the declaration site declare its unit *in the syntax*, so the
  fact returns to where the type is? `duration: 600ms` in a macro signature, rather
  than a convention in a doc comment. That is a language change, not a compiler
  change, which is why it belongs in a design conversation and not a follow-up.
]

#thread(5)[Does this make the language more learnable, or less?][
  The governing constraint: *knowing one thing should tell you the next.*

  "Syntax is the type" is a strong learnability claim. If every category is a
  visible form, then reading a value tells you its type with no lookup — the
  sigil IS the annotation. `--x` is a reference, `` `x` `` is a hole, `$x` is a
  binding, `&x` is an identity. Four sigils, four categories, no table to memorise.

  The risk is the opposite outcome. If types are grammatical, then *changing a
  type means changing how something is spelled*, and every retyping becomes a
  corpus-wide migration. That is exactly what happened with `&easing → --easing`:
  402 value sites, 11 declarations, one script, one afternoon.

  So the question is not "is this elegant" but "what is the migration story". A
  language where types are syntax needs migration to be a first-class, checkable
  operation — which is FUP-192, and which under this reading is not a nicety but
  a *load-bearing* part of the design.
]

= Part IV — The correction, concretely

One design change follows from all of the above, and it is a deletion.

The filed FUP-189 puts `%widens_to` rows on `%scalar_type`. That is the wrong
carrier. `%scalar_type` is attributive metadata — 14 rows about meaning and
widgets — while the thing that actually has extension, and therefore can be
compared, is the *production*.

#block(inset: (left: 8pt))[
  *As filed:* `%widens_to` on `%scalar_type`, 14 rows, relation over scalars.

  *As corrected:* `%widens_to` on `%capture_type`, relation over productions.
  Scalars inherit it because a scalar names a production. Values that are not
  scalars — references, holes, wide keywords, directive refs — get the relation
  *for free*, where under the filed design they were outside the system entirely.
]

That last point is the whole argument. Under the filed design, `--ink` has no
place in the relation, because it is not a scalar. Under the correction it does,
because it is a production. And `--ink` is precisely the value this arc has spent
weeks failing to type.

The verification story improves too: a declared edge can be *checked* against
language inclusion (Thread 1), which is not available for scalar rows because
scalars have no extension to compare.

#note[What this does not change][
  The `~` relation, its deliberate non-transitivity, the `O ≤ E` / `O ~ E`
  decision procedure, and every verification seed in FUP-189 through FUP-192.
  Those were right. Only the carrier moves — and it moves *down*, onto the thing
  that was already primary.
]

= Part V — Where the work stands

#block(inset: (left: 8pt))[
  *Done this week.* The backtick lexes as a sigil, one char, non-consuming, like
  every other sigil in the language. The leak was not a missing token: unknown
  bytes were pushed as `RawToken::Minus /* placeholder */` and then absorbed into
  the following identifier by the vendor-prefix merge rule, so `` "a `b` c" ``
  lexed as `IDENT("a") IDENT("`b") MINUS("`") IDENT("c")`. Spacetime's hole sigil
  arriving downstream as arithmetic. Unknown bytes now have their own category.

  *Available now.* A hole can be written as a production, so the last lexical
  escape in the validator can become a grammar query. Thread 2 asks whether it
  should be.

  *Waiting on this conversation.* Whether `≤` carries on productions rather than
  scalars (Part IV), and whether declared edges get checked against derived
  inclusion (Thread 1). Both change what gets built, so neither should be built
  first.
]

The next wave is the four filed follow-ups, corrected per Part IV. After them I
will stop and we should design properly — Thread 3 in particular (is a form a
function type, is a macro a lambda) is not a follow-up-sized question, and
getting it wrong would be the kind of locally-coherent, systemically-wrong move
this whole arc exists to avoid.
