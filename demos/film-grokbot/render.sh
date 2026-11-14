#!/usr/bin/env bash
# Deterministic native Spacetime render; no application JavaScript.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN=${SPACETIME_BIN:-$ROOT/result/bin/spacetime}
OUT=${1:-$ROOT/scratch/grokbot/spacetime-grokbot-1.0.mp4}
mkdir -p "$(dirname "$OUT")"
cd "$ROOT"
exec "$BIN" render demos/film-grokbot/ --width 1920 --height 1080 --fps 30 --out "$OUT"
