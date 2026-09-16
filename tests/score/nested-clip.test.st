/// Behavior gate: a nested `@score &.clip` moves reactively under the window
/// its PARENT score published on an ANCESTOR (BUG-266).
///
/// `.shot`'s `@score &.clip` declares `domain: inherited`: it does not own a
/// clock — it receives the window `.film` assigned it (`__clip_shot`, 6s of
/// the 12s reel). The nested score's children (`__clip_head`) must therefore
/// MOVE as the parent's window advances.
///
/// This is the film pole of demos/score-launch §3, mounted alone (its own
/// fixture, so the `__clip_*` signal names do not collide with the website
/// pole's — see two-pole.test.st for both poles in one mount).
///
/// THE DISCRIMINATOR: at 1500ms of the 12s reel, the shot is a quarter through
/// its inherited window and `head` (half the inherited window) is exactly
/// mid-progress — a correct page reads opacity ≈ 0.5. A `head` of 0 means the
/// nested score never reacted to the window its parent published on an
/// ancestor — the old bare `ST.watch` on the nested score's own node, which
/// resolved the initial value but never fired (the FUP-094 read/watch
/// mismatch this fix closes).
///
/// NB `--cdp` (real Chromium): timing-dependent by design; `--headless`
/// refuses.

@import "stdlib/testing/test"

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --story {
  &head for 50%;
  &tail for 50%;
}

.film {
  @score &.loop(12s) as &film {
    &shot at 0s for 6s;
  }
}

.shot {
  @score &.clip { --story; }
}

.head, .tail { opacity: 0; }
.head { @on &.clip: --clip-rise; }
.tail { @on &.clip: --clip-rise; }

@test "nested-clip: the nested &.clip reacts to the window its parent published" needs timing {
  @fixture {
    <div class="film">
      <div class="shot">
        <div class="head">head</div>
        <div class="tail">tail</div>
      </div>
    </div>
  }

  @wait 1500ms

  @eval (window.__nc1 = parseFloat(getComputedStyle(document.querySelector('.film .shot .head')).opacity))
  @eval (window.__nc2 = parseFloat(getComputedStyle(document.querySelector('.film .shot .tail')).opacity))

  @assert (window.__nc1 > 0.2 && window.__nc1 < 0.9)
    "BUG-266: at 1500ms the nested clip's head (half the inherited 6s shot window) is mid-progress, so opacity must be mid-window (≈0.5). 0 means the nested `@score &.clip` never reacted to the window its parent published on an ancestor — the old bare ST.watch, whose join never fires"
  @assert (window.__nc2 === 0)
    "at 1500ms the nested clip's tail (second half of the inherited window) has not started — nonzero means the nested score's two clips are not sequenced inside the inherited window"
}
