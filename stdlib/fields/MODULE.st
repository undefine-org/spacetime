/// Spacetime `fields` module — reusable typed field widgets.
///
/// `@import "stdlib/fields"` provides the pure, consumer-neutral field
/// vocabulary used by editors. Consumers own addressing and persistence rails;
/// descriptors supply their `data-*` address values to the rendered controls.
///
/// Public templates: `&field-text`, `&field-number`, `&field-toggle`,
/// `&field-color`, `&field-range-length`, `&field-range-duration`.
/// Compact authoring controls also live here: `&w-text`, `&w-number`,
/// `&w-range`, `&w-color`, `&w-toggle`, `&w-choice`, `&w-vec3`, and `@scrub`.
/// The old `stdlib/widgets` import forwards to this module. Existing CMS
/// addressing stays compatible while authoring consumers share one projection.

@import "./index.st"
