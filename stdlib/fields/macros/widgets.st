/// Shared typed field templates. Each accepts one descriptor `$f`.
///
/// Address contract: `$f.path` maps to `data-path`; source-span consumers pass
/// `$f.addr`, `$f.addrSel`, `$f.addrKind`, `$f.addrFile`, `$f.addrStart`, and
/// `$f.addrEnd`, which map directly to their `data-addr-*` counterparts.
/// Workbench params pass `$f.param`, `$f.paramKind`, `$f.spanStart`, and
/// `$f.spanEnd`, mapping to `data-param`, `data-param-kind`, and
/// `data-st-payload-span-start/end` respectively. Every actionable control keeps
/// `data-widget` for delegated consumer rails.

@template &field-text($f) {
  <div class="st-fld-field" data-path="`$f.path`">
    <label class="st-fld-label">`$f.label`</label>
    <input class="st-fld-input" value="`$f.value`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="text" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`">
  </div>
}

@template &field-number($f) {
  <div class="st-fld-field" data-path="`$f.path`">
    <label class="st-fld-label">`$f.label`</label>
    <input class="st-fld-input" type="number" value="`$f.value`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="number" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`">
  </div>
}

@template &field-toggle($f) {
  <div class="st-fld-field st-fld-field--toggle" data-path="`$f.path`" data-checked="`$f.value`">
    <label class="st-fld-label">`$f.label`</label>
    <label class="st-fld-toggle"><input type="checkbox" checked="`$f.checked`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="toggle" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`"><span class="st-fld-toggle-track"></span></label>
  </div>
}

@template &field-color($f) {
  <div class="st-fld-field st-fld-field--color" data-path="`$f.path`">
    <label class="st-fld-label">`$f.label`</label>
    <div class="st-fld-color-control">
      <input type="color" class="st-fld-color-swatch" value="`$f.value`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="color" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`">
      <input class="st-fld-input st-fld-color-text" value="`$f.value`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="color" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`">
      <label class="st-fld-picker-toggle"><input type="checkbox" class="st-fld-picker-open"><span class="st-fld-picker-caret"></span></label>
    </div>
    <div class="st-fld-picker-panel">
      <input type="range" class="st-fld-picker st-fld-picker-h" min="0" max="360" step="1" value="0" aria-label="Hue">
      <input type="range" class="st-fld-picker st-fld-picker-s" min="0" max="100" step="1" value="0" aria-label="Saturation">
      <input type="range" class="st-fld-picker st-fld-picker-v" min="0" max="100" step="1" value="0" aria-label="Value">
      <input type="range" class="st-fld-picker st-fld-picker-a" min="0" max="1" step="0.01" value="1" aria-label="Alpha">
    </div>
  </div>
  .st-fld-field--color { @field-color-control; }
}

@template &field-range-length($f) {
  <div class="st-fld-field st-fld-field--range" data-path="`$f.path`">
    <label class="st-fld-label">`$f.label` <span class="st-fld-range-value">`$f.value`</span></label>
    <input type="range" class="st-fld-range" min="`$f.rmin`" max="`$f.rmax`" step="`$f.rstep`" value="`$f.rnum`" data-unit="`$f.runit`">
    <input type="hidden" data-range-twin value="`$f.value`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="length" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`">
  </div>
  .st-fld-field--range { @field-range-control; }
}

@template &field-range-duration($f) {
  <div class="st-fld-field st-fld-field--range" data-path="`$f.path`">
    <label class="st-fld-label">`$f.label` <span class="st-fld-range-value">`$f.value`</span></label>
    <input type="range" class="st-fld-range" min="`$f.rmin`" max="`$f.rmax`" step="`$f.rstep`" value="`$f.rnum`" data-unit="`$f.runit`">
    <input type="hidden" data-range-twin value="`$f.value`" data-path="`$f.path`" data-param="`$f.param`" data-param-kind="`$f.paramKind`" data-st-payload-span-start="`$f.spanStart`" data-st-payload-span-end="`$f.spanEnd`" data-widget="duration" data-addr="`$f.addr`" data-addr-sel="`$f.addrSel`" data-data-source="`$f.source`" data-data-index="`$f.index`" data-data-field="`$f.dataField`" data-addr-kind="`$f.addrKind`" data-addr-file="`$f.addrFile`" data-addr-start="`$f.addrStart`" data-addr-end="`$f.addrEnd`" data-motion-sel="`$f.motionSel`" data-motion-kind="`$f.motionKind`" data-motion-file="`$f.file`">
  </div>
  .st-fld-field--range { @field-range-control; }
}
