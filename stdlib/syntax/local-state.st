/// Local State Syntax
///
/// Defines reactive local variables that trigger updates when modified.
/// Syntax: $name type: value;
///
/// Examples:
///   $count number: 0;
///   $name string: "default";
///   $items Product[]: [];
///   $visible bool: true;

%primitive local-state-impl(
  name: ident,
  initial: any
) {
  %emit js {
    // Initialize state. _localState is the backing store and SpacetimeLocal is
    // a Proxy over it that auto-dispatches local:<name>:updated on writes. The
    // Proxy is authoritative. If another primitive (e.g. an @data source or
    // @computed) created a PLAIN object on window.SpacetimeLocal before this
    // ran, migrate its keys into _localState and replace it with the Proxy so
    // the reactive write-trap is never lost (fixes add-to-cart where a plain
    // SpacetimeLocal made $cart unreachable through the Proxy).
    if (!window._localState) window._localState = {};
    if (window.SpacetimeLocal && !window.SpacetimeLocal.__stProxy) {
      for (const __k in window.SpacetimeLocal) {
        if (Object.prototype.hasOwnProperty.call(window.SpacetimeLocal, __k)) {
          window._localState[__k] = window.SpacetimeLocal[__k];
        }
      }
      window.SpacetimeLocal = undefined;
    }
    if (!window.SpacetimeLocal) {
      window.SpacetimeLocal = new Proxy(window._localState, {
        set(target, prop, value) {
          // Check if current value is a binding object (e.g., from localStorage macro)
          const current = target[prop];
          if (current && current.__stBinding) {
            // Delegate to binding's set method
            current.set(value);
            return true;
          }
          // Simple value - store directly
          target[prop] = value;
          document.dispatchEvent(new CustomEvent(`local:${prop}:updated`, {
            detail: value, bubbles: true, composed: true
          }));
          document.dispatchEvent(new CustomEvent('local:updated', {
            detail: { name: prop, value }, bubbles: true, composed: true
          }));
          document.documentElement.style.setProperty(
            `--st-${prop}`,
            Array.isArray(value) ? value.length : value
          );
          return true;
        },
        get(target, prop) {
          // Identity marker so other primitives can detect the reactive Proxy
          // and avoid clobbering it with a plain object.
          if (prop === '__stProxy') return true;
          const val = target[prop];
          // Unwrap binding objects - return their actual value
          if (val && val.__stBinding) {
            return val.get();
          }
          return val;
        }
      });
    }
    // Seed the initial value, then announce it on the body-scope channel so a
    // reactive binding (`.cls: $sig`, `text <- $sig`) that registered BEFORE this
    // init re-renders with the seeded value. The raw `_localState[name] = initial`
    // write bypasses the SpacetimeLocal proxy (which would have dispatched), so
    // without this explicit dispatch an initially-true `$b bool: true` never
    // reached a binding whose one-shot render ran while the value was still
    // undefined (BUG-119 §1: scoped-state decl whose initial must reflect with no
    // interaction). The write itself stays unconditional so each test's init
    // RESETS the value (test isolation: two tests sharing `$on` must each start
    // from their declared initial, not inherit the prior test's mutated value).
    window._localState[%name] = %initial;
    document.dispatchEvent(new CustomEvent(`local:${%name}:updated`, { detail: %initial }));
  }

  %exports {
    $value: any
  }
}

%macro local-state {
  // Prefix '$' extracted from %form pattern
  // Use typeref to support array types like CartItem[]
  %form { $name:ident $type:typeref : $value:expr ; }

  %binds {
    local-state-impl(name: $name, initial: $value)
  }
}

%macro local-state-uninitialized {
  // Uninitialized variant: $error string?;
  %form { $name:ident $type:typeref ; }

  %binds {
    local-state-impl(name: $name, initial: null)
  }
}
