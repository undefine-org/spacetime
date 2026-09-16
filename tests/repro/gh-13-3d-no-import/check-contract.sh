#!/usr/bin/env bash
# ============================================================================
# GH-13 — regression gate: missing-@import 3D primitive MUST fail loudly E0956.
#
# Issue: `.hero { @stage(camZ: 5) { @object(...) } }` WITHOUT `@import "stdlib/3d"`
# used to collapse silently into a `// Primitive not found: stage` comment in the
# emitted JS. Fix (BUG-268): unknown primitives are now a LOUD E0956 build
# failure. The issue's expectation is met.
#
# DESIRED CONTRACT: `check index.st` is a BUILD FAILURE whose diagnostic is
#   error[E0956]: unknown primitive: stage
# anchored to the `@stage(camZ: 5)` call. This is exactly the fix's promise:
# the unknown primitive is rejected, named, and the build does not succeed.
#
# WEAKNESS THIS GATE PREVIOUSLY HAD (W6 review): the old gate grepped for the
# bare code `E0956` ANYWHERE in the output. E0956 is the GENERIC "unknown
# primitive" diagnostic code — ANY unknown primitive anywhere (a different
# macro, a typo, one pulled in by the import graph) fires it. So the gate could
# go GREEN while `@stage` silently collapsed, as long as SOME E0956 surfaced
# somewhere. A code-anywhere grep is a weak proxy for "this exact construct
# failed loudly."
#
# STRONGER NOW: the gate pins THREE independent signals, each necessary and
# jointly sufficient for the contract:
#   1. The exit code is NON-ZERO — the build actually FAILED. A silent collapse
#      exits 0 and prints a comment, not an error.
#   2. The diagnostic line is `error[E0956]: unknown primitive: stage` — the
#      code AND the exact symbol that must be unknown. If E0956 fired for any
#      OTHER primitive, this line is absent → RED.
#   3. The diagnostic's caret context names `@stage(camZ: 5)` — the diagnostic
#      is anchored to the stage call, not incidental text.
# A regression that silently drops `@stage` fails every one of these.
#
# Why check-contract.sh and not a .test.st: a .test.st carrying the malformed
# shape FAILS to compile, which the runner reports as a FAILED file — the signal
# would be INVERTED (test passes exactly when the diagnostic regresses to
# silence). A stdout check on the build failure is the clean, non-inverted
# polarity.
#
# Run:   tests/repro/gh-13-3d-no-import/check-contract.sh
# Binary: overridable via SPACETIME_BIN env var.
# ============================================================================
set -u

BIN="${SPACETIME_BIN:-}"
if [ -z "$BIN" ]; then
  # Prefer the freshly BUILT binary over any `spacetime` on PATH: a stale
  # system install silently answers for the working tree and a gate then
  # reports on code nobody is editing (observed 2026-08-06: ~/.cargo/bin
  # shadowed the build and turned two GREEN contracts RED).
  _here="$(cd "$(dirname "$0")" && pwd)"
  _root="$(cd "$_here/../../.." && pwd)"
  for cand in "$_root/target/debug/spacetime" "$HOME/.cache/cargo-target/debug/spacetime"; do
    [ -x "$cand" ] && BIN="$cand" && break
  done
fi
if [ -z "$BIN" ]; then
  if command -v spacetime >/dev/null 2>&1; then
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

OUT="$("$BIN" check "$DIR/index.st" 2>&1)"
STATUS=$?

# 1) The unknown primitive must be a BUILD FAILURE, not a silent collapse.
if [ "$STATUS" -eq 0 ]; then
  echo "FAIL: check exited 0 — the missing-@import 3D primitive silently collapsed (GH-13 regression)."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi

# 2) The diagnostic must be E0956 naming the RIGHT symbol, `stage` — not just
#    any E0956 from any unknown primitive.
if ! echo "$OUT" | grep -q 'error\[E0956\]: unknown primitive: stage'; then
  echo "FAIL: no 'error[E0956]: unknown primitive: stage' — E0956 (if any) fired for a different construct than @stage."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi

# 3) The diagnostic must be anchored to the @stage call.
if ! echo "$OUT" | grep -q '@stage(camZ: 5)'; then
  echo "FAIL: the E0956 is not anchored to the @stage call."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi

echo "PASS: missing-@import 3D primitive is a loud E0956 build failure naming 'stage'."
echo "$OUT" | grep -i 'E0956' | head -1
exit 0
