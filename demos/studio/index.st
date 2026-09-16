/* ============================================================================
   NORTH — an independent design practice.

   The studio/agency shape: light, Swiss, type-led. An asymmetric case-study
   grid with hover reveals, a numbers strip, a client marquee, a numbered
   services index, and a close that fills the screen. Same variable font as
   the AURA launch, opposite register — the range is the point.

   Every behaviour is declared: @reveal for the type, CSS keyframes for the
   marquee, plain :hover for the case cards, @magnetic for the close.
   ============================================================================ */

@font("Aspekta") {
    src: url("/fonts/AspektaVF.woff2") format("woff2-variations"); /* BUG-353: absolute so serve and build agree */
    font-weight: 50 1000;
    font-style: normal;
    font-display: swap;
}

body {
    margin: 0;
    background: #f4f2ed;
    color: #141412;
    font-family: "Aspekta", "Inter", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
}

::selection { background: #141412; color: #f4f2ed; }

.page { position: relative; }

/* --------------------------------------------------------------------------
   Nav — a rule line, wordmark left, index right.
   -------------------------------------------------------------------------- */

.nav {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 26px 44px;
    border-bottom: 1px solid #141412;
}

.nav__brand { font-weight: 900; font-size: 17px; letter-spacing: 0.06em; }

.nav__meta {
    font-size: 11px;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: rgba(20, 20, 18, 0.55);
    display: flex;
    gap: 30px;
}

/* --------------------------------------------------------------------------
   Hero — the sentence IS the site.
   -------------------------------------------------------------------------- */

.hero {
    padding: 12vh 44px 10vh 44px;
    border-bottom: 1px solid #141412;
}

.hero__kicker {
    font-size: 12px;
    letter-spacing: 0.26em;
    text-transform: uppercase;
    color: rgba(20, 20, 18, 0.55);
    margin: 0 0 4vh 0;

    @reveal(split: "words", trigger: load, stagger: 30, duration: 500)
}

.hero__title {
    font-size: clamp(3.4rem, 9.5vw, 9.5rem);
    font-weight: 900;
    line-height: 0.98;
    letter-spacing: -0.035em;
    margin: 0;
    max-width: 12ch;

    @reveal(split: "words", trigger: load, stagger: 90, duration: 800)
}

.hero__title em {
    font-style: normal;
    font-weight: 200;
}

.hero__foot {
    display: flex;
    justify-content: space-between;
    margin-top: 8vh;
    font-size: 12px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: rgba(20, 20, 18, 0.55);
}

/* --------------------------------------------------------------------------
   Work — an asymmetric case grid. The hover is the portfolio.
   -------------------------------------------------------------------------- */

.work { padding: 10vh 44px; }

.work__head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 6vh;
}

.work__title { font-size: 13px; letter-spacing: 0.24em; text-transform: uppercase; margin: 0; }
.work__count { font-size: 13px; color: rgba(20, 20, 18, 0.45); }

.work__grid {
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    gap: 28px;
}

.case {
    position: relative;
    overflow: hidden;
    cursor: pointer;
    border: 1px solid #141412;
}

.case--a { grid-column: 1 / 8; aspect-ratio: 16 / 10; }
.case--b { grid-column: 8 / 13; aspect-ratio: 4 / 5; }
.case--c { grid-column: 1 / 6; aspect-ratio: 4 / 5; }
.case--d { grid-column: 6 / 13; aspect-ratio: 16 / 10; }

.case__art {
    position: absolute;
    inset: 0;
    transition: transform 700ms cubic-bezier(0.22, 1, 0.36, 1);
}

.case:hover .case__art { transform: scale(1.045); }

.case__num {
    position: absolute;
    top: 22px;
    left: 24px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.18em;
    color: rgba(255, 255, 255, 0.85);
}

.case__plate {
    position: absolute;
    left: 0; right: 0; bottom: 0;
    padding: 22px 24px;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    background: linear-gradient(transparent, rgba(10, 10, 12, 0.55));
    color: #fff;
    transform: translateY(8px);
    opacity: 0.92;
    transition: transform 500ms cubic-bezier(0.22, 1, 0.36, 1), opacity 500ms ease;
}

.case:hover .case__plate { transform: translateY(0); opacity: 1; }

.case__name { font-size: clamp(1.3rem, 2.2vw, 1.9rem); font-weight: 800; letter-spacing: -0.02em; }
.case__tag { font-size: 11px; letter-spacing: 0.2em; text-transform: uppercase; opacity: 0.75; }

/* Gradient art — each case gets a voice, no photography required. */
.art--fjord  { background: linear-gradient(135deg, #0e2a47 0%, #1d5c8a 45%, #7fb6c9 100%); }
.art--ember  { background: linear-gradient(160deg, #3b1208 0%, #a63d12 55%, #e8925a 100%); }
.art--moss   { background: linear-gradient(150deg, #12240f 0%, #3c5a2a 55%, #9db877 100%); }
.art--signal { background: linear-gradient(140deg, #1c1230 0%, #5b3b8c 50%, #c9a8e8 100%); }

/* --------------------------------------------------------------------------
   Numbers — proof, set huge.
   -------------------------------------------------------------------------- */

.numbers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    border-top: 1px solid #141412;
    border-bottom: 1px solid #141412;
}

.num {
    padding: 7vh 44px;
    border-right: 1px solid #141412;
}

.num:last-child { border-right: 0; }

.num__value {
    font-size: clamp(3.6rem, 8vw, 7.5rem);
    font-weight: 900;
    letter-spacing: -0.04em;
    line-height: 1;
    font-variant-numeric: tabular-nums;

    @reveal(split: "chars", stagger: 60, duration: 700)
}

.num__label {
    margin-top: 2vh;
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: rgba(20, 20, 18, 0.55);
}

/* --------------------------------------------------------------------------
   Clients — the quiet marquee.
   -------------------------------------------------------------------------- */

.clients {
    overflow: hidden;
    white-space: nowrap;
    border-bottom: 1px solid #141412;
    padding: 20px 0;
}

.clients__inner { display: inline-block; animation: clients-scroll 32s linear infinite; }

.clients__name {
    display: inline-block;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(20, 20, 18, 0.65);
    padding: 0 42px;
}

@keyframes clients-scroll {
    from { transform: translateX(0); }
    to { transform: translateX(-50%); }
}

/* --------------------------------------------------------------------------
   Services — a numbered index; hover shifts the line.
   -------------------------------------------------------------------------- */

.services { padding: 10vh 44px; }

.services__title { font-size: 13px; letter-spacing: 0.24em; text-transform: uppercase; margin: 0 0 5vh 0; }

.service {
    display: flex;
    align-items: baseline;
    gap: 34px;
    padding: 3.4vh 0;
    border-top: 1px solid rgba(20, 20, 18, 0.25);
    transition: padding-left 450ms cubic-bezier(0.22, 1, 0.36, 1);
}

.service:hover { padding-left: 22px; }

.service:last-child { border-bottom: 1px solid rgba(20, 20, 18, 0.25); }

.service__index { font-size: 12px; font-weight: 600; color: rgba(20, 20, 18, 0.45); min-width: 34px; }

.service__name {
    font-size: clamp(1.8rem, 4vw, 3.2rem);
    font-weight: 800;
    letter-spacing: -0.025em;
    line-height: 1.05;
}

.service__note {
    margin-left: auto;
    max-width: 30ch;
    font-size: 0.9rem;
    font-weight: 350;
    line-height: 1.55;
    color: rgba(20, 20, 18, 0.6);
    text-align: right;
}

/* --------------------------------------------------------------------------
   Close — the sentence again, bigger, and one magnetic promise.
   -------------------------------------------------------------------------- */

.close {
    min-height: 92vh;
    background: #141412;
    color: #f4f2ed;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0 44px;
}

.close__kicker {
    font-size: 12px;
    letter-spacing: 0.26em;
    text-transform: uppercase;
    color: rgba(244, 242, 237, 0.5);
    margin: 0 0 4vh 0;
}

.close__title {
    font-size: clamp(3.6rem, 10.5vw, 10.5rem);
    font-weight: 900;
    line-height: 0.98;
    letter-spacing: -0.035em;
    margin: 0;
    max-width: 11ch;

    @reveal(split: "words", stagger: 90, duration: 800)
}

.close__cta {
    display: inline-block;
    align-self: flex-start;
    margin-top: 7vh;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: #141412;
    background: #f4f2ed;
    padding: 20px 48px;
    border-radius: 999px;

    @magnetic(strength: 0.4, radius: 180, lerp: 0.14)
}

.footer {
    background: #141412;
    color: rgba(244, 242, 237, 0.4);
    border-top: 1px solid rgba(244, 242, 237, 0.14);
    padding: 26px 44px;
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
    <nav class="nav">
        <span class="nav__brand">NORTH</span>
        <div class="nav__meta">
            <span>Independent practice</span>
            <span>Oslo — est. 2016</span>
        </div>
    </nav>

    <section class="hero">
        <p class="hero__kicker">Brand, digital, and the space between</p>
        <h1 class="hero__title">We design brands <em>people remember.</em></h1>
        <div class="hero__foot">
            <span>Selected work below</span>
            <span>Currently booking autumn</span>
        </div>
    </section>

    <section class="work">
        <div class="work__head">
            <h2 class="work__title">Selected work</h2>
            <span class="work__count">2016 — today</span>
        </div>
        <div class="work__grid">
            <div class="case case--a">
                <div class="case__art art--fjord"></div>
                <span class="case__num">01</span>
                <div class="case__plate">
                    <span class="case__name">Fjord Bank</span>
                    <span class="case__tag">Identity — Digital</span>
                </div>
            </div>
            <div class="case case--b">
                <div class="case__art art--ember"></div>
                <span class="case__num">02</span>
                <div class="case__plate">
                    <span class="case__name">Ember Coffee</span>
                    <span class="case__tag">Packaging</span>
                </div>
            </div>
            <div class="case case--c">
                <div class="case__art art--moss"></div>
                <span class="case__num">03</span>
                <div class="case__plate">
                    <span class="case__name">Moss Architects</span>
                    <span class="case__tag">Brand — Web</span>
                </div>
            </div>
            <div class="case case--d">
                <div class="case__art art--signal"></div>
                <span class="case__num">04</span>
                <div class="case__plate">
                    <span class="case__name">Signal Festival</span>
                    <span class="case__tag">Campaign</span>
                </div>
            </div>
        </div>
    </section>

    <section class="numbers">
        <div class="num">
            <div class="num__value">120</div>
            <div class="num__label">Projects shipped</div>
        </div>
        <div class="num">
            <div class="num__value">18</div>
            <div class="num__label">International awards</div>
        </div>
        <div class="num">
            <div class="num__value">09</div>
            <div class="num__label">Years independent</div>
        </div>
    </section>

    <div class="clients">
        <div class="clients__inner">
            <span class="clients__name">Fjord Bank</span>
            <span class="clients__name">Ember Coffee</span>
            <span class="clients__name">Moss Architects</span>
            <span class="clients__name">Signal Festival</span>
            <span class="clients__name">Aurora Airlines</span>
            <span class="clients__name">Bergen Press</span>
            <span class="clients__name">Fjord Bank</span>
            <span class="clients__name">Ember Coffee</span>
            <span class="clients__name">Moss Architects</span>
            <span class="clients__name">Signal Festival</span>
            <span class="clients__name">Aurora Airlines</span>
            <span class="clients__name">Bergen Press</span>
        </div>
    </div>

    <section class="services">
        <h2 class="services__title">What we do</h2>
        <div class="service">
            <span class="service__index">01</span>
            <span class="service__name">Brand identity</span>
            <span class="service__note">Names, marks, systems — the things a company is recognised by.</span>
        </div>
        <div class="service">
            <span class="service__index">02</span>
            <span class="service__name">Digital products</span>
            <span class="service__note">Sites and tools that carry the brand instead of describing it.</span>
        </div>
        <div class="service">
            <span class="service__index">03</span>
            <span class="service__name">Art direction</span>
            <span class="service__note">Campaigns, photography, and the judgement to leave things out.</span>
        </div>
        <div class="service">
            <span class="service__index">04</span>
            <span class="service__name">Type and editorial</span>
            <span class="service__note">Custom letterforms and pages worth reading twice.</span>
        </div>
    </section>

    <section class="close">
        <p class="close__kicker">New business</p>
        <h2 class="close__title">Tell us what you are making.</h2>
        <span class="close__cta">Start a project</span>
    </section>

    <footer class="footer">
        <span>North — a Spacetime showcase</span>
        <span>Set in Aspekta</span>
    </footer>
</main>
