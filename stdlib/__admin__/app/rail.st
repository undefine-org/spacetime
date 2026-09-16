// =============================================================================
// Admin rail behavior — dynamic collection buttons + selection (FEAT-076 W1).
//
// The rail SKELETON (brand, section labels, tools nav, foot) is declared as
// file-scope markup in app.st. This file supplies only the DYNAMIC parts:
//   - a @template for one collection nav button
//   - an @each that fills the .ad-nav--collections mount point from $collections
//   - @on click delegation that sets $activeFile / $pane
// =============================================================================

// One collection nav button: label + entry count. data-file carries the key.
@template &ad-nav-item($col) {
  <button class="ad-nav-item" data-file="`$col.file`" data-type="`$col.type`" type="button">
    <span class="ad-nav-label">`$col.name`</span>
    <span class="ad-nav-count">`$col.count`</span>
  </button>
}

// Fill the collections nav from the server-provided array.
.ad-nav--collections {
  @each($collections as $col) {
    &ad-nav-item($col);
  }
}

// Auto-select the first collection once the list arrives, so the center isn't
// blank on load. Runs when $collections publishes; no-op once something's active.
.ad-rail {
  @on $collections.change(immediate: true) {
    $activeFile <- ST.adAutoSelect($collections, window.SpacetimeLocal.activeFile);
    $activeName <- ST.adAutoSelectField($collections, window.SpacetimeLocal.activeFile, 'name');
    $activeType <- ST.adAutoSelectField($collections, window.SpacetimeLocal.activeFile, 'type');
    $activeHasRelations <- ST.adTypeHasRelations(window.SpacetimeLocal.types, ST.adAutoSelectField($collections, window.SpacetimeLocal.activeFile, 'type'));
    $noop <- ST.adMarkActiveNav(window.SpacetimeLocal.activeFile);
  }
  // types.json may resolve AFTER the first auto-select (cold open): recompute the
  // relation flag so the Graph toggle appears once schemas land.
  @on $types.change {
    $activeHasRelations <- ST.adTypeHasRelations($types, window.SpacetimeLocal.activeType);
  }
}

// Selection delegation across the whole rail.
.ad-rail {
  // Selecting a collection: set the active file, reset the entry selection. Also
  // capture the human label/type so the center header + label resolution have
  // them without a brittle lookup derive (BUG-077 limits derive exprs), and mark
  // the clicked button active for immediate visual feedback.
  @on &.click [data-file] {
    $activeFile <- $.closest('[data-file]').dataset.file;
    $activeName <- $.closest('[data-file]').querySelector('.ad-nav-label').textContent;
    $activeType <- $.closest('[data-file]').dataset.type;
    $activeHasRelations <- ST.adTypeHasRelations(window.SpacetimeLocal.types, $.closest('[data-file]').dataset.type);
    $activeIndex <- -1;
    $drawerOpen <- false;
    $pane <- "list";
    $view <- "list";
    $noop <- ST.adMarkActiveNav($.closest('[data-file]').dataset.file);
  }
  // Selecting a tool view (close any open drawer so it can't hang over the pane).
  @on &.click [data-pane] {
    $drawerOpen <- false;
    $pane <- $.closest('[data-pane]').dataset.pane;
  }
}
