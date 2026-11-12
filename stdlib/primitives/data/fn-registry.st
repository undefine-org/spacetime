/// Function Registry Primitive
///
/// Registers a user-defined function for use in expressions and filters.
/// Functions are stored in the global ST.functions namespace.

%primitive fn-registry(
  /// Function name for registration
  name: ident,
  /// Parameter definitions
  params: param_list,
  /// Function body expression
  body: expr,
  /// Expected return type (for validation)
  returnType: typeref?
) {
  %emit js {
    // Initialize function registry if needed
    ST.functions = ST.functions || {};

    // Parse parameters and create function
    const fnName = %name;
    const paramDefs = %params;
    const fnBody = %body;

    // Build parameter names from definitions
    const paramNames = Array.isArray(paramDefs)
      ? paramDefs.map(p => typeof p === 'string' ? p : p.name)
      : [];

    // Create the function
    const fn = new Function(...paramNames, `return ${fnBody}`);

    // Register in global namespace
    ST.functions[fnName] = fn;

    // Also register as a filter for pipe syntax
    ST.filters = ST.filters || {};
    ST.filters[fnName] = (value, ...args) => fn(value, ...args);

    %yield fn -> $fn;
  }

  %exports {
    /// The registered function reference
    $fn: any                         // Dynamic function (signature depends on params)
  }
}
