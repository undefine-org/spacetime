// =============================================================================
// Relation graph — declarative SVG view of the active collection (FEAT-092 W3).
//
// The first DATA-DRIVEN VECTOR rendering in the platform: a bipartite graph of
// source entries (left) linked to their relation TARGETS (right), drawn as
// real <svg> markup via @each over a layout descriptor. No imperative DOM, no
// string-concatenated SVG — the W1 SVG-namespace fix (BUG-082) makes
// @each-built <circle>/<path>/<text> paint. ST.adGraphLayout is pure geometry
// (the accepted glue seam, like adFormFields); everything visible is markup.
//
// Edges read $graph.edges (cubic-bezier `d` strings); nodes read $graph.nodes
// (positioned {x,y,tx,ty,label,index,side}). Clicking a SOURCE node opens that
// entry's drawer (index >= 0); target nodes are display-only (index -1).
// =============================================================================

// The graph geometry, recomputed whenever the rows or schemas change. @on
// $sig.change bodies allow full JS, so the layout call stays out of the derive grammar.
@data inline $graph : null;
.ad-graph-pane {
  @on $entries.change(immediate: true) {
    $graph <- ST.adGraphLayout($entries, window.SpacetimeLocal.types, window.SpacetimeLocal.activeType);
  }
  @on $view.change {
    $graph <- ST.adGraphLayout(window.SpacetimeLocal.entries, window.SpacetimeLocal.types, window.SpacetimeLocal.activeType);
  }
}

// Derive the node/edge arrays for the @each loops (always-array, never null).
@data derive $graphNodes : ($graph && $graph.nodes) || [];
@data derive $graphEdges : ($graph && $graph.edges) || [];
@data derive $graphH : ($graph && $graph.h) || 200;
@data derive $graphViewBox : "0 0 760 " + $graphH;

// One edge: a cubic-bezier path under the nodes.
@template &ad-gedge($e) {
  <path class="ad-gedge" d="`$e.d`"></path>
}
// One node: a dot + its label, click-targetable (data-index carries the entry).
@template &ad-gnode($n) {
  <g class="ad-gnode" data-side="`$n.side`" data-index="`$n.index`">
    <circle class="ad-gdot" cx="`$n.x`" cy="`$n.y`" r="6"></circle>
    <text class="ad-glabel" x="`$n.tx`" y="`$n.ty`" text-anchor="end">`$n.label`</text>
  </g>
}

// The <svg> skeleton (with <g> edge/node mounts) lives in app.st's shell markup
// so the STATIC HTML path namespaces it as real SVG (markup nested in a selector
// scope is dropped; file-scope markup is not — FEAT-076 learning). These @each
// blocks fill the <g> mounts; their rows parse in SVG context (the <g> is
// SVG-namespaced), so @each-built <path>/<circle>/<text> paint (BUG-082 W1).
.ad-graph-edges {
  @each($graphEdges as $e) {
    &ad-gedge($e);
  }
}
.ad-graph-nodes {
  @each($graphNodes as $n) {
    &ad-gnode($n);
  }
}

// Keep the SVG viewBox height in sync with the computed layout height.
.ad-graph { viewBox <- $graphViewBox; }

// Clicking a SOURCE node opens that entry's drawer. Target nodes carry
// data-index="-1" (display-only); the guard no-ops on them.
.ad-graph-pane {
  @on &.click [data-index] {
    $activeIndex <- ST.adGraphNodeIndex($.closest('[data-index]').dataset.index);
    $drawerOpen <- ST.adGraphNodeOpens($.closest('[data-index]').dataset.index);
  }
}

// ── The List / Graph view toggle (header) ────────────────────────────────────
// data-view buttons set $view; the active button reflects $view.
.ad-viewtoggle {
  @on &.click [data-view] {
    $view <- $.closest('[data-view]').dataset.view;
  }
}
.ad-vt-list  { .ad-vt--on: $view === "list"; }
.ad-vt-graph { .ad-vt--on: $view === "graph"; }

// Hide the whole toggle for collections with no relations (nothing to graph).
.ad-viewtoggle { .hidden: !$activeHasRelations; }
