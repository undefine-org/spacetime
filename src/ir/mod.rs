//! Intermediate Representation for Spacetime compiler
//!
//! These types enable deferred stringification and introspection.
//! Code flows: Expand -> IR -> Emit (strings generated only at emit stage)

pub mod builder;
pub mod code;

#[cfg(test)]
mod tests;

pub use code::*;
