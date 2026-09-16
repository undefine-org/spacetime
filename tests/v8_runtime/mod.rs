//! V8 Runtime for Spacetime Testing
//!
//! Provides a headless JavaScript environment using V8 engine with LinkeDOM
//! for DOM simulation. This enables fast behavioral testing of Spacetime components
//! without browser overhead.
//!
//! # Example
//!
//! ```no_run
//! use spacetime::parser::parse;
//! use spacetime::codegen::generate_data_binding_code;
//!
//! let source = r#"
//!     @import "stdlib/testing/test.st";
//!
//!     @test "element exists" {
//!         @fixture { <div class="test">Hello</div> }
//!         @then .test should exist
//!     }
//! "#;
//!
//! let ast = parse(source).unwrap();
//! let code = generate_data_binding_code(&ast, false);
//!
//! let results = V8TestContext::new()
//!     .with_runtime()
//!     .unwrap()
//!     .load_compiled(&code.js)
//!     .unwrap()
//!     .run_tests(None);
//!
//! assert!(results.all_passed());
//! ```

mod attr_mutation_tests;
mod easing_tests;
mod prev_signal_tests;
mod bind_class_tests;
mod compute_fn_tests;
mod context;
mod data_registry_tests;
mod derived_signal_tests;
mod data_source_tests;
mod dev_mode_guard_tests;
mod editable_model_tests;
mod element_deps_tests;
mod exports_tests;
mod filter_tests;
mod media_dedup_tests;
mod param_list_tests;
mod pipeline_fixes_tests;
mod purity_tests;
mod reactive_properties_tests;
mod realtime_tests;
mod root_scope_tests;
mod scope_resolution_tests;
mod scoped_state_tests;
mod selector_init_tests;
mod templates_tests;
mod timeline_handle_tests;
mod timeline_macro_tests;

pub use context::{TestResults, V8TestContext};
mod per_instance_compiled_tests;
