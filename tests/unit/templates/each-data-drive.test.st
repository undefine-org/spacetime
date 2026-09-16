/// Headless test: driving an @each data source the live-page way (FUP-064).
///
/// The point this pins: a runtime `@each($src as $row)` renders by LISTENING for
/// the data-source event the live page dispatches — `data:<src>:loaded` (and
/// `local:<src>:updated`), with the array travelling in `e.detail`. That
/// listener is installed at template-INIT time, i.e. AFTER the template element
/// is appended and the runtime initializes it.
///
/// THEREFORE the headless seeding order is:
///   1. mount the template (`Spacetime.invokeTemplate`),
///   2. let it init (the @each listener is now installed),
///   3. publish the data via the documented runtime API `ST.setData(name, array)`
///      — which dispatches `data:name:loaded` with the array.
///
/// A bare `window.SpacetimeLocal.x = [...]` assignment does NOT dispatch that
/// event, so the @each never re-renders — that was the original (pre-FUP-064)
/// failure shape. `ST.setData` is the live-page publish API, so this is faithful
/// to how a real page drives a list, not a test-only hack.
///
/// NB: this covers the PLAIN selector form `.sel { @each($x as $i) { &row } }`,
/// which compiles to the @each primitive and renders headlessly. The
/// COLLECTION-REF form `&cards[] .sel { @each(...) }` is a separate, tracked
/// builder gap (BUG-085 class / PLAN-039) — see collection-ref-each.test.st.

@import "stdlib/testing/test"

// Row template rendered once per item.
@template &edd-row($item) {
    <li class="edd-row"></li>
}

// A template whose body hosts a plain selector-scoped @each over a data source.
@template &edd-list() {
    <ul class="edd-list"></ul>

    .edd-list {
        @each($eddItems as $item) {
            &edd-row($item);
        }
    }
}

@test "ST.setData after mount drives @each render (3 items)" {
    @fixture {
        <div class="host-edd"></div>
    }
    // 1+2: mount + init so the @each `data:eddItems:loaded` listener is installed.
    @eval (document.querySelector('.host-edd').appendChild(Spacetime.invokeTemplate('edd-list')))
    @eval (ST.initElement(document.querySelector('.host-edd .edd-list')))
    @wait 60ms
    // 3: publish the array the live-page way.
    @eval (ST.setData('eddItems', [{id: 'a'}, {id: 'b'}, {id: 'c'}]))
    @wait 120ms
    @assert (document.querySelector('.host-edd .edd-list').children.length === 3)
        "@each must render 3 rows after ST.setData publishes a 3-item source"
}

@test "local:<src>:updated re-publish re-renders @each (3 → 2)" {
    // A re-publish (the signal-mutation channel a live page uses when `$src` is
    // reassigned) goes through `local:<src>:updated` — a persistent document
    // listener the @each installs alongside the data-registry one. (ST.setData's
    // registry fires its subscribers ONCE then clears them, so it models the
    // INITIAL @data load; `local:<src>:updated` models subsequent mutations.)
    @fixture {
        <div class="host-edd2"></div>
    }
    @eval (document.querySelector('.host-edd2').appendChild(Spacetime.invokeTemplate('edd-list')))
    @eval (ST.initElement(document.querySelector('.host-edd2 .edd-list')))
    @wait 60ms
    @eval (document.dispatchEvent(new CustomEvent('local:eddItems:updated', { detail: [{id: 'a'}, {id: 'b'}, {id: 'c'}] })))
    @wait 90ms
    @eval (window.__eddAfterFirst = document.querySelector('.host-edd2 .edd-list').children.length)
    @eval (document.dispatchEvent(new CustomEvent('local:eddItems:updated', { detail: [{id: 'x'}, {id: 'y'}] })))
    @wait 90ms
    @assert (window.__eddAfterFirst === 3)
        "first publish must render 3 rows"
    @assert (document.querySelector('.host-edd2 .edd-list').children.length === 2)
        "re-publishing a 2-item source must re-render @each down to 2 rows"
}
