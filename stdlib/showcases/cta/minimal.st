/// scenario: cta
/// style: minimal
/// school: minimalism
/// description: Single-row CTA banner with one bold headline and a primary action button.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: End-of-page closers when readers have already absorbed the value prop.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-min-*`); this file keeps only
   layout + the entrance choreography. Extend the look by splicing the same
   forms in your own file — don't fork the pattern. */

@template &showcase-cta-minimal($headline, $cta) {
    <section class="showcase-cta-min">
        <div class="showcase-cta-min__inner">
            <h2 class="showcase-cta-min__headline">`$headline`</h2>
            <a class="showcase-cta-min__btn" href="#">`$cta`</a>
        </div>
    </section>
}

.showcase-cta-min {
    --sc-min-surface;

    background: #e0f2fe;
    padding: 80px 32px;
}

.showcase-cta-min__inner {
    max-width: 960px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 32px;
    flex-wrap: wrap;
}

.showcase-cta-min__headline {
    --sc-min-title;

    font-size: clamp(1.5rem, 3vw, 2.25rem);
    letter-spacing: -0.01em;
    margin: 0;
    flex: 1 1 320px;

    @on &.visible: --sc-enter;
}

.showcase-cta-min__btn {
    --sc-min-cta;

    background: #0369a1;
    padding: 14px 28px;
    font-weight: 500;
    flex: 0 0 auto;

    @on &.visible: --sc-enter;
}
