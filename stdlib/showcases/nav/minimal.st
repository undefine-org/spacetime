/// scenario: nav
/// style: minimal
/// school: minimalism
/// description: Single horizontal row with logo on the left and 3-5 text links on the right.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Most product and marketing sites with a flat IA; do not over-design the nav.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-min-*`); this file keeps only
   layout. Extend the look by splicing the same forms in your own file —
   don't fork the pattern. */

@template &showcase-nav-minimal($brand, $cta) {
    <nav class="showcase-nav-min">
        <a class="showcase-nav-min__brand" href="/">`$brand`</a>
        <ul class="showcase-nav-min__links">
            <li><a href="#features">Features</a></li>
            <li><a href="#pricing">Pricing</a></li>
            <li><a href="#docs">Docs</a></li>
        </ul>
        <a class="showcase-nav-min__cta" href="#signup">`$cta`</a>
    </nav>
}

.showcase-nav-min {
    --sc-min-surface;
    --sc-min-glass;

    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 14px 32px;
    border-bottom: 1px solid rgba(0, 0, 0, 0.08);
    position: sticky;
    top: 0;
    z-index: 10;
}

.showcase-nav-min__brand {
    font-weight: 600;
    text-decoration: none;
    color: #0f172a;
}

.showcase-nav-min__links {
    display: flex;
    list-style: none;
    margin: 0;
    padding: 0;
    gap: 24px;
}

.showcase-nav-min__links a {
    color: #0f172a;
    text-decoration: none;
    font-size: 0.95rem;
}

.showcase-nav-min__cta {
    --sc-min-cta;

    padding: 8px 16px;
    font-size: 0.95rem;
    font-weight: 500;
}
