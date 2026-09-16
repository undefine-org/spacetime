# Animation Presets

Animation presets are reusable animation patterns that can be referenced by name in Spacetime DSL files. They provide a consistent, declarative way to apply common animations like fade-in, slide-up, bounce, etc.

## Overview

The preset system consists of:

1. **Built-in Presets**: A collection of commonly-used animation patterns
2. **PresetRegistry**: A registry for storing and looking up presets
3. **Custom Presets**: User-defined animation patterns via `@preset` declarations

## Built-in Animation Presets

### Fade Animations

#### `&fade-in`
Fade in from transparent to opaque.
- Properties: `opacity: 0 -> 1`
- Default easing: `&ease-out-quad`

```st
.element {
    @scroll reveal {
        & {
            opacity: 0 -> 1;
            easing: &ease-out-quad;
        }
    }
}
```

#### `&fade-out`
Fade out from opaque to transparent.
- Properties: `opacity: 1 -> 0`
- Default easing: `&ease-in-quad`

### Slide Animations

#### `&slide-up`
Slide up from below with fade in.
- Properties: `translateY: 20px -> 0`, `opacity: 0 -> 1`
- Default easing: `&ease-out-quad`

#### `&slide-down`
Slide down from above with fade in.
- Properties: `translateY: -20px -> 0`, `opacity: 0 -> 1`
- Default easing: `&ease-out-quad`

#### `&slide-left`
Slide left from right with fade in.
- Properties: `translateX: 20px -> 0`, `opacity: 0 -> 1`
- Default easing: `&ease-out-quad`

#### `&slide-right`
Slide right from left with fade in.
- Properties: `translateX: -20px -> 0`, `opacity: 0 -> 1`
- Default easing: `&ease-out-quad`

### Scale Animations

#### `&scale-in`
Scale in from smaller size with fade in.
- Properties: `scale: 0.9 -> 1`, `opacity: 0 -> 1`
- Default easing: `&ease-out-back`

#### `&scale-out`
Scale out to smaller size with fade out.
- Properties: `scale: 1 -> 0.9`, `opacity: 1 -> 0`
- Default easing: `&ease-in-quad`

### Rotation Animations

#### `&rotate-in`
Rotate in from -180deg with fade in.
- Properties: `rotate: -180deg -> 0deg`, `opacity: 0 -> 1`
- Default easing: `&ease-out-back`

#### `&spin`
Full 360 degree rotation.
- Properties: `rotate: 0deg -> 360deg`
- Default easing: `&linear`

```st
.spinner {
    @loop spin(1000ms) {
        & {
            rotate: 0deg -> 360deg;
            easing: &linear;
        }
    }
}
```

### Keyframe Animations

#### `&bounce`
Bounce animation with translateY keyframes.
- Keyframes: 0% -> 25% (-20px) -> 50% (0) -> 75% (-10px) -> 100% (0)
- Default easing: `&ease-out-quad`

#### `&shake`
Shake animation with translateX keyframes.
- Keyframes: Oscillates between -10px and 10px
- Default easing: `&linear`

#### `&pulse`
Pulse animation with scale keyframes.
- Keyframes: 0% (1) -> 50% (1.05) -> 100% (1)
- Default easing: `&ease-in-out-quad`

## Using Presets in Code

### Rust API

```rust
use spacetime::presets::PresetRegistry;

let registry = PresetRegistry::new();

// Look up a preset
if let Some(preset) = registry.get("&fade-in") {
    println!("Preset: {}", preset.name);
    println!("Description: {}", preset.description);
    for prop in &preset.properties {
        println!("  {}: {} -> {}", prop.property, prop.from_value, prop.to_value);
    }
}

// Check if preset exists
if registry.contains("&slide-up") {
    println!("Slide-up preset is available");
}

// List all presets
for name in registry.list_presets() {
    println!("Available: {}", name);
}
```

### Custom Presets

You can define custom animation presets in `.st` files:

```st
// Define a custom easing preset
@preset easing &my-smooth: cubic-bezier(0.4, 0, 0.2, 1);

// Define a custom scroll preset
@preset scroll &quick-reveal: start: 0, end: 0.5;

// Define a custom animation preset
@preset animation &custom-fade: from: 0, to: 0.8;
```

### Custom Presets Override Built-ins

Custom presets with the same name as built-in presets will override them:

```rust
let mut registry = PresetRegistry::new();

// Register a custom preset that overrides the built-in fade-in
let custom = AnimationPreset {
    name: "&fade-in".to_string(),
    description: "Custom fade-in to 80%".to_string(),
    properties: vec![PropertyTransition {
        property: "opacity".to_string(),
        from_value: "0".to_string(),
        to_value: "0.8".to_string(),
    }],
    default_easing: "&linear".to_string(),
    keyframes: HashMap::new(),
};

registry.register(custom);
```

## Integration with Parser

The parser automatically captures preset references in `.st` files:

```st
.hero {
    @scroll entrance {
        .title {
            opacity: 0 -> 1;
            easing: &ease-out-quad;  // Preset reference
        }
    }
}
```

The `PresetRegistry` can be used alongside the parser to validate and resolve preset references.

## Complete Example

Here's a complete example using multiple presets:

```st
// Custom presets
@preset easing &smooth: cubic-bezier(0.4, 0, 0.2, 1);

// Hero section with fade and slide
.hero {
    @scroll reveal {
        h1 {
            opacity: 0 -> 1;
            translateY: 30px -> 0;
            easing: &smooth;
        }

        .subtitle {
            opacity: 0 -> 1;
            at: 0.2;
        }
    }
}

// Cards with scale-in effect
.features {
    @scroll cards {
        .card {
            scale: 0.9 -> 1;
            opacity: 0 -> 1;
            easing: &ease-out-back;
            stagger: 0.1 from first;
        }
    }
}

// Button with pulse effect
.cta-button {
    @loop pulse(2000ms) {
        & {
            scale: {
                0%: 1;
                50%: 1.05;
                100%: 1;
            };
            easing: &ease-in-out-quad;
        }
    }
}

// Spinner with continuous rotation
.spinner {
    @loop spin(1000ms) {
        & {
            rotate: 0deg -> 360deg;
            easing: &linear;
        }
    }
}
```

## API Reference

### `PresetRegistry`

```rust
impl PresetRegistry {
    /// Create a new registry with built-in presets
    pub fn new() -> Self;

    /// Register a custom preset
    pub fn register(&mut self, preset: AnimationPreset);

    /// Look up a preset by name (with or without & prefix)
    pub fn get(&self, name: &str) -> Option<&AnimationPreset>;

    /// Check if a preset exists
    pub fn contains(&self, name: &str) -> bool;

    /// List all available preset names
    pub fn list_presets(&self) -> Vec<String>;
}
```

### `AnimationPreset`

```rust
pub struct AnimationPreset {
    pub name: String,
    pub description: String,
    pub properties: Vec<PropertyTransition>,
    pub default_easing: String,
    pub keyframes: HashMap<String, Vec<Keyframe>>,
}
```

### `PropertyTransition`

```rust
pub struct PropertyTransition {
    pub property: String,
    pub from_value: String,
    pub to_value: String,
}
```

### `Keyframe`

```rust
pub struct Keyframe {
    pub percentage: u8,
    pub value: String,
}
```

## Testing

The preset system includes comprehensive tests:

- Built-in preset availability
- Preset lookup with/without `&` prefix
- Custom preset registration
- Custom preset override of built-ins
- Integration with parser
- Keyframe preset validation

Run tests with:

```bash
cargo test --lib presets
```

## Future Enhancements

Potential future additions to the preset system:

1. **Preset Composition**: Combine multiple presets
2. **Preset Parameters**: Allow presets to accept arguments
3. **Preset Namespaces**: Organize presets into categories
4. **Preset Variants**: Define multiple variations of a preset
5. **Dynamic Presets**: Generate presets programmatically
