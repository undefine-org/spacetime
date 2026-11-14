#!/usr/bin/env bash
# Render comparable reference/reconstruction contact sheets. Video extraction
# and arrangement are QA artifacts, never substitutes for authored scene UI.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN=${SPACETIME_BIN:-$ROOT/result/bin/spacetime}
REF=${1:-$ROOT/scratch/grokbot/reference/grokbot-1.0.mp4}
OUT=${2:-$ROOT/scratch/grokbot/comparison}
mkdir -p "$OUT/reference" "$OUT/reconstruction"
cd "$ROOT"
TIMES=2,6,10.5,16,20,22,26,30,37,40,43.5,46,49,52,56,59,62,65,68.5,71,72.7,77
"$BIN" render demos/film-grokbot/ --stills "$TIMES" --out "$OUT/reconstruction/frame.png" --width 1920 --height 1080 --fps 30
IFS=, read -ra STAMPS <<< "$TIMES"
for stamp in "${STAMPS[@]}"; do
  ms=$(awk -v t="$stamp" 'BEGIN {printf "%d", t*1000}')
  ffmpeg -y -hide_banner -loglevel error -ss "$stamp" -i "$REF" -frames:v 1 "$OUT/reference/still-${ms}ms.png"
done
python3 - "$OUT" "$TIMES" <<'PYLIST'
import pathlib,sys
root=pathlib.Path(sys.argv[1])
for sub in ['reference','reconstruction']:
 lines=[]
 for t in sys.argv[2].split(','):
  p=(root/sub/f'still-{round(float(t)*1000)}ms.png').resolve()
  lines.append("file '"+str(p).replace("'","'\\''")+"'\n")
 (root/(sub+'.txt')).write_text(''.join(lines))
PYLIST
ffmpeg -y -hide_banner -loglevel error -f concat -safe 0 -i "$OUT/reference.txt" -vf 'scale=480:270,tile=4x6:padding=4:margin=4' -frames:v 1 "$OUT/reference.jpg"
ffmpeg -y -hide_banner -loglevel error -f concat -safe 0 -i "$OUT/reconstruction.txt" -vf 'scale=480:270,tile=4x6:padding=4:margin=4' -frames:v 1 "$OUT/reconstruction.jpg"
echo "$OUT/reference.jpg"
echo "$OUT/reconstruction.jpg"
