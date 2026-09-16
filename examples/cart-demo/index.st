/* Cart Demo - Reactive State Example */

/* Template definitions */
@template &product-card($p) {
    <product-card data-id="`$p.id`" data-name="`$p.name`" data-price="`$p.price`" data-image="`$p.image`">
        <img slot="image" src="`$p.image`" alt="`$p.name`">
        <span slot="name">`$p.name`</span>
        <span slot="price">`$p.price`</span>
        <span slot="description">`$p.description`</span>
        <button data-add-to-cart>Add to Cart</button>
    </product-card>
}

@template &cart-item($item) {
    <cart-item data-item-id="`$item.id`">
        <img slot="image" src="`$item.image`" alt="`$item.name`">
        <span slot="name">`$item.name`</span>
        <span slot="price">`$item.price`</span>
        <span slot="qty">`$item.qty`</span>
    </cart-item>
}

/* Type definitions */
@type CartItem {
    id: string;
    name: string;
    price: number;
    qty: number;
    image: string;
}

@type Product {
    id: string;
    name: string;
    price: number;
    image: string;
    description: string;
}

/* Global reactive state */
body {
    $cart CartItem[]: [];
    $cartOpen bool: false;
}

/* Computed totals (unified @data fold surface, PLAN-023 W5).
   fold reduces over the source array with `acc`/`item` reducer locals; initial 0. */
@data fold $cartTotal number from $cart : acc + (item.price * item.qty) ;
@data fold $cartCount number from $cart : acc + item.qty ;

/* Bind derived totals to the DOM via file-scope reactive bindings (BUG-033).
   $cart mutates -> @data fold recomputes + publishes local:<name>:updated
   (BUG-034) -> these bindings update .total-amount / .cart-count live. */
.total-amount { text: $cartTotal; }
.cart-count { text: $cartCount; }

/* Product data source (unified @data <kind> surface, PLAN-023 / FEAT-047).
   inline object-literal array now parses as one value (BUG-042). */
@data inline $products Product[] : [
    { "id": "p1", "name": "Moroccan Rug", "price": 299, "image": "/images/rug.jpg", "description": "Hand-woven traditional carpet" },
    { "id": "p2", "name": "Ceramic Tagine", "price": 89, "image": "/images/tagine.jpg", "description": "Authentic cooking pot" },
    { "id": "p3", "name": "Brass Lantern", "price": 149, "image": "/images/lantern.jpg", "description": "Decorative Moroccan lamp" },
    { "id": "p4", "name": "Leather Pouf", "price": 179, "image": "/images/pouf.jpg", "description": "Handcrafted floor cushion" }
];

/* Render product cards */
.product-grid {
    @each($products as $p) {
        &product-card($p);
    }

    /* Add to cart mutation */
    @on &.click [data-add-to-cart] {
        $cart <- $cart.concat([{
            id: $.closest('product-card').dataset.id,
            name: $.closest('product-card').dataset.name,
            price: +$.closest('product-card').dataset.price,
            image: $.closest('product-card').dataset.image,
            qty: 1
        }]);
    }
}

/* Render cart items */
.cart-items {
    @each($cart as $item) {
        &cart-item($item);
    }
}

/* Cart item quantity controls */
.cart-items {
    @on &.click [data-increment] {
        $cart <- $cart.map(item =>
            item.id === $.closest('cart-item').dataset.itemId
                ? { ...item, qty: item.qty + 1 }
                : item
        );
    }

    @on &.click [data-decrement] {
        $cart <- $cart.map(item =>
            item.id === $.closest('cart-item').dataset.itemId && item.qty > 1
                ? { ...item, qty: item.qty - 1 }
                : item
        ).filter(item => item.qty > 0);
    }

    @on &.click [data-remove] {
        $cart <- $cart.filter(item => item.id !== $.closest('cart-item').dataset.itemId);
    }
}

/* Cart drawer — $cartOpen drives .is-open */
.cart-drawer {
    .is-open: $cartOpen;
    transform: translateX(100%);
}
.cart-drawer.is-open {
    transform: translateX(0);
}

/* Drawer open/close triggers */
.cart-toggle {
    @on &.click { $cartOpen <- true; }
}
.close-cart {
    @on &.click { $cartOpen <- false; }
}

/* Clear cart button */
.clear-cart-btn {
    @on &.click {
        $cart <- [];
    }
}
