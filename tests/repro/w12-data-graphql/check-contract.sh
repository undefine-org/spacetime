#!/usr/bin/env bash
# ============================================================================
# W1.2 (PLAN-141) — `@data graphql` is a WRITABLE surface.
#
# The `graphql` primitive (stdlib/primitives/fetch.st:149) shipped complete and
# tested with NO way to write it: no `%macro` bound it, so the capability was in
# every bundle and unreachable from a page. It surfaced in the G1 reachability
# report, which is the report's point — an unreachable primitive is capability
# the language BUILT and never exposed.
#
# The surface is an ARM of the existing data-kind family, not a new concept:
#
#     @data fetch   $posts : "/api/posts";
#     @data graphql $posts : "/graphql", query: "{ posts { id title } }";
#
# `$posts`, `$posts_loading`, `$posts_error` and `$posts_refetch` mean the same
# thing in both, so knowing the fetch arm tells you this one — the "learnable by
# pattern" rule, load-bearing.
#
# DESIRED CONTRACT:
#   1. it compiles with no errors
#   2. the endpoint and the query text both reach the bundle (it is not merely
#      parsed and dropped — the GH-14 failure mode)
#   3. all four bindings emit, so the shape really does match `@data fetch`
#   4. POSITIVE CONTROL: the sibling `@data fetch` still compiles, proving this
#      gate can tell a working data kind from a broken one
#
# Run:   tests/repro/w12-data-graphql/check-contract.sh
# Binary: overridable via SPACETIME_BIN.
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
if ! echo "$OUT" | grep -q 'All 1 file(s) passed'; then
  echo "FAIL: @data graphql does not compile."; echo "$OUT" | tail -4; fail=1
fi

# Build into a FRESH dir so a stale artifact cannot answer for a regression.
BUILD="$(mktemp -d)"; trap 'rm -rf "$BUILD"' EXIT
"$BIN" build "$DIR/index.st" -o "$BUILD" >/dev/null 2>&1
JS="$BUILD/spacetime.js"
if [ ! -f "$JS" ]; then
  echo "FAIL: no bundle produced."; exit 1
fi

# The endpoint AND the query text must reach the bundle: a form that parses and
# drops its captures is exactly the GH-14 silent-default failure.
for token in 'gqlendpoint' 'posts { id title }'; do
  grep -qF "$token" "$JS" || { echo "FAIL: '$token' never reached the bundle."; fail=1; }
done

# The full binding set, so the shape genuinely matches @data fetch.
for b in posts_loading posts_error posts_refetch; do
  grep -q "$b" "$JS" || { echo "FAIL: binding '$b' missing — the shape does not match @data fetch."; fail=1; }
done

# POSITIVE CONTROL: the sibling arm must still compile, proving this gate can
# tell a working data kind from a broken one. Uses `@data inline` rather than
# `@data fetch` because a fetch source is RESOLVED AS A LOCAL FILE at build time
# (E0504 "Data source file not found") — a control that needs a fixture on disk
# would fail for reasons unrelated to the family's health, which is exactly the
# spurious-failure mirror of a spuriously-passing gate.
CTRL="$(mktemp -d)/ctrl.st"
printf '@import "stdlib"\n@data inline $items : ["a", "b"];\n<ul class="l"></ul>\n.l { @each($items as $i) { <li>`$i`</li> } }\n' > "$CTRL"
"$BIN" check "$CTRL" >/dev/null 2>&1 || { echo "FAIL: control — the sibling @data inline no longer compiles."; fail=1; }

[ "$fail" -ne 0 ] && { echo; echo "W1.2: @data graphql is not a working surface."; exit 1; }
echo "PASS: @data graphql compiles, emits, and matches the @data fetch shape."
exit 0
