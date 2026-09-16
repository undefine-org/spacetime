/// scenario: features-grid
/// style: minimal
/// school: minimalism
/// description: 3-column responsive grid; each cell is title + one paragraph; no icons.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Product pages where the features deserve text not iconography.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-min-*`); this file keeps only
   layout + the entrance choreography. Extend the look by splicing the same
   forms in your own file — don't fork the pattern. */

@template &showcase-features-minimal($title) {
    <section class="showcase-features-min">
        <h2 class="showcase-features-min__title">`$title`</h2>
        <ul class="showcase-features-min__grid">
            <li>
                <h3>Fast by default</h3>
                <p>The runtime is small, single-threaded, and stays under 8KB gzipped.</p>
            </li>
            <li>
                <h3>Type-safe</h3>
                <p>Every animation, every binding, every signal is checked at compile time.</p>
            </li>
            <li>
                <h3>One file, one deploy</h3>
                <p>Static output. No build pipeline, no node_modules, no config dance.</p>
            </li>
        </ul>
    </section>
}

.showcase-features-min {
    --sc-min-surface;

    background: #ffffff;
    padding: 96px 32px;
}

.showcase-features-min__title {
    --sc-min-title;

    font-size: clamp(1.75rem, 3vw, 2.5rem);
    text-align: center;
    margin: 0 0 56px 0;
}

.showcase-features-min__grid {
    list-style: none;
    margin: 0 auto;
    padding: 0;
    max-width: 1080px;
    display: grid;
    gap: 40px;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
}

.showcase-features-min__grid li {
    @on &.visible: --sc-enter;
}

.showcase-features-min__grid h3 {
    font-size: 1.15rem;
    font-weight: 600;
    margin: 0 0 8px 0;
}

.showcase-features-min__grid p {
    --sc-min-muted;

    line-height: 1.55;
    margin: 0;
}
