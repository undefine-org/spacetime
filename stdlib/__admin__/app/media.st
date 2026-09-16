// =============================================================================
// Media library — declarative asset picker modal (FEAT-092 W2).
//
// A @portal modal over /__spacetime/dev/assets.json. The drawer's media widget
// opens it ($mediaOpen) carrying the field path being edited ($mediaPath);
// picking an asset persists the path through the standard adEditField arc and
// closes. Live search is a SIGNAL + DERIVE — the platform's reactive-filter
// pattern, zero imperative DOM:
//   $mediaQuery (input) → $assets derive re-filters → @each re-renders.
// =============================================================================

// The asset catalog. Fetched once at boot; refreshed after every upload by
// re-publishing through the same channel (ST.adUpload re-fetches).
@data fetch $assetDoc : "/__spacetime/dev/assets.json";

// Searchable view: a declarative @data query over the fetched catalog. The
// `where` pipe filters by path substring (case-insensitive `includes`), `take`
// caps the grid (server already caps at 2000), and `map` shapes each row to
// {path, name} — `| basename` strips the directory. The query reacts to the
// $mediaQuery search signal automatically (computed-source dep discovery), so a
// keystroke re-filters with zero imperative JS. (Was an arrow-fn derive — AP1.)
@data query $assets from $assetDoc.images {
  where: $.path | includes($mediaQuery);
  map: { path: $.path, name: $.path | basename };
  limit: 400;
}
@data derive $assetCount : $assets.length;
@data derive $assetCountLabel : $assetCount + (1 === $assetCount ? " asset" : " assets");

// One asset card: thumbnail + filename. data-asset carries the pick payload.
@template &ad-asset-card($a) {
  <button class="ad-medialib-item" data-asset="`$a.path`" type="button" title="`$a.path`">
    <span class="ad-medialib-thumb" data-thumb="`$a.path`"></span>
    <span class="ad-medialib-name">`$a.name`</span>
  </button>
}

// Fill the grid from the filtered view.
.ad-medialib-grid {
  @each($assets as $a) {
    &ad-asset-card($a);
  }
}

// Thumbnails: background-image URLs can't be expressed as a static attr value
// (style attribute would need URL-encoding the path). Reflect them after each
// render of the grid (the derive republish) — pure presentation glue.
.ad-medialib {
  @on $assets.change {
    $noop <- ST.adReflectThumbs();
  }
}

// Header count + the modal escape/open arc.
.ad-medialib-count { text <- $assetCountLabel; }
.ad-medialib { @portal(when: $mediaOpen); }

// Search box: live reactive filter.
.ad-medialib-search {
  @on &.input {
    $mediaQuery <- $.value;
  }
}

// Close affordances. The × button closes; a click on the BACKDROP closes too.
// The backdrop is a dedicated element behind the dialog, so a DELEGATED handler
// ([data-medialib-backdrop]) fires only for backdrop clicks — clicks inside the
// dialog (search, upload, cards) never match the selector, so they don't close.
// (A non-delegated `.ad-medialib @on click` would bind $ to the overlay itself,
// closing on every bubbled click — the backdrop-delegation avoids that.)
.ad-medialib-x { @on &.click { $mediaOpen <- false; } }
.ad-medialib {
  @on &.click [data-medialib-backdrop] {
    $mediaOpen <- false;
  }
}

// Picking an asset: persist into the field the drawer opened the library for,
// then close. adPickAsset routes through adEditField (the standard persist arc).
.ad-medialib-grid {
  @on &.click [data-asset] {
    $noop <- ST.adPickAsset($.closest('[data-asset]').dataset.asset, $activeIndex, window.SpacetimeLocal.mediaPath);
    $mediaOpen <- false;
  }
}

// Upload: delegate to the glue helper (hidden file input → multipart POST →
// the server returns the deduped path → persist + refresh the catalog).
.ad-medialib-upload {
  @on &.click {
    $noop <- ST.adUpload($activeIndex, window.SpacetimeLocal.mediaPath);
  }
}
