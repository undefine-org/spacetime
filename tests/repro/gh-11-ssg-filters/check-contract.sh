#!/usr/bin/env bash
# ============================================================================
# GH-11 — SSG @each unroll must APPLY filter pipes, not emit them as text.
#
# Issue: a statically-unrolled `@each` renders the filter as literal page text:
#     <li><span data-st-hole="0"></span> | uppercase</li>
# so view-source disagrees with the hydrated page.
#
# Root cause (measured, see PLAN-138): `| uppercase` is implemented THREE times
# — stdlib/primitives/data/filters.st (15 %primitive, the declared surface),
# public/runtime/st.js, and public/runtime/data-binding.js — and the three do
# not agree on which filters exist. `capitalize` is DECLARED in stdlib and
# implemented in neither runtime table, so it fails at build time AND runtime.
# The build-time unroller is simply the path that agrees with nobody.
#
# DESIRED CONTRACT:
#   1. the SSG'd HTML contains the FILTERED value (ADA LOVELACE)
#   2. it does NOT contain the literal pipe text `| uppercase`
#   3. `capitalize` — declared in stdlib — also works (Ada Lovelace)
#
# NB on `capitalize`: stdlib's %primitive uppercases only the FIRST character of
# the whole string ("ada lovelace" -> "Ada lovelace"), not each word. The gate
# asserts what stdlib DEFINES, not what the name might suggest — the point of
# this fix is that build and runtime agree on one definition, whatever it says.
#
# POLARITY: RED before PLAN-138, GREEN after.
# Do NOT make this pass by teaching fill_holes a Rust match on filter names —
# that is a fourth implementation and the project forbids Rust re-implementing
# what stdlib defines in Spacetime.
#
# Run:   tests/repro/gh-11-ssg-filters/check-contract.sh
# Binary: overridable via SPACETIME_BIN env var.
# ============================================================================
set -u

BIN="${SPACETIME_BIN:-}"
if [ -z "$BIN" ]; then
  # Prefer the freshly BUILT binary over any `spacetime` on PATH: a stale system
  # install silently answers for the working tree (observed 2026-08-06).
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

OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT
"$BIN" build "$DIR" -o "$OUT" >/dev/null 2>&1
HTML="$OUT/index.html"
[ -f "$HTML" ] || { echo "FAIL: no index.html produced"; exit 1; }

fail=0

# 1) the filter must be APPLIED, not printed.
if grep -q '| uppercase' "$HTML"; then
  echo "FAIL: the literal pipe text '| uppercase' leaked into the SSG'd HTML."
  grep -oE '<li class="up">[^<]*</li>' "$HTML" | head -2
  fail=1
fi

# 2) the uppercased value must be present.
if ! grep -q 'ADA LOVELACE' "$HTML"; then
  echo "FAIL: '| uppercase' was not applied at build time (no 'ADA LOVELACE')."
  grep -oE '<li class="up">.*</li>' "$HTML" | head -2
  fail=1
fi

# 3) capitalize is DECLARED in stdlib, so it must work too. This is the case
#    that proves the fix reads the declared registry rather than a hand-picked
#    subset — it currently works at neither build time nor runtime.
if ! grep -q 'Ada lovelace' "$HTML"; then
  echo "FAIL: '| capitalize' (declared in stdlib) was not applied at build time."
  grep -oE '<li class="cap">.*</li>' "$HTML" | head -2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo
  echo "GH-11 is live. See PLAN-138 — the fix is one filter table, not a fourth."
  exit 1
fi
echo "PASS: SSG unroll applies declared stdlib filters (GH-11 closed)."
exit 0
