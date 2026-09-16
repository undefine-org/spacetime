@import "stdlib/3d"
@import "stdlib/showcases/_schools.st"

/* ============================================================================
   AURA — a product launch site.

   The shape a studio gets paid for: a pinned WebGL particle field that morphs
   through four states as the page scrolls (sphere -> torus knot -> galaxy ->
   wave grid, one declarative @scroll-3d binding), a variable-font headline
   revealed glyph by glyph, a marquee spec rail, an animated weight study, a
   monospace spec table, and a magnetic close.

   No scroll listeners, no animation libraries, no JavaScript in sight — every
   line below is Spacetime.
   ============================================================================ */

@font("Aspekta") {
    src: url("/fonts/AspektaVF.woff2") /* BUG-353: serve rewrites the CSS base; absolute path works in both modes */ format("woff2-variations");
    font-weight: 50 1000;
    font-style: normal;
    font-display: swap;
}

body {
    margin: 0;
    background: #05060d;
    color: #eaeaf2;
    font-family: "Aspekta", "Inter", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
}

::selection { background: #8ab4ff; color: #05060d; }

/* --------------------------------------------------------------------------
   The field. One canvas, pinned behind everything; page scroll drives the
   morph 0 -> 3 across the four story beats below.
   -------------------------------------------------------------------------- */

canvas.field {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 0;
    background: radial-gradient(circle at 50% 42%, #12102e 0%, #07071a 48%, #05060d 100%);

    @stage(camZ: 6.5, tone: "aces", exposure: 0.9) {
        @particles(count: 30000, size: 0.018, radius: 2.2, color: #8ab4ff, spin: 0.05)
        @scroll-3d(drive: "morph", to: 3, ease: 0.06)
        @post(bloomStrength: 0.55, bloomRadius: 0.45, bloomThreshold: 0.35)
    }
}

/* Sections float over the field; the LAST ones get a solid ground so the
   table and footer stay legible. */
.page { position: relative; z-index: 1; }

/* --------------------------------------------------------------------------
   Nav — quiet, fixed, one action.
   -------------------------------------------------------------------------- */

.nav {
    position: fixed;
    top: 0; left: 0; right: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 22px 40px;
}

.nav__brand {
    font-weight: 800;
    font-size: 15px;
    letter-spacing: 0.32em;
    color: #f2f4fc;
}

.nav__links { display: flex; gap: 28px; align-items: center; }

.nav__link {
    font-size: 12px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.55);
}

.nav__cta {
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #05060d;
    background: #eaeaf2;
    padding: 10px 20px;
    border-radius: 999px;
    font-weight: 700;
}

/* --------------------------------------------------------------------------
   Hero — the word itself is the spectacle.
   -------------------------------------------------------------------------- */

.hero {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0 40px;
}

.hero__eyebrow {
    font-size: 12px;
    letter-spacing: 0.34em;
    text-transform: uppercase;
    color: #8ab4ff;
    margin: 0 0 3vh 0;

    @reveal(split: "words", stagger: 40, duration: 600)
}

.hero__title {
    font-size: clamp(6rem, 21vw, 21rem);
    font-weight: 1000;
    line-height: 0.88;
    letter-spacing: -0.03em;
    color: #f5f6ff;
    margin: 0;

    @reveal(split: "chars", trigger: load, stagger: 55, duration: 900)
}

.hero__sub {
    max-width: 34ch;
    font-size: clamp(1.05rem, 1.6vw, 1.35rem);
    font-weight: 350;
    line-height: 1.5;
    color: rgba(234, 234, 242, 0.78);
    margin: 4vh 0 0 0;
    /* The sphere is bright enough to swallow body copy — a soft shadow keeps
       the words theirs without dimming the field. */
    text-shadow: 0 1px 18px rgba(5, 6, 13, 0.95), 0 0 6px rgba(5, 6, 13, 0.9);

    @reveal(split: "words", trigger: load, stagger: 18, duration: 700, distance: 60%)
}

.hero__row {
    display: flex;
    align-items: center;
    gap: 28px;
    margin-top: 5vh;
}

.hero__cta {
    display: inline-block;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #05060d;
    background: #f5f6ff;
    padding: 18px 40px;
    border-radius: 999px;

    @magnetic(strength: 0.35, radius: 140, lerp: 0.14)
}

.hero__note {
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.45);
    text-shadow: 0 1px 12px rgba(5, 6, 13, 0.9);
}

.hero__cue {
    position: absolute;
    bottom: 34px;
    left: 40px;
    font-size: 11px;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.45);
    animation: cue-drift 2.6s ease-in-out infinite;
}

@keyframes cue-drift {
    0%, 100% { transform: translateY(0); opacity: 0.45; }
    50% { transform: translateY(8px); opacity: 1; }
}

/* --------------------------------------------------------------------------
   Marquee — the spec rail that never stops.
   -------------------------------------------------------------------------- */

.rail {
    overflow: hidden;
    white-space: nowrap;
    border-top: 1px solid rgba(138, 180, 255, 0.18);
    border-bottom: 1px solid rgba(138, 180, 255, 0.18);
    padding: 18px 0;
    background: rgba(5, 6, 13, 0.55);
    backdrop-filter: blur(6px);
}

.rail__inner {
    display: inline-block;
    animation: rail-scroll 28s linear infinite;
}

.rail__item {
    display: inline-block;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.7);
    padding: 0 34px;
}

.rail__dot { color: #8ab4ff; padding: 0; }

@keyframes rail-scroll {
    from { transform: translateX(0); }
    to { transform: translateX(-50%); }
}

/* --------------------------------------------------------------------------
   Story — three beats, one per particle state.
   -------------------------------------------------------------------------- */

.beat {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0 40px;
    max-width: 1200px;
}

.beat--right { margin-left: auto; text-align: right; align-items: flex-end; }

.beat__index {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.3em;
    color: #8ab4ff;
    margin: 0 0 2.5vh 0;
}

.beat__title {
    font-size: clamp(2.8rem, 6.5vw, 5.6rem);
    font-weight: 850;
    line-height: 1.02;
    letter-spacing: -0.025em;
    color: #f5f6ff;
    margin: 0;
    max-width: 14ch;

    @reveal(split: "words", stagger: 70, duration: 800)
}

.beat__body {
    max-width: 42ch;
    font-size: 1.05rem;
    font-weight: 350;
    line-height: 1.65;
    color: rgba(234, 234, 242, 0.68);
    margin: 3vh 0 0 0;

    @reveal(split: "words", stagger: 8, duration: 500, distance: 40%)
}

/* --------------------------------------------------------------------------
   Weight study — the variable font performing itself.
   -------------------------------------------------------------------------- */

.study {
    min-height: 80vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 0 40px;
}

.study__glyph {
    font-size: clamp(10rem, 30vw, 26rem);
    line-height: 1;
    color: #f5f6ff;
    animation: weight-breathe 5s ease-in-out infinite;
}

@keyframes weight-breathe {
    0%, 100% { font-weight: 100; }
    50% { font-weight: 1000; }
}

.study__caption {
    font-size: 12px;
    letter-spacing: 0.28em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.5);
    margin-top: 2vh;
}

/* --------------------------------------------------------------------------
   Specs — a solid ground, a monospace table, numbers that hold still.
   -------------------------------------------------------------------------- */

.specs {
    background: #07070f;
    border-top: 1px solid rgba(138, 180, 255, 0.14);
    padding: 14vh 40px;
}

.specs__inner { max-width: 1200px; margin: 0 auto; }

.specs__title {
    font-size: clamp(2.2rem, 4.5vw, 3.8rem);
    font-weight: 850;
    letter-spacing: -0.02em;
    color: #f5f6ff;
    margin: 0 0 8vh 0;

    @reveal(split: "words", stagger: 60, duration: 700)
}

.specs__grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1px;
    background: rgba(138, 180, 255, 0.14);
    border: 1px solid rgba(138, 180, 255, 0.14);
}

.spec {
    background: #07070f;
    padding: 40px 32px;
}

.spec__num {
    font-size: clamp(2.6rem, 5vw, 4.2rem);
    font-weight: 900;
    letter-spacing: -0.03em;
    color: #f5f6ff;
    font-variant-numeric: tabular-nums;
    line-height: 1;
}

.spec__unit { font-size: 1.1rem; font-weight: 500; color: #8ab4ff; }

.spec__label {
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.5);
    margin-top: 14px;
}

.specs__table {
    margin-top: 10vh;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 13px;
    color: rgba(234, 234, 242, 0.72);
}

.specs__row {
    display: flex;
    justify-content: space-between;
    padding: 16px 0;
    border-bottom: 1px solid rgba(234, 234, 242, 0.1);
}

.specs__key { letter-spacing: 0.08em; text-transform: uppercase; color: rgba(234, 234, 242, 0.45); }

/* --------------------------------------------------------------------------
   Close — the biggest type on the page, and one magnetic promise.
   -------------------------------------------------------------------------- */

.close {
    min-height: 90vh;
    background: #07070f;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 0 40px;
}

.close__title {
    font-size: clamp(4rem, 12vw, 12rem);
    font-weight: 1000;
    line-height: 0.92;
    letter-spacing: -0.03em;
    color: #f5f6ff;
    margin: 0;

    @reveal(split: "chars", stagger: 45, duration: 800)
}

.close__cta {
    display: inline-block;
    margin-top: 7vh;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #05060d;
    background: #f5f6ff;
    padding: 20px 52px;
    border-radius: 999px;

    @magnetic(strength: 0.4, radius: 180, lerp: 0.14)
}

.footer {
    background: #07070f;
    border-top: 1px solid rgba(234, 234, 242, 0.08);
    padding: 28px 40px;
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.35);
}

/* --------------------------------------------------------------------------
   Markup — the page IS the copy.
   -------------------------------------------------------------------------- */

<main class="page">
    <nav class="nav">
        <span class="nav__brand">AURA</span>
        <div class="nav__links">
            <span class="nav__link">Listen</span>
            <span class="nav__link">Design</span>
            <span class="nav__link">Specs</span>
            <span class="nav__cta">Reserve</span>
        </div>
    </nav>

    <section class="hero">
        <p class="hero__eyebrow">Aura One — spatial audio</p>
        <h1 class="hero__title">AURA</h1>
        <p class="hero__sub">Thirty-two drivers. One continuous field of sound. The first headphone that treats the room around you as an instrument.</p>
        <div class="hero__row">
            <span class="hero__cta">Reserve yours</span>
            <span class="hero__note">Ships spring — from $549</span>
        </div>
        <span class="hero__cue">Scroll — the field listens</span>
    </section>

    <div class="rail">
        <div class="rail__inner">
            <span class="rail__item">32 drivers</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">Head tracking</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">40-hour battery</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">Lossless over air</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">24-bit 96 kHz</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">32 drivers</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">Head tracking</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">40-hour battery</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">Lossless over air</span><span class="rail__item rail__dot">·</span>
            <span class="rail__item">24-bit 96 kHz</span><span class="rail__item rail__dot">·</span>
        </div>
    </div>

    <section class="beat">
        <p class="beat__index">01 — Place</p>
        <h2 class="beat__title">Every voice in its place.</h2>
        <p class="beat__body">A sphere of sound, mapped to the millimetre. Each instrument holds a fixed point in space while you move through it — the mix stops living inside your head.</p>
    </section>

    <section class="beat beat--right">
        <p class="beat__index">02 — Motion</p>
        <h2 class="beat__title">A field that follows you.</h2>
        <p class="beat__body">Turn your head and the world stays put. Head tracking at one thousand hertz keeps the stage anchored to the room, not to your skull.</p>
    </section>

    <section class="beat">
        <p class="beat__index">03 — Depth</p>
        <h2 class="beat__title">An ocean of signal.</h2>
        <p class="beat__body">Lossless over air, end to end. Thirty-two drivers per ear resolve detail that used to be the exclusive property of rooms with very expensive chairs.</p>
    </section>

    <section class="study">
        <div class="study__glyph">Aa</div>
        <p class="study__caption">One typeface — every weight, alive</p>
    </section>

    <section class="specs">
        <div class="specs__inner">
            <h2 class="specs__title">Numbers that hold still.</h2>
            <div class="specs__grid">
                <div class="spec">
                    <div class="spec__num">32<span class="spec__unit">×</span></div>
                    <div class="spec__label">Drivers per ear</div>
                </div>
                <div class="spec">
                    <div class="spec__num">40<span class="spec__unit">h</span></div>
                    <div class="spec__label">Battery, tracked</div>
                </div>
                <div class="spec">
                    <div class="spec__num">96<span class="spec__unit">kHz</span></div>
                    <div class="spec__label">Lossless, wireless</div>
                </div>
            </div>
            <div class="specs__table">
                <div class="specs__row"><span class="specs__key">Transducers</span><span>32 balanced-armature / ear</span></div>
                <div class="specs__row"><span class="specs__key">Tracking</span><span>1000 Hz, six-axis</span></div>
                <div class="specs__row"><span class="specs__key">Codec</span><span>Aura Link lossless, 24-bit / 96 kHz</span></div>
                <div class="specs__row"><span class="specs__key">Battery</span><span>40 h tracked, 60 h stereo</span></div>
                <div class="specs__row"><span class="specs__key">Weight</span><span>298 g, magnesium</span></div>
            </div>
        </div>
    </section>

    <section class="close">
        <h2 class="close__title">Hear everything.</h2>
        <span class="close__cta">Reserve Aura One</span>
    </section>

    <footer class="footer">
        <span>Aura — a Spacetime showcase</span>
        <span>Set in Aspekta</span>
    </footer>
</main>

<canvas class="field"></canvas>
