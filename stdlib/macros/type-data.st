/// Type Definition Macro
///
/// Defines a named type for use in data bindings.
///
/// @example
/// @type Product {
///   id: string
///   name: string
///   price: number
///   inStock: bool
///   tags: string[]
///   category: "electronics" | "clothing" | "food"
/// }

%macro type {
  %scope file
  %order 50

  %form {
    @type $name:ident {
      $fields:properties
    }
  }

  %registers type($name) {
    $fields
  }
}


/// @type (SUM form) — a TAGGED SUM: `@type R { A(T) | B(U) | C }` (PLAN-038 W0).
///
/// A second %macro carrying the SAME `@type Name { … }` surface but a SUM body
/// (`variant_list`) instead of a product field list. `variant_list` REQUIRES ≥1
/// top-level `|` alternation bar; a product body (zero bars) FAILS this form's
/// body match. The body capture names a SPECIFIC constrained capture type, so the
/// specificity scorer ranks this form ABOVE the product form (whose body is the
/// permissive `properties` collection) — it is tried FIRST, matches a sum, and a
/// product body falls through to the product form. The grammar IS the
/// discriminator (self-describing metasystem rule) — no Rust dispatch branch.
///
/// Registers the `variants` capture, read by `TypeRegistry::from_form_matches`
/// into `TypeDef.variants`. Both forms `%register type(...)` so they share the
/// `creates_name = "type"` the registry filters on.
%macro type-sum {
  %scope file
  %order 50

  %form {
    @type $name:ident {
      $variants:variant_list
    }
  }

  %registers type($name) {
    variants: $variants
  }
}


/// Legacy `@data NAME : T { src/value/cache/initial }` and `@computed NAME : T
/// { from/where/sort/reduce/... }` block-form macros were REMOVED in PLAN-023
/// W5/FEAT-047. The unified `@data <kind>` surface (stdlib/macros/data-kind.st)
/// is the one and only data construct:
///   @data x: T { src: "/u"; cache: 5m }   ->  @data fetch x T : "/u" { refresh: 5m }
///   @data x: T { src: inline; value: v }  ->  @data inline x T : v
///   @data x: T { initial: v }             ->  @data inline x T : v
///   @computed x: T { from: s; where: w }  ->  @data query x T from s { where: w }
///   @computed x: T { from: s; reduce: r } ->  @data fold x T from s : r

/// Function Definition Macro
///
/// Defines a reusable helper function.
///
/// @example
/// @fn formatPrice($price number): string {
///   "$" + price.toFixed(2)
/// }
///
/// // Usage: $.price | formatPrice
///
/// Parameters use the SPACE-form type annotation (`$price number`), matching
/// template params + signal declarations (FUP-040). The `$` sigil marks the
/// parameter name; its type follows after a space. The return type keeps `: T`
/// (a value-position colon). Inside the body, params are referenced by their bare
/// name (`price`), since the JS function binds positional argument names.

%macro fn {
  %scope file
  %order 500

  %form {
    @fn $name:ident($params:param_list) : $returnType:typeref {
      $body:expr
    }
  }

  %registers fn($name) {
    params: $params
    returns: $returnType
    body: $body
  }

  // Semantic resolution via registry
  %resolves {
    $name -> functions
  }

  // Delegate to primitive for JS emission
  %binds {
    fn-registry(
      name: $name,
      params: $params,
      body: $body,
      returnType: $returnType
    ) -> { $fn as $$name }
  }
}


/// Compute Function Macro
///
/// Declares a computed value with full multi-statement JavaScript logic.
/// Unlike @computed which only supports array operations (filter/sort/limit/map),
/// @compute allows arbitrary JavaScript for complex computations.
///
/// @example
/// @compute cartTotal($cart, $config) : number {
///   if (!$cart || $cart.length === 0) return 0;
///   const discounts = $config?.packDiscounts || {};
///   let total = $cart.reduce((sum, item) => sum + (item.price || 0), 0);
///   let maxDiscount = 0;
///   $cart.forEach(item => {
///     const d = discounts[item.packId]?.percentage || 0;
///     if (d > maxDiscount) maxDiscount = d;
///   });
///   return Math.round(total * (1 - maxDiscount));
/// }

%macro compute {
  %scope file | selector

  %form {
    @compute $name:ident($deps:param_list) : $returnType:typeref {
      $body:expr
    }
  }

  %binds {
    compute-fn(
      name: $name,
      deps: $deps,
      body: $body,
      returnType: $returnType
    ) -> { $value as $$name }
  }
}
