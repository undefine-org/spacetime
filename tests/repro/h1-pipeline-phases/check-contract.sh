#!/usr/bin/env bash
# ============================================================================
# PLAN-143 H1 — replace the hollow pipeline-probe tests with build-a-page gates.
#
# The 3 hollow Rust tests (src/pipeline/integration_tests.rs:
# test_full_pipeline_data_fetch / _element_ref / _mixed_phases) fed the string
# IN to `probe_context(&["data-fetch"])` (a stub primitive emitting a marker
# with the name) and asserted `output.js.contains("data-fetch")` — asserting
# their own input echoes back. Proven hollow: stubbing the probe emit body to
# a dead comment left all 5 full_pipeline tests GREEN. A dead bundle and a
# dead feature were indistinguishable at that assertion layer.
#
# This gate BUILDS a REAL page per phase and asserts the phase's ACTUAL
# emitted output reaches the bundle — the honest behavioral contract:
#
#   LEG 1 — data-fetch:   `@data fetch $users : "/api/users";` must put the
#                         endpoint URL and the generated data-source config
#                         in the bundle. Positive control (no directive) must
#                         NOT — proves the assertion discriminates.
#   LEG 2 — element-ref:  a DECLARED `&b .some-class;` reference lowers to the
#                         runtime call `ST.ref("b")` (BUG-333); an UNDECLARED
#                         `&$b` must DIAGNOSE E0965 naming the ref, never emit
#                         a dead bundle (E0952). Positive control (no ref) must
#                         NOT contain ST.ref.
#   LEG 3 — mixed phases: ONE page combining data-fetch + element-ref +
#                         local-state + an event handler emits all four, in a
#                         single bundle that PARSES. BUG-328 was a bundle that
#                         did not parse (a duplicate `const el`); `node --check`
#                         on the emitted JS is the syntax gate — demonstrated
#                         discriminating by injecting a duplicate `const el`.
#
# POLARITY: features are LIVE today -> PASS (exit 0 -> GREEN). Any leg whose
# emitted output is missing, a control that still emits, an undeclared ref
# that builds clean, or a bundle that does not parse -> RED.
#
# Run:   tests/repro/h1-pipeline-phases/check-contract.sh
# Binary: overridable via SPACETIME_BIN env var.
# ============================================================================
set -u

BIN="${SPACETIME_BIN:-}"
if [ -z "$BIN" ]; then
  # Prefer the freshly BUILT binary over any `spacetime` on PATH: a stale
  # install silently answers for the working tree and a gate then reports on
  # code nobody is editing.
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

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() { echo "FAIL: $*"; exit 1; }

build_ok() { # build_ok <file> <outdir> -> returns exit code, prints log path
  local f="$1" out="$2"
  "$BIN" build "$f" -o "$out" >"$TMP/$(basename "$f").log" 2>&1
  return $?
}

# ---------------------------------------------------------------------------
# LEG 1 — data-fetch: the endpoint URL + generated data-source config must be
# in the bundle; a page WITHOUT the directive must not.
# ---------------------------------------------------------------------------
build_ok "$DIR/fetch.st" "$TMP/fetch" || fail "fetch.st failed to build (data-fetch feature broken?)"
FETCH_JS="$TMP/fetch/spacetime.js"
[ -f "$FETCH_JS" ] || fail "fetch.st produced no spacetime.js"
if ! grep -q '/api/users' "$FETCH_JS"; then
  fail "fetch.st bundle lacks the endpoint URL '/api/users' — @data fetch did not reach the emitted bundle"
fi
if ! grep -q '__stDataSource' "$FETCH_JS"; then
  fail "fetch.st bundle lacks the data-source config call (__stDataSource) — the fetch primitive did not emit"
fi
if ! grep -q '"users"' "$FETCH_JS"; then
  fail "fetch.st bundle lacks the binding name 'users' — the generated data binding did not emit"
fi

# Positive control: identical page, no @data fetch -> must NOT emit any of it.
build_ok "$DIR/fetch-control.st" "$TMP/fc" || fail "fetch-control.st failed to build (control broken?)"
FC_JS="$TMP/fc/spacetime.js"
for needle in '/api/users' '__stDataSource' '"users"'; do
  if grep -qF -- "$needle" "$FC_JS"; then
    fail "fetch-control.st (no @data fetch) still emits '$needle' — the data-fetch assertion does not discriminate"
  fi
done

# ---------------------------------------------------------------------------
# LEG 2 — element-ref: a DECLARED ref lowers to ST.ref("name"); an UNDECLARED
# ref must DIAGNOSE (E0965) rather than emit a dead bundle (E0952). Control
# without any ref must not contain ST.ref.
# ---------------------------------------------------------------------------
build_ok "$DIR/element-ref.st" "$TMP/er" || fail "element-ref.st failed to build (declared ref broken?)"
ER_JS="$TMP/er/spacetime.js"
[ -f "$ER_JS" ] || fail "element-ref.st produced no spacetime.js"
if ! grep -q 'ST.ref("b")' "$ER_JS"; then
  fail "element-ref.st bundle lacks ST.ref(\"b\") — a DECLARED element reference did not lower to its runtime call"
fi

# Negative: an UNDECLARED `&$b` must fail with E0965 naming the ref, and must
# NEVER reach the E0952 dead-bundle check (BUG-333).
NEG_OUT="$("$BIN" build "$DIR/element-ref-undeclared.st" -o "$TMP/eru" 2>&1)"
NEG_EXIT=$?
if [ $NEG_EXIT -eq 0 ]; then
  fail "element-ref-undeclared.st (undeclared &\$b) built successfully — the undeclared reference was silently accepted instead of diagnosed"
fi
if echo "$NEG_OUT" | grep -q 'E0952'; then
  fail "element-ref-undeclared.st hit the E0952 dead-bundle check instead of a diagnostic naming the undeclared reference (BUG-333 regression)"
fi
if ! echo "$NEG_OUT" | grep -q 'E0965'; then
  fail "element-ref-undeclared.st diagnostic does not carry E0965 (undeclared element reference)"
fi
if ! echo "$NEG_OUT" | grep -q "'b'"; then
  fail "element-ref-undeclared.st diagnostic does not NAME the undeclared reference 'b'"
fi

# Positive control: no element reference -> no ST.ref.
build_ok "$DIR/element-ref-control.st" "$TMP/erc" || fail "element-ref-control.st failed to build (control broken?)"
ERC_JS="$TMP/erc/spacetime.js"
if grep -q 'ST.ref' "$ERC_JS"; then
  fail "element-ref-control.st (no element reference) still emits ST.ref — the element-ref assertion does not discriminate"
fi

# ---------------------------------------------------------------------------
# LEG 3 — mixed phases: data-fetch + element-ref + local-state + event handler
# all reach ONE bundle, which PARSES (node --check). BUG-328 was a bundle that
# did not parse (a duplicate `const el`) — the syntax check is the highest-
# value leg.
# ---------------------------------------------------------------------------
if ! command -v node >/dev/null 2>&1; then
  echo "SKIP: node not available; the emitted-JS syntax check (the BUG-328 gate) cannot run."
  echo "      Install node to enable the parse-validity leg."
  exit 3
fi
build_ok "$DIR/mixed.st" "$TMP/mx" || fail "mixed.st failed to build (a combined page broken?)"
MX_JS="$TMP/mx/spacetime.js"
[ -f "$MX_JS" ] || fail "mixed.st produced no spacetime.js"
# all four phases must emit in the SAME bundle
grep -q '/api/users' "$MX_JS"       || fail "mixed bundle lacks data-fetch URL (/api/users)"
grep -q '__stDataSource' "$MX_JS"   || fail "mixed bundle lacks data-fetch config (__stDataSource)"
grep -q 'ST.ref("b")' "$MX_JS"      || fail "mixed bundle lacks element-ref lowering (ST.ref(\"b\"))"
grep -q '"cartOpen"' "$MX_JS"       || fail "mixed bundle lacks local-state name (cartOpen)"
grep -q "registerSelectorInit('.cart-toggle'" "$MX_JS" || fail "mixed bundle lacks the event-handler wiring (registerSelectorInit for .cart-toggle)"

# Event-handler control: same markup, NO @on directive -> no selector-init wiring.
build_ok "$DIR/handler-control.st" "$TMP/hc" || fail "handler-control.st failed to build (control broken?)"
HC_JS="$TMP/hc/spacetime.js"
if grep -q "registerSelectorInit('.cart-toggle'" "$HC_JS"; then
  fail "handler-control.st (no @on handler) still emits registerSelectorInit for .cart-toggle — the event-handler assertion does not discriminate"
fi
# the emitted JS must PARSE — the BUG-328 gate
if ! node --check "$MX_JS" >"$TMP/nodecheck.log" 2>&1; then
  fail "mixed bundle does not PARSE under node --check (BUG-328 class: a dead/unparsable bundle):"
  cat "$TMP/nodecheck.log"
fi

# Demonstrate the syntax check DISCRIMINATES the BUG-328 shape: inject a
# duplicate `const el` into the same scope of an otherwise-valid bundle; the
# check must now fail. If it somehow still passes, the check is not catching
# the class it exists for.
DUPCONST="$TMP/dupconst.js"
awk '
  { print }
  /const el = / && !seen { seen=1; print "            const el = window.ST.refs?.[refName];" }
' "$MX_JS" > "$DUPCONST"
if node --check "$DUPCONST" >/dev/null 2>&1; then
  fail "BUG-328 demonstration: node --check did NOT flag a duplicate 'const el' bundle — the syntax gate is not discriminating the dead-bundle class"
fi

echo "PASS: build-a-page gate proves all three phases reach the emitted bundle:"
echo "  data-fetch   -> URL '/api/users' + __stDataSource config + binding 'users'"
echo "                 (control page emits none of them)"
echo "  element-ref  -> declared &b lowers to ST.ref(\"b\"); undeclared &b diagnoses E0965"
echo "                 (control page emits no ST.ref; undeclared never reaches E0952)"
echo "  mixed phases -> data-fetch + element-ref + local-state + event handler"
echo "                 in one bundle that parses under node --check; a duplicate"
echo "                 'const el' (BUG-328 shape) is caught by that check."
exit 0
