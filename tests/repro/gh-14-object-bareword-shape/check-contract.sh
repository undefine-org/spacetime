#!/usr/bin/env bash
# ============================================================================
# GH-14 — RED contract gate: `@object torusknot(size: 1.2)` is STILL silently
# defaulted (the bug is LIVE), and this gate flips GREEN when it is diagnosed.
#
# Issue: the doc-comment call style `@object torusknot(size: 1.2, ...)` — a
# leading bareword shape token — matches no %form (object.st declares the flat
# paren-arg list `@object(shape: $shape = "icosahedron", …)`), yet it is
# SILENTLY accepted with all defaults: `geomFor("icosahedron")` emitted from
# source that says `torusknot`, zero diagnostics.
#
# STATUS: still-live (re-filed). A first fix — extending the pre-ARG_LIST
# leftover check (form_compiler 2b) to forms with zero inline elements — WAS
# tried, but it also diagnosed `@value-change count(...)`, whose leading
# bareword is a DELIBERATE legacy drop (the 2026-07-26-on-cutover migration
# rewrite's `%drops ( _ )`). A blanket removal of the 2b gate broke that
# documented compat. The diagnosis needs a mechanism that RESPECTS a form's
# `%drops` (tracked in BUG-319 / the GH-14 docs-half follow-up).
#
# POLARITY: the bug is LIVE today, so this script FAILS (exit 1 → RED). It
# goes GREEN the moment `@object torusknot(...)` is diagnosed (E0946) while
# `@value-change count(...)` keeps working.
#
# Run:   tests/repro/gh-14-object-bareword-shape/check-contract.sh
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

OUT="$("$BIN" check "$DIR/index.st" 2>&1)"
N="$(echo "$OUT" | grep -c 'E0946')"
if [ "$N" -ge 1 ]; then
  echo "PASS: bareword/nested-body @object is diagnosed (E0946) — GH-14 consume-or-refuse half landed."
  exit 0
else
  echo "FAIL (RED, bug still live): @object torusknot(...) is silently defaulted — no E0946. GH-14 needs a %drops-respecting diagnosis (BUG-319)."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi
