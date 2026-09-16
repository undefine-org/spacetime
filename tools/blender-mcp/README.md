# Headless Blender MCP

A single-process, headless, stdio MCP server for generating glTF assets via
real Blender geometry operations — driven by an agent (or a script), zero GUI.

Not the community [`ahujasid/blender-mcp`](https://github.com/ahujasid/blender-mcp)
shape (GUI addon + TCP socket bridge, requires a human to click "Connect to
MCP" in a live Blender window). This is built for headless, agent-driven asset
generation in a repo pipeline instead.

See `!tasks/plans/PLAN-063-headless-blender-mcp-single-process-stdi.org` for
the full design rationale, verified findings, and hardening notes.

## Setup (one-time)

```sh
# Installs the `mcp` Python SDK into Blender's own interpreter (this Blender
# build uses the SYSTEM Python, not a vendored one).
./tools/blender-mcp/setup.sh

# Or, to avoid touching system site-packages:
BLENDER_MCP_DEPS_DIR=/some/isolated/dir ./tools/blender-mcp/setup.sh
```

## Usage — manual driver (no Spell/MCP-client registration needed)

```sh
# List available tools
python3 tools/blender-mcp/drive.py list

# Call one tool
python3 tools/blender-mcp/drive.py call create_primitive \
  '{"shape": "box", "name": "MyBox", "size": 2.0}'

# Run a multi-step build script (ONE Blender process/scene, sequential —
# the right shape for "model this asset end to end")
python3 tools/blender-mcp/drive.py script path/to/build.json
```

If you installed deps into an isolated dir, export `BLENDER_MCP_DEPS_DIR`
before running `drive.py` too (bootstrap.py reads it to `sys.path.insert()`
the deps — Blender ignores `PYTHONPATH`).

A build script is a JSON array of `{"tool": "...", "args": {...}}` calls, run
in order. See `demos/3d/genesis/models/build.json` for a real example (the
medallion showcase asset).

## Usage — registered MCP server (future / spell.kdl)

```kdl
mcp {
    server "blender" type=stdio timeout=120000 {
        command "blender"
        args "--background" "--python" "tools/blender-mcp/bootstrap.py"
    }
}
```

Not yet wired up — deferred per current project preference for the manual
driver. Both paths run the exact same `bootstrap.py` server; only the client
differs.

## Tool surface

| Tool | What it does |
|---|---|
| `clear_scene` | Delete every object/mesh/material — call first for a clean build |
| `create_primitive` | box / uv_sphere / ico_sphere / cylinder / cone / torus / plane — mirrors `stdlib/3d`'s `@object` shape surface |
| `extrude_polygon` | Custom N-gon → extrude → optional bevel. Closes `stdlib/3d` gap: no `@object` shape does arbitrary/custom-vertex geometry |
| `boolean_op` | union / difference / intersect between two named objects — real CSG, structurally impossible for a declarative shape macro |
| `assign_material` | PBR material (color/roughness/metalness/emissive/transmission/ior) — param surface matches `stdlib/3d`'s `@object` macro 1:1 |
| `set_face_material` | Assign a 2nd+ material slot to a face subset (top/bottom/sides/all by normal direction) — closes the "one material per mesh" gap (e.g. glazed cap vs. matte rim on a tile) |
| `export_glb` | Export as `.glb`. **Draco is always force-disabled** — required by `stdlib/3d`'s `three-entry.ts` build (no `DRACOLoader`); a Draco-compressed `.glb` silently fails to load client-side |

## Verified properties (not assumptions — see PLAN-063 for the test scripts)

- **Concurrency-safe by construction**: `_dispatch()` is fully synchronous
  (no internal `await`), so Python's asyncio scheduler can't preempt it
  mid-call — concurrent MCP requests serialize automatically. **Do not**
  introduce an `await` inside `_dispatch()` without adding an explicit lock
  around scene mutations, or this guarantee silently breaks.
- **Clean error surface**: a failed call (e.g. referencing a deleted/
  nonexistent object) returns a structured `{"error": ..., "trace": ...}`
  via the normal MCP content channel — never crashes the server, never
  corrupts scene state for subsequent calls.
- **Process model**: cold-per-`drive.py`-invocation, warm-within-a-script.
  One Blender process per multi-step build (the common real workload);
  fresh process per separate invocation — zero state-leakage risk between
  unrelated builds, no warm-process-pool bugs. `clear_scene` is available
  for intentional resets mid-script if a longer script builds multiple
  unrelated assets from one warm process.

## Known gotchas (all found + fixed here — see bootstrap.py comments for the "why")

- **Blender's stdout is corrupted by its own prints** (startup banner,
  "Blender quit"). Fixed via an fd-hijack at the top of `bootstrap.py`
  (dup the real stdout fd before bpy writes to it, rebind fd 1 to
  `/dev/null`).
- **Blender ignores `PYTHONPATH`** — deps must be `sys.path.insert()`'d
  inside the script itself (see `_DEPS_DIR` handling in `bootstrap.py`).
- **`StdioServerParameters` doesn't inherit the parent shell's env by
  default** — `drive.py` passes `env=dict(os.environ)` explicitly, or
  `BLENDER_MCP_DEPS_DIR` silently never reaches the spawned Blender process.
- **A boolean-modifier `apply` can leave a stray empty material slot** on
  the target mesh (Blender pads a slot for the tool object even if it had
  none). `assign_material` re-targets any face still on an empty slot to
  the newly-appended material, so a plain "assign one material" call always
  actually paints the whole object — otherwise glTF export silently drops
  the material as "unused" with no diagnostic.
