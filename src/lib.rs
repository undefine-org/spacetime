//! Spacetime - Declarative Animation DSL
//!
//! # Core Concept
//!
//! **Timelines are the universal abstraction.** All dynamic behavior on a page
//! can be described as transformations driven by timeline progress.
//!
//! ## Timeline Types
//!
//! - **Scroll**: Progress 0-1 based on element's position in viewport
//! - **Time**: Duration-based animation with iterations and alternation
//! - **Loop**: Cyclical time with configurable period (infinite motion)
//! - **Mouse**: Position-based, for parallax and cursor-following effects
//! - **Event**: Triggered by hover, click, focus, or intersection
//!
//! ## Usage
//!
//! ```rust,ignore
//! use spacetime::{parse, Compiler};
//!
//! let ast = parse(".hero { @scroll { opacity: 0 -> 1 } }").unwrap();
//! let compiled = Compiler::from_ast(&ast).compile();
//! // compiled.css - CSS with custom properties
//! // compiled.js  - Runtime JavaScript
//! ```
//!
//! ## CSS Variable Hooks
//!
//! Each timeline exposes its progress as a CSS custom property:
//! `--st-{timeline-id}-progress` (value 0-1)
//!
//! This allows pure CSS animations to hook into timelines:
//!
//! ```css
//! .element {
//!     transform: translateX(calc(var(--st-scroll-progress) * 100px));
//! }
//! ```

/// Zero-cost profiling span — compiles away when `perf-trace` feature is absent.
#[cfg(feature = "perf-trace")]
macro_rules! profile_span {
    ($name:expr) => {
        let _span = tracing::info_span!($name).entered();
    };
    ($name:expr, $($key:tt)*) => {
        let _span = tracing::info_span!($name, $($key)*).entered();
    };
}

#[cfg(not(feature = "perf-trace"))]
macro_rules! profile_span {
    ($name:expr) => {};
    ($name:expr, $($key:tt)*) => {};
}

pub(crate) use profile_span;

/// Ensure the V8 platform is initialized exactly once.
///
/// V8 requires single platform initialization — calling init_platform()
/// more than once is undefined behavior. All V8 Runtime creation must
/// go through this function first.
#[cfg(feature = "headless")]
pub fn ensure_v8_initialized() {
    use std::sync::Once;
    static V8_INIT: Once = Once::new();
    V8_INIT.call_once(|| {
        rustyscript::init_platform(4, true);
    });
}

pub mod analysis;
pub mod build_runtime;
#[cfg(feature = "cdp")]
pub mod cdp;
#[cfg(feature = "cdp")]
pub mod render;
pub mod cli;
pub mod color;
/// Structured comments (PLAN-123): THE `//@` scanner, the record model, and
/// the `.comments/` sidecar store. One scanner, three readers (pill routes,
/// `check`, MCP) — see the module docs for why that count is load-bearing.
pub mod comments;
pub mod compiler;
pub mod coverage;
pub mod debugger;
pub mod dev_server;
pub mod diagnostics;
pub mod edn;
pub mod editable;
pub mod emit;
pub mod error;
pub mod export;
pub mod host_package;
pub mod host_push_v2;
pub mod html;
pub mod introspect;
pub mod ir;
pub mod literate;
#[cfg(any(feature = "lsp", feature = "wasm"))]
pub mod lsp;
pub mod mcp;
pub mod metasystem;
pub mod migrate;
pub mod parser;
pub mod pipeline;
pub mod profiler;
pub mod rung;
pub mod serializer;
pub mod server;
pub mod stdlib_embedded;
pub mod sync;
pub mod syntax;
pub mod test_runner;
pub mod toolchain;
pub mod types;
pub mod treesitter_gen;
pub mod type_system;
pub mod utils;
pub mod validation;
pub mod vendor;
pub mod watcher;

// Browser testing (Playwright-based)
pub mod browser_test;

// WASM bindings
#[cfg(feature = "wasm")]
pub mod wasm;

pub use analysis::{
    BindingAnalysis, CompileAnalysis, ComputedAnalysis, DataAnalysis, DataSource, FunctionAnalysis,
    StateInfo, StateMachineAnalysis, TemplateAnalysis, TransitionInfo, TypeAnalysis,
};
pub use color::{Color, ColorSpace};
pub use compiler::{
    CompileCache, CompileOptions, CompiledSpacetime, Compiler, RegistrySource, compile,
};
#[cfg(feature = "headless")]
pub use compiler::{ValidationResult, compile_and_validate, validate_output};
pub use debugger::{DebuggerConfig, PanelPosition, generate_debug_panel, generate_debug_runtime};
pub use diagnostics::{Diagnostic, DiagnosticCode, DiagnosticCollector, Severity, SourceSpan};
pub use metasystem::MetaRegistry;
pub use parser::{ParseErrors, StFile, parse};
pub use profiler::{BundleProfile, ImageInfo, ImageProfile};
pub use serializer::serialize;
pub use test_runner::{
    CompileError, CompileOptions as TestCompileOptions, DiscoverOptions, TestError, TestResults,
    compile_test_html, compile_test_html_with_options, discover_tests, discover_tests_with_options,
    parse_test_results,
};
pub use type_system::{
    ResolvedType, ResolvedTypeField, TypeRegistry, generate_json_schema,
    validate_json_against_schema,
};
pub use watcher::{ChangeKind, FileChange, TestWatcher};

// Output validation (JS/CSS syntax checking)
#[cfg(feature = "headless")]
pub use validation::{CssSyntaxError, JsSyntaxError, validate_css, validate_js};

// Dev edit protocol for live content editing
pub use sync::{ClientMessage, HandleResult, ServerMessage, handle_message};

// HTML-aware validation
pub use html::{
    HtmlContext, RecommendationConfig, SelectorMatch, detect_html_context,
    detect_html_context_for_st, parse_html_context, validate_with_html,
};

// LSP support
#[cfg(feature = "lsp")]
pub use lsp::{
    DirectiveParam, DirectiveSignature, DocumentState, DocumentStore, FormRegistry, PositionMapper,
    SpacetimeLsp, run_lsp,
};

// Form Registry (available with LSP or WASM)
#[cfg(all(feature = "wasm", not(feature = "lsp")))]
pub use lsp::{DirectiveParam, DirectiveSignature, FormRegistry};

// WASM exports (renamed to avoid collision with compiler::compile)
#[cfg(feature = "wasm")]
pub use wasm::{compile as wasm_compile, completions, diagnostics, hover};
