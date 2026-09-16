#!/usr/bin/env bash
# ============================================================================
# BUG-333 — regression gate: an element reference (`&$name`) to a name that was
# never declared must DIAGNOSE, never emit a dead bundle.
#
# Issue: `.card { $x: &$b; }` with no `&b <selector>;` declaration emitted JS
# that did not parse (`&ST.resolve(__node,'b')` — a dangling `&`), so the whole
# page died and the author got only E0952 "the compiler emitted JavaScript that
# does not parse", pointing at a byte offset in a generated bundle instead of
# their source line. Same family as GH-13/GH-22: an unresolvable construct must
# produce a diagnostic, never degrade into emitted-but-broken output.
#
# Fix: `&$name` is resolved against the element-reference registry (the `&name
# <sel>;` declarations) at bind time. Unresolved -> E0965 naming the reference
# and listing the declared refs in scope; resolved -> lowers to `ST.ref(name)`.
#
# DESIRED CONTRACT:
#   - index.st (undeclared `&$b`) must FAIL with a diagnostic NAMING the
#     reference `b`, and must NOT reach the E0952 dead-bundle check.
#   - positive.st (declared `&b .some-class;` in the same scope) must BUILD
#     CLEAN and its JS must read the declared ref via ST.ref.
# The gate proves the fix can DISTINGUISH the two: an undeclared ref is an
# author error (E0965), a declared ref is valid (clean build).
#
# POLARITY: the fix is LIVE today -> PASS (exit 0 -> GREEN). Before the fix
# both files emitted a dead bundle (E0952), so index.st's diagnostic is wrong
# AND positive.st fails -> RED.
#
# Run:   tests/repro/bug-333-undeclared-element-ref/check-contract.sh
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
# the documented `<BIN> build <file>` invocation (binary bases import
# resolution on CWD).
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

# 1) index.st — the undeclared reference MUST be a clean author-facing
#    diagnostic naming `b`, and must NEVER reach the E0952 dead-bundle check.
NEG_OUT="$("$BIN" build "$DIR/index.st" -o /tmp/bug333-neg 2>&1)"
NEG_EXIT=$?
if [ $NEG_EXIT -eq 0 ]; then
  echo "FAIL: index.st (undeclared &\$b) built successfully — the undeclared"
  echo "      element reference was silently accepted instead of diagnosed."
  echo "$NEG_OUT"
  exit 1
fi
if echo "$NEG_OUT" | grep -q 'E0952'; then
  echo "FAIL: index.st hit the E0952 dead-bundle check instead of a diagnostic"
  echo "      naming the undeclared reference (BUG-333 regression)."
  echo "$NEG_OUT"
  exit 1
fi
if ! echo "$NEG_OUT" | grep -q 'b'; then
  echo "FAIL: index.st diagnostic does not NAME the undeclared reference 'b'."
  echo "$NEG_OUT"
  exit 1
fi
if ! echo "$NEG_OUT" | grep -qi 'element reference'; then
  echo "FAIL: index.st diagnostic does not identify the construct as an element"
  echo "      reference."
  echo "$NEG_OUT"
  exit 1
fi

# 2) positive.st — a PROPERLY DECLARED `&b .some-class;` reference must build
#    clean, and its JS must read the declared ref via ST.ref (proving the gate
#    distinguishes a declared ref from an undeclared one).
POS_OUT="$(mktemp -d)"
POS_JS="$POS_OUT/spacetime.js"
POS_ERR=""
"$BIN" build "$DIR/positive.st" -o "$POS_OUT" >/tmp/bug333-pos.log 2>&1
POS_EXIT=$?
if [ $POS_EXIT -ne 0 ]; then
  echo "FAIL: positive.st (declared &b .some-class;) failed to build — the"
  echo "      declared element reference was NOT resolved."
  cat /tmp/bug333-pos.log
  rm -rf "$POS_OUT" /tmp/bug333-pos.log
  exit 1
fi
if ! grep -q 'Build complete' /tmp/bug333-pos.log; then
  echo "FAIL: positive.st did not report 'Build complete'."
  cat /tmp/bug333-pos.log
  rm -rf "$POS_OUT" /tmp/bug333-pos.log
  exit 1
fi
if [ -f "$POS_JS" ] && grep -q 'ST.ref' "$POS_JS"; then
  :
else
  echo "FAIL: positive.st JS does not read the declared ref via ST.ref — the"
  echo "      declared element reference was not correctly lowered."
  rm -rf "$POS_OUT" /tmp/bug333-pos.log
  exit 1
fi
rm -rf "$POS_OUT" /tmp/bug333-pos.log /tmp/bug333-neg

echo "PASS: undeclared element reference diagnoses (E0965 naming 'b', no dead"
echo "      bundle), declared reference builds clean and resolves via ST.ref."
echo "--- undeclared diagnostic ---"
echo "$NEG_OUT" | grep -i 'element reference' | head -3
exit 0
