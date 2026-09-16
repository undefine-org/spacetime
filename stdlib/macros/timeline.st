/// Timeline Macros
///
/// Composes animation driver primitives with the apply-animations primitive.
/// This is the main entry point for declarative animations.
///
/// `@value-change` (below) needs real text-layout capability (line counts,
/// per-line widths, letter-spacing-correct measurement) to animate a content
/// change's reflow smoothly — that's the shared `stdlib/text` module's
/// vendored `pretext` engine, not a private copy. Importing it here (rather
/// than requiring every site author to remember `@import "stdlib/text"`
/// alongside `@import "stdlib"`) makes it available transitively wherever
/// `@value-change` is; the module's demand-emit rail still gates the actual
/// bytes shipped — a page that imports `stdlib` (and therefore this file)
/// but never uses `@value-change`/`@measure`/`@balance`/`@fit` ships ZERO of
/// pretext, exactly as before (see stdlib/text/MODULE.st "Lean law").
///
/// Architecture:
/// - Each macro binds a driver primitive (scroll-driver, load-driver, etc.) that yields $progress
/// - The apply-animations primitive receives $progress and the captured $body:keyframes
/// - No config.json is generated - animations emit directly as JS/CSS
///
/// Supported drivers:
/// - @scroll: Scroll-driven animations
/// - @load: On-load animations
/// - @hover / @click: Event-driven animations
/// - @loop: Continuously looping animations
/// - @mouse: Mouse-position driven animations
/// - @time: Duration-based playback
///
/// @example
/// .hero-image {
///   @scroll parallax(start: 0, end: 1, scrub: true) {
///     opacity: 0 -> 1
///     translate-y: 50px -> 0
///   }
/// }
///
/// .fade-in {
///   @load intro(delay: 200ms, duration: 600ms) {
///     opacity: 0 -> 1
///     translate-y: 20px -> 0
///   }
/// }


@import "stdlib/text"



/// Scroll Timeline Macro
///
/// Scroll-driven animation using scroll-driver primitive.

/// Load Timeline Macro
///
/// On-load animation using load-driver primitive.
/// Registers timeline name for @after chaining.

/// Hover Timeline Macro
///
/// Hover-triggered animation using event-driver primitive.

/// Click Timeline Macro
///
/// Click-triggered toggle animation.

/// Mouse Timeline Macro
///
/// Mouse-position driven animation.

/// Pointer Timeline Macro
///
/// Pointer-coordinate driven animation (author-facing alias for the mouse
/// scalar-progress driver).
/// Retired animation directives live in migration capsules.
