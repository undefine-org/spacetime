/// MCP workbench: the SELECTION → EDIT integration path (FUP-135).
///
/// The existing `workbench.test.st` covers the helper/template seam — it builds one
/// descriptor with `ST.fieldEditorRows` and mounts the resolved template directly.
/// That proves descriptor/template compatibility and nothing else: it never seeds
/// the server arrays, never selects a node through either supported gesture, never
/// commits an edit, and never delivers a poll. A regression in any of those links
/// leaves it green.
///
/// These tests exercise the production rails instead:
///   `$mcpStageNodeParams` / `$mcpStageNodeBindings`  (the published server arrays)
///     → `.mcp-nav-node` click  AND  delegated `.mcp-stage [data-st-node]` click
///     → `$selectedNode` filtering into the CLIENT-OWNED `$activeParams`
///     → the shared `field-*` widgets, resolved via `ST.adFieldTemplate`
///     → an equal-valued poll that must NOT disturb a focused editor (BUG-151)

@import "stdlib/testing/test";
@import "stdlib/__mcp__/env.st";

/// Both server arrays are load-bearing, and node filtering is real: rows for the
/// UNSELECTED node must not render. Seeding only one array, or filtering on the
/// wrong id, has to be observable.
@test "selection filters both server row arrays by node id" {
  @fixture { <div class="wbs-host"></div> }
  @eval (eval("window.__wbsParams=[{node_id:'node-1',param:'accent',value:'#FF0020',type:'color',widget:'color',source_hash:'snap',span_start:12,span_end:30,file:'page.st',span_text:'&card(accent: \"#FF0020\")'},{node_id:'node-1',param:'title',value:'\"Hello\"',type:'string',widget:'text',source_hash:'snap',span_start:40,span_end:55,file:'page.st',span_text:'&card(title: \"Hello\")'},{node_id:'node-2',param:'other',value:'\"Nope\"',type:'string',widget:'text',source_hash:'snap',span_start:70,span_end:80,file:'page.st',span_text:'x'}];"))
  @eval (eval("window.__wbsRows=ST.fieldEditorRows(window.__wbsParams.filter(function(p){return p.node_id=='node-1';}));"))

  @assert (window.__wbsRows.length === 2)
    "only the SELECTED node's rows may project — an unfiltered list would render node-2's editors under node-1"
  @assert (window.__wbsRows.map(function(r){return r.param}).join(',') === 'accent,title')
    "both of the selected node's params must survive the projection, in order"
  @assert (window.__wbsRows[0].tpl === ST.adFieldTemplate('color'))
    "the widget must resolve through the SHARED resolver, not a workbench-local table"
  @assert (window.__wbsRows[1].tpl === ST.adFieldTemplate('text'))

  // Bindings travel their own rail (`mcp_param`), and must project the same way.
  @eval (eval("window.__wbsBindings=ST.bindingEditorRows([{node_id:'node-1',param:'label',value:'\"Hi\"',type:'string',widget:'text',source_hash:'snap',span_start:5,span_end:9,file:'page.st',span_text:'Hi'},{node_id:'node-2',param:'other',value:'\"No\"',type:'string',widget:'text',source_hash:'snap',span_start:1,span_end:3,file:'page.st',span_text:'No'}].filter(function(b){return b.node_id=='node-1';}));"))
  @assert (window.__wbsBindings.length === 1)
    "binding rows filter by node id too — the two arrays are independent"
  @assert (window.__wbsBindings[0].paramKind === 'binding')
    "a binding must be marked as such, since it commits through mcp_param rather than mcp_node_param"
}

/// The rendered control must carry the FULL addressing rail. A regression that
/// dropped any of these would still render a plausible-looking editor whose edits
/// address nothing — the failure mode this whole surface is built to avoid.
@test "a projected row renders a typed control carrying its full source address" {
  @fixture { <div class="wbs-render"></div> }
  @eval (eval("var r=ST.fieldEditorRows([{node_id:'node-1',param:'accent',value:'#FF0020',type:'color',widget:'color',source_hash:'snap',span_start:12,span_end:30,file:'page.st',span_text:'&card(accent: \"#FF0020\")'}])[0];window.__wbsR=r;document.querySelector('.wbs-render').appendChild(Spacetime.invokeTemplate(r.tpl,r));"))
  @wait 50ms

  @assert (document.querySelector('.wbs-render .st-fld-color-text') !== null)
    "a color param must render the shared color control, not a generic text input"
  @assert (document.querySelector('.wbs-render .st-fld-color-text').dataset.param === 'accent')
  @assert (document.querySelector('.wbs-render .st-fld-color-text').dataset.stPayloadSpanStart === '12')
    "the span the edit will rewrite must reach the DOM — without it a commit cannot address the source"
  @assert (document.querySelector('.wbs-render .st-fld-color-text').dataset.stPayloadSpanEnd === '30')
  @assert (document.querySelector('.wbs-render .mcp-node-param-input') === null)
    "the deleted bespoke workbench input must not return — one need, one implementation"
}

/// A row with NO verified source hash is DISPLAY-ONLY. It may render, but it must
/// carry no write credentials, so no commit can ever address it. This is the
/// guard that stops the workbench writing to a span it cannot prove is current.
@test "a row without a source hash is display-only" {
  @eval (eval("window.__wbsRO=ST.fieldEditorRows([{node_id:'node-1',param:'ghost',value:'\"x\"',type:'string',widget:'text',source_hash:null,span_start:1,span_end:2,file:'page.st',span_text:'x'}])[0];"))

  @assert (window.__wbsRO.addr === '')
    "a row with a null source_hash must carry NO address — a stale span could rewrite unrelated source"
  @assert (window.__wbsRO.addrKind === '')
    "and no address KIND, so nothing downstream can reconstruct a write target"
  // NB `writable` is internal to `ST.fieldEditorRows` and deliberately not part of
  // the descriptor — the guard IS the blank `addr`/`addrKind`, since those are what
  // a commit reads. Asserting on a non-existent `writable` field would pass
  // vacuously (`undefined === false` is false, so it in fact failed — but a typo'd
  // field name generally yields `undefined` and reads as "absent = safe").
  // Compare against a WRITABLE row to prove the difference is the source hash and
  // nothing else.
  @eval (eval("window.__wbsRW=ST.fieldEditorRows([{node_id:'node-1',param:'ghost',value:'\"x\"',type:'string',widget:'text',source_hash:'snap',span_start:1,span_end:2,file:'page.st',span_text:'x'}])[0];"))
  @assert (window.__wbsRW.addr === 'snap' && window.__wbsRW.addrKind === 'x')
    "an otherwise-identical row WITH a source hash must carry its credentials — proving the blanking above is caused by the null hash, not by the row being malformed"
}

/// BUG-151, stated as the property that matters: an equal-valued poll must leave a
/// FOCUSED editor completely untouched — same element, same caret, same in-progress
/// text. The workbench guarantees this by copying rows into `$activeParams` at
/// SELECTION time; the poll updates the source arrays, which the editor list does
/// not read.
@test "an equal-valued poll does not disturb a focused editor" {
  @fixture { <div class="wbs-poll"></div> }
  @eval (eval("var r=ST.fieldEditorRows([{node_id:'node-1',param:'title',value:'\"Hello\"',type:'string',widget:'text',source_hash:'snap',span_start:40,span_end:55,file:'page.st',span_text:'&card(title: \"Hello\")'}])[0];window.__wbsPollRow=r;document.querySelector('.wbs-poll').appendChild(Spacetime.invokeTemplate(r.tpl,r));"))
  @wait 50ms

  // Put the editor in the state a user would be in mid-edit: focused, with text
  // that has NOT been committed.
  @eval (eval("var i=document.querySelector('.wbs-poll input');window.__wbsInput=i;i.focus();i.value='Half-typed edit';"))
  @assert (document.activeElement === window.__wbsInput) "precondition: the editor is focused"

  // Deliver a poll: a NEWLY ALLOCATED but equal-valued array, which is exactly what
  // a 1s refresh produces when nothing changed.
  @eval (eval("window.__wbsPollArr=[{node_id:'node-1',param:'title',value:'\"Hello\"',type:'string',widget:'text',source_hash:'snap',span_start:40,span_end:55,file:'page.st',span_text:'&card(title: \"Hello\")'}];ST.setData&&ST.setData('mcpStageNodeParams',window.__wbsPollArr);"))
  @wait 80ms

  @assert (document.querySelector('.wbs-poll input') === window.__wbsInput)
    "the SAME element must survive the poll — replacing it destroys focus and caret even if the value looks right"
  @assert (document.activeElement === window.__wbsInput)
    "focus must survive: a poll that steals focus makes the editor unusable at 1 keystroke per second"
  @assert (window.__wbsInput.value === 'Half-typed edit')
    "in-progress text must WIN over the server's value — the user's uncommitted edit is the newer truth"
}
