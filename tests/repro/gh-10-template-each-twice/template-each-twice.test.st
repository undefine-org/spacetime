/// gh-10: a template containing @each, invoked TWICE in one page — does the
/// second instance re-render its rows? (issue: second instance reused the first
/// instance's each-binding state, so its hole content stayed at placeholder.)
///
/// Shape follows tests/unit/templates/each-data-drive.test.st: @template with a
/// selector-scoped @each, mounted via Spacetime.invokeTemplate + ST.initElement,
/// data published via ST.setData. Two hosts -> two invocations of the SAME
/// template, then assert BOTH instances rendered their rows.

@import "stdlib/testing/test"

@template &t10-row($item) {
  <li class="t10-row">`$item`</li>
}

@template &t10-list() {
  <ul class="t10-list"></ul>

  .t10-list {
    @each($t10Items as $item) {
      &t10-row($item);
    }
  }
}

@test "template with @each renders rows in BOTH invocations" {
  @fixture {
    <div class="host-a"></div>
    <div class="host-b"></div>
  }
  // Mount + init the FIRST instance.
  @eval (document.querySelector('.host-a').appendChild(Spacetime.invokeTemplate('t10-list')))
  @eval (ST.initElement(document.querySelector('.host-a .t10-list')))
  // Mount + init the SECOND instance (the one the issue says stays empty).
  @eval (document.querySelector('.host-b').appendChild(Spacetime.invokeTemplate('t10-list')))
  @eval (ST.initElement(document.querySelector('.host-b .t10-list')))
  @wait 60ms
  // Publish the array the live-page way.
  @eval (ST.setData('t10Items', [{id: 'a'}, {id: 'b'}, {id: 'c'}]))
  @wait 150ms
  @eval (window.__aRows = document.querySelectorAll('.host-a .t10-list li.t10-row').length)
  @eval (window.__bRows = document.querySelectorAll('.host-b .t10-list li.t10-row').length)
  @assert (window.__aRows === 3)
    "first instance must render 3 rows"
  @assert (window.__bRows === 3)
    "SECOND instance must render 3 rows (gh-10: second @each invocation must re-render)"
}
