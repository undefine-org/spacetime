/// Cart Count Badge
///
/// Reactive binding that shows the number of items in $cart array.
/// Updates automatically when cart changes via local:cart:updated events.
///
/// @example
/// .jal-nav__cart-badge {
///     @cart-count;
/// }

%primitive cart-count-binding(&el) {
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

      const updateCount = () => {
        const cart = getCartFromStorage();
        const count = cart.length;
        el.textContent = count;
        // Hide badge when cart is empty
        el.style.display = count > 0 ? '' : 'none';
      };

      // Update on cart changes (jallete-cart is the localStorage key)
      document.addEventListener('local:jallete-cart:updated', updateCount);

      // Initial calculation - run immediately and also defer for safety
      updateCount();
      requestAnimationFrame(updateCount);

      // Cleanup
      ST.onCleanup(el, () => {
        document.removeEventListener('local:jallete-cart:updated', updateCount);
      });
  }
}

%macro cart-count {
  %form {
    @cart-count
  }

  %binds {
    cart-count-binding(&self)
  }
}
