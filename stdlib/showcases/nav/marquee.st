/// scenario: nav
/// style: marquee
/// school: motion-poetics
/// description: The kinetic spec rail — an unbroken marquee of facts that never stops, two keyframes and nothing else.
/// dependencies: none
/// screenshot: (pending)
/// useWhen: Between hero and story on a launch or studio page, when a strip of specs, clients, or claims should read as momentum rather than a list.

@import "../_schools.st"

/* The marquee is two keyframes and a duplicated track: the inner strip is
   exactly twice the content, translating -50% loops seamlessly. Pause on
   hover so the facts stay readable when someone reaches for them. */

@template &showcase-nav-marquee($items, $tone) {
    <div class="showcase-nav-marquee showcase-nav-marquee--`$tone`">
        <div class="showcase-nav-marquee__inner">
            <span class="showcase-nav-marquee__run">`$items`</span>
            <span class="showcase-nav-marquee__run" aria-hidden="true">`$items`</span>
        </div>
    </div>
}

.showcase-nav-marquee {
    overflow: hidden;
    white-space: nowrap;
    padding: 18px 0;
    border-top: 1px solid rgba(138, 180, 255, 0.18);
    border-bottom: 1px solid rgba(138, 180, 255, 0.18);
    background: rgba(5, 6, 13, 0.55);
    backdrop-filter: blur(6px);
}

.showcase-nav-marquee--light {
    border-color: rgba(20, 20, 18, 0.25);
    background: #f4f2ed;
}

.showcase-nav-marquee--light .showcase-nav-marquee__run {
    color: rgba(20, 20, 18, 0.65);
}

.showcase-nav-marquee__inner {
    display: inline-flex;
    animation: showcase-marquee-scroll 26s linear infinite;
}

.showcase-nav-marquee:hover .showcase-nav-marquee__inner {
    animation-play-state: paused;
}

.showcase-nav-marquee__run {
    display: inline-block;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(234, 234, 242, 0.7);
    padding: 0 18px;
}

@keyframes showcase-marquee-scroll {
    from { transform: translateX(0); }
    to { transform: translateX(-50%); }
}
