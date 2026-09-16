/// Behavior gate (W5/P10): a `.steps` score is a DISCRETE clock — the deck
/// advances one step per advance event, and each step's clip shows when the
/// playhead reaches it. (SIP-001 `.steps`: "transport (ordinal)", domain
/// `derived` — count of score lines.)
///
/// This is the missing behavioral half for a clock that shipped as a
/// `planned:` registry row (E0948). Before this wave `.steps` was an honest
/// compile-time error; the risk now is the opposite — it compiles but nothing
/// moves. So the assertions are NEGATIVE-FIRST, exactly like clip-consumer:
/// a clip whose window has not opened must be STILL, and the only thing that
/// may move it is a step event.
///
/// NB `--cdp` (real Chromium): "did the deck move when I fired a click" is an
/// event-timing claim; `--headless` refuses these by design.

@import "stdlib/testing/test"

@form motion --deck-rise { opacity: 0 -> 1; }

// A 3-step deck. Each clip owns one third of the deck (`for 30%`), so at
// step 0 only `a` is on its way, `b`/`c` are untouched; by the last step all
// three have completed.
.deck {
  @score &.steps(advance: &.click) {
    &a for 30%;
    &b for 30%;
    &c for 30%;
  }
}

.a { opacity: 0; @on &.clip: --deck-rise; }
.b { opacity: 0; @on &.clip: --deck-rise; }
.c { opacity: 0; @on &.clip: --deck-rise; }

// =============================================================================
// 1. THE NEGATIVE — at step 0, no clip has moved. If `.steps` compiled to a
//    frozen or auto-playing clip, this fails for the right reason.
// =============================================================================

@test "steps: at step 0 the deck is still" needs timing {
  @fixture {
    <div class="deck">
      <div class="a">a</div><div class="b">b</div><div class="c">c</div>
    </div>
  }

  @wait 200ms

  @eval (window.__sd0a = parseFloat(getComputedStyle(document.querySelector('.a')).opacity))
  @eval (window.__sd0c = parseFloat(getComputedStyle(document.querySelector('.c')).opacity))
  @assert (window.__sd0a === 0)
    "at step 0 no clip has moved — a nonzero `a` means the deck is auto-advancing or the wrong signal is driving it"
  @assert (window.__sd0c === 0)
    "the LAST clip's window has not opened at step 0"
}

// =============================================================================
// 2. One advance fires — `a` moves, `c` is STILL. This is the load-bearing
//    discrimination: the deck advanced by exactly one step, not to the end.
// =============================================================================

@test "steps: one advance moves the first clip and not the last" needs timing {
  @fixture {
    <div class="deck">
      <div class="a">a</div><div class="b">b</div><div class="c">c</div>
    </div>
  }

  @wait 200ms
  @eval (document.querySelector('.deck').dispatchEvent(new MouseEvent('click', { bubbles: true })))
  @wait 60ms

  @eval (window.__sd1a = parseFloat(getComputedStyle(document.querySelector('.a')).opacity))
  @eval (window.__sd1c = parseFloat(getComputedStyle(document.querySelector('.c')).opacity))
  @assert (window.__sd1a > 0)
    "after ONE advance the first clip must have moved — 0 means the advance event is not wired"
  @assert (window.__sd1c === 0)
    "after ONE advance the last clip (window 0.6..0.9) must be untouched — nonzero means the deck skipped to the end"
}

// =============================================================================
// 3. Two more advances reach the last step — the deck ran to its end, and the
//    LAST clip completes. This is the "the deck actually finishes" half.
// =============================================================================

@test "steps: advancing to the last step completes the last clip" needs timing {
  @fixture {
    <div class="deck">
      <div class="a">a</div><div class="b">b</div><div class="c">c</div>
    </div>
  }

  @wait 200ms
  @eval (document.querySelector('.deck').dispatchEvent(new MouseEvent('click', { bubbles: true })))
  @eval (document.querySelector('.deck').dispatchEvent(new MouseEvent('click', { bubbles: true })))
  @eval (document.querySelector('.deck').dispatchEvent(new MouseEvent('click', { bubbles: true })))
  @wait 60ms

  @eval (window.__sd2c = parseFloat(getComputedStyle(document.querySelector('.c')).opacity))
  @assert (window.__sd2c > 0.9)
    "at the last step the third clip is done — a low value means the deck did not reach its end"
}

// =============================================================================
// BARE UNION (P2): `advance: click | keydown` (no `&`) names DOM events
// directly. Before the fix it fell through the `&`-gated resolver, so the
// steps-driver split on `,` saw ONE dead `addEventListener("click | keydown")`
// — neither event advanced the deck. Now the `|` is normalized to `,`, so both
// events are real listeners.
// =============================================================================

.duck1 {
  @score &.steps(advance: click | keydown) {
    &d1 for 30%;
    &e1 for 30%;
    &f1 for 30%;
  }
}

.d1 { opacity: 0; @on &.clip: --deck-rise; }
.e1 { opacity: 0; @on &.clip: --deck-rise; }
.f1 { opacity: 0; @on &.clip: --deck-rise; }

@test "steps: a bare `click | keydown` union advances on a CLICK" needs timing {
  @fixture {
    <div class="duck1">
      <div class="d1">d</div><div class="e1">e</div><div class="f1">f</div>
    </div>
  }

  @wait 200ms
  @eval (document.querySelector('.duck1').dispatchEvent(new MouseEvent('click', { bubbles: true })))
  @wait 60ms

  @eval (window.__sbu1d = parseFloat(getComputedStyle(document.querySelector('.d1')).opacity))
  @eval (window.__sbu1f = parseFloat(getComputedStyle(document.querySelector('.f1')).opacity))
  @assert (window.__sbu1d > 0)
    "a bare `click | keydown` advance must move the first clip on a CLICK — 0 means the click listener is not wired"
  @assert (window.__sbu1f === 0)
    "after one click the LAST clip must still be untouched — nonzero means the deck skipped to the end"
}

.duck2 {
  @score &.steps(advance: click | keydown) {
    &d2 for 30%;
    &e2 for 30%;
    &f2 for 30%;
  }
}

.d2 { opacity: 0; @on &.clip: --deck-rise; }
.e2 { opacity: 0; @on &.clip: --deck-rise; }
.f2 { opacity: 0; @on &.clip: --deck-rise; }

@test "steps: a bare `click | keydown` union advances on a KEYDOWN" needs timing {
  @fixture {
    <div class="duck2">
      <div class="d2">d</div><div class="e2">e</div><div class="f2">f</div>
    </div>
  }

  @wait 200ms
  @eval (document.querySelector('.duck2').dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'ArrowRight' })))
  @wait 60ms

  @eval (window.__sbu2d = parseFloat(getComputedStyle(document.querySelector('.d2')).opacity))
  @eval (window.__sbu2f = parseFloat(getComputedStyle(document.querySelector('.f2')).opacity))
  @assert (window.__sbu2d > 0)
    "a bare `click | keydown` advance must move the first clip on a KEYDOWN — 0 means the keydown listener is not wired"
  @assert (window.__sbu2f === 0)
    "after one keydown the LAST clip must still be untouched — nonzero means the deck skipped to the end"
}
