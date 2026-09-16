/// H2 reactive-state behavioral gates (PLAN-143)
///
/// The 7 tests in tests/integration/reactive_properties.rs assert the STATE
/// NAME (`menuOpen`/`active`/`count`/`url`/`done`/`cartTotal`/`sizes`) in the
/// emitted JS. That name is emitted by STATE INIT (and, for the class-toggle
/// case, by a schema comment) regardless of whether the reactive feature
/// exists — proven by the counterexample in evidence.txt: delete the reactive
/// directive but keep the state declaration and the name is still in `js`, so
/// the assertion cannot distinguish a working feature from a deleted one.
///
/// These gates assert the BEHAVIORAL contract instead: mount a real reactive
/// component, drive it with real @when events, and assert the observable DOM
/// effect. If the class-toggle / text / attr / computed / data-source feature
/// is deleted, the behavioral assertion fails.
///
/// Logic rung: no layout/timing claims, so these run under --headless.
@import "stdlib/testing/test"

// =============================================================================
// Class toggle (H2: test_class_toggle_emits_st_watch_in_template,
//                   test_class_toggle_with_state_compiles_clean)
//   `.nav--open: $menuOpen;` must apply AND remove the class on the element as
//   the signal flips. A dead class-toggle leaves the class stuck at whatever
//   the markup starts with.
// =============================================================================

@test "class toggle applies and removes the class as the signal flips" {
  @mount {
    <button class="ct">Toggle</button>
    .ct {
      $open bool: false;
      .ct--on: $open;
      @on &.click { $open <- !$open; }
    }
  }
  @then .ct should not_have_class "ct--on"
  @when .ct click
  @then .ct should have_class "ct--on"
  @when .ct click
  @then .ct should not_have_class "ct--on"
}

// =============================================================================
// Full reactive template (H2: test_full_reactive_template_compiles)
//   state + class-toggle + content injection + event all together, as in the
//   original todo-item fixture.
// =============================================================================

@test "full reactive template: state, class-toggle, content and event work together" {
  @mount {
    <li class="todo"><span class="todo__text">Buy milk</span><button class="todo__toggle">✓</button></li>
    .todo {
      $done bool: false;
      .todo--done: $done;
      @on &.click(.todo__toggle) { $done <- !$done; }
    }
  }
  @then .todo should not_have_class "todo--done"
  @then .todo__text should have_text "Buy milk"
  @when .todo__toggle click
  @then .todo should have_class "todo--done"
}

// =============================================================================
// Text injection (H2: test_text_injection_emits_content_binding)
//   `text <- $count;` must make the element's text content reflect the signal.
// =============================================================================

@test "text injection reflects the signal value on the element" {
  @mount {
    <span class="val">0</span>
    .val {
      $count number: 0;
      text <- $count;
      @on &.click { $count <- $count + 1; }
    }
  }
  @then .val should have_exact_text "0"
  @when .val click
  @then .val should have_exact_text "1"
  @when .val click
  @then .val should have_exact_text "2"
}

// =============================================================================
// Attribute injection (H2: test_attr_injection_compiles)
//   `src <- $url;` must track the signal value on the element's attribute.
// =============================================================================

@test "attr injection tracks the signal value on the element attribute" {
  @mount {
    <img class="card-img" src="/default.jpg">
    .card-img {
      $url string: "/default.jpg";
      src <- $url;
      @on &.click { $url <- "/changed.jpg"; }
    }
  }
  @then .card-img should have_attr "src=/default.jpg"
  @when .card-img click
  @then .card-img should have_attr "src=/changed.jpg"
}

// =============================================================================
// Inline data source (H2: test_inline_data_source_compiles_with_value)
//   `@data inline $sizes` seeds the named signal with the inline value, and
//   `@each` renders every item. A dead data-source renders nothing.
// =============================================================================

@type Size { code: string; label: string; }
@data inline $sizes Size[] : [
  { "code": "S", "label": "Small" },
  { "code": "M", "label": "Medium" },
  { "code": "L", "label": "Large" }
] ;

@template &size-opt($s) {
  <li class="opt" data-code="`$s.code`">`$s.label`</li>
}

@test "inline data source seeds the signal and @each renders every item" {
  @mount {
    <div class="size-list"></div>
    .size-list { @each($sizes as $s) { &size-opt($s); } }
  }
  @then .size-list .opt should have_length 3
  @then .size-list should have_text "Small"
  @then .size-list should have_text "Medium"
  @then .size-list should have_text "Large"
}

// =============================================================================
// Computed / data fold (H2: test_computed_publishes_global_signal_and_dispatches)
//   `@data fold $cartTotal` derives from $cart; when $cart mutates the derived
//   value recomputes AND dispatches `local:cartTotal:updated` so the file-scope
//   `.total` binding re-runs. A dead fold never updates the rendered total.
// =============================================================================

@type Item { price: number; qty: number; }
body {
  $cart Item[]: [];
}
@data fold $cartTotal number from $cart : acc + (item.price * item.qty) ;
.total { text: $cartTotal; }

@test "data fold derived total recomputes on source mutation and dispatches local:cartTotal:updated" {
  @eval (window.__cartTotalDispatches = 0)
  @eval (window.addEventListener('local:cartTotal:updated', function () { window.__cartTotalDispatches += 1; }))
  @mount {
    <div class="total"></div>
    <button class="add2">+20</button>
    <button class="add5">+5</button>
    .add2 { @on &.click { $cart <- $cart.concat([{ price: 10, qty: 2 }]); } }
    .add5 { @on &.click { $cart <- $cart.concat([{ price: 5, qty: 1 }]); } }
  }
  // empty cart -> derived total 0
  @then .total should have_exact_text "0"
  // +10*2 => 20; text changing proves the fold recomputed AND the binding re-ran
  // (which requires the local:cartTotal:updated dispatch — the BUG-034 contract).
  @when .add2 click
  @then .total should have_exact_text "20"
  @when .add5 click
  @then .total should have_exact_text "25"
  @assert (window.__cartTotalDispatches >= 2)
    "each $cart mutation must dispatch local:cartTotal:updated"
}
