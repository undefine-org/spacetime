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
