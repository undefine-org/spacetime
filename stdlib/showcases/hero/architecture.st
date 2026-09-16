/// scenario: hero
/// style: editorial
/// school: information-architecture
/// description: Twelve-column hero with single headline + three metric callouts (number / unit / label) on a restrained palette.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Annual reports, data-rich landing pages, Pentagram-style brands where the metrics ARE the story.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-arch-*`); this file keeps only
   layout + the entrance choreography. Extend the look by splicing the same
   forms in your own file — don't fork the pattern. */

@template &showcase-hero-architecture($title, $metric_a_value, $metric_a_unit, $metric_a_label, $metric_b_value, $metric_b_unit, $metric_b_label, $metric_c_value, $metric_c_unit, $metric_c_label) {
    <section class="showcase-hero-arch">
        <div class="showcase-hero-arch__inner">
            <h1 class="showcase-hero-arch__title">`$title`</h1>
            <ul class="showcase-hero-arch__metrics">
                <li class="showcase-hero-arch__metric">
                    <span class="showcase-hero-arch__metric-number">`$metric_a_value`</span>
                    <span class="showcase-hero-arch__metric-unit">`$metric_a_unit`</span>
                    <span class="showcase-hero-arch__metric-label">`$metric_a_label`</span>
                </li>
                <li class="showcase-hero-arch__metric">
                    <span class="showcase-hero-arch__metric-number">`$metric_b_value`</span>
                    <span class="showcase-hero-arch__metric-unit">`$metric_b_unit`</span>
                    <span class="showcase-hero-arch__metric-label">`$metric_b_label`</span>
                </li>
                <li class="showcase-hero-arch__metric">
                    <span class="showcase-hero-arch__metric-number">`$metric_c_value`</span>
                    <span class="showcase-hero-arch__metric-unit">`$metric_c_unit`</span>
                    <span class="showcase-hero-arch__metric-label">`$metric_c_label`</span>
                </li>
            </ul>
        </div>
    </section>
}

.showcase-hero-arch {
    --sc-arch-hero-surface;
    padding: 96px 24px;
}

.showcase-hero-arch__inner {
    max-width: 1120px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    gap: 24px;
}

.showcase-hero-arch__title {
    --sc-arch-hero-title;
    grid-column: 1 / span 12;
    margin: 0 0 64px 0;

    @on &.visible: --sc-enter;
}

.showcase-hero-arch__metrics {
    grid-column: 1 / span 12;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0;
    list-style: none;
    margin: 0;
    padding: 0;
}

.showcase-hero-arch__metric {
    --sc-arch-metric;
    border-top-color: #d8d8d2;
    padding-top: 24px;

    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding-right: 24px;

    @on &.visible: --sc-enter(16px);
}

.showcase-hero-arch__metric-number {
    --sc-arch-hero-metric-number;
}

.showcase-hero-arch__metric-unit {
    --sc-arch-hero-metric-meta;
    margin-top: 8px;
}

.showcase-hero-arch__metric-label {
    --sc-arch-hero-metric-meta;
    font-size: 0.7rem;
    margin-top: 4px;
    max-width: 22ch;
}
