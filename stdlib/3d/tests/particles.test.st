/// 3D Module Tests — @particles (GPU point-cloud morph field)
///
/// FIDELITY: paint rung (`--cdp`); refuses on headless V8 (needs three.js + WebGL).
///
/// Proves the particle field builds a real Points cloud of the requested count and
/// exposes the morph hook (`_stParticleMorph`) that @scroll-3d drives.

@import "stdlib/testing/test"
@import "stdlib/3d"

@test "@particles builds a Points cloud of the requested count" {
  @fixture {
    <canvas class="t-pts" data-pts></canvas>
    [data-pts] {
      @stage(camZ: 6) {
        @particles(count: 5000, size: 0.02)
      }
    }
  }
  @let pts = document.querySelector('[data-pts]')._stThree.scene.children.find(function(c){ return c.isPoints; })
  @assert (!!$pts)
    "@particles adds a THREE.Points object to the scene"
  @assert ($pts.geometry.attributes.position.count === 5000)
    "the cloud has exactly `count` particles"
}

@test "@particles exposes the morph hook for @scroll-3d" {
  @fixture {
    <canvas class="t-morph" data-morph></canvas>
    [data-morph] {
      @stage(camZ: 6) {
        @particles(count: 2000, morph: 0)
      }
    }
  }
  @assert (typeof document.querySelector('[data-morph]')._stParticleMorph === 'number')
    "@particles publishes _stParticleMorph for scroll-driven morphing"
}

@test "@particles uses additive blending for the glow build-up" {
  @fixture {
    <canvas class="t-add" data-add></canvas>
    [data-add] {
      @stage(camZ: 6) {
        @particles(count: 1000)
      }
    }
  }
  @let pts = document.querySelector('[data-add]')._stThree.scene.children.find(function(c){ return c.isPoints; })
  @assert ($pts.material.blending === THREE.AdditiveBlending)
    "particle material blends additively so overlaps glow"
}
