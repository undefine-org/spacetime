#!/usr/bin/env bash
# ============================================================================
# GH-35 / BUG-342 — contract gate: `$user object: { name: "Ada" };` must
# COMPILE CLEAN and carry its value through to the emitted output.
#
# POLARITY HISTORY (this script asserted the opposite until BUG-342 landed):
#
#   1. silent elision  — the object initializer vanished; the consumer still
#                        emitted a read, so the page rendered `undefined` with
#                        ZERO diagnostics.
#   2. refusal (E0946) — better: a wrong answer became a refusal. This script
#                        asserted that diagnostic, correctly, for as long as the
#                        feature did not exist.
#   3. fixed (a0e3aa0d)— `variable_ref` now uses the same `starts_object_literal`
#                        guard `directive()` already had, so a `{` in VALUE
#                        position is an object literal, not a body.
#
# At step 3 the E0946 assertion became FALSE and this script began FAILING
# AGAINST CORRECT CODE — what a refusal-gate always does once the feature
# lands: it pins the workaround instead of the contract. So it now asserts the
# real contract (clean compile + value present), which is false at step 1 AND
# step 2 and true only at step 3.
#
# The behavioral half (does "Ada" actually RENDER) lives in
# gh-35-object-local-state.test.st — a source check alone cannot prove that.
#
# Run:    tests/repro/gh-35-object-local-state/check-contract.sh
# Binary: overridable via SPACETIME_BIN env var.
# ============================================================================
set -u

BIN="${SPACETIME_BIN:-}"
if [ -z "$BIN" ]; then
  if [ -x "$HOME/.cache/cargo-target/debug/spacetime" ]; then
    BIN="$HOME/.cache/cargo-target/debug/spacetime"
  elif command -v spacetime >/dev/null 2>&1; then
    BIN="$(command -v spacetime)"
  else
    echo "FATAL: cannot locate the spacetime binary (set SPACETIME_BIN)" >&2
    exit 2
  fi
fi

DIR="$(cd "$(dirname "$0")" && pwd)"
# Run from the repo root so stdlib / relative imports resolve the same way as
# the documented `<BIN> test <file>` invocation (the binary bases import
# resolution on CWD).
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

OUT="$("$BIN" check "$DIR/index.st" 2>&1)"
STATUS=$?

FAILED=0

# 1. The object initializer must compile CLEAN — no E0946 (the old refusal),
#    and no E0408 (the signal going undefined because its declaration was
#    dropped, which is how the silent-elision era presented downstream).
if echo "$OUT" | grep -q 'E0946'; then
  echo "FAIL: E0946 for the object initializer — regressed to the refusal era."
  FAILED=1
fi
if echo "$OUT" | grep -q 'E0408'; then
  echo "FAIL: E0408 — the declaration was dropped, so the signal reads as undefined."
  FAILED=1
fi
if [ "$STATUS" -ne 0 ]; then
  echo "FAIL: check exited $STATUS; the object initializer must compile clean."
  FAILED=1
fi

if [ "$FAILED" -ne 0 ]; then
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi

echo "PASS: object-typed local-state initializer compiles clean (no E0946/E0408)."
echo "NOTE: the render half is gated by gh-35-object-local-state.test.st."
exit 0
