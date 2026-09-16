// =============================================================================
// Admin styles (FEAT-076) — the editorial identity, ported from the monolith's
// injectStyles() to the rebuild's class-based structure.
//
// White surfaces, near-black #0a0a0a text, a single scarlet #FF0020 accent,
// Georgia serif headings, a 264px rail. The drawer is the 3rd grid child but
// stays display:none until @portal relocates it to <body> (aria-modal="true"),
// where it becomes the fixed right editor panel.
// =============================================================================

* { box-sizing: border-box; }
body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", Inter, sans-serif; color: #0a0a0a; background: #fff; }

// ── Shell ────────────────────────────────────────────────────────────────────
// Three zones: rail | center | optional live-preview. The preview column
// collapses to 0 unless a token pane needs it (.ad-root--preview, keyed on
// $pane === "brand" in tools.st) — posts/authors editing keeps full width.
.ad-root { display: grid; grid-template-columns: 264px 1fr 0; min-height: 100vh; transition: grid-template-columns .2s ease; }
.ad-root--preview { grid-template-columns: 264px minmax(420px, 1fr) minmax(440px, 46%); }

// Live-preview pane (FEAT-105): the project page in an <iframe>, recolored in
// real-time by the token-preview postMessage bridge. Hidden (0-width column +
// display:none) until a token pane is open.
.ad-preview { display: none; flex-direction: column; border-left: 1px solid #E6E6E6; background: #fbfbfb; min-width: 0; }
.ad-root--preview .ad-preview { display: flex; }
.ad-preview-bar { display: flex; align-items: center; gap: 7px; padding: 10px 16px; font-size: 11px; letter-spacing: 0.04em; text-transform: uppercase; color: #9a9a9a; border-bottom: 1px solid #Eee; background: #fff; }
.ad-preview-dot { width: 7px; height: 7px; border-radius: 50%; background: #1fbf6b; box-shadow: 0 0 0 3px rgba(31,191,107,0.16); }
.ad-preview-frame { flex: 1; width: 100%; border: 0; background: #fff; }

// Drawer: hidden in-grid by default; a fixed right panel once @portal opens it.
.ad-drawer { display: none; }
.ad-drawer[aria-modal="true"] { display: flex; flex-direction: column; position: fixed; top: 0; right: 0; width: 440px; height: 100vh; background: #fff; box-shadow: -12px 0 40px rgba(0,0,0,0.10); border-left: 1px solid #E6E6E6; overflow-y: auto; z-index: 40; }

// ── Rail ─────────────────────────────────────────────────────────────────────
.ad-rail { border-right: 1px solid #E6E6E6; padding: 22px 14px; display: flex; flex-direction: column; gap: 4px; }
.ad-brand { font-weight: 600; font-size: 15px; letter-spacing: -0.01em; padding: 2px 10px 18px; }
.ad-rail-section { font-size: 10px; letter-spacing: 0.13em; text-transform: uppercase; color: #9a9a9a; padding: 10px 10px 6px; }
.ad-rail-divider { height: 1px; background: #E6E6E6; margin: 12px 6px; }
.ad-nav { display: flex; flex-direction: column; gap: 1px; }
.ad-nav-item { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; border: 0; background: none; font: inherit; color: #3a3a3a; padding: 7px 10px; border-radius: 7px; cursor: pointer; text-align: left; border-left: 2px solid transparent; }
.ad-nav-item:hover { background: #f6f6f6; }
.ad-nav-item--active { background: #f6f6f6; color: #0a0a0a; border-left: 2px solid #FF0020; font-weight: 550; }
// When a tool pane is open, the collection nav's runtime --active marker is
// stale (a tool is the live destination, not a collection). Neutralize its
// highlight so exactly ONE rail item ever reads as selected.
.ad-nav--tools-open .ad-nav-item--active { background: none; color: #3a3a3a; border-left-color: transparent; font-weight: 400; }
.ad-nav-count { font-size: 11px; color: #9a9a9a; font-variant-numeric: tabular-nums; }
.ad-rail-foot { margin-top: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
.ad-pill { font-size: 11px; color: #6a6a6a; background: #f2f2f2; border-radius: 20px; padding: 4px 10px; }

// ── Center ───────────────────────────────────────────────────────────────────
.ad-center { padding: 30px 40px; max-width: 1100px; }
.ad-head { display: flex; align-items: flex-end; justify-content: space-between; margin-bottom: 22px; }
.ad-head-titles { display: flex; align-items: baseline; gap: 12px; }
.ad-title { font-family: Georgia, "Times New Roman", serif; font-weight: 500; font-size: 30px; margin: 0; letter-spacing: -0.01em; }
.ad-head-count { font-size: 13px; color: #9a9a9a; }
.ad-head-actions { display: flex; align-items: center; gap: 12px; }

// ── Buttons ──────────────────────────────────────────────────────────────────
.ad-btn { font: inherit; font-size: 13px; border-radius: 8px; padding: 8px 14px; border: 1px solid transparent; cursor: pointer; }
.ad-btn--primary { background: #FF0020; color: #fff; font-weight: 550; }
.ad-btn--primary:hover { filter: brightness(0.92); }

// ── List + rows ──────────────────────────────────────────────────────────────
.ad-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
.ad-row { display: flex; align-items: center; gap: 10px; padding: 4px 10px; border-bottom: 1px solid #f0f0f0; border-radius: 8px; }
.ad-row:hover { background: #fafafa; }
.ad-row-reorder { display: flex; flex-direction: column; gap: 1px; flex: 0 0 auto; }
.ad-row-up, .ad-row-down { border: 0; background: none; color: #cfcfcf; font-size: 11px; line-height: 1; padding: 1px 3px; cursor: pointer; border-radius: 4px; }
.ad-row-up:hover, .ad-row-down:hover { color: #FF0020; background: #f6f6f6; }
.ad-row-open { display: flex; align-items: center; gap: 14px; flex: 1; min-width: 0; border: 0; background: none; font: inherit; color: inherit; text-align: left; padding: 8px 4px; cursor: pointer; }
.ad-row-thumb { width: 42px; height: 42px; border-radius: 50%; background: #eee; flex: 0 0 auto; }
.ad-row-label { font-weight: 550; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

// ── Empty state ──────────────────────────────────────────────────────────────
.ad-empty { text-align: center; padding: 70px 20px; color: #6a6a6a; }
.ad-empty h2 { font-family: Georgia, serif; font-weight: 500; font-size: 20px; color: #0a0a0a; margin: 0 0 6px; }
.ad-empty p { margin: 0 0 20px; font-size: 14px; }

// ── Drawer (editor) ──────────────────────────────────────────────────────────
.ad-drawer-head { display: flex; align-items: center; gap: 10px; padding: 22px 22px 14px; border-bottom: 1px solid #E6E6E6; flex-wrap: wrap; }
.ad-drawer-title { font-family: Georgia, serif; font-weight: 500; font-size: 21px; margin: 0; flex: 1; }
.ad-drawer-kind { font-size: 11px; color: #9a9a9a; }
.ad-drawer-x { border: 0; background: none; font-size: 16px; color: #9a9a9a; cursor: pointer; }
.ad-form { padding: 18px 22px; flex: 1; overflow-y: auto; }
.ad-field-slot { display: block; }
.ad-field { margin-bottom: 16px; }
.ad-flabel { display: block; font-size: 11px; letter-spacing: 0.03em; text-transform: uppercase; color: #8a8a8a; margin-bottom: 6px; }
.ad-input { width: 100%; border: 1px solid #E0E0E0; border-radius: 8px; padding: 9px 11px; font: inherit; font-size: 14px; color: #0a0a0a; outline: none; transition: border-color .15s, box-shadow .15s; }
.ad-input:focus { border-color: #FF0020; box-shadow: 0 0 0 3px rgba(255,0,32,0.12); }
.ad-input--readonly { background: #f7f7f7; color: #9a9a9a; }
.ad-textarea { min-height: 90px; resize: vertical; }

// Group headers (nested fieldset / list items), indented by depth.
.ad-group { margin: 14px 0 8px; }
.ad-group-label { font-size: 11px; letter-spacing: 0.04em; text-transform: uppercase; color: #6a6a6a; font-weight: 600; }
.ad-group[data-depth="1"] { padding-left: 12px; border-left: 2px solid #E6E6E6; }
.ad-group[data-depth="2"] { padding-left: 24px; border-left: 2px solid #E6E6E6; }
.ad-group[data-depth="3"] { padding-left: 36px; border-left: 2px solid #E6E6E6; }

// Media widget.
.ad-media { display: flex; gap: 10px; align-items: center; }
.ad-media-prev { width: 54px; height: 54px; border-radius: 8px; background: #eee; flex: 0 0 auto; }

// Chips + relations.
.ad-chips { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; border: 1px solid #E0E0E0; border-radius: 8px; padding: 7px; }
.ad-chip { font-size: 11px; background: #f1f1f1; color: #4a4a4a; border-radius: 20px; padding: 3px 9px; white-space: nowrap; }
.ad-chip-add { border: 0; outline: none; font: inherit; font-size: 13px; min-width: 70px; flex: 1; }
.ad-relation { cursor: pointer; }
.ad-rel-multi { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; border: 1px solid #E6E6E6; border-radius: 8px; padding: 7px; }
.ad-chip--rel { display: inline-flex; align-items: center; gap: 4px; background: rgba(255,0,32,0.08); color: #a01020; }
.ad-rel-add { border: 0; outline: none; font: inherit; font-size: 12.5px; color: #8a8a8a; background: none; min-width: 80px; flex: 1; cursor: pointer; }

// Drawer footer.
.ad-drawer-foot { display: flex; align-items: center; justify-content: space-between; padding: 14px 22px; border-top: 1px solid #E6E6E6; position: sticky; bottom: 0; background: #fff; }
.ad-link-danger { border: 0; background: none; color: #9a4a4a; font: inherit; font-size: 13px; cursor: pointer; }
.ad-del.armed { color: #fff; background: #FF0020; padding: 5px 11px; border-radius: 7px; font-weight: 550; }

// ── Tool panes (Theme / Components / Motion) ──────────────────────────────
.hidden { display: none; }
.ad-tool-pane { display: none; }
.ad-tool-pane.shown { display: block; }
// Tool pane header (NOT .ad-head — that's reactively hidden outside the list
// pane). Mirrors the list header's editorial scale so Brand/Components/Motion
// read as first-class destinations, not a bare field dump.
.ad-tool-head { padding: 30px 30px 0; display: flex; align-items: flex-end; }
.ad-tool-head .ad-head-titles { display: flex; align-items: baseline; gap: 12px; }
.ad-tool-head .ad-tool-title { font-family: Georgia, "Times New Roman", serif; font-weight: 500; font-size: 30px; margin: 0; letter-spacing: -0.01em; }
.ad-tool-head .ad-tool-sub { font-size: 13px; color: #9a9a9a; }
.ad-tool-body { padding: 22px 30px 26px; }
.ad-rel-hint { display: block; margin-top: 5px; font-size: 11.5px; color: #b88; }
.ad-theme-group { margin-bottom: 30px; }
// Brand pane: a vertical stack of form fields (shared .ad-field from the drawer),
// in a contained card so the design-token set reads as one cohesive identity
// rather than free-floating inputs. Constrained width keeps line length tight.
.ad-brand-grid { display: none; flex-direction: column; gap: 18px; max-width: 460px; padding: 22px 22px 8px; border: 1px solid #ececec; border-radius: 14px; box-shadow: 0 1px 2px rgba(0,0,0,0.03); }
.ad-brand-grid.shown { display: flex; }
.ad-brand-grid .st-fld-field { margin-bottom: 0; }
// Derived token row (FEAT-108): read-only resolved swatch + a muted formula
// chip, visually distinct from the editable pills (no border/focus affordance,
// a chain-implied formula). The swatch shows the COMPUTED value live.
.ad-field--derived .ad-derived { display: flex; align-items: center; gap: 9px; padding: 4px 0; }
.ad-derived-swatch { width: 22px; height: 22px; border-radius: 6px; border: 1px solid rgba(0,0,0,0.10); box-shadow: inset 0 0 0 1px rgba(255,255,255,0.5); flex: 0 0 auto; }
.ad-derived-formula { font-family: ui-monospace, SFMono-Regular, monospace; font-size: 11.5px; color: #8a8a8a; background: #f4f4f4; border-radius: 6px; padding: 4px 8px; flex: 1; }
.ad-derived-formula::before { content: "\2190  "; color: #b8b8b8; }
.ad-derived-val { font-family: ui-monospace, SFMono-Regular, monospace; font-size: 11px; color: #b0b0b0; text-transform: uppercase; flex: 0 0 auto; }

.ad-palette-grid { display: none; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 14px; }
.ad-palette-grid.shown { display: grid; }
// Motion pane: a vertical field stack (shared .ad-field), same contained-card
// identity as the Brand pane — one form-renderer, one look. Group headers
// (per motion directive) separate each motion's params.
.ad-motion-grid { display: none; flex-direction: column; gap: 14px; max-width: 460px; padding: 22px 22px 8px; border: 1px solid #ececec; border-radius: 14px; box-shadow: 0 1px 2px rgba(0,0,0,0.03); }
.ad-motion-grid.shown { display: flex; }
.ad-motion-grid .st-fld-field { margin-bottom: 0; }
.ad-motion-grid .ad-group:first-child { margin-top: 0; }
.ad-tool-empty { display: none; }
.ad-tool-empty.shown { display: block; }
.ad-palette-card { border: 1px solid #e8e8e8; border-radius: 10px; padding: 14px; }
.ad-palette-card:hover { border-color: #FF0020; box-shadow: 0 4px 14px rgba(0,0,0,0.06); }
.ad-palette-name { font-size: 14px; color: #1a1a1a; text-transform: capitalize; margin-bottom: 8px; }
.ad-palette-sig { display: block; font-family: ui-monospace, monospace; font-size: 11.5px; color: #6a6a6a; word-break: break-word; }
.ad-sig { font-family: ui-monospace, monospace; font-size: 10.5px; padding: 2px 7px; border-radius: 5px; }
.ad-sig-binding { background: rgba(255,0,32,0.08); color: #a01020; }
.ad-sig-slot { background: #eef1ff; color: #3a3acc; }

// ── Media library modal (FEAT-092 W2) ──────────────────────────────────
// Hidden in-grid by default; @portal relocation (aria-modal) shows it as a
// full-viewport overlay — the same open-marker pattern as the drawer.
.ad-medialib { display: none; }
.ad-medialib[aria-modal="true"] { display: flex; align-items: center; justify-content: center; position: fixed; inset: 0; background: rgba(10,10,10,0.45); z-index: 60; }
.ad-medialib-backdrop { position: fixed; inset: 0; z-index: 0; }
.ad-medialib-dialog { position: relative; z-index: 1; width: min(860px, 92vw); max-height: 84vh; display: flex; flex-direction: column; background: #fff; border-radius: 14px; box-shadow: 0 24px 80px rgba(0,0,0,0.25); overflow: hidden; }
.ad-medialib-head { display: flex; align-items: center; gap: 12px; padding: 16px 20px; border-bottom: 1px solid #E6E6E6; }
.ad-medialib-title { margin: 0; font-size: 16px; font-weight: 650; color: #1a1a1a; }
.ad-medialib-count { font-size: 11.5px; color: #9a9a9a; }
.ad-medialib-search { flex: 1; max-width: 320px; margin-left: auto; }
.ad-medialib-upload { flex: 0 0 auto; }
.ad-medialib-x { border: 0; background: none; font-size: 22px; line-height: 1; color: #9a9a9a; cursor: pointer; padding: 2px 6px; }
.ad-medialib-x:hover { color: #1a1a1a; }
.ad-medialib-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(132px, 1fr)); gap: 12px; padding: 18px 20px; overflow-y: auto; }
.ad-medialib-item { display: flex; flex-direction: column; gap: 6px; border: 1px solid #ececec; border-radius: 10px; padding: 8px; background: #fff; cursor: pointer; text-align: left; font: inherit; }
.ad-medialib-item:hover { border-color: #FF0020; box-shadow: 0 4px 14px rgba(0,0,0,0.07); }
.ad-medialib-thumb { display: block; width: 100%; aspect-ratio: 1; border-radius: 7px; background: #f1f1f1 center / cover no-repeat; }
.ad-medialib-name { font-size: 11px; color: #4a4a4a; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ad-media-lib { flex: 0 0 auto; font-size: 12px; }


// ── Relation graph (FEAT-092 W3) ──────────────────────────────────
// Declarative SVG: the graph pane is hidden until $view === "graph".
.ad-graph-pane { display: none; padding: 24px 30px; overflow: auto; }
.ad-graph-pane.shown { display: block; }
.ad-graph { display: block; width: 100%; max-width: 820px; margin: 0 auto; font-size: 12.5px; }
.ad-gedge { fill: none; stroke: #d8d8d8; stroke-width: 1.5; }
.ad-gdot { fill: #FF0020; }
.ad-gnode[data-side="tgt"] .ad-gdot { fill: #1a1a1a; }
.ad-gnode { cursor: default; }
.ad-gnode[data-side="src"] { cursor: pointer; }
.ad-gnode[data-side="src"]:hover .ad-gdot { fill: #c00018; }
.ad-gnode[data-side="src"]:hover .ad-glabel { fill: #FF0020; }
.ad-glabel { fill: #2a2a2a; }
.ad-gnode[data-side="tgt"] .ad-glabel { text-anchor: start; }

// The List / Graph view toggle.
.ad-viewtoggle { display: inline-flex; border: 1px solid #e2e2e2; border-radius: 8px; overflow: hidden; }
.ad-vt { border: 0; background: #fff; font: inherit; font-size: 12.5px; padding: 6px 13px; color: #6a6a6a; cursor: pointer; }
.ad-vt + .ad-vt { border-left: 1px solid #e2e2e2; }
.ad-vt--on { background: #1a1a1a; color: #fff; }

