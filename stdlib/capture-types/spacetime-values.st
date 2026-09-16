// Spacetime's OWN value grammars — the shapes CSS does not have.
//
// PLAN-136 W5. `css-values.st` says what a colour, a length and a duration are;
// those are CSS's values and an upstream parser will always know them better
// than we do. This file holds the values SPACETIME invented, which no upstream
// parser can know because they do not exist outside this language.
//
// That distinction is the whole seam. A property's type is answered by asking
// lightningcss when the property is CSS's, and by reading these productions when
// it is ours. Neither half is a hand-maintained mirror of the other.
//
// WHY THESE TWO FIRST: they are the shapes the corpus writes that nothing could
// check. `stagger` appears 153 times and `range` 49, and W4's dry run refused 25
// of them as invalid before the rule was scoped back — not because the authors
// were wrong, but because no production described what they had written. A value
// with no grammar cannot be validated, cannot be completed by an editor, and
// cannot have its declaration tightened. Writing the grammar is what unblocks
// all three.

// ---------------------------------------------------------------------------
// Stagger
// ---------------------------------------------------------------------------

// Where a stagger starts from. A closed set, so it is an alternation of literals
// — the same spelling `css_wide` and the unit sets use. There is no
// `%enum` directive and there should not be: a closed set of keywords is an
// ordinary alternation, and one directive composed beats a new subsystem per
// concept.
%capture_type stagger_origin {
    "first" | "last" | "center" | "edges"
}

// A 2D stagger grid: `grid(13 13)`. Both dimensions are required — `grid(3)` is
// ambiguous between rows and columns, and guessing which would be a silent
// wrong answer in a visual effect nobody would trace back to the parser.
%capture_type stagger_grid {
    "grid" "(" $cols:number $rows:number ")"
}

// `stagger: 0` · `stagger: 50ms` · `stagger: 0.05 first` ·
// `stagger: 0.003 grid(13 13) center`
//
// THE DELAY IS A NUMBER *OR* A DURATION, and that is not laziness. Measured
// across the tree, the same concept is declared both ways and means different
// units in each: `stagger: $stagger:number = 100` (loop.st:52) alongside
// `stagger: $stagger:time = 50ms` (fade-in.st:108), and in authored code `0.05`
// means SECONDS while `duration: $duration:number = 600` (reveal.st:34) means
// MILLISECONDS. The unit is a property of the declaration site, not of the
// spelling, so a grammar that insisted on one would either reject valid code or
// silently change the speed of every animation that used the other.
//
// Resolving that ambiguity is a real fix, but it belongs to the runtime contract
// (`apply-animations.st` reads `{ delay, from, grid }`), not to a syntax
// production. This grammar states exactly what is TRUE today and no more.
%capture_type stagger_value {
    ( $delay:duration | $delay:number ) ( $grid:stagger_grid )? ( $from:stagger_origin )?
}

// ---------------------------------------------------------------------------
// Range
// ---------------------------------------------------------------------------

// `range: 0 to 0.5` — a scroll/animation window, 49 uses across the corpus and
// exactly one shape.
//
// `to` is a literal keyword rather than a symbol because that is how the corpus
// writes it, and because `0 -> 0.5` already means a TRANSITION elsewhere in the
// language. Reusing the arrow here would give one sigil two meanings, which the
// harmony rule exists to prevent: an arrow animates between two values over
// time, a range names two points on a timeline.
%capture_type range_value {
    $start:number "to" $end:number
}
