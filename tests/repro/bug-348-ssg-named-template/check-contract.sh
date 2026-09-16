#!/usr/bin/env bash
#
# BUG-348 — SSG `@each` must unroll a NAMED `&template` reference, not only an
# inline `@template { … }` body.
#
# `@each` has two body grammars:
#
#     @each($items as $i) { @template { <li>`$i`</li> } }   # inline html_block
#     @each($items as $i) { &row($i); }                     # template_invocation+
#
# Only the first was unrolled. The second produced an EMPTY container with no
# error and exit 0, so extracting a row into a named template — the refactor the
# language teaches for reuse — silently deleted the server-rendered HTML. The
# runtime bundle stayed complete (the data and the template were both in
# spacetime.js), so the loss showed up only as an empty first paint, nothing for
# crawlers, and a layout shift on hydrate.
#
# That is invisible to any assertion over emitted strings: the bundle is FINE.
# Only building a page and reading index.html catches it. Hence a contract gate.
#
# Legs:
#   1. inline  — the spelling that always worked (positive control; if this
#                fails the harness is wrong, not the feature)
#   2. named   — the BUG-348 case: same data, `&row($i);` body
#   3. parity  — inline and named must produce the SAME rows; drift between the
#                two spellings is the actual defect class (cf. GH-11, where SSG
#                and runtime disagreed about filter pipes)
#   4. typed   — objects + field holes (`$s.code` / `$s.label`) through a named
#                template, since field access is a separate substitution path
#   5. control — a page with the template DECLARED but never invoked emits no
#                rows, proving the assertions discriminate rather than matching
#                something incidental in the page
#
set -uo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$DIR/../../.." && pwd)"

# Run from the repo root, like every sibling gate: the working directory decides
# which stdlib the compiler loads, and building from inside the fixture folder
# picks up a different one (it fails with "Missing required parameter 'match'
# in bind: derive-match" on a page that builds fine from the root).
cd "$REPO_ROOT" || exit 2
if [ -n "${SPACETIME_BIN:-}" ]; then
  BIN="$SPACETIME_BIN"
elif [ -x "$REPO_ROOT/target/debug/spacetime" ]; then
  BIN="$REPO_ROOT/target/debug/spacetime"
elif [ -x "$HOME/.cache/cargo-target/debug/spacetime" ]; then
  BIN="$HOME/.cache/cargo-target/debug/spacetime"
else
  BIN="$(command -v spacetime)"
fi
[ -x "$BIN" ] || { echo "FAIL: no spacetime binary found"; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Build one single-page site per fixture and echo its index.html.
build() {
  local fixture="$1" dir="$TMP/$1"
  mkdir -p "$dir"
  cp "$DIR/$fixture.st" "$dir/index.st"
  if ! "$BIN" build "$dir" -o "$dir/out" >"$TMP/$fixture.log" 2>&1; then
    echo "FAIL: building $fixture.st did not succeed." >&2
    sed 's/^/      /' "$TMP/$fixture.log" >&2
    exit 1
  fi
  cat "$dir/out/index.html"
}

# Rows as plain text, attributes stripped: the contract is WHICH rows render,
# not the data-st-id / data-st-origin bookkeeping the compiler adds.
rows_of() {
  grep -oE '<li[^>]*>[^<]*</li>' <<<"$1" | sed -E 's/<li[^>]*>//; s#</li>##'
}

INLINE_HTML="$(build inline)"
NAMED_HTML="$(build named)"
TYPED_HTML="$(build typed)"
CONTROL_HTML="$(build control)"

INLINE_ROWS="$(rows_of "$INLINE_HTML")"
NAMED_ROWS="$(rows_of "$NAMED_HTML")"

# --- leg 1: inline body (positive control) ----------------------------------
if [ "$(wc -l <<<"$INLINE_ROWS")" -ne 2 ] || [ "$INLINE_ROWS" != "$(printf 'ab\ncd')" ]; then
  echo "FAIL: the inline @template body did not unroll to rows ab,cd."
  echo "      got: $(tr '\n' ',' <<<"$INLINE_ROWS")"
  echo "      This is the spelling that always worked — the harness is wrong,"
  echo "      not the feature."
  exit 1
fi

# --- leg 2: named template reference (the BUG-348 case) ---------------------
if [ -z "$NAMED_ROWS" ]; then
  echo "FAIL: a named &template reference unrolled to NOTHING (BUG-348)."
  echo "      .l { @each(\$items as \$i) { &row(\$i); } } emitted an empty container:"
  grep -oE '<ul class="l">[^!]*</ul>' <<<"$NAMED_HTML" | head -1 | sed 's/^/      /'
  echo "      The runtime bundle is complete, so this is invisible except as an"
  echo "      empty first paint. Only building the page catches it."
  exit 1
fi

# --- leg 3: the two spellings must agree ------------------------------------
if [ "$NAMED_ROWS" != "$INLINE_ROWS" ]; then
  echo "FAIL: inline and named bodies disagree — the two spellings have drifted."
  echo "      inline: $(tr '\n' ',' <<<"$INLINE_ROWS")"
  echo "      named:  $(tr '\n' ',' <<<"$NAMED_ROWS")"
  exit 1
fi

# --- leg 4: field holes through a named template ----------------------------
for expected in 'data-code="S"' 'data-code="M"' '>Small<' '>Medium<'; do
  if ! grep -qF "$expected" <<<"$TYPED_HTML"; then
    echo "FAIL: a typed row through a named template lost '$expected'."
    echo "      Field holes (\$s.code / \$s.label) are a separate substitution"
    echo "      path from a bare \$i and must survive the same unroll."
    grep -oE '<ul class="size-list">.*</ul>' <<<"$TYPED_HTML" | head -1 | sed 's/^/      /'
    exit 1
  fi
done

# --- leg 5: negative control ------------------------------------------------
# The template is DECLARED but never invoked, so no row may appear. Without this
# the legs above could pass by matching markup that had nothing to do with @each.
if [ -n "$(rows_of "$CONTROL_HTML")" ]; then
  echo "FAIL: a page that declares &row but never invokes it still emitted rows."
  echo "      got: $(rows_of "$CONTROL_HTML" | tr '\n' ',')"
  echo "      The assertions above are matching something incidental."
  exit 1
fi

echo "PASS: @each unrolls a named &template reference (BUG-348)."
echo "      inline and named agree on rows ab,cd; typed field holes survive;"
echo "      a declared-but-uninvoked template emits nothing."
