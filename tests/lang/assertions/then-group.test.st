/// BUG-300: the selector-less `@then { … }` grouping form.
///
/// The shape was in use at 13 corpus sites with NO declaration behind it —
/// it rode the same trailing-token leniency `@use`'s clauses rode (BUG-299),
/// and surfaced as E0946 once BUG-264 closed that leniency. `%macro then-group`
/// (stdlib/testing/test.st) declares it.
///
/// These are RUN gates, not compile gates, and deliberately so: the first
/// version of `then-group` COMPILED CLEAN while emitting nothing that ran
/// (`%$body` interpolates source tokens; `%$body.js` interpolates the expanded
/// emission). Only executing the file revealed it — "Test made no assertions".
/// A `spacetime check` gate would have passed the dead version.

@import "stdlib/testing/test"

@test "a selector-less @then groups independent subject assertions" {
  @mount { <main class="ed"><p>Hello</p><span class="s">World</span></main> }
  @then {
    @then .ed p { text ~= "Hello"; }
    @then .ed .s { text ~= "World"; }
  }
}

@test "a group carries @assert" {
  @mount { <main class="grp"><p>Hello</p></main> }
  @then {
    @assert (document.querySelector('.grp p').textContent === 'Hello') "the paragraph reads Hello";
  }
}

/// The grouping form and the claims form are DISJOINT: `$subject:selector`
/// cannot match an empty head, so declaring both introduces no ambiguity.
/// This test pins that the subject form still works beside its new sibling.
@test "the subject form still resolves with the group form declared" {
  @mount { <main class="cl"><p>Subject</p></main> }
  @then .cl p { text ~= "Subject"; }
}
