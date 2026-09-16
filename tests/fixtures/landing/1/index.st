/// Nexus Labs - Creative Technology Studio
/// Dark futuristic aesthetic: neon cyan + electric purple on deep black

// Easing presets
@version 2026-07-26;

@form easing --smooth { cubic-bezier(0.4, 0, 0.2, 1) }
@form easing --bounce { cubic-bezier(0.34, 1.56, 0.64, 1) }

// =============================================================================
// Reset & Base
// =============================================================================

*, *::before, *::after {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: #050510;
    color: #fff;
    overflow-x: hidden;
    line-height: 1.6;
    -webkit-font-smoothing: antialiased;
}

// =============================================================================
// WebGL Background
// =============================================================================

canvas.webgl-bg {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;

    @scene(clearColor: #050510, antialias: true) {
        // Purple ambient glow
        @fill glow(color: #4a0080, radius: 600px, intensity: 0.3, center: screen, blend: add) {
            @pulse-intensity(base: 0.3, amplitude: 0.1, frequency: 0.2)
            @pulse-radius(base: 600, amplitude: 120, frequency: 0.15)
        }

        // Blue ambient glow
        @fill glow(color: #002060, radius: 500px, intensity: 0.25, center: screen, blend: add) {
            @pulse-intensity(base: 0.25, amplitude: 0.08, frequency: 0.25)
            @pulse-radius(base: 500, amplitude: 80, frequency: 0.2)
        }

        // Cyan mouse-follow glow
        @fill glow(color: #00ffcc, radius: 200px, intensity: 1.0, center: mouse, blend: add) {
            @pulse-radius(base: 200, amplitude: 30, frequency: 0.7)
        }

        // Purple mouse-follow accent
        @fill glow(color: #7B2FFF, radius: 150px, intensity: 0.5, center: mouse, blend: add) {
            @pulse-falloff(base: 2.5, amplitude: 0.6, frequency: 0.4)
        }

        // Post-processing
        @distort chromatic(offset: 2px, angle: 45deg)
        @distort vignette(radius: 0.85, softness: 0.4)
    }
}

// =============================================================================
// Navigation
// =============================================================================

.nav {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 2rem;
    padding: 1.25rem 2.5rem;
    background: rgba(5, 5, 16, 0.6);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.nav-logo {
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    color: #00ffcc;
    text-decoration: none;
    margin-right: auto;
}

.nav-link {
    font-size: 0.8rem;
    color: #94a3b8;
    text-decoration: none;
    letter-spacing: 0.04em;
}

// =============================================================================
// Hero Section
// =============================================================================

.hero {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    padding: 2rem;
}

.hero-orbs {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 0;
}

.orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(80px);
}

.orb-1 {
    width: 400px;
    height: 400px;
    background: rgba(123, 47, 255, 0.2);
    top: 10%;
    left: 10%;
}

.orb-2 {
    width: 300px;
    height: 300px;
    background: rgba(0, 255, 204, 0.12);
    top: 55%;
    right: 12%;
}

.orb-3 {
    width: 260px;
    height: 260px;
    background: rgba(123, 47, 255, 0.15);
    bottom: 10%;
    left: 40%;
}

.hero-content {
    position: relative;
    z-index: 1;
    text-align: center;
    max-width: 800px;
}

.hero-tag {
    display: inline-block;
    padding: 0.35rem 1rem;
    background: rgba(0, 255, 204, 0.06);
    border: 1px solid rgba(0, 255, 204, 0.2);
    border-radius: 999px;
    font-size: 0.75rem;
    color: #00ffcc;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin-bottom: 2rem;
}

.hero-title {
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: clamp(2.5rem, 7vw, 5rem);
    font-weight: 700;
    line-height: 1.05;
    margin-bottom: 1.5rem;
    background: linear-gradient(135deg, #ffffff 0%, #00ffcc 50%, #7B2FFF 100%);
    background-size: 200% auto;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.hero-sub {
    font-size: clamp(1rem, 2vw, 1.15rem);
    color: #94a3b8;
    line-height: 1.7;
    max-width: 520px;
    margin: 0 auto 2.5rem;
    font-weight: 300;
}

// =============================================================================
// Buttons
// =============================================================================

.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.85rem 2rem;
    font-weight: 600;
    font-size: 0.9rem;
    border: none;
    border-radius: 10px;
    cursor: pointer;
    text-decoration: none;
}

.btn-primary {
    background: linear-gradient(135deg, #00ffcc 0%, #7B2FFF 100%);
    color: #050510;
    box-shadow: 0 0 30px rgba(0, 255, 204, 0.2), 0 0 60px rgba(123, 47, 255, 0.1);
}

// =============================================================================
// Section Common
// =============================================================================

.section-head {
    text-align: center;
    margin-bottom: 4rem;
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
}

.tag {
    display: inline-block;
    padding: 0.25rem 0.75rem;
    background: rgba(0, 255, 204, 0.06);
    border: 1px solid rgba(0, 255, 204, 0.15);
    border-radius: 999px;
    font-size: 0.7rem;
    color: #00ffcc;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 1.25rem;
}

.section-title {
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: clamp(1.8rem, 4vw, 2.5rem);
    font-weight: 700;
    margin-bottom: 1rem;
    background: linear-gradient(135deg, #fff 0%, #94a3b8 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.section-desc {
    color: #64748b;
    font-size: 1rem;
}

// =============================================================================
// Features Section
// =============================================================================

.features {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
}

.feature-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.25rem;
    max-width: 960px;
    margin: 0 auto;
}

.card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    padding: 2rem;
    backdrop-filter: blur(8px);
}

.card-icon {
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: 0.8rem;
    color: #00ffcc;
    margin-bottom: 1.25rem;
    opacity: 0.6;
}

.card h3 {
    font-size: 1.1rem;
    font-weight: 600;
    margin-bottom: 0.6rem;
    color: #e2e8f0;
}

.card p {
    color: #64748b;
    font-size: 0.88rem;
    line-height: 1.6;
}

// =============================================================================
// Showcase Section
// =============================================================================

.showcase {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
}

.showcase-panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    max-width: 800px;
    margin: 0 auto;
}

.panel {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    padding: 2.5rem;
}

.panel h3 {
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: 1rem;
    color: #00ffcc;
    margin-bottom: 1rem;
}

.panel p {
    color: #94a3b8;
    font-size: 0.92rem;
    line-height: 1.7;
}

// =============================================================================
// Dot Grid
// =============================================================================

.dots-section {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
}

.dot-grid {
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    gap: 10px;
    max-width: 420px;
    margin: 0 auto;
}

.dot {
    width: 100%;
    aspect-ratio: 1;
    background: linear-gradient(135deg, #00ffcc, #7B2FFF);
    border-radius: 50%;
    opacity: 0;
    transform: scale(0);
}

// =============================================================================
// Stats
// =============================================================================

.stats {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 2rem;
    max-width: 800px;
    margin: 0 auto;
    padding: 6rem 2rem;
    text-align: center;
}

.stat-num {
    display: block;
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: 2.5rem;
    font-weight: 700;
    background: linear-gradient(135deg, #00ffcc, #7B2FFF);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    margin-bottom: 0.5rem;
}

.stat-label {
    font-size: 0.8rem;
    color: #64748b;
    letter-spacing: 0.04em;
}

// =============================================================================
// CTA
// =============================================================================

.cta {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
    text-align: center;
}

.cta-title {
    font-family: "SF Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: clamp(1.8rem, 4vw, 2.5rem);
    font-weight: 700;
    margin-bottom: 1rem;
    background: linear-gradient(135deg, #fff 0%, #00ffcc 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.cta-desc {
    color: #64748b;
    margin-bottom: 2.5rem;
    font-size: 1rem;
}

// =============================================================================
// Footer
// =============================================================================

.footer {
    position: relative;
    z-index: 1;
    padding: 3rem 2rem;
    text-align: center;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.footer p {
    color: #475569;
    font-size: 0.85rem;
}

.accent {
    color: #00ffcc;
}

// =============================================================================
// ANIMATIONS
// =============================================================================

// Hero entrance
.hero {
    @on &.load(name: hero-entrance, duration: 1200ms, delay: 100ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -20px -> 0;
        easing: --ease-out-expo;
        range: 0 to 0.3;
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
        easing: --ease-out-expo;
        range: 0.1 to 0.5;
        opacity: 0 -> 1;
        translate-y: 30px -> 0;
        easing: --smooth;
        range: 0.25 to 0.65;
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
        scale: 0.9 -> 1;
        easing: --ease-out-back;
        range: 0.4 to 0.85;
      }
    @on &.loop(name: orb-drift, duration: 10s, mode: loop, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -25px -> 0;
        translate-x: 0 -> 20px -> 0;
        easing: --ease-in-out-quad;
        translate-y: 0 -> 20px -> 0;
        translate-x: 0 -> -15px -> 0;
        easing: --ease-in-out-quad;
        at: 0.33;
        translate-y: 0 -> -18px -> 0;
        translate-x: 0 -> 22px -> 0;
        easing: --ease-in-out-quad;
        at: 0.66;
      }
    @on &.mouse(name: hero-parallax, axis: both) {
        .orb-1 {
            translate-x: parallax(15px);
            translate-y: parallax(15px);
        }

        .orb-2 {
            translate-x: parallax(10px);
            translate-y: parallax(10px);
        }

        .orb-3 {
            translate-x: parallax(12px);
            translate-y: parallax(12px);
        }
    }
}

// Button hover
.btn-primary {
    @on &.hover(name: btn-hover, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 1.06;
        translate-y: 0 -> -3px;
        easing: --bounce;
      }
}

// Features reveal
.features {
    @on &.scroll(name: features-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 50px -> 0;
        easing: --ease-out-expo;
        stagger: 0.1 first;
      }
}

// Card hover
.card {
    @on &.hover(name: card-lift, duration: 350ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -6px;
        easing: --smooth;
      }
}

// Showcase reveal
.showcase {
    @on &.scroll(name: showcase-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-x: -40px -> 0;
        easing: --ease-out-expo;
        range: 0 to 0.6;
        opacity: 0 -> 1;
        translate-x: 40px -> 0;
        easing: --ease-out-expo;
        range: 0.15 to 0.7;
      }
}

// Panel hover
.panel {
    @on &.hover(name: panel-lift, duration: 350ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -4px;
        easing: --smooth;
      }
}

// Dot grid reveal
.dots-section {
    @on &.scroll(name: dot-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        scale: 0 -> 1;
        opacity: 0 -> 1;
        easing: --ease-out-back;
        stagger: 0.004 grid(10 10) center;
      }
}

// Stats reveal
.stats {
    @on &.scroll(name: stats-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 30px -> 0;
        easing: --ease-out-expo;
        stagger: 0.12 first;
      }
}

// CTA reveal
.cta {
    @on &.scroll(name: cta-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 35px -> 0;
        easing: --ease-out-expo;
        range: 0 to 0.5;
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
        easing: --smooth;
        range: 0.15 to 0.55;
        opacity: 0 -> 1;
        scale: 0.85 -> 1;
        easing: --ease-out-back;
        range: 0.3 to 0.8;
      }
}
