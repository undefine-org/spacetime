// =============================================================================
// App Shell demo (FEAT-074) — @view + @portal + @on $sig.change composed into one app.
//
// A 3-pane admin-style shell:
//   - the rail swaps the center @view between panes (list | graph | settings)
//   - a @portal modal escapes the layout grid with the full modal tax
//   - an @on $sig.change reacts to a data signal (bumps a live "sync" counter)
//
// All behavior is declarative Spacetime — no custom JavaScript.
// =============================================================================

body {
  $pane string: "list";        // which center pane is shown
  $modalOpen bool: false;      // the @portal modal
  $rows number: 3;             // a data signal the @on $sig.change watches
  $syncs number: 0;            // how many times the effect has fired
}

// ── Panes (templates the @view mounts) ──────────────────────────────────────
@template &listPane() {
  <div class="pane pane--list">
    <h2>List</h2>
    <ul><li>Alpha</li><li>Beta</li><li>Gamma</li></ul>
  </div>
}
@template &graphPane() {
  <div class="pane pane--graph">
    <h2>Graph</h2>
    <div class="bars"><span></span><span></span><span></span></div>
  </div>
}
@template &settingsPane() {
  <div class="pane pane--settings">
    <h2>Settings</h2>
    <p>Synced <strong class="sync-count">0</strong> times.</p>
  </div>
}

// ── The reactive view: mount the arm matching $pane, re-mount on change ──────
.stage { @view $pane { "list" => &listPane(); "graph" => &graphPane(); "settings" => &settingsPane(); } }

// ── Rail buttons drive the view + open the modal ─────────────────────────────
.rail__list     { @on &.click { $pane <- "list"; } }
.rail__graph    { @on &.click { $pane <- "graph"; } }
.rail__settings { @on &.click { $pane <- "settings"; } }
.rail__add      { @on &.click { $rows <- $rows + 1; } }
.rail__open     { @on &.click { $modalOpen <- true; } }

// ── The modal escapes the layout via @portal (scroll-lock, focus-trap, Esc) ──
.modal { @portal(when: $modalOpen); }
.modal__close { @on &.click { $modalOpen <- false; } }

// ── A side-effect: every data change bumps the live sync counter ─────────────
.app { @on $rows.change { $syncs <- $syncs + 1; } }

// ── Live readouts (file-scope reactive content) ──────────────────────────────
.sync-count { text <- $syncs; }
.row-count  { text <- $rows; }

// ============================================================================
// Styling (plain CSS).
// ============================================================================
body {
  margin: 0;
  min-height: 100vh;
  background: #0b0c10;
  color: #e8eaf2;
  font-family: "Inter", system-ui, sans-serif;
}

.app {
  display: grid;
  grid-template-columns: 200px 1fr;
  min-height: 100vh;
}

.rail {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 1.5rem 1rem;
  background: #14161f;
  border-right: 1px solid #232634;
}

.rail button {
  appearance: none;
  border: 1px solid #2a2e3e;
  background: #1a1d29;
  color: #cdd2e4;
  padding: 0.6rem 0.9rem;
  border-radius: 8px;
  text-align: left;
  cursor: pointer;
  font: inherit;
}

.rail button:hover { background: #222637; }

.stage { padding: 2.5rem; }

.pane h2 { margin: 0 0 1rem; font-size: 1.6rem; }
.pane ul { line-height: 1.9; color: #aeb4c8; }

.bars { display: flex; gap: 0.5rem; align-items: flex-end; height: 120px; }
.bars span { width: 40px; background: linear-gradient(#6c7cff, #3a48c0); border-radius: 6px 6px 0 0; }
.bars span:nth-child(1) { height: 40%; }
.bars span:nth-child(2) { height: 80%; }
.bars span:nth-child(3) { height: 55%; }

// The modal: fixed/centered while open (it lives at <body> via @portal).
.modal {
  position: fixed;
  inset: 0;
  margin: auto;
  width: min(440px, 90vw);
  height: fit-content;
  padding: 2rem;
  background: #181b26;
  border: 1px solid #2a2e3e;
  border-radius: 14px;
  box-shadow: 0 30px 80px rgba(0,0,0,0.6);
  z-index: 1000;
}

.modal[aria-modal="true"] ~ * { filter: blur(2px); }
.modal__close {
  appearance: none;
  border: 1px solid #2a2e3e;
  background: #222637;
  color: #cdd2e4;
  padding: 0.5rem 1rem;
  border-radius: 8px;
  cursor: pointer;
  font: inherit;
}
