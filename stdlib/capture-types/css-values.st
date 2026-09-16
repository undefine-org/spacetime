// CSS scalar value types — the language's leaf value grammars.
//
// EVERY scalar below ends with a `( $ref:binding )` arm. A value position in
// Spacetime is reactive — `@fade-in(duration: $speed)`, `color: $brand.ink` —
// so a scalar that accepted only literals would reject ordinary usage across
// the corpus. The arm is listed LAST so a literal always wins the alternation,
// and it is not a hole in the validation: a literal must still satisfy its
// grammar, and a binding is checked where bindings are checked (against the
// signal's declared type), not by guessing from its name here.
//
// PLAN-122 W1.2. These productions say what a colour, a length, a duration, an
// angle and an easing ARE. Until now that knowledge lived in Rust twice over:
// once in the lexer (a `#[0-9a-fA-F]+` regex, a hardcoded unit bag, a positional
// promotion rule) and once in a closed `CaptureType` enum with a hand-written
// extractor per arm. Adding one scalar cost ~14 Rust edit sites, measured twice
// (PLAN-121's `dashed_ident`, FEAT-164's `Counted`). Here a scalar is a grammar,
// and a new one is a paragraph in this file.
//
// WHY THIS FILE CAN EXIST NOW: a capture extractor consumes a whole number of
// TOKENS, so while the lexer fused `#e8eef7` into one atomic COLOR token no
// grammar could enter it — `"#" [0-9a-fA-F]{3|4|6|8}` returned None in exactly
// the value position it exists to serve. W1.0 demoted that fusion behind
// `LexTier::Kernel`, which is what hands the domain back. See
// tests/lexer_tier_parity_test.rs.
//
// TWO SPELLINGS ON PURPOSE:
//
//   - A UNION of literals (`"px" | "em" | …`) is how a closed set of keywords is
//     written. There is no `%unit_set` directive and there should not be: unit
//     sets are ordinary alternations, and `%capture_type` already composes by
//     reference (param_list/optmark prove it). One directive, composed — not a
//     new subsystem per concept.
//
//   - A COUNTED CHARACTER CLASS (`[0-9a-fA-F]{3|4|6|8}`) is how a value whose
//     shape is its LENGTH is written. A hex colour is legal at 3, 4, 6 or 8
//     digits and at no other length; the permitted counts are DATA in the
//     grammar, which is why `#fffa` and `#11223344` keep working where a naive
//     "3 or 6" rule would silently drop the alpha forms.
//
// REPRESENTATION: the captured source text IS the value. A colour is the string
// `"#e8eef7"`, a duration the string `"600ms"`. No parsing into a struct, no
// unit conversion here — conversion belongs at the boundary that needs it (the
// animation runtime turns `600ms` into milliseconds), not at the grammar. This
// is what lets one value type serve CSS, JSON seeds and admin widgets without a
// per-consumer representation.

// ---------------------------------------------------------------------------
// CSS-wide keywords
// ---------------------------------------------------------------------------

// Every CSS property accepts these, whatever its value type. Factored out and
// included by reference so the set is stated ONCE — a seventh keyword is one
// edit here, not one edit per scalar.
%capture_type css_wide {
    "inherit" | "initial" | "unset" | "revert" | "revert-layer"
}

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

// Hex digits. Named so the colour production reads as prose rather than as a
// character range, and so a second hex-shaped value (an opacity byte, a colour
// with a documented alpha) composes instead of restating the class.
%capture_type xdigit {
    [0-9a-fA-F]
}

// The hex arm: `#` followed by exactly 3, 4, 6 or 8 hex digits.
//
// The counted class is what makes this a real check rather than a shape hint.
// `#e8ee1` (5) and `#e8eef71` (7) FAIL — they are not truncated to a legal
// prefix, because a silent truncation moves the failure somewhere less obvious
// than the literal itself. Case-folding is on the terminal (CSS is
// case-insensitive throughout), so `#E8EEF7` matches without the grammar
// restating the range in upper case.
// NB the counted class is written BARE, not as `$hex:[0-9a-fA-F]{3|4|6|8}`.
// A `$name:` prefix declares a CAPTURE, and a capture's type is the word after
// the colon — so the `$hex:` spelling parses as a TYPELESS capture (which
// defaults to `expr`, matching almost anything) followed by a SEPARATE char
// class, and the greedy `expr` swallows the digits before the class ever runs.
// The count belongs to the class itself; the surrounding production names it.
%capture_type hex_color {
    "#" [0-9a-fA-F]{3|4|6|8}
}

// Colour functions. The arguments are captured as a balanced run rather than
// parsed: their grammar is per-function (`rgb()` takes three numbers or a
// space-separated triple with an optional `/ alpha`; `color()` takes a colour
// space then components), and lightningcss already validates them properly.
// Recognizing the SPAN is what the compiler needs — enumerating every legal
// argument shape here would duplicate a validator that exists and does it
// better.
// Ordered LONGEST FIRST where one name prefixes another: a leftmost-match
// alternation would accept the `color` inside `color-mix` and strand `-mix(...)`,
// which is the same trap `image_fn_name` and `easing_keyword` document.
%capture_type color_fn_name {
    "color-mix" | "light-dark" | "device-cmyk"
  | "rgba" | "rgb" | "hsla" | "hsl" | "hwb" | "lab" | "lch" | "oklab" | "oklch" | "color"
}

%capture_type color_fn {
    $fn:color_fn_name "(" $args:balanced(')') ")"
}

// Colour keywords that are not part of the named-colour table: these two are
// contextual (they resolve against the cascade), which is why they live with
// the colour type rather than in `css_wide`.
%capture_type color_keyword {
    "currentColor" | "currentcolor" | "transparent"
}

// A colour: hex, function, contextual keyword, or a CSS-wide keyword.
//
// The 148 NAMED colours (`rebeccapurple`, `cornflowerblue`) are deliberately
// absent. A bare identifier in a value position is ambiguous with every other
// keyword CSS defines, so recognizing them here would require the grammar to
// carry a 148-entry table purely to disambiguate — and lightningcss already
// resolves named colours downstream. An author writing `color: red` is
// unaffected: the property grammar accepts a keyword, this type simply does not
// claim it.
%capture_type color {
    ( $core:color_core ) | ( $wide:css_wide ) | ( $ref:binding )
}

// A colour LITERAL — the same arms without `css_wide` and without `binding`.
//
// Split out because two different questions are being asked of one grammar.
// `color` answers "is this legal in a colour position?", and there `inherit`
// and `$brand.ink` both are. `color_core` answers "IS this a colour?", and
// there neither is: `inherit` is an instruction to the cascade, and a binding
// is a value whose kind is not known until it renders. A consumer that ROUTES
// on the answer (see `background_route`) must ask the second question — routing
// `background: inherit` as if it were a colour would inherit one longhand where
// the author asked to inherit all of them.
%capture_type color_core {
    ( $hex:hex_color ) | ( $fn:color_fn ) | ( $keyword:color_keyword )
}

// ---------------------------------------------------------------------------
// Image
// ---------------------------------------------------------------------------

// The functions that produce a CSS `<image>`. Balanced args, for the same
// reason as the colour functions: each function's argument grammar is its own
// (a gradient takes an optional angle or side-or-corner then a colour-stop
// list; `image-set()` takes resolution-tagged candidates), lightningcss
// validates them downstream, and what a consumer needs from this production is
// the KIND — that this value is an image and not a colour.
//
// Ordered LONGEST FIRST for the same reason `easing_keyword` is: a
// leftmost-match alternation would accept the `linear-gradient` inside
// `repeating-linear-gradient` and strand the prefix.
%capture_type image_fn_name {
    "repeating-linear-gradient" | "repeating-radial-gradient" | "repeating-conic-gradient"
  | "linear-gradient" | "radial-gradient" | "conic-gradient"
  | "image-set" | "cross-fade" | "element" | "paint" | "image" | "url"
}

// An image LITERAL — the `_core` half, by the same split as `color_core`.
%capture_type image_core {
    $fn:image_fn_name "(" $args:balanced(')') ")"
}

// An image: a producing function, a CSS-wide keyword, or a binding.
%capture_type image {
    ( $core:image_core ) | ( $wide:css_wide ) | ( $ref:binding )
}

// One LAYER of a background-image list. `<bg-image>` is `none | <image>`, so
// `none` is a legal LAYER — `none, url(a)` punches a hole in the stack while
// keeping two layers' worth of per-layer longhands.
%capture_type image_none {
    "none"
}

%capture_type image_layer {
    ( $img:image_core ) | ( $blank:image_none )
}

// A background LAYER LIST — several layers, comma separated (FUP-184).
//
// CSS stacks background layers front-to-back with commas, so `url(noise.png),
// linear-gradient(red, blue)` is ONE value naming TWO layers. A single
// `image_core` cannot match it, which is why a multi-layer reactive background
// fell through `background_route` unrouted and kept resetting its longhands.
//
// The comma trap: `linear-gradient(red, blue)` CONTAINS a comma but is one
// layer. This production does not need to know that — `image_core` ends in
// `balanced(')')`, which consumes through the matching close-paren, so an
// inner comma is eaten before the separator is ever considered. That is the
// same property `color_fn` relies on, not a new mechanism.
//
// Two arms, and the asymmetry is deliberate: a MULTI-layer list may contain
// `none`, but a SINGLE value may not. A bare `background: none` is a
// whole-shorthand reset (it clears colour, position, size — every longhand), so
// routing it to `background-image` would narrow the author's instruction and
// silently keep longhands they asked to drop. Same reasoning that keeps
// `inherit` unrouted.
%capture_type image_list {
    ( $first:image_layer "," $second:image_layer ( "," $rest:image_layer )* )
  | ( $only:image_core )
}

// ---------------------------------------------------------------------------
// Dimensions
// ---------------------------------------------------------------------------

// Length units, in the exact set the lexer's hardcoded bag carried, plus the
// modern units it lacked. Corpus usage (measured over 420 .st files): px 1810,
// rem 720, em 131, vw 68, vh 48, fr 41, ch 19. `vmin`/`vmax` were in the bag
// but unused; kept, because a unit set is a claim about CSS, not about this
// repo's current pages.
//
// `fr` is a grid fraction rather than a length proper. It stays in this set
// because it appears in the same value positions and the lexer already fused
// it; splitting it into its own type would break `grid-template-columns` for a
// taxonomic point no author benefits from.
%capture_type length_unit {
    "px" | "rem" | "em" | "ch" | "ex" | "cap" | "ic" | "lh" | "rlh"
  | "vh" | "vw" | "vmin" | "vmax" | "vi" | "vb"
  | "cm" | "mm" | "in" | "pt" | "pc" | "q"
  | "fr"
}

// Time units. `m` means MINUTES and is load-bearing — `refresh: 5m` appears in
// stdlib/macros/data-kind.st, data.st, type-data.st, examples and parser tests,
// and src/syntax/conversions.rs converts it as `value * 60_000`. It reads like a
// fossil of `ms` and is not one; removing it would silently retime every polling
// data source in the repo.
//
// NB `m` is legal in a `refresh:` position and meaningless in a CSS duration.
// That argues for per-POSITION unit sets rather than one global bag — recorded
// as crack C7; this type is the global set until a position-aware surface
// exists, exactly as the lexer's bag was.
// `h` (hours) joins `m` (minutes) as a POLLING duration rather than a CSS one:
// `@data fetch ... { refresh: 1h }` is real usage in examples/data-binding and
// stdlib/__host__. Both live here because `refresh:` shares this type with CSS
// durations; splitting the sets is crack C7's job, not this wave's.
// `f` (frames) is a SCORE duration: `--dissolve(6f)` is authored in frames at
// the project's 60fps reference rate. It lived only in a private Rust parser
// (`pipeline::drivers::parse_duration_ms`) which knew `ms`/`s`/`f` and nothing
// else — so `2m` meant 120000ms canonically and 300ms through a driver. That
// second parser is gone (PLAN-141 W4); the unit belongs in the shared set, and
// declaring it HERE is what keeps the grammar and the converter agreeing about
// which units exist. A unit the grammar accepts but the converter rejects (or
// the reverse) is a legal duration that silently becomes no duration at all.
%capture_type time_unit {
    "ms" | "s" | "m" | "h" | "us" | "fps" | "f"
}

%capture_type angle_unit {
    "deg" | "rad" | "grad" | "turn"
}

// A dimension is a number followed IMMEDIATELY by its unit. At kernel tier that
// is two tokens (`NUMBER IDENT`), which is precisely the shape the lexer used to
// fuse away — and the reason these grammars could not be written before W1.0.
// A percentage IS a length here. `50%` is legal wherever a length is (widths,
// offsets, `translate()`), and `score_measure` in stdlib/macros/score.st is
// `( $len:length ) | ( $time:duration )` — so dropping `%` from this set took
// `&title for 50%` with it and broke every clip driver. The old Rust
// `LengthExtractor` accepted `%` through `parse_length_with_unit`; the grammar
// has to say so too, because it is now the only thing that says anything.
%capture_type length {
    ( $n:number $u:length_unit ) | ( $n:number "%" ) | ( $wide:css_wide ) | ( $ref:binding )
}

%capture_type duration {
    ( $n:number $u:time_unit ) | ( $wide:css_wide ) | ( $ref:binding )
}

// `time` is `duration` under its older name, and it is the name most of the
// tree actually writes: ~69 sites say `$duration:time` (fade-in, loop, data,
// websocket, now, the 3d macros, the testing assertions, both migration
// entries) against a handful saying `:duration`.
//
// The alias is NOT deferred tidying. The Rust arm it replaces mapped BOTH
// spellings to one variant, so dropping either one is a silent regression
// rather than a loud one: an unregistered `%capture_type` name resolves
// through `form_compiler.rs`'s `.or_else(|| extractors.get(&Expr))` fallback,
// and `Expr` matches nearly anything. A missing `time` production would not
// fail a build — it would quietly switch 69 validated captures over to
// accepting arbitrary expressions, which is why the gate for this alias is a
// NEGATIVE one (`duration: banana` must be REFUSED). A positive-only test
// passes just as happily with the alias absent.
//
// Renaming the call sites instead would be the tidier end state, but it is a
// separate mechanical sweep across stdlib and user projects; the alias keeps
// this wave to one idea.
%capture_type time {
    $d:duration
}

%capture_type angle {
    ( $n:number $u:angle_unit ) | ( $wide:css_wide ) | ( $ref:binding )
}

// A percentage keeps its `%` as its own token at kernel tier.
%capture_type percentage {
    ( $n:number "%" ) | ( $wide:css_wide ) | ( $ref:binding )
}

// ---------------------------------------------------------------------------
// Easing
// ---------------------------------------------------------------------------

%capture_type easing_keyword {
    "linear" | "ease-in-out" | "ease-in" | "ease-out" | "ease"
  | "step-start" | "step-end"
}

// The function forms: `cubic-bezier(…)`, `steps(…)`, and the newer `linear(…)`
// stop-list. Balanced args for the same reason as colour functions — the shapes
// are per-function and already validated downstream.
%capture_type easing_fn_name {
    "cubic-bezier" | "steps" | "linear"
}

%capture_type easing_fn {
    $fn:easing_fn_name "(" $args:balanced(')') ")"
}

// Keyword alternatives are ordered LONGEST FIRST (`ease-in-out` before
// `ease-in` before `ease`). A leftmost-match alternation would otherwise accept
// `ease` and leave `-in-out` to break the surrounding sequence — the same
// longest-match discipline the counted class applies to hex lengths.
%capture_type easing {
    ( $fn:easing_fn ) | ( $keyword:easing_keyword ) | ( $wide:css_wide ) | ( $ref:binding )
}

// ---------------------------------------------------------------------------
// Shorthand routing
// ---------------------------------------------------------------------------
//
// A REACTIVE declaration is applied inline (`node.style.setProperty(...)`),
// because its value is not known until a signal renders and so cannot be
// written into the stylesheet. That is correct for a longhand and destructive
// for a SHORTHAND: setting `background` inline resets every `background-*`
// longhand to its initial value, at inline specificity, which outranks the
// static rule the author wrote three lines above it (BUG-279 — a
// gradient-clipped headline rendered as a solid slab).
//
// The property a value should really be applied to is a question about the
// VALUE'S KIND, and this file is where value kinds are already answered. So the
// routing table is a production: each arm is a kind, and the arm's capture NAME
// is the longhand to apply. `_` stands in for `-` (a capture name is an
// identifier), which is a mechanical substitution at the consumer, not a
// mapping table.
//
// Read it as: a reactive `background:` whose value is an image becomes
// `background-image`; whose value is a colour becomes `background-color`.
//
// WHY THE `_core` ARMS: `background: inherit` must stay a shorthand — inherit on
// the shorthand inherits ALL longhands, and routing it to one would silently
// narrow the author's instruction. Likewise `background: $sig`, whose kind is
// unknowable until it renders. Both fall through unrouted, which is exactly the
// behavior that existed before this table.
//
// A SECOND SHORTHAND IS A PARAGRAPH HERE. The consumer looks up
// `{property}_route` by name and knows nothing about backgrounds; `font_route`,
// `border_route` and the rest need no Rust change, which is the whole reason
// this is a grammar and not a match arm.
%capture_type background_route {
    ( $background_image:image_list ) | ( $background_color:color_core )
}

