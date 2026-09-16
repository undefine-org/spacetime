/// scenario: features-grid
/// style: tabbed
/// school: experimental
/// description: Feature panels behind a tab rail — one signal selects the panel, animates the indicator, and swaps the copy, with the selected tab derived rather than tracked.
/// dependencies: none
/// screenshot: (pending)
/// useWhen: Dense product pages where three or four capabilities each deserve a paragraph and a visual, without four scroll-lengths of page.

@import "../_schools.st"

/* State without a state machine. `$panel` is the only thing stored; the active
   tab, the visible panel, the indicator position and the counter all DERIVE
   from it through the cascade. There is nothing to keep in sync, so there is
   nothing that can fall out of sync. */

@template &showcase-features-tabbed($title, $a, $b, $c) {
    <section class="showcase-features-tab">
        <div class="showcase-features-tab__inner">
            <h2 class="showcase-features-tab__title">`$title`</h2>

            <div class="showcase-features-tab__rail">
                <button class="showcase-features-tab__tab" data-panel="a">`$a`</button>
                <button class="showcase-features-tab__tab" data-panel="b">`$b`</button>
                <button class="showcase-features-tab__tab" data-panel="c">`$c`</button>
                <span class="showcase-features-tab__ind"></span>
            </div>

            <div class="showcase-features-tab__panels">
                <article class="showcase-features-tab__panel" data-panel="a">
                    <p class="showcase-features-tab__lead">Declared, not scripted.</p>
                    <p class="showcase-features-tab__body">
                        Behaviour is a property of an element, written beside its
                        style. No lifecycle to wire, no cleanup to forget.
                    </p>
                </article>
                <article class="showcase-features-tab__panel" data-panel="b">
                    <p class="showcase-features-tab__lead">One signal, many readers.</p>
                    <p class="showcase-features-tab__body">
                        State is written once and read everywhere it matters. The
                        panel, the indicator and the counter all follow the same
                        value, so they cannot disagree.
                    </p>
                </article>
                <article class="showcase-features-tab__panel" data-panel="c">
                    <p class="showcase-features-tab__lead">The cascade does the work.</p>
                    <p class="showcase-features-tab__body">
                        Selection is an attribute on the root and the rules follow
                        from it — which is why adding a panel needs no new
                        handler, and no list to update.
                    </p>
                </article>
            </div>

            <p class="showcase-features-tab__count"></p>
        </div>
    </section>
}

@data inline $panel : "a";

.showcase-features-tab {
    --sc-max-surface;
    display: flex;
    align-items: center;
    min-height: 92vh;
    padding: 96px 32px;
}

.showcase-features-tab__inner {
    max-width: 980px;
    margin: 0 auto;
    width: 100%;
}

.showcase-features-tab__title {
    --sc-max-title;
    font-size: clamp(2.4rem, 4vw, 3.6rem);
    margin: 0 0 40px 0;
}

.showcase-features-tab__rail {
    position: relative;
    display: flex;
    gap: 6px;
    padding-bottom: 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.16);
}

.showcase-features-tab__tab {
    font: inherit;
    font-size: 15px;
    padding: 10px 20px;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    cursor: pointer;
    border-radius: 10px 10px 0 0;
    transition: color 220ms ease;
}

/* The indicator is one element that MOVES, rather than three that light up —
   so the transition is a real slide and the rail reads as a single control. */
.showcase-features-tab__ind {
    position: absolute;
    left: 0;
    bottom: -1px;
    height: 2px;
    width: 33%;
    background: #ffffff;
    border-radius: 2px;
    transition: transform 360ms cubic-bezier(0.22, 1, 0.36, 1);
}

.showcase-features-tab__panels {
    position: relative;
    padding-top: 40px;
    min-height: 210px;
}

.showcase-features-tab__panel {
    position: absolute;
    inset: 40px 0 0 0;
    opacity: 0;
    transform: translateY(10px);
    transition: opacity 300ms ease, transform 300ms ease;
    pointer-events: none;
}

.showcase-features-tab__lead {
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.02em;
    color: #ffffff;
    margin: 0 0 12px 0;
}

.showcase-features-tab__body {
    font-size: 16px;
    line-height: 1.65;
    color: rgba(255, 255, 255, 0.66);
    max-width: 58ch;
    margin: 0;
}

.showcase-features-tab__count {
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.4);
    margin: 32px 0 0 0;
}

/* Write the signal on click; everything below READS it. */

.showcase-features-tab__tab {
    @on &.click {
        $panel <- $.dataset.panel;
    }
}

.showcase-features-tab { data-panel <- $panel; }

.showcase-features-tab__count {
    text <- $panel == "a" ? "01 / 03" : ($panel == "b" ? "02 / 03" : "03 / 03");
}

.showcase-features-tab[data-panel="a"] .showcase-features-tab__panel[data-panel="a"],
.showcase-features-tab[data-panel="b"] .showcase-features-tab__panel[data-panel="b"],
.showcase-features-tab[data-panel="c"] .showcase-features-tab__panel[data-panel="c"] {
    opacity: 1;
    transform: translateY(0);
    pointer-events: auto;
}

.showcase-features-tab[data-panel="a"] .showcase-features-tab__tab[data-panel="a"],
.showcase-features-tab[data-panel="b"] .showcase-features-tab__tab[data-panel="b"],
.showcase-features-tab[data-panel="c"] .showcase-features-tab__tab[data-panel="c"] {
    color: #ffffff;
}

.showcase-features-tab[data-panel="a"] .showcase-features-tab__ind { transform: translateX(0%); }
.showcase-features-tab[data-panel="b"] .showcase-features-tab__ind { transform: translateX(100%); }
.showcase-features-tab[data-panel="c"] .showcase-features-tab__ind { transform: translateX(200%); }
