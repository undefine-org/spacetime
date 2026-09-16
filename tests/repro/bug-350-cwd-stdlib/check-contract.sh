#!/usr/bin/env bash
#
# BUG-350 — the caller's working directory must NOT decide which stdlib loads.
#
# `STDLIB_DIRS` are relative (`stdlib/primitives`, `stdlib/macros`, …). The
# incremental cache mapped them with a bare `PathBuf::from`, so they resolved
# against the process's cwd:
#
#   cd <repo root>  && spacetime build /abs/site   # dirs exist -> ON-DISK stdlib
#   cd <repo>/src   && spacetime build /abs/site   # dirs miss  -> EMBEDDED stdlib
#
# Same site (absolute path), same binary — different stdlib. The embedded copy is
# frozen at the last `cargo build`, so a page that builds from the root failed
# from a subdirectory with a diagnostic about an unrelated internal primitive:
#
#   Missing required parameter 'match' in bind: derive-match
#
# Nothing named the real problem (the stdlib that was chosen), which is what made
# it expensive to find. Same class as BUG-227, one layer down.
#
# The gate asserts the contract that actually matters: building the SAME site
# from several working directories must produce BYTE-IDENTICAL output — not
# merely "both exit 0", which a pair of equally-wrong builds would satisfy.
#
set -uo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$DIR/../../.." && pwd)"

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

# The site lives INSIDE the checkout and is addressed by ABSOLUTE path, so the
# only thing varying across the runs below is the caller's cwd.
SITE="$TMP/site"
mkdir -p "$SITE"
cp "$DIR/site.st" "$SITE/index.st"

# Directories to build from. `docs/` is included deliberately: it contains its own
# `stdlib/` subdir, so it took a DIFFERENT resolution path and accidentally passed
# while its siblings failed — the asymmetry that located the bug.
CWDS=("$REPO_ROOT" "$REPO_ROOT/src" "$REPO_ROOT/tests" "$REPO_ROOT/docs" "$DIR")

first_html=""
first_js=""
first_label=""

for cwd in "${CWDS[@]}"; do
  label="${cwd#"$REPO_ROOT"}"
  label="${label:-/(root)}"
  out="$TMP/out$(echo "$label" | tr -c 'a-zA-Z0-9' '_')"

  if ! (cd "$cwd" && "$BIN" build "$SITE" -o "$out") >"$TMP/build.log" 2>&1; then
    echo "FAIL: building from '$label' did not succeed."
    echo "      The SAME absolute site builds fine from the repo root, so the cwd"
    echo "      changed which stdlib was loaded (BUG-350)."
    sed 's/^/      /' "$TMP/build.log"
    exit 1
  fi

  html="$(md5sum "$out/index.html" | cut -d' ' -f1)"
  js="$(md5sum "$out/spacetime.js" | cut -d' ' -f1)"

  if [ -z "$first_html" ]; then
    first_html="$html"; first_js="$js"; first_label="$label"
    continue
  fi

  if [ "$html" != "$first_html" ] || [ "$js" != "$first_js" ]; then
    echo "FAIL: output differs by working directory (BUG-350)."
    echo "      from '$first_label': index.html=$first_html spacetime.js=$first_js"
    echo "      from '$label': index.html=$html spacetime.js=$js"
    echo "      Both builds succeeded, so an exit-code-only check would have passed"
    echo "      them — the bytes are the contract."
    exit 1
  fi
done

# Positive control: the assertion above is only meaningful if the fixture really
# exercises the stdlib. A page that used nothing from it would hash identically
# under a broken AND a working resolver. `derive-match` (via @each + a named
# template) is precisely what the embedded-fallback path got wrong.
if ! grep -q 'ready' "$TMP/out_(root)/index.html" 2>/dev/null; then
  found=0
  for d in "$TMP"/out*; do
    grep -q 'ready' "$d/index.html" 2>/dev/null && found=1 && break
  done
  if [ "$found" -eq 0 ]; then
    echo "FAIL: no build produced the SSG row, so the fixture never exercised the"
    echo "      stdlib and the byte-comparison above proves nothing."
    exit 1
  fi
fi

echo "PASS: the same site builds byte-identically from ${#CWDS[@]} working"
echo "      directories (BUG-350); the fixture's SSG row proves the stdlib ran."
