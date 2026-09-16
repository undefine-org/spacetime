# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "opencv-python", "pillow"]
# ///
"""
Green screen compositing pipeline.

Composites print images into template photos containing green placeholder frames,
with automatic perspective correction and color harmonization.
"""

import argparse
import numpy as np
from pathlib import Path
from PIL import Image
import cv2


# =============================================================================
# Module 1: Green Screen Detection
# =============================================================================

def detect_green_mask(
    image: np.ndarray,
    hue_min: int = 35,
    hue_max: int = 85,
    sat_min: int = 40,
    val_min: int = 40,
) -> np.ndarray:
    """
    Detect green regions using HSV thresholding.

    Args:
        image: BGR image (OpenCV format)
        hue_min: Minimum hue (0-180 in OpenCV, green ~35-85)
        hue_max: Maximum hue
        sat_min: Minimum saturation (filters out grays)
        val_min: Minimum value (filters out darks)

    Returns:
        Binary mask (255 = green, 0 = not green)
    """
    hsv = cv2.cvtColor(image, cv2.COLOR_BGR2HSV)

    # Create mask for green region
    lower = np.array([hue_min, sat_min, val_min])
    upper = np.array([hue_max, 255, 255])
    mask = cv2.inRange(hsv, lower, upper)

    # Morphological cleanup
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))
    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, kernel)  # Fill holes
    mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, kernel)   # Remove noise

    return mask


# =============================================================================
# Module 2: Corner Detection
# =============================================================================

def order_corners_clockwise(corners: np.ndarray) -> np.ndarray:
    """
    Order 4 corners clockwise starting from top-left.

    Args:
        corners: Array of shape (4, 2) with unordered corner points

    Returns:
        Ordered corners: [top-left, top-right, bottom-right, bottom-left]
    """
    # Sort by y-coordinate (top points first)
    corners = corners[np.argsort(corners[:, 1])]

    # Top two points: leftmost is top-left
    top = corners[:2]
    top = top[np.argsort(top[:, 0])]

    # Bottom two points: leftmost is bottom-left
    bottom = corners[2:]
    bottom = bottom[np.argsort(bottom[:, 0])]

    return np.array([top[0], top[1], bottom[1], bottom[0]], dtype=np.float32)


def expand_corners_outward(corners: np.ndarray, expand_px: float) -> np.ndarray:
    """
    Expand corners outward from centroid by N pixels.

    This makes the print cover slightly more area, going under the frame edge.
    """
    centroid = corners.mean(axis=0)
    expanded = []
    for corner in corners:
        direction = corner - centroid
        length = np.linalg.norm(direction)
        if length > 0:
            unit = direction / length
            expanded.append(corner + unit * expand_px)
        else:
            expanded.append(corner)
    return np.array(expanded, dtype=np.float32)


def find_quadrilateral_corners(mask: np.ndarray, expand_px: float = 0) -> np.ndarray | None:
    """
    Find the 4 corners of the largest green region.

    Args:
        mask: Binary mask from detect_green_mask
        expand_px: Pixels to expand corners outward (to cover frame edge)

    Returns:
        Array of shape (4, 2) with corners ordered clockwise from top-left,
        or None if detection fails.
    """
    # Find contours
    contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    if not contours:
        return None

    # Get largest contour by area
    largest = max(contours, key=cv2.contourArea)

    # Approximate to polygon
    perimeter = cv2.arcLength(largest, True)
    epsilon = 0.02 * perimeter  # 2% of perimeter

    # Try to find 4-sided polygon
    for eps_mult in [0.02, 0.03, 0.04, 0.05, 0.01]:
        epsilon = eps_mult * perimeter
        approx = cv2.approxPolyDP(largest, epsilon, True)
        if len(approx) == 4:
            corners = approx.reshape(4, 2).astype(np.float32)
            corners = order_corners_clockwise(corners)
            if expand_px > 0:
                corners = expand_corners_outward(corners, expand_px)
            return corners

    # Fallback: use bounding box corners if polygon approximation fails
    rect = cv2.minAreaRect(largest)
    box = cv2.boxPoints(rect).astype(np.float32)
    corners = order_corners_clockwise(box)
    if expand_px > 0:
        corners = expand_corners_outward(corners, expand_px)
    return corners


def validate_and_order_corners(corners: list[tuple[int, int]]) -> np.ndarray:
    """Validate and order manually provided corners."""
    if len(corners) != 4:
        raise ValueError(f"Expected 4 corners, got {len(corners)}")
    arr = np.array(corners, dtype=np.float32)
    return order_corners_clockwise(arr)


def detect_corners(
    image: np.ndarray,
    manual_corners: list[tuple[int, int]] | None = None,
    hue_min: int = 35,
    hue_max: int = 85,
    sat_min: int = 40,
    val_min: int = 40,
    expand_px: float = 0,
) -> tuple[np.ndarray | None, np.ndarray | None]:
    """
    Detect or validate corners for the green frame region.

    Args:
        image: BGR image
        manual_corners: Optional manually specified corners
        hue_min, hue_max, sat_min, val_min: Green detection parameters
        expand_px: Pixels to expand corners outward (covers frame edge)

    Returns:
        Tuple of (corners array, mask) or (None, None) if detection fails
    """
    if manual_corners is not None:
        # Use manual corners, still generate mask for compositing
        mask = detect_green_mask(image, hue_min, hue_max, sat_min, val_min)
        corners = validate_and_order_corners(manual_corners)
        if expand_px > 0:
            corners = expand_corners_outward(corners, expand_px)
        return corners, mask

    mask = detect_green_mask(image, hue_min, hue_max, sat_min, val_min)
    corners = find_quadrilateral_corners(mask, expand_px)
    return corners, mask


# =============================================================================
# Module 3: Perspective Transform
# =============================================================================

def compute_quad_dimensions(corners: np.ndarray) -> tuple[float, float]:
    """
    Compute approximate width and height of a quadrilateral.

    Args:
        corners: 4 corners ordered [top-left, top-right, bottom-right, bottom-left]

    Returns:
        (width, height) as average of opposite edges
    """
    # Width: average of top and bottom edges
    top_width = np.linalg.norm(corners[1] - corners[0])
    bottom_width = np.linalg.norm(corners[2] - corners[3])
    width = (top_width + bottom_width) / 2

    # Height: average of left and right edges
    left_height = np.linalg.norm(corners[3] - corners[0])
    right_height = np.linalg.norm(corners[2] - corners[1])
    height = (left_height + right_height) / 2

    return width, height


def warp_print_to_frame(
    print_img: np.ndarray,
    dst_corners: np.ndarray,
    output_size: tuple[int, int],
    scale: float = 1.0,
) -> tuple[np.ndarray, np.ndarray]:
    """
    Warp print image to fit destination quadrilateral.

    Args:
        print_img: Source print image (BGR)
        dst_corners: Destination corners (4, 2) ordered clockwise from top-left
        output_size: (width, height) of output image
        scale: Scale factor for the print (1.0 = fit exactly, >1 = larger/crops, <1 = smaller/gaps)

    Returns:
        Tuple of (warped_image, warped_mask) both same size as output_size
    """
    h, w = print_img.shape[:2]

    # Calculate crop/padding based on scale
    # scale > 1: crop into print (show less of print, it appears larger)
    # scale < 1: would need padding (show more of print, it appears smaller)
    # We achieve this by adjusting the source rectangle
    if scale != 1.0 and scale > 0:
        # Inset factor: how much to crop from each edge (as fraction of dimension)
        # scale=1.0 -> inset=0, scale=2.0 -> inset=0.25 (show middle 50%)
        inset_frac = (1.0 - 1.0 / scale) / 2.0
        inset_x = w * inset_frac
        inset_y = h * inset_frac
        src_corners = np.array([
            [inset_x, inset_y],
            [w - 1 - inset_x, inset_y],
            [w - 1 - inset_x, h - 1 - inset_y],
            [inset_x, h - 1 - inset_y],
        ], dtype=np.float32)
    else:
        # Default: use full print image
        src_corners = np.array([
            [0, 0],
            [w - 1, 0],
            [w - 1, h - 1],
            [0, h - 1],
        ], dtype=np.float32)

    # Compute perspective transform
    M = cv2.getPerspectiveTransform(src_corners, dst_corners)

    # Warp the print image
    warped = cv2.warpPerspective(
        print_img, M, output_size,
        flags=cv2.INTER_LINEAR,
        borderMode=cv2.BORDER_CONSTANT,
        borderValue=(0, 0, 0),
    )

    # Create and warp a mask (white where print is)
    mask = np.ones((h, w), dtype=np.uint8) * 255
    warped_mask = cv2.warpPerspective(
        mask, M, output_size,
        flags=cv2.INTER_LINEAR,
        borderMode=cv2.BORDER_CONSTANT,
        borderValue=0,
    )

    return warped, warped_mask


# =============================================================================
# Module 4: Color Harmonization
# =============================================================================

def extract_near_frame_region(
    image: np.ndarray,
    corners: np.ndarray,
    mask: np.ndarray,
    margin_px: int = 50,
) -> np.ndarray:
    """
    Extract pixels from the wall region around the frame for color sampling.

    Args:
        image: BGR image
        corners: Frame corners
        mask: Green mask (to exclude)
        margin_px: Pixels to sample around the frame

    Returns:
        Array of sampled BGR pixels for statistics
    """
    h, w = image.shape[:2]

    # Get bounding box of corners
    x_min, y_min = corners.min(axis=0).astype(int)
    x_max, y_max = corners.max(axis=0).astype(int)

    # Expand bounding box by margin
    x_min_exp = max(0, x_min - margin_px)
    y_min_exp = max(0, y_min - margin_px)
    x_max_exp = min(w, x_max + margin_px)
    y_max_exp = min(h, y_max + margin_px)

    # Create sampling mask: expanded region minus green area
    sample_mask = np.zeros((h, w), dtype=np.uint8)
    sample_mask[y_min_exp:y_max_exp, x_min_exp:x_max_exp] = 255

    # Exclude the green region itself
    sample_mask[mask > 0] = 0

    # Also exclude the interior of the frame (just want wall)
    # Shrink the corner polygon slightly
    hull = cv2.convexHull(corners.astype(np.int32))
    cv2.fillConvexPoly(sample_mask, hull, 0)

    # Extract pixels
    pixels = image[sample_mask > 0]

    if len(pixels) < 100:
        # Fallback: use larger region if not enough pixels
        pixels = image[y_min_exp:y_max_exp, x_min_exp:x_max_exp].reshape(-1, 3)

    return pixels


def reinhard_color_transfer(
    source: np.ndarray,
    target_pixels: np.ndarray,
    strength: float = 1.0,
    brightness: float = 0.0,
) -> np.ndarray:
    """
    Transfer color statistics from target region to source using Reinhard's method.

    Args:
        source: Source image (BGR) to transform
        target_pixels: Target pixels (BGR) to match statistics from
        strength: How strongly to apply the transfer (0=none, 1=full, >1=exaggerated)
        brightness: Brightness adjustment (-100 to +100, applied to L channel)

    Returns:
        Color-transferred image (BGR)
    """
    # Convert to LAB color space
    source_lab = cv2.cvtColor(source, cv2.COLOR_BGR2LAB).astype(np.float32)

    # For target pixels, we need to convert them too
    # Reshape to image format for cvtColor
    target_reshaped = target_pixels.reshape(-1, 1, 3).astype(np.uint8)
    target_lab = cv2.cvtColor(target_reshaped, cv2.COLOR_BGR2LAB).astype(np.float32)
    target_lab = target_lab.reshape(-1, 3)

    # Compute statistics
    src_mean = source_lab.mean(axis=(0, 1))
    src_std = source_lab.std(axis=(0, 1))
    tgt_mean = target_lab.mean(axis=0)
    tgt_std = target_lab.std(axis=0)

    # Prevent division by zero
    src_std = np.maximum(src_std, 1e-6)

    # Apply transfer with strength control
    # Full transfer: (source - src_mean) * (tgt_std / src_std) + tgt_mean
    # Blended: lerp between original and transferred
    transferred_lab = (source_lab - src_mean) * (tgt_std / src_std) + tgt_mean
    result_lab = source_lab + strength * (transferred_lab - source_lab)

    # Apply brightness adjustment to L channel
    result_lab[:, :, 0] = result_lab[:, :, 0] + brightness

    # Clip to valid LAB range
    result_lab[:, :, 0] = np.clip(result_lab[:, :, 0], 0, 255)
    result_lab[:, :, 1] = np.clip(result_lab[:, :, 1], 0, 255)
    result_lab[:, :, 2] = np.clip(result_lab[:, :, 2], 0, 255)

    # Convert back to BGR
    result_bgr = cv2.cvtColor(result_lab.astype(np.uint8), cv2.COLOR_LAB2BGR)

    return result_bgr


# =============================================================================
# Module 5: Compositing
# =============================================================================

def composite_with_feathering(
    background: np.ndarray,
    foreground: np.ndarray,
    mask: np.ndarray,
    feather_radius: int = 5,
    erode_px: int = 0,
    green_mask: np.ndarray | None = None,
) -> np.ndarray:
    """
    Alpha composite foreground onto background with feathered edges.

    Args:
        background: Background image (BGR)
        foreground: Foreground image (BGR), same size as background
        mask: Binary mask indicating foreground region
        feather_radius: Gaussian blur radius for edge feathering
        erode_px: Pixels to erode mask inward (removes edge artifacts)
        green_mask: Original green detection mask (for foreground preservation)

    Returns:
        Composited image (BGR)
    """
    # =========================================================================
    # FINGER PRESERVATION ALGORITHM
    # =========================================================================
    # Goal: Preserve fingers/hands that overlap the green frame, while replacing
    # ALL green screen fabric (including shadowed or wrinkled areas).
    #
    # Key insight: We need BOTH conditions to identify a finger:
    #   1. Pixel has skin-tone colors (not just "non-green")
    #   2. Pixel is definitely not green
    #
    # Why both? Shadowed green fabric can shift hue outside the green range,
    # but it won't match skin tones. Real fingers match skin tones AND aren't green.
    # =========================================================================
    if green_mask is not None:
        hsv = cv2.cvtColor(background, cv2.COLOR_BGR2HSV)
        hue = hsv[:, :, 0]  # Hue: 0-180 in OpenCV (0=red, 60=green, 120=blue)
        sat = hsv[:, :, 1]  # Saturation: 0-255 (0=gray, 255=vivid)
        val = hsv[:, :, 2]  # Value: 0-255 (0=black, 255=bright)

        # ---------------------------------------------------------------------
        # CONDITION 1: Skin tone detection
        # ---------------------------------------------------------------------
        # Skin colors in HSV (balanced to catch fingers but not hair):
        #   - Hue: 3-22 (orange/peach tones, slightly wider for diverse skin)
        #   - Saturation: 20-170 (skin has moderate saturation)
        #   - Value: 50-255 (allows for shadowed skin like fingers in shadow)
        #
        # Note: We can be more permissive here because we limit the search
        # area to only green_mask regions (see LIMIT SEARCH below).
        # ---------------------------------------------------------------------
        is_skin_hue = (hue >= 3) & (hue <= 22)
        is_skin_sat = (sat >= 20) & (sat <= 170)
        is_skin_val = (val >= 50)
        is_skin_tone = is_skin_hue & is_skin_sat & is_skin_val

        # ---------------------------------------------------------------------
        # CONDITION 2: Definitely not green
        # ---------------------------------------------------------------------
        # Green in HSV: hue 35-85, with decent saturation (>30) and value (>30)
        # We use a WIDE range here to catch all green variants including shadows.
        # A pixel is "definitely not green" if it fails ANY of these conditions.
        # ---------------------------------------------------------------------
        is_green_hue = (hue >= 35) & (hue <= 85)
        is_green_sat = (sat >= 30)
        is_green_val = (val >= 30)
        is_green = is_green_hue & is_green_sat & is_green_val
        is_not_green = ~is_green

        # ---------------------------------------------------------------------
        # COMBINE: Finger = skin tone AND not green
        # ---------------------------------------------------------------------
        # This dual-check is the key fix:
        #   - Shadowed green fabric: might have shifted hue (~25-40), but fails
        #     skin tone check (hue too high) → NOT preserved
        #   - Real fingers: match skin tone (hue 0-25) AND not green → preserved
        # ---------------------------------------------------------------------
        finger_pixels = (is_skin_tone & is_not_green).astype(np.uint8) * 255

        # ---------------------------------------------------------------------
        # LIMIT SEARCH: Only look for fingers at the BOUNDARY of green areas
        # ---------------------------------------------------------------------
        # Fingers occlude the green screen, creating "holes" in the green_mask.
        # These holes are at the BOUNDARY between green and non-green.
        # - Thumb in front of green: creates a hole → at boundary of green
        # - Face beside frame: not adjacent to green → not at boundary
        #
        # We dilate the green_mask to extend the search area slightly beyond
        # the green edge, then look for skin in that dilated region.
        # ---------------------------------------------------------------------
        boundary_dilate = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (25, 25))
        extended_green = cv2.dilate(green_mask, boundary_dilate)
        search_area = cv2.bitwise_and(mask, extended_green)
        finger_in_frame = cv2.bitwise_and(search_area, finger_pixels)

        # ---------------------------------------------------------------------
        # DILATE: Expand finger regions slightly to capture edges
        # ---------------------------------------------------------------------
        # Fingers detected might miss edges due to color blending at boundaries.
        # A small dilation (5x5) ensures we preserve the full finger outline.
        # ---------------------------------------------------------------------
        dilate_kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))
        finger_in_frame = cv2.dilate(finger_in_frame, dilate_kernel)

        # ---------------------------------------------------------------------
        # FINAL MASK: Remove finger regions from composite
        # ---------------------------------------------------------------------
        # Everywhere we detected a finger, keep the original background (finger).
        # Everywhere else in the frame, replace with the print image.
        # ---------------------------------------------------------------------
        mask = cv2.bitwise_and(mask, cv2.bitwise_not(finger_in_frame))

    # Erode/dilate mask to adjust edge coverage
    # Positive values erode (shrink) to remove green halo
    # Negative values dilate (expand) to cover more of the frame edge
    if erode_px > 0:
        kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (erode_px * 2 + 1, erode_px * 2 + 1))
        mask = cv2.erode(mask, kernel)
    elif erode_px < 0:
        dilate_px = abs(erode_px)
        kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (dilate_px * 2 + 1, dilate_px * 2 + 1))
        mask = cv2.dilate(mask, kernel)

    # Ensure mask is float for blending
    alpha = mask.astype(np.float32) / 255.0

    # Feather the edges
    if feather_radius > 0:
        ksize = feather_radius * 2 + 1  # Must be odd
        alpha = cv2.GaussianBlur(alpha, (ksize, ksize), 0)

    # Expand alpha to 3 channels
    alpha = np.stack([alpha] * 3, axis=-1)

    # Alpha composite
    result = background.astype(np.float32) * (1 - alpha) + foreground.astype(np.float32) * alpha
    result = np.clip(result, 0, 255).astype(np.uint8)

    # Final cleanup: remove any remaining green fringe at boundaries
    if green_mask is not None:
        result_hsv = cv2.cvtColor(result, cv2.COLOR_BGR2HSV)
        result_hue = result_hsv[:, :, 0]
        result_sat = result_hsv[:, :, 1]

        # Find remaining green pixels (hue 30-90, saturation > 50)
        remaining_green = (result_hue >= 30) & (result_hue <= 90) & (result_sat > 50)

        # Only clean up pixels near the mask boundary (within ~10px of edge)
        boundary_kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (21, 21))
        mask_uint8 = (alpha[:, :, 0] * 255).astype(np.uint8)
        dilated = cv2.dilate(mask_uint8, boundary_kernel)
        eroded = cv2.erode(mask_uint8, boundary_kernel)
        boundary_region = cv2.subtract(dilated, eroded)

        # Green pixels in boundary region need cleanup
        green_fringe = remaining_green & (boundary_region > 0)

        # Inpaint these pixels using surrounding colors
        if np.any(green_fringe):
            fringe_mask = green_fringe.astype(np.uint8) * 255
            result = cv2.inpaint(result, fringe_mask, 3, cv2.INPAINT_TELEA)

    return result


# =============================================================================
# Module 6: Main Pipeline
# =============================================================================

def composite(
    template_path: str,
    print_path: str,
    output_path: str | None = None,
    corners: list[tuple[int, int]] | None = None,
    hue_min: int = 35,
    hue_max: int = 85,
    sat_min: int = 40,
    val_min: int = 40,
    expand: float = 5.0,
    scale: float = 1.0,
    color_margin: int = 50,
    feather: int = 5,
    erode: int = 3,
    brightness: float = -20.0,
    color_strength: float = 1.0,
    harmonize: bool = True,
    debug: bool = False,
) -> str:
    """
    Full compositing pipeline.

    Args:
        template_path: Path to template image with green frame
        print_path: Path to print image to composite
        output_path: Output path (default: {template}_composite.{ext})
        corners: Manual corner override (4 points)
        hue_min, hue_max, sat_min, val_min: Green detection parameters
        expand: Pixels to expand corners outward (covers frame edge)
        scale: Scale factor for print (1.0=fit, >1=larger/crops, <1=smaller)
        color_margin: Pixels around frame for color sampling
        feather: Edge feather radius
        erode: Pixels to erode mask inward (removes green halo)
        brightness: Brightness adjustment for print (-100 to +100)
        color_strength: Color transfer strength (0=none, 1=full)
        harmonize: Whether to apply color harmonization
        debug: Save intermediate images

    Returns:
        Path to output image
    """
    template_path = Path(template_path)
    print_path = Path(print_path)

    # Load images
    template = cv2.imread(str(template_path))
    print_img = cv2.imread(str(print_path))

    if template is None:
        raise FileNotFoundError(f"Could not load template: {template_path}")
    if print_img is None:
        raise FileNotFoundError(f"Could not load print: {print_path}")

    print(f"Template: {template_path} ({template.shape[1]}x{template.shape[0]})")
    print(f"Print: {print_path} ({print_img.shape[1]}x{print_img.shape[0]})")

    # Step 1: Detect corners
    print("Detecting green frame corners...")
    detected_corners, mask = detect_corners(
        template, corners, hue_min, hue_max, sat_min, val_min, expand
    )

    if detected_corners is None:
        raise RuntimeError(
            "Could not detect green frame corners. "
            "Try adjusting --hue-min/--hue-max or provide --corners manually."
        )

    print(f"Corners detected: {detected_corners.tolist()}")

    if debug:
        # Save mask
        mask_path = template_path.parent / f"{template_path.stem}_debug_mask.png"
        cv2.imwrite(str(mask_path), mask)
        print(f"  Debug: saved mask to {mask_path}")

        # Save corners visualization
        corners_vis = template.copy()
        for i, (x, y) in enumerate(detected_corners):
            cv2.circle(corners_vis, (int(x), int(y)), 10, (0, 0, 255), -1)
            cv2.putText(
                corners_vis, str(i), (int(x) + 15, int(y)),
                cv2.FONT_HERSHEY_SIMPLEX, 1, (0, 0, 255), 2
            )
        corners_path = template_path.parent / f"{template_path.stem}_debug_corners.png"
        cv2.imwrite(str(corners_path), corners_vis)
        print(f"  Debug: saved corners to {corners_path}")

    # Step 2: Warp print to frame
    print(f"Warping print to frame perspective (scale={scale})...")
    output_size = (template.shape[1], template.shape[0])
    warped_print, warped_mask = warp_print_to_frame(print_img, detected_corners, output_size, scale=scale)

    if debug:
        warped_path = template_path.parent / f"{template_path.stem}_debug_warped.png"
        cv2.imwrite(str(warped_path), warped_print)
        print(f"  Debug: saved warped print to {warped_path}")

    # Step 3: Color harmonization
    if harmonize:
        print("Applying color harmonization...")
        target_pixels = extract_near_frame_region(template, detected_corners, mask, color_margin)
        print(f"  Sampled {len(target_pixels)} pixels from near-frame region")

        # Only harmonize the print region (where mask > 0)
        harmonized = reinhard_color_transfer(warped_print, target_pixels, color_strength, brightness)

        if debug:
            harm_path = template_path.parent / f"{template_path.stem}_debug_harmonized.png"
            cv2.imwrite(str(harm_path), harmonized)
            print(f"  Debug: saved harmonized print to {harm_path}")
    else:
        harmonized = warped_print
        print("Skipping color harmonization (--no-harmonize)")

    # Step 4: Composite
    print(f"Compositing with feather={feather}, erode={erode}...")
    result = composite_with_feathering(
        template, harmonized, warped_mask, feather, erode,
        green_mask=mask
    )

    # Step 5: Save
    if output_path is None:
        output_path = template_path.parent / f"{template_path.stem}_composite{template_path.suffix}"
    else:
        output_path = Path(output_path)

    # Use PIL for better quality JPEG saving
    result_rgb = cv2.cvtColor(result, cv2.COLOR_BGR2RGB)
    Image.fromarray(result_rgb).save(output_path, quality=95)

    print(f"Saved: {output_path}")
    return str(output_path)


def parse_corners(corners_str: str) -> list[tuple[int, int]]:
    """Parse corner string like '100,50,400,50,420,350,80,350' into list of tuples."""
    values = [int(x.strip()) for x in corners_str.split(",")]
    if len(values) != 8:
        raise ValueError(f"Expected 8 values (4 corners x 2 coords), got {len(values)}")
    return [(values[i], values[i + 1]) for i in range(0, 8, 2)]


def main():
    parser = argparse.ArgumentParser(
        description="Composite print images into green placeholder frames",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
EXAMPLES:

  # Basic composite
  uv run composite.py template.jpg print.jpg

  # With debug output
  uv run composite.py template.jpg print.jpg --debug

  # Manual corners (clockwise from top-left)
  uv run composite.py template.jpg print.jpg --corners 100,50,400,50,420,350,80,350

  # Skip color harmonization
  uv run composite.py template.jpg print.jpg --no-harmonize

  # Adjust green detection
  uv run composite.py template.jpg print.jpg --hue-min 30 --hue-max 90
"""
    )

    parser.add_argument("template", help="Template image with green placeholder frame")
    parser.add_argument("print", help="Print image to composite into frame")
    parser.add_argument("-o", "--output", help="Output path (default: {template}_composite.{ext})")
    parser.add_argument(
        "--corners",
        type=parse_corners,
        help="Manual corners: X1,Y1,X2,Y2,X3,Y3,X4,Y4 (clockwise from top-left)"
    )
    parser.add_argument("--hue-min", type=int, default=35, help="Green hue minimum (default: 35)")
    parser.add_argument("--hue-max", type=int, default=85, help="Green hue maximum (default: 85)")
    parser.add_argument("--sat-min", type=int, default=40, help="Saturation minimum (default: 40)")
    parser.add_argument("--val-min", type=int, default=40, help="Value minimum (default: 40)")
    parser.add_argument(
        "--expand", type=float, default=5.0,
        help="Expand corners outward by N pixels to cover frame edge (default: 5)"
    )
    parser.add_argument(
        "--scale", type=float, default=1.0,
        help="Scale factor for print (1.0=fit, >1=larger/crops, <1=smaller)"
    )
    parser.add_argument(
        "--color-margin", type=int, default=50,
        help="Pixels around frame for color sampling (default: 50)"
    )
    parser.add_argument(
        "--feather", type=int, default=5,
        help="Edge feather radius in pixels (default: 5)"
    )
    parser.add_argument(
        "--erode", type=int, default=3,
        help="Erode mask inward by N pixels to remove green halo (default: 3)"
    )
    parser.add_argument(
        "--brightness", type=float, default=-20.0,
        help="Brightness adjustment for print, -100 to +100 (default: -20)"
    )
    parser.add_argument(
        "--color-strength", type=float, default=1.0,
        help="Color transfer strength, 0=none, 1=full (default: 1.0)"
    )
    parser.add_argument(
        "--no-harmonize", action="store_true",
        help="Skip color harmonization step"
    )
    parser.add_argument(
        "--debug", action="store_true",
        help="Save intermediate images (mask, corners, warped)"
    )

    args = parser.parse_args()

    composite(
        template_path=args.template,
        print_path=args.print,
        output_path=args.output,
        corners=args.corners,
        hue_min=args.hue_min,
        hue_max=args.hue_max,
        sat_min=args.sat_min,
        val_min=args.val_min,
        expand=args.expand,
        scale=args.scale,
        color_margin=args.color_margin,
        feather=args.feather,
        erode=args.erode,
        brightness=args.brightness,
        color_strength=args.color_strength,
        harmonize=not args.no_harmonize,
        debug=args.debug,
    )


if __name__ == "__main__":
    main()
