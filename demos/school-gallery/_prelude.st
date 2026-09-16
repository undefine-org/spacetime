/* ============================================================================
   School Gallery — five design languages, one page, switched live.

   THE IDEA. A pattern never names a school. It splices a ROLE — `--role-title`,
   `--role-surface` — and a school BINDS those roles to its own chunks. Swapping
   school is therefore a rebinding, not an edit to every splice in every
   pattern (PLAN-139 W2, which needed BUG-327: a form body may splice a form).

   Compile-time that is one edit. This page goes further and makes it LIVE: the
   roles resolve through CSS custom properties, and a `data-school` attribute
   bound to a signal repoints every one of them at once. Same idea, same
   vocabulary, no rebuild — click a school and the whole gallery becomes it.

   WHY CUSTOM PROPERTIES HERE. A `@form style` splice is resolved by the
   compiler, so a page can carry many schools but only one BINDING. To switch
   at runtime the choice has to survive into the cascade, and a custom property
   is the one mechanism CSS gives you for "a value my descendants read, which I
   can change from above". So each school declares its VALUES as properties,
   and the role forms read them. The forms stay the vocabulary; the properties
   carry the answer.
   ============================================================================ */

/* ---- the schools, as value sets ------------------------------------------
   Each block answers the same questions. Adding a school means adding one
   block and one button — nothing else on the page changes. */

/// Minimalism — calm ground, tight sans display, quiet confidence.
@form style --school-minimalism {
  --ground: #f8fafc;
  --ink: #0f172a;
  --muted: #475569;
  --accent: #2563eb;
  --text-family: system-ui, -apple-system, sans-serif;
  --display-family: system-ui, sans-serif;
  --display-size: clamp(2.25rem, 5vw, 4rem);
  --display-weight: 600;
  --display-tracking: -0.02em;
  --display-leading: 1.05;
  --display-transform: none;
  --cta-radius: 6px;
  --cta-pad: 12px 22px;
  --cta-ground: #2563eb;
  --cta-ink: #ffffff;
  --cta-border: 1px solid transparent;
  --cta-decoration: none;
  --rule: rgba(15, 23, 42, 0.1);
}

/// Experimental — deep ground, oversized uppercase display, high voltage.
@form style --school-experimental {
  --ground: #0b0b10;
  --ink: #fafafa;
  --muted: #a1a1aa;
  --accent: #facc15;
  --text-family: "Inter Tight", system-ui, sans-serif;
  --display-family: "Inter Tight", system-ui, sans-serif;
  --display-size: clamp(3rem, 9vw, 7rem);
  --display-weight: 800;
  --display-tracking: -0.04em;
  --display-leading: 0.95;
  --display-transform: uppercase;
  --cta-radius: 999px;
  --cta-pad: 16px 32px;
  --cta-ground: #facc15;
  --cta-ink: #0b0b10;
  --cta-border: 1px solid transparent;
  --cta-decoration: none;
  --rule: rgba(250, 250, 250, 0.16);
}

/// Architecture — paper white, measured serif, ruled and tabular.
@form style --school-architecture {
  --ground: #ffffff;
  --ink: #111827;
  --muted: #6b7280;
  --accent: #111827;
  --text-family: "Inter", system-ui, sans-serif;
  --display-family: Georgia, "Times New Roman", serif;
  --display-size: clamp(2rem, 4.5vw, 3.5rem);
  --display-weight: 400;
  --display-tracking: -0.01em;
  --display-leading: 1.1;
  --display-transform: none;
  --cta-radius: 0px;
  --cta-pad: 14px 28px;
  --cta-ground: #111827;
  --cta-ink: #ffffff;
  --cta-border: 1px solid transparent;
  --cta-decoration: none;
  --rule: #e5e7eb;
}

/// Motion poetics — technical dark canvas, taut kinetic type, ghost actions.
@form style --school-motion {
  --ground: #0a0a0c;
  --ink: #f1f5f9;
  --muted: #94a3b8;
  --accent: #5eead4;
  --text-family: "Inter", system-ui, sans-serif;
  --display-family: "Inter Tight", "Inter", system-ui, sans-serif;
  --display-size: clamp(2.5rem, 6.5vw, 5.5rem);
  --display-weight: 700;
  --display-tracking: -0.03em;
  --display-leading: 1;
  --display-transform: none;
  --cta-radius: 999px;
  --cta-pad: 12px 24px;
  --cta-ground: transparent;
  --cta-ink: #f5f5f5;
  --cta-border: 1px solid var(--rule);
  --cta-decoration: none;
  --rule: rgba(241, 245, 249, 0.32);
}

/// Eastern philosophy — warm paper, literary serif, generous negative space.
@form style --school-eastern {
  --ground: #f7f3ec;
  --ink: #2d2a26;
  --muted: #6b655c;
  --accent: #b25a3c;
  --text-family: Georgia, "Times New Roman", serif;
  --display-family: "EB Garamond", Garamond, "Times New Roman", serif;
  --display-size: clamp(2.6rem, 5.5vw, 4.8rem);
  --display-weight: 400;
  --display-tracking: -0.005em;
  --display-leading: 1.12;
  --display-transform: none;
  --cta-radius: 0px;
  --cta-pad: 8px 2px;
  --cta-ground: transparent;
  --cta-ink: #2d2a26;
  --cta-border: none;
  --cta-decoration: underline 1px;
  --rule: rgba(45, 42, 38, 0.18);
}

/* ---- the ROLES: the vocabulary every pattern speaks -----------------------
   A pattern splices these and nothing else. They read the active school's
   values, so the pattern is written once and every school renders it. */

/// The page ground, its ink, and the family everything that is NOT display
/// type is set in. A school that only owned its headline would leak the
/// browser default into every paragraph, label and button — which is most of
/// the page.
@form style --role-surface {
  background: var(--ground);
  color: var(--ink);
  font-family: var(--text-family);
}

/// The display voice — the largest type on the surface.
@form style --role-title {
  font-family: var(--display-family);
  font-size: var(--display-size);
  font-weight: var(--display-weight);
  letter-spacing: var(--display-tracking);
  line-height: var(--display-leading);
  text-transform: var(--display-transform);
  color: var(--ink);
}

/// The supporting line under a title.
@form style --role-subtitle {
  color: var(--muted);
  font-size: clamp(1.05rem, 1.8vw, 1.25rem);
  line-height: 1.65;
}

/// The call to action. A school decides not just the colours but whether an
/// action is a BUTTON at all: `--cta-border` and `--cta-decoration` let the
/// eastern school render an underlined phrase where the experimental school
/// renders a punch pill, without either needing its own rule here.
@form style --role-cta {
  display: inline-block;
  background: var(--cta-ground);
  color: var(--cta-ink);
  border: var(--cta-border);
  border-radius: var(--cta-radius);
  padding: var(--cta-pad);
  font-weight: 600;
  text-decoration: var(--cta-decoration);
  text-underline-offset: 6px;
}

/// A hairline divider in the school's own key.
@form style --role-rule {
  border: none;
  border-top: 1px solid var(--rule);
}

/// Small meta type — labels, kickers, captions.
@form style --role-meta {
  color: var(--muted);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

/* ---- choreography, declared once ----------------------------------------- */

/// The gallery's entrance curve — fast out, long soft settle.
@form easing --gallery-glide { cubic-bezier(0.22, 1, 0.36, 1) }

/// Standard entrance: rise while fading in, on the gallery curve.
@form motion --enter($distance = 20px) {
  opacity: 0 -> 1;
  translate-y: $distance -> 0;
  easing: --gallery-glide;
}
