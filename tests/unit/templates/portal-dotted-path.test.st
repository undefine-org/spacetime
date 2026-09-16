/// Headless test: BUG-155-class regression in `@portal`'s `portal-mount`
/// primitive (stdlib/primitives/portal.st) — `@portal(when: $ui.modalOpen)`
/// (a dotted-path controlling signal) never opened/closed the portal at all.
/// Root cause: `readValue()` checked `sig in window.SpacetimeLocal` (a literal
/// dotted string is never a real property name) and the `local:<sig>:updated`
/// channel/`ST.watch` were keyed by the same literal string — same shape as
/// BUG-155/160/161/effect-runner's fix. Found in a round-2 pressure-test
/// audit of every stdlib primitive sharing this dotted-signal idiom.
///
/// Also pins the WRITE-back half `setClose()` needed for `@portal(close:
/// $sig.field)` (defaults to the same `when` signal): writing `false` through
/// a dotted path can't just replace `SpacetimeLocal[sig]` (no such literal
/// key) — the fix clones the head object, patches the tail field, and
/// reassigns the WHOLE head through the Proxy so the write is observed.

@import "stdlib/testing/test"

@data inline $pdpUi : { "modalOpen": false };

@template &pdp-portal() {
    <div class="pdp-modal">
        <p>Modal content</p>
    </div>

    .pdp-modal {
        @portal(when: $pdpUi.modalOpen)
    }
}

@test "@portal opens when a dotted-path signal's head is reassigned true" {
    @fixture {
        <div class="host-pdp"></div>
    }
    @eval (document.querySelector('.host-pdp').appendChild(Spacetime.invokeTemplate('pdp-portal')))
    @eval (ST.initElement(document.querySelector('.host-pdp .pdp-modal')))
    @wait 60ms
    @assert (document.querySelector('.pdp-modal').getAttribute('aria-modal') !== 'true')
        "portal starts closed (initial dotted-path value is false)"
    @eval (ST._bridgeExport('pdpUi'))
    @eval (ST.set(document.body, 'pdpUi', { modalOpen: true }))
    @wait 90ms
    @assert (document.querySelector('.pdp-modal').getAttribute('aria-modal') === 'true')
        "@portal(when: $sig.field) must open when the head signal's field becomes true (BUG-155-class regression)"
    @assert (document.querySelector('.pdp-modal').parentElement === document.body)
        "an open portal must be relocated to document.body"
}

@test "@portal's Escape-close writes false back through a dotted close signal" {
    @fixture {
        <div class="host-pdp2"></div>
    }
    @eval (document.querySelector('.host-pdp2').appendChild(Spacetime.invokeTemplate('pdp-portal')))
    @eval (ST.initElement(document.querySelector('.host-pdp2 .pdp-modal')))
    @wait 60ms
    @eval (ST._bridgeExport('pdpUi'))
    @eval (ST.set(document.body, 'pdpUi', { modalOpen: true }))
    @wait 90ms
    @assert (document.querySelector('.pdp-modal').getAttribute('aria-modal') === 'true')
        "portal must be open before testing the close path"
    @eval (document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })))
    @wait 90ms
    @assert (document.querySelector('.pdp-modal').getAttribute('aria-modal') !== 'true')
        "Escape must close the portal"
    @assert (window.SpacetimeLocal.pdpUi.modalOpen === false)
        "closing must write false back into the NESTED field (not silently no-op or clobber sibling fields)"
}
