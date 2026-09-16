// stdlib/macros/score.st — `@score <driver> { … }` (SIP-001 §Proposal/4, PLAN-128 W4)
//
// A score is ARRANGEMENT: several elements sharing ONE playhead. `@on` is
// choreography — an element animating on its own driver. The two share one
// driver grammar, which is the whole bet: a website and a video differ only in
// WHICH SIGNAL DRIVES THE TIMELINE.
//
//   @score &.time(60s) as &film {      // a film
//   @score &.scroll(cover) { … }       // a scrollytelling section
//   @score &.clip { … }                // the same body, as a clip in a parent
//
// THE ONE IDEA (R3): `@score` adds NO runtime concept. Every driver already
// publishes exactly one 0..1 progress signal, so a score is a pure DERIVATION
// over that signal — each clip's progress is a function of its parent's:
//
//   &lower-third at 2s for 3s          // inside @score &.time(60s)
//   $lower_third_progress <- clamp01(($film_progress * 60000 - 2000) / 3000)
//
// `at/for` is an affine window; `during A to B` resolves one from two named
// spans; `rate:` is where the map stops being affine (which is why speed-ramp,
// freeze and reverse are ONE concept). Because the map is pure, a score is
// seekable by construction — render determinism falls out instead of being
// enforced. If you find yourself writing a tick loop over a clip list, the
// design has been lost: that is how live and rendered films diverge.
//
// P4 — THE PUNCTUATION RULE: `( … )` carries what the FUNCTION owns; trailing
// words carry what the LANGUAGE owns.
//
//   &broll(src: "city.mp4") in 4s out 9s during &opening;
//   ^^^^^^^^^^^^^^^^^^^^^^^ the clip's own param
//                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ placement vocabulary
//
// This is not cosmetic. The trailing words are exactly the surface a timeline
// editor drags, so dragging a clip's edge rewrites `in 4s` to `in 5.2s` in the
// source. The editor is a second view of the language, not a layer over it.
//
// R1 — THE DRIVER OWNS THE TIME DOMAIN. A score never declares a duration of
// its own; it inherits the domain of whatever drives it, read from the
// `domain:` column of the driver registry (stdlib/macros/drivers.st). So
// `at 2s` under `&.scroll` is E0951, not a silently meaningless number.
//
// R2 — GROUPING IS A FRAGMENT, not a keyword. `@form score --opening { … }`
// already exists (stdlib/macros/form.st, the sixth kind marker) and a fragment
// is spliceable into a score body. There is deliberately no `@scene`.

/// A measure in the score's own domain: `2s`, `30%`, `400ms`.
///
/// Deliberately ONE type across both domains rather than a time-type and a
/// percent-type. Which spellings are legal is not a property of the placement —
/// it is a property of the DRIVER (R1), so the check belongs to the registry
/// and the grammar stays domain-agnostic. Splitting it here would put the same
/// rule in two places and let them disagree.
%capture_type score_measure { ( $len:length ) | ( $time:duration ) }

/// Placement — the language's own vocabulary, per P4.
///
/// `at`    when it starts          `for`    how long it lasts
/// `in`    trim the source's head   `out`    trim the source's tail
/// `after` start when another ends  `during` span another clip (`to` extends)
/// `as`    name this span so `during`/`after` can reference it
%capture_type score_at     { "at" $at:score_measure }
%capture_type score_for    { "for" $for:score_measure }
%capture_type score_in     { "in" $in:score_measure }
%capture_type score_out    { "out" $out:score_measure }
%capture_type score_after  { "after" $after:element }
%capture_type score_during { "during" $from:element ( "to" $to:element )? }
%capture_type score_as     { "as" $alias:element }

%capture_type score_placement {
    ( $at:score_at ) | ( $for:score_for ) | ( $in:score_in ) | ( $out:score_out )
  | ( $after:score_after ) | ( $during:score_during ) | ( $as:score_as )
  | ( $rate:score_rate )
}

/// `rate:` — how fast a clip's own progress advances relative to the score's.
///
/// This is the point where the window map STOPS BEING AFFINE, and that is why
/// speed-ramp, freeze and reverse are ONE concept rather than three features:
/// `rate` is the DERIVATIVE of the map (SIP-001 W4/R3).
///
///   rate: 1 -> 2;   speed ramp — the map is the integral of a ramp
///   rate: 0;        freeze frame — the map is constant
///   rate: -1;       reverse — the map has negative slope
///
/// DISTINCT from `hold(until:)`, which stops the PLAYHEAD (the whole score
/// waits). `rate: 0` freezes ONE CLIP while the timeline keeps running. The
/// spec names conflating these as the likely bug, so they are deliberately
/// different surfaces: a property of a clip vs an entry in the score body.
%capture_type score_rate { "rate" $from:number ( "->" $to:number )? }

/// NB `rate` is a TRAILING WORD, not a braced settings block.
///
/// A braced block (`&title at 0s for 10s { rate: 1 -> 2; }`) was tried first
/// and does not work: the score body is captured as `{ $entries:score_entry+ }`
/// and the body capture ends at the first `}` regardless of nesting, so the
/// inner brace closes the score. The failure is LOUD (E0946, BUG-229's fix
/// doing its job) rather than silent — but it is still a wall.
///
/// The trailing-word form is the better surface anyway, and P4 says why: `rate`
/// modifies WHEN the clip's progress advances, which is placement vocabulary —
/// the same category as `at` and `for`, so it belongs in the same position.
/// Parens would have claimed it for the clip's own function, which it is not.

/// A step is WHAT plays: an element (`&title`), or a score fragment spliced in
/// (`--opening`, R2). Placements follow it as trailing words.
%capture_type score_subject { ( $form:form_application ) | ( $elem:element ) }
%capture_type score_step { $subject:score_subject $places:score_placement* }

/// A line is one or more steps joined by `->`. The arrow is SEQUENCE (each step
/// follows the previous), and a transition form sitting on an edge applies over
/// the overlap:
///
///   &intro -> --whip-pan(300ms) -> &features;
///
/// `->` keeps its one meaning — a step between values — exactly as in keyframes.
%capture_type score_line { $head:score_step ( "->" $next:score_step )* ";" }

/// `mark &chapter1;` — a named POINT other placements can reference. A mark has
/// no duration; it is where `after`/`during` can aim.
%capture_type score_mark { "mark" $name:element ";" }

/// `gap 500ms;` — advance the playhead without placing anything.
///
/// A gap is an ABSENCE, and an absence is already expressible by where the
/// next clip starts (`at`). It earns its place by keeping a score RELATIVE:
/// a body written entirely in `->` sequence and `gap` has no absolute offsets
/// in it, so inserting a clip near the top does not require rewriting every
/// number below. That is the difference between a score you can edit and one
/// you must recompute.
///
/// Spelled as a trailing measure rather than `gap(500ms)`: a gap takes no
/// arguments of its own, it IS a measure of the score's time, so P4 puts it
/// outside parens with the rest of the placement vocabulary.
%capture_type score_gap { "gap" $len:score_measure ";" }

/// `@audio(src: "bed.wav") as &bed at 0s;` — sound ON the score (PLAN-150 W7).
/// Sound is a score entity with a source, a name, and a start position: the
/// score is the ONE transport, and `&bed` is an `<audio>` element seeked/played
/// by the score's own progress. `gain`/`rate`/`pan` are keyframed on `&bed` in
/// an ordinary `@on &.clip` block, like any other entity.
%capture_type score_audio {
    "@audio" "(" "src" ":" $src:string ")" "as" $name:element "at" $at:score_measure ";"
}

%capture_type score_entry {
    ( $audio:score_audio ) | ( $mark:score_mark ) | ( $gap:score_gap ) | ( $line:score_line )
}

/// `as &film` — names the score's own timeline so other scores and `@on`
/// heads can reference it (`&film.done`).
///
/// NB this is NOT `@on`'s `on_as`, and the difference is the sigil discipline
/// working as designed. `@on … as $name` names a PROGRESS SIGNAL (`$` = data,
/// a value that flows); `@score … as &film` names a TIMELINE ENTITY (`&` =
/// identity, a thing in the world you can point at). Reusing `on_as` here
/// captured `$name:binding`, so the `&film` form silently failed the optional
/// capture and the alias VANISHED with no diagnostic — found in review.
/// One sigil, one meaning; two categories, two capture types.
%capture_type score_as_name { "as" $name:element }

/// `@score <driver> { … }` and `@score <driver> as &name { … }`.
///
/// NB the body is a DIRECT REPEATED BODY CAPTURE (`{ $entries:score_entry+ }`),
/// never a parenthesized group. A group compiles against stdlib capture types
/// only, so a same-file `%capture_type` would be invisible there and the
/// directive would silently never match (BUG-229's second lesson). The
/// difference is invisible in the source and total in effect.
%macro score-block {
  %scope file | selector
  %order 700
  %form { @score $driver:driver_expr $as:score_as_name? { $entries:score_entry+ } }
  %binds { score(&self, driver: $driver, entries: $entries, as: $as) }
}


/// ERROR SIBLING (BUG-264): an `as` with the WRONG SIGIL (`as $film` for
/// `as &film`) is a near-miss for the optional `$as:score_as_name?` capture —
/// `score_as_name` requires `&name` (an element), so a `$`-sigil fails it and the
/// alias would otherwise be silently dropped and VANISH with no diagnostic (the
/// exact failure mode the `score_as_name` capture type was introduced to fix;
/// the wrong-sigil case still slipped through). This sibling — which binds the
/// offending `$`-sigil — reports it. Declared AFTER `score-block`: the
/// productive form wins cleanly on `as &film`, leaving this sibling to fire only
/// on the wrong sigil.
%macro score-invalid-as {
  %diagnostic


  %scope file | selector
  %order 700

  %form { @score $driver:driver_expr as $bad:binding { $entries:score_entry+ } }

  %binds {
    optional-clause-error(
      message: "`as` must name a timeline with `&`, not `$`",
      hint: "write `as &film` — `&` names a timeline entity; `$` names a progress signal"
    )
  }
}
