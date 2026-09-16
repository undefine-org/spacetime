# /// script
# requires-python = ">=3.11"
# dependencies = ["replicate", "pillow"]
# ///
"""
AI-powered image upscaling using Real-ESRGAN via Replicate API.

Requires REPLICATE_API_KEY environment variable to be set.
"""

import argparse
import base64
import os
from pathlib import Path

import replicate
from PIL import Image


def upscale_image(
    input_path: str,
    output_path: str | None = None,
    scale: int = 2,
    face_enhance: bool = False,
) -> str:
    """
    Upscale image using Real-ESRGAN.

    Args:
        input_path: Path to input image
        output_path: Path for output (default: input_upscaled.ext)
        scale: Upscale factor (2 or 4)
        face_enhance: Enable GFPGAN face enhancement

    Returns:
        Path to output image
    """
    token = os.environ.get("REPLICATE_API_KEY")
    if not token:
        raise ValueError("REPLICATE_API_KEY environment variable not set")

    client = replicate.Client(api_token=token)
    input_path = Path(input_path)

    # Read and encode image
    with open(input_path, "rb") as f:
        img_data = base64.b64encode(f.read()).decode("utf-8")

    # Detect mime type
    suffix = input_path.suffix.lower()
    mime = {"jpg": "jpeg", "jpeg": "jpeg", "png": "png", "webp": "webp"}.get(
        suffix.lstrip("."), "jpeg"
    )
    data_uri = f"data:image/{mime};base64,{img_data}"

    print(f"Loaded image: {input_path}")
    print(f"Upscaling {scale}x with Real-ESRGAN...")

    output = client.run(
        "nightmareai/real-esrgan:f121d640bd286e1fdc67f9799164c1d5be36ff74576ee11c803ae5b665dd46aa",
        input={"image": data_uri, "scale": scale, "face_enhance": face_enhance},
    )

    # Download result
    import urllib.request

    if output_path is None:
        output_path = input_path.parent / f"{input_path.stem}_upscaled{input_path.suffix}"
    else:
        output_path = Path(output_path)

    # Download to temp, convert if needed
    temp_path = output_path.parent / f".tmp_upscale{output_path.suffix}"
    urllib.request.urlretrieve(str(output), str(temp_path))

    # Convert PNG output to target format if needed
    img = Image.open(temp_path)
    if output_path.suffix.lower() in [".jpg", ".jpeg"]:
        img = img.convert("RGB")
        img.save(output_path, quality=92)
    else:
        img.save(output_path)

    temp_path.unlink()  # Clean up temp file

    print(f"Saved upscaled image: {output_path}")
    return str(output_path)


def main():
    parser = argparse.ArgumentParser(
        description="Upscale images using Real-ESRGAN AI model",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
ABOUT:
  Uses Real-ESRGAN via Replicate API for high-quality AI upscaling.
  Requires REPLICATE_API_KEY environment variable.

EXAMPLES:
  # 2x upscale (default)
  uv run upscale.py photo.jpg

  # 4x upscale
  uv run upscale.py photo.jpg --scale 4

  # With face enhancement
  uv run upscale.py portrait.jpg --face-enhance

  # Specify output path
  uv run upscale.py photo.jpg -o photo_hires.jpg
""",
    )
    parser.add_argument("input", help="Input image path")
    parser.add_argument("-o", "--output", help="Output image path")
    parser.add_argument(
        "--scale",
        type=int,
        choices=[2, 4],
        default=2,
        help="Upscale factor (default: 2)",
    )
    parser.add_argument(
        "--face-enhance",
        action="store_true",
        help="Enable GFPGAN face enhancement",
    )

    args = parser.parse_args()

    upscale_image(
        input_path=args.input,
        output_path=args.output,
        scale=args.scale,
        face_enhance=args.face_enhance,
    )


if __name__ == "__main__":
    main()
