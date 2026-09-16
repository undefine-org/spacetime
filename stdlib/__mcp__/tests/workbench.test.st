/// MCP workbench editor regression coverage (PLAN-112 W5).
///
/// The workbench lists render `&$p.tpl($p)` where `ST.fieldEditorRows` assigns
/// `tpl` through the shared `ST.adFieldTemplate` resolver. Mount the resolved
/// template exactly as that runtime path does; fixture HTML would not test it.

@import "stdlib/testing/test";
@import "stdlib/__mcp__/env.st";

@test "workbench color param row resolves and renders the shared color field" {
  @fixture { <div class="mcp-workbench-test-host"></div> }
  @eval (eval("window.__mcpColorRow=ST.fieldEditorRows([{node_id:'node-1',param:'accent',value:'#FF0020',type:'color',widget:'color',source_hash:'snapshot',span_start:12,span_end:30,file:'page.st',span_text:'&card(accent: \"#FF0020\")'}])[0];document.querySelector('.mcp-workbench-test-host').appendChild(Spacetime.invokeTemplate(window.__mcpColorRow.tpl,window.__mcpColorRow));"))
  @wait 50ms
  @assert (window.__mcpColorRow.tpl === ST.adFieldTemplate('color'))
    "the workbench descriptor must use the shared widget resolver"
  @assert (document.querySelector('.mcp-workbench-test-host .st-fld-color-text') !== null)
    "a color parameter must render the shared hex/color control"
  @assert (document.querySelector('.mcp-workbench-test-host .mcp-node-param-input') === null)
    "a color parameter must not regress to the legacy plain workbench input"
  @assert (document.querySelector('.mcp-workbench-test-host .st-fld-color-text').dataset.param === 'accent')
    "the shared control must preserve the workbench data-param addressing rail"
  @assert (document.querySelector('.mcp-workbench-test-host .st-fld-color-text').dataset.stPayloadSpanStart === '12' && document.querySelector('.mcp-workbench-test-host .st-fld-color-text').dataset.stPayloadSpanEnd === '30')
    "the shared control must preserve the span payload addressing rail"
}
