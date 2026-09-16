/// Workbench visual system — warm-dark "organic interface birthplace" (FEAT-130).
///
/// Pure Spacetime/CSS: no custom page JavaScript. The palette lives as CSS custom
/// properties on `body` (Spacetime's grammar rejects a top-level `:root` selector
/// — FUP-065; `body` is the document root for inheritance, so `var()` resolves
/// everywhere). The palette is the warm-dark studio (espresso + ember, never
/// blue-black); the topology is a dominant glowing STAGE with chrome receded into
/// edge drawers summoned by warm glyphs. Mirrors
/// @research/mcp-design-language-organic-birthplace.org.

* { box-sizing: border-box; }
body {
  /* Warm-dark palette ("studio at night by a warm lamp"). */
  --mcp-bg: #1c1714;            /* deep warm espresso (page) */
  --mcp-stage: #241e19;          /* warm lit stage surface */
  --mcp-stage-glow: rgba(224, 145, 90, 0.08);
  --mcp-drawer-bg: #1f1a16;
  --mcp-cream: #f0e6d6;          /* text primary */
  --mcp-taupe: #b8a98f;          /* text secondary */
  --mcp-umber: #7a6b54;          /* text muted (decorative — fails AA on dark) */
  --mcp-umber-aa: #a8977a;       /* PLAN-064 U5: muted text that PASSES WCAG AA */
  --mcp-ember: #e0915a;          /* accent */
  --mcp-ember-hot: #ed9f6a;      /* accent hover */
  --mcp-panel: rgba(58, 48, 42, 0.32);
  --mcp-panel-strong: rgba(58, 48, 42, 0.5);
  --mcp-shadow: rgba(80, 55, 30, 0.25);

  /* Origin chip palette (muted, warm-harmonized). */
  --mcp-origin-stdlib: #6b9a92;  /* dusty teal — foundation */
  --mcp-origin-env: #8aa86a;     /* sage — living project */
  --mcp-origin-agent: #d4965a;   /* warm amber — sent to you */
  --mcp-origin-kit: #b87a82;     /* muted rose — library */

  --mcp-serif: "Fraunces", "Source Serif 4", Georgia, "Times New Roman", serif;
  --mcp-sans: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --mcp-mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;

  margin: 0;
  color-scheme: dark;
  background: var(--mcp-bg);
  color: var(--mcp-cream);
  font: 15px/1.6 var(--mcp-sans);
}

/* ── Shell: the stage fills the canvas; drawers + rails overlay it. ───────── */
.mcp-stage-shell {
  position: relative;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ── Topbar: slim, warm, minimal. ────────────────────────────────────────── */
.mcp-topbar {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 18px 28px;
  position: relative;
  z-index: 3;
}
.mcp-wordmark {
  font-family: var(--mcp-serif);
  font-size: 26px;
  font-weight: 600;
  letter-spacing: 0.01em;
  color: var(--mcp-cream);
}
.mcp-host-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  border-radius: 999px;
  background: rgba(58, 48, 42, 0.45);
  color: var(--mcp-taupe);
  font-family: var(--mcp-mono);
  font-size: 13px;
}
.mcp-host-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--mcp-origin-stdlib); }
.mcp-live {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--mcp-ember);
  font-family: var(--mcp-mono);
  font-size: 13px;
}
.mcp-ember {
  width: 9px; height: 9px;
  border-radius: 50%;
  background: var(--mcp-ember);
  box-shadow: 0 0 10px 2px rgba(224, 145, 90, 0.6);
  animation: mcp-pulse 2.4s ease-in-out infinite;
}
@keyframes mcp-pulse {
  0%, 100% { opacity: 0.55; transform: scale(0.92); }
  50%      { opacity: 1;    transform: scale(1.08); }
}

/* ── Stage: the dominant warm-glowing surface. ───────────────────────────── */
.mcp-stage {
  flex: 1;
  margin: 8px 88px 64px;
  border-radius: 28px;
  background:
    radial-gradient(120% 90% at 50% 38%, var(--mcp-stage-glow), transparent 70%),
    var(--mcp-stage);
  box-shadow: 0 24px 70px -30px var(--mcp-shadow);
  position: relative;
  overflow: hidden;
  z-index: 1;
  transition: transform 360ms cubic-bezier(0.22, 0.61, 0.36, 1), filter 360ms ease;
}
/* The stage region is a layout-NEUTRAL mount boundary: a region-mounted bundle
   is the direct child and owns its OWN layout. The workbench must not impose
   grid-centering or flex on mounted content (W4 reviewer P2), so we add NO
   layout here. `color-scheme: normal` keeps guest form controls light. */
.mcp-stage[data-st-region="stage"] {
  color-scheme: normal;
}
/* ── House-style baseline (M1 / PLAN-046, D-Q1) ─────────────────────────
   Newborn content composed onto the stage should look INTENTIONAL, not like a
   1995 browser. Every rule is wrapped WHOLE in `:where()` so it carries ZERO
   specificity (0,0,0): a guest's own rule — even a bare `h1 {}` at (0,0,1), and
   even though guest bundle CSS is `@scope`-wrapped — always wins, with no
   specificity war and no `!important`. This is OPT-OUT house style: readable by
   default, fully overridable. It is a READABLE NEUTRAL (system-font stack + warm
   off-white that reads on the dark stage + a sane type scale), deliberately NOT
   the chrome identity (no Inter, no `--mcp-cream`): the workbench dresses guests
   legibly without branding them. Typography + color ONLY — never layout.
   (Spacetime's CSS grammar parses a pseudo-led scope selector since the M1 work;
   wrapping the WHOLE selector — not just the element — is what zeroes the
   attr+class specificity that would otherwise clobber the empty-state serif and
   guest styles.) */
body {
  --mcp-guest-ink: #ddd5c8;       /* readable warm off-white, distinct from chrome cream */
  --mcp-guest-ink-soft: #b3ab9d;  /* secondary guest text */
  --mcp-guest-sans: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
:where(.mcp-stage[data-st-region="stage"] *) { font-family: var(--mcp-guest-sans); }
:where(.mcp-stage[data-st-region="stage"] h1, .mcp-stage[data-st-region="stage"] h2, .mcp-stage[data-st-region="stage"] h3, .mcp-stage[data-st-region="stage"] h4, .mcp-stage[data-st-region="stage"] span, .mcp-stage[data-st-region="stage"] label, .mcp-stage[data-st-region="stage"] td, .mcp-stage[data-st-region="stage"] th, .mcp-stage[data-st-region="stage"] dt, .mcp-stage[data-st-region="stage"] dd, .mcp-stage[data-st-region="stage"] figcaption, .mcp-stage[data-st-region="stage"] blockquote) { color: var(--mcp-guest-ink); line-height: 1.5; }
:where(.mcp-stage[data-st-region="stage"] h1) { font-size: 2rem;   font-weight: 650; line-height: 1.2; margin: 0 0 0.5em; }
:where(.mcp-stage[data-st-region="stage"] h2) { font-size: 1.5rem; font-weight: 600; line-height: 1.25; margin: 0 0 0.5em; }
:where(.mcp-stage[data-st-region="stage"] h3) { font-size: 1.2rem; font-weight: 600; margin: 0 0 0.4em; }
:where(.mcp-stage[data-st-region="stage"] p, .mcp-stage[data-st-region="stage"] ul, .mcp-stage[data-st-region="stage"] ol) { margin: 0 0 0.8em; }
:where(.mcp-stage[data-st-region="stage"] p, .mcp-stage[data-st-region="stage"] li) { color: var(--mcp-guest-ink-soft); line-height: 1.5; }
:where(.mcp-stage[data-st-region="stage"] a) { color: var(--mcp-ember); text-decoration: none; }
:where(.mcp-stage[data-st-region="stage"] a:hover) { text-decoration: underline; }
:where(.mcp-stage[data-st-region="stage"] button) { font-family: var(--mcp-guest-sans); font-size: 0.9rem; color: var(--mcp-guest-ink); background: var(--mcp-panel); border: 1px solid var(--mcp-panel-strong); border-radius: 10px; padding: 8px 16px; cursor: pointer; }
:where(.mcp-stage[data-st-region="stage"] input, .mcp-stage[data-st-region="stage"] textarea, .mcp-stage[data-st-region="stage"] select) { font-family: var(--mcp-guest-sans); font-size: 0.9rem; color: var(--mcp-guest-ink); background: rgba(0,0,0,0.18); border: 1px solid var(--mcp-panel-strong); border-radius: 10px; padding: 8px 12px; }
/* M3' selection ring: the selected stage frame gets an ember outline + a soft
   lift, the Figma "this is selected" affordance. Pure reactive class (.is-selected
   ← $selected == "stage"). The ring sits inside overflow:hidden so use an inset
   box-shadow rather than outline (which would be clipped at the rounded corners). */
.mcp-stage[data-st-region="stage"].is-selected {
  box-shadow: 0 24px 70px -30px var(--mcp-shadow), inset 0 0 0 2px var(--mcp-ember);
}
/* ── PLAN-051 B0: breakpoint switch + device-width stage frame. ────────────
   The topbar control sets $breakpoint; `data-breakpoint` on the shell constrains
   the stage's max-width so the guest re-lays-out at a device width. The region
   mount is untouched (width only); desktop = full, tablet/mobile = centered. */
.mcp-breakpoints {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border-radius: 999px;
  background: rgba(58, 48, 42, 0.4);
}
.mcp-bp {
  display: inline-flex; align-items: center; justify-content: center;
  width: 30px; height: 26px;
  border: none; border-radius: 999px;
  background: transparent; color: var(--mcp-umber);
  cursor: pointer;
  transition: color 200ms ease, background 200ms ease;
}
.mcp-bp:hover { color: var(--mcp-taupe); }
.mcp-bp svg { display: block; }
.mcp-bp.is-active {
  color: var(--mcp-ember);
  background: rgba(224, 145, 90, 0.14);
}
/* Device-width constraint on the stage. Width only (margin-inline auto centers
   the narrowed frame); the vertical margins + glow + region mount are intact.
   The transition rides the stage's existing transform/filter transition.
   PLAN-064 B6: TRUE device widths — tablet = iPad portrait (768px), mobile =
   modern phone (390px, iPhone 14-class). The width is what makes the guest
   re-lay-out at the device, so it must match a real viewport. */
.mcp-stage-shell[data-breakpoint="tablet"] .mcp-stage {
  max-width: 768px;
  margin-inline: auto;
  transition: max-width 360ms cubic-bezier(0.22, 0.61, 0.36, 1);
}
.mcp-stage-shell[data-breakpoint="mobile"] .mcp-stage {
  max-width: 390px;
  margin-inline: auto;
  transition: max-width 360ms cubic-bezier(0.22, 0.61, 0.36, 1);
}

/* ── PLAN-064 U6: the server-action echo toast. ───────────────────────────
   A small pill, bottom-center, that appears when a server action returns
   (`.is-shown` ← $mcpLastMessage). It sits ABOVE the stage but below drawers, and
   is pointer-transparent so it never blocks a click. The `is-shown` class fades
   it in; it stays subtle (the agent-comms feed remains the durable log). */
@keyframes mcp-toast-flash {
  0%   { opacity: 0; transform: translateX(-50%) translateY(6px); }
  12%  { opacity: 1; transform: translateX(-50%) translateY(0); }
  75%  { opacity: 1; transform: translateX(-50%) translateY(0); }
  100% { opacity: 0; transform: translateX(-50%) translateY(-4px); }
}
.mcp-action-toast {
  position: absolute;
  left: 50%;
  bottom: 74px;
  transform: translateX(-50%) translateY(6px);
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 14px;
  border-radius: 999px;
  background: rgba(31, 26, 22, 0.92);
  border: 1px solid var(--mcp-panel-strong);
  box-shadow: 0 10px 30px -12px rgba(0, 0, 0, 0.6);
  color: var(--mcp-taupe);
  font-family: var(--mcp-mono);
  font-size: 12px;
  opacity: 0;
  pointer-events: none;
  z-index: 7;
}
/* PLAN-064 U6 (review fix): `$mcpLastMessage` never resets to null, so a
   `.is-shown` that only sets opacity:1 would leave the toast STUCK visible. Play
   a one-shot fade-in-then-OUT keyframe instead (fill-mode forwards ends at
   opacity 0), so the toast is genuinely TRANSIENT — it flashes the action then
   settles hidden, and re-plays whenever `.is-shown` re-applies. */
.mcp-action-toast.is-shown {
  animation: mcp-toast-flash 2.6s cubic-bezier(0.22, 0.61, 0.36, 1) forwards;
}
/* PLAN-065 W_feedback (BUG-153): a FAILED action gets a red pill + a longer hold
   (4.6s vs 2.6s) so the user can actually read the server's error message before
   it fades. `.is-error` is driven by `$mcpLastMessage?.action?.ok === false`
   (env.st). The host bridge resets $mcpLastMessage ~2.8s after the action, but the
   keyframe (fill-mode forwards) owns the visual dwell, so the longer error
   animation is independent of that reset. */
.mcp-action-toast.is-error {
  background: rgba(58, 22, 22, 0.94);
  border-color: var(--mcp-ember);
  color: #f3c9c0;
}
.mcp-action-toast.is-error.is-shown {
  animation: mcp-toast-flash 4.6s cubic-bezier(0.22, 0.61, 0.36, 1) forwards;
}
.mcp-action-toast-dot {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--mcp-ember); flex: none;
}
.mcp-action-toast.is-error .mcp-action-toast-dot { background: #f3776a; }
/* An empty action name hides the toast entirely (no bare pill). */
.mcp-action-toast:not(:has(.mcp-action-toast-text:not(:empty))) { opacity: 0; }

/* ── PLAN-064 U7: hover-peek labels for the edge summon glyphs. ──────────────
   Four unlabeled warm glyphs were undiscoverable at rest. On hover/focus each
   reveals its `aria-label` as a small pill beside it (CSS `attr()` content), so
   a first-timer learns nav/structure/inspector/origin/comms without guessing.
   Focus-visible (keyboard) reveals it too. */
.mcp-summon { position: relative; }
.mcp-summon::after {
  content: attr(aria-label);
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  white-space: nowrap;
  padding: 5px 10px;
  border-radius: 8px;
  background: rgba(31, 26, 22, 0.95);
  border: 1px solid var(--mcp-panel-strong);
  color: var(--mcp-taupe);
  font-family: var(--mcp-mono);
  font-size: 11px;
  opacity: 0;
  pointer-events: none;
  transition: opacity 160ms ease;
  z-index: 8;
}
/* Left rail: label to the RIGHT; right rail: to the LEFT (styles.st sets sides). */
.mcp-rail-left  .mcp-summon::after { left: calc(100% + 10px); }
.mcp-rail-right .mcp-summon::after { right: calc(100% + 10px); }
.mcp-rail-bottom .mcp-summon::after { left: calc(100% + 10px); }
.mcp-summon:hover::after,
.mcp-summon:focus-visible::after { opacity: 1; }

/* ── PLAN-064 B3: canvas selection ring + hover outline. ───────────────────
   A stamped guest element lights an ember ring when SELECTED (its reactive
   `.is-selected-node` class, driven by $selectedNode in env.st) — the Webflow/
   Figma selection affordance. A hover outline previews selectability. Both use
   `outline` (not border) so they never shift the guest's layout, and
   `outline-offset` insets the ring so it reads on a full-bleed element. High
   specificity + the tooling `--mcp-ember` keep the ring visible over guest CSS.
   `:where(.mcp-stage …)` scopes it to the stage so chrome nodes are unaffected. */
.mcp-stage[data-st-region="stage"] [data-st-node] {
  cursor: pointer;
}
.mcp-stage[data-st-region="stage"] [data-st-node]:hover {
  outline: 1.5px dashed rgba(224, 145, 90, 0.5);
  outline-offset: -1px;
}
.mcp-stage[data-st-region="stage"] [data-st-node].is-selected-node {
  outline: 2px solid var(--mcp-ember);
  outline-offset: -1px;
}

/* ── PLAN-064 B5: the ARTBOARD. ─────────────────────────────────────────────
   A dark guest (e.g. a hero with a near-black background) was INVISIBLE on the
   warm-dark stage — guest bg (26,20,16) vs stage (36,30,25) measured 1.1:1
   contrast, so the design vanished with no visible bounds. Every real design
   tool sits the artwork on a NEUTRAL artboard (border + elevation) so ANY design
   — dark, light, transparent — reads clearly and has visible edges.

   The stage IS the region-mount boundary, so we can't wrap it without touching
   the mount. Instead we dress the GUEST ROOT (the direct non-empty child) as the
   artboard: a neutral light surface, a crisp border, and a drop shadow that
   lifts it off the stage. Applied ONLY when a guest is composed
   (`:has(> :not(.mcp-stage-empty))`), never to the empty-state. `:where()` keeps
   ZERO specificity so a guest's own background rule still wins INSIDE its root —
   the artboard is the backing the guest paints ONTO, not an override of guest
   styles (a guest with its own full-bleed bg covers the artboard; a guest with a
   transparent/dark bg reads against it). */
.mcp-stage[data-st-region="stage"]:has(> :not(.mcp-stage-empty)) {
  --mcp-artboard: #f4f1ea;        /* neutral warm-paper artboard */
  --mcp-artboard-edge: rgba(0, 0, 0, 0.14);
  background:
    radial-gradient(120% 90% at 50% 38%, var(--mcp-stage-glow), transparent 70%),
    var(--mcp-stage);
}
:where(.mcp-stage[data-st-region="stage"]:has(> :not(.mcp-stage-empty)) > :not(.mcp-stage-empty)) {
  background-color: var(--mcp-artboard);
  border: 1px solid var(--mcp-artboard-edge);
  border-radius: 14px;
  box-shadow:
    0 1px 0 rgba(255, 255, 255, 0.5) inset,
    0 30px 60px -24px rgba(0, 0, 0, 0.55);
  /* NB: deliberately NO `overflow: hidden` (reviewer B5 P2) — clipping the guest
     root would truncate a guest's own dropdowns/tooltips/popovers that paint
     outside the border box. The border-radius rounds the artboard's own surface;
     a guest with a sharp full-bleed background may poke a hair past the rounded
     corner (rare, cosmetic) but interactive overlays are never clipped. */
  transition: background-color 200ms ease, box-shadow 200ms ease;
}

/* M3' selected-frame inspector section: show the params + name once a frame is
   selected, the empty hint otherwise. Driven by `.has-selection` (← $selected). */
.mcp-selected-frame .mcp-selected-name,
.mcp-selected-frame .mcp-selected-params { display: none; }
.mcp-selected-frame.has-selection .mcp-selected-name,
.mcp-selected-frame.has-selection .mcp-selected-params { display: block; }
.mcp-selected-frame.has-selection .mcp-selected-empty { display: none; }
.mcp-selected-name { font-family: var(--mcp-serif); font-size: 20px; color: var(--mcp-cream); margin: 4px 0 12px; }
.mcp-selected-empty { color: var(--mcp-umber); font-family: var(--mcp-mono); font-size: 12px; }
.mcp-selected-frame { margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--mcp-panel-strong); }
/* M4 editable param rows in the selected-frame inspector. */
.mcp-param-edit { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
.mcp-param-edit-head { display: flex; align-items: center; gap: 8px; }
.mcp-param-input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 10px;
  border: 1px solid var(--mcp-panel-strong);
  background: rgba(0,0,0,0.22);
  color: var(--mcp-cream);
  font-family: var(--mcp-mono);
  font-size: 13px;
}
.mcp-param-input:focus { outline: none; border-color: var(--mcp-ember); }
/* PLAN-051 B4: editable per-instance node-param rows (mirror .mcp-param-edit). */
.mcp-node-params { display: flex; flex-direction: column; gap: 10px; }
/* PLAN-064 B4a: per-element binding rows reuse the node-param row styling. */
.mcp-node-bindings { display: flex; flex-direction: column; gap: 10px; }
/* M6 inline source editor in the selected-frame inspector. */
.mcp-source-editor { margin-top: 14px; }
.mcp-source-editor summary {
  cursor: pointer; color: var(--mcp-taupe); font-family: var(--mcp-mono);
  font-size: 12px; user-select: none; -webkit-user-select: none;
}
.mcp-source-editor summary:hover { color: var(--mcp-ember); }
.mcp-source-input {
  width: 100%; min-height: 160px; margin-top: 10px; resize: vertical;
  padding: 12px; border-radius: 12px;
  border: 1px solid var(--mcp-panel-strong); background: rgba(0,0,0,0.3);
  color: var(--mcp-cream); font-family: var(--mcp-mono); font-size: 12px; line-height: 1.5;
}
.mcp-source-input:focus { outline: none; border-color: var(--mcp-ember); }
.mcp-source-hint { color: var(--mcp-umber); font-family: var(--mcp-mono); font-size: 11px; margin: 8px 0 0; }
/* The editor lives inside .mcp-selected-frame, so it's hidden until selection
   like the rest of that section. Also hide when nothing selected explicitly. */
.mcp-selected-frame .mcp-source-editor { display: none; }
.mcp-selected-frame.has-selection .mcp-source-editor { display: block; }

/* PLAN-065 W_dim: the stage does NOT recede when a side drawer opens. A drawer
   (inspector/structure/nav/origin/comms) is a WORK SURFACE used WHILE looking at
   the canvas — dimming + shrinking the stage the moment you open the inspector to
   edit it is exactly backwards (you can no longer see what you edit). The side
   panel overlays the edge; the stage stays full-bright and geometrically stable.
   (Recession is a MODAL-dialog affordance; a side panel is not a modal. If a true
   blocking dialog is ever added, dim only for that flow, never for these drawers.) */

/* ── PLAN-051 B2: the structure NAVIGATOR tree. ─────────────────────────
   One indented row per node of the composed guest's template graph. The row's
   `--mcp-depth` (set inline from the projection) drives the indent; clicking a
   row selects that node (B3). Empty hint hidden once any node exists (:has). */
.mcp-nav-tree { display: flex; flex-direction: column; gap: 2px; }
.mcp-nav-node {
  display: flex; align-items: center; gap: 8px;
  width: 100%;
  padding: 7px 10px;
  padding-left: calc(10px + var(--mcp-depth, 0) * 16px);
  border: none; border-radius: 8px;
  background: transparent; color: var(--mcp-taupe);
  font-family: var(--mcp-mono); font-size: 13px; text-align: left;
  cursor: pointer;
  transition: background 160ms ease, color 160ms ease;
}
.mcp-nav-node:hover { background: rgba(224, 145, 90, 0.08); color: var(--mcp-cream); }
.mcp-nav-node-glyph {
  width: 6px; height: 6px; border-radius: 2px;
  background: var(--mcp-umber); flex: none;
}
.mcp-nav-node[data-kind="extern"] .mcp-nav-node-glyph {
  background: transparent; border: 1px solid var(--mcp-umber); border-radius: 50%;
}
.mcp-nav-node-label { color: var(--mcp-cream); font-weight: 500; }
.mcp-nav-node-tpl { margin-left: auto; color: var(--mcp-umber); font-size: 11px; }
.mcp-nav-empty { color: var(--mcp-umber); font-family: var(--mcp-mono); font-size: 12px; }
.mcp-drawer-structure:has(.mcp-nav-node) .mcp-nav-empty { display: none; }
/* Selected nav row (B3 selection spine): the row whose data-active is true lights
   with an ember wash + a left ember bar (Figma "this is selected"). ENABLED by
   FUP-094: the reactive attr `data-active="`$node.id == $selectedNode`"` on the
   node template resolves the OUTER $selectedNode signal lexically (ST.resolve +
   watchScoped), so the per-row comparison reacts to selection.
   BUG-136: a reactive BOOLEAN attr renders true as data-active="" (EMPTY string,
   not "true") and removes the attr when false. So match on attribute PRESENCE
   ([data-active]); `[data-active="true"]` never matched and the highlight never
   lit. */
.mcp-nav-node[data-active] {
  background: rgba(224, 145, 90, 0.14);
  box-shadow: inset 2px 0 0 0 var(--mcp-ember);
}
.mcp-nav-node[data-active] .mcp-nav-node-label { color: var(--mcp-ember); }

/* ── PLAN-051 B3: the selected-NODE detail in the inspector. ──────────────
   The filtered @each renders a .mcp-node-card only when a node is selected;
   :has toggles the empty hint. The card mirrors the selected-frame styling. */
.mcp-selected-node { margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--mcp-panel-strong); }
.mcp-node-empty { color: var(--mcp-umber); font-family: var(--mcp-mono); font-size: 12px; }
.mcp-selected-node:has(.mcp-node-card) .mcp-node-empty { display: none; }
.mcp-node-name { font-family: var(--mcp-serif); font-size: 19px; color: var(--mcp-cream); margin: 4px 0 10px; }
.mcp-node-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
.mcp-node-tpl-chip {
  font-family: var(--mcp-mono); font-size: 12px; color: var(--mcp-taupe);
  background: rgba(58, 48, 42, 0.5); padding: 3px 9px; border-radius: 999px;
}
.mcp-node-kind {
  font-family: var(--mcp-mono); font-size: 11px; color: var(--mcp-umber);
  text-transform: uppercase; letter-spacing: 0.04em;
}
.mcp-node-arity { color: var(--mcp-taupe); font-family: var(--mcp-mono); font-size: 12px; margin: 0; }
.mcp-node-bound {
  display: block; font-family: var(--mcp-mono); font-size: 12px;
  color: var(--mcp-cream); background: rgba(58, 48, 42, 0.4);
  padding: 8px 10px; border-radius: 8px; line-height: 1.6;
  word-break: break-word;
}
.mcp-node-card:not(:has(.mcp-node-bound:not(:empty))) .mcp-node-bound { display: none; }

/* Empty state (m1): sprout + editorial "Begin here" + ghost affordances. Centers
   ITSELF (the stage root is layout-neutral for mounted content), so it overlays
   the stage only while no bundle is mounted. */
.mcp-stage-empty {
  position: absolute;
  inset: 0;
  display: grid;
  place-content: center;
  justify-items: center;
  text-align: center;
  padding: 24px;
}
.mcp-stage-empty > * { max-width: 440px; }
/* FEAT-126 / BUG-111: once a guest bundle mounts into the stage region, the
   `@view` appends the guest as a sibling of the empty-state. Hide the
   empty-state whenever such a sibling exists so the stage shows ONLY the
   composed organism. Pure CSS — no page JS, no signal plumbing. */
.mcp-stage[data-st-region="stage"]:has(> :not(.mcp-stage-empty)) .mcp-stage-empty {
  display: none;
}

/* PLAN-049 W1.S2: the clear-stage control. Hidden by default; revealed only when
   the stage holds a composed guest (the same `:has(> :not(.mcp-stage-empty))`
   signal that hides the empty-state). Floats at the stage's top-right, receding
   chrome that brightens on hover — a one-gesture "empty the stage". No page JS. */
.mcp-stage-clear {
  display: none;
  position: absolute;
  top: 4.2rem;
  right: 1.4rem;
  z-index: 6;
  align-items: center;
  gap: 0.4rem;
  padding: 0.32rem 0.66rem;
  font: inherit;
  font-size: 0.74rem;
  letter-spacing: 0.02em;
  color: var(--mcp-guest-ink-soft);
  background: rgba(36, 30, 25, 0.72);
  border: 1px solid rgba(224, 145, 90, 0.22);
  border-radius: 999px;
  cursor: pointer;
  backdrop-filter: blur(6px);
  transition: color 0.16s ease, border-color 0.16s ease, background 0.16s ease;
}
.mcp-stage-shell:has(.mcp-stage[data-st-region="stage"] > :not(.mcp-stage-empty)) .mcp-stage-clear {
  display: inline-flex;
}
/* PLAN-064 U1: clear-stage sits top-RIGHT (z6) and overlapped an open RIGHT
   drawer (inspector/origin) header. Recede it while a right-side drawer is open
   so the drawer content is never occluded. Driven by `data-drawer` on the shell
   (the same single signal the drawers use). */
.mcp-stage-shell[data-drawer="inspector"] .mcp-stage-clear,
.mcp-stage-shell[data-drawer="origin"] .mcp-stage-clear {
  opacity: 0;
  pointer-events: none;
}
.mcp-stage-clear:hover {
  color: var(--mcp-ember);
  border-color: rgba(224, 145, 90, 0.5);
  background: rgba(46, 36, 28, 0.9);
}
.mcp-stage-clear svg { display: block; }

/* PLAN-049 W4.S8: the frame strip. A sibling of the stage, floated at its
   bottom-left; revealed only when a frame is composed (same `:has` signal as the
   clear-stage control). Each chip names a composed region + its origin accent,
   with a × to unmount that frame. Generalizes toward multi-frame. No page JS. */
.mcp-frame-strip {
  display: none;
  position: absolute;
  bottom: 1.4rem;
  left: 6.2rem;
  z-index: 6;
  gap: 6px;
  flex-wrap: wrap;
  max-width: 60vw;
}
.mcp-stage-shell:has(.mcp-stage[data-st-region="stage"] > :not(.mcp-stage-empty)) .mcp-frame-strip {
  display: inline-flex;
}
.mcp-frame-chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 4px 6px 4px 11px;
  border-radius: 999px;
  font-size: 11px;
  color: var(--mcp-cream);
  background: rgba(36, 30, 25, 0.82);
  border: 1px solid var(--mcp-panel-strong);
  backdrop-filter: blur(6px);
}
.mcp-frame-chip[data-origin="agent"] { border-color: rgba(212, 150, 90, 0.4); }
.mcp-frame-chip[data-origin="env"]   { border-color: rgba(138, 168, 106, 0.4); }
.mcp-frame-chip-name { font-family: var(--mcp-mono); }
.mcp-frame-chip-region {
  font-size: 9px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--mcp-umber);
}
.mcp-frame-unmount {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 999px;
  border: none;
  background: rgba(58, 48, 42, 0.6);
  color: var(--mcp-guest-ink-soft);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
  transition: color 0.16s ease, background 0.16s ease;
}
.mcp-frame-unmount:hover { color: #f0b48a; background: rgba(58, 36, 28, 0.9); }
.mcp-sprout {
  color: var(--mcp-ember);
  filter: drop-shadow(0 0 18px rgba(224, 145, 90, 0.5));
  animation: mcp-pulse 3.6s ease-in-out infinite;
}
.mcp-begin {
  font-family: var(--mcp-serif);
  font-size: 46px;
  font-weight: 600;
  margin: 14px 0 12px;
  color: var(--mcp-cream);
}
.mcp-begin-note { color: var(--mcp-taupe); margin: 0 0 26px; }

/* ── M2 living gallery (PLAN-046) ─────────────────────────────────────────
   A responsive grid of clickable function tiles in the empty-state. Clicking a
   tile composes its function onto the stage. The gallery and its empty-hint are
   siblings; when the gallery has cards, JS-free CSS hides the hint via `:has`. */
.mcp-gallery {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 16px;
  width: min(720px, 72vw);
  max-height: 52vh;
  overflow-y: auto;
  padding: 4px;
}
.mcp-gallery-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  padding: 18px;
  border-radius: 18px;
  border: 1px solid var(--mcp-panel-strong);
  background: var(--mcp-panel);
  color: var(--mcp-cream);
  cursor: pointer;
  text-align: left;
  font: inherit;
  transition: transform 180ms cubic-bezier(0.22,0.61,0.36,1), border-color 180ms ease, background 180ms ease;
}
.mcp-gallery-card:hover {
  transform: translateY(-3px);
  border-color: var(--mcp-ember);
  background: var(--mcp-panel-strong);
}
.mcp-gallery-spark {
  width: 26px; height: 26px; border-radius: 9px;
  background: radial-gradient(circle at 35% 30%, var(--mcp-ember-hot), var(--mcp-ember) 60%, transparent 75%);
  box-shadow: 0 0 16px rgba(224,145,90,0.45);
}
.mcp-gallery-name {
  font-family: var(--mcp-serif);
  font-size: 18px;
  font-weight: 600;
  color: var(--mcp-cream);
}
.mcp-gallery-meta { font-family: var(--mcp-mono); font-size: 11px; color: var(--mcp-umber); }
.mcp-gallery-empty-hint {
  color: var(--mcp-umber);
  font-family: var(--mcp-mono);
  font-size: 13px;
  margin: 18px 0 0;
}
.mcp-gallery-empty-hint code { color: var(--mcp-taupe); }
/* Hide the hint when the gallery actually has cards (pure CSS, no signal). */
.mcp-stage-empty:has(.mcp-gallery-card) .mcp-gallery-empty-hint { display: none; }
/* When the gallery is non-empty, relax the centering so the grid breathes. */
.mcp-stage-empty:has(.mcp-gallery-card) { align-content: start; padding-top: 9vh; }

/* ── BUG-122: frame-first active-environment view ─────────────────────────
   When an environment is open, its cards + tabs show ON THE STAGE (the dominant
   surface), so `st_env_open` is VISIBLE where the user is looking instead of
   only as a buried nav-drawer card. Pure JS-free `:has` reveal, mirroring the
   gallery-hint idiom above: the section is hidden until `.mcp-active-env-cards`
   actually holds an open-env card, then it appears and the "Begin here" prose +
   gallery recede. The guest-compose `$stageTpl` path is untouched (a composed
   guest still overlays via its own sibling mount). */
.mcp-active-env { display: none; width: min(720px, 72vw); text-align: left; }
.mcp-stage-empty:has(.mcp-active-env-cards .mcp-open-env-card) .mcp-active-env {
  display: block;
}
/* With an env open, lead with it: hide the zero-state prose + the gallery, and
   top-align so the env detail reads as the primary content. */
.mcp-stage-empty:has(.mcp-active-env-cards .mcp-open-env-card) .mcp-begin,
.mcp-stage-empty:has(.mcp-active-env-cards .mcp-open-env-card) .mcp-begin-note,
.mcp-stage-empty:has(.mcp-active-env-cards .mcp-open-env-card) .mcp-gallery,
.mcp-stage-empty:has(.mcp-active-env-cards .mcp-open-env-card) .mcp-gallery-empty-hint {
  display: none;
}
.mcp-stage-empty:has(.mcp-active-env-cards .mcp-open-env-card) {
  align-content: start;
  justify-items: stretch;
  padding-top: 7vh;
}
.mcp-active-env-cards { display: grid; gap: 12px; margin-bottom: 22px; }
.mcp-active-env-tabs { display: grid; gap: 10px; }
/* Hide the tabs label + empty hint until they are relevant. */
.mcp-active-env-tabs-label { margin-top: 18px; }
.mcp-active-env:has(.mcp-tab-card) .mcp-active-env-empty-tabs { display: none; }
.mcp-active-env-empty-tabs {
  color: var(--mcp-umber);
  font-family: var(--mcp-mono);
  font-size: 13px;
  margin: 8px 0 0;
}
.mcp-active-env-empty-tabs code { color: var(--mcp-taupe); }

/* ── Edge rails: columns of summon glyphs (the button-drawer chips). ─────── */
/* z-index 6 keeps the glyphs ABOVE an open drawer (z-index 5) + scrim (4), so a
   same-edge pair (inspector + origin share the right rail) can switch directly:
   clicking the other glyph just sets `$drawer` to its target. Without this the
   open drawer buries its sibling glyph and you must close before switching. */
.mcp-rail { position: absolute; z-index: 6; display: flex; gap: 14px; }
.mcp-rail-left   { left: 18px;  top: 50%; transform: translateY(-50%); flex-direction: column; }
.mcp-rail-right  { right: 18px; top: 50%; transform: translateY(-50%); flex-direction: column; }
.mcp-rail-bottom { bottom: 18px; left: 50%; transform: translateX(-50%); }
.mcp-summon {
  width: 50px; height: 50px;
  border: 0;
  border-radius: 16px;
  background: var(--mcp-panel-strong);
  color: var(--mcp-taupe);
  font-size: 19px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 220ms ease, color 220ms ease, transform 220ms ease;
}
.mcp-summon:hover {
  background: rgba(224, 145, 90, 0.16);
  color: var(--mcp-ember);
  transform: translateY(-1px);
}
/* Active-drawer glyph glows ember: the open pane is legible at a glance, and
   the same-edge pair reads as a two-state toggle. Driven by the `data-drawer`
   signal on the shell (one open at a time) — no JS, no extra state. */
.mcp-stage-shell[data-drawer="nav"]       .mcp-summon[data-summon="nav"],
.mcp-stage-shell[data-drawer="inspector"] .mcp-summon[data-summon="inspector"],
.mcp-stage-shell[data-drawer="origin"]    .mcp-summon[data-summon="origin"],
.mcp-stage-shell[data-drawer="comms"]     .mcp-summon[data-summon="comms"] {
  background: rgba(224, 145, 90, 0.2);
  color: var(--mcp-ember);
}

/* ── Scrim: dims the canvas behind an open drawer; click to dismiss. ─────── */
/* PLAN-065 W_dim: the side drawers are WORK surfaces, so the scrim neither dims
   the canvas nor steals canvas clicks. It stays fully transparent and
   click-through ALWAYS (no dark backdrop, no pointer capture): a user editing in
   the inspector must still see + click the stage. Drawer dismissal is the close
   glyph + Escape (env.st) — not a modal click-outside that would swallow a canvas
   node-selection click. The element is retained (harmless) but visually inert. */
.mcp-scrim {
  position: absolute;
  inset: 0;
  z-index: 4;
  background: transparent;
  opacity: 0;
  pointer-events: none;
}
/* No `.is-active` state: the scrim is unconditionally inert now (W_dim). It is
   kept as a harmless element only to avoid churning the env.st markup; its old
   click-to-dismiss is superseded by the close glyph + Escape. */

/* ── Drawers: warm panels that slide in over the stage edge. ─────────────── */
.mcp-drawer {
  position: absolute;
  z-index: 5;
  background: var(--mcp-drawer-bg);
  box-shadow: 0 30px 80px -24px var(--mcp-shadow);
  display: flex;
  flex-direction: column;
  opacity: 0;
  pointer-events: none;
  transition: transform 360ms cubic-bezier(0.22, 0.61, 0.36, 1), opacity 320ms ease;
}
.mcp-drawer.is-open { opacity: 1; pointer-events: auto; transform: none; }

.mcp-drawer-left {
  top: 16px; bottom: 16px; left: 16px;
  width: min(360px, 80vw);
  border-radius: 24px;
  transform: translateX(-24px);
}
.mcp-drawer-right {
  top: 16px; bottom: 16px; right: 16px;
  width: min(380px, 84vw);
  border-radius: 24px;
  transform: translateX(24px);
}
.mcp-drawer-bottom {
  left: 16px; right: 16px; bottom: 16px;
  max-height: 46vh;
  border-radius: 24px;
  transform: translateY(24px);
}

.mcp-drawer-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 22px 24px 14px;
}
.mcp-drawer-head h2 {
  margin: 0;
  font-family: var(--mcp-serif);
  font-size: 24px;
  font-weight: 600;
  color: var(--mcp-cream);
}
.mcp-drawer-close {
  width: 34px; height: 34px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--mcp-taupe);
  font-size: 20px;
  cursor: pointer;
  transition: background 200ms ease, color 200ms ease;
}
.mcp-drawer-close:hover { background: rgba(224, 145, 90, 0.14); color: var(--mcp-ember); }
.mcp-drawer-body { padding: 4px 18px 22px; overflow-y: auto; display: grid; gap: 12px; }
.mcp-drawer-label {
  margin: 10px 0 2px;
  /* PLAN-064 U5: was --mcp-umber (3.3:1, below WCAG AA). --mcp-umber-aa is 6.0:1
     on the drawer bg — these small-caps labels now pass AA for small text. */
  color: var(--mcp-umber-aa);
  text-transform: uppercase;
  letter-spacing: 0.12em;
  font-size: 11px;
  font-weight: 700;
}

/* ── Cards: warm, soft, flat. ────────────────────────────────────────────── */
.mcp-env-grid,
.mcp-open-envs,
.mcp-tab-list,
.mcp-function-list,
.mcp-instance-list,
.mcp-event-list { display: grid; gap: 10px; }
/* M5 comms feed: agent messages glow ember (left rail), server events stay
   muted. The accent reads at a glance which side spoke. */
.mcp-event-card { border-left: 3px solid var(--mcp-umber); }
.mcp-event-card[data-audience="agent"] { border-left-color: var(--mcp-ember); }
.mcp-event-value:empty { display: none; }
.mcp-comms-empty { color: var(--mcp-umber); font-family: var(--mcp-mono); font-size: 12px; margin: 8px 0 0; }
/* Hide the empty hint once any event card exists (pure CSS). */
.mcp-drawer-comms .mcp-drawer-body:has(.mcp-event-card) .mcp-comms-empty { display: none; }
.mcp-card {
  display: grid;
  gap: 7px;
  align-content: start;
  padding: 14px 16px;
  border-radius: 16px;
  background: var(--mcp-panel);
}
.mcp-card-label {
  margin: 0;
  color: var(--mcp-umber);
  text-transform: uppercase;
  letter-spacing: 0.12em;
  font-size: 10px;
  font-weight: 700;
}
.mcp-card h3 { margin: 0; font-size: 15px; color: var(--mcp-cream); }
.mcp-card code,
.mcp-card small { color: var(--mcp-taupe); overflow-wrap: anywhere; }
.mcp-card code { font-family: var(--mcp-mono); font-size: 12px; }
.mcp-card a { color: var(--mcp-ember); text-decoration: none; font-weight: 600; font-size: 13px; }
.mcp-card a:hover { color: var(--mcp-ember-hot); }

/* ── Provenance chip (FEAT-128), warm-harmonized: soft dot + label. ──────── */
.mcp-origin-chip {
  justify-self: start;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 3px 11px 3px 9px;
  border-radius: 999px;
  font-family: var(--mcp-mono);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.mcp-origin-chip::before { content: ""; width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
.mcp-origin-chip[data-origin="stdlib"] { background: rgba(107, 154, 146, 0.14); color: var(--mcp-origin-stdlib); }
.mcp-origin-chip[data-origin="env"]    { background: rgba(138, 168, 106, 0.14); color: var(--mcp-origin-env); }
.mcp-origin-chip[data-origin="agent"]  { background: rgba(212, 150, 90, 0.14);  color: var(--mcp-origin-agent); }
.mcp-origin-chip[data-origin="kit"]    { background: rgba(184, 122, 130, 0.14); color: var(--mcp-origin-kit); }

/* ── Inspector contract param rows (FEAT-132). ───────────────────────────── */
.mcp-param-rows { display: grid; gap: 6px; margin: 2px 0; }
.mcp-param-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: 10px;
  background: rgba(58, 48, 42, 0.28);
}
.mcp-param-name { color: var(--mcp-cream); font-family: var(--mcp-mono); font-size: 12px; }
.mcp-param-kind {
  padding: 1px 8px;
  border-radius: 999px;
  font-family: var(--mcp-mono);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.mcp-param-kind[data-kind="binding"] { background: rgba(138, 168, 106, 0.16); color: var(--mcp-origin-env); }
.mcp-param-kind[data-kind="element"] { background: rgba(212, 150, 90, 0.16);  color: var(--mcp-origin-agent); }
.mcp-param-type { margin-left: auto; color: var(--mcp-umber); font-family: var(--mcp-mono); font-size: 11px; }

/* ── PLAN-049 lifecycle + COLLAPSE affordances ──────────────────────────────
   Card action rows (instance / tab cards) + the lifecycle buttons (unmount,
   close, delete) + the env entry-point picker. Receding chrome that brightens
   on hover; destructive actions (delete/close) lean ember-warning on hover. */
.mcp-card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
  align-items: center;
}
.mcp-unmount, .mcp-env-close, .mcp-fn-delete, .mcp-tab-stage, .mcp-tab-close, .mcp-instance-focus {
  font-family: var(--mcp-sans, inherit);
  font-size: 11px;
  letter-spacing: 0.02em;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid var(--mcp-panel-strong);
  background: rgba(58, 48, 42, 0.3);
  color: var(--mcp-cream);
  cursor: pointer;
  transition: color 0.16s ease, border-color 0.16s ease, background 0.16s ease;
}
.mcp-tab-stage:hover, .mcp-instance-focus:hover {
  color: var(--mcp-ember);
  border-color: rgba(224, 145, 90, 0.5);
  background: rgba(46, 36, 28, 0.85);
}
/* Destructive verbs: a warmer warning accent on hover. */
.mcp-unmount:hover, .mcp-env-close:hover, .mcp-fn-delete:hover, .mcp-tab-close:hover {
  color: #f0b48a;
  border-color: rgba(224, 120, 90, 0.55);
  background: rgba(58, 36, 28, 0.85);
}
/* A compile-failed tab cannot be composed — hide its "Open on stage". */
.mcp-tab-card[data-compile-ok="false"] .mcp-tab-stage { display: none; }

/* The env entry-point picker: a column of clickable "+ tab" rows. */
.mcp-entry-list { display: flex; flex-direction: column; gap: 4px; }
.mcp-entry-card {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  text-align: left;
  padding: 7px 10px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: rgba(58, 48, 42, 0.22);
  color: var(--mcp-cream);
  font-family: var(--mcp-mono);
  font-size: 12px;
  cursor: pointer;
  transition: color 0.16s ease, border-color 0.16s ease, background 0.16s ease;
}
.mcp-entry-card:hover {
  color: var(--mcp-ember);
  border-color: rgba(224, 145, 90, 0.4);
  background: rgba(46, 36, 28, 0.7);
}
.mcp-entry-plus {
  font-weight: 700;
  color: var(--mcp-ember);
  font-size: 14px;
  line-height: 1;
}
.mcp-entry-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
/* Hide the entry-picker empty hint once any entry row exists (JS-free). */
.mcp-drawer-body:has(.mcp-entry-card) .mcp-entry-empty { display: none; }
.mcp-entry-empty { color: var(--mcp-umber); font-size: 12px; margin: 2px 0 8px; }

/* ── PLAN-049 W3: the "+" creation surface (scratch a new frame) ─────────── */
.mcp-create {
  margin-top: 18px;
  width: 100%;
  max-width: 440px;
  text-align: left;
}
.mcp-create > summary {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  cursor: pointer;
  list-style: none;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(224, 145, 90, 0.28);
  color: var(--mcp-cream);
  font-size: 13px;
  transition: color 0.16s ease, border-color 0.16s ease, background 0.16s ease;
}
.mcp-create > summary::-webkit-details-marker { display: none; }
.mcp-create > summary:hover {
  color: var(--mcp-ember);
  border-color: rgba(224, 145, 90, 0.55);
  background: rgba(46, 36, 28, 0.6);
}
.mcp-create-plus { color: var(--mcp-ember); font-weight: 700; font-size: 15px; line-height: 1; }
.mcp-create-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
  padding: 12px;
  border-radius: 12px;
  border: 1px solid var(--mcp-panel-strong);
  background: rgba(36, 30, 25, 0.6);
}
.mcp-create-name, .mcp-create-source {
  font-family: var(--mcp-mono);
  font-size: 12px;
  color: var(--mcp-cream);
  background: rgba(20, 16, 13, 0.6);
  border: 1px solid var(--mcp-panel-strong);
  border-radius: 8px;
  padding: 8px 10px;
}
.mcp-create-source { min-height: 120px; resize: vertical; line-height: 1.45; }
.mcp-create-name:focus, .mcp-create-source:focus {
  outline: none;
  border-color: rgba(224, 145, 90, 0.5);
}
.mcp-create-submit {
  align-self: flex-start;
  font-size: 12px;
  padding: 7px 14px;
  border-radius: 999px;
  border: 1px solid rgba(224, 145, 90, 0.4);
  background: rgba(224, 145, 90, 0.14);
  color: var(--mcp-ember-hot);
  cursor: pointer;
  transition: background 0.16s ease, border-color 0.16s ease;
}
.mcp-create-submit:hover {
  background: rgba(224, 145, 90, 0.26);
  border-color: rgba(224, 145, 90, 0.7);
}
.mcp-create-hint { color: var(--mcp-umber); font-size: 11px; margin: 0; }

@media (max-width: 720px) {
  .mcp-stage { margin: 8px 72px 72px; }
  .mcp-drawer-left, .mcp-drawer-right { width: min(320px, 88vw); }
}
