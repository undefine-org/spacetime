// =============================================================================
// Spacetime CMS Admin — declarative app shell (FEAT-076 rebuild)
//
// This is the full-Spacetime replacement for the 1349-line admin-app.st
// %emit js monolith. The admin is a META-RENDERER: it knows no project types
// at compile time. It fetches the server contract at runtime —
//   /__spacetime/dev/site.json   → { dataSourceMap: { <file>: {type,isArray,name} }, pages }
//   /__spacetime/dev/types.json  → { types: { <T>: {properties{widget}, display{roles}} } }
// — derives its collection list, and renders rail · center · drawer entirely
// from platform constructs (@data, @each, @view, @portal, @match, : / <-).
//
// Only persist.st keeps %emit js (the dev-ws bridge). Everything here is
// declarative. The editorial identity is preserved: white surfaces, near-black
// text, a single scarlet accent.
// =============================================================================

// ── App state (file-scope signals) ──────────────────────────────────────────
body {
  $activeFile string: "";       // selected collection's data file (the key)
  $activeIndex number: -1;      // selected entry index within the collection (-1 = none)
  $filter string: "";           // collection search query
  $pane string: "list";         // center pane: list | graph | brand | palette | motion
  $view string: "list";         // within the content pane: list | graph (relation graph, W3)
  $drawerOpen bool: false;      // entry editor drawer (@portal)
  $mediaOpen bool: false;       // media library modal (@portal, FEAT-092)
  $mediaPath string: "";        // the field path the media library is editing
  $mediaQuery string: "";       // media library live-search query
}

// ── The server contract (fetched at runtime) ────────────────────────────────
@data fetch $site : "/__spacetime/dev/site.json";
@data fetch $typeDoc : "/__spacetime/dev/types.json";

// Collections: the server contract already exposes a render-ready array
// (file key inlined, sorted by name, singles filtered). The admin @each-renders
// it directly — no client-side re-derivation (mirrors the types.json contract).
@data derive $collections : $site.collections;

// The map of type schemas (T → {properties, display, required}).
@data derive $types : $typeDoc.types;

// Header reflectors. $activeName is set by the rail when a collection is
// selected (the nav button carries the name); $entryCountLabel derives from the
// loaded rows. Derive exprs avoid arrow/brace bodies (the balanced(';') grammar).
// $entries is always an array (reactive-source defaults to []), so no guard
// needed. Derive exprs avoid leading-paren + member-in-ternary forms (two
// open derive-grammar gaps, BUG-077).
@data inline $activeName string: "";
@data inline $activeType string: "";
@data inline $activeHasRelations bool: false;   // active type has relation fields (W3 graph toggle)
@data inline $noop string: "";

// Drawer (entry editor) signals. $activeEntry = the selected row object;
// $activeSchema = its type's schema; $formFields = a flattened render list of
// {name,node,value,widget,path} descriptors the form @each-renders. Computed in
// @on $sig.change bodies (full JS) when the selection or data changes.
@data inline $activeEntry : null;
@data inline $formFields : [];
@data inline $drawerTitle string: "";
@data inline $delArmed bool: false;
@data derive $entryCount : $entries.length;
@data derive $entryCountLabel : $entryCount + (1 === $entryCount ? " entry" : " entries");

// ── The shell: rail · center · drawer (file-scope markup → DOM) ──────────────
// A full-window 3-zone grid. The rail lists collections + tool views; the
// center hosts the active pane; the drawer slides in as a @portal modal.
// The skeleton is declared here as nested file-scope markup; selector scopes in
// rail.st / list.st / drawer.st attach the dynamic @each + @on behavior. The
// .ad-nav--collections nav is an empty mount point the rail's @each fills.
<div class="ad-root">
  <aside class="ad-rail">
    <div class="ad-brand">Content</div>
    <div class="ad-rail-section">Collections</div>
    <nav class="ad-nav ad-nav--collections"></nav>
    <div class="ad-rail-divider"></div>
    <nav class="ad-nav ad-nav--tools">
      <button class="ad-nav-item ad-tool ad-tool--brand" data-pane="brand" type="button"><span class="ad-nav-label">Brand</span></button>
      <button class="ad-nav-item ad-tool ad-tool--palette" data-pane="palette" type="button"><span class="ad-nav-label">Components</span></button>
      <button class="ad-nav-item ad-tool ad-tool--motion" data-pane="motion" type="button"><span class="ad-nav-label">Motion</span></button>
    </nav>
    <div class="ad-rail-foot"><span class="ad-pill">Local &middot; saved</span></div>
  </aside>
  <main class="ad-center">
    <header class="ad-head">
      <div class="ad-head-titles">
        <h1 class="ad-title ad-title--active"></h1>
        <span class="ad-head-count"></span>
      </div>
      <div class="ad-head-actions">
        <div class="ad-viewtoggle">
          <button class="ad-vt ad-vt-list" data-view="list" type="button">List</button>
          <button class="ad-vt ad-vt-graph" data-view="graph" type="button">Graph</button>
        </div>
        <button class="ad-btn ad-btn--primary ad-new" type="button">+ New</button>
      </div>
    </header>
    <ul class="ad-list"></ul>
    <div class="ad-graph-pane">
      <svg class="ad-graph" viewBox="0 0 760 200" width="100%">
        <g class="ad-graph-edges"></g>
        <g class="ad-graph-nodes"></g>
      </svg>
    </div>
    <section class="ad-tool-pane">
      <header class="ad-tool-head">
        <div class="ad-head-titles">
          <h1 class="ad-title ad-tool-title"></h1>
          <span class="ad-head-count ad-tool-sub"></span>
        </div>
      </header>
      <div class="ad-tool-body">
        <div class="ad-brand-grid"></div>
        <div class="ad-palette-grid"></div>
        <div class="ad-motion-grid"></div>
        <p class="ad-tool-empty ad-rel-hint"></p>
      </div>
    </section>
  </main>
  <aside class="ad-preview">
    <div class="ad-preview-bar"><span class="ad-preview-dot"></span>Live preview &middot; no reload</div>
    <iframe class="ad-preview-frame" src="/" title="Live preview"></iframe>
  </aside>
  <section class="ad-medialib">
    <div class="ad-medialib-backdrop" data-medialib-backdrop></div>
    <div class="ad-medialib-dialog">
      <header class="ad-medialib-head">
        <h2 class="ad-medialib-title">Media library</h2>
        <span class="ad-medialib-count"></span>
        <input class="ad-input ad-medialib-search" type="search" placeholder="Search assets &hellip;">
        <button class="ad-btn ad-medialib-upload" type="button">Upload</button>
        <button class="ad-medialib-x" type="button">&times;</button>
      </header>
      <div class="ad-medialib-grid"></div>
    </div>
  </section>
  <section class="ad-drawer">
    <header class="ad-drawer-head">
      <h2 class="ad-drawer-title"></h2>
      <span class="ad-drawer-kind"></span>
      <button class="ad-drawer-x" type="button">&times;</button>
    </header>
    <div class="ad-form"></div>
    <footer class="ad-drawer-foot">
      <button class="ad-link-danger ad-del" type="button">Delete</button>
      <button class="ad-btn ad-btn--primary ad-done" type="button">Done</button>
    </footer>
  </section>
</div>
