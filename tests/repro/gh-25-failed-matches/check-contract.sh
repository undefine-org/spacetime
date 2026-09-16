#!/usr/bin/env bash
# ============================================================================
# GH-25 — regression gate: malformed known-directive shapes MUST fail E0946.
#
# Issue: three registered directives in shapes matching none of their %forms
#   @scroll(name: demo) / @load(name: intro) / @each(: $items)
# used to have their best MatchDiagnostic discarded → zero diagnostics + silent
# empty expansion. Fix (BUG-229 match_diagnostics_to_errors + BUG-264 guards):
# matcher diagnostics now surface as loud E0946 errors with locations.
#
# DESIRED CONTRACT: `check --verbose index.st` prints >=3 `E0946` errors.
#
# POLARITY: the fix is LIVE today, so this script PASSES (exit 0 → GREEN). If a
# regression returns the silent discard (diagnostics dropped), it goes RED.
#
# Why check-contract.sh and not a .test.st: a .test.st carrying the malformed
# shapes FAILS to compile, which the runner reports as a FAILED file — the signal
# would be INVERTED (test passes exactly when the diagnostics regress to
# silence). A stdout count of E0946 is the clean, non-inverted polarity.
#
# Run:   tests/repro/gh-25-failed-matches/check-contract.sh
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

OUT="$("$BIN" check --verbose "$DIR/index.st" 2>&1)"
N="$(echo "$OUT" | grep -c 'E0946')"
if [ "$N" -ge 3 ]; then
  echo "PASS: $N E0946 errors surfaced for the malformed directive shapes."
  exit 0
else
  echo "FAIL: expected >=3 E0946 errors, got $N (GH-25 regression: silent discard)."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi
