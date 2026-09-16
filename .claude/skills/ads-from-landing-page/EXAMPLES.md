# Examples: Ads from Landing Page

## Real Example: J'allète Breastfeeding Clothing

### Brand Analysis

**Extracted from landing page:**
```css
/* Colors */
--bg: #FFFBF8;        /* Warm white */
--accent: #E85A4F;    /* Coral red */
--rose: #FABCB7;      /* Powder pink */
--text: #2D3436;      /* Warm charcoal */

/* Typography */
font-family: 'Outfit', sans-serif;     /* Display/Headlines */
font-family: 'Inter', sans-serif;      /* Body text */
font-family: 'Caveat', cursive;        /* Handwritten accents */
```

**Brand tone:** Warm, supportive, empowering for new mothers

### Ad Variations Created

#### 1. Problem/Solution (9:16)
**Message:** "Tu en as marre de te cacher pour allaiter ?"
**Visual:** Lifestyle photo with gradient overlay
**CTA:** "Découvre nos packs"

```html
<div class="ad ad--9x16" id="ad-1">
  <div class="ad-image"></div>
  <div class="ad-content">
    <h1 class="ad-title">
      Tu en as marre de <span class="underline">te cacher</span><br>
      <span class="handwritten">pour allaiter ?</span>
    </h1>
    <p class="ad-subtitle">
      Des vêtements avec une ouverture invisible.<br>
      Personne ne voit rien. Toi seule sais.
    </p>
    <span class="cta">Découvre nos packs →</span>
  </div>
</div>
```

#### 2. Product Showcase (1:1)
**Message:** "Une ouverture invisible"
**Visual:** Split layout - product image | text
**CTA:** "Voir les packs"

#### 3. Social Proof (1:1)
**Message:** "Elles ont adopté J'allète"
**Visual:** Customer photo with quote
**CTA:** "Rejoins-les"

#### 4. Mystery/Feature Focus (1:1)
**Message:** "Tu vois l'ouverture ? Non."
**Visual:** Product demo showing invisible opening
**CTA:** "Découvrir"

#### 5. Urgency/Promo (9:16)
**Message:** "-151 dhs sur le Pack Winter Family"
**Visual:** Product with price callout
**CTA:** "J'en profite maintenant"

### Messaging Angles Used

| # | Angle | Hook | Target Emotion |
|---|-------|------|----------------|
| 1 | Pain Point | "Tu en as marre..." | Frustration → Relief |
| 2 | Feature | "Ouverture invisible" | Curiosity |
| 3 | Social Proof | "Elles ont adopté" | FOMO, Trust |
| 4 | Mystery | "Tu vois l'ouverture? Non." | Intrigue |
| 5 | Promo | "Offre limitée" | Urgency |
| 6 | Gift | "Le cadeau parfait" | Generosity |
| 7 | Empathy | "On sait ce que tu vis" | Understanding |
| 8 | Value | "3 pièces, 1 prix malin" | Smart shopping |
| 9 | Freedom | "Fini de te cacher" | Liberation |
| 10 | Testimonial | "C'était un cauchemar" | Relatability |

### Export Results

```
jallete/assets/ads-v2/
├── 01-marre-te-cacher-allaiter.png    (9:16)
├── 02-ouverture-invisible.png         (1:1)
├── 03-cree-par-une-soeur.png          (9:16)
├── 04-elles-ont-adopte-jallete.png    (1:1)
├── 05-pack-duo-maman-papa.png         (9:16)
├── 06-cadeau-nouvelles-mamans.png     (1:1)
├── 07-te-cacher-epuisant.png          (9:16)
├── 08-pack-3-pieces-prix-malin.png    (1:1)
├── 09-fini-te-cacher.png              (9:16)
├── 10-pack-famille-fetes.png          (1:1)
├── 11-temoignage-cauchemar.png        (9:16)
├── 12-offre-limitee-151dhs.png        (1:1)
├── 13-galeres-allaitement.png         (9:16)
├── 14-pack-mama-cozy.png              (1:1)
├── 15-pack-cocoon-maman-papa.png      (9:16)
├── 16-2-secondes-allaiter.png         (9:16)
├── 17-secret-jallete.png              (1:1)
├── 18-zero-compromis.png              (9:16)
├── 19-nouveau-basique.png             (1:1)
└── 20-offre-flash-mama-cozy.png       (9:16)
```

## Ad Templates by Use Case

### Awareness Ads
Focus on the problem, introduce solution.

```html
<div class="ad ad--9x16">
  <div class="ad-image" style="background-image: url('lifestyle.jpg');"></div>
  <div class="ad-overlay"></div>
  <div class="ad-content">
    <p class="eyebrow">ON SAIT CE QUE TU VIS</p>
    <h1 class="ad-title">
      [Pain point statement]<br>
      <span class="handwritten">[Emotional hook]</span>
    </h1>
    <p class="ad-subtitle">[Empathy statement]</p>
    <span class="cta">Découvre la solution →</span>
  </div>
</div>
```

### Consideration Ads
Highlight unique features.

```html
<div class="ad ad--1x1" style="display: grid; grid-template-columns: 1fr 1fr;">
  <div class="ad-image" style="background-image: url('product.jpg');"></div>
  <div class="ad-content">
    <span class="badge">NOUVEAU</span>
    <h1 class="ad-title">[Feature name]</h1>
    <p class="ad-subtitle">[Benefit explanation]</p>
    <p class="price">À partir de <span class="price-new">[Price]</span></p>
    <span class="cta">Voir les modèles</span>
  </div>
</div>
```

### Conversion Ads
Urgency + clear offer.

```html
<div class="ad ad--9x16">
  <div class="ad-header" style="background: var(--accent);">
    <p>⏰ OFFRE LIMITÉE</p>
  </div>
  <div class="ad-image" style="background-image: url('product.jpg');"></div>
  <div class="ad-content">
    <h1 class="ad-title">[Pack/Product name]</h1>
    <div class="price-block">
      <span class="price-old">[Original price]</span>
      <span class="price-new">[Sale price]</span>
      <span class="savings">[Savings amount]</span>
    </div>
    <span class="cta">J'en profite maintenant →</span>
  </div>
</div>
```

## Common Iterations

### Version 1 → Version 2: Bigger Text
When ads need more impact for social feeds:
- Headlines: +25-40% size increase
- CTAs: Larger padding, bigger font
- Badges: More prominent

### Image Positioning
When product feature is cut off:
```css
/* Adjust to show key area */
background-position: center 30%;  /* Show more of middle */
background-position: center 40%;  /* Even lower */
```

### Multiple Variations of Same Concept
When testing different angles with same image:
1. Speed focus: "2 secondes"
2. Mystery: "Tu vois l'ouverture? Non."
3. Style: "Zéro compromis"
4. Everyday: "Ton nouveau basique"
5. Promo: "Offre Flash"
