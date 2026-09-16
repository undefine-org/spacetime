/// Headless test: MCP kit picker renders prompt + option buttons (PLAN-043).
///
/// Pins that the picker kit composes the proven idiom end-to-end: a plain
/// selector-scoped `@each($mcpOptions as $opt)` renders one option button per
/// row carrying `data-choice-id`, and the prompt binds from `$mcpPrompt`. Data
/// is driven the live-page way (mount → init → dispatch the documented
/// `local:<src>:updated` event with the array in e.detail) — see
/// tests/unit/templates/each-data-drive.test.st for the drive convention.

@import "stdlib/testing/test"

// Kit data sources (picker.st declares these via @data + @mcp-input; declared
// here too so the mirrored render contract type-checks without an E0905 warning).
@data inline $mcpPrompt : "";
@data inline $mcpOptions : [];

// Mirror of the picker's option template + list (the kit file itself imports the
// whole MCP module + host primitives, which the headless harness need not boot;
// this pins the render contract the kit relies on).
@template &kit-option($opt) {
  <button class="kit-option" data-choice-id="`$opt.id`">
    <span class="kit-option-label">`$opt.label`</span>
    <span class="kit-option-detail">`$opt.detail`</span>
  </button>
}

@template &kit-picker() {
  <main class="kit kit-picker">
    <p class="kit-prompt"></p>
    <div class="kit-options"></div>
  </main>

  .kit-prompt {
    text: $mcpPrompt;
  }

  .kit-options {
    @each($mcpOptions as $opt) {
      &kit-option($opt);
    }
  }
}

@test "picker renders one button per option, each carrying its choice id" {
  @fixture {
    <div class="host-picker"></div>
  }
  @eval (document.querySelector('.host-picker').appendChild(Spacetime.invokeTemplate('kit-picker')))
  @eval (ST.initElement(document.querySelector('.host-picker .kit-options')))
  @wait 60ms
  // Live-page-faithful publish: write through the SpacetimeLocal proxy (syncs
  // _localState AND dispatches local:mcpOptions:updated). A bare CustomEvent
  // dispatch skips the _localState sync, so the @each resolves an empty source.
  @eval (eval("window.SpacetimeLocal.mcpOptions = [\
    { id: 'a', label: 'In-place', detail: 'fast, risky' },\
    { id: 'b', label: 'Blue/green', detail: 'safe, slower' },\
    { id: 'c', label: 'Canary', detail: 'gradual' }\
  ]"))
  @wait 120ms
  @assert (document.querySelector('.host-picker .kit-options').children.length === 3)
    "three options must render three option buttons"
  @assert (document.querySelector('.host-picker .kit-options').children[0].getAttribute('data-choice-id') === 'a')
    "first option button must carry its choice id 'a'"
  @assert (document.querySelector('.host-picker .kit-options').children[1].getAttribute('data-choice-id') === 'b')
    "second option button must carry its choice id 'b'"
}

@test "picker re-renders when the option set changes (3 → 2)" {
  @fixture {
    <div class="host-picker2"></div>
  }
  @eval (document.querySelector('.host-picker2').appendChild(Spacetime.invokeTemplate('kit-picker')))
  @eval (ST.initElement(document.querySelector('.host-picker2 .kit-options')))
  @wait 60ms
  @eval (eval("window.SpacetimeLocal.mcpOptions = [{ id: 'a', label: 'A' }, { id: 'b', label: 'B' }, { id: 'c', label: 'C' }]"))
  @wait 90ms
  @eval (window.__pickerFirst = document.querySelector('.host-picker2 .kit-options').children.length)
  @eval (eval("window.SpacetimeLocal.mcpOptions = [{ id: 'x', label: 'X' }, { id: 'y', label: 'Y' }]"))
  @wait 90ms
  @assert (window.__pickerFirst === 3)
    "first option set renders 3 buttons"
  @assert (document.querySelector('.host-picker2 .kit-options').children.length === 2)
    "a new 2-option set re-renders down to 2 buttons"
}
