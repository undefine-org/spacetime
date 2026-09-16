// A `&name` element ref works as an assertion SUBJECT.
//
// `&name` is how Spacetime names an element — `&box .num;` declares the ref,
// `@on &box.click` binds to it, `ST.ref("box")` returns it at runtime. So an
// author reasonably writes the same name where an assertion wants an element:
//
//     @then &box should have_text "01"
//
// That went straight into `document.querySelector("&box")`, which is not valid
// CSS, matches nothing, and reports `got "undefined"`.
//
// The failure direction is the dangerous one: the subject was ACCEPTED, so the
// test ran and looked like a real assertion about a real element. With a
// `not_exist` expectation it would have PASSED while asserting nothing at all.
//
// Resolved through `ST.subject`, one shared helper behind all ~47 assertion
// sites, so every assertion learned `&name` at once rather than 47 chances to
// miss one.
//
// NB the ref is declared with `&name selector;` — a scope-level declaration.
// `&box <div>…</div>` inside markup is NOT ref syntax (an early version of this
// gate assumed it was, and was testing a contract that never existed).

@test "a named ref resolves as an assertion subject" {
  @mount {
    <div class="host">
      <div class="num">01</div>
    </div>

    &box .num;
  }

  @then &box should have_text "01"
}

@test "a named ref subject scopes a descendant" {
  @mount {
    <div class="host">
      <div class="inner">hi</div>
    </div>

    &panel .host;
  }

  @then &panel .inner should have_text "hi"
}

// Over-widening guard: a plain CSS subject must keep working unchanged.
@test "a plain selector subject still resolves" {
  @mount {
    <div class="host">
      <div class="plain">ok</div>
    </div>
  }

  @then .plain should have_text "ok"
}
