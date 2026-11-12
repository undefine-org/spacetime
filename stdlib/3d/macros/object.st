/// Object Macro — a procedural 3D mesh in the stage.
///
/// `@object(shape: "<name>", …)` creates a geometry + material and adds it to
/// the parent `@stage`'s scene. Every argument is a NAMED, typed parameter
/// (flat, no nested capture grammar — reliable to compile and read).
///
/// NB (GH-14): these examples used to be written `@object torusknot(…)`, a form
/// the macro never had; it compiled clean and silently made the DEFAULT
/// icosahedron. Its examples are now `%example` clauses, which
/// `spacetime check --examples` compiles, so they can't drift from the `%form`
/// again (PLAN-156).

/// The geometry an object starts from.
%capture_type object_shape {
    "icosahedron"
  | "sphere"
  | "box"
  | "torus"
  | "torusknot"
  | "cone"
  | "cylinder"
    /// a flat square: `rotateX: -90` lays it down as a floor
  | "plane"
  | "coin"
}

/// What an object's surface is made of.
%capture_type object_material {
    /// physically based: roughness and metalness
    "standard"
    /// adds transmission, ior, thickness and clearcoat
  | "physical"
    /// physical, tuned for refraction
  | "glass"
    /// unlit, flat colour
  | "basic"
}

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
      shape: $shape:object_shape = icosahedron,
      /// overall scale
      size: $size:number = 1,
      /// subdivision level
      detail: $detail:number = 3,
      material: $material:object_material = standard,
      /// base color
      color: $color:color = white,
      /// how matte the surface is (0 is a mirror)
      roughness: $roughness:unit = 0.3,
      /// how metallic it is
      metalness: $metalness:unit = 0.6,
      /// how much light passes through (glass, physical)
      transmission: $transmission:unit = 0,
      /// index of refraction (glass is about 1.5)
      ior: $ior:number = 1.5,
      /// how thick the refracting volume reads
      thickness: $thickness:number = 1.0,
      /// a varnish layer over the surface
      clearcoat: $clearcoat:unit = 0,
      /// thin-film sheen, like a soap bubble or an oil slick (physical, glass)
      iridescence: $iridescence:unit = 0,
      /// iridescence film IOR 1.0..2.333 (shifts the colour band)
      iridescenceIOR: $iridescenceIOR:number = 1.3,
      /// the colour it glows with
      emissive: $emissive:color = black,
      emissiveIntensity: $emissiveIntensity:intensity = 1.0,
      /// auto-rotation speed (radians/second)
      spinX: $spinX:number = 0,
      spinY: $spinY:number = 0,
      spinZ: $spinZ:number = 0,
      /// position offset
      posX: $posX:number = 0,
      posY: $posY:number = 0,
      posZ: $posZ:number = 0,
      /// rest rotation: `rotateX: -90` lays a plane flat as a floor
      rotateX: $rotateX:degrees = 0,
      rotateY: $rotateY:degrees = 0,
      rotateZ: $rotateZ:degrees = 0,
      /// rest stretch per axis: a lamp post is a cylinder with `scaleY: 12`
      scaleX: $scaleX:number = 1,
      scaleY: $scaleY:number = 1,
      scaleZ: $scaleZ:number = 1,
      /// render flat per-face normals (crisp faceted/low-poly look)
      flatShading: $flatShading:bool = false
    ) {
      $children*
    }
  }

  // `pos: 0 3 -7` is `posX: 0, posY: 3, posZ: -7` (one value sets all three);
  // the later of a shorthand and a longhand wins, as in CSS
  %shorthand pos { posX posY posZ }
  %shorthand rotate { rotateX rotateY rotateZ }
  %shorthand scale { scaleX scaleY scaleZ }
  %shorthand spin { spinX spinY spinZ }

  %binds {
    three-object(&self, name: $named, ...) -> {
      $added
    }
  }

  %registers part($named) {
    kind: object
  }

  %template {
    $children
  }

  %example "a metallic torus knot, auto-spinning" {
    @import "stdlib/3d"
    canvas.hero {
      @stage {
        @light (type: directional, intensity: 2)
        @object (shape: torusknot, size: 1.2, color: #c0c5ff, roughness: 0.15, metalness: 1, spinY: 0.4)
      }
    }
    <canvas class="hero"></canvas>
  }

  %example "refractive glass" {
    @import "stdlib/3d"
    canvas.hero {
      @stage {
        @light (type: directional, intensity: 2)
        @object (shape: icosahedron, detail: 6, size: 1.4, material: glass, roughness: 0.04, transmission: 1, ior: 1.5, thickness: 1.2)
      }
    }
    <canvas class="hero"></canvas>
  }
}
