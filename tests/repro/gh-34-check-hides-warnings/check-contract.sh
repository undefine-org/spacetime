#!/usr/bin/env bash
# ============================================================================
# GH-34 — contract gate: plain `check` MUST surface warnings.
#
# Issue: a warning-only file (`@data inline $unused : 1;` → W0201 "defined but
# never used") prints green "All files passed" with ZERO warning text on a plain
# `spacetime check`. Warnings surface only under `--verbose` (gating:
# verbose || errors — see main.rs). So a normal local/CI check cannot observe
# warnings. The issue's core observation is LIVE.
#
# DESIRED CONTRACT (what this script asserts): a plain `check` (no --verbose)
# of the warning-only file must print the W0201 warning text.
#
# POLARITY: the bug is LIVE today, so this script FAILS (exit 1 → RED). When
# the gating is fixed so warnings surface on plain `check`, it goes GREEN.
#
# Run:   tests/repro/gh-34-check-hides-warnings/check-contract.sh
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
# Run from the repo root: the binary resolves `@import "stdlib"` relative to
# CWD, not the file location.
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

# 1. Plain check (no --verbose) MUST surface the W0201 warning text.
OUT="$("$BIN" check "$DIR/index.st" 2>&1)"
if ! echo "$OUT" | grep -q 'W0201'; then
  echo "FAIL: plain \`check\` hid the W0201 warning (GH-34 is LIVE)."
  echo "Expected stdout to contain: 'warning[W0201]: Data source .* never used'"
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi
echo "PASS: plain \`check\` surfaced the W0201 warning text."

# 2. --deny-warnings MUST exit non-zero on a warning-only file (CI gate).
"$BIN" check --deny-warnings "$DIR/index.st" >/dev/null 2>&1
if [ $? -eq 0 ]; then
  echo "FAIL: --deny-warnings exited 0 on a warning-only file (should be non-zero for CI)."
  exit 1
fi
echo "PASS: --deny-warnings exited non-zero on a warning-only file."

# 3. --quiet MUST print nothing (warning text suppressed) and stay exit 0.
QUIET_OUT="$("$BIN" check --quiet "$DIR/index.st" 2>&1)"
if [ -n "$QUIET_OUT" ]; then
  echo "FAIL: --quiet printed output on a warning-only file."
  echo "--- actual stdout ---"
  echo "$QUIET_OUT"
  exit 1
fi
"$BIN" check --quiet "$DIR/index.st" >/dev/null 2>&1
if [ $? -ne 0 ]; then
  echo "FAIL: --quiet exited non-zero on a warning-only file (warnings are not errors)."
  exit 1
fi
echo "PASS: --quiet printed nothing and exited 0."

exit 0
