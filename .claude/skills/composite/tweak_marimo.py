# /// script
# requires-python = ">=3.11"
# dependencies = ["marimo", "numpy", "opencv-python", "pillow"]
# ///
"""
Interactive tweaker for composite settings using marimo.

Usage:
    uvx marimo run --sandbox tweak_marimo.py -- TEMPLATE PRINT
    uvx marimo edit --sandbox tweak_marimo.py -- TEMPLATE PRINT
"""

import marimo

__generated_with = "0.10.0"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import sys
    import numpy as np
    import cv2
    from pathlib import Path
    from PIL import Image
    from composite import (
        detect_green_mask,
        find_quadrilateral_corners,
        expand_corners_outward,
        warp_print_to_frame,
        extract_near_frame_region,
        reinhard_color_transfer,
        composite_with_feathering,
    )
    return (
        mo, sys, np, cv2, Path, Image,
        detect_green_mask, find_quadrilateral_corners, expand_corners_outward,
        warp_print_to_frame, extract_near_frame_region, reinhard_color_transfer,
        composite_with_feathering,
    )


@app.cell
def _(mo, sys, cv2):
    # Get paths from CLI args (after --)
    args = sys.argv[1:] if len(sys.argv) > 1 else []

    if len(args) < 2:
        mo.stop(True, mo.md("**Usage:** `uvx marimo run tweak_marimo.py -- TEMPLATE PRINT`"))

    template_path = args[0]
    print_path = args[1]

    template = cv2.imread(template_path)
    print_img = cv2.imread(print_path)

    if template is None:
        mo.stop(True, mo.md(f"**Error:** Could not load template: `{template_path}`"))
    if print_img is None:
        mo.stop(True, mo.md(f"**Error:** Could not load print: `{print_path}`"))

    output_size = (template.shape[1], template.shape[0])
    return template_path, print_path, template, print_img, output_size


@app.cell
def _(mo):
    # Detection sliders (HSV thresholds)
    hue_min = mo.ui.slider(0, 90, value=35, step=1, label="Hue Min", show_value=True)
    hue_max = mo.ui.slider(45, 180, value=85, step=1, label="Hue Max", show_value=True)
    sat_min = mo.ui.slider(0, 100, value=40, step=1, label="Sat Min", show_value=True)
    val_min = mo.ui.slider(0, 100, value=40, step=1, label="Val Min", show_value=True)
    return hue_min, hue_max, sat_min, val_min


@app.cell
def _(mo):
    # Visual sliders
    expand = mo.ui.slider(0, 20, value=5, step=1, label="Expand Corners", show_value=True)
    erode = mo.ui.slider(-15, 15, value=3, step=1, label="Erode (neg=dilate)", show_value=True)
    feather = mo.ui.slider(0, 30, value=5, step=1, label="Feather Edge", show_value=True)
    brightness = mo.ui.slider(-80, 40, value=-20, step=5, label="Brightness", show_value=True)
    color_strength = mo.ui.slider(0, 2, value=1.0, step=0.1, label="Color Match", show_value=True)
    return expand, erode, feather, brightness, color_strength


@app.cell
def _(mo):
    # Debug toggle
    show_mask = mo.ui.checkbox(label="Show mask overlay")
    return (show_mask,)


@app.cell
def _(template, hue_min, hue_max, sat_min, val_min, detect_green_mask, find_quadrilateral_corners):
    # Detect mask and corners (re-runs when detection sliders change)
    mask = detect_green_mask(
        template,
        hue_min=int(hue_min.value),
        hue_max=int(hue_max.value),
        sat_min=int(sat_min.value),
        val_min=int(val_min.value),
    )
    base_corners = find_quadrilateral_corners(mask, expand_px=0)
    return mask, base_corners


@app.cell
def _(
    mo, np, cv2,
    template, print_img, output_size, mask, base_corners,
    expand, erode, feather, brightness, color_strength, show_mask,
    expand_corners_outward, warp_print_to_frame,
    extract_near_frame_region, reinhard_color_transfer, composite_with_feathering
):
    # Render composite (re-runs when any slider changes)
    if base_corners is None:
        mo.stop(True, mo.md("**Error:** Could not detect corners. Adjust detection sliders."))

    # Expand corners outward
    exp_val = float(expand.value)
    corners = expand_corners_outward(base_corners, exp_val) if exp_val > 0 else base_corners

    # Warp print to frame perspective
    warped_print, warped_mask = warp_print_to_frame(print_img, corners, output_size)

    # Color harmonization
    target_pixels = extract_near_frame_region(template, base_corners, mask, 50)
    harmonized = reinhard_color_transfer(
        warped_print, target_pixels,
        strength=float(color_strength.value),
        brightness=float(brightness.value)
    )

    # Final composite with feathering
    result = composite_with_feathering(
        template, harmonized, warped_mask,
        feather_radius=int(feather.value),
        erode_px=int(erode.value)
    )

    # Convert BGR to RGB for display
    result_rgb = cv2.cvtColor(result, cv2.COLOR_BGR2RGB)

    # Optional mask overlay for debugging
    if show_mask.value:
        mask_overlay = np.zeros_like(result_rgb)
        mask_overlay[:, :, 1] = mask  # Green channel shows mask
        result_rgb = cv2.addWeighted(result_rgb, 0.7, mask_overlay, 0.3, 0)

    return result, result_rgb


@app.cell
def _(mo, result_rgb):
    # Display the composite result
    mo.image(result_rgb, width="100%")


@app.cell
def _(mo, hue_min, hue_max, sat_min, val_min, expand, erode, feather, brightness, color_strength, show_mask):
    # Controls UI layout
    mo.vstack([
        mo.md("## Detection (HSV)"),
        mo.hstack([hue_min, hue_max], justify="start"),
        mo.hstack([sat_min, val_min], justify="start"),
        mo.md("## Visual Settings"),
        mo.hstack([expand, erode, feather], justify="start"),
        mo.hstack([brightness, color_strength], justify="start"),
        show_mask,
    ])


@app.cell
def _(mo, template_path, print_path, expand, erode, feather, brightness, color_strength, hue_min, hue_max, sat_min, val_min):
    # Generate CLI command with current settings
    cmd = f"""uv run .claude/skills/composite/composite.py \\
  {template_path} \\
  {print_path} \\
  --expand {expand.value} \\
  --erode {int(erode.value)} \\
  --feather {int(feather.value)} \\
  --brightness {brightness.value} \\
  --color-strength {color_strength.value} \\
  --hue-min {int(hue_min.value)} \\
  --hue-max {int(hue_max.value)} \\
  --sat-min {int(sat_min.value)} \\
  --val-min {int(val_min.value)}"""

    mo.md(f"## CLI Command\n\n```bash\n{cmd}\n```")
    return (cmd,)


@app.cell
def _(mo, cv2, Path, result, template_path):
    # Save button
    save_path = Path(template_path).parent / f"{Path(template_path).stem}_tweaked.jpg"

    def do_save(_):
        cv2.imwrite(str(save_path), result)
        return f"Saved to {save_path}"

    save_btn = mo.ui.button(label=f"Save to {save_path.name}", on_click=do_save)
    save_btn


if __name__ == "__main__":
    app.run()
