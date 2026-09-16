/// Unit Tests: @data derive … : @match — cond-mode value-producing match (PLAN-077 W2)
///
/// End-to-end: the new form parses through the live grammar
/// (stdlib/enum/capture-types/cond-arms.st), compiles via %macro
/// data-derive-match + %primitive derive-match (stdlib/enum/derive.st), and the
/// published `{type, ...}` union value flows through the SAME
/// SpacetimeLocal + local:<name>:updated channel every derive uses (so
/// ST.watch / watchTypedUnion would consume it unchanged).
///
/// Pinned behaviors:
///   1. guard → variant resolution (truthy guard picks its constructor)
///   2. first-match-wins (an earlier truthy guard beats a later one)
///   3. `_` catch-all fallback (no truthy guard → last arm)
///   4. reactivity (flipping a dep signal recomputes + republishes)
///   5. payload constructor publishes the bare-$signal arg under the signal's
///      own name (`Failed($errorText)` → {type:'Failed', errorText: …}) so a
///      `@state(when: $fs is Failed { $errorText })` destructure reads it

@import "stdlib/testing/test"

@type BadgeState { Blue | Orange | Turquoise | Green }
@type FetchState { Idle | Failed(string) }

@data inline $isOrange bool : false;
@data inline $isTurquoise bool : false;
@data inline $errorText string : "";

@data derive $badge BadgeState : @match {
  ($isOrange)    => Orange;
  ($isTurquoise) => Turquoise;
  _              => Green;
}

@data derive $fs FetchState : @match {
  $errorText != "" => Failed($errorText);
  _                 => Idle;
}

// =============================================================================
// 1. Catch-all fallback: no truthy guard → the `_` arm's variant
// =============================================================================

@test "derive-match: catch-all yields the fallback variant initially" {
  @fixture {
    <div class="dm-1">x</div>
  }

  @eval (window.__dm1_badge = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__dm1_badge === 'Green')
    "no truthy guard: $badge must fall through to the `_ => Green` arm"

  @eval (window.__dm1_fs = window.SpacetimeLocal.fs && window.SpacetimeLocal.fs.type)
  @assert (window.__dm1_fs === 'Idle')
    "empty $errorText: $fs must fall through to the `_ => Idle` arm"
}

// =============================================================================
// 2. Guard resolution + first-match-wins + reactivity
// =============================================================================

@test "derive-match: first truthy guard wins and recomputes on dep flips" {
  @fixture {
    <div class="dm-2">x</div>
  }

  // Flip the LATER guard: Turquoise wins (Orange still false).
  @eval (window.SpacetimeLocal.isTurquoise = true)
  @eval (window.__dm2_t = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__dm2_t === 'Turquoise')
    "isTurquoise=true: $badge must resolve the Turquoise arm"

  // Flip the EARLIER guard too: Orange is listed first — first-match-wins.
  @eval (window.SpacetimeLocal.isOrange = true)
  @eval (window.__dm2_both = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__dm2_both === 'Orange')
    "isOrange=true AND isTurquoise=true: the EARLIER Orange arm must win (first-match-wins)"

  // Unflip Orange: falls back to the still-truthy Turquoise guard.
  @eval (window.SpacetimeLocal.isOrange = false)
  @eval (window.__dm2_back = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__dm2_back === 'Turquoise')
    "isOrange back to false: $badge must recompute to Turquoise"

  // Unflip both: catch-all again.
  @eval (window.SpacetimeLocal.isTurquoise = false)
  @eval (window.__dm2_green = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__dm2_green === 'Green')
    "all guards false: $badge must recompute to the catch-all Green"
}

// =============================================================================
// 3. Payload constructor: bare-$signal arg publishes under the signal's name
// =============================================================================

@test "derive-match: payload constructor publishes the signal-named field" {
  @fixture {
    <div class="dm-3">x</div>
  }

  @eval (window.SpacetimeLocal.errorText = 'boom-077')
  @eval (window.__dm3_type = window.SpacetimeLocal.fs && window.SpacetimeLocal.fs.type)
  @eval (window.__dm3_payload = window.SpacetimeLocal.fs && window.SpacetimeLocal.fs.errorText)
  @assert (window.__dm3_type === 'Failed')
    "errorText set: $fs must resolve the Failed($errorText) arm"
  @assert (window.__dm3_payload === 'boom-077')
    "Failed($errorText) must publish the payload under the signal's own name (fs.errorText)"

  // Clearing the guard recomputes to the catch-all (payload gone with it).
  @eval (window.SpacetimeLocal.errorText = '')
  @eval (window.__dm3_idle = window.SpacetimeLocal.fs && window.SpacetimeLocal.fs.type)
  @assert (window.__dm3_idle === 'Idle')
    "errorText cleared: $fs must recompute to the `_ => Idle` arm"
}

// =============================================================================
// 4. Payload-only dependency (SWARM GATE 2 finding): a signal that appears in
//    NO guard but in a payload arg must STILL trigger recompute (the dep regex
//    scans combined guard + payload-arg text)
// =============================================================================

@type ProfileState { Empty | Ready(string) }

@data inline $avatarUrl string : "";

@data derive $profile ProfileState : @match {
  ($isOrange) => Ready($avatarUrl);
  _           => Empty;
}

@test "derive-match: payload-only dep recomputes the payload field" {
  @fixture {
    <div class="dm-4">x</div>
  }

  // Enter the Ready arm; payload is the (empty) avatarUrl.
  @eval (window.SpacetimeLocal.isOrange = true)
  @eval (window.__dm4_t0 = window.SpacetimeLocal.profile && window.SpacetimeLocal.profile.type)
  @eval (window.__dm4_p0 = window.SpacetimeLocal.profile && window.SpacetimeLocal.profile.avatarUrl)
  @assert (window.__dm4_t0 === 'Ready')
    "isOrange=true: $profile must resolve the Ready($avatarUrl) arm"
  @assert (window.__dm4_p0 === '')
    "initial payload: avatarUrl is ''"

  // Flip a signal referenced ONLY by the payload arg (no guard mentions it):
  // the payload field must recompute in place.
  @eval (window.SpacetimeLocal.avatarUrl = 'a-077.png')
  @eval (window.__dm4_p1 = window.SpacetimeLocal.profile && window.SpacetimeLocal.profile.avatarUrl)
  @assert (window.__dm4_p1 === 'a-077.png')
    "avatarUrl flip (payload-only dep): profile.avatarUrl must republish the new value"

  @eval (window.SpacetimeLocal.isOrange = false)
}

// =============================================================================
// 5. Throwing-guard containment (SWARM GATE 2 finding): a guard whose eval
//    throws counts as FALSY so the `_` catch-all still fires — the runtime
//    totality matches the compile-time E0931 invariant
// =============================================================================

@data derive $resilient BadgeState : @match {
  ($missingSignal.nested) => Orange;
  _                       => Green;
}

@test "derive-match: throwing guard falls through to the catch-all" {
  @fixture {
    <div class="dm-5">x</div>
  }

  @eval (window.__dm5_t = window.SpacetimeLocal.resilient && window.SpacetimeLocal.resilient.type)
  @assert (window.__dm5_t === 'Green')
    "guard throws (undefined.nested): must NOT abort the scan — the `_ => Green` arm still fires"
}

// =============================================================================
// 6. ST.watchTypedUnion consumes the GLOBAL channel (SWARM GATE 2 P2 fix):
//    `@state(when: $x is V)`'s runtime half must observe a derive-match
//    union published via SpacetimeLocal + local:<name>:updated
// =============================================================================

@test "derive-match: watchTypedUnion observes the global publish channel" {
  @fixture {
    <div class="dm-6">x</div>
  }

  // Wire the @state(when:) runtime half to the fixture element; the global
  // value ('Green' from the catch-all) applies IMMEDIATELY at mount.
  @eval (eval("window.__dm6_el = document.querySelector('.dm-6'); ST.watchTypedUnion(window.__dm6_el, 'badge', 'Orange', [])"))
  @eval (window.__dm6_a0 = window.__dm6_el.getAttribute('data-st-badge-type'))
  @assert (window.__dm6_a0 === 'Green')
    "initial global value must reach watchTypedUnion even though no element scope owns 'badge'"

  // Flip the dep: the republished union flows through the GLOBAL channel into
  // the element-local attribute.
  @eval (window.SpacetimeLocal.isOrange = true)
  @eval (window.__dm6_a1 = window.__dm6_el.getAttribute('data-st-badge-type'))
  @assert (window.__dm6_a1 === 'Orange')
    "isOrange flip: data-st-badge-type must track the globally-published union"

  @eval (window.SpacetimeLocal.isOrange = false)
  @eval (window.__dm6_a2 = window.__dm6_el.getAttribute('data-st-badge-type'))
  @assert (window.__dm6_a2 === 'Green')
    "isOrange unflip: attribute must follow the recompute back to Green"
}

