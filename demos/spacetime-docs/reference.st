/* ============================================================================
   Spacetime Macro Reference — a living map of the dispatch machine

   A `.st`-only page (no index.html, no authored JS) whose cards ARE the
   compiler's own macro forms. `@data dispatch $macros` hands every %form-carrying
   macro to @each as { directive, name, signature, doc, kind, scopes, binds,
   overloads } — the exact forms the predicate-dispatch scorer reads. Search and
   filter run on live signals via @each-filtered (reactive display:none); zero
   custom JavaScript. Add a %macro to the stdlib and a card appears next build —
   the reference cannot drift from the language.
   ============================================================================ */


@import "stdlib/text";
@import "stdlib/dispatch";

/* ---- live UI state ---- */
@data inline $q : "";            /* search query (two-way bound to the input) */
@data inline $kind : "all";       /* active kind filter */

/* ---- the catalog: the compiler's macro forms, as data ---- */
@data dispatch $macros;
@data derive $count number : $macros.length;

/* ===========================================================================
   PAGE
   =========================================================================== */

<canvas class="bg"></canvas>

<div class="wrap">

  <header class="head">
    <p class="eyebrow">Spacetime · macro reference</p>
    <h1 class="title">Every directive. Every form. Live.</h1>
    <p class="lede">
      These cards are the compiler's own dispatch table. Each one is a
      <code>%macro</code> form — the exact pattern Spacetime scores when it
      resolves a directive call. Search and filter are signals; nothing here is
      hand-written JavaScript.
    </p>
    <div class="controls">
      <input class="search" type="text" placeholder="Search directives, forms…" />
      <span class="count"><span class="count__n">`$count`</span> macros</span>
    </div>
    <div class="chips">
      <button class="chip" data-kind="all" type="button">all</button>
      <button class="chip" data-kind="data" type="button">data</button>
      <button class="chip" data-kind="events" type="button">events</button>
      <button class="chip" data-kind="scene" type="button">scene</button>
      <button class="chip" data-kind="animation" type="button">animation</button>
      <button class="chip" data-kind="motion" type="button">motion</button>
      <button class="chip" data-kind="text" type="button">text</button>
      <button class="chip" data-kind="test" type="button">test</button>
    </div>
  </header>

  <!-- LIVE DISPATCH PROBE -->
  <section class="probe-panel">
    <h2 class="probe-panel__title">Watch dispatch resolve</h2>
    <p class="probe-panel__lede">
      Type a directive call. The compiler's own scorer resolves it against every
      candidate form — the winner glows, losers show why they lost. Change the
      keyword and watch the winner change.
    </p>
    <input class="probe" type="text" value="@data fetch $api : &quot;/users&quot;;" spellcheck="false" />
    <div class="probe-table"></div>
  </section>

  <section class="grid"></section>

  <footer class="foot">
    <p>Generated from the live macro registry · one source, the language documents itself.</p>
  </footer>

</div>

/* ===========================================================================
   BEHAVIOR
   =========================================================================== */

/* Two-way search binding: the input drives $q, $q re-renders the readout.
   `$` is the event-target element in an @on body, so `$.value` reads the input. */
.search {
  value <- $q;
  @on &.input { $q <- $.value; }
}

/* Kind chips set the filter signal ($ = the clicked chip element). */
.chip { @on &.click { $kind <- $.dataset.kind; } }

/* The live dispatch probe: typing resolves the call over the dev endpoint. */
.probe { @dispatch-probe(out: ".probe-table") }

/* The card list: one card per macro, reactively filtered by search + kind.
   @each-filtered hides non-matching cards (display:none) and re-runs whenever
   $q or $kind changes — live search, zero custom JS. The `when` expression is
   evaluated against each row $m. */
.grid {
  @each($macros as $m, when ($kind == "all" || $m.kind == $kind) && ($q == "" || ($m.directive + " " + $m.name + " " + $m.signature).toLowerCase().indexOf(($q).toLowerCase()) >= 0)) {
    <article class="card" data-kind="`$m.kind`">
      <header class="card__top">
        <span class="card__dir">`$m.directive`</span>
        <span class="card__kind">`$m.kind`</span>
      </header>
      <code class="card__sig">@each($m.tokens as $t) { <span class="tok tok--`$t.role`">`$t.text`</span> }</code>
      <p class="card__doc">`$m.doc`</p>
      <footer class="card__meta">
        <span class="card__name">`$m.name`</span>
      </footer>
    </article>
  }
}

/* Hero balanced by the text engine. */
.title { @balance(lineHeight: 1.1) }

/* Ambient CSS backdrop (the old WebGL @scene engine was retired in PLAN-050). */
canvas.bg {
  background:
    radial-gradient(circle at 28% 18%, rgba(91, 33, 182, 0.28), transparent 55%),
    radial-gradient(circle at 74% 70%, rgba(45, 212, 191, 0.18), transparent 55%),
    #070710;
}

/* Card hover lift. */
.card {
  @on &.hover(name: card-lift, duration: 280ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -5px;
        easing: --smooth;
      }
}

/* ===========================================================================
   STYLE
   =========================================================================== */

@form easing --smooth { cubic-bezier(0.4, 0, 0.2, 1) }

*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

body {
  min-height: 100vh;
  background: #070710;
  color: #e7ebf7;
  font-family: "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
  line-height: 1.6;
  overflow-x: hidden;
}

canvas.bg { position: fixed; inset: 0; z-index: 0; pointer-events: none; }

.wrap {
  position: relative;
  z-index: 1;
  width: min(1100px, 94vw);
  margin: 0 auto;
  padding: clamp(2.5rem, 6vh, 4.5rem) 0 5rem;
}

.eyebrow {
  font-size: 0.72rem; letter-spacing: 0.24em; text-transform: uppercase;
  color: #a78bfa; margin-bottom: 0.9rem;
}
.title {
  font-size: clamp(2rem, 5vw, 3.2rem); font-weight: 800; line-height: 1.1;
  letter-spacing: -0.02em; color: #f4f2ff; text-shadow: 0 0 38px rgba(167,139,250,0.22);
  max-width: 18ch;
}
.lede { margin-top: 1.1rem; max-width: 62ch; color: #aeb6cf; font-size: 1.04rem; }

/* ---- Controls ---- */
.controls { margin-top: 1.8rem; display: flex; align-items: center; gap: 1rem; flex-wrap: wrap; }
.search {
  flex: 1 1 320px;
  font: inherit; font-size: 1rem; color: #eef1fb;
  background: rgba(255,255,255,0.05);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 12px; padding: 0.7rem 1rem;
  transition: border-color 160ms, background 160ms;
}
.search:focus { outline: none; border-color: #5eead4; background: rgba(94,234,212,0.06); }
.search::placeholder { color: #6f7891; }
.count { color: #aeb6cf; font-size: 0.92rem; }
.count__n { font-weight: 800; color: #5eead4; font-variant-numeric: tabular-nums; }

.chips { margin-top: 1rem; display: flex; gap: 0.5rem; flex-wrap: wrap; }
.chip {
  font: inherit; font-size: 0.82rem; color: #aeb6cf;
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 999px; padding: 0.34rem 0.9rem; cursor: pointer;
  transition: border-color 140ms, color 140ms, background 140ms;
}
.chip:hover { color: #fff; border-color: rgba(94,234,212,0.5); }

/* ---- Live dispatch probe ---- */
.probe-panel {
  margin-top: 2.2rem;
  padding: clamp(1.4rem, 3vw, 2rem);
  border-radius: 18px;
  background: linear-gradient(180deg, rgba(124,92,255,0.10), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.09);
}
.probe-panel__title { font-size: 1.3rem; font-weight: 700; color: #e8e4ff; }
.probe-panel__lede { margin: 0.5rem 0 1.1rem; max-width: 64ch; color: #aeb6cf; font-size: 0.95rem; }
.probe {
  width: 100%;
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 0.95rem; color: #9ff3e0;
  background: #0a0a16; border: 1px solid rgba(255,255,255,0.12);
  border-radius: 12px; padding: 0.75rem 1rem;
}
.probe:focus { outline: none; border-color: #5eead4; }
.probe-table { margin-top: 1rem; display: grid; gap: 0.3rem; }
.probe__dir { color: #a78bfa; font-family: "JetBrains Mono", ui-monospace, monospace; font-size: 0.85rem; margin-bottom: 0.3rem; }
.probe__row {
  display: grid; grid-template-columns: 3.5rem 12rem 1fr; gap: 0.6rem; align-items: baseline;
  padding: 0.4rem 0.6rem; border-radius: 8px;
  font-family: "JetBrains Mono", ui-monospace, monospace; font-size: 0.76rem;
  border: 1px solid transparent;
}
.probe__row--win { background: rgba(94,234,212,0.10); border-color: rgba(94,234,212,0.4); }
.probe__row--out { opacity: 0.5; }
.probe__score { font-weight: 800; color: #5eead4; }
.probe__row--out .probe__score { color: #f87171; }
.probe__macro { color: #c4b5fd; }
.probe__terms { display: flex; flex-wrap: wrap; gap: 0.3rem 0.6rem; color: #7c8aa5; }
.probe__term--literal-miss { color: #f87171; }
.probe__term--missing { color: #fb923c; }
.probe__term--type-mismatch { color: #fb923c; }
.probe__amb { color: #fbbf24; font-size: 0.82rem; margin-top: 0.4rem; }
.probe__margin { color: #6f7891; font-size: 0.8rem; margin-top: 0.4rem; }
.probe__err { color: #6f7891; font-size: 0.85rem; }

/* ---- Grid ---- */
.grid {
  margin-top: 2.2rem;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(310px, 1fr));
  gap: 1rem;
}

.card {
  display: flex; flex-direction: column; gap: 0.7rem;
  padding: 1.2rem 1.3rem;
  border-radius: 16px;
  background: linear-gradient(180deg, rgba(255,255,255,0.05), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 20px 60px rgba(0,0,0,0.35);
  backdrop-filter: blur(10px);
  transition: border-color 200ms;
}
.card:hover { border-color: rgba(94,234,212,0.4); }

.card__top { display: flex; align-items: center; justify-content: space-between; gap: 0.6rem; }
.card__dir {
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-weight: 700; font-size: 0.95rem; color: #c4b5fd;
}
.card__kind {
  font-size: 0.6rem; letter-spacing: 0.12em; text-transform: uppercase;
  color: #8a93ad; border: 1px solid rgba(255,255,255,0.12);
  border-radius: 999px; padding: 0.12em 0.6em;
}
.card__sig {
  display: block;
  font-family: "JetBrains Mono", ui-monospace, "SF Mono", monospace;
  font-size: 0.82rem; line-height: 1.7;
  background: #0a0a16; border: 1px solid rgba(255,255,255,0.07);
  border-radius: 10px; padding: 0.7rem 0.85rem;
  overflow-x: auto; word-break: break-word;
}
/* Each signature token coloured by its dispatch role (FEAT-089 tokens). */
.tok { white-space: pre; }
.tok + .tok { margin-left: 0.34ch; }
.tok--directive { color: #c4b5fd; font-weight: 700; }
.tok--binding   { color: #5eead4; }
.tok--type      { color: #7c8aa5; }
.tok--literal   { color: #f0abfc; font-weight: 600; }
.tok--punct     { color: #5b6479; }
.tok--param     { color: #93c5fd; }
.tok--delim     { color: #5b6479; }
.card__doc { font-size: 0.9rem; color: #c2c9de; line-height: 1.5; }
.card__meta { margin-top: auto; }
.card__name { font-size: 0.72rem; color: #6f7891; font-family: "JetBrains Mono", ui-monospace, monospace; }

.foot { margin-top: 3rem; text-align: center; color: #5b6479; font-size: 0.85rem; }
