/// Fields Module Tests — the SHARED typed widgets (PLAN-112 W2).
///
/// These tests instantiate the REAL `&field-*` templates through the REAL mount
/// path (`Spacetime.invokeTemplate`, the same one `@each` uses) and drive the
/// REAL `%primitive` control adapters. That distinction is the whole point: a
/// test that mounts hand-written `<input>`s and attaches its own listeners would
/// pass with this module deleted, and would say nothing about the widgets three
/// surfaces (admin CMS, MCP workbench, Inspector pill) depend on.
///
/// Covered per widget: it renders its control, it carries the addressing
/// attributes a consumer's persist rail keys on, and — for the composite
/// widgets — its control logic actually recomposes a value.

@import "stdlib/testing/test"
@import "stdlib/fields"

// =============================================================================
// Rendering + the address contract
//
// A widget is useless to a consumer unless the address rides through to the
// control: admin keys its dev-ws persist on `data-path`; the Inspector pill
// keys its source-span write on `data-addr-*`. Both must survive rendering.
// =============================================================================

@test "text widget renders its input and carries both address rails" {
  @fixture { <div class="host-text"></div> }
  @eval (document.querySelector('.host-text').appendChild(Spacetime.invokeTemplate('field-text', { label: 'Label', value: 'hello', path: '.label', addrFile: 'index.st', addrStart: '10', addrEnd: '42' })))
  @wait 50ms
  @let (el = document.querySelector('.host-text .st-fld-input'))
  @assert (el !== null) "the text widget must render an input"
  @assert (el.value === 'hello') "the input must carry the descriptor value"
  @assert (el.dataset.widget === 'text') "data-widget drives a consumer's coercion"
  @assert (el.dataset.path === '.label') "the admin JSON address must ride through"
  @assert (el.dataset.addrFile === 'index.st' && el.dataset.addrStart === '10' && el.dataset.addrEnd === '42')
    "the source-span address must ride through for the Inspector pill"
  @assert (document.querySelector('.host-text .st-fld-label').textContent.trim() === 'Label')
    "the label must render"
}

@test "number widget renders a numeric input" {
  @fixture { <div class="host-num"></div> }
  @eval (document.querySelector('.host-num').appendChild(Spacetime.invokeTemplate('field-number', { label: 'Count', value: '3', path: '.count' })))
  @wait 50ms
  @let (el = document.querySelector('.host-num .st-fld-input'))
  @assert (el !== null && el.type === 'number') "the number widget must render a number input"
  @assert (el.value === '3') "the input must carry the descriptor value"
  @assert (el.dataset.widget === 'number') "data-widget must be number"
}

@test "toggle widget renders a checkbox" {
  @fixture { <div class="host-tog"></div> }
  @eval (document.querySelector('.host-tog').appendChild(Spacetime.invokeTemplate('field-toggle', { label: 'Enabled', path: '.enabled' })))
  @wait 50ms
  @let (el = document.querySelector('.host-tog .st-fld-toggle input'))
  @assert (el !== null && el.type === 'checkbox') "the toggle widget must render a checkbox"
  @assert (el.dataset.widget === 'toggle') "data-widget must be toggle"
}

@test "color widget renders swatch, hex text, and every HSV channel" {
  @fixture { <div class="host-col"></div> }
  @eval (document.querySelector('.host-col').appendChild(Spacetime.invokeTemplate('field-color', { label: 'Tint', value: '#FF0020', path: '.tint' })))
  @wait 50ms
  @assert (document.querySelector('.host-col .st-fld-color-swatch') !== null) "a native swatch must render"
  @assert (document.querySelector('.host-col .st-fld-color-text').value === '#FF0020') "the hex text must carry the value"
  @assert (document.querySelector('.host-col .st-fld-picker-h') !== null
        && document.querySelector('.host-col .st-fld-picker-s') !== null
        && document.querySelector('.host-col .st-fld-picker-v') !== null
        && document.querySelector('.host-col .st-fld-picker-a') !== null)
    "all four perceptual channels must render, or the picker cannot express a color"
}

@test "range widget renders a slider over a hidden write twin" {
  @fixture { <div class="host-rng"></div> }
  @eval (document.querySelector('.host-rng').appendChild(Spacetime.invokeTemplate('field-range-length', { label: 'Gap', value: '8px', rnum: '8', runit: 'px', rmin: '0', rmax: '64', rstep: '1', path: '.gap' })))
  @wait 50ms
  @let (slider = document.querySelector('.host-rng .st-fld-range'))
  @assert (slider !== null && slider.value === '8') "the slider must render tuned to the number"
  @assert (slider.dataset.unit === 'px') "the unit must be preserved on the slider"
  @let (twin = document.querySelector('.host-rng [data-range-twin]'))
  @assert (twin !== null && twin.value === '8px')
    "the twin is the SINGLE write channel and must carry the composed value"
  @assert (twin.dataset.widget === 'length') "the twin must carry the widget kind"
}

// =============================================================================
// Control behavior (the primitives, driven through the real templates)
// =============================================================================

@test "moving the range slider composes number and unit onto the twin" {
  @fixture { <div class="host-rng2"></div> }
  @eval (document.querySelector('.host-rng2').appendChild(Spacetime.invokeTemplate('field-range-length', { label: 'Gap', value: '8px', rnum: '8', runit: 'px', rmin: '0', rmax: '64', rstep: '1', path: '.gap' })))
  @wait 200ms
  @eval (eval("(function(){var s=document.querySelector('.host-rng2 .st-fld-range');s.value='12';s.dispatchEvent(new Event('input',{bubbles:true}));})()"))
  @wait 50ms
  // The unit is reattached — the reason a slider can edit `8px` without
  // destroying its dimension.
  @assert (document.querySelector('.host-rng2 [data-range-twin]').value === '12px')
    "the twin must carry number+unit after a slider move"
  @assert (document.querySelector('.host-rng2 .st-fld-range-value').textContent.trim() === '12px')
    "the readout must mirror the composed value"
}

@test "a range edit bubbles to a consumer's delegated listener" {
  @fixture { <div class="host-rng3"></div> }
  @eval (document.querySelector('.host-rng3').appendChild(Spacetime.invokeTemplate('field-range-length', { label: 'Gap', value: '8px', rnum: '8', runit: 'px', rmin: '0', rmax: '64', rstep: '1', path: '.gap' })))
  @wait 200ms
  // A consumer persists via ONE delegated listener; prove the twin's event
  // bubbles to it, since that is what makes the shared widget rail-agnostic.
  @eval (eval("window.__fieldSeen=null;document.addEventListener('input',function(e){if(e.target&&e.target.hasAttribute&&e.target.hasAttribute('data-range-twin'))window.__fieldSeen=e.target.value;});"))
  @eval (eval("(function(){var s=document.querySelector('.host-rng3 .st-fld-range');s.value='24';s.dispatchEvent(new Event('input',{bubbles:true}));})()"))
  @wait 50ms
  @assert (window.__fieldSeen === '24px')
    "a range edit must reach a delegated consumer listener as a composed value"
}

@test "dragging the hue slider recomposes the hex through the text input" {
  @fixture { <div class="host-hue"></div> }
  @eval (document.querySelector('.host-hue').appendChild(Spacetime.invokeTemplate('field-color', { label: 'Tint', value: '#FF0000', path: '.tint' })))
  @wait 50ms
  // Pure red is hue 0; moving to 120 must yield pure green. This exercises the
  // real HSV→hex conversion inside the color control primitive.
  @eval (eval("(function(){var h=document.querySelector('.host-hue .st-fld-picker-h');h.value='120';h.dispatchEvent(new Event('input',{bubbles:true}));})()"))
  @wait 50ms
  @assert (document.querySelector('.host-hue .st-fld-color-text').value === '#00FF00')
    "hue 120 on pure red must recompose to pure green"
}

@test "an untouched channel does not drift when another slider moves" {
  @fixture { <div class="host-drift"></div> }
  @eval (document.querySelector('.host-drift').appendChild(Spacetime.invokeTemplate('field-color', { label: 'Tint', value: '#FF0020', path: '.tint' })))
  @wait 50ms
  // The picker caches full-precision HSV because the range inputs snap to their
  // integer step: reading channels back from the DOM would round-trip through
  // that snap and drift the RGB the user never touched (#FF0020 → #FF0022).
  @eval (eval("(function(){var a=document.querySelector('.host-drift .st-fld-picker-a');a.value='0.5';a.dispatchEvent(new Event('input',{bubbles:true}));})()"))
  @wait 50ms
  @assert (document.querySelector('.host-drift .st-fld-color-text').value.slice(0,7) === '#FF0020')
    "moving alpha must not drift the untouched RGB channels"
}

/// W5R REVIEW FINDING: `checked` is DOM STATE, not an attribute a hole renders.
/// A shared toggle mounted with `value: true` still appeared OFF, so the user's
/// first click turned it ON — inverting the meaning of every boolean it edits.
/// (The admin never hit this: its own `adHydrateControls` fills `.ad-form`
/// checkboxes. Nothing does that for the pill or the workbench.)
@test "a toggle carries its state so it cannot render inverted" {
  @fixture { <div class="tgstate-on"></div><div class="tgstate-off"></div> }
  // Assert the PROJECTION and the rendered MARKUP, not `input.checked`: the
  // headless world does not reliably reflect a `checked` attribute into the
  // property, and a test that reads the property passes even with the projection
  // sabotaged (verified). Both assertions below fail if `checked` is dropped.
  @eval (eval("window.__tgRows=ST.fieldEditorRows([{param:'on',value:'true',widget:'toggle',type:'bool',source_hash:'h',span_text:'x'},{param:'off',value:'false',widget:'toggle',type:'bool',source_hash:'h',span_text:'x'}]);"))
  @assert (window.__tgRows[0].checked === 'checked')
    "a true value must project as a checked attribute, or the box renders OFF and the first click INVERTS the value"
  @assert (window.__tgRows[1].checked === false)
    "a false value must project no checked attribute"

  @eval (document.querySelector('.tgstate-on').appendChild(Spacetime.invokeTemplate('field-toggle', window.__tgRows[0])))
  @eval (document.querySelector('.tgstate-off').appendChild(Spacetime.invokeTemplate('field-toggle', window.__tgRows[1])))
  @wait 100ms
  @eval (eval("var on=document.querySelector('.tgstate-on').querySelector('input[type=checkbox]');var off=document.querySelector('.tgstate-off').querySelector('input[type=checkbox]');window.__tgOnHas=on?on.hasAttribute('checked'):'NO-ON';window.__tgOffHas=off?off.hasAttribute('checked'):'NO-OFF';"))
  @assert (window.__tgOnHas === true)
    "the rendered ON toggle must carry the checked attribute"
  @assert (window.__tgOffHas === false)
    "the rendered OFF toggle must not carry it"
}

/// W5R REVIEW FINDING: range bounds were a single hardcoded 0-200 for both
/// kinds. The native slider CLAMPS to its max, so a `600ms` duration snapped to
/// 200ms the moment the user touched it — the control destroyed the value it
/// was editing. Bounds are per-kind, matching the admin's own derivation.
@test "range bounds follow the widget kind so real values stay reachable" {
  @eval (eval("window.__rowsD=ST.fieldEditorRows([{param:'fade',value:'600ms',widget:'range-duration',type:'duration',source_hash:'h',span_text:'x'}]);window.__rowsL=ST.fieldEditorRows([{param:'gap',value:'8px',widget:'range-length',type:'length',source_hash:'h',span_text:'x'}]);"))
  @assert (window.__rowsD[0].rmax >= 600)
    "a duration slider must reach at least 600ms, or editing 600ms destroys it"
  @assert (window.__rowsD[0].rnum === 600 && window.__rowsD[0].runit === 'ms')
    "the duration's number and unit must survive the projection"
  @assert (window.__rowsL[0].rmax === 64 && window.__rowsL[0].rstep === 1)
    "a length slider keeps the established 0-64 bounds"
}
