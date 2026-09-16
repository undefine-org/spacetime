// Parameter syntax reference file
// Shows all parameter variants in Spacetime

// ============================================
// Pattern Parameters ($name with optional default)
// Used in @pattern definitions
// ============================================

// Simple pattern params (single)
// @pattern example($name)

// Multiple pattern params
// @pattern example($value, $duration)

// Pattern params with defaults (number)
// @pattern fade($opacity: 0)

// Pattern params with defaults (string)
// @pattern slide($direction: "left")

// Pattern params with multiple defaults
// @pattern animate($direction: "left", $distance: 100)

// Pattern params with list defaults
// @pattern multi($items: [1, 2, 3])

// Pattern params with identifier defaults
// @pattern ease($easing: ease-in-out)

// Pattern params with mixed defaults
// @pattern complex($name, $delay: 0, $items: ["a", "b"])

// ============================================
// Primitive Parameters (for %primitive)
// ============================================

// Element reference parameter
// %primitive scroll-driver(&element) { ... }

// Multiple element parameters
// %primitive connect(&source, &target) { ... }

// Data parameter (untyped)
// %primitive fade($opacity) { ... }

// Multiple data parameters
// %primitive lerp($start, $end) { ... }

// Typed data parameter
// %primitive counter($count: number) { ... }

// Typed data with various types
// %primitive process($value: string) { ... }

// Typed parameter with default (number)
// %primitive animate(duration: number = 300) { ... }

// Typed parameter with default (string)
// %primitive label(text: string = "default") { ... }

// Typed parameter with default (boolean)
// %primitive toggle(enabled: bool = true) { ... }

// Combined element and data parameters
// %primitive observe(&el, $callback) { ... }

// Combined all parameter types
// %primitive scroll(&el, $progress: number, threshold: number = 0.5) { ... }

// Multiple typed parameters with defaults
// %primitive config(width: number = 100, height: number = 100, scale: number = 1) { ... }

// ============================================
// Function Parameters (for @fn)
// ============================================

// Simple typed parameter
// @fn double($x number): number { ... }

// Multiple typed parameters
// @fn add($a number, $b number): number { ... }

// String parameters
// @fn concat($a string, $b string): string { ... }

// Mixed type parameters
// @fn format($value number, $prefix string): string { ... }

// Boolean parameters
// @fn and($a boolean, $b boolean): boolean { ... }

// Array parameters
// @fn sum($values number[]): number { ... }

// Reference type parameters
// @fn getPrice($item Item): number { ... }

// Multiple reference type parameters
// @fn merge($a Item, $b Item): Item { ... }

// Complex type parameters
// @fn process($items Item[], $config { timeout: number }): Result { ... }

// Optional type parameters
// @fn find($items Item[], $predicate string?): Item? { ... }

// ============================================
// Parameter Syntax Details
// ============================================

// Pattern param format: $name or $name: default
// - $name - required parameter with $ prefix
// - $name: 100 - parameter with number default
// - $name: "str" - parameter with string default
// - $name: ident - parameter with identifier default
// - $name: [a, b] - parameter with list default

// Primitive param format: &name, $name, $name: type, name: type = default
// - &element - element reference with & prefix
// - $data - untyped data reference with $ prefix
// - $data: type - typed data reference
// - name: type - plain typed parameter
// - name: type = default - typed parameter with default

// Function param format: name: type
// - name: string - parameter name followed by type
// - Types can be primitives, references, arrays, or objects
