/// Workbench list bindings.
///
/// Each list owns exactly one source array and one card template. This file is
/// the full data-flow map from `data.st` sources to rendered panels.

.mcp-env-grid {
  @each($mcpDiscoveredEnvironments as $env) {
    &mcp-env-card($env);
  }
}

.mcp-open-envs {
  @each($mcpOpenEnvironments as $env) {
    &mcp-open-env-card($env);
  }
}

// BUG-122: frame-first active-environment view in the stage empty-state. The
// SAME open-env + tab sources, rendered on the dominant surface (not just the
// nav drawer) so opening an environment is VISIBLE where the user is looking.
// Reuses the existing card templates; CSS `:has` reveals the section only when
// an env is open (styles.st).
.mcp-active-env-cards {
  @each($mcpOpenEnvironments as $env) {
    &mcp-open-env-card($env);
  }
}

.mcp-active-env-tabs {
  @each($mcpTabs as $tab) {
    &mcp-tab-card($tab);
  }
}

.mcp-tab-list {
  @each($mcpTabs as $tab) {
    &mcp-tab-card($tab);
  }
}

// PLAN-049 W2 (COLLAPSE): the env entry-point picker — one clickable "+ tab" row
// per discoverable `.st` across open envs (server-flattened $mcpEnvEntries, each
// row carrying its env). Clicking fires st_tab_open (actions.st).
.mcp-entry-list {
  @each($mcpEnvEntries as $entry) {
    &mcp-entry-card($entry);
  }
}

.mcp-function-list {
  @each($mcpFunctions as $fn) {
    &mcp-function-card($fn);
  }
}

// M2 living gallery (PLAN-046): the empty-state's clickable tiles. Binds the
// server-filtered `$mcpGalleryFunctions` (composable guests only — no host page,
// no uncompilable fns). Clicking a tile fires `mcp_compose` (actions.st).
.mcp-gallery {
  @each($mcpGalleryFunctions as $fn) {
    &mcp-gallery-card($fn);
  }
}

// Inspector (FEAT-132): the server-flattened param rows, one card per param.
.mcp-contract-list {
  @each($mcpContractParams as $p) {
    &mcp-param-card($p);
  }
}

// M3' selected frame: the SELECTED stage guest's params. M4 makes each row an
// EDITABLE control (`&mcp-param-edit`) — typing fires `mcp_param` (actions.st),
// updating the guest live.
// PLAN-065 W_session/W_hygiene: the frame-level editable params render from the
// CLIENT-owned $activeFrameParams (projected from $mcpStageParams on frame
// selection), NOT the poll array — so a focused frame-param input survives a poll
// tick like the node-level editors (BUG-151).
.mcp-selected-params {
  @each($activeFrameParams as $p) {
    &mcp-param-edit($p);
  }
}

.mcp-instance-list {
  @each($mcpInstances as $inst) {
    &mcp-instance-card($inst);
  }
}

// PLAN-049 W4.S8: the frame strip — one chip per region composed on the stage
// host. Generalizes the single-stage model toward multi-frame (deferred PLAN)
// without a schema change.
.mcp-frame-strip {
  @each($mcpStageFrames as $frame) {
    &mcp-frame-chip($frame);
  }
}

// PLAN-051 B2: the NAVIGATOR — the composed guest's template graph as an
// indented tree. One @each over the server's flat depth-tagged structure
// projection ($mcpStageStructure); each node renders &mcp-structure-node, whose
// --mcp-depth drives the indent. Empty when the stage has no guest.
.mcp-nav-tree {
  @each($mcpStageStructure as $node) {
    &mcp-structure-node($node);
  }
}

// PLAN-051 B3: the SELECTED NODE detail — the inspector half of the selection
// spine. A FILTERED @each picks the one structure node whose id equals
// $selectedNode (the navigator click target), rendering its detail. The `when`
// clause makes selection a pure reactive filter (no per-node plumbing); the
// container collapses to nothing when no node matches.
.mcp-node-detail {
  @each($mcpStageStructure as $node, when $node.id == $selectedNode) {
    &mcp-node-detail($node);
  }
}

// PLAN-051 B4: the SELECTED node's EDITABLE param rows. A FILTERED @each over the
// PLAN-065 W_session: the SELECTED node's EDITABLE param rows, rendered from the
// CLIENT-owned $activeParams signal — NOT a filtered @each over the poll array.
// $activeParams is a snapshot COPY projected out of $mcpStageNodeParams at
// SELECTION time (env.st selection handlers), so it changes ONLY on selection,
// never on the 1s server poll. @each watches ONLY the signal it iterates
// (ST.watchScoped on the source name; template.st:428), so the poll can no longer
// re-render — and thus can no longer DESTROY — a focused editing input mid-edit
// (BUG-151). Editing fires `mcp_node_param` (actions.st) -> a source patch at the
// node's invoke_span (the G2 WRITE rail).
.mcp-node-params {
  @each($activeParams as $p) {
    &$p.tpl($p);
  }
}

// PLAN-064 B4a + PLAN-065 W_session: the SELECTED element's EDITABLE BINDING rows,
// rendered from the CLIENT-owned $activeBindings snapshot (projected out of
// $mcpStageNodeBindings at selection time, env.st) — same ownership rule as
// $activeParams: the poll cannot re-render this list, so a focused binding input
// survives a poll tick (BUG-151). Editing fires `mcp_param` (actions.st) -> the
// region param value override rail (provenance dispatch, no source span).
.mcp-node-bindings {
  @each($activeBindings as $b) {
    &$b.tpl($b);
  }
}

.mcp-event-list {
  @each($mcpEvents as $event) {
    &mcp-event-card($event);
  }
}
