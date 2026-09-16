/* ============================================================================
   Aurora — the brand, declared once.

   Every `@form` here is a GLOBAL brand attribute: the project overlay merges
   these declarations into every page's compile (PLAN-135 W1), so any page
   can splice `--card-surface`, ease with `--aurora-glide`, or score `--duet`
   without importing anything. The catalog page (index.st) lists them via
   `@data forms` — add a declaration here and a card appears next build.

   The `///` block above each declaration is the card's copy.
   ============================================================================ */

/// Surface for elevated content — panels, cards, dialogs. One parameter:
/// `$pad`, the inner breathing room (20px default).
@form style --card-surface($pad = 20px) {
  background: #ffffff;
  border: 1px solid #e7e3ee;
  border-radius: 14px;
  padding: $pad;
  box-shadow: 0 1px 2px rgba(28, 25, 38, 0.05), 0 10px 28px rgba(28, 25, 38, 0.07);
}

/// Compact label chip for meta information — kinds, statuses, tags.
@form style --brand-pill {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 999px;
  background: #f1ebff;
  color: #5b3df5;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}

/// Section title — small-caps eyebrow with a quiet rule below.
@form style --section-title {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #6f6880;
  border-bottom: 1px solid #ece8f3;
  padding-bottom: 10px;
}

/// The one curve everything eases with — fast out, long soft settle.
/// Declare a brand curve once; every animation in the project shares it.
@form easing --aurora-glide { cubic-bezier(0.22, 1, 0.36, 1) }

/// Standard entrance: rise from below while fading in, on the brand curve.
/// `$distance` tunes the travel (24px default; use 48px for heroes).
@form motion --rise($distance = 24px) {
  opacity: 0 -> 1;
  translate-y: $distance -> 0;
  easing: --aurora-glide;
}

/// Two-beat introduction for paired elements — first, then second, each
/// owning half the parent window.
@form score --duet {
  &first for 50%;
  &second for 50%;
}

/// Scale any measure by the golden ratio — spacing, type sizes, durations.
@form value --golden($x) { result: $x * 1.618; }

/// Inline "new" marker for fresh content.
@form markup --badge { <span class="badge">New</span> }

/* ---- ROLES: the vocabulary a page speaks -------------------------------

   A page splices a ROLE — `--role-title`, never `--aurora-display` — and the
   block below binds each role to one voice. Switching the brand's voice is
   therefore ONE edit here, not an edit to every splice on every page. This
   works because a form body may splice another form (BUG-327).

   Try it: swap `--aurora-display` for `--aurora-display-loud` below, rebuild,
   and every `--role-title` on the site changes at once.
   ------------------------------------------------------------------------ */

/// The quiet display voice — the brand's default headline.
@form style --aurora-display {
  font-size: clamp(2rem, 4vw, 3.25rem);
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.08;
  color: #1c1926;
}

/// The loud display voice — launches, campaigns, anything that must shout.
@form style --aurora-display-loud {
  font-size: clamp(2.75rem, 7vw, 5.5rem);
  font-weight: 800;
  letter-spacing: -0.04em;
  line-height: 0.95;
  text-transform: uppercase;
  color: #14111f;
}

/// ROLE — the display voice. Bound to the quiet variant.
@form style --role-title { --aurora-display; }

/// ROLE — the supporting line under a title.
@form style --role-subtitle {
  color: #5a5470;
  font-size: 1.0625rem;
  line-height: 1.65;
}
