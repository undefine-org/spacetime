//! Metasystem for Spacetime
//!
//! This module implements the % metasystem that allows primitives and macros
//! to be defined in Spacetime. Primitives emit JavaScript and export reactive
//! signals, while macros create @ patterns by composing primitives.
//!
//! The % system is compile-time only - it defines how patterns work,
//! but is not present in runtime output.
//!
//! ## Architecture
//!
/// ```text
/// Spacetime Source -> Parse -> Pipeline (Evaluate → Resolve → Sort → Expand → Emit)
///                                 |
///                            %binds {}  → Primitives → JS/CSS
///                            %derives {}
///                            %states {}  → state CSS + bindState JS
/// ```
mod diagnostics;
pub(crate) mod expand;
pub(crate) mod incremental_cache;
pub mod module;
mod registry;
pub mod relationships;
pub mod signature;
mod validate;

pub use crate::emit::metasystem_codegen::{
    GeneratedPrimitiveIR, PrimitiveArgs, generate_primitive_ir, generate_primitive_ir_with_registry,
};
pub use diagnostics::to_diagnostic;
pub(crate) use expand::convert_properties_to_keyframes_js;
pub use expand::{MacroExpansionError, MacroExpansionErrorKind};
pub use registry::{
    MetaRegistry, MetaRegistryError, MetaRegistryErrorKind, STDLIB_DIRS, TransformRegistry,
    TransformRule, macro_scope_matches,
};
pub use validate::{
    ValidationError, ValidationErrorKind, validate_macro, validate_migration,
    validate_migration_chains, validate_primitive,
};

#[cfg(any(feature = "lsp", feature = "wasm"))]
pub use validate::validate_scope_form_matches;

#[cfg(test)]
mod tests;
