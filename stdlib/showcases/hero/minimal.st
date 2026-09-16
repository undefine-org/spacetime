/// scenario: hero
/// style: minimal
/// school: minimalism
/// description: Centered headline, subhead, and a single primary CTA on a calm background.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Marketing pages where the offer is the page; you want the headline to do the work.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-min-*`); this file keeps only
   layout + the entrance choreography. Extend the look by splicing the same
   forms in your own file — don't fork the pattern. */

@template &showcase-hero-minimal($title, $subtitle, $cta) {
    <section class="showcase-hero-min">
        <div class="showcase-hero-min__inner">
            <h1 class="showcase-hero-min__title">`$title`</h1>
            <p class="showcase-hero-min__subtitle">`$subtitle`</p>
            <a class="showcase-hero-min__cta" href="#">`$cta`</a>
        </div>
    </section>
}

.showcase-hero-min {
    --sc-min-surface;

    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 96px 24px;
}

.showcase-hero-min__inner {
    max-width: 640px;
    text-align: center;
}

.showcase-hero-min__title {
    --sc-min-title;
    margin: 0 0 16px 0;

    @on &.visible: --sc-enter;
}

.showcase-hero-min__subtitle {
    --sc-min-subtitle;
    font-size: clamp(1.05rem, 1.8vw, 1.25rem);
    margin: 0 0 32px 0;

    @on &.visible: --sc-enter(16px);
}

.showcase-hero-min__cta {
    --sc-min-cta;

    @on &.visible: --sc-enter(12px);
}
