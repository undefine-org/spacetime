/// Spacetime Standard Library
///
/// This file exports all primitives and macros from the standard library.
/// Import this file to get access to all built-in directives.
///
/// @example
/// @import "stdlib"
///
/// .hero {
///   @fade-in(duration: 800ms)
/// }

// =============================================================================
// PRIMITIVES
// =============================================================================

// Observer primitives (wrap browser Observer APIs)
@import "./primitives/intersection.st"  // IntersectionObserver
@import "./primitives/resize.st"        // ResizeObserver
@import "./primitives/mutation.st"      // MutationObserver

// Input primitives (wrap browser input APIs)
@import "./primitives/scroll.st"        // Scroll position tracking
@import "./primitives/pointer.st"       // Mouse/touch/pen tracking
@import "./primitives/gesture.st"       // Drag gesture handling
@import "./primitives/smooth-scroll.st" // Lenis-style smooth scroll engine
@import "./primitives/cursor.st"        // Branded custom cursor
@import "./primitives/magnetic.st"      // Proximity magnetic hover
@import "./primitives/particle-field.st" // N-body particle simulation with cursor repulsion
@import "./primitives/reveal.st"        // Kinetic typography text reveals
@import "./primitives/post.st"          // @post DOM lens: bloom / grain / vignette (PLAN-150 W2)
@import "./primitives/scatter.st"       // @scatter positions sampled from a shape (PLAN-150 W5)
@import "./primitives/transition-edge.st" // @form transition on a score -> (PLAN-150 W8)
@import "./primitives/score-audio.st"   // @audio on the score (PLAN-150 W7)
@import "./primitives/scroll-spy.st"  // Scroll position section tracking
@import "./primitives/when-guard.st"  // @when &self.<sig> reactive guard (BUG-088)

// Environment primitives (wrap browser environment APIs)
@import "./primitives/media.st"         // Media queries, color scheme, reduced motion
@import "./primitives/tick.st"          // requestAnimationFrame, intervals
@import "./primitives/now.st"           // live wall-clock time
@import "./primitives/fetch.st"         // Fetch API, GraphQL

// CSS at-rule primitives (CSS-only, no JS)
@import "./primitives/font-face.st"            // @font-face declaration
@import "./primitives/property-registration.st" // @property custom property registration

// NOTE: The hand-rolled WebGL primitives + `@scene` macro family were removed in
// favour of the vendored three.js engine. 3D is now an opt-in module: write
// `@import "stdlib/3d"` and use `@stage` / `@object` / `@light` / `@post` / … .
// See stdlib/3d/ (PLAN-050).


// =============================================================================
// MACROS - Visibility & Scroll
// =============================================================================

@import "./macros/fade-in.st"           // @fade-in, @fade-in-up, @fade-in-down, @fade-in-stagger
@import "./macros/timeline.st"        // driver bodies behind `@on &.<driver>(…)`
                                      // (the eight directives it used to export —
                                      //  @scroll/@load/@hover/@click/@loop/@mouse/
                                      //  @pointer/@time — retired in the 2026-07-26
                                      //  cutover, as did @value-change → &.text-change)
@import "./macros/signal-flip.st"     // @flip subscribed-signal layout transitions
@import "./macros/smooth-scroll.st"     // @smooth-scroll Lenis-style engine
@import "./macros/reveal.st"            // @reveal kinetic typography


// =============================================================================
// MACROS - Interaction
// =============================================================================

@import "./macros/on-event.st"          // @on &.hover/click/focus/visible/intersect
@import "./macros/now.st"               // @now live wall-clock
@import "./macros/drag.st"              // @swipe, @resize (@drag moved to dnd)
@import "./dnd/index.st"                // @drag + @drop-zone (drag-and-drop)
@import "./macros/cursor.st"            // @cursor branded custom cursor
@import "./macros/magnetic.st"          // @magnetic proximity hover
@import "./macros/magnetic-glow.st"    // @magnetic-glow proximity glow
@import "./macros/scroll-spy.st"      // @scroll-spy section tracking
@import "./macros/when-guard.st"      // @when &self.<sig> reactive guard (BUG-088)
@import "./macros/particle-field.st"  // @particle-field ambient particle overlay


// =============================================================================
// MACROS - Animation
// =============================================================================

@import "./macros/loop.st"              // @sequence, @after, @chain
                                        // (@loop/@mouse/@value-change retired — see the
                                        //  2026-07-26 cutover migration)


// =============================================================================
// MACROS - Data & State
// =============================================================================

@import "./macros/each.st"              // @each, @for, @repeat, @if
@import "./macros/type-data.st"         // @type, @data, @computed, @fn
// @state (state-reflect) lives in stdlib/enum/state.st; @socket in
// stdlib/macros/websocket.st — both register via the STDLIB_DIRS scan
// (PLAN-077 W6), so no source-level import is needed here.


// =============================================================================
// MACROS - Responsive
// =============================================================================

@import "./macros/responsive.st"        // @media, @dark, @light, @reduced-motion, @breakpoint, @print
@import "./macros/css-at-rules.st"     // @font, @property CSS at-rule macros
@import "./macros/post.st"             // @post cinematic DOM lens (PLAN-150 W2)
@import "./macros/scatter.st"          // @scatter positions from a shape (PLAN-150 W5)


// =============================================================================
// PRESETS
// =============================================================================

@import "./macros/presets.st"           // @preset, @apply, standard easings/durations/animations


// =============================================================================
// 3D — vendored three.js (opt-in module, NOT auto-imported)
// =============================================================================
// The old hand-rolled `@scene`/`@fill`/`@form`/`@distort` WebGL engine was
// retired (PLAN-050). Declarative 3D now lives in the `stdlib/3d` module, pulled
// in explicitly with `@import "stdlib/3d"` so pages that don't use 3D ship none
// of three.js. See stdlib/3d/MODULE.st.
