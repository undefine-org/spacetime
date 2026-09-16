/// scenario: hero
/// style: media
/// school: motion-poetics
/// description: Full-bleed video hero whose overlay is choreographed by the video's own playhead — title, subtitle, CTA land as the media plays.
/// dependencies: stdlib
/// screenshot: (pending)
/// useWhen: Launch reels, case-study headers, campaign pages where the footage should conduct the typography.

@import "../_schools.st"

/* NB: NOT imported by `../index.st` yet — the pattern's core behavior
   (playhead-driven overlay choreography) is blocked on BUG-310: `__stRefs`
   has no writer, so the named-media ref a container score needs
   (`@score &reel.playback`) resolves null forever, and a score ON the
   <video> can never reach sibling subjects (void element, subjects resolve
   via ancestors). This file is the bug's acceptance fixture: import it,
   serve a reel, and the overlay must sequence exactly per --sc-media-hero
   (title 0–55%, subtitle 55–80%, cta 80–100% of the media duration).
   BUG-311 (dev-server Range support) blocks seek-testing it.

   The playback hero (PLAN-135 W6): a `<video>` the project points at its
   footage, with the overlay written against the MEDIA CLOCK — `@score
   &.playback` reads currentTime/duration, so the `--sc-media-hero` form's
   percentages are fractions of the video, not the page. The pattern ships no
   footage: pass `$src` (and `$poster`) from the project.
   Brand from `_schools.st` (`--sc-mtn-*` dark canvas). */

@template &showcase-hero-media($src, $poster, $title, $subtitle, $cta) {
    <section class="showcase-hero-media">
        <video class="showcase-hero-media__video" src="`$src`" poster="`$poster`" autoplay muted loop playsinline></video>
        <div class="showcase-hero-media__scrim"></div>
        <div class="showcase-hero-media__inner">
            <h1 class="showcase-hero-media__title">`$title`</h1>
            <p class="showcase-hero-media__subtitle">`$subtitle`</p>
            <a class="showcase-hero-media__cta" href="#">`$cta`</a>
        </div>
    </section>
}

.showcase-hero-media {
    --sc-mtn-surface;

    position: relative;
    overflow: hidden;
    min-height: 92vh;
    display: flex;
    align-items: flex-end;
    padding: 64px 32px;
}

.showcase-hero-media__video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;

    /* The video is the conductor: its playhead hands each overlay subject
       its slice of the media duration (form in _schools.st). */
    @score &.playback { --sc-media-hero; }
}

.showcase-hero-media__scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgba(10, 10, 12, 0.85), rgba(10, 10, 12, 0.15) 60%);
    pointer-events: none;
}

.showcase-hero-media__inner {
    position: relative;
    width: 100%;
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.showcase-hero-media__title {
    --sc-mtn-title;
    max-width: 16ch;
    margin: 0;

    @on &.clip: --sc-enter(32px);
}

.showcase-hero-media__subtitle {
    color: #94a3b8;
    font-size: clamp(1rem, 1.4vw, 1.25rem);
    max-width: 56ch;
    line-height: 1.55;
    margin: 0;

    @on &.clip: --sc-enter(20px);
}

.showcase-hero-media__cta {
    --sc-mtn-cta;
    align-self: flex-start;
    margin-top: 8px;

    @on &.clip: --sc-enter(12px);
}
