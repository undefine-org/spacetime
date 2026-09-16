#!/usr/bin/env bash
# One-time setup: install the `mcp` Python SDK into Blender's own interpreter.
#
# This Blender build uses the SYSTEM Python (verified: `sys.executable` inside
# `blender --background --python-expr "import sys; print(sys.executable)"`
# prints `/usr/bin/python3.<N>`, not a vendored interpreter) — so we install
# straight against it. `--break-system-packages` is required on Arch's
# PEP-668-guarded system Python.
#
# If you'd rather not touch system site-packages, set BLENDER_MCP_DEPS_DIR to
# an isolated target dir and bootstrap.py will sys.path-inject it instead
# (Blender ignores PYTHONPATH — see bootstrap.py's module docstring).

set -euo pipefail

BLENDER_PY="$(blender --background --python-expr 'import sys; print(sys.executable)' 2>/dev/null | grep -E '^/.*python' | head -1)"
if [[ -z "$BLENDER_PY" ]]; then
  echo "Could not determine Blender's Python interpreter. Is 'blender' on PATH?" >&2
  exit 1
fi
echo "Blender's Python: $BLENDER_PY"

if [[ -n "${BLENDER_MCP_DEPS_DIR:-}" ]]; then
  echo "Installing mcp SDK into isolated target: $BLENDER_MCP_DEPS_DIR"
  "$BLENDER_PY" -m pip install --break-system-packages --target "$BLENDER_MCP_DEPS_DIR" mcp
  echo "Set BLENDER_MCP_DEPS_DIR=$BLENDER_MCP_DEPS_DIR when launching bootstrap.py."
else
  echo "Installing mcp SDK directly into Blender's system Python site-packages"
  "$BLENDER_PY" -m pip install --break-system-packages mcp
fi

echo "Verifying import..."
"$BLENDER_PY" -c "import mcp; print('mcp SDK OK:', mcp.__file__)"
echo "Done. Test the server with: blender --background --python tools/blender-mcp/bootstrap.py"
