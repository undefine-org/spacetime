//! Diagnostic error and warning codes.

use std::fmt;

/// Diagnostic codes for errors and warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    // === Syntax Errors (E001-E099) ===
    /// Missing semicolon
    E001,
    /// Unmatched brace
    E002,
    /// Invalid syntax
    E003,

    // === Resolution Errors (E100-E199) ===
    /// Unknown preset
    E100,
    /// Unknown variable
    E101,
    /// Unknown CSS property (with typo detection)
    E102,
    /// Unknown function
    E150,
    /// Wrong function arity
    E151,
    /// Wrong argument type
    E152,

    // === Pattern Expansion Errors (E160-E166) ===
    /// Unknown pattern reference
    E160,
    /// Circular pattern dependency
    E161,
    /// Missing required pattern parameter
    E162,
    /// Pattern parameter type error
    E163,
    /// List used in scalar context
    E164,
    /// Unbound variable in pattern
    E165,
    /// Non-exhaustive match in pattern
    E166,

    // === Metasystem Errors (E170-E189) ===
    /// Unknown primitive reference
    E170,
    /// Unknown macro reference
    E171,
    /// Missing required primitive parameter
    E172,
    /// Primitive parameter type mismatch
    E173,
    /// Invalid emit language
    E174,
    /// Unbound variable in emit block
    E175,
    /// Invalid yield target
    E176,
    /// (reserved)
    E177,
    /// Invalid form pattern
    E178,
    /// Unknown capture type in form
    E179,
    /// Circular macro dependency
    E180,
    /// Unbound variable in macro body
    E181,
    /// Invalid binds declaration
    E182,
    /// Primitive not found for binds
    E183,
    /// Export type mismatch
    E184,
    /// Duplicate export declaration
    E185,
    /// Invalid derives expression
    E186,
    /// Missing form clause for directive
    E187,
    /// Invalid trigger reference
    E188,
    /// Macro body item in invalid context
    E189,
    /// Unknown directive (no macro creates it)
    E190,
    /// Missing required argument for directive
    E191,
    /// Unexpected argument for directive
    E192,
    /// Argument type mismatch for directive
    E193,
    /// Unsupported macro argument type
    E194,
    /// Unconsumed positional arguments
    E195,
    /// Yield to undeclared export
    E196,
    /// Export never yielded
    E197,
    /// Legacy function type (bare fn without signature)
    E198,
    /// Unknown type reference in export
    E199,

    // === Type Errors (E200-E299) ===
    /// Wrong preset type for context
    E200,
    /// Type mismatch
    E201,
    /// Unresolved parameter in emit block
    E202,
    /// Maximum macro expansion depth exceeded
    E203,
    /// Optional inline capture without explicit default
    E204,
    /// Invalid %migration definition (date shape, rewrite/hint exclusivity,
    /// unknown hole, live-macro collision, backward chain)
    E205,

    // === Value Errors (E300-E399) ===
    /// Progress value out of range (must be 0-1)
    E300,
    /// Duration must be positive
    E301,
    /// Invalid color format
    E302,
    /// Invalid cubic-bezier parameters
    E305,
    /// Invalid spring parameters
    E306,

    // === Semantic Errors (E400-E499) ===
    /// Duplicate timeline ID
    E400,
    /// Duplicate preset name
    E401,
    /// Circular preset dependency
    E402,
    /// Referenced timeline not found
    E403,

    // === Warnings: Unused Code (W001-W099) ===
    /// Unused preset definition
    W001,
    /// Unreachable animation (offset >= 1.0)
    W002,
    /// Zero-duration animation
    W003,

    // === Warnings: Suspicious Code (W100-W199) ===
    /// Duplicate timeline ID in scope
    W100,
    /// Overlapping animation timing (stacked visibility overlap)
    W101,
    /// Extreme spring parameters
    W102,
    /// Missing out-animation for scroll waypoint
    W110,
    /// Same-target scroll range overlap
    W111,
    /// Invalid scroll range (start >= end)
    W112,
    /// Insufficient read time for scroll content section
    W113,
    /// No hold time (content never fully visible before fading out)
    W114,
    /// A selector-init primitive body declares `el`, shadowing the injected element parameter.
    W0115,

    // === Type System Errors (E0101-E0199) ===
    /// Circular type dependency
    E0101,
    /// Unknown type reference
    E0102,

    // === Binding Errors (E0401-E0499) ===
    /// Property does not exist on type
    E0401,
    /// Template does not exist
    E0402,
    /// Slot does not exist in template
    E0403,
    /// Type mismatch (filter, binding)
    E0404,
    /// Function does not exist
    E0405,
    /// Wrong number of function arguments
    E0406,
    /// Invalid function argument type
    E0407,

    // === JSON Schema Errors (E0501-E0599) ===
    /// JSON schema mismatch (missing field)
    E0501,
    /// JSON schema mismatch (wrong type)
    E0502,
    /// JSON schema mismatch (invalid enum)
    E0503,
    /// JSON file not found
    E0504,
    /// JSON parse error
    E0505,

    // === Data System Warnings (W0201-W0399) ===
    /// Unused data source
    W0201,
    /// Unused template
    W0202,
    /// Unused type definition
    W0203,
    /// Unused helper function
    W0204,
    /// `@data` source declaration present but did not resolve to a usable
    /// source (file / inline / localStorage). Signals capture-shape drift
    /// between a `@data <kind>` macro and the analysis that reads it.
    W0205,
    /// State has no incoming transitions
    W0301,
    /// State has no outgoing transitions
    W0302,
    /// Duplicate state definition
    W0303,

    // === Signal Analysis (W0401-W0403, E0408) ===
    /// Unused signal definition (signal defined but never used)
    W0401,
    /// Undefined signal reference (signal used but not defined)
    E0408,
    /// Signal shadows outer signal
    W0403,

    // === Warnings: Visual Consistency (W201-W205) ===
    /// Low contrast text (WCAG 2.1 AA)
    W201,
    /// Invisible element (opacity:0 without reveal animation)
    W202,
    /// Untranslated content (text without data-t when @locale active)
    W203,
    /// Antipattern: inline event handler (use Spacetime @on bindings)
    W204,
    /// Unresolved {locale} template
    W205,

    // === Element Dependency Analysis (W0501-W0502) ===
    /// Circular element ref dependency detected
    W0501,
    /// Dependency on animating element (performance info)
    W0502,

    // === HTML Validation Errors (E0601-E0699) ===
    /// @each container selector not found in HTML
    E0601,
    /// Animation target selector not found in HTML
    E0602,
    /// Slot not found in template
    E0603,
    /// Element reference selector not found
    E0604,
    /// Template/component not defined
    E0605,
    /// Nested scope selector not found
    E0606,

    // === HTML Validation Warnings (W0601-W0699) ===
    /// Element has no animations defined
    W0601,
    /// Large section could benefit from scroll reveal
    W0602,
    /// Selector could be more specific
    W0603,
    /// Many stagger animations may impact performance
    W0604,
    /// Consider @on visible for below-fold content
    W0605,

    // === Output Validation Errors (E0701-E0799) ===
    /// Invalid generated JavaScript syntax
    E0701,
    /// Invalid generated CSS syntax
    E0702,

    // === Emit/Resolution Errors (E0800-E0899) ===
    /// Unknown parameter in emit resolution
    E0800,
    /// Unknown element in emit resolution
    E0801,
    /// Signal marker invalid in CSS context
    E0802,
    /// VarRef marker invalid in CSS context
    E0803,
    /// Directive marker invalid in CSS context
    E0804,
    /// Unresolved loop variable
    E0805,
    /// Unknown directive
    E0806,
    /// Unresolved placeholder reached emission (internal)
    E0807,
    /// Unresolved marker reached emission (internal)
    E0808,

    // === Stdlib Loading Errors (E0810-E0812) ===
    /// Stdlib file not found
    E0810,
    /// Stdlib parse error
    E0811,
    /// Duplicate macro definition in stdlib
    E0812,

    // === Component Body Errors (E0900-E0904) ===
    /// Invalid content type in component body
    E0900,
    /// Unclosed HTML tag in component body
    E0901,
    /// Invalid CSS selector in component body
    E0902,
    /// Invalid state declaration in component body
    E0903,
    /// Invalid export declaration in component body
    E0904,

    // === Scoped State Errors (E0905) ===
    /// $var reference to undefined variable in template scope
    E0905,
    /// Unsupported state action expression in @on body
    E0906,

    // === Reactive Property Errors (E0908-E0909) ===
    /// .class: used with non-boolean expression
    E0908,
    /// <- used outside selector block (content injection requires component body)
    E0909,
    /// Removed reactive macro (@bind / @show / @input) — use the : / <- surface (FEAT-072)
    E0910,
    /// Invalid `@version` declaration (duplicate, non-root, or malformed date)
    E0911,
    /// Syntax-migration apply exceeded its wave fuel (cycle) — stdlib authoring error
    E0912,

    // === Template Ref Errors (E0914-E0919) ===
    /// &ref.$var: $var is not exported by template
    E0914,
    /// &ref.$var <- value: export $var is read-only (not mut)
    E0915,
    /// &ref references undefined template instance
    E0916,
    /// @exports declares $var that is not defined in template
    E0917,
    /// &name[] collection ref used outside @each context
    E0918,
    /// Duplicate template instance name &name in same scope
    E0919,
    /// &name .selector { @each } without [] — singular ref contains iteration
    E0920,
    /// &name[] .selector { } without @each — collection marker but no iteration
    E0921,
    /// Malformed @match block — a segment begins `@match` but does not reify (e.g. an arm
    /// missing its trailing `;`). Surfaced instead of silently dropping the block.
    E0922,
    /// Ambiguous dispatch — a directive call's top two candidate macros tie on score,
    /// so resolution silently picks one by iteration order. Add a disambiguating
    /// literal/type. Warning, not error: first-match still resolves (FEAT-088).
    E0923,
    /// Ambiguous NAMESPACE dispatch (FEAT-118 FUP-055) — a bare directive call
    /// resolves to the same name in two or more imported modules. Qualify it
    /// (`@alias/name`) to disambiguate. Warning: first-match still resolves.
    E0924,
    /// Internal invariant break (FEAT-119) — a body-bearing construct's factory
    /// `body` could not resolve its World-A `@template:<name>` scope at emit. Every
    /// such construct (@template, &name(){}, @editable-*) is named and its scope is
    /// built during parsing, so this is a compiler bug, never author error.
    E0925,
    /// Unbound import qualifier (FEAT-118 FUP-057) — a qualified call
    /// `@q/name` names a qualifier `q` that is neither an `as`-alias nor a
    /// path-suffix of any `@use`'d namespace in this file. Almost always a typo
    /// in the alias or a missing `@use`. Error: the reference cannot resolve to
    /// the intended module.
    E0926,
    /// Import visibility violation (FEAT-118 FUP-057) — a reference names an
    /// imported definition that the file's `@use ... only (...)` allow-list
    /// excludes, or its `hiding (...)` deny-list removes. Error: the name was
    /// deliberately scoped out of this file.
    E0927,

    /// A `@host $b : live("Mod")` page fires (or decodes) an event the server
    /// LiveView module does not declare in its contract. The page's `send emit
    /// "X"` names an event absent from the module's `__spacetime_contract__`
    /// (sidecar `<page>.contract.json`). Wave B self-aware check (FEAT-135).
    E0928,

    /// Colon in a `@template`/`@editable-*` parameter declaration. A parameter
    /// declares its TYPE with a space (`$price number`) and its DEFAULT with `=`
    /// (`$title = "x"`); `:` is never valid in a param declaration. Previously a
    /// stray `:` (e.g. `$a: "one", $b: "two"`) SILENTLY truncated the list at the
    /// first param (the PEG could not consume `:` so the `( … )*` loop stopped and
    /// dropped every later param). Now surfaced as an error instead of losing data.
    E0929,

    /// Non-exhaustive match-derive (PLAN-077) — a cond-mode
    /// `@data derive $x T : @match { (guard) => Variant; }` whose arm chain does
    /// NOT terminate in a `_` catch-all. Guards are arbitrary booleans, so
    /// coverage is undecidable; the checkable invariant is that the chain always
    /// terminates in a value. Also fires when `_` appears in a NON-last
    /// position — first-match-wins makes every later arm unreachable. Error:
    /// without the catch-all the derive can publish nothing (stale/null value).
    E0931,
    /// Non-exhaustive variant dispatch (PLAN-077) — a dispatch-mode
    /// `@match $subject { … }` / `@view $subject { … }` whose subject's declared
    /// type IS a known `@type` sum (TypeRegistry), where the arm set covers
    /// neither all declared variants nor includes a `_` wildcard fallback.
    /// Unlike E0931 this IS decidable (closed variant set) — real
    /// exhaustiveness, Rust-`match`-style. Only fires on a POSITIVE sum-type
    /// match; string subjects and unknown types are skipped, never guessed.
    E0932,
    /// Unknown variant constructor (PLAN-077) — a dispatch-mode destructure
    /// pattern (`Variant { $b }`), a cond-mode consequence (`=> Variant;`), or
    /// a `@state(when: $x is Variant)` names a constructor that is NOT a
    /// declared variant of the subject/target enum's `@type` sum (or, for a
    /// cond-mode derive with an inline anonymous union `(A | B)`, not in that
    /// literal's variant list). Almost always a typo. Only fires on a positive
    /// sum-type/inline-union match — unknown types are skipped.
    E0933,
    /// Variant payload binding mismatch (PLAN-077) — a destructured
    /// `Variant { $a, $b }` pattern (dispatch arm or `@state(when:)`), or a
    /// cond-mode consequence's payload args, has a COUNT different from the
    /// variant's declared payload arity in `@type Name { Variant(A, B) }`.
    /// Only fires on a positive sum-type match.
    E0934,
    /// Duplicate page-global state declaration (PLAN-117 W2) — two files
    /// declare the same root-level `$name`, and `@import` merged them into ONE
    /// cell without saying so.
    ///
    /// Every other registry already refuses a duplicate definition
    /// (`DuplicateMacro`, `DuplicatePrimitive`); `$` was the sole exempt kind,
    /// so an accidental name clash across the import graph silently produced a
    /// page where one file's initial value won and the other's vanished. A page
    /// holds one cell per name, exactly as it holds one macro per name.
    ///
    /// Fix: rename one declaration, or (once PLAN-117 W3 lands) qualify the
    /// reference through an import alias.
    E0938,
    /// Ambiguous tight `/$` with a numeric head (PLAN-117 W3) — `1/$denominator`.
    ///
    /// A number can never be a module qualifier, so this IS division; but it is
    /// spelled exactly like the qualified-reference form `app/$host`. Rather
    /// than silently pick a reading, the compiler refuses and names the fix
    /// (add spaces). This is the one deliberate cost of the `/$` form.
    E0939,
    /// Unknown module qualifier on a cell reference (PLAN-117 W3) — `app/$host`
    /// where no `@use … as app` (and no module named `app`) is in scope.
    ///
    /// The `$`-registry mirror of E0926 (unbound qualifier for `@`). Before
    /// this, an unresolvable qualified reference PARSED CLEAN and emitted
    /// nothing — a silent drop.
    E0940,
    /// The named module publishes no such cell (PLAN-117 W3) — `app/$nope`
    /// where `app` resolves but declares no page-global `$nope`.
    E0941,
    /// Write through a module qualifier (PLAN-117 W3) — `app/$session <- v`.
    ///
    /// Reads may cross a module boundary; WRITES stay in the owning file. This
    /// is not ceremony: it is what makes const-folding SOUND. A cell's write-set
    /// must be settleable by scanning ONE file, and that is only true while a
    /// qualified reference cannot mutate. The mutation surface belongs to the
    /// owner — expose an event and let the owner write its own cell.
    ///
    /// A BARE write to an imported cell stays legal: a file that has taken the
    /// cell into its own scope writes it as its own.
    E0942,
    /// A colon used where a module qualifier needs a slash (BUG-232) — `@b:badge`.
    ///
    /// The CST lexer ends a directive-name run at `:`, so the directive matched
    /// no `%form` and was DROPPED IN SILENCE: `@b/badge` emits
    /// `.badge::before` into the stylesheet, `@b:badge` reports success and
    /// emits nothing. A silent drop is the worst possible outcome for an
    /// addressing mistake.
    E0943,
    /// Reading a cell the owning module does not publish (PLAN-117 W5) —
    /// `app/$internal` where `app`'s `@exports` clause omits `$internal`.
    ///
    /// A file with NO `@exports` publishes everything (public-by-default, the
    /// Odin floor), so this only fires once a module has stated its interface.
    /// The clause is the same `@exports { $x, $y: mut }` a `@template` body uses
    /// (FEAT-115), lifted to file scope.
    E0944,
    /// Malformed `@data subscribe` declaration (BUG-234). The `subscribe` keyword
    /// commits the directive to the live-data grammar; it must not fall through to
    /// a no-op/local declaration when its required `from` clause is missing.
    E0945,
    /// @on driver resolution (PLAN-124 W3.2): the projection member names no
    /// registered driver, the driver is `planned:` (registered but its
    /// primitive has not landed), or the motion form in the form slot is not
    /// declared. Before the drive expansion existed, the head grammar simply
    /// rejected the projection and the directive fell through silently.
    E0948,
    /// @on driver KIND error (PLAN-124 W3.2): the projection does not apply to
    /// this kind of ref (`&.playback` on a plain element), or the form slot
    /// holds a non-motion form. Distinct from E0948 (unknown/unbound) because
    /// the fix is a different subject or a different form, not a declaration.
    E0949,
    /// PLAN-126: facet-type/position mismatch (`$sig.done: $x <- …`, wrong consequence kind)
    E0950,
    /// E-MEASURE-DOMAIN (SIP-001 W4/R1): a `@score` placement is written in a
    /// measure its DRIVER'S time domain does not have — `at 2s` under
    /// `&.scroll`, whose progress is normalized 0..1, or `for 30%` under a
    /// driver whose domain is declared in time.
    ///
    /// A score never declares a duration of its own; it inherits the domain of
    /// whatever drives it, read from the `domain:` column of the driver
    /// registry. That is why this is one diagnostic rather than a rule per
    /// driver: the registry knows each driver's domain, so both the check and
    /// its did-you-mean are derived from data.
    ///
    /// Without it, `at 2s` under a scroll driver is a silently meaningless
    /// number — the banned class.
    E0951,
    /// The compiler emitted JavaScript that does not parse (BUG-262).
    ///
    /// ALWAYS a compiler bug, never an author error. A bundle that does not
    /// parse never evaluates, so a one-character emit defect kills the whole
    /// page — which is exactly what `text <- ($a || $b)` did before the
    /// filter-pipe split learned about `||`.
    E0952,
    /// A driver's `as` clause was written with an element sigil (BUG-261).
    ///
    /// `as &reveal` used to fail the optional capture and vanish, publishing
    /// under a counter fallback instead. One sigil, one meaning: a driver's
    /// publication is a progress SIGNAL, so it takes `$`.
    E0953,
    /// A data-defined error sibling consumed a malformed optional directive clause
    /// (BUG-264). The literal clause was present but its typed value was not;
    /// refuse it rather than silently discard author input.
    E0955,
    /// A resolved primitive whose name resolves to nothing (BUG-268).
    ///
    /// The expand layer used to emit a `// Primitive not found: <name>` comment
    /// INTO THE SHIPPED BUNDLE and report success — the banned silent-drop
    /// class. A primitive that is neither a registered `%primitive` nor a macro
    /// with `%emit` blocks can only be a genuine authoring error (a `%binds`
    /// binding a `%primitive` that does not exist), so it is now a hard build
    /// failure.
    E0956,
    /// PLAN-133: a bare `$` row reference (`$.field`) appeared in a `sexpr`
    /// derivation expression, which has no current-row context. `$` row refs
    /// are only meaningful inside per-item query expressions (@data query).
    E0957,
    /// A malformed value in a plain CSS declaration (FUP-176).
    ///
    /// `.x { color: #e8ee1 }` — a known property whose value its own CSS type
    /// refuses. The browser silently drops such a declaration, so without this
    /// the author sees an element render wrong with no diagnostic anywhere.
    ///
    /// Deliberately narrow: only fires when the property demonstrably ACCEPTS a
    /// scalar of some type and the author's value fails where a known-good
    /// sentinel of that type succeeds. CSS-wide keywords and `var()`/`env()`
    /// substitutions always pass — see `validate_declaration_value`.
    E0958,
    /// A statement-position form splice (`--name;` in a bare scope) names a
    /// form whose KIND is not `style` (BUG-298). Only a style form has
    /// declarations to splice into a scope; an easing/motion/score/value/markup
    /// form there is a category error that would otherwise compile green and
    /// contribute nothing (the arc's banned silent-acceptance class).
    E0959,
    /// A form application passes a NAMED argument the declaration does not
    /// have (BUG-298) — `--surface(padding: 3rem)` against
    /// `@form style --surface($pad = 2rem)`. Silently ignoring it would drop
    /// the author's intent; the did-you-mean lists the declared parameters.
    E0960,
    /// A typed `@data inline` seed's field disagrees with its declared type
    /// (FEAT-166). Either the field is not declared on the type, or its value
    /// fails the grammar of the scalar the type declares for it.
    E0961,
    /// An easing reference (`easing: --name`) names a form no `@form easing`
    /// declaration registers (BUG-297). The runtime easing map resolves by
    /// bare name and silently falls back to `linear` on a miss, so without
    /// this a typo'd easing compiles green and ships different motion.
    E0962,
    /// A macro declared `%scope element(<tag>)` is bound to a selector whose
    /// implied element cannot be that tag (GH-12). E.g. `@stage` — which only
    /// renders into a `<canvas>` — placed on `div.hero` compiled clean and
    /// failed only at runtime (three.js `canvas.getContext` throw). The compile
    /// now refuses it, naming the required element.
    E0963,
    /// A `%form` capture is never consumed (gh-18). Every capture a macro's
    /// `%form` binds must be passed to a primitive in `%binds`, referenced in an
    /// `%emit` block or a `%when`/`%derives`/`%includes` clause, or the macro
    /// author declares it intentionally-unused by prefixing its name with `_`
    /// (e.g. `$_:skip_block`). A capture that is bound and never read is a
    /// half-implemented macro: the author's intent silently vanishes. This is a
    /// lint over registry DATA — the form grammar already records every capture.
    E0964,
    /// An element reference `&$name` names a reference that was never declared
    /// (BUG-333). `&name <selector>;` (the element-ref macro) is the only thing
    /// that registers a name in the element-reference registry, so a `&$name`
    /// read with no matching declaration is an authoring mistake. Before this
    /// code existed it degraded into emitted-but-broken JS (`&ST.resolve(...)`
    /// — a dangling `&`) that failed to parse, killing the whole page with an
    /// E0952 that pointed at a byte offset in a generated bundle instead of the
    /// source line. Refuse it, naming the reference and listing the declared
    /// refs in scope, instead of shipping a dead bundle.
    E0965,
    /// A style form splices itself, directly or through a chain (BUG-327).
    /// `@form style --a { --b; }` + `@form style --b { --a; }` has no fixed
    /// point, so expansion would loop forever. Refused by name, with the chain
    /// printed — never a hang, and never the silently empty rule that the
    /// unexpanded case used to produce.
    E0966,
    /// A directive property's value fails the capture type its own `%form`
    /// declares for it (PLAN-136 W4).
    ///
    /// The expected type is DERIVED from the form declaration
    /// (`easing: $easing:easing`), never from a table — so this fires only
    /// where a form actually stated a contract, and stays silent for a property
    /// nobody declared.
    ///
    /// NB: this was E0963 on its own branch; the merge found PLAN-137 had
    /// shipped E0963 for `%scope element(<tag>)` first, so it moved here rather
    /// than renumbering a code already in released diagnostics.
    E0967,
    ///
    /// A negative rate means a reversed origin, so `rate 1 -> -1` has no
    /// single interpretation: it could advance then retreat, or be two clips.
    /// Refuse it rather than silently choosing one.
    E0954,
    /// A statement-position form splice (`--name;`) names a form no `@form`
    /// declaration has registered (BUG-241). Before the parse_body_item dispatch
    /// arm existed, the splice was silently DROPPED by the CSS-property
    /// fallthrough — recognized or not made no difference to the output. The arm
    /// recognizes it, and this code makes an unknown name a hard error with a
    /// did-you-mean over the declared forms, so a typo'd splice can never again
    /// vanish from an author's stylesheet.
    E0947,
    /// A keyframe value stop carries a POSITION (`0.85 at 25%`) or a per-step
    /// EASING form (`40px -> 0 --ease-out-expo -> …`) — the film-surface syntax
    /// of `docs/language/film.st.md` §2, which PLAN-150 W1 lands. Until then the
    /// value string flows into a keyframe untouched and the runtime parses
    /// `NaN`, so the stop is silently dead. This code makes the not-yet-landed
    /// syntax a hard, self-describing error instead of a green build that does
    /// not move (the exact silent-drop class the film-doc gate exists to close).
    E0968,
    /// `@reveal(split: outline)` — the SVG-path text mode of
    /// `docs/language/film.st.md` §11 (PLAN-150 W10), not yet landed. The reveal
    /// primitive accepts only `chars | words | lines` today, so an unrecognized
    /// split mode compiled green and animated nothing. Refused by name until the
    /// mode ships.
    E0969,
    /// A directive the film surface RESERVES but has not landed yet was used
    /// (`@scatter`, and the like) — docs/language/film.st.md names the wave in
    /// PLAN-150 that lands it. Without this the word fell through to W0714 (a
    /// warning) and the build stayed green while the feature did nothing. The
    /// reserved set turns "unknown directive, ignored" into "known directive,
    /// not yet built" — a hard error that names its wave.
    E0970,
    /// A directive body failed the `%capture_type` grammar it declared (BUG-229).
    /// The directive's prefix DID match, so this form was the author's intent; the
    /// declared capture type then rejected the body's content. Before this code
    /// existed, such a body was passed through as raw text and compiled green —
    /// which made a grammar that could never match indistinguishable from one that
    /// worked, and left grammar spikes unverifiable.
    E0946,

    // === Component Body Warnings (W0700-W0701) ===
    /// Interleaved HTML and CSS sections (recommend grouping)
    W0700,
    /// CSS rule targets no HTML elements in component body
    W0701,
    /// $var shadows parent scope variable
    W0702,
    /// $var declared but never read in template scope
    W0703,
    /// @bind is deprecated — use reactive properties instead
    W0707,
    /// Mixing @bind with reactive properties in same selector
    W0706,
    /// Template instance &name created but never referenced
    W0709,
    /// Export $var declared but never read from outside
    W0710,
    /// `text <- $expr` is deprecated — use `&self.content <- $expr` instead
    W0711,
    /// `@each`/`@view` placed directly in a @template body's MARKUP is silently
    /// dropped (BUG-130) — wrap it in a selector scope (`.sel { @each(…) { … } }`).
    W0712,
    /// `%animates { <prop>: $sig }` names a property that is neither a known
    /// transform channel (translate-*/scale*/rotate*) nor an obviously animatable
    /// CSS property — it falls back to a raw `el.style[<prop>]` write that may be
    /// invalid CSS (PLAN-054 audit #6).
    W0713,
    /// A top-level `@ident { ... }` directive matched no known directive/macro/primitive
    /// form and was silently ignored (BUG-133).
    W0714,
    /// Old syntax was auto-migrated in memory by a stdlib %migration entry
    /// (PLAN-076 compat shim) — persist via the migrations pill or
    /// `spacetime migrate`.
    W0715,
    /// A `<-` variant construction in a `@handle` receive arm was NOT lowered
    /// (PLAN-077 W4) because the arm's bind name collides with a key of the
    /// emitted union object (`type` or a payload field) — runMutations'
    /// bare-name locals substitution would corrupt the object at runtime.
    /// Rename the bind or construct the union in a `@data derive` instead.
    W0716,
    /// A `%comment_type` (PLAN-123) was declared in a PAGE file, where it
    /// compiles but never joins the comments roster — so the author's
    /// `//@<type>` comments would report "unknown type" for no visible
    /// reason. Project-scoped declarations belong in `_prelude.st`, the
    /// project overlay, which is auto-loaded before any page compiles.
    W0717,
    /// A score rate leaves part of its window or progress unused (BUG-259).
    W0718,
    /// A macro declared `%scope element(<tag>)` is bound to a selector whose
    /// implied element is UNKNOWABLE (no tag in the selector, e.g. `.hero`, or
    /// a runtime-inserted node). GH-12. Warn-not-error: the element may be the
    /// required tag created at runtime, so this is not refused — but it must
    /// never pass silently, because a wrong element is exactly how the runtime
    /// failed with no compile signal.
    W0963,
}

impl DiagnosticCode {
    /// Get the string code (e.g., "E102", "W001").
    pub fn as_str(&self) -> &'static str {
        match self {
            // Syntax
            DiagnosticCode::E001 => "E001",
            DiagnosticCode::E002 => "E002",
            DiagnosticCode::E003 => "E003",
            // Resolution
            DiagnosticCode::E100 => "E100",
            DiagnosticCode::E101 => "E101",
            DiagnosticCode::E102 => "E102",
            DiagnosticCode::E150 => "E150",
            DiagnosticCode::E151 => "E151",
            DiagnosticCode::E152 => "E152",
            // Pattern Expansion
            DiagnosticCode::E160 => "E160",
            DiagnosticCode::E161 => "E161",
            DiagnosticCode::E162 => "E162",
            DiagnosticCode::E163 => "E163",
            DiagnosticCode::E164 => "E164",
            DiagnosticCode::E165 => "E165",
            DiagnosticCode::E166 => "E166",
            // Metasystem
            DiagnosticCode::E170 => "E170",
            DiagnosticCode::E171 => "E171",
            DiagnosticCode::E172 => "E172",
            DiagnosticCode::E173 => "E173",
            DiagnosticCode::E174 => "E174",
            DiagnosticCode::E175 => "E175",
            DiagnosticCode::E176 => "E176",
            DiagnosticCode::E177 => "E177",
            DiagnosticCode::E178 => "E178",
            DiagnosticCode::E179 => "E179",
            DiagnosticCode::E180 => "E180",
            DiagnosticCode::E181 => "E181",
            DiagnosticCode::E182 => "E182",
            DiagnosticCode::E183 => "E183",
            DiagnosticCode::E184 => "E184",
            DiagnosticCode::E185 => "E185",
            DiagnosticCode::E186 => "E186",
            DiagnosticCode::E187 => "E187",
            DiagnosticCode::E188 => "E188",
            DiagnosticCode::E189 => "E189",
            DiagnosticCode::E190 => "E190",
            DiagnosticCode::E191 => "E191",
            DiagnosticCode::E192 => "E192",
            DiagnosticCode::E193 => "E193",
            DiagnosticCode::E194 => "E194",
            DiagnosticCode::E195 => "E195",
            DiagnosticCode::E196 => "E196",
            DiagnosticCode::E197 => "E197",
            DiagnosticCode::E198 => "E198",
            DiagnosticCode::E199 => "E199",
            // Type
            DiagnosticCode::E200 => "E200",
            DiagnosticCode::E201 => "E201",
            DiagnosticCode::E202 => "E202",
            DiagnosticCode::E203 => "E203",
            DiagnosticCode::E204 => "E204",
            DiagnosticCode::E205 => "E205",
            // Value
            DiagnosticCode::E300 => "E300",
            DiagnosticCode::E301 => "E301",
            DiagnosticCode::E302 => "E302",
            DiagnosticCode::E305 => "E305",
            DiagnosticCode::E306 => "E306",
            // Semantic
            DiagnosticCode::E400 => "E400",
            DiagnosticCode::E401 => "E401",
            DiagnosticCode::E402 => "E402",
            DiagnosticCode::E403 => "E403",
            // Warnings: Unused
            DiagnosticCode::W001 => "W001",
            DiagnosticCode::W002 => "W002",
            DiagnosticCode::W003 => "W003",
            // Warnings: Suspicious
            DiagnosticCode::W100 => "W100",
            DiagnosticCode::W101 => "W101",
            DiagnosticCode::W102 => "W102",
            DiagnosticCode::W110 => "W110",
            DiagnosticCode::W111 => "W111",
            DiagnosticCode::W112 => "W112",
            DiagnosticCode::W113 => "W113",
            DiagnosticCode::W114 => "W114",
            DiagnosticCode::W0115 => "W0115",
            // Visual Consistency
            DiagnosticCode::W201 => "W201",
            DiagnosticCode::W202 => "W202",
            DiagnosticCode::W203 => "W203",
            DiagnosticCode::W204 => "W204",
            DiagnosticCode::W205 => "W205",
            // Type System
            DiagnosticCode::E0101 => "E0101",
            DiagnosticCode::E0102 => "E0102",
            // Binding
            DiagnosticCode::E0401 => "E0401",
            DiagnosticCode::E0402 => "E0402",
            DiagnosticCode::E0403 => "E0403",
            DiagnosticCode::E0404 => "E0404",
            DiagnosticCode::E0405 => "E0405",
            DiagnosticCode::E0406 => "E0406",
            DiagnosticCode::E0407 => "E0407",
            // JSON Schema
            DiagnosticCode::E0501 => "E0501",
            DiagnosticCode::E0502 => "E0502",
            DiagnosticCode::E0503 => "E0503",
            DiagnosticCode::E0504 => "E0504",
            DiagnosticCode::E0505 => "E0505",
            // Data System Warnings
            DiagnosticCode::W0201 => "W0201",
            DiagnosticCode::W0202 => "W0202",
            DiagnosticCode::W0203 => "W0203",
            DiagnosticCode::W0204 => "W0204",
            DiagnosticCode::W0205 => "W0205",
            DiagnosticCode::W0301 => "W0301",
            DiagnosticCode::W0302 => "W0302",
            DiagnosticCode::W0303 => "W0303",
            // Signal Analysis
            DiagnosticCode::W0401 => "W0401",
            DiagnosticCode::E0408 => "E0408",
            DiagnosticCode::W0403 => "W0403",
            // Element Dependency Analysis
            DiagnosticCode::W0501 => "W0501",
            DiagnosticCode::W0502 => "W0502",
            // HTML Validation Errors
            DiagnosticCode::E0601 => "E0601",
            DiagnosticCode::E0602 => "E0602",
            DiagnosticCode::E0603 => "E0603",
            DiagnosticCode::E0604 => "E0604",
            DiagnosticCode::E0605 => "E0605",
            DiagnosticCode::E0606 => "E0606",
            // HTML Validation Warnings
            DiagnosticCode::W0601 => "W0601",
            DiagnosticCode::W0602 => "W0602",
            DiagnosticCode::W0603 => "W0603",
            DiagnosticCode::W0604 => "W0604",
            DiagnosticCode::W0605 => "W0605",
            // Output Validation Errors
            DiagnosticCode::E0701 => "E0701",
            DiagnosticCode::E0702 => "E0702",
            // Emit/Resolution Errors
            DiagnosticCode::E0800 => "E0800",
            DiagnosticCode::E0801 => "E0801",
            DiagnosticCode::E0802 => "E0802",
            DiagnosticCode::E0803 => "E0803",
            DiagnosticCode::E0804 => "E0804",
            DiagnosticCode::E0805 => "E0805",
            DiagnosticCode::E0806 => "E0806",
            DiagnosticCode::E0807 => "E0807",
            DiagnosticCode::E0808 => "E0808",
            // Stdlib Loading Errors
            DiagnosticCode::E0810 => "E0810",
            DiagnosticCode::E0811 => "E0811",
            DiagnosticCode::E0812 => "E0812",
            // Component Body
            DiagnosticCode::E0900 => "E0900",
            DiagnosticCode::E0901 => "E0901",
            DiagnosticCode::E0902 => "E0902",
            DiagnosticCode::E0903 => "E0903",
            DiagnosticCode::E0904 => "E0904",
            DiagnosticCode::E0905 => "E0905",
            DiagnosticCode::E0906 => "E0906",
            DiagnosticCode::E0908 => "E0908",
            DiagnosticCode::E0909 => "E0909",
            DiagnosticCode::E0910 => "E0910",
            DiagnosticCode::E0911 => "E0911",
            DiagnosticCode::E0912 => "E0912",
            // Template Ref
            DiagnosticCode::E0914 => "E0914",
            DiagnosticCode::E0915 => "E0915",
            DiagnosticCode::E0916 => "E0916",
            DiagnosticCode::E0917 => "E0917",
            DiagnosticCode::E0918 => "E0918",
            DiagnosticCode::E0919 => "E0919",
            DiagnosticCode::E0920 => "E0920",
            DiagnosticCode::E0921 => "E0921",
            DiagnosticCode::E0922 => "E0922",
            DiagnosticCode::E0923 => "E0923",
            DiagnosticCode::E0924 => "E0924",
            DiagnosticCode::E0925 => "E0925",
            DiagnosticCode::E0926 => "E0926",
            DiagnosticCode::E0927 => "E0927",
            DiagnosticCode::E0928 => "E0928",
            DiagnosticCode::E0929 => "E0929",
            DiagnosticCode::E0931 => "E0931",
            DiagnosticCode::E0932 => "E0932",
            DiagnosticCode::E0933 => "E0933",
            DiagnosticCode::E0934 => "E0934",
            DiagnosticCode::E0938 => "E0938",
            DiagnosticCode::E0939 => "E0939",
            DiagnosticCode::E0940 => "E0940",
            DiagnosticCode::E0941 => "E0941",
            DiagnosticCode::E0942 => "E0942",
            DiagnosticCode::E0943 => "E0943",
            DiagnosticCode::E0944 => "E0944",
            DiagnosticCode::E0945 => "E0945",
            DiagnosticCode::E0946 => "E0946",
            DiagnosticCode::E0947 => "E0947",
            DiagnosticCode::E0948 => "E0948",
            DiagnosticCode::E0949 => "E0949",
            DiagnosticCode::E0950 => "E0950",
            DiagnosticCode::E0951 => "E0951",
            DiagnosticCode::E0952 => "E0952",
            DiagnosticCode::E0953 => "E0953",
            DiagnosticCode::E0954 => "E0954",
            DiagnosticCode::E0955 => "E0955",
            DiagnosticCode::E0956 => "E0956",
            DiagnosticCode::E0957 => "E0957",
            DiagnosticCode::E0958 => "E0958",
            DiagnosticCode::E0959 => "E0959",
            DiagnosticCode::E0960 => "E0960",
            DiagnosticCode::E0961 => "E0961",
            DiagnosticCode::E0962 => "E0962",
            DiagnosticCode::E0963 => "E0963",
            DiagnosticCode::E0964 => "E0964",
            DiagnosticCode::E0965 => "E0965",
            DiagnosticCode::E0966 => "E0966",
            DiagnosticCode::E0967 => "E0967",
            DiagnosticCode::E0968 => "E0968",
            DiagnosticCode::E0969 => "E0969",
            DiagnosticCode::E0970 => "E0970",
            DiagnosticCode::W0700 => "W0700",
            DiagnosticCode::W0701 => "W0701",
            DiagnosticCode::W0702 => "W0702",
            DiagnosticCode::W0703 => "W0703",
            DiagnosticCode::W0706 => "W0706",
            DiagnosticCode::W0707 => "W0707",
            DiagnosticCode::W0709 => "W0709",
            DiagnosticCode::W0710 => "W0710",
            DiagnosticCode::W0711 => "W0711",
            DiagnosticCode::W0712 => "W0712",
            DiagnosticCode::W0713 => "W0713",
            DiagnosticCode::W0714 => "W0714",
            DiagnosticCode::W0715 => "W0715",
            DiagnosticCode::W0716 => "W0716",
            DiagnosticCode::W0717 => "W0717",
            DiagnosticCode::W0718 => "W0718",
            DiagnosticCode::W0963 => "W0963",
        }
    }

    /// Parse a code string back into its variant — the exact inverse of
    /// `as_str`.
    ///
    /// # Why this exists
    ///
    /// A diagnostic's code round-trips through `PipelineErrorInfo`, which
    /// carries it as a `String`. Rebuilding the variant used to mean an
    /// arm-per-code table hand-maintained at the call site in `main.rs`, whose
    /// fallback silently RELABELLED any absent code as E0806 ("unknown
    /// directive"). That is a silent-corruption hazard, not a cosmetic one: a
    /// correct diagnostic was reported under a wrong, unrelated name, and the
    /// only symptom was a confusing error message. It caught E0900, E0904,
    /// E0906, E0938 and E0946 in turn — each found by a human reading a wrong
    /// code, each fixed by appending one more line to the same doomed table.
    ///
    /// Deriving the inverse HERE, beside `as_str`, is the structural fix
    /// (FUP-148): the two tables sit together, `unknown_code_is_none` locks the
    /// fallback's honesty, and `every_code_round_trips` proves totality — a new
    /// variant cannot be half-registered.
    pub fn from_code_str(s: &str) -> Option<DiagnosticCode> {
        match s {
            "E001" => Some(DiagnosticCode::E001),
            "E002" => Some(DiagnosticCode::E002),
            "E003" => Some(DiagnosticCode::E003),
            "E0101" => Some(DiagnosticCode::E0101),
            "E0102" => Some(DiagnosticCode::E0102),
            "E0401" => Some(DiagnosticCode::E0401),
            "E0402" => Some(DiagnosticCode::E0402),
            "E0403" => Some(DiagnosticCode::E0403),
            "E0404" => Some(DiagnosticCode::E0404),
            "E0405" => Some(DiagnosticCode::E0405),
            "E0406" => Some(DiagnosticCode::E0406),
            "E0407" => Some(DiagnosticCode::E0407),
            "E0408" => Some(DiagnosticCode::E0408),
            "E0501" => Some(DiagnosticCode::E0501),
            "E0502" => Some(DiagnosticCode::E0502),
            "E0503" => Some(DiagnosticCode::E0503),
            "E0504" => Some(DiagnosticCode::E0504),
            "E0505" => Some(DiagnosticCode::E0505),
            "E0601" => Some(DiagnosticCode::E0601),
            "E0602" => Some(DiagnosticCode::E0602),
            "E0603" => Some(DiagnosticCode::E0603),
            "E0604" => Some(DiagnosticCode::E0604),
            "E0605" => Some(DiagnosticCode::E0605),
            "E0606" => Some(DiagnosticCode::E0606),
            "E0701" => Some(DiagnosticCode::E0701),
            "E0702" => Some(DiagnosticCode::E0702),
            "E0800" => Some(DiagnosticCode::E0800),
            "E0801" => Some(DiagnosticCode::E0801),
            "E0802" => Some(DiagnosticCode::E0802),
            "E0803" => Some(DiagnosticCode::E0803),
            "E0804" => Some(DiagnosticCode::E0804),
            "E0805" => Some(DiagnosticCode::E0805),
            "E0806" => Some(DiagnosticCode::E0806),
            "E0807" => Some(DiagnosticCode::E0807),
            "E0808" => Some(DiagnosticCode::E0808),
            "E0810" => Some(DiagnosticCode::E0810),
            "E0811" => Some(DiagnosticCode::E0811),
            "E0812" => Some(DiagnosticCode::E0812),
            "E0900" => Some(DiagnosticCode::E0900),
            "E0901" => Some(DiagnosticCode::E0901),
            "E0902" => Some(DiagnosticCode::E0902),
            "E0903" => Some(DiagnosticCode::E0903),
            "E0904" => Some(DiagnosticCode::E0904),
            "E0905" => Some(DiagnosticCode::E0905),
            "E0906" => Some(DiagnosticCode::E0906),
            "E0908" => Some(DiagnosticCode::E0908),
            "E0909" => Some(DiagnosticCode::E0909),
            "E0910" => Some(DiagnosticCode::E0910),
            "E0911" => Some(DiagnosticCode::E0911),
            "E0912" => Some(DiagnosticCode::E0912),
            "E0914" => Some(DiagnosticCode::E0914),
            "E0915" => Some(DiagnosticCode::E0915),
            "E0916" => Some(DiagnosticCode::E0916),
            "E0917" => Some(DiagnosticCode::E0917),
            "E0918" => Some(DiagnosticCode::E0918),
            "E0919" => Some(DiagnosticCode::E0919),
            "E0920" => Some(DiagnosticCode::E0920),
            "E0921" => Some(DiagnosticCode::E0921),
            "E0922" => Some(DiagnosticCode::E0922),
            "E0923" => Some(DiagnosticCode::E0923),
            "E0924" => Some(DiagnosticCode::E0924),
            "E0925" => Some(DiagnosticCode::E0925),
            "E0926" => Some(DiagnosticCode::E0926),
            "E0927" => Some(DiagnosticCode::E0927),
            "E0928" => Some(DiagnosticCode::E0928),
            "E0929" => Some(DiagnosticCode::E0929),
            "E0931" => Some(DiagnosticCode::E0931),
            "E0932" => Some(DiagnosticCode::E0932),
            "E0933" => Some(DiagnosticCode::E0933),
            "E0934" => Some(DiagnosticCode::E0934),
            "E0938" => Some(DiagnosticCode::E0938),
            "E0939" => Some(DiagnosticCode::E0939),
            "E0940" => Some(DiagnosticCode::E0940),
            "E0941" => Some(DiagnosticCode::E0941),
            "E0942" => Some(DiagnosticCode::E0942),
            "E0943" => Some(DiagnosticCode::E0943),
            "E0944" => Some(DiagnosticCode::E0944),
            "E0945" => Some(DiagnosticCode::E0945),
            "E0946" => Some(DiagnosticCode::E0946),
            "E0947" => Some(DiagnosticCode::E0947),
            "E0948" => Some(DiagnosticCode::E0948),
            "E0949" => Some(DiagnosticCode::E0949),
            "E0950" => Some(DiagnosticCode::E0950),
            "E0951" => Some(DiagnosticCode::E0951),
            "E0952" => Some(DiagnosticCode::E0952),
            "E0953" => Some(DiagnosticCode::E0953),
            "E0954" => Some(DiagnosticCode::E0954),
            "E0955" => Some(DiagnosticCode::E0955),
            "E0956" => Some(DiagnosticCode::E0956),
            "E0957" => Some(DiagnosticCode::E0957),
            "E0958" => Some(DiagnosticCode::E0958),
            "E0959" => Some(DiagnosticCode::E0959),
            "E0960" => Some(DiagnosticCode::E0960),
            "E0961" => Some(DiagnosticCode::E0961),
            "E0962" => Some(DiagnosticCode::E0962),
            "E0963" => Some(DiagnosticCode::E0963),
            "E0964" => Some(DiagnosticCode::E0964),
            "E0965" => Some(DiagnosticCode::E0965),
            "E0966" => Some(DiagnosticCode::E0966),
            "E0967" => Some(DiagnosticCode::E0967),
            "E0968" => Some(DiagnosticCode::E0968),
            "E0969" => Some(DiagnosticCode::E0969),
            "E0970" => Some(DiagnosticCode::E0970),
            "E100" => Some(DiagnosticCode::E100),
            "E101" => Some(DiagnosticCode::E101),
            "E102" => Some(DiagnosticCode::E102),
            "E150" => Some(DiagnosticCode::E150),
            "E151" => Some(DiagnosticCode::E151),
            "E152" => Some(DiagnosticCode::E152),
            "E160" => Some(DiagnosticCode::E160),
            "E161" => Some(DiagnosticCode::E161),
            "E162" => Some(DiagnosticCode::E162),
            "E163" => Some(DiagnosticCode::E163),
            "E164" => Some(DiagnosticCode::E164),
            "E165" => Some(DiagnosticCode::E165),
            "E166" => Some(DiagnosticCode::E166),
            "E170" => Some(DiagnosticCode::E170),
            "E171" => Some(DiagnosticCode::E171),
            "E172" => Some(DiagnosticCode::E172),
            "E173" => Some(DiagnosticCode::E173),
            "E174" => Some(DiagnosticCode::E174),
            "E175" => Some(DiagnosticCode::E175),
            "E176" => Some(DiagnosticCode::E176),
            "E177" => Some(DiagnosticCode::E177),
            "E178" => Some(DiagnosticCode::E178),
            "E179" => Some(DiagnosticCode::E179),
            "E180" => Some(DiagnosticCode::E180),
            "E181" => Some(DiagnosticCode::E181),
            "E182" => Some(DiagnosticCode::E182),
            "E183" => Some(DiagnosticCode::E183),
            "E184" => Some(DiagnosticCode::E184),
            "E185" => Some(DiagnosticCode::E185),
            "E186" => Some(DiagnosticCode::E186),
            "E187" => Some(DiagnosticCode::E187),
            "E188" => Some(DiagnosticCode::E188),
            "E189" => Some(DiagnosticCode::E189),
            "E190" => Some(DiagnosticCode::E190),
            "E191" => Some(DiagnosticCode::E191),
            "E192" => Some(DiagnosticCode::E192),
            "E193" => Some(DiagnosticCode::E193),
            "E194" => Some(DiagnosticCode::E194),
            "E195" => Some(DiagnosticCode::E195),
            "E196" => Some(DiagnosticCode::E196),
            "E197" => Some(DiagnosticCode::E197),
            "E198" => Some(DiagnosticCode::E198),
            "E199" => Some(DiagnosticCode::E199),
            "E200" => Some(DiagnosticCode::E200),
            "E201" => Some(DiagnosticCode::E201),
            "E202" => Some(DiagnosticCode::E202),
            "E203" => Some(DiagnosticCode::E203),
            "E204" => Some(DiagnosticCode::E204),
            "E205" => Some(DiagnosticCode::E205),
            "E300" => Some(DiagnosticCode::E300),
            "E301" => Some(DiagnosticCode::E301),
            "E302" => Some(DiagnosticCode::E302),
            "E305" => Some(DiagnosticCode::E305),
            "E306" => Some(DiagnosticCode::E306),
            "E400" => Some(DiagnosticCode::E400),
            "E401" => Some(DiagnosticCode::E401),
            "E402" => Some(DiagnosticCode::E402),
            "E403" => Some(DiagnosticCode::E403),
            "W001" => Some(DiagnosticCode::W001),
            "W002" => Some(DiagnosticCode::W002),
            "W003" => Some(DiagnosticCode::W003),
            "W0115" => Some(DiagnosticCode::W0115),
            "W0201" => Some(DiagnosticCode::W0201),
            "W0202" => Some(DiagnosticCode::W0202),
            "W0203" => Some(DiagnosticCode::W0203),
            "W0204" => Some(DiagnosticCode::W0204),
            "W0205" => Some(DiagnosticCode::W0205),
            "W0301" => Some(DiagnosticCode::W0301),
            "W0302" => Some(DiagnosticCode::W0302),
            "W0303" => Some(DiagnosticCode::W0303),
            "W0401" => Some(DiagnosticCode::W0401),
            "W0403" => Some(DiagnosticCode::W0403),
            "W0501" => Some(DiagnosticCode::W0501),
            "W0502" => Some(DiagnosticCode::W0502),
            "W0601" => Some(DiagnosticCode::W0601),
            "W0602" => Some(DiagnosticCode::W0602),
            "W0603" => Some(DiagnosticCode::W0603),
            "W0604" => Some(DiagnosticCode::W0604),
            "W0605" => Some(DiagnosticCode::W0605),
            "W0700" => Some(DiagnosticCode::W0700),
            "W0701" => Some(DiagnosticCode::W0701),
            "W0702" => Some(DiagnosticCode::W0702),
            "W0703" => Some(DiagnosticCode::W0703),
            "W0706" => Some(DiagnosticCode::W0706),
            "W0707" => Some(DiagnosticCode::W0707),
            "W0709" => Some(DiagnosticCode::W0709),
            "W0710" => Some(DiagnosticCode::W0710),
            "W0711" => Some(DiagnosticCode::W0711),
            "W0712" => Some(DiagnosticCode::W0712),
            "W0713" => Some(DiagnosticCode::W0713),
            "W0714" => Some(DiagnosticCode::W0714),
            "W0715" => Some(DiagnosticCode::W0715),
            "W0716" => Some(DiagnosticCode::W0716),
            "W0717" => Some(DiagnosticCode::W0717),
            "W0718" => Some(DiagnosticCode::W0718),
            "W0963" => Some(DiagnosticCode::W0963),
            "W100" => Some(DiagnosticCode::W100),
            "W101" => Some(DiagnosticCode::W101),
            "W102" => Some(DiagnosticCode::W102),
            "W110" => Some(DiagnosticCode::W110),
            "W111" => Some(DiagnosticCode::W111),
            "W112" => Some(DiagnosticCode::W112),
            "W113" => Some(DiagnosticCode::W113),
            "W114" => Some(DiagnosticCode::W114),
            "W201" => Some(DiagnosticCode::W201),
            "W202" => Some(DiagnosticCode::W202),
            "W203" => Some(DiagnosticCode::W203),
            "W204" => Some(DiagnosticCode::W204),
            "W205" => Some(DiagnosticCode::W205),
            _ => None,
        }
    }

    /// Every code string that `as_str` can produce must parse back to the SAME
    /// variant. This is what makes `from_code_str` trustworthy: it is checked
    /// against the real table rather than a copy of it, so a variant added to
    /// one and not the other fails here instead of silently mislabelling a
    /// user's error.
    #[cfg(test)]
    pub(crate) fn round_trips(self) -> bool {
        Self::from_code_str(self.as_str()) == Some(self)
    }

    /// Check if this is an error (vs warning).
    ///
    /// DERIVED, not tabulated: an `E`-prefixed code is an error, full stop —
    /// except the four codes whose ONLY construction sites build them as
    /// warnings (validator soft-errors and the ambiguity notices). The
    /// previous hand-maintained match list was the FUP-148 rot in its purest
    /// form: every new code silently defaulted to WARNING in the two channels
    /// that consult this table (SignalDiagnostic severity, miette severity),
    /// so E0948/E0949/E0953 printed as "error[...]" while the LSP and the
    /// signals layer called them warnings. It also listed E150/E151 as errors
    /// while their only construction site builds them as warnings — the table
    /// was wrong in BOTH directions.
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            DiagnosticCode::E0923
                | DiagnosticCode::E0924
                | DiagnosticCode::E150
                | DiagnosticCode::E151
        ) == false
            && self.as_str().starts_with('E')
    }

    /// Get this code as a miette-compatible code string (e.g., "spacetime::E102").
    pub fn as_miette_code(&self) -> String {
        format!("spacetime::{}", self.as_str())
    }

    /// Convert to miette severity.
    pub fn to_miette_severity(&self) -> miette::Severity {
        if self.is_error() {
            miette::Severity::Error
        } else {
            miette::Severity::Warning
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod code_round_trip_tests {
    use super::*;

    /// Every code string the `as_str` table can produce, read FROM THAT TABLE
    /// in this very file at test time.
    ///
    /// The first version of this test carried a hand-written 190-element list —
    /// which is precisely the hazard the `from_code_str` cutover deleted from
    /// `main.rs`, reintroduced in the test that was supposed to guard against
    /// it. A reviewer caught it: a variant added to the enum and to `as_str`
    /// but not to that list would pass, and be silently relabelled E0806 in
    /// production exactly as before.
    ///
    /// Reading the source removes the list, and with it the possibility of the
    /// list being wrong. `as_str` is an exhaustive match, so the compiler
    /// guarantees every variant appears here; this test then guarantees every
    /// one of them survives the round trip.
    fn codes_in_as_str_table() -> Vec<String> {
        let src = include_str!("codes.rs");
        let re_line = regex::Regex::new(r#"DiagnosticCode::([EW][0-9]+) => "([EW][0-9]+)""#)
            .expect("valid regex");
        let mut found: Vec<String> = re_line
            .captures_iter(src)
            .map(|c| {
                assert_eq!(
                    &c[1], &c[2],
                    "as_str maps a variant to a DIFFERENT code string — the table itself is wrong"
                );
                c[1].to_string()
            })
            .collect();
        found.sort();
        found.dedup();
        found
    }

    /// TOTALITY: every code `as_str` can emit parses back to the same variant.
    #[test]
    fn every_code_round_trips() {
        let codes = codes_in_as_str_table();
        assert!(
            codes.len() > 150,
            "only {} codes found in the as_str table — the scraper's pattern has \
             drifted from the source, so this test is no longer checking anything",
            codes.len()
        );

        let broken: Vec<&String> = codes
            .iter()
            .filter(|s| DiagnosticCode::from_code_str(s).map(|c| c.as_str()) != Some(s.as_str()))
            .collect();
        assert!(
            broken.is_empty(),
            "these codes do not survive as_str -> from_code_str: {broken:?}. \
             A code that cannot round-trip is RELABELLED when it crosses the \
             PipelineErrorInfo boundary, so the user sees a correct diagnostic \
             under a wrong name (FUP-148)."
        );
    }

    /// The fallback must be HONEST. Returning a real code for an unrecognized
    /// string is exactly the bug this replaced: `_ => E0806` turned every
    /// unmapped diagnostic into "unknown directive".
    #[test]
    fn unknown_code_is_none() {
        assert_eq!(DiagnosticCode::from_code_str("E9999"), None);
        assert_eq!(DiagnosticCode::from_code_str(""), None);
        assert_eq!(DiagnosticCode::from_code_str("nonsense"), None);
    }

    /// The regression that motivated the structural fix: E0951 is newer than
    /// the hand-maintained table in `main.rs`, and under that table it rendered
    /// as E0806. Now it round-trips like every other code, with no per-code
    /// registration step to forget.
    #[test]
    fn a_newly_added_code_needs_no_registration_step() {
        assert_eq!(
            DiagnosticCode::from_code_str("E0951"),
            Some(DiagnosticCode::E0951)
        );
    }
}
