"""
Headless Blender MCP server — single process, stdio transport, no GUI, no TCP
socket bridge. Runs entirely inside Blender's own `--background` Python
interpreter (this build uses the SYSTEM python, not a vendored one — verified
2026-07-04).

Launch:
    blender --background --python tools/blender-mcp/bootstrap.py

Registered in spell.kdl as:
    mcp {
        server "blender" type=stdio timeout=120000 {
            command "blender"
            args "--background" "--python" "tools/blender-mcp/bootstrap.py"
        }
    }

──────────────────────────────────────────────────────────────────────────
WHY THIS FILE LOOKS THE WAY IT DOES (two verified, non-obvious fixes)
──────────────────────────────────────────────────────────────────────────
1. Blender writes its startup banner + "Blender quit" directly to the
   process's real stdout (fd 1). A naive `mcp.server.stdio.stdio_server()`
   reads/writes JSON-RPC framing on that SAME fd — Blender's own prints
   corrupt the stream and the client sees "Connection closed". FIX: dup the
   real stdout fd immediately, before bpy/Blender gets a chance to write to
   it, then rebind fd 1 -> /dev/null for the rest of the process. The
   duplicated fd is re-wrapped as a text-mode async file and handed to
   `stdio_server()` explicitly, so JSON-RPC rides a clean channel Blender
   itself never touches. This must happen at the TOP of the file, before
   `import bpy`.

2. Blender's isolated Python startup IGNORES the `PYTHONPATH` env var
   entirely (verified: exporting it does nothing to `sys.path` inside
   Blender). Any pip-installed dependency (the `mcp` SDK itself) must be
   injected via `sys.path.insert()` inside this script, before importing it.
   See `_DEPS_DIR` below — point it at wherever `mcp` was installed
   (`pip install --break-system-packages --target <dir> mcp`, or the system
   site-packages if installed with `--break-system-packages` directly).
"""

import os
import sys

# ── Fix #1: protect the stdio JSON-RPC channel from Blender's own prints ───
_real_stdout_fd = os.dup(1)  # clone the real pipe FIRST, before any bpy import
_devnull_fd = os.open(os.devnull, os.O_WRONLY)
os.dup2(_devnull_fd, 1)  # Blender's C-level / print() output -> /dev/null now
os.close(_devnull_fd)
_clean_stdout_raw = os.fdopen(_real_stdout_fd, "w", buffering=1, encoding="utf-8")

# ── Fix #2: Blender ignores PYTHONPATH — inject deps explicitly ────────────
# Prefer an env override (set by the launcher / spell.kdl) so this isn't
# hardcoded to one operator's machine; fall back to a repo-local vendored dir
# if present, else assume `mcp` was installed with --break-system-packages
# directly into Blender's system Python (no injection needed in that case).
_DEPS_DIR = os.environ.get("BLENDER_MCP_DEPS_DIR")
if _DEPS_DIR and os.path.isdir(_DEPS_DIR):
    sys.path.insert(0, _DEPS_DIR)

import anyio  # noqa: E402

_clean_stdout = anyio.wrap_file(_clean_stdout_raw)

import asyncio  # noqa: E402
import json  # noqa: E402
import math  # noqa: E402
import traceback  # noqa: E402

import bpy  # noqa: E402
import bmesh  # noqa: E402

from mcp.server import Server  # noqa: E402
from mcp.server.stdio import stdio_server  # noqa: E402
from mcp.types import Tool, TextContent  # noqa: E402

app = Server("blender-headless")


# ═══════════════════════════════════════════════════════════════════════
# Scene helpers
# ═══════════════════════════════════════════════════════════════════════

def _clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    # Also purge orphaned meshes/materials left over from a prior tool call
    # in this same (potentially warm) process, so repeated calls don't leak.
    for block in list(bpy.data.meshes):
        if block.users == 0:
            bpy.data.meshes.remove(block)
    for block in list(bpy.data.materials):
        if block.users == 0:
            bpy.data.materials.remove(block)


def _get_object(name: str):
    obj = bpy.data.objects.get(name)
    if obj is None:
        raise ValueError(f"no object named {name!r} in the current scene")
    return obj


def _make_material(name, color, roughness, metalness, emissive, emissive_intensity, transmission, ior):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metalness
    if "Emission Color" in bsdf.inputs:
        bsdf.inputs["Emission Color"].default_value = (*emissive, 1.0)
    if "Emission Strength" in bsdf.inputs:
        bsdf.inputs["Emission Strength"].default_value = emissive_intensity
    if transmission > 0 and "Transmission Weight" in bsdf.inputs:
        bsdf.inputs["Transmission Weight"].default_value = transmission
        if "IOR" in bsdf.inputs:
            bsdf.inputs["IOR"].default_value = ior
    return mat


# ═══════════════════════════════════════════════════════════════════════
# Tools
# ═══════════════════════════════════════════════════════════════════════

TOOLS = [
    Tool(
        name="clear_scene",
        description="Delete every object/mesh/material in the current Blender scene. Call this first for a clean build.",
        inputSchema={"type": "object", "properties": {}},
    ),
    Tool(
        name="create_primitive",
        description=(
            "Create a primitive mesh object (box, uv_sphere, ico_sphere, cylinder, cone, torus, plane). "
            "Mirrors stdlib/3d's @object shape surface, plus a few extras bpy exposes natively."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "shape": {"type": "string", "enum": ["box", "uv_sphere", "ico_sphere", "cylinder", "cone", "torus", "plane"]},
                "name": {"type": "string"},
                "size": {"type": "number", "default": 1.0},
                "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0, 0, 0]},
                "rotation": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0, 0, 0]},
            },
            "required": ["shape", "name"],
        },
    ),
    Tool(
        name="extrude_polygon",
        description=(
            "Build a custom N-gon from 2D points (in the XY plane) and extrude it to a depth, with optional "
            "bevel on the resulting edges. Closes stdlib/3d gap: no @object shape does arbitrary "
            "extrude/custom-vertex geometry (FUP-102 gap #1) — this is the tool for that."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "points": {
                    "type": "array",
                    "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2},
                    "minItems": 3,
                },
                "depth": {"type": "number", "default": 0.25},
                "bevel_offset": {"type": "number", "default": 0.0},
                "bevel_segments": {"type": "integer", "default": 0},
                "position": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0, 0, 0]},
                "rotation": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0, 0, 0]},
            },
            "required": ["name", "points"],
        },
    ),
    Tool(
        name="boolean_op",
        description=(
            "Boolean union/difference/intersect between two named mesh objects (the `tool` object is consumed). "
            "A capability arbitrary extrude/geometry macros structurally can't express — genuine CSG."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "target": {"type": "string"},
                "tool": {"type": "string"},
                "op": {"type": "string", "enum": ["union", "difference", "intersect"]},
            },
            "required": ["target", "tool", "op"],
        },
    ),
    Tool(
        name="assign_material",
        description=(
            "Assign a PBR material to an object (or one of its face-material slots — see set_face_material). "
            "Param surface matches stdlib/3d's @object macro 1:1 (color/roughness/metalness/emissive/transmission/ior)."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "object": {"type": "string"},
                "material_name": {"type": "string"},
                "color": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0.8, 0.8, 0.8]},
                "roughness": {"type": "number", "default": 0.5},
                "metalness": {"type": "number", "default": 0.0},
                "emissive": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3, "default": [0, 0, 0]},
                "emissive_intensity": {"type": "number", "default": 0.0},
                "transmission": {"type": "number", "default": 0.0},
                "ior": {"type": "number", "default": 1.5},
            },
            "required": ["object", "material_name"],
        },
    ),
    Tool(
        name="set_face_material",
        description=(
            "Assign a second (or Nth) material slot to a subset of an object's faces, selected by normal "
            "direction (top/bottom/sides) or explicit face-index list. Closes stdlib/3d gap: @object binds "
            "ONE material per mesh; a real object (e.g. a ceramic tile) often needs glaze-cap vs matte-rim "
            "(FUP-102 gap #2). Call assign_material first to create the material, THEN this to place it on "
            "a face subset — the object ends up with 2+ material slots, which glTF export emits as geometry "
            "groups (three.js material array, matching the demo already-vendored pattern)."
        ),
        inputSchema={
            "type": "object",
            "properties": {
                "object": {"type": "string"},
                "material_name": {"type": "string"},
                "faces": {
                    "type": "string",
                    "enum": ["top", "bottom", "sides", "all"],
                    "description": "which faces (by normal direction) get this material",
                },
            },
            "required": ["object", "material_name", "faces"],
        },
    ),
    Tool(
        name="export_glb",
        description=(
            "Export the current scene as a binary glTF (.glb). Draco compression is ALWAYS force-disabled "
            "(stdlib/3d's three-entry.ts build excludes DRACOLoader — a Draco-compressed .glb silently fails "
            "to load client-side). Returns the output path and byte size."
        ),
        inputSchema={
            "type": "object",
            "properties": {"out_path": {"type": "string"}},
            "required": ["out_path"],
        },
    ),
]


@app.list_tools()
async def list_tools():
    return TOOLS


@app.call_tool()
async def call_tool(name: str, arguments: dict):
    try:
        result = _dispatch(name, arguments or {})
        return [TextContent(type="text", text=json.dumps(result))]
    except Exception as exc:  # noqa: BLE001 — surface every failure to the MCP client, not just stderr
        return [TextContent(type="text", text=json.dumps({"error": str(exc), "trace": traceback.format_exc()}))]


def _dispatch(name: str, args: dict) -> dict:
    if name == "clear_scene":
        _clear_scene()
        return {"ok": True}

    if name == "create_primitive":
        shape = args["shape"]
        obj_name = args["name"]
        size = args.get("size", 1.0)
        pos = args.get("position", [0, 0, 0])
        rot = args.get("rotation", [0, 0, 0])
        rot_rad = [math.radians(r) for r in rot]

        add_fns = {
            "box": lambda: bpy.ops.mesh.primitive_cube_add(size=size, location=pos, rotation=rot_rad),
            "uv_sphere": lambda: bpy.ops.mesh.primitive_uv_sphere_add(radius=size / 2, location=pos, rotation=rot_rad),
            "ico_sphere": lambda: bpy.ops.mesh.primitive_ico_sphere_add(radius=size / 2, location=pos, rotation=rot_rad),
            "cylinder": lambda: bpy.ops.mesh.primitive_cylinder_add(radius=size / 2, depth=size, location=pos, rotation=rot_rad),
            "cone": lambda: bpy.ops.mesh.primitive_cone_add(radius1=size / 2, depth=size, location=pos, rotation=rot_rad),
            "torus": lambda: bpy.ops.mesh.primitive_torus_add(major_radius=size / 2, minor_radius=size / 6, location=pos, rotation=rot_rad),
            "plane": lambda: bpy.ops.mesh.primitive_plane_add(size=size, location=pos, rotation=rot_rad),
        }
        if shape not in add_fns:
            raise ValueError(f"unknown shape {shape!r}")
        add_fns[shape]()
        obj = bpy.context.active_object
        obj.name = obj_name
        return {"ok": True, "object": obj.name, "vertices": len(obj.data.vertices)}

    if name == "extrude_polygon":
        obj_name = args["name"]
        points = args["points"]
        depth = args.get("depth", 0.25)
        bevel_offset = args.get("bevel_offset", 0.0)
        bevel_segments = args.get("bevel_segments", 0)
        pos = args.get("position", [0, 0, 0])
        rot = args.get("rotation", [0, 0, 0])

        mesh = bpy.data.meshes.new(obj_name)
        obj = bpy.data.objects.new(obj_name, mesh)
        bpy.context.collection.objects.link(obj)
        obj.location = pos
        obj.rotation_euler = [math.radians(r) for r in rot]

        bm = bmesh.new()
        verts = [bm.verts.new((px, py, 0)) for px, py in points]
        face = bm.faces.new(verts)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
        if depth != 0:
            ret = bmesh.ops.extrude_face_region(bm, geom=[face])
            ex_verts = [g for g in ret["geom"] if isinstance(g, bmesh.types.BMVert)]
            bmesh.ops.translate(bm, vec=(0, 0, depth), verts=ex_verts)
        if bevel_offset > 0 and bevel_segments > 0:
            bmesh.ops.bevel(bm, geom=bm.edges[:], offset=bevel_offset, segments=bevel_segments)
        bm.to_mesh(mesh)
        bm.free()
        mesh.update()

        return {"ok": True, "object": obj.name, "vertices": len(mesh.vertices), "faces": len(mesh.polygons)}

    if name == "boolean_op":
        target = _get_object(args["target"])
        tool_obj = _get_object(args["tool"])
        op = args["op"]

        bpy.context.view_layer.objects.active = target
        mod = target.modifiers.new(name="boolean_op", type="BOOLEAN")
        mod.operation = op.upper()
        mod.object = tool_obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
        bpy.data.objects.remove(tool_obj, do_unlink=True)

        return {"ok": True, "object": target.name, "vertices": len(target.data.vertices)}

    if name == "assign_material":
        obj = _get_object(args["object"])
        mat = _make_material(
            args["material_name"],
            args.get("color", [0.8, 0.8, 0.8]),
            args.get("roughness", 0.5),
            args.get("metalness", 0.0),
            args.get("emissive", [0, 0, 0]),
            args.get("emissive_intensity", 0.0),
            args.get("transmission", 0.0),
            args.get("ior", 1.5),
        )
        obj.data.materials.append(mat)
        new_slot = len(obj.data.materials) - 1

        # A boolean-modifier apply can leave a stray EMPTY (None) material slot
        # on the target mesh (Blender pads a slot for the tool object even when
        # it had no material). If every face still points at such an empty
        # slot, the freshly-appended material has no face using it and glTF
        # export silently drops it as unused. Re-target any face on an empty
        # slot to the material we just added, so a plain "assign one material"
        # call always actually paints the whole object (the common case);
        # multi-material objects still work via set_face_material afterward.
        mesh = obj.data
        empty_slots = {i for i, m in enumerate(obj.data.materials) if m is None}
        if empty_slots:
            for poly in mesh.polygons:
                if poly.material_index in empty_slots:
                    poly.material_index = new_slot
            mesh.update()

        return {"ok": True, "object": obj.name, "material": mat.name, "slot": new_slot}

    if name == "set_face_material":
        obj = _get_object(args["object"])
        mat_name = args["material_name"]
        which = args["faces"]

        # BUG-157: a prior boolean_op (union/difference) can leave a stray EMPTY
        # (None) material slot on the mesh (Blender pads a slot for the tool
        # object even when it had no material) -- guard the None case, same fix
        # assign_material already applies to ITS OWN slot handling above.
        slot_index = next(
            (i for i, m in enumerate(obj.data.materials) if m is not None and m.name == mat_name),
            None,
        )
        if slot_index is None:
            raise ValueError(f"material {mat_name!r} not assigned to {obj.name!r} yet — call assign_material first")

        mesh = obj.data
        for poly in mesh.polygons:
            nz = poly.normal.z
            match = (
                which == "all"
                or (which == "top" and nz > 0.5)
                or (which == "bottom" and nz < -0.5)
                or (which == "sides" and abs(nz) <= 0.5)
            )
            if match:
                poly.material_index = slot_index
        mesh.update()

        return {"ok": True, "object": obj.name, "material": mat_name, "faces": which}

    if name == "export_glb":
        out_path = args["out_path"]
        os.makedirs(os.path.dirname(os.path.abspath(out_path)) or ".", exist_ok=True)
        bpy.ops.export_scene.gltf(
            filepath=out_path,
            export_format="GLB",
            export_draco_mesh_compression_enable=False,  # HARD requirement — see module docstring
        )
        size = os.path.getsize(out_path)
        return {"ok": True, "out_path": out_path, "bytes": size}

    raise ValueError(f"unknown tool {name!r}")


async def main():
    async with stdio_server(stdin=None, stdout=_clean_stdout) as (read, write):
        await app.run(read, write, app.create_initialization_options())


if __name__ == "__main__":
    asyncio.run(main())
