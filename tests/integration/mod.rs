//! Integration tests for Spacetime data binding system.
//!
//! These tests verify the full compilation pipeline from parsing through
//! code generation and analysis.

mod bare_ident_string_arg;
mod bind_cutover_invariant;
mod compile_report;
mod computed_codegen;
mod cursor_defaults;
mod data_kind_dispatch;
mod uses_prelude;
mod each_inline_html;
mod example_files;
mod export_site_routes;
mod export_validation;
mod i18n_locale;
mod inspector_protocol;
mod js_emit_snapshots;
mod landing_pages;
mod literate_tangle;
mod provenance_tests;
mod reactive_properties;
mod responsive_nested_directives;
mod ssg_showcase;
mod ssg_unroll_each;
mod ssg_unroll_template_invocations;
mod template_event_ordering;
mod template_refs;
mod test_runner_site;
mod wave1_primitives;
