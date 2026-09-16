#!/usr/bin/env bash
# ============================================================================
# GH-20 — regression gate: raw `@font-face` is SUPPORTED and emitted.
#
# Issue: a standard top-level `@font-face { font-family: "Test Font"; … }` used
# to compile clean but vanish — no CSS, no IR, and a misleading W0714 "not a
# known directive" (because `@font-face` is the macro NAME whose `%form` is the
# `@font("Family")` spelling). Product ruled (PLAN-137 W7): the host CSS
# spelling must WORK, not vanish.
#
# Fix: `@font-face` joins the host CSS at-rule passthrough registry
# (`HOST_CSS_PASSTHROUGH_AT_RULES` in src/parser/mod.rs) alongside `@media` /
# `@supports` / `@keyframes` (BUG-087). It is emitted verbatim into the
# stylesheet, converging with the `@font("Family")` macro on the same
# `@font-face` rule. The misleading W0714 is gone.
#
# WEAKNESS THIS GATE PREVIOUSLY HAD (W6 review) — it could go GREEN for the
# wrong reason three ways:
#   (a) STALE ARTIFACT: the old gate grepped `$DIR/spacetime.css`, which
#       PERSISTS between runs. If a regression made `build` stop emitting the
#       @font-face rule (the silent-vanish mode), the PREVIOUS run's CSS still
#       carried the rule and both greps passed — the gate reported on a build
#       that never happened. The gate now DELETES the CSS/JS before building, so
#       a regression that stops writing leaves no file → RED.
#   (b) UNTIED GREPS: it grep'd `@font-face` and separately grep'd `Test Font`.
#       `Test Font` ALSO appears in the source's `body { font-family: "Test
#       Font" }` rule, so the family string is emitted even when the @font-face
#       rule is dropped or mis-spelled. Both greps could pass while the actual
#       @font-face rule declared a DIFFERENT (or no) family. The gate now
#       extracts the @font-face rule BLOCK and asserts `font-family: "Test
#       Font"` sits INSIDE it — rule and family are tied to the same construct.
#   (c) NEGATIVE WITHOUT A POSITIVE: the W0714 half was a bare "absence" grep;
#       it is trivially true if `check` fails to compile the file at all. The
#       gate now requires `check` to exit 0 (the file COMPILED — the positive
#       control) before the W0714 absence means anything.
#
# DESIRED CONTRACT:
#   - `check index.st` compiles clean (exit 0) with NO W0714.
#   - `build index.st` — against a FRESH (deleted) artifact dir — emits a CSS
#     stylesheet whose `@font-face` rule declares `font-family: "Test Font"`.
#
# POLARITY: the fix is LIVE today → PASS (exit 0 → GREEN). If a regression
# re-drops `@font-face`, the CSS assertions fail → RED.
#
# Why check-contract.sh and not a .test.st: the real behavioral gate (the rule
# reaches `document.styleSheets` and the family resolves) needs CSSOM, i.e. the
# `--cdp` layout rung — see `font-face.cdp.test.st`, unrunnable here (no
# Chromium). Headless, the built stylesheet is the assertable contract; the
# .cdp gate is the browser half. This script asserts the emitted-stylesheet half.
#
# Run:   tests/repro/gh-20-font-face/check-contract.sh
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
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

# 1) check must COMPILE the file: a clean exit proves the directive was actually
#    seen and handled, so the W0714 absence below is meaningful (not a vacuous
#    negative from a failed/aborted compile).
CHECK_OUT="$("$BIN" check "$DIR/index.st" 2>&1)"
CHECK_STATUS=$?
if [ "$CHECK_STATUS" -ne 0 ]; then
  echo "FAIL: check did not compile index.st cleanly (exit $CHECK_STATUS) — the W0714 check has nothing to assert."
  echo "$CHECK_OUT"
  exit 1
fi
if echo "$CHECK_OUT" | grep -q 'W0714'; then
  echo "FAIL: check still warns W0714 'not a known directive' for @font-face (GH-20 regression)."
  echo "$CHECK_OUT" | grep 'W0714'
  exit 1
fi

# 2) build a FRESH stylesheet: remove any stale artifact from a previous run so
#    a regression that stops emitting CSS cannot pass on yesterday's file.
rm -f "$DIR/spacetime.css" "$DIR/spacetime.js"
BUILD_OUT="$("$BIN" build "$DIR/index.st" 2>&1)"
if ! echo "$BUILD_OUT" | grep -q 'Build complete'; then
  echo "FAIL: build failed."
  echo "$BUILD_OUT"
  exit 1
fi
CSS="$DIR/spacetime.css"
if [ ! -f "$CSS" ]; then
  echo "FAIL: no $CSS produced by THIS build (stale artifact was removed; build emitted nothing)."
  echo "$BUILD_OUT"
  exit 1
fi

# 3) extract the @font-face rule BLOCK and require the family INSIDE it — ties
#    rule to family, so a stray `Test Font` in the `body` rule no longer suffices.
FACE_BLOCK="$(awk '/^@font-face/{f=1} f{print} f&&/^}/{exit}' "$CSS")"
if [ -z "$FACE_BLOCK" ]; then
  echo "FAIL: the emitted stylesheet contains no @font-face rule (GH-20 regression)."
  echo "--- actual CSS ---"
  cat "$CSS"
  exit 1
fi
if ! echo "$FACE_BLOCK" | grep -q 'font-family: "Test Font"'; then
  echo "FAIL: the @font-face rule does not declare 'Test Font'."
  echo "--- @font-face block ---"
  echo "$FACE_BLOCK"
  exit 1
fi

echo "PASS: raw @font-face is emitted (check compiles clean, fresh stylesheet carries the rule + family)."
echo "--- emitted @font-face ---"
echo "$FACE_BLOCK"
exit 0
