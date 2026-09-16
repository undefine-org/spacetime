// =============================================================================
// Admin tool panes — Theme · Components · Motion (FEAT-076 W4).
//
// The rail's tool buttons set $pane (theme | palette | motion); the list pane is
// shown when $pane === "list", the tool pane otherwise. Each tool view fetches
// its server contract (/__spacetime/dev/{theme,templates,motion}.json) and
// renders declaratively. The Theme editor is the flagship: it reads every CSS
// custom-property token the project declares and lets the writer retune real,
// exportable design tokens — a category opaque-runtime builders can't reach.
// =============================================================================

// Show the content pane for $pane === "list"; the tool pane otherwise. Within
// the content pane, $view (list | graph, W3) picks the row list vs the relation
// graph. Toggling display by reactive classes keyed on $pane/$view (FEAT-072).
.ad-list       { .hidden: $pane !== "list" || $view !== "list"; }
.ad-graph-pane { .shown: $pane === "list" && $view === "graph"; }
.ad-head       { .hidden: $pane !== "list"; }
.ad-tool-pane  { .shown: $pane !== "list"; }

// ── The three tool contracts (fetched once at boot) ──────────────────────────
// Brand is the SINGLETON design-token value (PLAN-034 Wave B): the project's
// `@data inline $brand Brand : {…}` delivered with widget-annotated schema +
// current values. The pane renders it with the SHARED form-renderer (the same
// ST.adFormFields → &fld-<widget> dynamic dispatch the drawer uses) and edits it
// via EditAst (the .st literal) — no CSS-custom-prop scan, no shadow schema (AP2).
@data fetch $brandDoc : "/__spacetime/dev/brand.json";
@data fetch $templateDoc : "/__spacetime/dev/templates.json";
@data fetch $motionDoc : "/__spacetime/dev/motion.json";

// Shape the brand singleton into form-field descriptors with the SAME shaper the
// drawer uses (schema × values → {tpl:'fld-<widget>', label, value, path}).
// Editable literals + read-only derived chips in ONE field list, rendered by a
// SINGLE @each (sequential @each blocks in one scope body don't both render —
// BUG-085 — so the form-renderer always concatenates into one list, as the
// motion pane does). adFormFields yields the editable token fields; adDerived
// Fields appends a `Derived` group header + read-only formula chips.
@data derive $brandFields : ST.adBrandPane(($brandDoc.brand||{}));
// Components + motions are read DIRECTLY from the server contracts — the server
// delivers the source-determined display fields (template `sig`, motion `label`),
// so the panes are a clean @each over fetched rows. No client shaping (AP2 gone).
@data derive $componentCards : ($templateDoc||{}).templates;
// Flatten the motion inventory into ONE field list the shared form-renderer can
// @each over: a group header per motion directive, then its widget-typed param
// fields (each carrying its motion's selector+kind as the EditAst address). One
// flat @each avoids the nested-@each-in-template gap (BUG-085).
@data derive $motionFields : ST.adMotionFields(($motionDoc||{}).motions);

// Pane header: reactive title + subtitle keyed on $pane (shaping helpers —
// presentation strings, and avoids the nested-ternary-in-parens grammar gap
// BUG-084). Recompute on $pane change in an @on $sig.change (full JS).
@data inline $toolTitle string: "";
@data inline $toolSub string: "";
.ad-tool-pane {
  @on $pane.change {
    $toolTitle <- ST.adToolTitle($pane);
    $toolSub <- ST.adToolSub($pane);
  }
}
.ad-tool-title { text <- $toolTitle; }
.ad-tool-sub   { text <- $toolSub; }

// ── Component/motion card templates (dynamic-dispatch by descriptor `tpl`) ────
// A component template card: name + its param signature (one line — nested
// per-param @each chips can't render in a template body yet, BUG-085).
@template &ad-palette-card($c) {
  <div class="ad-palette-card">
    <div class="ad-palette-name">`$c.name`</div>
    <code class="ad-palette-sig">`$c.sig`</code>
  </div>
}


// ── Fill the pane grids via dynamic dispatch ─────────────────────────────────
// Brand fields render through the SHARED form-renderer (&$f.tpl → &fld-<widget>,
// the same templates the drawer uses). A color field gets &fld-color, etc.
.ad-brand-grid {
  @each($brandFields as $f) {
    &$f.tpl($f);
  }
}
.ad-palette-grid {
  @each($componentCards as $c) {
    &ad-palette-card($c);
  }
}
.ad-motion-grid {
  @each($motionFields as $f) {
    &$f.tpl($f);
  }
}

// Show only the active pane's grid (keyed on $pane).
.ad-brand-grid   { .shown: $pane === "brand"; }
.ad-palette-grid { .shown: $pane === "palette"; }
.ad-motion-grid  { .shown: $pane === "motion"; }

// Rail selection state. ONE source of truth: the active rail item is whichever
// pane is open. Collection items carry --active only while a collection is open
// ($pane === "list"); the three tool buttons each light when their pane is open.
// Without the list-pane guard the collection nav would stay lit "underneath" an
// open tool pane (two items looking selected). Tool active is a pure reactive
// class keyed on $pane — no JS marker needed (the collection nav still uses the
// imperative adMarkActiveNav because its items are @each-generated at runtime).
.ad-nav--collections { .ad-nav--tools-open: $pane !== "list"; }
.ad-tool--brand   { .ad-nav-item--active: $pane === "brand"; }
.ad-tool--palette { .ad-nav-item--active: $pane === "palette"; }
.ad-tool--motion  { .ad-nav-item--active: $pane === "motion"; }

// Open the live-preview column only for token-editing panes (Brand) — the place
// where in-place recolor matters. List/collection editing keeps full width.
.ad-root { .ad-root--preview: $pane === "brand"; }

// Empty-state hint when the active pane has no rows (shaping helpers; full JS
// in the @on $sig.change avoids the nested-ternary grammar gap BUG-084).
// Recompute on $pane change AND when any contract fetch lands late (the @data
// fetches resolve after boot; opening a pane before its contract arrives would
// otherwise leave a stale "empty" hint over a grid that fills reactively — the
// same late-data race list.st guards with a second @on $sig.change). @on heads
// only track their declared signal, so one companion head per descriptor source.
@data inline $toolEmpty bool: false;
@data inline $toolEmptyText string: "";
.ad-tool-pane {
  @on $pane.change {
    $toolEmpty <- ST.adToolEmpty($pane, window.SpacetimeLocal.brandFields, window.SpacetimeLocal.componentCards, window.SpacetimeLocal.motionFields);
    $toolEmptyText <- ST.adToolEmptyText($pane);
  }
  @on $brandFields.change {
    $toolEmpty <- ST.adToolEmpty(window.SpacetimeLocal.pane, $brandFields, window.SpacetimeLocal.componentCards, window.SpacetimeLocal.motionFields);
  }
  @on $componentCards.change {
    $toolEmpty <- ST.adToolEmpty(window.SpacetimeLocal.pane, window.SpacetimeLocal.brandFields, $componentCards, window.SpacetimeLocal.motionFields);
  }
  @on $motionFields.change {
    $toolEmpty <- ST.adToolEmpty(window.SpacetimeLocal.pane, window.SpacetimeLocal.brandFields, window.SpacetimeLocal.componentCards, $motionFields);
  }
}
.ad-tool-empty { .shown: $toolEmpty; text <- $toolEmptyText; }

// Persist a brand-token edit to the .st SINGLETON via EditAst. The descriptor
// path (`.scarlet`) names the field; the singleton's name + file come from the
// fetched brand doc. Live-edits the typed literal, preserving sibling tokens.
// FEAT-105 token-preview split:
//   input  → IMMEDIATE preview push to the iframe (postMessage, no disk) +
//            debounced EditAst persist (adBrandInput's own 400ms timer).
//   change → EditAst persist (commit on blur / picker close).
// The preview push is the singleton binding name + the field leaf so the iframe
// recolors every $<binding>.<field> reference within one frame (BUG-091).
// A motion param field carries data-motion-sel + data-motion-kind (the EditAst
// address); a brand field carries neither. One dispatcher routes by that marker:
// motion fields persist via adMotionInput (scoped EditAst), brand fields via the
// FEAT-105 preview-push + adBrandInput (singleton EditAst). Both on input AND
// change so picker/blur commits land.
.ad-tool-pane {
  @on &.input [data-path] {
    $noop <- ST.adTokenPreview($, ($brandDoc.brand||{}).name);
    $noop <- ST.adBrandInput($, ($brandDoc.brand||{}).name, ($brandDoc.brand||{}).file);
    $noop <- ST.adMotionInput($, ($motionDoc||{}).motions);
    $noop <- ST.adRefreshDerived();
  }
  @on &.change [data-path] {
    $noop <- ST.adTokenPreview($, ($brandDoc.brand||{}).name);
    $noop <- ST.adBrandInput($, ($brandDoc.brand||{}).name, ($brandDoc.brand||{}).file);
    $noop <- ST.adMotionInput($, ($motionDoc||{}).motions);
  }
}
