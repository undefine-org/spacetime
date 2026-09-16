/// scenario: pricing
/// style: interactive
/// school: information-architecture
/// description: Three-tier pricing with a live monthly/annual toggle — one signal drives the prices, the savings badge, and the period label at once.
/// dependencies: none
/// screenshot: (pending)
/// useWhen: Any plan comparison where the billing period changes the numbers; the toggle is the pattern, not decoration.

@import "../_schools.st"

/* One signal, four consequences. `$annual` flips the three prices, the period
   suffix, and the savings badge — no handler wiring per element, no derived
   state to keep in sync. This is the pattern's whole point: a pricing table is
   a DERIVED VIEW of one boolean. */

@template &showcase-pricing-interactive($headline, $tier_a, $tier_b, $tier_c) {
    <section class="showcase-pricing-int">
        <div class="showcase-pricing-int__inner">
            <h2 class="showcase-pricing-int__title">`$headline`</h2>

            <div class="showcase-pricing-int__switch">
                <button class="showcase-pricing-int__opt" data-annual="false">Monthly</button>
                <button class="showcase-pricing-int__opt" data-annual="true">Annual</button>
                <span class="showcase-pricing-int__save">Save 20%</span>
            </div>

            <div class="showcase-pricing-int__grid">
                <article class="showcase-pricing-int__card">
                    <p class="showcase-pricing-int__tier">`$tier_a`</p>
                    <p class="showcase-pricing-int__price">
                        <span class="showcase-pricing-int__amt showcase-pricing-int__amt--a"></span>
                        <span class="showcase-pricing-int__per"></span>
                    </p>
                    <p class="showcase-pricing-int__note">For trying things out.</p>
                </article>
                <article class="showcase-pricing-int__card showcase-pricing-int__card--lead">
                    <p class="showcase-pricing-int__tier">`$tier_b`</p>
                    <p class="showcase-pricing-int__price">
                        <span class="showcase-pricing-int__amt showcase-pricing-int__amt--b"></span>
                        <span class="showcase-pricing-int__per"></span>
                    </p>
                    <p class="showcase-pricing-int__note">For teams that ship.</p>
                </article>
                <article class="showcase-pricing-int__card">
                    <p class="showcase-pricing-int__tier">`$tier_c`</p>
                    <p class="showcase-pricing-int__price">
                        <span class="showcase-pricing-int__amt showcase-pricing-int__amt--c"></span>
                        <span class="showcase-pricing-int__per"></span>
                    </p>
                    <p class="showcase-pricing-int__note">For everything else.</p>
                </article>
            </div>
        </div>
    </section>
}

@data inline $annual : "false";

.showcase-pricing-int {
    --sc-arch-surface;
    display: flex;
    align-items: center;
    min-height: 92vh;
    padding: 96px 32px;
}

.showcase-pricing-int__inner {
    max-width: 1080px;
    margin: 0 auto;
    width: 100%;
}

.showcase-pricing-int__title {
    --sc-arch-title;
    text-align: center;
    margin: 0 0 32px 0;
}

.showcase-pricing-int__switch {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    padding-bottom: 48px;
}

.showcase-pricing-int__opt {
    font: inherit;
    font-size: 14px;
    padding: 10px 22px;
    border: 1px solid rgba(20, 20, 26, 0.18);
    background: transparent;
    border-radius: 999px;
    cursor: pointer;
    transition: background 200ms ease, color 200ms ease;
}

.showcase-pricing-int__save {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 6px 12px;
    border-radius: 999px;
    background: rgba(46, 125, 90, 0.12);
    color: #2e7d5a;
    opacity: 0;
    transition: opacity 260ms ease;
}

.showcase-pricing-int__grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 20px;
}

.showcase-pricing-int__card {
    padding: 32px 28px;
    border: 1px solid rgba(20, 20, 26, 0.12);
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.6);
}

.showcase-pricing-int__card--lead {
    border-color: rgba(20, 20, 26, 0.6);
    box-shadow: 0 18px 40px -28px rgba(20, 20, 26, 0.5);
}

.showcase-pricing-int__tier {
    font-size: 13px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    opacity: 0.62;
    margin: 0 0 14px 0;
}

.showcase-pricing-int__price {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 0 0 14px 0;
}

.showcase-pricing-int__amt {
    font-size: 46px;
    font-weight: 600;
    letter-spacing: -0.03em;
}

.showcase-pricing-int__per {
    font-size: 14px;
    opacity: 0.55;
}

.showcase-pricing-int__note {
    font-size: 14px;
    line-height: 1.5;
    opacity: 0.7;
    margin: 0;
}

/* The switch writes the signal; everything else READS it. Note there is no
   per-element wiring: the three amounts, the three period labels and the badge
   all follow from `$annual` through the cascade and three text bindings. */

.showcase-pricing-int__opt {
    @on &.click {
        $annual <- $.dataset.annual;
    }
}

.showcase-pricing-int__amt--a { text <- $annual == "true" ? "$86" : "$9"; }
.showcase-pricing-int__amt--b { text <- $annual == "true" ? "$278" : "$29"; }
.showcase-pricing-int__amt--c { text <- $annual == "true" ? "$758" : "$79"; }

.showcase-pricing-int__per { text <- $annual == "true" ? "/ year" : "/ month"; }

.showcase-pricing-int {
    data-annual <- $annual;
}

.showcase-pricing-int[data-annual="true"] .showcase-pricing-int__save { opacity: 1; }

.showcase-pricing-int[data-annual="false"] .showcase-pricing-int__opt[data-annual="false"],
.showcase-pricing-int[data-annual="true"] .showcase-pricing-int__opt[data-annual="true"] {
    background: #14141a;
    border-color: #14141a;
    color: #ffffff;
}
