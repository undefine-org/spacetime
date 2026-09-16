// stdlib/macros/on.st — `@on <driver> --form;` (PLAN-124 W3.2)
//
// The driver-projection head. ONE directive whose first slot is a DRIVER
// EXPRESSION — `&.visible`, `&card.scroll`, `&.time(3s)` — resolved through
// the driver registry (stdlib/macros/drivers.st), and whose second slot is a
// MOTION FORM application resolved through the form registry (form.st).
//
// This file holds the NEW head. The OLD dispatcher (on-event.st's
// `on-event-dispatcher`, `@on hover spin(1s) { … }`) stays until the W3.4
// cutover rewrites its usages via migration capsules — the two are sibling
// forms of one directive, discriminated by the first token (`&` vs ident).
//
// The `%binds` names ONE pseudo-primitive, `drive`, which src/pipeline/
// drivers.rs expands between Evaluate and Resolve into the entry's real
// primitive + apply-animations. The registry entry is the dispatch table;
// this macro never learns which drivers exist.

/// `&card.visible` — an elem-ref (subject optional: `&.visible` is self)
/// acquiring temporal sense through a registry projection. Params use the
/// call-site shape (`&.visible(threshold: 0.5)`, `&.scroll(start: 0.2)`) —
/// the same call_arg surface as form applications.
%capture_type driver_expr { ( "&" $subject:ident? | "$" $ssubject:ident ) "." $member:ident ( "(" $driver_params:call_arg* ")" )? }

/// `--rise` — a motion form applied at the point of use. `dashed_ident`
/// enforces the sigil (BUG-242: `--rise` lexes as ONE ident).
%capture_type form_application { $form:dashed_ident ( "(" $args:call_arg* ")" )? }

/// One call-site argument: `280ms`, `distance: 12px`, `"grid"`. Positional or
/// `name: value`. (The RETIRED `params` type was a lenient-Expr trap — D9 in
/// the implementation report; this is its replacement.)
%capture_type call_arg { ( $name:ident ":" )? $value:expr ","? }

/// `@on &.visible --rise;` — the statement shape: driver × form, no body.
/// The body shape (`@on <driver> { … }`, inline choreography and arms) is
/// W3.3; its fence failing until then is honest, not a gap to paper over.
/// A body-less consequence (D28/D30): a motion form application OR a signal
/// mutation — `@on &.click: $open <- !$open;`. The colon introduces the value;
/// braces are choreography.
/// A mutation WITHOUT the trailing `;` — in statement position the form's
/// own bare `;` is the commitment point (the `mutation` type's optional
/// `";"?` would eat it and fail the form).
%capture_type on_mutation { "$" $target:ident "<-" $expr:expr }
%capture_type on_consequence { ( $form:form_application ) | ( $mut:on_mutation ) }

%macro on-driver-form {
  %scope file | selector
  %order 710
  // NB the trailing `;` is a BARE literal — a quoted ";" is stored as the
  // three-character literal `";"` and never matches (data-kind.st:361 is the
  // working precedent). The BUG-241 spike never surfaced this because its
  // form was never consulted.
  // The `:` is OPTIONAL during the PLAN-126 window (the W3.4 capsule emitted
  // the space form); W5 makes it mandatory and re-migrates. The `as` clause
  // names the driver's progress publication (D24: timelines are signals) and
  // binds BEFORE the consequence (D30 clause order A).
  %form { @on $driver:driver_expr $as:on_as? : $consequence:on_consequence ; }
  %binds { drive(&self, driver: $driver, form: $consequence, as: $as) }
}

/// The naming clause (D24/D30): `as $name` binds the driver's progress
/// publication into scope as a progress-TYPED signal. A capture TYPE because
/// %form heads are flat — no groups, no optional literals (the `(` would be
/// read as a params list; quoted literals keep their quotes and never match).
%capture_type on_as { "as" $name:binding }

/// ERROR SIBLING (BUG-261), the same shape as the D28 space form below: an
/// `as` clause with `&reveal` or bare `reveal` used to vanish. `on_as` is
/// OPTIONAL in every `@on` head, so a wrong spelling failed the capture and
/// the whole clause disappeared — no diagnostic, because an optional capture
/// that does not match is not an error. The driver then published under its
/// counter fallback (`__drive_visible_1`), so the author's name was gone.
///

/// One invalid capture union, and one error primitive, are deliberate: both
/// spellings violate the same `$`-only publication rule. `event_name` is the
/// existing bare-identifier capture — unlike `ident`, it never strips `$` — so
/// this fallback cannot capture valid `$reveal` forms.
%capture_type on_as_invalid { "as" ( $name:element | $name:event_name ) }

/// ERROR SIBLING (BUG-261) for the colon form. Matches every non-binding
/// spelling so it can be REFUSED rather than silently dropped.
%macro on-driver-form-as-invalid {
  %diagnostic


  %scope file | selector
  %order 712
  %form { @on $driver:driver_expr $as:on_as_invalid : $consequence:on_consequence ; }
  %binds { drive-as-error(&self, driver: $driver, as: $as) }
}

/// ERROR SIBLING (BUG-261) for the body form.
%macro on-driver-body-as-invalid {
  %diagnostic


  %scope file | selector
  %order 713
  %form { @on $driver:driver_expr $as:on_as_invalid { $body:on_motion_body } }
  %binds { drive-as-error(&self, driver: $driver, as: $as) }
}

/// ERROR SIBLING (D28): the pre-colon space form matches, then refuses
/// LOUDLY — a body-less @on without `:` is never a silent drop. The
/// trailing-`;` commitment point still names extra input first
/// (`--a --b;` → ExtraInlineTokens, the W3 review P2 surface).
// PLAN-150 W3-arc: `@on <driver> --form;` (no colon) is the film surface's
// natural spelling (`@on &.clip --crane;`). It now LOWERS — identically to the
// colon form — rather than erroring: a `form_application` is a valid
// `on_consequence`, so this binds the same `drive` primitive. The `:` stays
// valid too (both spellings work); `{ }` is choreography. This supersedes the
// D28/W5 mandatory-colon decision for the bare-form case, because the golden
// film contract (docs/language/film.st.md) uses the colon-free spelling and the
// two forms are unambiguous (a trailing `;` after a form application).
%macro on-driver-form-bare {
  %scope file | selector
  %order 711
  %form { @on $driver:driver_expr $as:on_as? $consequence:on_consequence ; }
  %binds { drive(&self, driver: $driver, form: $consequence, as: $as) }
}

// --- Signal arms (W3.3a): `@on $signal { pattern => --form(args); … }` ------
// A pattern is a signal VALUE (`true`, `"grid"`, a bare word), a two-ended
// transition (`a -> b`), or the wildcard `_`. Arms discriminate on `->`: a
// bare pattern means "arrived here from anywhere" (the 90% case — N states
// never force N² arms); `a -> b` is opt-in. Grammar proven in the W1 spikes
// and promoted verbatim (plus call_args, D9).

%capture_type on_leaf { ( $lit:string ) | ( $wild:ident ) }
%capture_type on_transition { $from:on_leaf "->" $to:on_leaf }
/// A binding pattern (D31-iii): `$old -> $new` BINDS the pre/post values for
/// the guard. `$` in pattern position INTRODUCES a binding — a documented
/// positional extension (everywhere else `$x` reads).
%capture_type on_bind_transition { $from:binding "->" $to:binding }
%capture_type on_pat { ( $bind_transition:on_bind_transition ) | ( $transition:on_transition ) | ( $leaf:on_leaf ) }
/// An arm with an optional MATCH-TIME guard (D31-iii): `where <expr>` filters
/// after the pattern test; expr stops at `=>` (the ExprExtractor's FAT_ARROW
/// boundary). Consequence args still bake at compile time (dynamic args =
/// FUP-156).
%capture_type on_arm { $pat:on_pat ( "where" $guard:expr )? "=>" $form:form_application ";" }

/// An arms head: a bare signal (`$count`) or a FACET PATH (`$reveal.done` —
/// the facet is a value-typed read on the signal, lowered by the pipeline;
/// D26). No `as` clause: publication is a driver concern, arms consume values.
%capture_type signal_path { $base:binding ( "." $facet:ident )? }

/// `@on $open { true => --slide-open(280ms); false => --slide-shut(200ms); }`
/// Two-state arms are a GATE (reversible); fired arms are an EVENT (latch) —
/// the difference is the ARM SHAPE, not a declaration (SIP-001 §2).
%macro on-signal-arms {
  %scope file | selector
  %order 705
  %form { @on $signal:signal_path { $arms:on_arm+ } }
  %binds { drive-arms(&self, signal: $signal, arms: $arms) }
}

// --- Driver body (W3.3b): `@on <driver> { … }` -------------------------------
// Inline choreography: splices of declared motion forms interleaved with
// keyframe lines, settings (`stagger: 70ms;` — the apply-animations builder
// already intercepts those names), and no-arrow lines (static properties
// while the driver runs). A splice starts with `--`, a line with an ident —
// the alternation discriminates on the first token.

%capture_type on_splice { $form:form_application ";" }
/// A body line: `stagger: 70ms;` · `translate-x: 0 -> 4px;` ·
/// `border-left-color: $brand.accent;`. `balanced` keeps the whole value run
/// (the keyframes.st precedent) — `raw_until_semicolon` fragments it
/// (`70ms` → "7"), a dead grammar this wave measured and replaced.
%capture_type motion_line { $prop:ident ":" $value:balanced(';') ";"? }
/// A scoped block: `.orb-1 { translate-x: parallax(15px); }` — nested
/// keyframes, inlined like keyframes.st's own scope alternation (a named
/// sub-capture wrapper starved the body token flow — measured: body: []).
/// A text-change slot (showcase §7): `:entering { … }` / `:exiting { … }` —
/// the two-slot consequence a `body: entering-exiting` registry entry
/// declares. The colon leads (pseudo-class shape); the slot name is data to
/// the pipeline, the body is ordinary motion lines.
%capture_type on_text_slot { ":" $slot:ident "{" $slot_body:motion_line* "}" }
/// A bare signal call — `$ping();` — a legal change-rail body: ST.runMutations
/// dispatches a bare `$name(args)` statement (signal.st:558 registers the
/// callable by name; st.js runMutations' signalCallMatch fires it).
/// `call_arg*` preserves the arg run verbatim (any count, nested braces), so
/// the reconstructed statement survives runMutations' splitArgs. Tried AFTER
/// `mutation`, so a `$dst <- $src` write stays classified as a mutation, never
/// misread as a call (both start `$`).
%capture_type on_signal_call { "$" $signal:ident "(" $args:call_arg* ")" ";"? }

/// A mutator call on a MEMBER — `$rows.insert(7);` — the collection-mutator
/// shape `%capture_type signal_call` declares (capture-types/on-actions.st) and
/// st.js `runMutations` already dispatches (its `$coll.method(args)` branch
/// reaches ST._collections). `call_arg*` for the same reason as on_signal_call:
/// the arg run survives splitArgs verbatim, nested braces and all.
///
/// Tried BEFORE `on_signal_call` and AFTER `mutation`: after `mutation` so a
/// write stays a write, before the bare form because both open on `$` and the
/// bare rule would otherwise commit on the ident and choke at the `.`.
%capture_type on_method_call { "$" $signal:ident "." $method:ident "(" $args:call_arg* ")" ";"? }

/// The body alternatives, in priority order. `$bad:property_mutation_invalid`
/// sits LAST (BUG-346): every legitimate shape gets first refusal, so it only
/// ever claims `<prop> <- value` — a line nothing else can match.
///
/// It is here rather than in a sibling `%macro` because `on_motion_body` is a
/// `( … )*` repetition and therefore matches EMPTY: a sibling whose `%form`
/// differs only in the body capture ties with `on-driver-body` on every score,
/// and the winner is then arbitrary — observed flipping between `drive`,
/// `drive-arms` and `drive-as-error` across identical runs of the same file.
/// An alternative INSIDE the repetition has no such ambiguity.
%capture_type on_motion_body { ( $slot:on_text_slot | $splice:on_splice | $mut:mutation | $call:on_method_call | $stmt:on_signal_call | $sel:selector "{" $scope:on_motion_body "}" | "&" "{" $scope:on_motion_body "}" | $line:motion_line | $bad:property_mutation_invalid )* }

%macro on-driver-body {
  %scope file | selector
  %order 708
  %form { @on $driver:driver_expr $as:on_as? { $body:on_motion_body } }
  %binds { drive-body(&self, driver: $driver, body: $body, as: $as) }
}

