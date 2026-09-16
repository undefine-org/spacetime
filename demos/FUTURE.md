# Future Demo Ideas

Ideas for additional demos that showcase Spacetime's scene DSL capabilities.

> **Note:** The WebGL scene DSL macros (`@scene`, `@camera`, `@form`, `@light`, `@react-to`, `@fill`, etc.) shown below are **design proposals** documented in `stdlib/macros/scene/`. They are not yet fully implemented in the parser. The current demos use the timeline macros (`@scroll`, `@load`, `@hover`) which are working.

## Interactive Product Viewer

A 3D product viewer with orbit camera controls. Users can drag to rotate the model with momentum physics.

```spacetime
canvas.product-canvas {
  @scene(clearColor: #1a1a2e) {
    @camera orbit(distance: 3, minDistance: 2, maxDistance: 6) {
      @react-to $deltaX, $deltaY, $active from gesture(&self) {
        rotate: $deltaX * 0.01, $deltaY * 0.01
        momentum: true
      }
    }

    @light ambient(color: white, intensity: 0.3)
    @light point(position: [2, 3, 2], color: white, intensity: 1.0)
    @light point(position: [-2, 1, -2], color: #4a9eff, intensity: 0.5)

    @model("/assets/product.glb") {
      @surface(roughness: 0.2, metallic: 0.9)
    }
  }
}
```

**Features:**
- Orbit camera with drag-to-rotate
- Momentum physics for smooth interaction
- PBR materials with metallic surfaces
- Multiple point lights for product lighting

---

## Mouse-Reactive Particle Swarm

A particle system that reacts to mouse position. Particles flee from or attract to the cursor.

```spacetime
canvas.particles-canvas {
  @scene(clearColor: #0a0a0f) {
    @swarm(count: 1000, shape: circle, size: 2px) {
      distribution: random(bounds: viewport)
      color: rgba(100, 200, 255, 0.8)

      @react-to $normX, $normY from pointer(&self) {
        flee: $normX, $normY
        strength: 0.2
      }

      @react-to $progress from scroll(document) {
        size: 2px -> 6px at $progress
      }
    }
  }
}
```

**Features:**
- 1000+ particles with efficient instanced rendering
- Mouse-reactive flee/attract behavior
- Scroll-driven size changes
- Soft, glowing particle aesthetic

---

## Animated Gradient Background

A fullscreen animated gradient that rotates continuously.

```spacetime
canvas.gradient-canvas {
  @scene {
    @fill gradient(from: #667eea, to: #764ba2, type: radial) {
      @react-to $t from tick() {
        angle: $t * 0.0005 * 360deg
      }
    }
  }
}
```

**Features:**
- Smooth radial gradient
- Time-driven rotation
- Minimal code for maximum effect

---

## Combined Multi-Scene Showcase

A landing page that combines multiple scene DSL features:

1. **Hero section**: Scroll-driven 3D cube with metallic surface
2. **Background**: Animated gradient fill
3. **Mouse interaction**: Glow effect following cursor
4. **Text animations**: `@load` and `@after` timeline macros

```spacetime
/// Hero with 3D cube
canvas.hero-canvas {
  @scene(clearColor: transparent) {
    @camera perspective(fov: 60deg) {
      position: 0, 0, 5
    }

    @form cube(size: 2) {
      @surface(color: gold, metallic: 0.8)
      @react-to $progress from scroll(&page) {
        rotate-y: $progress * 720deg
      }
    }
  }
}

/// Background gradient
canvas.bg-canvas {
  @scene {
    @fill gradient(from: #0a0a1f, to: #1a1a3e, type: radial) {
      @react-to $t from tick() {
        angle: $t * 0.0001 * 360deg
      }
    }
  }
}

/// Mouse-following glow
canvas.glow-canvas {
  @scene {
    @fill glow(color: cyan, radius: 300px, intensity: 0.5) {
      @react-to $localX, $localY from pointer(&self) {
        position: [$localX, $localY]
      }
    }
  }
}

/// Text animations
h1.hero-title {
  @load intro(duration: 600ms, delay: 200ms) {
    opacity: 0 -> 1
    translate-y: 30px -> 0
  }
}

p.hero-subtitle {
  @after intro subtitle(duration: 400ms, delay: 100ms) {
    opacity: 0 -> 1
    translate-y: 20px -> 0
  }
}
```

**Features:**
- Multiple canvas layers with different effects
- Combined 3D and 2D rendering
- Mouse-reactive lighting
- Sequenced text animations with `@load` and `@after`

---

## Custom Shader Demo

A custom GLSL shader with mouse-reactive waves.

```spacetime
canvas.shader-canvas {
  @scene {
    @shader fragment {
      precision mediump float;
      uniform float u_time;
      uniform vec2 u_resolution;
      uniform vec2 u_mouse;

      void main() {
        vec2 uv = gl_FragCoord.xy / u_resolution;
        vec2 mouse = u_mouse / u_resolution;

        // Colorful animated waves
        vec3 col = 0.5 + 0.5 * cos(u_time + uv.xyx * 3.0 + vec3(0.0, 2.0, 4.0));

        // Wave distortion
        float wave = sin(uv.x * 10.0 + u_time * 2.0) * 0.02;
        wave += sin(uv.y * 8.0 - u_time * 1.5) * 0.02;
        col += wave;

        // Mouse glow
        float d = length(uv - mouse);
        float glow = exp(-d * 5.0) * 0.5;
        col += glow;

        // Vignette
        float vignette = 1.0 - length(uv - 0.5) * 0.8;
        col *= vignette;

        gl_FragColor = vec4(col, 1.0);
      }
    }

    uniforms {
      u_mouse: $x, $y from pointer(&self)
    }
  }
}
```

**Features:**
- Raw GLSL escape hatch for custom effects
- Mouse-reactive uniforms
- Animated wave patterns
- Vignette post-processing

---

## Implementation Notes

These demos require the full Spacetime pipeline with stdlib loading. Some features depend on parser improvements noted in `.local/upgrades/scene-dsl-regressions.md`:

- **CSS emission**: Currently requires static CSS file workaround
- **Conditional `%binds`**: Use specific macros (`@on-visible`) instead of generic dispatchers
- **`%fn` helpers**: Functions like `colorToVec3()` are in JavaScript runtime

As the parser improves, these demos will work with fewer workarounds.
