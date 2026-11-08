/// Behavior gate: a stage shows the score's frame after ONE long advance.
///
/// A virtual clock's `advance(ms)` ticks every frame in between synchronously
/// (a render's still at 13 s is ~800 ticks in one call). A signal's changes
/// reach their dependents in a microtask batch, several hops deep (score ->
/// clip window -> `@on` body), so every tick's writes used to land after the
/// last tick: the DOM was right, but the stage had folded its channels inside
/// the ticks and showed the PREVIOUS advance's values (a render's first still
/// at its rest pose, every later one a seek behind). Now each virtual tick
/// settles its signal batch before the next (`ST._settleSignals`), and the
/// stage folds after the score writes.
///
/// FIDELITY: paint rung (`--cdp`): three.js + WebGL.

@import "stdlib/testing/test"
@import "stdlib/3d"

.reel {
  @score &.time(6s) as &film { &stage at 0s for 6s; }
}
.stage {
  width: 200px; height: 200px;
  @on &.clip { &orb { posX: 0 -> 6; } }
}
canvas.field {
  width: 200px; height: 200px;
  @stage(camZ: 6) {
    @object &orb (shape: "sphere", size: 1)
  }
}

@test "one long advance shows the score's frame on the stage" needs timing {
  @fixture { <div class="reel"><div class="stage"><canvas class="field"></canvas></div></div> }

  // virtual time from the first frame, as `spacetime render` runs it
  @clock install
  @wait 400ms
  @clock advance 3000
  @eval (window.__x = JSON.parse(document.querySelector('[data-st-world] [data-st-name="orb"], [data-st-world] i[data-3d="Mesh"]').getAttribute('data-props')).position[0])
  @eval (window.__mx = document.querySelector('canvas.field')._stThree.scene.children.find(function (c) { return c.isMesh; }).position.x)

  @assert (Math.abs(window.__x - 3) < 0.1)
    "after one advance to 3 s the part's props must be the score's frame (posX ≈ 3); 0 means the stage showed the frame before the advance"
  @assert (Math.abs(window.__mx - 3) < 0.1)
    "the three.js mesh must be at the score's frame too"
}

// A stepped `clip` channel is a model's animation clip. Seeking into the
// middle of a clip must play it at its AGE in the score — measured from the
// stop it started at, in the window's ms — not restart it (PLAN-155).
.reel2 { @score &.time(6s) as &reel2 { &stage2 at 0s for 6s; } }
.stage2 {
  width: 200px; height: 200px;
  @on &.clip { &bot { clip: "Idle" -> "Walk" at 50% --steps(1); } }
}
canvas.field2 {
  width: 200px; height: 200px;
  @stage(camZ: 6) { @object &bot (shape: "box", size: 1) }
}

@test "a seek lands mid-clip at the clip's age" needs timing {
  @fixture { <div class="reel2"><div class="stage2"><canvas class="field2"></canvas></div></div> }

  @wait 400ms
  @eval (ST._transports.reel2.pause(), ST._transports.reel2.seek(4500))
  @wait 300ms
  @eval (window.__bot = document.querySelector('canvas.field2')._stStageObjects['&bot']._stPart)
  @eval (window.__age = window.__bot.channels.clipAge)
  @assert (window.__bot.props.animation === 'Walk')
    "at 4.5 s of a 6 s window the clip is the one that began at 50%"
  @assert (Math.abs(window.__age - 1500) < 40)
    "its age is (75% - 50%) x 6000 ms = 1500 ms, not 0 (a seek must not restart it)"
}
