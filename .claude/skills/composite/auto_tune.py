# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "opencv-python", "pillow", "optuna"]
# ///
"""
Auto-tune composite parameters using optimization.

Two separate objectives:
1. Composition - minimize green, preserve fingers, smooth edges
2. Color - luminance/saturation harmony with scene

Usage:
    uv run auto_tune.py template.jpg print.jpg --mode composition --trials 50
    uv run auto_tune.py template.jpg print.jpg --mode color --trials 30
    uv run auto_tune.py template.jpg print.jpg --mode all --trials 50
"""

import argparse
from dataclasses import dataclass, field
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

from optimize import ParamOptimizer
from composite import (
    detect_green_mask,
    find_quadrilateral_corners,
    expand_corners_outward,
    warp_print_to_frame,
    extract_near_frame_region,
    reinhard_color_transfer,
    composite_with_feathering,
)


# =============================================================================
# Parameter Dataclasses
# =============================================================================

@dataclass
class CompositionParams:
    """Parameters affecting mask and composition quality."""
    expand: float = field(default=5.0, metadata={'min': 0, 'max': 25, 'step': 1.0})
    erode: int = field(default=3, metadata={'min': 0, 'max': 12})
    feather: int = field(default=5, metadata={'min': 1, 'max': 25})
    hue_min: int = field(default=35, metadata={'min': 15, 'max': 50})
    hue_max: int = field(default=85, metadata={'min': 70, 'max': 120})
    sat_min: int = field(default=40, metadata={'min': 20, 'max': 80})
    val_min: int = field(default=40, metadata={'min': 20, 'max': 80})


@dataclass
class ColorParams:
    """Parameters affecting color harmony."""
    brightness: float = field(default=-20.0, metadata={'min': -80, 'max': 40, 'step': 5.0})
    color_strength: float = field(default=1.0, metadata={'min': 0.0, 'max': 2.0, 'step': 0.1})
    color_margin: int = field(default=50, metadata={'min': 10, 'max': 150, 'step': 10})


# =============================================================================
# Runner Functions
# =============================================================================

def run_composition(
    params: CompositionParams,
    template: np.ndarray,
    print_img: np.ndarray,
    color_params: ColorParams | None = None,
) -> np.ndarray:
    """Run composite with composition parameters."""
    if color_params is None:
        color_params = ColorParams()

    # Detect green mask with current params
    mask = detect_green_mask(
        template,
        hue_min=params.hue_min,
        hue_max=params.hue_max,
        sat_min=params.sat_min,
        val_min=params.val_min,
    )

    corners = find_quadrilateral_corners(mask, expand_px=0)
    if corners is None:
        return template.copy()

    corners = expand_corners_outward(corners, params.expand)
    output_size = (template.shape[1], template.shape[0])

    warped, warped_mask = warp_print_to_frame(print_img, corners, output_size)
    target_pixels = extract_near_frame_region(template, corners, mask, color_params.color_margin)
    harmonized = reinhard_color_transfer(warped, target_pixels, color_params.color_strength, color_params.brightness)

    result = composite_with_feathering(
        template, harmonized, warped_mask,
        feather_radius=params.feather,
        erode_px=params.erode,
        green_mask=mask,
    )

    return result


def run_color(
    params: ColorParams,
    template: np.ndarray,
    print_img: np.ndarray,
    composition_params: CompositionParams | None = None,
) -> np.ndarray:
    """Run composite with color parameters."""
    if composition_params is None:
        composition_params = CompositionParams()

    # Use fixed composition params
    mask = detect_green_mask(
        template,
        hue_min=composition_params.hue_min,
        hue_max=composition_params.hue_max,
        sat_min=composition_params.sat_min,
        val_min=composition_params.val_min,
    )

    corners = find_quadrilateral_corners(mask, expand_px=0)
    if corners is None:
        return template.copy()

    corners = expand_corners_outward(corners, composition_params.expand)
    output_size = (template.shape[1], template.shape[0])

    warped, warped_mask = warp_print_to_frame(print_img, corners, output_size)
    target_pixels = extract_near_frame_region(template, corners, mask, params.color_margin)
    harmonized = reinhard_color_transfer(warped, target_pixels, params.color_strength, params.brightness)

    result = composite_with_feathering(
        template, harmonized, warped_mask,
        feather_radius=composition_params.feather,
        erode_px=composition_params.erode,
        green_mask=mask,
    )

    return result


# =============================================================================
# Objective Functions
# =============================================================================

def composition_objective(result: np.ndarray, context: dict) -> float:
    """
    Measures composition quality. Lower is better.
    Weights: green (1.0) > fingers (0.7) > edges (0.3)
    """
    template = context['template']
    hsv = cv2.cvtColor(result, cv2.COLOR_BGR2HSV)

    # Get frame region from the result (approximate using green detection on template)
    template_hsv = cv2.cvtColor(template, cv2.COLOR_BGR2HSV)
    frame_mask = ((template_hsv[:, :, 0] >= 30) & (template_hsv[:, :, 0] <= 90) &
                  (template_hsv[:, :, 1] > 40)).astype(np.uint8) * 255

    # 1. GREEN PENALTY (weight: 1.0) - remaining green pixels in result
    green = (hsv[:, :, 0] >= 25) & (hsv[:, :, 0] <= 95) & (hsv[:, :, 1] > 40)
    green_in_frame = green & (frame_mask > 0)
    frame_area = max(np.sum(frame_mask > 0), 1)
    green_score = np.sum(green_in_frame) / frame_area

    # 2. FINGER PRESERVATION (weight: 0.7) - non-green edges should match template
    edge_kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (15, 15))
    edge_region = cv2.dilate(frame_mask, edge_kernel) - frame_mask

    # Fingers: non-green hue in template (skin tones)
    template_fingers = (template_hsv[:, :, 0] < 25) | (template_hsv[:, :, 0] > 95)
    finger_region = (edge_region > 0) & template_fingers

    if np.sum(finger_region) > 100:
        finger_diff = np.abs(result.astype(float) - template.astype(float))
        finger_score = np.mean(finger_diff[finger_region]) / 255.0
    else:
        finger_score = 0

    # 3. EDGE SMOOTHNESS (weight: 0.3) - low gradient at composition boundary
    gray = cv2.cvtColor(result, cv2.COLOR_BGR2GRAY)
    grad_x = cv2.Sobel(gray, cv2.CV_64F, 1, 0, ksize=3)
    grad_y = cv2.Sobel(gray, cv2.CV_64F, 0, 1, ksize=3)
    gradient = np.sqrt(grad_x**2 + grad_y**2)

    boundary = cv2.dilate(frame_mask, edge_kernel) - cv2.erode(frame_mask, edge_kernel)
    if np.sum(boundary > 0) > 0:
        edge_score = np.mean(gradient[boundary > 0]) / 255.0
    else:
        edge_score = 0

    return green_score * 1.0 + finger_score * 0.7 + edge_score * 0.3


def color_objective(result: np.ndarray, context: dict) -> float:
    """
    Measures color harmony. Lower is better.
    Compares LAB statistics of print region vs surrounding scene.
    """
    template = context['template']

    # Get frame region
    template_hsv = cv2.cvtColor(template, cv2.COLOR_BGR2HSV)
    frame_mask = ((template_hsv[:, :, 0] >= 30) & (template_hsv[:, :, 0] <= 90) &
                  (template_hsv[:, :, 1] > 40)).astype(np.uint8) * 255

    # Sample region around frame (the "scene" colors)
    dilate_kernel = np.ones((50, 50), np.uint8)
    dilated = cv2.dilate(frame_mask, dilate_kernel)
    scene_region = (dilated > 0) & (frame_mask == 0)

    if np.sum(frame_mask > 0) < 100 or np.sum(scene_region) < 100:
        return 1.0  # Can't evaluate

    # Convert to LAB
    result_lab = cv2.cvtColor(result, cv2.COLOR_BGR2LAB).astype(float)

    # Stats of composited print region
    print_pixels = result_lab[frame_mask > 0]
    print_mean = print_pixels.mean(axis=0)
    print_std = print_pixels.std(axis=0)

    # Stats of surrounding scene
    scene_pixels = result_lab[scene_region]
    scene_mean = scene_pixels.mean(axis=0)
    scene_std = scene_pixels.std(axis=0)

    # Penalize large differences in luminance (L channel)
    l_diff = abs(print_mean[0] - scene_mean[0]) / 255.0

    # Penalize if print is much more saturated than scene (a,b channels)
    ab_diff = np.sqrt((print_std[1] - scene_std[1])**2 + (print_std[2] - scene_std[2])**2) / 128.0

    return l_diff * 0.6 + ab_diff * 0.4


# =============================================================================
# CLI
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Auto-tune composite parameters",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("template", help="Template image with green frame")
    parser.add_argument("print", help="Print image to composite")
    parser.add_argument("-o", "--output", help="Output path")
    parser.add_argument(
        "--mode",
        choices=["composition", "color", "all"],
        default="composition",
        help="What to optimize (default: composition)",
    )
    parser.add_argument("--trials", type=int, default=50, help="Number of trials (default: 50)")
    parser.add_argument("--timeout", type=float, help="Max seconds to run")
    parser.add_argument("--quiet", action="store_true", help="Suppress progress output")

    args = parser.parse_args()

    # Load images
    template = cv2.imread(args.template)
    print_img = cv2.imread(args.print)

    if template is None:
        print(f"Error: Could not load template: {args.template}")
        return
    if print_img is None:
        print(f"Error: Could not load print: {args.print}")
        return

    print(f"Template: {args.template} ({template.shape[1]}x{template.shape[0]})")
    print(f"Print: {args.print} ({print_img.shape[1]}x{print_img.shape[0]})")

    context = {
        'template': template,
        'print_img': print_img,
    }

    best_composition = CompositionParams()
    best_color = ColorParams()

    # Optimize composition
    if args.mode in ["composition", "all"]:
        print(f"\n{'='*60}")
        print("Optimizing COMPOSITION parameters...")
        print(f"{'='*60}")

        optimizer = ParamOptimizer(
            CompositionParams,
            runner=run_composition,
            objective=composition_objective,
        )

        best_composition, score = optimizer.optimize(
            n_trials=args.trials,
            timeout=args.timeout,
            show_progress=not args.quiet,
            verbose=not args.quiet,
            **context,
        )

        print(f"\nBest composition score: {score:.6f}")
        print(f"Best params: {best_composition}")

    # Optimize color
    if args.mode in ["color", "all"]:
        print(f"\n{'='*60}")
        print("Optimizing COLOR parameters...")
        print(f"{'='*60}")

        def run_color_with_composition(params, **ctx):
            return run_color(params, composition_params=best_composition, **ctx)

        optimizer = ParamOptimizer(
            ColorParams,
            runner=run_color_with_composition,
            objective=color_objective,
        )

        best_color, score = optimizer.optimize(
            n_trials=args.trials // 2 if args.mode == "all" else args.trials,
            timeout=args.timeout,
            show_progress=not args.quiet,
            verbose=not args.quiet,
            **context,
        )

        print(f"\nBest color score: {score:.6f}")
        print(f"Best params: {best_color}")

    # Generate final result
    print(f"\n{'='*60}")
    print("Generating final composite...")
    print(f"{'='*60}")

    result = run_composition(
        params=best_composition,
        template=template,
        print_img=print_img,
        color_params=best_color,
    )

    # Save
    template_path = Path(args.template)
    if args.output:
        output_path = Path(args.output)
    else:
        output_path = template_path.parent / f"{template_path.stem}_optimized{template_path.suffix}"

    result_rgb = cv2.cvtColor(result, cv2.COLOR_BGR2RGB)
    Image.fromarray(result_rgb).save(output_path, quality=95)

    print(f"\nSaved: {output_path}")

    # Print CLI command to reproduce
    print(f"\n{'='*60}")
    print("CLI command to reproduce:")
    print(f"{'='*60}")
    cmd = f"uv run composite.py '{args.template}' '{args.print}' \\\n"
    cmd += f"  --expand {best_composition.expand:.1f} --erode {best_composition.erode} \\\n"
    cmd += f"  --feather {best_composition.feather} \\\n"
    cmd += f"  --hue-min {best_composition.hue_min} --hue-max {best_composition.hue_max} \\\n"
    cmd += f"  --sat-min {best_composition.sat_min} --val-min {best_composition.val_min} \\\n"
    cmd += f"  --brightness {best_color.brightness:.1f} --color-strength {best_color.color_strength:.1f} \\\n"
    cmd += f"  --color-margin {best_color.color_margin}"
    print(cmd)


if __name__ == "__main__":
    main()
