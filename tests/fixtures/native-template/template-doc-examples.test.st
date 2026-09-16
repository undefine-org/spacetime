// PLAN-070 W5 doc+fixture convergence: every @example in
// stdlib/macros/template.st compiles verbatim. Colocated here (not scattered
// across BUG-165/166 fixtures) as the canonical "does the doc lie" regression
// suite for this macro file. BUG-165's optional-element-param.test.st and
// BUG-166's on-visible-animation-only.test.st already cover the "with element
// params" and "with scoped animations" examples respectively — this file
// covers the remaining ones: basic value params, invocation styles (bare /
// body-slot / named-ref), and optional value params.
//
// NOTE: every invocation below uses a NAMED REF (`&inst &tpl(...)`) instead of
// the doc's bare form (`&tpl(...);`), originally as a BUG-169 workaround for
// "a bare invocation's holes never interpolate under `@mount`".
//
// That premise is STALE — re-measured directly, a bare invocation interpolates
// correctly under `@mount`:
//
//   .page { &card("01", "Design"); }   ->  .num renders "01"   ✓
//
// What was actually broken was the named-ref form these tests switched TO:
// `&inst &tpl(...)` never registered its element in `window.__stRefs`, so
// every `&inst`-subject assertion resolved to null and reported `got
// undefined` (BUG-361, fixed). The workaround, not the bare form, was the
// thing that did not work.
//
// The named-ref spelling is kept here deliberately: it is now the only
// coverage of the ref-registry path in this directory, and it is worth
// asserting on its own.
//
// NOT covered here: the `template-inline` unlinked-ref DEFINITION form
// (`&name(params) { body }`) — that example is a confirmed RED fixture
// (BUG-168, filed separately: a distinct construct-recognition gap, not a
// body-content gap like BUG-165/166/167).
@import "stdlib/testing/test"

@test "basic value params: multi-value-param interpolation" {
  // Uses plain string params (not the doc's dotted-field `$s.id` object form)
  // — this test's purpose is proving multi-value-param interpolation compiles
  // and renders; the `$s.field` dotted-access form is exercised more directly
  // by BUG-165/166's fixtures and the ssg_unroll integration tests.
  @mount {
    @template &probe-service-card($number, $title) {
      <div class="service">
        <span class="num">`$number`</span>
        <h3>`$title`</h3>
      </div>
    }
    <main class="page"></main>
    .page { &inst &probe-service-card("01", "Design"); }
  }
  @then &inst .num {
    text == "01";
  }
  @then &inst h3 {
    text == "Design";
  }
}

// A ref resolves to the template's ROOT element, so `&slotInst` IS the
// `.greet` node — `&slotInst .greet` asks for a `.greet` INSIDE it and
// correctly finds nothing. That was a test-authoring error, not a compiler
// bug: measured, `&si` carries class `greet` and the right text, while
// `&si .greet` is empty. Fixed below to assert on the ref itself.
@test "invocation styles: named-ref and body-slot both work" {
  @mount {
    @template &probe-greeting($name) {
      <p class="greet">Hello, `$name`!</p>
    }
    @template &probe-counter() {
      <div class="ctr">0</div>
    }
    <main class="page">
      <div class="slot-wrap"></div>
      <div class="named-wrap"></div>
    </main>
    .slot-wrap {
      &slotInst &probe-greeting("Slotted") {
        <span class="extra">ignored (no &param to fill)</span>
      }
    }
    .named-wrap {
      &myCounter &probe-counter();
    }
  }
  // The ref IS the template root (`.greet`), not a wrapper around it.
  @then &slotInst {
    text == "Hello, Slotted!";
  }
  @then &myCounter {
    exist;
  }
}

// STATUS — RED for a feature that does not exist: `@if` has no implementation
// anywhere (BUG-362). Not a regression, and not specific to templates — even a
// literal `@if(true)` at page level renders nothing (`E0956: unknown
// primitive: conditional`). Kept red on purpose: this is the executable
// statement of what `$title?` is FOR, and the docs it mirrors have been
// corrected to the unconditional shape in the meantime.
@test "optional value param: $title? renders conditionally per the documented contract" {
  @mount {
    @template &probe-greeting2($name, $title?) {
      <div class="greeting">
        @if($title) {
          <span class="title">`$title`</span>
        }
        <span class="name">`$name`</span>
      </div>
    }
    <main class="page"></main>
    .page {
      &noTitle &probe-greeting2("Alice");
      &withTitle &probe-greeting2("Alice", "Dr.");
    }
  }
  @then &noTitle .title {
    not_exist;
  }
  @then &withTitle .title {
    text == "Dr.";
  }
}
