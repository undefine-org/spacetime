# Both-Failed Parse Cases Analysis

This document catalogs the 29 `.st` files that both the legacy PEST parser and the new Chumsky parser fail to parse. These represent syntax features or experimental constructs that neither parser fully supports.

## Summary by Category

| Category | Count | Priority |
|----------|-------|----------|
| Preset Ampersand Syntax | 3 | High - used in real demos |
| Comparison Operators | 3 | High - used in assertions |
| Scene/WebGL Domain Files | 11 | Low - domain-specific |
| Function Type Annotations | 2 | Medium - type system |
| Double-Dollar Syntax | 3 | Medium - meta-references |
| Meta Conditionals | 1 | Medium - metasystem |
| % Directives | 3 | Medium - macro system |
| Test/Token Files | 3 | Low - test scaffolding |

---

## Category 1: Preset Ampersand Syntax (3 files)

**Syntax**: `@preset easing &name: value`

The ampersand (`&`) prefix for preset names is not recognized by either parser.

### Files:
1. **examples/demo.st** (line 8)
   ```
   @preset easing &hero-spring: spring(350, 15, 1);
   ```

2. **examples/phase7-demo.st** (line 5)
   ```
   @preset easing &ease-out: cubic-bezier(0, 0, 0.58, 1);
   ```

3. **tests/parser/syntax/complete.st** (line 16)
   ```
   @preset easing ~smooth: cubic-bezier(0.4, 0, 0.2, 1)
   ```
   Note: Uses `~` instead of `&`

**Fix Required**: Extend preset name parser to accept `&name` or `~name` prefix patterns.

---

## Category 2: Comparison Operators in Forms (3 files)

**Syntax**: `>= $value` in form definitions

Comparison operators (`>=`, `<=`, `>`, `<`) are not supported in form parameter positions.

### Files:
1. **stdlib/testing/assertions/binding.st** (line 360)
   ```
   @then $signal:binding on: $target:selector should have_update_count >= $count:number
   ```

2. **stdlib/testing/assertions/state.st** (line 235)
   ```
   @then $target:selector should have_transition_count >= $count:number
   ```

3. **stdlib/testing/assertions/timeline.st** (line 28)
   ```
   @then $target:selector should have_timeline_progress $name:string >= $threshold:number
   ```

**Fix Required**: Add comparison operator parsing in form inline elements for assertion DSL support.

---

## Category 3: Scene/WebGL Domain Files (11 files)

These are specialized 3D/WebGL scene graph macro definitions using advanced syntax.

### 3a. `%provides` / `%binds` Directives (2 files)
- **stdlib/macros/scene/camera.st** (line 78) - `%provides {`
- **stdlib/macros/scene/scene.st** (line 91) - `%provides {`

### 3b. Double-Dollar Meta References (4 files)
**Syntax**: `$$scene.$gl` - meta-level variable references

- **stdlib/macros/scene/scene-2d.st** (line 40) - `$$scene.$gl`
- **stdlib/macros/scene/scene-3d.st** (line 75) - `$$scene.$gl`
- **stdlib/macros/scene/shader-escape.st** (line 108) - `$$scene.$gl`
- **stdlib/macros/scene/react-to.st** (line 40) - `$primitive -> $captures`

### 3c. Advanced Macro Call Syntax (5 files)
- **stdlib/macros/scene/distort.st** (line 35) - Pattern with `*` modifier: `$params:effect_params*`
- **stdlib/macros/scene/examples/cube.st** (line 31) - `60deg` unit in macro call
- **stdlib/macros/scene/examples/custom-shader.st** (line 25) - GLSL code block
- **stdlib/macros/scene/examples/glow.st** (line 13) - Complex macro call body
- **stdlib/macros/scene/examples/gradient.st** (line 13) - Similar macro call issue
- **stdlib/macros/scene/examples/particles.st** (line 20) - `@react-to` with multiple captures
- **stdlib/macros/scene/examples/product-viewer.st** (line 26) - `@react-to` gesture syntax

**Note**: These files are experimental WebGL/3D scene graph macros. They may need their own specialized parser or DSL subset.

---

## Category 4: Function Type Annotations (2 files)

**Syntax**: `fn(param?: type) ~> return_type`

### Files:
1. **stdlib/primitives/webgl/texture.st** (line 199)
   ```
   $bind: fn(unit?: number) ~> number
   ```

2. **stdlib/testing/test.st** (line 124)
   ```
   $runTests: fn(filter?: string) ~> object
   ```

**Fix Required**: Add function type annotation parsing with optional parameters (`?:`) and return type arrow (`~>`).

---

## Category 5: Registry Percent Directives (2 files)

**Syntax**: `%get`, `%preset light_type`

### Files:
1. **stdlib/runtime/registries.st** (line 35)
   ```
   %get($name) { ST.timelines.get($name) }
   ```

2. **stdlib/macros/scene/light.st** (line 67)
   ```
   %preset light_type ambient {
   ```

**Fix Required**: Parse `%` directive syntax in appropriate contexts.

---

## Category 6: Meta Conditionals (1 file)

**Syntax**: `%if condition {`

### Files:
1. **tests/parser/syntax/metasystem.st** (line 135)
   ```
   %if useResize {
   ```

**Note**: The `%if` without `$` prefix is experimental conditional syntax.

---

## Category 7: String Concatenation in Macros (2 files)

**Syntax**: `"string" + $variable`

### Files:
1. **examples/scripts/analyze-directives.st** (line 11)
   ```
   @log "Analyzing: " + $path
   ```

2. **examples/scripts/migrate-timeline-syntax.st** (line 22)
   ```
   @log "Migrating: " + $path
   ```

**Fix Required**: Support string concatenation expressions in macro inline arguments.

---

## Category 8: Reporter/Test Infrastructure (1 file)

### Files:
1. **stdlib/testing/reporter.st** (line 85)
   ```
   %binds {
   ```

---

## Category 9: Test Scaffolding Files (3 files)

These are minimal test files for parser syntax coverage:

1. **tests/parser/syntax/tokens.st** (line 8)
   - Contains bare identifiers without context

2. **tests/parser/syntax/types.st** (line 9)
   - Contains type keywords as bare tokens

3. **tests/parser/syntax/scopes.st** (line 93)
   - Contains `:hover {` pseudo-selector block

---

## Priority Implementation Order

1. **High Priority (Real Usage)**
   - Preset ampersand syntax (`&name`) - 3 files
   - Comparison operators (`>=`) - 3 files

2. **Medium Priority (Type System)**
   - Function type annotations (`fn() ~> type`) - 2 files
   - Meta conditionals (`%if`) - 1 file
   - String concatenation in macros - 2 files

3. **Low Priority (Domain-Specific)**
   - Scene/WebGL files - 11 files (may need dedicated parser)
   - Registry directives - 2 files
   - Test scaffolding - 3 files

---

## Notes for Future Implementation

1. **Scene/WebGL files** may benefit from a separate parsing mode or DSL extension rather than trying to fit everything into the main Spacetime grammar.

2. **Comparison operators** in forms are specifically for the testing/assertion DSL - consider whether this should be a general feature or assertion-specific.

3. **Double-dollar syntax** (`$$`) appears to be for compile-time meta references distinct from runtime variables (`$`).

4. **Function type syntax** follows a pattern of `fn(params) ~> return` that's different from JavaScript arrow functions.

---

*Generated during PEST to Chumsky migration - January 2025*
