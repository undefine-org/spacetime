/// Workbench action wiring.
///
/// Buttons do not call bespoke endpoints. They emit typed `mcp-action` events
/// through the one host bridge. These are SERVER-audience actions (PLAN-045):
/// the signal sink resolves them SYNCHRONOUSLY (open env / open tab / refresh),
/// updates `input.json`, and `data.st` polls that file and repaints — the agent
/// is never woken for navigation. Only agent-audience actions (the interaction
/// kit's kit-* answers) are delivered to the agent via `st_await`.

@import "../index.st";

.mcp-refresh {
  @mcp-action(action: "mcp_refresh", target: "workbench")
}

.mcp-open-env {
  @mcp-action(action: "st_env_open", targetAttr: "data-env-path")
}

// M2 living gallery: clicking a gallery tile composes its function onto the
// stage. Server-audience `mcp_compose` resolves against the originating host
// (event.instance_id) + the `data-fn-name` it reads; input.json repaints and the
// stage @view mounts the guest. `region` defaults to "stage" server-side.
.mcp-gallery-card {
  @mcp-action(action: "mcp_compose", targetAttr: "data-fn-name")
}

// M4 editable params: typing in a selected-frame param input fires `mcp_param`
// on `change` (commit on blur/enter), carrying the param name (`data-param`) and
// the input's live value. The server updates the guest's region param override
// and the bundle poll re-seeds it onto the guest — live edit, no agent wake.
// Selects via data-param-kind="binding" to distinguish from node params (span-based).
[data-param-kind="binding"][data-param][data-widget] {
  @mcp-action(action: "mcp_param", targetAttr: "data-param", on: "change")
}
// Composite widgets (range, color) commit on `input` event via their partners.
[data-param-kind="binding"][data-range-twin] {
  @mcp-action(action: "mcp_param", targetAttr: "data-param", on: "input")
}
[data-param-kind="binding"].st-fld-color-text {
  @mcp-action(action: "mcp_param", targetAttr: "data-param", on: "input")
}
// Legacy class-based selector for backward compatibility.
.mcp-param-input {
  @mcp-action(action: "mcp_param", targetAttr: "data-param", on: "change")
}

// PLAN-051 B4: editing a SELECTED-node param input fires `mcp_node_param` on
// `change`, carrying the param name (`data-param`) + the live value + the node's
// invoke_span (forwarded via the `data-st-payload-span-start/-end` rail). The
// server patches the guest SOURCE at that span (the G2 WRITE) and hot-swaps the
// guest — per-instance two-level editing, no agent wake.
// Selects via data-param-kind="node-param" to distinguish from bindings (value-based).
[data-param-kind="node-param"][data-param][data-widget] {
  @mcp-action(action: "mcp_node_param", targetAttr: "data-param", on: "change")
}
// Composite widgets (range, color) commit on `input` event via their partners.
[data-param-kind="node-param"][data-range-twin] {
  @mcp-action(action: "mcp_node_param", targetAttr: "data-param", on: "input")
}
[data-param-kind="node-param"].st-fld-color-text {
  @mcp-action(action: "mcp_node_param", targetAttr: "data-param", on: "input")
}
// Legacy class-based selector for backward compatibility.
.mcp-node-param-input {
  @mcp-action(action: "mcp_node_param", targetAttr: "data-param", on: "change")
}

// PLAN-064 B4a: editing a SELECTED-element BINDING input fires `mcp_param` on
// `change`, carrying the param name (data-param) + the input's live value. The
// server sets the region param value override (provenance dispatch — the hole's
// value lives upstream in the param, not at the element span); the bundle poll
// re-seeds the guest. This is the SAME value rail the frame-level param editor
// uses, now reachable per-element by selecting a canvas node.
.mcp-node-binding-input {
  @mcp-action(action: "mcp_param", targetAttr: "data-param", on: "change")
}

// PLAN-049 W1.S1: clicking an instance card's Unmount button fires `mcp_unmount`
// (server-audience), carrying the instance id from `data-instance-id`. The sink
// removes the instance + clears any host region it occupied; the snapshot poll
// repaints the instances list (and the stage if it was the composed guest).
.mcp-unmount {
  @mcp-action(action: "mcp_unmount", targetAttr: "data-instance-id")
}

// PLAN-049 W4.S9: focus an instance's function on the stage (mcp_compose w/ the
// fn id) — the same stage-compose path the gallery + tab cards use.
.mcp-instance-focus {
  @mcp-action(action: "mcp_compose", targetAttr: "data-fn-name")
}

// PLAN-049 W1.S2: clicking the clear-stage control fires `mcp_stage_clear`
// (server-audience). The sink clears the host's "stage" region (the default),
// emptying it; the snapshot poll repaints the empty-state.
.mcp-stage-clear {
  @mcp-action(action: "mcp_stage_clear", target: "stage")
}

// PLAN-049 W4.S8: a frame-strip chip's × unmounts that frame from its region —
// `mcp_stage_clear` carrying the region (data-region -> payload.target, read as
// the region by the resolver). Generalizes "clear stage" to per-frame.
.mcp-frame-unmount {
  @mcp-action(action: "mcp_stage_clear", targetAttr: "data-region")
}

// PLAN-049 W1.S3: closing an environment / deleting a function. Both server-
// audience; refused at the sink while a dependent is still mounted (the host
// bridge surfaces the refusal message). `data-env-id` / `data-fn-id` carry the
// target the action reads.
.mcp-env-close {
  @mcp-action(action: "mcp_env_close", targetAttr: "data-env-id")
}
.mcp-fn-delete {
  @mcp-action(action: "mcp_fn_delete", targetAttr: "data-fn-id")
}

// PLAN-049 W2 (COLLAPSE): a tab card drives the SAME stage-compose path as a
// gallery tile — "Open on stage" fires `mcp_compose` with the tab's function_id
// (data-fn-name), switching the stage to that tab. "Close" fires `mcp_tab_close`
// (data-tab-id), dropping the tab + its env-origin function.
.mcp-tab-stage {
  @mcp-action(action: "mcp_compose", targetAttr: "data-fn-name")
}
.mcp-tab-close {
  @mcp-action(action: "mcp_tab_close", targetAttr: "data-tab-id")
}

// PLAN-049 W2 (COLLAPSE): clicking an env entry row fires `st_tab_open`, carrying
// the env (data-env -> payload.target) and entry path (data-entry -> payload
// .value). The sink registers it as an env-origin function (a tab) + records the
// tab — the "+ tab" gesture, no agent wake.
.mcp-entry-card {
  @mcp-action(action: "st_tab_open", targetAttr: "data-env", valueAttr: "data-entry")
}

// PLAN-049 W3: the "+" creation surface. Clicking "Create" fires `mcp_create`
// via `@mcp-submit`, which COLLECTS the region's named controls (create_name +
// create_source) into payload.values — the same collect-submit the kit forms
// use. The sink registers an Agent-origin function + composes it onto the stage.
//
// Reviewer W3 P2: the binding is SCOPED to the chrome empty-state
// (`.mcp-stage-empty` is the host's own zero-state, NEVER inside a composed
// guest — guests mount as SIBLINGS of it under `.mcp-stage`). So a user-authored
// guest frame that happens to render a `.mcp-create-submit` button cannot match
// this selector and drive the server create path. Only the host chrome's own
// Create button fires it.
.mcp-stage-empty .mcp-create-submit {
  @mcp-submit(action: "mcp_create")
}

// M6 inline source edit: editing the selected guest's source textarea + blur
// fires `mcp_source_edit` (the textarea's live `.value` is the new source). The
// server recompiles + hot-swaps; a compile error leaves the guest unchanged.
.mcp-source-input {
  @mcp-action(action: "mcp_source_edit", on: "change")
}

// FEAT-127: @mcp-host is selector-scoped — one bridge per region root. The
// workbench instance page tags its body root with `data-st-instance` (live.rs),
// so this fires on the workbench's own region root.
[data-st-instance] {
  @mcp-host
}
