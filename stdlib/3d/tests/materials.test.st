/// 3D Module Tests — material families (@object material params)
///
/// FIDELITY: paint rung (`--cdp`); refuses on headless V8 (needs three.js + WebGL).
///
/// Proves the @object material switch maps each `material:` to the right three.js
/// material class and threads the physical params (glass transmission, iridescence,
/// flat shading) through to the live material — the surface contract the showcases
/// depend on.

@import "stdlib/testing/test"
@import "stdlib/3d"

@test "material: glass produces a transmissive MeshPhysicalMaterial" {
  @fixture {
    <canvas class="t-glass" data-glass></canvas>
    [data-glass] {
      @stage(camZ: 5) {
        @object(shape: "icosahedron", material: "glass", transmission: 1, ior: 1.5)
      }
    }
  }
  @let mat = document.querySelector('[data-glass]')._stObject.material
  @assert ($mat.isMeshPhysicalMaterial === true)
    "glass maps to MeshPhysicalMaterial"
  @assert ($mat.transmission === 1)
    "transmission param threads through to the material"
}

@test "material: physical with iridescence threads iridescence params" {
  @fixture {
    <canvas class="t-irid" data-irid></canvas>
    [data-irid] {
      @stage(camZ: 5) {
        @object(shape: "icosahedron", material: "physical", iridescence: 1, iridescenceIOR: 1.5)
      }
    }
  }
  @let mat = document.querySelector('[data-irid]')._stObject.material
  @assert ($mat.iridescence === 1)
    "iridescence param threads through"
  @assert ($mat.iridescenceIOR === 1.5)
    "iridescenceIOR param threads through"
}

@test "material: standard produces a MeshStandardMaterial" {
  @fixture {
    <canvas class="t-std" data-std></canvas>
    [data-std] {
      @stage(camZ: 5) {
        @object(shape: "box", material: "standard", roughness: 0.4, metalness: 0.6)
      }
    }
  }
  @let mat = document.querySelector('[data-std]')._stObject.material
  @assert ($mat.isMeshStandardMaterial === true)
    "standard maps to MeshStandardMaterial"
  @assert ($mat.roughness === 0.4 && $mat.metalness === 0.6)
    "roughness + metalness thread through"
}

@test "flatShading: true sets flat shading on the material" {
  @fixture {
    <canvas class="t-flat" data-flat></canvas>
    [data-flat] {
      @stage(camZ: 5) {
        @object(shape: "icosahedron", material: "standard", flatShading: true)
      }
    }
  }
  @assert (document.querySelector('[data-flat]')._stObject.material.flatShading === true)
    "flatShading param threads through to the material"
}

@test "geometry shape maps to the requested three.js geometry" {
  @fixture {
    <canvas class="t-geo" data-geo></canvas>
    [data-geo] {
      @stage(camZ: 5) {
        @object(shape: "torusknot", size: 1)
      }
    }
  }
  @assert (document.querySelector('[data-geo]')._stObject.geometry.type === 'TorusKnotGeometry')
    "shape: torusknot builds a TorusKnotGeometry"
}
