/// Headless test: BUG-155-class regression in `each-inline` (the inline-HTML-body
/// sibling of `each-with-templates`) — a dotted-path @each source silently
/// rendered nothing here too, for the SAME reason as BUG-155's
/// `each-with-templates` bug: the reactive channels (`data:*:loaded`,
/// `local:*:updated`) and the initial-paint lookup were all keyed by the
/// LITERAL dotted string, never the head signal name. Found in a round-2
/// pressure-test after fixing `each-with-templates` — this sibling primitive
/// was never covered by that fix since it's a separate `.st` file.
///
/// Mirrors each-dotted-path.test.st's structure + each-data-drive.test.st's
/// headless-seeding convention, but for the `@each(...) { <li>raw html</li> }`
/// inline-body form instead of the `&template(...)` invocation form.

@import "stdlib/testing/test"

@template &eid-list() {
    <ul class="eid-dotted"></ul>

    .eid-dotted {
        @each($eidWrap.items as $item) {
            <li>`$item`</li>
        }
    }
}

@test "inline-body @each renders a dotted-path source ($sig.field)" {
    @fixture {
        <div class="host-eid"></div>
    }
    @eval (document.querySelector('.host-eid').appendChild(Spacetime.invokeTemplate('eid-list')))
    @eval (ST.initElement(document.querySelector('.host-eid .eid-dotted')))
    @wait 60ms
    @eval (ST.setData('eidWrap', { items: ['x', 'y'] }))
    @wait 120ms
    @assert (document.querySelector('.host-eid .eid-dotted').children.length === 2)
        "dotted-path-source inline @each must render 2 rows (BUG-155-class regression)"
    @assert (document.querySelector('.host-eid .eid-dotted').children[0].textContent === 'x')
        "first row must interpolate the item value, not a stale/empty token"
}

@test "inline-body @each re-renders when the HEAD signal is reassigned" {
    @fixture {
        <div class="host-eid2"></div>
    }
    @eval (document.querySelector('.host-eid2').appendChild(Spacetime.invokeTemplate('eid-list')))
    @eval (ST.initElement(document.querySelector('.host-eid2 .eid-dotted')))
    @wait 60ms
    @eval (ST._bridgeExport('eidWrap'))
    @eval (ST.set(document.body, 'eidWrap', { items: ['a', 'b', 'c'] }))
    @wait 90ms
    @eval (window.__eidAfterFirst = document.querySelector('.host-eid2 .eid-dotted').children.length)
    @eval (ST.set(document.body, 'eidWrap', { items: ['z'] }))
    @wait 90ms
    @assert (window.__eidAfterFirst === 3)
        "first publish must render 3 rows"
    @assert (document.querySelector('.host-eid2 .eid-dotted').children.length === 1)
        "re-publishing the head signal must re-render the inline @each down to 1 row"
}
