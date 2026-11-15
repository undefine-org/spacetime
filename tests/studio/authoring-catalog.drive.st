/// Serve a disposable copy of demos/authoring-catalog on port 4560.
@import "stdlib/testing/test";
@import "stdlib/testing/automation";
@drive "http://127.0.0.1:4560/" {
  @wait_until (document.querySelector('.catalog-properties .w-field')) timeout: 20s
  @assert (document.querySelectorAll('.catalog-choice').length >= 3) "the catalog discovers its actual definitions"
  @assert (document.querySelector('.catalog-properties .w-field').dataset.file === 'fixtures.st') "the shared control targets the imported owning module"
  @assert (document.querySelector('.catalog-properties .w-swatch').value === '#eaf0f5') "a typed markup parameter produces a color control"
  @assert (Number(document.querySelector('.catalog-properties .w-field').dataset.end) > Number(document.querySelector('.catalog-properties .w-field').dataset.start)) "the default has a real editable source span"
  @eval (document.querySelector('[data-name="--light-surface"]').click())
  @wait 150ms
  @assert (document.querySelector('.catalog-properties .w-number').value === '1.8') "a scalar style parameter uses the same property editor with a number control"
  @eval (document.querySelector('[data-name="--arrival"]').click())
  @wait 150ms
  @assert (document.querySelector('.catalog-properties .w-text').value === '80px') "a motion parameter preserves its dimension"
  @assert (document.querySelectorAll('.catalog-properties .au-property').length === 2) "both motion parameters come from the signature"
  @assert (!!document.querySelector('.catalog-tool-cards [data-definition="&property-editor"]')) "the catalog discovers the shared component rendering its inspector"
  @assert (!!document.querySelector('.catalog-macro-cards [data-definition="authoring-edit"]')) "behavior macros belong to the same catalog"
}
