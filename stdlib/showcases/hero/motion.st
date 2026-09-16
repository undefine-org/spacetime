/// scenario: hero
/// style: editorial
/// school: motion-poetics
/// description: Scroll-driven kinetic hero — a scored sequence (readouts, title, subtitle, CTA) choreographed by the page's scroll window.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Product launches, conference sites, agency portfolios where the page should feel alive and current.

@import "../_schools.st"

/* Brand from `_schools.st` (`--sc-mtn-*`); the choreography is the
   `--sc-mtn-hero` SCORE FORM — one declarative sequence, consumed by each
   element through `@on &.clip`. Scroll: readouts settle, the title rises,
   the subtitle follows, the CTA lands last. */

@template &showcase-hero-motion($title, $subtitle, $readout_a, $readout_b, $cta) {
    <section class="showcase-hero-mtn">
        <div class="showcase-hero-mtn__grid"></div>
        <div class="showcase-hero-mtn__inner">
            <div class="showcase-hero-mtn__readouts">
                <span class="showcase-hero-mtn__readout">`$readout_a`</span>
                <span class="showcase-hero-mtn__readout">`$readout_b`</span>
            </div>
            <h1 class="showcase-hero-mtn__title">`$title`</h1>
            <p class="showcase-hero-mtn__subtitle">`$subtitle`</p>
            <a class="showcase-hero-mtn__cta" href="#">
                <span class="showcase-hero-mtn__cta-label">`$cta`</span>
                <span class="showcase-hero-mtn__cta-glyph">&rarr;</span>
            </a>
        </div>
    </section>
}

.showcase-hero-mtn {
    --sc-mtn-surface;

    position: relative;
    overflow: hidden;
    min-height: 92vh;
    display: flex;
    align-items: center;
    padding: 64px 32px;

    /* The score: this section's scroll window hands each subject its slice
       (form declared in _schools.st — retune the sequence there, once). */
    @score &.scroll { --sc-mtn-hero; }
}

.showcase-hero-mtn__grid {
    position: absolute;
    inset: 0;
    background-image:
        linear-gradient(to right, rgba(241, 245, 249, 0.06) 1px, transparent 1px),
        linear-gradient(to bottom, rgba(241, 245, 249, 0.06) 1px, transparent 1px);
    background-size: 64px 64px;
    pointer-events: none;
}

.showcase-hero-mtn__inner {
    position: relative;
    width: 100%;
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.showcase-hero-mtn__readouts {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
}

.showcase-hero-mtn__readout {
    --sc-mtn-readout;
    font-size: 0.7rem;
    padding: 4px 10px;
    border: 1px solid rgba(241, 245, 249, 0.12);
    border-radius: 999px;
}

/* Each subject consumes its score window with the shared entrance motion. */
.showcase-hero-mtn__readouts { @on &.clip: --sc-enter(8px); }

.showcase-hero-mtn__title {
    --sc-mtn-title;
    font-size: clamp(3rem, 8vw, 7rem);
    letter-spacing: -0.04em;
    line-height: 0.95;
    max-width: 18ch;
    margin: 16px 0 0 0;

    @on &.clip: --sc-enter(32px);
}

.showcase-hero-mtn__subtitle {
    --sc-mtn-readout;
    font-family: "Inter", system-ui, sans-serif;
    font-size: clamp(1rem, 1.4vw, 1.25rem);
    letter-spacing: normal;
    text-transform: none;
    max-width: 60ch;
    line-height: 1.5;
    margin: 0;

    @on &.clip: --sc-enter(20px);
}

.showcase-hero-mtn__cta {
    --sc-mtn-cta;
    margin-top: 16px;
    align-self: flex-start;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 0.85rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    transition: background 0.2s ease, color 0.2s ease;

    @on &.clip: --sc-enter(12px);
}

.showcase-hero-mtn__cta:hover {
    background: #f5f5f5;
    color: #0a0a0c;
}

.showcase-hero-mtn__cta-glyph {
    font-size: 1.1rem;
}
