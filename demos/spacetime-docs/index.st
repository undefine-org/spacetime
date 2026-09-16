/* ============================================================================
   Spacetime Docs — a real documentation READER, written in Spacetime

   A `.st`-only page (no index.html, no hand-written JS) that READS the project's
   own testing docs. Every long-form section is a Markdown file under
   `docs/testing/*.md`, rendered to HTML at build time by the `md` module
   (`@doc(src: …)` → vendored snarkdown). The page cannot drift from the docs,
   because it IS the docs: edit a `.md`, rebuild, the reader updates.

     • Sidebar    — pure-anchor nav (zero JS), one link per doc section
     • Hero       — what Spacetime is, with a live @on-click counter as proof
                    the page is alive, not a static export
     • Sections   — @doc renders GUIDE / FIDELITY / GENERATIVE / ANIMATION /
                    AUTOMATION / CDP straight from the canonical Markdown
     • Scene      — a quiet WebGL @scene backdrop; @scroll reveals each section

   North star (mirrors the test infra): the docs ARE the feature. One source
   describes behavior; the runtime renders it. Zero npm, zero authored JS.
   ============================================================================ */

@version 2026-07-26;

@import "stdlib/text";
@import "stdlib/md";

/* ===========================================================================
   STATE — one signal proves the rendered page is live, not a static dump
   =========================================================================== */

@data inline $clicks : 0;

/* ===========================================================================
   PAGE MARKUP — first-class tag literals; text holes via the signal sigil
   =========================================================================== */

<canvas class="bg"></canvas>

<div class="shell">

  <!-- SIDEBAR (pure-anchor nav, no JS) ----------------------------------- -->
  <aside class="side">
    <p class="side__brand">Spacetime<span>· testing</span></p>
    <nav class="side__nav">
      <a href="#overview">Overview</a>
      <a href="#guide">Core grammar</a>
      <a href="#fidelity">Fidelity ladder</a>
      <a href="#generative">Generative</a>
      <a href="#animation">Animation</a>
      <a href="#automation">Automation</a>
      <a href="#cdp">CDP backend</a>
      <a href="/reference" class="side__ref">Macro reference →</a>
    </nav>
    <p class="side__foot">One source · many worlds</p>
  </aside>

  <!-- READER ------------------------------------------------------------- -->
  <main class="reader">

    <!-- HERO ------------------------------------------------------------- -->
    <header class="hero" id="overview">
      <p class="eyebrow">The self-documenting docs</p>
      <h1 class="hero__title">
        Test it. Script it. Automate it. One vocabulary.
      </h1>
      <p class="hero__lede">
        Every section below is a Markdown file in <code>docs/testing/</code>,
        rendered to HTML at build time by Spacetime's own <code>md</code> module.
        No <code>index.html</code>, no hand-written JavaScript, nothing from npm.
        Change a doc, rebuild, this page updates — it cannot drift from the truth.
      </p>
      <div class="alive">
        <button class="alive__btn" type="button">This page is alive</button>
        <span class="alive__out">proof: <span class="alive__n">`$clicks`</span> clicks</span>
      </div>
    </header>

    <!-- DOC SECTIONS — each renders a canonical Markdown file -------------- -->
    <section class="doc-card" id="guide">
      <article class="doc"><div class="doc__body" data-doc="guide"></div></article>
    </section>

    <section class="doc-card" id="fidelity">
      <article class="doc"><div class="doc__body" data-doc="fidelity"></div></article>
    </section>

    <section class="doc-card" id="generative">
      <article class="doc"><div class="doc__body" data-doc="generative"></div></article>
    </section>

    <section class="doc-card" id="animation">
      <article class="doc"><div class="doc__body" data-doc="animation"></div></article>
    </section>

    <section class="doc-card" id="automation">
      <article class="doc"><div class="doc__body" data-doc="automation"></div></article>
    </section>

    <section class="doc-card" id="cdp">
      <article class="doc"><div class="doc__body" data-doc="cdp"></div></article>
    </section>

    <footer class="foot">
      <p>Rendered from <code>docs/testing/*.md</code> by the <code>md</code> module · built with Spacetime.</p>
    </footer>

  </main>

</div>

/* ===========================================================================
   BEHAVIOR — render docs, the live counter, and the ambient scene
   =========================================================================== */

/* Each doc section renders a Markdown file. The compiler reads the file at
   build time and snarkdown turns it into HTML — the reader content is the
   canonical doc, never a hand-copied duplicate. */
[data-doc="guide"]      { @doc(src: "docs/testing/GUIDE.md") }
[data-doc="fidelity"]   { @doc(src: "docs/testing/FIDELITY_LADDER.md") }
[data-doc="generative"] { @doc(src: "docs/testing/GENERATIVE.md") }
[data-doc="animation"]  { @doc(src: "docs/testing/ANIMATION.md") }
[data-doc="automation"] { @doc(src: "docs/testing/AUTOMATION.md") }
[data-doc="cdp"]        { @doc(src: "docs/testing/CDP_BACKEND.md") }

/* The "alive" proof: one signal, mutated on click; the readout repaints. */
.alive__btn { @on &.click { $clicks <- $clicks + 1; } }

/* Hero headline balanced by the text engine (reflow-safe). */
.hero__title { @balance(lineHeight: 1.12) }

/* Ambient CSS backdrop (the old WebGL @scene engine was retired in PLAN-050;
   for live 3D see the demos/3d showcases + the stdlib/3d module). */
canvas.bg {
  background:
    radial-gradient(circle at 30% 20%, rgba(91, 33, 182, 0.30), transparent 55%),
    radial-gradient(circle at 72% 68%, rgba(14, 165, 233, 0.22), transparent 55%),
    #070710;
}

/* Scroll-staggered reveals per doc card. */
.reader {
  @on &.scroll(name: reader-reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        .doc-card {
            opacity: 0 -> 1;
            translate-y: 26px -> 0;
            easing: --ease-out-expo;
            stagger: 0.05 first;
        }

      }
}

/* ===========================================================================
   STYLE — dark glass, neon accents, and legible long-form typography.
   =========================================================================== */

@form easing --smooth { cubic-bezier(0.4, 0, 0.2, 1) }
@form easing --ease-out-expo { cubic-bezier(0.16, 1, 0.3, 1) }

*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

body {
  min-height: 100vh;
  background: #070710;
  color: #e7ebf7;
  font-family: "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
  line-height: 1.65;
  overflow-x: hidden;
}

canvas.bg { position: fixed; inset: 0; z-index: 0; pointer-events: none; }

/* ---- Shell: sidebar + reader ---- */
.shell {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: 230px minmax(0, 1fr);
  gap: clamp(1.5rem, 4vw, 3.5rem);
  width: min(1180px, 94vw);
  margin: 0 auto;
  padding: clamp(2rem, 5vh, 3.5rem) 0 5rem;
  align-items: start;
}

/* ---- Sidebar ---- */
.side { position: sticky; top: 2rem; display: grid; gap: 1.4rem; }
.side__brand { font-weight: 800; letter-spacing: -0.01em; font-size: 1.05rem; color: #f3f1ff; }
.side__brand span { color: #8b93ad; font-weight: 500; margin-left: 0.35rem; }
.side__nav { display: grid; gap: 0.15rem; }
.side__nav a {
  color: #aeb6cf;
  text-decoration: none;
  font-size: 0.92rem;
  padding: 0.42rem 0.7rem;
  border-radius: 9px;
  border-left: 2px solid transparent;
  transition: background 160ms, color 160ms, border-color 160ms;
}
.side__nav a:hover { background: rgba(255,255,255,0.05); color: #fff; border-left-color: #5eead4; }
.side__ref { margin-top: 0.5rem; color: #5eead4 !important; font-weight: 600; }
.side__foot { color: #5b6479; font-size: 0.74rem; letter-spacing: 0.08em; text-transform: uppercase; }

/* ---- Reader column ---- */
.reader { display: grid; gap: clamp(1.4rem, 3vh, 2.2rem); min-width: 0; }

.eyebrow {
  font-size: 0.72rem; letter-spacing: 0.24em; text-transform: uppercase;
  color: #a78bfa; margin-bottom: 0.9rem;
}

/* ---- Hero ---- */
.hero {
  padding: clamp(1.8rem, 4vw, 2.8rem);
  border-radius: 22px;
  background: linear-gradient(180deg, rgba(124,92,255,0.10), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.09);
}
.hero__title {
  font-size: clamp(1.9rem, 4.4vw, 3rem);
  font-weight: 800; line-height: 1.12; letter-spacing: -0.02em;
  color: #f4f2ff; text-shadow: 0 0 38px rgba(167,139,250,0.22);
  max-width: 20ch;
}
.hero__lede { margin-top: 1.2rem; max-width: 64ch; color: #aeb6cf; font-size: 1.04rem; }

.alive { margin-top: 1.6rem; display: flex; align-items: center; gap: 1rem; flex-wrap: wrap; }
.alive__btn {
  font: inherit; font-weight: 600; color: #07070f;
  background: linear-gradient(120deg, #c4b5fd, #5eead4);
  border: 0; border-radius: 12px; padding: 0.6rem 1.2rem; cursor: pointer;
  box-shadow: 0 10px 30px rgba(94,234,212,0.22);
  transition: transform 160ms cubic-bezier(0.4,0,0.2,1), box-shadow 160ms;
}
.alive__btn:hover { transform: translateY(-2px); box-shadow: 0 16px 40px rgba(94,234,212,0.32); }
.alive__btn:active { transform: translateY(0) scale(0.97); }
.alive__out { color: #aeb6cf; font-size: 0.95rem; }
.alive__n { font-weight: 800; font-variant-numeric: tabular-nums; color: #5eead4; }

/* ---- Doc cards ---- */
.doc-card {
  padding: clamp(1.6rem, 3.4vw, 2.6rem);
  border-radius: 20px;
  background: linear-gradient(180deg, rgba(255,255,255,0.05), rgba(255,255,255,0.018));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 30px 90px rgba(0,0,0,0.4);
  backdrop-filter: blur(14px);
  scroll-margin-top: 1.5rem;
}

/* ---- Rendered Markdown typography (snarkdown output) ---- */
.doc__body { color: #c8cee3; font-size: 1.02rem; }
.doc__body h1 {
  font-size: clamp(1.5rem, 3vw, 2rem); font-weight: 800; letter-spacing: -0.015em;
  color: #f4f2ff; line-height: 1.2; margin-bottom: 0.4rem;
}
.doc__body h2 {
  font-size: 1.3rem; font-weight: 700; color: #e8e4ff;
  margin: 2rem 0 0.6rem; padding-top: 1.2rem; border-top: 1px solid rgba(255,255,255,0.07);
}
.doc__body h3 { font-size: 1.08rem; font-weight: 700; color: #d9d4f5; margin: 1.4rem 0 0.4rem; }
.doc__body strong { color: #eef1fb; font-weight: 700; }
.doc__body em { color: #cdd3e8; }
.doc__body a { color: #5eead4; text-decoration: none; border-bottom: 1px solid rgba(94,234,212,0.35); }
.doc__body a:hover { border-bottom-color: #5eead4; }

.doc__body code {
  font-family: "JetBrains Mono", ui-monospace, "SF Mono", monospace;
  font-size: 0.88em; color: #5eead4;
  background: rgba(45,212,191,0.10); padding: 0.08em 0.4em; border-radius: 6px;
}

.doc__body pre.code {
  margin: 1rem 0; padding: 1.1rem 1.3rem; border-radius: 14px;
  background: #0a0a16; border: 1px solid rgba(255,255,255,0.08);
  overflow-x: auto; line-height: 1.55;
}
.doc__body pre.code code {
  color: #cfd6ee; background: none; padding: 0; font-size: 0.85rem;
  white-space: pre; display: block;
}

.doc__body ul, .doc__body ol { margin: 0.6rem 0 1rem 1.3rem; }
.doc__body li { margin: 0.3rem 0; }

.doc__body blockquote {
  margin: 1rem 0; padding: 0.7rem 1.1rem;
  border-left: 3px solid #a78bfa;
  background: rgba(167,139,250,0.08); border-radius: 0 10px 10px 0;
  color: #c2bce0;
}

/* ---- Footer ---- */
.foot { text-align: center; color: #5b6479; font-size: 0.85rem; padding-top: 1rem; }

/* ---- Responsive: collapse sidebar above the reader on narrow screens ---- */
@media (max-width: 760px) {
  .shell { grid-template-columns: 1fr; }
  .side { position: static; }
  .side__nav { grid-auto-flow: column; grid-auto-columns: max-content; overflow-x: auto; }
}
