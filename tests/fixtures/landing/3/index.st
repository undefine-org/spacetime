/// PRISM Records - Electronic Music Label
/// Bold energetic: magenta + electric blue + orange on rich dark

// Easing presets
@version 2026-07-26;

@form easing --smooth { cubic-bezier(0.4, 0, 0.2, 1) }
@form easing --snap { cubic-bezier(0.2, 1.4, 0.4, 1) }

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
    background: #0A0A0F;
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

    @scene(clearColor: #0A0A0F, antialias: true) {
        // Magenta ambient
        @fill glow(color: #FF2D6B, radius: 500px, intensity: 0.2, center: screen, blend: add) {
            @pulse-intensity(base: 0.2, amplitude: 0.08, frequency: 0.3)
            @pulse-radius(base: 500, amplitude: 100, frequency: 0.2)
        }

        // Blue ambient
        @fill glow(color: #2D7FFF, radius: 450px, intensity: 0.18, center: screen, blend: add) {
            @pulse-intensity(base: 0.18, amplitude: 0.07, frequency: 0.25)
            @pulse-radius(base: 450, amplitude: 90, frequency: 0.18)
        }

        // Orange ambient accent
        @fill glow(color: #FF8A2D, radius: 350px, intensity: 0.12, center: screen, blend: add) {
            @pulse-intensity(base: 0.12, amplitude: 0.05, frequency: 0.35)
            @pulse-radius(base: 350, amplitude: 70, frequency: 0.22)
        }

        // White mouse-follow highlight
        @fill glow(color: #ffffff, radius: 180px, intensity: 0.6, center: mouse, blend: add) {
            @pulse-radius(base: 180, amplitude: 25, frequency: 0.8)
        }

        // Magenta mouse-follow accent
        @fill glow(color: #FF2D6B, radius: 220px, intensity: 0.8, center: mouse, blend: add) {
            @pulse-falloff(base: 2.5, amplitude: 0.6, frequency: 0.5)
        }

        // Post-processing
        @distort chromatic(offset: 3px, angle: 30deg)
        @distort ripple(amplitude: 1.5, frequency: 3, speed: 0.5)
        @distort vignette(radius: 0.8, softness: 0.4)
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
    background: rgba(10, 10, 15, 0.7);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.nav-logo {
    font-size: 1rem;
    font-weight: 800;
    letter-spacing: 0.15em;
    color: #FF2D6B;
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
// Hero
// =============================================================================

.hero {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    padding: 2rem;
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
    background: rgba(255, 45, 107, 0.08);
    border: 1px solid rgba(255, 45, 107, 0.25);
    border-radius: 999px;
    font-size: 0.75rem;
    color: #FF2D6B;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin-bottom: 2rem;
}

.hero-title {
    font-size: clamp(3rem, 8vw, 5.5rem);
    font-weight: 800;
    line-height: 1;
    margin-bottom: 1.5rem;
    letter-spacing: -0.03em;
    background: linear-gradient(135deg, #FF2D6B 0%, #2D7FFF 50%, #FF8A2D 100%);
    background-size: 200% auto;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.hero-sub {
    font-size: clamp(1rem, 2vw, 1.15rem);
    color: #94a3b8;
    line-height: 1.7;
    max-width: 480px;
    margin: 0 auto 2.5rem;
    font-weight: 300;
}

.hero-actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
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
    background: linear-gradient(135deg, #FF2D6B 0%, #FF8A2D 100%);
    color: #fff;
    box-shadow: 0 0 30px rgba(255, 45, 107, 0.25), 0 0 60px rgba(255, 138, 45, 0.1);
}

.btn-ghost {
    background: transparent;
    color: #94a3b8;
    border: 1px solid rgba(255, 255, 255, 0.12);
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
    background: rgba(255, 45, 107, 0.06);
    border: 1px solid rgba(255, 45, 107, 0.15);
    border-radius: 999px;
    font-size: 0.7rem;
    color: #FF2D6B;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 1.25rem;
}

.section-title {
    font-size: clamp(1.8rem, 4vw, 2.5rem);
    font-weight: 800;
    margin-bottom: 1rem;
    letter-spacing: -0.02em;
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
// Features
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
    position: relative;
    overflow: hidden;
}

.card-accent {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, #FF2D6B, #2D7FFF, #FF8A2D);
}

.card h3 {
    font-size: 1.1rem;
    font-weight: 700;
    margin-bottom: 0.6rem;
    color: #e2e8f0;
}

.card p {
    color: #64748b;
    font-size: 0.88rem;
    line-height: 1.6;
}

// =============================================================================
// Dot Grid
// =============================================================================

.demo {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
}

.dot-grid {
    display: grid;
    grid-template-columns: repeat(13, 1fr);
    gap: 8px;
    max-width: 500px;
    margin: 0 auto;
}

.dot {
    width: 100%;
    aspect-ratio: 1;
    background: linear-gradient(135deg, #FF2D6B, #2D7FFF);
    border-radius: 50%;
    opacity: 0;
    transform: scale(0);
}

// =============================================================================
// Testimonials
// =============================================================================

.testimonials {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
}

.testimonial-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.5rem;
    max-width: 960px;
    margin: 0 auto;
}

.testimonial {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    padding: 2rem;
}

.testimonial blockquote {
    font-size: 0.95rem;
    color: #cbd5e1;
    line-height: 1.7;
    font-style: italic;
    margin-bottom: 1.25rem;
}

.testimonial-author {
    font-size: 0.8rem;
    color: #FF2D6B;
    font-style: normal;
    letter-spacing: 0.04em;
}

// =============================================================================
// Pricing
// =============================================================================

.pricing {
    position: relative;
    z-index: 1;
    padding: 8rem 2rem;
}

.pricing-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    max-width: 700px;
    margin: 0 auto;
}

.price-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 16px;
    padding: 2.5rem;
    text-align: center;
}

.price-card-featured {
    border-color: rgba(255, 45, 107, 0.3);
    background: rgba(255, 45, 107, 0.04);
}

.price-card h3 {
    font-size: 1.1rem;
    font-weight: 700;
    color: #e2e8f0;
    margin-bottom: 1rem;
}

.price {
    font-size: 2.5rem;
    font-weight: 800;
    margin-bottom: 1.5rem;
    background: linear-gradient(135deg, #FF2D6B, #FF8A2D);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.price span {
    font-size: 1rem;
    font-weight: 400;
}

.price-card ul {
    list-style: none;
    margin-bottom: 2rem;
    text-align: left;
}

.price-card li {
    padding: 0.5rem 0;
    color: #94a3b8;
    font-size: 0.88rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
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
    color: #FF2D6B;
}

// =============================================================================
// ANIMATIONS
// =============================================================================

// Hero entrance - fast and energetic
.hero {
    @on &.load(name: hero-entrance, duration: 1000ms, delay: 50ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -20px -> 0;
        easing: --snap;
        range: 0 to 0.25;
        opacity: 0 -> 1;
        translate-y: 50px -> 0;
        scale: 0.92 -> 1;
        easing: --snap;
        range: 0.05 to 0.45;
        opacity: 0 -> 1;
        translate-y: 30px -> 0;
        easing: --smooth;
        range: 0.2 to 0.55;
        opacity: 0 -> 1;
        translate-y: 25px -> 0;
        easing: --snap;
        range: 0.35 to 0.7;
      }
    @on &.mouse(name: hero-parallax, axis: both) {
        .hero-content {
            translate-x: parallax(12px);
            translate-y: parallax(12px);
        }
    }
}

// Primary button hover
.btn-primary {
    @on &.hover(name: btn-primary-hover, duration: 250ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 1.08;
        translate-y: 0 -> -3px;
        easing: --snap;
      }
}

// Ghost button hover
.btn-ghost {
    @on &.hover(name: btn-ghost-hover, duration: 250ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        scale: 1 -> 1.04;
        easing: --smooth;
      }
}

// Features reveal with stagger
.features {
    @on &.scroll(name: features-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 50px -> 0;
        scale: 0.95 -> 1;
        easing: --snap;
        stagger: 0.08 grid(3 2) center;
      }
}

// Card hover
.card {
    @on &.hover(name: card-lift, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -8px;
        easing: --snap;
      }
}

// Dot grid reveal
.demo {
    @on &.scroll(name: dot-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        scale: 0 -> 1;
        opacity: 0 -> 1;
        easing: --ease-out-back;
        stagger: 0.003 grid(13 13) center;
      }
}

// Testimonials reveal
.testimonials {
    @on &.scroll(name: testimonials-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
        easing: --snap;
        stagger: 0.12 first;
      }
}

// Testimonial hover
.testimonial {
    @on &.hover(name: testimonial-lift, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -5px;
        easing: --smooth;
      }
}

// Pricing reveal
.pricing {
    @on &.scroll(name: pricing-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 50px -> 0;
        scale: 0.93 -> 1;
        easing: --snap;
        stagger: 0.15 first;
      }
}

// Price card hover
.price-card {
    @on &.hover(name: price-lift, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -6px;
        easing: --snap;
      }
}
