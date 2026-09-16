# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "scikit-image", "pillow", "scipy"]
# ///
"""
Mathematical image deblurring using deconvolution.

Corrects camera shake and motion blur by estimating the Point Spread Function (PSF)
and applying Wiener or Richardson-Lucy deconvolution.
"""

import argparse
import numpy as np
from pathlib import Path
from PIL import Image
from scipy import ndimage
from skimage import restoration, color, img_as_float, img_as_ubyte
from skimage.restoration import wiener, richardson_lucy
from skimage.filters import unsharp_mask


def create_motion_psf(size: int, angle: float) -> np.ndarray:
    """
    Create a motion blur PSF (Point Spread Function).

    Args:
        size: Kernel size in pixels (length of motion)
        angle: Angle of motion in degrees

    Returns:
        Normalized PSF kernel
    """
    psf = np.zeros((size, size))
    center = size // 2

    # Create line at given angle
    angle_rad = np.deg2rad(angle)
    for i in range(size):
        offset = i - center
        x = int(center + offset * np.cos(angle_rad))
        y = int(center + offset * np.sin(angle_rad))
        if 0 <= x < size and 0 <= y < size:
            psf[y, x] = 1

    # Normalize
    psf = psf / psf.sum() if psf.sum() > 0 else psf
    return psf


def create_gaussian_psf(size: int, sigma: float = 1.0) -> np.ndarray:
    """Create a Gaussian PSF for general blur."""
    x = np.arange(size) - size // 2
    y = np.arange(size) - size // 2
    X, Y = np.meshgrid(x, y)
    psf = np.exp(-(X**2 + Y**2) / (2 * sigma**2))
    return psf / psf.sum()


def estimate_blur_angle(image: np.ndarray) -> float:
    """
    Estimate motion blur angle from image using gradient analysis.

    Analyzes the dominant direction of edges/gradients to infer motion direction.
    """
    # Convert to grayscale if needed
    if image.ndim == 3:
        gray = color.rgb2gray(image)
    else:
        gray = image

    # Compute gradients
    gy, gx = np.gradient(gray)

    # Compute gradient magnitude and angle
    magnitude = np.sqrt(gx**2 + gy**2)

    # Weight angles by gradient magnitude (stronger edges = more reliable)
    # Focus on high-magnitude gradients (edges)
    threshold = np.percentile(magnitude, 90)
    mask = magnitude > threshold

    if not np.any(mask):
        return 0.0

    # Compute weighted average angle
    angles = np.arctan2(gy[mask], gx[mask])
    weights = magnitude[mask]

    # The blur direction is perpendicular to edge direction
    # Use circular mean for angles
    sin_sum = np.sum(weights * np.sin(2 * angles))
    cos_sum = np.sum(weights * np.cos(2 * angles))
    mean_angle = 0.5 * np.arctan2(sin_sum, cos_sum)

    # Convert to degrees and rotate 90° (blur is perpendicular to edges)
    return np.rad2deg(mean_angle) + 90


def deblur_wiener(image: np.ndarray, psf: np.ndarray, noise_ratio: float = 0.01) -> np.ndarray:
    """
    Apply Wiener deconvolution to remove blur.

    Args:
        image: Input blurred image
        psf: Point Spread Function (blur kernel)
        noise_ratio: Noise-to-signal power ratio (regularization)

    Returns:
        Deblurred image
    """
    if image.ndim == 3:
        # Process each channel separately
        result = np.zeros_like(image)
        for c in range(image.shape[2]):
            result[:, :, c] = wiener(image[:, :, c], psf, noise_ratio)
        return result
    else:
        return wiener(image, psf, noise_ratio)


def deblur_richardson_lucy(image: np.ndarray, psf: np.ndarray, iterations: int = 30) -> np.ndarray:
    """
    Apply Richardson-Lucy deconvolution.

    Better for unknown noise characteristics, iteratively refines the estimate.

    Args:
        image: Input blurred image
        psf: Point Spread Function
        iterations: Number of iterations

    Returns:
        Deblurred image
    """
    if image.ndim == 3:
        result = np.zeros_like(image)
        for c in range(image.shape[2]):
            result[:, :, c] = richardson_lucy(image[:, :, c], psf, num_iter=iterations)
        return result
    else:
        return richardson_lucy(image, psf, num_iter=iterations)


def sharpen_unsharp(image: np.ndarray, radius: float = 1.0, amount: float = 1.0) -> np.ndarray:
    """
    Apply unsharp mask sharpening.

    Good for general sharpening when deconvolution isn't appropriate
    (e.g., subject motion blur, mild softness).

    Args:
        image: Input image
        radius: Gaussian blur radius for the mask
        amount: Strength of sharpening (1.0 = standard, 2.0 = strong)

    Returns:
        Sharpened image
    """
    # Use channel_axis=2 explicitly for RGB images (channel_axis=-1 has a bug in some skimage versions)
    return unsharp_mask(image, radius=radius, amount=amount, channel_axis=2 if image.ndim == 3 else None)


def deblur_image(
    input_path: str,
    output_path: str | None = None,
    method: str = "wiener",
    psf_size: int = 5,
    angle: float | None = None,
    iterations: int = 30,
    noise_ratio: float = 0.01,
    sharpen_radius: float = 1.0,
    sharpen_amount: float = 1.5,
) -> str:
    """
    Main deblurring function.

    Args:
        input_path: Path to input image
        output_path: Path for output (default: input_deblurred.ext)
        method: 'wiener', 'richardson-lucy', or 'unsharp'
        psf_size: Size of PSF kernel
        angle: Motion blur angle (None = auto-detect)
        iterations: R-L iterations
        noise_ratio: Wiener noise parameter
        sharpen_radius: Unsharp mask radius
        sharpen_amount: Unsharp mask strength

    Returns:
        Path to output image
    """
    # Load image
    input_path = Path(input_path)
    img = Image.open(input_path)
    image = img_as_float(np.array(img))

    print(f"Loaded image: {input_path} ({image.shape})")

    # Apply method
    if method == "unsharp":
        print(f"Applying unsharp mask (radius={sharpen_radius}, amount={sharpen_amount})...")
        result = sharpen_unsharp(image, sharpen_radius, sharpen_amount)
    else:
        # Deconvolution methods need PSF
        if angle is None:
            angle = estimate_blur_angle(image)
            print(f"Auto-detected blur angle: {angle:.1f}°")
        else:
            print(f"Using specified blur angle: {angle:.1f}°")

        # Create PSF
        psf = create_motion_psf(psf_size, angle)
        print(f"Created {psf_size}x{psf_size} motion PSF")

        if method == "wiener":
            print(f"Applying Wiener deconvolution (noise_ratio={noise_ratio})...")
            result = deblur_wiener(image, psf, noise_ratio)
        elif method == "richardson-lucy":
            print(f"Applying Richardson-Lucy ({iterations} iterations)...")
            result = deblur_richardson_lucy(image, psf, iterations)
        else:
            raise ValueError(f"Unknown method: {method}")

    # Clip to valid range
    result = np.clip(result, 0, 1)

    # Convert back to uint8
    result_uint8 = img_as_ubyte(result)

    # Save
    if output_path is None:
        output_path = input_path.parent / f"{input_path.stem}_deblurred{input_path.suffix}"
    else:
        output_path = Path(output_path)

    Image.fromarray(result_uint8).save(output_path, quality=95)
    print(f"Saved deblurred image: {output_path}")

    return str(output_path)


def main():
    parser = argparse.ArgumentParser(
        description="Deblur images using mathematical deconvolution",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
METHODS:

  wiener (default for camera shake)
    + Fast, single-pass frequency domain filtering
    + Good for camera shake with known motion direction
    + Predictable results, minimal artifacts
    - Can amplify noise if noise parameter is wrong
    - Requires accurate PSF estimation

  richardson-lucy
    + Better for unknown noise characteristics
    + Iteratively refines, can handle heavier blur
    + Preserves positivity (no negative values)
    - Slower (many iterations needed)
    - Can create ringing artifacts if over-iterated
    - May amplify noise with too many iterations

  unsharp
    + Simple, fast, no PSF estimation needed
    + Good for general softness or subject motion blur
    + Works when blur direction is unknown/variable
    - Doesn't truly reverse blur, just enhances edges
    - Can create halos around high-contrast edges

EXAMPLES:
  # Quick sharpening (no PSF needed)
  uv run deblur.py photo.jpg -m unsharp --amount 1.5

  # Camera shake with Wiener
  uv run deblur.py photo.jpg -m wiener --psf-size 3 --noise 0.01

  # Heavy blur with Richardson-Lucy
  uv run deblur.py photo.jpg -m richardson-lucy --iterations 50

  # Specify known motion direction
  uv run deblur.py photo.jpg -m wiener --angle 45 --psf-size 7
"""
    )
    parser.add_argument("input", help="Input image path")
    parser.add_argument("-o", "--output", help="Output image path")
    parser.add_argument(
        "--method", "-m",
        choices=["wiener", "richardson-lucy", "unsharp"],
        default=None,
        help="Processing method (see METHODS below)",
    )
    parser.add_argument(
        "--psf-size",
        type=int,
        default=5,
        help="PSF kernel size in pixels - larger = more blur correction (default: 5)",
    )
    parser.add_argument(
        "--angle",
        type=float,
        default=None,
        help="Motion blur angle in degrees (default: auto-detect from edges)",
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=30,
        help="Richardson-Lucy iterations - more = sharper but risk artifacts (default: 30)",
    )
    parser.add_argument(
        "--noise",
        type=float,
        default=0.01,
        help="Noise-to-signal ratio for Wiener - lower = sharper but more noise (default: 0.01)",
    )
    parser.add_argument(
        "--radius",
        type=float,
        default=1.0,
        help="Unsharp mask blur radius (default: 1.0)",
    )
    parser.add_argument(
        "--amount",
        type=float,
        default=1.5,
        help="Unsharp mask strength - higher = more sharpening (default: 1.5)",
    )

    args = parser.parse_args()

    # Warn if method not specified
    method = args.method
    if method is None:
        print("\n" + "=" * 60)
        print("WARNING: No --method specified")
        print("=" * 60)
        print("Defaulting to 'wiener' - fast frequency-domain deconvolution.")
        print("Good for mild blur, but may amplify noise.")
        print("")
        print("For heavier blur, try: --method richardson-lucy --iterations 50")
        print("=" * 60 + "\n")
        method = "wiener"

    deblur_image(
        input_path=args.input,
        output_path=args.output,
        method=method,
        psf_size=args.psf_size,
        angle=args.angle,
        iterations=args.iterations,
        noise_ratio=args.noise,
        sharpen_radius=args.radius,
        sharpen_amount=args.amount,
    )


if __name__ == "__main__":
    main()
