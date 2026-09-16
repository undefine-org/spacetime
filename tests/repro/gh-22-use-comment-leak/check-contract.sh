#!/usr/bin/env bash
# ============================================================================
# GH-22 — regression gate: @use build must ship ZERO leaked comments AND the
# module's CSS.
#
# Issue: `@use "./module.st"` leaked a `// Primitive not found: use` comment into
# dist/spacetime.js. Fix: the namespaced @use resolves the module macro and
# emits its primitive cleanly (index2.st — the corrected full-page call with the
# badge body required by its %form).
#
# DESIRED CONTRACT: `build index2.st` produces
#   (a) spacetime.js with ZERO occurrences of `Primitive not found`, and
#   (b) spacetime.css containing the module's `.badge::before` rule.
# This is emitted-artifact inspection — the natural shell contract, not a
# DOM-behavior .test.st.
#
# POLARITY: the fix is LIVE today, so this script PASSES (exit 0 → GREEN). If
# the leak returns or the CSS is dropped, it goes RED.
#
# Run:   tests/repro/gh-22-use-comment-leak/check-contract.sh
# Binary: overridable via SPACETIME_BIN env var.
# ============================================================================
set -u

BIN="${SPACETIME_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x /home/user/.cache/cargo-target/debug/spacetime ]; then
    BIN=/home/user/.cache/cargo-target/debug/spacetime
  elif command -v spacetime >/dev/null 2>&1; then
    BIN="$(command -v spacetime)"
  else
    echo "FATAL: cannot locate the spacetime binary (set SPACETIME_BIN)" >&2
    exit 2
  fi
fi
DIR="$(cd "$(dirname "$0")" && pwd)"
# Run from the repo root so stdlib / relative imports resolve the same way as
# the documented `<BIN> test <file>` invocation (binary bases import resolution
# on CWD).
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2
OUTDIR="$(mktemp -d)"
trap 'rm -rf "$OUTDIR"' EXIT

"$BIN" build "$DIR/index2.st" -o "$OUTDIR" >/dev/null 2>&1
JS="$OUTDIR/spacetime.js"
CSS="$OUTDIR/spacetime.css"
LEAK="$(grep -c 'Primitive not found' "$JS" 2>/dev/null)"
LEAK="${LEAK:-0}"
CSS_OK=no
[ -f "$CSS" ] && grep -q '.badge::before' "$CSS" && CSS_OK=yes

if [ "$LEAK" -eq 0 ] && [ "$CSS_OK" = yes ]; then
  echo "PASS: 0 leaked 'Primitive not found'; module CSS (.badge::before) emitted."
  exit 0
else
  echo "FAIL: leaked-comment count=$LEAK (want 0); module CSS present=$CSS_OK (want yes) (GH-22 regression)."
  [ -f "$JS" ] && echo "--- spacetime.js ---" && cat "$JS"
  [ -f "$CSS" ] && echo "--- spacetime.css ---" && cat "$CSS"
  exit 1
fi
