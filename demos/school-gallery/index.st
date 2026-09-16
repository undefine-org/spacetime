/* ============================================================================
   School Gallery — the same page, in five design languages.

   Every block below is written ONCE, against roles. Click a school and the
   whole gallery becomes it: the switcher writes `data-school` (a signal-bound
   attribute), the school's value block answers the roles' questions, and the
   cascade does the rest. No rebuild, no second copy of any pattern, and not a
   line of hand-written JavaScript.

   Read `_prelude.st` first: it declares the five schools, the roles they
   answer, and the entrance choreography.
   ============================================================================ */

@version 2026-08-07;

/* The active school. Every school block is keyed off this one signal. */
@data inline $school : "minimalism";

/* The catalog of what this page declares, for the footer readout. */
@data forms $forms;
@data derive $formCount number : $forms.length;

/* ---- the switch -----------------------------------------------------------
   One attribute binding is the entire mechanism: `$school` reaches the DOM as
   `data-school`, and the blocks below bind each school's values under the
   matching attribute selector. Adding a school is a value block + a button. */

.gallery { data-school <- $school; }

.gallery[data-school="minimalism"]   { --school-minimalism; }
.gallery[data-school="experimental"] { --school-experimental; }
.gallery[data-school="architecture"] { --school-architecture; }
.gallery[data-school="motion"]       { --school-motion; }
.gallery[data-school="eastern"]      { --school-eastern; }

/* The switcher buttons write the signal ($ = the clicked button). */
.switch__btn { @on &.click { $school <- $.dataset.school; } }

/* ===========================================================================
   PAGE
   =========================================================================== */

<div class="gallery">

  <nav class="switch">
    <span class="switch__label">school</span>
    <button class="switch__btn" data-school="minimalism" type="button">minimalism</button>
    <button class="switch__btn" data-school="experimental" type="button">experimental</button>
    <button class="switch__btn" data-school="architecture" type="button">architecture</button>
    <button class="switch__btn" data-school="motion" type="button">motion poetics</button>
    <button class="switch__btn" data-school="eastern" type="button">eastern</button>
  </nav>

  <!-- HERO ---------------------------------------------------------------- -->
  <section class="hero">
    <p class="hero__kicker">a gallery of design languages</p>
    <h1 class="hero__title">The same page, five ways.</h1>
    <p class="hero__lede">
      Nothing below is written twice. Every block splices ROLES — a title, a
      supporting line, an action — and the school you pick answers what those
      roles mean. Switch it and watch the page change its mind, not its markup.
    </p>
    <a class="hero__cta" href="#features">See what changed</a>
  </section>

  <hr class="rule" />

  <!-- FEATURES ------------------------------------------------------------- -->
  <section class="features" id="features">
    <h2 class="section__title">What a school decides</h2>
    <div class="features__grid">
      <article class="feature">
        <p class="feature__meta">ground</p>
        <h3 class="feature__title">Surface &amp; ink</h3>
        <p class="feature__body">
          Background, text colour, and the hairline every rule is drawn in.
          One school's calm near-white is another's deep canvas.
        </p>
      </article>
      <article class="feature">
        <p class="feature__meta">voice</p>
        <h3 class="feature__title">Display type</h3>
        <p class="feature__body">
          Family, size, weight, tracking, leading, and whether the headline
          shouts in uppercase or speaks in a serif.
        </p>
      </article>
      <article class="feature">
        <p class="feature__meta">action</p>
        <h3 class="feature__title">The call to action</h3>
        <p class="feature__body">
          A blue pill, an inverted punch, a squared block, a ghost outline, or
          an underlined phrase that refuses to be a button at all.
        </p>
      </article>
    </div>
  </section>

  <hr class="rule" />

  <!-- SPECIMEN ------------------------------------------------------------- -->
  <section class="specimen">
    <h2 class="section__title">Type specimen</h2>
    <p class="specimen__display">Aa</p>
    <p class="specimen__line">
      The quick brown fox jumps over the lazy dog — 0123456789
    </p>
    <div class="specimen__row">
      <a class="specimen__cta" href="#">Primary action</a>
      <span class="specimen__meta">meta · label · caption</span>
    </div>
  </section>

  <hr class="rule" />

  <!-- CLOSE ---------------------------------------------------------------- -->
  <footer class="close">
    <h2 class="close__title">One vocabulary, five answers.</h2>
    <p class="close__body">
      This page declares <span class="close__n">`$formCount`</span> forms: five
      schools, the roles they answer, and one entrance curve. Every section
      above splices the roles — so a sixth school is a value block and a button,
      and nothing else changes.
    </p>
    <p class="close__meta">Spacetime · school gallery</p>
  </footer>

</div>

/* ===========================================================================
   LAYOUT — the only thing that does NOT change with the school.

   Every rule here is either structure (grid, spacing, flow) or a role splice.
   No colour, no type family, no radius is written at this level: that is what
   makes one markup tree able to be five designs.
   =========================================================================== */

.gallery {
  --role-surface;
  min-height: 100vh;
  transition: background 420ms ease, color 420ms ease;
}

/* ---- switcher ---- */
.switch {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: 14px 24px;
  background: var(--ground);
  border-bottom: 1px solid var(--rule);
}
.switch__label { --role-meta; margin-right: 4px; }
.switch__btn {
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  color: var(--ink);
  background: transparent;
  border: 1px solid var(--rule);
  border-radius: 999px;
  padding: 6px 14px;
  cursor: pointer;
  transition: background 200ms ease, color 200ms ease, border-color 200ms ease;

  @on &.hover(180ms) { opacity: 1 -> 0.62; }
}

/* ---- hero ---- */
.hero {
  max-width: 960px;
  margin: 0 auto;
  padding: 96px 24px 72px;
}
.hero__kicker {
  --role-meta;
  margin: 0 0 18px;

  @on &.visible(560ms): --enter(12px);
}
.hero__title {
  --role-title;
  margin: 0 0 20px;

  @on &.visible(720ms): --enter(24px);
}
.hero__lede {
  --role-subtitle;
  max-width: 620px;
  margin: 0 0 32px;

  @on &.visible(760ms): --enter(18px);
}
.hero__cta {
  --role-cta;

  @on &.visible(800ms): --enter(14px);
}

.rule {
  --role-rule;
  max-width: 960px;
  margin: 0 auto;
}

/* ---- features ---- */
.features {
  max-width: 960px;
  margin: 0 auto;
  padding: 72px 24px;
}
.section__title {
  --role-title;
  font-size: clamp(1.5rem, 2.6vw, 2.1rem);
  margin: 0 0 36px;

  @on &.visible(620ms): --enter(16px);
}
.features__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 32px;
}
.feature__meta { --role-meta; margin: 0 0 10px; }
.feature__title {
  --role-title;
  font-size: 1.15rem;
  margin: 0 0 10px;
}
.feature__body { --role-subtitle; font-size: 0.98rem; margin: 0; }
.feature {
  @on &.visible(640ms): --enter(20px);
}

/* ---- specimen ---- */
.specimen {
  max-width: 960px;
  margin: 0 auto;
  padding: 72px 24px;
}
.specimen__display {
  --role-title;
  font-size: clamp(5rem, 18vw, 12rem);
  margin: 0 0 8px;
  line-height: 0.9;

  @on &.visible(700ms): --enter(28px);
}
.specimen__line { --role-subtitle; margin: 0 0 28px; }
.specimen__row { display: flex; align-items: center; gap: 20px; flex-wrap: wrap; }
.specimen__cta { --role-cta; }
.specimen__meta { --role-meta; }

/* ---- close ---- */
.close {
  max-width: 960px;
  margin: 0 auto;
  padding: 72px 24px 120px;
}
.close__title {
  --role-title;
  font-size: clamp(1.5rem, 2.6vw, 2.1rem);
  margin: 0 0 16px;

  @on &.visible(640ms): --enter(18px);
}
.close__body { --role-subtitle; max-width: 620px; margin: 0 0 28px; }
.close__n { font-weight: 800; color: var(--accent); font-variant-numeric: tabular-nums; }
.close__meta { --role-meta; }
