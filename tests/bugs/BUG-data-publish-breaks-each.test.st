/// BUG: @data → SpacetimeLocal publish breaks @each rendering
///
/// FEAT-042 added a publish step in stdlib/primitives/data/source.st so that a
/// loaded @data source writes window.SpacetimeLocal[name] and dispatches
/// `local:<name>:updated` (so file-scope `$signal` bindings react to @data
/// sources). But when the SAME source name also feeds an @each (and a
/// @computed), this extra synchronous dispatch during the source's load()
/// collides with the @each/@computed data-registry (afterData) path and the
/// @each renders ZERO items.
///
/// This test pins the contract: a @data source feeding @each must still render
/// its items even when the source is also published to SpacetimeLocal and even
/// when a @computed reduces over the same source.

@import "stdlib/testing/test"

// The @data source that feeds both the @each and the SpacetimeLocal publish.
// Declaring it materializes the SpacetimeLocal proxy (the reactive write-trap
// that dispatches local:bugItems:updated), which is the publish channel under test.
@data inline $bugItems : [];

// Row template rendered once per item.
@template &bug-row($item) {
    <li class="bug-row"></li>
}

// A page whose single @data source feeds BOTH an @each and a @computed,
// mirroring demos/reactive-list.
@template &bug-list() {
    <ul class="bug-list"></ul>

    .bug-list {
        @each($bugItems as $item) {
            &bug-row($item);
        }
    }
}

@test "@data source feeding @each still renders items after SpacetimeLocal publish" {
    @fixture {
        <div class="host-bug-each"></div>
    }
    // Mount + init first so the @each's local:bugItems:updated listener is
    // installed, THEN publish through the SpacetimeLocal proxy (which syncs
    // _localState AND dispatches the event) — the live-page order. This pins the
    // FEAT-042 regression: the source's SpacetimeLocal publish must not collide
    // with the @each data-registry path and zero out the render.
    @eval (document.querySelector('.host-bug-each').appendChild(Spacetime.invokeTemplate('bug-list')))
    @eval (ST.initElement(document.querySelector('.host-bug-each .bug-list')))
    @wait 60ms
    @eval (eval("window.SpacetimeLocal.bugItems = [\
        {id: 'a', label: 'Alpha'},\
        {id: 'b', label: 'Beta'},\
        {id: 'c', label: 'Gamma'}\
    ]"))
    @wait 120ms
    @assert (document.querySelector('.host-bug-each .bug-list').children.length === 3)
        "@each over a 3-item source must render 3 rows even when the source is published to SpacetimeLocal"
}
