---
name: deblur
description: Image enhancement toolkit - deblurring (Wiener/Richardson-Lucy/unsharp) and AI upscaling (Real-ESRGAN). Use for camera shake, motion blur, ghosting, soft images, or low resolution.
---

# Image Enhancement Skill

Two tools for image quality improvement:
- **deblur.py** - Mathematical sharpening and deconvolution
- **upscale.py** - AI-powered upscaling via Replicate

## deblur.py - Sharpening & Deconvolution

```bash
uv run .claude/skills/deblur/deblur.py <image> [options]
```

### Methods
- `unsharp` - Fast edge sharpening, good for general softness
- `wiener` - Deconvolution for camera shake with known direction
- `richardson-lucy` - Iterative deconvolution for heavy blur

### Options
- `-m`, `--method`: unsharp, wiener, richardson-lucy
- `-o`, `--output`: Output path
- `--amount`: Unsharp strength (default: 1.5)
- `--radius`: Unsharp blur radius (default: 1.0)
- `--psf-size`: Deconvolution kernel size (default: 5)
- `--angle`: Motion blur angle in degrees (auto-detect if omitted)
- `--noise`: Wiener noise ratio (default: 0.01)
- `--iterations`: Richardson-Lucy iterations (default: 30)

### Examples
```bash
# Quick sharpening
uv run .claude/skills/deblur/deblur.py photo.jpg -m unsharp --amount 2.0

# Camera shake correction
uv run .claude/skills/deblur/deblur.py photo.jpg -m wiener --angle 0 --psf-size 3
```

## upscale.py - AI Upscaling

Requires `REPLICATE_API_KEY` environment variable.

```bash
uv run .claude/skills/deblur/upscale.py <image> [options]
```

### Options
- `-o`, `--output`: Output path
- `--scale`: 2 or 4 (default: 2)
- `--face-enhance`: Enable GFPGAN face enhancement

### Examples
```bash
# 2x upscale
uv run .claude/skills/deblur/upscale.py photo.jpg

# 4x upscale with face enhancement
uv run .claude/skills/deblur/upscale.py portrait.jpg --scale 4 --face-enhance
```

## Typical Workflow

1. Sharpen first: `uv run .claude/skills/deblur/deblur.py img.jpg -m unsharp --amount 2`
2. Then upscale: `uv run .claude/skills/deblur/upscale.py img_deblurred.jpg`
