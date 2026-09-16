#!/usr/bin/env bash
# ============================================================================
# GH-21 — regression gate: a `.st`-only entry sets the page head (title, html
# lang, meta, remote fonts) with NO authored index.html, and an authored
# index.html's override of the .st body is DIAGNOSED.
#
# Issue: `render_page_shell` hard-coded `<html lang="en">` and `<title>Spacetime</title>`,
# so a full-Spacetime page could not set lang / title / meta / remote fonts. And an
# authored index.html won over the .st entry's body markup VERBATIM and EXCLUSIVELY —
# a silent escape hatch that turned a missing feature into data loss.
#
# Fix (PLAN-137 W7 #21 / PLAN-035):
#   - file-scope `<title>`/`<meta>`/`<link>` are hoisted into `<head>` (pre-existing);
#   - an authored top-level `<html lang="fr">` wrapper sets the generated `<html lang>`
#     (the shell owns the real `<html>`, so the wrapper's lang feeds it and the wrapper
#     is stripped) — src/html/treesink.rs re-emits the lang wrapper, page_shell.rs reads it;
#   - an authored index.html that shadows an index.st with body markup emits a warning
#     (src/export/emit.rs).
#
# DESIRED CONTRACT:
#   - `build tests/repro/gh-21-page-head` emits dist/index.html with `<html lang="fr">`,
#     `<title>Accueil — Backdesk</title>`, the meta description, the remote-font
#     preconnect links, and the body markup (no authored index.html needed).
#   - a site with BOTH index.st (with body markup) and an authored index.html emits a
#     warning naming the override.
#
# Run:   tests/repro/gh-21-page-head/check-contract.sh
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
ROOT="$DIR"
while [ ! -d "$ROOT/stdlib" ] && [ "$ROOT" != "/" ]; do ROOT="$(dirname "$ROOT")"; done
cd "$ROOT" || exit 2

# 1) .st-only build produces the head output with NO authored index.html.
OUT_DIR="$DIR/dist"
rm -rf "$OUT_DIR"
BUILD_OUT="$("$BIN" build "$DIR" 2>&1)"
PAGE="$OUT_DIR/index.html"
if [ ! -f "$PAGE" ]; then
  echo "FAIL: no $PAGE synthesized (GH-21)."
  echo "$BUILD_OUT"
  exit 1
fi
check() {
  local desc="$1"; local pattern="$2"
  if ! grep -q -- "$pattern" "$PAGE"; then
    echo "FAIL: $desc — missing '$pattern' in $PAGE"
    echo "--- actual page ---"
    cat "$PAGE"
    exit 1
  fi
}
check "html lang"            '<html lang="fr">'
check "title"                '<title>Accueil — Backdesk</title>'
check "meta description"     'name="description" content="Réservation'
check "remote font preconnect" 'rel="preconnect" href="https://fonts.googleapis.com"'
check "body markup ships"    '<h1>Bienvenue chez Backdesk</h1>'

# 2) an authored index.html that shadows an index.st with body markup is DIAGNOSED.
TMP="$(mktemp -d)"
cp "$DIR/index.st" "$TMP/index.st"
echo '<!DOCTYPE html><html><head><title>Authored</title></head><body><h1>from authored html</h1></body></html>' > "$TMP/index.html"
OVERRIDE_OUT="$("$BIN" build "$TMP" 2>&1)"
if ! echo "$OVERRIDE_OUT" | grep -qi 'index.html.*overrides'; then
  echo "FAIL: an authored index.html shadowing index.st body markup did NOT warn."
  echo "--- actual build output ---"
  echo "$OVERRIDE_OUT"
  rm -rf "$TMP"
  exit 1
fi
rm -rf "$TMP"

echo "PASS: .st-only head output (lang/title/meta/fonts) synthesized; index.html override diagnosed."
grep -E '<html lang|  <title>|preconnect' "$PAGE" | head -4
exit 0
