---
name: replicate-image-gen
description: Generate product images using Replicate's nano-banana (Gemini) model. Use when user asks to generate images, create product photos, AI image generation, or transform reference images into styled scenes.
---

# Replicate Image Generation Skill

Generate AI-powered product images using Replicate's nano-banana (Gemini) model. This skill transforms a reference product image into various styled scenes.

## When to Use

Activate this skill when the user mentions:
- Generate product images
- Create AI images from reference
- Transform product photos
- Generate marketing images
- Create styled product scenes

## Prerequisites

1. **Replicate API Token**: Must be set in `.env` file as `REPLICATE_API_TOKEN`
   - Get your token at: https://replicate.com/account/api-tokens

2. **Python dependencies**: `requests`, `python-dotenv`
   ```bash
   pip install requests python-dotenv
   ```

## How to Execute

### Step 1: Create the Python Script

Create a file `generate_images.py` with this structure:

```python
#!/usr/bin/env python3
import os
import requests
import time
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()

OUTPUT_DIR = Path("images/generated")
REPLICATE_API_TOKEN = os.getenv("REPLICATE_API_TOKEN")
REPLICATE_API_URL = "https://api.replicate.com/v1/models/google/nano-banana/predictions"

if not REPLICATE_API_TOKEN:
    raise ValueError("REPLICATE_API_TOKEN not found in .env file")

# Define your reference image URL
REFERENCE_IMAGE_URL = "https://example.com/your-product-image.png"

# Define prompts for each image to generate
IMAGE_PROMPTS = {
    "output-filename.jpg": """Your detailed prompt here describing the scene.
    Include context about the product and desired style.""",
}


def generate_image(prompt: str, filename: str):
    """Generate a single image using Replicate's nano-banana model."""
    print(f"\n🎨 Generating: {filename}")

    headers = {
        "Authorization": f"Bearer {REPLICATE_API_TOKEN}",
        "Content-Type": "application/json",
        "Prefer": "wait"
    }

    payload = {
        "input": {
            "prompt": prompt,
            "image_input": [REFERENCE_IMAGE_URL]
        }
    }

    try:
        response = requests.post(REPLICATE_API_URL, headers=headers, json=payload, timeout=300)

        if response.status_code in [200, 201]:
            result = response.json()

            if result.get("status") == "succeeded":
                output = result.get("output")
                if output:
                    image_url = output[0] if isinstance(output, list) else output
                    img_response = requests.get(image_url, timeout=60)
                    if img_response.status_code == 200:
                        output_path = OUTPUT_DIR / filename
                        with open(output_path, "wb") as f:
                            f.write(img_response.content)
                        print(f"   ✅ Saved to: {output_path}")
                        return True

            elif result.get("status") in ["processing", "starting"]:
                prediction_url = result.get("urls", {}).get("get")
                if prediction_url:
                    return poll_for_result(prediction_url, filename, headers)

        print(f"   ❌ Error {response.status_code}: {response.text}")
        return False

    except Exception as e:
        print(f"   ❌ Error: {e}")
        return False


def poll_for_result(url: str, filename: str, headers: dict, max_attempts: int = 60):
    """Poll for prediction result."""
    print("   ⏳ Processing...")

    for attempt in range(max_attempts):
        time.sleep(5)
        try:
            response = requests.get(url, headers=headers, timeout=30)
            if response.status_code == 200:
                result = response.json()
                status = result.get("status")

                if status == "succeeded":
                    output = result.get("output")
                    if output:
                        image_url = output[0] if isinstance(output, list) else output
                        img_response = requests.get(image_url, timeout=60)
                        if img_response.status_code == 200:
                            output_path = OUTPUT_DIR / filename
                            with open(output_path, "wb") as f:
                                f.write(img_response.content)
                            print(f"   ✅ Saved to: {output_path}")
                            return True

                elif status == "failed":
                    print(f"   ❌ Generation failed: {result.get('error')}")
                    return False

                print(f"   ⏳ Still processing... ({attempt + 1}/{max_attempts})")

        except Exception as e:
            print(f"   ⚠️ Poll error: {e}")

    print(f"   ❌ Timeout after {max_attempts} attempts")
    return False


def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    print(f"📁 Output directory: {OUTPUT_DIR.absolute()}")
    print(f"📝 Images to generate: {len(IMAGE_PROMPTS)}")

    success_count = 0
    for filename, prompt in IMAGE_PROMPTS.items():
        if generate_image(prompt, filename):
            success_count += 1
        time.sleep(2)

    print(f"\n✨ Complete! Generated {success_count}/{len(IMAGE_PROMPTS)} images")


if __name__ == "__main__":
    main()
```

### Step 2: Configure Prompts

Edit `IMAGE_PROMPTS` dictionary with your desired outputs:

```python
IMAGE_PROMPTS = {
    "hero-image.jpg": """Transform this product into a professional hero shot.
    Modern lighting, clean background, premium feel.""",

    "lifestyle-shot.jpg": """Show this product in a lifestyle setting.
    Natural environment, warm lighting, aspirational context.""",
}
```

### Step 3: Run the Script

```bash
source venv/bin/activate  # if using virtualenv
python generate_images.py
```

## Prompt Tips

For best results with nano-banana model:

1. **Be specific**: Describe the exact scene, lighting, and mood
2. **Reference the product**: Mention keeping the product's original appearance
3. **Include context**: Describe the environment and use case
4. **Style keywords**: Use terms like "professional photography", "high-end", "natural lighting"

## Example Prompts

```python
# Kitchen scene
"Show this food container in a professional kitchen with a chef preparing meals.
Warm lighting, stainless steel surfaces, busy restaurant atmosphere."

# Eco-friendly lifestyle
"Display this product in a natural setting with green plants and leaves.
Natural daylight, earth tones, sustainable lifestyle aesthetic."

# Freezer scene
"Show this container inside a professional freezer with frost effects.
Cool blue lighting, frozen food context, commercial kitchen setting."
```

## Output

Generated images are saved to `images/generated/` directory. Review and copy approved images to your final location.
