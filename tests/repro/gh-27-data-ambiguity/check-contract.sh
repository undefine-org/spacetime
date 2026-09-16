#!/usr/bin/env bash
# ============================================================================
# GH-27 — contract gate: canonical `@data inline` kind forms MUST NOT fire E0923.
#
# Issue: `@data inline $events : [];` and `@data inline $filterCategory : "all";`
# are literal-aware (parse selects data-inline and stores it in FormMatch), yet
# `check --verbose` still reported warning[E0923] ambiguous dispatch (data-inline
# vs data-derive, both score 13) on both. That is a FALSE POSITIVE on
# unambiguous canonical kind forms. Also: the second E0923 anchored wrong (6:27).
#
# Fix: `analyze_dispatch` honors parse's decision — when `matched_macro` is set
# (the literal kind-word settled it, e.g. `data-inline`), it skips the capture
# re-score that produced the false tie. E0923 now fires only when parse genuinely
# did NOT decide.
#
# WEAKNESS THIS GATE PREVIOUSLY HAD (W6 review): it was NEGATIVE-ONLY — it
# grepped for the ABSENCE of E0923 and exited GREEN on any output without it.
# That is trivially true in exactly the failure mode that matters: if `check
# --verbose` failed to PROCESS the @data inline forms at all (a parse abort, a
# dropped file, a broken import), the forms never reach the ambiguity lint and NO
# E0923 appears — the gate passes while the fix is dead. A negative with no proof
# the harness saw the thing is vacuous.
#
# STRONGER NOW: the negative is PAIRED with a POSITIVE control proving the
# harness actually compiled the two canonical forms. `check --verbose` prints a
# "Data Sources" report; the gate requires BOTH `events` and `filterCategory` to
# be listed there. If the forms were skipped/dropped, the report lacks them → RED,
# regardless of E0923. Only when the harness demonstrably saw the forms AND
# emitted no E0923 does the gate pass — i.e. "these exact canonical forms compiled
# with no false ambiguity," not "no E0923 anywhere in some output."
#
# DESIRED CONTRACT: `check --verbose index.st` lists both canonical @data inline
# data sources (events, filterCategory) AND contains ZERO E0923.
#
# Run:   tests/repro/gh-27-data-ambiguity/check-contract.sh
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
# Run from the repo root: the binary resolves `@import "stdlib"` relative to
# CWD, not the file location.
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

OUT="$("$BIN" check --verbose "$DIR/index.st" 2>&1)"

# POSITIVE CONTROL: the harness MUST have compiled the two canonical @data
# inline forms — they appear as data sources in the compile report. If they were
# dropped or never reached the ambiguity lint, the E0923 absence below would be
# trivially true.
if ! echo "$OUT" | grep -q 'events: any' || ! echo "$OUT" | grep -q 'filterCategory: any'; then
  echo "FAIL: compile report does not list both canonical @data inline data sources (events, filterCategory) — the forms were not processed."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi

# NEGATIVE: no false E0923 on the canonical forms.
if echo "$OUT" | grep -q 'E0923'; then
  echo "FAIL: E0923 fired on canonical @data inline forms (GH-27 is LIVE)."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi

echo "PASS: canonical @data inline forms compiled (events, filterCategory listed) with no E0923."
exit 0
