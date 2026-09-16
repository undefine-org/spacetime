/// The showcase tree, rendered — and the page does not know how many patterns
/// there are.
///
/// `stdlib/showcases/` curates patterns across five design schools and, until
/// this page existed, compiled to ZERO pages: `index.st` there is a pure import
/// manifest. Worse, not one pattern had a call site anywhere in the repo — they
/// were authored, restyled and catalogued in Markdown, never instantiated.
///
/// The specimen list is GENERATED from `@data declarations … from template`,
/// so publishing a pattern (a file plus its line in the tree's manifest) makes
/// a specimen appear here with no edit to this page. The metadata beside each
/// one is the pattern's own `/// scenario: / school: / useWhen:` header, read
/// as data — not copied.
///
/// What stays authored is the COPY. A pattern deserves real words to show what
/// it is for; generated lorem would make the gallery useless. So each slot
/// below is filled by name, and a pattern with no authored copy still appears
/// (with an empty frame) rather than vanishing silently.

@import "../../stdlib/showcases/index.st"

@data declarations $patterns from template ;
@data inline $school : "all";

<main class="gal">

  <header class="gal__head">
    <p class="gal__kicker">stdlib/showcases</p>
    <h1 class="gal__title">The patterns, rendered.</h1>
    <p class="gal__lede">
      Every specimen is the real template, invoked. The list is generated from
      the tree, so this page is never out of date. Filter by the school you are
      designing in — the taxonomy is the navigation.
    </p>

    <nav class="gal__rail">
      <button class="chip chip--school" data-school="all">all</button>
      <button class="chip chip--school" data-school="minimalism">minimalism</button>
      <button class="chip chip--school" data-school="information-architecture">information-architecture</button>
      <button class="chip chip--school" data-school="motion-poetics">motion-poetics</button>
      <button class="chip chip--school" data-school="experimental">experimental</button>
      <button class="chip chip--school" data-school="eastern-philosophy">eastern-philosophy</button>
    </nav>
  </header>

  <div class="gal__specimens"></div>

  <section class="flag">
    <h2 class="flag__title">The flagships</h2>
    <p class="flag__lede">
      Three complete sites, assembled from this vocabulary alone — live, in
      their frames. Serve them and the frames wake up:
      <code>cargo run -- serve demos/brand-launch/</code> (4342),
      <code>demos/studio/</code> (4343), <code>demos/portfolio/</code> (4344).
    </p>
    <div class="flag__row">
      <figure class="flag__card">
        <div class="flag__frame"><iframe class="flag__site" src="http://127.0.0.1:4342/" title="AURA — brand launch" loading="lazy"></iframe></div>
        <figcaption class="flag__cap"><strong>AURA</strong> — a product launch. Thirty thousand particles morph through four states as you scroll; one declarative binding drives all of it.</figcaption>
      </figure>
      <figure class="flag__card">
        <div class="flag__frame"><iframe class="flag__site" src="http://127.0.0.1:4343/" title="NORTH — studio and agency" loading="lazy"></iframe></div>
        <figcaption class="flag__cap"><strong>NORTH</strong> — a design practice. The same variable font in the opposite register: Swiss, light, type-led, hover-plated case grid.</figcaption>
      </figure>
      <figure class="flag__card">
        <div class="flag__frame"><iframe class="flag__site" src="http://127.0.0.1:4344/" title="Mara Voss — portfolio" loading="lazy"></iframe></div>
        <figcaption class="flag__cap"><strong>MARA VOSS</strong> — a portfolio. Sticky-stacked project panels and one italic sentence the size of the page.</figcaption>
      </figure>
    </div>
  </section>

  <footer class="gal__foot">
    <hr class="gal__hr" />
    <p class="gal__note">
      Patterns live in <code>stdlib/showcases/&lt;scenario&gt;/&lt;style&gt;.st</code>.
      Splice the school forms from <code>_schools.st</code> to restyle one —
      don't fork the pattern.
    </p>
  </footer>

</main>

/* One specimen per declared template. The frame carries a slot class derived
   from the pattern's own name, which is how the authored copy at the bottom of
   this file finds its pattern — the page never names the set. */
.gal__specimens {
  @each($patterns as $p) {
    <section class="spec" data-school="`$p.fields.school`" data-scenario="`$p.fields.scenario`" data-style="`$p.fields.style`">
      <div class="spec__meta">
        <p class="spec__id">`$p.fields.scenario` / `$p.fields.style` · <span class="spec__school">`$p.fields.school`</span></p>
        <p class="spec__desc">`$p.fields.description`</p>
        <p class="spec__use">`$p.fields.useWhen`</p>
      </div>
      <div class="spec__frame"><div class="spec__stage slot-`$p.name`"></div></div>
    </section>
  }
}

/* ---- gallery chrome ------------------------------------------------ */

.gal {
    --gal-ground;
    max-width: 1120px;
    margin: 0 auto;
    padding: 72px 24px 96px;
}

.gal__kicker { --gal-eyebrow; margin: 0 0 12px 0; }

.gal__title {
    font-size: clamp(2.2rem, 4.6vw, 3.4rem);
    line-height: 1.05;
    letter-spacing: -0.03em;
    font-weight: 620;
    margin: 0 0 16px 0;
}

.gal__lede {
    font-size: 1.05rem;
    line-height: 1.6;
    color: #5a5a6b;
    max-width: 62ch;
    margin: 0 0 32px 0;
}

.gal__rail {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding-bottom: 40px;
}

.chip { --gal-chip; }



.spec {
    padding: 40px 0 8px;
    border-top: 1px solid #eeeef4;
}

.spec__meta { padding-bottom: 18px; }

.spec__id { --gal-eyebrow; margin: 0 0 8px 0; }

.spec__school { color: #14141a; }

.spec__desc {
    font-size: 1rem;
    line-height: 1.55;
    margin: 0 0 4px 0;
    max-width: 70ch;
}

.spec__use {
    font-size: 0.9rem;
    line-height: 1.55;
    color: #6b6b7b;
    margin: 0;
    max-width: 70ch;
}

.spec__frame { --gal-frame; }

.spec__stage { --gal-stage; }

/* A short pattern must not sit in a tall box. `nav` and `footer` are ~200px
   designs; in a frame sized for a 1120px hero they float in 200px of dead
   space, which reads as a broken specimen rather than a small one. So a
   compact pattern gets a frame that hugs it — keyed off the pattern's own
   scenario, which arrives as data on the specimen. */

/* A pattern sizes itself to the VIEWPORT (`min-height: 100vh`), which inside a
   scaled stage still means the real browser window — so a specimen would run
   taller than its stage and clip anyway.

   Bind it to the stage instead. `min-height` is RELAXED rather than pinned: a
   pattern whose content needs more room (a 112px display plus 96px padding
   runs past 1100px) must be allowed to grow, and the stage grows with it. The
   two-class selector outranks the pattern's own single class, so no
   `!important` is needed and the pattern files stay untouched. */
.gal .spec__stage > * {
    min-height: 820px;
}

/* Nav, CTA and footer are BANDS, not pages: they declare their own modest
   height and a page-sized `min-height` would stretch them into something they
   are not. Relax the floor and shrink the frame to match, so each reads at its
   true proportion. `data-scenario` comes from the pattern's own `/// scenario:`
   header, so this needs no list of names. */
.spec[data-scenario="nav"] .spec__stage > *,
.spec[data-scenario="footer"] .spec__stage > * {
    min-height: 0;
}

.spec[data-scenario="nav"] .spec__frame,
.spec[data-scenario="footer"] .spec__frame {
    height: 190px;
}

/* CTA covers both a compact banner and a full-height magnetic section, so it
   gets no shared override — the default page-sized frame fits the taller one,
   and the banner's own `--compact` rule below trims it. Guessing one height
   for a scenario is what cropped these in the first place. */
.spec[data-scenario="cta"][data-style="minimal"] .spec__stage > * {
    min-height: 420px;
}

.spec[data-scenario="cta"][data-style="minimal"] .spec__frame {
    height: 470px;
}

.flag { padding-top: 72px; }

.flag__title { font-size: 26px; font-weight: 800; letter-spacing: -0.02em; color: #14141c; margin: 0 0 10px 0; }

.flag__lede {
    max-width: 70ch;
    margin: 0 0 26px 0;
    font-size: 14px;
    line-height: 1.6;
    color: #5a5a6b;
}

.flag__lede code {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 12px;
    background: rgba(20, 20, 24, 0.06);
    padding: 2px 6px;
    border-radius: 4px;
}

.flag__row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 22px;
}

.flag__card { margin: 0; }

/* The same scaled-stage trick as the specimens: the site lays out at 1440x900
   inside a frame a third of the page wide, transform does the rest. */
.flag__frame {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 10;
    overflow: hidden;
    border: 1px solid rgba(20, 20, 24, 0.18);
    border-radius: 10px;
    background: #0b0b10;
}

.flag__site {
    position: absolute;
    top: 0;
    left: 0;
    width: 1440px;
    height: 900px;
    border: 0;
    transform: scale(0.32);
    transform-origin: top left;
    pointer-events: none;
}

.flag__cap {
    --gal-mono;
    color: #6b6b7b;
    margin-top: 12px;
    font-size: 12.5px;
    line-height: 1.55;
}

.gal__foot { padding-top: 56px; }

.gal__hr { --gal-rule; margin: 0 0 20px 0; }

.gal__note {
    --gal-mono;
    color: #6b6b7b;
    line-height: 1.7;
    margin: 0;
}

/* The interactive patterns. These are the ones worth clicking: each shows a
   capability rather than a composition. */

.slot-showcase-pricing-interactive { &showcase-pricing-interactive("One price, two ways to pay.", "Starter", "Team", "Scale"); }

.slot-showcase-cta-magnetic { &showcase-cta-magnetic("Come a little closer.", "The button is attracted to your cursor — pointer physics, declared in one line.", "Feel it"); }

.slot-showcase-features-tabbed { &showcase-features-tabbed("Three things worth knowing.", "Declared", "Signals", "Cascade"); }

.slot-showcase-nav-marquee { &showcase-nav-marquee("32 drivers · head tracking · 40-hour battery · lossless over air · 24-bit 96 kHz ·", "dark"); }

.slot-showcase-cta-editorial { &showcase-cta-editorial("Let's make", "something.", "studio@maravoss.nl"); }

/* ---- browse: one signal, every specimen ----------------------------

   The selected school is published onto the ROOT as an attribute, and the
   cascade does the filtering. That is the whole mechanism: one signal, one
   attribute, a handful of declarative rules — no per-specimen state, no list
   to keep in sync, and nothing to re-run when the set of patterns changes.

   A specimen is hidden by a rule that only matches when the root's school
   disagrees with the specimen's own, so adding a pattern needs no filter
   edit — it participates by carrying `data-school`. */

.gal { data-school <- $school; }

.chip--school {
    @on &.click {
        $school <- $.dataset.school;
    }
}

.gal[data-school="minimalism"] .spec:not([data-school="minimalism"]),
.gal[data-school="information-architecture"] .spec:not([data-school="information-architecture"]),
.gal[data-school="motion-poetics"] .spec:not([data-school="motion-poetics"]),
.gal[data-school="experimental"] .spec:not([data-school="experimental"]),
.gal[data-school="eastern-philosophy"] .spec:not([data-school="eastern-philosophy"]) {
    display: none;
}

/* The active chip reads as selected — same attribute, same cascade, so the
   highlight can never drift from what is actually filtered. */
.gal[data-school="all"] .chip--school[data-school="all"],
.gal[data-school="minimalism"] .chip--school[data-school="minimalism"],
.gal[data-school="information-architecture"] .chip--school[data-school="information-architecture"],
.gal[data-school="motion-poetics"] .chip--school[data-school="motion-poetics"],
.gal[data-school="experimental"] .chip--school[data-school="experimental"],
.gal[data-school="eastern-philosophy"] .chip--school[data-school="eastern-philosophy"] {
    --gal-chip-on;
}



/* ---- the authored copy -----------------------------------------------

   A pattern needs real words to show what it is for, so the ARGUMENTS stay
   written by hand — only the list is generated. Each block targets the slot
   class the loop above derived from the pattern's name, so adding copy for a
   new pattern is a local edit here and needs no change to the page structure.

   A pattern with no copy yet still gets a specimen and its metadata; its frame
   is simply empty. That is the honest failure mode — visible, not silent. */

.slot-showcase-hero-minimal { &showcase-hero-minimal("Everything you need. Nothing you don't.", "One idea, stated once, with room to breathe.", "Start building"); }
.slot-showcase-hero-editorial { &showcase-hero-editorial("Issue 04", "The long form is not dead. It moved.", "A quarterly on craft, tools, and the people who sharpen them.", "Read the issue", "Subscribe"); }
.slot-showcase-hero-maximal { &showcase-hero-maximal("SHIP IT LOUD", "For the launch that refuses to be tasteful about it.", "Get early access"); }
.slot-showcase-hero-architecture { &showcase-hero-architecture("A year measured in shipped work.", "312", "k", "requests / minute", "99.98", "%", "uptime, rolling 12mo", "41", "ms", "median response"); }
.slot-showcase-hero-motion { &showcase-hero-motion("Motion is a material.", "Scored to the scroll, not sprinkled on top.", "00 — enter", "01 — settle", "See the score"); }
.slot-showcase-hero-eastern { &showcase-hero-eastern("間 — ma", "The space between is part of the design.", "A slow house for tea, paper, and unhurried work.", "Visit the studio"); }
.slot-showcase-nav-minimal { &showcase-nav-minimal("Aurora", "Sign in"); }
.slot-showcase-features-minimal { &showcase-features-minimal("Built for the long middle of a project"); }
.slot-showcase-cta-minimal { &showcase-cta-minimal("Ready when you are.", "Start building"); }
.slot-showcase-footer-minimal { &showcase-footer-minimal("Aurora", "2026"); }

/* ---- browse: one signal, every specimen ----------------------------

   The selected school is published onto the ROOT as an attribute and the
   cascade does the filtering: one signal, one attribute, a handful of
   declarative rules. A specimen participates by carrying `data-school`, which
   it gets from its own declaration — so a new pattern is filterable the moment
   it exists, with no filter edit. */

.gal { data-school <- $school; }

.chip--school {
    @on &.click {
        $school <- $.dataset.school;
    }
}

.gal[data-school="minimalism"] .spec:not([data-school="minimalism"]),
.gal[data-school="information-architecture"] .spec:not([data-school="information-architecture"]),
.gal[data-school="motion-poetics"] .spec:not([data-school="motion-poetics"]),
.gal[data-school="experimental"] .spec:not([data-school="experimental"]),
.gal[data-school="eastern-philosophy"] .spec:not([data-school="eastern-philosophy"]) {
    display: none;
}

.gal[data-school="all"] .chip--school[data-school="all"],
.gal[data-school="minimalism"] .chip--school[data-school="minimalism"],
.gal[data-school="information-architecture"] .chip--school[data-school="information-architecture"],
.gal[data-school="motion-poetics"] .chip--school[data-school="motion-poetics"],
.gal[data-school="experimental"] .chip--school[data-school="experimental"],
.gal[data-school="eastern-philosophy"] .chip--school[data-school="eastern-philosophy"] {
    --gal-chip-on;
}
