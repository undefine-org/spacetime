// BUG-361: `&inst &tpl(...)` must register the created element under `inst`.
//
// `template-invoke-named` documents its contract as "the created element is
// stored under the ref name for later access". It called `ST.set(container,
// ref, element)` — the SIGNAL store — while `&inst` as a selector subject, and
// `ST.ref("inst")` everywhere else, read `window.__stRefs`, the ELEMENT
// registry. Two disjoint maps, so the name was written and never findable.
//
// This is the behavior gate for that contract: it asserts through `&inst`
// (the registry path), NOT through `.class` (which passes either way and is
// why the break survived — see the note in template-doc-examples.test.st).
@import "stdlib/testing/test"

@test "a named-ref invocation registers the element under its ref" {
  @mount {
    @template &nrr-card($n) {
      <div class="nrr"><span class="num">`$n`</span></div>
    }
    <main class="page"></main>
    .page { &inst &nrr-card("01"); }
  }

  // The ref itself resolves to the created element.
  @then &inst should exist

  // And it scopes a descendant query, which is the shape assertions use.
  @then &inst .num should have_text "01"
}

@test "two named invocations register independently" {
  @mount {
    @template &nrr-two($n) {
      <div class="nrr2"><span class="v">`$n`</span></div>
    }
    <main class="page"></main>
    .page {
      &first &nrr-two("A");
      &second &nrr-two("B");
    }
  }

  @then &first .v should have_text "A"
  @then &second .v should have_text "B"
}

@test "an unregistered ref still resolves to nothing" {
  // The null contract must survive the fix: an absent ref is null, not a
  // stale element and not a throw. `not_exist` is the verb; `not exist`
  // (spaced) is not.
  @mount {
    <main class="page"></main>
  }

  @then &neverRegistered should not_exist
}
