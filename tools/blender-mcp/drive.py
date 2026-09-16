#!/usr/bin/env python3
"""
Manual driver for the headless Blender MCP server — lets an agent (or a human)
call bootstrap.py's tools from the command line WITHOUT registering the server
in spell.kdl / reloading Spell. Same server code either way; this is just an
alternate client.

Usage:
    python3 tools/blender-mcp/drive.py list
    python3 tools/blender-mcp/drive.py call <tool_name> '<json_args>'
    python3 tools/blender-mcp/drive.py script <path/to/script.json>

`script` runs a JSON array of {"tool": ..., "args": {...}} calls in one
Blender process (one scene, sequential — mirrors how a real modeling session
builds up state across multiple tool calls before a single export_glb).

Requires: the `mcp` SDK on THIS python's path (pip install mcp), independent
of whatever Blender's own interpreter has installed (see setup.sh).
"""
import asyncio
import json
import os
import sys
from pathlib import Path

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

BOOTSTRAP = str(Path(__file__).parent / "bootstrap.py")


async def _run(calls):
    params = StdioServerParameters(command="blender", args=["--background", "--python", BOOTSTRAP], env=dict(os.environ))
    results = []
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            for tool_name, args in calls:
                result = await session.call_tool(tool_name, args)
                for c in result.content:
                    results.append((tool_name, c.text))
    return results


async def _list():
    params = StdioServerParameters(command="blender", args=["--background", "--python", BOOTSTRAP], env=dict(os.environ))
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            return tools.tools


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "list":
        tools = asyncio.run(_list())
        for t in tools:
            print(f"- {t.name}: {t.description}")
        return

    if cmd == "call":
        tool_name = sys.argv[2]
        args = json.loads(sys.argv[3]) if len(sys.argv) > 3 else {}
        results = asyncio.run(_run([(tool_name, args)]))
        for name, text in results:
            print(f"[{name}] {text}")
        return

    if cmd == "script":
        script_path = sys.argv[2]
        calls = json.loads(Path(script_path).read_text())
        pairs = [(c["tool"], c.get("args", {})) for c in calls]
        results = asyncio.run(_run(pairs))
        for name, text in results:
            print(f"[{name}] {text}")
        return

    print(__doc__)
    sys.exit(1)


if __name__ == "__main__":
    main()
