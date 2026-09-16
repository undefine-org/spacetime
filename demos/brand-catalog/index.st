/* ============================================================================
   Aurora — Brand Form Catalog

   A `.st`-only catalog page (zero hand-written JS) that lists the brand's
   `@form` declarations as DATA: `@data forms` enumerates every declaration
   the build can see — this page, its imports, the `_prelude.st` project
   overlay, and the stdlib preset library — and `@each` unrolls the rows into
   cards at build time. Add a form to `_prelude.st` and its card appears here
   next build; this page never names a form by hand.

   Each kind section pairs the auto-generated cards with a LIVE STAGE that
   consumes the same forms for real: style splices expand into CSS, motion
   forms drive entrances, the easing curve drives a scrub bar, the score form
   choreographs a pair. `value` and `markup` kinds are specification cards —
   their consumers land in a follow-up wave.
   ============================================================================ */

@version 2026-08-05;

/* The catalog data — one slice per kind. Rows: {name, kind, params, body,
   doc, source}. stdlib's preset library joins the easing slice. */
/* ONE unfiltered list — the browser below narrows it live, so search, both
   filters, and the count read the same rows and can never disagree. Rows:
   {name, kind, params, body, doc, source, origin}. `origin` is the LAYER a
   declaration came from (stdlib preset / project overlay / this page), which
   is the first question a reader of a 40-form list actually has. */
@data forms $forms;
@data derive $total number : $forms.length;

/* Live browser state — three signals, no JavaScript. */
@data inline $q      : "";     /* search query, two-way bound to the input */
@data inline $kind   : "all";  /* kind filter   — the chips */
@data inline $origin : "all";  /* origin filter — stdlib / project / page */

/* Per-kind slices still feed the LIVE STAGES below: each kind's section pairs
   its stage with that kind's cards, which is how a reader learns one kind at a
   time. The browser above is for finding a form you half-remember. */
@data forms $styles  from kind "style" ;
@data forms $motions from kind "motion" ;
@data forms $easings from kind "easing" ;
@data forms $scores  from kind "score" ;
@data forms $values  from kind "value" ;
@data forms $markups from kind "markup" ;

/* ===========================================================================
   LIVE STAGES — the same declarations, consumed
   =========================================================================== */

/* Style: the splice expands into plain CSS at compile time (W2). */
.stage-style .demo-card-a { --card-surface; }
.stage-style .demo-card-b { --card-surface(36px); }
.stage-style .demo-pill   { --brand-pill; }
.stage-style h3           { --section-title; }

/* Motion: entrance choreography on the brand curve. */
.demo-rise  { @on &.visible: --rise; }
.demo-rise2 { @on &.visible: --rise(48px); }

/* Easing: two scrub dots on the same driver — the brand curve vs linear.
   At any scroll position the difference IS the curve (W3). */
.stage-easing .dot-glide {
  @on &.scroll(name: glideDot, start: 0, end: 1, scrub: true) {
    translate-x: 0px -> 240px;
    opacity: 0.15 -> 1;
    easing: --aurora-glide;
  }
}
.stage-easing .dot-linear {
  @on &.scroll(name: linearDot, start: 0, end: 1, scrub: true) {
    translate-x: 0px -> 240px;
    opacity: 0.15 -> 1;
    easing: linear;
  }
}

/* Score: the --duet splice hands each subject half the scroll window. */
.stage-score {
  @score &.scroll { --duet; }
}
.stage-score .first  { @on &.clip: --rise; }
.stage-score .second { @on &.clip: --rise; }

/* ---- the live browser ---------------------------------------------------
   Search and the two filter rails are SIGNALS; `@each(... when ...)` hides
   non-matching cards and re-runs whenever any of them changes. Zero custom
   JavaScript — the same shape `demos/spacetime-docs/reference.st` proves over
   the compiler's own macro table.
   ------------------------------------------------------------------------ */

/* Two-way search binding: the input drives $q, $q re-renders the count.
   `$` is the event-target element in an @on body, so `$.value` reads it. */
.search {
  value <- $q;
  @on &.input { $q <- $.value; }
}

/* The filter rails ($ = the clicked button). */
.chip--kind   { @on &.click { $kind   <- $.dataset.kind; } }
.chip--origin { @on &.click { $origin <- $.dataset.origin; } }

/* ONE grid, every kind, narrowed live. A card matches when its kind and its
   origin pass their rails and the query appears anywhere an author would look
   for it — name, params, doc, or body. */
.cards--all {
  @each($forms as $f, when ($kind == "all" || $f.kind == $kind) && ($origin == "all" || $f.origin == $origin) && ($q == "" || ($f.name + " " + $f.params + " " + $f.doc + " " + $f.body).toLowerCase().indexOf(($q).toLowerCase()) >= 0)) {
    <article class="fcard">
      <header>
        <code class="fcard__name">`$f.name`</code>
        <code class="fcard__params">`$f.params`</code>
      </header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">
        <span class="fcard__kind">`$f.kind`</span>
        <span class="fcard__origin">`$f.origin`</span>
        <span class="fcard__path">`$f.source`</span>
      </footer>
    </article>
  }
}

/* The catalog cards: one per declaration row, unrolled at build time. The
   @each lives in a SELECTOR block (the proven reference.st shape — a
   directive nested inside HTML markup is not a directive surface). */
.cards--styles {
  @each($styles as $f) {
    <article class="fcard">
      <header><code class="fcard__name">`$f.name`</code><code class="fcard__params">`$f.params`</code></header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">`$f.source`</footer>
    </article>
  }
}
.cards--motions {
  @each($motions as $f) {
    <article class="fcard">
      <header><code class="fcard__name">`$f.name`</code><code class="fcard__params">`$f.params`</code></header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">`$f.source`</footer>
    </article>
  }
}
.cards--easings {
  @each($easings as $f) {
    <article class="fcard">
      <header><code class="fcard__name">`$f.name`</code><code class="fcard__params">`$f.params`</code></header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">`$f.source`</footer>
    </article>
  }
}
.cards--scores {
  @each($scores as $f) {
    <article class="fcard">
      <header><code class="fcard__name">`$f.name`</code><code class="fcard__params">`$f.params`</code></header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">`$f.source`</footer>
    </article>
  }
}
.cards--values {
  @each($values as $f) {
    <article class="fcard">
      <header><code class="fcard__name">`$f.name`</code><code class="fcard__params">`$f.params`</code></header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">`$f.source`</footer>
    </article>
  }
}
.cards--markups {
  @each($markups as $f) {
    <article class="fcard">
      <header><code class="fcard__name">`$f.name`</code><code class="fcard__params">`$f.params`</code></header>
      <p class="fcard__doc">`$f.doc`</p>
      <pre class="fcard__body">`$f.body`</pre>
      <footer class="fcard__src">`$f.source`</footer>
    </article>
  }
}

/* ===========================================================================
   PAGE
   =========================================================================== */

<div class="shell">

  <aside class="side">
    <p class="side__brand">Aurora<span>· brand forms</span></p>
    <nav class="side__nav">
      <a href="#overview">Overview</a>
      <a href="#browse">Browse all</a>
      <a href="#style">Style</a>
      <a href="#motion">Motion</a>
      <a href="#easing">Easing</a>
      <a href="#score">Score</a>
      <a href="#value">Value</a>
      <a href="#markup">Markup</a>
      <a href="/guidelines" class="side__ref">Guidelines →</a>
    </nav>
    <p class="side__foot">Declared once · everywhere</p>
  </aside>

  <main class="reader">

    <header class="hero" id="overview">
      <p class="hero__eyebrow">Aurora design system</p>
      <h1>The brand is a set of forms.</h1>
      <p class="hero__lede">
        Every attribute below is declared once in <code>_prelude.st</code> and
        catalogued here automatically — the cards are generated from
        <code>@data forms</code>, the stages consume the same declarations live.
      </p>
    </header>

    <!-- BROWSE ------------------------------------------------------------
         Find a form you half-remember. Every card in the tree, narrowed by
         search + two rails, live. -->
    <section class="block" id="browse">
      <h2 class="browse__h">Browse every form</h2>
      <p class="browse__lede">
        <span class="browse__n">`$total`</span> declarations — this page, the
        project overlay, and the stdlib preset library. Type to search names,
        parameters, docs and bodies at once.
      </p>

      <div class="controls">
        <input class="search" type="text" placeholder="Search forms — try &quot;rise&quot;, &quot;spring&quot;, &quot;pad&quot;…" />
      </div>

      <div class="rails">
        <div class="rail">
          <span class="rail__label">kind</span>
          <button class="chip chip--kind" data-kind="all" type="button">all</button>
          <button class="chip chip--kind" data-kind="style" type="button">style</button>
          <button class="chip chip--kind" data-kind="motion" type="button">motion</button>
          <button class="chip chip--kind" data-kind="easing" type="button">easing</button>
          <button class="chip chip--kind" data-kind="score" type="button">score</button>
          <button class="chip chip--kind" data-kind="value" type="button">value</button>
          <button class="chip chip--kind" data-kind="markup" type="button">markup</button>
        </div>
        <div class="rail">
          <span class="rail__label">declared in</span>
          <button class="chip chip--origin" data-origin="all" type="button">anywhere</button>
          <button class="chip chip--origin" data-origin="project" type="button">this brand</button>
          <button class="chip chip--origin" data-origin="stdlib" type="button">stdlib</button>
          <button class="chip chip--origin" data-origin="page" type="button">this page</button>
        </div>
      </div>

      <div class="cards cards--all"></div>
    </section>

    <!-- STYLE ------------------------------------------------------------ -->
    <section class="block" id="style">
      <div class="stage stage-style">
        <h3>Live — style splices</h3>
        <div class="stage-row">
          <div class="demo-card-a">
            <span class="demo-pill">card-surface</span>
            <p>Default padding (20px), straight from the form.</p>
          </div>
          <div class="demo-card-b">
            <span class="demo-pill">card-surface(36px)</span>
            <p>The same form with its parameter filled.</p>
          </div>
        </div>
      </div>
      <div class="cards cards--styles"></div>
    </section>

    <!-- MOTION ----------------------------------------------------------- -->
    <section class="block" id="motion">
      <div class="stage">
        <h3>Live — motion forms</h3>
        <div class="stage-row">
          <div class="demo-rise stage-chip">--rise (24px)</div>
          <div class="demo-rise2 stage-chip">--rise(48px)</div>
        </div>
        <p class="stage-note">Scroll them out and back in — each re-enters on the brand curve.</p>
      </div>
      <div class="cards cards--motions"></div>
    </section>

    <!-- EASING ------------------------------------------------------------ -->
    <section class="block" id="easing">
      <div class="stage stage-easing">
        <h3>Live — the brand curve vs linear</h3>
        <div class="track"><div class="dot dot-glide"></div></div>
        <div class="track"><div class="dot dot-linear"></div></div>
        <p class="stage-note">Same driver, same scroll. The top dot rides <code>--aurora-glide</code>; the bottom one is linear. The gap is the curve.</p>
      </div>
      <div class="cards cards--easings"></div>
    </section>

    <!-- SCORE ------------------------------------------------------------- -->
    <section class="block" id="score">
      <div class="stage stage-score">
        <h3>Live — a scored duet</h3>
        <div class="stage-row">
          <div class="first stage-chip">first</div>
          <div class="second stage-chip">second</div>
        </div>
        <p class="stage-note">Scroll: <code>--duet</code> gives each half the window.</p>
      </div>
      <div class="cards cards--scores"></div>
    </section>

    <!-- VALUE ------------------------------------------------------------- -->
    <section class="block" id="value">
      <div class="stage stage-spec">
        <h3>Specification</h3>
        <p class="stage-note">Value forms declare pure functions over brand measures. Their consumer (the <code>: --name(args)</code> value position) lands in a follow-up wave — today the card IS the deliverable: one canonical place the function is written down.</p>
      </div>
      <div class="cards cards--values"></div>
    </section>

    <!-- MARKUP ------------------------------------------------------------ -->
    <section class="block" id="markup">
      <div class="stage stage-spec">
        <h3>Specification</h3>
        <p class="stage-note">Markup forms declare canonical element chunks. Their consumer (splicing the chunk into markup) lands with the value consumer — the declaration below is the single source of truth until then.</p>
      </div>
      <div class="cards cards--markups"></div>
    </section>

  </main>
</div>

/* ===========================================================================
   SHELL STYLES
   =========================================================================== */

.shell {
  display: flex;
  min-height: 100vh;
  background: #faf9fc;
  color: #1c1926;
  font-family: system-ui, sans-serif;
}

.side {
  position: sticky;
  top: 0;
  align-self: flex-start;
  width: 240px;
  min-height: 100vh;
  padding: 32px 24px;
  border-right: 1px solid #ece8f3;
  display: flex;
  flex-direction: column;
  gap: 28px;
  background: #ffffff;
}
.side__brand { font-weight: 800; font-size: 18px; margin: 0; }
.side__brand span { color: #5b3df5; font-weight: 500; }
.side__nav { display: flex; flex-direction: column; gap: 4px; }
.side__nav a {
  color: #4d4763;
  text-decoration: none;
  padding: 7px 10px;
  border-radius: 8px;
  font-size: 14px;
}
.side__nav a:hover { background: #f4f1fa; color: #1c1926; }
.side__ref { margin-top: 12px; color: #5b3df5 !important; font-weight: 600; }
.side__foot { margin-top: auto; color: #a49fb5; font-size: 12px; }

.reader { flex: 1; padding: 48px 56px 160px; max-width: 980px; }

.hero { margin-bottom: 56px; }
.hero__eyebrow {
  color: #5b3df5;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  margin: 0 0 10px;
}
.hero h1 { font-size: 44px; letter-spacing: -0.02em; margin: 0 0 14px; }
.hero__lede { color: #5a5470; font-size: 17px; max-width: 640px; line-height: 1.6; }
.hero code, .stage-note code { background: #f1eef8; padding: 2px 6px; border-radius: 6px; font-size: 0.9em; }

.block { margin-bottom: 72px; }

.stage {
  background: #f4f2f9;
  border: 1px dashed #d9d2e8;
  border-radius: 16px;
  padding: 24px 28px;
  margin-bottom: 24px;
}
.stage h3 { margin: 0 0 16px; }
.stage-row { display: flex; gap: 20px; align-items: stretch; }
.stage-note { color: #6f6880; font-size: 13px; margin: 14px 0 0; }
.stage-chip {
  background: #ffffff;
  border: 1px solid #e7e3ee;
  border-radius: 12px;
  padding: 18px 26px;
  font-weight: 600;
  color: #39334e;
}
.demo-card-a p, .demo-card-b p { margin: 10px 0 0; color: #5a5470; font-size: 14px; }

.track {
  position: relative;
  height: 34px;
  background: #ffffff;
  border: 1px solid #e7e3ee;
  border-radius: 999px;
  margin: 10px 0;
  width: 300px;
}
.dot {
  position: absolute;
  top: 4px;
  left: 4px;
  width: 26px;
  height: 26px;
  border-radius: 999px;
}
.dot-glide { background: #5b3df5; }
.dot-linear { background: #b9b2cc; }

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}
.fcard {
  background: #ffffff;
  border: 1px solid #e7e3ee;
  border-radius: 14px;
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.fcard header { display: flex; justify-content: space-between; align-items: baseline; gap: 10px; }
.fcard__name { font-size: 15px; font-weight: 700; color: #5b3df5; }
.fcard__params { color: #a49fb5; font-size: 12px; }
.fcard__doc { color: #4d4763; font-size: 14px; line-height: 1.55; margin: 0; }
.fcard__body {
  background: #f8f6fc;
  border: 1px solid #efeaf8;
  border-radius: 10px;
  padding: 10px 12px;
  font-size: 12px;
  color: #5a5470;
  overflow-x: auto;
  margin: 0;
  white-space: pre-wrap;
}
.fcard__src { color: #b6b0c8; font-size: 11px; }

/* ---- the browser ---------------------------------------------------------
   Brand attributes, spliced: the panel is a `--card-surface`, the rail chips
   are `--brand-pill`s. The catalog is built out of the thing it catalogues.
   -------------------------------------------------------------------------- */

.browse__h { --role-title; margin: 0 0 10px; font-size: 28px; }
.browse__lede { --role-subtitle; margin: 0 0 20px; max-width: 640px; }
.browse__n { font-weight: 800; color: #5b3df5; font-variant-numeric: tabular-nums; }

.controls { margin-bottom: 14px; }
.search {
  width: 100%;
  max-width: 520px;
  font: inherit;
  font-size: 15px;
  color: #1c1926;
  background: #ffffff;
  border: 1px solid #e7e3ee;
  border-radius: 12px;
  padding: 11px 15px;
  transition: border-color 160ms ease, box-shadow 160ms ease;
}
.search:focus {
  outline: none;
  border-color: #5b3df5;
  box-shadow: 0 0 0 4px rgba(91, 61, 245, 0.12);
}
.search::placeholder { color: #a49fb5; }

.rails { display: flex; flex-direction: column; gap: 8px; margin-bottom: 22px; }
.rail { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.rail__label {
  width: 92px;
  flex: none;
  color: #a49fb5;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.chip {
  --brand-pill;
  border: 1px solid #e7e3ee;
  background: #ffffff;
  color: #4d4763;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  padding: 5px 12px;
  @on &.hover(160ms) { background: #ffffff -> #f1ebff; }
}

.fcard__src { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.fcard__kind {
  --brand-pill;
  font-size: 10px;
  padding: 2px 8px;
}
.fcard__origin {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #8b84a0;
}
.fcard__path { color: #b6b0c8; font-size: 11px; }
