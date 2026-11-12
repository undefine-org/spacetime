/// Binding Macros
///
/// Expression-level macros for reactive data bindings.
/// Enables clean syntax like: $cart CartItem[]: localStorage("demo-cart", []);


/// localStorage Expression Macro
///
/// Creates a reactive binding backed by localStorage.
/// Used in typed declarations: $cart CartItem[]: localStorage("key", []);
///
/// Features:
/// - Auto-load from localStorage on init
/// - Debounced auto-save (300ms) on mutation
/// - Cross-tab sync via storage events
/// - CSS variable: --st-{key}
/// - Event dispatch: local:{key}:updated

%macro localStorage {

  %form {
    localStorage($key:string, $default:expr? = undefined)
  }

  %binds {
    local-storage-source(key: $key, default: $default) -> { $binding }
  }

  %example "@localStorage" {
    .shopping-cart {
      $cart string[]: localStorage("demo-cart", []);

      .count { text <- $cart.length; }
      .add-item { @on &.click { $cart <- [...$cart, "tea"]; } }
    }
    <div class="shopping-cart">
      <span class="count">0</span>
      <button class="add-item">Add tea</button>
    </div>
  }
}
