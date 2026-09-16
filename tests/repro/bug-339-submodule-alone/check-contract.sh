#!/usr/bin/env bash
# ============================================================================
# BUG-339 — contract gate: importing a stdlib SUBMODULE alone MUST register the
# macros that submodule declares. The documented single-import spelling is
# exactly what `stdlib/<mod>/index.st`'s own doc comment promises, and what the
# 14 three_*/doc_* snapshot tests in src/compiler.rs write.
#
# Failure mode: `@import "stdlib/3d"` without also `@import "stdlib"` resolved
# to the EMBEDDED-stdlib dead end (`ResolvedImport::Stdlib`, which
# `resolve_imports` skips because MetaRegistry only loads `STDLIB_DIRS` — and
# those deliberately exclude the opt-in modules). So the submodule's `%macro`
# set never registered, and `@stage`/`@doc` fell through to a nonexistent
# primitive of the same name:
#
#   error[E0956]: unknown primitive: stage
#
# Same root as BUG-336 (`@script`) and BUG-338 (every literate `.st.md`, whose
# tangler synthesizes `@import "stdlib/md"`), and the same root as BUG-340
# (the opt-in modules were unreachable at all). One fix (filesystem-first
# stdlib resolution) closed all four.
#
# DESIRED CONTRACT (four legs, all must hold):
#   (1) `check` on three.st (`@import "stdlib/3d"` + `.s { @stage; }`) → compiles clean
#   (2) `check` on md.st   (`@import "stdlib/md"`   + `.d { @doc(content: …); }`) → clean
#   (3) positive control: control.st (no stdlib import) → clean, so a RED leg
#       (1)/(2) is genuinely a submodule-registration failure, not a broken check
#   (4) unresolvable.st → FAILS with a diagnostic that NAMES the module
#       (`Could not resolve import 'nonexistent-submodule-zz'`), proving the
#       gate's failure detection is live (a module that cannot resolve is
#       called out by name, not silently swallowed into a downstream E0956).
#
# POLARITY: today (bug live) legs (1) and (2) FAIL with E0956 "unknown
# primitive" and legs (3)/(4) behave as described. With the fix all four pass.
# This script exits 1 → RED when the bug is live.
#
# Run:   tests/repro/bug-339-submodule-alone/check-contract.sh
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

# Leg 1: @import "stdlib/3d" alone + @stage → clean.
THREE_OUT="$("$BIN" check "$DIR/three.st" 2>&1)"
if echo "$THREE_OUT" | grep -qE 'error\[E?[0-9]+\]|✗ 1 of 1 file\(s\) had errors'; then
  echo "FAIL leg1: @import \"stdlib/3d\" alone did NOT register @stage — a submodule "
  echo "          imported alone must be self-sufficient."
  echo "--- actual stdout (three.st) ---"
  echo "$THREE_OUT"
  fail=1
fi

# Leg 2: @import "stdlib/md" alone + @doc → clean.
MD_OUT="$("$BIN" check "$DIR/md.st" 2>&1)"
if echo "$MD_OUT" | grep -qE 'error\[E?[0-9]+\]|✗ 1 of 1 file\(s\) had errors'; then
  echo "FAIL leg2: @import \"stdlib/md\" alone did NOT register @doc — every literate "
  echo "          .st.md page is dead with it."
  echo "--- actual stdout (md.st) ---"
  echo "$MD_OUT"
  fail=1
fi

# Leg 3: positive control — no stdlib import must compile clean. Proves the
# harness can pass, so a RED leg1/leg2 is the bug, not a broken check.
CTRL_OUT="$("$BIN" check "$DIR/control.st" 2>&1)"
if echo "$CTRL_OUT" | grep -qE 'error\[E?[0-9]+\]|✗ 1 of 1 file\(s\) had errors'; then
  echo "FAIL leg3: control.st (no stdlib import) did NOT compile clean — the harness is "
  echo "          broken, not the submodule path."
  echo "--- actual stdout (control.st) ---"
  echo "$CTRL_OUT"
  fail=1
fi

# Leg 4: unresolvable module → the failure must NAME the module (not a silent
# downstream E0956). Proves an unresolvable import is diagnosed by name.
UNRES_OUT="$("$BIN" check "$DIR/unresolvable.st" 2>&1)"
if ! echo "$UNRES_OUT" | grep -qE "Could not resolve import 'nonexistent-submodule-zz'"; then
  echo "FAIL leg4: unresolvable module did NOT produce a diagnostic naming the module."
  echo "--- actual stdout (unresolvable.st) ---"
  echo "$UNRES_OUT"
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "PASS: 3d+@stage alone compiles, md+@doc alone compiles, control clean,"
  echo "      unresolvable names its module."
  exit 0
fi
echo "BUG-339 contract gate FAILED."
exit 1
