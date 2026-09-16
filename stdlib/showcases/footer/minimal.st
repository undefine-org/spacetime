/// scenario: footer
/// style: minimal
/// school: minimalism
/// description: Quiet single-row footer with brand, copyright, and three secondary links.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Default for utility sites and product pages where the footer should be invisible until needed.

@import "../_schools.st"

/* Brand comes from `_schools.st` splices (`--sc-min-*`); this file keeps only
   layout. Extend the look by splicing the same forms in your own file —
   don't fork the pattern. */

@template &showcase-footer-minimal($brand, $year) {
    <footer class="showcase-footer-min">
        <span class="showcase-footer-min__brand">`$brand`</span>
        <span class="showcase-footer-min__copy">&copy; `$year`</span>
        <nav class="showcase-footer-min__links">
            <a href="/privacy">Privacy</a>
            <a href="/terms">Terms</a>
            <a href="/contact">Contact</a>
        </nav>
    </footer>
}

.showcase-footer-min {
    --sc-min-surface;
    --sc-min-muted;

    background: #ffffff;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 24px 32px;
    border-top: 1px solid rgba(0, 0, 0, 0.08);
    font-size: 0.9rem;
    flex-wrap: wrap;
}

.showcase-footer-min__brand {
    font-weight: 500;
}

.showcase-footer-min__links {
    display: flex;
    gap: 20px;
}

.showcase-footer-min__links a {
    color: inherit;
    text-decoration: none;
}
