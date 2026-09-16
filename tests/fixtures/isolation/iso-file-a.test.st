/// BUG-107 isolation guard — file A.
///
/// Mounts a template and drives an `@each` data source the live-page way
/// (`ST.setData`). Paired with iso-file-b (a DIFFERENT source name); both are
/// run through the multi-file headless harness in ONE invocation. Before the
/// per-file-context fix, file B's mount left its global MutationObserver
/// observing a `document.body` that still held file A's stale nodes, so the
/// observer's `_scheduleInit`/`_handleRemovedNode` callbacks walked a null node
/// and threw `Cannot read properties of null (reading 'nodeType')` — flipping
/// these passing tests RED purely by run order. With a fresh V8 context per
/// file, no observer/registry/DOM leaks across files, so both stay GREEN.

@import "stdlib/testing/test"

@template &isoa-row($item) {
    <li class="isoa-row"></li>
}

@template &isoa-list() {
    <ul class="isoa-list"></ul>

    .isoa-list {
        @each($isoaItems as $item) {
            &isoa-row($item);
        }
    }
}

@test "iso A ST.setData drives @each render (3 items)" {
    @fixture {
        <div class="host-isoa"></div>
    }
    @eval (document.querySelector('.host-isoa').appendChild(Spacetime.invokeTemplate('isoa-list')))
    @eval (ST.initElement(document.querySelector('.host-isoa .isoa-list')))
    @wait 60ms
    @eval (ST.setData('isoaItems', [{id: 'a'}, {id: 'b'}, {id: 'c'}]))
    @wait 120ms
    @assert (document.querySelector('.host-isoa .isoa-list').children.length === 3)
        "iso A @each must render 3 rows after ST.setData"
}
