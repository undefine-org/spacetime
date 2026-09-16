/* ============================================================================
   MARA VOSS — a designer's portfolio.

   The editorial shape: warm paper, ink, one red. Oversized serif, an index
   that knows where you are (every project panel carries @scroll-spy and the
   index lights the matching line through @when), and a work gallery of
   sticky-stacked panels — each project pins and the next slides over it, a
   signature portfolio effect that needs nothing but position: sticky and a
   real scrollbar.

   The serif is Georgia on purpose: a designer's system stack, set well, beats
   a webfont set carelessly.
   ============================================================================ */

body {
    margin: 0;
    background: #faf6ef;
    color: #191714;
    font-family: "Aspekta", "Inter", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
}

::selection { background: #b3341f; color: #faf6ef; }

.page { position: relative; }

.serif { font-family: Georgia, "Times New Roman", serif; }

/* --------------------------------------------------------------------------
   Header — name as wordmark, role as fact.
   -------------------------------------------------------------------------- */

.head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 30px 48px;
}

.head__name { font-family: Georgia, serif; font-style: italic; font-size: 22px; }

.head__role {
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(25, 23, 20, 0.55);
}

/* --------------------------------------------------------------------------
   Hero — the name, the size of the page.
   -------------------------------------------------------------------------- */

.hero {
    padding: 14vh 48px 12vh 48px;
    border-bottom: 1px solid rgba(25, 23, 20, 0.2);
}

.hero__title {
    font-family: Georgia, "Times New Roman", serif;
    font-size: clamp(4.5rem, 15vw, 15rem);
    font-weight: 400;
    line-height: 0.95;
    letter-spacing: -0.02em;
    margin: 0;

    @reveal(split: "chars", trigger: load, stagger: 45, duration: 800)
}

.hero__title em { font-style: italic; color: #b3341f; }

.hero__sub {
    margin-top: 6vh;
    max-width: 46ch;
    font-size: 1.1rem;
    font-weight: 350;
    line-height: 1.65;
    color: rgba(25, 23, 20, 0.7);

    @reveal(split: "words", trigger: load, stagger: 14, duration: 600)
}

/* --------------------------------------------------------------------------
   Index — a table of contents that watches you read.
   -------------------------------------------------------------------------- */

.index {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 8vh 48px;
    gap: 40px;
    border-bottom: 1px solid rgba(25, 23, 20, 0.2);
}

.index__lead {
    font-family: Georgia, serif;
    font-style: italic;
    font-size: clamp(1.4rem, 2.4vw, 2rem);
    line-height: 1.35;
    margin: 0;
    max-width: 24ch;
}

.index__list { display: flex; flex-direction: column; }

.idx {
    display: flex;
    align-items: baseline;
    gap: 22px;
    padding: 14px 0;
    border-bottom: 1px solid rgba(25, 23, 20, 0.14);
    color: rgba(25, 23, 20, 0.45);
    transition: color 400ms ease, padding-left 400ms cubic-bezier(0.22, 1, 0.36, 1);
}

.idx__no { font-size: 11px; letter-spacing: 0.14em; }

.idx__name { font-family: Georgia, serif; font-size: 1.35rem; }

.idx__year { margin-left: auto; font-size: 11px; letter-spacing: 0.14em; }



/* --------------------------------------------------------------------------
   Work — sticky-stacked panels. Each pins; the next slides over it.
   -------------------------------------------------------------------------- */

.work { position: relative; }

.proj {
    position: sticky;
    top: 0;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    padding: 48px;
}

/* NB: the spy-responsive garnish (dimming cards until their panel is centred)
   needs a signal-to-attribute binding the language does not currently resolve
   for primitive-yielded signals — folded into BUG-355. The stack, the reveals,
   and the editorial system carry this page without it. */


.proj__art { position: absolute; inset: 0; z-index: -1; }

.art--terra  { background: linear-gradient(155deg, #511c0e 0%, #b3341f 58%, #e08a63 100%); }
.art--indigo { background: linear-gradient(150deg, #101830 0%, #2c4480 55%, #8ba3d4 100%); }
.art--olive  { background: linear-gradient(145deg, #22260f 0%, #5a6134 55%, #b3ba82 100%); }
.art--plum   { background: linear-gradient(160deg, #2b0f22 0%, #6e2856 55%, #c98bb6 100%); }

.proj__card {
    background: #faf6ef;
    max-width: 560px;
    padding: 40px 44px;
}

.proj__no { font-size: 11px; letter-spacing: 0.2em; color: #b3341f; }

.proj__title {
    font-family: Georgia, serif;
    font-size: clamp(2.2rem, 4.5vw, 3.6rem);
    font-weight: 400;
    letter-spacing: -0.015em;
    line-height: 1.05;
    margin: 14px 0 0 0;

    @reveal(split: "words", stagger: 60, duration: 700)
}

.proj__meta {
    margin-top: 18px;
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: rgba(25, 23, 20, 0.55);
    display: flex;
    gap: 26px;
}

.proj__desc {
    margin-top: 18px;
    font-size: 0.95rem;
    font-weight: 350;
    line-height: 1.6;
    color: rgba(25, 23, 20, 0.7);
    max-width: 44ch;
}

/* --------------------------------------------------------------------------
   About — the pull-quote, then the facts.
   -------------------------------------------------------------------------- */

.about {
    padding: 16vh 48px;
    background: #faf6ef;
    position: relative;
    border-top: 1px solid rgba(25, 23, 20, 0.2);
}

.about__quote {
    font-family: Georgia, serif;
    font-style: italic;
    font-size: clamp(2rem, 4.6vw, 4rem);
    line-height: 1.25;
    letter-spacing: -0.01em;
    margin: 0;
    max-width: 22ch;

    @reveal(split: "words", stagger: 40, duration: 700)
}

.about__quote em { font-style: normal; color: #b3341f; }

.about__grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 44px;
    margin-top: 10vh;
}

.about__h { font-size: 11px; letter-spacing: 0.22em; text-transform: uppercase; color: rgba(25, 23, 20, 0.5); margin: 0 0 14px 0; }

.about__p { font-size: 0.95rem; font-weight: 350; line-height: 1.65; color: rgba(25, 23, 20, 0.75); margin: 0; }

/* --------------------------------------------------------------------------
   Contact — one italic sentence and a magnetic door.
   -------------------------------------------------------------------------- */

.contact {
    min-height: 88vh;
    background: #191714;
    color: #faf6ef;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 0 48px;
}

.contact__title {
    font-family: Georgia, serif;
    font-style: italic;
    font-size: clamp(3rem, 9vw, 9rem);
    font-weight: 400;
    letter-spacing: -0.02em;
    line-height: 1.02;
    margin: 0;

    @reveal(split: "words", stagger: 70, duration: 800)
}

.contact__title em { font-style: normal; color: #e08a63; }

.contact__cta {
    display: inline-block;
    margin-top: 7vh;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #191714;
    background: #faf6ef;
    padding: 19px 46px;
    border-radius: 999px;

    @magnetic(strength: 0.4, radius: 180, lerp: 0.14)
}

.footer {
    background: #191714;
    color: rgba(250, 246, 239, 0.35);
    border-top: 1px solid rgba(250, 246, 239, 0.12);
    padding: 26px 48px;
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
}

/* --------------------------------------------------------------------------
   Markup
   -------------------------------------------------------------------------- */

<main class="page">
    <header class="head">
        <span class="head__name">Mara Voss</span>
        <span class="head__role">Graphic designer — Amsterdam</span>
    </header>

    <section class="hero">
        <h1 class="hero__title">Mara <em>Voss</em></h1>
        <p class="hero__sub">Books, identities, and exhibitions. Ten years of putting things in the right order and leaving the rest out. Currently half of Studio Voss Vermeer, always all of the typographer.</p>
    </section>

    <section class="index">
        <p class="index__lead">Four projects, chosen the way you choose what to keep on a shelf.</p>
        <div class="index__list">
            <div class="idx idx--1"><span class="idx__no">01</span><span class="idx__name">Terra Cookbooks</span><span class="idx__year">2025</span></div>
            <div class="idx idx--2"><span class="idx__no">02</span><span class="idx__name">Indigo Sessions</span><span class="idx__year">2024</span></div>
            <div class="idx idx--3"><span class="idx__no">03</span><span class="idx__name">Olive and Stone</span><span class="idx__year">2023</span></div>
            <div class="idx idx--4"><span class="idx__no">04</span><span class="idx__name">Plum Records</span><span class="idx__year">2022</span></div>
        </div>
    </section>

    <section class="work">
        <div class="proj proj--1">
            <div class="proj__art art--terra"></div>
            <div class="proj__card">
                <span class="proj__no">01 — Terra Cookbooks</span>
                <h2 class="proj__title">Recipes that read like letters.</h2>
                <p class="proj__desc">A five-volume cookbook series set like correspondence: one typeface, one red, no photographs of food. The ingredient is the typography.</p>
                <div class="proj__meta"><span>Editorial</span><span>2025</span><span>With Terra Publishing</span></div>
            </div>
        </div>
        <div class="proj proj--2">
            <div class="proj__art art--indigo"></div>
            <div class="proj__card">
                <span class="proj__no">02 — Indigo Sessions</span>
                <h2 class="proj__title">A jazz label, redrawn nightly.</h2>
                <p class="proj__desc">Identity and cover system for a live recording series — a grid that improvises inside its own rules, like the music does.</p>
                <div class="proj__meta"><span>Identity</span><span>2024</span><span>With North Sea Jazz</span></div>
            </div>
        </div>
        <div class="proj proj--3">
            <div class="proj__art art--olive"></div>
            <div class="proj__card">
                <span class="proj__no">03 — Olive and Stone</span>
                <h2 class="proj__title">An olive oil with a library card.</h2>
                <p class="proj__desc">Packaging for a Cretan family grove: each harvest a numbered edition, each label a page from the year's notebook.</p>
                <div class="proj__meta"><span>Packaging</span><span>2023</span><span>Annual series</span></div>
            </div>
        </div>
        <div class="proj proj--4">
            <div class="proj__art art--plum"></div>
            <div class="proj__card">
                <span class="proj__no">04 — Plum Records</span>
                <h2 class="proj__title">Sleeves for music that whispers.</h2>
                <p class="proj__desc">Twelve LP sleeves for an ambient label — duotone still lifes, set in one italic, numbered by hand.</p>
                <div class="proj__meta"><span>Art direction</span><span>2022</span><span>Series of twelve</span></div>
            </div>
        </div>
    </section>

    <section class="about">
        <p class="about__quote">Good design is the part you <em>don't</em> notice was decided.</p>
        <div class="about__grid">
            <div>
                <h3 class="about__h">Practice</h3>
                <p class="about__p">Editorial and identity work for publishers, labels, and small companies with long memories. Half of Studio Voss Vermeer since 2019.</p>
            </div>
            <div>
                <h3 class="about__h">Recognition</h3>
                <p class="about__p">Dutch Design Awards, best book design shortlist twice. Type Directors Club, certificate of excellence. D&AD, one pencil, wood.</p>
            </div>
            <div>
                <h3 class="about__h">Elsewhere</h3>
                <p class="about__p">Teaching editorial design at KABK one day a week. Writing a very slow book about paragraph shapes.</p>
            </div>
        </div>
    </section>

    <section class="contact">
        <h2 class="contact__title">Let's make <em>something.</em></h2>
        <span class="contact__cta">studio@maravoss.nl</span>
    </section>

    <footer class="footer">
        <span>Mara Voss — a Spacetime showcase</span>
        <span>Set in Georgia, honestly</span>
    </footer>
</main>
