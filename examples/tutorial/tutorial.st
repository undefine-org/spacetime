/// ============================================
/// SPACETIME INTERACTIVE TUTORIAL
/// ============================================
///
/// This file contains all the Spacetime animations for the tutorial.
/// Each section demonstrates the technique it teaches - meta-learning!
///
/// Run with: spacetime serve examples/tutorial


$toggleOn bool: false;



/// ============================================
/// SECTION 1: @load - PAGE LOAD ANIMATIONS
/// ============================================
///
/// The @load directive triggers animations when the page loads.
/// Perfect for hero sections, welcome screens, and above-fold content.
///
/// Syntax: @load timeline-name(duration) { property: start -> end; }

.welcome-section {
    @on &.load(name: welcome-entrance, duration: 1000ms, delay: 0ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: -30px -> 0;
      }
    h1 {
        @on &.load(name: title-entrance, duration: 1000ms, delay: 200ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: -20px -> 0;
      }
    }

    .lead {
        @on &.load(name: subtitle-entrance, duration: 1000ms, delay: 400ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: -15px -> 0;
      }
    }

    .subtitle {
        @on &.load(name: explanation-entrance, duration: 1000ms, delay: 600ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
      }
    }

    pre {
        @on &.load(name: code-entrance, duration: 1000ms, delay: 800ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: 20px -> 0;
      }
    }

    .scroll-hint {
        @on &.load(name: hint-entrance, duration: 1000ms, delay: 1200ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
      }
    }
}



/// ============================================
/// SECTION 2: @scroll - SCROLL-DRIVEN ANIMATIONS
/// ============================================
///
/// The @scroll directive creates scroll-linked animation timelines.
/// Elements animate as they enter/exit the viewport.
///
/// Key options:
/// - threshold: how much of the element must be visible (0.0 - 1.0)
/// - stagger: delay between multiple elements
/// - once: whether to animate only once or on every scroll

// Reveal items as they scroll into view
// This creates the signature scroll-reveal effect
.reveal-item {
    @on &.scroll(name: reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: 40px -> 0;
        stagger: 0.1 first;
      }
}

// Alternative: @on visible for simpler one-time reveals
// Use this when you want more control over when animation triggers
// .reveal-item {
//     @on visible entrance(600ms, threshold: 0.2) {
//         & { opacity: 0 -> 1; translateY: 40px -> 0; }
//     }
// }



/// ============================================
/// SCROLL PROGRESS INDICATOR
/// ============================================
///
/// @scroll-progress tracks page scroll position (0% to 100%)
/// Great for progress bars, reading indicators, parallax effects

.scroll-progress {
    @scroll-progress(axis: "y") {
        width: 0% -> 100%;
    }
}



/// ============================================
/// SECTION 3: @on hover - HOVER INTERACTIONS
/// ============================================
///
/// The @on hover directive creates hover-triggered animations.
/// Unlike CSS :hover, these can use springs, sequences, and complex timing.
///
/// Spring physics: spring(stiffness, damping)
/// - Higher stiffness = snappier motion
/// - Higher damping = less oscillation

.hover-card {
    @on &.hover(name: lift, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> -8px;
        box-shadow: 0 4px 6px rgba(0,0,0,0.1) -> 0 20px 40px rgba(0,0,0,0.3);
        easing: cubic-bezier(0.34, 1.56, 0.64, 1);
      }
    .icon {
        @on &.hover(name: icon-pop, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 1.1;
        easing: cubic-bezier(0.34, 1.56, 0.64, 1);
      }
    }

    // Subtle color shift on card title
    h3 {
        @on &.hover(name: title-glow, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        color: var(--text) -> var(--primary);
      }
    }
}

// Individual card effects - nth-child targeting
.hover-card:nth-child(1) {
    @on &.hover(name: card-1-lift, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> -12px;
        easing: cubic-bezier(0.34, 1.56, 0.64, 1);
      }
}

.hover-card:nth-child(2) {
    @on &.hover(name: card-2-scale, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 1.02;
        easing: cubic-bezier(0.34, 1.56, 0.64, 1);
      }
}

.hover-card:nth-child(3) {
    @on &.hover(name: card-3-glow, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        border-color: var(--border) -> var(--primary);
        box-shadow: none -> 0 0 30px rgba(99, 102, 241, 0.3);
      }
}



/// ============================================
/// SECTION 4: $-SIGNALS - REACTIVE STATE
/// ============================================
///
/// File-scope signals give components reactive state without macros.
/// A boolean signal can drive a CSS class with `.is-on: $signalName`,
/// and `@on click { $signalName <- !$signalName; }` toggles it.
///
/// Components:
/// - `$name bool: false;` - declares a file-scope signal
/// - `.class: $name;` - binds a reactive CSS class to the signal
/// - `@on click { $name <- expr; }` - updates the signal on click

.toggle-button {
    // Default / off visual state
    background: var(--bg-card);
    border-color: var(--border);
    color: var(--text);

    // Reactive class: turns on when $toggleOn is true
    .is-on: $toggleOn;

    // Toggle the signal on click
    @on &.click {
        $toggleOn <- !$toggleOn;
    }

    // Hover / press animations are preserved
    @on &.hover(name: button-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 1.05;
      }

    @on &.click(name: button-press, duration: 100ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 0.95 -> 1;
      }
}

.toggle-button.is-on {
    background: var(--success);
    border-color: var(--success);
    color: white;
    box-shadow: 0 0 20px rgba(16, 185, 129, 0.4);
}

// Update the status display based on toggle state
.state-display {
    text <- $toggleOn ? "on" : "off";
}



/// ============================================
/// SECTION 5: SIGNALS & REACTIVE DATA
/// ============================================
///
/// NOTE: Full signal syntax ($signals, reactive props) requires runtime support.
/// This demo focuses on click/hover animations that work standalone.
///
/// Example signal syntax (for documentation):
///   .counter { $count number: 0; }
///   .counter-value { text <- $count; }
///   .btn { @on click { $count <- $count + 1; } }

// Counter button animations - visual feedback on interaction
.counter-btn.increment {
    @on &.click(name: increment-press, duration: 150ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.9 -> 1;
        }

      }
}

.counter-btn.decrement {
    @on &.click(name: decrement-press, duration: 150ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 0.9 -> 1;
        }

      }
}

// Hover effects for counter buttons
.counter-btn {
    @on &.hover(name: btn-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        & {
            scale: 1 -> 1.1;
        }

      }
}



/// ============================================
/// SECTION 6: COMPOSITION - COMBINING TECHNIQUES
/// ============================================
///
/// The real power of Spacetime is combining multiple animations.
/// Layer scroll, hover, state, and continuous animations for rich UX.

.composition-demo {
    @on &.scroll(name: composition-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translateY: 60px -> 0;
      }
    @on &.hover(name: composition-lift, duration: 400ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> -5px;
        box-shadow: 0 4px 20px rgba(0,0,0,0.2) -> 0 20px 60px rgba(0,0,0,0.4);
        easing: cubic-bezier(0.34, 1.56, 0.64, 1);
      }
    @on &.hover(name: border-glow, duration: 400ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        border-color: var(--border) -> var(--primary);
      }
}

// Floating orbs - continuous looping animation
// These create ambient motion in the background
.floating-orb {
    @on &.loop(name: float, duration: 8s, mode: "pingpong", phase: 0) {
        easing: ease-in-out;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> 30px;
        opacity: 0.2 -> 0.5;
      }
}

// Stagger the orbs so they don't all move in sync
.floating-orb.orb-1 {
    @on &.loop(name: float-1, duration: 6s, mode: "pingpong", phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> 25px;
        translateX: 0 -> 15px;
      }
}

.floating-orb.orb-2 {
    @on &.loop(name: float-2, duration: 10s, mode: "pingpong", phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> 40px;
        translateX: 0 -> -10px;
      }
}

.floating-orb.orb-3 {
    @on &.loop(name: float-3, duration: 7s, mode: "pingpong", phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        translateY: 0 -> 20px;
        rotate: 0deg -> 10deg;
      }
}

// Composition content inherits parent animations
// But can also have its own
.composition-content {
    h3 {
        @on &.hover(name: title-shimmer, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        color: var(--text) -> var(--primary);
      }
    }
}



/// ============================================
/// NAVIGATION ENHANCEMENTS
/// ============================================

nav a {
    @on &.hover(name: nav-hover, duration: 200ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        color: var(--text-muted) -> var(--primary);
      }
}



/// ============================================
/// TIPS & BEST PRACTICES
/// ============================================
///
/// 1. Performance: Use transform properties (translateX/Y, scale, rotate)
///    over layout-triggering properties (width, height, top, left)
///
/// 2. Timing: Keep most animations under 500ms for responsiveness.
///    Only use longer durations for dramatic reveals.
///
/// 3. Easing: Use spring() for natural, physical motion.
///    Use ease-out for entrances, ease-in for exits.
///
/// 4. Stagger: Add stagger to lists/grids for a polished look.
///    Values of 0.05-0.15s work well for most cases.
///
/// 5. States: Use file-scope `$name bool: false;` signals for reactive state.
///    Bind a class with `.is-on: $name;` and update it with `@on click { $name <- expr; }`.
///
/// 6. Composition: Layer animations thoughtfully - don't overwhelm users.
///    Subtle ambient motion + interactive feedback = great UX
