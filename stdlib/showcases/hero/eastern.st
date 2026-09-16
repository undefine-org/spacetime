/// scenario: hero
/// style: editorial
/// school: eastern-philosophy
/// description: Asymmetric warm hero with serif display, generous "ma" negative space, and a quiet ink-stroke kicker.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Wellness, hospitality, craft, slow-food, or contemplative tech where the brand wants to feel warm and rooted.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-east-*`); this file keeps only
   layout + the entrance choreography. Extend the look by splicing the same
   forms in your own file — don't fork the pattern. */

@template &showcase-hero-eastern($kicker, $title, $subtitle, $cta) {
    <section class="showcase-hero-east">
        <div class="showcase-hero-east__inner">
            <span class="showcase-hero-east__kicker">`$kicker`</span>
            <h1 class="showcase-hero-east__title">`$title`</h1>
            <p class="showcase-hero-east__subtitle">`$subtitle`</p>
            <a class="showcase-hero-east__cta" href="#">`$cta`</a>
        </div>
        <div class="showcase-hero-east__seal" aria-hidden="true"></div>
    </section>
}

.showcase-hero-east {
    --sc-east-surface;

    position: relative;
    padding: 128px 32px 144px 32px;
    overflow: hidden;
}

.showcase-hero-east__inner {
    max-width: 880px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr;
    gap: 28px;
    padding-left: 8%;
}

.showcase-hero-east__kicker {
    --sc-east-hero-kicker;

    @on &.visible: --sc-enter(12px);
}

.showcase-hero-east__title {
    --sc-east-hero-title;
    margin: 0;
    max-width: 18ch;

    @on &.visible: --sc-enter;
}

.showcase-hero-east__subtitle {
    --sc-east-hero-subtitle;
    margin: 0;
    max-width: 52ch;

    @on &.visible: --sc-enter(16px);
}

.showcase-hero-east__cta {
    --sc-east-hero-cta;
    margin-top: 12px;
    align-self: flex-start;

    @on &.visible: --sc-enter(12px);
}

.showcase-hero-east__cta:hover {
    color: #b25a3c;
    border-color: #b25a3c;
}

.showcase-hero-east__seal {
    position: absolute;
    right: 8%;
    bottom: 48px;
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: #b25a3c;
    opacity: 0.85;
}
