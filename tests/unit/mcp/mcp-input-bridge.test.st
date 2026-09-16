/// Headless test: BUG-156 regression — @mcp-input's own signal must self-bridge.
///
/// `%yield expr -> $var`'s `ST.set(el, name, value)` call only dispatches the
/// body-scope `local:<name>:updated` event (which `@data derive`/`@each`'s
/// dependency-watch listen for) when `name` is in `ST._bridgedExports` — an
/// allowlist the COMPILER populates per-page, but ONLY for names some markup
/// directly interpolates (src/emit/html_hydrate.rs's reactive-binding codegen).
/// `@mcp-input` publishes `$mcpInput`/`$mcpInputLoading`/`$mcpInputError` via
/// exactly that `%yield` path — so a page that ONLY derives/`@each`s off
/// `$mcpInput` (never interpolates it directly anywhere) used to see the derive
/// silently never fire, contingent on unrelated sibling markup existing
/// elsewhere on the page. Fixed by having `mcp-input`'s own `%emit js` block
/// unconditionally `ST._bridgeExport` its three exports BEFORE the first yield —
/// see BUG-156-a-data-derive-off-an-mcp-input-signal-si.org.
///
/// This test mirrors the kit-picker convention (mount → init → publish via the
/// SpacetimeLocal proxy write) but pins the SPECIFIC regression: NO markup on
/// the page directly interpolates `$mcpInput` — the ONLY consumer is a nested
/// `@each` over a dotted path into it. This is the actual FEAT-151 Forge Deck
/// shape (`@each($mcpInput.steps as $step)`) that surfaced the bug.

@import "stdlib/testing/test"
@import "stdlib/__mcp__"

@template &mib-row($step) {
  <span class="mib-row">`$step.label`</span>
}

// The ONLY consumer of $mcpInput anywhere in this template is the dotted-path
// @each below — no sibling interpolation exists to accidentally bridge it.
@template &mib-rail() {
  <div class="mib-rail"></div>

  .mib-rail {
    @each($mcpInput.steps as $step) {
      &mib-row($step);
    }
  }
}

@test "@each over $mcpInput.field renders with no other consumer on the page" {
  @fixture {
    <div class="host-mib" data-st-instance></div>
  }
  @eval (document.querySelector('.host-mib').appendChild(Spacetime.invokeTemplate('mib-rail')))
  @eval (ST.initElement(document.querySelector('.host-mib .mib-rail')))
  @wait 60ms
  // Live-page-faithful publish: mcp-input's primitive republishes via
  // ST.set(body, 'mcpInput', value) on every input.json poll — mirror that
  // exact call (not a bare SpacetimeLocal assignment) so this test exercises
  // the SAME code path @mcp-input itself uses, catching a regression in either
  // the primitive's self-bridge OR ST.set's bridging contract.
  @eval (ST.set(document.body, 'mcpInput', { steps: [
    { id: 'blank-disc', label: 'Blank Disc' },
    { id: 'cut-facet', label: 'Cut Facet' }
  ] }))
  @wait 120ms
  @assert (document.querySelector('.host-mib .mib-rail').children.length === 2)
    "the dotted-path @each must render 2 rows from $mcpInput.steps with zero other consumers on the page"
}

@test "@each over $mcpInput.field re-renders when input.json is re-polled" {
  @fixture {
    <div class="host-mib2" data-st-instance></div>
  }
  @eval (document.querySelector('.host-mib2').appendChild(Spacetime.invokeTemplate('mib-rail')))
  @eval (ST.initElement(document.querySelector('.host-mib2 .mib-rail')))
  @wait 60ms
  @eval (ST.set(document.body, 'mcpInput', { steps: [
    { id: 'a', label: 'A' }, { id: 'b', label: 'B' }, { id: 'c', label: 'C' }
  ] }))
  @wait 90ms
  @eval (window.__mibAfterFirst = document.querySelector('.host-mib2 .mib-rail').children.length)
  @eval (ST.set(document.body, 'mcpInput', { steps: [
    { id: 'x', label: 'X' }
  ] }))
  @wait 90ms
  @assert (window.__mibAfterFirst === 3)
    "first poll renders 3 rows"
  @assert (document.querySelector('.host-mib2 .mib-rail').children.length === 1)
    "a re-poll with a 1-step payload must re-render down to 1 row"
}
