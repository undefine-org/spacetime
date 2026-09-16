/// @state — generalized state→CSS reflection (PLAN-077 W6)
///
/// ONE reactive writer, ONE attribute model, three declaration forms:
///
///   @state(when: "loading")              { … }   // string/boolean gate
///   @state(when: $ws is Connected)       { … }   // union variant
///   @state(when: $ws is Connected { $send, $received }) { … }  // + destructure
///
/// The string form's condition names a SIGNAL (`"loading"` → `$loading`) and
/// keeps the LEGACY `ST.bindState` gate semantics exactly: any truthy value
/// reflects the declaration's NAME, falsy clears it (GATE 6 P2: a string-VALUE
/// passthrough was designed in early drafts but is incoherent — the emitted
/// CSS gate keys on the declaration name, so reflecting a differing value
/// would produce dead CSS; value-keyed styling is the UNION form's job).
/// The falsy clear is OWNERSHIP-guarded: multiple gates on one element share
/// the `data-st-state` carrier, and a gate may only clear its OWN state
/// (legacy bindState semantics; GATE 6 P1).
///
/// THE ORPHAN FIX (FUP-074 #2): the subscription is `ST.watchScoped` (host +
/// ancestor scopes + the global `local:<sig>:updated` channel) — the old
/// `ST.bindState` watched element-locally only, so a GLOBAL signal (every
/// `@data fetch` `$loading`, every `@data derive`) never fired the reflection
/// and `@state(when: "str")` had NO reactive writer at all.
///
/// Union signals (`{type: 'V', ...payload}` — from @socket, `@data derive
/// … : @match`, or a W4 `<-` construction) reflect to BOTH attributes:
/// `data-st-state` (the unified mutually-exclusive carrier) and
/// `data-st-<sig>-type` (the legacy namespaced carrier the pattern CSS and
/// existing consumers key on), with payload fields extracted into element
/// scope (ST.set — the watchTypedUnion destructure contract).
///
/// The instrumentation the retired `%primitive state` carried is PRESERVED
/// (SWARM GATE 6 criterion c): __stStateHistory/__stStateEntries/
/// __stStateExits/__stTransitionCount/__stStateDurations/__stTransitionTimes,
/// `state.enter`/`state.exit` CustomEvents, and __stDebugHook registration —
/// stdlib/testing/assertions/state.st reads every one of them.
///
/// THE WRITER LIVES IN THE RUNTIME (GATE 7 P1): the reflection logic is
/// `ST.bindState` in public/runtime/st.js — ONE implementation shared by this
/// primitive AND the metasystem `%states { x when $sig }` clause (whose Rust
/// emitter has always emitted `ST.bindState(el, cond, name)`). This
/// primitive's %emit js is a thin delegating call; stdlib/enum/state.st
/// previously inlined a second copy of the writer, which is exactly the
/// parallel-implementation drift the PLAN-077 Acceptance grep targets.

/// The `@state(when: …)` pattern value as ONE structured capture: `$x is V`
/// or `$x is V { $b, … }`. A custom type is REQUIRED — the inline form
/// (`$signal:ident is $variant:ident { $bindings:ident* }`) collapses: the
/// events parser treats the brace group as the directive BODY and the whole
/// when-value lands as one opaque `signal: Expr` (observed W3/W6). Reuses
/// W1's variant_binding for the destructure list.
%capture_type state_when {
    $signal:ident "is" $variant:ident ( "{" $bindings:variant_binding+ "}" )?
}

%macro state {
  %form {
    @state(when: $condition:string) {
      $styles:properties
    }
  }

  %binds {
    state-reflect(&self, signal: $condition, value: $condition, styles: $styles)
  }
}

/// Pattern-match state (with payload destructure): @state(when: $ws is Connected { $send, $received }) { … }
///
/// BOTH variant forms bind state-reflect (the ONE reactive writer): the
/// destructure's fields land in element scope via the primitive's payload
/// extraction (the watchTypedUnion contract), and the variant reflects to
/// data-st-state + data-st-<sig>-type.
%macro state-match {

  %form {
    @state(when: $when:state_when) {
      $styles:properties
    }
  }

  %binds {
    state-reflect(&self, signal: $when.signal, value: $when.variant, styles: $styles)
  }
}

/// State Reflect Primitive (PLAN-077 W6) — the ONE reactive writer for
/// data-st-state / data-st-<sig>-type. See the module header for the model.
///
/// @param &el    - the element to reflect state onto
/// @param signal - the signal to watch (bare name, no `$`)
/// @param value  - the state VALUE this declaration gates on (the string
///   condition, or the variant name) — drives the emitted `[attr=value]` CSS
/// @param styles - the properties to apply while the state is active
%primitive state-reflect(
  &el,
  signal: string = "",
  value: string = "",
  styles: properties
) {
  %emit css {
    %self[data-st-state=%value] {
      %styles
    }
  }

  %emit js {
    (() => {
      const host = %&el;
      const sig = %signal;
      // This declaration's OWN state name (the CSS gate value): the string
      // form's condition, the variant form's variant.
      const gate = %value;
      if (!host || !sig) return;
      // THE CANONICAL WRITER: ST.bindState (public/runtime/st.js) carries the
      // full state-reflect semantics — watchScoped orphan fix, initial
      // ST.resolve reflect, ownership-guarded falsy clear, union destructure,
      // both carriers, and the retired state() instrumentation. ONE writer
      // shared with the %states clause path (GATE 7 P1: this primitive
      // previously inlined a second, drifting copy of the same logic).
      if (typeof ST !== 'undefined' && ST.bindState) ST.bindState(host, sig, gate);
    })();
  }
}
