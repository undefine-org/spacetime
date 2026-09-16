/// `@form <kind> --name { … }` — the one declaration surface for a named,
/// reusable chunk of language (SIP-001c, PLAN-121 W2.3 → BUG-245 fix).
///
///   @form style  --card-surface { background: $brand.panel; }
///   @form value  --double($x)   { result: $x * 2; }
///   @form motion --rise($distance = 24px) { opacity: 0 -> 1; }
///   @form easing --ease-out-expo { cubic-bezier(0.16, 1, 0.3, 1) }
///   @form score  --intro { title: a -> b; }
///   @form markup --badge { <span class="badge">New</span> }
///
/// THE KIND IS THE MACRO — the exact shape `stdlib/macros/data-kind.st` uses for
/// `@data <kind>`: each kind is its own `%macro` whose `%form` carries the
/// kind-word in second position, so parse disambiguates on a TOKEN the author
/// wrote, never on a guess about the body. The kind-word is ALWAYS REQUIRED and
/// ALWAYS LEADS the name: `@form motion --rise`, never `@form --rise motion`
/// (parentheses hold what the form owns; the word before the name is the
/// language's, and the language's words come first — SIP-001c's position rule).
///
/// # Why the kind-word is a one-alternative UNION, not a bare literal
///
/// `data-kind.st` writes its kind-word as a bare literal (`@data inline …`).
/// Here it is a single-alternative union capture — `$kind:("motion")` — and the
/// difference is the whole point of BUG-245:
///
///   * a bare literal that does not match yields a `LiteralMismatch`, which the
///     matcher deliberately does NOT report (literals also discriminate sibling
///     overloads and CSS at-rule queries, so reporting them would turn valid CSS
///     into compile errors — `src/syntax/events/match_sink.rs`). Measured: with
///     bare literals, `@form mootion --x { … }` compiles GREEN and is silently
///     dropped — the exact silent-drop class BUG-229/BUG-243 spent three waves
///     eliminating.
///   * a union capture that does not match yields a `CaptureTypeMismatch`, which
///     IS reported (E0946). A misspelt kind is now a hard compile error naming
///     the `$kind` slot and the word the form expected.
///
/// It is the same second-token discrimination — the matcher's strongest path —
/// expressed as a TYPED capture so the failure is author-visible. The captured
/// `$kind` is also available to `%binds`/`%registers` (see `@form value` below,
/// where nothing else distinguishes it from `@form style`).
///
/// # Body capture types: which kinds are TIGHT and which are LENIENT today
///
/// The MARKER is the discriminator, by design. The body capture only validates
/// the body against the declared kind, so a lenient body type costs nothing in
/// selection — it only means a malformed body of that kind is not yet caught.
///
///   | kind   | body capture | tight? | note                                    |
///   |--------|--------------|--------|-----------------------------------------|
///   | style  | properties   | ~      | a `;`-list of `name: value` declarations |
///   | value  | properties   | ~      | `result: <expr>;` is a declaration; a    |
///   |        |              |        | dedicated `result_body` capture that     |
///   |        |              |        | REQUIRES `result:` is FUP work           |
///   | motion | keyframes    | ~      | accepts any nonempty body as raw text    |
///   |        |              |        | (`form_compiler.rs`) — permissive        |
///   | easing | expr         | ✗      | a raw curve expression; nothing checks   |
///   |        |              |        | it names a curve function                |
///   | score  | score_entry+ | ✓      | real score lines (`&a -> &b for 30%`),   |
///   |        |              |        | spliceable into a `@score` body (W4/R2)  |
///   | markup | html_block   | ✗      | accepts any body as HTML-ish text;     |
///   |        |              |        | well-formedness is not checked         |
///
/// NO body capture currently rejects a mismatched body (measured:
/// `@form markup --badge { color: red; }` compiles green). The MARKER alone
/// discriminates today; per-kind body validation (SIP-001c §4's E-FORM-BODY)
/// is additive FUP work: tighten a capture type here, add a negative test.
/// No macro is selected BY its body, so no selection changes when that lands.
///
/// # `%order 700`
///
/// The band the other SIP-001 / late-dispatch macros use (`@effect`, `@handle`,
/// `@on`, `@portal`, `@when` are all 700), and what the W2.3 spike used. Forms
/// declare; they do not race any emit ordering, so the band is free of the
/// 248-250 data/host sequencing constraints.
///
/// # `%binds { noop(…) }`
///
/// W2.3 scope is the SURFACE: that each kind parses, carries its sigil, and
/// registers. Statement-position splice EXPANSION (`--name;` splicing a
/// style form's declarations into the scope, with call-site param
/// substitution) landed with BUG-298 (`expand_style_form_splices`,
/// parser/mod.rs); kind discipline there is E0959/E0960. Motion forms resolve
/// in `@on`'s form slot (drivers.rs), score fragments splice into `@score`
/// bodies (score.rs), easings resolve by name at the runtime boundary.
/// `noop` keeps the declaration observable as a match without inventing emit
/// behaviour a later wave has to undo.
///
/// # `%registers form($name, kind: "<kind>")`
///
/// The consumption half. Each declaration registers into category `form` keyed
/// by the captured `$name` (the `%registers binding($name, …)` shape in
/// data-kind.st — a captured key resolves per invocation), carrying its kind as
/// a static field. The statement-position splice arm reads this registry to ask
/// "is `--card-surface` a declared form?" — an unknown name is a hard error
/// there, which is only possible because the registry is authoritative here.


/// `@form style --name { … }` — a chunk of DECLARATIONS. The `@mixin` role:
/// spliced into a scope, it contributes its properties verbatim.
%macro form-style {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("style") $name:dashed_ident($params:param_list?) { $body:properties }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "style") {
    kind: "style"
    params: $params
    body: $body
  }
}


/// `@form value --name($x) { result: … ; }` — a pure FUNCTION. The `@function`
/// role: called in value position, it evaluates to its `result:`.
///
/// Shares `properties` with `style` — `result: <expr>;` IS a declaration — so
/// nothing but the kind-word tells the two apart. That is the design, not a
/// limitation: the author states which one they meant.
%macro form-value {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("value") $name:dashed_ident($params:param_list?) { $body:properties }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "value") {
    kind: "value"
    params: $params
    body: $body
  }
}


/// `@form motion --name($d = 24px) { a: 0 -> 1; }` — a KEYFRAME chunk. The
/// parameterized replacement for a motion library's 40 near-identical blocks:
/// one form, N call sites (`@on &.visible --rise(distance: 12px)`).
%macro form-motion {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("motion") $name:dashed_ident($params:param_list?) { $body:keyframes }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "motion") {
    kind: "motion"
    params: $params
    body: $body
  }
}


/// `@form camera --crane { position: 0 9.5 4.5 -> 0 2.1 9.5; fov: 38; }` — a
/// named CAMERA MOVE (PLAN-150 W3). A camera move is MOTION, so it is consumed
/// exactly where motion is: `@on &.clip --crane;`. Its body is keyframes over a
/// reserved axis vocabulary (`position` / `target` / `fov` / `roll`); the rig
/// is the element the `@on` sits on — a `@stage` canvas (real camera) or any
/// `preserve-3d` element (lowered to one inverse matrix on a rig child).
%macro form-camera {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("camera") $name:dashed_ident($params:param_list?) { $body:keyframes }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "camera") {
    kind: "camera"
    params: $params
    body: $body
  }
}


/// `@form easing --ease-out-expo { cubic-bezier(0.16, 1, 0.3, 1) }` — a named
/// CURVE. Replaces `&ease-out-expo`: an easing has no identity, so it never
/// belonged to the identity sigil (SIP-001c).
%macro form-easing {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("easing") $name:dashed_ident($params:param_list?) { $body:expr }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "easing") {
    kind: "easing"
    params: $params
    body: $body
  }
}


/// `@form post --cinema { bloom: 0.45; grain: 0.035; vignette: 0.55; }`
/// (PLAN-150 W12) — a named POST-processing look: the initial `@post { … }`
/// values as a reusable form, applied with `@post --cinema;`. Body is the same
/// `key: value` property list `@post` takes.
%macro form-post {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("post") $name:dashed_ident($params:param_list?) { $body:properties }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "post") {
    kind: "post"
    params: $params
    body: $body
  }
}


/// `@form score --intro { … }` — a named SCORE fragment: a sequence over one
/// shared playhead, spliceable into a `@score` body.
%macro form-score {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("score") $name:dashed_ident($params:param_list?) { $entries:score_entry+ }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "score") {
    kind: "score"
    params: $params
    body: $entries
  }
}


/// `@form markup --badge { <span …>…</span> }` — a named MARKUP chunk.
/// NOTE: `html_block` is NOT a tight capture — it accepts any body as HTML-ish
/// text (measured: `{ color: red; }` compiles green), so this kind, like the
/// others, is discriminated by its marker alone today.
%macro form-markup {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("markup") $name:dashed_ident($params:param_list?) { $body:html_block }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "markup") {
    kind: "markup"
    params: $params
    body: $body
  }
}


/// `@form transition --crossfade($d = 400ms) { &from { … } &to { … } }`
/// (PLAN-150 W8) — a named TRANSITION placed on a score `->`: two `@on`-style
/// keyframe blocks, one for the OUTGOING clip (`&from`) and one for the INCOMING
/// clip (`&to`), applied over the overlap of the two windows the arrow joins.
/// The `&from`/`&to` region names are the transition's two parameters; the body
/// is captured verbatim (each region's inner text is a keyframe block reified
/// when the transition is placed).
// `transition_body` capture lives in stdlib/capture-types/transition-body.st.

%macro form-transition {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("transition") $name:dashed_ident($params:param_list?) { $body:transition_body }
  }

  %binds {
    noop(&self, decl: $name)
  }

  %registers form($name, kind: "transition") {
    kind: "transition"
    params: $params
    body: $body
  }
}


/// `@form shot --card($t) { :markup { <div>…</div> } :motion { opacity: 0 -> 1; } }`
/// (PLAN-150 W9) — a form whose body carries MARKUP and MOTION: a component with
/// a timeline. Placed on a score, it instantiates its markup at that point and
/// binds its motion to that window; outside a score its markup slot is an
/// ordinary `@template` call. The two slots are captured verbatim (shot_body,
/// stdlib/capture-types/shot-body.st) and reified at placement.
%macro form-shot {
  %scope file | selector
  %order 700

  %form {
    @form $kind:("shot") $name:dashed_ident($params:param_list?) { $body:shot_body }
  }

  // A shot IS a template with a timeline. Sub-field bind access (`$body.markup`)
  // walks the shot_body Named record (PLAN-077 W6), so register a template
  // factory whose body is the markup slot and whose animations are the motion
  // slot — the golden's "the markup slot IS a template; :motion is the same body
  // an @on &.clip takes". No new primitive: register-template already takes an
  // optional `animations:`.
  %binds {
    register-template(name: $name, body: $body.markup, params: $params, animations: $body.motion) -> { $factory }
  }

  %registers form($name, kind: "shot") {
    kind: "shot"
    params: $params
    body: $body
  }
}
