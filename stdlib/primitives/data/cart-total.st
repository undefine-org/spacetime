/// Cart Total Helper
///
/// Reactive binding that computes cart total from $cart array.
/// Updates automatically when cart changes via local:cart:updated events.
///
/// @example
/// .jal-cart__total-price {
///     @cart-total;
/// }

%primitive cart-total-binding(&el) {
  %emit js {
      const getCartFromStorage = () => {
        // Try SpacetimeLocal first (reactive state), fall back to localStorage
        if (window.SpacetimeLocal?.cart) {
          return SpacetimeLocal.cart;
        }
        try {
          const stored = localStorage.getItem('jallete-cart');
          return stored ? JSON.parse(stored) : [];
        } catch (e) {
          return [];
        }
      };
      
      const updateTotal = () => {
        const cart = getCartFromStorage();
        // Use cartTotal helper if available (applies highest discount to original prices)
        // Otherwise fall back to simple sum of item prices
        const total = window.cartTotal
          ? window.cartTotal(cart)
          : cart.reduce((sum, item) => sum + (item.price || 0), 0);
        el.textContent = total + ' dhs';
      };
      
      // Update on cart changes (jallete-cart is the localStorage key)
      document.addEventListener('local:jallete-cart:updated', updateTotal);
      
      // Initial calculation - run immediately and also defer for safety
      updateTotal();
      requestAnimationFrame(updateTotal);
      
      // Cleanup
      ST.onCleanup(el, () => {
        document.removeEventListener('local:jallete-cart:updated', updateTotal);
      });
  }
}

%macro cart-total {
  %form {
    @cart-total
  }
  
  %binds {
    cart-total-binding(&self)
  }
}
