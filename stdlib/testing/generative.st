/// Generative testing (PLAN-027 W4): @property and @fuzz.
///
/// Property tests generate many inputs from a type spec, run the body, and on
/// failure shrink to a minimal counterexample (seeded + reproducible). Fuzz
/// tests generate adversarial inputs and FAIL on a thrown error or a per-case
/// timeout. The generator / shrinker live in the test runtime (window.__stGen /
/// __stProperty / __stFuzz / __stParseBinding); these macros capture the binding
/// `name: spec` as a raw string and split it at runtime.
///
/// Type specs (after the `:` in the binding):
///   number | number A..B | int | int A..B | bool
///   string | string ~ <corpus>   (corpus ∈ {ascii, unicode, json, html})
///   <scalar>[]                    (array of a scalar, length 0..16)
///
/// The body uses `@assert (cond) "msg"`; the generated value is bound to the
/// declared name. On failure the property shrinks + reports the seed.
///
/// @example
///   @property "min is bounded" forall (n: int 0..100) {
///     @assert (Math.min(n, 10) <= 10) "min <= 10";
///   }
///
///   @fuzz "parser survives junk" inputs (s: string ~ unicode) {
///     @assert (ST.parseColor(s) === null || typeof ST.parseColor(s) === 'object') "no crash";
///   }

%macro property {

  %form {
    @property $desc:string forall ( $binding:balanced(')') ) {
      $body:block
    }
  }

  %emit js {
    window.__spacetime_register_test(%$desc, async () => {
      const __ctx = { startTime: Date.now() };
      const __b = window.__stParseBinding("%$binding");
      const __cases = (typeof __ST_PROPERTY_CASES !== 'undefined') ? __ST_PROPERTY_CASES : 100;
      const __seed = (typeof __ST_PROPERTY_SEED !== 'undefined') ? __ST_PROPERTY_SEED : 0x5eed;
      window.__stProperty(__b.spec, __cases, __seed, (__value) => {
        let __ok = true;
        const __assert = (c, m) => { if (!c) { __ok = false; throw new Error(m || 'property body failed'); } };
        // Bind the generated value to the declared name in this scope. eval here
        // is harness-internal (test layer), not user JS \u2014 the binding name is a
        // grammar-captured identifier.
        eval('var ' + __b.name + ' = __value;');
        %$body.js
        return __ok;
      });
    }, { generative: 'property' });
  }
}


%macro fuzz {

  %form {
    @fuzz $desc:string inputs ( $binding:balanced(')') ) {
      $body:block
    }
  }

  %emit js {
    window.__spacetime_register_test(%$desc, async () => {
      const __ctx = { startTime: Date.now() };
      const __b = window.__stParseBinding("%$binding");
      const __cases = (typeof __ST_FUZZ_CASES !== 'undefined') ? __ST_FUZZ_CASES : 200;
      const __seed = (typeof __ST_FUZZ_SEED !== 'undefined') ? __ST_FUZZ_SEED : 0xf0;
      const __budget = (typeof __ST_FUZZ_BUDGET_MS !== 'undefined') ? __ST_FUZZ_BUDGET_MS : 250;
      window.__stFuzz(__b.spec, __cases, __seed, __budget, (__value) => {
        const __assert = (c, m) => { if (!c) throw new Error(m || 'fuzz body failed'); };
        eval('var ' + __b.name + ' = __value;');
        %$body.js
      });
    }, { generative: 'fuzz' });
  }
}
