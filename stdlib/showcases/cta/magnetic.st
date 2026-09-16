/// scenario: cta
/// style: magnetic
/// school: motion-poetics
/// description: A closing CTA whose button is magnetically attracted to the cursor and whose headline reveals per-word — pointer physics and text choreography, both declared.
/// dependencies: none
/// screenshot: (pending)
/// useWhen: A final ask that should feel alive under the hand; launches, portfolios, anywhere the click deserves anticipation.

@import "../_schools.st"

/* Two capabilities, both DECLARED rather than scripted:
   `@magnetic` gives the button pointer physics (attraction, radius, easing),
   `@reveal` splits the headline and staggers it in on visibility. Neither
   needs an event handler, a rAF loop, or a per-element setup call. */

@template &showcase-cta-magnetic($headline, $sub, $cta) {
    <section class="showcase-cta-mag">
        <div class="showcase-cta-mag__inner">
            <h2 class="showcase-cta-mag__title">`$headline`</h2>
            <p class="showcase-cta-mag__sub">`$sub`</p>
            <div class="showcase-cta-mag__zone">
                <a class="showcase-cta-mag__btn" href="#">`$cta`</a>
            </div>
        </div>
    </section>
}

.showcase-cta-mag {
    --sc-mtn-surface;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 92vh;
    padding: 96px 32px;
    overflow: hidden;
}

.showcase-cta-mag__inner {
    max-width: 820px;
    text-align: center;
}

.showcase-cta-mag__title {
    --sc-mtn-title;
    margin: 0 0 20px 0;

    /* Per-word entrance: the split, the stagger and the trigger are all
       declared here. Words, not chars, because a headline this size reads as
       phrases. */
    @reveal(split: "words", stagger: 60, duration: 700);
}

.showcase-cta-mag__sub {
    --sc-min-muted;
    margin: 0 auto 44px auto;
    max-width: 52ch;
}

/* The magnet's catchment. Giving the zone real size means the button starts
   reacting BEFORE the cursor is on it — which is the whole sensation. */
.showcase-cta-mag__zone {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 160px;
}

.showcase-cta-mag__btn {
    --sc-mtn-cta;
    display: inline-block;

    /* Pointer physics as a declaration: pull toward the cursor within 180px,
       at 40% strength, eased so it trails the hand instead of snapping. */
    @magnetic(strength: 0.4, radius: 180, lerp: 0.14);
}
