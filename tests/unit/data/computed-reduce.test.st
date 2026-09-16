/// Unit Tests: @computed reduce/initial via computed-source primitive
///
/// Behaviorally validates the JS pipeline that @computed's %binds block
/// generates through the computed-source primitive. Each test simulates the
/// exact code path: source → filter → sort → limit → map → reduce → result
///
/// The compiled pipeline (from computed-source.st) produces:
///   let result = %source;
///   if (filterExpr) result = result.filter(filterFn);
///   if (sortField)  result = [...result].sort(comparator);
///   if (limitVal)   result = result.slice(0, limitVal);
///   if (transformFn) result = result.map(transformFn);
///   if (reduceFn)   result = result.reduce(reduceFn, initial?);
///   value = result;

@import "stdlib/testing/test"

// =============================================================================
// 1. Basic Reduce: Array → Scalar
// =============================================================================

@test "@computed reduce: sum of numbers" {
  @fixture {
    <div class="t1">Reduce Sum</div>
  }

  @eval (eval("                                                          \
    var source = [10, 20, 30, 40];                                       \
    var result = source.reduce(function(sum, n) { return sum + n }, 0);  \
    window.__t1_result = result;                                         \
    window.__t1_type = typeof result;                                    \
  "))

  @assert (window.__t1_result === 100)
    "reduce((sum, n) => sum + n, 0) on [10,20,30,40] should produce 100"

  @assert (window.__t1_type === 'number')
    "reduce should produce a number, not an array"
}

@test "@computed reduce: product of numbers" {
  @fixture {
    <div class="t2">Reduce Product</div>
  }

  @eval (eval("                                                              \
    var source = [2, 3, 5];                                                  \
    var result = source.reduce(function(prod, n) { return prod * n }, 1);    \
    window.__t2_result = result;                                             \
  "))

  @assert (window.__t2_result === 30)
    "reduce((prod, n) => prod * n, 1) on [2,3,5] should produce 30"
}

@test "@computed reduce: string concatenation" {
  @fixture {
    <div class="t3">Reduce Concat</div>
  }

  @eval (eval("                                                             \
    var source = ['hello', ' ', 'world'];                                   \
    var result = source.reduce(function(acc, s) { return acc + s }, '');     \
    window.__t3_result = result;                                            \
    window.__t3_type = typeof result;                                       \
  "))

  @assert (window.__t3_result === 'hello world')
    "reduce with string concat should produce 'hello world'"

  @assert (window.__t3_type === 'string')
    "reduce to string should produce type 'string'"
}


// =============================================================================
// 2. Initial Value Semantics
// =============================================================================

@test "@computed reduce: initial value seeds accumulator" {
  @fixture {
    <div class="t4">Initial Value</div>
  }

  @eval (eval("                                                          \
    var source = [1, 2, 3];                                              \
    var r100 = source.reduce(function(a, n) { return a + n }, 100);      \
    var r0   = source.reduce(function(a, n) { return a + n }, 0);        \
    window.__t4_r100 = r100;                                             \
    window.__t4_r0 = r0;                                                 \
  "))

  @assert (window.__t4_r100 === 106)
    "reduce with initial=100 on [1,2,3] should produce 106"

  @assert (window.__t4_r0 === 6)
    "reduce with initial=0 on [1,2,3] should produce 6"
}

@test "@computed reduce: without initial uses first element as accumulator" {
  @fixture {
    <div class="t5">No Initial</div>
  }

  // JS .reduce() without 2nd arg uses arr[0] as initial, starts from arr[1]
  @eval (eval("                                                              \
    var source = [5, 3, 7];                                                  \
    var result = source.reduce(function(acc, n) { return acc + n });          \
    window.__t5_result = result;                                             \
  "))

  @assert (window.__t5_result === 15)
    "reduce without initial on [5,3,7] should produce 15 (5 is accumulator seed)"
}

@test "@computed reduce: initial with non-zero start" {
  @fixture {
    <div class="t6">Initial Non-Zero</div>
  }

  // Simulates: initial: 1000, reduce: (sum, v) => sum + v
  @eval (eval("                                                          \
    var source = [1, 2, 3, 4, 5];                                        \
    var result = source.reduce(function(s, v) { return s + v }, 1000);   \
    window.__t6_result = result;                                         \
  "))

  @assert (window.__t6_result === 1015)
    "reduce with initial=1000 on [1..5] should produce 1015"
}


// =============================================================================
// 3. Map → Reduce Pipeline (Pulsar pattern: map field, then sum)
// =============================================================================

@test "@computed map+reduce: extract field then sum (Pulsar $totalStrength)" {
  @fixture {
    <div class="t7">Map Reduce</div>
  }

  // This is exactly the Pulsar pattern:
  //   @computed $totalStrength : number {
  //     from: $signals
  //     map: (s) => s.strength
  //     reduce: (sum, v) => sum + v
  //     initial: 0
  //   }
  @eval (eval("                                                          \
    var source = [                                                       \
      { name: 'alpha', strength: 10 },                                   \
      { name: 'beta', strength: 25 },                                    \
      { name: 'gamma', strength: 15 }                                    \
    ];                                                                   \
    var mapped = source.map(function(s) { return s.strength });          \
    var result = mapped.reduce(function(sum, v) { return sum + v }, 0);  \
    window.__t7_result = result;                                         \
    window.__t7_type = typeof result;                                    \
    window.__t7_mapped = JSON.stringify(mapped);                         \
  "))

  @assert (window.__t7_result === 50)
    "map(s => s.strength) + reduce((sum,v) => sum+v, 0) should produce 50"

  @assert (window.__t7_type === 'number')
    "map+reduce pipeline should produce a scalar number"

  @assert (window.__t7_mapped === '[10,25,15]')
    "map step should produce intermediate array [10,25,15]"
}

@test "@computed map+reduce: count items (Pulsar $signalCount)" {
  @fixture {
    <div class="t8">Count</div>
  }

  // Pulsar pattern for count:
  //   @computed $signalCount : number {
  //     from: $signals
  //     map: () => 1
  //     reduce: (sum, v) => sum + v
  //     initial: 0
  //   }
  @eval (eval("                                                          \
    var source = [{}, {}, {}, {}, {}];                                    \
    var mapped = source.map(function() { return 1 });                    \
    var result = mapped.reduce(function(sum, v) { return sum + v }, 0);  \
    window.__t8_result = result;                                         \
  "))

  @assert (window.__t8_result === 5)
    "map(() => 1) + reduce(sum) should count 5 items"
}

@test "@computed map+reduce: extract nested field" {
  @fixture {
    <div class="t9">Nested Field</div>
  }

  @eval (eval("                                                              \
    var source = [                                                           \
      { meta: { score: 4.5 } },                                             \
      { meta: { score: 3.2 } },                                             \
      { meta: { score: 4.8 } }                                              \
    ];                                                                       \
    var mapped = source.map(function(item) { return item.meta.score });      \
    var result = mapped.reduce(function(max, v) { return v > max ? v : max });\
    window.__t9_result = result;                                             \
  "))

  @assert (window.__t9_result === 4.8)
    "map(item.meta.score) + reduce(max) should produce 4.8"
}


// =============================================================================
// 4. Filter → Reduce Pipeline
// =============================================================================

@test "@computed filter+reduce: sum only active items" {
  @fixture {
    <div class="t10">Filter Reduce</div>
  }

  @eval (eval("                                                              \
    var source = [                                                           \
      { id: 1, value: 10, active: true },                                    \
      { id: 2, value: 20, active: false },                                   \
      { id: 3, value: 30, active: true },                                    \
      { id: 4, value: 40, active: false }                                    \
    ];                                                                       \
    var filtered = source.filter(function(s) { return s.active });           \
    var result = filtered.reduce(                                            \
      function(sum, s) { return sum + s.value }, 0);                         \
    window.__t10_result = result;                                            \
    window.__t10_count = filtered.length;                                    \
  "))

  @assert (window.__t10_result === 40)
    "filter(active) + reduce(sum value, 0) should produce 40 (10+30)"

  @assert (window.__t10_count === 2)
    "filter(active) should select 2 items"
}

@test "@computed filter+reduce: filter removes all items" {
  @fixture {
    <div class="t11">Filter All Out</div>
  }

  @eval (eval("                                                              \
    var source = [                                                           \
      { value: 10, active: false },                                          \
      { value: 20, active: false }                                           \
    ];                                                                       \
    var filtered = source.filter(function(s) { return s.active });           \
    var result = filtered.reduce(                                            \
      function(sum, s) { return sum + s.value }, 0);                         \
    window.__t11_result = result;                                            \
    window.__t11_count = filtered.length;                                    \
  "))

  @assert (window.__t11_result === 0)
    "filter→reduce on empty filtered set with initial=0 should produce 0"

  @assert (window.__t11_count === 0)
    "filter should produce 0 items"
}


// =============================================================================
// 5. Full Pipeline: filter → sort → limit → map → reduce
// =============================================================================

@test "@computed full pipeline: filter → sort → limit → map → reduce" {
  @fixture {
    <div class="t12">Full Pipeline</div>
  }

  // Simulates the complete computed-source pipeline:
  //   where: $.inStock
  //   sort: $.price asc
  //   limit: 3
  //   map: (p) => p.price
  //   reduce: (sum, v) => sum + v
  //   initial: 0
  @eval (eval("                                                              \
    var source = [                                                           \
      { name: 'D', price: 40, inStock: true  },                              \
      { name: 'A', price: 10, inStock: true  },                              \
      { name: 'E', price: 50, inStock: false },                              \
      { name: 'C', price: 30, inStock: true  },                              \
      { name: 'B', price: 20, inStock: true  }                               \
    ];                                                                       \
    var step1 = source;                                                      \
    var step2 = step1.filter(function(p) { return p.inStock });              \
    var step3 = step2.slice().sort(function(a, b) { return a.price - b.price }); \
    var step4 = step3.slice(0, 3);                                           \
    var step5 = step4.map(function(p) { return p.price });                   \
    var result = step5.reduce(function(sum, v) { return sum + v }, 0);       \
    window.__t12_result = result;                                            \
    window.__t12_step2 = step2.length;                                       \
    window.__t12_step3_first = step3[0].name;                                \
    window.__t12_step4 = JSON.stringify(step5);                              \
  "))

  @assert (window.__t12_step2 === 4)
    "filter(inStock) should keep 4 of 5 items"

  @assert (window.__t12_step3_first === 'A')
    "sort by price asc should put A (price=10) first"

  @assert (window.__t12_step4 === '[10,20,30]')
    "limit(3) + map(price) should produce [10,20,30]"

  @assert (window.__t12_result === 60)
    "full pipeline should produce 60 (10+20+30)"
}


// =============================================================================
// 6. Edge Cases
// =============================================================================

@test "@computed reduce: empty array with initial returns initial" {
  @fixture {
    <div class="t13">Empty With Initial</div>
  }

  // JS spec: [].reduce(fn, initial) → initial
  @eval (eval("                                                          \
    var source = [];                                                     \
    var result = source.reduce(function(s, n) { return s + n }, 0);      \
    window.__t13_result = result;                                        \
  "))

  @assert (window.__t13_result === 0)
    "reduce on empty array with initial=0 should return 0"
}

@test "@computed reduce: single element with initial" {
  @fixture {
    <div class="t14">Single With Initial</div>
  }

  // JS spec: [x].reduce(fn, init) → fn(init, x)
  @eval (eval("                                                          \
    var source = [42];                                                   \
    var result = source.reduce(function(s, n) { return s + n }, 100);    \
    window.__t14_result = result;                                        \
  "))

  @assert (window.__t14_result === 142)
    "reduce([42], fn, 100) should produce 142"
}

@test "@computed reduce: single element without initial" {
  @fixture {
    <div class="t15">Single No Initial</div>
  }

  // JS spec: [x].reduce(fn) → x (fn never called)
  @eval (eval("                                                          \
    var source = [42];                                                   \
    var result = source.reduce(function(s, n) { return s + n });         \
    window.__t15_result = result;                                        \
  "))

  @assert (window.__t15_result === 42)
    "reduce([42], fn) without initial should return 42 (fn never called)"
}

@test "@computed reduce: min value aggregation" {
  @fixture {
    <div class="t16">Min Value</div>
  }

  @eval (eval("                                                          \
    var source = [                                                       \
      { name: 'a', price: 30 },                                          \
      { name: 'b', price: 10 },                                          \
      { name: 'c', price: 50 }                                           \
    ];                                                                   \
    var mapped = source.map(function(s) { return s.price });             \
    var result = mapped.reduce(function(min, v) {                        \
      return v < min ? v : min                                           \
    }, Infinity);                                                        \
    window.__t16_result = result;                                        \
  "))

  @assert (window.__t16_result === 10)
    "min aggregation should produce 10"
}

@test "@computed reduce: max value aggregation" {
  @fixture {
    <div class="t17">Max Value</div>
  }

  @eval (eval("                                                          \
    var source = [                                                       \
      { name: 'a', score: 85 },                                          \
      { name: 'b', score: 92 },                                          \
      { name: 'c', score: 78 }                                           \
    ];                                                                   \
    var mapped = source.map(function(s) { return s.score });             \
    var result = mapped.reduce(function(max, v) {                        \
      return v > max ? v : max                                           \
    }, -Infinity);                                                       \
    window.__t17_result = result;                                        \
  "))

  @assert (window.__t17_result === 92)
    "max aggregation should produce 92"
}

@test "@computed reduce: boolean AND (all active)" {
  @fixture {
    <div class="t18">Boolean AND</div>
  }

  @eval (eval("                                                          \
    var all_active = [                                                   \
      { active: true },                                                  \
      { active: true },                                                  \
      { active: true }                                                   \
    ];                                                                   \
    var some_inactive = [                                                \
      { active: true },                                                  \
      { active: false },                                                 \
      { active: true }                                                   \
    ];                                                                   \
    var r1 = all_active.map(function(s) { return s.active }).reduce(     \
      function(a, v) { return a && v }, true);                           \
    var r2 = some_inactive.map(function(s) { return s.active }).reduce(  \
      function(a, v) { return a && v }, true);                           \
    window.__t18_all = r1;                                               \
    window.__t18_some = r2;                                              \
  "))

  @assert (window.__t18_all === true)
    "all-active reduce(&&) should produce true"

  @assert (window.__t18_some === false)
    "some-inactive reduce(&&) should produce false"
}

@test "@computed reduce: average via two-pass" {
  @fixture {
    <div class="t19">Average</div>
  }

  // Common pattern: map to values, reduce to sum, then divide by length
  @eval (eval("                                                          \
    var source = [                                                       \
      { score: 80 },                                                     \
      { score: 90 },                                                     \
      { score: 70 },                                                     \
      { score: 100 }                                                     \
    ];                                                                   \
    var mapped = source.map(function(s) { return s.score });             \
    var sum = mapped.reduce(function(a, v) { return a + v }, 0);         \
    var avg = sum / mapped.length;                                       \
    window.__t19_sum = sum;                                              \
    window.__t19_avg = avg;                                              \
  "))

  @assert (window.__t19_sum === 340)
    "sum of [80,90,70,100] should be 340"

  @assert (window.__t19_avg === 85)
    "average of [80,90,70,100] should be 85"
}


// =============================================================================
// 7. Backward Compatibility: No Reduce → Still Array
// =============================================================================

@test "@computed without reduce: filter+sort+limit produces array" {
  @fixture {
    <div class="t20">No Reduce</div>
  }

  // When reduce is absent, computed-source's ternary falls through:
  //   if (reduceFn && typeof reduceFn === 'function') { ... }
  // and result stays as array
  @eval (eval("                                                          \
    var source = [5, 3, 8, 1, 9, 2];                                     \
    var step2 = source.filter(function(n) { return n > 3 });             \
    var step3 = step2.slice().sort(function(a, b) { return a - b });     \
    var step4 = step3.slice(0, 2);                                       \
    var result = step4;                                                   \
    window.__t20_result = JSON.stringify(result);                        \
    window.__t20_isArray = Array.isArray(result);                        \
  "))

  @assert (window.__t20_isArray === true)
    "without reduce, result should still be an array"

  @assert (window.__t20_result === '[5,8]')
    "filter(>3) + sort(asc) + limit(2) should produce [5,8]"
}

@test "@computed without reduce: map-only produces array" {
  @fixture {
    <div class="t21">Map Only</div>
  }

  @eval (eval("                                                          \
    var source = [                                                       \
      { name: 'Alice' },                                                 \
      { name: 'Bob' },                                                   \
      { name: 'Charlie' }                                                \
    ];                                                                   \
    var result = source.map(function(s) { return s.name });              \
    window.__t21_result = JSON.stringify(result);                        \
    window.__t21_isArray = Array.isArray(result);                        \
  "))

  @assert (window.__t21_isArray === true)
    "map-only should still produce an array"

  @assert (window.__t21_result === JSON.stringify(['Alice','Bob','Charlie']))
    "map(s => s.name) should produce names array"
}

@test "@computed without reduce: passthrough preserves source" {
  @fixture {
    <div class="t22">Passthrough</div>
  }

  // No filter, sort, limit, map, reduce -- just returns source as-is
  @eval (eval("                                                          \
    var source = [1, 2, 3];                                              \
    var result = source;                                                  \
    window.__t22_result = JSON.stringify(result);                        \
  "))

  @assert (window.__t22_result === '[1,2,3]')
    "passthrough (no operations) should preserve source array"
}


// =============================================================================
// 8. Reduce to Object (advanced aggregation)
// =============================================================================

@test "@computed reduce: group by category" {
  @fixture {
    <div class="t23">Group By</div>
  }

  // Reduce can produce objects, not just scalars
  @eval (eval("                                                          \
    var source = [                                                       \
      { name: 'a', category: 'x', value: 10 },                           \
      { name: 'b', category: 'y', value: 20 },                           \
      { name: 'c', category: 'x', value: 30 }                            \
    ];                                                                   \
    var result = source.reduce(function(acc, item) {                     \
      if (!acc[item.category]) acc[item.category] = 0;                   \
      acc[item.category] += item.value;                                  \
      return acc;                                                        \
    }, {});                                                              \
    window.__t23_x = result.x;                                           \
    window.__t23_y = result.y;                                           \
    window.__t23_type = typeof result;                                   \
  "))

  @assert (window.__t23_x === 40)
    "group-by reduce: category 'x' should sum to 40"

  @assert (window.__t23_y === 20)
    "group-by reduce: category 'y' should sum to 20"

  @assert (window.__t23_type === 'object')
    "reduce to object should produce type 'object'"
}


// =============================================================================
// 9. Reduce Guard: computed-source checks typeof reduceFn
// =============================================================================

@test "@computed reduce guard: non-function reduce is skipped" {
  @fixture {
    <div class="t24">Non-Function Reduce</div>
  }

  // computed-source.st has: if (reduceFn && typeof reduceFn === 'function')
  // When reduce param is null/undefined, it should skip the reduce step
  @eval (eval("                                                          \
    var source = [1, 2, 3];                                              \
    var reduceFn = null;                                                  \
    var result;                                                          \
    if (reduceFn && typeof reduceFn === 'function') {                    \
      result = source.reduce(reduceFn, 0);                               \
    } else {                                                             \
      result = source;                                                   \
    }                                                                    \
    window.__t24_isArray = Array.isArray(result);                        \
    window.__t24_result = JSON.stringify(result);                        \
  "))

  @assert (window.__t24_isArray === true)
    "null reduceFn should skip reduce, preserving array"

  @assert (window.__t24_result === '[1,2,3]')
    "result should be unchanged source array"
}

@test "@computed reduce guard: undefined reduce is skipped" {
  @fixture {
    <div class="t25">Undefined Reduce</div>
  }

  @eval (eval("                                                          \
    var source = [4, 5, 6];                                              \
    var reduceFn = undefined;                                            \
    var result;                                                          \
    if (reduceFn && typeof reduceFn === 'function') {                    \
      result = source.reduce(reduceFn);                                  \
    } else {                                                             \
      result = source;                                                   \
    }                                                                    \
    window.__t25_isArray = Array.isArray(result);                        \
  "))

  @assert (window.__t25_isArray === true)
    "undefined reduceFn should skip reduce, preserving array"
}


// =============================================================================
// 10. Initial Value Null/Undefined Semantics
// =============================================================================

@test "@computed reduce: null initial calls reduce without second arg" {
  @fixture {
    <div class="t26">Null Initial</div>
  }

  // computed-source.st uses: initialVal != null ? .reduce(fn, init) : .reduce(fn)
  // null initial should fall through to .reduce(fn) without second arg
  @eval (eval("                                                          \
    var source = [10, 20, 30];                                           \
    var reduceFn = function(a, b) { return a + b };                      \
    var initialVal = null;                                               \
    var hasInit = initialVal != null;                                    \
    var result = hasInit                                                 \
      ? source.reduce(reduceFn, initialVal)                              \
      : source.reduce(reduceFn);                                         \
    window.__t26_result = result;                                        \
  "))

  @assert (window.__t26_result === 60)
    "null initial: reduce(fn) uses arr[0]=10 as seed, 10+20+30=60"
}

@test "@computed reduce: explicit 0 initial passes 0 to reduce" {
  @fixture {
    <div class="t27">Zero Initial</div>
  }

  // Ensure 0 is NOT treated as null/undefined -- it's a valid initial
  @eval (eval("                                                          \
    var source = [10, 20, 30];                                           \
    var reduceFn = function(a, b) { return a + b };                      \
    var initialVal = 0;                                                  \
    var hasInit = initialVal != null;                                    \
    var result = hasInit                                                 \
      ? source.reduce(reduceFn, initialVal)                              \
      : source.reduce(reduceFn);                                         \
    window.__t27_result = result;                                        \
  "))

  @assert (window.__t27_result === 60)
    "initial=0 is valid: reduce(fn, 0) on [10,20,30] should produce 60"
}

@test "@computed reduce: empty string initial is valid" {
  @fixture {
    <div class="t28">Empty String Initial</div>
  }

  @eval (eval("                                                          \
    var source = ['a', 'b', 'c'];                                        \
    var reduceFn = function(a, b) { return a + b };                      \
    var initialVal = '';                                                  \
    var hasInit = initialVal != null;                                    \
    var result = hasInit                                                 \
      ? source.reduce(reduceFn, initialVal)                              \
      : source.reduce(reduceFn);                                         \
    window.__t28_result = result;                                        \
  "))

  @assert (window.__t28_result === 'abc')
    "initial='' is valid: reduce(concat, '') on ['a','b','c'] should produce 'abc'"
}

@test "@computed reduce: false initial is valid" {
  @fixture {
    <div class="t29">False Initial</div>
  }

  @eval (eval("                                                          \
    var source = [true, false, true];                                     \
    var reduceFn = function(a, b) { return a || b };                     \
    var initialVal = false;                                              \
    var hasInit = initialVal != null;                                    \
    var result = hasInit                                                 \
      ? source.reduce(reduceFn, initialVal)                              \
      : source.reduce(reduceFn);                                         \
    window.__t29_result = result;                                        \
  "))

  @assert (window.__t29_result === true)
    "initial=false is valid: reduce(||, false) on [true,false,true] should produce true"
}
