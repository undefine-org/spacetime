// Demonstration of @on mutation blocks for declarative state updates
// This example shows a shopping cart with add/remove functionality

@type CartItem {
    id: number;
    name: string;
    price: number;
    qty: number;
}

// Global state scoped to body
body {
    $cart CartItem[]: [];
}

// Add to cart button
.add-btn {
    @on &.click {
        $cart <- $cart.concat([{ id: 1, name: "Product", price: 29.99, qty: 1 }]);
    }
}

// Event delegation example: multiple product cards
.product-grid {
    @on &.click [data-add-to-cart] {
        $cart <- $cart.concat([{
            id: +$.dataset.id,
            name: $.dataset.name,
            price: +$.dataset.price,
            qty: 1
        }]);
    }
}

// Remove from cart (with delegation)
.cart-items {
    @on &.click [data-remove] {
        $cart <- $cart.filter(item => item.id !== +$.dataset.id);
    }
}

// Update quantity
.cart-items {
    @on &.change [data-qty-input] {
        $cart <- $cart.map(item =>
            item.id === +$.dataset.id
                ? { ...item, qty: +$.value }
                : item
        );
    }
}

// Clear cart button
.clear-cart-btn {
    @on &.click {
        $cart <- [];
    }
}
