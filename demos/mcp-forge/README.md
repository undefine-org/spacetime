# demos/mcp-forge — Forge Deck: agent-driven MCP + 3D + Blender pressure test

A concrete UI that talks **directly to the driving agent** through the
`spacetime/st_*` MCP host tools, while the agent drives real geometry through
the headless Blender MCP (`tools/blender-mcp/`) and hot-swaps the result into
a live `stdlib/3d` (`@stage`) viewport — no page reload, no bespoke Rust route,
no hand-authored page JavaScript anywhere (repo AGENTS.md law).

See `!tasks/features/FEAT-151-forge-deck-agent-driven-mcp3d-control-su.org`
for the full design + build-wave record.

## Status: intentionally blocked, not silently patched

The rail/log (`.forge-steps` / `.forge-log`, both a plain
`@each($mcpInput.steps as $step)`) render nothing today. That IS the correct,
most idiomatic syntax for this data shape — it is blocked by two runtime bugs
found and precisely isolated by this build, not a mistake in this demo:

- **BUG-155** — `@each` with a dotted-path source (`$sig.field`) never
  resolves (`each-with-templates` calls `ST.resolve` instead of the
  already-existing `ST.resolvePath`).
- **BUG-156** — a `@data derive` off an `@mcp-input` signal only works by
  accident, contingent on unrelated markup elsewhere on the page.

Do not "fix" this file with a `@data derive` workaround — one was tried,
found to ALSO be blocked (BUG-156), and reverted. Once BUG-155 lands, this
file renders correctly with **zero further changes**.

Also surfaced building this: **FEAT-152** (stdlib/3d has no click/hover
pick affordance) and **FEAT-153** (stdlib/3d params are write-once at
mount — `@gltf(src: $sig)` never hot-reloads) — both filed with elegant
target designs that reuse existing mechanisms (`@on`, the `ctx` engine
boundary) rather than adding new ones.

## Shape

```
demos/mcp-forge/
  mission-control.st   — HOST shell. Agent-control page (agent_control:true).
                          Left rail: forge directive buttons (@mcp-action) +
                          a scrolling agent log. Composes forge-preview.st
                          into its "stage" region via st_mount{host,region}.
  forge-preview.st     — GUEST. @stage + @env + @light + @gltf + @orbit,
                          reading $mcpInput.glbPath / $mcpInput.stepLabel.
  models/
    build-step-*.json  — one Blender MCP build script per forge stage
                          (blank disc -> cut facet -> raise boss -> bronze
                          finish -> export), each producing a fresh .glb.
    *.glb              — exported assets, one per stage (kept small/uncompressed
                          — export_glb always force-disables Draco, matching
                          stdlib/3d's loader).
```

## The loop (verified live, not simulated)

1. `st_fn_put` registers both functions.
2. `st_mount` mounts `mission-control` standalone with `agent_control: true` —
   its otherwise-unknown `@mcp-action` clicks route to the agent via
   `st_await` instead of being rejected at the sink (PLAN-045).
3. `st_mount { host: <mission-control instance>, region: "stage" }` composes
   `forge-preview` into the host's stage region — one DOM, one runtime.
4. The human clicks a directive button (or the agent proceeds on its own for
   the automated pressure-test run). `st_await` on the mission-control
   instance returns `{action: "forge-directive", value: "<step-id>"}`.
5. The agent runs the matching Blender build script via
   `python3 tools/blender-mcp/drive.py script demos/mcp-forge/models/build-step-N.json`,
   which exports `models/step-N.glb`.
6. The agent re-mounts `forge-preview` into the SAME host region with a
   `glbPath` input pointing at the fresh `.glb` (cache-busted) — the stage
   hot-swaps to the new geometry/material without touching the host DOM.
7. Repeat for every stage; the agent appends each step to the host's log via
   a fresh `st_mount` input update (see mission-control.st's `$mcpInput.log`).

## Run it yourself

```sh
cargo run -- serve demos/mcp-forge/
# static preview only — the LIVE agent loop requires driving it through the
# spacetime/st_* MCP tools from an agent session, not a browser alone.
```
