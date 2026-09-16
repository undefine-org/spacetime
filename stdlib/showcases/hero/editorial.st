/// scenario: hero
/// style: editorial
/// school: minimalism
/// description: Magazine-style hero with eyebrow kicker, large serif headline, and dual CTAs.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Story-driven pages or premium brands where authority matters as much as conversion.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-min-*` / `--sc-min-ed-*`); this
   file keeps only layout + the entrance choreography. Extend the look by
   splicing the same forms in your own file — don't fork the pattern. */

@template &showcase-hero-editorial($eyebrow, $title, $subtitle, $cta_primary, $cta_secondary) {
    <section class="showcase-hero-ed">
        <div class="showcase-hero-ed__inner">
            <span class="showcase-hero-ed__eyebrow">`$eyebrow`</span>
            <h1 class="showcase-hero-ed__title">`$title`</h1>
            <p class="showcase-hero-ed__subtitle">`$subtitle`</p>
            <div class="showcase-hero-ed__ctas">
                <a class="showcase-hero-ed__cta showcase-hero-ed__cta--primary" href="#">`$cta_primary`</a>
                <a class="showcase-hero-ed__cta showcase-hero-ed__cta--secondary" href="#">`$cta_secondary`</a>
            </div>
        </div>
    </section>
}

.showcase-hero-ed {
    --sc-min-ed-surface;

    display: grid;
    place-items: center;
    min-height: 100vh;
    padding: 80px 32px;
}

.showcase-hero-ed__inner {
    max-width: 760px;
    text-align: center;
}

.showcase-hero-ed__eyebrow {
    --sc-min-ed-eyebrow;

    display: inline-block;
    margin-bottom: 24px;

    @on &.visible: --sc-enter(12px);
}

.showcase-hero-ed__title {
    --sc-min-ed-title;
    margin: 0 0 28px 0;

    @on &.visible: --sc-enter;
}

.showcase-hero-ed__subtitle {
    --sc-min-ed-subtitle;
    margin: 0 0 40px 0;

    @on &.visible: --sc-enter(16px);
}

.showcase-hero-ed__ctas {
    display: flex;
    gap: 16px;
    justify-content: center;
    flex-wrap: wrap;

    @on &.visible: --sc-enter(16px);
}

.showcase-hero-ed__cta {
    --sc-min-ed-cta;
}

.showcase-hero-ed__cta--primary {
    --sc-min-ed-cta-primary;
}

.showcase-hero-ed__cta--secondary {
    --sc-min-ed-cta-secondary;
}
