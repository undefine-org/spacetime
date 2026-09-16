#!/usr/bin/env bash
# ============================================================================
# W2 — an unknown directive inside a SELECTOR SCOPE must be diagnosed.
#
# A top-level unknown directive already warns (W0714, BUG-133):
#     warning[W0714]: `@notarealdirective` is not a known directive and was ignored
# The same spelling inside a selector scope is SILENT — and a selector scope is
# where directives actually live, so the check that matters is the one missing.
#
#     .card { @notarealdirective(x: 1) }   ->  ✓ All 1 file(s) passed
#
# That is the GH-13 / GH-22 experience exactly: forget an import, get a blank
# page and a green build. GH-13 established the rule for unknown PRIMITIVES
# (E0956 is a hard error); this extends it to the author-facing surface.
#
# It also blocks honest verification of anything else: a demo whose `check`
# cannot fail is verified by a command that cannot fail.
#
# DESIRED CONTRACT:
#   - `check` on a scope-level unknown directive emits W0714 naming it
#   - the message names the directive so the author can find it
#   - a page using only REAL directives stays silent (no false positives)
#
# POLARITY: RED before the fix (the scoped case is silent), GREEN after.
#
# Run:   tests/repro/w2-unknown-scoped-directive/check-contract.sh
# Binary: overridable via SPACETIME_BIN env var.
# ============================================================================
set -u

BIN="${SPACETIME_BIN:-}"
if [ -z "$BIN" ]; then
  _here="$(cd "$(dirname "$0")" && pwd)"
  _root="$(cd "$_here/../../.." && pwd)"
  for cand in "$_root/target/debug/spacetime" "$HOME/.cache/cargo-target/debug/spacetime"; do
    [ -x "$cand" ] && BIN="$cand" && break
  done
fi
if [ -z "$BIN" ]; then
  if command -v spacetime >/dev/null 2>&1; then BIN="$(command -v spacetime)"; else
    echo "FATAL: cannot locate the spacetime binary (set SPACETIME_BIN)" >&2; exit 2; fi
fi
DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$DIR"; while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

fail=0
OUT="$("$BIN" check "$DIR/index.st" 2>&1)"

# 1) the unknown scoped directive must be diagnosed, and named.
if ! echo "$OUT" | grep -q 'notarealdirective'; then
  echo "FAIL: a scope-level unknown directive was NOT diagnosed."
  echo "      .card { @notarealdirective(x: 1) } compiled silently."
  echo "$OUT" | tail -3
  fail=1
fi

# 2) no false positives: a page of REAL directives must stay silent.
CLEAN="$(mktemp -d)/clean.st"
cat > "$CLEAN" <<'EOF'
@import "stdlib"
<div class="ok">hi</div>
.ok {
  @on &.visible(300ms) { opacity: 0 -> 1; }
}
EOF
CLEAN_OUT="$("$BIN" check "$CLEAN" 2>&1)"
if echo "$CLEAN_OUT" | grep -qE 'W0714|not a known directive'; then
  echo "FAIL: a page using only REAL directives was flagged (false positive)."
  echo "$CLEAN_OUT" | grep -E 'W0714|not a known directive' | head -3
  fail=1
fi

[ "$fail" -ne 0 ] && { echo; echo "W2 is live. An unresolvable construct must diagnose, never vanish."; exit 1; }
echo "PASS: an unknown scoped directive is diagnosed; real directives stay silent."
exit 0
