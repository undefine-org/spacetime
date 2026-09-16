// THE VALUE NODE — what a declaration value can BE, as data.
//
// Every escape list in the compiler exists to answer one question: "is this
// value CSS, or is it Spacetime?" Today that question is answered by inspecting
// characters — `value.contains('$') || value.contains('&') || …` — in ~55
// places, each of which can be wrong in a different direction. A wrong answer
// in one direction is a missed diagnostic; in the other it is a FALSE REFUSAL,
// which blocks a build the author cannot fix.
//
// The question is being asked at the wrong moment. By the time a validator sees
// `--ink` it has a STRING and must reverse-engineer what it is. But the parser
// already knew: it lexed two dashes. This file is where that knowledge is
// written down once, as a grammar, so nothing downstream has to guess.
//
// SHAPE, NOT RESOLUTION. A `value_node` says what a value IS, never whether it
// resolves. `--ink` may be declared in a project prelude, a `@tokens` block, or
// plain CSS this compiler never sees; `$brand.ink` is known only at render.
// Refusing either would assert something we cannot know — see `%capture_type
// deferred` below, and the consistency rule that consumes it.
//
// ARM ORDER IS SEMANTIC, exactly as in `css-values.st`: the most specific arm
// wins, and the catch-all `css` arm is LAST so a Spacetime value is never
// mistaken for CSS text. This mirrors the existing rule that a literal beats a
// `( $ref:binding )` arm.

// ---------------------------------------------------------------------------
// The Spacetime arms — a value that is NOT CSS
// ---------------------------------------------------------------------------

/// A token reference: `--ink`, `var(--ink)`, `var(--ink, #222)`.
///
/// Both spellings name the same thing. Spacetime prefers the bare `--ink` —
/// `--name` already names a value unambiguously, so `var()` is CSS ceremony the
/// language does not need — but `var()` is legal and common, and lowering
/// happens at emit, not here.
///
/// The `var()` interior is deliberately NOT validated: a fallback may nest
/// arbitrarily (`var(--a, var(--b, red))`) and its correctness is the browser's
/// to judge.
%capture_type token_ref {
    ( $bare:dashed_ident ) | ( $fn:var_call )
}

/// `var(` … `)` — the interior is a balanced token run, unjudged.
%capture_type var_call {
    ( "var" | "VAR" ) "(" $inner:balanced(')') ")"
}

/// An environment substitution: `env(safe-area-inset-top)`. Same deferral rule
/// as `var()` — the value is the user agent's to supply.
%capture_type env_call {
    ( "env" | "ENV" ) "(" $inner:balanced(')') ")"
}

/// A CSS-wide keyword. Legal on EVERY property, whatever its value type, which
/// is why it is an arm here and not a colour/length/time concern.
///
/// Defined once in `css-values.st` and referenced, not restated — a seventh
/// keyword is one edit there.
%capture_type wide_keyword {
    $kw:css_wide
}

/// An animation range: `0 -> 1`, `blur(18px) -> blur(0)`.
///
/// `->` means TRANSITION everywhere in the language (sigil harmony), so a value
/// containing one is a keyframe pair, never a CSS value. Both endpoints are
/// themselves value nodes, which is what makes `--a -> --b` work.
%capture_type range_pair {
    $from:value_node "->" $to:value_node
}

/// An element reference: `&self`, `&card`.
///
/// `&` is the IDENTITY sigil — it names an ELEMENT, and an element is never a
/// CSS value. Written out rather than wrapping the builtin `selector`, which is
/// not precise enough: `{ $sel:selector }` accepts `#ff0000` as an id selector
/// and would steal a real colour.
%capture_type element_ref {
    "&" $rest:balanced(';')?
}

/// A directive or meta reference: `@fade-in`, `%scalar_type`.
///
/// Written out for the same reason: `{ $e:expr }` is a catch-all and accepts
/// `notadirective`. The sigil plus an identifier is exact.
%capture_type directive_ref {
    ( "%" | "@" ) $name:ident
}

/// A HOLE: `` `$x` ``, `` `$c.id` ``.
///
/// The backtick is THE hole form everywhere (sigil harmony), and a hole is a
/// value whose content is decided at RENDER. Where every other arm answers
/// "what is this?", a hole answers "ask later" — which is why it belongs in
/// `deferred` below and claims nothing about the value's type.
///
/// This production could not be written until the backtick became a lexer token
/// (FUP-046). A grammar consumes tokens; before that fix there was no token for
/// a grammar to consume, and the hole had to be recognised by character
/// inspection in Rust instead.
///
/// The closing backtick is REQUIRED. An unterminated one is text, not a
/// deferred value — refusing to guess is this file's doctrine.
///
/// The inner is `binding | ident`, NOT `balanced('`')`. Measured: `balanced`
/// stops AT its delimiter but counts it as consumed, so a following literal has
/// nothing left to match and `"`" $i:balanced('`') "`"` never matches. It also
/// does not REQUIRE its delimiter — `` `x `` passes — which would admit an
/// unterminated hole. `expr` fails outright (it swallows the closer). The union
/// covers all three real spellings: `` `x` ``, `` `$x` ``, `` `$c.id` ``.
%capture_type hole {
    "`" ( $binding:binding | $name:ident ) "`"
}

/// A COMPOUND value: CSS text with a Spacetime node somewhere inside it.
///
/// `1px solid $brand.rule` — CSS shorthand whose colour slot is a binding.
/// `border: 1px solid --ink`, `box-shadow: 0 2px 8px `$c.tint``, and every
/// other shorthand that mixes literal CSS with a deferred value.
///
/// WHY THIS ARM EXISTS. The lexical check it replaced asked `contains('$')` and
/// therefore did not care WHERE the sigil sat. A grammar whose arms all anchor
/// at the start of the value does care, and routes a compound to `css` — where
/// lightningcss refuses it. Measured on the first grammar-only run: seven
/// E0958s across the demos, all of the form "`1px solid $brand.rule` is not a
/// valid color for `border`".
///
/// Those are FALSE REFUSALS: they block a build the author cannot fix, which is
/// strictly worse than the missed diagnostic the check was meant to prevent.
/// The rule the old code embodied has to be stated, not dropped — a Spacetime
/// node ANYWHERE in a value makes the whole value not-CSS, because the value is
/// rewritten before it is ever handed to a CSS parser.
///
/// Each arm is `balanced(<the node's own sigil>)` followed by the node. That
/// shape is exact, and it is the only one that works:
///
///   - `balanced` stops AT its delimiter, so `balanced('$') $g:binding` consumes
///     the CSS prefix and then requires a real binding. `1px solid notacolor`
///     fails, because there is no `$`.
///   - a delimiter ABSENT from the value makes `balanced` consume to the end and
///     the trailing alternation never matches — measured, `balanced('\u{0}')`
///     made EVERY value a compound and lost five refusal gates at once.
///   - `$before:ident+` does not match `1px` (a number with a unit).
///   - `$b:expr` followed by an alternation fails outright.
///
/// Three arms rather than one alternation because the delimiter must be the
/// sigil of the node being sought.
///
/// NO HOLE ARM, and that is a grammar limitation rather than a decision.
/// `balanced('`')` causes the production containing it to be dropped from the
/// registry silently (neighbours survive), and a `"`"` literal after a capture
/// destabilises the pattern parse. Filed as FUP-194. Nothing is refused today:
/// every corpus value with a non-leading backtick is diagnostic prose inside a
/// quoted string. The gate is written and #[ignore]d in
/// tests/value_consistency_test.rs, ready for the day one appears.
%capture_type compound_value {
      ( $css_before:balanced('$') $binding:binding )
    | ( $css_before:balanced('-') $token:token_ref )
}


// ---------------------------------------------------------------------------
// The value node itself
// ---------------------------------------------------------------------------

/// What a declaration value can BE.
///
/// This is the sum type the compiler has been approximating with character
/// checks. Each arm is NAMED, so a consumer routes on the arm rather than
/// re-deriving the kind from the text — the same pattern `color` already uses
/// (`( $core:color_core ) | ( $wide:css_wide ) | ( $ref:binding )`).
///
/// `css` is LAST and is the only arm a CSS validator may judge. Every other arm
/// is not CSS by construction, and that fact is now stated once here instead of
/// being re-inferred at every call site.
%capture_type value_node {
      ( $range:range_pair )
    | ( $token:token_ref )
    | ( $env:env_call )
    | ( $wide:wide_keyword )
    | ( $binding:binding )
    | ( $hole:hole )
    | ( $element:element_ref )
    | ( $directive:directive_ref )
    | ( $compound:compound_value )
    | ( $css:balanced(';') )
}

/// The arms whose value is UNKNOWABLE at build time.
///
/// This is the vocabulary of the consistency relation: a value matching any of
/// these is `Unknown`, and Unknown is CONSISTENT WITH EVERY TYPE — it is
/// admitted everywhere and claims nothing anywhere. That is what makes a false
/// refusal structurally impossible rather than a thing we remember to avoid.
///
/// NB Unknown is not a lattice top. Top would CLAIM everything; consistency
/// admits without claiming, which is why `Unknown ~ length` and `Unknown ~
/// color` do not license `length ~ color`. (Siek & Taha's `~`, deliberately
/// non-transitive.)
%capture_type deferred {
    ( $token:token_ref ) | ( $env:env_call ) | ( $binding:binding ) | ( $hole:hole )
}
