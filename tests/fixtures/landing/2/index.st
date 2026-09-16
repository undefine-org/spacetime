/// Atelier Mura - Luxury Architecture Firm
/// Minimal elegant: warm off-white + charcoal + gold accent

// Easing presets
@version 2026-07-26;

@form easing --smooth { cubic-bezier(0.4, 0, 0.2, 1) }
@form easing --elegant { cubic-bezier(0.22, 1, 0.36, 1) }

// =============================================================================
// Reset & Base
// =============================================================================

*, *::before, *::after {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: "Georgia", "Times New Roman", "Palatino", serif;
    background: #FAF8F5;
    color: #1A1A1A;
    overflow-x: hidden;
    line-height: 1.7;
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

    @scene(clearColor: #FAF8F5, antialias: true) {
        // Warm gold ambient glow
        @fill glow(color: #C9A96E, radius: 600px, intensity: 0.08, center: screen, blend: add) {
            @pulse-intensity(base: 0.08, amplitude: 0.03, frequency: 0.15)
            @pulse-radius(base: 600, amplitude: 100, frequency: 0.1)
        }

        // Gentle mouse-follow warmth
        @fill glow(color: #C9A96E, radius: 300px, intensity: 0.12, center: mouse, blend: add) {
            @pulse-radius(base: 300, amplitude: 40, frequency: 0.3)
        }

        // Vignette only - no chromatic for elegance
        @distort vignette(radius: 0.9, softness: 0.5)
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
    gap: 2.5rem;
    padding: 1.5rem 3rem;
    background: rgba(250, 248, 245, 0.8);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid rgba(26, 26, 26, 0.06);
}

.nav-logo {
    font-size: 0.75rem;
    font-weight: 400;
    letter-spacing: 0.25em;
    color: #1A1A1A;
    text-decoration: none;
    margin-right: auto;
}

.nav-link {
    font-size: 0.78rem;
    color: #4A4A4A;
    text-decoration: none;
    letter-spacing: 0.06em;
}

// =============================================================================
// Hero
// =============================================================================

.hero {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 2rem 3rem;
}

.hero-content {
    position: relative;
    z-index: 1;
    text-align: center;
    max-width: 700px;
}

.hero-tag {
    display: inline-block;
    font-size: 0.7rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #C9A96E;
    margin-bottom: 2.5rem;
}

.hero-title {
    font-size: clamp(2.5rem, 6vw, 4.5rem);
    font-weight: 400;
    line-height: 1.1;
    margin-bottom: 2rem;
    color: #1A1A1A;
    letter-spacing: -0.02em;
}

.hero-sub {
    font-size: 1.05rem;
    color: #4A4A4A;
    line-height: 1.8;
    max-width: 500px;
    margin: 0 auto 3rem;
    font-weight: 400;
}

.scroll-hint {
    position: absolute;
    bottom: 3rem;
    left: 50%;
    transform: translateX(-50%);
}

.scroll-line {
    width: 1px;
    height: 60px;
    background: linear-gradient(180deg, #C9A96E, transparent);
}

// =============================================================================
// Buttons
// =============================================================================

.btn {
    display: inline-block;
    padding: 0.85rem 2.5rem;
    font-family: "Georgia", "Times New Roman", "Palatino", serif;
    font-size: 0.85rem;
    letter-spacing: 0.08em;
    text-decoration: none;
    border: 1px solid #1A1A1A;
    color: #1A1A1A;
    background: transparent;
    cursor: pointer;
}

// =============================================================================
// Section Common
// =============================================================================

.section-head {
    text-align: center;
    margin-bottom: 4rem;
    max-width: 500px;
    margin-left: auto;
    margin-right: auto;
}

.tag {
    display: inline-block;
    font-size: 0.65rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #C9A96E;
    margin-bottom: 1.25rem;
}

.section-title {
    font-size: clamp(1.6rem, 3.5vw, 2.2rem);
    font-weight: 400;
    color: #1A1A1A;
    letter-spacing: -0.01em;
}

// =============================================================================
// Philosophy
// =============================================================================

.philosophy {
    position: relative;
    z-index: 1;
    padding: 8rem 3rem;
    max-width: 680px;
    margin: 0 auto;
}

.philosophy-content {
    text-align: center;
}

.philosophy .section-title {
    margin-bottom: 2.5rem;
}

.philosophy-text {
    font-size: 1.05rem;
    color: #4A4A4A;
    line-height: 1.9;
    margin-bottom: 1.5rem;
}

// =============================================================================
// Projects
// =============================================================================

.projects {
    position: relative;
    z-index: 1;
    padding: 8rem 3rem;
}

.project-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    max-width: 900px;
    margin: 0 auto;
}

.project-card {
    background: #fff;
    border: 1px solid rgba(26, 26, 26, 0.08);
    padding: 0;
    overflow: hidden;
}

.project-img {
    width: 100%;
    height: 240px;
    background: linear-gradient(135deg, #E8E4DF, #D4CFC8);
}

.project-card h3 {
    font-size: 1.1rem;
    font-weight: 400;
    color: #1A1A1A;
    padding: 1.5rem 1.5rem 0.5rem;
}

.project-card p {
    font-size: 0.88rem;
    color: #4A4A4A;
    line-height: 1.7;
    padding: 0 1.5rem 1.5rem;
}

// =============================================================================
// Process
// =============================================================================

.process {
    position: relative;
    z-index: 1;
    padding: 8rem 3rem;
}

.process-steps {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 3rem;
    max-width: 900px;
    margin: 0 auto;
}

.step {
    text-align: center;
    padding: 2rem 1.5rem;
}

.step-num {
    display: block;
    font-size: 0.7rem;
    letter-spacing: 0.15em;
    color: #C9A96E;
    margin-bottom: 1.5rem;
}

.step h3 {
    font-size: 1.2rem;
    font-weight: 400;
    color: #1A1A1A;
    margin-bottom: 1rem;
}

.step p {
    font-size: 0.9rem;
    color: #4A4A4A;
    line-height: 1.8;
}

// =============================================================================
// CTA
// =============================================================================

.cta {
    position: relative;
    z-index: 1;
    padding: 8rem 3rem;
    text-align: center;
    background: #1A1A1A;
    color: #FAF8F5;
}

.cta-title {
    font-size: clamp(1.6rem, 3.5vw, 2.2rem);
    font-weight: 400;
    color: #FAF8F5;
    margin-bottom: 1rem;
}

.cta-desc {
    color: #8A8A8A;
    margin-bottom: 2.5rem;
    font-size: 0.95rem;
    line-height: 1.8;
    max-width: 480px;
    margin-left: auto;
    margin-right: auto;
    margin-bottom: 2.5rem;
}

.cta .btn {
    border-color: #C9A96E;
    color: #C9A96E;
}

// =============================================================================
// Footer
// =============================================================================

.footer {
    position: relative;
    z-index: 1;
    padding: 3rem;
    text-align: center;
    border-top: 1px solid rgba(26, 26, 26, 0.08);
}

.footer p {
    color: #8A8A8A;
    font-size: 0.8rem;
    letter-spacing: 0.04em;
}

.accent {
    color: #C9A96E;
}

// =============================================================================
// ANIMATIONS
// =============================================================================

// Hero entrance - slow and graceful
.hero {
    @on &.load(name: hero-entrance, duration: 1800ms, delay: 200ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -15px -> 0;
        easing: --elegant;
        range: 0 to 0.3;
        opacity: 0 -> 1;
        translate-y: 30px -> 0;
        easing: --elegant;
        range: 0.1 to 0.5;
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
        easing: --smooth;
        range: 0.3 to 0.65;
        opacity: 0 -> 1;
        translate-y: 15px -> 0;
        easing: --elegant;
        range: 0.45 to 0.85;
        opacity: 0 -> 1;
        easing: --smooth;
        range: 0.7 to 1;
      }
    @on &.mouse(name: hero-parallax, axis: both) {
        .hero-content {
            translate-x: parallax(8px);
            translate-y: parallax(8px);
        }
    }
}

// Scroll hint pulse
.scroll-hint {
    @on &.loop(name: scroll-pulse, duration: 2s, mode: loop, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0.3 -> 1 -> 0.3;
        easing: --ease-in-out-quad;
      }
}

// Button hover
.btn {
    @on &.hover(name: btn-hover, duration: 400ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -2px;
        easing: --elegant;
      }
}

// Philosophy reveal
.philosophy {
    @on &.scroll(name: philosophy-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -10px -> 0;
        easing: --elegant;
        range: 0 to 0.3;
        opacity: 0 -> 1;
        translate-y: 25px -> 0;
        easing: --elegant;
        range: 0.1 to 0.5;
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
        easing: --smooth;
        stagger: 0.15 first;
      }
}

// Project cards reveal
.projects {
    @on &.scroll(name: projects-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
        easing: --elegant;
        stagger: 0.12 first;
      }
}

// Project card hover
.project-card {
    @on &.hover(name: project-lift, duration: 400ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -4px;
        easing: --elegant;
      }
}

// Process steps reveal
.process {
    @on &.scroll(name: process-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 30px -> 0;
        easing: --elegant;
        stagger: 0.15 first;
      }
}

// CTA reveal
.cta {
    @on &.scroll(name: cta-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 25px -> 0;
        easing: --elegant;
        range: 0 to 0.5;
        opacity: 0 -> 1;
        translate-y: 15px -> 0;
        easing: --smooth;
        range: 0.15 to 0.55;
        opacity: 0 -> 1;
        easing: --elegant;
        range: 0.3 to 0.7;
      }
}
