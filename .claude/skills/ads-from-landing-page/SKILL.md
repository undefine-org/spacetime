---
name: ads-from-landing-page
description: Generate marketing ads from landing pages. Creates social media ads (Instagram, Facebook, TikTok) in 1:1 and 9:16 formats matching the brand's visual identity. Use when creating ad campaigns, promotional content, social media creatives, or marketing materials from an existing website or landing page.
---

# Ads from Landing Page

Generate professional marketing ads that match your brand's visual identity by analyzing an existing landing page.

## What This Skill Does

1. **Analyzes** a landing page to extract brand styling (colors, typography, tone)
2. **Creates** multiple ad variations with different messaging angles
3. **Outputs** HTML ads in standard formats (1:1 feed, 9:16 stories/reels)
4. **Exports** high-quality PNGs ready for social media

## Quick Start

Ask Claude:
- "Create ads for my landing page at index.html"
- "Generate Instagram ads from my website"
- "Make 5 ad variations for my product page"

## Workflow

### Step 1: Analyze the Landing Page

Read the landing page HTML/CSS to extract:
- **Colors**: Background, accent, text colors (CSS variables or inline)
- **Typography**: Font families, weights, sizes
- **Tone**: Brand voice, key messages, CTAs
- **Images**: Product photos, lifestyle images, logos

### Step 2: Create Ad HTML File

Create an `ads.html` file with:
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <!-- Same fonts as landing page -->
  <link href="https://fonts.googleapis.com/css2?family=..." rel="stylesheet">
  <style>
    /* Brand colors as CSS variables */
    :root {
      --bg: #FFFBF8;
      --accent: #E85A4F;
      --text: #2D3436;
    }

    /* Ad containers */
    .ad--1x1 { width: 1080px; height: 1080px; }
    .ad--9x16 { width: 1080px; height: 1920px; }
  </style>
</head>
<body>
  <!-- Individual ads with unique IDs -->
  <div class="ad ad--9x16" id="ad-1">...</div>
  <div class="ad ad--1x1" id="ad-2">...</div>
</body>
</html>
```

### Step 3: Design Ad Variations

Create ads with different messaging angles:

| Angle | Description | Best For |
|-------|-------------|----------|
| Problem/Solution | Address pain point, show solution | Awareness |
| Product Showcase | Highlight key feature | Consideration |
| Social Proof | Testimonials, reviews | Trust |
| Emotional/Story | Brand narrative, connection | Engagement |
| Offer/Promo | Discount, limited time | Conversion |
| Urgency | Scarcity, countdown | Action |

### Step 4: Export to PNG

Use Chrome DevTools MCP to capture each ad:

```javascript
// Hide all ads, show one at a time
const ads = document.querySelectorAll('.ad');
ads.forEach(ad => ad.style.display = 'none');
document.getElementById('ad-1').style.display = 'flex';
```

Then screenshot with `fullPage: true` to the output directory.

## Ad Formats

### 1:1 Square (1080x1080)
- Instagram Feed
- Facebook Feed
- LinkedIn Feed
- Twitter/X

### 9:16 Vertical (1080x1920)
- Instagram Stories/Reels
- TikTok
- Facebook Stories
- YouTube Shorts

## Brand Style Elements to Replicate

### Typography
- **Display font**: For headlines (bold, impactful)
- **Body font**: For descriptions (readable)
- **Handwritten/Accent**: For emphasis (cursive, rotated)

### Visual Effects
- **Underline highlights**: Colored bar behind text
- **Badges**: Uppercase, letter-spacing, rounded corners
- **CTAs**: Bold buttons with brand accent color
- **Gradients**: Overlay on images for text readability

### Common CSS Patterns
```css
/* Handwritten accent */
.handwritten {
  font-family: 'Caveat', cursive;
  transform: rotate(-2deg);
  color: var(--accent);
}

/* Pink underline highlight */
.underline::after {
  content: '';
  position: absolute;
  bottom: 4px;
  left: -6px;
  right: -6px;
  height: 12px;
  background: var(--rose);
  opacity: 0.6;
  z-index: -1;
}

/* Badge style */
.badge {
  text-transform: uppercase;
  letter-spacing: 0.1em;
  padding: 12px 24px;
  border-radius: 8px;
}
```

### Step 5: Trim Margins

After exporting, use ImageMagick to remove gray margins around the ads:

```bash
# Install ImageMagick if needed
brew install imagemagick

# Trim all PNGs in the output folder
cd assets/ads/
for f in *.png; do magick "$f" -trim "$f"; done
```

This removes the browser chrome/margins and keeps only the ad content.

## Output Structure

```
project/
├── ads.html              # All ads in one HTML file
└── assets/ads/
    ├── 01-problem-solution.png
    ├── 02-product-showcase.png
    ├── 03-social-proof.png
    └── ...
```

## Tips for Effective Ads

### Text Size
- Headlines: 60-100px (must be readable on mobile)
- Subtitles: 28-38px
- CTAs: 24-32px with generous padding
- Badges: 18-24px

### Image Guidelines
- Use high-quality product photos
- Show the product in use (lifestyle)
- Ensure key features are visible
- Position images to leave room for text

### Copy Guidelines
- Lead with benefit, not feature
- Use "you/your" language
- Create urgency without being pushy
- Include clear CTA

## Example Prompts

1. **Basic**: "Create 5 ads for my landing page"
2. **Specific**: "Generate Instagram story ads highlighting our product's unique opening feature"
3. **Themed**: "Make holiday-themed ads with gift messaging"
4. **Targeted**: "Create ads focusing on pain points our customers face"
5. **Promo**: "Generate ads for our winter sale with pricing"

## Requirements

- Chrome DevTools MCP for screenshots
- Local server running (e.g., `npx serve .`)
- Landing page with extractable brand styling

## Related Files

After running this skill, you'll have:
- `ads.html` - Viewable at `http://localhost:PORT/ads.html`
- `assets/ads/*.png` - Ready-to-upload ad images
