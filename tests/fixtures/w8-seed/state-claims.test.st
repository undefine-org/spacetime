/// W8 SEED: state assertions rewritten to @mount + @then{claims}.
@import "stdlib/testing/test"

@test "have_state passes when element is in state" {
  @mount { <div class="state-test" data-st-state="active">State Test</div> }
  @then .state-test { state == "active"; }
}

@test "have_state gates on wrong state" {
  // An element in state "inactive" must NOT satisfy a claim of "active" — the
  // gate's job. Assert the negative directly (state != "active"); a positive
  // `state == "active"` here would be asserting the element IS active, which it
  // is not, and fail. `!=` pins that the gate correctly distinguishes states.
  @mount { <div class="wrong-state" data-st-state="inactive">Wrong</div> }
  @then .wrong-state { state != "active"; }
}

// FAILING BY DESIGN — BUG-218.
//
// This test's `@mount` body mixes markup with a NESTED SCOPE, and such a body
// reaches emit as a serialized capture dump rather than source text, so it fails
// to parse and compiles to NOTHING. It therefore mounted nothing and its `exist`
// claim asserted nothing, while reporting PASS — for as long as it has existed.
//
// BUG-216's fix (a body that fails to parse now emits a throw instead of an empty
// string) is what made that visible. The test is left failing on purpose: per
// PLAN-113, green is not restored by weakening an assertion. It goes green when
// BUG-218 is fixed and the body actually mounts.
//
// `!tasks/bugs/BUG-218-mount-body-with-nested-scope-reaches-em.org`
@test "transition_count via claim" {
  @mount { <button class="t">+</button>
    .t { @behavior .t machine(initial: "a") { a -> b on click; } } }
  @then .t { exist; }
}
