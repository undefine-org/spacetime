// =============================================================================
// Collection list pane — the center column (FEAT-076 W1).
//
// Renders the ACTIVE collection's entries. The admin is a meta-renderer, so the
// rows come from `@data source $entries from $activeFile` — a reactive fetch
// whose URL is the rail's selected file. Selecting a collection (rail sets
// $activeFile) re-fetches and re-renders the list. Clicking a row opens the
// entry drawer (W2) by setting $activeIndex + $drawerOpen.
// =============================================================================

// The active collection's rows (re-fetched when $activeFile changes).
@data source $entries from $activeFile;

// Display rows: each entry enriched with a resolved `_label` (the type's
// display.title role, falling back to title/name/label/heading/id). The admin
// is a meta-renderer, so the title FIELD varies per type — we resolve it here
// rather than hardcode a field in the row template. @on $sig.change bodies allow
// full JS (map/arrow), so this stays declarative without tripping the derive grammar.
// $rows = $entries enriched with display labels. Recompute whenever the entries
// change (covers collection switch — reactive-source publishes a new array) OR
// the type schema lands late (types.json resolves after a fast first selection).
// immediate:true seeds $rows on mount in case $entries is already populated.
@data inline $rows : [];
.ad-center {
  @on $entries.change(immediate: true) {
    $rows <- ST.adLabelRows($entries, window.SpacetimeLocal.types, window.SpacetimeLocal.activeType);
  }
  @on $types.change {
    $rows <- ST.adLabelRows(window.SpacetimeLocal.entries, $types, window.SpacetimeLocal.activeType);
  }
}

// A label for an entry: prefer a human field (title/name/label/heading), else
// fall back to the id, else a positional "Entry N". Resolved per row in the
// template via a small expression on the item.
// The each-with-templates primitive stamps each rendered element with
// dataset.index automatically, so the row needs only the item; the click
// handler reads data-index off the clicked row.
@template &ad-row($entry) {
  <li class="ad-row">
    <span class="ad-row-reorder">
      <button class="ad-row-up" type="button" title="Move up">↑</button>
      <button class="ad-row-down" type="button" title="Move down">↓</button>
    </span>
    <button class="ad-row-open" type="button">
      <span class="ad-row-thumb"></span>
      <span class="ad-row-label">`$entry._label`</span>
    </button>
  </li>
}

// The center skeleton (header + .ad-list mount) lives in app.st's shell markup.
// This file supplies the dynamic behavior: row rendering, header reflectors,
// row-click → drawer.

// Render the rows from $rows (entries enriched with display labels).
.ad-list {
  @each($rows as $entry) {
    &ad-row($entry);
  }
}

// Header title + count reflect the active collection (file-scope reactive text).
// $activeFile names the file; the title shows the collection name. (A derived
// signal gives the human name; see app.st $activeName.)
.ad-title--active { text <- $activeName; }
.ad-head-count    { text <- $entryCountLabel; }

// Open the entry editor drawer when the row's LABEL/thumb is clicked. Delegating
// on .ad-row-open (not .ad-row) means clicks on the ↑/↓ reorder buttons — which
// live in a sibling .ad-row-reorder span — don't match this handler at all, so
// reordering never pops the drawer.
.ad-list {
  @on &.click .ad-row-open {
    $activeIndex <- parseInt($.closest('.ad-row').dataset.index, 10);
    $drawerOpen <- true;
  }
}

// Add a new blank entry (built from the active type schema) + open its drawer.
.ad-new {
  @on &.click {
    $noop <- ST.adInsert(ST.adBlankEntry(ST.adActiveSchema(window.SpacetimeLocal.types, window.SpacetimeLocal.activeType)));
    $activeIndex <- window.SpacetimeLocal.entries.length - 1;
    $drawerOpen <- true;
  }
}

// Reorder: ↑/↓ affordances on each row move the entry up/down (declarative,
// avoids drag-and-drop wiring; persists via the same optimistic path).
.ad-list {
  @on &.click .ad-row-up {
    $noop <- ST.adReorderUp(parseInt($.closest('.ad-row').dataset.index, 10));
  }
  @on &.click .ad-row-down {
    $noop <- ST.adReorderDown(parseInt($.closest('.ad-row').dataset.index, 10));
  }
}
