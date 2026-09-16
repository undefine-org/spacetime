/// Compute Function Primitive
///
/// Creates a reactive computed value with a full JavaScript function body.
/// Unlike @computed which only supports array operations, @compute allows
/// multi-statement custom logic for complex computations.
///
/// @param name - Name for the computed value (becomes SpacetimeLocal property)
/// @param deps - Parameter list of dependencies (e.g., $cart, $config)
/// @param body - JavaScript function body (multi-statement)
/// @param returnType - Optional return type annotation
///
/// @yields $value - The computed result

%primitive compute-fn(
  name: ident,
  deps: param_list,
  body: expr,
  returnType: typeref?
) {
  %emit js {
    (() => {
      const fnName = '%name';
      const depDefs = %deps;

      // Extract dependency names from parameter definitions
      const depNames = Array.isArray(depDefs)
        ? depDefs.map(p => {
            const name = typeof p === 'string' ? p : (p.name || p);
            return name.replace(/^\$/, '');
          })
        : [];

      // Create the compute function from the body
      // The body is raw JS that can use the parameter names directly
      const computeFn = new Function(...depNames, `%body`);

      // Store in function registry for debugging/introspection
      ST.functions = ST.functions || {};
      ST.functions[fnName] = computeFn;

      let value = null;

      const compute = () => {
        const args = depNames.map(name => SpacetimeLocal[name]);
        try {
          value = computeFn(...args);
        } catch (e) {
          console.error(`[compute-fn] Error in ${fnName}:`, e);
          value = null;
        }
        return value;
      };

      // Initial computation
      value = compute();

      // Watch dependencies for reactive updates
      depNames.forEach(name => {
        document.addEventListener(`local:${name}:updated`, () => {
          value = compute();
          document.dispatchEvent(new CustomEvent(`local:${fnName}:updated`, { detail: value }));
        });
      });

      // Define as a property on SpacetimeLocal
      Object.defineProperty(SpacetimeLocal, fnName, {
        get() { return value; },
        enumerable: true,
        configurable: true
      });

      %yield value -> $value;
    })()
  }

  %exports {
    $value: any
  }
}
