//! Tree-sitter grammar generator for Spacetime
//!
//! Generates `tree-sitter-spacetime/grammar.js` from the compiler's meta-AST.
//! The compiler parses stdlib + user `.st` files into structured `MacroDefAst`
//! with `FormClause` patterns. This module walks those structures and emits
//! tree-sitter rules.

mod capture_map;
mod emit;
mod walker;

pub use emit::generate_grammar;
pub use walker::extract_directives;

use crate::parser::meta_ast::{CaptureModifier, CaptureType, ParamDefault};

/// Intermediate representation for a directive rule
#[derive(Debug, Clone)]
pub struct DirectiveRule {
    /// Directive name (e.g., "data", "fade-in")
    pub name: String,
    /// Parameters in parentheses
    pub params: Vec<ParamRule>,
    /// Inline captures before parentheses (e.g., @on $event:event)
    pub inline_elements: Vec<InlineRule>,
    /// Whether the directive has a body block
    pub has_body: bool,
    /// Body parameter captures (for directives with structured body content)
    pub body_params: Vec<ParamRule>,
}

/// Rule for a named parameter
#[derive(Debug, Clone)]
pub struct ParamRule {
    /// Parameter name
    pub name: String,
    /// Elements making up this parameter's pattern
    pub elements: Vec<InlineRule>,
    /// Default value if any
    pub default: Option<ParamDefault>,
}

/// Rule for an inline element (capture or literal)
#[derive(Debug, Clone)]
pub enum InlineRule {
    /// A capture: $varname:type
    Capture {
        var_name: String,
        capture_type: CaptureType,
        modifier: CaptureModifier,
        /// For "as $alias:type" patterns
        alias: Option<Box<InlineRule>>,
    },
    /// A literal token: ":", "->", keyword
    Literal(String),
    /// Comparison operator with capture: >= $count:number
    Comparison {
        operator: String,
        capture_type: CaptureType,
        modifier: CaptureModifier,
    },
    /// Keyword block: keyword { captures }modifier
    KeywordBlock {
        keyword: String,
        body_params: Vec<ParamRule>,
        modifier: CaptureModifier,
    },
    /// Pseudo-selector: (:name { captures })modifier
    PseudoSelector {
        name: String,
        body_params: Vec<ParamRule>,
        modifier: CaptureModifier,
    },
    /// Pseudo-class: :name { captures }
    PseudoClass {
        name: String,
        body_params: Vec<ParamRule>,
    },
}

/// Custom capture type definition for tree-sitter
#[derive(Debug, Clone)]
pub struct CustomTypeRule {
    /// Type name
    pub name: String,
    /// Tree-sitter rule body
    pub rule: String,
}
