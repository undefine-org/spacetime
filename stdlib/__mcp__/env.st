/// Spacetime MCP workbench — the organic interface birthplace (FEAT-130).
///
/// One Spacetime-authored host page for the MCP Function Environment. The design
/// language is the warm-dark "organic interface birthplace"
/// (@research/mcp-design-language-organic-birthplace.org): the live mounted
/// content is the STAGE (dominant, warm-glowing); chrome RECEDES into edge
/// button-drawers (nav / inspector / origin / comms) summoned by warm glyphs.
///
/// Topology (mockups m1-m6):
///   - slim topbar: serif wordmark + `host · stdlib` pill + live ember + glyphs.
///   - stage (~70%, region `stage`): mounts content; empty state = "Begin here".
///   - left edge: nav summon glyph (environments + tabs).
///   - right edge: inspector + origin summon glyphs.
///   - bottom edge: comms summon glyph (agent affordances).
///   - drawers: `data-st-region="drawer-X" data-st-drawer`, overlay the stage,
///     shown when the `$drawer` signal equals X. One open at a time falls out of
///     a single string signal (no custom JS — `@on click { $drawer <- "X" }`).
///
/// This file owns page structure + the drawer open-state wiring; data, templates,
/// behaviour, and the warm-dark visual system live in `workbench/` siblings.

@import "stdlib/__mcp__";
@import "stdlib/fields";
@import "stdlib/__mcp__/workbench/data.st";
@import "stdlib/__mcp__/workbench/cards.st";
@import "stdlib/__mcp__/workbench/lists.st";
@import "stdlib/__mcp__/workbench/actions.st";
@import "stdlib/__mcp__/workbench/styles.st";

// The single drawer-open signal: "none" | "nav" | "inspector" | "origin" | "comms".
// One open at a time is intrinsic to a single-value signal.
@data inline $drawer : "none";

// M3' in-stage selection (PLAN-046): the SELECTED frame id, or "" for none.
// Clicking the composed stage frame sets `$selected <- "stage"` and opens the
// inspector, which binds to the selected frame's contract. Mirrors the $drawer
// pattern (a single string signal + reactive classes); no custom JS.
@data inline $selected : "";

// PLAN-051 B0: the builder BREAKPOINT preview. One string signal
// "desktop" | "tablet" | "mobile" drives the stage frame's max-width via a
// reactive `data-breakpoint` attr on the shell (mirrors $drawer/$selected; no
// page JS). The region mount is untouched — only the frame's width changes, so
// the guest re-lays-out at the device width (the Webflow/Squarespace gesture).
@data inline $breakpoint : "desktop";

// PLAN-051 B3: the SELECTION SPINE node id. Clicking a navigator row sets this
// to that node's structure id (-1 = nothing selected). The selected nav row
// lights via a reactive class; the inspector's node-detail section filters the
// structure to this id (a `when` @each). One signal, two synced views — the
// builder selection spine (mirrors $selected/$drawer). Numeric to match the
// structure projection's integer node ids.
@data inline $selectedNode : "";

// PLAN-065 W_session: the CLIENT-OWNED editor session. On selection, the click
// handlers COPY the selected node's editable rows out of the SERVER poll arrays
// ($mcpStageNodeBindings / $mcpStageNodeParams) into these client signals; the
// inspector's editable @each lists (lists.st) iterate THESE, not the poll arrays.
// Because @each watches only the signal it iterates, and these change ONLY on
// selection (never on the 1s poll), a focused editing input is never destroyed
// mid-edit by a repaint (BUG-151). This is the ownership split the whole rethink
// rests on: SERVER arrays inform read-only lists; the CLIENT owns edit state.
@data inline $activeBindings : array = [];
@data inline $activeParams : array = [];
// The frame-level (whole composed guest) editable params, client-owned for the
// same poll-survival reason as $activeBindings/$activeParams. Projected from
// $mcpStageParams when a frame is selected ($selected == "stage").
@data inline $activeFrameParams : array = [];

<div class="mcp-stage-shell" data-drawer="`$drawer`" data-selected="`$selected`" data-breakpoint="`$breakpoint`">

  <!-- Slim topbar: wordmark · host pill · live ember. -->
  <header class="mcp-topbar">
    <span class="mcp-wordmark">Spacetime</span>
    <span class="mcp-host-pill"><span class="mcp-host-dot"></span>host · stdlib</span>
    <span class="mcp-live"><span class="mcp-ember"></span>live</span>

    <!-- PLAN-051 B0: breakpoint switch. Each button sets $breakpoint; the active
         one lights via a reactive class. Drives the stage frame width (styles).
         Pure Spacetime (signal + @on &.click + reactive class), no page JS. -->
    <div class="mcp-breakpoints" role="group" aria-label="Preview width">
      <button class="mcp-bp" data-bp="desktop" title="Desktop">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M9 20 h6 M12 16 v4"/></svg>
      </button>
      <button class="mcp-bp" data-bp="tablet" title="Tablet">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="3" width="14" height="18" rx="1.6"/><path d="M11 18 h2"/></svg>
      </button>
      <button class="mcp-bp" data-bp="mobile" title="Mobile">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="7" y="3" width="10" height="18" rx="1.6"/><path d="M11 18 h2"/></svg>
      </button>
    </div>
  </header>

  <!-- The stage: dominant warm surface; mounted content lives here. FEAT-126:
       `@view $stageTpl` shows the empty-state until a guest function is composed
       into the `stage` region (st_mount host=inst-N region=stage), then mounts
       the guest's main template reactively. `@mcp-region` polls the bundle set. -->
  <main class="mcp-stage" data-st-region="stage">
    <!-- Empty-state is the region root's initial child. When a guest is composed
         into the `stage` region, `@view` (bound on the region root below) mounts
         the guest's main template as a SIBLING of this empty-state; a CSS rule
         hides the empty-state once a guest sibling exists. No page JS. -->
    <div class="mcp-stage-empty">
      <div class="mcp-sprout" aria-hidden="true">
        <svg viewBox="0 0 32 36" width="46" height="52" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M16 33 V18"/>
          <path d="M16 20 C16 13 10 11 5 11 C5 17 9 21 16 21 Z"/>
          <path d="M16 17 C16 10 22 7 28 7 C28 13 23 18 16 18 Z"/>
          <path d="M14 6 l2 -4 l2 4"/>
          <circle cx="11" cy="4" r="0.8" fill="currentColor" stroke="none"/>
          <circle cx="22" cy="3" r="0.8" fill="currentColor" stroke="none"/>
        </svg>
      </div>
      <h1 class="mcp-begin">Begin here</h1>
      <p class="mcp-begin-note">Pick a function to compose onto the stage,<br>or let the agent send one.</p>
      <!-- BUG-122: frame-first active-environment view. Opening an environment
           (st_env_open / the nav drawer's "Open environment") now SHOWS in the
           dominant surface, not just a buried drawer card. `.mcp-active-env`
           binds the already-published `$mcpOpenEnvironments` + `$mcpTabs`
           (data.st); when an env is open, CSS `:has` reveals this block and
           recedes the "Begin here" zero-state + gallery. No second @view (the
           guest-compose `$stageTpl` slot is untouched), no server change. The
           empty containers populate reactively from the list bindings (lists.st). -->
      <section class="mcp-active-env">
        <p class="mcp-card-label">open environment</p>
        <div class="mcp-active-env-cards"></div>
        <p class="mcp-card-label mcp-active-env-tabs-label">tabs</p>
        <div class="mcp-active-env-tabs"></div>
        <p class="mcp-active-env-empty-tabs">No tabs open yet — open one with <code>st_tab_open</code>.</p>
      </section>
      <!-- M2 living gallery: one clickable tile per registered function; click
           composes it onto the stage (mcp_compose → mount_into_host). The list
           binding lives in lists.st (`.mcp-gallery { @each(…) }`) — in-page
           `@each` must be a SELECTOR-scoped block, not inline HTML-body text.
           Empty container; populated reactively from $mcpFunctions. -->
      <div class="mcp-gallery"></div>
      <p class="mcp-gallery-empty-hint">No functions yet — register one with <code>st_fn_put</code>.</p>
      <!-- PLAN-049 W3: the "+" creation surface. A collapsible scratch editor so
           the HUMAN can START a frame (not just rearrange agent-seeded ones).
           On "Create", `@mcp-submit(action: "mcp_create")` collects the region's
           named controls — here create_name + create_source — and the sink
           registers an Agent-origin function + composes it onto the stage. Pure
           Spacetime (reuses the kit's collect-submit primitive). -->
      <details class="mcp-create">
        <summary><span class="mcp-create-plus" aria-hidden="true">+</span> new frame</summary>
        <div class="mcp-create-body">
          <input class="mcp-create-name" name="create_name" placeholder="name (optional)" spellcheck="false">
          <textarea class="mcp-create-source" name="create_source" spellcheck="false" placeholder="@template &main() { &lt;h1&gt;Hello&lt;/h1&gt; }"></textarea>
          <button class="mcp-create-submit">Create &amp; compose</button>
          <p class="mcp-create-hint">Registers a frame + composes it onto the stage. A compile error is reported and nothing is composed.</p>
        </div>
      </details>
    </div>
  </main>

  <!-- PLAN-049 W1.S2: clear-stage control. A sibling of the stage (NOT a child,
       so it never trips the empty-state `:has` reveal). Shown only when a guest
       is composed on the stage (shell-level `:has`, styles.st). Clicking fires
       `mcp_stage_clear` (server-audience), emptying the stage region. -->
  <button class="mcp-stage-clear" title="Clear stage">
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6 l12 12 M18 6 l-12 12"/></svg>
    <span>clear stage</span>
  </button>

  <!-- PLAN-049 W4.S8: the frame strip. A sibling of the stage (NOT a child, so
       it never trips the empty-state `:has`). Lists every region composed on the
       stage host; each chip can be unmounted from its region. Shown only when a
       frame exists (shell-level `:has`, styles.st). Empty container; populated
       reactively from $mcpStageFrames (lists.st). -->
  <div class="mcp-frame-strip"></div>

  <!-- Left edge: navigation summon (environments + tabs) + structure navigator. -->
  <nav class="mcp-rail mcp-rail-left">
    <button class="mcp-summon" data-summon="nav" title="Environments &amp; tabs" aria-label="Environments &amp; tabs">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7 a2 2 0 0 1 2 -2 h4 l2 2 h6 a2 2 0 0 1 2 2 v8 a2 2 0 0 1 -2 2 H5 a2 2 0 0 1 -2 -2 Z"/></svg>
    </button>
    <!-- PLAN-051 B2: the builder NAVIGATOR summon (the composed guest's template
         structure tree). Shown as a layers glyph; opens the structure drawer. -->
    <button class="mcp-summon" data-summon="structure" title="Structure" aria-label="Structure navigator">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6 h10 M4 12 h13 M4 18 h7"/><circle cx="19" cy="6" r="1.4"/><circle cx="20" cy="12" r="1.4"/></svg>
    </button>
  </nav>

  <!-- Right edge: inspector + origin summons. -->
  <nav class="mcp-rail mcp-rail-right">
    <button class="mcp-summon" data-summon="inspector" title="Inspector" aria-label="Inspector">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 11 v5"/><circle cx="12" cy="8" r="0.9" fill="currentColor" stroke="none"/></svg>
    </button>
    <button class="mcp-summon" data-summon="origin" title="Origin registry" aria-label="Origin registry">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12 V6 a2 2 0 0 1 2 -2 h6 l8 8 -8 8 -8 -8 Z"/><circle cx="8.5" cy="8.5" r="1.3"/></svg>
    </button>
  </nav>

  <!-- Bottom edge: agent comms summon. -->
  <nav class="mcp-rail mcp-rail-bottom">
    <button class="mcp-summon" data-summon="comms" title="Agent comms" aria-label="Agent comms">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6 a2 2 0 0 1 2 -2 h12 a2 2 0 0 1 2 2 v8 a2 2 0 0 1 -2 2 H9 l-4 4 V6 Z"/></svg>
    </button>
  </nav>

  <!-- Scrim behind an open drawer: dims + recedes the stage; click to dismiss. -->
  <div class="mcp-scrim"></div>

  <!-- PLAN-064 U6: server-action ECHO. Server-audience actions (compose, param,
       clear, close…) resolve synchronously and never reach the agent-comms feed,
       so a click had NO acknowledgement until the 1s poll repainted. This toast
       reacts to the host bridge's `$mcpLastMessage` (every action's signal
       response), showing the action name so a click FEELS acknowledged. A CSS
       key-frame re-triggers on each change (keyed off the reactive text) and
       fades it out — pure reactive binding + CSS, no page JS. -->
  <div class="mcp-action-toast" aria-live="polite"><span class="mcp-action-toast-dot"></span><span class="mcp-action-toast-text"></span></div>

  <!-- Drawer: navigation (environments + tabs). -->
  <aside class="mcp-drawer mcp-drawer-nav mcp-drawer-left" data-st-region="drawer-nav" data-st-drawer role="dialog" aria-label="Environments &amp; tabs">
    <header class="mcp-drawer-head">
      <h2>environments</h2>
      <button class="mcp-drawer-close" title="Close" aria-label="Close drawer">&times;</button>
    </header>
    <div class="mcp-drawer-body">
      <p class="mcp-drawer-label">discovered</p>
      <div class="mcp-env-grid"></div>
      <p class="mcp-drawer-label">open</p>
      <div class="mcp-open-envs"></div>
      <!-- PLAN-049 W2 (COLLAPSE): the entry-point picker. One "+ tab" row per
           discoverable `.st` across open envs; click registers it as a tab
           (env-origin function). Empty until an env is open. -->
      <p class="mcp-drawer-label">add a tab</p>
      <div class="mcp-entry-list"></div>
      <p class="mcp-entry-empty">Open an environment to list its <code>.st</code> entries.</p>
      <p class="mcp-drawer-label">tabs</p>
      <div class="mcp-tab-list"></div>
    </div>
  </aside>

  <!-- Drawer: structure navigator (PLAN-051 B2). The composed guest's template
       graph as an indented tree, projected by the server ($mcpStageStructure)
       and rendered by `.mcp-nav-tree`'s @each (lists.st). Clicking a node row
       selects it (B3). Empty hint until a guest is on the stage. -->
  <aside class="mcp-drawer mcp-drawer-structure mcp-drawer-left" data-st-region="drawer-structure" data-st-drawer role="dialog" aria-label="Structure navigator">
    <header class="mcp-drawer-head">
      <h2>structure</h2>
      <button class="mcp-drawer-close" title="Close" aria-label="Close drawer">&times;</button>
    </header>
    <div class="mcp-drawer-body">
      <div class="mcp-nav-tree"></div>
      <p class="mcp-nav-empty">No guest on the stage. Compose one to see its structure.</p>
    </div>
  </aside>

  <!-- Drawer: origin registry (every loaded fragment + its provenance chip). -->
  <aside class="mcp-drawer mcp-drawer-origin mcp-drawer-right" data-st-region="drawer-origin" data-st-drawer role="dialog" aria-label="Origin registry">
    <header class="mcp-drawer-head">
      <h2>origin</h2>
      <button class="mcp-drawer-close" title="Close" aria-label="Close drawer">&times;</button>
    </header>
    <div class="mcp-drawer-body">
      <div class="mcp-function-list"></div>
    </div>
  </aside>

  <!-- Drawer: inspector (the mounted region's contract — FEAT-132). -->
  <aside class="mcp-drawer mcp-drawer-inspector mcp-drawer-right" data-st-region="drawer-inspector" data-st-drawer role="dialog" aria-label="Inspector">
    <header class="mcp-drawer-head">
      <h2>inspector</h2>
      <button class="mcp-drawer-close" title="Close" aria-label="Close drawer">&times;</button>
    </header>
    <div class="mcp-drawer-body">
      <!-- PLAN-051 B3: the SELECTED NODE detail. When a navigator node is
           selected ($selectedNode), this filtered @each (lists.st) renders the
           matching structure node's detail (&mcp-node-detail) — the inspector
           half of the selection spine. Empty hint until a node is selected. -->
      <section class="mcp-selected-node">
        <p class="mcp-drawer-label">selected node</p>
        <div class="mcp-node-detail"></div>
        <p class="mcp-node-empty">No node selected. Click a row in the structure navigator.</p>
        <!-- PLAN-051 B4: the SELECTED node's EDITABLE param rows. A filtered @each
             (lists.st) renders one input per bound param of the selected node;
             editing fires `mcp_node_param` -> a source patch at the node's
             invoke_span (the G2 WRITE rail), hot-swapping the guest. Empty until
             a node with bound params is selected. -->
        <p class="mcp-drawer-label mcp-node-params-label">edit params</p>
        <div class="mcp-node-params"></div>
        <!-- PLAN-064 B4a: the SELECTED ELEMENT's editable BINDING rows. When an
             element (an <h1>, a <p>) is selected, its holes that read entry-template
             params become editable here — editing fires `mcp_param` (the value rail,
             provenance dispatch). This is what makes clicking a hero's title on the
             canvas and typing a new one actually work. Empty until an element with
             param bindings is selected. -->
        <p class="mcp-drawer-label mcp-node-bindings-label">edit content</p>
        <div class="mcp-node-bindings"></div>
      </section>
      <!-- M3' selected frame: when a stage frame is selected, this section shows
           ITS contract (name + params). `data-st-selected` mirrors `$selected`
           so CSS reveals this block only while a frame is selected. -->
      <section class="mcp-selected-frame">
        <p class="mcp-drawer-label">selected frame</p>
        <h3 class="mcp-selected-name">`$mcpStageMountName`</h3>
        <div class="mcp-selected-params"></div>
        <!-- M6 inline source edit (BUG-114): a USER edit + commit (change) fires
             `mcp_source_edit` (actions.st), which recompiles + hot-swaps the live
             guest. A bad edit leaves the running guest unchanged (server-guarded).
             The action is guarded to USER-trusted events only, so the reactive
             `text: $mcpStageSource` binding setting the value during hydration
             never self-dispatches an edit (it previously corrupted the guest). -->
        <details class="mcp-source-editor">
          <summary>edit source</summary>
          <textarea class="mcp-source-input" spellcheck="false"></textarea>
          <p class="mcp-source-hint">Edit, then commit (blur) to save · recompiles + hot-swaps. A compile error leaves the guest unchanged.</p>
        </details>
        <p class="mcp-selected-empty">Nothing selected. Click a frame on the stage.</p>
      </section>
      <p class="mcp-drawer-label">all contracts</p>
      <div class="mcp-contract-list"></div>
      <p class="mcp-drawer-label">mounted</p>
      <div class="mcp-instance-list"></div>
    </div>
  </aside>

  <!-- Drawer: agent comms (instances + recent host events). -->
  <aside class="mcp-drawer mcp-drawer-comms mcp-drawer-bottom" data-st-region="drawer-comms" data-st-drawer role="dialog" aria-label="Agent comms">
    <header class="mcp-drawer-head">
      <h2>comms</h2>
      <button class="mcp-drawer-close" title="Close" aria-label="Close drawer">&times;</button>
    </header>
    <div class="mcp-drawer-body">
      <!-- M5: the agent-comms feed. Each event renders as a card (agent vs server
           accent). Empty hint shows until the first event; CSS hides it via :has
           once any card exists. -->
      <div class="mcp-event-list"></div>
      <p class="mcp-comms-empty">No messages yet. Agent actions and your answers appear here.</p>
    </div>
  </aside>

</div>

// --- Drawer open-state wiring (pure Spacetime: signal + @on click) ---------
// Each summon sets `$drawer` to its target; the close glyphs and the scrim
// reset to "none". The visual open/closed transitions are reactive classes
// driven by `$drawer` (see styles.st) — no custom page JS.
.mcp-summon[data-summon="nav"]       { @on &.click { $drawer <- "nav"; } }
.mcp-summon[data-summon="structure"] { @on &.click { $drawer <- "structure"; } }
.mcp-summon[data-summon="inspector"] { @on &.click { $drawer <- "inspector"; } }
.mcp-summon[data-summon="origin"]    { @on &.click { $drawer <- "origin"; } }
.mcp-summon[data-summon="comms"]     { @on &.click { $drawer <- "comms"; } }
.mcp-drawer-close { @on &.click { $drawer <- "none"; } }

// PLAN-065 W_hygiene (swarm P2): composing a DIFFERENT guest onto the stage (or
// clearing it) invalidates the client editor session — the old $activeFrameParams
// / $activeBindings / $activeParams belong to the PREVIOUS guest. These selectors
// ALSO carry an @mcp-action (compose/clear, actions.st); the @on click here RESETS
// the editor session alongside, so the inspector never renders (or edits) stale
// rows against the new guest. Selection restarts clean on the next canvas/nav
// click. (Both directives coexist on one selector.)
.mcp-gallery-card, .mcp-instance-focus, .mcp-tab-stage, .mcp-stage-clear, .mcp-frame-unmount {
  @on &.click { $selectedNode <- ""; $activeBindings <- []; $activeParams <- []; $activeFrameParams <- []; }
}
.mcp-scrim        { @on &.click { $drawer <- "none"; } }

// PLAN-064 U3: keyboard — Escape closes any open drawer AND clears the node
// selection (the Figma/Webflow reflex). Attached to the INSTANCE ROOT
// (`[data-st-instance]`, the workbench body root where focus rests by default)
// — NOT the inner shell — so a keydown fires even when nothing inside is
// focused (review fix: a keydown on `body` does not bubble DOWN into the shell;
// the instance root IS the focus target / bubble path). Pure signal reset, no
// page JS.
//
// BUG-375: this was written `@on &.keydown:Escape`, and the comment here
// claimed that spelling emitted an `e.key === 'Escape'` filtered listener. It
// did not — `driver_expr` has no qualifier slot, so `:Escape` was parsed and
// DISCARDED, and this handler ran on EVERY keydown: typing anywhere in the
// workbench cleared the inspector's selection and closed its drawer. The key is
// now a driver PARAM, which is the spelling that actually filters.
[data-st-instance] { @on &.keydown("Escape") { $drawer <- "none"; $selectedNode <- ""; $activeBindings <- []; $activeParams <- []; $activeFrameParams <- []; } }

// --- PLAN-064 B3: the UNIFIED node selection spine ------------------------
// ONE signal ($selectedNode, a dotted node id like "0.0") is driven from BOTH
// directions:
//   (a) the NAVIGATOR row — clicking a structure tree row selects that node;
//   (b) the CANVAS — clicking a stamped guest ELEMENT selects the SAME node.
// Both read `data-st-node` (the id the stamped builder wrote onto the guest DOM,
// == the navigator node id — PLAN-064 B3.2). A navigator row also carries
// `data-st-node`, so both handlers are the same shape. Selecting opens the
// inspector, whose node-detail + editable-param sections filter to $selectedNode.
// PLAN-065 W_session: selecting also PROJECTS the node's editable rows from the
// server poll arrays into the client-owned $activeBindings/$activeParams (the
// inspector edits those, so they survive the poll — BUG-151). Statements run
// sequentially and reads see prior writes, so the filter uses the just-set
// $selectedNode. The projection is a COPY: subsequent polls do not touch it.
.mcp-nav-node {
  @on &.click {
    $selectedNode <- $.closest('[data-st-node]').dataset.stNode;
    $activeBindings <- ST.bindingEditorRows($mcpStageNodeBindings.filter((b) => b.node_id == $selectedNode));
    $activeParams <- ST.fieldEditorRows($mcpStageNodeParams.filter((p) => p.node_id == $selectedNode));
    $drawer <- "inspector";
  }
}
// The CANVAS click is DELEGATED to the nearest stamped element: clicking a nested
// guest node (an <h1> inside a <section>) resolves via `closest` to that element's
// id. This REPLACES the old coarse `$selected <- "stage"` whole-frame select —
// selection is now element-granular (the Webflow/Figma gesture). It ALSO sets
// $selected so the frame-level inspector section (guest name + inline source
// editor) stays reachable: one canvas click populates BOTH the node-detail (via
// $selectedNode) and the frame-detail (via $selected) inspector views.
.mcp-stage[data-st-region="stage"] {
  @on &.click [data-st-node] {
    $selectedNode <- $.closest('[data-st-node]').dataset.stNode;
    $activeBindings <- ST.bindingEditorRows($mcpStageNodeBindings.filter((b) => b.node_id == $selectedNode));
    $activeParams <- ST.fieldEditorRows($mcpStageNodeParams.filter((p) => p.node_id == $selectedNode));
    $activeFrameParams <- $mcpStageParams;
    $selected <- "stage";
    $drawer <- "inspector";
  }
}

// --- PLAN-051 B0: breakpoint switch wiring (signal + @on click + reactive class)
// Each button sets $breakpoint; the active button lights via `.is-active`. The
// stage frame width is driven by `data-breakpoint` on the shell (styles.st).
.mcp-bp[data-bp="desktop"] { @on &.click { $breakpoint <- "desktop"; } }
.mcp-bp[data-bp="tablet"]  { @on &.click { $breakpoint <- "tablet"; } }
.mcp-bp[data-bp="mobile"]  { @on &.click { $breakpoint <- "mobile"; } }
.mcp-bp[data-bp="desktop"] { .is-active: $breakpoint == "desktop"; }
.mcp-bp[data-bp="tablet"]  { .is-active: $breakpoint == "tablet"; }
.mcp-bp[data-bp="mobile"]  { .is-active: $breakpoint == "mobile"; }

// Reactive open classes: the scrim and each drawer light up when `$drawer`
// matches. One open at a time is intrinsic to the single signal value.
.mcp-scrim            { .is-active: $drawer != "none"; }
.mcp-drawer-nav       { .is-open: $drawer == "nav"; }
.mcp-drawer-structure { .is-open: $drawer == "structure"; }
.mcp-drawer-origin    { .is-open: $drawer == "origin"; }
.mcp-drawer-inspector { .is-open: $drawer == "inspector"; }
.mcp-drawer-comms     { .is-open: $drawer == "comms"; }

// PLAN-064 U4 (review fix): a CLOSED drawer is present in the DOM but hidden; a
// static `aria-modal="true"` would announce every hidden drawer as an active
// modal. Bind `aria-hidden` reactively to the CLOSED state so only the OPEN
// drawer is exposed to assistive tech (the reactive-attr `[attr]: expr` idiom).
.mcp-drawer-nav       { [aria-hidden]: $drawer != "nav"; }
.mcp-drawer-structure { [aria-hidden]: $drawer != "structure"; }
.mcp-drawer-origin    { [aria-hidden]: $drawer != "origin"; }
.mcp-drawer-inspector { [aria-hidden]: $drawer != "inspector"; }
.mcp-drawer-comms     { [aria-hidden]: $drawer != "comms"; }

// --- M3' in-stage selection (PLAN-046) -------------------------------------
// Clicking the composed stage frame SELECTS it (the Figma loop) and opens the
// inspector bound to that frame. A second signal `$selected` (mirrors $drawer's
// pattern). The empty-state is NOT selectable — only a mounted guest is, so the
// click handler sits on the guest-bearing region. Re-clicking the stage keeps it
// selected; the close glyph / scrim leave selection intact (selection outlives a
// drawer toggle, like Figma). Selecting also opens the inspector so the contract
// is visible immediately.
// PLAN-064 B3: the coarse whole-frame stage click was REMOVED (superseded by the
// element-granular delegated `@on click [data-st-node]` above). A click that
// lands on the stage but NOT on any stamped element (the padding around the
// artboard) now selects nothing new — the last node stays selected (Figma-like).
// The stage border no longer doubles as a selection ring; the per-element ring
// (below) is the selection affordance.
.mcp-stage[data-st-region="stage"] { .is-selected: $selected == "stage"; }
// The selected-frame inspector section reveals its content vs the empty hint
// based on whether anything is selected (CSS reads `data-selected` on the shell).
.mcp-selected-frame { .has-selection: $selected != ""; }
// PLAN-064 B3: the per-element SELECTION RING. Every stamped guest element gets a
// reactive `.is-selected-node` class that is true when ITS OWN `data-st-node` id
// (read via the reactive-render `node` alias — NOT `$`, which is event-target
// context) equals $selectedNode. The ring itself is CSS (styles.st). This is the
// canvas half of the selection spine: selecting a navigator row lights the
// matching element, and clicking an element lights it — one signal, both ways.
// Pure reactive class + CSS: the ring follows layout/scroll with no rect JS.
.mcp-stage[data-st-region="stage"] [data-st-node] { .is-selected-node: node.dataset.stNode == $selectedNode; }
// M6: bind the source editor's textarea content to the selected guest's source.
// `text:` is the proven reactive content binding (same idiom the kit uses for
// `text: $mcpPrompt`); for a <textarea> textContent IS the value. Reads the
// published `$mcpStageSource` via the data registry, so it tracks selection.
.mcp-source-input { text: $mcpStageSource; }

// PLAN-064 U6 + PLAN-065 W_feedback (BUG-153): the action toast acknowledges
// every server-audience click AND makes failures VISIBLE. The signal sink returns
// `{ok:true, event, action}` where `action.ok` is the real outcome; on failure
// `action.error` carries the server's message. So:
//   - on FAILURE (`action.ok == false`) show `action.error` and flag `.is-error`
//     (styles.st paints it red + holds it longer);
//   - on SUCCESS show `action.summary` (a human sentence like "Opened environment
//     projects/x.") when present, else fall back to the action name.
// The host bridge nulls `$mcpLastMessage` ~2.8s after each action, so `.is-shown`
// goes true->false and the toast is transient + re-triggerable. Pure reactive
// text + class, no page JS. A click now ALWAYS produces immediate feedback,
// independent of the 1s snapshot poll.
.mcp-action-toast-text { text: $mcpActionText; }
.mcp-action-toast {
  .is-shown: $mcpActionText != null;
  .is-error: $mcpActionOk == false;
}

// --- Stage region mount (FEAT-126 / BUG-111) -------------------------------
// The stage region root carries BOTH the bundle poll and the reactive mount:
//   @mcp-region  starts polling `/bundles?region=stage`; a guest bundle arrival
//                calls `registerBundle`, which sets the element-scoped `stageTpl`
//                signal ON THIS ROOT and registers the guest's templates.
//   @view        is bound on the SAME root, so its `ST.get(host,'stageTpl')`
//                reads exactly what registerBundle wrote; the wildcard arm mounts
//                the guest's main template (`&$stageTpl()`) as a child.
// The empty-state (region's initial child) is hidden by CSS once a guest sibling
// mounts. Scoped to the stage alone so drawers don't spin up their own polls.
.mcp-stage[data-st-region="stage"] {
  @mcp-region
  @view $stageTpl {
    _ => &$stageTpl();
  }
}
