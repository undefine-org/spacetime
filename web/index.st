// =============================================================================
// Zypsy — Homepage Animations & Data
// Colocated with index.html
// =============================================================================

// -----------------------------------------------------------------------------
// Easing Presets
// -----------------------------------------------------------------------------

@preset easing ~zypsy-smooth: cubic-bezier(0.19, 1, 0.22, 1);
@preset easing ~zypsy-ease: cubic-bezier(0.4, 0, 0.2, 1);

@preset animation &reveal {
    opacity: 0 -> 1;
    translate-y: 40px -> 0;
    easing: &ease-out-expo;
}

@preset animation &fade-in {
    opacity: 0 -> 1;
    easing: &ease-out-expo;
}

// -----------------------------------------------------------------------------
// Type Definitions
// -----------------------------------------------------------------------------

@type Project {
    id: string;
    slug: string;
    title: string;
    category: string;
    thumbnail: url;
    investors: string[];
    tags: string[];
}

@type Capability {
    id: string;
    title: string;
    stage: string;
    services: string[];
}

@type FAQ {
    id: string;
    question: string;
    answer: string;
}

@type NewsItem {
    id: string;
    title: string;
    date: string;
    source: string;
}

@type TeamMember {
    id: string;
    name: string;
    role: string;
    photo: url;
}

@type Post {
    id: string;
    title: string;
    excerpt: string;
    image: url;
    category: string;
    date: string;
}

// -----------------------------------------------------------------------------
// Data Sources
// -----------------------------------------------------------------------------

@data projects: Project[] { src: "/data/projects.json"; }
@data capabilities: Capability[] { src: "/data/capabilities.json"; }
@data faqs: FAQ[] { src: "/data/faqs.json"; }

// Accordion open state (see FUP-075 for per-item independent state).
$accordionOpen bool: false;
@data news: NewsItem[] { src: "/data/news.json"; }
@data team: TeamMember[] { src: "/data/team.json"; }
@data posts: Post[] { src: "/data/posts.json"; }

// -----------------------------------------------------------------------------
// Templates
// -----------------------------------------------------------------------------

@template &zypsy-work-card($p) {
    <article class="zypsy-work-card" data-slug="`$p.slug`">
        <div class="zypsy-work-card__image">
            <img src="`$p.thumbnail`" alt="`$p.title`" loading="lazy">
        </div>
        <div class="zypsy-work-card__info">
            <span class="zypsy-work-card__category">`$p.category`</span>
            <h3 class="zypsy-work-card__title">`$p.title`</h3>
            <p class="zypsy-work-card__investors">`$p.investors`</p>
            <span class="zypsy-work-card__cta">Discover &rarr;</span>
        </div>
    </article>
}

@template &zypsy-capability-card($c) {
    <div class="zypsy-capability-card" data-id="`$c.id`">
        <span class="zypsy-capability-card__stage">`$c.stage`</span>
        <h3 class="zypsy-capability-card__title">`$c.title`</h3>
        <ul class="zypsy-capability-card__services">
            <!-- services rendered as comma-joined text, list handled in HTML -->
        </ul>
    </div>
}

@template &zypsy-faq-item($q) {
    <div class="zypsy-accordion__item" data-id="`$q.id`">
        <button class="zypsy-accordion__trigger" aria-expanded="false">
            <span>`$q.question`</span>
            <span class="zypsy-accordion__icon">
                <svg viewBox="0 0 24 24"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
            </span>
        </button>
        <div class="zypsy-accordion__body">
            <p class="zypsy-accordion__content">`$q.answer`</p>
        </div>
    </div>
}

@template &zypsy-news-item($n) {
    <div class="zypsy-news-item" data-id="`$n.id`">
        <span class="zypsy-news-item__date">`$n.date`</span>
        <span class="zypsy-news-item__title">`$n.title`</span>
        <span class="zypsy-news-item__source">`$n.source`</span>
    </div>
}

@template &zypsy-team-member($m) {
    <div class="zypsy-team-member">
        <img class="zypsy-team-member__photo" src="`$m.photo`" alt="`$m.name`" loading="lazy">
        <p class="zypsy-team-member__name">`$m.name`</p>
        <p class="zypsy-team-member__role">`$m.role`</p>
    </div>
}

// =============================================================================
// Structural Templates (PROJ-057 partials)
// =============================================================================

// Nav: brand text + link list via body slot
@template &zypsy-nav($brand, &links) {
    <nav class="zypsy-nav">
        <div class="zypsy-nav__inner">
            <a href="/" class="zypsy-nav__logo">`$brand`</a>
            <div class="zypsy-nav__links">`&links`</div>
            <button class="zypsy-nav__menu" aria-label="Menu">
                <span class="zypsy-nav__menu-line"></span>
                <span class="zypsy-nav__menu-line"></span>
            </button>
        </div>
    </nav>
}

// Footer: brand text + body fills links/social/bottom structure
@template &zypsy-footer($brand, &body) {
    <footer class="zypsy-footer">
        <div class="zypsy-footer__inner">
            <div class="zypsy-footer__brand">
                `$brand`
                <p class="zypsy-footer__tagline">A design team for founders</p>
            </div>
            `&body`
        </div>
    </footer>
}

// =============================================================================
// STRUCTURE — Template invocations for nav + footer
// =============================================================================

header {
    &zypsy-nav("Zypsy") {
        <a href="/work.html">Work</a>
        <a href="#about">About</a>
        <a href="#capabilities">Capabilities</a>
        <a href="#insights">Insights</a>
        <a href="#contact">Contact</a>
    }
}

[data-footer] {
    &zypsy-footer("Zypsy") {
        <div class="zypsy-footer__links">
            <a href="/work.html">Work</a>
            <a href="#about">About</a>
            <a href="#capabilities">Capabilities</a>
            <a href="#insights">Insights</a>
            <a href="mailto:hello@zypsy.com">Contact</a>
            <a href="/careers.html">Careers</a>
        </div>
        <div class="zypsy-footer__social">
            <a href="https://twitter.com/zaborystudio" target="_blank" aria-label="Twitter">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M22 4s-.7 2.1-2 3.4c1.6 10-9.4 17.3-18 11.6 2.2.1 4.4-.6 6-2C3 15.5.5 9.6 3 5c2.2 2.6 5.6 4.1 9 4-.9-4.2 4-6.6 7-3.8 1.1 0 3-1.2 3-1.2z"/></svg>
            </a>
            <a href="https://instagram.com/zypsy" target="_blank" aria-label="Instagram">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2" y="2" width="20" height="20" rx="5"/><path d="M16 11.37A4 4 0 1 1 12.63 8 4 4 0 0 1 16 11.37z"/><line x1="17.5" y1="6.5" x2="17.51" y2="6.5"/></svg>
            </a>
            <a href="https://linkedin.com/company/zypsy" target="_blank" aria-label="LinkedIn">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-4 0v7h-4v-7a6 6 0 0 1 6-6z"/><rect x="2" y="9" width="4" height="12"/><circle cx="4" cy="4" r="2"/></svg>
            </a>
        </div>
        <div class="zypsy-footer__bottom">
            <div class="zypsy-footer__legal">
                <a href="/privacy.html">Privacy</a>
                <a href="/terms.html">Terms</a>
            </div>
            <p class="zypsy-footer__copyright">&copy; 2024 Zypsy Inc.</p>
        </div>
    }
}

// -----------------------------------------------------------------------------
// Data Binding — Populate grids from JSON
// -----------------------------------------------------------------------------

.zypsy-work-grid {
    @each($projects as $p) {
        &zypsy-work-card($p);
    }
}

.zypsy-capability-grid {
    @each($capabilities as $c) {
        &zypsy-capability-card($c);
    }
}

.zypsy-accordion {
    @each($faqs as $q) {
        &zypsy-faq-item($q);
    }
}

.zypsy-news-list {
    @each($news as $n) {
        &zypsy-news-item($n);
    }
}

.zypsy-team-track {
    @each($team as $m) {
        &zypsy-team-member($m);
    }
    // Duplicate for seamless marquee loop
    @each($team as $m) {
        &zypsy-team-member($m);
    }
}

// =============================================================================
// ACCORDION
// =============================================================================

// Accordion: click toggles open/closed; CSS styles the open state via
// `.zypsy-accordion__item.is-open`. NB per-item independent open state on
// @each-generated items is the proper target — see FUP-075 (a page-global signal
// here couples all items).
.zypsy-accordion__item {
    @on click { $accordionOpen <- !$accordionOpen; }
    .is-open: $accordionOpen;
}

// =============================================================================
// ANIMATIONS
// =============================================================================

// -----------------------------------------------------------------------------
// Nav — Scroll-direction hide/show + reveal on load
// -----------------------------------------------------------------------------

.zypsy-nav {
    @on visible nav-reveal(400ms, immediate: true) {
        & {
            opacity: 0 -> 1;
            translate-y: -20px -> 0;
            easing: &ease-out-expo;
        }
    }
}

.zypsy-nav__links a {
    @on hover nav-link-hover(200ms) {
        & {
            translate-y: 0 -> -2px;
            easing: &ease-out-quad;
        }
    }
}

// -----------------------------------------------------------------------------
// Hero — Staggered entrance
// -----------------------------------------------------------------------------

.zypsy-hero {
    @on visible hero-entrance(1200ms, immediate: true) {
        .zypsy-hero__media {
            opacity: 0 -> 1;
            scale: 1.05 -> 1;
            easing: &ease-out-expo;
            range: 0 to 0.6;
        }

        .zypsy-hero__title {
            opacity: 0 -> 1;
            translate-y: 60px -> 0;
            easing: &ease-out-expo;
            range: 0.2 to 0.7;
        }

        .zypsy-hero__subtitle {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            easing: &ease-out-expo;
            range: 0.35 to 0.8;
        }

        .zypsy-hero__cta {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
            easing: &ease-out-expo;
            range: 0.5 to 0.9;
        }

        .zypsy-hero__timezone {
            opacity: 0 -> 1;
            easing: &ease-out-quad;
            range: 0.7 to 1;
        }
    }
}

// -----------------------------------------------------------------------------
// Work Section — Scroll reveal + hover
// -----------------------------------------------------------------------------

.zypsy-section--work {
    @scroll work-section-reveal(start: 0, end: 0.4) {
        .zypsy-section__eyebrow {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: &ease-out-expo;
            range: 0 to 0.5;
        }
    }
}

.zypsy-work-grid {
    @scroll work-reveal(start: 0, end: 0.4) {
        .zypsy-work-card {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            easing: &ease-out-expo;
            stagger: 0.12 first;
        }
    }

    @on hover work-hover(600ms) {
        .zypsy-work-card__image img {
            scale: 1 -> 1.03;
            easing: &ease-out-expo;
        }
    }
}

// -----------------------------------------------------------------------------
// Capabilities — Scroll reveal + hover
// -----------------------------------------------------------------------------

.zypsy-section--capabilities {
    @scroll cap-section-reveal(start: 0, end: 0.4) {
        .zypsy-section__eyebrow {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: &ease-out-expo;
            range: 0 to 0.4;
        }

        .zypsy-section__title {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
            easing: &ease-out-expo;
            range: 0.1 to 0.5;
        }
    }
}

.zypsy-capability-grid {
    @scroll cap-reveal(start: 0, end: 0.4) {
        .zypsy-capability-card {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            easing: &ease-out-expo;
            stagger: 0.1 first;
        }
    }

    @on hover cap-hover(600ms) {
        .zypsy-capability-card__title {
            translate-x: 0 -> 8px;
            easing: &ease-out-expo;
        }
    }
}

// -----------------------------------------------------------------------------
// Values — Scroll reveal with stagger
// -----------------------------------------------------------------------------

.zypsy-values__item {
    @scroll values-reveal(start: 0, end: 0.5) {
        &reveal;
        stagger: 0.15 first;
    }
}

// -----------------------------------------------------------------------------
// News — Scroll reveal with stagger
// -----------------------------------------------------------------------------

.zypsy-section--news {
    @scroll news-section-reveal(start: 0, end: 0.4) {
        .zypsy-section__eyebrow {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: &ease-out-expo;
        }
    }
}

.zypsy-news-list {
    @scroll news-reveal(start: 0, end: 0.3) {
        .zypsy-news-item {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            easing: &ease-out-expo;
            stagger: 0.08 first;
        }
    }
}

// -----------------------------------------------------------------------------
// Stats — Scroll reveal + value counting
// -----------------------------------------------------------------------------

.zypsy-stats {
    @scroll stats-reveal(start: 0, end: 0.4) {
        .zypsy-stat {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
            easing: &ease-out-expo;
            stagger: 0.1 first;
        }
    }
}

.zypsy-stat__number {
    @value-change(duration: 250ms, stagger: 30ms, from: "start") {
        :entering {
            translate-y: -60% -> 0
            opacity: 0 -> 1
            easing: cubic-bezier(0, 0, 0.2, 1)
        }
        :exiting {
            translate-y: 0 -> 60%
            opacity: 1 -> 0
            easing: cubic-bezier(0.4, 0, 1, 1)
        }
    }
}

// -----------------------------------------------------------------------------
// FAQ Accordion — Hover + click
// -----------------------------------------------------------------------------

.zypsy-accordion {
    @scroll faq-reveal(start: 0, end: 0.3) {
        .zypsy-accordion__item {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            easing: &ease-out-expo;
            stagger: 0.08 first;
        }
    }

    @on hover accordion-hover(200ms) {
        .zypsy-accordion__trigger {
            color: var(--st-accent-light);
            easing: &ease-out-quad;
        }
    }

    @on click accordion-press(150ms) {
        .zypsy-accordion__trigger {
            scale: 1 -> 0.99;
            easing: &ease-out-quad;
        }
    }
}

// -----------------------------------------------------------------------------
// Team — Scroll reveal
// -----------------------------------------------------------------------------

.zypsy-team {
    @scroll team-reveal(start: 0, end: 0.4) {
        & {
            opacity: 0 -> 1;
            easing: &ease-out-expo;
        }
    }
}

// -----------------------------------------------------------------------------
// Footer — Staggered reveal
// -----------------------------------------------------------------------------

.zypsy-footer {
    @scroll footer-reveal(start: 0, end: 0.5) {
        .zypsy-footer__brand {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: &ease-out-expo;
            range: 0 to 0.4;
        }

        .zypsy-footer__links {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: &ease-out-expo;
            range: 0.1 to 0.5;
        }

        .zypsy-footer__social {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
            easing: &ease-out-expo;
            range: 0.2 to 0.6;
        }

        .zypsy-footer__bottom {
            opacity: 0 -> 1;
            easing: &ease-out-quad;
            range: 0.3 to 0.7;
        }
    }
}

.zypsy-footer__links a {
    @on hover footer-link-hover(200ms) {
        & {
            translate-y: 0 -> -2px;
            easing: &ease-out-quad;
        }
    }
}

// -----------------------------------------------------------------------------
// CTA Buttons — Global hover + press
// -----------------------------------------------------------------------------

.zypsy-cta {
    @on hover cta-hover(200ms) {
        & {
            scale: 1 -> 1.02;
            easing: &ease-out-quad;
        }
    }

    @on click cta-press(150ms) {
        & {
            scale: 1 -> 0.98;
            easing: &ease-out-quad;
        }
    }
}
