/// Function Registry Primitive
///
/// Registers a user-defined function for use in expressions and filters.
/// Functions are stored in the global ST.functions namespace.
///
/// @param name - Function name for registration
/// @param params - Parameter definitions
/// @param body - Function body expression
/// @param returnType - Expected return type (for validation)
///
/// @yields $fn - The registered function reference

%primitive fn-registry(
  name: ident,
  params: param_list,
  body: expr,
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
    $fn: any                         // Dynamic function (signature depends on params)
  }
}
