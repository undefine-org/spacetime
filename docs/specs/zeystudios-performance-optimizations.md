# Zey Studios Performance Optimization Spec

This document outlines all opportunities for optimizing load time and bundle size for the Zey Studios website.

## Why Performance Is Critical

Zey Studios is a fine art photography e-commerce site selling prints at $95-$395. Performance directly impacts revenue:

### The Business Case

1. **Visual experience IS the product** - Art buyers make emotional purchases. Slow loading breaks the contemplative spell and undermines premium positioning.

2. **Conversion economics** - At $95-$395 per sale:
   - 53% of mobile users abandon sites >3 seconds (Google)
   - 1 second delay = 7% conversion reduction (Aberdeen)
   - Current payload: ~15MB (15+ seconds on 3G)

3. **Mobile-first audience** - Photography buyers discover on Instagram/Pinterest on phones, then purchase. Current images aren't responsive (1400px files on 375px screens).

4. **Competitive pressure** - Society6, Minted, Saatchi Art all have sub-3-second loads with progressive image loading.

5. **The "spell" factor** - Spacetime animations are designed for "cinematic contemplation" (800-1200ms durations). White flashes from loading images destroy immersion.

---

## Current State

| Resource | Size | Issue |
|----------|------|-------|
| **Images** | ~15MB (24 JPEGs) | All load immediately, no lazy loading |
| **Hero Image** | 456KB | Not preloaded, delays LCP |
| **Image Formats** | JPEG only | Missing WebP/AVIF (25-50% savings) |
| **Responsive Images** | None | Full-size images on all devices |
| **CSS** | 34KB unminified | Render-blocking |
| **Critical CSS** | Not inlined | Delays FCP |

---

## Optimization Tiers

### Tier 1: Critical Path (Quick Wins)

These require no build tooling changes and provide the largest immediate impact.

#### 1.1 Lazy Loading Images

Add `loading="lazy"` to all images except the hero.

**Before:**
```html
<img src="/prints/IN-004.jpg" alt="The Gathering">
```

**After:**
```html
<img src="/prints/IN-004.jpg" loading="lazy" decoding="async" alt="The Gathering">
```

**Files:** `zeystudios/index.html`, `zeystudios/templates.html`

**Impact:** Initial payload drops from ~15MB to ~500KB (hero + visible gallery items)

#### 1.2 Preload Hero Image

Add to `<head>` before stylesheets:

```html
<link rel="preload" as="image" href="/prints/IN-003.jpg" fetchpriority="high">
```

Add to hero `<img>`:
```html
<img src="/prints/IN-003.jpg" fetchpriority="high" alt="...">
```

**File:** `zeystudios/index.html`

**Impact:** Hero appears 200-400ms faster (improved LCP)

#### 1.3 Async Image Decoding

Add `decoding="async"` to all images:

```html
<img src="..." decoding="async" loading="lazy" alt="...">
```

**Impact:** Image decoding doesn't block main thread

---

### Tier 2: Responsive Images

Generate multiple sizes and serve appropriately sized images to each device.

#### 2.1 Generate Image Variants

Create 400w, 800w, 1200w versions of each image:

```bash
# Example with ImageMagick
for img in prints/*.jpg; do
  convert "$img" -resize 400x "$img:r-400w.jpg"
  convert "$img" -resize 800x "$img:r-800w.jpg"
  convert "$img" -resize 1200x "$img:r-1200w.jpg"
done
```

Or with Sharp (Node.js):

```javascript
const sharp = require('sharp');

async function generateResponsive(inputPath) {
  const widths = [400, 800, 1200];
  for (const w of widths) {
    await sharp(inputPath)
      .resize(w)
      .jpeg({ quality: 80 })
      .toFile(inputPath.replace('.jpg', `-${w}w.jpg`));
  }
}
```

#### 2.2 Update HTML with srcset

**Before:**
```html
<img src="/prints/IN-003.jpg" alt="Between Worlds">
```

**After:**
```html
<img
  src="/prints/IN-003.jpg"
  srcset="/prints/IN-003-400w.jpg 400w,
          /prints/IN-003-800w.jpg 800w,
          /prints/IN-003-1200w.jpg 1200w,
          /prints/IN-003.jpg 1500w"
  sizes="(max-width: 768px) 100vw,
         (max-width: 1200px) 50vw,
         600px"
  loading="lazy"
  decoding="async"
  alt="Between Worlds">
```

**Impact:** Mobile users download 100-200KB instead of 500KB per image (60-80% reduction)

---

### Tier 3: Modern Image Formats

WebP and AVIF offer significant compression improvements over JPEG.

#### 3.1 Generate WebP/AVIF Variants

```javascript
// Sharp example
await sharp(inputPath)
  .resize(800)
  .webp({ quality: 80 })
  .toFile(inputPath.replace('.jpg', '-800w.webp'));

await sharp(inputPath)
  .resize(800)
  .avif({ quality: 65 })
  .toFile(inputPath.replace('.jpg', '-800w.avif'));
```

#### 3.2 Use `<picture>` for Format Negotiation

```html
<picture>
  <source
    type="image/avif"
    srcset="/prints/IN-003-400w.avif 400w,
            /prints/IN-003-800w.avif 800w"
    sizes="(max-width: 768px) 100vw, 50vw">
  <source
    type="image/webp"
    srcset="/prints/IN-003-400w.webp 400w,
            /prints/IN-003-800w.webp 800w"
    sizes="(max-width: 768px) 100vw, 50vw">
  <img
    src="/prints/IN-003.jpg"
    srcset="/prints/IN-003-400w.jpg 400w,
            /prints/IN-003-800w.jpg 800w"
    sizes="(max-width: 768px) 100vw, 50vw"
    loading="lazy"
    decoding="async"
    alt="Between Worlds">
</picture>
```

**Impact:**
- WebP: ~25% smaller than JPEG at equivalent quality
- AVIF: ~50% smaller (but slower to decode, less browser support)

#### 3.3 Progressive JPEG Encoding

If staying with JPEG, re-encode as progressive:

```javascript
await sharp(inputPath)
  .jpeg({ quality: 80, progressive: true })
  .toFile(outputPath);
```

**Impact:** Image appears blurry-then-sharp instead of top-to-bottom (perceived performance)

---

### Tier 4: CSS Optimization

#### 4.1 Critical CSS Inlining

Extract above-the-fold styles and inline in `<head>`:

```html
<head>
  <style>
    /* Critical CSS: nav, hero, initial gallery (~5KB) */
    .zey-nav { ... }
    .zey-hero { ... }
    .zey-gallery { ... }
  </style>

  <!-- Async load remaining CSS -->
  <link rel="preload" href="/zeystudios.css" as="style" onload="this.onload=null;this.rel='stylesheet'">
  <noscript><link rel="stylesheet" href="/zeystudios.css"></noscript>
</head>
```

**Tools for extraction:**
- [critical](https://github.com/addyosmani/critical) (Node.js)
- [penthouse](https://github.com/pocketjoso/penthouse) (Node.js)

**Impact:** First Contentful Paint 100-300ms faster

#### 4.2 CSS Minification

Minify CSS files:

```bash
# Using cssnano via postcss
npx postcss zeystudios.css -o zeystudios.min.css
```

**Impact:** 34KB → ~22KB (35% reduction)

#### 4.3 Remove Unused CSS

Use PurgeCSS to remove unused selectors:

```javascript
// postcss.config.js
module.exports = {
  plugins: [
    require('@fullhuman/postcss-purgecss')({
      content: ['./zeystudios/**/*.html'],
    }),
    require('cssnano'),
  ],
};
```

---

### Tier 5: Font Optimization

#### 5.1 Font Subsetting

Only load characters actually used on the page:

```html
<!-- Current (loads all characters) -->
<link href="https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,400;0,500;0,600;1,400&display=swap" rel="stylesheet">

<!-- Subset to Latin only -->
<link href="https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,400;0,500;0,600;1,400&display=swap&subset=latin" rel="stylesheet">
```

Or use `&text=` parameter for extreme subsetting (only specific characters).

**Impact:** Font payload reduced 50-70%

#### 5.2 Self-Host Fonts

Download and serve fonts locally:
- Eliminates external request latency
- Enables better caching control
- Allows custom subsetting

**Tools:** [google-webfonts-helper](https://gwfh.mranftl.com/fonts)

#### 5.3 Font Display Strategy

Already using `display=swap` - verify it's working:

```css
@font-face {
  font-family: 'Playfair Display';
  font-display: swap; /* Shows fallback immediately, swaps when loaded */
}
```

---

### Tier 6: Server Configuration

#### 6.1 HTTP Caching Headers

Configure Axum/Spacetime to send appropriate headers:

```
# Immutable versioned assets (images, fonts)
Cache-Control: public, max-age=31536000, immutable

# HTML (short cache, revalidate)
Cache-Control: public, max-age=86400, must-revalidate

# CSS/JS (medium cache)
Cache-Control: public, max-age=604800
```

#### 6.2 Compression (Brotli/Gzip)

Verify server compresses text responses:

```
# Check with curl
curl -H "Accept-Encoding: br, gzip" -I https://zeystudios.com/zeystudios.css
```

Expected savings:
- CSS: 34KB → ~8KB (Brotli)
- HTML: 9KB → ~3KB (Brotli)
- JS: 4KB → ~1.5KB (Brotli)

#### 6.3 HTTP/2 or HTTP/3

Enable multiplexing for parallel asset loading over single connection.

#### 6.4 CDN Deployment

Consider serving static assets via CDN:
- Cloudflare, Fastly, or AWS CloudFront
- Geographic distribution reduces latency
- Automatic compression and caching

---

### Tier 7: Advanced Optimizations

#### 7.1 Content Visibility

Skip rendering off-screen sections:

```css
.zey-story,
.zey-gift,
.zey-footer {
  content-visibility: auto;
  contain-intrinsic-size: 0 500px;
}
```

**Impact:** Browser skips rendering below-fold content initially

#### 7.2 Contain Property

Isolate layout calculations:

```css
.zey-print {
  contain: layout style;
}
```

#### 7.3 Preconnect to Required Origins

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="preconnect" href="https://cdn.jsdelivr.net">
```

#### 7.4 DNS Prefetch for Analytics/Third-Party

```html
<link rel="dns-prefetch" href="https://www.google-analytics.com">
```

#### 7.5 Resource Hints for Navigation

Prefetch next likely page:

```html
<link rel="prefetch" href="/cart">
```

---

## Implementation Priority

| Phase | Tasks | Effort | Impact |
|-------|-------|--------|--------|
| **1** | Lazy loading, hero preload, async decode | 30 min | **70%** |
| **2** | Responsive images (srcset) | 2-3 hrs | **40-60%** additional |
| **3** | WebP/AVIF formats | 2-3 hrs | **25-50%** additional |
| **4** | Critical CSS, minification | 1-2 hrs | **10-15%** |
| **5** | Server caching, compression | 1 hr | **Repeat visits** |
| **6** | Font subsetting, content-visibility | 1 hr | **5-10%** |

---

## Expected Results

| Metric | Before | After Phase 1 | After All |
|--------|--------|---------------|-----------|
| Initial Payload | ~15MB | ~1MB | ~300KB |
| LCP (3G) | 15+ sec | ~4 sec | ~2 sec |
| FCP | ~3 sec | ~2 sec | ~1 sec |
| Lighthouse Mobile | ~20 | ~60 | ~90 |

---

## Measurement

Before and after each phase:

1. **Lighthouse** - Performance score, LCP, FCP, CLS, TBT
2. **WebPageTest** - Waterfall, filmstrip, Speed Index
3. **Chrome DevTools Network** - Payload size, request count, timing
4. **Real Device** - Actual phone on throttled 3G connection

---

## Files to Modify

### Immediate (Tier 1)
- `zeystudios/index.html` - Preload, lazy loading attributes
- `zeystudios/templates.html` - Image attributes in templates

### Image Pipeline (Tiers 2-3)
- New: `scripts/optimize-images.js` or build configuration
- `zeystudios/prints/` - Generated responsive/format variants

### CSS (Tier 4)
- `zeystudios/zeystudios.css` - Minification, content-visibility
- `zeystudios/index.html` - Critical CSS inline

### Server (Tiers 5-6)
- Axum/Spacetime configuration - Headers, compression
