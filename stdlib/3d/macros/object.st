/// Object Macro — a procedural 3D mesh in the stage.
///
/// `@object(shape: "<name>", …)` creates a geometry + material and adds it to
/// the parent `@stage`'s scene. Every argument is a NAMED, typed parameter
/// (flat, no nested capture grammar — reliable to compile and read).
///
/// NB (GH-14): these examples used to be written `@object torusknot(…)`, with
/// the shape as a leading keyword. That form was never in the `%form` below —
/// the docs described a grammar the macro did not implement, and because a
/// non-matching form left every capture at its default, it compiled clean and
/// silently produced the DEFAULT icosahedron. It is now diagnosed (E0946).
/// Doc examples are only as trustworthy as the last time someone compiled
/// them; prefer `.st.md` or `@doc(src:)` for examples that must stay honest.
///
/// Material family is chosen by the `material` param:
///   standard  — physically-based (roughness/metalness)        [default]
///   physical  — adds transmission / ior / thickness / clearcoat
///   glass     — alias of physical, tuned for refraction
///   basic     — unlit, flat color
///
/// @example a metallic torus knot, auto-spinning
/// canvas.hero {
///   @stage {
///     @light(type: "directional", intensity: 2)
///     @object(shape: "torusknot", size: 1.2, color: #c0c5ff,
///             roughness: 0.15, metalness: 1, spinY: 0.4)
///   }
/// }
///
/// @example refractive glass icosahedron
/// @object(shape: "icosahedron", detail: 6, size: 1.4,
///   material: "glass", roughness: 0.04, transmission: 1, ior: 1.5, thickness: 1.2)
///
/// @param shape       - icosahedron|sphere|box|torus|torusknot|cone|cylinder|plane
/// @param size        - overall scale (default 1)
/// @param detail      - subdivision level (default 3)
/// @param material    - standard|physical|glass|basic (default standard)
/// @param color       - base color (default white)
/// @param roughness/metalness            - PBR params
/// @param transmission/ior/thickness/clearcoat - glass params
/// @param emissive/emissiveIntensity     - emission
/// @param spinX/Y/Z   - auto-rotation speed (radians/second)
/// @param posX/Y/Z    - position offset

%macro object {

  // GH-12 note: @object deliberately does NOT take `element(canvas)`. It is a
  // SCENE CHILD — it composes against the stage's handle, not a canvas-bounded
  // DOM element — so requiring a canvas selector here would be redundant and
  // wrong (an @object legitimately lives in the @stage body). The @stage macro
  // alone owns the canvas element constraint.
  %scope selector

  // PLAN-150 W4: an object MAY be named (`@object &floor grid(...)`) so a
  // `@on &.clip { &floor { reveal: 0 -> 1; } }` block can address it. The name
  // is optional; an anonymous object is still the most-recent `_stObject`.
  %capture_type object_name { $name:element }

  %form {
    @object $named:object_name? (
      shape: $shape:string = "icosahedron",
      size: $size:number = 1,
      detail: $detail:number = 3,
      material: $material:string = "standard",
      color: $color:color = white,
      roughness: $roughness:number = 0.3,
      metalness: $metalness:number = 0.6,
      transmission: $transmission:number = 0,
      ior: $ior:number = 1.5,
      thickness: $thickness:number = 1.0,
      clearcoat: $clearcoat:number = 0,
      iridescence: $iridescence:number = 0,
      iridescenceIOR: $iridescenceIOR:number = 1.3,
      emissive: $emissive:color = black,
      emissiveIntensity: $emissiveIntensity:number = 1.0,
      spinX: $spinX:number = 0,
      spinY: $spinY:number = 0,
      spinZ: $spinZ:number = 0,
      posX: $posX:number = 0,
      posY: $posY:number = 0,
      posZ: $posZ:number = 0,
      flatShading: $flatShading:bool = false
    ) {
      $children*
    }
  }

  %binds {
    three-object(&self,
      name: $named,
      shape: $shape,
      size: $size,
      detail: $detail,
      material: $material,
      color: $color,
      roughness: $roughness,
      metalness: $metalness,
      transmission: $transmission,
      ior: $ior,
      thickness: $thickness,
      clearcoat: $clearcoat,
      iridescence: $iridescence,
      iridescenceIOR: $iridescenceIOR,
      emissive: $emissive,
      emissiveIntensity: $emissiveIntensity,
      spinX: $spinX,
      spinY: $spinY,
      spinZ: $spinZ,
      posX: $posX,
      posY: $posY,
      posZ: $posZ,
      flatShading: $flatShading
    ) -> {
      $added
    }
  }

  %template {
    $children
  }
}
