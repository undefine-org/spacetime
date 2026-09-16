/// 3D Module Tests — @stage / @object / @light (three.js engine)
///
/// FIDELITY: these assertions exercise the vendored three.js engine, which needs
/// a real WebGL-capable browser. They therefore run on the `--cdp` paint rung and
/// REFUSE on the headless V8 `logic` backend (no WebGL / no real canvas) — the
/// fidelity ladder's "refuse, don't fake" contract. Run with:
///
///   cargo run --features cdp -- test stdlib/3d/tests/ --cdp
///
/// STATUS: these do not GATE yet — blocked on BUG-099 (the CDP test-runner
/// harness fails to inject in this environment) AND BUG-051 (nested directives
/// inside a @fixture body don't execute under the V8 backend). Until the CDP
/// harness lands, the 3D surface is gated by the Rust emit tests in
/// src/compiler.rs (three_*: engine injection, material/geometry mapping, lights,
/// particles, post, orbit, lean-law) and verified end-to-end by the four
/// demos/3d showcases (rendered + screenshotted in a real browser).
///
/// What they prove (structural truths that hold WITHOUT asserting pixels):
///   - importing stdlib/3d + using @stage ships the vendored three.js engine
///   - @stage builds a live scene/camera/renderer handle on the canvas
///   - @object adds a mesh; @light adds a light; counts add up
///   - the camera is a real PerspectiveCamera with the requested params

@import "stdlib/testing/test"
@import "stdlib/3d"

// =============================================================================
// Vendored engine availability (demand-driven emission)
// =============================================================================

@test "three.js global is injected when @stage is used" {
  @fixture {
    <canvas class="t-avail" data-stage-avail></canvas>
    [data-stage-avail] { @stage(camZ: 5) {} }
  }
  @assert (typeof THREE === 'object' && typeof THREE.REVISION === 'string')
    "vendored three.js engine should be present on the page"
}

// =============================================================================
// @stage builds the engine handle (scene + camera + renderer)
// =============================================================================

@test "@stage builds a scene/camera/renderer handle on the canvas" {
  @fixture {
    <canvas class="t-stage" data-stage></canvas>
    [data-stage] { @stage(camZ: 5) {} }
  }
  @assert (!!document.querySelector('[data-stage]')._stThree)
    "@stage stores the shared three handle on the canvas element"
  @assert (document.querySelector('[data-stage]')._stThree.scene.isScene === true)
    "the handle carries a real THREE.Scene"
  @assert (document.querySelector('[data-stage]')._stThree.camera.isPerspectiveCamera === true)
    "the handle carries a real PerspectiveCamera"
}

// =============================================================================
// @object adds a mesh to the scene
// =============================================================================

@test "@object adds a mesh to the stage's scene" {
  @fixture {
    <canvas class="t-obj" data-obj></canvas>
    [data-obj] {
      @stage(camZ: 5) {
        @object(shape: "icosahedron", size: 1)
      }
    }
  }
  @assert (document.querySelector('[data-obj]')._stObjects.length === 1)
    "@object registers one mesh on the canvas"
  @assert (document.querySelector('[data-obj]')._stObject.isMesh === true)
    "the registered object is a real THREE.Mesh"
}

// =============================================================================
// @light adds a light to the scene
// =============================================================================

@test "@light adds a directional light to the stage's scene" {
  @fixture {
    <canvas class="t-light" data-light></canvas>
    [data-light] {
      @stage(camZ: 5) {
        @light(type: "directional", intensity: 2)
      }
    }
  }
  @assert (document.querySelector('[data-light]')._stThree.scene.children.some(function(c){ return c.isDirectionalLight; }))
    "@light(type: directional) adds a DirectionalLight to the scene"
}
