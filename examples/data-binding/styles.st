/* ============================================
   SPACETIME DATA BINDING DEMO
   ============================================ */


/* ============================================
   TYPE DEFINITIONS
   ============================================ */

@type Product {
    id: string;
    name: string;
    description: string;
    price: number;
    category: string;
    imageUrl: url;
    inStock: boolean;
}

@type Testimonial {
    quote: string;
    name: string;
    role: string;
    avatar: string;
}

@type CartItem {
    productId: string;
    quantity: number;
}

/* ============================================
   DATA SOURCES
   ============================================ */

/* Load products from JSON (unified @data <kind> surface, PLAN-023 / FEAT-047) */
@data fetch $products Product[] : "/data/products.json" { refresh: 1h }

/* Testimonials data */
@data fetch $testimonials Testimonial[] : "/data/testimonials.json"

/* Cart persisted in localStorage */
@data fetch $cart CartItem[] : localStorage("demo-cart") { initial: [] }

/* ============================================
   COMPUTED DATA
   ============================================ */

/* Calculate total cart value (fold reduces over the source with acc/item locals).
   Each cart item carries its own price/quantity — no external lookup needed. */
@data fold $cartTotal number from $cart : acc + (item.price * item.quantity) ;

/* Count items in cart */
@data fold $cartCount number from $cart : acc + item.quantity ;

/* Filter products by category (query pipeline over an array source) */
@data query $electronicsProducts Product[] from $products {
    where: item.category == "electronics"
}

/* ============================================
   HELPER FUNCTIONS
   ============================================ */

/* Helper function - optional return type not yet fully supported
@fn getProduct($id string): Product? {
    return products.find(p => p.id == id);
}
*/

/* ============================================
   TEMPLATE DEFINITIONS
   ============================================ */

@template &product-card($p) {
    <div class="product-card" data-id="`$p.id`" data-category="`$p.category`" class="`$p.inStock` ? 'in-stock' : 'out-of-stock'">
        <img slot="image" src="`$p.imageUrl`" alt="`$p.name`" loading="lazy">
        <span slot="category">`$p.category` | uppercase</span>
        <h3 slot="name">`$p.name`</h3>
        <p slot="description">`$p.description` | truncate(100)</p>
        <span slot="price">`$p.price` | currency("$")</span>
        <span slot="stock">`$p.inStock` | if("In Stock", "Out of Stock")</span>
    </div>
}

@template &testimonial-card($t) {
    <div class="testimonial-card">
        <blockquote slot="quote">`$t.quote`</blockquote>
        <div class="avatar" slot="avatar">`$t.avatar`</div>
        <span slot="name">`$t.name`</span>
        <span slot="role">`$t.role`</span>
    </div>
}

/* ============================================
   PRODUCT GRID BINDING
   ============================================ */

.product-grid {
    /* Reactive loading state derived from the $products signal */
    .is-loading: $products == null;
    .is-ready: $products != null && $products.length > 0;
    .is-empty: $products != null && $products.length == 0;

    /* Default / loading visual state */
    opacity: 0.5;
    min-height: 400px;

    /* Generate a product card for each product */
    @each($products as $p) {
        &product-card($p);
    }

    /* Animate product cards on scroll */
    > .product-card {
        @on &.scroll(name: reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
        duration: 600ms;
        easing: --ease-out;
        stagger: 0.08 first;
      }

        @on &.hover(name: lift, duration: 300ms) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        translate-y: 0 -> -4px;
        box-shadow: 0 2px 8px rgba(0,0,0,0.1) -> 0 8px 24px rgba(0,0,0,0.15);
      }
    }
}

.product-grid.is-loading {
    opacity: 0.5;
    min-height: 400px;
}

.product-grid.is-ready {
    opacity: 1;
}

.product-grid.is-empty {
    min-height: 200px;
}

/* ============================================
   TESTIMONIALS BINDING
   ============================================ */

.testimonials-grid {
    @each($testimonials as $t) {
        &testimonial-card($t);
    }

    /* Stagger animation for testimonials */
    > .testimonial-card {
        @on &.scroll(name: reveal, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 30px -> 0;
        duration: 500ms;
        stagger: 0.15 first;
      }
    }
}

/* ============================================
   CART TOTAL BINDING
   ============================================ */

.cart-total-value {
    text <- $cartTotal | currency("$");
}

.cart-count {
    text <- $cartCount;
    .has-items: $cartCount > 0;
}

/* ============================================
   HERO ANIMATIONS
   ============================================ */

.hero {
    @on &.load(name: entrance, duration: 1000ms, delay: 0ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -20px -> 0;
      }

    h1 {
        @on &.load(name: title-entrance, duration: 1000ms, delay: 0ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -30px -> 0;
        delay: 200ms;
      }
    }

    p {
        @on &.load(name: subtitle-entrance, duration: 1000ms, delay: 0ms, threshold: 0.1) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: -20px -> 0;
        delay: 400ms;
      }
    }
}

/* ============================================
   ANIMATION PRESETS
   ============================================ */

@preset quick-reveal {
    opacity: 0 -> 1;
    translate-y: 40px -> 0;
    duration: 600ms;
    easing: --ease-out;
}

@preset reveal {
    opacity: 0 -> 1;
    translate-y: 60px -> 0;
    duration: 800ms;
    easing: --ease-out-expo;
}
