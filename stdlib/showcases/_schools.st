/* ============================================================================
   Showcase schools — the design languages, declared as forms (PLAN-135 W6).

   Every showcase pattern splices these instead of hand-writing brand CSS:
   colors, type, radius, and shadow live here ONCE per school; a pattern file
   keeps only its layout (flex/grid/positioning). Because these are `@form`
   declarations, they are also DATA — `@data forms` catalogs them, and a
   project importing `stdlib/showcases` can splice the same chunks to extend
   a pattern without forking it.

   Choreography is declared the same way: one brand curve, one entrance
   motion, and score forms for sequenced heroes.
   ============================================================================ */

/* ---- ROLES: the vocabulary a pattern speaks ------------------------------

   A pattern never names a school. It splices a ROLE — `--role-surface`,
   `--role-title` — and the block below binds each role to one school's chunk.
   Switching a page's school is therefore ONE edit here, not an edit to every
   splice in every pattern.

   This works because a `@form style` body may splice another form (BUG-327):

       @form style --role-title { --sc-min-title; }   // the binding
       .hero__title { --role-title; }                 // the pattern

   To switch school, rebind the roles — copy one of the alternate blocks below
   over the active bindings. Nothing else in the tree changes.

   The roles are deliberately few and generic; a pattern that needs something
   only its school has (an editorial eyebrow rule, a metric's tabular figures)
   still splices that school chunk directly. Roles carry what EVERY school must
   answer: ground, display voice, supporting voice, and call to action.
   ------------------------------------------------------------------------- */

/// The page ground — background, ink, and type family.
@form style --role-surface { --sc-min-surface; }

/// The display voice — the largest type on the surface.
@form style --role-title { --sc-min-title; }

/// The supporting voice — the line under the title.
@form style --role-subtitle { --sc-min-subtitle; }

/// The call to action.
@form style --role-cta { --sc-min-cta; }

/* --- alternate bindings (swap these over the four above) ------------------

   experimental / maximal:
     @form style --role-surface  { --sc-max-surface; }
     @form style --role-title    { --sc-max-title; }
     @form style --role-subtitle { --sc-max-hero-subtitle; }
     @form style --role-cta      { --sc-max-cta; }

   information architecture:
     @form style --role-surface  { --sc-arch-surface; }
     @form style --role-title    { --sc-arch-title; }
     @form style --role-subtitle { --sc-min-subtitle; }
     @form style --role-cta      { --sc-min-cta; }

   motion poetics:
     @form style --role-surface  { --sc-mtn-surface; }
     @form style --role-title    { --sc-mtn-title; }
     @form style --role-subtitle { --sc-min-subtitle; }
     @form style --role-cta      { --sc-mtn-cta; }

   eastern philosophy:
     @form style --role-surface  { --sc-east-surface; }
     @form style --role-title    { --sc-east-title; }
     @form style --role-subtitle { --sc-east-subtitle; }
     @form style --role-cta      { --sc-east-cta; }
   ------------------------------------------------------------------------- */

/* ---- shared choreography ------------------------------------------------- */

/// The showcase entrance curve — fast out, long soft settle.
@form easing --sc-enter-glide { cubic-bezier(0.22, 1, 0.36, 1) }

/// Standard element entrance: rise while fading in, on the showcase curve.
@form motion --sc-enter($distance = 24px) {
  opacity: 0 -> 1;
  translate-y: $distance -> 0;
  easing: --sc-enter-glide;
}

/// Motion-poetics hero sequence: readouts, then title, then subtitle, then
/// CTA — each owning a slice of the parent window. Subjects name the
/// pattern's classes directly; the score stays choreography-only.
@form score --sc-mtn-hero {
  &showcase-hero-mtn__readouts for 25%;
  &showcase-hero-mtn__title for 35%;
  &showcase-hero-mtn__subtitle for 25%;
  &showcase-hero-mtn__cta for 15%;
}

/// Media hero sequence: the title holds the opening, the subtitle and CTA
/// trade the close — written against the video's own playhead (playback).
@form score --sc-media-hero {
  &showcase-hero-media__title for 55%;
  &showcase-hero-media__subtitle for 25%;
  &showcase-hero-media__cta for 20%;
}

/* ---- minimalism (min) ---------------------------------------------------- */

/// Calm centered surface — near-white ground, ink text, system type.
@form style --sc-min-surface {
  background: #f8fafc;
  color: #0f172a;
  font-family: system-ui, sans-serif;
}

/// Display title — tight, quiet, confident.
@form style --sc-min-title {
  font-size: clamp(2.25rem, 5vw, 4rem);
  font-weight: 600;
  letter-spacing: -0.02em;
  line-height: 1.05;
}

/// Supporting line — muted slate, relaxed measure.
@form style --sc-min-subtitle {
  color: #475569;
  font-size: 1.125rem;
  line-height: 1.65;
}

/// Primary action — blue pill, white label, small radius.
@form style --sc-min-cta {
  display: inline-block;
  background: #2563eb;
  color: #ffffff;
  border-radius: 6px;
  padding: 12px 22px;
  font-weight: 600;
  text-decoration: none;
}

/* ---- experimental / maximal (max) ---------------------------------------- */

/// High-voltage surface — deep ground, gradient glow, oversized energy.
@form style --sc-max-surface {
  background: #0b0b10;
  color: #fafafa;
  font-family: "Inter Tight", system-ui, sans-serif;
}

/// Oversized display type — the headline IS the artwork.
@form style --sc-max-title {
  font-size: clamp(3rem, 9vw, 7.5rem);
  font-weight: 800;
  letter-spacing: -0.04em;
  line-height: 0.95;
  text-transform: uppercase;
}

/// High-contrast action — inverted pill that cannot be missed.
@form style --sc-max-cta {
  display: inline-block;
  background: #fafafa;
  color: #0b0b10;
  border-radius: 999px;
  padding: 16px 32px;
  font-weight: 800;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  text-decoration: none;
}

/* ---- information architecture (arch) ------------------------------------- */

/// Data-forward surface — paper white, ruled lines, tabular calm.
@form style --sc-arch-surface {
  background: #ffffff;
  color: #111827;
  font-family: "Inter", system-ui, sans-serif;
}

/// Structural title — precise, measured, never loud.
@form style --sc-arch-title {
  font-size: clamp(2rem, 4.5vw, 3.5rem);
  font-weight: 650;
  letter-spacing: -0.015em;
  line-height: 1.1;
}

/// Metric callout — tabular figures with a hairline rule.
@form style --sc-arch-metric {
  border-top: 1px solid #e5e7eb;
  padding-top: 16px;
  font-variant-numeric: tabular-nums;
}

/* ---- motion poetics (mtn) ------------------------------------------------ */

/// Dark canvas — technical grid surface for kinetic work.
@form style --sc-mtn-surface {
  background: #0a0a0c;
  color: #f1f5f9;
  font-family: "Inter Tight", "Inter", system-ui, sans-serif;
}

/// Kinetic display type — wide, taut, alive.
@form style --sc-mtn-title {
  font-size: clamp(2.5rem, 6.5vw, 5.5rem);
  font-weight: 700;
  letter-spacing: -0.03em;
  line-height: 1.0;
}

/// Technical readout — mono microtype for telemetry rows.
@form style --sc-mtn-readout {
  font-family: ui-monospace, SFMono-Regular, monospace;
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: #94a3b8;
}

/// Ghost action — outlined pill that glows on the dark canvas.
@form style --sc-mtn-cta {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  border: 1px solid rgba(241, 245, 249, 0.32);
  border-radius: 999px;
  padding: 12px 24px;
  color: #f5f5f5;
  font-weight: 600;
  text-decoration: none;
}

/* ---- eastern philosophy (east) ------------------------------------------- */

/// Warm paper surface — generous "ma" negative space, unhurried.
@form style --sc-east-surface {
  background: #f7f3ec;
  color: #2d2a26;
  font-family: Georgia, "Times New Roman", serif;
}

/// Serif display — warm, asymmetric, quietly literary.
@form style --sc-east-title {
  font-size: clamp(2.25rem, 5.5vw, 4.5rem);
  font-weight: 500;
  letter-spacing: -0.01em;
  line-height: 1.15;
}

/// Literary supporting line — warm gray, long measure.
@form style --sc-east-subtitle {
  color: #6b655c;
  font-size: 1.0625rem;
  line-height: 1.8;
}

/// Understated action — ink underline link, no button chrome.
@form style --sc-east-cta {
  color: #2d2a26;
  font-weight: 600;
  text-decoration: underline;
  text-underline-offset: 6px;
  text-decoration-thickness: 1px;
}

/* ---- pattern chunks (school-prefixed; added by the W6 hero restyle) -------
   Brand chunks a hero pattern needs that the shared school forms don't
   carry. Prefixed with the pattern's school so parallel pattern work can't
   collide. */

/// Editorial paper surface — warm off-white, ink text (minimalism school).
@form style --sc-min-ed-surface {
  background: #fafaf7;
  color: #1a1a1a;
}

/// Editorial eyebrow — accent kicker over a hairline rule.
@form style --sc-min-ed-eyebrow {
  font-size: 0.85rem;
  font-weight: 500;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #a51d2d;
  padding-bottom: 6px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.12);
}

/// Editorial serif display — magazine authority.
@form style --sc-min-ed-title {
  font-family: Georgia, "Iowan Old Style", serif;
  font-size: clamp(2.5rem, 6vw, 5rem);
  font-weight: 400;
  line-height: 1.05;
  letter-spacing: -0.02em;
}

/// Editorial standfirst — muted, relaxed measure.
@form style --sc-min-ed-subtitle {
  font-size: 1.2rem;
  line-height: 1.6;
  color: #6b6b6b;
}

/// Editorial action base — squared, quiet chrome.
@form style --sc-min-ed-cta {
  padding: 14px 28px;
  text-decoration: none;
  font-weight: 500;
  border-radius: 0;
}

/// Editorial primary action — solid ink block.
@form style --sc-min-ed-cta-primary {
  background: #1a1a1a;
  color: #fafaf7;
}

/// Editorial secondary action — underlined text link.
@form style --sc-min-ed-cta-secondary {
  color: #1a1a1a;
  border-bottom: 1px solid #1a1a1a;
  padding: 14px 4px;
}

/// Maximal hero subtitle — bright, soft-focus support line.
@form style --sc-max-hero-subtitle {
  font-size: clamp(1.1rem, 2vw, 1.4rem);
  line-height: 1.45;
  opacity: 0.85;
}

/// Maximal hero action — accent-yellow punch pill.
@form style --sc-max-hero-cta {
  display: inline-block;
  padding: 16px 36px;
  background: #facc15;
  color: #0f172a;
  text-decoration: none;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-radius: 999px;
}

/// Architecture hero surface — warm paper, serif display voice.
@form style --sc-arch-hero-surface {
  background: #fafaf7;
  color: #1a1a1a;
  font-family: Georgia, "Times New Roman", serif;
}

/// Architecture hero title — serif headline closed by a rule.
@form style --sc-arch-hero-title {
  font-size: clamp(2.4rem, 5vw, 4rem);
  font-weight: 400;
  line-height: 1.05;
  letter-spacing: -0.01em;
  border-bottom: 1px solid #d8d8d2;
  padding-bottom: 32px;
}

/// Architecture metric number — oversized tabular figure.
@form style --sc-arch-hero-metric-number {
  font-size: clamp(3rem, 6vw, 5.5rem);
  font-variant-numeric: tabular-nums;
  line-height: 1;
  letter-spacing: -0.02em;
}

/// Architecture metric meta — mono microtype for unit + label.
@form style --sc-arch-hero-metric-meta {
  font-family: ui-monospace, SFMono-Regular, monospace;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #686868;
}

/// Eastern hero title — EB Garamond display, asymmetric measure.
@form style --sc-east-hero-title {
  font-family: "EB Garamond", Garamond, "Times New Roman", serif;
  font-weight: 400;
  font-size: clamp(2.6rem, 5.5vw, 4.8rem);
  line-height: 1.08;
  letter-spacing: -0.005em;
}

/// Eastern hero subtitle — warm gray literary measure.
@form style --sc-east-hero-subtitle {
  color: #6e6358;
  font-size: clamp(1rem, 1.3vw, 1.2rem);
  line-height: 1.7;
}

/// Eastern kicker — persimmon italic stroke.
@form style --sc-east-hero-kicker {
  font-family: "EB Garamond", Garamond, "Times New Roman", serif;
  font-style: italic;
  font-size: 1rem;
  color: #b25a3c;
  letter-spacing: 0.04em;
}

/// Eastern ink CTA — italic serif with a hand-drawn underline.
@form style --sc-east-hero-cta {
  font-family: "EB Garamond", Garamond, "Times New Roman", serif;
  font-style: italic;
  font-size: 1.05rem;
  color: #2d2a26;
  text-decoration: none;
  border-bottom: 1px solid #2d2a26;
  padding-bottom: 4px;
  transition: color 0.2s ease, border-color 0.2s ease;
}


/* ---- minimalism: chrome surfaces (added for nav/footer/features W6) ------ */

/// Frosted glass surface — translucent white over page content, blurred backdrop.
@form style --sc-min-glass {
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(8px);
}

/// Muted slate — secondary text and quiet links.
@form style --sc-min-muted {
  color: #475569;
}

