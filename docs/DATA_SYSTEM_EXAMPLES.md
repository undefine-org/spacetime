# Data Binding Examples

Complete examples showing how to migrate each site to the declarative data binding system.

---

## Table of Contents

1. [zeystudios](#1-zeystudios)
2. [ikarchitecte](#2-ikarchitecte)
3. [jallete](#3-jallete)

---

## 1. zeystudios

A photography print shop with galleries, curations, and a shopping cart.

### Type Definitions

```css
/* zeystudios.st */

/* ============================================
   TYPE DEFINITIONS
   ============================================ */

@type Price {
    S: number;
    M: number;
    L: number;
}

@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: Price;
    featured?: boolean;
    tags?: string[];
}

@type Curation {
    id: string;
    slug: string;
    title: string;
    description: string;
    hero: url;
    accent: color;
    prints: string[];       /* Array of Print IDs */
}

@type CartItem {
    printId: string;
    size: "S" | "M" | "L";
    quantity: number;
}
```

### Data Sources

```css
/* ============================================
   DATA SOURCES
   ============================================ */

@data fetch $prints Print[] : "/data/prints.json";

@data fetch $curations Curation[] : "/data/curations.json";

@data fetch $cart CartItem[] : localStorage("zey-cart") {
    initial: [];
};
```

### Computed Data & Helper Functions

```css
/* ============================================
   COMPUTED DATA
   ============================================ */

@data query $featuredPrints Print[] from $prints {
    where: $.featured == true;
    limit: 6;
}

@data fold $cartTotal number from $cart : acc + priceFor(item.printId, item.size) * item.quantity;

@data fold $cartCount number from $cart : acc + item.quantity;

/* ============================================
   HELPER FUNCTIONS
   ============================================ */

@fn getPrint(id: string): Print? {
    return $prints.find(p => p.id == id);
}

@fn priceFor(printId: string, size: "S" | "M" | "L"): number {
    let print = getPrint(printId);
    return print?.prices[size] ?? 0;
}
```

### Gallery Binding

```css
/* ============================================
   GALLERY PAGE
   ============================================ */

.zey-gallery {
    data-state: $prints_loading ? "loading" : ($prints.length == 0 ? "empty" : "ready");

    @state(when: "loading") {
        min-height: 400px;
        opacity: 0.5;
    }

    @state(when: "ready") {
        opacity: 1;
    }

    @state(when: "empty") {
        min-height: 200px;
    }

    @each($prints as $print) {
        template: "zey-print";

        [slot="image"] {
            src: $print.image;
            alt: $print.title;
            loading: "lazy";
        }
        [slot="title"]: $print.title;
        [slot="subtitle"]: $print.subtitle;

        :host {
            data-id: $print.id;
            data-prices: $print.prices | json;
            data-featured: $print.featured | default(false);
        }
    }

    /* Stagger animation for cards */
    > zey-print {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.08 first;
        }

        @on &.hover lift(300ms) {
            translate-y: 0 -> -8px;
            box-shadow: 0 4px 20px rgba(0,0,0,0.1) -> 0 12px 40px rgba(0,0,0,0.15);

            .zey-print__image img {
                scale: 1 -> 1.05;
            }
        }
    }
}
```

### Curations Binding

```css
/* ============================================
   CURATIONS GRID (Home Page)
   ============================================ */

.curations-grid {
    @each($curations as $curation) {
        template: "zey-curation-card";

        [slot="image"] {
            src: $curation.hero;
            alt: $curation.title;
        }
        [slot="title"]: $curation.title;
        [slot="description"]: $curation.description;

        :host {
            data-slug: $curation.slug;
            style: "--accent-color: " + $curation.accent;
        }
    }

    > zey-curation-card {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 60px -> 0;
            stagger: 0.12 first;
        }
    }
}
```

### Cart Binding

```css
/* ============================================
   CART DRAWER
   ============================================ */

$cartStatus <- $cart_loading ? "checking" : ($cart.length == 0 ? "empty" : "has-items");

.cart-items {
    data-state: $cartStatus;

    .is-loading: $cartStatus == "checking";
    .is-empty: $cartStatus == "empty";
    .has-items: $cartStatus == "has-items";

    @state(when: "empty") {
        /* Show empty cart message */
    }

    @state(when: "has-items") {
        /* Show cart items */
    }

    @each($cart as $item) {
        template: "zey-cart-item";

        /* Cross-reference: look up print by ID */
        @let print = getPrint($item.printId);

        [slot="image"] {
            src: print.image;
            alt: print.title;
        }
        [slot="title"]: print.title;
        [slot="size"]: $item.size;
        [slot="quantity"]: $item.quantity;
        [slot="price"]: priceFor($item.printId, $item.size) * $item.quantity | currency("$");

        :host {
            data-print-id: $item.printId;
            data-size: $item.size;
        }
    }
}

.cart-total {
    @bind {
        text: $cartTotal | currency("$");
    }
}

.cart-count {
    @bind {
        text: $cartCount;
        class: $cartCount > 0 ? "has-items" : "empty";
    }
}
```

### JSON Data Files

```json
// /data/prints.json
[
    {
        "id": "IN-003",
        "title": "Between Worlds",
        "subtitle": "Surfer in morning mist, Morocco",
        "image": "/prints/IN-003.jpg",
        "prices": { "S": 95, "M": 195, "L": 395 },
        "featured": true,
        "tags": ["morocco", "surf", "mist"]
    },
    {
        "id": "IN-004",
        "title": "Golden Hour",
        "subtitle": "Desert dunes at sunset",
        "image": "/prints/IN-004.jpg",
        "prices": { "S": 95, "M": 195, "L": 395 },
        "featured": false,
        "tags": ["desert", "sunset"]
    },
    {
        "id": "IN-007",
        "title": "Medina Blues",
        "subtitle": "Traditional doorway in Chefchaouen",
        "image": "/prints/IN-007.jpg",
        "prices": { "S": 95, "M": 195, "L": 395 },
        "featured": true,
        "tags": ["morocco", "architecture", "blue"]
    }
]
```

```json
// /data/curations.json
[
    {
        "id": "moroccan-soul",
        "slug": "moroccan-soul",
        "title": "Moroccan Soul",
        "description": "Vibrant colors and textures from the heart of Morocco",
        "hero": "/curations/moroccan-hero.jpg",
        "accent": "#d4a574",
        "prints": ["IN-003", "IN-007", "IN-012"]
    },
    {
        "id": "twilight-gradient",
        "slug": "twilight-gradient",
        "title": "Twilight Gradient",
        "description": "The magic hour captured across continents",
        "hero": "/curations/twilight-hero.jpg",
        "accent": "#7b5ea7",
        "prints": ["IN-004", "IN-008", "IN-015"]
    }
]
```

### Simplified HTML

```html
<!-- gallery.html — Before: 23 repeated zey-print elements -->
<!-- gallery.html — After: -->
<section class="gallery-section">
    <h1>All Prints</h1>
    <div class="zey-gallery">
        <!-- Populated by Spacetime -->
    </div>
</section>
```

```html
<!-- index.html curations section -->
<section class="curations-section">
    <h2>Curated Collections</h2>
    <div class="curations-grid">
        <!-- Populated by Spacetime -->
    </div>
</section>
```

---

## 2. ikarchitecte

An architecture firm portfolio with projects and services.

### Type Definitions

```css
/* animations.st */

/* ============================================
   TYPE DEFINITIONS
   ============================================ */

@type Project {
    id: string;
    title: string;
    location: string;
    type: string;           /* "Rénovation intérieure", "Construction neuve", etc. */
    image: url;
    slug?: string;
    featured?: boolean;
}

@type Service {
    id: string;
    number: string;         /* "01", "02", "03" */
    title: string;
    description: string;
}

@type NavLink {
    href: string;
    label: string;
}
```

### Data Sources

```css
/* ============================================
   DATA SOURCES
   ============================================ */

@data fetch $projects Project[] : "/data/projects.json";

@data fetch $services Service[] : "/data/services.json";

@data inline $navLinks : [
    { "href": "#projects", "label": "Réalisations" },
    { "href": "#philosophy", "label": "Approche" },
    { "href": "#services", "label": "Savoir-faire" },
    { "href": "#contact", "label": "Contact" }
];
```

### Computed Data

```css
/* ============================================
   COMPUTED DATA
   ============================================ */

@data query $featuredProjects Project[] from $projects {
    where: $.featured == true;
    limit: 6;
}
```

### Projects Grid Binding

```css
/* ============================================
   PROJECTS GRID
   ============================================ */

$projectsStatus <- $projects_loading ? "loading" : "ready";

.ik-projects-grid {
    data-state: $projectsStatus;

    .is-loading: $projectsStatus == "loading";
    .is-ready: $projectsStatus == "ready";

    @state(when: "loading") {
        min-height: 600px;
    }

    @state(when: "ready");

    @each($projects as $project) {
        template: "ik-project";

        [slot="image"] {
            src: $project.image;
            alt: $project.title;
            loading: "lazy";
        }
        [slot="location"]: $project.location;
        [slot="title"]: $project.title;
        [slot="type"]: $project.type;

        :host {
            data-id: $project.id;
            data-slug: $project.slug | default($project.id);
        }
    }

    > ik-project {
        @scroll project-reveal(&reveal) {
            opacity: 0 -> 1;
            translate-y: 80px -> 0;
            easing: &ease-out-expo;
            stagger: 0.15 first;
        }

        @on &.hover project-hover(400ms) {
            translate-y: 0 -> -8px;

            .ik-project__image img {
                scale: 1 -> 1.05;
            }
        }
    }
}
```

### Services Binding

```css
/* ============================================
   SERVICES SECTION
   ============================================ */

.ik-services-list {
    @each($services as $service) {
        template: "ik-service";

        [slot="number"]: $service.number;
        [slot="title"]: $service.title;
        [slot="desc"]: $service.description;

        :host {
            data-service-id: $service.id;
        }
    }

    > ik-service {
        @scroll service-reveal(&reveal) {
            opacity: 0 -> 1;
            translate-x: -40px -> 0;
            stagger: 0.2 first;
        }
    }
}
```

### Navigation Binding

```css
/* ============================================
   NAVIGATION
   ============================================ */

.ik-nav__links {
    @each($navLinks as $link) {
        template: "ik-nav-link";

        [slot="link"] {
            href: $link.href;
            text: $link.label;
        }
    }
}

.ik-footer__links {
    @each($navLinks as $link) {
        template: "ik-footer-link";

        [slot="link"] {
            href: $link.href;
            text: $link.label;
        }
    }
}
```

### JSON Data Files

```json
// /data/projects.json
[
    {
        "id": "appartement-m",
        "title": "Appartement M",
        "location": "Casablanca, Maroc",
        "type": "Rénovation intérieure",
        "image": "/images/projects/appartement-m.jpg",
        "featured": true
    },
    {
        "id": "villa-s",
        "title": "Villa S",
        "location": "Marrakech, Maroc",
        "type": "Construction neuve",
        "image": "/images/projects/villa-s.jpg",
        "featured": true
    },
    {
        "id": "riad-k",
        "title": "Riad K",
        "location": "Fès, Maroc",
        "type": "Restauration patrimoine",
        "image": "/images/projects/riad-k.jpg",
        "featured": true
    }
]
```

```json
// /data/services.json
[
    {
        "id": "architecture",
        "number": "01",
        "title": "Architecture",
        "description": "Conception architecturale complète, de l'esquisse à la livraison. Nous créons des espaces qui dialoguent avec leur environnement."
    },
    {
        "id": "interior-design",
        "number": "02",
        "title": "Design d'intérieur",
        "description": "Aménagement et décoration intérieure. Nous harmonisons matériaux, couleurs et lumière pour créer des ambiances uniques."
    },
    {
        "id": "renovation",
        "number": "03",
        "title": "Rénovation",
        "description": "Transformation et réhabilitation de l'existant. Nous révélons le potentiel caché de chaque espace."
    }
]
```

### Simplified HTML

```html
<!-- index.html — Before: 6 repeated ik-project elements -->
<!-- index.html — After: -->
<section id="projects" class="ik-projects">
    <div class="ik-container">
        <h2 class="ik-section__title">Réalisations</h2>
        <div class="ik-projects-grid">
            <!-- Populated by Spacetime -->
        </div>
    </div>
</section>

<section id="services" class="ik-services">
    <div class="ik-container">
        <h2 class="ik-section__title">Savoir-faire</h2>
        <div class="ik-services-list">
            <!-- Populated by Spacetime -->
        </div>
    </div>
</section>
```

---

## 3. jallete

A nursing wear e-commerce site with product packs, features, testimonials, and FAQ.

### Type Definitions

```css
/* animations.st */

/* ============================================
   TYPE DEFINITIONS
   ============================================ */

@type PackItem {
    name: string;
    price: number;
}

@type Pack {
    id: string;
    title: string;
    badge?: string;         /* "Économisez 81 dhs" */
    image: url;
    items: PackItem[];
    originalPrice: number;
    price: number;
    wooCommerceId?: string;
}

@type Feature {
    id: string;
    icon: url;
    title: string;
    description: string;
}

@type Testimonial {
    id: string;
    quote: string;
    avatar: string;         /* Initial letter like "S" */
    name: string;
    role: string;           /* "Maman de 2 enfants, Casablanca" */
}

@type FAQ {
    id: string;
    question: string;
    answer: string;
}

@type TrustItem {
    icon: url;
    text: string;
}

@type CartItem {
    packId: string;
    size: "S" | "M" | "L" | "XL";
    quantity: number;
}
```

### Data Sources

```css
/* ============================================
   DATA SOURCES
   ============================================ */

@data fetch $packs Pack[] : "/data/packs.json";

@data fetch $features Feature[] : "/data/features.json";

@data fetch $testimonials Testimonial[] : "/data/testimonials.json";

@data fetch $faq FAQ[] : "/data/faq.json";

@data inline $trustItems : [
    { "icon": "/assets/icons/flag.svg", "text": "Fabriqué au Maroc" },
    { "icon": "/assets/icons/truck.svg", "text": "Livraison rapide" },
    { "icon": "/assets/icons/refresh.svg", "text": "Retours faciles" },
    { "icon": "/assets/icons/lock.svg", "text": "Paiement sécurisé" }
];

@data fetch $cart CartItem[] : localStorage("jallete-cart") {
    initial: [];
};
```

### Helper Functions

```css
/* ============================================
   HELPER FUNCTIONS
   ============================================ */

@fn getPack(id: string): Pack? {
    return $packs.find(p => p.id == id);
}

@fn packPrice(packId: string): number {
    let pack = getPack(packId);
    return pack?.price ?? 0;
}

@data fold $cartTotal number from $cart : acc + packPrice(item.packId) * item.quantity;
```

### Pack Cards Binding

```css
/* ============================================
   PACK CARDS
   ============================================ */

$packsStatus <- $packs_loading ? "loading" : "ready";

.jal-packs-grid {
    data-state: $packsStatus;

    .is-loading: $packsStatus == "loading";
    .is-ready: $packsStatus == "ready";

    @state(when: "loading") {
        min-height: 400px;
    }

    @state(when: "ready");

    @each($packs as $pack) {
        template: "jal-pack-card";

        [slot="badge"]: $pack.badge;
        [slot="image"] {
            src: $pack.image;
            alt: $pack.title;
        }
        [slot="title"]: $pack.title;
        [slot="original"]: $pack.originalPrice | currency("dhs");
        [slot="price"]: $pack.price | currency("dhs");
        [slot="savings"]: "Économisez " + ($pack.originalPrice - $pack.price) + " dhs";

        /* Nested iteration for pack items */
        [slot="items"] {
            @each($pack.items as $item) {
                template: "jal-pack-item";

                [slot="name"]: $item.name;
                [slot="price"]: $item.price | currency("dhs");
            }
        }

        :host {
            data-pack-id: $pack.id;
            data-price: $pack.price;
            data-woo-id: $pack.wooCommerceId | default("");
        }
    }

    > jal-pack-card {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 60px -> 0;
            stagger: 0.15 first;
        }
    }
}
```

### Features Binding

```css
/* ============================================
   FEATURES GRID
   ============================================ */

.jal-features-grid {
    @each($features as $feature) {
        template: "jal-feature";

        [slot="icon"] {
            src: $feature.icon;
            alt: "";
        }
        [slot="title"]: $feature.title;
        [slot="desc"]: $feature.description;
    }

    > jal-feature {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.08 first;
        }
    }
}
```

### Testimonials Binding

```css
/* ============================================
   TESTIMONIALS
   ============================================ */

.jal-testimonials-grid {
    @each($testimonials as $t) {
        template: "jal-testimonial";

        [slot="quote"]: $t.quote;
        [slot="avatar"]: $t.avatar;
        [slot="name"]: $t.name;
        [slot="role"]: $t.role;
    }

    > jal-testimonial {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.1 first;
        }
    }
}
```

### FAQ Binding

```css
/* ============================================
   FAQ SECTION
   ============================================ */

.jal-faq-list {
    @each($faq as $item) {
        template: "jal-faq-item";

        [slot="question"]: $item.question;
        [slot="answer"]: $item.answer;

        :host {
            data-faq-id: $item.id;
        }
    }
}
```

### Trust Bar Binding

```css
/* ============================================
   TRUST BAR
   ============================================ */

.jal-trust-bar {
    @each($trustItems as $item) {
        template: "jal-trust-item";

        [slot="icon"] {
            src: $item.icon;
            alt: "";
        }
        [slot="text"]: $item.text;
    }
}
```

### JSON Data Files

```json
// /data/packs.json
[
    {
        "id": "pack-mama-cozy",
        "title": "Pack Mama Cozy",
        "badge": "Économisez 81 dhs",
        "image": "/assets/pull-mama-vibes.jpg",
        "originalPrice": 760,
        "price": 679,
        "wooCommerceId": "12345",
        "items": [
            { "name": "1 sweat d'allaitement", "price": 390 },
            { "name": "1 t-shirt d'allaitement", "price": 290 },
            { "name": "Livraison offerte", "price": 80 }
        ]
    },
    {
        "id": "pack-family-matchy",
        "title": "Pack Family Matchy",
        "badge": "Économisez 81 dhs",
        "image": "/assets/family-matchy.jpg",
        "originalPrice": 780,
        "price": 699,
        "wooCommerceId": "12346",
        "items": [
            { "name": "1 sweat d'allaitement", "price": 390 },
            { "name": "1 sweat enfant assorti", "price": 310 },
            { "name": "Livraison offerte", "price": 80 }
        ]
    }
]
```

```json
// /data/features.json
[
    {
        "id": "discrete",
        "icon": "/assets/icons/eye-off.svg",
        "title": "Ouverture discrète",
        "description": "Allaitez en toute discrétion grâce à notre système d'ouverture invisible."
    },
    {
        "id": "confort",
        "icon": "/assets/icons/heart.svg",
        "title": "Confort optimal",
        "description": "Tissus doux et stretch pour un confort maximal pendant l'allaitement."
    },
    {
        "id": "style",
        "icon": "/assets/icons/star.svg",
        "title": "Style moderne",
        "description": "Des designs tendance que vous aurez plaisir à porter au quotidien."
    }
]
```

```json
// /data/testimonials.json
[
    {
        "id": "sarah",
        "quote": "Enfin des vêtements d'allaitement que j'ai envie de porter ! Le sweat est super confortable et l'ouverture vraiment discrète.",
        "avatar": "S",
        "name": "Sarah M.",
        "role": "Maman de 2 enfants, Casablanca"
    },
    {
        "id": "amina",
        "quote": "J'adore le concept du pack famille ! Mon fils est trop fier de son sweat assorti au mien.",
        "avatar": "A",
        "name": "Amina K.",
        "role": "Maman d'un petit garçon, Rabat"
    }
]
```

```json
// /data/faq.json
[
    {
        "id": "discrete",
        "question": "C'est vraiment discret ?",
        "answer": "Oui. L'ouverture est invisible et permet d'allaiter en toute discrétion, même en public."
    },
    {
        "id": "tailles",
        "question": "Comment choisir ma taille ?",
        "answer": "Nos vêtements sont conçus avec une coupe ample. Prenez votre taille habituelle, ou une taille au-dessus si vous préférez plus d'aisance."
    }
]
```

### Simplified HTML

```html
<!-- index.html — Before: 3 repeated jal-pack-card, 9 features, 4 testimonials, 4 FAQ -->
<!-- index.html — After: -->
<section class="jal-trust-bar">
    <!-- Populated by Spacetime -->
</section>

<section class="jal-packs">
    <h2>Nos Packs</h2>
    <div class="jal-packs-grid">
        <!-- Populated by Spacetime -->
    </div>
</section>

<section class="jal-features">
    <h2>Pourquoi Jallete ?</h2>
    <div class="jal-features-grid">
        <!-- Populated by Spacetime -->
    </div>
</section>

<section class="jal-testimonials">
    <h2>Ce qu'elles en disent</h2>
    <div class="jal-testimonials-grid">
        <!-- Populated by Spacetime -->
    </div>
</section>

<section class="jal-faq">
    <h2>Questions fréquentes</h2>
    <div class="jal-faq-list">
        <!-- Populated by Spacetime -->
    </div>
</section>
```

---

## Summary: Before & After

### Lines of HTML Saved

| Site | Before | After | Reduction |
|------|--------|-------|-----------|
| zeystudios gallery | ~400 lines (23 prints) | ~10 lines | 97% |
| zeystudios curations | ~120 lines (6 cards) | ~10 lines | 92% |
| ikarchitecte projects | ~100 lines (6 projects) | ~10 lines | 90% |
| ikarchitecte services | ~50 lines (3 services) | ~10 lines | 80% |
| jallete packs | ~150 lines (3 packs) | ~10 lines | 93% |
| jallete features | ~100 lines (9 features) | ~10 lines | 90% |
| jallete testimonials | ~80 lines (4 testimonials) | ~10 lines | 88% |

### Benefits

1. **Single source of truth**: Data in JSON, templates in HTML, behavior in .st
2. **Type-safe**: Compiler catches errors before runtime
3. **Maintainable**: Change price structure once, applies everywhere
4. **Scalable**: Add 100 items by updating JSON, not HTML
5. **Animations work**: Generated elements inherit Spacetime animations
