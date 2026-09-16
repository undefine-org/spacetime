/// Headless test: MCP kit form renders prompt + one input per field (PLAN-043).
///
/// Pins the form kit's render contract: a plain selector-scoped
/// `@each($mcpFields as $field)` renders one labeled input per field descriptor,
/// each input carrying its `name`/`type` from the descriptor. Data is driven the
/// live-page way (mount → init → dispatch `local:mcpFields:updated`).

@import "stdlib/testing/test"

@data inline $mcpPrompt : "";
@data inline $mcpFields : [];

@template &kit-field($field) {
  <label class="kit-field">
    <span class="kit-field-label">`$field.label`</span>
    <input class="kit-field-input" name="`$field.name`" type="`$field.type`" placeholder="`$field.placeholder`" value="`$field.value`">
  </label>
}

@template &kit-form() {
  <main class="kit kit-form">
    <p class="kit-prompt"></p>
    <div class="kit-fields"></div>
  </main>

  .kit-prompt {
    text: $mcpPrompt;
  }

  .kit-fields {
    @each($mcpFields as $field) {
      &kit-field($field);
    }
  }
}

@test "form renders one labeled input per declared field" {
  @fixture {
    <div class="host-form"></div>
  }
  @eval (document.querySelector('.host-form').appendChild(Spacetime.invokeTemplate('kit-form')))
  @eval (ST.initElement(document.querySelector('.host-form .kit-fields')))
  @wait 60ms
  // Publish the array the live-page way: write through the SpacetimeLocal proxy,
  // which syncs _localState AND dispatches local:mcpFields:updated so the @each
  // re-renders. A bare document.dispatchEvent(CustomEvent) skips the _localState
  // sync, leaving the source data and the event out of step — the @each then
  // resolves an empty source and renders nothing.
  @eval (eval("window.SpacetimeLocal.mcpFields = [\
    { name: 'service', label: 'Service', type: 'text', placeholder: 'api', value: '' },\
    { name: 'replicas', label: 'Replicas', type: 'number', placeholder: '', value: '3' }\
  ]"))
  @wait 120ms
  @assert (document.querySelector('.host-form .kit-fields').children.length === 2)
    "two field descriptors must render two field rows"
  @assert (document.querySelector('.host-form .kit-fields').children[0].querySelector('input').getAttribute('name') === 'service')
    "first input must carry its field name 'service'"
  @assert (document.querySelector('.host-form .kit-fields').children[1].querySelector('input').getAttribute('type') === 'number')
    "second input must carry its declared type 'number'"
}
