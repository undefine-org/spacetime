/// gh-14 — RENDERED-SHAPE GATE (REQUIRES `--cdp`, WebGL/paint rung).
///
/// The headless V8 `logic` backend has no WebGL and no real canvas, so a rendered
/// 3D shape/material is NOT observable headless. Only a real browser can prove
/// that what the compiler emitted becomes the geometry you asked for.
///
/// HISTORY, because this file's polarity flipped and the reason matters:
///
/// The issue reported that the DOCUMENTED spelling `@object torusknot(size: 1.2)`
/// compiled clean and silently rendered the DEFAULT icosahedron. This file was
/// originally written to assert that bareword spelling renders a TorusKnot — a
/// RED gate that would go GREEN when the documented form was honoured.
///
/// The resolution went the other way, and correctly: the bareword form was never
/// in `@object`'s `%form` at all. The DOCS described a grammar the macro did not
/// implement. So the fix diagnoses the bareword (E0946, see check-contract.sh)
/// and the doc examples were rewritten to the real named form.
///
/// A gate asserting the bareword renders would therefore now be asserting that a
/// deliberately-refused spelling compiles — it would pin the bug, not the fix.
/// What still NEEDS proving in a browser is the other half, which no headless
/// gate can reach: that the canonical named form actually produces the requested
/// geometry and material, rather than compiling clean and rendering the default.
/// That is the claim the original issue really cared about.
///
/// Run:  spacetime test tests/repro/gh-14-object-bareword-shape/object-bareword.cdp.test.st --cdp
/// BLOCKED ON BUG-324 (2026-08-06) — do not "fix" by deleting the wait.
///
/// `_stObject` is assigned ASYNCHRONOUSLY, after three.js finishes loading:
/// verified under --cdp, a synchronous assert right after mount fails with
/// "_stObject not set (async mount)". So these assertions REQUIRE a wait.
///
/// But `@wait_until` inside a fixture that mounts a 3D scene makes the compiler
/// emit `await` in a non-async function — `error[E0952] ... AwaitInFunction` —
/// i.e. a bundle that does not parse. Bisected in BUG-324: neither the 3D import,
/// nor the fixture, nor the paint rung, nor `@wait_until` is independently at
/// fault; it takes a 3D-scene fixture AND an awaiting directive together.
///
/// Asserting without the wait would produce a gate that fails for a reason
/// unrelated to GH-14 (the race, not the shape binding) — the same defect this
/// file already had once, when it scanned for a bareword form that had been
/// deliberately refused. A gate must fail for its own reason or it is noise.
///
/// So this file stays as the STATEMENT of the contract and is expected to be
/// blocked until BUG-324 lands. The compile-time half of GH-14 is fully gated by
/// check-contract.sh (E0946 on the undeclared bareword) and is GREEN.
@import "stdlib/testing/test"
@import "stdlib/3d"

@test "cdp gh-14 the canonical shape argument binds to real geometry" needs paint {
  @fixture {
    <canvas class="c14a" data-obj14a></canvas>
    [data-obj14a] {
      @stage(camZ: 5) {
        @object(shape: "torusknot", size: 1.2, color: #c0c5ff, roughness: 0.15, metalness: 1, spinY: 0.4)
      }
    }
  }
  @assert (document.querySelector('[data-obj14a]')._stObject.geometry.type === 'TorusKnotGeometry')
    "shape: \"torusknot\" must bind to TorusKnotGeometry — a silent fallback to IcosahedronGeometry is the GH-14 failure mode"
}

@test "cdp gh-14 a glass material binds to a physical material" needs paint {
  @fixture {
    <canvas class="c14b" data-obj14b></canvas>
    [data-obj14b] {
      @stage(camZ: 5) {
        @object(shape: "icosahedron", detail: 6, size: 1.4, material: "glass",
                roughness: 0.04, transmission: 1, ior: 1.5, thickness: 1.2)
      }
    }
  }
  @assert (document.querySelector('[data-obj14b]')._stObject.material.type === 'MeshPhysicalMaterial')
    "material: \"glass\" must bind to MeshPhysicalMaterial — a silent fallback to MeshStandardMaterial is the GH-14 failure mode"
}
