/// Cart Items Renderer
///
/// Reactive binding that renders cart items and re-renders on cart changes.
/// Updates automatically when cart changes via local:cart:updated events.
///
/// @example
/// .jal-cart__items {
///     @cart-items;
/// }

%primitive cart-items-binding(&el) {
  %emit js {
    (() => {
      const container = %&el;
      if (!container) return;  // Guard against missing element

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

      const renderCartItems = () => {
        const cart = getCartFromStorage();

        // Clear existing items
        container.innerHTML = '';

        // Render each cart item
        cart.forEach(item => {
          const itemEl = document.createElement('div');
          itemEl.className = 'jal-cart-item';
          itemEl.dataset.itemId = item.id;

          itemEl.innerHTML = `
            <img class="jal-cart-item__img" src="${item.image || ''}" alt="${item.packTitle || ''}">
            <div class="jal-cart-item__info">
              <span class="jal-cart-item__title">${item.packTitle || ''}</span>
              <span class="jal-cart-item__variant">${item.sweatVariant || ''} - ${item.sweatColor || ''}</span>
              <span class="jal-cart-item__sizes">M: ${item.sizeMom || 'M'} | P: ${item.sizeDad || 'L'}</span>
            </div>
            <span class="jal-cart-item__price">${item.price || 0} dhs</span>
            <button class="jal-cart-item__remove" data-remove-item="${item.id}">×</button>
          `;

          container.appendChild(itemEl);
        });

        // Show empty state if no items
        if (cart.length === 0) {
          const emptyEl = document.createElement('div');
          emptyEl.className = 'jal-cart-empty';
          emptyEl.textContent = 'Votre panier est vide';
          container.appendChild(emptyEl);
        }
      };

      // Update on cart changes (jallete-cart is the localStorage key)
      document.addEventListener('local:jallete-cart:updated', renderCartItems);

      // Initial render - run immediately and also defer for safety
      renderCartItems();
      requestAnimationFrame(renderCartItems);

      // Cleanup
      ST.onCleanup(container, () => {
        document.removeEventListener('local:jallete-cart:updated', renderCartItems);
      });
    })();
  }
}

%macro cart-items {
  %form {
    @cart-items
  }

  %binds {
    cart-items-binding(&self)
  }
}
