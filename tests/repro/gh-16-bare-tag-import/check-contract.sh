#!/usr/bin/env bash
# ============================================================================
# GH-16 — regression gate: bare-tag-led rule after semicolon-less @import MUST
# reach the emitted CSS.
#
# Issue: `@import "stdlib"` (NO trailing ';') immediately followed by a
# bare-tag-led selector rule (`canvas.hero { ... }`) as the FIRST rule used to be
# silently absorbed into @import's own parsing → zero output. Fix: the rule now
# survives and emits identically to the `;` control (repro4.st).
#
# DESIRED CONTRACT: `build repro1.st` → emitted spacetime.css contains the
# `canvas.hero` rule. (A .test.st can't assert this: the rule is static
# width/height, not reactive — observing computed style is the layout/CDP rung,
# and there is no headless stylesheet prop. Emitted-artifact inspection is the
# honest gate.)
#
# POLARITY: the fix is LIVE today, so this script PASSES (exit 0 → GREEN). If a
# regression re-absorbs the rule, it goes RED.
#
# Run:   tests/repro/gh-16-bare-tag-import/check-contract.sh
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

"$BIN" build "$DIR/repro1.st" -o "$OUTDIR" >/dev/null 2>&1
CSS="$OUTDIR/spacetime.css"
if [ -f "$CSS" ] && grep -q 'canvas.hero' "$CSS"; then
  echo "PASS: canvas.hero rule reached emitted CSS after the semicolon-less @import."
  grep 'canvas.hero' "$CSS"
  exit 0
else
  echo "FAIL: canvas.hero rule missing from emitted CSS (GH-16 regression)."
  [ -f "$CSS" ] && echo "--- emitted css ---" && cat "$CSS"
  exit 1
fi
