/// Behavior gate: PLAN-150 W4 — @score/@on drives a @stage object property.
/// The "score drives 3D" of docs/language/film.st.md §5.
///
/// A named stage object (`@object &orb (...)`) is addressable from an
/// `@on &.clip { &orb { scale: 1 -> 2; } }` block. The keyframe engine that
/// drives DOM also drives the 3D mesh: the object registers a channel proxy,
/// the @on engine writes the keyframed value to it, and the object's frame loop
/// applies the channel to the REAL three.js mesh. This asserts the BEHAVIOUR —
/// the mesh's actual `scale.x` moved across the window (BUG-252), read off the
/// live scene graph, not the emitted string.
///
/// FIDELITY: paint rung (`--cdp`); refuses on headless V8 (needs three.js +
/// WebGL + a real clock).

@import "stdlib/testing/test"
@import "stdlib/3d"

// The score lives on the PARENT (.reel); the STAGE div consumes the clip window
// with `@on &.clip` and hosts the canvas as a child. `@on` bound directly on a
// <canvas> does not receive a score driver, so the consumer is the div and the
// stage-object resolver descends into the hosted canvas to find `&orb`. The 3D
// mesh is driven by the same keyframe engine that drives DOM.
.reel {
  @score &.time(6s) as &film { &stage at 0s for 6s; }
}
.stage {
  width: 200px; height: 200px;
  @on &.clip { &orb { scale: 1 -> 2.5; } }
}
canvas.field {
  width: 200px; height: 200px;
  @stage(camZ: 6) {
    @object &orb (shape: "sphere", size: 1)
  }
}

@test "a named stage object animates on the score clip" needs timing {
  @fixture { <div class="reel"><div class="stage"><canvas class="field"></canvas></div></div> }

  // The stage handle is set asynchronously on module init — wait for it.
  @wait 400ms
  @eval (window.__cv = document.querySelector('canvas.field'))
  @eval (window.__mesh = window.__cv._stThree.scene.children.find(function (c) { return c.isMesh; }))
  @eval (window.__hasProxy = !!(window.__cv._stStageObjects && window.__cv._stStageObjects['&orb']))

  @assert (!!window.__mesh)
    "@object must add a real mesh to the scene"
  @assert (window.__hasProxy === true)
    "a named object must register a channel proxy under its `&name` so @on can address it"

  @eval (window.__s0 = window.__mesh.scale.x)
  @wait 3200ms
  @eval (window.__s1 = window.__cv._stThree.scene.children.find(function (c) { return c.isMesh; }).scale.x)

    "DEBUG channels: the proxy must have received a scale write"
  @assert (window.__s1 > window.__s0 + 0.3)
    "the mesh's real scale.x must GROW across the clip window as `scale: 1 -> 2.5` drives it — a frozen scale means the keyframed channel never reached the 3D object"
}
