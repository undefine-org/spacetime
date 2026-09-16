#!/usr/bin/env bash
# ============================================================================
# GH-37 — contract gate: bare `@data signal $cursor(x number, y number)` MUST
# be DIAGNOSED (E0946), not silently emit `callParams = []`.
#
# Issue: `param_list`'s grammar is `( item )*` — a `*` repetition that LEGALLY
# matches zero items. So `x number, y number` (bare names, where the grammar
# only accepts `$binding`/`&element`, typed form `$x number`) matched ZERO
# items, the paren-group content was discarded unexamined, and the match was
# accepted with defaults — emitting `callParams = []` and silently changing the
# wire payload shape at runtime.
#
# Fix (W3 consume-or-refuse): a CUSTOM capture that consumes ZERO tokens from a
# non-empty bounded region has not consumed its declared extent → the match
# fails with E0946 (form_compiler.rs Required branch, `consumed == 0` guard).
#
# DESIRED CONTRACT: `check index.st` prints E0946 naming `param_list` for the
# `$cursor(x number, y number)` line (index.st:9) and NO error at the canonical
# `$add($text string)` control (index.st:17).
#
# POLARITY: the fix is LIVE today, so this script PASSES (exit 0 → GREEN). A
# regression back to silent elision (diagnostics dropped, `callParams = []`)
# goes RED.
#
# Why check-contract.sh and not a .test.st: a .test.st carrying the bare form
# FAILS to compile, which the runner reports as a FAILED file — the signal
# would be INVERTED (test passes exactly when the diagnostics regress to
# silence). A stdout assertion of the diagnostic is the clean non-inverted
# polarity; the canonical `$add` control is kept as a real behavioral .test.st.
#
# Run:   tests/repro/gh-37-data-signal-params/check-contract.sh
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

OUT="$("$BIN" check "$DIR/index.st" 2>&1)"

# 1. The bare `$cursor(x number, y number)` MUST surface an E0946.
N_CURSOR="$(echo "$OUT" | grep -c 'E0946.*param_list')"
# 2. The canonical `$add($text string)` control MUST NOT error.
N_ADD_ERR="$(echo "$OUT" | grep -E 'E0946|^error' | grep -c 'gh-37-data-signal-params/index.st:1[0-9]' || true)"
N_ADD_ERR="$(echo "$OUT" | grep -c 'index.st:17')"

if [ "$N_CURSOR" -ge 1 ] && [ "$N_ADD_ERR" -eq 0 ]; then
  echo "PASS: bare \`\$cursor(x number, y number)\` diagnosed (E0946·param_list); canonical \`\$add(\$text string)\` stays clean."
  exit 0
else
  echo "FAIL: want >=1 E0946·param_list and 0 errors at the \$add control; got cursor=$N_CURSOR add_errors=$N_ADD_ERR."
  echo "--- actual stdout ---"
  echo "$OUT"
  exit 1
fi
