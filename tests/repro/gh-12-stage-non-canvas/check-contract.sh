#!/usr/bin/env bash
# ============================================================================
# GH-12 — contract gate: `check` MUST refuse a `@stage` bound to a non-canvas
# element, accept a `@stage` on a `<canvas>` selector, and WARN (not silently
# pass) when the element is unknowable from the selector.
#
# Issue: `@stage` (and the whole 3D surface) only ever renders into a
# `<canvas>`, but could bind to ANY selector with zero compile-time
# diagnostic. `div.hero { @stage }` compiled clean and failed only at runtime
# (three.js `canvas.getContext` throw + sibling `console.warn`s) — nothing in
# the compiler's output could be searched for.
#
# Fix: the stage macro declares `%scope selector element(canvas)`. The compiler
# resolves the bound selector's implied element and errors when it cannot be a
# canvas, and warns when the element is unknowable (no tag in the selector).
#
# DESIRED CONTRACT (three legs, all must hold):
#   (1) `check` on index.st  (div.hero)     → E0963 ERROR naming `<canvas>`
#   (2) `check` on canvas.st (canvas.hero)  → NO E0963 (correct spelling compiles)
#   (3) `check` on unknowable.st (.hero)    → W0963 WARNING (never silent)
#
# POLARITY: today (bug live) legs (1) and (3) FAIL silently (no diagnostic) and
# leg (2) passes. This script exits 1 → RED. With the fix it exits 0 → GREEN.
#
# Run:   tests/repro/gh-12-stage-non-canvas/check-contract.sh
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
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

fail=0

# Leg 1: div.hero → E0963 error.
DIV_OUT="$("$BIN" check --verbose "$DIR/index.st" 2>&1)"
if ! echo "$DIV_OUT" | grep -q 'E0963'; then
  echo "FAIL leg1: check on div.hero did NOT emit E0963 (GH-12 still compiles clean)."
  echo "--- actual stdout (index.st) ---"
  echo "$DIV_OUT"
  fail=1
fi

# Leg 2: canvas.hero → no E0963.
CAN_OUT="$("$BIN" check --verbose "$DIR/canvas.st" 2>&1)"
if echo "$CAN_OUT" | grep -q 'E0963'; then
  echo "FAIL leg2: check on canvas.hero emitted E0963 — the correct spelling was refused."
  echo "--- actual stdout (canvas.st) ---"
  echo "$CAN_OUT"
  fail=1
fi

# Leg 3: .hero (unknowable) → W0963 warning, not silent.
UNK_OUT="$("$BIN" check --verbose "$DIR/unknowable.st" 2>&1)"
if ! echo "$UNK_OUT" | grep -q 'W0963'; then
  echo "FAIL leg3: check on .hero (unknowable element) did NOT emit W0963 — it passed silently."
  echo "--- actual stdout (unknowable.st) ---"
  echo "$UNK_OUT"
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "PASS: div.hero→E0963, canvas.hero→clean, .hero→W0963."
  exit 0
fi
echo "GH-12 contract gate FAILED."
exit 1
