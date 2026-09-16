# Future Capabilities Enabled by Modularization

This document describes the future capabilities that Spacetime's recent modularization work enables. The separation of concerns between the registry, expansion engine, and code generation creates a foundation for powerful tooling and optimization.

---

## Table of Contents

1. [LSP Intelligence](#1-lsp-intelligence)
2. [Static Analysis](#2-static-analysis)
3. [Optimization Passes](#3-optimization-passes)
4. [Alternative Emit Targets](#4-alternative-emit-targets)
5. [User-Defined Macros](#5-user-defined-macros)
6. [Visual Debugger](#6-visual-debugger)

---

## 1. LSP Intelligence

The `MetaRegistry` stores all primitive and macro definitions with full type information, enabling rich IDE features through the Language Server Protocol.

### What It Enables

- **Hover Information**: Display macro documentation, parameter types, and exported signals on hover
- **Auto-Completion**: Suggest valid directives, parameters, and signal names contextually
- **Go-to-Definition**: Jump from `@fade-in` usage to its `%macro fade-in` definition in stdlib
- **Signature Help**: Show parameter signatures as users type macro arguments
- **Diagnostics**: Real-time error detection for invalid directives or type mismatches

### Available Data from Registry

```rust
// From MetaRegistry
pub fn get_primitive(&self, name: &str) -> Option<&PrimitiveDefAst>
pub fn get_macro(&self, name: &str) -> Option<&MacroDefAst>
pub fn get_macro_by_creates(&self, directive_name: &str) -> Option<&MacroDefAst>
pub fn primitive_names(&self) -> impl Iterator<Item = &str>
pub fn macro_names(&self) -> impl Iterator<Item = &str>
```

Each definition includes:

| Data | Source | Use Case |
|------|--------|----------|
| Parameters | `%form` clause | Signature help, validation |
| Exports | `%exports` block | Signal auto-complete |
| Creates directive | `%creates @name` | Directive lookup |
| Source span | AST nodes | Go-to-definition |

### Architecture for LSP Server

```
┌───────────────────────────────────────────────────────────────┐
│                     LSP Server                                 │
├───────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────────┐     ┌────────────────────────────────┐  │
│  │   Parser        │     │      MetaRegistry              │  │
│  │                 │────►│                                │  │
│  │ • Incremental   │     │ • Primitives: IntersectionObs  │  │
│  │ • Error-tolerant│     │ • Macros: fade-in, scroll...   │  │
│  └─────────────────┘     │ • Presets: easing curves       │  │
│                          └────────────────────────────────┘  │
│           │                          │                        │
│           ▼                          ▼                        │
│  ┌─────────────────┐     ┌────────────────────────────────┐  │
│  │   Analysis      │     │      Completion Provider       │  │
│  │                 │     │                                │  │
│  │ • Type checking │     │ • @ directives                 │  │
│  │ • Signal refs   │     │ • Parameter names              │  │
│  │ • Reactive states │     │ • Signal names ($visible)      │  │
│  └─────────────────┘     │ • Property paths ($.field)     │  │
│                          └────────────────────────────────┘  │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### Examples of User Experience

**Hover on `@fade-in`**:
```
@fade-in(duration: time = 600ms, threshold: number = 0.1)

Fades element in when it becomes visible.

Exports:
  $visible: bool - Whether element is in viewport
  $ratio: number - Intersection ratio (0-1)

Source: stdlib/macros/fade-in.st:1
```

**Auto-completion after `@`**:
```
@fade-in      Visibility animation
@scroll       Scroll-driven timeline
@on hover     Hover animation
@on click     Click animation
@loop         Looping animation
@each         Data iteration
```

**Signature help for `@scroll`**:
```
@scroll $name:ident (trigger: element = &self) {
         ^^^^^^ cursor here

$name: Timeline identifier
trigger: Element that triggers scroll tracking (default: &self)
```

### Additional Work Needed

1. **Incremental parsing**: Parser needs to handle partial/invalid files gracefully
2. **Position mapping**: Track source spans through macro expansion
3. **Document synchronization**: Handle file changes and reload affected registrations
4. **Workspace support**: Multi-file projects with cross-file references
5. **Semantic tokens**: Syntax highlighting for signals, directives, elements

### Potential Challenges

- **Macro expansion context**: Signals are only available after `%binds` - need expansion simulation
- **Performance**: Registry lookups must be fast for real-time completion
- **Cross-file references**: Imports and stdlib loading affect available symbols
- **Error recovery**: Parser must produce useful AST even with syntax errors

---

## 2. Static Analysis

The structured IR (intermediate representation) after parsing enables powerful static analysis that catches errors before runtime.

### Detecting Unused Signals

**What it enables**: Find signals exported by primitives that are never consumed by animations or derives.

**How IR enables it**:

The `%binds` clause captures signal names:
```rust
// From BindDecl in meta_ast
pub struct BindDecl {
    pub primitive: String,
    pub args: Vec<BindArg>,
    pub outputs: Vec<BindOutput>,  // Signal names like $visible
}
```

Analysis walks the macro body looking for signal references:
```rust
fn find_unused_signals(macro_def: &MacroDefAst) -> Vec<String> {
    let mut defined: HashSet<String> = HashSet::new();
    let mut used: HashSet<String> = HashSet::new();

    // Collect from %binds
    for bind in &macro_def.binds {
        for output in &bind.outputs {
            defined.insert(output.name.clone());
        }
    }

    // Walk %when, %animates, %derives looking for $var references
    walk_body(&macro_def.body, &mut used);

    defined.difference(&used).cloned().collect()
}
```

**Warning example**:
```
warning[W0101]: Signal '$ratio' is defined but never used
  --> stdlib/macros/fade-in.st:8
   |
 8 |   intersection(&self, threshold: $th) -> { $visible, $ratio }
   |                                                      ^^^^^^
   |
   = help: Remove unused signal or use it in %when or %derives
```

### Finding Unreachable Reactive States

**What it enables**: Detect state classes that can never be applied because the signal that drives them never takes the triggering value.

**How IR enables it**:

Reactive classes are parsed as conditional rules tied to signals:
```rust
pub struct ReactiveClass {
    pub selector: String,
    pub class: String,
    pub condition: SignalExpr,
}
```

Analysis collects the set of values ever assigned to the signal:
```rust
fn find_unreachable_states(
    classes: &[ReactiveClass],
    assignments: &SignalAssignments,
) -> Vec<String> {
    let mut unreachable = Vec::new();

    for cls in classes {
        if let SignalExpr::Eq(signal, literal) = &cls.condition {
            let values = assignments.possible_values(signal);
            if !values.contains(literal) {
                unreachable.push(format!("{}.{}", cls.selector, cls.class));
            }
        }
    }

    unreachable
}
```

**Warning example**:
```
warning[W0301]: Class '.gallery.is-loading' can never be applied
  --> gallery.st:12
   |
12 |     .is-loading: $status == "loading";
   |     ^^^^^^^^^^^
   |
   = note: Signal '$status' is assigned only "idle" and "error"
   = help: Assign "loading" to $status, update the condition, or remove the unused class
```

### Animation Timing Conflict Detection

**What it enables**: Detect when multiple animations target the same property in overlapping time ranges.

**How IR enables it**:

Animations are parsed into structured keyframes:
```rust
pub struct PropertyAnimation {
    pub property: String,
    pub keyframes: Vec<Keyframe>,
    pub easing: Option<String>,
}

pub struct Keyframe {
    pub at: f64,      // 0.0-1.0 progress
    pub value: String,
}
```

With range information:
```rust
pub struct AnimationScope {
    pub selector: String,
    pub properties: Vec<PropertyAnimation>,
    pub range: Option<(f64, f64)>,  // start to end
}
```

Analysis detects overlaps:
```rust
fn detect_timing_conflicts(scopes: &[AnimationScope]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    for (i, a) in scopes.iter().enumerate() {
        for b in scopes.iter().skip(i + 1) {
            if selectors_match(&a.selector, &b.selector) {
                let overlapping_props = a.properties.iter()
                    .filter(|pa| b.properties.iter().any(|pb| pa.property == pb.property))
                    .map(|p| p.property.clone());

                if ranges_overlap(a.range, b.range) {
                    for prop in overlapping_props {
                        conflicts.push(Conflict {
                            property: prop,
                            scopes: (a.selector.clone(), b.selector.clone()),
                        });
                    }
                }
            }
        }
    }

    conflicts
}
```

**Warning example**:
```
warning[W0401]: Property 'opacity' animated by multiple timelines in overlapping ranges
  --> hero.st:12
   |
12 |     @scroll reveal { opacity: 0 -> 1; range: 0 to 0.6 }
   |                      ^^^^^^^
   |
  --> hero.st:18
   |
18 |     @scroll parallax { opacity: 0.5 -> 1; range: 0.4 to 1 }
   |                        ^^^^^^^
   |
   = note: Ranges 0-0.6 and 0.4-1 overlap at 0.4-0.6
   = help: Adjust ranges to avoid overlap or use different properties
```

### CSS Property Conflict Warnings

**What it enables**: Detect when state-based CSS could create specificity conflicts or impossible combinations.

**How IR enables it**:

State definitions capture CSS properties:
```rust
pub struct InlineState {
    pub name: String,
    pub properties: Vec<(String, String)>,
}
```

Analysis checks for conflicts:
```rust
fn detect_css_conflicts(states: &[InlineState]) -> Vec<CssConflict> {
    let mut conflicts = Vec::new();

    // Check for conflicting shorthand/longhand
    for state in states {
        let shorthands = ["background", "border", "margin", "padding", "font"];
        for sh in shorthands {
            let has_shorthand = state.properties.iter().any(|(p, _)| p == sh);
            let has_longhand = state.properties.iter().any(|(p, _)| p.starts_with(&format!("{}-", sh)));
            if has_shorthand && has_longhand {
                conflicts.push(CssConflict::ShorthandLonghand {
                    state: state.name.clone(),
                    shorthand: sh.to_string(),
                });
            }
        }
    }

    conflicts
}
```

### Additional Work Needed

1. **Type inference**: Derive signal types from primitive exports
2. **Flow analysis**: Track signal values through conditionals
3. **Selector analysis**: Parse CSS selectors to detect specificity issues
4. **Cross-file analysis**: Follow imports to analyze full dependency graph

### Potential Challenges

- **Macro expansion**: Some issues only visible after full expansion
- **Runtime conditions**: Can't statically analyze `%when` with runtime values
- **CSS specificity**: Complex selectors require full CSS parser
- **Performance**: Analysis must complete in reasonable time for large projects

---

## 3. Optimization Passes

The modular architecture separates parsing from code generation, enabling optimization passes on the intermediate representation.

### Dead Code Elimination

**What it enables**: Remove unused primitives, signals, and state definitions from output.

**How to implement as IR transformation**:

```rust
pub fn eliminate_dead_code(graph: &mut DeclarationGraph) {
    // Build usage graph
    let mut used_signals: HashSet<String> = HashSet::new();
    let mut used_primitives: HashSet<String> = HashSet::new();

    // Start from animations (output) and trace back
    for anim in &graph.animations {
        collect_signal_refs(&anim.keyframes, &mut used_signals);
    }

    // Mark primitives that export used signals
    for binding in &graph.primitive_bindings {
        if binding.outputs.iter().any(|o| used_signals.contains(&o.name)) {
            used_primitives.insert(binding.primitive.clone());
        }
    }

    // Remove unused bindings
    graph.primitive_bindings.retain(|b| used_primitives.contains(&b.primitive));

    // Remove unused derived signals
    graph.derived_signals.retain(|d| used_signals.contains(&d.name));
}
```

**Before optimization**:
```javascript
// Generated JS
const obs1 = new IntersectionObserver(...);  // $visible
const obs2 = new ResizeObserver(...);        // $width, $height (unused!)
ST.set(el, 'visible', ...);
ST.set(el, 'width', ...);   // Never consumed
ST.set(el, 'height', ...);  // Never consumed
```

**After optimization**:
```javascript
// Generated JS
const obs1 = new IntersectionObserver(...);  // $visible
ST.set(el, 'visible', ...);
```

### CSS Deduplication and Minification

**What it enables**: Merge identical CSS rules and minify output.

**How to implement**:

```rust
pub fn deduplicate_css(css_blocks: &mut Vec<String>) -> String {
    let mut rule_map: HashMap<String, Vec<String>> = HashMap::new();

    // Parse each block and group by properties
    for block in css_blocks.iter() {
        let (selector, properties) = parse_css_rule(block);
        let prop_key = normalize_properties(&properties);
        rule_map.entry(prop_key).or_default().push(selector);
    }

    // Merge selectors with identical properties
    let mut output = String::new();
    for (properties, selectors) in rule_map {
        output.push_str(&format!(
            "{} {{ {} }}\n",
            selectors.join(", "),
            properties
        ));
    }

    minify_css(&output)
}

fn minify_css(css: &str) -> String {
    css.replace("  ", " ")
       .replace(" { ", "{")
       .replace("; }", "}")
       .replace(";\n", ";")
       .lines()
       .map(|l| l.trim())
       .collect::<Vec<_>>()
       .join("")
}
```

**Before**:
```css
.card[data-state="idle"] { cursor: grab; user-select: none; }
.draggable[data-state="idle"] { cursor: grab; user-select: none; }
.card[data-state="dragging"] { cursor: grabbing; }
```

**After**:
```css
.card[data-state="idle"],.draggable[data-state="idle"]{cursor:grab;user-select:none}
.card[data-state="dragging"]{cursor:grabbing}
```

### Animation Batching

**What it enables**: Combine multiple element animations into a single observer/driver.

**How to implement**:

```rust
pub fn batch_animations(bindings: &mut Vec<PrimitiveBinding>) {
    // Group intersection observers by threshold
    let mut intersection_groups: HashMap<String, Vec<PrimitiveBinding>> = HashMap::new();

    for binding in bindings.drain(..) {
        if binding.primitive == "intersection" {
            let threshold = binding.args.get("threshold").cloned().unwrap_or("0.5".into());
            intersection_groups.entry(threshold).or_default().push(binding);
        } else {
            // Keep non-intersection bindings as-is
            bindings.push(binding);
        }
    }

    // Create batched observers
    for (threshold, group) in intersection_groups {
        if group.len() > 1 {
            // Create single observer for multiple elements
            bindings.push(create_batched_intersection_binding(threshold, group));
        } else {
            bindings.extend(group);
        }
    }
}
```

**Before** (3 observers):
```javascript
new IntersectionObserver(cb1, { threshold: 0.5 }).observe(el1);
new IntersectionObserver(cb2, { threshold: 0.5 }).observe(el2);
new IntersectionObserver(cb3, { threshold: 0.5 }).observe(el3);
```

**After** (1 observer):
```javascript
const obs = new IntersectionObserver((entries) => {
    for (const entry of entries) {
        callbacks.get(entry.target)?.(entry);
    }
}, { threshold: 0.5 });
obs.observe(el1);
obs.observe(el2);
obs.observe(el3);
```

### Signal Dependency Optimization

**What it enables**: Minimize signal updates by computing dependency order and batching changes.

**How to implement**:

```rust
pub fn optimize_signal_dependencies(signals: &[DerivedSignal]) -> Vec<DerivedSignal> {
    // Build dependency graph
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for signal in signals {
        deps.insert(
            signal.name.clone(),
            extract_dependencies(&signal.expression)
        );
    }

    // Topological sort
    let order = topological_sort(&deps);

    // Reorder signals and mark update batches
    let mut optimized = Vec::new();
    let mut current_batch = Vec::new();
    let mut batch_deps: HashSet<String> = HashSet::new();

    for name in order {
        let signal = signals.iter().find(|s| s.name == name).unwrap();
        let signal_deps = &deps[&name];

        // If this signal depends on any in current batch, flush batch
        if signal_deps.intersection(&batch_deps).next().is_some() {
            optimized.extend(current_batch.drain(..));
            batch_deps.clear();
        }

        current_batch.push(signal.clone());
        batch_deps.insert(name);
    }

    optimized.extend(current_batch);
    optimized
}
```

### Additional Work Needed

1. **IR normalization**: Standardize representation before optimization
2. **Optimization flags**: Allow users to control which passes run
3. **Source maps**: Track optimizations for debugging
4. **Benchmarking**: Measure optimization impact

### Potential Challenges

- **Correctness**: Optimizations must preserve semantics
- **Debugging**: Optimized code harder to debug - need source maps
- **Compile time**: Complex optimizations slow compilation
- **Edge cases**: Some patterns resist optimization

---

## 4. Alternative Emit Targets

The separation between IR and code generation enables targeting platforms beyond web browsers.

### WASM for Performance-Critical Animations

**What it enables**: Run animation calculations in WebAssembly for complex particle systems or physics.

**How the emit layer enables this**:

The current codegen produces JavaScript:
```rust
// From codegen.rs
pub fn generate_primitive_js(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
) -> GeneratedPrimitive {
    // Transform %yield -> ST.set()
    // Transform %&el -> element references
    // Transform %param -> literal values
}
```

A WASM emitter would target Rust/WASM:
```rust
pub fn generate_primitive_wasm(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
) -> GeneratedWasmModule {
    // Generate Rust struct for state
    // Generate update function
    // Generate JS bindings for DOM interaction
}
```

**Example output**:
```rust
// Generated Rust (compiled to WASM)
#[wasm_bindgen]
pub struct ParticleSystem {
    particles: Vec<Particle>,
    time: f64,
}

#[wasm_bindgen]
impl ParticleSystem {
    pub fn update(&mut self, dt: f64) {
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += GRAVITY * dt;
        }
    }

    // Returns Float32Array for efficient transfer
    pub fn get_positions(&self) -> Vec<f32> {
        self.particles.iter()
            .flat_map(|p| [p.x as f32, p.y as f32])
            .collect()
    }
}
```

### Native iOS/Android Animations

**What it enables**: Generate native animation code for mobile apps.

**Emit target structure**:

```rust
pub trait EmitTarget {
    fn emit_animation(&self, anim: &AnimationBinding) -> String;
    fn emit_state_machine(&self, sm: &StateMachineAst) -> String;
    fn emit_driver(&self, driver: &TimelineDriver) -> String;
}

pub struct SwiftEmitter;
impl EmitTarget for SwiftEmitter {
    fn emit_animation(&self, anim: &AnimationBinding) -> String {
        format!(r#"
UIView.animate(
    withDuration: {},
    animations: {{
        view.alpha = {}
        view.transform = CGAffineTransform(translationX: 0, y: {})
    }}
)"#,
            anim.duration,
            anim.end_values.get("opacity").unwrap_or(&"1"),
            anim.end_values.get("translate-y").unwrap_or(&"0")
        )
    }
}

pub struct KotlinEmitter;
impl EmitTarget for KotlinEmitter {
    fn emit_animation(&self, anim: &AnimationBinding) -> String {
        format!(r#"
view.animate()
    .alpha({})
    .translationY({})
    .setDuration({})
    .start()
"#,
            anim.end_values.get("opacity").unwrap_or(&"1.0f"),
            anim.end_values.get("translate-y").unwrap_or(&"0f"),
            anim.duration
        )
    }
}
```

### React Native Bindings

**What it enables**: Use Spacetime animations in React Native apps.

**Emit approach**:

```rust
pub struct ReactNativeEmitter;

impl EmitTarget for ReactNativeEmitter {
    fn emit_animation(&self, anim: &AnimationBinding) -> String {
        format!(r#"
const {} = useRef(new Animated.Value({})).current;

useEffect(() => {{
    Animated.timing({}, {{
        toValue: {},
        duration: {},
        useNativeDriver: true,
    }}).start();
}}, []);

// In render:
<Animated.View style={{{{ opacity: {}, transform: [{{ translateY: {} }}] }}}}>
"#,
            anim.signal_name,
            anim.start_values.get("opacity").unwrap_or(&"0"),
            anim.signal_name,
            anim.end_values.get("opacity").unwrap_or(&"1"),
            anim.duration,
            anim.signal_name,
            anim.signal_name
        )
    }
}
```

### Static HTML for SSG

**What it enables**: Generate static HTML with CSS animations for static site generators.

**Emit approach**:

```rust
pub struct StaticHtmlEmitter;

impl EmitTarget for StaticHtmlEmitter {
    fn emit(&self, graph: &DeclarationGraph) -> StaticOutput {
        let mut css = String::new();
        let mut html_attrs: HashMap<String, String> = HashMap::new();

        // Convert scroll animations to CSS scroll-timeline
        for anim in &graph.animations {
            if anim.driver == "scroll" {
                css.push_str(&format!(r#"
@keyframes {} {{
    from {{ {} }}
    to {{ {} }}
}}

{} {{
    animation: {} linear;
    animation-timeline: scroll();
}}
"#,
                    anim.name,
                    format_keyframe(&anim.start_values),
                    format_keyframe(&anim.end_values),
                    anim.selector,
                    anim.name
                ));
            }
        }

        // State-based styles use data attributes
        for state in &graph.states {
            html_attrs.insert(
                "data-st-state".to_string(),
                state.initial.clone()
            );
        }

        StaticOutput { css, html_attrs, js: None }
    }
}
```

### How the Emit Layer Abstraction Enables This

The current architecture:

```
Spacetime Source
       │
       ▼
   ┌───────┐
   │ Parse │
   └───────┘
       │
       ▼
   ┌─────────┐
   │ Expand  │ (macro expansion)
   └─────────┘
       │
       ▼
   ┌─────────────────┐
   │ DeclarationGraph│ (IR)
   └─────────────────┘
       │
       ▼
   ┌─────────┐
   │ Codegen │ ◄── Currently: JavaScript only
   └─────────┘
       │
       ▼
  JS + CSS Output
```

The modular emit layer:

```
   ┌─────────────────┐
   │ DeclarationGraph│ (IR)
   └─────────────────┘
       │
       ├──────────────┬────────────────┬───────────────┐
       ▼              ▼                ▼               ▼
   ┌────────┐    ┌────────┐      ┌──────────┐    ┌─────────┐
   │ JS/CSS │    │  WASM  │      │  Swift   │    │ Static  │
   │ Emitter│    │ Emitter│      │  Emitter │    │ Emitter │
   └────────┘    └────────┘      └──────────┘    └─────────┘
       │              │                │               │
       ▼              ▼                ▼               ▼
    Web App      WASM Module       iOS App        SSG Site
```

### Additional Work Needed

1. **Emit trait definition**: Abstract interface for emitters
2. **Platform-specific primitives**: Some primitives only make sense on web
3. **Feature flags**: Conditionally include platform code
4. **Build system integration**: Integrate with Xcode, Gradle, etc.

### Potential Challenges

- **Feature parity**: Not all features translate to all platforms
- **Testing**: Need device testing for native targets
- **Maintenance**: Multiple emit targets multiply maintenance burden
- **Platform-specific optimization**: Each platform has different performance characteristics

---

## 5. User-Defined Macros

The stdlib demonstrates how macros work today. The path to user-defined macros extends this to application code.

### How Stdlib Macros Work Today

Macros are defined in `.st` files using the `%macro` syntax:

```st
%macro fade-in {
  %creates @fade-in

  %form {
    @fade-in($duration:time = 600ms, threshold: $th:number = 0.1)
  }

  %binds {
    intersection(&self, threshold: $th, once: true) -> { $visible }
  }

  %when $visible {
    %animates {
      opacity: 0 -> 1
    }

    %timing {
      duration: $duration
    }
  }
}
```

The registry loads these at compile time:

```rust
// From registry.rs
pub fn load_stdlib_from_dir(&mut self, dir: &Path) -> Result<(), MetaRegistryError> {
    self.load_dir_recursive(dir)
}
```

### Path to User-Defined Macros

**Step 1: Local macro files**

Allow `%macro` definitions in project files:

```st
// my-project/macros/brand-animation.st
%macro brand-fade {
  %creates @brand-fade

  %form {
    @brand-fade($direction: ("in" | "out") = "in")
  }

  %binds {
    intersection(&self, threshold: 0.2, once: true) -> { $visible }
  }

  %if $direction == "in" {
    %when $visible {
      %animates {
        opacity: 0 -> 1
        translate-y: 20px -> 0
      }
    }
  }
}
```

**Step 2: Import system**

```st
// my-project/main.st
@import "./macros/brand-animation.st"

.hero {
  @brand-fade(direction: "in")
}
```

**Step 3: Package distribution**

```toml
# spacetime.toml
[dependencies]
spacetime-animations = "1.0"
spacetime-3d = "0.5"
```

```st
// main.st
@import "spacetime-animations/morph"
@import "spacetime-3d/orbit"

.logo {
  @morph(target: ".logo-alt", duration: 500ms)
}
```

### Macro Distribution/Sharing

**Registry integration**:

```bash
# Publish macro package
st publish my-animations

# Install in project
st add my-animations
```

**Package structure**:
```
my-animations/
├── spacetime.toml       # Package metadata
├── src/
│   ├── index.st         # Main exports
│   ├── primitives/      # Custom primitives (if any)
│   └── macros/          # Macro definitions
└── examples/            # Usage examples
```

**spacetime.toml**:
```toml
[package]
name = "my-animations"
version = "1.0.0"
description = "Custom animation macros"

[exports]
macros = ["morph", "stagger-reveal", "flip"]

[dependencies]
# Can depend on other packages
```

### Examples of Useful User Macros

**1. Project-specific design system**:
```st
%macro brand-button-hover {
  %creates @brand-hover

  %form { @brand-hover }

  %binds {
    pointer(&self, events: ["enter", "leave"]) -> { $isOver }
  }

  %when $isOver {
    %animates {
      background: var(--brand-primary) -> var(--brand-accent)
      scale: 1 -> 1.05
    }
  }
}
```

**2. E-commerce interactions**:
```st
%macro add-to-cart {
  %creates @add-to-cart

  %form {
    @add-to-cart(target: $cart:selector)
  }

  %states {
    idle {}
    adding { pointer-events: none }
    added { background: var(--success) }
  }

  %on click {
    %transition idle -> adding
    %trigger cart-add-item
  }

  %on cart-add-complete {
    %transition adding -> added
    %after 2s { %transition added -> idle }
  }
}
```

**3. Dashboard widgets**:
```st
%macro realtime-chart {
  %creates @realtime-chart

  %form {
    @realtime-chart(src: $url:string, interval: $ms:number = 1000)
  }

  %binds {
    websocket($url) -> { $data, $connected }
    tick(fps: 60) -> { $frame }
  }

  %derives {
    $points: $data.values.slice(-100)
  }

  // ...
}
```

### Additional Work Needed

1. **Import resolution**: Resolve relative/package imports
2. **Package format**: Define package structure and metadata
3. **Version resolution**: Handle version conflicts in dependencies
4. **Sandboxing**: Primitives need security review for user code
5. **Documentation generation**: Auto-generate docs from macro definitions

### Potential Challenges

- **Security**: User primitives run JavaScript - need sandboxing
- **Versioning**: Breaking changes in macro APIs affect users
- **Discovery**: How do users find useful macro packages?
- **Quality control**: No guaranteed quality for user packages
- **Debugging**: Errors in macros harder to trace

---

## 6. Visual Debugger

The structured IR enables visual debugging tools for understanding Spacetime behavior.

### AST Visualization

**What it enables**: See the parsed structure of Spacetime code.

**Implementation approach**:

```rust
pub fn ast_to_visualization(ast: &StFile) -> AstVisualization {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for scope in &ast.scopes {
        let scope_id = add_node(&mut nodes, "scope", &scope.selector);

        for timeline in &scope.timelines {
            let timeline_id = add_node(&mut nodes, "timeline", &timeline.name);
            edges.push((scope_id, timeline_id));

            for animation in &timeline.animations {
                let anim_id = add_node(&mut nodes, "animation", &animation.target);
                edges.push((timeline_id, anim_id));
            }
        }
    }

    AstVisualization { nodes, edges }
}
```

**Visual representation**:
```
┌─────────────────────────────────────────────┐
│                  .hero                       │
│                (scope)                       │
└─────────────────────────────────────────────┘
                    │
        ┌───────────┴───────────┐
        ▼                       ▼
┌───────────────┐       ┌───────────────┐
│ @scroll       │       │ @on &.hover     │
│ reveal        │       │ lift          │
└───────────────┘       └───────────────┘
        │                       │
        ▼                       ▼
┌───────────────┐       ┌───────────────┐
│ .title        │       │ :host         │
│ opacity: 0→1  │       │ scale: 1→1.05 │
│ translateY    │       │               │
└───────────────┘       └───────────────┘
```

### Signal Flow Animation

**What it enables**: Visualize how signals flow through the system in real-time.

**Data structure**:
```rust
pub struct SignalFlowVisualization {
    pub signals: Vec<SignalNode>,
    pub connections: Vec<SignalConnection>,
    pub current_values: HashMap<String, f64>,
}

pub struct SignalNode {
    pub name: String,
    pub source: SignalSource,  // Primitive, Derive, or Input
    pub position: (f32, f32),
}

pub struct SignalConnection {
    pub from: String,
    pub to: String,
    pub transform: Option<String>,  // e.g., "* 100px"
}
```

**Real-time visualization**:
```
┌──────────────────────────────────────────────────────────────┐
│                     Signal Flow                               │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│   ┌──────────────┐                                           │
│   │ intersection │                                           │
│   │ (primitive)  │                                           │
│   └──────┬───────┘                                           │
│          │                                                   │
│          ├─── $visible: true ────►┌─────────────┐           │
│          │                        │   %when     │           │
│          └─── $ratio: 0.73 ──────►│   $visible  │           │
│                                   └──────┬──────┘           │
│                                          │                   │
│                                          ▼                   │
│                                   ┌─────────────┐           │
│                                   │  %animates  │           │
│                                   │  opacity    │───► CSS   │
│                                   │  translateY │           │
│                                   └─────────────┘           │
│                                                               │
│   Legend: ○ Primitive  ◇ Derive  □ Animation                │
└──────────────────────────────────────────────────────────────┘
```

### State Machine Visualization

**What it enables**: Interactive visualization of state machine definitions and transitions.

**From the AST**:
```rust
pub struct StateMachineAst {
    pub initial: String,
    pub inline_states: Vec<InlineState>,
    pub arrow_transitions: Vec<ArrowTransition>,
}
```

**Visual representation**:
```
┌─────────────────────────────────────────────────────────────┐
│                   Modal State Machine                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│                      ┌───────────┐                          │
│             ┌───────►│  closed   │◄────────┐               │
│             │        │ (initial) │         │               │
│             │        └─────┬─────┘         │               │
│             │              │               │               │
│         close          open.click     escape.keydown       │
│             │              │               │               │
│             │              ▼               │               │
│             │        ┌───────────┐         │               │
│             └────────│   open    │─────────┘               │
│                      │           │                          │
│                      └─────┬─────┘                          │
│                            │                                │
│                      submit.click                           │
│                            │                                │
│                            ▼                                │
│                      ┌───────────┐                          │
│                      │  saving   │                          │
│                      └─────┬─────┘                          │
│                            │                                │
│              ┌─────────────┴─────────────┐                 │
│              │                           │                 │
│          success                      error                │
│              │                           │                 │
│              ▼                           ▼                 │
│        ┌───────────┐              ┌───────────┐           │
│        │  closed   │              │   error   │           │
│        └───────────┘              └───────────┘           │
│                                                              │
│   Current state: [ open ] ◄── click to simulate            │
└─────────────────────────────────────────────────────────────┘
```

### Animation Timeline Preview

**What it enables**: Scrub through animations to preview their effect.

**Timeline data**:
```rust
pub struct TimelinePreview {
    pub duration_ms: u32,
    pub keyframes: Vec<PreviewKeyframe>,
    pub current_progress: f64,
}

pub struct PreviewKeyframe {
    pub at: f64,           // 0.0-1.0
    pub property: String,
    pub value: String,
    pub easing: String,
}
```

**Visual interface**:
```
┌─────────────────────────────────────────────────────────────┐
│                   Timeline Preview                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   @scroll hero-reveal (duration: scroll-based)              │
│                                                              │
│   Progress: ░░░░░░░░░░░░░█░░░░░░░░░░░░░░░░░░░░ 0.43         │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ opacity                                              │   │
│   │ 0 ─────────────────────────────────────────────── 1 │   │
│   │     ╭───────────────────╮                           │   │
│   │ 0.5 │                   │                           │   │
│   │     │       ▲ 0.43      │                           │   │
│   │ 0 ──╯                   ╰───────────────────────── │   │
│   │    0.0        0.3       0.5                   1.0   │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ translateY                                           │   │
│   │ 0px ─────────────────────────────────────────────── │   │
│   │         ╭──────────────────────────────╮            │   │
│   │ 20px ───╯                              ╰────────── │   │
│   │ 40px                 ▲ 20px                         │   │
│   │        0.0                            0.5    1.0    │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
│   Preview element: ┌───────────────────┐                    │
│                    │      .hero        │                    │
│                    │  opacity: 0.43    │                    │
│                    │  translateY: 20px │                    │
│                    └───────────────────┘                    │
│                                                              │
│   [▶ Play] [⏸ Pause] [⏮ Reset]                              │
└─────────────────────────────────────────────────────────────┘
```

### Implementation Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Visual Debugger                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   AST View   │    │ Signal Flow  │    │  State View  │  │
│  │              │    │              │    │              │  │
│  │ Tree display │    │ Flow diagram │    │ Graph view   │  │
│  │ Node inspect │    │ Live values  │    │ Transitions  │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                  Timeline Scrubber                      │ │
│  │                                                         │ │
│  │  Progress: ░░░░░░░░░█░░░░░░░░░░░░░░░░░ 0.35            │ │
│  │                                                         │ │
│  │  Property curves | Keyframe markers | Live preview     │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Runtime Inspector                    │   │
│  │                                                       │   │
│  │  Connected to: localhost:3000                        │   │
│  │  Active timelines: 3                                 │   │
│  │  Signal updates/sec: 47                              │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Additional Work Needed

1. **Debug protocol**: Define communication between runtime and debugger
2. **UI framework**: Build the visualization interface
3. **Performance instrumentation**: Add timing data to runtime
4. **Source mapping**: Link runtime state to source locations
5. **Browser extension**: Integrate with browser devtools

### Potential Challenges

- **Performance overhead**: Debugging instrumentation affects performance
- **UI complexity**: Rich visualizations require significant UI work
- **Real-time updates**: Need efficient protocol for live data
- **Cross-browser support**: Browser devtools APIs vary
- **State capture**: Capturing state without affecting behavior

---

## Summary

The modularization work creates a foundation for significant future capabilities:

| Capability | Enabled By | Difficulty | Impact |
|-----------|------------|-----------|--------|
| LSP Intelligence | MetaRegistry | Medium | High |
| Static Analysis | Structured IR | Medium | High |
| Optimization Passes | IR-based Codegen | Medium | Medium |
| Alternative Targets | Emit Layer Abstraction | High | High |
| User-Defined Macros | Macro System | Low | High |
| Visual Debugger | Runtime Instrumentation | High | Medium |

The key architectural decisions that enable these capabilities:

1. **Separation of parsing and expansion**: Allows analysis before code generation
2. **Registry as single source of truth**: Centralizes type information for tooling
3. **Structured IR**: Enables optimization passes and alternative emit targets
4. **Declarative macro system**: Allows static analysis of macro behavior
5. **Modular code generation**: Supports multiple output formats

These capabilities, once implemented, will make Spacetime a more powerful and developer-friendly animation and interaction system.
