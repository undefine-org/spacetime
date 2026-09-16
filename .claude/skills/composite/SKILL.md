---
name: composite
description: Green screen compositing - place print images into template frames with perspective correction and color harmonization. Use for product mockups, ads, and frame replacements.
---

# Composite Skill

## Overview

Composites artwork into green placeholder frames with automatic:
- Green screen detection and corner finding
- Perspective transformation to match frame orientation
- Color harmonization to match scene lighting
- Feathered edge blending for seamless results

## Usage

```bash
uv run .claude/skills/composite/composite.py TEMPLATE PRINT [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `TEMPLATE` | Template image containing a green placeholder frame |
| `PRINT` | Print/artwork image to composite into the frame |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `-o, --output PATH` | `{template}_composite.{ext}` | Output file path |
| `--corners X1,Y1,X2,Y2,...` | Auto-detect | Manual corner override (4 points, clockwise from top-left) |
| `--hue-min INT` | 35 | Green hue minimum (0-180 in OpenCV HSV) |
| `--hue-max INT` | 85 | Green hue maximum |
| `--sat-min INT` | 40 | Saturation minimum (filters out gray/white) |
| `--val-min INT` | 40 | Value minimum (filters out dark areas) |
| `--color-margin INT` | 50 | Pixels around frame for color sampling |
| `--feather INT` | 5 | Edge feather radius in pixels |
| `--no-harmonize` | False | Skip color harmonization step |
| `--debug` | False | Save intermediate images (mask, corners, warped) |

## Examples

### Basic Usage
```bash
# Composite a print into a template
uv run .claude/skills/composite/composite.py \
  photos/room_with_frame.jpg \
  artwork/landscape.jpg

# Output: photos/room_with_frame_composite.jpg
```

### With Debug Output
```bash
# See intermediate processing steps
uv run .claude/skills/composite/composite.py \
  template.jpg print.jpg --debug

# Creates:
#   template_debug_mask.png       - Green detection mask
#   template_debug_corners.png    - Detected corner points
#   template_debug_warped.png     - Warped print before harmonization
#   template_composite.jpg        - Final result
```

### Manual Corner Override
```bash
# If auto-detection fails, specify corners manually
# (clockwise from top-left: TL, TR, BR, BL)
uv run .claude/skills/composite/composite.py \
  template.jpg print.jpg \
  --corners 100,50,400,50,420,350,80,350
```

### Adjust Green Detection
```bash
# For different shades of green
uv run .claude/skills/composite/composite.py \
  template.jpg print.jpg \
  --hue-min 30 --hue-max 90 --sat-min 30
```

### Skip Color Harmonization
```bash
# Keep original print colors (for testing or stylized results)
uv run .claude/skills/composite/composite.py \
  template.jpg print.jpg --no-harmonize
```

## Pipeline Details

### 1. Green Screen Detection
Converts image to HSV color space and creates a binary mask where:
- Hue is in the green range (default 35-85, ~78°-170° in standard HSV)
- Saturation > threshold (filters grays)
- Value > threshold (filters darks)

Morphological operations clean up the mask (remove noise, fill holes).

### 2. Corner Detection
Uses OpenCV's contour detection on the mask:
1. `findContours()` extracts the green region boundary
2. `approxPolyDP()` simplifies to a quadrilateral (4 vertices)
3. Corners are ordered clockwise from top-left for consistent homography

### 3. Perspective Transform
Warps the print image to fit the detected quadrilateral:
- Source: rectangular print corners
- Destination: detected frame corners
- Uses `getPerspectiveTransform()` + `warpPerspective()`

### 4. Color Harmonization (Reinhard Transfer)
Matches print colors to scene lighting using LAB color space:
1. Sample pixels from wall region around the frame (not the green itself)
2. Compute mean and std of L, A, B channels
3. Apply linear transfer: `result = (source - μ_src) * (σ_tgt / σ_src) + μ_tgt`

This makes warm-lit scenes properly tint the print warm, etc.

### 5. Alpha Compositing
Blends the warped print into the template:
- Gaussian blur on mask edges creates feathering
- Alpha blend: `result = template * (1-α) + print * α`

## Troubleshooting

### Green not detected
- Check hue range: use `--debug` to see the mask
- Adjust `--hue-min` / `--hue-max` for your specific green
- Lower `--sat-min` if green is desaturated

### Wrong corners detected
- Use `--debug` to visualize detected corners
- Provide manual corners with `--corners`
- Ensure green region is the largest green area in image

### Colors look wrong
- The harmonization samples the wall area around the frame
- Try `--no-harmonize` to keep original colors
- Adjust `--color-margin` to sample different area

### Hard edges visible
- Increase `--feather` (default 5, try 10-20)
- Check that mask doesn't include frame edges

## Interactive Tweaker

Fine-tune all composite settings with a live preview in your browser:

```bash
uvx marimo run --sandbox .claude/skills/composite/tweak_marimo.py -- TEMPLATE PRINT
```

Opens an interactive UI with:
- **Detection sliders**: hue_min, hue_max, sat_min, val_min (HSV thresholds)
- **Visual sliders**: expand, erode, feather, brightness, color_strength
- **Mask overlay toggle**: See what the detection is picking up
- **Save button**: Export directly to `{template}_tweaked.jpg`
- **CLI command**: Copy the exact `composite.py` command with your settings

Use `uvx marimo edit --sandbox` instead of `run` to modify the notebook itself.
