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
///
/// @example
/// .shopping-cart {
///   $cart CartItem[]: localStorage("demo-cart", []);
///
///   [slot="count"]: $cart.length;
///
///   @on click .add-item {
///     $cart <- $cart.concat([newItem]);
///   }
/// }

%macro localStorage {

  %form {
    localStorage($key:string, $default:expr? = undefined)
  }

  %binds {
    local-storage-source(key: $key, default: $default) -> { $binding }
  }
}
