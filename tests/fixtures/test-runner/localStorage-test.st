/// localStorage Integration Example
///
/// Demonstrates the localStorage binding syntax.
/// Run with: cargo run -- compile tests/fixtures/test-runner/localStorage-test.st

/// Shopping cart with localStorage persistence
.shopping-cart {
  /// Cart items backed by localStorage
  /// Syntax: $name Type: localStorage("key", default);
  $cart CartItem[]: localStorage("demo-cart", []);

  /// Display cart count
  [slot="count"]: $cart.length;

  /// Add item button
  @on &.click .add-item {
    $cart <- $cart.concat([{ id: Date.now(), name: "New Item" }]);
  }

  /// Clear cart
  @on &.click .clear-cart {
    $cart <- [];
  }
}

/// User preferences with localStorage
.preferences {
  $prefs UserPrefs: localStorage("user-prefs", { theme: "light", language: "en" });

  [slot="theme"]: $prefs.theme;

  @on &.click .toggle-theme {
    $prefs <- { ...$prefs, theme: $prefs.theme === "light" ? "dark" : "light" };
  }
}

/// Counter without persistence (plain value)
.counter {
  $count number: 0;

  [slot="value"]: $count;

  @on &.click .increment {
    $count <- $count + 1;
  }

  @on &.click .decrement {
    $count <- $count - 1;
  }
}
