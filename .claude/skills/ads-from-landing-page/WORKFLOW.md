# Workflow: Ads from Landing Page

## Pre-flight Checklist

- [ ] Landing page exists and is accessible
- [ ] Local server running (`npx serve .` or similar)
- [ ] Chrome DevTools MCP connected
- [ ] Output directory exists for PNGs

## Step-by-Step Process

### 1. Analyze Landing Page (5 min)

```
Read the landing page HTML and CSS to extract:
```

**Colors:**
- [ ] Background color
- [ ] Primary accent color
- [ ] Secondary/rose color
- [ ] Text color
- [ ] Muted text color

**Typography:**
- [ ] Display/headline font
- [ ] Body font
- [ ] Accent/handwritten font (if any)
- [ ] Font weights used

**Visual Style:**
- [ ] Button style (rounded? shadow?)
- [ ] Badge/tag style
- [ ] Text highlight effects
- [ ] Image treatment (gradients, overlays)

**Brand Voice:**
- [ ] Formal or casual?
- [ ] Key phrases/slogans
- [ ] Call-to-action style

### 2. Gather Assets (2 min)

- [ ] Logo (SVG preferred)
- [ ] Product images (high-res)
- [ ] Lifestyle images
- [ ] Customer photos (if using social proof)

### 3. Plan Ad Variations (3 min)

Choose 5-15 messaging angles:

| # | Angle | Format | Message Hook |
|---|-------|--------|--------------|
| 1 | | 9:16 | |
| 2 | | 1:1 | |
| 3 | | 9:16 | |
| 4 | | 1:1 | |
| 5 | | 9:16 | |

### 4. Create ads.html (15-30 min)

```html
<!DOCTYPE html>
<html lang="fr">
<head>
  <!-- Fonts -->
  <link href="fonts..." rel="stylesheet">

  <style>
    /* Brand variables */
    :root { ... }

    /* Ad containers */
    .ad--1x1 { width: 1080px; height: 1080px; }
    .ad--9x16 { width: 1080px; height: 1920px; }

    /* Reusable components */
    .badge { ... }
    .cta { ... }
    .handwritten { ... }
    .underline { ... }
  </style>
</head>
<body>
  <!-- Ads here -->
</body>
</html>
```

### 5. Preview & Iterate (5 min)

- [ ] Open in browser
- [ ] Check text readability
- [ ] Verify image positioning
- [ ] Test on mobile viewport

### 6. Export PNGs (5 min)

For each ad:
```javascript
// In Chrome DevTools console or via MCP
const ads = document.querySelectorAll('.ad');
ads.forEach(ad => ad.style.display = 'none');
document.getElementById('ad-X').style.display = 'flex';
```

Then screenshot with `fullPage: true`.

### 7. Trim Margins (1 min)

Remove browser margins from exported PNGs:

```bash
# Install ImageMagick if needed
brew install imagemagick

# Trim all PNGs
cd assets/ads/
for f in *.png; do magick "$f" -trim "$f"; done
```

### 8. Final Review

- [ ] All PNGs exported
- [ ] File names are descriptive
- [ ] Images show key product features
- [ ] Text is readable at thumbnail size

## Quick Reference: CSS Snippets

### Gradient Overlay (dark bottom)
```css
background: linear-gradient(to bottom,
  transparent 40%,
  rgba(0,0,0,0.7) 100%
);
```

### Gradient Overlay (light bottom)
```css
background: linear-gradient(to bottom,
  transparent 30%,
  rgba(255,251,248,0.95) 70%
);
```

### Split Layout (1:1)
```css
display: grid;
grid-template-columns: 1fr 1fr;
```

### Vertical Stack (9:16)
```css
display: flex;
flex-direction: column;
```

### Center Content
```css
display: flex;
flex-direction: column;
justify-content: center;
align-items: center;
text-align: center;
```

## Naming Convention

```
XX-short-description.png

Examples:
01-problem-solution.png
02-product-feature.png
03-testimonial-sarah.png
04-promo-winter-sale.png
05-pack-family.png
```

## Common Issues & Fixes

| Issue | Fix |
|-------|-----|
| Image too zoomed | Adjust `background-position` |
| Text hard to read | Add/darken gradient overlay |
| CTA too small | Increase font-size and padding |
| Ad feels empty | Add trust badges or secondary text |
| Colors don't match | Check CSS variable inheritance |
