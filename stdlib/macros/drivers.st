// stdlib/macros/drivers.st — the DRIVER REGISTRY (SIP-001 §2, PLAN-124 W3.1)
//
// A projection (`.visible`, `.scroll`, `.time`, …) reads a ref's relationship
// to its world and yields a DRIVER. The registry is DATA: each driver is one
// `%macro` whose `%registers driver(<name>)` clause is the entry, consumed via
// `MetaRegistry::entries_of("driver")` (W2.1). Adding a driver with an
// existing primitive family is THIS FILE ONLY — no Rust, no `%if` ladder.
//
// Entry fields (all scalar RegisterValues — the shapes the clause supports):
//
//   primitive       the binds primitive the @on resolution emits for this
//                   driver (the runtime vocabulary: intersection, scroll-driver,
//                   event-driver, mouse-driver, time-driver, loop-driver,
//                   load-driver). `planned:<name>` marks a driver whose
//                   primitive does not exist yet — registered now so the
//                   grammar and diagnostics know the word, bound when its
//                   primitive lands (W3.2+). This is the dispatch table:
//                   the member name is the key, the primitive is the value.
//   value_type      progress | transport | gate | event (SIP-001 §2:
//                   Progress DRIVES, Transport seeks, Gate MASKS, Event TRIGGERS)
//   seekable        can a render/score seek this driver's timeline? (scroll:
//                   no; time/loop/playback/clip: yes — render-safety data, W5)
//   reduced_motion  skip  — entrance choreography: jump to the final state
//                   hold  — ambient/repeating: do not auto-play at all
//                   allow — user-initiated motion (interaction/scroll-linked)
//   on_kinds        which ref kinds the projection applies to (elem, media,
//                   timeline, clip) — W3.2's E-DRIVER-KIND diagnostic reads
//                   this: `&intro.playback` on a template is a KIND error, not
//                   an unknown-driver error.
//   primitive_args  constants this ROW passes to a SHARED primitive, as
//                   `"name:value,name:value"`. When several rows name the same
//                   primitive, this is what tells them apart: the five event
//                   rows are one `event-driver` differing only by
//                   `trigger:`. An author-written param of the same name WINS
//                   (a row constant is a default, not an override).
//                   BUG-253: without this slot the rows were literally
//                   indistinguishable — every one emitted the primitive's
//                   `trigger` default, so `@on &.click:` listened for
//                   mouseenter and `@on &.submit:` registered no listener at
//                   all. Two rows compiling to identical logic is now a test
//                   failure (tests/driver_row_identity_test.rs).
//   primitive       the primitive this driver's progress comes from. THREE
//                   kinds of value, and the difference is deliberate:
//                     <name>        a real primitive; it produces the clock
//                     planned:<n>   RESERVED but unbuilt — E0948 at compile.
//                                   Loud by design: this is why `.clip` was
//                                   never silently wrong the way five event
//                                   drivers were (BUG-253)
//                     none          produces NO clock, positively. Its
//                                   progress is inherited from elsewhere
//                                   (`.clip` reads the window its parent
//                                   score assigned). Distinct from a MISSING
//                                   field, which is a malformed row
//   domain          W4/R1 (SIP-001 §Proposal/4). Which TIME DOMAIN a `@score`
//                   driven by this driver measures its placements in. A score
//                   never declares a duration of its own — it inherits the
//                   domain of whatever drives it:
//                     declared   the driver's own param IS the domain
//                                (`&.time(60s)` ⇒ `at 2s for 3s`)
//                     normalized 0..1, so placements are percentages
//                                (`&.scroll(cover)` ⇒ `for 30%`)
//                     inherited  the span the PARENT assigned (`&.clip`) or
//                                the media element's duration (`.playback`)
//                     derived    computed from the score's own content
//                                (`.steps` ⇒ one step per line)
//                   This is what makes `@score &.scroll(cover) { &x at 2s; }`
//                   an E-MEASURE-DOMAIN error with a did-you-mean, rather
//                   than a silently meaningless number (pressure test C2).
//   interrupt       W4/B8. Policy when a driver's gate closes mid-run:
//                   reverse | hold | settle | cut. Defaults are DERIVED from
//                   value_type (event/gate reverse — today's hover-away;
//                   transport holds, since a playhead should not rewind on
//                   interruption), so a row states this only to deviate.
//   param_map       renames an AUTHOR-supplied param to the primitive's own
//                   spelling (`"stagger:charStagger"`). Sibling of
//                   primitive_args: that one SUPPLIES a constant the author
//                   never writes, this one RENAMES what the author did write.
//
// Params are deliberately NOT duplicated here: they live in the @on head
// grammar (driver_expr's param list) and the primitive's own arg signature.
// The SIP's projection table documents the defaults; the registry entry says
// WHAT the driver is, the grammar says HOW it is called, the primitive says
// HOW it runs. Three owners, one fact each.
//
// The `%form { @driver-<name> }` on each macro is a NON-USER directive: the
// macro exists to carry the `%registers` clause (a clause attaches to a macro
// def; a formless macro would match anything at score 0). The same pattern as
// the W2.1 fixture — proven.

%macro driver-visible {
  %scope file
  %order 700
  %form { @driver-visible }
  %registers driver(visible) {
    // An ENTRANCE, not a sensor. `@on &.visible(300ms)` means "when enough of
    // me is on screen, play this over 300ms" — the contract the retired
    // `@on visible $name($duration:time, threshold: …)` macro had, and the one
    // the cutover rewrite still lowers to (`stdlib/migrations/entries/
    // 2026-07-26-on-cutover.st` maps `@on visible` onto
    // `@on &.load(duration:, delay:, threshold:, immediate:)`).
    //
    // It used to bind `intersection`, a SENSOR whose only progress export is
    // the intersection ratio. That had three consequences, all silent:
    //   - the author's duration reached nothing (intersection has no
    //     `duration` param) and vanished from the bundle entirely;
    //   - progress WAS the ratio, so an element only partly on screen sat
    //     permanently mid-animation (BUG-323), and one taller than the
    //     viewport could never finish;
    //   - re-entering the viewport re-published a low ratio, reverting a
    //     completed entrance (BUG-315).
    // `load-driver` is the same threshold gate driving a real rAF timeline,
    // so all three go away and `threshold:` keeps its meaning as the GATE.
    primitive: load-driver
    // `@on &.visible(300ms)` — the unnamed arg is the entrance's duration.
    positional_params: "duration"
    value_type: event
    seekable: false
    domain: normalized
    reduced_motion: skip
    on_kinds: elem
  }
}

%macro driver-hover {
  %scope file
  %order 700
  %form { @driver-hover }
  %registers driver(hover) {
    primitive: event-driver
    primitive_args: "trigger:mouseenter,untrigger:mouseleave"
    // `@on &.hover(200ms)` — how long the reaction takes.
    positional_params: "duration"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-click {
  %scope file
  %order 700
  %form { @driver-click }
  %registers driver(click) {
    primitive: event-driver
    // `@on &.click(200ms)` — how long the reaction takes.
    positional_params: "duration"
    primitive_args: "trigger:click,toggle:true"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-focus {
  %scope file
  %order 700
  %form { @driver-focus }
  %registers driver(focus) {
    primitive: event-driver
    // `@on &.focus(200ms)` — how long the reaction takes.
    positional_params: "duration"
    primitive_args: "trigger:focus"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

// BUG-329 cutover: `@on input`, `@on keydown` and the pointer events were live
// surfaces with NO registered driver, so the sigil form had nothing to resolve.
// They are added HERE, as registry DATA, rather than as a grammar exemption — a
// driver is a `%registers driver(<name>)` entry, which is exactly what E0948's
// hint tells an author who reaches for a missing one.
//
// (This comment named `@on enter` too, for a driver that was never registered.
// Corrected in BUG-375: a key is not a driver, it is a PARAM of a keyboard
// driver — `@on &.keydown("Enter")` — which keeps one row per DOM event
// instead of one per key.)
//
// `input` alone accounts for the admin panel, the host page, the inspector and
// the comments pill: retiring the bare head without these would have deleted
// those surfaces rather than migrated them.

%macro driver-input {
  %scope file
  %order 700
  %form { @driver-input }
  %registers driver(input) {
    primitive: event-driver
    primitive_args: "trigger:input"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-keydown {
  %scope file
  %order 700
  %form { @driver-keydown }
  %registers driver(keydown) {
    primitive: event-driver
    // BUG-375: `@on &.keydown(key: "Enter")` — the ONE key this binding
    // answers to. Positional so the common case reads `&.keydown("Enter")`.
    // Without it a keyboard binding fires on every keystroke: the qualifier
    // authors reached for (`&.keydown:Enter`) has no slot in `driver_expr`'s
    // grammar, so it was parsed and discarded in silence.
    positional_params: "key"
    primitive_args: "trigger:keydown"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
  }
}

%macro driver-mouseenter {
  %scope file
  %order 700
  %form { @driver-mouseenter }
  %registers driver(mouseenter) {
    primitive: event-driver
    primitive_args: "trigger:mouseenter"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-mouseleave {
  %scope file
  %order 700
  %form { @driver-mouseleave }
  %registers driver(mouseleave) {
    primitive: event-driver
    primitive_args: "trigger:mouseleave"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-mousemove {
  %scope file
  %order 700
  %form { @driver-mousemove }
  %registers driver(mousemove) {
    primitive: event-driver
    primitive_args: "trigger:mousemove"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-pointerdown {
  %scope file
  %order 700
  %form { @driver-pointerdown }
  %registers driver(pointerdown) {
    primitive: event-driver
    primitive_args: "trigger:pointerdown"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-pointerup {
  %scope file
  %order 700
  %form { @driver-pointerup }
  %registers driver(pointerup) {
    primitive: event-driver
    primitive_args: "trigger:pointerup"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-submit {
  %scope file
  %order 700
  %form { @driver-submit }
  %registers driver(submit) {
    primitive: event-driver
    // `@on &.submit(200ms)` — how long the reaction takes.
    positional_params: "duration"
    primitive_args: "trigger:submit"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
  }
}

%macro driver-key {
  %scope file
  %order 700
  %form { @driver-key }
  %registers driver(key) {
    primitive: event-driver
    // `@on &.key("Enter")` — the key this binding answers to (BUG-375).
    // `duration` is gone from the positional list: this driver's first
    // unnamed argument is overwhelmingly a key name, and a keyboard event has
    // no duration to speak of. Pass a duration by name if one is ever needed.
    positional_params: "key"
    primitive_args: "trigger:keydown"
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
  }
}

%macro driver-scroll {
  %scope file
  %order 700
  %form { @driver-scroll }
  %registers driver(scroll) {
    primitive: scroll-driver
    value_type: progress
    seekable: false
    domain: normalized
    reduced_motion: allow
    on_kinds: elem
  }
}

%macro driver-mouse {
  %scope file
  %order 700
  %form { @driver-mouse }
  %registers driver(mouse) {
    primitive: mouse-driver
    value_type: progress
    seekable: false
    domain: normalized
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-pointer {
  %scope file
  %order 700
  %form { @driver-pointer }
  %registers driver(pointer) {
    primitive: mouse-driver
    // A DELIBERATE alias: `.pointer` and `.mouse` are the same driver, spelled
    // two ways. Declared, because the identity gate
    // (tests/driver_row_identity_test.rs) treats two rows compiling to the same
    // logic as a bug — an intentional alias has to SAY so, which is what keeps
    // an accident from hiding behind the same shape.
    alias_of: mouse
    value_type: progress
    seekable: false
    domain: normalized
    reduced_motion: allow
    on_kinds: elem
    interactive: true
  }
}

%macro driver-time {
  %scope file
  %order 700
  %form { @driver-time }
  %registers driver(time) {
    primitive: time-driver
    // `@on &.time(300ms)` — the unnamed arg is how long the timeline runs.
    positional_params: "duration"
    value_type: transport
    seekable: true
    domain: declared
    reduced_motion: skip
    on_kinds: elem
  }
}

%macro driver-loop {
  %scope file
  %order 700
  %form { @driver-loop }
  %registers driver(loop) {
    primitive: loop-driver
    // `@on &.loop(2s)` — the unnamed arg is one cycle's length.
    positional_params: "duration"
    value_type: transport
    seekable: true
    domain: declared
    reduced_motion: hold
    on_kinds: elem
  }
}

%macro driver-load {
  %scope file
  %order 700
  %form { @driver-load }
  %registers driver(load) {
    primitive: load-driver
    // `@on &.load(300ms)` — the unnamed arg is the entrance's duration.
    positional_params: "duration"
    value_type: event
    seekable: false
    reduced_motion: skip
    on_kinds: elem
  }
}

// --- Planned primitives: the WORD is registry data today; the primitive ---
// --- lands with its wave. Binding one of these is a loud error, never a ---
// --- silent no-op (W3.2's resolution must say "planned, not yet bound"). ---

%macro driver-steps {
  %scope file
  %order 700
  %form { @driver-steps }
  %registers driver(steps) {
    primitive: steps-driver
    // A `.steps` deck advances on a DOM event by default — the SAME trigger
    // resolution the `@on` rail uses for `advance:` event-exprs, so an
    // author-written `advance: &.click` overrides this the way any row
    // constant is a default, not an override.
    primitive_args: "advance:click"
    value_type: transport
    seekable: true
    domain: derived
    reduced_motion: skip
    on_kinds: elem
  }
}

%macro driver-playback {
  %scope file
  %order 700
  %form { @driver-playback }
  %registers driver(playback) {
    primitive: media-driver
    value_type: transport
    seekable: true
    domain: inherited
    reduced_motion: hold
    on_kinds: media
  }
}

%macro driver-clip {
  %scope file
  %order 700
  %form { @driver-clip }
  // W4/t5: PROMOTED out of `planned:`. A clip has no primitive of its own —
  // `domain: inherited` means its progress IS the window its parent score
  // assigned (`__clip_<element>`), so the score lowering wires it directly and
  // there is nothing to start. `primitive: none` states that positively: not
  // "unimplemented", but "this driver produces no clock".
  %registers driver(clip) {
    primitive: none
    value_type: transport
    seekable: true
    domain: inherited
    reduced_motion: skip
    on_kinds: elem
  }
}

/// PLAN-126 (D26/D31): VALUE FACETS and SIGNAL-SUBJECT drivers.
/// A facet entry is either a DRIVER (primitive: the binds primitive the @on
/// resolution emits) or a VALUE FACET (`lowers_to` / `facet_primitive` /
/// `expr_lowering`): not a driver at all, but a typed READ on a subject —
/// `@on $reveal.done { true => … }` desugars to arms on the lowered signal.

/// `.done` — progress reached 1. lowers_to: a SIGNAL-expression template
/// (`$%subject` = the subject signal name) evaluated by derived-signal for
/// statement/arms positions; expr_lowering: the JS form for guard/CSS
/// expression positions (`%subject` = the bare name).
%macro facet-done {
  %scope file
  %order 700
  %form { @facet-done }
  %registers driver(done) {
    lowers_to: "($%subject >= 1)"
    expr_lowering: "(ST.resolve(__node, '%subject') >= 1)"
    value_type: bool
    seekable: false
    reduced_motion: allow
    on_kinds: signal
  }
}

/// `.prev` — the previous value (mount: the initial value). Statement/arms
/// positions go through the prev-signal primitive (derive is stateless —
/// prev needs memory); expression positions inline the ST.prev read.
%macro facet-prev {
  %scope file
  %order 700
  %form { @facet-prev }
  %registers driver(prev) {
    facet_primitive: prev-signal
    expr_lowering: "ST.prev(__node, '%subject')"
    value_type: value
    seekable: false
    reduced_motion: allow
    on_kinds: signal
  }
}

/// `.running` — 0 < progress < 1. PLANNED (D2): the word is known so
/// diagnostics are precise; per-frame flicker semantics need their own
/// measurement before the primitive lands.
%macro facet-running {
  %scope file
  %order 700
  %form { @facet-running }
  %registers driver(running) {
    primitive: planned:running-driver
    value_type: bool
    seekable: false
    reduced_motion: allow
    on_kinds: signal
  }
}

/// `.change` — a signal's value changed; the ONLY driver-facet on the signal
/// rail. Drives one-shot bodies (`@on $count.change(duration: 500ms) { … }`)
/// and statements (`@on $count.change: --pulse;`).
%macro driver-change {
  %scope file
  %order 700
  %form { @driver-change }
  %registers driver(change) {
    primitive: change-driver
    progress_export: progress
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: signal
  }
}

/// `.text-change` — an element's TEXT content changed (the retired
/// `@value-change`'s home). Its consequence is the two-slot body
/// (`:entering`/`:exiting`), declared as data via `body:`.
%macro driver-text-change {
  %scope file
  %order 700
  %form { @driver-text-change }
  %registers driver(text-change) {
    primitive: text-change-driver
    value_type: event
    seekable: false
    reduced_motion: allow
    on_kinds: elem
    body: entering-exiting
    param_map: "stagger:charStagger,from:staggerFrom"
  }
}
