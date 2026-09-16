// Value syntax reference file
// Shows all value types supported in Spacetime

// ============================================
// Numeric Values
// ============================================

// Plain numbers
// 42
// -17
// 3.14
// .5

// Numbers with units
// 100px
// 50%
// 2em
// 1.5rem
// 100vh
// 360deg
// 200ms
// 2s

// ============================================
// Colors
// ============================================

// Hex colors
// #fff
// #aabbcc
// #11223344

// Color functions
// rgb(255, 0, 0)
// rgba(255, 0, 0, 0.5)
// hsl(120, 100%, 50%)
// hsla(120, 100%, 50%, 0.5)
// oklch(0.7 0.15 180)

// ============================================
// Strings
// ============================================

// "hello world"
// 'single quotes'
// "with \"escapes\""

// ============================================
// Calc Expressions
// ============================================

// calc(100% - 20px)
// calc(50vh + 100px)
// calc(100% / 3)
// calc((100% - 40px) / 2)

// ============================================
// CSS Functions
// ============================================

// var(--primary-color)
// var(--spacing, 16px)
// clamp(200px, 50%, 800px)
// min(100%, 500px)
// max(300px, 50vw)
// translate(10px, 20px)
// rotate(45deg)
// scale(1.5)
// translateX(100%)
// translateY(-50%)

// ============================================
// Element References (Spacetime-specific)
// ============================================

// &button
// &modal.opacity
// &card.transform.scale

// ============================================
// Preset references (RETIRED — `~` refs error loudly; easings are now
// `@form easing --name` referenced as `easing: --name`)

// --smooth
// --bouncy
// --spring-gentle

// ============================================
// Function Calls (Spacetime-specific)
// ============================================

// lerp(0, 100, $progress)
// clamp($value, 0, 1)
// format("Hello {}", $name)

// ============================================
// Variables
// ============================================

// $progress
// $value
// $name

// ============================================
// Complex Expressions
// ============================================

// Property values with multiple parts
// transform: translateX(100%) scale(1.5);
// box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
// background: linear-gradient(to right, #fff, #000);
