#!/usr/bin/env bash
# ============================================================================
# GH-26 — contract gate: `check` MUST emit E0408 for an undefined signal.
#
# Issue: E0408 ("signal used but not defined") was implemented in
# src/analysis/signals.rs but never wired into check_file, so `$definitelyMissing`
# compiled to a runtime ST.resolve lookup that rendered empty with ZERO
# diagnostics. The repro page (index.st) and its test body both use the signal.
#
# DESIRED CONTRACT: `check` on the repro page emits E0408 (and, since the page
# is also mounted inside the .test.st's @mount fixture, check on the .test.st
# emits it too). BEFORE the fix: zero E0408 (silent). AFTER: E0408 fires.
#
# POLARITY: the bug was LIVE (no E0408), so this script FAILED (exit 1 → RED).
# With E0408 wired it exits 0 → GREEN.
#
# Run:   tests/repro/gh-26-e0408/check-contract.sh
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

PAGE_OUT="$("$BIN" check --verbose "$DIR/index.st" 2>&1)"
if ! echo "$PAGE_OUT" | grep -q 'E0408'; then
  echo "FAIL: check on the repro page did NOT emit E0408 (GH-26 is LIVE — silent acceptance)."
  echo "--- actual stdout ---"
  echo "$PAGE_OUT"
  exit 1
fi

TEST_OUT="$("$BIN" check --verbose "$DIR/gh-26-e0408.test.st" 2>&1)"
if ! echo "$TEST_OUT" | grep -q 'E0408'; then
  echo "FAIL: check on the .test.st did NOT emit E0408."
  echo "--- actual stdout ---"
  echo "$TEST_OUT"
  exit 1
fi

echo "PASS: check emits E0408 for the undefined signal on the page and the test body."
exit 0
