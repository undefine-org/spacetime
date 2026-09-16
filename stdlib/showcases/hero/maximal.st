/// scenario: hero
/// style: maximal
/// school: experimental
/// description: Bold gradient hero with oversized type, decorative grid, and high contrast CTAs.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Launches, tech products, or moments where the brand needs to land like a punch.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-max-*`); this file keeps only
   layout + the entrance choreography. Extend the look by splicing the same
   forms in your own file — don't fork the pattern. */

@template &showcase-hero-maximal($title, $subtitle, $cta) {
    <section class="showcase-hero-max">
        <div class="showcase-hero-max__grid"></div>
        <div class="showcase-hero-max__inner">
            <h1 class="showcase-hero-max__title">`$title`</h1>
            <p class="showcase-hero-max__subtitle">`$subtitle`</p>
            <a class="showcase-hero-max__cta" href="#">`$cta`</a>
        </div>
    </section>
}

.showcase-hero-max {
    --sc-max-surface;
    background: linear-gradient(135deg, #0f172a, #6d28d9);

    position: relative;
    overflow: hidden;
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 100px 32px;
}

.showcase-hero-max__grid {
    position: absolute;
    inset: 0;
    background-image:
        linear-gradient(rgba(255, 255, 255, 0.06) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.06) 1px, transparent 1px);
    background-size: 48px 48px;
    pointer-events: none;
}

.showcase-hero-max__inner {
    position: relative;
    max-width: 820px;
    text-align: center;
}

.showcase-hero-max__title {
    --sc-max-title;
    font-size: clamp(3rem, 9vw, 7rem);
    margin: 0 0 24px 0;

    @on &.visible: --sc-enter;
}

.showcase-hero-max__subtitle {
    --sc-max-hero-subtitle;
    max-width: 560px;
    margin: 0 auto 36px;

    @on &.visible: --sc-enter(16px);
}

.showcase-hero-max__cta {
    --sc-max-hero-cta;

    @on &.visible: --sc-enter(12px);
}
