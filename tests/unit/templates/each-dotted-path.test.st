/// Headless test: BUG-155 regression — @each with a dotted-path source.
///
/// `@each($sig.field as $item)` (source is a dotted path into an object signal,
/// not a top-level array signal) used to silently render nothing: the
/// `each-with-templates` primitive's `resolveSourceData()` called `ST.resolve`
/// (exact signal-NAME lookup) with the LITERAL dotted string as the name — which
/// never matches any real signal, since no signal is ever named "sig.field".
/// `ST.resolvePath` (resolve the head, then walk remaining segments as property
/// access) already existed and is exactly right; the primitive just never used
/// it. Root cause + fix: see BUG-155-each-dotted-path-source-sig-field-silent.org.
///
/// This mirrors each-data-drive.test.st's headless-seeding convention (mount ->
/// init -> publish via the documented runtime API) — one `@each` per template,
/// mirroring each-data-drive.test.st's own shape (a pre-existing, separate gap
/// means two live `@each` blocks sharing one @template body don't both init
/// correctly under the headless harness; out of scope here — see the two
/// templates below, kept single-@each each, same as the file this mirrors).

@import "stdlib/testing/test"

@template &edp-row($item) {
    <li class="edp-row"></li>
}

// Control: a plain top-level array signal — must keep working unchanged.
@template &edp-plain-list() {
    <ul class="edp-plain"></ul>

    .edp-plain {
        @each($edpFlat as $item) {
            &edp-row($item);
        }
    }
}

// The regression this test pins: a dotted path into an OBJECT signal.
@template &edp-dotted-list() {
    <ul class="edp-dotted"></ul>

    .edp-dotted {
        @each($edpWrap.items as $item) {
            &edp-row($item);
        }
    }
}

@test "@each renders a plain top-level array source (control — must keep working)" {
    @fixture {
        <div class="host-edp-plain"></div>
    }
    @eval (document.querySelector('.host-edp-plain').appendChild(Spacetime.invokeTemplate('edp-plain-list')))
    @eval (ST.initElement(document.querySelector('.host-edp-plain .edp-plain')))
    @wait 60ms
    @eval (ST.setData('edpFlat', ['a', 'b', 'c']))
    @wait 120ms
    @assert (document.querySelector('.host-edp-plain .edp-plain').children.length === 3)
        "plain-source @each must render 3 rows"
}

@test "@each renders a dotted-path source ($sig.field) — BUG-155 regression" {
    @fixture {
        <div class="host-edp-dotted"></div>
    }
    @eval (document.querySelector('.host-edp-dotted').appendChild(Spacetime.invokeTemplate('edp-dotted-list')))
    @eval (ST.initElement(document.querySelector('.host-edp-dotted .edp-dotted')))
    @wait 60ms
    // Dotted source: an OBJECT signal whose nested `.items` field is the array
    // the dotted @each actually reads.
    @eval (ST.setData('edpWrap', { items: ['p', 'q', 'r'] }))
    @wait 120ms
    @assert (document.querySelector('.host-edp-dotted .edp-dotted').children.length === 3)
        "dotted-path-source @each must render 3 rows (BUG-155 regression)"
}

@test "dotted-path @each re-renders when the HEAD signal is reassigned" {
    @fixture {
        <div class="host-edp-dotted2"></div>
    }
    @eval (document.querySelector('.host-edp-dotted2').appendChild(Spacetime.invokeTemplate('edp-dotted-list')))
    @eval (ST.initElement(document.querySelector('.host-edp-dotted2 .edp-dotted')))
    @wait 60ms
    // Bridge $edpWrap so ST.set's body-scope write dispatches local:edpWrap:updated
    // (BUG-156's fix: a real primitive bridges its OWN exports unconditionally;
    // this test has no direct interpolation of $edpWrap to trigger the compiler's
    // codegen bridge, so it bridges explicitly here, same effect).
    @eval (ST._bridgeExport('edpWrap'))
    // Reassigning the HEAD signal ($edpWrap, not a literal "edpWrap.items" signal)
    // must re-derive the dotted path and re-render via ST.set on the body element
    // (the same body-scope write path @mcp-input's setSource or a reactive signal
    // assignment ultimately goes through).
    @eval (ST.set(document.body, 'edpWrap', { items: ['p', 'q', 'r'] }))
    @wait 90ms
    @eval (window.__edpAfterFirst = document.querySelector('.host-edp-dotted2 .edp-dotted').children.length)
    @eval (ST.set(document.body, 'edpWrap', { items: ['x', 'y'] }))
    @wait 90ms
    @assert (window.__edpAfterFirst === 3)
        "first publish must render 3 rows"
    @assert (document.querySelector('.host-edp-dotted2 .edp-dotted').children.length === 2)
        "re-publishing the head signal must re-render the dotted @each down to 2 rows"
}
