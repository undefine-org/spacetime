/// Workbench card templates.
///
/// Templates are deliberately dumb: each receives one JSON object and renders its
/// facts. Behavior attaches in `actions.st`; list ownership lives in `lists.st`.

@template &mcp-env-card($env) {
  <article class="mcp-card mcp-env-card">
    <p class="mcp-card-label">discovered</p>
    <h3>`$env.title`</h3>
    <code>`$env.path`</code>
    <button class="mcp-open-env" data-env-path="`$env.path`">Open environment</button>
  </article>
}

@template &mcp-open-env-card($env) {
  <article class="mcp-card mcp-open-env-card">
    <p class="mcp-card-label">open</p>
    <h3>`$env.title`</h3>
    <code>`$env.id`</code>
    <small>`$env.root`</small>
    <!-- PLAN-049 W1.S3: close this environment (mcp_env_close). `data-env-id`
         carries the id; refused server-side while a frame from it is mounted. -->
    <button class="mcp-env-close" data-env-id="`$env.id`">Close</button>
  </article>
}

/// PLAN-049 W4.S8: a frame-strip chip — one composed region on the stage host.
/// Shows the frame name + origin; the × unmounts it from its region
/// (`mcp_stage_clear` carrying the region). Clicking the chip body re-selects
/// the stage frame (opens the inspector). `data-region` carries the region the
/// clear action targets.
@template &mcp-frame-chip($frame) {
  <span class="mcp-frame-chip" data-origin="`$frame.origin.kind`">
    <span class="mcp-frame-chip-name">`$frame.name`</span>
    <span class="mcp-frame-chip-region">`$frame.region`</span>
    <button class="mcp-frame-unmount" data-region="`$frame.region`" title="Unmount this frame">×</button>
  </span>
}

/// PLAN-051 B2: a NAVIGATOR row — one node of the composed guest's template
/// graph (the structure projection, $mcpStageStructure). `--mcp-depth` drives the
/// indent (depth * step, set inline); `data-node` carries the node id for the
/// B3 selection spine; `data-kind` styles template vs extern nodes. Clicking the
/// row selects that node (wired in env.st).
@template &mcp-structure-node($node) {
  <button class="mcp-nav-node" data-st-node="`$node.id`" data-kind="`$node.kind`" data-active="`$node.id == $selectedNode`" style="--mcp-depth: `$node.depth`">
    <span class="mcp-nav-node-glyph" aria-hidden="true"></span>
    <span class="mcp-nav-node-label">`$node.label`</span>
    <span class="mcp-nav-node-tpl">`$node.template`</span>
  </button>
}

/// PLAN-051 B3: the SELECTED NODE detail — the inspector view of the node the
/// navigator selected. Shows its identity (label/template/kind) + its param
/// schema as read-only rows (the introspection-generated inspector, READ tier;
/// per-param editable widgets are B4). `$node.params` is the per-template param
/// list the server flattened from the bundle.
/// NB: a template body cannot host @each (nested @each is a selector-scope
/// construct). So the node's per-param ROWS are rendered by a SEPARATE filtered
/// @each over a server-flattened selected-node param list (B4), NOT nested here.
/// This detail card shows the node's identity; `param_count` (from the
/// projection) gives the at-a-glance arity until B4's editable widgets land.
@template &mcp-node-detail($node) {
  <div class="mcp-node-card">
    <h3 class="mcp-node-name">`$node.label`</h3>
    <div class="mcp-node-meta">
      <span class="mcp-node-tpl-chip">`$node.template`</span>
      <span class="mcp-node-kind" data-kind="`$node.kind`">`$node.kind`</span>
    </div>
    <p class="mcp-node-arity">`$node.param_count` param(s) · depth `$node.depth`</p>
    <p class="mcp-drawer-label">bound values</p>
    <code class="mcp-node-bound">`$node.bound_summary`</code>
  </div>
}

/// PLAN-049 W2 (COLLAPSE): an env entry-point picker row. Clicking it fires
/// `st_tab_open` with the entry's env + path, registering it as an env-origin
/// function (a tab) — the "+ tab" gesture. `data-env`/`data-entry` carry the
/// payload the action reads.
@template &mcp-entry-card($entry) {
  <button class="mcp-entry-card" data-env="`$entry.env`" data-entry="`$entry.entry`">
    <span class="mcp-entry-plus" aria-hidden="true">+</span>
    <span class="mcp-entry-name">`$entry.title`</span>
  </button>
}

@template &mcp-tab-card($tab) {
  <article class="mcp-card mcp-tab-card" data-compile-ok="`$tab.compile_ok`">
    <p class="mcp-card-label">tab</p>
    <h3>`$tab.title`</h3>
    <code>`$tab.env`</code>
    <small>`$tab.source.path`</small>
    <!-- PLAN-049 W2 (COLLAPSE): a tab IS an env-origin function. Its card drives
         the SAME verbs as a gallery tile: compose onto the stage (mcp_compose
         carrying the tab's function_id) and delete the underlying function
         (mcp_fn_delete). CSS hides "Open on stage" for a non-composable
         (compile-failed) tab via data-compile-ok. -->
    <div class="mcp-card-actions">
      <button class="mcp-tab-stage" data-fn-name="`$tab.function_id`">Open on stage</button>
      <button class="mcp-tab-close" data-tab-id="`$tab.id`">Close</button>
    </div>
  </article>
}

@template &mcp-function-card($fn) {
  <article class="mcp-card mcp-function-card">
    <p class="mcp-card-label">function</p>
    <h3>`$fn.id` r`$fn.revision`</h3>
    <code>`$fn.name`</code>
    <span class="mcp-origin-chip" data-origin="`$fn.origin.kind`">`$fn.origin.label`</span>
    <small>`$fn.compile.html_len`B html · `$fn.compile.css_len`B css · `$fn.compile.js_len`B js</small>
    <!-- PLAN-049 W1.S3: delete this function (mcp_fn_delete). `data-fn-id`
         carries the id; refused server-side while it is still mounted. -->
    <button class="mcp-fn-delete" data-fn-id="`$fn.id`">Delete</button>
  </article>
}

@template &mcp-instance-card($inst) {
  <article class="mcp-card mcp-instance-card">
    <p class="mcp-card-label">instance</p>
    <h3>`$inst.title`</h3>
    <code>`$inst.id` → `$inst.function_id` r`$inst.function_revision`</code>
    <div class="mcp-card-actions">
      <a href="`$inst.url`" target="_blank" rel="noreferrer">Open preview</a>
      <!-- PLAN-049 W4.S9: focus this instance's function on the stage
           (mcp_compose w/ its function_id) — in-workbench focus, no escape to a
           new tab. -->
      <button class="mcp-instance-focus" data-fn-name="`$inst.function_id`">Focus on stage</button>
      <!-- PLAN-049 W1.S1: unmount this instance (mcp_unmount, server-audience).
           `data-instance-id` carries the id the action reads (targetAttr). -->
      <button class="mcp-unmount" data-instance-id="`$inst.id`">Unmount</button>
    </div>
  </article>
}

/// Comms event card (M5 / PLAN-046): one entry in the agent-comms feed. The
/// `data-audience` attribute (agent | server) drives the accent — agent
/// messages (kit answers, agent-sent actions) glow ember; server/navigation
/// events stay muted. Shows the action (from payload) + value when present, so
/// the feed reads as a conversation, not a raw log.
@template &mcp-event-card($event) {
  <article class="mcp-card mcp-event-card" data-audience="`$event.audience`">
    <p class="mcp-card-label">`$event.audience` · #`$event.seq`</p>
    <h3>`$event.payload.action`</h3>
    <code class="mcp-event-value">`$event.payload.value`</code>
    <small>`$event.type` · `$event.instance_id`</small>
  </article>
}

/// Inspector (FEAT-132): one row per template param across all functions. The
/// flat `$mcpContractParams` source (server-flattened) carries each param's
/// owning function + origin, so a single top-level `@each` renders the contract
/// WITHOUT a nested `@each` (unsupported inside a template body — it is only a
/// selector/slot form). name · kind chip · type · owning function + origin.
@template &mcp-param-card($p) {
  <article class="mcp-card mcp-param-card">
    <div class="mcp-param-row">
      <code class="mcp-param-name">`$p.name`</code>
      <span class="mcp-param-kind" data-kind="`$p.kind`">`$p.kind`</span>
      <code class="mcp-param-type">`$p.type`</code>
    </div>
    <small class="mcp-param-owner">`$p.function` · `$p.template`</small>
    <span class="mcp-origin-chip" data-origin="`$p.origin.kind`">`$p.origin.label`</span>
  </article>
}


/// Gallery card (M2 / PLAN-046): a clickable tile in the stage empty-state that
/// composes its function onto the stage. The behavior (`mcp_compose`) attaches
/// in actions.st via `.mcp-gallery-card`; `data-fn-name` carries the function id
/// the action reads (`targetAttr`). Compile-failed functions are still listed but
/// marked — the action server-side rejects an uncompilable guest with a clear
/// message, surfaced by the host bridge.
@template &mcp-gallery-card($fn) {
  <button class="mcp-gallery-card" data-fn-name="`$fn.id`">
    <span class="mcp-gallery-spark" aria-hidden="true"></span>
    <span class="mcp-gallery-name">`$fn.name`</span>
    <span class="mcp-gallery-meta">`$fn.id` r`$fn.revision`</span>
    <span class="mcp-origin-chip" data-origin="`$fn.origin.kind`">`$fn.origin.label`</span>
  </button>
}

/// M4 (PLAN-046): an EDITABLE param row for the selected-frame inspector. Unlike
/// the read-only `&mcp-param-card`, this carries a text input whose change fires
/// `mcp_param` (actions.st) to set the param override on the composed guest live.
/// `data-param` carries the param name the action reads (`targetAttr`); the input
/// value flows via `valueAttr="value"` — the host bridge reads the live value.
@template &mcp-param-edit($p) {
  <label class="mcp-param-edit" data-param="`$p.name`">
    <span class="mcp-param-edit-head">
      <code class="mcp-param-name">`$p.name`</code>
      <span class="mcp-param-kind" data-kind="`$p.kind`">`$p.kind`</span>
    </span>
    <input class="mcp-param-input" name="`$p.name`" data-param="`$p.name`" value="`$p.default`" placeholder="`$p.name`">
  </label>
}
