/// Headless test: BUG-155-class regression in `@view`'s `view-mount` primitive
/// (stdlib/primitives/view.st) — a dotted-path subject (`@view $wrap.kind { ... }`)
/// silently rendered NOTHING (not even the initial pane), for the same root
/// cause as BUG-155/BUG-160: `resolveValue()` checked `sig in
/// window.SpacetimeLocal` and the reactive channels were keyed by
/// `'local:' + sig + ':updated'` with the LITERAL dotted string — no signal
/// is ever named "wrap.kind", so the lookup and the event channel both never
/// matched. Found in a round-2 pressure test after fixing BUG-155/BUG-160 in
/// sibling `@each` primitives; `@view`/`@match` share the same arm-resolution
/// idiom and needed the identical head+tail split.
///
/// `@match` (the render-ONCE sibling, backed by `match-render.st`) was
/// ALREADY correct — it already split on `.` — confirmed by direct source
/// read; only `@view`'s `view-mount` (the RE-RENDER-on-change sibling) had
/// the bug. This test exercises `@view` specifically.

@import "stdlib/testing/test"

@template &vdp-pane-a() {
    <p class="vdp-a">A</p>
}
@template &vdp-pane-b() {
    <p class="vdp-b">B</p>
}

@template &vdp-view() {
    <div class="vdp-host"></div>

    .vdp-host {
        @view $vdpWrap.kind {
            "a" => &vdp-pane-a();
            "b" => &vdp-pane-b();
        }
    }
}

@test "@view renders the initial pane from a dotted-path subject ($sig.field)" {
    @fixture {
        <div class="host-vdp"></div>
    }
    // @view (unlike @each) only reads SpacetimeLocal/ST.get, not the
    // ST.setData registry channel -- seed via the SAME body-scope write path
    // @mcp-input's setSource or a reactive signal assignment goes through, so
    // this exercises view-mount's actual supported channel.
    @eval (ST._bridgeExport('vdpWrap'))
    @eval (ST.set(document.body, 'vdpWrap', { kind: 'a' }))
    @eval (document.querySelector('.host-vdp').appendChild(Spacetime.invokeTemplate('vdp-view')))
    @eval (ST.initElement(document.querySelector('.host-vdp .vdp-host')))
    @wait 120ms
    @assert (document.querySelector('.host-vdp .vdp-host .vdp-a') !== null)
        "the 'a' pane must mount from a dotted-path subject (BUG-155-class regression)"
}

@test "@view re-mounts the matching pane when the HEAD signal is reassigned" {
    @fixture {
        <div class="host-vdp2"></div>
    }
    @eval (document.querySelector('.host-vdp2').appendChild(Spacetime.invokeTemplate('vdp-view')))
    @eval (ST.initElement(document.querySelector('.host-vdp2 .vdp-host')))
    @wait 60ms
    @eval (ST._bridgeExport('vdpWrap'))
    @eval (ST.set(document.body, 'vdpWrap', { kind: 'a' }))
    @wait 90ms
    @eval (window.__vdpAfterFirst = document.querySelector('.host-vdp2 .vdp-host .vdp-a') !== null)
    @eval (ST.set(document.body, 'vdpWrap', { kind: 'b' }))
    @wait 90ms
    @assert (window.__vdpAfterFirst === true)
        "first publish must mount the 'a' pane"
    @assert (document.querySelector('.host-vdp2 .vdp-host .vdp-b') !== null)
        "re-publishing the head signal must re-mount to the 'b' pane"
    @assert (document.querySelector('.host-vdp2 .vdp-host .vdp-a') === null)
        "the 'a' pane must be torn down after the swap (no stale pane left mounted)"
}
