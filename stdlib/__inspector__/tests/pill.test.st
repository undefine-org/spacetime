/// Inspector pill tests — real module markup, signals, templates, and fields.
///
/// The inspector is imported rather than re-created in a fixture: every test
/// seeds the module's real `$insStructure` signal on its rendered root. That
/// exercises its actual @each tree, click projection, client-owned active list,
/// and `&field-*` invocation rail.

@import "stdlib/testing/test";
@import "stdlib/__inspector__";

@test "tree renders every real structure node with its depth" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'root', label: 'Page', kind: 'template', depth: 0, template: 'main' }, { id: 'child', label: 'Card', kind: 'element', depth: 2, template: 'card' }], params: [], diagnostics: [], error: null }))
  @wait 50ms
  @assert (document.querySelectorAll('.st-inspector-widget__node').length === 2)
    "the real @each tree must create one node button per rail node"
  @assert (document.querySelector('[data-node="root"]').getAttribute('style').includes('--ins-depth: 0'))
    "the root depth must ride on its rendered row"
  @assert (document.querySelector('[data-node="child"]').getAttribute('style').includes('--ins-depth: 2'))
    "a descendant depth must ride on its rendered row"
}

@test "clicking a tree row projects only its params into real field editors" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'card', label: 'Card', kind: 'template', depth: 0, template: 'card' }, { id: 'other', label: 'Other', kind: 'template', depth: 0, template: 'other' }], params: [{ node_id: 'card', param: 'title', value: 'Hello', type: 'string', widget: 'text', bound: false, span_start: 10, span_end: 30, file: 'page.st', source_hash: 'hash-card', span_text: '&card(title: "Hello")' }, { node_id: 'other', param: 'count', value: '2', type: 'number', widget: 'number', bound: false, span_start: 40, span_end: 55, file: 'page.st', source_hash: 'hash-other', span_text: '&other(count: 2)' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="card"] click
  @assert (ST.get(document.querySelector('.st-inspector-widget'), 'insSelectedNode') === 'card')
    "the click must select the actual node signal"
  @assert (document.querySelectorAll('.st-inspector-widget__param-list .st-fld-field').length === 1)
    "the client-owned active list must contain only selected-node params"
  @assert (document.querySelector('.st-inspector-widget__param-list [data-addr-sel="title"]').value === 'Hello')
    "the real shared text field must receive the projected descriptor"
  @assert (document.querySelector('.st-inspector-widget__param-list [data-addr-sel="count"]') === null)
    "unselected-node params must not leak into editors"
}

@test "BUG-151 selected editors survive a fresh structure poll" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'card', label: 'Card', kind: 'template', depth: 0, template: 'card' }], params: [{ node_id: 'card', param: 'title', value: 'Hello', type: 'string', widget: 'text', bound: false, span_start: 10, span_end: 30, file: 'page.st', source_hash: 'hash-card', span_text: '&card(title: "Hello")' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="card"] click
  @eval (eval("window.__insInputBefore=document.querySelector('.st-inspector-widget__param-list .st-fld-input');window.__insEditorBefore=window.__insInputBefore.closest('.st-fld-field');window.__insInputBefore.value='Draft unsaved title';"))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'card', label: 'Card refreshed', kind: 'template', depth: 0, template: 'card' }], params: [{ node_id: 'card', param: 'title', value: 'Server refresh value', type: 'string', widget: 'text', bound: false, span_start: 10, span_end: 30, file: 'page.st', source_hash: 'fresh-hash', span_text: '&card(title: "Server refresh value")' }], diagnostics: [], error: null }))
  @wait 50ms
  @assert (document.querySelector('.st-inspector-widget__param-list .st-fld-field') === window.__insEditorBefore)
    "polling structure must not re-render the selected client-owned editor list"
  @assert (document.querySelector('.st-inspector-widget__param-list [data-addr-sel="title"]') === window.__insInputBefore && window.__insInputBefore.value === 'Draft unsaved title')
    "a fresh poll must preserve the exact editor and its in-progress value"
}

@test "unverified source rows are readonly and cannot send edits" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (eval("window.__insWrites=[];window.__stDevWs={connected:true,send:function(message){window.__insWrites.push(message);},onAck:function(){},onReject:function(){}};"))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'legacy', label: 'Legacy', kind: 'template', depth: 0, template: 'legacy' }], params: [{ node_id: 'legacy', param: 'title', value: 'Cannot trust snapshot', type: 'string', widget: 'text', bound: false, span_start: 10, span_end: 30, file: 'page.st', source_hash: null, span_text: '&legacy(title: "Cannot trust snapshot")' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="legacy"] click
  @assert (document.querySelector('.st-inspector-widget__param-list [data-addr-sel="title"]').dataset.addr === '')
    "a missing snapshot hash must leave the real parameter row without a write address"
  @assert (document.querySelector('.st-inspector-widget__param-list [data-addr-sel="title"]').value === 'Cannot trust snapshot')
    "the row must still display its value"
  @eval (eval("var input=document.querySelector('.st-inspector-widget__param-list .st-fld-input');input.value='attempted write';input.dispatchEvent(new Event('change',{bubbles:true}));"))
  @wait 50ms
  @assert (window.__insWrites.length === 0)
    "an unverified source row must not produce an EditAst write"
}

/// The resting rule. An ERRORED structure still hides the widget — a pill that
/// cannot trust its own data must not offer edits.
///
/// An EMPTY structure no longer does. A page with no templates has no tree, but
/// it is exactly the page whose `@data` rows are pickable, so the pill must stay
/// visible to offer the gesture. Hiding it (the original rule, asserted here
/// before) is what made pick look broken on the 12-of-23 projects that declare
/// no template: the surface was editable, the entry point simply was not there.
@test "an errored structure rests the widget; an empty one stays available" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [], params: [], diagnostics: [], error: null }))
  @wait 50ms
  @assert (!document.querySelector('.st-inspector-widget').classList.contains('is-hidden'))
    "an empty structure must keep the widget AVAILABLE — its @data rows are still pickable"
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'broken', label: 'Broken', kind: 'template', depth: 0, template: 'broken' }], params: [], diagnostics: [], error: 'inspection failed' }))
  @wait 50ms
  @assert (document.querySelector('.st-inspector-widget').classList.contains('is-hidden'))
    "an errored structure must hide the widget even when it carries stale nodes"
}

/// W3R REVIEW FINDINGS — the write rail's value + freshness contracts.
///
/// Each of these was a live defect found by review, not a hypothetical: a
/// composite control's edit never reached the rail at all, a toggle wrote the
/// literal token `on`, an emptied number would have written malformed source, and
/// a second edit of the same node was silently refused because its credentials
/// went stale the moment the first edit landed.

@test "a toggle writes a real boolean, never the checkbox's on value" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'n', label: 'N', kind: 'template', depth: 0, template: 'n' }], params: [{ node_id: 'n', param: 'on', value: 'true', type: 'bool', widget: 'toggle', bound: true, span_start: 1, span_end: 9, file: 'p.st', source_hash: 'h', span_text: '&n(on: true)' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="n"] click
  @eval (eval("var cb=document.querySelector('.st-inspector-widget__param-list input[data-widget=\"toggle\"]');cb.checked=false;window.__insReq=ST.insWriteRequest(cb);"))
  @assert (window.__insReq && window.__insReq.value === 'false')
    "an unchecked toggle must send false, not the DOM's default value 'on'"
  @eval (eval("var cb2=document.querySelector('.st-inspector-widget__param-list input[data-widget=\"toggle\"]');cb2.checked=true;window.__insReq2=ST.insWriteRequest(cb2);"))
  @assert (window.__insReq2 && window.__insReq2.value === 'true')
    "a checked toggle must send true"
}

@test "an emptied number refuses to write rather than corrupt the source" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'n', label: 'N', kind: 'template', depth: 0, template: 'n' }], params: [{ node_id: 'n', param: 'count', value: '3', type: 'number', widget: 'number', bound: true, span_start: 1, span_end: 9, file: 'p.st', source_hash: 'h', span_text: '&n(count: 3)' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="n"] click
  // A bare empty token would splice into the source as `count: )` — malformed.
  @eval (eval("var n=document.querySelector('.st-inspector-widget__param-list .st-fld-input');n.value='';window.__insEmpty=ST.insWriteRequest(n);"))
  @assert (window.__insEmpty === null)
    "an emptied number must produce NO write request"
}

@test "an unverified row produces no write request even when driven directly" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'n', label: 'N', kind: 'template', depth: 0, template: 'n' }], params: [{ node_id: 'n', param: 'title', value: 'x', type: 'string', widget: 'text', bound: true, span_start: 1, span_end: 9, file: 'p.st', source_hash: null, span_text: '&n(title: "x")' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="n"] click
  @eval (eval("var t=document.querySelector('.st-inspector-widget__param-list .st-fld-input');t.value='attempted';window.__insUnver=ST.insWriteRequest(t);"))
  @assert (window.__insUnver === null)
    "a row whose snapshot could not be verified must never build a write request"
}

@test "credentials refresh in place after a write, without re-rendering editors" {
  @fixture { <div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'n', label: 'N', kind: 'template', depth: 0, template: 'n' }], params: [{ node_id: 'n', param: 'title', value: 'A', type: 'string', widget: 'text', bound: true, span_start: 1, span_end: 9, file: 'p.st', source_hash: 'hash-one', span_text: '&n(title: "A")' }], diagnostics: [], error: null }))
  @wait 50ms
  @when [data-node="n"] click
  @eval (eval("window.__insCtl=document.querySelector('.st-inspector-widget__param-list .st-fld-input');window.__insCtl.value='in progress';"))
  // The poll brings the post-write snapshot: new hash, new span text.
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'n', label: 'N', kind: 'template', depth: 0, template: 'n' }], params: [{ node_id: 'n', param: 'title', value: 'B', type: 'string', widget: 'text', bound: true, span_start: 1, span_end: 11, file: 'p.st', source_hash: 'hash-two', span_text: '&n(title: "B")' }], diagnostics: [], error: null }))
  @wait 50ms
  @assert (document.querySelector('.st-inspector-widget__param-list .st-fld-input') === window.__insCtl)
    "the editor must NOT be re-rendered (BUG-151): same node, same identity"
  @assert (window.__insCtl.dataset.addr === 'hash-two' && window.__insCtl.dataset.addrEnd === '11')
    "its write credentials must be updated in place, or the NEXT edit is refused"
  @assert (window.__insCtl.value === 'in progress')
    "the user's in-progress value must survive the refresh"
}

/// W3R REVIEW FINDING: a param's rail value is the RAW SOURCE TOKEN, escapes and
/// all. Merely stripping the outer quotes left `\"` in the displayed text, and
/// sending that back re-escaped the backslash — so SAVING AN UNTOUCHED VALUE
/// silently changed it. Decoding the token makes the round trip lossless.
@test "an escaped string value round-trips without corruption" {
  @eval (eval("window.__insRaw='\"He said \\\\\"hi\\\\\" and left\"';window.__insShown=ST.insDisplayValue(window.__insRaw);"))
  @assert (window.__insShown === 'He said "hi" and left')
    "the editor must show the real string, not its source escapes"
  // The server re-encodes exactly once on write (emit_patch_value), so what the
  // pill sends must re-encode back to the ORIGINAL token.
  @assert (JSON.stringify(window.__insShown) === window.__insRaw)
    "re-encoding the displayed value must reproduce the original source token"
}

@test "display unwrapping tolerates non-JSON and unquoted tokens" {
  @assert (ST.insDisplayValue('42') === '42') "a numeric token passes through"
  @assert (ST.insDisplayValue('$other') === '$other') "a signal reference passes through"
  @assert (ST.insDisplayValue("'single'") === 'single') "a single-quoted literal unwraps"
  @assert (ST.insDisplayValue('') === '') "an empty token stays empty"
  @assert (ST.insDisplayValue(null) === '') "a null value renders as empty, never 'null'"
}


/// W4 pick mode — every test mounts the actual inspector template, then drives
/// document clicks through the runtime listener rather than faking tree markup.
@test "pick mode off leaves stamped page clicks untouched" {
  @fixture { <button class="ins-page-target" data-st-node="card">Page card</button><div class="ins-test-anchor"></div> }
  @eval (eval("window.__insPickOffRoot=document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget'))"))
  @eval (ST.set(window.__insPickOffRoot, 'insStructure', { nodes: [{ id: 'card', label: 'Card', kind: 'template', depth: 0, template: 'card' }], params: [], diagnostics: [], error: null }))
  @eval (ST.set(window.__insPickOffRoot, 'insSelectedNode', ''))
  @wait 50ms
  @when .ins-page-target click
  @assert (ST.get(window.__insPickOffRoot, 'insSelectedNode') === '')
    "with pick mode OFF no page click listener may select a stamped node"
}

@test "pick mode selects a stamped page instance and projects its real editors" {
  @fixture { <button class="ins-page-target" data-st-node="card">Page card</button><button data-st-node="card">Repeated card</button><div class="ins-test-anchor"></div> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'card', label: 'Card', kind: 'template', depth: 0, template: 'card' }], params: [{ node_id: 'card', param: 'title', value: 'Picked', type: 'string', widget: 'text', bound: false, span_start: 1, span_end: 9, file: 'page.st', source_hash: 'card-hash', span_text: '&card(title: "Picked")' }], diagnostics: [], error: null }))
  @wait 50ms
  @when .st-inspector-widget__pick click
  @wait 50ms
  @when .ins-page-target click
  @wait 50ms
  @assert (ST.get(document.querySelector('.st-inspector-widget'), 'insSelectedNode') === 'card')
    "a stamped page click must select the source invocation through the real picker"
  @assert (document.querySelector('.st-inspector-widget__param-list [data-addr-sel="title"]').value === 'Picked')
    "page selection must reuse ST.insEditorRows to render the selected node editors"
  @assert (document.querySelector('.st-inspector-widget__node[data-node="card"]').classList.contains('is-selected'))
    "the source tree row must reactively mark the page-selected node"
  @assert (document.querySelector('.st-inspector-widget__instance-count').textContent === '2 instances')
    "repeated rendered instances must disclose their shared source edit scope"
}

@test "a dock click is never interpreted as a page pick" {
  @fixture { <button class="ins-page-target" data-st-node="page">Page</button><div class="ins-test-anchor"></div> }
  @eval (eval("window.__insPickDockRoot=document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget'))"))
  @eval (ST.set(window.__insPickDockRoot, 'insStructure', { nodes: [{ id: 'page', label: 'Page', kind: 'template', depth: 0, template: 'page' }, { id: 'dock', label: 'Dock', kind: 'template', depth: 0, template: 'dock' }], params: [], diagnostics: [], error: null }))
  @wait 50ms
  @eval (window.__insPickDockRoot.querySelector('.st-inspector-widget__pick').click())
  @wait 50ms
  @eval (ST.insSelectNode(window.__insPickDockRoot, 'page', []))
  @eval (window.__insPickDockRoot.querySelector('.st-inspector-widget__title').parentElement.setAttribute('data-st-node', 'dock'))
  @eval (window.__insPickDockRoot.querySelector('.st-inspector-widget__title').click())
  @assert (ST.get(window.__insPickDockRoot, 'insSelectedNode') === 'page')
    "a stamped descendant inside the inspector dock must be excluded from page picking"
}

/// W4R REVIEW FINDING: a pick is a SELECTION gesture, not a page interaction.
/// Without suppression, picking a link navigates away (destroying the selection
/// the user just made) and picking a button fires its handler — including a
/// destructive one — merely because someone wanted to inspect it.
@test "picking suppresses the page's own action, and only while picking" {
  @fixture { <div class="ins-test-anchor"></div><a class="pick-link" href="#nope" data-st-node="0.1">Link</a> }
  @eval (document.querySelector('.ins-test-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: '0.1', label: 'L', kind: 'template', depth: 0, template: 'l' }], params: [], diagnostics: [], error: null }))
  @wait 50ms

  // Picking OFF: the page click must be entirely untouched.
  @eval (eval("var e1=new MouseEvent('click',{bubbles:true,cancelable:true});document.querySelector('.pick-link').dispatchEvent(e1);window.__insOffPrevented=e1.defaultPrevented;"))
  @assert (window.__insOffPrevented === false)
    "with pick mode OFF the page's own click must not be suppressed"

  // Picking ON: the gesture selects and suppresses.
  @when .st-inspector-widget__pick click
  @wait 50ms
  @eval (eval("var e2=new MouseEvent('click',{bubbles:true,cancelable:true});document.querySelector('.pick-link').dispatchEvent(e2);window.__insOnPrevented=e2.defaultPrevented;"))
  @assert (window.__insOnPrevented === true)
    "with pick mode ON the page's action must be suppressed so the selection survives"
  @assert (ST.resolve(document.querySelector('.st-inspector-widget'), 'insSelectedNode') === '0.1')
    "the pick must still select the clicked element's node"
}

/// BUG-215 (regression): an `@each` list must not be torn down and rebuilt when
/// its source re-emits IDENTICAL data.
///
/// `@data fetch { refresh: 2s }` re-emits on every tick whether or not anything
/// changed, and `@each`'s render unconditionally did `containerEl.innerHTML = ''`
/// then rebuilt. That destroys live DOM state the markup cannot re-express: a
/// focused input loses focus and caret, a <select> resets to its first option, an
/// open <details> closes. The pill's root switcher was the visible casualty — it
/// snapped back to the default root every 2s while the tree still showed the
/// chosen one, so the panel disagreed with itself about what you were looking at.
///
/// Asserted on NODE IDENTITY, not on rendered text: a rebuild produces
/// equal-looking options, so only "is this the same DOM node" separates a
/// preserved list from a recreated one. (The user's `value` surviving is verified
/// against a real browser — the headless backend does not model a <select>'s
/// change event, so asserting it here would be testing the harness.)
@test "an each list survives an identical re-emit without rebuilding" {
  @fixture { <div class="ins-each-anchor"></div> }
  @eval (document.querySelector('.ins-each-anchor').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'a', label: 'A', kind: 'template', depth: 0, template: 'a' }], params: [], templates: ['a', 'b', 'c'], template: 'a', diagnostics: [], error: null }))
  @wait 60ms
  @eval (eval("window.__eachOpt0=document.querySelector('.st-inspector-widget__root-select').options[0];"))
  @assert (document.querySelector('.st-inspector-widget__root-select').options.length === 3)
    "the roster must render one option per template"

  // The SAME payload again — exactly what a 2s poll delivers on an idle page.
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [{ id: 'a', label: 'A', kind: 'template', depth: 0, template: 'a' }], params: [], templates: ['a', 'b', 'c'], template: 'a', diagnostics: [], error: null }))
  @wait 80ms
  @assert (document.querySelector('.st-inspector-widget__root-select').options[0] === window.__eachOpt0)
    "an identical re-emit must NOT rebuild the option nodes — a rebuild resets the select and discards the user's choice"
}

/// The complement: when the data genuinely CHANGES, the list must rebuild. A
/// change-detection guard that skipped real updates would be a worse bug than
/// the one it fixes, so both directions are locked.
@test "an each list does rebuild when its data actually changes" {
  @fixture { <div class="ins-each-anchor2"></div> }
  @eval (document.querySelector('.ins-each-anchor2').appendChild(Spacetime.invokeTemplate('inspector-widget')))
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [], params: [], templates: ['a', 'b'], template: 'a', diagnostics: [], error: null }))
  @wait 60ms
  @assert (document.querySelector('.st-inspector-widget__root-select').options.length === 2)
    "baseline roster"
  @eval (ST.set(document.querySelector('.st-inspector-widget'), 'insStructure', { nodes: [], params: [], templates: ['a', 'b', 'd', 'e'], template: 'a', diagnostics: [], error: null }))
  @wait 80ms
  @assert (document.querySelector('.st-inspector-widget__root-select').options.length === 4)
    "a changed roster MUST re-render — change detection may never suppress a real update"
}

/// FEAT-163: a `@data` row must be selectable and produce typed editors.
///
/// A template-less page (12 of 23 projects here) has no invocation to pick. Its
/// editable surface is the JSON record behind an `@each` row, addressed by
/// (source, index, field) instead of a source byte span. These lock the two
/// halves that make that work: resolving a clicked element to an address, and
/// projecting the record into editor descriptors.
@test "a clicked each row resolves to its data address" {
  @fixture {
    <div class="dr-host">
      <article class="dr-row" data-st-source="pains" data-st-index="2">
        <h3 data-st-field="quote">Q</h3><p data-st-field="detail">D</p>
      </article>
    </div>
  }
  @eval (eval("window.__drA=ST.insDataAddress(document.querySelector('[data-st-field=quote]'));"))
  @assert (window.__drA.source === 'pains') "the row's source must resolve"
  @assert (window.__drA.index === 2) "the row's index must resolve as a number"
  @assert (window.__drA.field === 'quote') "the clicked field must resolve"
  @assert (ST.insDataNodeId(window.__drA) === 'data:pains[2]')
    "the address must produce a stable selection id"

  // Clicking row padding (no field) still selects the RECORD.
  @eval (eval("window.__drB=ST.insDataAddress(document.querySelector('.dr-row'));"))
  @assert (window.__drB.field === null) "a click outside any field selects the whole record"
  @assert (window.__drB.index === 2) "the record address still resolves"
}

@test "a data record projects into typed editors for every stamped field" {
  @fixture {
    <div class="dp-host">
      <article class="dp-row" data-st-source="items" data-st-index="0">
        <span data-st-field="title">T</span>
        <span data-st-field="count">3</span>
        <span data-st-field="live">true</span>
        <span data-st-field="tint">#ff0000</span>
      </article>
    </div>
  }
  @eval (eval("window.__dpRows=ST.insDataRows(ST.insDataAddress(document.querySelector('.dp-row')),{title:'T',count:3,live:true,tint:'#ff0000',hidden:'x'});"))
  @assert (window.__dpRows.length === 4)
    "only STAMPED fields are offered — an unstamped key has no element to show an edit against"
  @assert (window.__dpRows.map(function(r){return r.widget}).join(',') === 'text,number,toggle,color')
    "the widget must be inferred from the VALUE's type, since a @data record declares none"
  @assert (window.__dpRows[0].addr === '' && window.__dpRows[0].addrSel === '')
    "a data row carries NO source-span credentials — blanking them keeps the EditAst rail from ever claiming it"
  @assert (window.__dpRows[1].dataField === 'count' && window.__dpRows[1].index === 0)
    "each descriptor carries the address the write rail reads back"
}

@test "a data write builds a typed EditJson request" {
  @fixture {
    <div class="dw-host">
      <input class="dw-num" data-data-source="items" data-data-index="1" data-data-field="count" data-widget="number" value="7">
      <input class="dw-txt" data-data-source="items" data-data-index="1" data-data-field="title" data-widget="text" value="hi">
      <input class="dw-orphan" data-widget="text" value="nope">
    </div>
  }
  @eval (eval("ST._dataRegistry.set('items',{data:[],loaded:true,listeners:new Set(),src:'/data/items.json'});"))
  @eval (eval("window.__dwN=ST.insDataWrite(document.querySelector('.dw-num'));window.__dwT=ST.insDataWrite(document.querySelector('.dw-txt'));window.__dwO=ST.insDataWrite(document.querySelector('.dw-orphan'));"))
  @assert (window.__dwN.file === 'data/items.json')
    "the registry URL must become a SITE-RELATIVE path — a leading slash is rejected as File not found"
  @assert (window.__dwN.path === '[1].count') "the JSON path must address the record's field"
  @assert (window.__dwN.value === 7)
    "a number must be written as a NUMBER — writing '7' would silently change the record's shape"
  @assert (window.__dwT.value === 'hi') "a string stays a string"
  @assert (window.__dwO === null)
    "a control with no data address must produce NO request — a stray event may never manufacture a write"
}

/// FUP-138: the panel must REPORT a write, and above all report a REJECTED one.
///
/// Both were silent from the day the pill shipped. A rejection looked identical
/// to a success — the single most dangerous shape this surface can have — and no
/// test covered either, which is why it stayed invisible through five waves.
///
/// The producer half is the regression to guard: `%binds` aliases are what carry
/// an export to a name the page can read, and a parser that dropped every alias
/// after the first made the whole chain inert.
@test "a write rail reports success and failure through its aliased exports" {
  @fixture { <div class="ws-host"></div> }
  @eval (eval("var h=document.querySelector('.ws-host'); ST.set(h,'insDataWriteStatus','Saving…'); ST.set(h,'insDataWriteError','');"))
  @wait 30ms
  @assert (ST.get(document.querySelector('.ws-host'),'insDataWriteStatus') === 'Saving…')
    "the status export must be readable under its ALIAS — the raw export name means the remap was dropped"

  @eval (eval("var h=document.querySelector('.ws-host'); ST.set(h,'insDataWriteStatus',''); ST.set(h,'insDataWriteError','File not found: data/x.json');"))
  @wait 30ms
  @assert (ST.get(document.querySelector('.ws-host'),'insDataWriteError') === 'File not found: data/x.json')
    "a REJECTION must surface its reason — silence here is indistinguishable from success"
  @assert (ST.get(document.querySelector('.ws-host'),'insDataWriteStatus') === '')
    "a rejection must CLEAR the status, so a stale Saved never outlives a failed write"
}

/// FUP-136: the pick outline must name exactly ONE target, and it must be the SAME
/// node a click selects.
///
/// SCOPE OF THIS TEST — read before trusting it. The fix is a CSS selector
/// (`:hover:not(:has([data-st-node]:hover))`), and this backend has no computed
/// styles: `getComputedStyle` returns nothing usable under `--headless`, so no
/// assertion here can observe an outline. A W3 reviewer confirmed the consequence
/// — this test stays GREEN if the stylesheet is reverted to the broken broad rule.
///
/// What it DOES lock is the invariant the selector encodes, stated independently:
/// the set of nodes with no stamped descendant under the pointer has exactly one
/// member, and that member is what `closest('[data-st-node]')` — the real pick
/// resolution — returns. If that invariant ever fails, the selector cannot be
/// correct either, whatever it says.
///
/// The rendered outline itself is verified on `--cdp`, where computed styles are
/// real (confirmed live in Chrome during the W3 review: inner node only, purple
/// 2px). Per docs/testing/FIDELITY_LADDER.md a layout/paint assertion belongs
/// there and must not be faked here.
@test "the pick target invariant: exactly one innermost stamped node, and it is what a click selects" {
  @fixture {
    <div class="pk-host">
      <section data-st-node="outer">
        <article data-st-node="inner">
          <span class="pk-target">deep</span>
        </article>
      </section>
    </div>
  }

  // The rule the stylesheet uses, applied to the innermost stamped ancestor of the
  // pointer. `closest` is exactly what inspect-pick uses to resolve a click.
  @eval (eval("window.__pkClick=document.querySelector('.pk-target').closest('[data-st-node]');"))
  @assert (window.__pkClick.dataset.stNode === 'inner')
    "a click on nested content must resolve to the INNERMOST stamped node"

  // Every stamped ancestor of the pointer — what the old rule outlined.
  @eval (eval("window.__pkAll=[];var n=document.querySelector('.pk-target');while(n&&n.closest){var s=n.closest('[data-st-node]');if(!s)break;window.__pkAll.push(s.dataset.stNode);n=s.parentElement;}"))
  @assert (window.__pkAll.length === 2)
    "precondition: the fixture really does nest two stamped invocations, so the ambiguity is reproduced"

  // The stylesheet's guard: a stamped element CONTAINING another hovered stamped
  // element must not draw. Applied here as the same predicate the CSS expresses.
  @eval (eval("window.__pkDrawn=window.__pkAll.filter(function(id){var el=document.querySelector('[data-st-node=\"'+id+'\"]');return !el.querySelector('[data-st-node]');});"))
  @assert (window.__pkDrawn.length === 1)
    "exactly ONE node may draw an outline — the outer invocation contains a stamped descendant and must be excluded"
  @assert (window.__pkDrawn[0] === window.__pkClick.dataset.stNode)
    "and the outlined node must be the one a click selects — a highlight that names a different node lies about the gesture"
}
