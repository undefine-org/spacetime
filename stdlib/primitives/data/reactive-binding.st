/// Reactive Binding Primitive
///
/// Wires a binding source to SpacetimeLocal.
/// Handles both binding-protocol objects and plain values.
///
/// Binding protocol objects must have:
/// - __stBinding: true
/// - get(): returns current value
/// - set(v): sets new value
///
/// @param name - The variable name to bind
/// @param source - The source expression (evaluated)
///
/// @yields $value - The bound value

%primitive reactive-binding(name: ident, source: expr) {
  %emit js {
    (() => {
      const _source = %source;
      const _name = '%name';

      if (_source && _source.__stBinding) {
        // Binding source - wire get/set to SpacetimeLocal
        Object.defineProperty(SpacetimeLocal, _name, {
          get() { return _source.get(); },
          set(v) { _source.set(v); },
          enumerable: true,
          configurable: true
        });

        // Initialize CSS variable
        const _initVal = _source.get();
        document.documentElement.style.setProperty(
          `--st-${_name}`,
          Array.isArray(_initVal) ? _initVal.length : _initVal
        );

        // Return the initial value
        return _initVal;
      } else {
        // Plain value - standard local binding
        let _val = _source;
        Object.defineProperty(SpacetimeLocal, _name, {
          get() { return _val; },
          set(v) {
            _val = v;
            document.dispatchEvent(new CustomEvent(`local:${_name}:updated`, {
              detail: v,
              bubbles: true
            }));
            document.documentElement.style.setProperty(
              `--st-${_name}`,
              Array.isArray(v) ? v.length : v
            );
          },
          enumerable: true,
          configurable: true
        });

        // Initialize CSS variable
        document.documentElement.style.setProperty(
          `--st-${_name}`,
          Array.isArray(_val) ? _val.length : _val
        );

        // Return the initial value
        return _val;
      }
    })()
  }

  %exports {
    $value: any
  }
}
