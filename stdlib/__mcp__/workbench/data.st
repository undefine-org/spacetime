/// Workbench data contract.
///
/// The Rust `st_workbench` tool mounts `env.st` with a snapshot shaped as:
/// `{ environments: { discovered, open }, tabs, functions, instances, events }`.
/// `@mcp-input(refresh: 1000)` keeps these arrays synchronized with the mounted
/// instance's `input.json`; `st_await` action dispatch updates that JSON after
/// handling browser-originated `mcp-action` events.

@import "../index.st";

@data inline $mcpDiscoveredEnvironments : [];
@data inline $mcpOpenEnvironments : [];
@data inline $mcpTabs : [];
// PLAN-049 W2 (COLLAPSE): flattened `.st` entries across open envs (each row
// carries its env), feeding the open-env card's one-click entry picker.
@data inline $mcpEnvEntries : [];
@data inline $mcpFunctions : [];
@data inline $mcpGalleryFunctions : [];
@data inline $mcpContractParams : [];
@data inline $mcpStageMountName : "";
@data inline $mcpStageParams : [];
// PLAN-049 W4.S8: the frame strip — every region composed on the stage host
// (region + function identity), driving switch/unmount chips.
@data inline $mcpStageFrames : [];
// PLAN-051 B1: the stage STRUCTURE — the composed guest's template graph as a
// flat depth-tagged node list ({id,label,kind,depth,template,params}), feeding
// the builder NAVIGATOR (B2). Server-flattened from the guest bundle's body.refs.
@data inline $mcpStageStructure : [];
// PLAN-051 B4: FLAT per-node param rows ({node_id,param,value,type,span_start,
// span_end}) for the inspector's editable per-instance widgets. The inspector
// filters to the SELECTED node ($selectedNode); editing a row fires
// `mcp_node_param` carrying the invoke_span → a source patch on the G2 rail.
@data inline $mcpStageNodeParams : [];
// PLAN-064 B4a: per-ELEMENT binding edit rows. When an element node is selected,
// its bindings that resolve to entry-template params are editable here via the
// mcp_param value rail (provenance dispatch). Each row: {node_id, param, value,
// type, default, rail, region}. The inspector filters to $selectedNode.
@data inline $mcpStageNodeBindings : [];
@data inline $mcpStageSource : "";
@data inline $mcpInstances : [];
@data inline $mcpEvents : [];

@mcp-input(refresh: 1000);
